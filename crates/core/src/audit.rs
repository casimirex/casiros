//! Immutable audit event types.
//!
//! An [`AuditEvent`] records who did what to which resource, and how it turned
//! out. These are Domain Layer value objects: constructing one performs no I/O
//! and never fails. Persistence is the responsibility of an `AuditLog`
//! implementation in the Infrastructure Layer.
//!
//! The string forms produced by [`AuditAction::as_str`] and
//! [`AuditResult::as_str`] are the contract shared with the `audit_action` and
//! `audit_result` `PostgreSQL` enums declared in `migrations/0003_audit_log.sql`.
//! Changing one without the other is a schema break.

use std::collections::HashMap;
use std::fmt;

use time::OffsetDateTime;
use uuid::Uuid;

use crate::tenant::Principal;

/// The kind of operation a principal attempted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AuditAction {
    /// A single graph evaluation.
    Evaluate,

    /// A Monte Carlo simulation.
    Simulate,

    /// Creation of a snapshot.
    SnapshotCreate,

    /// Retrieval of a snapshot.
    SnapshotRead,

    /// Deletion of a snapshot.
    SnapshotDelete,

    /// Enqueueing of an asynchronous simulation job.
    JobCreate,

    /// Retrieval of job status or results.
    JobRead,

    /// Cancellation of a queued or running job.
    JobCancel,
}

impl AuditAction {
    /// Returns the wire/database representation of this action.
    ///
    /// # Examples
    ///
    /// ```
    /// use casiros_core::audit::AuditAction;
    ///
    /// assert_eq!(AuditAction::SnapshotCreate.as_str(), "snapshot_create");
    /// assert_eq!(AuditAction::Evaluate.as_str(), "evaluate");
    /// ```
    #[must_use]
    pub fn as_str(self) -> &'static str {
        return match self {
            Self::Evaluate => "evaluate",
            Self::Simulate => "simulate",
            Self::SnapshotCreate => "snapshot_create",
            Self::SnapshotRead => "snapshot_read",
            Self::SnapshotDelete => "snapshot_delete",
            Self::JobCreate => "job_create",
            Self::JobRead => "job_read",
            Self::JobCancel => "job_cancel",
        };
    }

    /// Parses an action from its database representation.
    ///
    /// # Examples
    ///
    /// ```
    /// use casiros_core::audit::AuditAction;
    ///
    /// assert_eq!(AuditAction::parse("job_cancel"), Some(AuditAction::JobCancel));
    /// assert_eq!(AuditAction::parse("nonsense"), None);
    /// ```
    #[must_use]
    pub fn parse(value: &str) -> Option<Self> {
        return match value {
            "evaluate" => Some(Self::Evaluate),
            "simulate" => Some(Self::Simulate),
            "snapshot_create" => Some(Self::SnapshotCreate),
            "snapshot_read" => Some(Self::SnapshotRead),
            "snapshot_delete" => Some(Self::SnapshotDelete),
            "job_create" => Some(Self::JobCreate),
            "job_read" => Some(Self::JobRead),
            "job_cancel" => Some(Self::JobCancel),
            _ => None,
        };
    }
}

impl fmt::Display for AuditAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        return f.write_str(self.as_str());
    }
}

/// The outcome of an audited operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuditResult {
    /// The operation completed successfully.
    Success,

    /// The principal was not permitted to perform the operation.
    Forbidden,

    /// The addressed resource does not exist within the principal's scope.
    NotFound,

    /// The operation failed; the payload carries a human-readable reason.
    Error(String),
}

impl AuditResult {
    /// Returns the database enum representation, discarding any error detail.
    ///
    /// Detail is carried separately by [`AuditResult::error_message`] because
    /// the `audit_result` `PostgreSQL` enum has no payload.
    ///
    /// # Examples
    ///
    /// ```
    /// use casiros_core::audit::AuditResult;
    ///
    /// assert_eq!(AuditResult::NotFound.as_str(), "not_found");
    /// assert_eq!(AuditResult::Error("boom".into()).as_str(), "error");
    /// ```
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        return match *self {
            Self::Success => "success",
            Self::Forbidden => "forbidden",
            Self::NotFound => "not_found",
            Self::Error(_) => "error",
        };
    }

    /// Returns the failure detail, if this result carries one.
    ///
    /// # Examples
    ///
    /// ```
    /// use casiros_core::audit::AuditResult;
    ///
    /// assert_eq!(AuditResult::Error("boom".into()).error_message(), Some("boom"));
    /// assert_eq!(AuditResult::Success.error_message(), None);
    /// ```
    #[must_use]
    pub fn error_message(&self) -> Option<&str> {
        return match *self {
            Self::Error(ref message) => Some(message.as_str()),
            _ => None,
        };
    }

    /// Reconstructs a result from its database representation and detail column.
    ///
    /// Unrecognised values are treated as errors rather than silently mapped to
    /// success, so a corrupt row can never read back as a clean outcome.
    ///
    /// # Examples
    ///
    /// ```
    /// use casiros_core::audit::AuditResult;
    ///
    /// assert_eq!(AuditResult::parse("success", None), AuditResult::Success);
    /// assert_eq!(
    ///     AuditResult::parse("error", Some("boom")),
    ///     AuditResult::Error("boom".to_string())
    /// );
    /// ```
    #[must_use]
    pub fn parse(value: &str, error_message: Option<&str>) -> Self {
        return match value {
            "success" => Self::Success,
            "forbidden" => Self::Forbidden,
            "not_found" => Self::NotFound,
            _ => Self::Error(error_message.unwrap_or("unknown error").to_string()),
        };
    }

    /// Returns true when the operation succeeded.
    ///
    /// # Examples
    ///
    /// ```
    /// use casiros_core::audit::AuditResult;
    ///
    /// assert!(AuditResult::Success.is_success());
    /// assert!(!AuditResult::Forbidden.is_success());
    /// ```
    #[must_use]
    pub fn is_success(&self) -> bool {
        return matches!(*self, Self::Success);
    }
}

impl fmt::Display for AuditResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        return f.write_str(self.as_str());
    }
}

/// A single immutable entry in the audit trail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditEvent {
    /// Unique identifier for this event.
    pub id: Uuid,

    /// When the event was recorded.
    pub timestamp: OffsetDateTime,

    /// Who performed the operation.
    pub principal: Principal,

    /// What was attempted.
    pub action: AuditAction,

    /// The resource the action addressed (e.g. a snapshot id or route).
    pub resource: String,

    /// How the attempt turned out.
    pub result: AuditResult,

    /// Free-form contextual detail (HTTP method, status code, and similar).
    pub metadata: HashMap<String, String>,
}

impl AuditEvent {
    /// Creates an event stamped with a fresh identifier and the current time.
    ///
    /// # Examples
    ///
    /// ```
    /// use casiros_core::audit::{AuditAction, AuditEvent, AuditResult};
    /// use casiros_core::tenant::{Principal, TenantId, WorkspaceId};
    ///
    /// let principal = Principal::new(
    ///     TenantId::new("tenant_a").unwrap(),
    ///     WorkspaceId::new("workspace_a").unwrap(),
    ///     "key_a",
    /// );
    /// let event = AuditEvent::new(
    ///     principal,
    ///     AuditAction::SnapshotRead,
    ///     "snapshot-1",
    ///     AuditResult::Success,
    /// );
    ///
    /// assert_eq!(event.action, AuditAction::SnapshotRead);
    /// assert!(event.metadata.is_empty());
    /// ```
    #[must_use]
    pub fn new(
        principal: Principal,
        action: AuditAction,
        resource: impl Into<String>,
        result: AuditResult,
    ) -> Self {
        return Self {
            id: Uuid::new_v4(),
            timestamp: OffsetDateTime::now_utc(),
            principal,
            action,
            resource: resource.into(),
            result,
            metadata: HashMap::new(),
        };
    }

    /// Attaches a metadata key/value pair, replacing any previous value.
    ///
    /// # Examples
    ///
    /// ```
    /// use casiros_core::audit::{AuditAction, AuditEvent, AuditResult};
    /// use casiros_core::tenant::{Principal, TenantId, WorkspaceId};
    ///
    /// let principal = Principal::new(
    ///     TenantId::new("t").unwrap(),
    ///     WorkspaceId::new("w").unwrap(),
    ///     "k",
    /// );
    /// let event = AuditEvent::new(principal, AuditAction::Evaluate, "/evaluate", AuditResult::Success)
    ///     .with_metadata("status", "200");
    ///
    /// assert_eq!(event.metadata.get("status").map(String::as_str), Some("200"));
    /// assert_eq!(event.metadata.len(), 1);
    /// ```
    #[must_use]
    pub fn with_metadata(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        return self;
    }
}

/// Pagination window for audit queries.
///
/// Both fields are clamped on construction so that a caller can never request an
/// unbounded result set, satisfying the fixed-upper-bound rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pagination {
    /// Maximum number of rows to return.
    limit: u32,

    /// Number of rows to skip.
    offset: u32,
}

/// Largest page size any caller may request.
pub const MAX_AUDIT_PAGE_SIZE: u32 = 1_000;

impl Pagination {
    /// Creates a pagination window, clamping `limit` to `1..=MAX_AUDIT_PAGE_SIZE`.
    ///
    /// # Examples
    ///
    /// ```
    /// use casiros_core::audit::{MAX_AUDIT_PAGE_SIZE, Pagination};
    ///
    /// let page = Pagination::new(50, 100);
    /// assert_eq!(page.limit(), 50);
    ///
    /// let clamped = Pagination::new(u32::MAX, 0);
    /// assert_eq!(clamped.limit(), MAX_AUDIT_PAGE_SIZE);
    /// ```
    #[must_use]
    pub fn new(limit: u32, offset: u32) -> Self {
        return Self {
            limit: limit.clamp(1, MAX_AUDIT_PAGE_SIZE),
            offset,
        };
    }

    /// Returns the clamped page size.
    #[must_use]
    pub fn limit(self) -> u32 {
        return self.limit;
    }

    /// Returns the row offset.
    #[must_use]
    pub fn offset(self) -> u32 {
        return self.offset;
    }
}

impl Default for Pagination {
    fn default() -> Self {
        return Self::new(100, 0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tenant::{TenantId, WorkspaceId};

    fn principal() -> Principal {
        return Principal::new(
            TenantId::new("tenant_a").unwrap(),
            WorkspaceId::new("workspace_a").unwrap(),
            "key_a",
        );
    }

    #[test]
    fn action_round_trips_through_string() {
        for action in [
            AuditAction::Evaluate,
            AuditAction::Simulate,
            AuditAction::SnapshotCreate,
            AuditAction::SnapshotRead,
            AuditAction::SnapshotDelete,
            AuditAction::JobCreate,
            AuditAction::JobRead,
            AuditAction::JobCancel,
        ] {
            assert_eq!(AuditAction::parse(action.as_str()), Some(action));
            assert!(!action.as_str().is_empty());
        }
    }

    #[test]
    fn action_rejects_unknown_value() {
        assert_eq!(AuditAction::parse(""), None);
        assert_eq!(AuditAction::parse("drop_tables"), None);
    }

    #[test]
    fn result_splits_detail_from_discriminant() {
        let failure = AuditResult::Error("disk on fire".to_string());
        assert_eq!(failure.as_str(), "error");
        assert_eq!(failure.error_message(), Some("disk on fire"));
    }

    #[test]
    fn result_round_trips_through_columns() {
        for result in [
            AuditResult::Success,
            AuditResult::Forbidden,
            AuditResult::NotFound,
            AuditResult::Error("boom".to_string()),
        ] {
            let parsed = AuditResult::parse(result.as_str(), result.error_message());
            assert_eq!(parsed, result);
            assert!(!result.as_str().is_empty());
        }
    }

    #[test]
    fn unknown_result_never_reads_back_as_success() {
        let parsed = AuditResult::parse("garbage", None);
        assert!(!parsed.is_success());
        assert_eq!(parsed, AuditResult::Error("unknown error".to_string()));
    }

    #[test]
    fn event_carries_principal_and_metadata() {
        let event = AuditEvent::new(
            principal(),
            AuditAction::Simulate,
            "/simulate",
            AuditResult::Success,
        )
        .with_metadata("status", "200")
        .with_metadata("method", "POST");

        assert_eq!(event.principal.tenant_id.as_str(), "tenant_a");
        assert_eq!(event.metadata.len(), 2);
    }

    #[test]
    fn events_receive_distinct_identifiers() {
        let first = AuditEvent::new(
            principal(),
            AuditAction::Evaluate,
            "/evaluate",
            AuditResult::Success,
        );
        let second = AuditEvent::new(
            principal(),
            AuditAction::Evaluate,
            "/evaluate",
            AuditResult::Success,
        );

        assert_ne!(first.id, second.id);
        assert_eq!(first.action, second.action);
    }

    #[test]
    fn pagination_clamps_to_bounds() {
        assert_eq!(Pagination::new(0, 0).limit(), 1);
        assert_eq!(Pagination::new(u32::MAX, 7).offset(), 7);
    }

    #[test]
    fn default_pagination_is_bounded() {
        let page = Pagination::default();
        assert_eq!(page.limit(), 100);
        assert_eq!(page.offset(), 0);
    }
}
