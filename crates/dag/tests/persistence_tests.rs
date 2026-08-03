//! Integration tests for DAG save/load persistence.

use std::collections::HashMap;

use casiros_dag::graph::{CausalityEngine, FormulaKind, Port};
use casiros_dag::persistence::{EngineSnapshot, SnapshotNode, SnapshotNodeKind};
use rust_decimal_macros::dec;

#[test]
fn empty_engine_round_trips() {
    let engine = CausalityEngine::new();
    let snapshot = engine.to_snapshot();
    assert!(snapshot.nodes.is_empty());
    assert!(snapshot.edges.is_empty());

    let restored = CausalityEngine::from_snapshot(&snapshot).unwrap();
    assert!(restored.is_empty());
}

#[test]
fn input_and_formula_round_trip() {
    let mut engine = CausalityEngine::new();
    engine.add_input("principal");
    engine.add_formula(
        "fv",
        FormulaKind::FutureValue {
            present_value: Port::Constant(dec!(100.0)),
            rate: Port::Constant(dec!(0.05)),
            periods: Port::Constant(dec!(10)),
        },
    );

    let snapshot = engine.to_snapshot();
    assert_eq!(snapshot.nodes.len(), 2);
    assert!(snapshot.edges.is_empty());

    let restored = CausalityEngine::from_snapshot(&snapshot).unwrap();
    assert_eq!(restored.len(), 2);
}

#[test]
fn chained_graph_round_trip_preserves_evaluation() {
    let mut engine = CausalityEngine::new();
    let net_income = engine.add_input("net_income");
    let equity = engine.add_input("equity");
    let payout = engine.add_input("payout");

    let roe = engine.add_formula(
        "roe",
        FormulaKind::ReturnOnEquity {
            net_income: Port::Output(net_income),
            equity: Port::Output(equity),
        },
    );
    let sgr = engine.add_formula(
        "sgr",
        FormulaKind::SustainableGrowthRate {
            roe: Port::Output(roe),
            dividend_payout_ratio: Port::Output(payout),
        },
    );
    engine.add_edge(net_income, roe).unwrap();
    engine.add_edge(equity, roe).unwrap();
    engine.add_edge(roe, sgr).unwrap();
    engine.add_edge(payout, sgr).unwrap();

    let snapshot = engine.to_snapshot();
    assert_eq!(snapshot.nodes.len(), 5);
    assert_eq!(snapshot.edges.len(), 4);

    let restored = CausalityEngine::from_snapshot(&snapshot).unwrap();

    let mut inputs = HashMap::new();
    inputs.insert(net_income, dec!(150.0));
    inputs.insert(equity, dec!(1000.0));
    inputs.insert(payout, dec!(0.40));

    let original_outputs = engine.evaluate(&inputs).unwrap();
    // Evaluate the restored engine using the original input ids because the
    // snapshot preserves insertion order and node ids are sequential.
    let restored_outputs = restored.evaluate(&inputs).unwrap();

    assert_eq!(original_outputs.get(&sgr), restored_outputs.get(&sgr));
}

#[test]
fn snapshot_serializes_to_valid_json() {
    let mut engine = CausalityEngine::new();
    engine.add_input("principal");

    let snapshot = engine.to_snapshot();
    let json = serde_json::to_string_pretty(&snapshot).unwrap();
    assert!(json.contains("principal"));

    let deserialized: EngineSnapshot = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized, snapshot);
}

#[test]
fn from_snapshot_rejects_unknown_edge_node() {
    let snapshot = EngineSnapshot {
        nodes: vec![SnapshotNode {
            name: "only".to_string(),
            kind: SnapshotNodeKind::Input,
        }],
        edges: vec![("only".to_string(), "missing".to_string())],
    };

    let err = CausalityEngine::from_snapshot(&snapshot).unwrap_err();
    assert!(err.to_string().contains("missing"));
}
