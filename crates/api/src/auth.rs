//! API key authentication and tenant-scoped rate limiting.
//!
//! Protected endpoints require either an `Authorization: Bearer <key>` header or
//! an `X-API-Key: <key>` header. Public paths (`/healthz`, `/openapi.json`, and
//! `/swagger-ui/*`) bypass authentication.
//!
//! After authentication, a [`Principal`] is resolved from the key and attached to
//! the request. Rate limiting is keyed by tenant/workspace so that separate
//! workspaces belonging to the same API key are throttled independently.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use actix_web::HttpMessage;
use actix_web::body::{BoxBody, MessageBody};
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::http::header;
use actix_web::{Error, HttpResponse, middleware::Next};
use casiros_core::tenant::Principal;
use tracing::{info, warn};

use crate::tenant::{InMemoryTenantResolver, TenantResolver};

/// Paths that are always accessible without authentication.
const PUBLIC_PATHS: [&str; 6] = [
    "/healthz",
    "/metrics",
    "/openapi.json",
    "/swagger-ui",
    "/swagger-ui/",
    "/api-docs",
];

/// Runtime authentication configuration.
#[derive(Debug, Clone, Default)]
pub struct AuthConfig {
    /// Optional set of valid API keys. When `None`, authentication is
    /// disabled and a warning is emitted at startup.
    api_keys: Option<HashSet<String>>,

    /// Maximum requests per minute allowed for a single tenant/workspace.
    rate_limit_rpm: u32,
}

impl AuthConfig {
    /// Creates a configuration for testing with explicit keys and rate limit.
    #[must_use]
    pub fn with_keys(keys: HashSet<String>, rate_limit_rpm: u32) -> Self {
        return Self {
            api_keys: Some(keys),
            rate_limit_rpm,
        };
    }
}

impl AuthConfig {
    /// Loads configuration from environment variables.
    ///
    /// - `CASIROS_API_KEYS`: comma-separated list of valid keys. If unset or
    ///   empty, authentication is disabled.
    /// - `CASIROS_RATE_LIMIT_RPM`: requests per minute per tenant/workspace.
    ///   Defaults to 60.
    ///
    /// # Panics
    ///
    /// Never panics. Logging statements are guarded by the `Option` state.
    #[allow(clippy::cognitive_complexity)]
    #[must_use]
    pub fn from_env() -> Self {
        let api_keys = parse_api_keys_from_env();
        let rate_limit_rpm = parse_rate_limit_from_env();
        log_auth_state(api_keys.as_ref(), rate_limit_rpm);

        return Self {
            api_keys,
            rate_limit_rpm,
        };
    }

    /// Returns true if authentication is required.
    #[must_use]
    pub fn enabled(&self) -> bool {
        return self.api_keys.is_some();
    }
}

/// In-memory sliding-window rate limiter keyed by tenant and workspace.
#[derive(Debug, Clone, Default)]
pub struct RateLimiter {
    /// Timestamp history per tenant/workspace, guarded by a mutex.
    requests: Arc<Mutex<HashMap<String, Vec<Instant>>>>,
}

impl RateLimiter {
    /// Creates a new rate limiter.
    #[must_use]
    pub fn new() -> Self {
        return Self::default();
    }

    /// Returns true if the tenant/workspace has exceeded `limit_rpm` requests in
    /// the last minute.
    ///
    /// # Panics
    ///
    /// Panics only if the internal mutex is poisoned, which should not happen
    /// in normal operation.
    #[must_use]
    pub fn is_rate_limited(&self, tenant_workspace_key: &str, limit_rpm: u32) -> bool {
        if limit_rpm == 0 {
            return false;
        }

        let now = Instant::now();
        let window = Duration::from_secs(60);
        let mut map = self.requests.lock().expect("rate limiter mutex poisoned");
        let history = map.entry(tenant_workspace_key.to_string()).or_default();
        history.retain(|ts| now.duration_since(*ts) < window);

        if history.len() >= limit_rpm as usize {
            return true;
        }

        history.push(now);
        return false;
    }
}

/// Authentication middleware: validates API keys, resolves tenants, and
/// enforces tenant-scoped rate limits.
///
/// Public paths skip validation. When `CASIROS_API_KEYS` is unset, all
/// protected paths are allowed and resolve to the default tenant/workspace.
///
/// # Errors
///
/// Returns an `actix_web::Error` response (wrapped in `Ok`) when the request is
/// unauthorized or rate-limited. Returns the underlying error if the inner
/// service fails.
///
/// # Panics
///
/// Panics only if the authentication configuration is internally inconsistent
/// (e.g. marked as enabled but missing API keys), which should never happen
/// because `AuthConfig::enabled()` is derived from the same `Option`.
pub async fn auth_middleware<B>(
    req: ServiceRequest,
    next: Next<B>,
    config: Arc<AuthConfig>,
    resolver: Arc<dyn TenantResolver>,
    limiter: Arc<RateLimiter>,
) -> Result<ServiceResponse<BoxBody>, Error>
where
    B: MessageBody + 'static,
{
    let path = req.request().path();
    if is_public_path(path) {
        return Ok(next.call(req).await?.map_into_boxed_body());
    }

    let key = extract_key(req.request());

    if config.enabled() {
        let Some(ref key) = key else {
            return Ok(req.into_response(HttpResponse::Unauthorized().json(
                crate::models::ErrorResponse {
                    error: "Missing API key".to_string(),
                },
            )));
        };
        let valid = config
            .api_keys
            .as_ref()
            .expect("enabled implies keys are present")
            .contains(key);
        if !valid {
            return Ok(req.into_response(HttpResponse::Unauthorized().json(
                crate::models::ErrorResponse {
                    error: "Invalid API key".to_string(),
                },
            )));
        }
    }

    let principal = match &key {
        Some(key) => resolver
            .resolve(key)
            .await
            .unwrap_or_else(default_principal),
        None => default_principal(),
    };

    let rate_limit_key = format!(
        "{}:{}",
        principal.tenant_id.as_str(),
        principal.workspace_id.as_str()
    );
    if limiter.is_rate_limited(&rate_limit_key, config.rate_limit_rpm) {
        return Ok(req.into_response(HttpResponse::TooManyRequests().json(
            crate::models::ErrorResponse {
                error: "Rate limit exceeded".to_string(),
            },
        )));
    }

    // Propagate the authenticated principal and key for observability.
    req.request().extensions_mut().insert(principal.clone());
    req.request()
        .extensions_mut()
        .insert(ApiKeyContext { key: key.clone() });

    return Ok(next.call(req).await?.map_into_boxed_body());
}

/// Context attached to requests that have passed authentication.
#[derive(Debug, Clone)]
pub struct ApiKeyContext {
    /// The API key used to authenticate the request, if any.
    pub key: Option<String>,
}

/// Returns the default principal used when no explicit mapping exists.
fn default_principal() -> Principal {
    let tenant = casiros_core::tenant::TenantId::new("tenant_default")
        .expect("static default tenant is valid");
    let workspace = casiros_core::tenant::WorkspaceId::new("workspace_default")
        .expect("static default workspace is valid");
    return Principal::new(tenant, workspace, "default");
}

fn is_public_path(path: &str) -> bool {
    return PUBLIC_PATHS
        .iter()
        .any(|public| path == *public || path.starts_with(public));
}

fn parse_api_keys_from_env() -> Option<HashSet<String>> {
    return std::env::var("CASIROS_API_KEYS")
        .ok()
        .filter(|s| !s.is_empty())
        .map(|s| s.split(',').map(str::trim).map(String::from).collect());
}

fn parse_rate_limit_from_env() -> u32 {
    return std::env::var("CASIROS_RATE_LIMIT_RPM")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(60);
}

#[allow(clippy::cognitive_complexity)]
fn log_auth_state(api_keys: Option<&HashSet<String>>, rate_limit_rpm: u32) {
    if let Some(keys) = api_keys {
        info!(
            "API authentication enabled with {} key(s) and {} req/min per tenant/workspace",
            keys.len(),
            rate_limit_rpm
        );
    } else {
        warn!("CASIROS_API_KEYS is not set; API authentication is disabled");
    }
}

fn extract_key(req: &actix_web::HttpRequest) -> Option<String> {
    // Prefer X-API-Key header.
    if let Some(value) = req.headers().get("X-API-Key") {
        if let Ok(text) = value.to_str() {
            return Some(text.trim().to_string());
        }
    }

    // Fall back to Authorization: Bearer <key>.
    if let Some(value) = req.headers().get(header::AUTHORIZATION) {
        if let Ok(text) = value.to_str() {
            let trimmed = text.trim();
            if trimmed.to_lowercase().starts_with("bearer ") {
                return Some(trimmed[7..].trim().to_string());
            }
        }
    }

    return None;
}

/// Builds the default [`TenantResolver`] for the API server.
///
/// Uses `CASIROS_API_KEY_TENANTS` when set; otherwise every authenticated key
/// resolves to the default tenant/workspace.
#[must_use]
pub fn build_tenant_resolver() -> Arc<dyn TenantResolver> {
    return if std::env::var("CASIROS_API_KEY_TENANTS").is_ok() {
        Arc::new(InMemoryTenantResolver::from_env())
    } else {
        Arc::new(InMemoryTenantResolver::default_for_any_key())
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_limiter_allows_under_limit() {
        let limiter = RateLimiter::new();
        assert!(!limiter.is_rate_limited("t:w", 2));
        assert!(!limiter.is_rate_limited("t:w", 2));
    }

    #[test]
    fn rate_limiter_blocks_at_limit() {
        let limiter = RateLimiter::new();
        assert!(!limiter.is_rate_limited("t:w", 1));
        assert!(limiter.is_rate_limited("t:w", 1));
    }

    #[test]
    fn rate_limiter_keys_are_independent() {
        let limiter = RateLimiter::new();
        assert!(!limiter.is_rate_limited("t1:w1", 1));
        assert!(!limiter.is_rate_limited("t2:w2", 1));
    }
}
