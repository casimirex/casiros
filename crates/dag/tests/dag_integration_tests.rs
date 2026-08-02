//! Integration tests for the CASIROS causality graph engine.

use std::collections::HashMap;

use casiros_dag::DagError;
use casiros_dag::graph::{CausalityEngine, FormulaKind, Port};
use rust_decimal_macros::dec;

#[test]
fn wacc_pipeline_computes_from_inputs() {
    let mut engine = CausalityEngine::new();

    let equity_value = engine.add_input("equity_value");
    let debt_value = engine.add_input("debt_value");
    let cost_of_equity = engine.add_input("cost_of_equity");
    let cost_of_debt = engine.add_input("cost_of_debt");
    let tax_rate = engine.add_input("tax_rate");

    let wacc = engine.add_formula(
        "wacc",
        FormulaKind::Wacc {
            equity_value: Port::Output(equity_value),
            debt_value: Port::Output(debt_value),
            cost_of_equity: Port::Output(cost_of_equity),
            cost_of_debt: Port::Output(cost_of_debt),
            tax_rate: Port::Output(tax_rate),
        },
    );

    engine.add_edge(equity_value, wacc).unwrap();
    engine.add_edge(debt_value, wacc).unwrap();
    engine.add_edge(cost_of_equity, wacc).unwrap();
    engine.add_edge(cost_of_debt, wacc).unwrap();
    engine.add_edge(tax_rate, wacc).unwrap();

    let mut inputs = HashMap::new();
    inputs.insert(equity_value, dec!(600.0));
    inputs.insert(debt_value, dec!(400.0));
    inputs.insert(cost_of_equity, dec!(0.12));
    inputs.insert(cost_of_debt, dec!(0.06));
    inputs.insert(tax_rate, dec!(0.30));

    let outputs = engine.evaluate(&inputs).unwrap();
    assert_eq!(outputs[&wacc].round_dp(4), dec!(0.0888));
}

#[test]
fn present_value_is_inverse_of_future_value() {
    let mut engine = CausalityEngine::new();

    let principal = engine.add_input("principal");
    let rate = engine.add_input("rate");
    let periods = engine.add_input("periods");

    let fv = engine.add_formula(
        "future_value",
        FormulaKind::FutureValue {
            present_value: Port::Output(principal),
            rate: Port::Output(rate),
            periods: Port::Output(periods),
        },
    );
    let pv = engine.add_formula(
        "present_value",
        FormulaKind::PresentValue {
            future_value: Port::Output(fv),
            rate: Port::Output(rate),
            periods: Port::Output(periods),
        },
    );

    engine.add_edge(principal, fv).unwrap();
    engine.add_edge(rate, fv).unwrap();
    engine.add_edge(periods, fv).unwrap();
    engine.add_edge(fv, pv).unwrap();
    engine.add_edge(rate, pv).unwrap();
    engine.add_edge(periods, pv).unwrap();

    let mut inputs = HashMap::new();
    inputs.insert(principal, dec!(100.0));
    inputs.insert(rate, dec!(0.05));
    inputs.insert(periods, dec!(10));

    let outputs = engine.evaluate(&inputs).unwrap();
    assert_eq!(outputs[&pv].round_dp(2), dec!(100.0));
}

#[test]
fn missing_dependency_without_edge_returns_error() {
    let mut engine = CausalityEngine::new();

    let principal = engine.add_input("principal");
    let _fv = engine.add_formula(
        "future_value",
        FormulaKind::FutureValue {
            present_value: Port::Output(principal),
            rate: Port::Constant(dec!(0.05)),
            periods: Port::Constant(dec!(10)),
        },
    );
    // Deliberately omit the edge from principal to the formula.

    let mut inputs = HashMap::new();
    inputs.insert(principal, dec!(100.0));

    assert!(matches!(
        engine.evaluate(&inputs),
        Err(DagError::MissingDependency { id }) if id == principal
    ));
}

#[test]
fn three_node_cycle_is_detected() {
    let mut engine = CausalityEngine::new();

    let a = engine.add_input("a");
    let b = engine.add_formula(
        "b",
        FormulaKind::ReturnOnEquity {
            net_income: Port::Output(a),
            equity: Port::Constant(dec!(100.0)),
        },
    );
    let c = engine.add_formula(
        "c",
        FormulaKind::ReturnOnEquity {
            net_income: Port::Output(b),
            equity: Port::Constant(dec!(100.0)),
        },
    );
    let d = engine.add_formula(
        "d",
        FormulaKind::ReturnOnEquity {
            net_income: Port::Output(c),
            equity: Port::Constant(dec!(100.0)),
        },
    );

    engine.add_edge(a, b).unwrap();
    engine.add_edge(b, c).unwrap();
    engine.add_edge(c, d).unwrap();
    engine.add_edge(d, b).unwrap(); // closes the cycle

    assert!(matches!(
        engine.evaluate(&HashMap::new()),
        Err(DagError::CycleDetected { .. })
    ));
}

#[test]
fn formula_evaluation_error_includes_node_context() {
    let mut engine = CausalityEngine::new();

    let wacc = engine.add_formula(
        "wacc",
        FormulaKind::Wacc {
            equity_value: Port::Constant(dec!(600.0)),
            debt_value: Port::Constant(dec!(400.0)),
            cost_of_equity: Port::Constant(dec!(0.12)),
            cost_of_debt: Port::Constant(dec!(0.06)),
            tax_rate: Port::Constant(dec!(1.50)), // invalid tax rate
        },
    );

    let result = engine.evaluate(&HashMap::new());
    assert!(
        matches!(result, Err(DagError::FormulaEvaluation { node, .. }) if node == wacc),
        "expected FormulaEvaluation error tied to the WACC node, got {result:?}"
    );
}
