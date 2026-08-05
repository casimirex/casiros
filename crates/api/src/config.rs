//! Application configuration for the CASIROS API server.
//!
//! Configuration is layered from an embedded default, an optional
//! `config/default.toml`, and environment variables prefixed with `CASIROS`.
//!
//! The prefix is joined to the key with the same `__` separator used between
//! nested keys. A top-level key is therefore `CASIROS__BIND_ADDR`, and a nested
//! key is `CASIROS__SNAPSHOT__BACKEND` (overriding `[snapshot].backend`). A
//! single underscore after the prefix is silently ignored.
//!
//! Note that `CASIROS_API_KEYS`, `CASIROS_ADMIN_KEY`, `CASIROS_RATE_LIMIT_RPM`,
//! `CASIROS_API_KEY_TENANTS`, `CASIROS_API_VERSION`, and `CASIROS_OTLP_ENDPOINT`
//! are read directly via `std::env::var` elsewhere in the crate and keep a
//! single underscore.

use config::{Config, ConfigError, Environment, File, FileFormat};
use serde::{Deserialize, Serialize};

/// Embedded default configuration used when `config/default.toml` is missing.
const DEFAULT_CONFIG: &str = include_str!("../../../config/default.toml");

/// Runtime application configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct AppConfig {
    /// Address and port the HTTP server binds to.
    pub bind_addr: String,

    /// Tracing log level filter (e.g. `info`, `debug`, `casiros_api=debug`).
    pub log_level: String,

    /// Default per-API-key rate limit in requests per minute.
    pub rate_limit_rpm: u32,

    /// Snapshot persistence backend selection.
    pub snapshot: SnapshotConfig,

    /// `PostgreSQL` connection settings when the snapshot backend is `postgres`.
    pub postgres: PostgresConfig,
}

/// Snapshot persistence backend configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct SnapshotConfig {
    /// Backend kind: `"memory"` or `"postgres"`.
    pub backend: String,
}

/// `PostgreSQL` connection configuration.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct PostgresConfig {
    /// Connection URL passed to `SQLx`.
    pub url: String,
}

/// Errors that can occur while loading application configuration.
#[derive(Debug, thiserror::Error)]
pub enum AppConfigError {
    /// Underlying configuration builder error.
    #[error("Configuration error: {0}")]
    Config(#[from] ConfigError),
}

impl AppConfig {
    /// Loads configuration from an embedded default, an optional
    /// `config/default.toml`, and `CASIROS_*` environment variables.
    ///
    /// # Errors
    ///
    /// Returns [`AppConfigError::Config`] if the TOML is malformed or required
    /// values are missing.
    pub fn load() -> Result<Self, AppConfigError> {
        let config = Config::builder()
            .add_source(File::from_str(DEFAULT_CONFIG, FileFormat::Toml))
            .add_source(File::with_name("config/default").required(false))
            .add_source(Environment::with_prefix("CASIROS").separator("__"))
            .build()?;

        return Ok(config.try_deserialize()?);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_loads() {
        let config = AppConfig::load().expect("default config should load");
        assert!(!config.bind_addr.is_empty());
        assert!(!config.log_level.is_empty());
    }

    /// Pins the environment-variable naming contract.
    ///
    /// The builder uses `Environment::with_prefix("CASIROS").separator("__")`.
    /// The `config` crate joins the prefix to the key with the separator, so a
    /// top-level key needs `CASIROS__BIND_ADDR` and a nested key needs
    /// `CASIROS__SNAPSHOT__BACKEND` — a single underscore after the prefix is
    /// silently ignored, which once left the server running on defaults while
    /// operators believed their overrides had applied.
    ///
    /// This builds the same source the loader uses and asserts the resulting
    /// key names, rather than mutating process environment variables (the
    /// crate forbids the `unsafe` blocks that `set_var` now requires, and
    /// mutation would race other tests).
    #[test]
    fn env_overrides_use_double_underscore_after_prefix() {
        let source = Environment::with_prefix("CASIROS")
            .separator("__")
            .source(Some({
                let mut vars = std::collections::HashMap::new();
                vars.insert(
                    "CASIROS__BIND_ADDR".to_string(),
                    "10.0.0.1:9999".to_string(),
                );
                vars.insert(
                    "CASIROS__SNAPSHOT__BACKEND".to_string(),
                    "postgres".to_string(),
                );
                // A single underscore after the prefix must NOT be recognised.
                vars.insert(
                    "CASIROS_SNAPSHOT__BACKEND".to_string(),
                    "ignored".to_string(),
                );
                vars
            }));

        let config = Config::builder()
            .add_source(File::from_str(DEFAULT_CONFIG, FileFormat::Toml))
            .add_source(source)
            .build()
            .expect("config with env overrides should build");
        let config: AppConfig = config.try_deserialize().expect("config should deserialize");

        assert_eq!(config.bind_addr, "10.0.0.1:9999");
        assert_eq!(config.snapshot.backend, "postgres");
    }
}
