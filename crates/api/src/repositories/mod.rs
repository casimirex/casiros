//! Storage backend implementations for snapshot persistence.
//!
//! This module provides infrastructure-layer repositories that implement the
//! [`casiros_dag::repository::SnapshotRepository`] trait defined in the
//! Application Layer.

use std::sync::Arc;

use async_trait::async_trait;
use casiros_dag::DagError;
use casiros_dag::persistence::EngineSnapshot;
use casiros_dag::repository::{SnapshotRepository, SnapshotSummary};

/// A concrete, object-safe wrapper around any [`SnapshotRepository`].
///
/// This lets Actix-Web handlers accept a single concrete type while still
/// supporting in-memory, Postgres, and S3 backends at runtime.
#[derive(Clone)]
pub struct SnapshotRepo {
    /// The inner repository, type-erased.
    inner: Arc<dyn SnapshotRepository>,
}

impl SnapshotRepo {
    /// Wraps any [`SnapshotRepository`] implementation.
    #[must_use]
    pub fn new<R: SnapshotRepository + 'static>(repo: R) -> Self {
        return Self {
            inner: Arc::new(repo),
        };
    }
}

#[async_trait]
impl SnapshotRepository for SnapshotRepo {
    async fn save(&self, id: &str, snapshot: &EngineSnapshot) -> Result<(), DagError> {
        return self.inner.save(id, snapshot).await;
    }

    async fn load(&self, id: &str) -> Result<EngineSnapshot, DagError> {
        return self.inner.load(id).await;
    }

    async fn delete(&self, id: &str) -> Result<(), DagError> {
        return self.inner.delete(id).await;
    }

    async fn list(&self) -> Result<Vec<SnapshotSummary>, DagError> {
        return self.inner.list().await;
    }
}

mod in_memory {
    //! Re-export the Application Layer in-memory repository for convenience.
    pub use casiros_dag::repository::InMemorySnapshotRepository;
}

pub use in_memory::InMemorySnapshotRepository;
