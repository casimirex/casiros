//! HTTP handlers for the CASIROS REST API.

use actix_web::{HttpResponse, Responder, web};
use tracing::{info, instrument};

use crate::engine_builder::{EngineBuilder, distribution_from_request, map_inputs_by_id};
use crate::models::{
    AmortizationPeriodResponse, AmortizationScheduleRequest, AmortizationScheduleResponse,
    ErrorResponse, EvaluateRequest, EvaluateResponse, HealthzResponse, SimulateRequest,
    SimulateResponse,
};
use crate::validation::{
    validate_amortization_schedule, validate_depth, validate_evaluate, validate_simulate,
};

/// Health check endpoint for liveness and readiness probes.
#[utoipa::path(
    get,
    path = "/healthz",
    responses(
        (status = 200, description = "Service is healthy", body = HealthzResponse),
    )
)]
#[instrument(name = "healthz")]
pub async fn healthz() -> impl Responder {
    info!("Health check requested");
    return HttpResponse::Ok().json(HealthzResponse::ok());
}

/// Prometheus metrics endpoint.
///
/// Returns all registered metrics in Prometheus text format. This endpoint is
/// public (no authentication required) so that monitoring systems can scrape it
/// without an API key.
#[instrument(name = "metrics")]
pub async fn metrics() -> impl Responder {
    match crate::metrics::render() {
        Ok(body) => HttpResponse::Ok()
            .content_type("text/plain; charset=utf-8")
            .body(body),
        Err(err) => HttpResponse::InternalServerError().body(err),
    }
}

/// Evaluates a causality graph with fixed inputs.
#[utoipa::path(
    post,
    path = "/evaluate",
    request_body = EvaluateRequest,
    responses(
        (status = 200, description = "Evaluation succeeded", body = EvaluateResponse),
        (status = 400, description = "Invalid request or evaluation failure", body = ErrorResponse),
    )
)]
#[instrument(name = "evaluate", skip(payload))]
pub async fn evaluate(payload: web::Json<EvaluateRequest>) -> impl Responder {
    info!("Evaluate request received");

    if let Err(err) = validate_evaluate(&payload) {
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

    let inputs = match map_inputs_by_id(&builder, &payload.inputs) {
        Ok(map) => map,
        Err(err) => {
            return HttpResponse::BadRequest().json(ErrorResponse {
                error: err.to_string(),
            });
        }
    };

    let engine = builder.build();
    if let Err(err) = validate_depth(&engine) {
        return HttpResponse::BadRequest().json(ErrorResponse {
            error: err.to_string(),
        });
    }

    let outputs = match engine.evaluate(&inputs) {
        Ok(map) => map,
        Err(err) => {
            return HttpResponse::BadRequest().json(ErrorResponse {
                error: err.to_string(),
            });
        }
    };

    let name_to_id: std::collections::HashMap<_, _> = payload
        .nodes
        .iter()
        .map(|node| match node {
            crate::models::NodeRequest::Input { name }
            | crate::models::NodeRequest::Formula { name, .. } => name.clone(),
        })
        .enumerate()
        .map(|(idx, name)| (casiros_dag::graph::NodeId(idx), name))
        .collect();

    let response: std::collections::HashMap<String, casiros_core::prelude::Decimal> = outputs
        .into_iter()
        .map(|(id, value)| (name_to_id.get(&id).cloned().unwrap_or_default(), value))
        .collect();

    return HttpResponse::Ok().json(EvaluateResponse { outputs: response });
}

/// Generates a full amortization schedule for a fixed-rate loan.
///
/// Separate from `/evaluate` because the result is a table rather than a
/// single value. Every formula reachable through `/evaluate` produces one
/// `Decimal`, which is what a graph node evaluates to; a per-period repayment
/// breakdown cannot be expressed that way without discarding the breakdown.
#[utoipa::path(
    post,
    path = "/schedule/amortization",
    request_body = AmortizationScheduleRequest,
    responses(
        (status = 200, description = "Schedule generated", body = AmortizationScheduleResponse),
        (status = 400, description = "Invalid request", body = ErrorResponse),
    )
)]
#[instrument(name = "amortization_schedule", skip(payload))]
pub async fn amortization_schedule(
    payload: web::Json<AmortizationScheduleRequest>,
) -> impl Responder {
    info!("Amortization schedule request received");

    if let Err(err) = validate_amortization_schedule(&payload) {
        return HttpResponse::BadRequest().json(ErrorResponse {
            error: err.to_string(),
        });
    }

    let rows = match casiros_core::general::amortization_schedule(
        payload.principal,
        payload.rate,
        payload.periods,
    ) {
        Ok(rows) => rows,
        Err(err) => {
            return HttpResponse::BadRequest().json(ErrorResponse {
                error: err.to_string(),
            });
        }
    };

    // The level payment is principal + interest, identical every period by
    // construction. A zero-period request is legal and yields an empty
    // schedule, so there is no row to read it from.
    let payment = rows
        .first()
        .map(|row| row.principal_paid + row.interest_paid)
        .unwrap_or_default();

    let total_interest = rows
        .iter()
        .map(|row| row.interest_paid)
        .sum::<casiros_core::prelude::Decimal>();

    let schedule = rows
        .into_iter()
        .map(|row| AmortizationPeriodResponse {
            period: row.period,
            principal_paid: row.principal_paid,
            interest_paid: row.interest_paid,
            remaining_balance: row.remaining_balance,
        })
        .collect();

    return HttpResponse::Ok().json(AmortizationScheduleResponse {
        payment,
        total_interest,
        schedule,
    });
}

/// Runs a Monte Carlo simulation against a causality graph.
#[utoipa::path(
    post,
    path = "/simulate",
    request_body = SimulateRequest,
    responses(
        (status = 200, description = "Simulation succeeded", body = SimulateResponse),
        (status = 400, description = "Invalid request or simulation failure", body = ErrorResponse),
    )
)]
#[instrument(name = "simulate", skip(payload))]
pub async fn simulate(payload: web::Json<SimulateRequest>) -> impl Responder {
    info!("Simulate request received");

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

    let result = match config.run(&engine, target_id) {
        Ok(result) => result,
        Err(err) => {
            return HttpResponse::BadRequest().json(ErrorResponse {
                error: err.to_string(),
            });
        }
    };

    return HttpResponse::Ok().json(SimulateResponse {
        count: result.count,
        mean: result.mean,
        median: result.median,
        min: result.min,
        max: result.max,
    });
}
