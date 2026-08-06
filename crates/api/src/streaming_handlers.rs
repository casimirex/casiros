//! Server-sent event streaming for long-running Monte Carlo simulations.
//!
//! The `POST /simulate/stream` endpoint runs a simulation in chunks and emits
//! progress updates as SSE messages. The final message contains the aggregated
//! [`SimulateResponse`].

use std::sync::Arc;

use actix_web::{HttpResponse, Responder, web};
use serde::Serialize;
use tokio::sync::mpsc;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tracing::{info, instrument};

use crate::engine_builder::{EngineBuilder, distribution_from_request};
use crate::models::{ErrorResponse, SimulateRequest, SimulateResponse};
use crate::validation::{validate_depth, validate_simulate};

/// SSE event emitted while a simulation is running.
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum StreamEvent {
    /// Progress update containing a partial aggregate.
    Progress {
        /// Number of universes completed so far.
        completed: usize,
        /// Total number of universes requested.
        total: usize,
        /// Partial aggregate up to `completed` universes.
        partial: SimulateResponse,
    },
    /// Final simulation result.
    #[serde(rename = "result")]
    Result {
        /// Final aggregated response.
        result: SimulateResponse,
    },
    /// Error event if the simulation fails.
    Error {
        /// Human-readable error message.
        message: String,
    },
}

/// Streams a Monte Carlo simulation as server-sent events.
#[allow(clippy::too_many_lines)]
#[instrument(name = "simulate_stream", skip(payload))]
pub async fn simulate_stream(payload: web::Json<SimulateRequest>) -> impl Responder {
    info!("Streaming simulate request received");

    if let Err(err) = validate_simulate(&payload) {
        return HttpResponse::BadRequest().json(ErrorResponse {
            error: err.to_string(),
        });
    }

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

    let Some(target_id) = builder.node_id(&payload.target) else {
        return HttpResponse::BadRequest().json(ErrorResponse {
            error: "Target node not found".to_string(),
        });
    };

    let mut config = match casiros_simulator::MonteCarloConfig::new(
        payload.universe_count,
        payload.seed.unwrap_or(42),
    ) {
        Ok(cfg) => cfg,
        Err(err) => {
            return HttpResponse::BadRequest().json(ErrorResponse {
                error: err.to_string(),
            });
        }
    };

    for binding in &payload.bindings {
        let Some(node_id) = builder.node_id(&binding.node) else {
            return HttpResponse::BadRequest().json(ErrorResponse {
                error: format!("Binding references unknown node '{}'", binding.node),
            });
        };
        config.bind(node_id, distribution_from_request(&binding.distribution));
    }

    let engine = builder.build();
    if let Err(err) = validate_depth(&engine) {
        return HttpResponse::BadRequest().json(ErrorResponse {
            error: err.to_string(),
        });
    }

    let engine = Arc::new(engine);
    let config = Arc::new(config);
    let target_id = Arc::new(target_id);
    let universe_count = payload.universe_count;

    let (tx, rx) = mpsc::unbounded_channel::<
        Result<actix_web::web::Bytes, casiros_simulator::SimulationError>,
    >();

    tokio::task::spawn_blocking(move || {
        let chunk_size = (universe_count / 10).clamp(1, 1_000);
        let mut all_values = Vec::with_capacity(universe_count);

        for offset in (0..universe_count).step_by(chunk_size) {
            let count = chunk_size.min(universe_count - offset);
            match config.run_batch(&engine, *target_id, offset, count) {
                Ok(values) => all_values.extend(values),
                Err(err) => {
                    let _ = tx.send(Ok(event_bytes(&StreamEvent::Error {
                        message: err.to_string(),
                    })));
                    return;
                }
            }

            let completed = offset + count;
            if let Ok(partial) = casiros_simulator::MonteCarloConfig::aggregate(all_values.clone())
            {
                let _ = tx.send(Ok(event_bytes(&StreamEvent::Progress {
                    completed,
                    total: universe_count,
                    partial: SimulateResponse {
                        count: partial.count,
                        mean: partial.mean,
                        median: partial.median,
                        min: partial.min,
                        max: partial.max,
                    },
                })));
            }
        }

        match casiros_simulator::MonteCarloConfig::aggregate(all_values) {
            Ok(result) => {
                let _ = tx.send(Ok(event_bytes(&StreamEvent::Result {
                    result: SimulateResponse {
                        count: result.count,
                        mean: result.mean,
                        median: result.median,
                        min: result.min,
                        max: result.max,
                    },
                })));
            }
            Err(err) => {
                let _ = tx.send(Ok(event_bytes(&StreamEvent::Error {
                    message: err.to_string(),
                })));
            }
        }
    });

    let stream = UnboundedReceiverStream::new(rx).map(|item| {
        item.map_err(|err| actix_web::error::ErrorInternalServerError(err.to_string()))
    });

    return HttpResponse::Ok()
        .content_type("text/event-stream")
        .insert_header(("Cache-Control", "no-cache"))
        .streaming(stream);
}

/// Serializes a [`StreamEvent`] into an SSE data frame.
fn event_bytes(event: &StreamEvent) -> actix_web::web::Bytes {
    let json = serde_json::to_string(event).unwrap_or_default();
    return actix_web::web::Bytes::from(format!("data: {json}\n\n"));
}
