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

/// Computes Free Cash Flow to Equity (FCFE).
///
/// # Mathematical Definition
///
/// \[ FCFE = FCFF - \text{Interest}(1 - t) + \text{Net Borrowing} \]
///
/// # Errors
///
/// Never returns an error in the current implementation, but the `Result`
/// type is reserved for future defensive checks.
///
/// # Examples
///
/// ```
/// use casiros_core::corporate::free_cash_flow_to_equity;
/// use rust_decimal_macros::dec;
///
/// let fcfe = free_cash_flow_to_equity(dec!(550.0), dec!(35.0), dec!(50.0)).unwrap();
/// assert_eq!(fcfe, dec!(565.0));
/// assert!(fcfe > dec!(0.0)); // Assertion 2
/// ```
pub fn free_cash_flow_to_equity(
    fcff: Decimal,
    interest_expense_after_tax: Decimal,
    net_borrowing: Decimal,
) -> Result<Decimal, CalculationError> {
    return Ok(fcff - interest_expense_after_tax + net_borrowing);
}

/// Computes Economic Value Added (EVA).
///
/// # Mathematical Definition
///
/// \[ EVA = NOPAT - (IC \times WACC) \]
///
/// where \( NOPAT \) is net operating profit after tax, \( IC \) is invested
/// capital, and \( WACC \) is the weighted average cost of capital.
///
/// # Errors
///
/// Never returns an error in the current implementation, but the `Result`
/// type is reserved for future defensive checks.
///
/// # Examples
///
/// ```
/// use casiros_core::corporate::economic_value_added;
/// use rust_decimal_macros::dec;
///
/// let eva = economic_value_added(dec!(200.0), dec!(1000.0), dec!(0.10)).unwrap();
/// assert_eq!(eva, dec!(100.0));
/// assert!(eva > dec!(0.0)); // Assertion 2
/// ```
pub fn economic_value_added(
    nopat: Decimal,
    invested_capital: Decimal,
    wacc: Decimal,
) -> Result<Decimal, CalculationError> {
    return Ok(nopat - (invested_capital * wacc));
}

/// Computes the Internal Growth Rate (IGR).
///
/// # Mathematical Definition
///
/// \[ IGR = \frac{\text{return_on_assets} \times (1 - \text{DPR})}{1 - \text{return_on_assets} \times (1 - \text{DPR})} \]
///
/// where \( \text{return_on_assets} \) is the return on assets and \( \text{DPR} \)
/// is the dividend payout ratio.
///
/// # Errors
///
/// Returns [`CalculationError::RangeViolation`] if `dividend_payout_ratio` is
/// outside [0, 1].
/// Returns [`CalculationError::DivisionByZero`] if the denominator is zero
/// or negative.
///
/// # Examples
///
/// ```
/// use casiros_core::corporate::internal_growth_rate;
/// use rust_decimal_macros::dec;
///
/// let igr = internal_growth_rate(dec!(0.15), dec!(0.40)).unwrap();
/// assert_eq!(igr.round_dp(4), dec!(0.0989));
/// assert!(igr > dec!(0.0)); // Assertion 2
/// ```
pub fn internal_growth_rate(
    roe: Decimal,
    dividend_payout_ratio: Decimal,
) -> Result<Decimal, CalculationError> {
    if dividend_payout_ratio < Decimal::ZERO || dividend_payout_ratio > dec!(1.0) {
        return Err(CalculationError::RangeViolation {
            context: "IGR - dividend_payout_ratio",
            value: dividend_payout_ratio,
        });
    }
    let retention = dec!(1.0) - dividend_payout_ratio;
    let numerator = roe * retention;
    let denominator = dec!(1.0) - numerator;
    if denominator <= Decimal::ZERO {
        return Err(CalculationError::DivisionByZero {
            formula: "Internal Growth Rate",
        });
    }
    return Ok(numerator / denominator);
}

/// Calculates the present value of the tax shield from debt financing.
///
/// The tax shield is the reduction in taxable income from interest payments
/// on debt. Its present value is the corporate tax rate times the debt amount.
///
/// # Mathematical Definition
///
/// \[ PV(\text{Tax Shield}) = T_c \times D \]
///
/// where \(T_c\) is the corporate tax rate and \(D\) is the debt amount.
///
/// # Constraints
///
/// - `tax_rate` MUST be in the range [0, 1].
/// - `debt` MUST be non-negative.
///
/// # Errors
///
/// Returns [`CalculationError::RangeViolation`] if `tax_rate` is outside [0, 1].
/// Returns [`CalculationError::NegativeValueInvalid`] if `debt` is negative.
///
/// # Examples
///
/// ```
/// use casiros_core::corporate::tax_shield;
/// use rust_decimal_macros::dec;
///
/// let shield = tax_shield(dec!(0.21), dec!(1_000_000)).unwrap();
/// assert_eq!(shield, dec!(210_000));
/// ```
pub fn tax_shield(tax_rate: Decimal, debt: Decimal) -> Result<Decimal, CalculationError> {
    if tax_rate < Decimal::ZERO || tax_rate > Decimal::ONE {
        return Err(CalculationError::RangeViolation {
            context: "tax_shield - tax_rate",
            value: tax_rate,
        });
    }
    if debt < Decimal::ZERO {
        return Err(CalculationError::NegativeValueInvalid {
            context: "tax_shield - debt",
            value: debt,
        });
    }
    return Ok(tax_rate * debt);
}

/// Calculates the adjusted present value (APV) of a project or firm.
///
/// APV separates the value of the unlevered project from the financing
/// side effects (primarily the tax shield on debt).
///
/// # Mathematical Definition
///
/// \[ APV = NPV_{\text{unlevered}} + PV(\text{Tax Shield}) \]
///
/// # Constraints
///
/// All inputs MUST be non-negative.
///
/// # Errors
///
/// Returns [`CalculationError::NegativeValueInvalid`] if any input is negative.
///
/// # Examples
///
/// ```
/// use casiros_core::corporate::adjusted_present_value;
/// use rust_decimal_macros::dec;
///
/// let apv = adjusted_present_value(dec!(500_000), dec!(210_000)).unwrap();
/// assert_eq!(apv, dec!(710_000));
/// ```
pub fn adjusted_present_value(
    unlevered_npv: Decimal,
    pv_tax_shield: Decimal,
) -> Result<Decimal, CalculationError> {
    if unlevered_npv < Decimal::ZERO {
        return Err(CalculationError::NegativeValueInvalid {
            context: "adjusted_present_value - unlevered_npv",
            value: unlevered_npv,
        });
    }
    if pv_tax_shield < Decimal::ZERO {
        return Err(CalculationError::NegativeValueInvalid {
            context: "adjusted_present_value - pv_tax_shield",
            value: pv_tax_shield,
        });
    }
    return Ok(unlevered_npv + pv_tax_shield);
}
