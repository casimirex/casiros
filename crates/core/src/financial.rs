//! Classic financial statement ratios and DuPont-style decompositions.

use super::prelude::*;
use rust_decimal::Decimal;

/// Computes Return on Equity (ROE).
///
/// # Mathematical Definition
///
/// \[ ROE = \frac{\text{Net Income}}{\text{Average Shareholders' Equity}} \]
///
/// # Errors
///
/// Returns [`CalculationError::DivisionByZero`] if `avg_shareholders_equity` is zero.
///
/// # Examples
///
/// ```
/// use casiros_core::financial::return_on_equity;
/// use rust_decimal_macros::dec;
///
/// let roe = return_on_equity(dec!(150.0), dec!(1000.0)).unwrap();
/// assert_eq!(roe, dec!(0.15));
/// assert!(roe > dec!(0.0)); // Assertion 2
/// ```
pub fn return_on_equity(
    net_income: Decimal,
    avg_shareholders_equity: Decimal,
) -> Result<Decimal, CalculationError> {
    if avg_shareholders_equity == Decimal::ZERO {
        return Err(CalculationError::DivisionByZero {
            formula: "Return on Equity (ROE)",
        });
    }
    return Ok(net_income / avg_shareholders_equity);
}

/// Computes Return on Assets (ROA).
///
/// # Mathematical Definition
///
/// \[ ROA = \frac{\text{Net Income}}{\text{Average Total Assets}} \]
///
/// # Errors
///
/// Returns [`CalculationError::DivisionByZero`] if `avg_total_assets` is zero.
///
/// # Examples
///
/// ```
/// use casiros_core::financial::return_on_assets;
/// use rust_decimal_macros::dec;
///
/// let roa = return_on_assets(dec!(150.0), dec!(2000.0)).unwrap();
/// assert_eq!(roa, dec!(0.075));
/// assert!(roa < dec!(0.10)); // Assertion 2
/// ```
pub fn return_on_assets(
    net_income: Decimal,
    avg_total_assets: Decimal,
) -> Result<Decimal, CalculationError> {
    if avg_total_assets == Decimal::ZERO {
        return Err(CalculationError::DivisionByZero {
            formula: "Return on Assets (ROA)",
        });
    }
    return Ok(net_income / avg_total_assets);
}

/// Computes the DuPont decomposition of ROE.
///
/// # Mathematical Definition
///
/// \[ ROE = \text{Profit Margin} \times \text{Asset Turnover} \times \text{Equity Multiplier} \]
///
/// # Errors
///
/// Never returns an error in the current implementation, but the `Result`
/// type is reserved for future defensive checks.
///
/// # Examples
///
/// ```
/// use casiros_core::financial::dupont_roe;
/// use rust_decimal_macros::dec;
///
/// let roe = dupont_roe(dec!(0.10), dec!(0.80), dec!(2.0)).unwrap();
/// assert_eq!(roe, dec!(0.16));
/// assert!(roe > dec!(0.10)); // Assertion 2
/// ```
pub fn dupont_roe(
    profit_margin: Decimal,
    asset_turnover: Decimal,
    equity_multiplier: Decimal,
) -> Result<Decimal, CalculationError> {
    return Ok(profit_margin * asset_turnover * equity_multiplier);
}

/// Computes the Current Ratio.
///
/// # Mathematical Definition
///
/// \[ \text{Current Ratio} = \frac{\text{Current Assets}}{\text{Current Liabilities}} \]
///
/// # Errors
///
/// Returns [`CalculationError::DivisionByZero`] if `current_liabilities` is zero.
///
/// # Examples
///
/// ```
/// use casiros_core::financial::current_ratio;
/// use rust_decimal_macros::dec;
///
/// let cr = current_ratio(dec!(1000.0), dec!(500.0)).unwrap();
/// assert_eq!(cr, dec!(2.0));
/// assert!(cr > dec!(1.0)); // Assertion 2
/// ```
pub fn current_ratio(
    current_assets: Decimal,
    current_liabilities: Decimal,
) -> Result<Decimal, CalculationError> {
    if current_liabilities == Decimal::ZERO {
        return Err(CalculationError::DivisionByZero {
            formula: "Current Ratio",
        });
    }
    return Ok(current_assets / current_liabilities);
}

/// Computes the Debt-to-Equity Ratio.
///
/// # Mathematical Definition
///
/// \[ D/E = \frac{\text{Total Liabilities}}{\text{Shareholders' Equity}} \]
///
/// # Errors
///
/// Returns [`CalculationError::DivisionByZero`] if `shareholders_equity` is zero.
///
/// # Examples
///
/// ```
/// use casiros_core::financial::debt_to_equity;
/// use rust_decimal_macros::dec;
///
/// let de = debt_to_equity(dec!(500.0), dec!(1000.0)).unwrap();
/// assert_eq!(de, dec!(0.5));
/// assert!(de < dec!(1.0)); // Assertion 2
/// ```
pub fn debt_to_equity(
    total_liabilities: Decimal,
    shareholders_equity: Decimal,
) -> Result<Decimal, CalculationError> {
    if shareholders_equity == Decimal::ZERO {
        return Err(CalculationError::DivisionByZero {
            formula: "Debt-to-Equity Ratio",
        });
    }
    return Ok(total_liabilities / shareholders_equity);
}
