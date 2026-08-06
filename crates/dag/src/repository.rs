//! Storage-agnostic snapshot persistence.
//!
//! The [`SnapshotRepository`] trait is defined in the Application Layer so that
//! infrastructure implementations (Postgres, S3, in-memory) can be swapped
//! without changing the graph engine or the API handlers. All operations are
//! scoped to a [`TenantId`] and [`WorkspaceId`] from the Domain Layer.

use std::collections::HashMap;

use async_trait::async_trait;
use casiros_core::tenant::{TenantId, WorkspaceId};

use crate::error::DagError;
use crate::persistence::EngineSnapshot;

/// A snapshot summary returned by listing operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotSummary {
    /// Unique identifier of the snapshot.
    pub id: String,

    /// Optional human-readable name.
    pub name: Option<String>,
}

/// Storage backend for [`EngineSnapshot`] persistence.
///
/// Implementations are infrastructure-specific and may use `PostgreSQL`, S3, a
/// local file system, or an in-memory map for testing. Every method is scoped to
/// a tenant/workspace pair; cross-tenant access must be rejected by
/// implementations.
#[async_trait]
pub trait SnapshotRepository: Send + Sync {
    /// Persists a snapshot under the given identifier.
    ///
    /// # Errors
    ///
    /// Returns [`DagError::Repository`] if the backend fails.
    async fn save(
        &self,
        tenant: TenantId,
        workspace: WorkspaceId,
        id: &str,
        snapshot: &EngineSnapshot,
    ) -> Result<(), DagError>;

    /// Loads a previously persisted snapshot.
    ///
    /// # Errors
    ///
    /// Returns [`DagError::Repository`] if the snapshot is missing, the tenant or
    /// workspace do not match, or the backend fails.
    async fn load(
        &self,
        tenant: TenantId,
        workspace: WorkspaceId,
        id: &str,
    ) -> Result<EngineSnapshot, DagError>;

    /// Deletes a persisted snapshot.
    ///
    /// # Errors
    ///
    /// Returns [`DagError::Repository`] if the snapshot is missing or the backend
    /// fails.
    async fn delete(
        &self,
        tenant: TenantId,
        workspace: WorkspaceId,
        id: &str,
    ) -> Result<(), DagError>;

    /// Lists all persisted snapshots within the tenant/workspace scope.
    ///
    /// # Errors
    ///
    /// Returns [`DagError::Repository`] if the backend fails.
    async fn list(
        &self,
        tenant: TenantId,
        workspace: WorkspaceId,
    ) -> Result<Vec<SnapshotSummary>, DagError>;
}

/// Storage key for an in-memory snapshot: `(tenant, workspace, id)`.
type SnapshotKey = (TenantId, WorkspaceId, String);

/// Shared, mutex-guarded snapshot lookup table.
type SnapshotStore = std::sync::Arc<std::sync::Mutex<HashMap<SnapshotKey, EngineSnapshot>>>;

/// An in-memory implementation of [`SnapshotRepository`] for tests and
/// lightweight deployments.
#[derive(Debug, Default, Clone)]
pub struct InMemorySnapshotRepository {
    /// Stored snapshots keyed by tenant, workspace, and id.
    snapshots: SnapshotStore,
}

impl InMemorySnapshotRepository {
    /// Creates a new empty repository.
    #[must_use]
    pub fn new() -> Self {
        return Self::default();
    }
}

#[async_trait]
impl SnapshotRepository for InMemorySnapshotRepository {
    async fn save(
        &self,
        tenant: TenantId,
        workspace: WorkspaceId,
        id: &str,
        snapshot: &EngineSnapshot,
    ) -> Result<(), DagError> {
        let mut map = self.snapshots.lock().map_err(|_| DagError::Repository {
            message: "in-memory repository mutex poisoned".to_string(),
        })?;
        map.insert((tenant, workspace, id.to_string()), snapshot.clone());
        return Ok(());
    }

    async fn load(
        &self,
        tenant: TenantId,
        workspace: WorkspaceId,
        id: &str,
    ) -> Result<EngineSnapshot, DagError> {
        let map = self.snapshots.lock().map_err(|_| DagError::Repository {
            message: "in-memory repository mutex poisoned".to_string(),
        })?;
        return map
            .get(&(tenant, workspace, id.to_string()))
            .cloned()
            .ok_or_else(|| DagError::Repository {
                message: format!("snapshot '{id}' not found"),
            });
    }

    async fn delete(
        &self,
        tenant: TenantId,
        workspace: WorkspaceId,
        id: &str,
    ) -> Result<(), DagError> {
        let mut map = self.snapshots.lock().map_err(|_| DagError::Repository {
            message: "in-memory repository mutex poisoned".to_string(),
        })?;
        return map
            .remove(&(tenant, workspace, id.to_string()))
            .map(|_| ())
            .ok_or_else(|| DagError::Repository {
                message: format!("snapshot '{id}' not found"),
            });
    }

    async fn list(
        &self,
        tenant: TenantId,
        workspace: WorkspaceId,
    ) -> Result<Vec<SnapshotSummary>, DagError> {
        let map = self.snapshots.lock().map_err(|_| DagError::Repository {
            message: "in-memory repository mutex poisoned".to_string(),
        })?;
        return Ok(map
            .iter()
            .filter(|((t, w, _), _)| t == &tenant && w == &workspace)
            .map(|((_, _, id), _)| SnapshotSummary {
                id: id.clone(),
                name: None,
            })
            .collect());
    }
}
