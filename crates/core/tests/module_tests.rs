//! Integration tests for banking, market, and stocks/bonds formulas.

use casiros_core::{banking, markets, stocks_bonds};
use rust_decimal_macros::dec;

#[test]
fn test_net_interest_margin() {
    let nim = banking::net_interest_margin(dec!(1200.0), dec!(400.0), dec!(10000.0)).unwrap();
    assert_eq!(nim, dec!(0.08));
}

#[test]
fn test_loan_to_deposit_ratio() {
    let ldr = banking::loan_to_deposit_ratio(dec!(800.0), dec!(1000.0)).unwrap();
    assert_eq!(ldr, dec!(0.8));
}

#[test]
fn test_sharpe_ratio() {
    let sharpe = markets::sharpe_ratio(dec!(0.12), dec!(0.03), dec!(0.15)).unwrap();
    assert_eq!(sharpe, dec!(0.6));
}

#[test]
fn test_jensens_alpha() {
    let alpha = markets::jensens_alpha(dec!(0.15), dec!(0.03), dec!(0.10), dec!(1.2)).unwrap();
    assert_eq!(alpha, dec!(0.036));
}

#[test]
fn test_dividend_discount_model() {
    let price = stocks_bonds::dividend_discount_model(dec!(2.0), dec!(0.10), dec!(0.04)).unwrap();
    assert_eq!(price.round_dp(4), dec!(33.3333));
}

#[test]
fn test_bond_price() {
    let price = stocks_bonds::bond_price(dec!(1000.0), dec!(50.0), dec!(0.05), 10).unwrap();
    assert_eq!(price, dec!(1000.0));
}
