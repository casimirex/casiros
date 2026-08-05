//! `PostgreSQL`-backed simulation job store.
//!
//! Jobs are stored in the `simulation_jobs` table declared in
//! `migrations/0004_simulation_jobs.sql`. The table has foreign keys to `tenants`
//! and `workspaces`, so a principal's rows must exist before its jobs can be
//! created; see [`crate::audit::PostgresAuditLog::provision_tenant`].

use async_trait::async_trait;
use casiros_core::job::{JobId, JobProgress, JobStatus};
use casiros_core::tenant::{TenantId, WorkspaceId};
use casiros_dag::DagError;
use casiros_dag::job::{JobStore, SimulationJob};
use sqlx::{PgPool, Row};
use time::OffsetDateTime;

/// `PostgreSQL` implementation of [`JobStore`].
#[derive(Debug, Clone)]
pub struct PostgresJobStore {
    /// `SQLx` connection pool.
    pool: PgPool,
}

impl PostgresJobStore {
    /// Creates a job store backed by an existing `SQLx` [`PgPool`].
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        return Self { pool };
    }
}

#[async_trait]
impl JobStore for PostgresJobStore {
    async fn enqueue(&self, job: SimulationJob) -> Result<(), DagError> {
        let progress = serde_json::json!({
            "universes_total": job.progress.universes_total,
            "universes_completed": job.progress.universes_completed,
            "last_checkpoint_at": job.progress.last_checkpoint_at,
        });

        sqlx::query(
            "INSERT INTO simulation_jobs \
             (id, tenant_id, workspace_id, status, request, progress) \
             VALUES ($1, $2, $3, 'queued'::job_status, $4, $5)",
        )
        .bind(job.id.as_uuid())
        .bind(job.tenant_id.as_str())
        .bind(job.workspace_id.as_str())
        .bind(job.request)
        .bind(progress)
        .execute(&self.pool)
        .await
        .map_err(|err| DagError::Repository {
            message: format!("job enqueue failed: {err}"),
        })?;

        return Ok(());
    }

    async fn claim_next(&self, worker_id: &str) -> Option<SimulationJob> {
        let result: Result<sqlx::postgres::PgQueryResult, sqlx::Error> = sqlx::query(
            "UPDATE simulation_jobs \
             SET status = 'running'::job_status, \
                 claimed_by = $1, \
                 claimed_until = now() + interval '5 minutes' \
             WHERE id = ( \
                 SELECT id FROM simulation_jobs \
                 WHERE status = 'queued'::job_status \
                 ORDER BY created_at ASC \
                 LIMIT 1 \
                 FOR UPDATE SKIP LOCKED \
             )",
        )
        .bind(worker_id)
        .execute(&self.pool)
        .await;

        let Ok(result) = result else {
            return None;
        };
        if result.rows_affected() == 0 {
            return None;
        }

        // Fetch the updated row to return it.
        let row = sqlx::query(
            "SELECT id, tenant_id, workspace_id, status::text AS status, \
                    request, progress, result_snapshot_id, error_message, \
                    created_at, updated_at \
             FROM simulation_jobs \
             WHERE claimed_by = $1 AND status = 'running'::job_status \
             ORDER BY updated_at DESC LIMIT 1",
        )
        .bind(worker_id)
        .fetch_one(&self.pool)
        .await
        .ok()?;

        return row_to_job(&row).ok();
    }

    async fn update_progress(&self, id: &JobId, progress: &JobProgress) -> Result<(), DagError> {
        let progress_json = serde_json::json!({
            "universes_total": progress.universes_total,
            "universes_completed": progress.universes_completed,
            "last_checkpoint_at": progress.last_checkpoint_at,
        });

        let result = sqlx::query(
            "UPDATE simulation_jobs \
             SET progress = $1 \
             WHERE id = $2 AND status = 'running'::job_status",
        )
        .bind(progress_json)
        .bind(id.as_uuid())
        .execute(&self.pool)
        .await
        .map_err(|err| DagError::Repository {
            message: format!("job progress update failed: {err}"),
        })?;

        if result.rows_affected() == 0 {
            return Err(DagError::Repository {
                message: format!("job '{id}' not found or not running"),
            });
        }

        return Ok(());
    }

    async fn complete(&self, id: &JobId, _result: serde_json::Value) -> Result<(), DagError> {
        let result = sqlx::query(
            "UPDATE simulation_jobs \
             SET status = 'completed'::job_status, \
                 progress = jsonb_set(progress, '{universes_completed}', \
                     to_jsonb((progress->>'universes_total')::int)) \
             WHERE id = $1 AND status = 'running'::job_status",
        )
        .bind(id.as_uuid())
        .execute(&self.pool)
        .await
        .map_err(|err| DagError::Repository {
            message: format!("job completion failed: {err}"),
        })?;

        if result.rows_affected() == 0 {
            return Err(DagError::Repository {
                message: format!("job '{id}' not found or not running"),
            });
        }

        return Ok(());
    }

    async fn fail(&self, id: &JobId, error: String) -> Result<(), DagError> {
        let result = sqlx::query(
            "UPDATE simulation_jobs \
             SET status = 'failed'::job_status, error_message = $1 \
             WHERE id = $2 AND status = 'running'::job_status",
        )
        .bind(&error)
        .bind(id.as_uuid())
        .execute(&self.pool)
        .await
        .map_err(|err| DagError::Repository {
            message: format!("job fail failed: {err}"),
        })?;

        if result.rows_affected() == 0 {
            return Err(DagError::Repository {
                message: format!("job '{id}' not found or not running"),
            });
        }

        return Ok(());
    }

    async fn cancel(&self, id: &JobId) -> Result<bool, DagError> {
        let result = sqlx::query(
            "UPDATE simulation_jobs \
             SET status = 'cancelled'::job_status \
             WHERE id = $1 AND status IN ('queued'::job_status, 'running'::job_status)",
        )
        .bind(id.as_uuid())
        .execute(&self.pool)
        .await
        .map_err(|err| DagError::Repository {
            message: format!("job cancel failed: {err}"),
        })?;

        return Ok(result.rows_affected() > 0);
    }

    async fn get(
        &self,
        tenant: &TenantId,
        workspace: &WorkspaceId,
        id: &JobId,
    ) -> Result<SimulationJob, DagError> {
        let row = sqlx::query(
            "SELECT id, tenant_id, workspace_id, status::text AS status, \
                    request, progress, result_snapshot_id, error_message, \
                    created_at, updated_at \
             FROM simulation_jobs \
             WHERE id = $1 AND tenant_id = $2 AND workspace_id = $3",
        )
        .bind(id.as_uuid())
        .bind(tenant.as_str())
        .bind(workspace.as_str())
        .fetch_one(&self.pool)
        .await
        .map_err(|err| DagError::Repository {
            message: format!("job get failed: {err}"),
        })?;

        return row_to_job(&row);
    }
}

/// Rehydrates a [`SimulationJob`] from a database row.
fn row_to_job(row: &sqlx::postgres::PgRow) -> Result<SimulationJob, DagError> {
    let id: uuid::Uuid = row.try_get("id").map_err(|err| DagError::Repository {
        message: format!("failed to read job id: {err}"),
    })?;
    let tenant_id: String = row
        .try_get("tenant_id")
        .map_err(|err| DagError::Repository {
            message: format!("failed to read tenant_id: {err}"),
        })?;
    let workspace_id: String = row
        .try_get("workspace_id")
        .map_err(|err| DagError::Repository {
            message: format!("failed to read workspace_id: {err}"),
        })?;
    let status_text: String = row.try_get("status").map_err(|err| DagError::Repository {
        message: format!("failed to read status: {err}"),
    })?;
    let request: serde_json::Value =
        row.try_get("request").map_err(|err| DagError::Repository {
            message: format!("failed to read request: {err}"),
        })?;
    let progress_json: serde_json::Value =
        row.try_get("progress")
            .map_err(|err| DagError::Repository {
                message: format!("failed to read progress: {err}"),
            })?;
    let error_message: Option<String> =
        row.try_get("error_message")
            .map_err(|err| DagError::Repository {
                message: format!("failed to read error_message: {err}"),
            })?;
    let created_at: OffsetDateTime =
        row.try_get("created_at")
            .map_err(|err| DagError::Repository {
                message: format!("failed to read created_at: {err}"),
            })?;
    let updated_at: OffsetDateTime =
        row.try_get("updated_at")
            .map_err(|err| DagError::Repository {
                message: format!("failed to read updated_at: {err}"),
            })?;

    let tenant = TenantId::new(tenant_id).map_err(|err| DagError::Repository {
        message: format!("invalid tenant id in job row: {err}"),
    })?;
    let workspace = WorkspaceId::new(workspace_id).map_err(|err| DagError::Repository {
        message: format!("invalid workspace id in job row: {err}"),
    })?;

    let status = JobStatus::parse(&status_text).ok_or_else(|| DagError::Repository {
        message: format!("unknown job status '{status_text}'"),
    })?;

    #[allow(clippy::cast_possible_truncation)]
    let progress = JobProgress {
        universes_total: progress_json
            .get("universes_total")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0) as usize,
        universes_completed: progress_json
            .get("universes_completed")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0) as usize,
        last_checkpoint_at: None,
    };

    return Ok(SimulationJob {
        id: JobId::from_uuid(id),
        tenant_id: tenant,
        workspace_id: workspace,
        status,
        request,
        progress,
        result: None,
        error: error_message,
        created_at,
        updated_at,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use casiros_core::job::JobProgress;

    fn test_db_url() -> String {
        return std::env::var("CASIROS_POSTGRES__URL")
            .unwrap_or_else(|_| "postgresql://casiros:casiros@localhost:5432/casiros".to_string());
    }

    fn sample_job_for(tenant: &TenantId, workspace: &WorkspaceId) -> SimulationJob {
        SimulationJob {
            id: JobId::new(),
            tenant_id: tenant.clone(),
            workspace_id: workspace.clone(),
            status: JobStatus::Queued,
            request: serde_json::json!({"nodes": [], "target": "fv", "universe_count": 100}),
            progress: JobProgress::new(100),
            result: None,
            error: None,
            created_at: OffsetDateTime::now_utc(),
            updated_at: OffsetDateTime::now_utc(),
        }
    }

    async fn create_store_for(
        tenant: &TenantId,
        workspace: &WorkspaceId,
    ) -> Option<PostgresJobStore> {
        let pool = match PgPool::connect(&test_db_url()).await {
            Ok(pool) => pool,
            Err(err) => {
                eprintln!("Skipping Postgres job tests: failed to connect ({err})");
                return None;
            }
        };
        let repo = crate::repositories::PostgresSnapshotRepository::new(pool.clone());
        repo.migrate()
            .await
            .expect("migrations must apply cleanly against a reachable database");

        sqlx::query("INSERT INTO tenants (id, name) VALUES ($1, $1) ON CONFLICT DO NOTHING")
            .bind(tenant.as_str())
            .execute(&pool)
            .await
            .expect("seeding tenant must succeed");
        sqlx::query(
            "INSERT INTO workspaces (id, tenant_id, name) VALUES ($1, $2, $1)              ON CONFLICT DO NOTHING",
        )
        .bind(workspace.as_str())
        .bind(tenant.as_str())
        .execute(&pool)
        .await
        .expect("seeding workspace must succeed");

        // Clear all simulation jobs so claim_next doesn't pick up stale
        // jobs left by a previous test that ran in the same database.
        sqlx::query("DELETE FROM simulation_jobs")
            .execute(&pool)
            .await
            .expect("cleanup must succeed");

        return Some(PostgresJobStore::new(pool));
    }

    #[tokio::test]
    async fn enqueue_and_get_round_trip() {
        let tenant = TenantId::new("tenant_job_roundtrip").unwrap();
        let workspace = WorkspaceId::new("workspace_job_roundtrip").unwrap();
        let Some(store) = create_store_for(&tenant, &workspace).await else {
            return;
        };
        let job = sample_job_for(&tenant, &workspace);
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
        let tenant = TenantId::new("tenant_job_claim").unwrap();
        let workspace = WorkspaceId::new("workspace_job_claim").unwrap();
        let Some(store) = create_store_for(&tenant, &workspace).await else {
            return;
        };
        let job = sample_job_for(&tenant, &workspace);
        store.enqueue(job.clone()).await.unwrap();

        let claimed = store.claim_next("worker-1").await.unwrap();
        assert_eq!(claimed.id, job.id);
        assert_eq!(claimed.status, JobStatus::Running);
    }

    #[tokio::test]
    async fn complete_lifecycle() {
        let tenant = TenantId::new("tenant_job_complete").unwrap();
        let workspace = WorkspaceId::new("workspace_job_complete").unwrap();
        let Some(store) = create_store_for(&tenant, &workspace).await else {
            return;
        };
        let job = sample_job_for(&tenant, &workspace);
        store.enqueue(job.clone()).await.unwrap();
        store.claim_next("worker-1").await.unwrap();

        store
            .complete(&job.id, serde_json::json!({"mean": 0.5}))
            .await
            .unwrap();

        let loaded = store
            .get(&job.tenant_id, &job.workspace_id, &job.id)
            .await
            .unwrap();
        assert_eq!(loaded.status, JobStatus::Completed);
    }

    #[tokio::test]
    async fn fail_lifecycle() {
        let tenant = TenantId::new("tenant_job_fail").unwrap();
        let workspace = WorkspaceId::new("workspace_job_fail").unwrap();
        let Some(store) = create_store_for(&tenant, &workspace).await else {
            return;
        };
        let job = sample_job_for(&tenant, &workspace);
        store.enqueue(job.clone()).await.unwrap();
        store.claim_next("worker-1").await.unwrap();

        store
            .fail(&job.id, "simulation error".to_string())
            .await
            .unwrap();

        let loaded = store
            .get(&job.tenant_id, &job.workspace_id, &job.id)
            .await
            .unwrap();
        assert_eq!(loaded.status, JobStatus::Failed);
        assert_eq!(loaded.error, Some("simulation error".to_string()));
    }

    #[tokio::test]
    async fn cancel_queued_job() {
        let tenant = TenantId::new("tenant_job_cancel_q").unwrap();
        let workspace = WorkspaceId::new("workspace_job_cancel_q").unwrap();
        let Some(store) = create_store_for(&tenant, &workspace).await else {
            return;
        };
        let job = sample_job_for(&tenant, &workspace);
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
        let tenant = TenantId::new("tenant_job_cancel_c").unwrap();
        let workspace = WorkspaceId::new("workspace_job_cancel_c").unwrap();
        let Some(store) = create_store_for(&tenant, &workspace).await else {
            return;
        };
        let job = sample_job_for(&tenant, &workspace);
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
        let tenant = TenantId::new("tenant_job_isolation").unwrap();
        let workspace = WorkspaceId::new("workspace_job_isolation").unwrap();
        let Some(store) = create_store_for(&tenant, &workspace).await else {
            return;
        };
        let job = sample_job_for(&tenant, &workspace);
        store.enqueue(job.clone()).await.unwrap();

        let other_tenant = TenantId::new("tenant_other").unwrap();
        let other_workspace = WorkspaceId::new("workspace_other").unwrap();
        // Provision the other tenant so the FK constraint is satisfied.
        sqlx::query("INSERT INTO tenants (id, name) VALUES ($1, $1) ON CONFLICT DO NOTHING")
            .bind(other_tenant.as_str())
            .execute(&store.pool)
            .await
            .expect("seeding other tenant must succeed");
        sqlx::query(
            "INSERT INTO workspaces (id, tenant_id, name) VALUES ($1, $2, $1) \
             ON CONFLICT DO NOTHING",
        )
        .bind(other_workspace.as_str())
        .bind(other_tenant.as_str())
        .execute(&store.pool)
        .await
        .expect("seeding other workspace must succeed");
        let result = store.get(&other_tenant, &other_workspace, &job.id).await;
        assert!(result.is_err());
    }
}
