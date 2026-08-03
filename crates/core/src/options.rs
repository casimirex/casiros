//! Option pricing formulas.
//!
//! Black-Scholes closed-form pricing uses `f64` internally for the standard
//! normal distribution and transcendental functions. Inputs and outputs are
/// `Decimal` to preserve the crate's financial boundary contract.
use super::prelude::*;
use rust_decimal::Decimal;

/// Standard normal probability density function at `x`.
fn normal_pdf(x: f64) -> f64 {
    return (-0.5 * x * x).exp() / (2.0 * std::f64::consts::PI).sqrt();
}

/// Standard normal cumulative distribution function at `x`.
///
/// Uses the Hastings approximation (Abramowitz & Stegun 26.2.17) with a maximum
/// absolute error of `7.5e-8`, sufficient for typical option pricing use cases.
#[allow(clippy::suboptimal_flops)]
fn normal_cdf(x: f64) -> f64 {
    if x.is_nan() {
        return f64::NAN;
    }

    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    let abs_x = x.abs();
    let t = 1.0 / (1.0 + 0.231_641_9 * abs_x);

    let polynomial = 0.319_381_530 * t - 0.356_563_782 * t.powi(2) + 1.781_477_937 * t.powi(3)
        - 1.821_255_978 * t.powi(4)
        + 1.330_274_429 * t.powi(5);

    let cdf = 1.0 - normal_pdf(abs_x) * polynomial;
    return if sign < 0.0 { 1.0 - cdf } else { cdf };
}

/// Computes the Black-Scholes price of a European call option.
///
/// # Mathematical Definition
///
/// \\[ C = S_0 N(d_1) - K e^{-rT} N(d_2) \\]
///
/// where
/// \\[ d_1 = \\frac{\\ln(S_0/K) + (r + \\sigma^2/2)T}{\\sigma \\sqrt{T}} \\]
/// and `d_2 = d_1 - sigma * sqrt(T)`.
///
/// # Errors
///
/// Returns [`CalculationError::NegativeValueInvalid`] if any of `spot`, `strike`,
/// `risk_free_rate`, or `volatility` is negative, or if `time_to_maturity` is
/// not strictly positive.
/// Returns [`CalculationError::Overflow`] if the intermediate `f64` computation
/// overflows.
///
/// # Examples
///
/// ```
/// use casiros_core::options::black_scholes_call;
/// use rust_decimal_macros::dec;
///
/// let call = black_scholes_call(dec!(100.0), dec!(100.0), dec!(0.05), dec!(0.2), dec!(1.0)).unwrap();
/// assert!(call > dec!(0.0));
/// assert!(call < dec!(20.0));
/// ```
#[allow(clippy::cast_precision_loss)]
pub fn black_scholes_call(
    spot: Decimal,
    strike: Decimal,
    risk_free_rate: Decimal,
    volatility: Decimal,
    time_to_maturity: Decimal,
) -> Result<Decimal, CalculationError> {
    validate_black_scholes_inputs(spot, strike, risk_free_rate, volatility, time_to_maturity)?;

    let s: f64 = spot.try_into().map_err(|_| {
        return CalculationError::Overflow {
            formula: "black_scholes_call",
        };
    })?;
    let k: f64 = strike.try_into().map_err(|_| {
        return CalculationError::Overflow {
            formula: "black_scholes_call",
        };
    })?;
    let r: f64 = risk_free_rate.try_into().map_err(|_| {
        return CalculationError::Overflow {
            formula: "black_scholes_call",
        };
    })?;
    let sigma: f64 = volatility.try_into().map_err(|_| {
        return CalculationError::Overflow {
            formula: "black_scholes_call",
        };
    })?;
    let t: f64 = time_to_maturity.try_into().map_err(|_| {
        return CalculationError::Overflow {
            formula: "black_scholes_call",
        };
    })?;

    let sqrt_t = t.sqrt();
    let d1 = ((s / k).ln() + (r + 0.5 * sigma * sigma) * t) / (sigma * sqrt_t);
    let d2 = d1 - sigma * sqrt_t;

    let price = s * normal_cdf(d1) - k * (-r * t).exp() * normal_cdf(d2);
    return Decimal::try_from(price).map_err(|_| {
        return CalculationError::Overflow {
            formula: "black_scholes_call",
        };
    });
}

/// Computes the Black-Scholes price of a European put option.
///
/// # Mathematical Definition
///
/// \\[ P = K e^{-rT} N(-d_2) - S_0 N(-d_1) \\]
///
/// # Errors
///
/// Same validation as [`black_scholes_call`].
///
/// # Examples
///
/// ```
/// use casiros_core::options::black_scholes_put;
/// use rust_decimal_macros::dec;
///
/// let put = black_scholes_put(dec!(100.0), dec!(100.0), dec!(0.05), dec!(0.2), dec!(1.0)).unwrap();
/// assert!(put > dec!(0.0));
/// assert!(put < dec!(20.0));
/// ```
#[allow(clippy::cast_precision_loss)]
pub fn black_scholes_put(
    spot: Decimal,
    strike: Decimal,
    risk_free_rate: Decimal,
    volatility: Decimal,
    time_to_maturity: Decimal,
) -> Result<Decimal, CalculationError> {
    validate_black_scholes_inputs(spot, strike, risk_free_rate, volatility, time_to_maturity)?;

    let s: f64 = spot.try_into().map_err(|_| {
        return CalculationError::Overflow {
            formula: "black_scholes_put",
        };
    })?;
    let k: f64 = strike.try_into().map_err(|_| {
        return CalculationError::Overflow {
            formula: "black_scholes_put",
        };
    })?;
    let r: f64 = risk_free_rate.try_into().map_err(|_| {
        return CalculationError::Overflow {
            formula: "black_scholes_put",
        };
    })?;
    let sigma: f64 = volatility.try_into().map_err(|_| {
        return CalculationError::Overflow {
            formula: "black_scholes_put",
        };
    })?;
    let t: f64 = time_to_maturity.try_into().map_err(|_| {
        return CalculationError::Overflow {
            formula: "black_scholes_put",
        };
    })?;

    let sqrt_t = t.sqrt();
    let d1 = ((s / k).ln() + (r + 0.5 * sigma * sigma) * t) / (sigma * sqrt_t);
    let d2 = d1 - sigma * sqrt_t;

    let price = k * (-r * t).exp() * normal_cdf(-d2) - s * normal_cdf(-d1);
    return Decimal::try_from(price).map_err(|_| {
        return CalculationError::Overflow {
            formula: "black_scholes_put",
        };
    });
}

/// Validates Black-Scholes numeric preconditions.
///
/// Returns [`CalculationError::NegativeValueInvalid`] if any of `spot`, `strike`,
/// `risk_free_rate`, or `volatility` is negative, or if `time_to_maturity` is
/// not strictly positive.
fn validate_black_scholes_inputs(
    spot: Decimal,
    strike: Decimal,
    risk_free_rate: Decimal,
    volatility: Decimal,
    time_to_maturity: Decimal,
) -> Result<(), CalculationError> {
    if spot < Decimal::ZERO {
        return Err(CalculationError::NegativeValueInvalid {
            context: "black_scholes - spot",
            value: spot,
        });
    }
    if strike < Decimal::ZERO {
        return Err(CalculationError::NegativeValueInvalid {
            context: "black_scholes - strike",
            value: strike,
        });
    }
    if risk_free_rate < Decimal::ZERO {
        return Err(CalculationError::NegativeValueInvalid {
            context: "black_scholes - risk_free_rate",
            value: risk_free_rate,
        });
    }
    if volatility < Decimal::ZERO {
        return Err(CalculationError::NegativeValueInvalid {
            context: "black_scholes - volatility",
            value: volatility,
        });
    }
    if time_to_maturity <= Decimal::ZERO {
        return Err(CalculationError::NegativeValueInvalid {
            context: "black_scholes - time_to_maturity",
            value: time_to_maturity,
        });
    }
    return Ok(());
}
