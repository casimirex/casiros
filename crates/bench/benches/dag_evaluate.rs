//! Benchmark DAG evaluation throughput, with and without formula cache.
#![allow(missing_docs)]

use std::collections::HashMap;
use std::sync::Arc;

use casiros_dag::cache::InMemoryFormulaCache;
use casiros_dag::graph::{CausalityEngine, FormulaKind, Port};
use criterion::{Criterion, criterion_group, criterion_main};
use rust_decimal_macros::dec;

/// Builds a small chain: inputs -> roe -> sgr.
fn build_chain_engine() -> (
    CausalityEngine,
    HashMap<casiros_dag::graph::NodeId, rust_decimal::Decimal>,
) {
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

    let mut inputs = HashMap::new();
    inputs.insert(net_income, dec!(150.0));
    inputs.insert(equity, dec!(1000.0));
    inputs.insert(payout, dec!(0.40));

    (engine, inputs)
}

fn bench_dag_evaluate(c: &mut Criterion) {
    let (engine, inputs) = build_chain_engine();
    c.bench_function("dag_evaluate_chain", |b| {
        b.iter(|| engine.evaluate(&inputs).unwrap());
    });
}

fn bench_dag_evaluate_with_cache(c: &mut Criterion) {
    let cache = Arc::new(InMemoryFormulaCache::new());
    let (engine, inputs) = build_chain_engine();
    let engine = engine.with_cache(cache);

    // First evaluation populates the cache.
    engine.evaluate(&inputs).unwrap();

    c.bench_function("dag_evaluate_cached", |b| {
        b.iter(|| engine.evaluate(&inputs).unwrap());
    });
}

criterion_group!(benches, bench_dag_evaluate, bench_dag_evaluate_with_cache);
criterion_main!(benches);
