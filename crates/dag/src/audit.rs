//! Storage-agnostic audit trail persistence.
//!
//! Like [`crate::repository::SnapshotRepository`], the [`AuditLog`] trait is
//! declared in the Application Layer so infrastructure implementations
//! (`PostgreSQL`, in-memory) can be swapped without changing callers. The trail
//! is append-only: there is deliberately no update or delete operation.

use async_trait::async_trait;
use casiros_core::audit::{AuditEvent, Pagination};
use casiros_core::tenant::TenantId;

use crate::error::DagError;

/// Append-only sink for [`AuditEvent`] records.
///
/// Implementations must preserve events exactly as recorded and must scope
/// [`AuditLog::list`] to the requested tenant; leaking another tenant's trail is
/// a security defect, not merely a bug.
#[async_trait]
pub trait AuditLog: Send + Sync {
    /// Appends an event to the trail.
    ///
    /// # Errors
    ///
    /// Returns [`DagError::Repository`] if the backend rejects the write.
    async fn record(&self, event: AuditEvent) -> Result<(), DagError>;

    /// Lists a tenant's events, newest first.
    ///
    /// # Errors
    ///
    /// Returns [`DagError::Repository`] if the backend fails.
    async fn list(
        &self,
        tenant: TenantId,
        pagination: Pagination,
    ) -> Result<Vec<AuditEvent>, DagError>;
}
