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

#[test]
fn test_return_on_investment() {
    let roi = financial::return_on_investment(dec!(150.0), dec!(100.0)).unwrap();
    assert_eq!(roi, dec!(0.5));
}

#[test]
fn test_return_on_investment_rejects_zero_cost() {
    let result = financial::return_on_investment(dec!(150.0), dec!(0.0));
    assert!(matches!(
        result,
        Err(casiros_core::error::CalculationError::DivisionByZero { .. })
    ));
}

#[test]
fn test_return_on_investment_allows_negative_gain() {
    let roi = financial::return_on_investment(dec!(50.0), dec!(100.0)).unwrap();
    assert_eq!(roi, dec!(-0.5));
}

#[test]
fn test_profit_margin() {
    let pm = financial::profit_margin(dec!(150.0), dec!(1000.0)).unwrap();
    assert_eq!(pm, dec!(0.15));
}

#[test]
fn test_profit_margin_rejects_zero_revenue() {
    let result = financial::profit_margin(dec!(150.0), dec!(0.0));
    assert!(matches!(
        result,
        Err(casiros_core::error::CalculationError::NegativeValueInvalid { .. })
    ));
}

#[test]
fn test_asset_turnover() {
    let at = financial::asset_turnover(dec!(1000.0), dec!(500.0)).unwrap();
    assert_eq!(at, dec!(2.0));
}

#[test]
fn test_asset_turnover_rejects_zero_assets() {
    let result = financial::asset_turnover(dec!(1000.0), dec!(0.0));
    assert!(matches!(
        result,
        Err(casiros_core::error::CalculationError::DivisionByZero { .. })
    ));
}

#[test]
fn test_equity_multiplier() {
    let em = financial::equity_multiplier(dec!(2000.0), dec!(1000.0)).unwrap();
    assert_eq!(em, dec!(2.0));
}

#[test]
fn test_equity_multiplier_rejects_zero_equity() {
    let result = financial::equity_multiplier(dec!(2000.0), dec!(0.0));
    assert!(matches!(
        result,
        Err(casiros_core::error::CalculationError::DivisionByZero { .. })
    ));
}

#[test]
fn test_quick_ratio() {
    let qr = financial::quick_ratio(dec!(1000.0), dec!(300.0), dec!(500.0)).unwrap();
    assert_eq!(qr, dec!(1.4));
}

#[test]
fn test_quick_ratio_rejects_zero_liabilities() {
    let result = financial::quick_ratio(dec!(1000.0), dec!(300.0), dec!(0.0));
    assert!(matches!(
        result,
        Err(casiros_core::error::CalculationError::DivisionByZero { .. })
    ));
}

#[test]
fn test_interest_coverage() {
    let icr = financial::interest_coverage(dec!(500.0), dec!(100.0)).unwrap();
    assert_eq!(icr, dec!(5.0));
}

#[test]
fn test_interest_coverage_rejects_zero_interest() {
    let result = financial::interest_coverage(dec!(500.0), dec!(0.0));
    assert!(matches!(
        result,
        Err(casiros_core::error::CalculationError::DivisionByZero { .. })
    ));
}

#[test]
fn test_interest_coverage_allows_negative_ebit() {
    let icr = financial::interest_coverage(dec!(-100.0), dec!(50.0)).unwrap();
    assert_eq!(icr, dec!(-2.0));
}

#[test]
fn test_inventory_turnover() {
    let it = financial::inventory_turnover(dec!(600.0), dec!(100.0)).unwrap();
    assert_eq!(it, dec!(6.0));
}

#[test]
fn test_inventory_turnover_rejects_zero_inventory() {
    let result = financial::inventory_turnover(dec!(600.0), dec!(0.0));
    assert!(matches!(
        result,
        Err(casiros_core::error::CalculationError::DivisionByZero { .. })
    ));
}

#[test]
fn test_cash_conversion_cycle() {
    let ccc = financial::cash_conversion_cycle(dec!(30.0), dec!(45.0), dec!(25.0)).unwrap();
    assert_eq!(ccc, dec!(50.0));
}
