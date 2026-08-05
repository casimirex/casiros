//! Tenant, workspace, and principal identity types.
//!
//! These value objects live in the Domain Layer because every persisted resource
//! in CASIROS must be scoped to an authenticated principal. They contain no I/O
//! and no infrastructure logic.

use std::fmt;

/// A globally unique tenant identifier.
///
/// Tenants represent the top-level billing/organizational boundary (e.g., a
/// company or deployment). Every snapshot, simulation, and audit event belongs
/// to exactly one tenant.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TenantId(String);

impl TenantId {
    /// Creates a new tenant identifier.
    ///
    /// # Errors
    ///
    /// Returns an error if the provided id is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use casiros_core::tenant::TenantId;
    ///
    /// let id = TenantId::new("tenant_abc123").unwrap();
    /// assert_eq!(id.as_str(), "tenant_abc123");
    /// ```
    pub fn new(id: impl Into<String>) -> Result<Self, TenantError> {
        let value = id.into();
        if value.is_empty() {
            return Err(TenantError::EmptyId);
        }
        return Ok(Self(value));
    }

    /// Returns the underlying identifier string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        return &self.0;
    }
}

impl fmt::Display for TenantId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        return self.0.fmt(f);
    }
}

/// A workspace identifier scoped to a tenant.
///
/// Workspaces partition data within a tenant (e.g., teams, projects, or
/// environments). A principal may operate in a default workspace or override
/// it per request.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WorkspaceId(String);

impl WorkspaceId {
    /// Creates a new workspace identifier.
    ///
    /// # Errors
    ///
    /// Returns an error if the provided id is empty.
    ///
    /// # Examples
    ///
    /// ```
    /// use casiros_core::tenant::WorkspaceId;
    ///
    /// let id = WorkspaceId::new("workspace_prod").unwrap();
    /// assert_eq!(id.as_str(), "workspace_prod");
    /// ```
    pub fn new(id: impl Into<String>) -> Result<Self, TenantError> {
        let value = id.into();
        if value.is_empty() {
            return Err(TenantError::EmptyId);
        }
        return Ok(Self(value));
    }

    /// Returns the underlying identifier string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        return &self.0;
    }
}

impl fmt::Display for WorkspaceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        return self.0.fmt(f);
    }
}

/// Identity of an authenticated caller.
///
/// A principal is resolved from an API key and carries the tenant/workspace
/// boundary that handlers must enforce on every mutating and read operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Principal {
    /// Tenant that owns the request.
    pub tenant_id: TenantId,

    /// Workspace in which the request operates.
    pub workspace_id: WorkspaceId,

    /// Identifier of the API key used to authenticate the request.
    pub api_key_id: String,
}

impl Principal {
    /// Creates a new principal.
    ///
    /// # Examples
    ///
    /// ```
    /// use casiros_core::tenant::{Principal, TenantId, WorkspaceId};
    ///
    /// let p = Principal::new(
    ///     TenantId::new("t1").unwrap(),
    ///     WorkspaceId::new("w1").unwrap(),
    ///     "key_001",
    /// );
    /// assert_eq!(p.tenant_id.as_str(), "t1");
    /// assert_eq!(p.workspace_id.as_str(), "w1");
    /// assert_eq!(p.api_key_id, "key_001");
    /// ```
    pub fn new(
        tenant_id: TenantId,
        workspace_id: WorkspaceId,
        api_key_id: impl Into<String>,
    ) -> Self {
        return Self {
            tenant_id,
            workspace_id,
            api_key_id: api_key_id.into(),
        };
    }
}

/// Errors that can occur when constructing tenant-scoped identifiers.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum TenantError {
    /// The identifier string was empty.
    #[error("tenant/workspace identifier must not be empty")]
    EmptyId,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tenant_id_rejects_empty() {
        assert!(matches!(TenantId::new(""), Err(TenantError::EmptyId)));
    }

    #[test]
    fn workspace_id_rejects_empty() {
        assert!(matches!(WorkspaceId::new(""), Err(TenantError::EmptyId)));
    }

    #[test]
    fn principal_holds_values() {
        let p = Principal::new(
            TenantId::new("t2").unwrap(),
            WorkspaceId::new("w2").unwrap(),
            "key_002",
        );
        assert_eq!(p.tenant_id.as_str(), "t2");
        assert_eq!(p.workspace_id.as_str(), "w2");
        assert_eq!(p.api_key_id, "key_002");
    }
}
