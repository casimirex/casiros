//! # CASIROS Causality Graph Engine
//!
//! This crate builds the directed acyclic graph (DAG) of formula dependencies
//! and evaluates formulas in topological order.
//!
//! ## Layer
//!
//! Application Layer — depends only on [`casiros_core`] (Domain Layer).

#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![deny(clippy::pedantic)]
#![deny(warnings)]
#![allow(clippy::needless_return)]

/// Formula node identifiers used in the causality graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FormulaNode {
    /// Future value of a present sum.
    FutureValue,
    /// Present value of a future sum.
    PresentValue,
    /// Return on equity.
    ReturnOnEquity,
    /// Weighted average cost of capital.
    Wacc,
    /// Sustainable growth rate.
    SustainableGrowthRate,
}

/// A stub causality engine for the MVP.
///
/// Full implementation will construct a [`petgraph::graph::DiGraph`] of all
/// formula dependencies and provide topological evaluation.
#[derive(Debug, Clone)]
pub struct CausalityEngine;

impl CausalityEngine {
    /// Creates a new, empty causality engine.
    #[must_use]
    pub fn new() -> Self {
        return Self;
    }
}

impl Default for CausalityEngine {
    fn default() -> Self {
        return Self::new();
    }
}
