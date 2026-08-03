//! Application configuration for the CASIROS API server.
//!
//! Configuration is layered from an embedded default, an optional
//! `config/default.toml`, and environment variables prefixed with `CASIROS_`.
//! Nested keys are separated by double underscores, so `CASIROS_SNAPSHOT__BACKEND`
//! overrides `[snapshot].backend`.

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
}
