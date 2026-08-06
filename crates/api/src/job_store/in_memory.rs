//! In-memory simulation job store for tests and single-process deployments.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use casiros_core::job::{JobId, JobProgress, JobStatus};
use casiros_core::tenant::{TenantId, WorkspaceId};
use casiros_dag::DagError;
use casiros_dag::job::{JobStore, SimulationJob};
use time::OffsetDateTime;

/// Job store that retains all state in process memory.
///
/// Jobs are lost when the process exits, so this backend is intended for tests
/// and local development rather than for deployments that need durable job
/// tracking.
#[derive(Debug, Clone, Default)]
pub struct InMemoryJobStore {
    /// Jobs keyed by their identifier, guarded by a mutex.
    jobs: Arc<Mutex<HashMap<JobId, SimulationJob>>>,
}

impl InMemoryJobStore {
    /// Creates an empty job store.
    #[must_use]
    pub fn new() -> Self {
        return Self::default();
    }

    /// Returns the number of jobs in the store, across all tenants.
    ///
    /// # Panics
    ///
    /// Panics only if the internal mutex is poisoned.
    #[must_use]
    pub fn len(&self) -> usize {
        return self.jobs.lock().expect("job store mutex poisoned").len();
    }

    /// Returns true when no jobs have been stored.
    ///
    /// # Panics
    ///
    /// Panics only if the internal mutex is poisoned.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        return self.len() == 0;
    }
}

#[async_trait]
impl JobStore for InMemoryJobStore {
    async fn enqueue(&self, job: SimulationJob) -> Result<(), DagError> {
        let mut jobs = self.jobs.lock().map_err(|_| DagError::Repository {
            message: "job store mutex poisoned".to_string(),
        })?;
        jobs.insert(job.id, job);
        return Ok(());
    }

    async fn claim_next(&self, _worker_id: &str) -> Option<SimulationJob> {
        let mut jobs = self.jobs.lock().ok()?;
        for job in jobs.values_mut() {
            if job.status == JobStatus::Queued {
                job.status = JobStatus::Running;
                job.updated_at = OffsetDateTime::now_utc();
                return Some(job.clone());
            }
        }
        return None;
    }

    async fn update_progress(&self, id: &JobId, progress: &JobProgress) -> Result<(), DagError> {
        let mut jobs = self.jobs.lock().map_err(|_| DagError::Repository {
            message: "job store mutex poisoned".to_string(),
        })?;
        let Some(job) = jobs.get_mut(id) else {
            return Err(DagError::Repository {
                message: format!("job '{id}' not found"),
            });
        };
        if job.status != JobStatus::Running {
            return Err(DagError::Repository {
                message: format!(
                    "cannot update progress for job '{id}' in status '{}'",
                    job.status
                ),
            });
        }
        job.progress = *progress;
        job.updated_at = OffsetDateTime::now_utc();
        return Ok(());
    }

    async fn complete(&self, id: &JobId, result: serde_json::Value) -> Result<(), DagError> {
        let mut jobs = self.jobs.lock().map_err(|_| DagError::Repository {
            message: "job store mutex poisoned".to_string(),
        })?;
        let Some(job) = jobs.get_mut(id) else {
            return Err(DagError::Repository {
                message: format!("job '{id}' not found"),
            });
        };
        job.status = JobStatus::Completed;
        job.result = Some(result);
        job.updated_at = OffsetDateTime::now_utc();
        return Ok(());
    }

    async fn fail(&self, id: &JobId, error: String) -> Result<(), DagError> {
        let mut jobs = self.jobs.lock().map_err(|_| DagError::Repository {
            message: "job store mutex poisoned".to_string(),
        })?;
        let Some(job) = jobs.get_mut(id) else {
            return Err(DagError::Repository {
                message: format!("job '{id}' not found"),
            });
        };
        job.status = JobStatus::Failed;
        job.error = Some(error);
        job.updated_at = OffsetDateTime::now_utc();
        return Ok(());
    }

    async fn cancel(&self, id: &JobId) -> Result<bool, DagError> {
        let mut jobs = self.jobs.lock().map_err(|_| DagError::Repository {
            message: "job store mutex poisoned".to_string(),
        })?;
        let Some(job) = jobs.get_mut(id) else {
            return Ok(false);
        };
        if !job.status.is_cancellable() {
            return Ok(false);
        }
        job.status = JobStatus::Cancelled;
        job.updated_at = OffsetDateTime::now_utc();
        return Ok(true);
    }

    async fn get(
        &self,
        tenant: &TenantId,
        workspace: &WorkspaceId,
        id: &JobId,
    ) -> Result<SimulationJob, DagError> {
        let jobs = self.jobs.lock().map_err(|_| DagError::Repository {
            message: "job store mutex poisoned".to_string(),
        })?;
        let Some(job) = jobs.get(id) else {
            return Err(DagError::Repository {
                message: format!("job '{id}' not found"),
            });
        };
        if job.tenant_id != *tenant || job.workspace_id != *workspace {
            return Err(DagError::Repository {
                message: format!("job '{id}' not found"),
            });
        }
        return Ok(job.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use casiros_core::job::JobProgress;

    fn sample_job(tenant: &str, workspace: &str) -> SimulationJob {
        SimulationJob {
            id: JobId::new(),
            tenant_id: TenantId::new(tenant).unwrap(),
            workspace_id: WorkspaceId::new(workspace).unwrap(),
            status: JobStatus::Queued,
            request: serde_json::json!({"nodes": []}),
            progress: JobProgress::new(100),
            result: None,
            error: None,
            created_at: OffsetDateTime::now_utc(),
            updated_at: OffsetDateTime::now_utc(),
        }
    }

    #[tokio::test]
    async fn enqueue_and_get_round_trip() {
        let store = InMemoryJobStore::new();
        let job = sample_job("tenant_a", "workspace_a");
        store.enqueue(job.clone()).await.unwrap();

        let loaded = store
            .get(&job.tenant_id, &job.workspace_id, &job.id)
            .await
            .unwrap();
        assert_eq!(loaded.id, job.id);
        assert_eq!(loaded.status, JobStatus::Queued);
    }

    #[tokio::test]
    async fn claim_next_transitions_to_running() {
        let store = InMemoryJobStore::new();
        let job = sample_job("tenant_a", "workspace_a");
        store.enqueue(job.clone()).await.unwrap();

        let claimed = store.claim_next("worker-1").await.unwrap();
        assert_eq!(claimed.id, job.id);
        assert_eq!(claimed.status, JobStatus::Running);
    }

    #[tokio::test]
    async fn claim_next_returns_none_when_empty() {
        let store = InMemoryJobStore::new();
        assert!(store.claim_next("worker-1").await.is_none());
    }

    #[tokio::test]
    async fn complete_lifecycle() {
        let store = InMemoryJobStore::new();
        let job = sample_job("tenant_a", "workspace_a");
        store.enqueue(job.clone()).await.unwrap();
        store.claim_next("worker-1").await.unwrap();

        let result = serde_json::json!({"mean": 0.5});
        store.complete(&job.id, result.clone()).await.unwrap();

        let loaded = store
            .get(&job.tenant_id, &job.workspace_id, &job.id)
            .await
            .unwrap();
        assert_eq!(loaded.status, JobStatus::Completed);
        assert_eq!(loaded.result, Some(result));
    }

    #[tokio::test]
    async fn fail_lifecycle() {
        let store = InMemoryJobStore::new();
        let job = sample_job("tenant_a", "workspace_a");
        store.enqueue(job.clone()).await.unwrap();
        store.claim_next("worker-1").await.unwrap();

        store
            .fail(&job.id, "out of memory".to_string())
            .await
            .unwrap();

        let loaded = store
            .get(&job.tenant_id, &job.workspace_id, &job.id)
            .await
            .unwrap();
        assert_eq!(loaded.status, JobStatus::Failed);
        assert_eq!(loaded.error, Some("out of memory".to_string()));
    }

    #[tokio::test]
    async fn cancel_queued_job() {
        let store = InMemoryJobStore::new();
        let job = sample_job("tenant_a", "workspace_a");
        store.enqueue(job.clone()).await.unwrap();

        assert!(store.cancel(&job.id).await.unwrap());

        let loaded = store
            .get(&job.tenant_id, &job.workspace_id, &job.id)
            .await
            .unwrap();
        assert_eq!(loaded.status, JobStatus::Cancelled);
    }

    #[tokio::test]
    async fn cancel_completed_job_returns_false() {
        let store = InMemoryJobStore::new();
        let job = sample_job("tenant_a", "workspace_a");
        store.enqueue(job.clone()).await.unwrap();
        store.claim_next("worker-1").await.unwrap();
        store
            .complete(&job.id, serde_json::json!({}))
            .await
            .unwrap();

        assert!(!store.cancel(&job.id).await.unwrap());
    }

    #[tokio::test]
    async fn get_rejects_cross_tenant_access() {
        let store = InMemoryJobStore::new();
        let job = sample_job("tenant_a", "workspace_a");
        store.enqueue(job.clone()).await.unwrap();

        let other_tenant = TenantId::new("tenant_b").unwrap();
        let result = store.get(&other_tenant, &job.workspace_id, &job.id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn update_progress_requires_running_status() {
        let store = InMemoryJobStore::new();
        let job = sample_job("tenant_a", "workspace_a");
        store.enqueue(job.clone()).await.unwrap();

        let progress = JobProgress::new(100);
        let result = store.update_progress(&job.id, &progress).await;
        assert!(result.is_err());
    }
}
