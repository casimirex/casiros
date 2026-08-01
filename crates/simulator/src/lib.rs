//! # CASIROS Multiverse Simulator
//!
//! This crate runs thousands of parallel "what-if" scenarios using Rayon and
//! aggregates the resulting financial metrics.
//!
//! ## Layer
//!
//! Application Layer — depends only on [`casiros_core`] (Domain Layer).

#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![deny(clippy::pedantic)]
#![deny(warnings)]
#![allow(clippy::needless_return)]

/// A single economic scenario — one "universe" in the multiverse.
///
/// Full implementation will contain all input variables that can be perturbed
/// during Monte Carlo simulation.
#[derive(Debug, Clone)]
pub struct Universe;

/// The complete set of computed metrics for a single universe.
#[derive(Debug, Clone)]
pub struct UniverseMetrics;

/// A stub Monte Carlo configuration for the MVP.
#[derive(Debug, Clone, Copy)]
pub struct MonteCarloConfig;

impl MonteCarloConfig {
    /// Creates a new default Monte Carlo configuration.
    #[must_use]
    pub fn new() -> Self {
        return Self;
    }
}

impl Default for MonteCarloConfig {
    fn default() -> Self {
        return Self::new();
    }
}
