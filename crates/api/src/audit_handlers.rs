//! HTTP handlers for reading the audit trail.
//!
//! # Tenant Scope
//!
//! The trail is always read for the authenticated principal's own tenant. The
//! tenant is never taken from the query string, so a caller cannot page through
//! another tenant's history by guessing an identifier.

use actix_web::{HttpMessage, HttpRequest, HttpResponse, Responder, web};
use casiros_core::audit::Pagination;
use casiros_core::tenant::Principal;
use casiros_dag::audit::AuditLog;
use tracing::{info, instrument};

use crate::audit::AuditSink;
use crate::models::{AuditEventResponse, AuditListQuery, AuditListResponse, ErrorResponse};

/// Returns the principal for the current request, or the default principal.
fn principal_from_request(req: &HttpRequest) -> Principal {
    return req
        .extensions()
        .get::<Principal>()
        .cloned()
        .unwrap_or_else(default_principal);
}

/// Default principal used when middleware has not injected one.
fn default_principal() -> Principal {
    let tenant = casiros_core::tenant::TenantId::new("tenant_default")
        .expect("static default tenant is valid");
    let workspace = casiros_core::tenant::WorkspaceId::new("workspace_default")
        .expect("static default workspace is valid");
    return Principal::new(tenant, workspace, "anonymous");
}

/// Lists audit events for the caller's tenant, newest first.
#[utoipa::path(
    get,
    path = "/audit",
    responses(
        (status = 200, description = "Audit events", body = AuditListResponse),
        (status = 500, description = "Audit backend failure", body = ErrorResponse),
    ),
    params(
        ("limit" = Option<u32>, Query, description = "Page size (clamped to 1..=1000)"),
        ("offset" = Option<u32>, Query, description = "Rows to skip"),
    )
)]
#[instrument(name = "list_audit_events", skip(sink))]
pub async fn list_audit_events(
    req: HttpRequest,
    query: web::Query<AuditListQuery>,
    sink: web::Data<AuditSink>,
) -> impl Responder {
    info!("Audit list request received");
    let principal = principal_from_request(&req);
    let pagination = Pagination::new(query.limit.unwrap_or(100), query.offset.unwrap_or(0));

    return match sink.list(principal.tenant_id, pagination).await {
        Ok(events) => {
            let events: Vec<AuditEventResponse> =
                events.iter().map(AuditEventResponse::from_event).collect();
            HttpResponse::Ok().json(AuditListResponse {
                total: events.len(),
                events,
            })
        }
        Err(err) => HttpResponse::InternalServerError().json(ErrorResponse {
            error: err.to_string(),
        }),
    };
}
