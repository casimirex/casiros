//! Integration tests for the CASIROS Monte Carlo simulator.

use casiros_dag::graph::{CausalityEngine, FormulaKind, Port};
use casiros_simulator::distribution::Distribution;
use casiros_simulator::simulation::MonteCarloConfig;
use rand::SeedableRng;
use rust_decimal_macros::dec;

#[test]
fn uniform_distribution_bounds_are_respected() {
    use rand::rngs::SmallRng;
    let mut rng = SmallRng::seed_from_u64(42);
    let dist = Distribution::Uniform {
        low: 90.0,
        high: 110.0,
    };

    for _ in 0..1_000 {
        let sample = dist.sample(&mut rng);
        assert!(sample >= 90.0);
        assert!(sample <= 110.0);
    }
}

#[test]
fn fixed_distribution_returns_constant() {
    use rand::rngs::SmallRng;
    let mut rng = SmallRng::seed_from_u64(42);
    let dist = Distribution::Fixed { value: 5.0 };

    for _ in 0..100 {
        let sample = dist.sample(&mut rng);
        assert!((sample - 5.0).abs() < f64::EPSILON);
    }
}

#[test]
fn monte_carlo_mean_is_within_expected_range() {
    let mut engine = CausalityEngine::new();
    let principal = engine.add_input("principal");
    let fv = engine.add_formula(
        "future_value",
        FormulaKind::FutureValue {
            present_value: Port::Output(principal),
            rate: Port::Constant(dec!(0.05)),
            periods: Port::Constant(dec!(1)),
        },
    );
    engine.add_edge(principal, fv).unwrap();

    let mut config = MonteCarloConfig::new(2_000, 42).unwrap();
    config.bind(
        principal,
        Distribution::Uniform {
            low: 0.0,
            high: 200.0,
        },
    );

    let result = config.run(&engine, fv).unwrap();
    assert_eq!(result.count, 2_000);
    // Uniform[0, 200] mean is 100; FV_1 = PV * 1.05, so expected mean ≈ 105.
    assert!(result.mean > dec!(95.0), "mean too low: {}", result.mean);
    assert!(result.mean < dec!(115.0), "mean too high: {}", result.mean);
    assert!(result.min >= dec!(0.0));
    assert!(result.max <= dec!(210.0));
}

#[test]
fn reproducible_seed_produces_identical_results() {
    let mut engine = CausalityEngine::new();
    let rate = engine.add_input("rate");
    let fv = engine.add_formula(
        "future_value",
        FormulaKind::FutureValue {
            present_value: Port::Constant(dec!(100.0)),
            rate: Port::Output(rate),
            periods: Port::Constant(dec!(5)),
        },
    );
    engine.add_edge(rate, fv).unwrap();

    let mut config = MonteCarloConfig::new(500, 123).unwrap();
    config.bind(
        rate,
        Distribution::Uniform {
            low: 0.01,
            high: 0.10,
        },
    );

    let first = config.run(&engine, fv).unwrap();
    let second = config.run(&engine, fv).unwrap();

    assert_eq!(first.mean, second.mean);
    assert_eq!(first.median, second.median);
    assert_eq!(first.min, second.min);
    assert_eq!(first.max, second.max);
}

#[test]
fn rejects_zero_universes() {
    assert!(MonteCarloConfig::new(0, 42).is_err());
}

#[test]
fn rejects_missing_samplers() {
    let mut engine = CausalityEngine::new();
    let target = engine.add_input("target");
    let config = MonteCarloConfig::new(100, 42).unwrap();

    assert!(matches!(
        config.run(&engine, target),
        Err(casiros_simulator::SimulationError::MissingSamplers)
    ));
}
