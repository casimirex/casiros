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

/// Side of an option contract used by Greek and binomial formulas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptionStyle {
    /// A call option gives the holder the right to buy the underlying.
    Call,
    /// A put option gives the holder the right to sell the underlying.
    Put,
}

/// Computes the price of a European call option using the Cox-Ross-Rubinstein
/// binomial tree model.
///
/// # Parameters
///
/// - `steps`: Number of time-steps in the tree. Must be strictly positive.
///
/// # Errors
///
/// Returns [`CalculationError::NegativeValueInvalid`] if any of the continuous
/// inputs is invalid, or if `steps` is zero.
/// Returns [`CalculationError::Overflow`] if any `f64` intermediate overflows.
///
/// # Examples
///
/// ```
/// use casiros_core::options::binomial_option_call;
/// use rust_decimal_macros::dec;
///
/// let call = binomial_option_call(dec!(100.0), dec!(100.0), dec!(0.05), dec!(0.2), dec!(1.0), 100).unwrap();
/// assert!(call > dec!(0.0));
/// assert!(call < dec!(20.0));
/// ```
#[allow(clippy::cast_precision_loss)]
pub fn binomial_option_call(
    spot: Decimal,
    strike: Decimal,
    risk_free_rate: Decimal,
    volatility: Decimal,
    time_to_maturity: Decimal,
    steps: u32,
) -> Result<Decimal, CalculationError> {
    return binomial_option(
        spot,
        strike,
        risk_free_rate,
        volatility,
        time_to_maturity,
        steps,
        OptionStyle::Call,
    );
}

/// Computes the price of a European put option using the Cox-Ross-Rubinstein
/// binomial tree model.
///
/// # Parameters
///
/// - `steps`: Number of time-steps in the tree. Must be strictly positive.
///
/// # Errors
///
/// Returns [`CalculationError::NegativeValueInvalid`] if any of the continuous
/// inputs is invalid, or if `steps` is zero.
/// Returns [`CalculationError::Overflow`] if any `f64` intermediate overflows.
///
/// # Examples
///
/// ```
/// use casiros_core::options::binomial_option_put;
/// use rust_decimal_macros::dec;
///
/// let put = binomial_option_put(dec!(100.0), dec!(100.0), dec!(0.05), dec!(0.2), dec!(1.0), 100).unwrap();
/// assert!(put > dec!(0.0));
/// assert!(put < dec!(20.0));
/// ```
#[allow(clippy::cast_precision_loss)]
pub fn binomial_option_put(
    spot: Decimal,
    strike: Decimal,
    risk_free_rate: Decimal,
    volatility: Decimal,
    time_to_maturity: Decimal,
    steps: u32,
) -> Result<Decimal, CalculationError> {
    return binomial_option(
        spot,
        strike,
        risk_free_rate,
        volatility,
        time_to_maturity,
        steps,
        OptionStyle::Put,
    );
}

/// Shared implementation for Cox-Ross-Rubinstein binomial option pricing.
#[allow(clippy::cast_precision_loss, clippy::many_single_char_names)]
fn binomial_option(
    spot: Decimal,
    strike: Decimal,
    risk_free_rate: Decimal,
    volatility: Decimal,
    time_to_maturity: Decimal,
    steps: u32,
    style: OptionStyle,
) -> Result<Decimal, CalculationError> {
    validate_black_scholes_inputs(spot, strike, risk_free_rate, volatility, time_to_maturity)?;
    if steps == 0 {
        return Err(CalculationError::NegativeValueInvalid {
            context: "binomial - steps",
            value: Decimal::from(steps),
        });
    }

    let s: f64 = decimal_to_f64(spot, "binomial_option")?;
    let k: f64 = decimal_to_f64(strike, "binomial_option")?;
    let r: f64 = decimal_to_f64(risk_free_rate, "binomial_option")?;
    let sigma: f64 = decimal_to_f64(volatility, "binomial_option")?;
    let t: f64 = decimal_to_f64(time_to_maturity, "binomial_option")?;

    let n = f64::from(steps);
    let dt = t / n;
    let sqrt_dt = dt.sqrt();
    let up = (sigma * sqrt_dt).exp();
    let down = (-sigma * sqrt_dt).exp();
    let discount = (r * dt).exp();
    let p = (discount - down) / (up - down);

    if !(0.0..=1.0).contains(&p) {
        return Err(CalculationError::Overflow {
            formula: "binomial_option",
        });
    }

    let mut values = vec![0.0_f64; (steps + 1) as usize];
    for j in 0..=steps {
        let j_f = f64::from(j);
        let price = s * up.powf(j_f) * down.powf(n - j_f);
        let intrinsic = match style {
            OptionStyle::Call => price - k,
            OptionStyle::Put => k - price,
        };
        values[j as usize] = intrinsic.max(0.0);
    }

    for _ in 0..steps {
        for j in 0..values.len() - 1 {
            values[j] = (p * values[j + 1] + (1.0 - p) * values[j]) / discount;
        }
    }

    return Decimal::try_from(values[0]).map_err(|_| {
        return CalculationError::Overflow {
            formula: "binomial_option",
        };
    });
}

/// Converts a `Decimal` into an `f64`, reporting an overflow error on failure.
fn decimal_to_f64(value: Decimal, formula: &'static str) -> Result<f64, CalculationError> {
    return value.try_into().map_err(|_| {
        return CalculationError::Overflow { formula };
    });
}

/// Computes `d_1` and `d_2` for the Black-Scholes family of formulas.
#[allow(clippy::suboptimal_flops)]
fn black_scholes_d1_d2(
    spot: f64,
    strike: f64,
    risk_free_rate: f64,
    volatility: f64,
    time_to_maturity: f64,
) -> (f64, f64) {
    let sqrt_t = time_to_maturity.sqrt();
    let d1 = ((spot / strike).ln()
        + (risk_free_rate + 0.5 * volatility * volatility) * time_to_maturity)
        / (volatility * sqrt_t);
    let d2 = d1 - volatility * sqrt_t;
    return (d1, d2);
}

/// Computes the Black-Scholes delta of an option.
///
/// # Mathematical Definition
///
/// - Call: `N(d_1)`
/// - Put: `N(d_1) - 1`
///
/// # Errors
///
/// Same validation as [`black_scholes_call`].
///
/// # Examples
///
/// ```
/// use casiros_core::options::{black_scholes_delta, OptionStyle};
/// use rust_decimal_macros::dec;
///
/// let call_delta = black_scholes_delta(dec!(100.0), dec!(100.0), dec!(0.05), dec!(0.2), dec!(1.0), OptionStyle::Call).unwrap();
/// assert!(call_delta > dec!(0.0) && call_delta < dec!(1.0));
///
/// let put_delta = black_scholes_delta(dec!(100.0), dec!(100.0), dec!(0.05), dec!(0.2), dec!(1.0), OptionStyle::Put).unwrap();
/// assert!(put_delta > dec!(-1.0) && put_delta < dec!(0.0));
/// ```
#[allow(clippy::cast_precision_loss)]
pub fn black_scholes_delta(
    spot: Decimal,
    strike: Decimal,
    risk_free_rate: Decimal,
    volatility: Decimal,
    time_to_maturity: Decimal,
    style: OptionStyle,
) -> Result<Decimal, CalculationError> {
    validate_black_scholes_inputs(spot, strike, risk_free_rate, volatility, time_to_maturity)?;
    let s = decimal_to_f64(spot, "black_scholes_delta")?;
    let k = decimal_to_f64(strike, "black_scholes_delta")?;
    let r = decimal_to_f64(risk_free_rate, "black_scholes_delta")?;
    let sigma = decimal_to_f64(volatility, "black_scholes_delta")?;
    let t = decimal_to_f64(time_to_maturity, "black_scholes_delta")?;
    let (d1, _) = black_scholes_d1_d2(s, k, r, sigma, t);

    let delta = match style {
        OptionStyle::Call => normal_cdf(d1),
        OptionStyle::Put => normal_cdf(d1) - 1.0,
    };
    return Decimal::try_from(delta).map_err(|_| {
        return CalculationError::Overflow {
            formula: "black_scholes_delta",
        };
    });
}

/// Computes the Black-Scholes gamma of an option.
///
/// Gamma is identical for calls and puts:
///
/// \\[ \\Gamma = \\frac{N'(d_1)}{S_0 \\sigma \\sqrt{T}} \\]
///
/// # Errors
///
/// Same validation as [`black_scholes_call`].
///
/// # Examples
///
/// ```
/// use casiros_core::options::black_scholes_gamma;
/// use rust_decimal_macros::dec;
///
/// let gamma = black_scholes_gamma(dec!(100.0), dec!(100.0), dec!(0.05), dec!(0.2), dec!(1.0)).unwrap();
/// assert!(gamma > dec!(0.0));
/// ```
#[allow(clippy::cast_precision_loss)]
pub fn black_scholes_gamma(
    spot: Decimal,
    strike: Decimal,
    risk_free_rate: Decimal,
    volatility: Decimal,
    time_to_maturity: Decimal,
) -> Result<Decimal, CalculationError> {
    validate_black_scholes_inputs(spot, strike, risk_free_rate, volatility, time_to_maturity)?;
    let s = decimal_to_f64(spot, "black_scholes_gamma")?;
    let k = decimal_to_f64(strike, "black_scholes_gamma")?;
    let r = decimal_to_f64(risk_free_rate, "black_scholes_gamma")?;
    let sigma = decimal_to_f64(volatility, "black_scholes_gamma")?;
    let t = decimal_to_f64(time_to_maturity, "black_scholes_gamma")?;
    let (d1, _) = black_scholes_d1_d2(s, k, r, sigma, t);

    let gamma = normal_pdf(d1) / (s * sigma * t.sqrt());
    return Decimal::try_from(gamma).map_err(|_| {
        return CalculationError::Overflow {
            formula: "black_scholes_gamma",
        };
    });
}

/// Computes the Black-Scholes vega of an option.
///
/// Vega is identical for calls and puts:
///
/// \\[ \\mathcal{V} = S_0 N'(d_1) \\sqrt{T} \\]
///
/// # Errors
///
/// Same validation as [`black_scholes_call`].
///
/// # Examples
///
/// ```
/// use casiros_core::options::black_scholes_vega;
/// use rust_decimal_macros::dec;
///
/// let vega = black_scholes_vega(dec!(100.0), dec!(100.0), dec!(0.05), dec!(0.2), dec!(1.0)).unwrap();
/// assert!(vega > dec!(0.0));
/// ```
#[allow(clippy::cast_precision_loss)]
pub fn black_scholes_vega(
    spot: Decimal,
    strike: Decimal,
    risk_free_rate: Decimal,
    volatility: Decimal,
    time_to_maturity: Decimal,
) -> Result<Decimal, CalculationError> {
    validate_black_scholes_inputs(spot, strike, risk_free_rate, volatility, time_to_maturity)?;
    let s = decimal_to_f64(spot, "black_scholes_vega")?;
    let k = decimal_to_f64(strike, "black_scholes_vega")?;
    let r = decimal_to_f64(risk_free_rate, "black_scholes_vega")?;
    let sigma = decimal_to_f64(volatility, "black_scholes_vega")?;
    let t = decimal_to_f64(time_to_maturity, "black_scholes_vega")?;
    let (d1, _) = black_scholes_d1_d2(s, k, r, sigma, t);

    let vega = s * normal_pdf(d1) * t.sqrt();
    return Decimal::try_from(vega).map_err(|_| {
        return CalculationError::Overflow {
            formula: "black_scholes_vega",
        };
    });
}

/// Computes the Black-Scholes theta of an option.
///
/// The result is expressed as the change in option value per one year of time
/// decay.
///
/// # Mathematical Definition
///
/// Call:
/// \\[ \\Theta_{call} = -\\frac{S_0 N'(d_1) \\sigma}{2\\sqrt{T}} - r K e^{-rT} N(d_2) \\]
///
/// Put:
/// \\[ \\Theta_{put} = -\\frac{S_0 N'(d_1) \\sigma}{2\\sqrt{T}} + r K e^{-rT} N(-d_2) \\]
///
/// # Errors
///
/// Same validation as [`black_scholes_call`].
///
/// # Examples
///
/// ```
/// use casiros_core::options::{black_scholes_theta, OptionStyle};
/// use rust_decimal_macros::dec;
///
/// let call_theta = black_scholes_theta(dec!(100.0), dec!(100.0), dec!(0.05), dec!(0.2), dec!(1.0), OptionStyle::Call).unwrap();
/// assert!(call_theta < dec!(0.0));
/// ```
#[allow(clippy::cast_precision_loss)]
pub fn black_scholes_theta(
    spot: Decimal,
    strike: Decimal,
    risk_free_rate: Decimal,
    volatility: Decimal,
    time_to_maturity: Decimal,
    style: OptionStyle,
) -> Result<Decimal, CalculationError> {
    validate_black_scholes_inputs(spot, strike, risk_free_rate, volatility, time_to_maturity)?;
    let s = decimal_to_f64(spot, "black_scholes_theta")?;
    let k = decimal_to_f64(strike, "black_scholes_theta")?;
    let r = decimal_to_f64(risk_free_rate, "black_scholes_theta")?;
    let sigma = decimal_to_f64(volatility, "black_scholes_theta")?;
    let t = decimal_to_f64(time_to_maturity, "black_scholes_theta")?;
    let (d1, d2) = black_scholes_d1_d2(s, k, r, sigma, t);

    let common = -(s * normal_pdf(d1) * sigma) / (2.0 * t.sqrt());
    let theta = match style {
        OptionStyle::Call => common - r * k * (-r * t).exp() * normal_cdf(d2),
        OptionStyle::Put => common + r * k * (-r * t).exp() * normal_cdf(-d2),
    };
    return Decimal::try_from(theta).map_err(|_| {
        return CalculationError::Overflow {
            formula: "black_scholes_theta",
        };
    });
}

/// Computes the Black-Scholes rho of an option.
///
/// # Mathematical Definition
///
/// Call:
/// \\[ \\rho_{call} = K T e^{-rT} N(d_2) \\]
///
/// Put:
/// \\[ \\rho_{put} = -K T e^{-rT} N(-d_2) \\]
///
/// # Errors
///
/// Same validation as [`black_scholes_call`].
///
/// # Examples
///
/// ```
/// use casiros_core::options::{black_scholes_rho, OptionStyle};
/// use rust_decimal_macros::dec;
///
/// let call_rho = black_scholes_rho(dec!(100.0), dec!(100.0), dec!(0.05), dec!(0.2), dec!(1.0), OptionStyle::Call).unwrap();
/// assert!(call_rho > dec!(0.0));
/// ```
#[allow(clippy::cast_precision_loss)]
pub fn black_scholes_rho(
    spot: Decimal,
    strike: Decimal,
    risk_free_rate: Decimal,
    volatility: Decimal,
    time_to_maturity: Decimal,
    style: OptionStyle,
) -> Result<Decimal, CalculationError> {
    validate_black_scholes_inputs(spot, strike, risk_free_rate, volatility, time_to_maturity)?;
    let s = decimal_to_f64(spot, "black_scholes_rho")?;
    let k = decimal_to_f64(strike, "black_scholes_rho")?;
    let r = decimal_to_f64(risk_free_rate, "black_scholes_rho")?;
    let sigma = decimal_to_f64(volatility, "black_scholes_rho")?;
    let t = decimal_to_f64(time_to_maturity, "black_scholes_rho")?;
    let (_, d2) = black_scholes_d1_d2(s, k, r, sigma, t);

    let rho = match style {
        OptionStyle::Call => k * t * (-r * t).exp() * normal_cdf(d2),
        OptionStyle::Put => -k * t * (-r * t).exp() * normal_cdf(-d2),
    };
    return Decimal::try_from(rho).map_err(|_| {
        return CalculationError::Overflow {
            formula: "black_scholes_rho",
        };
    });
}

#[cfg(test)]
mod tests {
    //! Unit tests for the option pricing module.

    use super::*;

    /// Verifies that a zero-volatility call has zero value when out of the money.
    #[test]
    fn black_scholes_call_zero_vol_out_of_the_money() {
        let call =
            black_scholes_call(dec!(100.0), dec!(110.0), dec!(0.05), dec!(0.0), dec!(1.0)).unwrap();
        return assert!(call < dec!(0.0001));
    }

    /// Verifies that put-call parity holds for the Black-Scholes prices.
    #[test]
    fn black_scholes_put_call_parity() {
        let spot = dec!(100.0);
        let strike = dec!(95.0);
        let r = dec!(0.05);
        let sigma = dec!(0.2);
        let t = dec!(1.0);

        let call = black_scholes_call(spot, strike, r, sigma, t).unwrap();
        let put = black_scholes_put(spot, strike, r, sigma, t).unwrap();
        let left = call - put;
        let right = spot - strike * dec!(0.951_229_424_500_714); // e^{-0.05}
        return assert!((left - right).abs() < dec!(0.01));
    }

    /// Verifies that a large number of binomial steps converges to Black-Scholes.
    #[test]
    fn binomial_converges_to_black_scholes() {
        let spot = dec!(100.0);
        let strike = dec!(100.0);
        let r = dec!(0.05);
        let sigma = dec!(0.2);
        let t = dec!(1.0);

        let bs_call = black_scholes_call(spot, strike, r, sigma, t).unwrap();
        let bin_call = binomial_option_call(spot, strike, r, sigma, t, 500).unwrap();
        return assert!((bs_call - bin_call).abs() < dec!(0.05));
    }

    /// Verifies that delta responds correctly to call/put style.
    #[test]
    fn black_scholes_delta_signs() {
        let spot = dec!(100.0);
        let strike = dec!(100.0);
        let r = dec!(0.05);
        let sigma = dec!(0.2);
        let t = dec!(1.0);

        let call_delta = black_scholes_delta(spot, strike, r, sigma, t, OptionStyle::Call).unwrap();
        let put_delta = black_scholes_delta(spot, strike, r, sigma, t, OptionStyle::Put).unwrap();
        return assert!(call_delta > dec!(0.0) && put_delta < dec!(0.0));
    }

    /// Verifies that gamma and vega are positive.
    #[test]
    fn black_scholes_greeks_positive() {
        let spot = dec!(100.0);
        let strike = dec!(100.0);
        let r = dec!(0.05);
        let sigma = dec!(0.2);
        let t = dec!(1.0);

        let gamma = black_scholes_gamma(spot, strike, r, sigma, t).unwrap();
        let vega = black_scholes_vega(spot, strike, r, sigma, t).unwrap();
        return assert!(gamma > dec!(0.0) && vega > dec!(0.0));
    }

    /// Verifies that theta is negative for a call in typical market conditions.
    #[test]
    fn black_scholes_call_theta_negative() {
        let theta = black_scholes_theta(
            dec!(100.0),
            dec!(100.0),
            dec!(0.05),
            dec!(0.2),
            dec!(1.0),
            OptionStyle::Call,
        )
        .unwrap();
        return assert!(theta < dec!(0.0));
    }

    /// Verifies that rho is positive for calls and negative for puts.
    #[test]
    fn black_scholes_rho_signs() {
        let spot = dec!(100.0);
        let strike = dec!(100.0);
        let r = dec!(0.05);
        let sigma = dec!(0.2);
        let t = dec!(1.0);

        let call_rho = black_scholes_rho(spot, strike, r, sigma, t, OptionStyle::Call).unwrap();
        let put_rho = black_scholes_rho(spot, strike, r, sigma, t, OptionStyle::Put).unwrap();
        return assert!(call_rho > dec!(0.0) && put_rho < dec!(0.0));
    }

    /// Verifies that zero steps is rejected by the binomial pricer.
    #[test]
    fn binomial_zero_steps_rejected() {
        let result = binomial_option_call(
            dec!(100.0),
            dec!(100.0),
            dec!(0.05),
            dec!(0.2),
            dec!(1.0),
            0,
        );
        return assert!(matches!(
            result,
            Err(CalculationError::NegativeValueInvalid { .. })
        ));
    }
}
