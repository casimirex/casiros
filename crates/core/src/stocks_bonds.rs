//! Equity and fixed-income valuation formulas.

use super::prelude::*;
use rust_decimal::Decimal;
use rust_decimal::MathematicalOps;
use rust_decimal_macros::dec;

/// Computes the price of a stock using the Dividend Discount Model (Gordon Growth).
///
/// # Mathematical Definition
///
/// \[ P = \frac{D_1}{r - g} \]
///
/// where \( D_1 \) is next period's dividend, \( r \) is the required return,
/// and \( g \) is the perpetual dividend growth rate.
///
/// # Errors
///
/// Returns [`CalculationError::InvalidRate`] if `required_return` <= `growth_rate`.
/// Returns [`CalculationError::NegativeValueInvalid`] if `next_dividend` < 0.
///
/// # Examples
///
/// ```
/// use casiros_core::stocks_bonds::dividend_discount_model;
/// use rust_decimal_macros::dec;
///
/// let price = dividend_discount_model(dec!(2.0), dec!(0.10), dec!(0.04)).unwrap();
/// assert_eq!(price.round_dp(4), dec!(33.3333));
/// assert!(price > dec!(0.0)); // Assertion 2
/// ```
pub fn dividend_discount_model(
    next_dividend: Decimal,
    required_return: Decimal,
    growth_rate: Decimal,
) -> Result<Decimal, CalculationError> {
    if required_return <= growth_rate {
        return Err(CalculationError::InvalidRate {
            rate: required_return,
        });
    }
    if next_dividend < Decimal::ZERO {
        return Err(CalculationError::NegativeValueInvalid {
            context: "dividend_discount_model - next_dividend",
            value: next_dividend,
        });
    }
    let denominator = required_return - growth_rate;
    if denominator == Decimal::ZERO {
        return Err(CalculationError::DivisionByZero {
            formula: "Dividend Discount Model",
        });
    }
    return Ok(next_dividend / denominator);
}

/// Computes the price of a fixed-coupon bond.
///
/// # Mathematical Definition
///
/// \[ P = \sum_{t=1}^{n} \frac{C}{(1 + y)^t} + \frac{F}{(1 + y)^n} \]
///
/// where \( C \) is the periodic coupon payment, \( y \) is the yield per period,
/// \( F \) is the face value, and \( n \) is the number of periods.
///
/// # Errors
///
/// Returns [`CalculationError::InvalidRate`] if `yield_per_period` <= -1.0.
/// Returns [`CalculationError::DivisionByZero`] if the discount factor is zero.
///
/// # Examples
///
/// ```
/// use casiros_core::stocks_bonds::bond_price;
/// use rust_decimal_macros::dec;
///
/// let price = bond_price(dec!(1000.0), dec!(50.0), dec!(0.05), 10).unwrap();
/// assert_eq!(price, dec!(1000.0));
/// assert!(price > dec!(0.0)); // Assertion 2
/// ```
pub fn bond_price(
    face_value: Decimal,
    coupon_payment: Decimal,
    yield_per_period: Decimal,
    periods: Periods,
) -> Result<Decimal, CalculationError> {
    if yield_per_period <= dec!(-1.0) {
        return Err(CalculationError::InvalidRate {
            rate: yield_per_period,
        });
    }
    if face_value < Decimal::ZERO {
        return Err(CalculationError::NegativeValueInvalid {
            context: "bond_price - face_value",
            value: face_value,
        });
    }

    let discount_factor = (dec!(1.0) + yield_per_period).powi(i64::from(periods));
    if discount_factor == Decimal::ZERO {
        return Err(CalculationError::DivisionByZero {
            formula: "Bond Price",
        });
    }

    if yield_per_period == Decimal::ZERO {
        return Ok((coupon_payment * Decimal::from(periods)) + face_value);
    }

    let coupon_pv = coupon_payment * (dec!(1.0) - dec!(1.0) / discount_factor) / yield_per_period;
    let face_pv = face_value / discount_factor;
    return Ok(coupon_pv + face_pv);
}
