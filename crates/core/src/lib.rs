//! # CASIROS Core Mathematics
//!
//! This crate contains the fundamental, immutable financial formulas that form
//! the computational backbone of CASIROS.
//!
//! ## Design Principles
//!
//! - **Purity:** Every public function is a pure computation. Given the same
//!   inputs, it always produces the same output. No I/O, no global state, no
//!   side effects.
//! - **Decimal Precision:** All monetary values and ratios use
//!   [`rust_decimal::Decimal`]. IEEE 754 floating-point (`f32`, `f64`) is
//!   **banned** for financial computation.
//! - **Defensive:** Every function validates its preconditions and returns
//!   [`Result<T, CalculationError>`]. No panics in business logic.
//! - **Documented:** Every public item has a doc-comment with at least one
//!   comprehensive doc-test demonstrating usage and edge cases.
//!
//! ## Compiler Directives (NASA JPL Standard)
//!
//! - `#![forbid(unsafe_code)]` — Memory safety is absolute.
//! - `#![deny(missing_docs)]` — Undocumented public items are compile errors.
//! - `#![deny(clippy::pedantic)]` — All Clippy lints are hard errors.
//! - `#![deny(warnings)]` — Warnings are compile errors.

#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![deny(clippy::pedantic)]
#![deny(clippy::missing_docs_in_private_items)]
#![deny(clippy::implicit_return)]
#![deny(clippy::cognitive_complexity)]
#![deny(unreachable_pub)]
#![deny(rust_2018_idioms)]
#![allow(clippy::doc_markdown)] // Math docs are full of un-backticked symbols
#![allow(clippy::needless_return)] // Explicit returns preferred for auditability

pub mod prelude {
    //! Re-exports for ergonomic use across the workspace.
    pub use crate::error::CalculationError;
    pub use crate::types::{Amounts, Dollar, Periods, Rate, Ratio};
    pub use rust_decimal::Decimal;
    pub use rust_decimal_macros::dec;
}

pub mod banking;
pub mod corporate;
pub mod error;
pub mod financial;
pub mod general;
pub mod markets;
pub mod stocks_bonds;
pub mod types;
