//! Storage-agnostic simulation job persistence.
//!
//! The [`JobStore`] trait is declared in the Application Layer so infrastructure
//! implementations (`PostgreSQL`, in-memory) can be swapped without changing the
//! API handlers or the worker. Every read operation is scoped to a tenant and
//! workspace; cross-tenant access must be rejected by implementations.

use async_trait::async_trait;
use casiros_core::job::{JobId, JobProgress, JobStatus};
use casiros_core::tenant::{TenantId, WorkspaceId};

use crate::error::DagError;

/// A simulation job as stored and returned by a [`JobStore`].
#[derive(Debug, Clone)]
pub struct SimulationJob {
    /// Unique job identifier.
    pub id: JobId,

    /// Tenant that owns the job.
    pub tenant_id: TenantId,

    /// Workspace in which the job runs.
    pub workspace_id: WorkspaceId,

    /// Current lifecycle state.
    pub status: JobStatus,

    /// Serialised simulation request (JSON).
    pub request: serde_json::Value,

    /// How far the job has progressed.
    pub progress: JobProgress,

    /// Serialised simulation results, present when `status == Completed`.
    pub result: Option<serde_json::Value>,

    /// Error message, present when `status == Failed`.
    pub error: Option<String>,

    /// When the job was created.
    pub created_at: time::OffsetDateTime,

    /// When the job was last updated.
    pub updated_at: time::OffsetDateTime,
}

/// Backend for simulation job lifecycle.
///
/// Implementations must be thread-safe and must scope every read to the
/// caller's tenant/workspace.
#[async_trait]
pub trait JobStore: Send + Sync {
    /// Inserts a new job in `Queued` status.
    ///
    /// # Errors
    ///
    /// Returns [`DagError::Repository`] if the backend fails.
    async fn enqueue(&self, job: SimulationJob) -> Result<(), DagError>;

    /// Claims the next `Queued` job for a worker, transitioning it to `Running`.
    ///
    /// Returns `None` when no queued job is available. Implementations should
    /// use `FOR UPDATE SKIP LOCKED` or equivalent to avoid double-claiming.
    async fn claim_next(&self, worker_id: &str) -> Option<SimulationJob>;

    /// Updates a running job's progress.
    ///
    /// # Errors
    ///
    /// Returns [`DagError::Repository`] if the job is not in `Running` status
    /// or the backend fails.
    async fn update_progress(&self, id: &JobId, progress: &JobProgress) -> Result<(), DagError>;

    /// Marks a job as `Completed` with the given result.
    ///
    /// # Errors
    ///
    /// Returns [`DagError::Repository`] if the backend fails.
    async fn complete(&self, id: &JobId, result: serde_json::Value) -> Result<(), DagError>;

    /// Marks a job as `Failed` with the given error message.
    ///
    /// # Errors
    ///
    /// Returns [`DagError::Repository`] if the backend fails.
    async fn fail(&self, id: &JobId, error: String) -> Result<(), DagError>;

    /// Cancels a queued or running job, returning true if the state changed.
    ///
    /// # Errors
    ///
    /// Returns [`DagError::Repository`] if the backend fails.
    async fn cancel(&self, id: &JobId) -> Result<bool, DagError>;

    /// Returns a single job scoped to the given tenant and workspace.
    ///
    /// # Errors
    ///
    /// Returns [`DagError::Repository`] if the job is missing or the backend
    /// fails.
    async fn get(
        &self,
        tenant: &TenantId,
        workspace: &WorkspaceId,
        id: &JobId,
    ) -> Result<SimulationJob, DagError>;
}
