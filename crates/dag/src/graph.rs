//! Directed acyclic graph engine for CASIROS formulas.
//!
//! This module provides [`CausalityEngine`], a runtime graph of formula nodes
//! that can be topologically evaluated. Each node is either a raw numeric
//! [`Input`][`NodeKind::Input`] or a [`Formula`][`NodeKind::Formula`] from the
//! core catalog. Dependencies between nodes are declared with directed edges;
//! the engine rejects cycles and evaluates nodes in dependency order.

use std::collections::HashMap;

use casiros_core::prelude::{CalculationError, Decimal};
use petgraph::Direction;
use petgraph::algo::toposort;
use petgraph::graph::{DiGraph, NodeIndex};
use rust_decimal::prelude::ToPrimitive;

use crate::error::DagError;

/// Unique identifier for a node in the causality graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(pub usize);

/// A port binding: either a constant value or the output of another node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Port {
    /// A literal constant value.
    Constant(Decimal),
    /// The computed output of another node.
    Output(NodeId),
}

/// The kind of computation a node performs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NodeKind {
    /// A raw numeric input provided by the caller at evaluation time.
    Input,
    /// A formula from the CASIROS core catalog.
    Formula(FormulaKind),
}

/// Supported core formulas that can be used inside the DAG.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormulaKind {
    /// Future value: `FV = PV * (1 + r)^n`.
    FutureValue {
        /// Present value input port.
        present_value: Port,
        /// Periodic rate input port.
        rate: Port,
        /// Number of periods input port.
        periods: Port,
    },
    /// Present value: `PV = FV / (1 + r)^n`.
    PresentValue {
        /// Future value input port.
        future_value: Port,
        /// Periodic rate input port.
        rate: Port,
        /// Number of periods input port.
        periods: Port,
    },
    /// Return on equity: `ROE = Net Income / Equity`.
    ReturnOnEquity {
        /// Net income input port.
        net_income: Port,
        /// Shareholders' equity input port.
        equity: Port,
    },
    /// Weighted average cost of capital.
    Wacc {
        /// Total equity value.
        equity_value: Port,
        /// Total debt value.
        debt_value: Port,
        /// Cost of equity.
        cost_of_equity: Port,
        /// Cost of debt.
        cost_of_debt: Port,
        /// Tax rate.
        tax_rate: Port,
    },
    /// Sustainable growth rate: `SGR = ROE * (1 - Dividend Payout Ratio)`.
    SustainableGrowthRate {
        /// Return on equity input port.
        roe: Port,
        /// Dividend payout ratio input port.
        dividend_payout_ratio: Port,
    },
}

/// A node in the causality graph.
#[derive(Debug, Clone)]
pub struct Node {
    id: NodeId,
    name: String,
    kind: NodeKind,
}

impl Node {
    /// Creates a new node with the given identifier, name, and kind.
    ///
    /// # Examples
    ///
    /// ```
    /// use casiros_dag::graph::{Node, NodeId, NodeKind};
    ///
    /// let node = Node::new(NodeId(0), "principal", NodeKind::Input);
    /// assert_eq!(node.id(), NodeId(0));
    /// ```
    #[must_use]
    pub fn new(id: NodeId, name: impl Into<String>, kind: NodeKind) -> Self {
        return Self {
            id,
            name: name.into(),
            kind,
        };
    }

    /// Returns the node's identifier.
    #[must_use]
    pub fn id(&self) -> NodeId {
        return self.id;
    }

    /// Returns the node's human-readable name.
    #[must_use]
    pub fn name(&self) -> &str {
        return &self.name;
    }

    /// Returns the node's computational kind.
    #[must_use]
    pub fn kind(&self) -> &NodeKind {
        return &self.kind;
    }
}

/// Directed acyclic graph execution engine.
///
/// `CausalityEngine` stores formula nodes, their dependencies, and evaluates
/// them in topological order. The engine is immutable during evaluation:
/// graph construction requires `&mut self`, while evaluation requires only
/// `&self`.
#[derive(Debug, Default)]
pub struct CausalityEngine {
    graph: DiGraph<NodeId, ()>,
    nodes: HashMap<NodeId, Node>,
    indices: HashMap<NodeId, NodeIndex>,
    next_id: usize,
}

impl CausalityEngine {
    /// Creates a new, empty causality engine.
    ///
    /// # Examples
    ///
    /// ```
    /// use casiros_dag::graph::CausalityEngine;
    ///
    /// let engine = CausalityEngine::new();
    /// assert_eq!(engine.len(), 0);
    /// ```
    #[must_use]
    pub fn new() -> Self {
        return Self::default();
    }

    /// Returns the number of nodes currently in the graph.
    ///
    /// # Examples
    ///
    /// ```
    /// use casiros_dag::graph::CausalityEngine;
    ///
    /// let mut engine = CausalityEngine::new();
    /// engine.add_input("principal");
    /// assert_eq!(engine.len(), 1);
    /// ```
    #[must_use]
    pub fn len(&self) -> usize {
        return self.nodes.len();
    }

    /// Returns `true` if the graph contains no nodes.
    ///
    /// # Examples
    ///
    /// ```
    /// use casiros_dag::graph::CausalityEngine;
    ///
    /// let engine = CausalityEngine::new();
    /// assert!(engine.is_empty());
    /// ```
    #[must_use]
    pub fn is_empty(&self) -> bool {
        return self.nodes.is_empty();
    }

    /// Adds a new raw-input node and returns its identifier.
    ///
    /// # Examples
    ///
    /// ```
    /// use casiros_dag::graph::CausalityEngine;
    ///
    /// let mut engine = CausalityEngine::new();
    /// let principal = engine.add_input("principal");
    /// assert_eq!(engine.len(), 1);
    /// ```
    pub fn add_input(&mut self, name: impl Into<String>) -> NodeId {
        let id = self.next_id();
        let node = Node::new(id, name, NodeKind::Input);
        self.insert_node(id, node);
        return id;
    }

    /// Adds a new formula node and returns its identifier.
    ///
    /// # Examples
    ///
    /// ```
    /// use casiros_dag::graph::{CausalityEngine, FormulaKind, Port};
    /// use rust_decimal_macros::dec;
    ///
    /// let mut engine = CausalityEngine::new();
    /// let fv = engine.add_formula(
    ///     "future_value",
    ///     FormulaKind::FutureValue {
    ///         present_value: Port::Constant(dec!(100.0)),
    ///         rate: Port::Constant(dec!(0.05)),
    ///         periods: Port::Constant(dec!(10)),
    ///     },
    /// );
    /// assert_eq!(engine.len(), 1);
    /// ```
    pub fn add_formula(&mut self, name: impl Into<String>, formula: FormulaKind) -> NodeId {
        let id = self.next_id();
        let node = Node::new(id, name, NodeKind::Formula(formula));
        self.insert_node(id, node);
        return id;
    }

    /// Adds a dependency edge from `dependency` to `dependent`.
    ///
    /// The `dependent` node will be evaluated only after `dependency` has
    /// produced its output.
    ///
    /// # Errors
    ///
    /// Returns [`DagError::NodeNotFound`] if either endpoint does not exist.
    ///
    /// # Examples
    ///
    /// ```
    /// use casiros_dag::graph::{CausalityEngine, FormulaKind, Port};
    /// use rust_decimal_macros::dec;
    ///
    /// let mut engine = CausalityEngine::new();
    /// let principal = engine.add_input("principal");
    /// let rate = engine.add_input("rate");
    /// let fv = engine.add_formula(
    ///     "future_value",
    ///     FormulaKind::FutureValue {
    ///         present_value: Port::Output(principal),
    ///         rate: Port::Output(rate),
    ///         periods: Port::Constant(dec!(10)),
    ///     },
    /// );
    /// engine.add_edge(principal, fv).unwrap();
    /// engine.add_edge(rate, fv).unwrap();
    /// ```
    pub fn add_edge(&mut self, dependency: NodeId, dependent: NodeId) -> Result<(), DagError> {
        let dep_idx = self.index(dependency)?;
        let dst_idx = self.index(dependent)?;
        self.graph.add_edge(dep_idx, dst_idx, ());
        return Ok(());
    }

    /// Evaluates the graph and returns the computed value for every node.
    ///
    /// `inputs` must contain a value for every [`NodeKind::Input`] node; any
    /// missing input produces [`DagError::MissingInput`]. Formula nodes compute
    /// their values by resolving [`Port`] bindings against already-evaluated
    /// nodes.
    ///
    /// # Errors
    ///
    /// - [`DagError::CycleDetected`] if the graph contains a directed cycle.
    /// - [`DagError::MissingInput`] if an input node lacks a value.
    /// - [`DagError::MissingDependency`] if a port references an unevaluated node.
    /// - [`DagError::InvalidPeriod`] if a period port resolves to a non-integer.
    /// - [`DagError::FormulaEvaluation`] if a core formula returns an error.
    ///
    /// # Examples
    ///
    /// ```
    /// use casiros_dag::graph::{CausalityEngine, FormulaKind, Port};
    /// use rust_decimal_macros::dec;
    /// use std::collections::HashMap;
    ///
    /// let mut engine = CausalityEngine::new();
    /// let fv = engine.add_formula(
    ///     "future_value",
    ///     FormulaKind::FutureValue {
    ///         present_value: Port::Constant(dec!(100.0)),
    ///         rate: Port::Constant(dec!(0.05)),
    ///         periods: Port::Constant(dec!(10)),
    ///     },
    /// );
    /// let outputs = engine.evaluate(&HashMap::new()).unwrap();
    /// assert_eq!(outputs.get(&fv).unwrap().round_dp(4), dec!(162.8895));
    /// ```
    pub fn evaluate(
        &self,
        inputs: &HashMap<NodeId, Decimal>,
    ) -> Result<HashMap<NodeId, Decimal>, DagError> {
        let order = self.topological_order()?;
        let mut outputs: HashMap<NodeId, Decimal> = HashMap::with_capacity(self.nodes.len());

        for id in order {
            let node = self.nodes.get(&id).ok_or(DagError::NodeNotFound { id })?;
            let value: Decimal = match &node.kind {
                NodeKind::Input => *inputs.get(&id).ok_or(DagError::MissingInput { id })?,
                NodeKind::Formula(formula) => Self::evaluate_formula(formula, &outputs, id)?,
            };
            outputs.insert(id, value);
        }

        return Ok(outputs);
    }

    /// Returns the nodes in topological evaluation order.
    ///
    /// # Errors
    ///
    /// Returns [`DagError::CycleDetected`] if the graph contains a directed
    /// cycle.
    ///
    /// # Panics
    ///
    /// Panics only if the internal graph invariant is violated (every
    /// `NodeIndex` in the graph must have a `NodeId` weight).
    ///
    /// # Examples
    ///
    /// ```
    /// use casiros_dag::graph::CausalityEngine;
    ///
    /// let engine = CausalityEngine::new();
    /// assert!(engine.topological_order().unwrap().is_empty());
    /// ```
    pub fn topological_order(&self) -> Result<Vec<NodeId>, DagError> {
        let sorted = toposort(&self.graph, None).map_err(|cycle| DagError::CycleDetected {
            node: cycle.node_id().index(),
        })?;
        return Ok(sorted
            .into_iter()
            .map(|idx| {
                *self
                    .graph
                    .node_weight(idx)
                    .expect("internal graph invariant: every NodeIndex has a NodeId weight")
            })
            .collect());
    }

    /// Returns the length of the longest dependency chain in the graph.
    ///
    /// Input nodes have depth `1`. The depth of any formula node is `1 +` the
    /// maximum depth of its dependencies.
    ///
    /// # Errors
    ///
    /// Returns [`DagError::CycleDetected`] if the graph contains a directed
    /// cycle.
    ///
    /// # Panics
    ///
    /// Panics only if the internal graph invariant is violated.
    ///
    /// # Examples
    ///
    /// ```
    /// use casiros_dag::graph::{CausalityEngine, FormulaKind, Port};
    /// use rust_decimal_macros::dec;
    ///
    /// let mut engine = CausalityEngine::new();
    /// let a = engine.add_input("a");
    /// let b = engine.add_formula(
    ///     "b",
    ///     FormulaKind::ReturnOnEquity {
    ///         net_income: Port::Output(a),
    ///         equity: Port::Constant(dec!(100.0)),
    ///     },
    /// );
    /// let c = engine.add_formula(
    ///     "c",
    ///     FormulaKind::ReturnOnEquity {
    ///         net_income: Port::Output(b),
    ///         equity: Port::Constant(dec!(100.0)),
    ///     },
    /// );
    /// engine.add_edge(a, b).unwrap();
    /// engine.add_edge(b, c).unwrap();
    ///
    /// assert_eq!(engine.max_depth().unwrap(), 3);
    /// ```
    pub fn max_depth(&self) -> Result<usize, DagError> {
        let order = self.topological_order()?;
        let mut depth: HashMap<NodeId, usize> = HashMap::with_capacity(self.nodes.len());

        for id in order {
            let idx = self.index(id)?;
            let mut max_dependency_depth = 0;
            for neighbor in self.graph.neighbors_directed(idx, Direction::Incoming) {
                let dependency_id = *self
                    .graph
                    .node_weight(neighbor)
                    .expect("internal graph invariant: every NodeIndex has a NodeId weight");
                let dependency_depth = depth.get(&dependency_id).copied().unwrap_or(1);
                max_dependency_depth = max_dependency_depth.max(dependency_depth);
            }
            depth.insert(id, max_dependency_depth + 1);
        }

        return Ok(depth.values().copied().max().unwrap_or(0));
    }

    fn next_id(&mut self) -> NodeId {
        let id = NodeId(self.next_id);
        self.next_id += 1;
        return id;
    }

    fn insert_node(&mut self, id: NodeId, node: Node) {
        let idx = self.graph.add_node(id);
        self.nodes.insert(id, node);
        self.indices.insert(id, idx);
    }

    fn index(&self, id: NodeId) -> Result<NodeIndex, DagError> {
        return self
            .indices
            .get(&id)
            .copied()
            .ok_or(DagError::NodeNotFound { id });
    }

    fn resolve_port(port: &Port, outputs: &HashMap<NodeId, Decimal>) -> Result<Decimal, DagError> {
        match *port {
            Port::Constant(value) => Ok(value),
            Port::Output(id) => outputs
                .get(&id)
                .copied()
                .ok_or(DagError::MissingDependency { id }),
        }
    }

    fn evaluate_formula(
        formula: &FormulaKind,
        outputs: &HashMap<NodeId, Decimal>,
        node_id: NodeId,
    ) -> Result<Decimal, DagError> {
        match formula {
            FormulaKind::FutureValue {
                present_value,
                rate,
                periods,
            } => Self::eval_future_value(present_value, rate, periods, outputs, node_id),
            FormulaKind::PresentValue {
                future_value,
                rate,
                periods,
            } => Self::eval_present_value(future_value, rate, periods, outputs, node_id),
            FormulaKind::ReturnOnEquity { net_income, equity } => {
                Self::eval_return_on_equity(net_income, equity, outputs, node_id)
            }
            FormulaKind::Wacc {
                equity_value,
                debt_value,
                cost_of_equity,
                cost_of_debt,
                tax_rate,
            } => Self::eval_wacc(
                equity_value,
                debt_value,
                cost_of_equity,
                cost_of_debt,
                tax_rate,
                outputs,
                node_id,
            ),
            FormulaKind::SustainableGrowthRate {
                roe,
                dividend_payout_ratio,
            } => Self::eval_sustainable_growth_rate(roe, dividend_payout_ratio, outputs, node_id),
        }
    }

    fn eval_future_value(
        present_value: &Port,
        rate: &Port,
        periods: &Port,
        outputs: &HashMap<NodeId, Decimal>,
        node_id: NodeId,
    ) -> Result<Decimal, DagError> {
        let pv = Self::resolve_port(present_value, outputs)?;
        let r = Self::resolve_port(rate, outputs)?;
        let n = Self::resolve_period(periods, outputs)?;
        return casiros_core::general::future_value(pv, r, n)
            .map_err(|err| Self::wrap_formula_error(node_id, err));
    }

    fn eval_present_value(
        future_value: &Port,
        rate: &Port,
        periods: &Port,
        outputs: &HashMap<NodeId, Decimal>,
        node_id: NodeId,
    ) -> Result<Decimal, DagError> {
        let fv = Self::resolve_port(future_value, outputs)?;
        let r = Self::resolve_port(rate, outputs)?;
        let n = Self::resolve_period(periods, outputs)?;
        return casiros_core::general::present_value(fv, r, n)
            .map_err(|err| Self::wrap_formula_error(node_id, err));
    }

    fn eval_return_on_equity(
        net_income: &Port,
        equity: &Port,
        outputs: &HashMap<NodeId, Decimal>,
        node_id: NodeId,
    ) -> Result<Decimal, DagError> {
        let ni = Self::resolve_port(net_income, outputs)?;
        let eq = Self::resolve_port(equity, outputs)?;
        return casiros_core::financial::return_on_equity(ni, eq)
            .map_err(|err| Self::wrap_formula_error(node_id, err));
    }

    fn eval_wacc(
        equity_value: &Port,
        debt_value: &Port,
        cost_of_equity: &Port,
        cost_of_debt: &Port,
        tax_rate: &Port,
        outputs: &HashMap<NodeId, Decimal>,
        node_id: NodeId,
    ) -> Result<Decimal, DagError> {
        let e = Self::resolve_port(equity_value, outputs)?;
        let d = Self::resolve_port(debt_value, outputs)?;
        let re = Self::resolve_port(cost_of_equity, outputs)?;
        let rd = Self::resolve_port(cost_of_debt, outputs)?;
        let t = Self::resolve_port(tax_rate, outputs)?;
        return casiros_core::corporate::wacc(e, d, re, rd, t)
            .map_err(|err| Self::wrap_formula_error(node_id, err));
    }

    fn eval_sustainable_growth_rate(
        roe: &Port,
        dividend_payout_ratio: &Port,
        outputs: &HashMap<NodeId, Decimal>,
        node_id: NodeId,
    ) -> Result<Decimal, DagError> {
        let r = Self::resolve_port(roe, outputs)?;
        let payout = Self::resolve_port(dividend_payout_ratio, outputs)?;
        return casiros_core::corporate::sustainable_growth_rate(r, payout)
            .map_err(|err| Self::wrap_formula_error(node_id, err));
    }

    fn resolve_period(
        port: &Port,
        outputs: &HashMap<NodeId, Decimal>,
    ) -> Result<casiros_core::prelude::Periods, DagError> {
        let value = Self::resolve_port(port, outputs)?;
        return value.to_u32().ok_or(DagError::InvalidPeriod { value });
    }

    fn wrap_formula_error(node_id: NodeId, err: CalculationError) -> DagError {
        return DagError::FormulaEvaluation {
            node: node_id,
            source: err,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn new_engine_is_empty() {
        let engine = CausalityEngine::new();
        assert!(engine.is_empty());
        assert_eq!(engine.len(), 0);
    }

    #[test]
    fn add_input_increments_len() {
        let mut engine = CausalityEngine::new();
        let id = engine.add_input("x");
        assert_eq!(engine.len(), 1);
        assert_eq!(id, NodeId(0));
    }

    #[test]
    fn add_formula_increments_len() {
        let mut engine = CausalityEngine::new();
        let id = engine.add_formula(
            "fv",
            FormulaKind::FutureValue {
                present_value: Port::Constant(dec!(100.0)),
                rate: Port::Constant(dec!(0.05)),
                periods: Port::Constant(dec!(10)),
            },
        );
        assert_eq!(engine.len(), 1);
        assert_eq!(id, NodeId(0));
    }

    #[test]
    fn topological_order_empty() {
        let engine = CausalityEngine::new();
        let order = engine.topological_order().unwrap();
        assert!(order.is_empty());
    }

    #[test]
    fn topological_order_chain() {
        let mut engine = CausalityEngine::new();
        let a = engine.add_input("a");
        let b = engine.add_input("b");
        let c = engine.add_formula(
            "c",
            FormulaKind::FutureValue {
                present_value: Port::Output(a),
                rate: Port::Output(b),
                periods: Port::Constant(dec!(1)),
            },
        );
        engine.add_edge(a, c).unwrap();
        engine.add_edge(b, c).unwrap();

        let order = engine.topological_order().unwrap();
        let c_pos = order.iter().position(|id| *id == c).unwrap();
        let a_pos = order.iter().position(|id| *id == a).unwrap();
        let b_pos = order.iter().position(|id| *id == b).unwrap();
        assert!(c_pos > a_pos);
        assert!(c_pos > b_pos);
    }

    #[test]
    fn detects_cycle() {
        let mut engine = CausalityEngine::new();
        let a = engine.add_input("a");
        let b = engine.add_formula(
            "b",
            FormulaKind::ReturnOnEquity {
                net_income: Port::Output(a),
                equity: Port::Constant(dec!(100.0)),
            },
        );
        let c = engine.add_formula(
            "c",
            FormulaKind::ReturnOnEquity {
                net_income: Port::Output(b),
                equity: Port::Constant(dec!(100.0)),
            },
        );
        engine.add_edge(a, b).unwrap();
        engine.add_edge(b, c).unwrap();
        engine.add_edge(c, b).unwrap();

        assert!(matches!(
            engine.topological_order(),
            Err(DagError::CycleDetected { .. })
        ));
    }

    #[test]
    fn evaluates_future_value_with_constants() {
        let mut engine = CausalityEngine::new();
        let fv = engine.add_formula(
            "fv",
            FormulaKind::FutureValue {
                present_value: Port::Constant(dec!(100.0)),
                rate: Port::Constant(dec!(0.05)),
                periods: Port::Constant(dec!(10)),
            },
        );
        let outputs = engine.evaluate(&HashMap::new()).unwrap();
        assert_eq!(outputs[&fv].round_dp(4), dec!(162.8895));
    }

    #[test]
    fn evaluates_chained_sustainable_growth_rate() {
        let mut engine = CausalityEngine::new();
        let net_income = engine.add_input("net_income");
        let equity = engine.add_input("equity");
        let payout = engine.add_input("payout");

        let roe = engine.add_formula(
            "roe",
            FormulaKind::ReturnOnEquity {
                net_income: Port::Output(net_income),
                equity: Port::Output(equity),
            },
        );
        let sgr = engine.add_formula(
            "sgr",
            FormulaKind::SustainableGrowthRate {
                roe: Port::Output(roe),
                dividend_payout_ratio: Port::Output(payout),
            },
        );

        engine.add_edge(net_income, roe).unwrap();
        engine.add_edge(equity, roe).unwrap();
        engine.add_edge(roe, sgr).unwrap();
        engine.add_edge(payout, sgr).unwrap();

        let mut inputs = HashMap::new();
        inputs.insert(net_income, dec!(150.0));
        inputs.insert(equity, dec!(1000.0));
        inputs.insert(payout, dec!(0.40));

        let outputs = engine.evaluate(&inputs).unwrap();
        assert_eq!(outputs[&roe], dec!(0.15));
        assert_eq!(outputs[&sgr], dec!(0.09));
    }

    #[test]
    fn missing_input_returns_error() {
        let mut engine = CausalityEngine::new();
        let principal = engine.add_input("principal");
        let fv = engine.add_formula(
            "fv",
            FormulaKind::FutureValue {
                present_value: Port::Output(principal),
                rate: Port::Constant(dec!(0.05)),
                periods: Port::Constant(dec!(10)),
            },
        );
        engine.add_edge(principal, fv).unwrap();

        assert!(matches!(
            engine.evaluate(&HashMap::new()),
            Err(DagError::MissingInput { id }) if id == principal
        ));
    }
}
