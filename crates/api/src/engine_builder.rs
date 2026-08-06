//! Build a [`casiros_dag::graph::CausalityEngine`] from JSON request models.
//!
//! This module translates the flat, serialization-friendly API types into the
//! richer graph types used by the application layer. All errors are reported as
//! human-readable strings suitable for HTTP 400 responses.

use std::collections::HashMap;
use std::sync::Arc;

use casiros_dag::cache::FormulaCache;
use casiros_dag::graph::{CausalityEngine, FormulaKind, NodeId, Port};
use rust_decimal::Decimal;

use crate::models::{
    DistributionRequest, EdgeRequest, FormulaRequest, NodeRequest, OptionStyle, PortRequest,
};

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

    /// Attaches a formula result cache to the engine.
    #[must_use]
    pub fn with_cache(mut self, cache: Arc<dyn FormulaCache>) -> Self {
        self.engine = self.engine.with_cache(cache);
        return self;
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

    #[allow(clippy::too_many_lines)]
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
            FormulaRequest::AmortizationPayment {
                principal,
                rate,
                periods,
            } => Ok(FormulaKind::AmortizationPayment {
                principal: self.port(principal)?,
                rate: self.port(rate)?,
                periods: self.port(periods)?,
            }),
            FormulaRequest::YieldToMaturityApproximation {
                face_value,
                coupon_payment,
                price,
                periods,
            } => Ok(FormulaKind::YieldToMaturityApproximation {
                face_value: self.port(face_value)?,
                coupon_payment: self.port(coupon_payment)?,
                price: self.port(price)?,
                periods: self.port(periods)?,
            }),
            FormulaRequest::SimpleMovingAverage { prices, window } => {
                Ok(FormulaKind::SimpleMovingAverage {
                    prices: self.port(prices)?,
                    window: self.port(window)?,
                })
            }
            FormulaRequest::BlackScholesCall {
                spot,
                strike,
                risk_free_rate,
                volatility,
                time_to_maturity,
            } => Ok(FormulaKind::BlackScholesCall {
                spot: self.port(spot)?,
                strike: self.port(strike)?,
                risk_free_rate: self.port(risk_free_rate)?,
                volatility: self.port(volatility)?,
                time_to_maturity: self.port(time_to_maturity)?,
            }),
            FormulaRequest::BlackScholesPut {
                spot,
                strike,
                risk_free_rate,
                volatility,
                time_to_maturity,
            } => Ok(FormulaKind::BlackScholesPut {
                spot: self.port(spot)?,
                strike: self.port(strike)?,
                risk_free_rate: self.port(risk_free_rate)?,
                volatility: self.port(volatility)?,
                time_to_maturity: self.port(time_to_maturity)?,
            }),
            FormulaRequest::BinomialOptionCall {
                spot,
                strike,
                risk_free_rate,
                volatility,
                time_to_maturity,
                steps,
            } => Ok(FormulaKind::BinomialOptionCall {
                spot: self.port(spot)?,
                strike: self.port(strike)?,
                risk_free_rate: self.port(risk_free_rate)?,
                volatility: self.port(volatility)?,
                time_to_maturity: self.port(time_to_maturity)?,
                steps: self.port(steps)?,
            }),
            FormulaRequest::BinomialOptionPut {
                spot,
                strike,
                risk_free_rate,
                volatility,
                time_to_maturity,
                steps,
            } => Ok(FormulaKind::BinomialOptionPut {
                spot: self.port(spot)?,
                strike: self.port(strike)?,
                risk_free_rate: self.port(risk_free_rate)?,
                volatility: self.port(volatility)?,
                time_to_maturity: self.port(time_to_maturity)?,
                steps: self.port(steps)?,
            }),
            FormulaRequest::BlackScholesDelta {
                spot,
                strike,
                risk_free_rate,
                volatility,
                time_to_maturity,
                style,
            } => Ok(FormulaKind::BlackScholesDelta {
                spot: self.port(spot)?,
                strike: self.port(strike)?,
                risk_free_rate: self.port(risk_free_rate)?,
                volatility: self.port(volatility)?,
                time_to_maturity: self.port(time_to_maturity)?,
                style: Self::option_style(*style),
            }),
            FormulaRequest::BlackScholesGamma {
                spot,
                strike,
                risk_free_rate,
                volatility,
                time_to_maturity,
            } => Ok(FormulaKind::BlackScholesGamma {
                spot: self.port(spot)?,
                strike: self.port(strike)?,
                risk_free_rate: self.port(risk_free_rate)?,
                volatility: self.port(volatility)?,
                time_to_maturity: self.port(time_to_maturity)?,
            }),
            FormulaRequest::BlackScholesVega {
                spot,
                strike,
                risk_free_rate,
                volatility,
                time_to_maturity,
            } => Ok(FormulaKind::BlackScholesVega {
                spot: self.port(spot)?,
                strike: self.port(strike)?,
                risk_free_rate: self.port(risk_free_rate)?,
                volatility: self.port(volatility)?,
                time_to_maturity: self.port(time_to_maturity)?,
            }),
            FormulaRequest::BlackScholesTheta {
                spot,
                strike,
                risk_free_rate,
                volatility,
                time_to_maturity,
                style,
            } => Ok(FormulaKind::BlackScholesTheta {
                spot: self.port(spot)?,
                strike: self.port(strike)?,
                risk_free_rate: self.port(risk_free_rate)?,
                volatility: self.port(volatility)?,
                time_to_maturity: self.port(time_to_maturity)?,
                style: Self::option_style(*style),
            }),
            FormulaRequest::BlackScholesRho {
                spot,
                strike,
                risk_free_rate,
                volatility,
                time_to_maturity,
                style,
            } => Ok(FormulaKind::BlackScholesRho {
                spot: self.port(spot)?,
                strike: self.port(strike)?,
                risk_free_rate: self.port(risk_free_rate)?,
                volatility: self.port(volatility)?,
                time_to_maturity: self.port(time_to_maturity)?,
                style: Self::option_style(*style),
            }),
            FormulaRequest::GrowingPerpetuityPresentValue {
                payment,
                rate,
                growth_rate,
            } => Ok(FormulaKind::GrowingPerpetuityPresentValue {
                payment: self.port(payment)?,
                rate: self.port(rate)?,
                growth_rate: self.port(growth_rate)?,
            }),
            FormulaRequest::ContinuousCompoundingFutureValue {
                present_value,
                rate,
                time,
            } => Ok(FormulaKind::ContinuousCompoundingFutureValue {
                present_value: self.port(present_value)?,
                rate: self.port(rate)?,
                time: self.port(time)?,
            }),
            FormulaRequest::ReturnOnInvestment { gain, cost } => {
                Ok(FormulaKind::ReturnOnInvestment {
                    gain: self.port(gain)?,
                    cost: self.port(cost)?,
                })
            }
            FormulaRequest::ProfitMargin {
                net_income,
                revenue,
            } => Ok(FormulaKind::ProfitMargin {
                net_income: self.port(net_income)?,
                revenue: self.port(revenue)?,
            }),
            FormulaRequest::AssetTurnover {
                revenue,
                total_assets,
            } => Ok(FormulaKind::AssetTurnover {
                revenue: self.port(revenue)?,
                total_assets: self.port(total_assets)?,
            }),
            FormulaRequest::EquityMultiplier {
                total_assets,
                shareholders_equity,
            } => Ok(FormulaKind::EquityMultiplier {
                total_assets: self.port(total_assets)?,
                shareholders_equity: self.port(shareholders_equity)?,
            }),
            FormulaRequest::QuickRatio {
                current_assets,
                inventory,
                current_liabilities,
            } => Ok(FormulaKind::QuickRatio {
                current_assets: self.port(current_assets)?,
                inventory: self.port(inventory)?,
                current_liabilities: self.port(current_liabilities)?,
            }),
            FormulaRequest::InterestCoverage {
                ebit,
                interest_expense,
            } => Ok(FormulaKind::InterestCoverage {
                ebit: self.port(ebit)?,
                interest_expense: self.port(interest_expense)?,
            }),
            FormulaRequest::InventoryTurnover { cogs, inventory } => {
                Ok(FormulaKind::InventoryTurnover {
                    cogs: self.port(cogs)?,
                    inventory: self.port(inventory)?,
                })
            }
            FormulaRequest::CashConversionCycle {
                days_inventory_outstanding,
                days_sales_outstanding,
                days_payables_outstanding,
            } => Ok(FormulaKind::CashConversionCycle {
                days_inventory_outstanding: self.port(days_inventory_outstanding)?,
                days_sales_outstanding: self.port(days_sales_outstanding)?,
                days_payables_outstanding: self.port(days_payables_outstanding)?,
            }),
            FormulaRequest::CapitalAdequacyRatio {
                total_capital,
                risk_weighted_assets,
            } => Ok(FormulaKind::CapitalAdequacyRatio {
                total_capital: self.port(total_capital)?,
                risk_weighted_assets: self.port(risk_weighted_assets)?,
            }),
            FormulaRequest::ProvisionCoverageRatio {
                provisions,
                non_performing_assets,
            } => Ok(FormulaKind::ProvisionCoverageRatio {
                provisions: self.port(provisions)?,
                non_performing_assets: self.port(non_performing_assets)?,
            }),
            FormulaRequest::TreynorRatio {
                portfolio_return,
                risk_free_rate,
                beta,
            } => Ok(FormulaKind::TreynorRatio {
                portfolio_return: self.port(portfolio_return)?,
                risk_free_rate: self.port(risk_free_rate)?,
                beta: self.port(beta)?,
            }),
            FormulaRequest::ValueAtRisk {
                portfolio_value,
                mean_return,
                std_dev,
                z_score,
            } => Ok(FormulaKind::ValueAtRisk {
                portfolio_value: self.port(portfolio_value)?,
                mean_return: self.port(mean_return)?,
                std_dev: self.port(std_dev)?,
                z_score: self.port(z_score)?,
            }),
            FormulaRequest::ExpectedShortfall {
                portfolio_value,
                mean_return,
                std_dev,
                z_score,
            } => Ok(FormulaKind::ExpectedShortfall {
                portfolio_value: self.port(portfolio_value)?,
                mean_return: self.port(mean_return)?,
                std_dev: self.port(std_dev)?,
                z_score: self.port(z_score)?,
            }),
            FormulaRequest::DiscountedCashFlow {
                cash_flows,
                discount_rate,
            } => Ok(FormulaKind::DiscountedCashFlow {
                cash_flows: self.port(cash_flows)?,
                discount_rate: self.port(discount_rate)?,
            }),
            FormulaRequest::MacaulayDuration {
                cash_flows,
                yield_per_period,
            } => Ok(FormulaKind::MacaulayDuration {
                cash_flows: self.port(cash_flows)?,
                yield_per_period: self.port(yield_per_period)?,
            }),
            FormulaRequest::ModifiedDuration {
                macaulay_duration,
                yield_per_period,
            } => Ok(FormulaKind::ModifiedDuration {
                macaulay_duration: self.port(macaulay_duration)?,
                yield_per_period: self.port(yield_per_period)?,
            }),
            FormulaRequest::Convexity {
                cash_flows,
                yield_per_period,
            } => Ok(FormulaKind::Convexity {
                cash_flows: self.port(cash_flows)?,
                yield_per_period: self.port(yield_per_period)?,
            }),
            FormulaRequest::FreeCashFlowToEquity {
                fcff,
                interest_expense_after_tax,
                net_borrowing,
            } => Ok(FormulaKind::FreeCashFlowToEquity {
                fcff: self.port(fcff)?,
                interest_expense_after_tax: self.port(interest_expense_after_tax)?,
                net_borrowing: self.port(net_borrowing)?,
            }),
            FormulaRequest::EconomicValueAdded {
                nopat,
                invested_capital,
                wacc,
            } => Ok(FormulaKind::EconomicValueAdded {
                nopat: self.port(nopat)?,
                invested_capital: self.port(invested_capital)?,
                wacc: self.port(wacc)?,
            }),
            FormulaRequest::InternalGrowthRate {
                roe,
                dividend_payout_ratio,
            } => Ok(FormulaKind::InternalGrowthRate {
                roe: self.port(roe)?,
                dividend_payout_ratio: self.port(dividend_payout_ratio)?,
            }),
            FormulaRequest::Beta {
                asset_returns,
                market_returns,
            } => Ok(FormulaKind::Beta {
                asset_returns: self.port(asset_returns)?,
                market_returns: self.port(market_returns)?,
            }),
            FormulaRequest::SortinoRatio {
                portfolio_return,
                risk_free_rate,
                downside_deviation,
            } => Ok(FormulaKind::SortinoRatio {
                portfolio_return: self.port(portfolio_return)?,
                risk_free_rate: self.port(risk_free_rate)?,
                downside_deviation: self.port(downside_deviation)?,
            }),
            FormulaRequest::CalmarRatio { cagr, max_drawdown } => Ok(FormulaKind::CalmarRatio {
                cagr: self.port(cagr)?,
                max_drawdown: self.port(max_drawdown)?,
            }),
            FormulaRequest::AltmanZScore {
                working_capital_to_assets,
                retained_earnings_to_assets,
                ebit_to_assets,
                equity_to_liabilities,
                sales_to_assets,
            } => Ok(FormulaKind::AltmanZScore {
                working_capital_to_assets: self.port(working_capital_to_assets)?,
                retained_earnings_to_assets: self.port(retained_earnings_to_assets)?,
                ebit_to_assets: self.port(ebit_to_assets)?,
                equity_to_liabilities: self.port(equity_to_liabilities)?,
                sales_to_assets: self.port(sales_to_assets)?,
            }),
            FormulaRequest::TaxShield { tax_rate, debt } => Ok(FormulaKind::TaxShield {
                tax_rate: self.port(tax_rate)?,
                debt: self.port(debt)?,
            }),
            FormulaRequest::AdjustedPresentValue {
                unlevered_npv,
                pv_tax_shield,
            } => Ok(FormulaKind::AdjustedPresentValue {
                unlevered_npv: self.port(unlevered_npv)?,
                pv_tax_shield: self.port(pv_tax_shield)?,
            }),
            FormulaRequest::NetPresentValue { rate, cash_flows } => {
                Ok(FormulaKind::NetPresentValue {
                    rate: self.port(rate)?,
                    cash_flows: self.port(cash_flows)?,
                })
            }
            FormulaRequest::InternalRateOfReturn { cash_flows } => {
                Ok(FormulaKind::InternalRateOfReturn {
                    cash_flows: self.port(cash_flows)?,
                })
            }
            FormulaRequest::AnnuityPresentValue {
                payment,
                rate,
                periods,
            } => Ok(FormulaKind::AnnuityPresentValue {
                payment: self.port(payment)?,
                rate: self.port(rate)?,
                periods: self.port(periods)?,
            }),
            FormulaRequest::AnnuityFutureValue {
                payment,
                rate,
                periods,
            } => Ok(FormulaKind::AnnuityFutureValue {
                payment: self.port(payment)?,
                rate: self.port(rate)?,
                periods: self.port(periods)?,
            }),
            FormulaRequest::PerpetuityPresentValue { payment, rate } => {
                Ok(FormulaKind::PerpetuityPresentValue {
                    payment: self.port(payment)?,
                    rate: self.port(rate)?,
                })
            }
            FormulaRequest::EffectiveAnnualRate {
                nominal_rate,
                compounding_periods,
            } => Ok(FormulaKind::EffectiveAnnualRate {
                nominal_rate: self.port(nominal_rate)?,
                compounding_periods: self.port(compounding_periods)?,
            }),
            FormulaRequest::ReturnOnAssets {
                net_income,
                avg_total_assets,
            } => Ok(FormulaKind::ReturnOnAssets {
                net_income: self.port(net_income)?,
                avg_total_assets: self.port(avg_total_assets)?,
            }),
            FormulaRequest::DupontRoe {
                profit_margin,
                asset_turnover,
                equity_multiplier,
            } => Ok(FormulaKind::DupontRoe {
                profit_margin: self.port(profit_margin)?,
                asset_turnover: self.port(asset_turnover)?,
                equity_multiplier: self.port(equity_multiplier)?,
            }),
            FormulaRequest::CurrentRatio {
                current_assets,
                current_liabilities,
            } => Ok(FormulaKind::CurrentRatio {
                current_assets: self.port(current_assets)?,
                current_liabilities: self.port(current_liabilities)?,
            }),
            FormulaRequest::DebtToEquity {
                total_liabilities,
                shareholders_equity,
            } => Ok(FormulaKind::DebtToEquity {
                total_liabilities: self.port(total_liabilities)?,
                shareholders_equity: self.port(shareholders_equity)?,
            }),
            FormulaRequest::NetInterestMargin {
                interest_income,
                interest_expense,
                avg_earning_assets,
            } => Ok(FormulaKind::NetInterestMargin {
                interest_income: self.port(interest_income)?,
                interest_expense: self.port(interest_expense)?,
                avg_earning_assets: self.port(avg_earning_assets)?,
            }),
            FormulaRequest::LoanToDepositRatio {
                total_loans,
                total_deposits,
            } => Ok(FormulaKind::LoanToDepositRatio {
                total_loans: self.port(total_loans)?,
                total_deposits: self.port(total_deposits)?,
            }),
            FormulaRequest::SharpeRatio {
                portfolio_return,
                risk_free_rate,
                portfolio_std_dev,
            } => Ok(FormulaKind::SharpeRatio {
                portfolio_return: self.port(portfolio_return)?,
                risk_free_rate: self.port(risk_free_rate)?,
                portfolio_std_dev: self.port(portfolio_std_dev)?,
            }),
            FormulaRequest::JensensAlpha {
                portfolio_return,
                risk_free_rate,
                market_return,
                beta,
            } => Ok(FormulaKind::JensensAlpha {
                portfolio_return: self.port(portfolio_return)?,
                risk_free_rate: self.port(risk_free_rate)?,
                market_return: self.port(market_return)?,
                beta: self.port(beta)?,
            }),
            FormulaRequest::DividendDiscountModel {
                next_dividend,
                required_return,
                growth_rate,
            } => Ok(FormulaKind::DividendDiscountModel {
                next_dividend: self.port(next_dividend)?,
                required_return: self.port(required_return)?,
                growth_rate: self.port(growth_rate)?,
            }),
            FormulaRequest::BondPrice {
                face_value,
                coupon_payment,
                yield_per_period,
                periods,
            } => Ok(FormulaKind::BondPrice {
                face_value: self.port(face_value)?,
                coupon_payment: self.port(coupon_payment)?,
                yield_per_period: self.port(yield_per_period)?,
                periods: self.port(periods)?,
            }),
            FormulaRequest::FreeCashFlowToFirm {
                ebit,
                tax_rate,
                depreciation,
                delta_working_capital,
                capex,
            } => Ok(FormulaKind::FreeCashFlowToFirm {
                ebit: self.port(ebit)?,
                tax_rate: self.port(tax_rate)?,
                depreciation: self.port(depreciation)?,
                delta_working_capital: self.port(delta_working_capital)?,
                capex: self.port(capex)?,
            }),
        }
    }

    fn option_style(style: OptionStyle) -> casiros_dag::graph::OptionStyle {
        return match style {
            OptionStyle::Call => casiros_dag::graph::OptionStyle::Call,
            OptionStyle::Put => casiros_dag::graph::OptionStyle::Put,
        };
    }

    fn port(&self, request: &PortRequest) -> Result<Port, EngineBuilderError> {
        match request {
            PortRequest::Constant(value) => Ok(Port::Constant(*value)),
            PortRequest::Series(values) => Ok(Port::Series(values.clone())),
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
