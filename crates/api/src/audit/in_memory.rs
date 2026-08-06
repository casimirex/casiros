//! In-memory audit log for tests and single-process deployments.

use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use casiros_core::audit::{AuditEvent, Pagination};
use casiros_core::tenant::TenantId;
use casiros_dag::DagError;
use casiros_dag::audit::AuditLog;

/// Audit log that retains events in process memory.
///
/// Events are lost when the process exits, so this backend is intended for
/// tests and local development rather than for deployments that must retain a
/// durable trail.
#[derive(Debug, Clone, Default)]
pub struct InMemoryAuditLog {
    /// Recorded events in insertion order, guarded by a mutex.
    events: Arc<Mutex<Vec<AuditEvent>>>,
}

impl InMemoryAuditLog {
    /// Creates an empty audit log.
    #[must_use]
    pub fn new() -> Self {
        return Self::default();
    }

    /// Returns the number of events recorded so far, across all tenants.
    ///
    /// # Panics
    ///
    /// Panics only if the internal mutex is poisoned.
    #[must_use]
    pub fn len(&self) -> usize {
        return self.events.lock().expect("audit log mutex poisoned").len();
    }

    /// Returns true when no events have been recorded.
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
impl AuditLog for InMemoryAuditLog {
    async fn record(&self, event: AuditEvent) -> Result<(), DagError> {
        let mut events = self.events.lock().map_err(|_| DagError::Repository {
            message: "audit log mutex poisoned".to_string(),
        })?;
        events.push(event);
        return Ok(());
    }

    async fn list(
        &self,
        tenant: TenantId,
        pagination: Pagination,
    ) -> Result<Vec<AuditEvent>, DagError> {
        let events = self.events.lock().map_err(|_| DagError::Repository {
            message: "audit log mutex poisoned".to_string(),
        })?;

        return Ok(events
            .iter()
            .rev()
            .filter(|event| event.principal.tenant_id == tenant)
            .skip(pagination.offset() as usize)
            .take(pagination.limit() as usize)
            .cloned()
            .collect());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use casiros_core::audit::{AuditAction, AuditResult};
    use casiros_core::tenant::{Principal, WorkspaceId};

    fn principal(tenant: &str) -> Principal {
        return Principal::new(
            TenantId::new(tenant).unwrap(),
            WorkspaceId::new("workspace_a").unwrap(),
            "key_a",
        );
    }

    fn event(tenant: &str, resource: &str) -> AuditEvent {
        return AuditEvent::new(
            principal(tenant),
            AuditAction::SnapshotRead,
            resource,
            AuditResult::Success,
        );
    }

    #[tokio::test]
    async fn record_appends_event() {
        let log = InMemoryAuditLog::new();
        assert!(log.is_empty());

        log.record(event("tenant_a", "snap-1")).await.unwrap();
        assert_eq!(log.len(), 1);
    }

    #[tokio::test]
    async fn list_returns_newest_first() {
        let log = InMemoryAuditLog::new();
        log.record(event("tenant_a", "older")).await.unwrap();
        log.record(event("tenant_a", "newer")).await.unwrap();

        let listed = log
            .list(TenantId::new("tenant_a").unwrap(), Pagination::default())
            .await
            .unwrap();

        assert_eq!(listed.len(), 2);
        assert_eq!(listed[0].resource, "newer");
    }

    #[tokio::test]
    async fn list_is_scoped_to_tenant() {
        let log = InMemoryAuditLog::new();
        log.record(event("tenant_a", "mine")).await.unwrap();
        log.record(event("tenant_b", "theirs")).await.unwrap();

        let listed = log
            .list(TenantId::new("tenant_a").unwrap(), Pagination::default())
            .await
            .unwrap();

        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].resource, "mine");
    }

    #[tokio::test]
    async fn list_honours_limit_and_offset() {
        let log = InMemoryAuditLog::new();
        for index in 0..5_u32 {
            log.record(event("tenant_a", &format!("snap-{index}")))
                .await
                .unwrap();
        }

        let page = log
            .list(TenantId::new("tenant_a").unwrap(), Pagination::new(2, 1))
            .await
            .unwrap();

        assert_eq!(page.len(), 2);
        assert_eq!(page[0].resource, "snap-3");
    }
}
