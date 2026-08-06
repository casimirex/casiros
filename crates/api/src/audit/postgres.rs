//! `PostgreSQL`-backed audit log.
//!
//! Events are written to the `audit_events` table declared in
//! `migrations/0003_audit_log.sql`. The table has foreign keys to `tenants` and
//! `workspaces`, so a principal's rows must exist before its events can be
//! recorded; see [`PostgresAuditLog::provision_tenant`].

use async_trait::async_trait;
use casiros_core::audit::{AuditAction, AuditEvent, AuditResult, Pagination};
use casiros_core::tenant::{Principal, TenantId, WorkspaceId};
use casiros_dag::DagError;
use casiros_dag::audit::AuditLog;
use sqlx::{PgPool, Row};
use time::OffsetDateTime;
use uuid::Uuid;

/// `PostgreSQL` implementation of [`AuditLog`].
#[derive(Debug, Clone)]
pub struct PostgresAuditLog {
    /// `SQLx` connection pool.
    pool: PgPool,
}

impl PostgresAuditLog {
    /// Creates an audit log backed by an existing `SQLx` [`PgPool`].
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        return Self { pool };
    }

    /// Ensures the tenant and workspace rows referenced by `principal` exist.
    ///
    /// `audit_events`, `snapshots`, and `simulation_jobs` all carry foreign keys
    /// into `tenants` and `workspaces`. A deployment that maps API keys to
    /// tenants via `CASIROS_API_KEY_TENANTS` would otherwise fail every write
    /// for a tenant that was never provisioned, so the server calls this at
    /// startup for each configured principal.
    ///
    /// # Errors
    ///
    /// Returns [`DagError::Repository`] if the backend rejects the write.
    pub async fn provision_tenant(&self, principal: &Principal) -> Result<(), DagError> {
        sqlx::query("INSERT INTO tenants (id, name) VALUES ($1, $1) ON CONFLICT (id) DO NOTHING")
            .bind(principal.tenant_id.as_str())
            .execute(&self.pool)
            .await
            .map_err(|err| DagError::Repository {
                message: format!("provisioning tenant failed: {err}"),
            })?;

        sqlx::query(
            "INSERT INTO workspaces (id, tenant_id, name) VALUES ($1, $2, $1) \
             ON CONFLICT (id) DO NOTHING",
        )
        .bind(principal.workspace_id.as_str())
        .bind(principal.tenant_id.as_str())
        .execute(&self.pool)
        .await
        .map_err(|err| DagError::Repository {
            message: format!("provisioning workspace failed: {err}"),
        })?;

        return Ok(());
    }
}

#[async_trait]
impl AuditLog for PostgresAuditLog {
    async fn record(&self, event: AuditEvent) -> Result<(), DagError> {
        let metadata =
            serde_json::to_value(&event.metadata).map_err(|err| DagError::Repository {
                message: format!("failed to serialize audit metadata: {err}"),
            })?;

        sqlx::query(
            "INSERT INTO audit_events \
             (id, tenant_id, workspace_id, api_key_id, action, resource, result, \
              error_message, metadata, created_at) \
             VALUES ($1, $2, $3, $4, $5::audit_action, $6, $7::audit_result, $8, $9, $10)",
        )
        .bind(event.id)
        .bind(event.principal.tenant_id.as_str())
        .bind(event.principal.workspace_id.as_str())
        .bind(event.principal.api_key_id.as_str())
        .bind(event.action.as_str())
        .bind(event.resource.as_str())
        .bind(event.result.as_str())
        .bind(event.result.error_message())
        .bind(metadata)
        .bind(event.timestamp)
        .execute(&self.pool)
        .await
        .map_err(|err| DagError::Repository {
            message: format!("audit record failed: {err}"),
        })?;

        return Ok(());
    }

    async fn list(
        &self,
        tenant: TenantId,
        pagination: Pagination,
    ) -> Result<Vec<AuditEvent>, DagError> {
        let rows = sqlx::query(
            "SELECT id, tenant_id, workspace_id, api_key_id, action::text AS action, \
                    resource, result::text AS result, error_message, metadata, created_at \
             FROM audit_events \
             WHERE tenant_id = $1 \
             ORDER BY created_at DESC, id DESC \
             LIMIT $2 OFFSET $3",
        )
        .bind(tenant.as_str())
        .bind(i64::from(pagination.limit()))
        .bind(i64::from(pagination.offset()))
        .fetch_all(&self.pool)
        .await
        .map_err(|err| DagError::Repository {
            message: format!("audit list failed: {err}"),
        })?;

        let mut events = Vec::with_capacity(rows.len());
        for row in rows {
            events.push(row_to_event(&row)?);
        }
        return Ok(events);
    }
}

/// Rehydrates an [`AuditEvent`] from a database row.
fn row_to_event(row: &sqlx::postgres::PgRow) -> Result<AuditEvent, DagError> {
    let tenant_id: String = row.try_get("tenant_id").map_err(|err| column_error(&err))?;
    let workspace_id: String = row
        .try_get("workspace_id")
        .map_err(|err| column_error(&err))?;
    let api_key_id: String = row
        .try_get("api_key_id")
        .map_err(|err| column_error(&err))?;
    let action_text: String = row.try_get("action").map_err(|err| column_error(&err))?;
    let result_text: String = row.try_get("result").map_err(|err| column_error(&err))?;
    let error_message: Option<String> = row
        .try_get("error_message")
        .map_err(|err| column_error(&err))?;
    let metadata: serde_json::Value = row.try_get("metadata").map_err(|err| column_error(&err))?;
    let id: Uuid = row.try_get("id").map_err(|err| column_error(&err))?;
    let timestamp: OffsetDateTime = row
        .try_get("created_at")
        .map_err(|err| column_error(&err))?;
    let resource: String = row.try_get("resource").map_err(|err| column_error(&err))?;

    let tenant = TenantId::new(tenant_id).map_err(|err| DagError::Repository {
        message: format!("invalid tenant id in audit row: {err}"),
    })?;
    let workspace = WorkspaceId::new(workspace_id).map_err(|err| DagError::Repository {
        message: format!("invalid workspace id in audit row: {err}"),
    })?;

    // A row whose action we cannot parse is a schema/code mismatch; surface it
    // rather than guessing at what the operator actually did.
    let action = AuditAction::parse(&action_text).ok_or_else(|| DagError::Repository {
        message: format!("unknown audit action '{action_text}'"),
    })?;

    return Ok(AuditEvent {
        id,
        timestamp,
        principal: Principal::new(tenant, workspace, api_key_id),
        action,
        resource,
        result: AuditResult::parse(&result_text, error_message.as_deref()),
        metadata: serde_json::from_value(metadata).unwrap_or_default(),
    });
}

/// Converts a `SQLx` column access failure into a repository error.
fn column_error(err: &sqlx::Error) -> DagError {
    return DagError::Repository {
        message: format!("failed to read audit column: {err}"),
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use casiros_core::audit::AuditResult;

    fn test_db_url() -> String {
        return std::env::var("CASIROS_POSTGRES__URL")
            .unwrap_or_else(|_| "postgresql://casiros:casiros@localhost:5432/casiros".to_string());
    }

    fn principal(tenant: &str) -> Principal {
        return Principal::new(
            TenantId::new(tenant).unwrap(),
            WorkspaceId::new(format!("workspace_{tenant}")).unwrap(),
            "key_audit",
        );
    }

    /// Connects, migrates, and provisions the given tenants.
    ///
    /// Tests share one database and `cargo test` runs them in parallel, so each
    /// test must pass its own tenant identifiers. Reusing a tenant across tests
    /// would let one test's cleanup delete another's rows mid-run.
    async fn create_log(tenants: &[&str]) -> Option<PostgresAuditLog> {
        let pool = match PgPool::connect(&test_db_url()).await {
            Ok(pool) => pool,
            Err(err) => {
                eprintln!("Skipping Postgres audit tests: failed to connect ({err})");
                return None;
            }
        };
        let repo = crate::repositories::PostgresSnapshotRepository::new(pool.clone());
        repo.migrate()
            .await
            .expect("migrations must apply cleanly against a reachable database");

        let log = PostgresAuditLog::new(pool);
        for tenant in tenants {
            log.provision_tenant(&principal(tenant))
                .await
                .expect("provisioning must succeed");
            sqlx::query("DELETE FROM audit_events WHERE tenant_id = $1")
                .bind(*tenant)
                .execute(&log.pool)
                .await
                .expect("cleanup must succeed");
        }
        return Some(log);
    }

    #[tokio::test]
    async fn record_and_list_round_trip() {
        let Some(log) = create_log(&["tenant_audit_roundtrip"]).await else {
            return;
        };
        let event = AuditEvent::new(
            principal("tenant_audit_roundtrip"),
            AuditAction::SnapshotCreate,
            "snap-1",
            AuditResult::Success,
        )
        .with_metadata("method", "POST");
        log.record(event).await.unwrap();

        let listed = log
            .list(
                TenantId::new("tenant_audit_roundtrip").unwrap(),
                Pagination::default(),
            )
            .await
            .unwrap();

        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].action, AuditAction::SnapshotCreate);
    }

    #[tokio::test]
    async fn error_detail_survives_the_round_trip() {
        let Some(log) = create_log(&["tenant_audit_error"]).await else {
            return;
        };
        log.record(AuditEvent::new(
            principal("tenant_audit_error"),
            AuditAction::Evaluate,
            "/evaluate",
            AuditResult::Error("cycle detected".to_string()),
        ))
        .await
        .unwrap();

        let listed = log
            .list(
                TenantId::new("tenant_audit_error").unwrap(),
                Pagination::default(),
            )
            .await
            .unwrap();

        assert_eq!(listed.len(), 1);
        assert_eq!(
            listed[0].result,
            AuditResult::Error("cycle detected".to_string())
        );
    }

    #[tokio::test]
    async fn list_does_not_leak_other_tenants() {
        let Some(log) = create_log(&["tenant_audit_mine", "tenant_audit_theirs"]).await else {
            return;
        };
        log.record(AuditEvent::new(
            principal("tenant_audit_mine"),
            AuditAction::JobRead,
            "mine",
            AuditResult::Success,
        ))
        .await
        .unwrap();
        log.record(AuditEvent::new(
            principal("tenant_audit_theirs"),
            AuditAction::JobRead,
            "theirs",
            AuditResult::Success,
        ))
        .await
        .unwrap();

        let listed = log
            .list(
                TenantId::new("tenant_audit_mine").unwrap(),
                Pagination::default(),
            )
            .await
            .unwrap();

        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].resource, "mine");
    }

    #[tokio::test]
    async fn metadata_round_trips_as_jsonb() {
        let Some(log) = create_log(&["tenant_audit_metadata"]).await else {
            return;
        };
        log.record(
            AuditEvent::new(
                principal("tenant_audit_metadata"),
                AuditAction::Simulate,
                "/simulate",
                AuditResult::Success,
            )
            .with_metadata("status", "200")
            .with_metadata("method", "POST"),
        )
        .await
        .unwrap();

        let listed = log
            .list(
                TenantId::new("tenant_audit_metadata").unwrap(),
                Pagination::default(),
            )
            .await
            .unwrap();

        assert_eq!(
            listed[0].metadata.get("status").map(String::as_str),
            Some("200")
        );
        assert_eq!(listed[0].metadata.len(), 2);
    }
}
