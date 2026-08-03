//! Trait for compile-time narrative generation.
//!
//! The [`Narrative`] trait is implemented by the `#[derive(Narrative)]` macro
//! provided by `casiros_macros`. It produces a human-readable sentence from
//! a struct's fields, suitable for CFO memos, audit trails, and report
//! summaries.

/// Types that can describe themselves as a human-readable sentence.
///
/// Implementations are typically generated automatically by the
/// `#[derive(Narrative)]` procedural macro from `casiros_macros`. The
/// generated text includes each field's name and its `Display` value.
///
/// # Examples
///
/// ```
/// use casiros_core::narrative::Narrative;
///
/// struct Position {
///     asset: String,
///     value: casiros_core::prelude::Decimal,
/// }
///
/// impl Narrative for Position {
///     fn narrative(&self) -> String {
///         format!("Position: asset = {}, value = {}", self.asset, self.value)
///     }
/// }
///
/// let position = Position {
///     asset: "AAPL".to_string(),
///     value: casiros_core::prelude::Decimal::from(1000),
/// };
/// assert!(position.narrative().contains("AAPL"));
/// ```
pub trait Narrative {
    /// Returns a human-readable description of the value.
    fn narrative(&self) -> String;
}
