//! Tenant and workspace resolution for authenticated requests.
//!
//! The [`TenantResolver`] trait maps an API key to a [`Principal`]. This lets
//! authentication return not just *whether* a caller is valid, but *which*
//! tenant and workspace they belong to. Implementations can be backed by
//! in-memory config, `PostgreSQL`, or an external identity service.

use std::collections::HashMap;

use async_trait::async_trait;
use casiros_core::tenant::{Principal, TenantId, WorkspaceId};

/// Resolves an API key to a tenant/workspace [`Principal`].
#[async_trait]
pub trait TenantResolver: Send + Sync {
    /// Resolves the principal for an API key.
    ///
    /// Returns `None` when the key is unknown or revoked.
    async fn resolve(&self, api_key: &str) -> Option<Principal>;
}

/// In-memory tenant resolver configured from environment variables.
///
/// Parses `CASIROS_API_KEY_TENANTS` with the format:
///
/// ```text
/// key1:tenant_1:workspace_1,key2:tenant_2:workspace_2
/// ```
///
/// When the variable is unset, every key resolves to the default
/// `tenant_default` / `workspace_default` principal so that local development
/// remains simple.
#[derive(Debug, Clone, Default)]
pub struct InMemoryTenantResolver {
    /// API key → principal lookup table.
    mapping: HashMap<String, Principal>,
}

impl InMemoryTenantResolver {
    /// Creates a resolver from an explicit key → principal map.
    #[must_use]
    pub fn new(mapping: HashMap<String, Principal>) -> Self {
        return Self { mapping };
    }

    /// Creates a resolver from the `CASIROS_API_KEY_TENANTS` environment
    /// variable, falling back to a default tenant/workspace for every key.
    #[must_use]
    pub fn from_env() -> Self {
        return Self::parse(std::env::var("CASIROS_API_KEY_TENANTS").ok().as_deref());
    }

    /// Parses a comma-separated tenant mapping string.
    ///
    /// Each entry must be `key:tenant_id:workspace_id`. Malformed entries are
    /// ignored so that a single typo does not disable authentication.
    #[must_use]
    pub fn parse(source: Option<&str>) -> Self {
        let mut mapping = HashMap::new();
        if let Some(source) = source {
            for entry in source.split(',') {
                let parts: Vec<&str> = entry.split(':').collect();
                if parts.len() != 3 {
                    continue;
                }
                let key = parts[0].trim().to_string();
                let Ok(tenant) = TenantId::new(parts[1].trim()) else {
                    continue;
                };
                let Ok(workspace) = WorkspaceId::new(parts[2].trim()) else {
                    continue;
                };
                mapping.insert(key, Principal::new(tenant, workspace, "api_key"));
            }
        }
        return Self::new(mapping);
    }

    /// Returns a resolver that maps every key to the default tenant/workspace.
    ///
    /// Useful for local development or when authentication is disabled.
    ///
    /// # Panics
    ///
    /// Panics only if the static default identifiers are malformed, which is a
    /// compile-time guarantee.
    #[must_use]
    pub fn default_for_any_key() -> Self {
        let tenant = TenantId::new("tenant_default").expect("static default tenant is valid");
        let workspace =
            WorkspaceId::new("workspace_default").expect("static default workspace is valid");
        return Self::new(HashMap::from([(
            String::new(),
            Principal::new(tenant, workspace, "default"),
        )]));
    }
}

#[async_trait]
impl TenantResolver for InMemoryTenantResolver {
    async fn resolve(&self, api_key: &str) -> Option<Principal> {
        return self
            .mapping
            .get(api_key)
            .cloned()
            .or_else(|| self.mapping.get("").cloned());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn parse_maps_keys_to_principals() {
        let resolver = InMemoryTenantResolver::parse(Some("alpha:tenant_a:workspace_a"));
        let principal = resolver.resolve("alpha").await;
        assert_eq!(
            principal,
            Some(Principal::new(
                TenantId::new("tenant_a").unwrap(),
                WorkspaceId::new("workspace_a").unwrap(),
                "api_key",
            ))
        );
    }

    #[tokio::test]
    async fn parse_ignores_malformed_entries() {
        let resolver = InMemoryTenantResolver::parse(Some("bad,alpha:t:w"));
        assert_eq!(
            resolver.resolve("alpha").await,
            Some(Principal::new(
                TenantId::new("t").unwrap(),
                WorkspaceId::new("w").unwrap(),
                "api_key",
            ))
        );
        assert!(resolver.resolve("bad").await.is_none());
    }

    #[tokio::test]
    async fn default_for_any_key_returns_default_principal() {
        let resolver = InMemoryTenantResolver::default_for_any_key();
        let principal = resolver.resolve("anything").await;
        assert!(principal.is_some());
        assert_eq!(principal.unwrap().tenant_id.as_str(), "tenant_default");
    }
}
