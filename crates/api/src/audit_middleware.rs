//! Middleware that leaves an audit event for every authenticated request.
//!
//! The middleware runs *inside* [`crate::auth::auth_middleware`], so by the time
//! it executes a [`Principal`] has already been attached to the request. It
//! classifies the request into an [`AuditAction`] from the method and path, runs
//! the handler, and derives an [`AuditResult`] from the response status.
//!
//! ## Failure policy
//!
//! A failed audit write is logged at error level but does **not** fail the
//! request. Recording is best-effort so that an audit backend outage degrades
//! observability rather than taking the whole API down. Deployments that require
//! a fail-closed trail should alert on the `audit.write_failed` error log.

use std::sync::Arc;

use actix_web::HttpMessage;
use actix_web::body::{BoxBody, MessageBody};
use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::http::Method;
use actix_web::{Error, middleware::Next};
use casiros_core::audit::{AuditAction, AuditEvent, AuditResult};
use casiros_core::tenant::Principal;
use casiros_dag::audit::AuditLog;
use tracing::error;

use crate::audit::AuditSink;

/// Paths that are never audited because they carry no principal or resource.
const UNAUDITED_PREFIXES: [&str; 5] = [
    "/healthz",
    "/openapi.json",
    "/swagger-ui",
    "/api-docs",
    "/dashboard",
];

/// Records an [`AuditEvent`] for each request that reaches a real handler.
///
/// # Errors
///
/// Returns the underlying error if the inner service fails. Audit write
/// failures are logged rather than propagated.
pub async fn audit_middleware<B>(
    req: ServiceRequest,
    next: Next<B>,
    sink: Arc<AuditSink>,
) -> Result<ServiceResponse<BoxBody>, Error>
where
    B: MessageBody + 'static,
{
    let path = req.request().path().to_string();
    if is_unaudited(&path) {
        return Ok(next.call(req).await?.map_into_boxed_body());
    }

    let method = req.method().clone();
    let principal = req.request().extensions().get::<Principal>().cloned();
    let action = classify(&method, &path);

    let response = next.call(req).await?.map_into_boxed_body();

    // Only authenticated traffic carries a principal; unauthenticated rejects
    // are surfaced by the auth middleware's own logging instead.
    if let (Some(principal), Some(action)) = (principal, action) {
        let status = response.status();
        record_event(&sink, principal, action, &path, &method, status).await;
    }

    return Ok(response);
}

/// Writes one audit event, logging rather than propagating a backend failure.
async fn record_event(
    sink: &AuditSink,
    principal: Principal,
    action: AuditAction,
    path: &str,
    method: &Method,
    status: actix_web::http::StatusCode,
) {
    let event = AuditEvent::new(principal, action, path, result_for_status(status))
        .with_metadata("method", method.as_str())
        .with_metadata("status", status.as_u16().to_string());

    if let Err(err) = sink.record(event).await {
        error!(target: "audit.write_failed", "failed to record audit event: {err}");
    }
}

/// Returns true when a path should not produce audit events.
fn is_unaudited(path: &str) -> bool {
    return UNAUDITED_PREFIXES
        .iter()
        .any(|prefix| path == *prefix || path.starts_with(prefix));
}

/// Maps an HTTP method and path onto the action they represent.
///
/// Returns `None` for routes outside the audited surface, which are skipped.
fn classify(method: &Method, path: &str) -> Option<AuditAction> {
    if path.starts_with("/snapshots") {
        return match *method {
            Method::POST | Method::PUT => Some(AuditAction::SnapshotCreate),
            Method::GET => Some(AuditAction::SnapshotRead),
            Method::DELETE => Some(AuditAction::SnapshotDelete),
            _ => None,
        };
    }

    if path.starts_with("/simulate/jobs") {
        if path.ends_with("/cancel") {
            return Some(AuditAction::JobCancel);
        }
        return match *method {
            Method::POST => Some(AuditAction::JobCreate),
            Method::GET => Some(AuditAction::JobRead),
            _ => None,
        };
    }

    if path.starts_with("/simulate") || path.starts_with("/ws/simulate") {
        return Some(AuditAction::Simulate);
    }

    // Both are deterministic calculations against caller-supplied numbers.
    // They share an action because the event's `resource` field carries the
    // path, so the two remain distinguishable in the log without widening the
    // audit_action enum, which is a Postgres type and needs a migration to
    // change.
    if path.starts_with("/evaluate") || path.starts_with("/schedule") {
        return Some(AuditAction::Evaluate);
    }

    if path.starts_with("/audit") {
        return Some(AuditAction::JobRead);
    }

    return None;
}

/// Derives an [`AuditResult`] from the response status code.
fn result_for_status(status: actix_web::http::StatusCode) -> AuditResult {
    if status.is_success() {
        return AuditResult::Success;
    }
    if status == actix_web::http::StatusCode::FORBIDDEN
        || status == actix_web::http::StatusCode::UNAUTHORIZED
    {
        return AuditResult::Forbidden;
    }
    if status == actix_web::http::StatusCode::NOT_FOUND {
        return AuditResult::NotFound;
    }
    return AuditResult::Error(format!("request failed with status {}", status.as_u16()));
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::http::StatusCode;

    #[test]
    fn health_and_docs_are_not_audited() {
        assert!(is_unaudited("/healthz"));
        assert!(is_unaudited("/swagger-ui/index.html"));
    }

    #[test]
    fn business_routes_are_audited() {
        assert!(!is_unaudited("/evaluate"));
        assert!(!is_unaudited("/snapshots/abc"));
    }

    #[test]
    fn snapshot_methods_map_to_distinct_actions() {
        assert_eq!(
            classify(&Method::POST, "/snapshots"),
            Some(AuditAction::SnapshotCreate)
        );
        assert_eq!(
            classify(&Method::DELETE, "/snapshots/abc"),
            Some(AuditAction::SnapshotDelete)
        );
    }

    #[test]
    fn job_routes_map_to_job_actions() {
        assert_eq!(
            classify(&Method::POST, "/simulate/jobs"),
            Some(AuditAction::JobCreate)
        );
        assert_eq!(
            classify(&Method::POST, "/simulate/jobs/abc/cancel"),
            Some(AuditAction::JobCancel)
        );
    }

    #[test]
    fn job_routes_take_precedence_over_simulate() {
        assert_eq!(
            classify(&Method::GET, "/simulate/jobs/abc"),
            Some(AuditAction::JobRead)
        );
        assert_eq!(
            classify(&Method::POST, "/simulate"),
            Some(AuditAction::Simulate)
        );
    }

    #[test]
    fn unknown_routes_are_not_classified() {
        assert_eq!(classify(&Method::GET, "/nonsense"), None);
        assert_eq!(classify(&Method::PATCH, "/snapshots"), None);
    }

    #[test]
    fn status_maps_to_result() {
        assert_eq!(result_for_status(StatusCode::OK), AuditResult::Success);
        assert_eq!(
            result_for_status(StatusCode::NOT_FOUND),
            AuditResult::NotFound
        );
    }

    #[test]
    fn auth_failures_map_to_forbidden() {
        assert_eq!(
            result_for_status(StatusCode::UNAUTHORIZED),
            AuditResult::Forbidden
        );
        assert!(matches!(
            result_for_status(StatusCode::INTERNAL_SERVER_ERROR),
            AuditResult::Error(_)
        ));
    }
}
