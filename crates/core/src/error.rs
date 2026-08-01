//! Universal error type for all CASIROS computations.

use rust_decimal::Decimal;
use thiserror::Error;

/// The universal error type for all CASIROS computations.
///
/// Every fallible operation in the core crate returns `Result<T, CalculationError>`.
/// No function in the business logic may panic — all error paths are enumerated here.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum CalculationError {
    /// A division by zero was attempted.
    #[error("Division by zero in {formula}")]
    DivisionByZero {
        /// The name of the formula where the error occurred.
        formula: &'static str,
    },

    /// A negative or zero value was provided where a strictly positive value is required.
    #[error("Invalid value {value} in {context}: must be strictly positive")]
    NegativeValueInvalid {
        /// The formula or parameter name for context.
        context: &'static str,
        /// The invalid value that was provided.
        value: Decimal,
    },

    /// A value outside the valid range [0, 1] was provided for a ratio or probability.
    #[error("Value {value} in {context} is outside the valid range [0, 1]")]
    RangeViolation {
        /// The parameter name for context.
        context: &'static str,
        /// The out-of-range value.
        value: Decimal,
    },

    /// Logarithm of a non-positive number was attempted.
    #[error("Cannot compute logarithm of {value}: must be strictly positive")]
    LogarithmDomainError {
        /// The non-positive value.
        value: Decimal,
    },

    /// An invalid interest rate was provided (must be > -1.0 for compounding).
    #[error("Invalid rate {rate}: must be greater than -1.0 (i.e., > -100%)")]
    InvalidRate {
        /// The invalid rate.
        rate: Decimal,
    },

    /// A numeric overflow occurred during computation.
    #[error("Numeric overflow in {formula}")]
    Overflow {
        /// The formula where overflow occurred.
        formula: &'static str,
    },

    /// An iterative algorithm failed to converge within the maximum number of iterations.
    #[error("{formula} failed to converge after {iterations} iterations")]
    ConvergenceFailure {
        /// The formula being computed.
        formula: &'static str,
        /// The number of iterations attempted.
        iterations: u32,
    },

    /// A required input parameter was missing or invalid in the DAG context.
    #[error("Missing required input '{parameter}' for formula '{formula}'")]
    MissingInput {
        /// The formula that requires the input.
        formula: &'static str,
        /// The missing parameter name.
        parameter: &'static str,
    },
}
