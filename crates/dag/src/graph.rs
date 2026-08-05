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
use serde::{Deserialize, Serialize};

use crate::error::DagError;

/// Unique identifier for a node in the causality graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NodeId(pub usize);

/// A port binding: either a constant value or the output of another node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Port {
    /// A literal constant value.
    Constant(Decimal),
    /// The computed output of another node.
    Output(NodeId),
}

/// The kind of computation a node performs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    /// A raw numeric input provided by the caller at evaluation time.
    Input,
    /// A formula from the CASIROS core catalog.
    Formula(FormulaKind),
}

/// Side of an option contract used inside formula variants that depend on style.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OptionStyle {
    /// A call option gives the holder the right to buy the underlying.
    Call,
    /// A put option gives the holder the right to sell the underlying.
    Put,
}

/// Supported core formulas that can be used inside the DAG.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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

    /// Amortization payment: `PMT = P * r * (1 + r)^n / ((1 + r)^n - 1)`.
    AmortizationPayment {
        /// Loan principal input port.
        principal: Port,
        /// Periodic interest rate input port.
        rate: Port,
        /// Number of periods input port.
        periods: Port,
    },

    /// Yield-to-maturity approximation for a fixed-coupon bond.
    YieldToMaturityApproximation {
        /// Face value input port.
        face_value: Port,
        /// Periodic coupon payment input port.
        coupon_payment: Port,
        /// Current market price input port.
        price: Port,
        /// Periods to maturity input port.
        periods: Port,
    },

    /// Simple moving average over a price series.
    SimpleMovingAverage {
        /// Price series input port (single vector-valued input).
        prices: Port,
        /// Window size input port.
        window: Port,
    },

    /// Black-Scholes European call option price.
    BlackScholesCall {
        /// Current spot price input port.
        spot: Port,
        /// Option strike price input port.
        strike: Port,
        /// Risk-free rate input port.
        risk_free_rate: Port,
        /// Volatility input port.
        volatility: Port,
        /// Time to maturity input port.
        time_to_maturity: Port,
    },

    /// Black-Scholes European put option price.
    BlackScholesPut {
        /// Current spot price input port.
        spot: Port,
        /// Option strike price input port.
        strike: Port,
        /// Risk-free rate input port.
        risk_free_rate: Port,
        /// Volatility input port.
        volatility: Port,
        /// Time to maturity input port.
        time_to_maturity: Port,
    },

    /// Cox-Ross-Rubinstein binomial tree European call option price.
    BinomialOptionCall {
        /// Current spot price input port.
        spot: Port,
        /// Option strike price input port.
        strike: Port,
        /// Risk-free rate input port.
        risk_free_rate: Port,
        /// Volatility input port.
        volatility: Port,
        /// Time to maturity input port.
        time_to_maturity: Port,
        /// Number of time-steps input port.
        steps: Port,
    },

    /// Cox-Ross-Rubinstein binomial tree European put option price.
    BinomialOptionPut {
        /// Current spot price input port.
        spot: Port,
        /// Option strike price input port.
        strike: Port,
        /// Risk-free rate input port.
        risk_free_rate: Port,
        /// Volatility input port.
        volatility: Port,
        /// Time to maturity input port.
        time_to_maturity: Port,
        /// Number of time-steps input port.
        steps: Port,
    },

    /// Black-Scholes delta of a European option.
    BlackScholesDelta {
        /// Current spot price input port.
        spot: Port,
        /// Option strike price input port.
        strike: Port,
        /// Risk-free rate input port.
        risk_free_rate: Port,
        /// Volatility input port.
        volatility: Port,
        /// Time to maturity input port.
        time_to_maturity: Port,
        /// Option style.
        style: OptionStyle,
    },

    /// Black-Scholes gamma of a European option.
    BlackScholesGamma {
        /// Current spot price input port.
        spot: Port,
        /// Option strike price input port.
        strike: Port,
        /// Risk-free rate input port.
        risk_free_rate: Port,
        /// Volatility input port.
        volatility: Port,
        /// Time to maturity input port.
        time_to_maturity: Port,
    },

    /// Black-Scholes vega of a European option.
    BlackScholesVega {
        /// Current spot price input port.
        spot: Port,
        /// Option strike price input port.
        strike: Port,
        /// Risk-free rate input port.
        risk_free_rate: Port,
        /// Volatility input port.
        volatility: Port,
        /// Time to maturity input port.
        time_to_maturity: Port,
    },

    /// Black-Scholes theta of a European option.
    BlackScholesTheta {
        /// Current spot price input port.
        spot: Port,
        /// Option strike price input port.
        strike: Port,
        /// Risk-free rate input port.
        risk_free_rate: Port,
        /// Volatility input port.
        volatility: Port,
        /// Time to maturity input port.
        time_to_maturity: Port,
        /// Option style.
        style: OptionStyle,
    },

    /// Black-Scholes rho of a European option.
    BlackScholesRho {
        /// Current spot price input port.
        spot: Port,
        /// Option strike price input port.
        strike: Port,
        /// Risk-free rate input port.
        risk_free_rate: Port,
        /// Volatility input port.
        volatility: Port,
        /// Time to maturity input port.
        time_to_maturity: Port,
        /// Option style.
        style: OptionStyle,
    },

    /// Present value of a growing perpetuity.
    GrowingPerpetuityPresentValue {
        /// Periodic payment input port.
        payment: Port,
        /// Discount rate input port.
        rate: Port,
        /// Perpetual growth rate input port.
        growth_rate: Port,
    },

    /// Future value with continuous compounding.
    ContinuousCompoundingFutureValue {
        /// Present value input port.
        present_value: Port,
        /// Continuous rate input port.
        rate: Port,
        /// Time horizon input port.
        time: Port,
    },

    /// Return on investment.
    ReturnOnInvestment {
        /// Gain input port.
        gain: Port,
        /// Cost input port.
        cost: Port,
    },

    /// Profit margin.
    ProfitMargin {
        /// Net income input port.
        net_income: Port,
        /// Revenue input port.
        revenue: Port,
    },

    /// Asset turnover.
    AssetTurnover {
        /// Revenue input port.
        revenue: Port,
        /// Total assets input port.
        total_assets: Port,
    },

    /// Equity multiplier.
    EquityMultiplier {
        /// Total assets input port.
        total_assets: Port,
        /// Shareholders' equity input port.
        shareholders_equity: Port,
    },

    /// Quick ratio.
    QuickRatio {
        /// Current assets input port.
        current_assets: Port,
        /// Inventory input port.
        inventory: Port,
        /// Current liabilities input port.
        current_liabilities: Port,
    },

    /// Interest coverage ratio.
    InterestCoverage {
        /// EBIT input port.
        ebit: Port,
        /// Interest expense input port.
        interest_expense: Port,
    },

    /// Inventory turnover.
    InventoryTurnover {
        /// Cost of goods sold input port.
        cogs: Port,
        /// Inventory input port.
        inventory: Port,
    },

    /// Cash conversion cycle.
    CashConversionCycle {
        /// Days inventory outstanding input port.
        days_inventory_outstanding: Port,
        /// Days sales outstanding input port.
        days_sales_outstanding: Port,
        /// Days payables outstanding input port.
        days_payables_outstanding: Port,
    },

    /// Capital adequacy ratio.
    CapitalAdequacyRatio {
        /// Total capital input port.
        total_capital: Port,
        /// Risk-weighted assets input port.
        risk_weighted_assets: Port,
    },

    /// Provision coverage ratio.
    ProvisionCoverageRatio {
        /// Provisions input port.
        provisions: Port,
        /// Non-performing assets input port.
        non_performing_assets: Port,
    },

    /// Treynor ratio.
    TreynorRatio {
        /// Portfolio return input port.
        portfolio_return: Port,
        /// Risk-free rate input port.
        risk_free_rate: Port,
        /// Beta input port.
        beta: Port,
    },

    /// Value at Risk.
    ValueAtRisk {
        /// Portfolio value input port.
        portfolio_value: Port,
        /// Mean return input port.
        mean_return: Port,
        /// Standard deviation input port.
        std_dev: Port,
        /// Z-score input port.
        z_score: Port,
    },

    /// Expected shortfall.
    ExpectedShortfall {
        /// Portfolio value input port.
        portfolio_value: Port,
        /// Mean return input port.
        mean_return: Port,
        /// Standard deviation input port.
        std_dev: Port,
        /// Z-score input port.
        z_score: Port,
    },

    /// Discounted cash flow over a comma-separated cash-flow series.
    DiscountedCashFlow {
        /// Comma-separated cash-flow series input port.
        cash_flows: Port,
        /// Discount rate input port.
        discount_rate: Port,
    },

    /// Macaulay duration over a comma-separated cash-flow series.
    MacaulayDuration {
        /// Comma-separated cash-flow series input port.
        cash_flows: Port,
        /// Yield per period input port.
        yield_per_period: Port,
    },

    /// Modified duration from Macaulay duration.
    ModifiedDuration {
        /// Macaulay duration input port.
        macaulay_duration: Port,
        /// Yield per period input port.
        yield_per_period: Port,
    },

    /// Convexity over a comma-separated cash-flow series.
    Convexity {
        /// Comma-separated cash-flow series input port.
        cash_flows: Port,
        /// Yield per period input port.
        yield_per_period: Port,
    },

    /// Free cash flow to equity.
    FreeCashFlowToEquity {
        /// Free cash flow to firm input port.
        fcff: Port,
        /// Interest expense after tax input port.
        interest_expense_after_tax: Port,
        /// Net borrowing input port.
        net_borrowing: Port,
    },

    /// Economic value added.
    EconomicValueAdded {
        /// NOPAT input port.
        nopat: Port,
        /// Invested capital input port.
        invested_capital: Port,
        /// WACC input port.
        wacc: Port,
    },

    /// Internal growth rate.
    InternalGrowthRate {
        /// ROE input port.
        roe: Port,
        /// Dividend payout ratio input port.
        dividend_payout_ratio: Port,
    },

    /// Beta coefficient — systematic risk.
    Beta {
        /// Asset returns port.
        asset_returns: Port,
        /// Market returns port.
        market_returns: Port,
    },

    /// Sortino ratio — downside risk-adjusted return.
    SortinoRatio {
        /// Portfolio return port.
        portfolio_return: Port,
        /// Risk-free rate port.
        risk_free_rate: Port,
        /// Downside deviation port.
        downside_deviation: Port,
    },

    /// Calmar ratio — return / max drawdown.
    CalmarRatio {
        /// CAGR port.
        cagr: Port,
        /// Maximum drawdown port.
        max_drawdown: Port,
    },

    /// Altman Z-score — bankruptcy prediction.
    AltmanZScore {
        /// Working capital / total assets port.
        working_capital_to_assets: Port,
        /// Retained earnings / total assets port.
        retained_earnings_to_assets: Port,
        /// EBIT / total assets port.
        ebit_to_assets: Port,
        /// Market equity / book liabilities port.
        equity_to_liabilities: Port,
        /// Sales / total assets port.
        sales_to_assets: Port,
    },

    /// Present value of the tax shield from debt financing.
    TaxShield {
        /// Corporate tax rate port.
        tax_rate: Port,
        /// Debt amount port.
        debt: Port,
    },

    /// Adjusted present value (APV).
    AdjustedPresentValue {
        /// Unlevered NPV port.
        unlevered_npv: Port,
        /// PV of tax shield port.
        pv_tax_shield: Port,
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
pub struct CausalityEngine {
    graph: DiGraph<NodeId, ()>,
    nodes: HashMap<NodeId, Node>,
    indices: HashMap<NodeId, NodeIndex>,
    next_id: usize,
    /// Optional formula result cache for memoization.
    cache: Option<std::sync::Arc<dyn crate::cache::FormulaCache>>,
}

impl std::fmt::Debug for CausalityEngine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return f
            .debug_struct("CausalityEngine")
            .field("graph", &self.graph)
            .field("nodes", &self.nodes)
            .field("indices", &self.indices)
            .field("next_id", &self.next_id)
            .field("cache", &self.cache.as_ref().map(|_| "Some(FormulaCache)"))
            .finish();
    }
}

impl Default for CausalityEngine {
    fn default() -> Self {
        return Self {
            graph: DiGraph::new(),
            nodes: HashMap::new(),
            indices: HashMap::new(),
            next_id: 0,
            cache: None,
        };
    }
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

    /// Attaches a formula result cache to this engine.
    ///
    /// When a cache is set, the evaluator checks it before computing each
    /// formula node. Identical inputs produce a cache hit, avoiding
    /// recomputation.
    #[must_use]
    pub fn with_cache(mut self, cache: std::sync::Arc<dyn crate::cache::FormulaCache>) -> Self {
        self.cache = Some(cache);
        return self;
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
                NodeKind::Formula(formula) => {
                    // Check the formula result cache before computing.
                    if let Some(ref cache) = self.cache {
                        let deps = self.dependency_inputs(id, &outputs);
                        let key = crate::cache::CacheKey {
                            node: id,
                            inputs: deps,
                        };
                        if let Some(cached) = cache.get_sync(&key) {
                            cached.value
                        } else {
                            let value = Self::evaluate_formula(formula, &outputs, id)?;
                            cache.put_sync(key, crate::cache::EvaluationResult { value });
                            value
                        }
                    } else {
                        Self::evaluate_formula(formula, &outputs, id)?
                    }
                }
            };
            outputs.insert(id, value);
        }

        return Ok(outputs);
    }

    /// Collects the dependency inputs for a node from the current outputs.
    fn dependency_inputs(
        &self,
        id: NodeId,
        outputs: &HashMap<NodeId, Decimal>,
    ) -> Vec<(NodeId, Decimal)> {
        let mut deps: Vec<(NodeId, Decimal)> = Vec::new();
        if let Some(&idx) = self.indices.get(&id) {
            let mut neighbors: Vec<NodeId> = self
                .graph
                .neighbors_directed(idx, petgraph::Direction::Incoming)
                .filter_map(|n| self.graph.node_weight(n).copied())
                .collect();
            neighbors.sort();
            for dep_id in neighbors {
                if let Some(&value) = outputs.get(&dep_id) {
                    deps.push((dep_id, value));
                }
            }
        }
        return deps;
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

    #[allow(clippy::too_many_lines)]
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
            FormulaKind::AmortizationPayment {
                principal,
                rate,
                periods,
            } => Self::eval_amortization_payment(principal, rate, periods, outputs, node_id),
            FormulaKind::YieldToMaturityApproximation {
                face_value,
                coupon_payment,
                price,
                periods,
            } => Self::eval_yield_to_maturity_approximation(
                face_value,
                coupon_payment,
                price,
                periods,
                outputs,
                node_id,
            ),
            FormulaKind::SimpleMovingAverage { prices, window } => {
                Self::eval_simple_moving_average(prices, window, outputs, node_id)
            }
            FormulaKind::BlackScholesCall {
                spot,
                strike,
                risk_free_rate,
                volatility,
                time_to_maturity,
            } => Self::eval_black_scholes_call(
                spot,
                strike,
                risk_free_rate,
                volatility,
                time_to_maturity,
                outputs,
                node_id,
            ),
            FormulaKind::BlackScholesPut {
                spot,
                strike,
                risk_free_rate,
                volatility,
                time_to_maturity,
            } => Self::eval_black_scholes_put(
                spot,
                strike,
                risk_free_rate,
                volatility,
                time_to_maturity,
                outputs,
                node_id,
            ),
            FormulaKind::BinomialOptionCall {
                spot,
                strike,
                risk_free_rate,
                volatility,
                time_to_maturity,
                steps,
            } => Self::eval_binomial_option_call(
                spot,
                strike,
                risk_free_rate,
                volatility,
                time_to_maturity,
                steps,
                outputs,
                node_id,
            ),
            FormulaKind::BinomialOptionPut {
                spot,
                strike,
                risk_free_rate,
                volatility,
                time_to_maturity,
                steps,
            } => Self::eval_binomial_option_put(
                spot,
                strike,
                risk_free_rate,
                volatility,
                time_to_maturity,
                steps,
                outputs,
                node_id,
            ),
            FormulaKind::BlackScholesDelta {
                spot,
                strike,
                risk_free_rate,
                volatility,
                time_to_maturity,
                style,
            } => Self::eval_black_scholes_delta(
                spot,
                strike,
                risk_free_rate,
                volatility,
                time_to_maturity,
                *style,
                outputs,
                node_id,
            ),
            FormulaKind::BlackScholesGamma {
                spot,
                strike,
                risk_free_rate,
                volatility,
                time_to_maturity,
            } => Self::eval_black_scholes_gamma(
                spot,
                strike,
                risk_free_rate,
                volatility,
                time_to_maturity,
                outputs,
                node_id,
            ),
            FormulaKind::BlackScholesVega {
                spot,
                strike,
                risk_free_rate,
                volatility,
                time_to_maturity,
            } => Self::eval_black_scholes_vega(
                spot,
                strike,
                risk_free_rate,
                volatility,
                time_to_maturity,
                outputs,
                node_id,
            ),
            FormulaKind::BlackScholesTheta {
                spot,
                strike,
                risk_free_rate,
                volatility,
                time_to_maturity,
                style,
            } => Self::eval_black_scholes_theta(
                spot,
                strike,
                risk_free_rate,
                volatility,
                time_to_maturity,
                *style,
                outputs,
                node_id,
            ),
            FormulaKind::BlackScholesRho {
                spot,
                strike,
                risk_free_rate,
                volatility,
                time_to_maturity,
                style,
            } => Self::eval_black_scholes_rho(
                spot,
                strike,
                risk_free_rate,
                volatility,
                time_to_maturity,
                *style,
                outputs,
                node_id,
            ),
            FormulaKind::GrowingPerpetuityPresentValue {
                payment,
                rate,
                growth_rate,
            } => Self::eval_growing_perpetuity_present_value(
                payment,
                rate,
                growth_rate,
                outputs,
                node_id,
            ),
            FormulaKind::ContinuousCompoundingFutureValue {
                present_value,
                rate,
                time,
            } => Self::eval_continuous_compounding_future_value(
                present_value,
                rate,
                time,
                outputs,
                node_id,
            ),
            FormulaKind::ReturnOnInvestment { gain, cost } => {
                Self::eval_return_on_investment(gain, cost, outputs, node_id)
            }
            FormulaKind::ProfitMargin {
                net_income,
                revenue,
            } => Self::eval_profit_margin(net_income, revenue, outputs, node_id),
            FormulaKind::AssetTurnover {
                revenue,
                total_assets,
            } => Self::eval_asset_turnover(revenue, total_assets, outputs, node_id),
            FormulaKind::EquityMultiplier {
                total_assets,
                shareholders_equity,
            } => Self::eval_equity_multiplier(total_assets, shareholders_equity, outputs, node_id),
            FormulaKind::QuickRatio {
                current_assets,
                inventory,
                current_liabilities,
            } => Self::eval_quick_ratio(
                current_assets,
                inventory,
                current_liabilities,
                outputs,
                node_id,
            ),
            FormulaKind::InterestCoverage {
                ebit,
                interest_expense,
            } => Self::eval_interest_coverage(ebit, interest_expense, outputs, node_id),
            FormulaKind::InventoryTurnover { cogs, inventory } => {
                Self::eval_inventory_turnover(cogs, inventory, outputs, node_id)
            }
            FormulaKind::CashConversionCycle {
                days_inventory_outstanding,
                days_sales_outstanding,
                days_payables_outstanding,
            } => Self::eval_cash_conversion_cycle(
                days_inventory_outstanding,
                days_sales_outstanding,
                days_payables_outstanding,
                outputs,
                node_id,
            ),
            FormulaKind::CapitalAdequacyRatio {
                total_capital,
                risk_weighted_assets,
            } => Self::eval_capital_adequacy_ratio(
                total_capital,
                risk_weighted_assets,
                outputs,
                node_id,
            ),
            FormulaKind::ProvisionCoverageRatio {
                provisions,
                non_performing_assets,
            } => Self::eval_provision_coverage_ratio(
                provisions,
                non_performing_assets,
                outputs,
                node_id,
            ),
            FormulaKind::TreynorRatio {
                portfolio_return,
                risk_free_rate,
                beta,
            } => Self::eval_treynor_ratio(portfolio_return, risk_free_rate, beta, outputs, node_id),
            FormulaKind::ValueAtRisk {
                portfolio_value,
                mean_return,
                std_dev,
                z_score,
            } => Self::eval_value_at_risk(
                portfolio_value,
                mean_return,
                std_dev,
                z_score,
                outputs,
                node_id,
            ),
            FormulaKind::ExpectedShortfall {
                portfolio_value,
                mean_return,
                std_dev,
                z_score,
            } => Self::eval_expected_shortfall(
                portfolio_value,
                mean_return,
                std_dev,
                z_score,
                outputs,
                node_id,
            ),
            FormulaKind::DiscountedCashFlow {
                cash_flows,
                discount_rate,
            } => Self::eval_discounted_cash_flow(cash_flows, discount_rate, outputs, node_id),
            FormulaKind::MacaulayDuration {
                cash_flows,
                yield_per_period,
            } => Self::eval_macaulay_duration(cash_flows, yield_per_period, outputs, node_id),
            FormulaKind::ModifiedDuration {
                macaulay_duration,
                yield_per_period,
            } => {
                Self::eval_modified_duration(macaulay_duration, yield_per_period, outputs, node_id)
            }
            FormulaKind::Convexity {
                cash_flows,
                yield_per_period,
            } => Self::eval_convexity(cash_flows, yield_per_period, outputs, node_id),
            FormulaKind::FreeCashFlowToEquity {
                fcff,
                interest_expense_after_tax,
                net_borrowing,
            } => Self::eval_free_cash_flow_to_equity(
                fcff,
                interest_expense_after_tax,
                net_borrowing,
                outputs,
                node_id,
            ),
            FormulaKind::EconomicValueAdded {
                nopat,
                invested_capital,
                wacc,
            } => Self::eval_economic_value_added(nopat, invested_capital, wacc, outputs, node_id),
            FormulaKind::InternalGrowthRate {
                roe,
                dividend_payout_ratio,
            } => Self::eval_internal_growth_rate(roe, dividend_payout_ratio, outputs, node_id),
            FormulaKind::Beta {
                asset_returns,
                market_returns,
            } => Self::eval_beta(asset_returns, market_returns, outputs, node_id),
            FormulaKind::SortinoRatio {
                portfolio_return,
                risk_free_rate,
                downside_deviation,
            } => Self::eval_sortino_ratio(
                portfolio_return,
                risk_free_rate,
                downside_deviation,
                outputs,
                node_id,
            ),
            FormulaKind::CalmarRatio { cagr, max_drawdown } => {
                Self::eval_calmar_ratio(cagr, max_drawdown, outputs, node_id)
            }
            FormulaKind::AltmanZScore {
                working_capital_to_assets,
                retained_earnings_to_assets,
                ebit_to_assets,
                equity_to_liabilities,
                sales_to_assets,
            } => Self::eval_altman_z_score(
                working_capital_to_assets,
                retained_earnings_to_assets,
                ebit_to_assets,
                equity_to_liabilities,
                sales_to_assets,
                outputs,
                node_id,
            ),
            FormulaKind::TaxShield { tax_rate, debt } => {
                Self::eval_tax_shield(tax_rate, debt, outputs, node_id)
            }
            FormulaKind::AdjustedPresentValue {
                unlevered_npv,
                pv_tax_shield,
            } => Self::eval_adjusted_present_value(unlevered_npv, pv_tax_shield, outputs, node_id),
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

    fn eval_amortization_payment(
        principal: &Port,
        rate: &Port,
        periods: &Port,
        outputs: &HashMap<NodeId, Decimal>,
        node_id: NodeId,
    ) -> Result<Decimal, DagError> {
        let p = Self::resolve_port(principal, outputs)?;
        let r = Self::resolve_port(rate, outputs)?;
        let n = Self::resolve_period(periods, outputs)?;
        return casiros_core::general::amortization_payment(p, r, n)
            .map_err(|err| Self::wrap_formula_error(node_id, err));
    }

    fn eval_yield_to_maturity_approximation(
        face_value: &Port,
        coupon_payment: &Port,
        price: &Port,
        periods: &Port,
        outputs: &HashMap<NodeId, Decimal>,
        node_id: NodeId,
    ) -> Result<Decimal, DagError> {
        let f = Self::resolve_port(face_value, outputs)?;
        let c = Self::resolve_port(coupon_payment, outputs)?;
        let p = Self::resolve_port(price, outputs)?;
        let n = Self::resolve_period(periods, outputs)?;
        return casiros_core::stocks_bonds::yield_to_maturity_approximation(f, c, p, n)
            .map_err(|err| Self::wrap_formula_error(node_id, err));
    }

    fn eval_simple_moving_average(
        prices: &Port,
        window: &Port,
        outputs: &HashMap<NodeId, Decimal>,
        node_id: NodeId,
    ) -> Result<Decimal, DagError> {
        let prices_value = Self::resolve_port(prices, outputs)?;
        let window_value = Self::resolve_port(window, outputs)?;
        let window_u = window_value.to_u32().ok_or(DagError::InvalidPeriod {
            value: window_value,
        })?;

        // The DAG operates on scalar ports. A price series is represented as a
        // single comma-separated Decimal value encoded as a string. This is a
        // pragmatic MVP encoding for vector inputs; future phases may introduce a
        // first-class vector port type.
        let series: Vec<Decimal> = prices_value
            .to_string()
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| {
                s.parse().map_err(|_| DagError::InvalidPeriod {
                    value: prices_value,
                })
            })
            .collect::<Result<_, _>>()?;

        return casiros_core::markets::simple_moving_average(&series, window_u as usize)
            .map_err(|err| Self::wrap_formula_error(node_id, err));
    }

    fn eval_black_scholes_call(
        spot: &Port,
        strike: &Port,
        risk_free_rate: &Port,
        volatility: &Port,
        time_to_maturity: &Port,
        outputs: &HashMap<NodeId, Decimal>,
        node_id: NodeId,
    ) -> Result<Decimal, DagError> {
        let s = Self::resolve_port(spot, outputs)?;
        let k = Self::resolve_port(strike, outputs)?;
        let r = Self::resolve_port(risk_free_rate, outputs)?;
        let sigma = Self::resolve_port(volatility, outputs)?;
        let t = Self::resolve_port(time_to_maturity, outputs)?;
        return casiros_core::options::black_scholes_call(s, k, r, sigma, t)
            .map_err(|err| Self::wrap_formula_error(node_id, err));
    }

    fn eval_black_scholes_put(
        spot: &Port,
        strike: &Port,
        risk_free_rate: &Port,
        volatility: &Port,
        time_to_maturity: &Port,
        outputs: &HashMap<NodeId, Decimal>,
        node_id: NodeId,
    ) -> Result<Decimal, DagError> {
        let s = Self::resolve_port(spot, outputs)?;
        let k = Self::resolve_port(strike, outputs)?;
        let r = Self::resolve_port(risk_free_rate, outputs)?;
        let sigma = Self::resolve_port(volatility, outputs)?;
        let t = Self::resolve_port(time_to_maturity, outputs)?;
        return casiros_core::options::black_scholes_put(s, k, r, sigma, t)
            .map_err(|err| Self::wrap_formula_error(node_id, err));
    }

    fn eval_binomial_option_call(
        spot: &Port,
        strike: &Port,
        risk_free_rate: &Port,
        volatility: &Port,
        time_to_maturity: &Port,
        steps: &Port,
        outputs: &HashMap<NodeId, Decimal>,
        node_id: NodeId,
    ) -> Result<Decimal, DagError> {
        let spot_f = Self::resolve_port(spot, outputs)?;
        let strike_f = Self::resolve_port(strike, outputs)?;
        let rate_f = Self::resolve_port(risk_free_rate, outputs)?;
        let vol_f = Self::resolve_port(volatility, outputs)?;
        let maturity_f = Self::resolve_port(time_to_maturity, outputs)?;
        let step_count = Self::resolve_period(steps, outputs)?;
        return casiros_core::options::binomial_option_call(
            spot_f, strike_f, rate_f, vol_f, maturity_f, step_count,
        )
        .map_err(|err| Self::wrap_formula_error(node_id, err));
    }

    fn eval_binomial_option_put(
        spot: &Port,
        strike: &Port,
        risk_free_rate: &Port,
        volatility: &Port,
        time_to_maturity: &Port,
        steps: &Port,
        outputs: &HashMap<NodeId, Decimal>,
        node_id: NodeId,
    ) -> Result<Decimal, DagError> {
        let spot_f = Self::resolve_port(spot, outputs)?;
        let strike_f = Self::resolve_port(strike, outputs)?;
        let rate_f = Self::resolve_port(risk_free_rate, outputs)?;
        let vol_f = Self::resolve_port(volatility, outputs)?;
        let maturity_f = Self::resolve_port(time_to_maturity, outputs)?;
        let step_count = Self::resolve_period(steps, outputs)?;
        return casiros_core::options::binomial_option_put(
            spot_f, strike_f, rate_f, vol_f, maturity_f, step_count,
        )
        .map_err(|err| Self::wrap_formula_error(node_id, err));
    }

    fn eval_black_scholes_delta(
        spot: &Port,
        strike: &Port,
        risk_free_rate: &Port,
        volatility: &Port,
        time_to_maturity: &Port,
        style: OptionStyle,
        outputs: &HashMap<NodeId, Decimal>,
        node_id: NodeId,
    ) -> Result<Decimal, DagError> {
        let s = Self::resolve_port(spot, outputs)?;
        let k = Self::resolve_port(strike, outputs)?;
        let r = Self::resolve_port(risk_free_rate, outputs)?;
        let sigma = Self::resolve_port(volatility, outputs)?;
        let t = Self::resolve_port(time_to_maturity, outputs)?;
        let core_style = Self::option_style_to_core(style);
        return casiros_core::options::black_scholes_delta(s, k, r, sigma, t, core_style)
            .map_err(|err| Self::wrap_formula_error(node_id, err));
    }

    fn eval_black_scholes_gamma(
        spot: &Port,
        strike: &Port,
        risk_free_rate: &Port,
        volatility: &Port,
        time_to_maturity: &Port,
        outputs: &HashMap<NodeId, Decimal>,
        node_id: NodeId,
    ) -> Result<Decimal, DagError> {
        let s = Self::resolve_port(spot, outputs)?;
        let k = Self::resolve_port(strike, outputs)?;
        let r = Self::resolve_port(risk_free_rate, outputs)?;
        let sigma = Self::resolve_port(volatility, outputs)?;
        let t = Self::resolve_port(time_to_maturity, outputs)?;
        return casiros_core::options::black_scholes_gamma(s, k, r, sigma, t)
            .map_err(|err| Self::wrap_formula_error(node_id, err));
    }

    fn eval_black_scholes_vega(
        spot: &Port,
        strike: &Port,
        risk_free_rate: &Port,
        volatility: &Port,
        time_to_maturity: &Port,
        outputs: &HashMap<NodeId, Decimal>,
        node_id: NodeId,
    ) -> Result<Decimal, DagError> {
        let s = Self::resolve_port(spot, outputs)?;
        let k = Self::resolve_port(strike, outputs)?;
        let r = Self::resolve_port(risk_free_rate, outputs)?;
        let sigma = Self::resolve_port(volatility, outputs)?;
        let t = Self::resolve_port(time_to_maturity, outputs)?;
        return casiros_core::options::black_scholes_vega(s, k, r, sigma, t)
            .map_err(|err| Self::wrap_formula_error(node_id, err));
    }

    fn eval_black_scholes_theta(
        spot: &Port,
        strike: &Port,
        risk_free_rate: &Port,
        volatility: &Port,
        time_to_maturity: &Port,
        style: OptionStyle,
        outputs: &HashMap<NodeId, Decimal>,
        node_id: NodeId,
    ) -> Result<Decimal, DagError> {
        let s = Self::resolve_port(spot, outputs)?;
        let k = Self::resolve_port(strike, outputs)?;
        let r = Self::resolve_port(risk_free_rate, outputs)?;
        let sigma = Self::resolve_port(volatility, outputs)?;
        let t = Self::resolve_port(time_to_maturity, outputs)?;
        let core_style = Self::option_style_to_core(style);
        return casiros_core::options::black_scholes_theta(s, k, r, sigma, t, core_style)
            .map_err(|err| Self::wrap_formula_error(node_id, err));
    }

    fn eval_black_scholes_rho(
        spot: &Port,
        strike: &Port,
        risk_free_rate: &Port,
        volatility: &Port,
        time_to_maturity: &Port,
        style: OptionStyle,
        outputs: &HashMap<NodeId, Decimal>,
        node_id: NodeId,
    ) -> Result<Decimal, DagError> {
        let s = Self::resolve_port(spot, outputs)?;
        let k = Self::resolve_port(strike, outputs)?;
        let r = Self::resolve_port(risk_free_rate, outputs)?;
        let sigma = Self::resolve_port(volatility, outputs)?;
        let t = Self::resolve_port(time_to_maturity, outputs)?;
        let core_style = Self::option_style_to_core(style);
        return casiros_core::options::black_scholes_rho(s, k, r, sigma, t, core_style)
            .map_err(|err| Self::wrap_formula_error(node_id, err));
    }

    fn eval_growing_perpetuity_present_value(
        payment: &Port,
        rate: &Port,
        growth_rate: &Port,
        outputs: &HashMap<NodeId, Decimal>,
        node_id: NodeId,
    ) -> Result<Decimal, DagError> {
        let pmt = Self::resolve_port(payment, outputs)?;
        let r = Self::resolve_port(rate, outputs)?;
        let g = Self::resolve_port(growth_rate, outputs)?;
        return casiros_core::general::growing_perpetuity_present_value(pmt, r, g)
            .map_err(|err| Self::wrap_formula_error(node_id, err));
    }

    fn eval_continuous_compounding_future_value(
        present_value: &Port,
        rate: &Port,
        time: &Port,
        outputs: &HashMap<NodeId, Decimal>,
        node_id: NodeId,
    ) -> Result<Decimal, DagError> {
        let pv = Self::resolve_port(present_value, outputs)?;
        let r = Self::resolve_port(rate, outputs)?;
        let t = Self::resolve_port(time, outputs)?;
        return casiros_core::general::continuous_compounding_future_value(pv, r, t)
            .map_err(|err| Self::wrap_formula_error(node_id, err));
    }

    fn eval_return_on_investment(
        gain: &Port,
        cost: &Port,
        outputs: &HashMap<NodeId, Decimal>,
        node_id: NodeId,
    ) -> Result<Decimal, DagError> {
        let g = Self::resolve_port(gain, outputs)?;
        let c = Self::resolve_port(cost, outputs)?;
        return casiros_core::financial::return_on_investment(g, c)
            .map_err(|err| Self::wrap_formula_error(node_id, err));
    }

    fn eval_profit_margin(
        net_income: &Port,
        revenue: &Port,
        outputs: &HashMap<NodeId, Decimal>,
        node_id: NodeId,
    ) -> Result<Decimal, DagError> {
        let ni = Self::resolve_port(net_income, outputs)?;
        let rev = Self::resolve_port(revenue, outputs)?;
        return casiros_core::financial::profit_margin(ni, rev)
            .map_err(|err| Self::wrap_formula_error(node_id, err));
    }

    fn eval_asset_turnover(
        revenue: &Port,
        total_assets: &Port,
        outputs: &HashMap<NodeId, Decimal>,
        node_id: NodeId,
    ) -> Result<Decimal, DagError> {
        let rev = Self::resolve_port(revenue, outputs)?;
        let assets = Self::resolve_port(total_assets, outputs)?;
        return casiros_core::financial::asset_turnover(rev, assets)
            .map_err(|err| Self::wrap_formula_error(node_id, err));
    }

    fn eval_equity_multiplier(
        total_assets: &Port,
        shareholders_equity: &Port,
        outputs: &HashMap<NodeId, Decimal>,
        node_id: NodeId,
    ) -> Result<Decimal, DagError> {
        let assets = Self::resolve_port(total_assets, outputs)?;
        let equity = Self::resolve_port(shareholders_equity, outputs)?;
        return casiros_core::financial::equity_multiplier(assets, equity)
            .map_err(|err| Self::wrap_formula_error(node_id, err));
    }

    fn eval_quick_ratio(
        current_assets: &Port,
        inventory: &Port,
        current_liabilities: &Port,
        outputs: &HashMap<NodeId, Decimal>,
        node_id: NodeId,
    ) -> Result<Decimal, DagError> {
        let assets = Self::resolve_port(current_assets, outputs)?;
        let inv = Self::resolve_port(inventory, outputs)?;
        let liab = Self::resolve_port(current_liabilities, outputs)?;
        return casiros_core::financial::quick_ratio(assets, inv, liab)
            .map_err(|err| Self::wrap_formula_error(node_id, err));
    }

    fn eval_interest_coverage(
        ebit: &Port,
        interest_expense: &Port,
        outputs: &HashMap<NodeId, Decimal>,
        node_id: NodeId,
    ) -> Result<Decimal, DagError> {
        let e = Self::resolve_port(ebit, outputs)?;
        let interest = Self::resolve_port(interest_expense, outputs)?;
        return casiros_core::financial::interest_coverage(e, interest)
            .map_err(|err| Self::wrap_formula_error(node_id, err));
    }

    fn eval_inventory_turnover(
        cogs: &Port,
        inventory: &Port,
        outputs: &HashMap<NodeId, Decimal>,
        node_id: NodeId,
    ) -> Result<Decimal, DagError> {
        let cost = Self::resolve_port(cogs, outputs)?;
        let inv = Self::resolve_port(inventory, outputs)?;
        return casiros_core::financial::inventory_turnover(cost, inv)
            .map_err(|err| Self::wrap_formula_error(node_id, err));
    }

    fn eval_cash_conversion_cycle(
        days_inventory_outstanding: &Port,
        days_sales_outstanding: &Port,
        days_payables_outstanding: &Port,
        outputs: &HashMap<NodeId, Decimal>,
        node_id: NodeId,
    ) -> Result<Decimal, DagError> {
        let dio = Self::resolve_port(days_inventory_outstanding, outputs)?;
        let dso = Self::resolve_port(days_sales_outstanding, outputs)?;
        let dpo = Self::resolve_port(days_payables_outstanding, outputs)?;
        return casiros_core::financial::cash_conversion_cycle(dio, dso, dpo)
            .map_err(|err| Self::wrap_formula_error(node_id, err));
    }

    fn eval_capital_adequacy_ratio(
        total_capital: &Port,
        risk_weighted_assets: &Port,
        outputs: &HashMap<NodeId, Decimal>,
        node_id: NodeId,
    ) -> Result<Decimal, DagError> {
        let capital = Self::resolve_port(total_capital, outputs)?;
        let rwa = Self::resolve_port(risk_weighted_assets, outputs)?;
        return casiros_core::banking::capital_adequacy_ratio(capital, rwa)
            .map_err(|err| Self::wrap_formula_error(node_id, err));
    }

    fn eval_provision_coverage_ratio(
        provisions: &Port,
        non_performing_assets: &Port,
        outputs: &HashMap<NodeId, Decimal>,
        node_id: NodeId,
    ) -> Result<Decimal, DagError> {
        let prov = Self::resolve_port(provisions, outputs)?;
        let npa = Self::resolve_port(non_performing_assets, outputs)?;
        return casiros_core::banking::provision_coverage_ratio(prov, npa)
            .map_err(|err| Self::wrap_formula_error(node_id, err));
    }

    fn eval_treynor_ratio(
        portfolio_return: &Port,
        risk_free_rate: &Port,
        beta: &Port,
        outputs: &HashMap<NodeId, Decimal>,
        node_id: NodeId,
    ) -> Result<Decimal, DagError> {
        let rp = Self::resolve_port(portfolio_return, outputs)?;
        let rf = Self::resolve_port(risk_free_rate, outputs)?;
        let b = Self::resolve_port(beta, outputs)?;
        return casiros_core::markets::treynor_ratio(rp, rf, b)
            .map_err(|err| Self::wrap_formula_error(node_id, err));
    }

    fn eval_value_at_risk(
        portfolio_value: &Port,
        mean_return: &Port,
        std_dev: &Port,
        z_score: &Port,
        outputs: &HashMap<NodeId, Decimal>,
        node_id: NodeId,
    ) -> Result<Decimal, DagError> {
        let pv = Self::resolve_port(portfolio_value, outputs)?;
        let mu = Self::resolve_port(mean_return, outputs)?;
        let sigma = Self::resolve_port(std_dev, outputs)?;
        let z = Self::resolve_port(z_score, outputs)?;
        return casiros_core::markets::value_at_risk(pv, mu, sigma, z)
            .map_err(|err| Self::wrap_formula_error(node_id, err));
    }

    fn eval_expected_shortfall(
        portfolio_value: &Port,
        mean_return: &Port,
        std_dev: &Port,
        z_score: &Port,
        outputs: &HashMap<NodeId, Decimal>,
        node_id: NodeId,
    ) -> Result<Decimal, DagError> {
        let pv = Self::resolve_port(portfolio_value, outputs)?;
        let mu = Self::resolve_port(mean_return, outputs)?;
        let sigma = Self::resolve_port(std_dev, outputs)?;
        let z = Self::resolve_port(z_score, outputs)?;
        return casiros_core::markets::expected_shortfall(pv, mu, sigma, z)
            .map_err(|err| Self::wrap_formula_error(node_id, err));
    }

    fn parse_decimal_series(
        port: &Port,
        outputs: &HashMap<NodeId, Decimal>,
    ) -> Result<Vec<Decimal>, DagError> {
        let value = Self::resolve_port(port, outputs)?;
        return value
            .to_string()
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(|s| s.parse().map_err(|_| DagError::InvalidPeriod { value }))
            .collect::<Result<_, _>>();
    }

    fn eval_discounted_cash_flow(
        cash_flows: &Port,
        discount_rate: &Port,
        outputs: &HashMap<NodeId, Decimal>,
        node_id: NodeId,
    ) -> Result<Decimal, DagError> {
        let flows = Self::parse_decimal_series(cash_flows, outputs)?;
        let r = Self::resolve_port(discount_rate, outputs)?;
        return casiros_core::stocks_bonds::discounted_cash_flow(&flows, r)
            .map_err(|err| Self::wrap_formula_error(node_id, err));
    }

    fn eval_macaulay_duration(
        cash_flows: &Port,
        yield_per_period: &Port,
        outputs: &HashMap<NodeId, Decimal>,
        node_id: NodeId,
    ) -> Result<Decimal, DagError> {
        let flows = Self::parse_decimal_series(cash_flows, outputs)?;
        let y = Self::resolve_port(yield_per_period, outputs)?;
        return casiros_core::stocks_bonds::macaulay_duration(&flows, y)
            .map_err(|err| Self::wrap_formula_error(node_id, err));
    }

    fn eval_modified_duration(
        macaulay_duration: &Port,
        yield_per_period: &Port,
        outputs: &HashMap<NodeId, Decimal>,
        node_id: NodeId,
    ) -> Result<Decimal, DagError> {
        let d = Self::resolve_port(macaulay_duration, outputs)?;
        let y = Self::resolve_port(yield_per_period, outputs)?;
        return casiros_core::stocks_bonds::modified_duration(d, y)
            .map_err(|err| Self::wrap_formula_error(node_id, err));
    }

    fn eval_convexity(
        cash_flows: &Port,
        yield_per_period: &Port,
        outputs: &HashMap<NodeId, Decimal>,
        node_id: NodeId,
    ) -> Result<Decimal, DagError> {
        let flows = Self::parse_decimal_series(cash_flows, outputs)?;
        let y = Self::resolve_port(yield_per_period, outputs)?;
        return casiros_core::stocks_bonds::convexity(&flows, y)
            .map_err(|err| Self::wrap_formula_error(node_id, err));
    }

    fn eval_free_cash_flow_to_equity(
        fcff: &Port,
        interest_expense_after_tax: &Port,
        net_borrowing: &Port,
        outputs: &HashMap<NodeId, Decimal>,
        node_id: NodeId,
    ) -> Result<Decimal, DagError> {
        let f = Self::resolve_port(fcff, outputs)?;
        let interest = Self::resolve_port(interest_expense_after_tax, outputs)?;
        let borrowing = Self::resolve_port(net_borrowing, outputs)?;
        return casiros_core::corporate::free_cash_flow_to_equity(f, interest, borrowing)
            .map_err(|err| Self::wrap_formula_error(node_id, err));
    }

    fn eval_economic_value_added(
        nopat: &Port,
        invested_capital: &Port,
        wacc: &Port,
        outputs: &HashMap<NodeId, Decimal>,
        node_id: NodeId,
    ) -> Result<Decimal, DagError> {
        let n = Self::resolve_port(nopat, outputs)?;
        let ic = Self::resolve_port(invested_capital, outputs)?;
        let w = Self::resolve_port(wacc, outputs)?;
        return casiros_core::corporate::economic_value_added(n, ic, w)
            .map_err(|err| Self::wrap_formula_error(node_id, err));
    }

    fn eval_internal_growth_rate(
        roe: &Port,
        dividend_payout_ratio: &Port,
        outputs: &HashMap<NodeId, Decimal>,
        node_id: NodeId,
    ) -> Result<Decimal, DagError> {
        let r = Self::resolve_port(roe, outputs)?;
        let payout = Self::resolve_port(dividend_payout_ratio, outputs)?;
        return casiros_core::corporate::internal_growth_rate(r, payout)
            .map_err(|err| Self::wrap_formula_error(node_id, err));
    }

    fn eval_beta(
        asset_returns: &Port,
        market_returns: &Port,
        outputs: &HashMap<NodeId, Decimal>,
        node_id: NodeId,
    ) -> Result<Decimal, DagError> {
        let a = Self::resolve_port(asset_returns, outputs)?;
        let m = Self::resolve_port(market_returns, outputs)?;
        // Beta requires arrays; parse comma-separated values.
        let asset_vec: Vec<Decimal> = a
            .to_string()
            .split(',')
            .filter_map(|s| s.trim().parse::<Decimal>().ok())
            .collect();
        let market_vec: Vec<Decimal> = m
            .to_string()
            .split(',')
            .filter_map(|s| s.trim().parse::<Decimal>().ok())
            .collect();
        return casiros_core::markets::beta(&asset_vec, &market_vec)
            .map_err(|err| Self::wrap_formula_error(node_id, err));
    }

    fn eval_sortino_ratio(
        portfolio_return: &Port,
        risk_free_rate: &Port,
        downside_deviation: &Port,
        outputs: &HashMap<NodeId, Decimal>,
        node_id: NodeId,
    ) -> Result<Decimal, DagError> {
        let pr = Self::resolve_port(portfolio_return, outputs)?;
        let rf = Self::resolve_port(risk_free_rate, outputs)?;
        let dd = Self::resolve_port(downside_deviation, outputs)?;
        return casiros_core::markets::sortino_ratio(pr, rf, dd)
            .map_err(|err| Self::wrap_formula_error(node_id, err));
    }

    fn eval_calmar_ratio(
        cagr: &Port,
        max_drawdown: &Port,
        outputs: &HashMap<NodeId, Decimal>,
        node_id: NodeId,
    ) -> Result<Decimal, DagError> {
        let c = Self::resolve_port(cagr, outputs)?;
        let m = Self::resolve_port(max_drawdown, outputs)?;
        return casiros_core::markets::calmar_ratio(c, m)
            .map_err(|err| Self::wrap_formula_error(node_id, err));
    }

    fn eval_altman_z_score(
        working_capital_to_assets: &Port,
        retained_earnings_to_assets: &Port,
        ebit_to_assets: &Port,
        equity_to_liabilities: &Port,
        sales_to_assets: &Port,
        outputs: &HashMap<NodeId, Decimal>,
        node_id: NodeId,
    ) -> Result<Decimal, DagError> {
        let wc = Self::resolve_port(working_capital_to_assets, outputs)?;
        let re = Self::resolve_port(retained_earnings_to_assets, outputs)?;
        let ebit = Self::resolve_port(ebit_to_assets, outputs)?;
        let eq = Self::resolve_port(equity_to_liabilities, outputs)?;
        let sales = Self::resolve_port(sales_to_assets, outputs)?;
        return casiros_core::financial::altman_z_score(wc, re, ebit, eq, sales)
            .map_err(|err| Self::wrap_formula_error(node_id, err));
    }

    fn eval_tax_shield(
        tax_rate: &Port,
        debt: &Port,
        outputs: &HashMap<NodeId, Decimal>,
        node_id: NodeId,
    ) -> Result<Decimal, DagError> {
        let t = Self::resolve_port(tax_rate, outputs)?;
        let d = Self::resolve_port(debt, outputs)?;
        return casiros_core::corporate::tax_shield(t, d)
            .map_err(|err| Self::wrap_formula_error(node_id, err));
    }

    fn eval_adjusted_present_value(
        unlevered_npv: &Port,
        pv_tax_shield: &Port,
        outputs: &HashMap<NodeId, Decimal>,
        node_id: NodeId,
    ) -> Result<Decimal, DagError> {
        let u = Self::resolve_port(unlevered_npv, outputs)?;
        let p = Self::resolve_port(pv_tax_shield, outputs)?;
        return casiros_core::corporate::adjusted_present_value(u, p)
            .map_err(|err| Self::wrap_formula_error(node_id, err));
    }

    fn option_style_to_core(style: OptionStyle) -> casiros_core::options::OptionStyle {
        return match style {
            OptionStyle::Call => casiros_core::options::OptionStyle::Call,
            OptionStyle::Put => casiros_core::options::OptionStyle::Put,
        };
    }

    fn resolve_period(
        port: &Port,
        outputs: &HashMap<NodeId, Decimal>,
    ) -> Result<casiros_core::prelude::Periods, DagError> {
        let value = Self::resolve_port(port, outputs)?;
        return value.to_u32().ok_or(DagError::InvalidPeriod { value });
    }

    /// Returns an iterator over all nodes in insertion order.
    ///
    /// # Panics
    ///
    /// Panics if the internal `NodeId` → `Node` invariant is violated. This
    /// should never happen for a graph built through the public API.
    #[must_use = "iterator is lazy; consume it to traverse the graph"]
    pub fn nodes(&self) -> impl Iterator<Item = &Node> {
        return (0..self.next_id).map(NodeId).map(|id| {
            self.nodes
                .get(&id)
                .expect("internal invariant: every NodeId has a Node")
        });
    }

    /// Returns an iterator over all edges as `(dependency, dependent)` name
    /// pairs.
    ///
    /// # Panics
    ///
    /// Panics if the internal `petgraph` edge or node-weight invariants are
    /// violated. This should never happen for a graph built through the public
    /// API.
    #[must_use = "iterator is lazy; consume it to traverse the graph"]
    pub fn edges(&self) -> impl Iterator<Item = (&str, &str)> {
        return self.graph.edge_indices().map(|edge_idx| {
            let (source_idx, target_idx) = self
                .graph
                .edge_endpoints(edge_idx)
                .expect("internal invariant: every edge index has endpoints");
            let source_id = *self
                .graph
                .node_weight(source_idx)
                .expect("internal invariant: every NodeIndex has a NodeId weight");
            let target_id = *self
                .graph
                .node_weight(target_idx)
                .expect("internal invariant: every NodeIndex has a NodeId weight");
            let source_name = self
                .nodes
                .get(&source_id)
                .expect("internal invariant: every NodeId has a Node")
                .name();
            let target_name = self
                .nodes
                .get(&target_id)
                .expect("internal invariant: every NodeId has a Node")
                .name();
            return (source_name, target_name);
        });
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

    fn evaluate_constant_formula(formula: FormulaKind) -> Decimal {
        let mut engine = CausalityEngine::new();
        let id = engine.add_formula("f", formula);
        return engine.evaluate(&HashMap::new()).unwrap()[&id];
    }

    #[test]
    fn evaluates_present_value() {
        let value = evaluate_constant_formula(FormulaKind::PresentValue {
            future_value: Port::Constant(dec!(162.8895)),
            rate: Port::Constant(dec!(0.05)),
            periods: Port::Constant(dec!(10)),
        });
        assert!((value - dec!(100.0)).abs() < dec!(0.01));
    }

    #[test]
    fn evaluates_amortization_payment() {
        let value = evaluate_constant_formula(FormulaKind::AmortizationPayment {
            principal: Port::Constant(dec!(1000.0)),
            rate: Port::Constant(dec!(0.01)),
            periods: Port::Constant(dec!(12)),
        });
        assert!((value - dec!(88.85)).abs() < dec!(0.01));
    }

    #[test]
    fn evaluates_yield_to_maturity_approximation() {
        let value = evaluate_constant_formula(FormulaKind::YieldToMaturityApproximation {
            face_value: Port::Constant(dec!(1000.0)),
            coupon_payment: Port::Constant(dec!(50.0)),
            price: Port::Constant(dec!(950.0)),
            periods: Port::Constant(dec!(10)),
        });
        assert!((value - dec!(0.0564)).abs() < dec!(0.0001));
    }

    #[test]
    fn evaluates_simple_moving_average() {
        let value = evaluate_constant_formula(FormulaKind::SimpleMovingAverage {
            prices: Port::Constant(dec!(100.0)),
            window: Port::Constant(dec!(1)),
        });
        assert_eq!(value, dec!(100.0));
    }

    #[test]
    fn evaluates_black_scholes_call() {
        let value = evaluate_constant_formula(FormulaKind::BlackScholesCall {
            spot: Port::Constant(dec!(100.0)),
            strike: Port::Constant(dec!(100.0)),
            risk_free_rate: Port::Constant(dec!(0.05)),
            volatility: Port::Constant(dec!(0.2)),
            time_to_maturity: Port::Constant(dec!(1.0)),
        });
        assert!(value > dec!(0.0));
        assert!(value < dec!(20.0));
    }

    #[test]
    fn evaluates_black_scholes_put() {
        let value = evaluate_constant_formula(FormulaKind::BlackScholesPut {
            spot: Port::Constant(dec!(100.0)),
            strike: Port::Constant(dec!(100.0)),
            risk_free_rate: Port::Constant(dec!(0.05)),
            volatility: Port::Constant(dec!(0.2)),
            time_to_maturity: Port::Constant(dec!(1.0)),
        });
        assert!(value > dec!(0.0));
        assert!(value < dec!(20.0));
    }

    #[test]
    fn evaluates_binomial_option_call() {
        let value = evaluate_constant_formula(FormulaKind::BinomialOptionCall {
            spot: Port::Constant(dec!(100.0)),
            strike: Port::Constant(dec!(100.0)),
            risk_free_rate: Port::Constant(dec!(0.05)),
            volatility: Port::Constant(dec!(0.2)),
            time_to_maturity: Port::Constant(dec!(1.0)),
            steps: Port::Constant(dec!(100)),
        });
        assert!(value > dec!(0.0));
        assert!(value < dec!(20.0));
    }

    #[test]
    fn evaluates_binomial_option_put() {
        let value = evaluate_constant_formula(FormulaKind::BinomialOptionPut {
            spot: Port::Constant(dec!(100.0)),
            strike: Port::Constant(dec!(100.0)),
            risk_free_rate: Port::Constant(dec!(0.05)),
            volatility: Port::Constant(dec!(0.2)),
            time_to_maturity: Port::Constant(dec!(1.0)),
            steps: Port::Constant(dec!(100)),
        });
        assert!(value > dec!(0.0));
        assert!(value < dec!(20.0));
    }

    #[test]
    fn evaluates_black_scholes_delta() {
        let call = evaluate_constant_formula(FormulaKind::BlackScholesDelta {
            spot: Port::Constant(dec!(100.0)),
            strike: Port::Constant(dec!(100.0)),
            risk_free_rate: Port::Constant(dec!(0.05)),
            volatility: Port::Constant(dec!(0.2)),
            time_to_maturity: Port::Constant(dec!(1.0)),
            style: OptionStyle::Call,
        });
        assert!(call > dec!(0.0));

        let put = evaluate_constant_formula(FormulaKind::BlackScholesDelta {
            spot: Port::Constant(dec!(100.0)),
            strike: Port::Constant(dec!(100.0)),
            risk_free_rate: Port::Constant(dec!(0.05)),
            volatility: Port::Constant(dec!(0.2)),
            time_to_maturity: Port::Constant(dec!(1.0)),
            style: OptionStyle::Put,
        });
        assert!(put < dec!(0.0));
    }

    #[test]
    fn evaluates_black_scholes_gamma() {
        let value = evaluate_constant_formula(FormulaKind::BlackScholesGamma {
            spot: Port::Constant(dec!(100.0)),
            strike: Port::Constant(dec!(100.0)),
            risk_free_rate: Port::Constant(dec!(0.05)),
            volatility: Port::Constant(dec!(0.2)),
            time_to_maturity: Port::Constant(dec!(1.0)),
        });
        assert!(value > dec!(0.0));
    }

    #[test]
    fn evaluates_black_scholes_vega() {
        let value = evaluate_constant_formula(FormulaKind::BlackScholesVega {
            spot: Port::Constant(dec!(100.0)),
            strike: Port::Constant(dec!(100.0)),
            risk_free_rate: Port::Constant(dec!(0.05)),
            volatility: Port::Constant(dec!(0.2)),
            time_to_maturity: Port::Constant(dec!(1.0)),
        });
        assert!(value > dec!(0.0));
    }

    #[test]
    fn evaluates_black_scholes_theta() {
        let call = evaluate_constant_formula(FormulaKind::BlackScholesTheta {
            spot: Port::Constant(dec!(100.0)),
            strike: Port::Constant(dec!(100.0)),
            risk_free_rate: Port::Constant(dec!(0.05)),
            volatility: Port::Constant(dec!(0.2)),
            time_to_maturity: Port::Constant(dec!(1.0)),
            style: OptionStyle::Call,
        });
        assert!(call < dec!(0.0));

        let put = evaluate_constant_formula(FormulaKind::BlackScholesTheta {
            spot: Port::Constant(dec!(100.0)),
            strike: Port::Constant(dec!(100.0)),
            risk_free_rate: Port::Constant(dec!(0.05)),
            volatility: Port::Constant(dec!(0.2)),
            time_to_maturity: Port::Constant(dec!(1.0)),
            style: OptionStyle::Put,
        });
        assert!(put < dec!(0.0));
    }

    #[test]
    fn evaluates_black_scholes_rho() {
        let call = evaluate_constant_formula(FormulaKind::BlackScholesRho {
            spot: Port::Constant(dec!(100.0)),
            strike: Port::Constant(dec!(100.0)),
            risk_free_rate: Port::Constant(dec!(0.05)),
            volatility: Port::Constant(dec!(0.2)),
            time_to_maturity: Port::Constant(dec!(1.0)),
            style: OptionStyle::Call,
        });
        assert!(call > dec!(0.0));

        let put = evaluate_constant_formula(FormulaKind::BlackScholesRho {
            spot: Port::Constant(dec!(100.0)),
            strike: Port::Constant(dec!(100.0)),
            risk_free_rate: Port::Constant(dec!(0.05)),
            volatility: Port::Constant(dec!(0.2)),
            time_to_maturity: Port::Constant(dec!(1.0)),
            style: OptionStyle::Put,
        });
        assert!(put < dec!(0.0));
    }

    #[test]
    fn evaluates_growing_perpetuity_present_value() {
        let value = evaluate_constant_formula(FormulaKind::GrowingPerpetuityPresentValue {
            payment: Port::Constant(dec!(100.0)),
            rate: Port::Constant(dec!(0.08)),
            growth_rate: Port::Constant(dec!(0.03)),
        });
        assert_eq!(value, dec!(2000.0));
    }

    #[test]
    fn evaluates_continuous_compounding_future_value() {
        let value = evaluate_constant_formula(FormulaKind::ContinuousCompoundingFutureValue {
            present_value: Port::Constant(dec!(100.0)),
            rate: Port::Constant(dec!(0.05)),
            time: Port::Constant(dec!(10.0)),
        });
        assert_eq!(value.round_dp(4), dec!(164.8721));
    }

    #[test]
    fn evaluates_return_on_investment() {
        let value = evaluate_constant_formula(FormulaKind::ReturnOnInvestment {
            gain: Port::Constant(dec!(150.0)),
            cost: Port::Constant(dec!(100.0)),
        });
        assert_eq!(value, dec!(0.5));
    }

    #[test]
    fn evaluates_profit_margin() {
        let value = evaluate_constant_formula(FormulaKind::ProfitMargin {
            net_income: Port::Constant(dec!(150.0)),
            revenue: Port::Constant(dec!(1000.0)),
        });
        assert_eq!(value, dec!(0.15));
    }

    #[test]
    fn evaluates_asset_turnover() {
        let value = evaluate_constant_formula(FormulaKind::AssetTurnover {
            revenue: Port::Constant(dec!(1000.0)),
            total_assets: Port::Constant(dec!(500.0)),
        });
        assert_eq!(value, dec!(2.0));
    }

    #[test]
    fn evaluates_equity_multiplier() {
        let value = evaluate_constant_formula(FormulaKind::EquityMultiplier {
            total_assets: Port::Constant(dec!(2000.0)),
            shareholders_equity: Port::Constant(dec!(1000.0)),
        });
        assert_eq!(value, dec!(2.0));
    }

    #[test]
    fn evaluates_quick_ratio() {
        let value = evaluate_constant_formula(FormulaKind::QuickRatio {
            current_assets: Port::Constant(dec!(1000.0)),
            inventory: Port::Constant(dec!(300.0)),
            current_liabilities: Port::Constant(dec!(500.0)),
        });
        assert_eq!(value, dec!(1.4));
    }

    #[test]
    fn evaluates_interest_coverage() {
        let value = evaluate_constant_formula(FormulaKind::InterestCoverage {
            ebit: Port::Constant(dec!(500.0)),
            interest_expense: Port::Constant(dec!(100.0)),
        });
        assert_eq!(value, dec!(5.0));
    }

    #[test]
    fn evaluates_inventory_turnover() {
        let value = evaluate_constant_formula(FormulaKind::InventoryTurnover {
            cogs: Port::Constant(dec!(600.0)),
            inventory: Port::Constant(dec!(100.0)),
        });
        assert_eq!(value, dec!(6.0));
    }

    #[test]
    fn evaluates_cash_conversion_cycle() {
        let value = evaluate_constant_formula(FormulaKind::CashConversionCycle {
            days_inventory_outstanding: Port::Constant(dec!(30.0)),
            days_sales_outstanding: Port::Constant(dec!(45.0)),
            days_payables_outstanding: Port::Constant(dec!(25.0)),
        });
        assert_eq!(value, dec!(50.0));
    }

    #[test]
    fn evaluates_capital_adequacy_ratio() {
        let value = evaluate_constant_formula(FormulaKind::CapitalAdequacyRatio {
            total_capital: Port::Constant(dec!(100.0)),
            risk_weighted_assets: Port::Constant(dec!(1000.0)),
        });
        assert_eq!(value, dec!(0.1));
    }

    #[test]
    fn evaluates_provision_coverage_ratio() {
        let value = evaluate_constant_formula(FormulaKind::ProvisionCoverageRatio {
            provisions: Port::Constant(dec!(80.0)),
            non_performing_assets: Port::Constant(dec!(100.0)),
        });
        assert_eq!(value, dec!(0.8));
    }

    #[test]
    fn evaluates_treynor_ratio() {
        let value = evaluate_constant_formula(FormulaKind::TreynorRatio {
            portfolio_return: Port::Constant(dec!(0.12)),
            risk_free_rate: Port::Constant(dec!(0.03)),
            beta: Port::Constant(dec!(1.2)),
        });
        assert_eq!(value, dec!(0.075));
    }

    #[test]
    fn evaluates_value_at_risk() {
        let value = evaluate_constant_formula(FormulaKind::ValueAtRisk {
            portfolio_value: Port::Constant(dec!(100000.0)),
            mean_return: Port::Constant(dec!(0.10)),
            std_dev: Port::Constant(dec!(0.15)),
            z_score: Port::Constant(dec!(1.645)),
        });
        assert!(value < dec!(0.0));
    }

    #[test]
    fn evaluates_expected_shortfall() {
        let value = evaluate_constant_formula(FormulaKind::ExpectedShortfall {
            portfolio_value: Port::Constant(dec!(100000.0)),
            mean_return: Port::Constant(dec!(0.10)),
            std_dev: Port::Constant(dec!(0.15)),
            z_score: Port::Constant(dec!(1.645)),
        });
        assert!(value < dec!(0.0));
    }

    #[test]
    fn evaluates_discounted_cash_flow() {
        let value = evaluate_constant_formula(FormulaKind::DiscountedCashFlow {
            cash_flows: Port::Constant(dec!(100.0)),
            discount_rate: Port::Constant(dec!(0.05)),
        });
        assert_eq!(value.round_dp(2), dec!(95.24));
    }

    #[test]
    fn evaluates_macaulay_duration() {
        let value = evaluate_constant_formula(FormulaKind::MacaulayDuration {
            cash_flows: Port::Constant(dec!(100.0)),
            yield_per_period: Port::Constant(dec!(0.05)),
        });
        assert_eq!(value, dec!(1.0));
    }

    #[test]
    fn evaluates_modified_duration() {
        let value = evaluate_constant_formula(FormulaKind::ModifiedDuration {
            macaulay_duration: Port::Constant(dec!(1.967)),
            yield_per_period: Port::Constant(dec!(0.05)),
        });
        assert_eq!(value.round_dp(3), dec!(1.873));
    }

    #[test]
    fn evaluates_convexity() {
        let value = evaluate_constant_formula(FormulaKind::Convexity {
            cash_flows: Port::Constant(dec!(100.0)),
            yield_per_period: Port::Constant(dec!(0.05)),
        });
        assert!(value > dec!(0.0));
    }

    #[test]
    fn evaluates_free_cash_flow_to_equity() {
        let value = evaluate_constant_formula(FormulaKind::FreeCashFlowToEquity {
            fcff: Port::Constant(dec!(550.0)),
            interest_expense_after_tax: Port::Constant(dec!(35.0)),
            net_borrowing: Port::Constant(dec!(50.0)),
        });
        assert_eq!(value, dec!(565.0));
    }

    #[test]
    fn evaluates_economic_value_added() {
        let value = evaluate_constant_formula(FormulaKind::EconomicValueAdded {
            nopat: Port::Constant(dec!(200.0)),
            invested_capital: Port::Constant(dec!(1000.0)),
            wacc: Port::Constant(dec!(0.10)),
        });
        assert_eq!(value, dec!(100.0));
    }

    #[test]
    fn evaluates_internal_growth_rate() {
        let value = evaluate_constant_formula(FormulaKind::InternalGrowthRate {
            roe: Port::Constant(dec!(0.15)),
            dividend_payout_ratio: Port::Constant(dec!(0.40)),
        });
        assert_eq!(value.round_dp(4), dec!(0.0989));
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
