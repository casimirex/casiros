//! Market and portfolio risk metrics.

use super::prelude::*;
use rust_decimal::Decimal;

/// Computes the Sharpe Ratio.
///
/// # Mathematical Definition
///
/// \[ S = \frac{R_p - R_f}{\sigma_p} \]
///
/// where \( R_p \) is the portfolio return, \( R_f \) is the risk-free rate,
/// and \( \sigma_p \) is the portfolio standard deviation.
///
/// # Errors
///
/// Returns [`CalculationError::DivisionByZero`] if `portfolio_std_dev` is zero.
///
/// # Examples
///
/// ```
/// use casiros_core::markets::sharpe_ratio;
/// use rust_decimal_macros::dec;
///
/// let sharpe = sharpe_ratio(dec!(0.12), dec!(0.03), dec!(0.15)).unwrap();
/// assert_eq!(sharpe, dec!(0.6));
/// assert!(sharpe > dec!(0.0)); // Assertion 2
/// ```
pub fn sharpe_ratio(
    portfolio_return: Decimal,
    risk_free_rate: Decimal,
    portfolio_std_dev: Decimal,
) -> Result<Decimal, CalculationError> {
    if portfolio_std_dev == Decimal::ZERO {
        return Err(CalculationError::DivisionByZero {
            formula: "Sharpe Ratio",
        });
    }
    let excess_return = portfolio_return - risk_free_rate;
    return Ok(excess_return / portfolio_std_dev);
}

/// Computes Jensen's Alpha.
///
/// # Mathematical Definition
///
/// \[ \alpha = R_p - \left[ R_f + \beta \times (R_m - R_f) \right] \]
///
/// where \( R_p \) is the portfolio return, \( R_f \) is the risk-free rate,
/// \( R_m \) is the market return, and \( \beta \) is the portfolio beta.
///
/// # Errors
///
/// Never returns an error in the current implementation, but the `Result`
/// type is reserved for future defensive checks.
///
/// # Examples
///
/// ```
/// use casiros_core::markets::jensens_alpha;
/// use rust_decimal_macros::dec;
///
/// let alpha = jensens_alpha(dec!(0.15), dec!(0.03), dec!(0.10), dec!(1.2)).unwrap();
/// assert_eq!(alpha, dec!(0.036));
/// assert!(alpha > dec!(0.0)); // Assertion 2: positive alpha
/// ```
pub fn jensens_alpha(
    portfolio_return: Decimal,
    risk_free_rate: Decimal,
    market_return: Decimal,
    beta: Decimal,
) -> Result<Decimal, CalculationError> {
    let expected_return = risk_free_rate + beta * (market_return - risk_free_rate);
    return Ok(portfolio_return - expected_return);
}
