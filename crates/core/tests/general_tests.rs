//! Integration tests for the general time-value-of-money formulas.

use casiros_core::general;
use rust_decimal_macros::dec;

#[test]
fn test_future_value_normal_case() {
    let fv = general::future_value(dec!(100.0), dec!(0.05), 10).unwrap();
    assert_eq!(fv.round_dp(4), dec!(162.8895));
}

#[test]
fn test_future_value_zero_periods() {
    let fv = general::future_value(dec!(100.0), dec!(0.05), 0).unwrap();
    assert_eq!(fv, dec!(100.0));
}

#[test]
fn test_future_value_rejects_invalid_rate() {
    let result = general::future_value(dec!(100.0), dec!(-2.0), 10);
    assert!(matches!(
        result,
        Err(casiros_core::error::CalculationError::InvalidRate { .. })
    ));
}

#[test]
fn test_present_value_is_inverse_of_future_value() {
    let pv = dec!(100.0);
    let rate = dec!(0.05);
    let periods = 10;
    let fv = general::future_value(pv, rate, periods).unwrap();
    let recovered_pv = general::present_value(fv, rate, periods).unwrap();
    let diff = (recovered_pv - pv).abs();
    assert!(diff < dec!(0.01));
}

#[test]
fn test_perpetuity_present_value() {
    let pv = general::perpetuity_present_value(dec!(100.0), dec!(0.05)).unwrap();
    assert_eq!(pv, dec!(2000.0));
}

#[test]
fn test_effective_annual_rate() {
    let ear = general::effective_annual_rate(dec!(0.05), 12).unwrap();
    assert_eq!(ear.round_dp(6), dec!(0.051162));
}

#[test]
fn test_growing_perpetuity_present_value() {
    let pv =
        general::growing_perpetuity_present_value(dec!(100.0), dec!(0.08), dec!(0.03)).unwrap();
    assert_eq!(pv, dec!(2000.0));
}

#[test]
fn test_growing_perpetuity_rejects_rate_less_than_or_equal_to_growth() {
    let result = general::growing_perpetuity_present_value(dec!(100.0), dec!(0.03), dec!(0.03));
    assert!(matches!(
        result,
        Err(casiros_core::error::CalculationError::InvalidRate { .. })
    ));
}

#[test]
fn test_growing_perpetuity_rejects_negative_payment() {
    let result = general::growing_perpetuity_present_value(dec!(-100.0), dec!(0.08), dec!(0.03));
    assert!(matches!(
        result,
        Err(casiros_core::error::CalculationError::NegativeValueInvalid { .. })
    ));
}

#[test]
fn test_continuous_compounding_future_value() {
    let fv =
        general::continuous_compounding_future_value(dec!(100.0), dec!(0.05), dec!(10.0)).unwrap();
    assert_eq!(fv.round_dp(4), dec!(164.8721));
}

#[test]
fn test_continuous_compounding_rejects_negative_present_value() {
    let result = general::continuous_compounding_future_value(dec!(-100.0), dec!(0.05), dec!(10.0));
    assert!(matches!(
        result,
        Err(casiros_core::error::CalculationError::NegativeValueInvalid { .. })
    ));
}

#[test]
fn test_continuous_compounding_rejects_negative_time() {
    let result = general::continuous_compounding_future_value(dec!(100.0), dec!(0.05), dec!(-1.0));
    assert!(matches!(
        result,
        Err(casiros_core::error::CalculationError::NegativeValueInvalid { .. })
    ));
}

#[test]
fn test_continuous_compounding_rejects_invalid_rate() {
    let result = general::continuous_compounding_future_value(dec!(100.0), dec!(-1.5), dec!(10.0));
    assert!(matches!(
        result,
        Err(casiros_core::error::CalculationError::InvalidRate { .. })
    ));
}
