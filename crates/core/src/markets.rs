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

/// Calculates the beta coefficient — systematic risk relative to the market.
///
/// Beta measures the sensitivity of an asset's returns to market returns.
/// A beta of 1.0 means the asset moves with the market; >1.0 means more
/// volatile; <1.0 means less volatile.
///
/// # Mathematical Definition
///
/// \[ \beta = \frac{\text{Cov}(R_i, R_m)}{\text{Var}(R_m)} \]
///
/// # Constraints
///
/// - Both slices MUST have the same length.
/// - The market variance MUST be positive.
///
/// # Errors
///
/// Returns [`CalculationError::InvalidInput`] if the slices are empty or
/// have different lengths, or if market variance is zero.
///
/// # Examples
///
/// ```
/// use casiros_core::markets::beta;
/// use rust_decimal_macros::dec;
///
/// let asset_returns = [dec!(0.05), dec!(0.02), dec!(-0.01), dec!(0.03)];
/// let market_returns = [dec!(0.03), dec!(0.01), dec!(-0.02), dec!(0.02)];
/// let b = beta(&asset_returns, &market_returns).unwrap();
/// assert!(b > dec!(0));
/// ```
pub fn beta(
    asset_returns: &[Decimal],
    market_returns: &[Decimal],
) -> Result<Decimal, CalculationError> {
    if asset_returns.is_empty() || market_returns.is_empty() {
        return Err(CalculationError::InvalidInput {
            message: "return slices must not be empty".to_string(),
        });
    }
    if asset_returns.len() != market_returns.len() {
        return Err(CalculationError::InvalidInput {
            message: "return slices must have the same length".to_string(),
        });
    }

    let n = Decimal::from(asset_returns.len());
    let sum_asset: Decimal = asset_returns.iter().copied().sum();
    let sum_market: Decimal = market_returns.iter().copied().sum();
    let mean_asset = sum_asset / n;
    let mean_market = sum_market / n;

    let mut cov = Decimal::ZERO;
    let mut var = Decimal::ZERO;
    for (a, m) in asset_returns.iter().zip(market_returns.iter()) {
        let dev_asset = a - mean_asset;
        let dev_market = m - mean_market;
        cov += dev_asset * dev_market;
        var += dev_market * dev_market;
    }

    if var == Decimal::ZERO {
        return Err(CalculationError::InvalidInput {
            message: "market variance is zero".to_string(),
        });
    }

    return Ok(cov / var);
}

/// Calculates the Sortino ratio — downside risk-adjusted return.
///
/// Unlike the Sharpe ratio, the Sortino ratio only penalises downside
/// volatility, making it a better measure for asymmetric return distributions.
///
/// # Mathematical Definition
///
/// \[ \text{Sortino} = \frac{R_p - R_f}{\sigma_d} \]
///
/// where \(\sigma_d\) is the downside deviation (standard deviation of
/// negative excess returns).
///
/// # Constraints
///
/// - `returns` MUST not be empty.
/// - `downside_deviation` MUST be positive.
///
/// # Errors
///
/// Returns [`CalculationError::InvalidInput`] if returns are empty or
/// downside deviation is zero.
///
/// # Examples
///
/// ```
/// use casiros_core::markets::sortino_ratio;
/// use rust_decimal_macros::dec;
///
/// let ratio = sortino_ratio(dec!(0.12), dec!(0.02), dec!(0.08)).unwrap();
/// assert!(ratio > dec!(0));
/// ```
pub fn sortino_ratio(
    portfolio_return: Decimal,
    risk_free_rate: Decimal,
    downside_deviation: Decimal,
) -> Result<Decimal, CalculationError> {
    if downside_deviation <= Decimal::ZERO {
        return Err(CalculationError::InvalidInput {
            message: "downside deviation must be positive".to_string(),
        });
    }
    return Ok((portfolio_return - risk_free_rate) / downside_deviation);
}

/// Calculates the Calmar ratio — return relative to maximum drawdown.
///
/// The Calmar ratio measures the ratio of the compound annual growth rate
/// (CAGR) to the maximum drawdown, providing a risk-adjusted return metric
/// focused on downside risk.
///
/// # Mathematical Definition
///
/// \[ \text{Calmar} = \frac{\text{CAGR}}{\text{Max Drawdown}} \]
///
/// # Constraints
///
/// - `max_drawdown` MUST be positive (expressed as a positive number).
///
/// # Errors
///
/// Returns [`CalculationError::InvalidInput`] if max drawdown is zero.
///
/// # Examples
///
/// ```
/// use casiros_core::markets::calmar_ratio;
/// use rust_decimal_macros::dec;
///
/// let ratio = calmar_ratio(dec!(0.15), dec!(0.20)).unwrap();
/// assert!(ratio < dec!(1));
/// ```
pub fn calmar_ratio(
    cagr: Decimal,
    max_drawdown: Decimal,
) -> Result<Decimal, CalculationError> {
    if max_drawdown <= Decimal::ZERO {
        return Err(CalculationError::InvalidInput {
            message: "max drawdown must be positive".to_string(),
        });
    }
    return Ok(cagr / max_drawdown);
}
