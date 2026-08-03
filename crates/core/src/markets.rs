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

/// Computes the simple moving average (SMA) of a price series.
///
/// # Mathematical Definition
///
/// \\[ SMA = \\frac{1}{n} \\sum_{i=0}^{n-1} P_i \\]
///
/// where `n` is the window size and `P_i` are the most recent prices.
///
/// # Errors
///
/// Returns [`CalculationError::DivisionByZero`] if `window` is zero.
/// Returns [`CalculationError::Overflow`] if `window` exceeds the number of
/// provided prices.
///
/// # Examples
///
/// ```
/// use casiros_core::markets::simple_moving_average;
/// use rust_decimal_macros::dec;
///
/// let prices = vec![dec!(10.0), dec!(20.0), dec!(30.0)];
/// let sma = simple_moving_average(&prices, 3).unwrap();
/// assert_eq!(sma, dec!(20.0));
/// ```
pub fn simple_moving_average(
    prices: &[Decimal],
    window: usize,
) -> Result<Decimal, CalculationError> {
    if window == 0 {
        return Err(CalculationError::DivisionByZero {
            formula: "Simple Moving Average",
        });
    }
    if window > prices.len() {
        return Err(CalculationError::Overflow {
            formula: "Simple Moving Average",
        });
    }

    let sum: Decimal = prices.iter().rev().take(window).copied().sum();
    return Ok(sum / Decimal::from(window));
}

/// Computes the Treynor Ratio.
///
/// # Mathematical Definition
///
/// \[ T = \frac{R_p - R_f}{\beta} \]
///
/// where \( R_p \) is the portfolio return, \( R_f \) is the risk-free rate,
/// and \( \beta \) is the portfolio beta.
///
/// # Errors
///
/// Returns [`CalculationError::DivisionByZero`] if `beta` is zero.
///
/// # Examples
///
/// ```
/// use casiros_core::markets::treynor_ratio;
/// use rust_decimal_macros::dec;
///
/// let treynor = treynor_ratio(dec!(0.12), dec!(0.03), dec!(1.2)).unwrap();
/// assert_eq!(treynor, dec!(0.075));
/// assert!(treynor > dec!(0.0)); // Assertion 2
/// ```
pub fn treynor_ratio(
    portfolio_return: Decimal,
    risk_free_rate: Decimal,
    beta: Decimal,
) -> Result<Decimal, CalculationError> {
    if beta == Decimal::ZERO {
        return Err(CalculationError::DivisionByZero {
            formula: "Treynor Ratio",
        });
    }
    return Ok((portfolio_return - risk_free_rate) / beta);
}

/// Computes Value at Risk (VaR) using the parametric method.
///
/// # Mathematical Definition
///
/// \[ VaR = PV \times (\mu - z \times \sigma) \]
///
/// where \( PV \) is the portfolio value, \( \mu \) is the mean return,
/// \( \sigma \) is the standard deviation, and \( z \) is the z-score.
///
/// # Errors
///
/// Returns [`CalculationError::NegativeValueInvalid`] if `portfolio_value` is negative.
///
/// # Examples
///
/// ```
/// use casiros_core::markets::value_at_risk;
/// use rust_decimal_macros::dec;
///
/// let var = value_at_risk(dec!(100000.0), dec!(0.10), dec!(0.15), dec!(1.645)).unwrap();
/// assert!(var < dec!(0.0)); // Assertion 1: VaR is a loss
/// assert!(var.abs() < dec!(25000.0)); // Assertion 2
/// ```
pub fn value_at_risk(
    portfolio_value: Decimal,
    mean_return: Decimal,
    std_dev: Decimal,
    z_score: Decimal,
) -> Result<Decimal, CalculationError> {
    if portfolio_value < Decimal::ZERO {
        return Err(CalculationError::NegativeValueInvalid {
            context: "value_at_risk - portfolio_value",
            value: portfolio_value,
        });
    }
    return Ok(portfolio_value * (mean_return - z_score * std_dev));
}

/// Computes the Expected Shortfall (CVaR) using a simple VaR approximation.
///
/// # Mathematical Definition
///
/// \[ CVaR \approx PV \times (\mu - (z + 1) \times \sigma) \]
///
/// where \( PV \) is the portfolio value, \( \mu \) is the mean return,
/// \( \sigma \) is the standard deviation, and \( z \) is the z-score.
///
/// # Errors
///
/// Returns [`CalculationError::NegativeValueInvalid`] if `portfolio_value` is negative.
///
/// # Examples
///
/// ```
/// use casiros_core::markets::expected_shortfall;
/// use rust_decimal_macros::dec;
///
/// let cvar = expected_shortfall(dec!(100000.0), dec!(0.10), dec!(0.15), dec!(1.645)).unwrap();
/// assert!(cvar < dec!(0.0)); // Assertion 1: CVaR is a loss
/// assert!(cvar.abs() > dec!(25000.0)); // Assertion 2
/// ```
pub fn expected_shortfall(
    portfolio_value: Decimal,
    mean_return: Decimal,
    std_dev: Decimal,
    z_score: Decimal,
) -> Result<Decimal, CalculationError> {
    if portfolio_value < Decimal::ZERO {
        return Err(CalculationError::NegativeValueInvalid {
            context: "expected_shortfall - portfolio_value",
            value: portfolio_value,
        });
    }
    let adjusted_z = z_score + dec!(1.0);
    return Ok(portfolio_value * (mean_return - adjusted_z * std_dev));
}
