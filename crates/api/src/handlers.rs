//! HTTP handlers for the CASIROS REST API.

use actix_web::{HttpResponse, Responder, web};
use tracing::{info, instrument};

use crate::engine_builder::{EngineBuilder, distribution_from_request, map_inputs_by_id};
use crate::models::{EvaluateRequest, EvaluateResponse, SimulateRequest, SimulateResponse};
use crate::validation::{validate_depth, validate_evaluate, validate_simulate};

/// Health check endpoint for liveness and readiness probes.
#[instrument(name = "healthz")]
pub async fn healthz() -> impl Responder {
    info!("Health check requested");
    return HttpResponse::Ok().json(serde_json::json!({ "status": "ok" }));
}

/// Evaluates a causality graph with fixed inputs.
///
/// # Request
///
/// POST `/evaluate` with a JSON body defining nodes, edges, and input values.
///
/// # Response
///
/// Returns HTTP 200 with a map of node names to computed values, or HTTP 400
/// if the request is invalid or evaluation fails.
#[instrument(name = "evaluate", skip(payload))]
pub async fn evaluate(payload: web::Json<EvaluateRequest>) -> impl Responder {
    info!("Evaluate request received");

    if let Err(err) = validate_evaluate(&payload) {
        return HttpResponse::BadRequest().json(serde_json::json!({ "error": err.to_string() }));
    }

    let mut builder = EngineBuilder::new();
    if let Err(err) = builder.add_nodes(&payload.nodes) {
        return HttpResponse::BadRequest().json(serde_json::json!({ "error": err.to_string() }));
    }
    if let Err(err) = builder.add_edges(&payload.edges) {
        return HttpResponse::BadRequest().json(serde_json::json!({ "error": err.to_string() }));
    }

    let inputs = match map_inputs_by_id(&builder, &payload.inputs) {
        Ok(map) => map,
        Err(err) => {
            return HttpResponse::BadRequest()
                .json(serde_json::json!({ "error": err.to_string() }));
        }
    };

    let engine = builder.build();
    if let Err(err) = validate_depth(&engine) {
        return HttpResponse::BadRequest().json(serde_json::json!({ "error": err.to_string() }));
    }

    let outputs = match engine.evaluate(&inputs) {
        Ok(map) => map,
        Err(err) => {
            return HttpResponse::BadRequest()
                .json(serde_json::json!({ "error": err.to_string() }));
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

/// Runs a Monte Carlo simulation against a causality graph.
///
/// # Request
///
/// POST `/simulate` with a JSON body defining nodes, edges, input
/// distributions, target node, and universe count.
///
/// # Response
///
/// Returns HTTP 200 with aggregated statistics, or HTTP 400 if the request is
/// invalid or simulation fails.
#[instrument(name = "simulate", skip(payload))]
pub async fn simulate(payload: web::Json<SimulateRequest>) -> impl Responder {
    info!("Simulate request received");

    if let Err(err) = validate_simulate(&payload) {
        return HttpResponse::BadRequest().json(serde_json::json!({ "error": err.to_string() }));
    }

    let mut builder = EngineBuilder::new();
    if let Err(err) = builder.add_nodes(&payload.nodes) {
        return HttpResponse::BadRequest().json(serde_json::json!({ "error": err.to_string() }));
    }
    if let Err(err) = builder.add_edges(&payload.edges) {
        return HttpResponse::BadRequest().json(serde_json::json!({ "error": err.to_string() }));
    }

    let Some(target_id) = builder.node_id(&payload.target) else {
        return HttpResponse::BadRequest()
            .json(serde_json::json!({ "error": "Target node not found" }));
    };

    let mut config = match casiros_simulator::MonteCarloConfig::new(
        payload.universe_count,
        payload.seed.unwrap_or(42),
    ) {
        Ok(cfg) => cfg,
        Err(err) => {
            return HttpResponse::BadRequest()
                .json(serde_json::json!({ "error": err.to_string() }));
        }
    };

    for binding in &payload.bindings {
        let Some(node_id) = builder.node_id(&binding.node) else {
            return HttpResponse::BadRequest()
                .json(serde_json::json!({ "error": format!("Binding references unknown node '{}'", binding.node) }));
        };
        config.bind(node_id, distribution_from_request(&binding.distribution));
    }

    let engine = builder.build();
    if let Err(err) = validate_depth(&engine) {
        return HttpResponse::BadRequest().json(serde_json::json!({ "error": err.to_string() }));
    }

    let result = match config.run(&engine, target_id) {
        Ok(result) => result,
        Err(err) => {
            return HttpResponse::BadRequest()
                .json(serde_json::json!({ "error": err.to_string() }));
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
