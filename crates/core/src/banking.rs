//! Banking-sector financial metrics.

use super::prelude::*;
use rust_decimal::Decimal;

/// Computes Net Interest Margin (NIM).
///
/// # Mathematical Definition
///
/// \[ NIM = \frac{\text{Interest Income} - \text{Interest Expense}}{\text{Average Earning Assets}} \]
///
/// # Errors
///
/// Returns [`CalculationError::DivisionByZero`] if `avg_earning_assets` is zero.
///
/// # Examples
///
/// ```
/// use casiros_core::banking::net_interest_margin;
/// use rust_decimal_macros::dec;
///
/// let nim = net_interest_margin(dec!(1200.0), dec!(400.0), dec!(10000.0)).unwrap();
/// assert_eq!(nim, dec!(0.08));
/// assert!(nim > dec!(0.0)); // Assertion 2
/// ```
pub fn net_interest_margin(
    interest_income: Decimal,
    interest_expense: Decimal,
    avg_earning_assets: Decimal,
) -> Result<Decimal, CalculationError> {
    if avg_earning_assets == Decimal::ZERO {
        return Err(CalculationError::DivisionByZero {
            formula: "Net Interest Margin (NIM)",
        });
    }
    let net_interest_income = interest_income - interest_expense;
    return Ok(net_interest_income / avg_earning_assets);
}

/// Computes the Loan-to-Deposit Ratio (LDR).
///
/// # Mathematical Definition
///
/// \[ LDR = \frac{\text{Total Loans}}{\text{Total Deposits}} \]
///
/// # Errors
///
/// Returns [`CalculationError::DivisionByZero`] if `total_deposits` is zero.
///
/// # Examples
///
/// ```
/// use casiros_core::banking::loan_to_deposit_ratio;
/// use rust_decimal_macros::dec;
///
/// let ldr = loan_to_deposit_ratio(dec!(800.0), dec!(1000.0)).unwrap();
/// assert_eq!(ldr, dec!(0.8));
/// assert!(ldr < dec!(1.0)); // Assertion 2
/// ```
pub fn loan_to_deposit_ratio(
    total_loans: Decimal,
    total_deposits: Decimal,
) -> Result<Decimal, CalculationError> {
    if total_deposits == Decimal::ZERO {
        return Err(CalculationError::DivisionByZero {
            formula: "Loan-to-Deposit Ratio (LDR)",
        });
    }
    return Ok(total_loans / total_deposits);
}
