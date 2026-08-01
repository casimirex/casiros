//! Integration tests for the financial ratio formulas.

use casiros_core::financial;
use rust_decimal_macros::dec;

#[test]
fn test_return_on_equity() {
    let roe = financial::return_on_equity(dec!(150.0), dec!(1000.0)).unwrap();
    assert_eq!(roe, dec!(0.15));
}

#[test]
fn test_return_on_equity_rejects_zero_equity() {
    let result = financial::return_on_equity(dec!(150.0), dec!(0.0));
    assert!(matches!(
        result,
        Err(casiros_core::error::CalculationError::DivisionByZero { .. })
    ));
}

#[test]
fn test_dupont_roe() {
    let roe = financial::dupont_roe(dec!(0.10), dec!(0.80), dec!(2.0)).unwrap();
    assert_eq!(roe, dec!(0.16));
}

#[test]
fn test_current_ratio() {
    let cr = financial::current_ratio(dec!(1000.0), dec!(500.0)).unwrap();
    assert_eq!(cr, dec!(2.0));
}

#[test]
fn test_debt_to_equity() {
    let de = financial::debt_to_equity(dec!(500.0), dec!(1000.0)).unwrap();
    assert_eq!(de, dec!(0.5));
}
