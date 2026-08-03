//! Build a [`casiros_dag::graph::CausalityEngine`] from JSON request models.
//!
//! This module translates the flat, serialization-friendly API types into the
//! richer graph types used by the application layer. All errors are reported as
//! human-readable strings suitable for HTTP 400 responses.

use std::collections::HashMap;

use casiros_dag::graph::{CausalityEngine, FormulaKind, NodeId, Port};
use rust_decimal::Decimal;

use crate::models::{DistributionRequest, EdgeRequest, FormulaRequest, NodeRequest, PortRequest};

/// Errors that can occur while translating an API request into a causality
/// engine.
#[derive(Debug, Clone, thiserror::Error)]
pub enum EngineBuilderError {
    /// A node name was referenced but not defined.
    #[error("Unknown node '{0}'")]
    UnknownNode(String),

    /// A node name was defined more than once.
    #[error("Duplicate node name '{0}'")]
    DuplicateNode(String),

    /// An edge referenced an unknown node.
    #[error("Edge references unknown node: {0}")]
    UnknownEdgeNode(String),

    /// A port referenced an unknown node.
    #[error("Port references unknown node: {0}")]
    UnknownPortNode(String),

    /// A DAG topology error occurred (for example, a cycle).
    #[error("Graph error: {0}")]
    GraphError(String),
}

/// Builder that accumulates API request pieces and produces a validated
/// [`CausalityEngine`].
#[derive(Debug, Default)]
pub struct EngineBuilder {
    engine: CausalityEngine,
    name_to_id: HashMap<String, NodeId>,
}

impl EngineBuilder {
    /// Creates a new empty builder.
    #[must_use]
    pub fn new() -> Self {
        return Self::default();
    }

    /// Adds all nodes from a request and returns a mapping from node name to
    /// identifier.
    ///
    /// # Errors
    ///
    /// Returns [`EngineBuilderError::DuplicateNode`] if a name is reused.
    pub fn add_nodes(
        &mut self,
        nodes: &[NodeRequest],
    ) -> Result<&HashMap<String, NodeId>, EngineBuilderError> {
        for node in nodes {
            let (name, kind) = match node {
                NodeRequest::Input { name } => (name.clone(), casiros_dag::graph::NodeKind::Input),
                NodeRequest::Formula { name, kind } => (
                    name.clone(),
                    casiros_dag::graph::NodeKind::Formula(self.formula_kind(kind)?),
                ),
            };

            if self.name_to_id.contains_key(&name) {
                return Err(EngineBuilderError::DuplicateNode(name));
            }

            let id = match kind {
                casiros_dag::graph::NodeKind::Input => self.engine.add_input(name.clone()),
                casiros_dag::graph::NodeKind::Formula(formula) => {
                    self.engine.add_formula(name.clone(), formula)
                }
            };
            self.name_to_id.insert(name, id);
        }
        return Ok(&self.name_to_id);
    }

    /// Adds all edges from a request.
    ///
    /// # Errors
    ///
    /// Returns [`EngineBuilderError::UnknownEdgeNode`] if an endpoint is
    /// missing, or [`EngineBuilderError::GraphError`] if the engine rejects the
    /// edge.
    pub fn add_edges(&mut self, edges: &[EdgeRequest]) -> Result<(), EngineBuilderError> {
        for edge in edges {
            let dependency = self
                .name_to_id
                .get(&edge.dependency)
                .copied()
                .ok_or_else(|| EngineBuilderError::UnknownEdgeNode(edge.dependency.clone()))?;
            let dependent = self
                .name_to_id
                .get(&edge.dependent)
                .copied()
                .ok_or_else(|| EngineBuilderError::UnknownEdgeNode(edge.dependent.clone()))?;

            self.engine
                .add_edge(dependency, dependent)
                .map_err(|err| EngineBuilderError::GraphError(err.to_string()))?;
        }
        return Ok(());
    }

    /// Consumes the builder and returns the constructed engine.
    #[must_use]
    pub fn build(self) -> CausalityEngine {
        return self.engine;
    }

    /// Returns the node identifier for a given name, if known.
    #[must_use]
    pub fn node_id(&self, name: &str) -> Option<NodeId> {
        return self.name_to_id.get(name).copied();
    }

    fn formula_kind(&self, request: &FormulaRequest) -> Result<FormulaKind, EngineBuilderError> {
        match request {
            FormulaRequest::FutureValue {
                present_value,
                rate,
                periods,
            } => Ok(FormulaKind::FutureValue {
                present_value: self.port(present_value)?,
                rate: self.port(rate)?,
                periods: self.port(periods)?,
            }),
            FormulaRequest::PresentValue {
                future_value,
                rate,
                periods,
            } => Ok(FormulaKind::PresentValue {
                future_value: self.port(future_value)?,
                rate: self.port(rate)?,
                periods: self.port(periods)?,
            }),
            FormulaRequest::ReturnOnEquity { net_income, equity } => {
                Ok(FormulaKind::ReturnOnEquity {
                    net_income: self.port(net_income)?,
                    equity: self.port(equity)?,
                })
            }
            FormulaRequest::Wacc {
                equity_value,
                debt_value,
                cost_of_equity,
                cost_of_debt,
                tax_rate,
            } => Ok(FormulaKind::Wacc {
                equity_value: self.port(equity_value)?,
                debt_value: self.port(debt_value)?,
                cost_of_equity: self.port(cost_of_equity)?,
                cost_of_debt: self.port(cost_of_debt)?,
                tax_rate: self.port(tax_rate)?,
            }),
            FormulaRequest::SustainableGrowthRate {
                roe,
                dividend_payout_ratio,
            } => Ok(FormulaKind::SustainableGrowthRate {
                roe: self.port(roe)?,
                dividend_payout_ratio: self.port(dividend_payout_ratio)?,
            }),
        }
    }

    fn port(&self, request: &PortRequest) -> Result<Port, EngineBuilderError> {
        match request {
            PortRequest::Constant(value) => Ok(Port::Constant(*value)),
            PortRequest::Output { node } => {
                let id = self
                    .name_to_id
                    .get(node)
                    .copied()
                    .ok_or_else(|| EngineBuilderError::UnknownPortNode(node.clone()))?;
                return Ok(Port::Output(id));
            }
        }
    }
}

/// Converts an API distribution request into a [`casiros_simulator::Distribution`].
///
/// # Examples
///
/// ```
/// use casiros_api::engine_builder::distribution_from_request;
/// use casiros_api::models::DistributionRequest;
///
/// let dist = distribution_from_request(
///     &DistributionRequest::Uniform { low: 0.0, high: 1.0 },
/// );
/// assert!(matches!(dist, casiros_simulator::Distribution::Uniform { .. }));
/// ```
#[must_use]
pub fn distribution_from_request(request: &DistributionRequest) -> casiros_simulator::Distribution {
    match *request {
        DistributionRequest::Uniform { low, high } => {
            casiros_simulator::Distribution::Uniform { low, high }
        }
        DistributionRequest::Normal { mean, std_dev } => {
            casiros_simulator::Distribution::Normal { mean, std_dev }
        }
        DistributionRequest::Fixed { value } => casiros_simulator::Distribution::Fixed { value },
    }
}

/// Translates a name-keyed input map into a [`NodeId`]-keyed map for engine
/// evaluation.
///
/// # Errors
///
/// Returns [`EngineBuilderError::UnknownNode`] if a provided input name does
/// not match a node in the engine.
pub fn map_inputs_by_id<S>(
    builder: &EngineBuilder,
    inputs: &HashMap<String, Decimal, S>,
) -> Result<HashMap<NodeId, Decimal>, EngineBuilderError>
where
    S: std::hash::BuildHasher,
{
    let mut by_id = HashMap::with_capacity(inputs.len());
    for (name, value) in inputs {
        let id = builder
            .node_id(name)
            .ok_or_else(|| EngineBuilderError::UnknownNode(name.clone()))?;
        by_id.insert(id, *value);
    }
    return Ok(by_id);
}
