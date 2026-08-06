//! Benchmark Monte Carlo simulation throughput.
#![allow(missing_docs)]

use casiros_dag::graph::{CausalityEngine, FormulaKind, Port};
use casiros_simulator::distribution::Distribution;
use casiros_simulator::simulation::MonteCarloConfig;
use criterion::{Criterion, criterion_group, criterion_main};
use rust_decimal_macros::dec;

/// Builds a tiny FV DAG and returns the engine, input ids, and target node.
fn build_fv_engine() -> (
    CausalityEngine,
    casiros_dag::graph::NodeId,
    casiros_dag::graph::NodeId,
    casiros_dag::graph::NodeId,
) {
    let mut engine = CausalityEngine::new();
    let principal = engine.add_input("principal");
    let rate = engine.add_input("rate");
    let fv = engine.add_formula(
        "fv",
        FormulaKind::FutureValue {
            present_value: Port::Output(principal),
            rate: Port::Output(rate),
            periods: Port::Constant(dec!(10)),
        },
    );
    engine.add_edge(principal, fv).unwrap();
    engine.add_edge(rate, fv).unwrap();
    (engine, fv, principal, rate)
}

fn bench_simulator_run(c: &mut Criterion) {
    let (engine, target, principal, rate) = build_fv_engine();
    let mut config = MonteCarloConfig::new(10_000, 42).unwrap();
    config.bind(
        principal,
        Distribution::Uniform {
            low: 90.0,
            high: 110.0,
        },
    );
    config.bind(
        rate,
        Distribution::Uniform {
            low: 0.03,
            high: 0.07,
        },
    );

    c.bench_function("simulator_run_10k_universes", |b| {
        b.iter(|| config.run(&engine, target).unwrap());
    });
}

criterion_group!(benches, bench_simulator_run);
criterion_main!(benches);
