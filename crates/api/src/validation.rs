//! Request validation and security limits for the CASIROS HTTP API.
//!
//! All public endpoints that accept user-defined graphs enforce hard limits on
//! node count, edge count, simulation size, and graph depth. These bounds defend
//! against accidental or malicious requests that would consume excessive CPU,
//! memory, or wall-clock time.

use crate::models::{AmortizationScheduleRequest, EvaluateRequest, SimulateRequest};

/// Maximum number of nodes allowed in a single request graph.
pub const MAX_NODES: usize = 100;

/// Maximum number of edges allowed in a single request graph.
pub const MAX_EDGES: usize = 500;

/// Maximum depth (longest dependency chain) allowed in a request graph.
pub const MAX_DEPTH: usize = 50;

/// Maximum number of universes allowed in a single simulation request.
pub const MAX_UNIVERSE_COUNT: usize = 100_000;

/// Maximum number of input-to-distribution bindings allowed in a simulation.
pub const MAX_BINDINGS: usize = 50;

/// Maximum number of periods allowed in an amortization schedule.
///
/// Matches the core crate's own ceiling. Duplicated here so the caller gets
/// "period count 5000 exceeds maximum 1000" rather than the bare `Overflow`
/// the core function raises, which says nothing about what to change.
pub const MAX_SCHEDULE_PERIODS: u32 = 1_000;

/// Errors returned when a request violates security limits.
#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum ValidationError {
    /// Too many nodes in the graph.
    #[error("Too many nodes: {count} (max {MAX_NODES})")]
    TooManyNodes {
        /// The number of nodes provided.
        count: usize,
    },

    /// Too many edges in the graph.
    #[error("Too many edges: {count} (max {MAX_EDGES})")]
    TooManyEdges {
        /// The number of edges provided.
        count: usize,
    },

    /// Graph depth exceeds the configured limit.
    #[error("Graph depth {depth} exceeds maximum {MAX_DEPTH}")]
    DepthExceeded {
        /// The computed depth.
        depth: usize,
    },

    /// Too many universes requested.
    #[error("Universe count {count} exceeds maximum {MAX_UNIVERSE_COUNT}")]
    TooManyUniverses {
        /// The requested universe count.
        count: usize,
    },

    /// Too many input bindings requested.
    #[error("Too many bindings: {count} (max {MAX_BINDINGS})")]
    TooManyBindings {
        /// The number of bindings provided.
        count: usize,
    },

    /// The number of bindings exceeds the number of input nodes.
    #[error("Bindings count {bindings} exceeds input node count {inputs}")]
    BindingsExceedInputs {
        /// Number of bindings.
        bindings: usize,
        /// Number of input nodes.
        inputs: usize,
    },

    /// The graph contains a directed cycle.
    #[error("Graph contains a directed cycle")]
    Cycle,

    /// Too many periods requested for an amortization schedule.
    #[error("Period count {count} exceeds maximum {MAX_SCHEDULE_PERIODS}")]
    TooManySchedulePeriods {
        /// The requested period count.
        count: u32,
    },
}

/// Validates an evaluate request before any engine construction.
///
/// # Errors
///
/// Returns [`ValidationError`] if any request limit is exceeded.
pub fn validate_evaluate(request: &EvaluateRequest) -> Result<(), ValidationError> {
    if request.nodes.len() > MAX_NODES {
        return Err(ValidationError::TooManyNodes {
            count: request.nodes.len(),
        });
    }
    if request.edges.len() > MAX_EDGES {
        return Err(ValidationError::TooManyEdges {
            count: request.edges.len(),
        });
    }
    return Ok(());
}

/// Validates an amortization schedule request.
///
/// A schedule allocates one row per period, so the period count is the only
/// thing a caller can use to make the response arbitrarily large.
///
/// # Errors
///
/// Returns [`ValidationError::TooManySchedulePeriods`] if the request would
/// generate more than [`MAX_SCHEDULE_PERIODS`] rows.
pub fn validate_amortization_schedule(
    request: &AmortizationScheduleRequest,
) -> Result<(), ValidationError> {
    if request.periods > MAX_SCHEDULE_PERIODS {
        return Err(ValidationError::TooManySchedulePeriods {
            count: request.periods,
        });
    }
    return Ok(());
}

/// Validates a simulate request before any engine construction.
///
/// # Errors
///
/// Returns [`ValidationError`] if any request limit is exceeded.
pub fn validate_simulate(request: &SimulateRequest) -> Result<(), ValidationError> {
    validate_evaluate(&EvaluateRequest {
        nodes: request.nodes.clone(),
        edges: request.edges.clone(),
        inputs: std::collections::HashMap::new(),
    })?;

    if request.universe_count > MAX_UNIVERSE_COUNT {
        return Err(ValidationError::TooManyUniverses {
            count: request.universe_count,
        });
    }
    if request.bindings.len() > MAX_BINDINGS {
        return Err(ValidationError::TooManyBindings {
            count: request.bindings.len(),
        });
    }

    let input_count = request
        .nodes
        .iter()
        .filter(|node| matches!(node, crate::models::NodeRequest::Input { .. }))
        .count();
    if request.bindings.len() > input_count {
        return Err(ValidationError::BindingsExceedInputs {
            bindings: request.bindings.len(),
            inputs: input_count,
        });
    }

    return Ok(());
}

/// Validates the depth of a constructed engine.
///
/// # Errors
///
/// Returns [`ValidationError::DepthExceeded`] if the computed depth exceeds
/// [`MAX_DEPTH`], or [`ValidationError::Cycle`] if topological ordering fails.
pub fn validate_depth(engine: &casiros_dag::graph::CausalityEngine) -> Result<(), ValidationError> {
    let depth = engine.max_depth().map_err(|_| ValidationError::Cycle)?;
    if depth > MAX_DEPTH {
        return Err(ValidationError::DepthExceeded { depth });
    }
    return Ok(());
}
