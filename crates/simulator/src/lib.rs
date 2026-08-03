//! # CASIROS Multiverse Simulator
//!
//! This crate runs thousands of reproducible "what-if" scenarios against a
//! [`casiros_dag::graph::CausalityEngine`] and aggregates the resulting
//! financial metrics.
//!
//! ## Layer
//!
//! Application Layer — depends on [`casiros_dag`] (Application Layer) and
//! [`casiros_core`] (Domain Layer).
//!
//! ## Public API
//!
//! - [`simulation::MonteCarloConfig`] — configuration and runner for a Monte
//!   Carlo sweep.
//! - [`simulation::SimulationResult`] — aggregated statistics for a target
//!   node.
//! - [`distribution::Distribution`] — input distributions (uniform, normal,
//!   fixed).
//! - [`error::SimulationError`] — universal error type for simulator operations.

#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![deny(clippy::pedantic)]
#![deny(warnings)]
#![allow(clippy::needless_return)]

pub mod distribution;
pub mod error;
pub mod simulation;

pub use distribution::Distribution;
pub use error::SimulationError;
pub use simulation::{MonteCarloConfig, SimulationResult};
