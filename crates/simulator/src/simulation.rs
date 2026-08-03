//! Monte Carlo simulation runner.
//!
//! This module provides [`MonteCarloConfig`] and [`SimulationResult`]. A
//! `MonteCarloConfig` binds input nodes in a [`CausalityEngine`] to
//! [`Distribution`]s and runs a fixed number of independent universes. Each
//! universe produces a full set of DAG outputs; the runner then aggregates a
//! target node's output into mean, median, and percentile statistics.

use std::collections::HashMap;

use casiros_core::prelude::Decimal;
use casiros_dag::graph::{CausalityEngine, NodeId};
use rand::SeedableRng;
use rand::rngs::SmallRng;
use rayon::prelude::*;
use rust_decimal_macros::dec;

use crate::distribution::Distribution;
use crate::error::SimulationError;

/// Statistics aggregated across all simulated universes for a single target
/// node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SimulationResult {
    /// Number of universes that contributed to the statistics.
    pub count: usize,

    /// Arithmetic mean of the target node across all universes.
    pub mean: Decimal,

    /// Median value of the target node across all universes.
    pub median: Decimal,

    /// Minimum observed value of the target node.
    pub min: Decimal,

    /// Maximum observed value of the target node.
    pub max: Decimal,
}

/// A single binding between a DAG input node and the distribution used to
/// perturb it.
#[derive(Debug, Clone, Copy)]
pub struct InputBinding {
    /// The input node whose value will be sampled each universe.
    pub node: NodeId,
    /// The distribution to sample from.
    pub distribution: Distribution,
}

/// Configuration for a Monte Carlo run.
///
/// # Examples
///
/// ```
/// use casiros_simulator::simulation::MonteCarloConfig;
///
/// let config = MonteCarloConfig::new(1_000, 42).unwrap();
/// assert_eq!(config.universe_count(), 1_000);
/// ```
#[derive(Debug, Clone)]
pub struct MonteCarloConfig {
    universe_count: usize,
    seed: u64,
    bindings: Vec<InputBinding>,
}

impl MonteCarloConfig {
    /// Creates a new configuration with the given number of universes and RNG
    /// seed.
    ///
    /// # Errors
    ///
    /// Returns [`SimulationError::InvalidUniverseCount`] if `universe_count` is
    /// zero.
    ///
    /// # Examples
    ///
    /// ```
    /// use casiros_simulator::simulation::MonteCarloConfig;
    ///
    /// let config = MonteCarloConfig::new(1_000, 42).unwrap();
    /// assert_eq!(config.universe_count(), 1_000);
    /// ```
    pub fn new(universe_count: usize, seed: u64) -> Result<Self, SimulationError> {
        if universe_count == 0 {
            return Err(SimulationError::InvalidUniverseCount {
                count: universe_count,
            });
        }
        return Ok(Self {
            universe_count,
            seed,
            bindings: Vec::new(),
        });
    }

    /// Returns the configured number of universes.
    #[must_use]
    pub fn universe_count(&self) -> usize {
        return self.universe_count;
    }

    /// Returns the RNG seed used for reproducibility.
    #[must_use]
    pub fn seed(&self) -> u64 {
        return self.seed;
    }

    /// Binds an input node to a distribution.
    ///
    /// # Examples
    ///
    /// ```
    /// use casiros_dag::graph::{CausalityEngine, Port};
    /// use casiros_simulator::distribution::Distribution;
    /// use casiros_simulator::simulation::MonteCarloConfig;
    /// use rust_decimal_macros::dec;
    ///
    /// let mut engine = CausalityEngine::new();
    /// let principal = engine.add_input("principal");
    /// let mut config = MonteCarloConfig::new(100, 42).unwrap();
    /// config.bind(principal, Distribution::Uniform { low: 90.0, high: 110.0 });
    /// assert_eq!(config.bindings().len(), 1);
    /// ```
    pub fn bind(&mut self, node: NodeId, distribution: Distribution) {
        self.bindings.push(InputBinding { node, distribution });
    }

    /// Returns the current input bindings (read-only).
    #[must_use]
    pub fn bindings(&self) -> &[InputBinding] {
        return &self.bindings;
    }

    /// Runs the simulation against the provided engine and target node.
    ///
    /// For each universe, the config samples every bound input, evaluates the
    /// engine, and records the value of `target_node`. The returned
    /// [`SimulationResult`] contains aggregated statistics across all universes.
    ///
    /// # Errors
    ///
    /// - [`SimulationError::MissingSamplers`] if no bindings were registered.
    /// - [`SimulationError::InvalidSample`] if a sampled `f64` cannot be
    ///   converted to `Decimal`.
    /// - [`SimulationError::MissingTarget`] if `target_node` is absent from the
    ///   DAG outputs.
    /// - [`SimulationError::EvaluationFailure`] if the engine fails in any
    ///   universe.
    ///
    /// # Examples
    ///
    /// ```
    /// use casiros_dag::graph::{CausalityEngine, FormulaKind, Port};
    /// use casiros_simulator::distribution::Distribution;
    /// use casiros_simulator::simulation::MonteCarloConfig;
    /// use rust_decimal_macros::dec;
    ///
    /// let mut engine = CausalityEngine::new();
    /// let principal = engine.add_input("principal");
    /// let rate = engine.add_input("rate");
    /// let fv = engine.add_formula(
    ///     "future_value",
    ///     FormulaKind::FutureValue {
    ///         present_value: Port::Output(principal),
    ///         rate: Port::Output(rate),
    ///         periods: Port::Constant(dec!(10)),
    ///     },
    /// );
    /// engine.add_edge(principal, fv).unwrap();
    /// engine.add_edge(rate, fv).unwrap();
    ///
    /// let mut config = MonteCarloConfig::new(1_000, 42).unwrap();
    /// config.bind(principal, Distribution::Uniform { low: 90.0, high: 110.0 });
    /// config.bind(rate, Distribution::Uniform { low: 0.03, high: 0.07 });
    ///
    /// let result = config.run(&engine, fv).unwrap();
    /// assert!(result.mean > dec!(0.0));
    /// assert!(result.count == 1_000);
    /// ```
    pub fn run(
        &self,
        engine: &CausalityEngine,
        target_node: NodeId,
    ) -> Result<SimulationResult, SimulationError> {
        if self.bindings.is_empty() {
            return Err(SimulationError::MissingSamplers);
        }

        let samples: Result<Vec<Decimal>, SimulationError> = (0..self.universe_count)
            .into_par_iter()
            .map(|idx| {
                let mut rng = SmallRng::seed_from_u64(self.seed.wrapping_add(idx as u64));
                let mut inputs: HashMap<NodeId, Decimal> =
                    HashMap::with_capacity(self.bindings.len());
                for binding in &self.bindings {
                    let sample = binding.distribution.sample(&mut rng);
                    let decimal = Distribution::to_decimal(sample).map_err(|_| {
                        SimulationError::InvalidSample {
                            input: format!("node {:?}", binding.node),
                            sample,
                        }
                    })?;
                    inputs.insert(binding.node, decimal);
                }

                let outputs =
                    engine
                        .evaluate(&inputs)
                        .map_err(|err| SimulationError::EvaluationFailure {
                            universe: idx,
                            source: err,
                        })?;

                outputs
                    .get(&target_node)
                    .copied()
                    .ok_or(SimulationError::MissingTarget { node: target_node })
            })
            .collect();

        let values = samples?;
        return Self::aggregate(values);
    }

    fn aggregate(mut values: Vec<Decimal>) -> Result<SimulationResult, SimulationError> {
        let count = values.len();
        if count == 0 {
            return Err(SimulationError::InvalidUniverseCount { count: 0 });
        }

        values.sort_unstable();

        let sum: Decimal = values.iter().sum();
        if sum == Decimal::ZERO {
            return Ok(SimulationResult {
                count,
                mean: dec!(0.0),
                median: dec!(0.0),
                min: dec!(0.0),
                max: dec!(0.0),
            });
        }
        let mean = sum / Decimal::from(count as u64);
        let median = values[count / 2];
        let min = values[0];
        let max = values[count - 1];

        return Ok(SimulationResult {
            count,
            mean,
            median,
            min,
            max,
        });
    }
}

impl Default for MonteCarloConfig {
    fn default() -> Self {
        return Self::new(1_000, 42).expect("1_000 is a valid universe count");
    }
}
