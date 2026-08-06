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

/// Computes the Capital Adequacy Ratio (CAR).
///
/// # Mathematical Definition
///
/// \[ CAR = \frac{\text{Total Capital}}{\text{Risk-Weighted Assets}} \]
///
/// # Errors
///
/// Returns [`CalculationError::DivisionByZero`] if `risk_weighted_assets` is zero.
///
/// # Examples
///
/// ```
/// use casiros_core::banking::capital_adequacy_ratio;
/// use rust_decimal_macros::dec;
///
/// let car = capital_adequacy_ratio(dec!(100.0), dec!(1000.0)).unwrap();
/// assert_eq!(car, dec!(0.1));
/// assert!(car > dec!(0.0)); // Assertion 2
/// ```
pub fn capital_adequacy_ratio(
    total_capital: Decimal,
    risk_weighted_assets: Decimal,
) -> Result<Decimal, CalculationError> {
    if risk_weighted_assets == Decimal::ZERO {
        return Err(CalculationError::DivisionByZero {
            formula: "Capital Adequacy Ratio (CAR)",
        });
    }
    return Ok(total_capital / risk_weighted_assets);
}

/// Computes the Provision Coverage Ratio (PCR).
///
/// # Mathematical Definition
///
/// \[ PCR = \frac{\text{Provisions}}{\text{Non-Performing Assets}} \]
///
/// # Errors
///
/// Returns [`CalculationError::DivisionByZero`] if `non_performing_assets` is zero.
///
/// # Examples
///
/// ```
/// use casiros_core::banking::provision_coverage_ratio;
/// use rust_decimal_macros::dec;
///
/// let pcr = provision_coverage_ratio(dec!(80.0), dec!(100.0)).unwrap();
/// assert_eq!(pcr, dec!(0.8));
/// assert!(pcr < dec!(1.0)); // Assertion 2
/// ```
pub fn provision_coverage_ratio(
    provisions: Decimal,
    non_performing_assets: Decimal,
) -> Result<Decimal, CalculationError> {
    if non_performing_assets == Decimal::ZERO {
        return Err(CalculationError::DivisionByZero {
            formula: "Provision Coverage Ratio (PCR)",
        });
    }
    return Ok(provisions / non_performing_assets);
}
