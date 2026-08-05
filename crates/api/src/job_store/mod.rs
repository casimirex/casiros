//! Simulation job store backends.
//!
//! Infrastructure-layer implementations of the
//! [`casiros_dag::job::JobStore`] trait declared in the Application Layer.
//! [`JobStoreHandle`] type-erases the chosen backend so Actix-Web handlers can
//! accept one concrete type regardless of whether jobs live in memory or in
//! `PostgreSQL`.

use std::sync::Arc;

use async_trait::async_trait;
use casiros_core::job::{JobId, JobProgress};
use casiros_core::tenant::{TenantId, WorkspaceId};
use casiros_dag::DagError;
use casiros_dag::job::{JobStore, SimulationJob};

/// A concrete, object-safe wrapper around any [`JobStore`].
///
/// Without this wrapper the API would have to name a single concrete store in
/// its handler signatures, which is what previously pinned every deployment to
/// the in-memory backend regardless of configuration.
#[derive(Clone)]
pub struct JobStoreHandle {
    /// The inner job store, type-erased.
    inner: Arc<dyn JobStore>,
}

impl JobStoreHandle {
    /// Wraps any [`JobStore`] implementation.
    #[must_use]
    pub fn new<S: JobStore + 'static>(store: S) -> Self {
        return Self {
            inner: Arc::new(store),
        };
    }
}

#[async_trait]
impl JobStore for JobStoreHandle {
    async fn enqueue(&self, job: SimulationJob) -> Result<(), DagError> {
        return self.inner.enqueue(job).await;
    }

    async fn claim_next(&self, worker_id: &str) -> Option<SimulationJob> {
        return self.inner.claim_next(worker_id).await;
    }

    async fn update_progress(&self, id: &JobId, progress: &JobProgress) -> Result<(), DagError> {
        return self.inner.update_progress(id, progress).await;
    }

    async fn complete(&self, id: &JobId, result: serde_json::Value) -> Result<(), DagError> {
        return self.inner.complete(id, result).await;
    }

    async fn fail(&self, id: &JobId, error: String) -> Result<(), DagError> {
        return self.inner.fail(id, error).await;
    }

    async fn cancel(&self, id: &JobId) -> Result<bool, DagError> {
        return self.inner.cancel(id).await;
    }

    async fn get(
        &self,
        tenant: &TenantId,
        workspace: &WorkspaceId,
        id: &JobId,
    ) -> Result<SimulationJob, DagError> {
        return self.inner.get(tenant, workspace, id).await;
    }
}

pub mod in_memory;
pub mod postgres;

pub use in_memory::InMemoryJobStore;
pub use postgres::PostgresJobStore;
