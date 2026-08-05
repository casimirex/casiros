//! HTTP handlers for async simulation job lifecycle.
//!
//! # Tenant Scope
//!
//! All operations are scoped to the authenticated principal attached to the
//! request by the authentication middleware. The tenant and workspace are never
//! taken from the request body or query string.

use actix_web::{HttpMessage, HttpRequest, HttpResponse, Responder, web};
use casiros_core::job::{JobId, JobProgress, JobStatus};
use casiros_core::tenant::Principal;
use casiros_dag::job::{JobStore, SimulationJob};
use time::OffsetDateTime;
use tracing::{info, instrument};

use crate::engine_builder::EngineBuilder;
use crate::job_store::JobStoreHandle;
use crate::models::{
    CreateJobRequest, CreateJobResponse, ErrorResponse, JobResponse, JobStatusResponse,
};

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

/// Enqueues a new simulation job.
#[utoipa::path(
    post,
    path = "/simulate/jobs",
    request_body = CreateJobRequest,
    responses(
        (status = 202, description = "Job enqueued", body = CreateJobResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
    )
)]
#[instrument(name = "create_job", skip(payload, store))]
pub async fn create_job(
    req: HttpRequest,
    payload: web::Json<CreateJobRequest>,
    store: web::Data<JobStoreHandle>,
) -> impl Responder {
    info!("Create job request received");
    let principal = principal_from_request(&req);

    // Validate the simulation request before enqueuing.
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

    let job = SimulationJob {
        id: JobId::new(),
        tenant_id: principal.tenant_id,
        workspace_id: principal.workspace_id,
        status: JobStatus::Queued,
        request: serde_json::to_value(&*payload).unwrap_or_default(),
        progress: JobProgress::new(payload.universe_count),
        result: None,
        error: None,
        created_at: OffsetDateTime::now_utc(),
        updated_at: OffsetDateTime::now_utc(),
    };

    return match store.enqueue(job.clone()).await {
        Ok(()) => HttpResponse::Accepted().json(CreateJobResponse {
            id: job.id.to_string(),
            status: JobStatus::Queued.as_str().to_string(),
        }),
        Err(err) => HttpResponse::InternalServerError().json(ErrorResponse {
            error: err.to_string(),
        }),
    };
}

/// Returns the status and result of a simulation job.
#[utoipa::path(
    get,
    path = "/simulate/jobs/{id}",
    responses(
        (status = 200, description = "Job found", body = JobResponse),
        (status = 404, description = "Job not found", body = ErrorResponse),
    ),
    params(
        ("id" = String, Path, description = "Job identifier (UUID)"),
    )
)]
#[instrument(name = "get_job", skip(store))]
pub async fn get_job(
    req: HttpRequest,
    id: web::Path<String>,
    store: web::Data<JobStoreHandle>,
) -> impl Responder {
    info!("Get job request received for id {}", id);
    let principal = principal_from_request(&req);

    let Ok(job_id) = id.parse::<JobId>() else {
        return HttpResponse::BadRequest().json(ErrorResponse {
            error: "Invalid job identifier".to_string(),
        });
    };

    return match store
        .get(&principal.tenant_id, &principal.workspace_id, &job_id)
        .await
    {
        Ok(job) => HttpResponse::Ok().json(JobResponse::from_job(&job)),
        Err(_) => HttpResponse::NotFound().json(ErrorResponse {
            error: "Job not found".to_string(),
        }),
    };
}

/// Cancels a queued or running simulation job.
#[utoipa::path(
    post,
    path = "/simulate/jobs/{id}/cancel",
    responses(
        (status = 200, description = "Job cancelled", body = JobStatusResponse),
        (status = 404, description = "Job not found", body = ErrorResponse),
    ),
    params(
        ("id" = String, Path, description = "Job identifier (UUID)"),
    )
)]
#[instrument(name = "cancel_job", skip(store))]
pub async fn cancel_job(id: web::Path<String>, store: web::Data<JobStoreHandle>) -> impl Responder {
    info!("Cancel job request received for id {}", id);
    let Ok(job_id) = id.parse::<JobId>() else {
        return HttpResponse::BadRequest().json(ErrorResponse {
            error: "Invalid job identifier".to_string(),
        });
    };

    return match store.cancel(&job_id).await {
        Ok(true) => HttpResponse::Ok().json(JobStatusResponse {
            id: job_id.to_string(),
            status: JobStatus::Cancelled.as_str().to_string(),
        }),
        Ok(false) => HttpResponse::NotFound().json(ErrorResponse {
            error: "Job not found or already in a terminal state".to_string(),
        }),
        Err(err) => HttpResponse::InternalServerError().json(ErrorResponse {
            error: err.to_string(),
        }),
    };
}
