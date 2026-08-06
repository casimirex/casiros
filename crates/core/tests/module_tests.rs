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

#[test]
fn test_capital_adequacy_ratio() {
    let car = banking::capital_adequacy_ratio(dec!(100.0), dec!(1000.0)).unwrap();
    assert_eq!(car, dec!(0.1));
}

#[test]
fn test_provision_coverage_ratio() {
    let pcr = banking::provision_coverage_ratio(dec!(80.0), dec!(100.0)).unwrap();
    assert_eq!(pcr, dec!(0.8));
}

#[test]
fn test_treynor_ratio() {
    let treynor = markets::treynor_ratio(dec!(0.12), dec!(0.03), dec!(1.2)).unwrap();
    assert_eq!(treynor, dec!(0.075));
}

#[test]
fn test_value_at_risk() {
    let var = markets::value_at_risk(dec!(100000.0), dec!(0.10), dec!(0.15), dec!(1.645)).unwrap();
    assert!(var < dec!(0.0));
}

#[test]
fn test_expected_shortfall() {
    let cvar =
        markets::expected_shortfall(dec!(100000.0), dec!(0.10), dec!(0.15), dec!(1.645)).unwrap();
    assert!(cvar < dec!(0.0));
}

#[test]
fn test_discounted_cash_flow() {
    let cash_flows = vec![dec!(100.0), dec!(100.0), dec!(100.0)];
    let pv = stocks_bonds::discounted_cash_flow(&cash_flows, dec!(0.05)).unwrap();
    assert_eq!(pv.round_dp(2), dec!(272.32));
}

#[test]
fn test_macaulay_duration() {
    let cash_flows = vec![dec!(100.0), dec!(100.0), dec!(100.0)];
    let duration = stocks_bonds::macaulay_duration(&cash_flows, dec!(0.05)).unwrap();
    assert_eq!(duration.round_dp(3), dec!(1.967));
}

#[test]
fn test_modified_duration() {
    let md = stocks_bonds::modified_duration(dec!(1.967), dec!(0.05)).unwrap();
    assert_eq!(md.round_dp(3), dec!(1.873));
}

#[test]
fn test_convexity() {
    let cash_flows = vec![dec!(100.0), dec!(100.0), dec!(100.0)];
    let c = stocks_bonds::convexity(&cash_flows, dec!(0.05)).unwrap();
    assert_eq!(c.round_dp(3), dec!(5.900));
}
