//! Tests for the `#[derive(Narrative)]` procedural macro.

use casiros_core::narrative::Narrative;
use casiros_macros::Narrative;
use rust_decimal_macros::dec;

#[derive(Narrative)]
#[allow(dead_code)]
struct SimpleMetric {
    revenue: rust_decimal::Decimal,
    cost: rust_decimal::Decimal,
}

#[derive(Narrative)]
#[narrative(prefix = "Capital Structure")]
#[allow(dead_code)]
struct CapitalStructure {
    equity: rust_decimal::Decimal,
    #[narrative(name = "total debt")]
    debt: rust_decimal::Decimal,
    #[narrative(skip)]
    secret: rust_decimal::Decimal,
}

#[derive(Narrative)]
#[allow(dead_code)]
struct AllSkipped {
    #[narrative(skip)]
    a: i32,
    #[narrative(skip)]
    b: i32,
}

#[test]
fn default_prefix_is_struct_name() {
    let metric = SimpleMetric {
        revenue: dec!(1000.0),
        cost: dec!(400.0),
    };
    let narrative = metric.narrative();
    assert!(narrative.starts_with("SimpleMetric:"));
    assert!(narrative.contains("revenue = 1000"));
    assert!(narrative.contains("cost = 400"));
}

#[test]
fn custom_prefix_and_field_name() {
    let cs = CapitalStructure {
        equity: dec!(600.0),
        debt: dec!(400.0),
        secret: dec!(999.0),
    };
    let narrative = cs.narrative();
    assert!(narrative.starts_with("Capital Structure:"));
    assert!(narrative.contains("equity = 600"));
    assert!(narrative.contains("total debt = 400"));
    assert!(!narrative.contains("secret"));
    assert!(!narrative.contains("999"));
}

#[test]
fn all_skipped_fields_produces_empty_narrative() {
    let all = AllSkipped { a: 1, b: 2 };
    assert_eq!(all.narrative(), "AllSkipped: (empty)");
}
