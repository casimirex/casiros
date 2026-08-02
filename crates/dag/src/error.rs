//! Error types for the causality graph engine.

use rust_decimal::Decimal;
use thiserror::Error;

use crate::graph::NodeId;

/// The universal error type for all [`crate::graph::CausalityEngine`] operations.
///
/// Every fallible DAG operation returns `Result<T, DagError>`. No function in
/// the graph engine panics in business logic; all error paths are enumerated
/// here or wrapped from [`casiros_core::prelude::CalculationError`].
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum DagError {
    /// A referenced node does not exist in the graph.
    #[error("Node {id:?} not found in the causality graph")]
    NodeNotFound {
        /// The missing node identifier.
        id: NodeId,
    },

    /// The graph contains a directed cycle and cannot be topologically sorted.
    #[error("Cyclic dependency detected in the causality graph at node index {node}")]
    CycleDetected {
        /// The node index where petgraph detected the cycle.
        node: usize,
    },

    /// An input node was not supplied with a value during evaluation.
    #[error("Missing required input value for node {id:?}")]
    MissingInput {
        /// The input node that was not provided.
        id: NodeId,
    },

    /// A formula depends on the output of a node that has not been evaluated.
    #[error("Dependency node {id:?} has not been evaluated")]
    MissingDependency {
        /// The unevaluated dependency.
        id: NodeId,
    },

    /// A constant used as a period count cannot be converted to `u32`.
    #[error("Period value {value} is not a valid non-negative integer")]
    InvalidPeriod {
        /// The invalid period value.
        value: Decimal,
    },

    /// A core formula computation failed at a specific node.
    #[error("Formula evaluation failed at node {node:?}: {source}")]
    FormulaEvaluation {
        /// The node where the failure occurred.
        node: NodeId,
        /// The underlying core computation error.
        source: casiros_core::prelude::CalculationError,
    },
}
