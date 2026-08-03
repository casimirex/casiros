//! Input distributions for Monte Carlo simulation.
//!
//! A [`Distribution`] describes how to sample a single numeric input across the
//! multiverse. All samples are produced as `f64` and then converted to
//! [`rust_decimal::Decimal`] for evaluation.

use rust_decimal::Decimal;

/// A distribution that can be sampled to perturb a single input variable.
///
/// Distributions are intentionally simple for the MVP. They produce `f64`
/// samples which the simulator converts to [`Decimal`] before feeding into the
/// causality graph.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Distribution {
    /// A uniform distribution over the inclusive range `[low, high]`.
    Uniform {
        /// Lower bound of the range.
        low: f64,
        /// Upper bound of the range.
        high: f64,
    },
    /// A normal distribution with the given mean and standard deviation.
    Normal {
        /// Distribution mean.
        mean: f64,
        /// Standard deviation.
        std_dev: f64,
    },
    /// A fixed value — no randomness. Useful for control variables.
    Fixed {
        /// The constant value.
        value: f64,
    },
}

impl Distribution {
    /// Samples a single value from the distribution using the provided RNG.
    ///
    /// # Type Parameters
    ///
    /// - `R`: an RNG implementing [`rand::Rng`].
    ///
    /// # Panics
    ///
    /// Panics if a `Normal` distribution is constructed with a non-positive
    /// `std_dev`. This is a programmer error and should be validated at config
    /// time in a future release.
    ///
    /// # Examples
    ///
    /// ```
    /// use casiros_simulator::distribution::Distribution;
    /// use rand::SeedableRng;
    /// use rand::rngs::SmallRng;
    ///
    /// let mut rng = SmallRng::seed_from_u64(42);
    /// let dist = Distribution::Uniform { low: 0.0, high: 1.0 };
    /// let sample = dist.sample(&mut rng);
    /// assert!(sample >= 0.0);
    /// assert!(sample <= 1.0);
    /// ```
    #[must_use]
    pub fn sample<R: rand::Rng>(&self, rng: &mut R) -> f64 {
        match *self {
            Self::Uniform { low, high } => rng.random_range(low..=high),
            Self::Normal { mean, std_dev } => {
                let normal =
                    rand_distr::Normal::new(mean, std_dev).expect("std_dev must be positive");
                rng.sample(normal)
            }
            Self::Fixed { value } => value,
        }
    }

    /// Converts the distribution's nominal value to a [`Decimal`].
    ///
    /// This is a helper used when the caller already has a sampled `f64` and
    /// wants a deterministic `Decimal` representation.
    ///
    /// # Errors
    ///
    /// Returns [`casiros_core::prelude::CalculationError::Overflow`] if the
    /// sample cannot be represented as a `Decimal`.
    ///
    /// # Examples
    ///
    /// ```
    /// use casiros_simulator::distribution::Distribution;
    /// use rust_decimal_macros::dec;
    ///
    /// let decimal = Distribution::to_decimal(0.05).unwrap();
    /// assert_eq!(decimal.round_dp(2), dec!(0.05));
    /// ```
    pub fn to_decimal(sample: f64) -> Result<Decimal, casiros_core::prelude::CalculationError> {
        Decimal::try_from(sample).map_err(|_| casiros_core::prelude::CalculationError::Overflow {
            formula: "Distribution::to_decimal",
        })
    }
}
