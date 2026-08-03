//! Storage-agnostic snapshot persistence.
//!
//! The [`SnapshotRepository`] trait is defined in the Application Layer so that
//! infrastructure implementations (Postgres, S3, in-memory) can be swapped
//! without changing the graph engine or the API handlers.

use std::collections::HashMap;

use async_trait::async_trait;

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
/// local file system, or an in-memory map for testing.
#[async_trait]
pub trait SnapshotRepository: Send + Sync {
    /// Persists a snapshot under the given identifier.
    ///
    /// # Errors
    ///
    /// Returns [`DagError::Repository`] if the backend fails.
    async fn save(&self, id: &str, snapshot: &EngineSnapshot) -> Result<(), DagError>;

    /// Loads a previously persisted snapshot.
    ///
    /// # Errors
    ///
    /// Returns [`DagError::Repository`] if the snapshot is missing or the backend
    /// fails.
    async fn load(&self, id: &str) -> Result<EngineSnapshot, DagError>;

    /// Deletes a persisted snapshot.
    ///
    /// # Errors
    ///
    /// Returns [`DagError::Repository`] if the snapshot is missing or the backend
    /// fails.
    async fn delete(&self, id: &str) -> Result<(), DagError>;

    /// Lists all persisted snapshot identifiers.
    ///
    /// # Errors
    ///
    /// Returns [`DagError::Repository`] if the backend fails.
    async fn list(&self) -> Result<Vec<SnapshotSummary>, DagError>;
}

/// An in-memory implementation of [`SnapshotRepository`] for tests and
/// lightweight deployments.
#[derive(Debug, Default, Clone)]
pub struct InMemorySnapshotRepository {
    /// Stored snapshots keyed by id.
    snapshots: std::sync::Arc<std::sync::Mutex<HashMap<String, EngineSnapshot>>>,
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
    async fn save(&self, id: &str, snapshot: &EngineSnapshot) -> Result<(), DagError> {
        let mut map = self.snapshots.lock().map_err(|_| DagError::Repository {
            message: "in-memory repository mutex poisoned".to_string(),
        })?;
        map.insert(id.to_string(), snapshot.clone());
        return Ok(());
    }

    async fn load(&self, id: &str) -> Result<EngineSnapshot, DagError> {
        let map = self.snapshots.lock().map_err(|_| DagError::Repository {
            message: "in-memory repository mutex poisoned".to_string(),
        })?;
        return map.get(id).cloned().ok_or_else(|| DagError::Repository {
            message: format!("snapshot '{id}' not found"),
        });
    }

    async fn delete(&self, id: &str) -> Result<(), DagError> {
        let mut map = self.snapshots.lock().map_err(|_| DagError::Repository {
            message: "in-memory repository mutex poisoned".to_string(),
        })?;
        return map
            .remove(id)
            .map(|_| ())
            .ok_or_else(|| DagError::Repository {
                message: format!("snapshot '{id}' not found"),
            });
    }

    async fn list(&self) -> Result<Vec<SnapshotSummary>, DagError> {
        let map = self.snapshots.lock().map_err(|_| DagError::Repository {
            message: "in-memory repository mutex poisoned".to_string(),
        })?;
        return Ok(map
            .keys()
            .map(|id| SnapshotSummary {
                id: id.clone(),
                name: None,
            })
            .collect());
    }
}
