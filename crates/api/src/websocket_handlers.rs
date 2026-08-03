//! WebSocket streaming for Monte Carlo simulations.
//!
//! `GET /ws/simulate` upgrades the connection. The client sends a single text
//! frame containing a JSON [`SimulateRequest`]; the server replies with progress
//! frames followed by a final result frame.

use std::sync::Arc;

use actix_web::{HttpRequest, HttpResponse, Responder, web};
use actix_ws::{MessageStream, Session, handle};
use serde::Serialize;
use tokio::sync::mpsc;
use tokio_stream::StreamExt;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tracing::{info, instrument};

use crate::engine_builder::{EngineBuilder, distribution_from_request};
use crate::models::{ErrorResponse, SimulateRequest, SimulateResponse};
use crate::validation::{validate_depth, validate_simulate};

/// WebSocket event emitted while a simulation is running.
#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum WsEvent {
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

/// Upgrades a request to a WebSocket and streams simulation progress.
#[instrument(name = "simulate_ws", skip(req, body))]
pub async fn simulate_ws(req: HttpRequest, body: web::Payload) -> impl Responder {
    info!("WebSocket simulate connection requested");

    let (response, session, stream) = match handle(&req, body) {
        Ok(parts) => parts,
        Err(err) => {
            return HttpResponse::BadRequest().json(ErrorResponse {
                error: format!("websocket handshake failed: {err}"),
            });
        }
    };

    let mut session = session;
    let mut stream = stream;

    actix_web::rt::spawn(async move {
        let request_json = match read_first_text_frame(&mut stream, &mut session).await {
            Ok(Some(text)) => text,
            Ok(None) => return,
            Err(message) => {
                let _ = session.text(event_json(&WsEvent::Error { message })).await;
                return;
            }
        };

        let request: SimulateRequest = match serde_json::from_str(&request_json) {
            Ok(req) => req,
            Err(err) => {
                let _ = session
                    .text(event_json(&WsEvent::Error {
                        message: format!("invalid SimulateRequest: {err}"),
                    }))
                    .await;
                return;
            }
        };

        if let Err(err) = run_simulation_stream(&request, &mut session).await {
            let _ = session
                .text(event_json(&WsEvent::Error { message: err }))
                .await;
        }
    });

    return response;
}

async fn read_first_text_frame(
    stream: &mut MessageStream,
    session: &mut Session,
) -> Result<Option<String>, String> {
    use actix_ws::Message;

    loop {
        let item = stream
            .next()
            .await
            .ok_or_else(|| "websocket stream closed before request".to_string())?;

        let msg = item.map_err(|err| format!("websocket error: {err}"))?;
        match msg {
            Message::Text(text) => return Ok(Some(text.to_string())),
            Message::Close(_) => {
                let _ = session.clone().close(None).await;
                return Ok(None);
            }
            Message::Ping(bytes) => {
                let _ = session.pong(&bytes).await;
            }
            _ => {}
        }
    }
}

async fn run_simulation_stream(
    request: &SimulateRequest,
    session: &mut Session,
) -> Result<(), String> {
    validate_simulate(request).map_err(|err| err.to_string())?;

    let mut builder = EngineBuilder::new();
    builder
        .add_nodes(&request.nodes)
        .map_err(|err| err.to_string())?;
    builder
        .add_edges(&request.edges)
        .map_err(|err| err.to_string())?;

    let target_id = builder
        .node_id(&request.target)
        .ok_or_else(|| "Target node not found".to_string())?;

    let mut config = casiros_simulator::MonteCarloConfig::new(
        request.universe_count,
        request.seed.unwrap_or(42),
    )
    .map_err(|err| err.to_string())?;

    for binding in &request.bindings {
        let node_id = builder
            .node_id(&binding.node)
            .ok_or_else(|| format!("Binding references unknown node '{}'", binding.node))?;
        config.bind(node_id, distribution_from_request(&binding.distribution));
    }

    let engine = builder.build();
    validate_depth(&engine).map_err(|err| err.to_string())?;

    let engine = Arc::new(engine);
    let config = Arc::new(config);
    let target_id = Arc::new(target_id);
    let universe_count = request.universe_count;

    let (tx, rx) = mpsc::unbounded_channel::<WsEvent>();

    tokio::task::spawn_blocking(move || {
        let chunk_size = (universe_count / 10).clamp(1, 1_000);
        let mut all_values = Vec::with_capacity(universe_count);

        for offset in (0..universe_count).step_by(chunk_size) {
            let count = chunk_size.min(universe_count - offset);
            match config.run_batch(&engine, *target_id, offset, count) {
                Ok(values) => all_values.extend(values),
                Err(err) => {
                    let _ = tx.send(WsEvent::Error {
                        message: err.to_string(),
                    });
                    return;
                }
            }

            let completed = offset + count;
            if let Ok(partial) = casiros_simulator::MonteCarloConfig::aggregate(all_values.clone())
            {
                let _ = tx.send(WsEvent::Progress {
                    completed,
                    total: universe_count,
                    partial: SimulateResponse {
                        count: partial.count,
                        mean: partial.mean,
                        median: partial.median,
                        min: partial.min,
                        max: partial.max,
                    },
                });
            }
        }

        match casiros_simulator::MonteCarloConfig::aggregate(all_values) {
            Ok(result) => {
                let _ = tx.send(WsEvent::Result {
                    result: SimulateResponse {
                        count: result.count,
                        mean: result.mean,
                        median: result.median,
                        min: result.min,
                        max: result.max,
                    },
                });
            }
            Err(err) => {
                let _ = tx.send(WsEvent::Error {
                    message: err.to_string(),
                });
            }
        }
    });

    let mut stream = UnboundedReceiverStream::new(rx);
    while let Some(event) = stream.next().await {
        session
            .text(event_json(&event))
            .await
            .map_err(|err| format!("websocket send failed: {err}"))?;
    }

    return Ok(());
}

/// Serializes a [`WsEvent`] into a JSON string.
fn event_json(event: &WsEvent) -> String {
    return serde_json::to_string(event).unwrap_or_default();
}
