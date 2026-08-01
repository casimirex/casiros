//! Corporate finance valuation and capital-structure formulas.

use super::prelude::*;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

/// Computes the Weighted Average Cost of Capital (WACC).
///
/// # Mathematical Definition
///
/// \[ WACC = \frac{E}{V} \times r_e + \frac{D}{V} \times r_d \times (1 - t) \]
///
/// where \( V = E + D \).
///
/// # Errors
///
/// Returns [`CalculationError::DivisionByZero`] if `equity_value + debt_value` is zero.
/// Returns [`CalculationError::RangeViolation`] if `tax_rate` is outside [0, 1].
///
/// # Examples
///
/// ```
/// use casiros_core::corporate::wacc;
/// use rust_decimal_macros::dec;
///
/// let w = wacc(dec!(600.0), dec!(400.0), dec!(0.12), dec!(0.06), dec!(0.30)).unwrap();
/// assert_eq!(w.round_dp(4), dec!(0.0888));
/// assert!(w > dec!(0.05)); // Assertion 2
/// ```
pub fn wacc(
    equity_value: Decimal,
    debt_value: Decimal,
    cost_of_equity: Decimal,
    cost_of_debt: Decimal,
    tax_rate: Decimal,
) -> Result<Decimal, CalculationError> {
    let total_value = equity_value + debt_value;
    if total_value == Decimal::ZERO {
        return Err(CalculationError::DivisionByZero { formula: "WACC" });
    }
    if tax_rate < Decimal::ZERO || tax_rate > dec!(1.0) {
        return Err(CalculationError::RangeViolation {
            context: "WACC - tax_rate",
            value: tax_rate,
        });
    }
    let equity_weight = equity_value / total_value;
    let debt_weight = debt_value / total_value;
    let after_tax_cost_of_debt = cost_of_debt * (dec!(1.0) - tax_rate);
    return Ok(equity_weight * cost_of_equity + debt_weight * after_tax_cost_of_debt);
}

/// Computes Free Cash Flow to Firm (FCFF).
///
/// # Mathematical Definition
///
/// \[ FCFF = EBIT \times (1 - t) + D&A - \Delta WC - CapEx \]
///
/// # Errors
///
/// Returns [`CalculationError::RangeViolation`] if `tax_rate` is outside [0, 1].
///
/// # Examples
///
/// ```
/// use casiros_core::corporate::free_cash_flow_to_firm;
/// use rust_decimal_macros::dec;
///
/// let fcff = free_cash_flow_to_firm(
///     dec!(1000.0), dec!(0.30), dec!(100.0), dec!(50.0), dec!(200.0)
/// ).unwrap();
/// assert_eq!(fcff, dec!(550.0));
/// assert!(fcff > dec!(0.0)); // Assertion 2
/// ```
pub fn free_cash_flow_to_firm(
    ebit: Decimal,
    tax_rate: Decimal,
    depreciation: Decimal,
    delta_working_capital: Decimal,
    capex: Decimal,
) -> Result<Decimal, CalculationError> {
    if tax_rate < Decimal::ZERO || tax_rate > dec!(1.0) {
        return Err(CalculationError::RangeViolation {
            context: "FCFF - tax_rate",
            value: tax_rate,
        });
    }
    let nopat = ebit * (dec!(1.0) - tax_rate);
    return Ok(nopat + depreciation - delta_working_capital - capex);
}

/// Computes the Sustainable Growth Rate (SGR).
///
/// # Mathematical Definition
///
/// \[ SGR = ROE \times (1 - \text{Dividend Payout Ratio}) \]
///
/// # Errors
///
/// Returns [`CalculationError::RangeViolation`] if `dividend_payout_ratio` is outside [0, 1].
///
/// # Examples
///
/// ```
/// use casiros_core::corporate::sustainable_growth_rate;
/// use rust_decimal_macros::dec;
///
/// let sgr = sustainable_growth_rate(dec!(0.15), dec!(0.40)).unwrap();
/// assert_eq!(sgr, dec!(0.09));
/// assert!(sgr > dec!(0.0)); // Assertion 2
/// ```
pub fn sustainable_growth_rate(
    roe: Decimal,
    dividend_payout_ratio: Decimal,
) -> Result<Decimal, CalculationError> {
    if dividend_payout_ratio < Decimal::ZERO || dividend_payout_ratio > dec!(1.0) {
        return Err(CalculationError::RangeViolation {
            context: "SGR - dividend_payout_ratio",
            value: dividend_payout_ratio,
        });
    }
    let retention_ratio = dec!(1.0) - dividend_payout_ratio;
    return Ok(roe * retention_ratio);
}
