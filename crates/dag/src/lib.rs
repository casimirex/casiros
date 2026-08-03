//! # CASIROS Causality Graph Engine
//!
//! This crate builds the directed acyclic graph (DAG) of formula dependencies
//! and evaluates formulas in topological order.
//!
//! ## Layer
//!
//! Application Layer — depends only on [`casiros_core`] (Domain Layer).
//!
//! ## Public API
//!
//! - [`graph::CausalityEngine`] — the DAG builder and evaluator.
//! - [`graph::Node`], [`graph::NodeId`], [`graph::NodeKind`], [`graph::Port`],
//!   [`graph::FormulaKind`] — graph building blocks.
//! - [`error::DagError`] — the universal error type for DAG operations.

#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![deny(clippy::pedantic)]
#![deny(warnings)]
#![allow(clippy::needless_return)]

pub mod error;
pub mod graph;
pub mod persistence;

pub use error::DagError;
pub use graph::{CausalityEngine, FormulaKind, Node, NodeId, NodeKind, Port};
pub use persistence::{EngineSnapshot, SnapshotNode, SnapshotNodeKind};
