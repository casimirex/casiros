//! General time-value-of-money formulas.
//!
//! This module contains the foundational formulas for computing future value,
//! present value, annuities, perpetuities, and rate conversions.

use super::prelude::*;
use rust_decimal::Decimal;
use rust_decimal::MathematicalOps;
use rust_decimal_macros::dec;

/// Maximum number of schedule periods that can be generated.
const MAX_AMORTIZATION_PERIODS: u32 = 1_000;

/// Calculates the future value of a present sum using compound interest.
///
/// # Mathematical Definition
///
/// \[ FV = PV \times (1 + r)^n \]
///
/// # Constraints
///
/// - `present_value` MUST be >= 0.
/// - `rate` MUST be > -1.0 (to prevent invalid negative compounding).
/// - `periods` MUST be > 0 for growth; zero returns the principal unchanged.
///
/// # Errors
///
/// Returns [`CalculationError::InvalidRate`] if `rate` <= -1.0.
/// Returns [`CalculationError::NegativeValueInvalid`] if `present_value` < 0.
///
/// # Examples
///
/// ```
/// use casiros_core::general::future_value;
/// use rust_decimal_macros::dec;
///
/// // $100 at 5% for 10 years = $162.89
/// let fv = future_value(dec!(100.0), dec!(0.05), 10).unwrap();
/// assert_eq!(fv.round_dp(4), dec!(162.8895));
/// assert!(fv > dec!(100.0)); // Assertion 2: positive growth
/// ```
pub fn future_value(
    present_value: Decimal,
    rate: Decimal,
    periods: Periods,
) -> Result<Decimal, CalculationError> {
    if rate <= dec!(-1.0) {
        return Err(CalculationError::InvalidRate { rate });
    }
    if present_value < Decimal::ZERO {
        return Err(CalculationError::NegativeValueInvalid {
            context: "future_value - present_value",
            value: present_value,
        });
    }
    if periods == 0 {
        return Ok(present_value);
    }

    let growth_factor = (dec!(1.0) + rate).powi(i64::from(periods));
    return Ok(present_value * growth_factor);
}

/// Calculates the present value of a future sum using discounting.
///
/// # Mathematical Definition
///
/// \[ PV = \frac{FV}{(1 + r)^n} \]
///
/// # Errors
///
/// Returns [`CalculationError::InvalidRate`] if `rate` <= -1.0.
/// Returns [`CalculationError::NegativeValueInvalid`] if `future_value` < 0.
///
/// # Examples
///
/// ```
/// use casiros_core::general::present_value;
/// use rust_decimal_macros::dec;
///
/// let pv = present_value(dec!(162.8895), dec!(0.05), 10).unwrap();
/// assert_eq!(pv.round_dp(2), dec!(100.00));
/// assert!(pv > dec!(0.0)); // Assertion 2
/// ```
pub fn present_value(
    future_value: Decimal,
    rate: Decimal,
    periods: Periods,
) -> Result<Decimal, CalculationError> {
    if rate <= dec!(-1.0) {
        return Err(CalculationError::InvalidRate { rate });
    }
    if future_value < Decimal::ZERO {
        return Err(CalculationError::NegativeValueInvalid {
            context: "present_value - future_value",
            value: future_value,
        });
    }
    if periods == 0 {
        return Ok(future_value);
    }

    let discount_factor = (dec!(1.0) + rate).powi(i64::from(periods));
    if discount_factor == Decimal::ZERO {
        return Err(CalculationError::DivisionByZero {
            formula: "Present Value",
        });
    }
    return Ok(future_value / discount_factor);
}

/// Calculates the future value of an annuity (series of equal payments).
///
/// # Mathematical Definition
///
/// \[ FV_{\text{annuity}} = PMT \times \frac{(1 + r)^n - 1}{r} \]
///
/// # Errors
///
/// Returns [`CalculationError::InvalidRate`] if `rate` <= -1.0.
/// Returns [`CalculationError::NegativeValueInvalid`] if `payment` < 0.
///
/// # Examples
///
/// ```
/// use casiros_core::general::annuity_future_value;
/// use rust_decimal_macros::dec;
///
/// // $1,000/year at 5% for 10 years
/// let fv = annuity_future_value(dec!(1000.0), dec!(0.05), 10).unwrap();
/// assert_eq!(fv.round_dp(2), dec!(12577.89));
/// assert!(fv > dec!(10000.0)); // Assertion 2
/// ```
pub fn annuity_future_value(
    payment: Decimal,
    rate: Decimal,
    periods: Periods,
) -> Result<Decimal, CalculationError> {
    if rate <= dec!(-1.0) {
        return Err(CalculationError::InvalidRate { rate });
    }
    if payment < Decimal::ZERO {
        return Err(CalculationError::NegativeValueInvalid {
            context: "annuity_future_value - payment",
            value: payment,
        });
    }
    if rate == Decimal::ZERO {
        return Ok(payment * Decimal::from(periods));
    }

    let growth = (dec!(1.0) + rate).powi(i64::from(periods));
    return Ok(payment * (growth - dec!(1.0)) / rate);
}

/// Calculates the present value of an annuity.
///
/// # Mathematical Definition
///
/// \[ PV_{\text{annuity}} = PMT \times \frac{1 - (1 + r)^{-n}}{r} \]
///
/// # Errors
///
/// Returns [`CalculationError::InvalidRate`] if `rate` <= -1.0.
/// Returns [`CalculationError::NegativeValueInvalid`] if `payment` < 0.
///
/// # Examples
///
/// ```
/// use casiros_core::general::annuity_present_value;
/// use rust_decimal_macros::dec;
///
/// let pv = annuity_present_value(dec!(1000.0), dec!(0.05), 10).unwrap();
/// assert_eq!(pv.round_dp(2), dec!(7721.73));
/// assert!(pv < dec!(10000.0)); // Assertion 2
/// ```
pub fn annuity_present_value(
    payment: Decimal,
    rate: Decimal,
    periods: Periods,
) -> Result<Decimal, CalculationError> {
    if rate <= dec!(-1.0) {
        return Err(CalculationError::InvalidRate { rate });
    }
    if payment < Decimal::ZERO {
        return Err(CalculationError::NegativeValueInvalid {
            context: "annuity_present_value - payment",
            value: payment,
        });
    }
    if rate == Decimal::ZERO {
        return Ok(payment * Decimal::from(periods));
    }

    let discount = (dec!(1.0) + rate).powi(i64::from(periods));
    if discount == Decimal::ZERO {
        return Err(CalculationError::DivisionByZero {
            formula: "Annuity Present Value",
        });
    }
    return Ok(payment * (dec!(1.0) - dec!(1.0) / discount) / rate);
}

/// Calculates the present value of a perpetuity.
///
/// # Mathematical Definition
///
/// \[ PV_{\text{perpetuity}} = \frac{PMT}{r} \]
///
/// # Errors
///
/// Returns [`CalculationError::InvalidRate`] if `rate` <= 0.
/// Returns [`CalculationError::NegativeValueInvalid`] if `payment` < 0.
///
/// # Examples
///
/// ```
/// use casiros_core::general::perpetuity_present_value;
/// use rust_decimal_macros::dec;
///
/// let pv = perpetuity_present_value(dec!(100.0), dec!(0.05)).unwrap();
/// assert_eq!(pv, dec!(2000.0));
/// assert!(pv > dec!(0.0)); // Assertion 2
/// ```
pub fn perpetuity_present_value(
    payment: Decimal,
    rate: Decimal,
) -> Result<Decimal, CalculationError> {
    if rate <= Decimal::ZERO {
        return Err(CalculationError::InvalidRate { rate });
    }
    if payment < Decimal::ZERO {
        return Err(CalculationError::NegativeValueInvalid {
            context: "perpetuity_present_value - payment",
            value: payment,
        });
    }
    return Ok(payment / rate);
}

/// Converts a nominal annual rate to an effective annual rate (EAR).
///
/// # Mathematical Definition
///
/// \[ EAR = \left(1 + \frac{r_{\text{nom}}}{m}\right)^m - 1 \]
///
/// where `m` is the number of compounding periods per year.
///
/// # Errors
///
/// Returns [`CalculationError::InvalidRate`] if `nominal_rate` <= -1.0.
/// Returns [`CalculationError::DivisionByZero`] if `compounding_periods` is 0.
///
/// # Examples
///
/// ```
/// use casiros_core::general::effective_annual_rate;
/// use rust_decimal_macros::dec;
///
/// // 5% nominal compounded monthly
/// let ear = effective_annual_rate(dec!(0.05), 12).unwrap();
/// assert_eq!(ear.round_dp(6), dec!(0.051162));
/// assert!(ear > dec!(0.05)); // Assertion 2
/// ```
pub fn effective_annual_rate(
    nominal_rate: Decimal,
    compounding_periods: u32,
) -> Result<Decimal, CalculationError> {
    if nominal_rate <= dec!(-1.0) {
        return Err(CalculationError::InvalidRate { rate: nominal_rate });
    }
    if compounding_periods == 0 {
        return Err(CalculationError::DivisionByZero {
            formula: "Effective Annual Rate",
        });
    }
    let periodic_rate = nominal_rate / Decimal::from(compounding_periods);
    let factor = (dec!(1.0) + periodic_rate).powi(i64::from(compounding_periods));
    return Ok(factor - dec!(1.0));
}

/// Calculates the periodic payment for a fully amortizing loan.
///
/// # Mathematical Definition
///
/// \\[ PMT = \frac{P \times r \times (1 + r)^n}{(1 + r)^n - 1} \\]
///
/// where `P` is the principal, `r` is the periodic rate, and `n` is the number
/// of periods.
///
/// # Errors
///
/// Returns [`CalculationError::InvalidRate`] if `rate` <= -1.0.
/// Returns [`CalculationError::DivisionByZero`] if `rate` is zero and periods is
/// zero (treated as a degenerate case).
///
/// # Examples
///
/// ```
/// use casiros_core::general::amortization_payment;
/// use rust_decimal_macros::dec;
///
/// // $100,000 loan at 5% annual for 30 years, monthly payments.
/// let pmt = amortization_payment(dec!(100000.0), dec!(0.05) / dec!(12.0), 360).unwrap();
/// assert!(pmt > dec!(0.0));
/// assert!(pmt < dec!(600.0));
/// ```
pub fn amortization_payment(
    principal: Decimal,
    rate: Decimal,
    periods: Periods,
) -> Result<Decimal, CalculationError> {
    if rate <= dec!(-1.0) {
        return Err(CalculationError::InvalidRate { rate });
    }
    if principal < Decimal::ZERO {
        return Err(CalculationError::NegativeValueInvalid {
            context: "amortization_payment - principal",
            value: principal,
        });
    }
    if periods == 0 {
        return Ok(Decimal::ZERO);
    }

    let n = Decimal::from(periods);
    if rate == Decimal::ZERO {
        return Ok(principal / n);
    }

    let growth = (dec!(1.0) + rate).powi(i64::from(periods));
    let numerator = principal * rate * growth;
    let denominator = growth - dec!(1.0);
    if denominator == Decimal::ZERO {
        return Err(CalculationError::DivisionByZero {
            formula: "amortization_payment",
        });
    }
    return Ok(numerator / denominator);
}

/// A single period of an amortization schedule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AmortizationPeriod {
    /// Period number (1-indexed).
    pub period: u32,

    /// Principal portion of the payment.
    pub principal_paid: Decimal,

    /// Interest portion of the payment.
    pub interest_paid: Decimal,

    /// Remaining balance after this payment.
    pub remaining_balance: Decimal,
}

/// Generates a full amortization schedule for a fixed-rate loan.
///
/// The schedule contains one entry per period, showing the principal and
/// interest components of each payment plus the remaining balance.
///
/// # Errors
///
/// Returns [`CalculationError::InvalidRate`] if `rate` <= -1.0.
/// Returns [`CalculationError::Overflow`] if `periods` exceeds
/// `MAX_AMORTIZATION_PERIODS`.
///
/// # Examples
///
/// ```
/// use casiros_core::general::amortization_schedule;
/// use rust_decimal_macros::dec;
///
/// let schedule = amortization_schedule(dec!(1000.0), dec!(0.12) / dec!(12.0), 12).unwrap();
/// assert_eq!(schedule.len(), 12);
/// assert!(schedule.last().unwrap().remaining_balance < dec!(0.01));
/// ```
pub fn amortization_schedule(
    principal: Decimal,
    rate: Decimal,
    periods: Periods,
) -> Result<Vec<AmortizationPeriod>, CalculationError> {
    if rate <= dec!(-1.0) {
        return Err(CalculationError::InvalidRate { rate });
    }
    if principal < Decimal::ZERO {
        return Err(CalculationError::NegativeValueInvalid {
            context: "amortization_schedule - principal",
            value: principal,
        });
    }
    if periods == 0 {
        return Ok(Vec::new());
    }
    if periods > MAX_AMORTIZATION_PERIODS {
        return Err(CalculationError::Overflow {
            formula: "amortization_schedule",
        });
    }

    let payment = amortization_payment(principal, rate, periods)?;
    let mut schedule = Vec::with_capacity(periods as usize);
    let mut balance = principal;

    for period in 1..=periods {
        let interest = balance * rate;
        let principal_paid = payment - interest;
        balance -= principal_paid;
        schedule.push(AmortizationPeriod {
            period,
            principal_paid,
            interest_paid: interest,
            remaining_balance: balance,
        });
    }

    return Ok(schedule);
}
