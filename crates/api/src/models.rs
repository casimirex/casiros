//! JSON request and response models for the CASIROS HTTP API.
//!
//! These models are intentionally flat and serialization-friendly. They are the
//! public contract of the REST API and are translated into the richer domain
//! types provided by [`casiros_dag`] and [`casiros_simulator`].

#![allow(clippy::large_stack_arrays)]

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// Side of an option contract used by option-related formula requests.
#[derive(Debug, Clone, Copy, Deserialize, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum OptionStyle {
    /// A call option gives the holder the right to buy the underlying.
    Call,
    /// A put option gives the holder the right to sell the underlying.
    Put,
}

/// A single node in a DAG request.
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
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
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
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

    /// Amortization payment.
    AmortizationPayment {
        /// Loan principal binding.
        principal: PortRequest,
        /// Periodic rate binding.
        rate: PortRequest,
        /// Number of periods binding.
        periods: PortRequest,
    },

    /// Yield-to-maturity approximation for a bond.
    YieldToMaturityApproximation {
        /// Face value binding.
        face_value: PortRequest,
        /// Periodic coupon payment binding.
        coupon_payment: PortRequest,
        /// Current market price binding.
        price: PortRequest,
        /// Periods to maturity binding.
        periods: PortRequest,
    },

    /// Simple moving average over a price series.
    SimpleMovingAverage {
        /// Comma-separated price series binding.
        prices: PortRequest,
        /// Window size binding.
        window: PortRequest,
    },

    /// Black-Scholes European call option price.
    BlackScholesCall {
        /// Spot price binding.
        spot: PortRequest,
        /// Strike price binding.
        strike: PortRequest,
        /// Risk-free rate binding.
        risk_free_rate: PortRequest,
        /// Volatility binding.
        volatility: PortRequest,
        /// Time to maturity binding.
        time_to_maturity: PortRequest,
    },

    /// Black-Scholes European put option price.
    BlackScholesPut {
        /// Spot price binding.
        spot: PortRequest,
        /// Strike price binding.
        strike: PortRequest,
        /// Risk-free rate binding.
        risk_free_rate: PortRequest,
        /// Volatility binding.
        volatility: PortRequest,
        /// Time to maturity binding.
        time_to_maturity: PortRequest,
    },

    /// Cox-Ross-Rubinstein binomial tree European call option price.
    BinomialOptionCall {
        /// Spot price binding.
        spot: PortRequest,
        /// Strike price binding.
        strike: PortRequest,
        /// Risk-free rate binding.
        risk_free_rate: PortRequest,
        /// Volatility binding.
        volatility: PortRequest,
        /// Time to maturity binding.
        time_to_maturity: PortRequest,
        /// Number of time-steps binding.
        steps: PortRequest,
    },

    /// Cox-Ross-Rubinstein binomial tree European put option price.
    BinomialOptionPut {
        /// Spot price binding.
        spot: PortRequest,
        /// Strike price binding.
        strike: PortRequest,
        /// Risk-free rate binding.
        risk_free_rate: PortRequest,
        /// Volatility binding.
        volatility: PortRequest,
        /// Time to maturity binding.
        time_to_maturity: PortRequest,
        /// Number of time-steps binding.
        steps: PortRequest,
    },

    /// Black-Scholes delta of a European option.
    BlackScholesDelta {
        /// Spot price binding.
        spot: PortRequest,
        /// Strike price binding.
        strike: PortRequest,
        /// Risk-free rate binding.
        risk_free_rate: PortRequest,
        /// Volatility binding.
        volatility: PortRequest,
        /// Time to maturity binding.
        time_to_maturity: PortRequest,
        /// Option style.
        style: OptionStyle,
    },

    /// Black-Scholes gamma of a European option.
    BlackScholesGamma {
        /// Spot price binding.
        spot: PortRequest,
        /// Strike price binding.
        strike: PortRequest,
        /// Risk-free rate binding.
        risk_free_rate: PortRequest,
        /// Volatility binding.
        volatility: PortRequest,
        /// Time to maturity binding.
        time_to_maturity: PortRequest,
    },

    /// Black-Scholes vega of a European option.
    BlackScholesVega {
        /// Spot price binding.
        spot: PortRequest,
        /// Strike price binding.
        strike: PortRequest,
        /// Risk-free rate binding.
        risk_free_rate: PortRequest,
        /// Volatility binding.
        volatility: PortRequest,
        /// Time to maturity binding.
        time_to_maturity: PortRequest,
    },

    /// Black-Scholes theta of a European option.
    BlackScholesTheta {
        /// Spot price binding.
        spot: PortRequest,
        /// Strike price binding.
        strike: PortRequest,
        /// Risk-free rate binding.
        risk_free_rate: PortRequest,
        /// Volatility binding.
        volatility: PortRequest,
        /// Time to maturity binding.
        time_to_maturity: PortRequest,
        /// Option style.
        style: OptionStyle,
    },

    /// Black-Scholes rho of a European option.
    BlackScholesRho {
        /// Spot price binding.
        spot: PortRequest,
        /// Strike price binding.
        strike: PortRequest,
        /// Risk-free rate binding.
        risk_free_rate: PortRequest,
        /// Volatility binding.
        volatility: PortRequest,
        /// Time to maturity binding.
        time_to_maturity: PortRequest,
        /// Option style.
        style: OptionStyle,
    },

    /// Present value of a growing perpetuity.
    GrowingPerpetuityPresentValue {
        /// Periodic payment binding.
        payment: PortRequest,
        /// Discount rate binding.
        rate: PortRequest,
        /// Perpetual growth rate binding.
        growth_rate: PortRequest,
    },

    /// Future value with continuous compounding.
    ContinuousCompoundingFutureValue {
        /// Present value binding.
        present_value: PortRequest,
        /// Continuous rate binding.
        rate: PortRequest,
        /// Time horizon binding.
        time: PortRequest,
    },

    /// Return on investment.
    ReturnOnInvestment {
        /// Gain binding.
        gain: PortRequest,
        /// Cost binding.
        cost: PortRequest,
    },

    /// Profit margin.
    ProfitMargin {
        /// Net income binding.
        net_income: PortRequest,
        /// Revenue binding.
        revenue: PortRequest,
    },

    /// Asset turnover.
    AssetTurnover {
        /// Revenue binding.
        revenue: PortRequest,
        /// Total assets binding.
        total_assets: PortRequest,
    },

    /// Equity multiplier.
    EquityMultiplier {
        /// Total assets binding.
        total_assets: PortRequest,
        /// Shareholders' equity binding.
        shareholders_equity: PortRequest,
    },

    /// Quick ratio.
    QuickRatio {
        /// Current assets binding.
        current_assets: PortRequest,
        /// Inventory binding.
        inventory: PortRequest,
        /// Current liabilities binding.
        current_liabilities: PortRequest,
    },

    /// Interest coverage ratio.
    InterestCoverage {
        /// EBIT binding.
        ebit: PortRequest,
        /// Interest expense binding.
        interest_expense: PortRequest,
    },

    /// Inventory turnover.
    InventoryTurnover {
        /// Cost of goods sold binding.
        cogs: PortRequest,
        /// Inventory binding.
        inventory: PortRequest,
    },

    /// Cash conversion cycle.
    CashConversionCycle {
        /// Days inventory outstanding binding.
        days_inventory_outstanding: PortRequest,
        /// Days sales outstanding binding.
        days_sales_outstanding: PortRequest,
        /// Days payables outstanding binding.
        days_payables_outstanding: PortRequest,
    },

    /// Capital adequacy ratio.
    CapitalAdequacyRatio {
        /// Total capital binding.
        total_capital: PortRequest,
        /// Risk-weighted assets binding.
        risk_weighted_assets: PortRequest,
    },

    /// Provision coverage ratio.
    ProvisionCoverageRatio {
        /// Provisions binding.
        provisions: PortRequest,
        /// Non-performing assets binding.
        non_performing_assets: PortRequest,
    },

    /// Treynor ratio.
    TreynorRatio {
        /// Portfolio return binding.
        portfolio_return: PortRequest,
        /// Risk-free rate binding.
        risk_free_rate: PortRequest,
        /// Beta binding.
        beta: PortRequest,
    },

    /// Value at Risk.
    ValueAtRisk {
        /// Portfolio value binding.
        portfolio_value: PortRequest,
        /// Mean return binding.
        mean_return: PortRequest,
        /// Standard deviation binding.
        std_dev: PortRequest,
        /// Z-score binding.
        z_score: PortRequest,
    },

    /// Expected shortfall.
    ExpectedShortfall {
        /// Portfolio value binding.
        portfolio_value: PortRequest,
        /// Mean return binding.
        mean_return: PortRequest,
        /// Standard deviation binding.
        std_dev: PortRequest,
        /// Z-score binding.
        z_score: PortRequest,
    },

    /// Discounted cash flow over a comma-separated cash-flow series.
    DiscountedCashFlow {
        /// Comma-separated cash-flow series binding.
        cash_flows: PortRequest,
        /// Discount rate binding.
        discount_rate: PortRequest,
    },

    /// Macaulay duration over a comma-separated cash-flow series.
    MacaulayDuration {
        /// Comma-separated cash-flow series binding.
        cash_flows: PortRequest,
        /// Yield per period binding.
        yield_per_period: PortRequest,
    },

    /// Modified duration from Macaulay duration.
    ModifiedDuration {
        /// Macaulay duration binding.
        macaulay_duration: PortRequest,
        /// Yield per period binding.
        yield_per_period: PortRequest,
    },

    /// Convexity over a comma-separated cash-flow series.
    Convexity {
        /// Comma-separated cash-flow series binding.
        cash_flows: PortRequest,
        /// Yield per period binding.
        yield_per_period: PortRequest,
    },

    /// Free cash flow to equity.
    FreeCashFlowToEquity {
        /// Free cash flow to firm binding.
        fcff: PortRequest,
        /// Interest expense after tax binding.
        interest_expense_after_tax: PortRequest,
        /// Net borrowing binding.
        net_borrowing: PortRequest,
    },

    /// Economic value added.
    EconomicValueAdded {
        /// NOPAT binding.
        nopat: PortRequest,
        /// Invested capital binding.
        invested_capital: PortRequest,
        /// WACC binding.
        wacc: PortRequest,
    },

    /// Internal growth rate.
    InternalGrowthRate {
        /// ROE binding.
        roe: PortRequest,
        /// Dividend payout ratio binding.
        dividend_payout_ratio: PortRequest,
    },
}

/// A port binding: either a literal value or a reference to another node by
/// name.
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
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
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct EdgeRequest {
    /// Name of the dependency node.
    pub dependency: String,
    /// Name of the dependent node.
    pub dependent: String,
}

/// Request body for `POST /evaluate`.
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct EvaluateRequest {
    /// Nodes that make up the DAG.
    pub nodes: Vec<NodeRequest>,
    /// Directed edges between nodes.
    pub edges: Vec<EdgeRequest>,
    /// Values for every input node, keyed by node name.
    pub inputs: std::collections::HashMap<String, Decimal>,
}

/// Response body for `POST /evaluate`.
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct EvaluateResponse {
    /// Computed value for every node, keyed by node name.
    pub outputs: std::collections::HashMap<String, Decimal>,
}

/// Response body for `GET /healthz`.
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct HealthzResponse {
    /// Always `"ok"` when the service is healthy.
    pub status: String,
}

impl HealthzResponse {
    /// Returns the canonical healthy response.
    #[must_use]
    pub fn ok() -> Self {
        return Self {
            status: "ok".to_string(),
        };
    }
}

/// A distribution request used by the simulator.
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
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
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct BindingRequest {
    /// Name of the input node.
    pub node: String,
    /// Distribution to sample from.
    pub distribution: DistributionRequest,
}

/// Request body for `POST /simulate`.
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
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

/// Error response body returned for invalid requests.
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct ErrorResponse {
    /// Human-readable error message.
    pub error: String,
}

/// Response body for `POST /simulate`.
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
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

/// Request body for `POST /snapshots`.
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct SaveSnapshotRequest {
    /// Unique identifier for the snapshot.
    pub id: String,

    /// Nodes that make up the DAG.
    pub nodes: Vec<NodeRequest>,

    /// Directed edges between nodes.
    pub edges: Vec<EdgeRequest>,
}

/// Response body for `POST /snapshots`.
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct SaveSnapshotResponse {
    /// Identifier of the saved snapshot.
    pub id: String,
}

/// Request body for `DELETE /snapshots/{id}`.
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct DeleteSnapshotRequest {
    /// Identifier of the snapshot to delete.
    pub id: String,
}

/// Response body for `GET /snapshots/{id}`.
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct SnapshotResponse {
    /// Identifier of the snapshot.
    pub id: String,

    /// Snapshot payload as a JSON object.
    pub data: serde_json::Value,
}

/// A single snapshot entry returned by `GET /snapshots`.
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct SnapshotSummaryResponse {
    /// Identifier of the snapshot.
    pub id: String,
}

/// Response body for `GET /snapshots`.
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct SnapshotListResponse {
    /// List of stored snapshots.
    pub snapshots: Vec<SnapshotSummaryResponse>,
}

/// Query parameters for `GET /audit`.
#[derive(Debug, Clone, Default, Deserialize, Serialize, ToSchema)]
pub struct AuditListQuery {
    /// Page size. Clamped to `1..=1000` by the domain layer.
    pub limit: Option<u32>,

    /// Number of rows to skip.
    pub offset: Option<u32>,
}

/// A single audit event returned by `GET /audit`.
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct AuditEventResponse {
    /// Unique event identifier.
    pub id: String,

    /// RFC 3339 timestamp of the event.
    pub timestamp: String,

    /// Tenant that owned the request.
    pub tenant_id: String,

    /// Workspace in which the request ran.
    pub workspace_id: String,

    /// Identifier of the API key used.
    pub api_key_id: String,

    /// Action that was attempted.
    pub action: String,

    /// Resource the action addressed.
    pub resource: String,

    /// Outcome of the attempt.
    pub result: String,

    /// Failure detail, when the outcome was an error.
    pub error: Option<String>,

    /// Contextual metadata such as HTTP method and status.
    pub metadata: std::collections::HashMap<String, String>,
}

impl AuditEventResponse {
    /// Converts a domain [`casiros_core::audit::AuditEvent`] into its wire form.
    ///
    /// Timestamps are rendered as RFC 3339. A timestamp that cannot be formatted
    /// falls back to an empty string rather than failing the whole response.
    #[must_use]
    pub fn from_event(event: &casiros_core::audit::AuditEvent) -> Self {
        return Self {
            id: event.id.to_string(),
            timestamp: event
                .timestamp
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap_or_default(),
            tenant_id: event.principal.tenant_id.as_str().to_string(),
            workspace_id: event.principal.workspace_id.as_str().to_string(),
            api_key_id: event.principal.api_key_id.clone(),
            action: event.action.as_str().to_string(),
            resource: event.resource.clone(),
            result: event.result.as_str().to_string(),
            error: event.result.error_message().map(String::from),
            metadata: event.metadata.clone(),
        };
    }
}

/// Response body for `GET /audit`.
#[derive(Debug, Clone, Deserialize, Serialize, ToSchema)]
pub struct AuditListResponse {
    /// Number of events in this page.
    pub total: usize,

    /// The events, newest first.
    pub events: Vec<AuditEventResponse>,
}
