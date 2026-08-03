//! Command implementations for `casiros-cli`.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use casiros_api::engine_builder::EngineBuilder;
use casiros_api::models::{EvaluateRequest, SimulateRequest};
use casiros_api::validation::{validate_depth, validate_evaluate, validate_simulate};
use casiros_dag::graph::{CausalityEngine, FormulaKind, Port};
use casiros_dag::persistence::{EngineSnapshot, SnapshotNodeKind};

/// Errors that can occur while running a CLI command.
#[derive(Debug, thiserror::Error)]
pub(crate) enum CliError {
    /// Failed to read a file.
    #[error("Failed to read {path}: {source}")]
    Read {
        /// Path of the file.
        path: String,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// Failed to write a file.
    #[error("Failed to write {path}: {source}")]
    Write {
        /// Path of the file.
        path: String,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// Failed to parse JSON.
    #[error("Failed to parse JSON in {path}: {source}")]
    Json {
        /// Path of the file.
        path: String,
        /// Underlying serde error.
        #[source]
        source: serde_json::Error,
    },

    /// Engine construction or validation failed.
    #[error("Engine error: {0}")]
    Engine(String),

    /// Simulation configuration failed.
    #[error("Simulation error: {0}")]
    Simulation(String),
}

/// Evaluates a graph from a JSON file and prints the response JSON.
pub(crate) fn evaluate(path: &Path) -> Result<String, CliError> {
    let request = read_evaluate_request(path)?;
    let engine = build_engine(&request.nodes, &request.edges)?;
    let outputs = engine
        .evaluate(&engine_inputs(&request.nodes, &request.inputs))
        .map_err(|err| CliError::Engine(err.to_string()))?;

    let response = casiros_api::models::EvaluateResponse {
        outputs: name_outputs(&request.nodes, &outputs),
    };
    return serde_json::to_string_pretty(&response).map_err(|err| CliError::Json {
        path: path.display().to_string(),
        source: err,
    });
}

/// Runs a Monte Carlo simulation from a JSON file and prints the response.
pub(crate) fn simulate(path: &Path) -> Result<String, CliError> {
    let request = read_simulate_request(path)?;
    validate_simulate(&request).map_err(|err| CliError::Engine(err.to_string()))?;

    let mut builder = EngineBuilder::new();
    builder
        .add_nodes(&request.nodes)
        .map_err(|err| CliError::Engine(err.to_string()))?;
    builder
        .add_edges(&request.edges)
        .map_err(|err| CliError::Engine(err.to_string()))?;

    let target_id = builder
        .node_id(&request.target)
        .ok_or_else(|| CliError::Engine("Target node not found".to_string()))?;

    let mut config = casiros_simulator::MonteCarloConfig::new(
        request.universe_count,
        request.seed.unwrap_or(42),
    )
    .map_err(|err| CliError::Simulation(err.to_string()))?;

    for binding in &request.bindings {
        let node_id = builder.node_id(&binding.node).ok_or_else(|| {
            CliError::Engine(format!(
                "Binding references unknown node '{}'",
                binding.node
            ))
        })?;
        config.bind(
            node_id,
            casiros_api::engine_builder::distribution_from_request(&binding.distribution),
        );
    }

    let engine = builder.build();
    validate_depth(&engine).map_err(|err| CliError::Engine(err.to_string()))?;

    let result = config
        .run(&engine, target_id)
        .map_err(|err| CliError::Simulation(err.to_string()))?;

    let response = casiros_api::models::SimulateResponse {
        count: result.count,
        mean: result.mean,
        median: result.median,
        min: result.min,
        max: result.max,
    };
    return serde_json::to_string_pretty(&response).map_err(|err| CliError::Json {
        path: path.display().to_string(),
        source: err,
    });
}

/// Validates a graph request and reports constraints.
pub(crate) fn validate(path: &Path) -> Result<String, CliError> {
    let request = read_evaluate_request(path)?;
    validate_evaluate(&request).map_err(|err| CliError::Engine(err.to_string()))?;

    let engine = build_engine(&request.nodes, &request.edges)?;
    let depth = engine
        .max_depth()
        .map_err(|err| CliError::Engine(err.to_string()))?;

    let report = serde_json::json!({
        "valid": true,
        "node_count": engine.len(),
        "edge_count": request.edges.len(),
        "depth": depth,
    });
    return Ok(report.to_string());
}

/// Loads an engine request JSON file and writes a stable snapshot JSON file.
pub(crate) fn save(engine_path: &Path, snapshot_path: &Path) -> Result<(), CliError> {
    let request = read_evaluate_request(engine_path)?;
    let engine = build_engine(&request.nodes, &request.edges)?;
    let snapshot = engine.to_snapshot();
    let json = serde_json::to_string_pretty(&snapshot).map_err(|err| CliError::Json {
        path: snapshot_path.display().to_string(),
        source: err,
    })?;
    fs::write(snapshot_path, json).map_err(|err| CliError::Write {
        path: snapshot_path.display().to_string(),
        source: err,
    })?;
    return Ok(());
}

/// Loads a snapshot JSON file and writes an engine request JSON file.
pub(crate) fn load(snapshot_path: &Path, engine_path: &Path) -> Result<(), CliError> {
    let snapshot = read_snapshot(snapshot_path)?;
    let engine = CausalityEngine::from_snapshot(&snapshot)
        .map_err(|err| CliError::Engine(err.to_string()))?;

    let id_to_name: HashMap<casiros_dag::graph::NodeId, String> = engine
        .nodes()
        .map(|node| (node.id(), node.name().to_string()))
        .collect();

    let nodes: Vec<casiros_api::models::NodeRequest> = snapshot
        .nodes
        .iter()
        .map(|node| match &node.kind {
            SnapshotNodeKind::Input => casiros_api::models::NodeRequest::Input {
                name: node.name.clone(),
            },
            SnapshotNodeKind::Formula { formula } => casiros_api::models::NodeRequest::Formula {
                name: node.name.clone(),
                kind: formula_request_from_kind(formula, &id_to_name),
            },
        })
        .collect();

    let edges: Vec<casiros_api::models::EdgeRequest> = snapshot
        .edges
        .iter()
        .map(|(dependency, dependent)| casiros_api::models::EdgeRequest {
            dependency: dependency.clone(),
            dependent: dependent.clone(),
        })
        .collect();

    let request = EvaluateRequest {
        nodes,
        edges,
        inputs: HashMap::new(),
    };

    let json = serde_json::to_string_pretty(&request).map_err(|err| CliError::Json {
        path: engine_path.display().to_string(),
        source: err,
    })?;
    fs::write(engine_path, json).map_err(|err| CliError::Write {
        path: engine_path.display().to_string(),
        source: err,
    })?;
    return Ok(());
}

fn read_evaluate_request(path: &Path) -> Result<EvaluateRequest, CliError> {
    let text = fs::read_to_string(path).map_err(|err| CliError::Read {
        path: path.display().to_string(),
        source: err,
    })?;
    return serde_json::from_str(&text).map_err(|err| CliError::Json {
        path: path.display().to_string(),
        source: err,
    });
}

fn read_simulate_request(path: &Path) -> Result<SimulateRequest, CliError> {
    let text = fs::read_to_string(path).map_err(|err| CliError::Read {
        path: path.display().to_string(),
        source: err,
    })?;
    return serde_json::from_str(&text).map_err(|err| CliError::Json {
        path: path.display().to_string(),
        source: err,
    });
}

fn read_snapshot(path: &Path) -> Result<EngineSnapshot, CliError> {
    let text = fs::read_to_string(path).map_err(|err| CliError::Read {
        path: path.display().to_string(),
        source: err,
    })?;
    return serde_json::from_str(&text).map_err(|err| CliError::Json {
        path: path.display().to_string(),
        source: err,
    });
}

fn build_engine(
    nodes: &[casiros_api::models::NodeRequest],
    edges: &[casiros_api::models::EdgeRequest],
) -> Result<CausalityEngine, CliError> {
    let mut builder = EngineBuilder::new();
    builder
        .add_nodes(nodes)
        .map_err(|err| CliError::Engine(err.to_string()))?;
    builder
        .add_edges(edges)
        .map_err(|err| CliError::Engine(err.to_string()))?;
    return Ok(builder.build());
}

fn engine_inputs(
    nodes: &[casiros_api::models::NodeRequest],
    inputs: &HashMap<String, casiros_core::prelude::Decimal>,
) -> HashMap<casiros_dag::graph::NodeId, casiros_core::prelude::Decimal> {
    let name_to_index: HashMap<String, usize> = nodes
        .iter()
        .enumerate()
        .map(|(idx, node)| {
            let name = match node {
                casiros_api::models::NodeRequest::Input { name }
                | casiros_api::models::NodeRequest::Formula { name, .. } => name.clone(),
            };
            (name, idx)
        })
        .collect();

    let mut by_id = HashMap::new();
    for (name, value) in inputs {
        if let Some(&idx) = name_to_index.get(name) {
            by_id.insert(casiros_dag::graph::NodeId(idx), *value);
        }
    }
    return by_id;
}

fn name_outputs(
    nodes: &[casiros_api::models::NodeRequest],
    outputs: &HashMap<casiros_dag::graph::NodeId, casiros_core::prelude::Decimal>,
) -> HashMap<String, casiros_core::prelude::Decimal> {
    return nodes
        .iter()
        .enumerate()
        .map(|(idx, node)| {
            let name = match node {
                casiros_api::models::NodeRequest::Input { name }
                | casiros_api::models::NodeRequest::Formula { name, .. } => name.clone(),
            };
            let value = outputs
                .get(&casiros_dag::graph::NodeId(idx))
                .copied()
                .unwrap_or_default();
            (name, value)
        })
        .collect();
}

#[allow(clippy::too_many_lines)]
fn formula_request_from_kind(
    kind: &FormulaKind,
    id_to_name: &HashMap<casiros_dag::graph::NodeId, String>,
) -> casiros_api::models::FormulaRequest {
    fn port(
        port: &Port,
        id_to_name: &HashMap<casiros_dag::graph::NodeId, String>,
    ) -> casiros_api::models::PortRequest {
        match *port {
            Port::Constant(value) => casiros_api::models::PortRequest::Constant(value),
            Port::Output(id) => casiros_api::models::PortRequest::Output {
                node: id_to_name.get(&id).cloned().unwrap_or_default(),
            },
        }
    }

    return match kind {
        FormulaKind::FutureValue {
            present_value,
            rate,
            periods,
        } => casiros_api::models::FormulaRequest::FutureValue {
            present_value: port(present_value, id_to_name),
            rate: port(rate, id_to_name),
            periods: port(periods, id_to_name),
        },
        FormulaKind::PresentValue {
            future_value,
            rate,
            periods,
        } => casiros_api::models::FormulaRequest::PresentValue {
            future_value: port(future_value, id_to_name),
            rate: port(rate, id_to_name),
            periods: port(periods, id_to_name),
        },
        FormulaKind::ReturnOnEquity { net_income, equity } => {
            casiros_api::models::FormulaRequest::ReturnOnEquity {
                net_income: port(net_income, id_to_name),
                equity: port(equity, id_to_name),
            }
        }
        FormulaKind::Wacc {
            equity_value,
            debt_value,
            cost_of_equity,
            cost_of_debt,
            tax_rate,
        } => casiros_api::models::FormulaRequest::Wacc {
            equity_value: port(equity_value, id_to_name),
            debt_value: port(debt_value, id_to_name),
            cost_of_equity: port(cost_of_equity, id_to_name),
            cost_of_debt: port(cost_of_debt, id_to_name),
            tax_rate: port(tax_rate, id_to_name),
        },
        FormulaKind::SustainableGrowthRate {
            roe,
            dividend_payout_ratio,
        } => casiros_api::models::FormulaRequest::SustainableGrowthRate {
            roe: port(roe, id_to_name),
            dividend_payout_ratio: port(dividend_payout_ratio, id_to_name),
        },
        FormulaKind::AmortizationPayment {
            principal,
            rate,
            periods,
        } => casiros_api::models::FormulaRequest::AmortizationPayment {
            principal: port(principal, id_to_name),
            rate: port(rate, id_to_name),
            periods: port(periods, id_to_name),
        },
        FormulaKind::YieldToMaturityApproximation {
            face_value,
            coupon_payment,
            price,
            periods,
        } => casiros_api::models::FormulaRequest::YieldToMaturityApproximation {
            face_value: port(face_value, id_to_name),
            coupon_payment: port(coupon_payment, id_to_name),
            price: port(price, id_to_name),
            periods: port(periods, id_to_name),
        },
        FormulaKind::SimpleMovingAverage { prices, window } => {
            casiros_api::models::FormulaRequest::SimpleMovingAverage {
                prices: port(prices, id_to_name),
                window: port(window, id_to_name),
            }
        }
        FormulaKind::BlackScholesCall {
            spot,
            strike,
            risk_free_rate,
            volatility,
            time_to_maturity,
        } => casiros_api::models::FormulaRequest::BlackScholesCall {
            spot: port(spot, id_to_name),
            strike: port(strike, id_to_name),
            risk_free_rate: port(risk_free_rate, id_to_name),
            volatility: port(volatility, id_to_name),
            time_to_maturity: port(time_to_maturity, id_to_name),
        },
        FormulaKind::BlackScholesPut {
            spot,
            strike,
            risk_free_rate,
            volatility,
            time_to_maturity,
        } => casiros_api::models::FormulaRequest::BlackScholesPut {
            spot: port(spot, id_to_name),
            strike: port(strike, id_to_name),
            risk_free_rate: port(risk_free_rate, id_to_name),
            volatility: port(volatility, id_to_name),
            time_to_maturity: port(time_to_maturity, id_to_name),
        },
    };
}
