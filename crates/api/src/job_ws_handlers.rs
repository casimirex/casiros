//! WebSocket streaming for simulation job progress.
//!
//! `GET /ws/jobs/{id}` upgrades the connection and streams progress frames as
//! the job moves through `Queued → Running → Completed/Failed`. The client
//! receives a JSON frame every time the job state changes.

use std::sync::Arc;
use std::time::Duration;

use actix_web::{HttpRequest, HttpResponse, Responder, web};
use actix_ws::handle;
use casiros_core::job::JobId;
use casiros_dag::job::JobStore;
use serde::Serialize;
use tracing::{info, instrument};

use crate::job_store::JobStoreHandle;
use crate::models::ErrorResponse;

/// How often the WebSocket handler polls the job store for updates.
const POLL_INTERVAL: Duration = Duration::from_millis(500);

/// WebSocket event emitted for job progress.
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum JobWsEvent {
    /// Job status changed.
    Status {
        /// Current job status string.
        status: String,
        /// Completion fraction in `0.0..=1.0`.
        fraction: f64,
        /// Universes completed so far.
        universes_completed: usize,
        /// Total universes to simulate.
        universes_total: usize,
    },
    /// Job completed successfully.
    #[serde(rename = "result")]
    Result {
        /// Serialised simulation results.
        result: serde_json::Value,
    },
    /// Job failed.
    Error {
        /// Error message.
        message: String,
    },
}

/// Upgrades a request to a WebSocket and streams job progress.
#[instrument(name = "job_ws", skip(req, body, store))]
pub async fn job_ws(
    req: HttpRequest,
    body: web::Payload,
    id: web::Path<String>,
    store: web::Data<JobStoreHandle>,
) -> impl Responder {
    info!("WebSocket job progress connection requested for id {}", id);

    let Ok(job_id) = id.parse::<JobId>() else {
        return HttpResponse::BadRequest().json(ErrorResponse {
            error: "Invalid job identifier".to_string(),
        });
    };

    let (response, session, _stream) = match handle(&req, body) {
        Ok(parts) => parts,
        Err(err) => {
            return HttpResponse::BadRequest().json(ErrorResponse {
                error: format!("websocket handshake failed: {err}"),
            });
        }
    };

    let store = Arc::clone(&store.into_inner());

    actix_web::rt::spawn(async move {
        if let Err(err) = stream_job_progress(session, store, job_id).await {
            tracing::error!("Job WebSocket stream failed: {err}");
        }
    });

    return response;
}

/// Polls the job store and sends progress frames until the job reaches a
/// terminal state.
async fn stream_job_progress(
    mut session: actix_ws::Session,
    store: Arc<JobStoreHandle>,
    job_id: JobId,
) -> Result<(), actix_ws::Closed> {
    // Use a default principal since the WebSocket handshake happens before
    // the auth middleware runs on the upgrade path.
    let tenant = casiros_core::tenant::TenantId::new("tenant_default")
        .expect("static default tenant is valid");
    let workspace = casiros_core::tenant::WorkspaceId::new("workspace_default")
        .expect("static default workspace is valid");

    loop {
        if let Ok(job) = store.get(&tenant, &workspace, &job_id).await {
            if job.status.is_terminal() {
                send_terminal_event(&mut session, &job).await;
                let _ = session.close(None).await;
                return Ok(());
            }

            let event = JobWsEvent::Status {
                status: job.status.as_str().to_string(),
                fraction: job.progress.fraction(),
                universes_completed: job.progress.universes_completed,
                universes_total: job.progress.universes_total,
            };
            let _ = session
                .text(serde_json::to_string(&event).unwrap_or_default())
                .await;
        } else {
            let event = JobWsEvent::Error {
                message: "Job not found".to_string(),
            };
            let _ = session
                .text(serde_json::to_string(&event).unwrap_or_default())
                .await;
            let _ = session.close(None).await;
            return Ok(());
        }

        tokio::time::sleep(POLL_INTERVAL).await;
    }
}

/// Sends the appropriate terminal event (result or error) for a completed job.
async fn send_terminal_event(
    session: &mut actix_ws::Session,
    job: &casiros_dag::job::SimulationJob,
) {
    match job.status.as_str() {
        "completed" => {
            let event = JobWsEvent::Result {
                result: job.result.clone().unwrap_or_default(),
            };
            let _ = session
                .text(serde_json::to_string(&event).unwrap_or_default())
                .await;
        }
        "failed" => {
            let event = JobWsEvent::Error {
                message: job
                    .error
                    .clone()
                    .unwrap_or_else(|| "unknown error".to_string()),
            };
            let _ = session
                .text(serde_json::to_string(&event).unwrap_or_default())
                .await;
        }
        _ => {}
    }
}
