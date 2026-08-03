//! HTTP handlers for snapshot persistence.
//!
//! These handlers use the concrete [`SnapshotRepo`] wrapper so the same code
//! works with in-memory, Postgres, or S3 backends while remaining compatible
//! with `utoipa` `OpenAPI` generation.

use actix_web::{HttpResponse, Responder, web};
use casiros_dag::repository::SnapshotRepository;
use tracing::{info, instrument};

use crate::engine_builder::EngineBuilder;
use crate::models::{
    ErrorResponse, SaveSnapshotRequest, SaveSnapshotResponse, SnapshotListResponse,
    SnapshotResponse, SnapshotSummaryResponse,
};
use crate::repositories::SnapshotRepo;

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
    payload: web::Json<SaveSnapshotRequest>,
    repo: web::Data<SnapshotRepo>,
) -> impl Responder {
    info!("Save snapshot request received for id {}", payload.id);

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

    return match repo.save(&payload.id, &snapshot).await {
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
pub async fn load_snapshot(id: web::Path<String>, repo: web::Data<SnapshotRepo>) -> impl Responder {
    info!("Load snapshot request received for id {}", id);

    return match repo.load(&id).await {
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
    id: web::Path<String>,
    repo: web::Data<SnapshotRepo>,
) -> impl Responder {
    info!("Delete snapshot request received for id {}", id);

    return match repo.delete(&id).await {
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
pub async fn list_snapshots(repo: web::Data<SnapshotRepo>) -> impl Responder {
    info!("List snapshots request received");

    return match repo.list().await {
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
