//! Error types for the Monte Carlo simulator.

use thiserror::Error;

use casiros_dag::DagError;

/// The universal error type for all simulator operations.
///
/// Every fallible simulator operation returns `Result<T, SimulationError>`.
/// No function in the simulator panics in business logic; all error paths are
/// enumerated here or wrapped from lower layers.
#[derive(Debug, Clone, Error, PartialEq)]
pub enum SimulationError {
    /// The requested number of universes is zero or would overflow a `usize`.
    #[error("Invalid universe count {count}: must be greater than 0")]
    InvalidUniverseCount {
        /// The invalid count provided.
        count: usize,
    },

    /// A sampled input value could not be represented as a `Decimal`.
    #[error("Sampled value {sample} cannot be represented as Decimal for input {input}")]
    InvalidSample {
        /// The input parameter name.
        input: String,
        /// The invalid sampled value.
        sample: f64,
    },

    /// The simulator was asked to run without any registered input samplers.
    #[error("No input samplers registered; at least one input must be sampled")]
    MissingSamplers,

    /// The target node was not found in the engine after evaluation.
    #[error("Target node {node:?} was not found in evaluation outputs")]
    MissingTarget {
        /// The missing target node identifier.
        node: casiros_dag::graph::NodeId,
    },

    /// A DAG evaluation failed in one of the simulated universes.
    #[error("Evaluation failed in universe {universe}: {source}")]
    EvaluationFailure {
        /// The zero-based universe index where the failure occurred.
        universe: usize,
        /// The underlying DAG error.
        source: DagError,
    },
}
