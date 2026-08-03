//! Audit trail backends.
//!
//! Infrastructure-layer implementations of the
//! [`casiros_dag::audit::AuditLog`] trait declared in the Application Layer.
//! [`AuditSink`] type-erases the chosen backend so Actix-Web handlers can accept
//! one concrete type regardless of whether events land in memory or `PostgreSQL`.

use std::sync::Arc;

use async_trait::async_trait;
use casiros_core::audit::{AuditEvent, Pagination};
use casiros_core::tenant::TenantId;
use casiros_dag::DagError;
use casiros_dag::audit::AuditLog;

/// A concrete, object-safe wrapper around any [`AuditLog`].
#[derive(Clone)]
pub struct AuditSink {
    /// The inner audit log, type-erased.
    inner: Arc<dyn AuditLog>,
}

impl AuditSink {
    /// Wraps any [`AuditLog`] implementation.
    #[must_use]
    pub fn new<L: AuditLog + 'static>(log: L) -> Self {
        return Self {
            inner: Arc::new(log),
        };
    }
}

#[async_trait]
impl AuditLog for AuditSink {
    async fn record(&self, event: AuditEvent) -> Result<(), DagError> {
        return self.inner.record(event).await;
    }

    async fn list(
        &self,
        tenant: TenantId,
        pagination: Pagination,
    ) -> Result<Vec<AuditEvent>, DagError> {
        return self.inner.list(tenant, pagination).await;
    }
}

pub mod in_memory;
pub mod postgres;

pub use in_memory::InMemoryAuditLog;
pub use postgres::PostgresAuditLog;
