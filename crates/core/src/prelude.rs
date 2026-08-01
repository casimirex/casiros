//! Convenience re-exports for consumers of `casiros_core`.
//!
//! Import everything in this module to get the most commonly used types and
//! macros without multiple `use` statements.

pub use crate::error::CalculationError;
pub use crate::types::{Amounts, Dollar, Periods, Rate, Ratio};
pub use rust_decimal::Decimal;
pub use rust_decimal::MathematicalOps;
pub use rust_decimal_macros::dec;
