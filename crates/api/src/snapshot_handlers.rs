//! HTTP handlers for snapshot persistence.
//!
//! These handlers use the concrete [`SnapshotRepo`] wrapper so the same code
//! works with in-memory, Postgres, or S3 backends while remaining compatible
//! with `utoipa` `OpenAPI` generation.
//!
//! # Tenant/Workspace Scope
//!
//! All operations are scoped to the authenticated principal attached to the
//! request by the authentication middleware.

use actix_web::{HttpMessage, HttpRequest, HttpResponse, Responder, web};
use casiros_core::tenant::Principal;
use casiros_dag::repository::SnapshotRepository;
use tracing::{info, instrument};

use crate::engine_builder::EngineBuilder;
use crate::models::{
    ErrorResponse, SaveSnapshotRequest, SaveSnapshotResponse, SnapshotListResponse,
    SnapshotResponse, SnapshotSummaryResponse,
};
use crate::repositories::SnapshotRepo;

/// Returns the principal for the current request.
///
/// The authentication middleware inserts a [`Principal`] into the request
/// extensions. If it is missing (for example, in a test that does not include
/// the middleware), a default tenant/workspace is returned so handlers remain
/// callable.
#[must_use]
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

/// Saves a causality graph as a named snapshot.
#[utoipa::path(
    post,
    path = "/snapshots",
    request_body = SaveSnapshotRequest,
    responses(
        (status = 200, description = "Snapshot saved", body = SaveSnapshotResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
    )
)]
#[instrument(name = "save_snapshot", skip(payload, repo))]
pub async fn save_snapshot(
    req: HttpRequest,
    payload: web::Json<SaveSnapshotRequest>,
    repo: web::Data<SnapshotRepo>,
) -> impl Responder {
    info!("Save snapshot request received for id {}", payload.id);
    let principal = principal_from_request(&req);

    let mut builder = EngineBuilder::new();
    if let Err(err) = builder.add_nodes(&payload.nodes) {
        return HttpResponse::BadRequest().json(ErrorResponse {
            error: err.to_string(),
        });
    }
    if let Err(err) = builder.add_edges(&payload.edges) {
        return HttpResponse::BadRequest().json(ErrorResponse {
            error: err.to_string(),
        });
    }

    let engine = builder.build();
    let snapshot = engine.to_snapshot();

    return match repo
        .save(
            principal.tenant_id,
            principal.workspace_id,
            &payload.id,
            &snapshot,
        )
        .await
    {
        Ok(()) => HttpResponse::Ok().json(SaveSnapshotResponse {
            id: payload.id.clone(),
        }),
        Err(err) => HttpResponse::BadRequest().json(ErrorResponse {
            error: err.to_string(),
        }),
    };
}

/// Loads a previously saved snapshot and returns its node/edge structure.
#[utoipa::path(
    get,
    path = "/snapshots/{id}",
    responses(
        (status = 200, description = "Snapshot found", body = SnapshotResponse),
        (status = 404, description = "Snapshot not found", body = ErrorResponse),
    ),
    params(
        ("id" = String, Path, description = "Snapshot identifier"),
    )
)]
#[instrument(name = "load_snapshot", skip(repo))]
pub async fn load_snapshot(
    req: HttpRequest,
    id: web::Path<String>,
    repo: web::Data<SnapshotRepo>,
) -> impl Responder {
    info!("Load snapshot request received for id {}", id);
    let principal = principal_from_request(&req);

    return match repo
        .load(principal.tenant_id, principal.workspace_id, &id)
        .await
    {
        Ok(snapshot) => match serde_json::to_value(&snapshot) {
            Ok(data) => HttpResponse::Ok().json(SnapshotResponse {
                id: id.to_string(),
                data,
            }),
            Err(err) => HttpResponse::InternalServerError().json(ErrorResponse {
                error: err.to_string(),
            }),
        },
        Err(err) => HttpResponse::NotFound().json(ErrorResponse {
            error: err.to_string(),
        }),
    };
}

/// Deletes a snapshot.
#[utoipa::path(
    delete,
    path = "/snapshots/{id}",
    responses(
        (status = 200, description = "Snapshot deleted"),
        (status = 404, description = "Snapshot not found", body = ErrorResponse),
    ),
    params(
        ("id" = String, Path, description = "Snapshot identifier"),
    )
)]
#[instrument(name = "delete_snapshot", skip(repo))]
pub async fn delete_snapshot(
    req: HttpRequest,
    id: web::Path<String>,
    repo: web::Data<SnapshotRepo>,
) -> impl Responder {
    info!("Delete snapshot request received for id {}", id);
    let principal = principal_from_request(&req);

    return match repo
        .delete(principal.tenant_id, principal.workspace_id, &id)
        .await
    {
        Ok(()) => HttpResponse::Ok().finish(),
        Err(err) => HttpResponse::NotFound().json(ErrorResponse {
            error: err.to_string(),
        }),
    };
}

/// Lists all stored snapshots.
#[utoipa::path(
    get,
    path = "/snapshots",
    responses(
        (status = 200, description = "Snapshot list", body = SnapshotListResponse),
    )
)]
#[instrument(name = "list_snapshots", skip(repo))]
pub async fn list_snapshots(req: HttpRequest, repo: web::Data<SnapshotRepo>) -> impl Responder {
    info!("List snapshots request received");
    let principal = principal_from_request(&req);

    return match repo.list(principal.tenant_id, principal.workspace_id).await {
        Ok(summaries) => HttpResponse::Ok().json(SnapshotListResponse {
            snapshots: summaries
                .into_iter()
                .map(|s| SnapshotSummaryResponse { id: s.id })
                .collect(),
        }),
        Err(err) => HttpResponse::BadRequest().json(ErrorResponse {
            error: err.to_string(),
        }),
    };
}
