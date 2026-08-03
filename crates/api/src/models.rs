//! JSON request and response models for the CASIROS HTTP API.
//!
//! These models are intentionally flat and serialization-friendly. They are the
//! public contract of the REST API and are translated into the richer domain
//! types provided by [`casiros_dag`] and [`casiros_simulator`].

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// A single node in a DAG request.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeRequest {
    /// A raw numeric input provided by the caller.
    Input {
        /// Human-readable name for the node.
        name: String,
    },
    /// A formula from the CASIROS catalog.
    Formula {
        /// Human-readable name for the node.
        name: String,
        /// The specific formula and its port bindings.
        kind: FormulaRequest,
    },
}

/// A formula request with concrete port bindings.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", tag = "formula")]
pub enum FormulaRequest {
    /// Future value: `FV = PV * (1 + r)^n`.
    FutureValue {
        /// Present value binding.
        present_value: PortRequest,
        /// Rate binding.
        rate: PortRequest,
        /// Periods binding.
        periods: PortRequest,
    },
    /// Present value: `PV = FV / (1 + r)^n`.
    PresentValue {
        /// Future value binding.
        future_value: PortRequest,
        /// Rate binding.
        rate: PortRequest,
        /// Periods binding.
        periods: PortRequest,
    },
    /// Return on equity.
    ReturnOnEquity {
        /// Net income binding.
        net_income: PortRequest,
        /// Equity binding.
        equity: PortRequest,
    },
    /// Weighted average cost of capital.
    Wacc {
        /// Equity value binding.
        equity_value: PortRequest,
        /// Debt value binding.
        debt_value: PortRequest,
        /// Cost of equity binding.
        cost_of_equity: PortRequest,
        /// Cost of debt binding.
        cost_of_debt: PortRequest,
        /// Tax rate binding.
        tax_rate: PortRequest,
    },
    /// Sustainable growth rate.
    SustainableGrowthRate {
        /// ROE binding.
        roe: PortRequest,
        /// Dividend payout ratio binding.
        dividend_payout_ratio: PortRequest,
    },
}

/// A port binding: either a literal value or a reference to another node by
/// name.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum PortRequest {
    /// A literal constant value.
    Constant(Decimal),
    /// A reference to the output of another node.
    Output {
        /// Name of the source node.
        node: String,
    },
}

/// A directed edge between two nodes.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EdgeRequest {
    /// Name of the dependency node.
    pub dependency: String,
    /// Name of the dependent node.
    pub dependent: String,
}

/// Request body for `POST /evaluate`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct EvaluateRequest {
    /// Nodes that make up the DAG.
    pub nodes: Vec<NodeRequest>,
    /// Directed edges between nodes.
    pub edges: Vec<EdgeRequest>,
    /// Values for every input node, keyed by node name.
    pub inputs: std::collections::HashMap<String, Decimal>,
}

/// Response body for `POST /evaluate`.
#[derive(Debug, Clone, Serialize)]
pub struct EvaluateResponse {
    /// Computed value for every node, keyed by node name.
    pub outputs: std::collections::HashMap<String, Decimal>,
}

/// A distribution request used by the simulator.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "snake_case", tag = "kind")]
pub enum DistributionRequest {
    /// Uniform distribution over `[low, high]`.
    Uniform {
        /// Lower bound.
        low: f64,
        /// Upper bound.
        high: f64,
    },
    /// Normal distribution with mean and standard deviation.
    Normal {
        /// Mean.
        mean: f64,
        /// Standard deviation.
        std_dev: f64,
    },
    /// Fixed value.
    Fixed {
        /// Constant value.
        value: f64,
    },
}

/// A single input-to-distribution binding.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BindingRequest {
    /// Name of the input node.
    pub node: String,
    /// Distribution to sample from.
    pub distribution: DistributionRequest,
}

/// Request body for `POST /simulate`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SimulateRequest {
    /// Nodes that make up the DAG.
    pub nodes: Vec<NodeRequest>,
    /// Directed edges between nodes.
    pub edges: Vec<EdgeRequest>,
    /// Input nodes to perturb, each with a distribution.
    pub bindings: Vec<BindingRequest>,
    /// Name of the node whose output should be aggregated.
    pub target: String,
    /// Number of universes to simulate.
    pub universe_count: usize,
    /// Optional RNG seed for reproducibility.
    pub seed: Option<u64>,
}

/// Response body for `POST /simulate`.
#[derive(Debug, Clone, Serialize)]
pub struct SimulateResponse {
    /// Number of universes simulated.
    pub count: usize,
    /// Mean of the target node.
    pub mean: Decimal,
    /// Median of the target node.
    pub median: Decimal,
    /// Minimum observed value.
    pub min: Decimal,
    /// Maximum observed value.
    pub max: Decimal,
}
