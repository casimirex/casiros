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

/// Approximates the yield-to-maturity (YTM) of a fixed-coupon bond.
///
/// Uses the common linear approximation:
///
/// \\[ YTM \\approx \\frac{C + \\frac{F - P}{n}}{\\frac{F + P}{2}} \\]
///
/// where `C` is the periodic coupon payment, `F` is the face value, `P` is the
/// current price, and `n` is the number of periods to maturity. The result is a
/// periodic yield consistent with [`bond_price`].
///
/// # Errors
///
/// Returns [`CalculationError::DivisionByZero`] if `price + face_value` is zero.
/// Returns [`CalculationError::NegativeValueInvalid`] if `periods` is zero or if
/// `price` or `face_value` is negative.
///
/// # Examples
///
/// ```
/// use casiros_core::stocks_bonds::yield_to_maturity_approximation;
/// use rust_decimal_macros::dec;
///
/// let ytm = yield_to_maturity_approximation(dec!(1000.0), dec!(50.0), dec!(950.0), 10).unwrap();
/// assert!(ytm > dec!(0.05));
/// assert!(ytm < dec!(0.06));
/// ```
pub fn yield_to_maturity_approximation(
    face_value: Decimal,
    coupon_payment: Decimal,
    price: Decimal,
    periods: Periods,
) -> Result<Decimal, CalculationError> {
    if periods == 0 {
        return Err(CalculationError::NegativeValueInvalid {
            context: "yield_to_maturity_approximation - periods",
            value: Decimal::ZERO,
        });
    }
    if face_value < Decimal::ZERO {
        return Err(CalculationError::NegativeValueInvalid {
            context: "yield_to_maturity_approximation - face_value",
            value: face_value,
        });
    }
    if price < Decimal::ZERO {
        return Err(CalculationError::NegativeValueInvalid {
            context: "yield_to_maturity_approximation - price",
            value: price,
        });
    }

    let average_price = (face_value + price) / dec!(2.0);
    if average_price == Decimal::ZERO {
        return Err(CalculationError::DivisionByZero {
            formula: "Yield to Maturity Approximation",
        });
    }

    let capital_gain = (face_value - price) / Decimal::from(periods);
    return Ok((coupon_payment + capital_gain) / average_price);
}

/// Computes the present value of a series of future cash flows.
///
/// # Mathematical Definition
///
/// \[ PV = \sum_{t=1}^{n} \frac{CF_t}{(1 + r)^t} \]
///
/// where \( CF_t \) is the cash flow at period \( t \) and \( r \) is the
/// discount rate per period.
///
/// # Errors
///
/// Returns [`CalculationError::InvalidRate`] if `discount_rate` <= -1.0.
/// Returns [`CalculationError::DivisionByZero`] if a discount factor is zero.
///
/// # Examples
///
/// ```
/// use casiros_core::stocks_bonds::discounted_cash_flow;
/// use rust_decimal_macros::dec;
///
/// let cash_flows = vec![dec!(100.0), dec!(100.0), dec!(100.0)];
/// let pv = discounted_cash_flow(&cash_flows, dec!(0.05)).unwrap();
/// assert_eq!(pv.round_dp(2), dec!(272.32));
/// assert!(pv > dec!(0.0)); // Assertion 2
/// ```
pub fn discounted_cash_flow(
    cash_flows: &[Decimal],
    discount_rate: Decimal,
) -> Result<Decimal, CalculationError> {
    if discount_rate <= dec!(-1.0) {
        return Err(CalculationError::InvalidRate {
            rate: discount_rate,
        });
    }
    let mut present_value = Decimal::ZERO;
    for (index, cash_flow) in cash_flows.iter().enumerate() {
        let period = match i64::try_from(index) {
            Ok(value) => value + 1,
            Err(_) => {
                return Err(CalculationError::Overflow {
                    formula: "Discounted Cash Flow",
                });
            }
        };
        let discount_factor = (dec!(1.0) + discount_rate).powi(period);
        if discount_factor == Decimal::ZERO {
            return Err(CalculationError::DivisionByZero {
                formula: "Discounted Cash Flow",
            });
        }
        present_value += cash_flow / discount_factor;
    }
    return Ok(present_value);
}

/// Computes the Macaulay duration of a series of cash flows.
///
/// # Mathematical Definition
///
/// \[ D = \frac{\sum_{t=1}^{n} t \times PV(CF_t)}{P} \]
///
/// where \( PV(CF_t) = CF_t / (1 + y)^t \) and \( P \) is the total present
/// value of all cash flows.
///
/// # Errors
///
/// Returns [`CalculationError::InvalidRate`] if `yield_per_period` <= -1.0.
/// Returns [`CalculationError::DivisionByZero`] if the total present value is
/// zero or a discount factor is zero.
///
/// # Examples
///
/// ```
/// use casiros_core::stocks_bonds::macaulay_duration;
/// use rust_decimal_macros::dec;
///
/// let cash_flows = vec![dec!(100.0), dec!(100.0), dec!(100.0)];
/// let duration = macaulay_duration(&cash_flows, dec!(0.05)).unwrap();
/// assert_eq!(duration.round_dp(3), dec!(1.967));
/// assert!(duration > dec!(0.0)); // Assertion 2
/// ```
pub fn macaulay_duration(
    cash_flows: &[Decimal],
    yield_per_period: Decimal,
) -> Result<Decimal, CalculationError> {
    if yield_per_period <= dec!(-1.0) {
        return Err(CalculationError::InvalidRate {
            rate: yield_per_period,
        });
    }
    let mut price = Decimal::ZERO;
    let mut weighted_sum = Decimal::ZERO;
    for (index, cash_flow) in cash_flows.iter().enumerate() {
        let period = match i64::try_from(index) {
            Ok(value) => value + 1,
            Err(_) => {
                return Err(CalculationError::Overflow {
                    formula: "Macaulay Duration",
                });
            }
        };
        let discount_factor = (dec!(1.0) + yield_per_period).powi(period);
        if discount_factor == Decimal::ZERO {
            return Err(CalculationError::DivisionByZero {
                formula: "Macaulay Duration",
            });
        }
        let pv = cash_flow / discount_factor;
        price += pv;
        weighted_sum += Decimal::from(period) * pv;
    }
    if price == Decimal::ZERO {
        return Err(CalculationError::DivisionByZero {
            formula: "Macaulay Duration",
        });
    }
    return Ok(weighted_sum / price);
}

/// Computes Modified Duration from Macaulay duration.
///
/// # Mathematical Definition
///
/// \[ MD = \frac{D}{1 + y} \]
///
/// where \( D \) is the Macaulay duration and \( y \) is the yield per period.
///
/// # Errors
///
/// Returns [`CalculationError::InvalidRate`] if `yield_per_period` <= -1.0.
/// Returns [`CalculationError::DivisionByZero`] if `1 + yield_per_period` is zero.
///
/// # Examples
///
/// ```
/// use casiros_core::stocks_bonds::modified_duration;
/// use rust_decimal_macros::dec;
///
/// let md = modified_duration(dec!(1.967), dec!(0.05)).unwrap();
/// assert_eq!(md.round_dp(3), dec!(1.873));
/// assert!(md > dec!(0.0)); // Assertion 2
/// ```
pub fn modified_duration(
    macaulay_duration: Decimal,
    yield_per_period: Decimal,
) -> Result<Decimal, CalculationError> {
    if yield_per_period <= dec!(-1.0) {
        return Err(CalculationError::InvalidRate {
            rate: yield_per_period,
        });
    }
    let denominator = dec!(1.0) + yield_per_period;
    if denominator == Decimal::ZERO {
        return Err(CalculationError::DivisionByZero {
            formula: "Modified Duration",
        });
    }
    return Ok(macaulay_duration / denominator);
}

/// Computes the convexity of a series of cash flows.
///
/// # Mathematical Definition
///
/// \[ C = \frac{\sum_{t=1}^{n} t(t+1) \times PV(CF_t)}{P \times (1 + y)^2} \]
///
/// where \( PV(CF_t) = CF_t / (1 + y)^t \), \( P \) is the total present
/// value, and \( y \) is the yield per period.
///
/// # Errors
///
/// Returns [`CalculationError::InvalidRate`] if `yield_per_period` <= -1.0.
/// Returns [`CalculationError::DivisionByZero`] if the total present value is
/// zero or a discount factor is zero.
///
/// # Examples
///
/// ```
/// use casiros_core::stocks_bonds::convexity;
/// use rust_decimal_macros::dec;
///
/// let cash_flows = vec![dec!(100.0), dec!(100.0), dec!(100.0)];
/// let c = convexity(&cash_flows, dec!(0.05)).unwrap();
/// assert_eq!(c.round_dp(3), dec!(5.900));
/// assert!(c > dec!(0.0)); // Assertion 2
/// ```
pub fn convexity(
    cash_flows: &[Decimal],
    yield_per_period: Decimal,
) -> Result<Decimal, CalculationError> {
    if yield_per_period <= dec!(-1.0) {
        return Err(CalculationError::InvalidRate {
            rate: yield_per_period,
        });
    }
    let mut price = Decimal::ZERO;
    let mut weighted_sum = Decimal::ZERO;
    for (index, cash_flow) in cash_flows.iter().enumerate() {
        let period_i64 = match i64::try_from(index) {
            Ok(value) => value + 1,
            Err(_) => {
                return Err(CalculationError::Overflow {
                    formula: "Convexity",
                });
            }
        };
        let period = Decimal::from(period_i64);
        let discount_factor = (dec!(1.0) + yield_per_period).powi(period_i64);
        if discount_factor == Decimal::ZERO {
            return Err(CalculationError::DivisionByZero {
                formula: "Convexity",
            });
        }
        let pv = cash_flow / discount_factor;
        price += pv;
        weighted_sum += period * (period + dec!(1.0)) * pv;
    }
    if price == Decimal::ZERO {
        return Err(CalculationError::DivisionByZero {
            formula: "Convexity",
        });
    }
    let denominator = price * (dec!(1.0) + yield_per_period).powi(2);
    if denominator == Decimal::ZERO {
        return Err(CalculationError::DivisionByZero {
            formula: "Convexity",
        });
    }
    return Ok(weighted_sum / denominator);
}
