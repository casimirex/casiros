//! Integration tests for the corporate finance formulas.

use casiros_core::corporate;
use rust_decimal_macros::dec;

#[test]
fn test_wacc() {
    let w = corporate::wacc(dec!(600.0), dec!(400.0), dec!(0.12), dec!(0.06), dec!(0.30)).unwrap();
    assert_eq!(w.round_dp(4), dec!(0.0888));
}

#[test]
fn test_wacc_rejects_invalid_tax_rate() {
    let result = corporate::wacc(dec!(600.0), dec!(400.0), dec!(0.12), dec!(0.06), dec!(1.5));
    assert!(matches!(
        result,
        Err(casiros_core::error::CalculationError::RangeViolation { .. })
    ));
}

#[test]
fn test_free_cash_flow_to_firm() {
    let fcff = corporate::free_cash_flow_to_firm(
        dec!(1000.0),
        dec!(0.30),
        dec!(100.0),
        dec!(50.0),
        dec!(200.0),
    )
    .unwrap();
    assert_eq!(fcff, dec!(550.0));
}

#[test]
fn test_sustainable_growth_rate() {
    let sgr = corporate::sustainable_growth_rate(dec!(0.15), dec!(0.40)).unwrap();
    assert_eq!(sgr, dec!(0.09));
}

#[test]
fn test_free_cash_flow_to_equity() {
    let fcfe = corporate::free_cash_flow_to_equity(dec!(550.0), dec!(35.0), dec!(50.0)).unwrap();
    assert_eq!(fcfe, dec!(565.0));
}

#[test]
fn test_economic_value_added() {
    let eva = corporate::economic_value_added(dec!(200.0), dec!(1000.0), dec!(0.10)).unwrap();
    assert_eq!(eva, dec!(100.0));
}

#[test]
fn test_internal_growth_rate() {
    let igr = corporate::internal_growth_rate(dec!(0.15), dec!(0.40)).unwrap();
    assert_eq!(igr.round_dp(4), dec!(0.0989));
}

#[test]
fn test_internal_growth_rate_rejects_invalid_payout_ratio() {
    let result = corporate::internal_growth_rate(dec!(0.15), dec!(1.5));
    assert!(matches!(
        result,
        Err(casiros_core::error::CalculationError::RangeViolation { .. })
    ));
}

#[test]
fn test_internal_growth_rate_rejects_non_positive_denominator() {
    let result = corporate::internal_growth_rate(dec!(3.0), dec!(0.60));
    assert!(matches!(
        result,
        Err(casiros_core::error::CalculationError::DivisionByZero { .. })
    ));
}
