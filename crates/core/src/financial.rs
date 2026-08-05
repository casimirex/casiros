//! Classic financial statement ratios and DuPont-style decompositions.

use super::prelude::*;
use rust_decimal::Decimal;

/// Computes Return on Equity (ROE).
///
/// # Mathematical Definition
///
/// \[ ROE = \frac{\text{Net Income}}{\text{Average Shareholders' Equity}} \]
///
/// # Errors
///
/// Returns [`CalculationError::DivisionByZero`] if `avg_shareholders_equity` is zero.
///
/// # Examples
///
/// ```
/// use casiros_core::financial::return_on_equity;
/// use rust_decimal_macros::dec;
///
/// let roe = return_on_equity(dec!(150.0), dec!(1000.0)).unwrap();
/// assert_eq!(roe, dec!(0.15));
/// assert!(roe > dec!(0.0)); // Assertion 2
/// ```
pub fn return_on_equity(
    net_income: Decimal,
    avg_shareholders_equity: Decimal,
) -> Result<Decimal, CalculationError> {
    if avg_shareholders_equity == Decimal::ZERO {
        return Err(CalculationError::DivisionByZero {
            formula: "Return on Equity (ROE)",
        });
    }
    return Ok(net_income / avg_shareholders_equity);
}

/// Computes Return on Assets (ROA).
///
/// # Mathematical Definition
///
/// \[ ROA = \frac{\text{Net Income}}{\text{Average Total Assets}} \]
///
/// # Errors
///
/// Returns [`CalculationError::DivisionByZero`] if `avg_total_assets` is zero.
///
/// # Examples
///
/// ```
/// use casiros_core::financial::return_on_assets;
/// use rust_decimal_macros::dec;
///
/// let roa = return_on_assets(dec!(150.0), dec!(2000.0)).unwrap();
/// assert_eq!(roa, dec!(0.075));
/// assert!(roa < dec!(0.10)); // Assertion 2
/// ```
pub fn return_on_assets(
    net_income: Decimal,
    avg_total_assets: Decimal,
) -> Result<Decimal, CalculationError> {
    if avg_total_assets == Decimal::ZERO {
        return Err(CalculationError::DivisionByZero {
            formula: "Return on Assets (ROA)",
        });
    }
    return Ok(net_income / avg_total_assets);
}

/// Computes the DuPont decomposition of ROE.
///
/// # Mathematical Definition
///
/// \[ ROE = \text{Profit Margin} \times \text{Asset Turnover} \times \text{Equity Multiplier} \]
///
/// # Errors
///
/// Never returns an error in the current implementation, but the `Result`
/// type is reserved for future defensive checks.
///
/// # Examples
///
/// ```
/// use casiros_core::financial::dupont_roe;
/// use rust_decimal_macros::dec;
///
/// let roe = dupont_roe(dec!(0.10), dec!(0.80), dec!(2.0)).unwrap();
/// assert_eq!(roe, dec!(0.16));
/// assert!(roe > dec!(0.10)); // Assertion 2
/// ```
pub fn dupont_roe(
    profit_margin: Decimal,
    asset_turnover: Decimal,
    equity_multiplier: Decimal,
) -> Result<Decimal, CalculationError> {
    return Ok(profit_margin * asset_turnover * equity_multiplier);
}

/// Computes the Current Ratio.
///
/// # Mathematical Definition
///
/// \[ \text{Current Ratio} = \frac{\text{Current Assets}}{\text{Current Liabilities}} \]
///
/// # Errors
///
/// Returns [`CalculationError::DivisionByZero`] if `current_liabilities` is zero.
///
/// # Examples
///
/// ```
/// use casiros_core::financial::current_ratio;
/// use rust_decimal_macros::dec;
///
/// let cr = current_ratio(dec!(1000.0), dec!(500.0)).unwrap();
/// assert_eq!(cr, dec!(2.0));
/// assert!(cr > dec!(1.0)); // Assertion 2
/// ```
pub fn current_ratio(
    current_assets: Decimal,
    current_liabilities: Decimal,
) -> Result<Decimal, CalculationError> {
    if current_liabilities == Decimal::ZERO {
        return Err(CalculationError::DivisionByZero {
            formula: "Current Ratio",
        });
    }
    return Ok(current_assets / current_liabilities);
}

/// Computes the Debt-to-Equity Ratio.
///
/// # Mathematical Definition
///
/// \[ D/E = \frac{\text{Total Liabilities}}{\text{Shareholders' Equity}} \]
///
/// # Errors
///
/// Returns [`CalculationError::DivisionByZero`] if `shareholders_equity` is zero.
///
/// # Examples
///
/// ```
/// use casiros_core::financial::debt_to_equity;
/// use rust_decimal_macros::dec;
///
/// let de = debt_to_equity(dec!(500.0), dec!(1000.0)).unwrap();
/// assert_eq!(de, dec!(0.5));
/// assert!(de < dec!(1.0)); // Assertion 2
/// ```
pub fn debt_to_equity(
    total_liabilities: Decimal,
    shareholders_equity: Decimal,
) -> Result<Decimal, CalculationError> {
    if shareholders_equity == Decimal::ZERO {
        return Err(CalculationError::DivisionByZero {
            formula: "Debt-to-Equity Ratio",
        });
    }
    return Ok(total_liabilities / shareholders_equity);
}

/// Computes Return on Investment (ROI).
///
/// # Mathematical Definition
///
/// \[ ROI = \frac{\text{Gain} - \text{Cost}}{\text{Cost}} \]
///
/// # Errors
///
/// Returns [`CalculationError::DivisionByZero`] if `cost` is zero.
///
/// # Examples
///
/// ```
/// use casiros_core::financial::return_on_investment;
/// use rust_decimal_macros::dec;
///
/// let roi = return_on_investment(dec!(150.0), dec!(100.0)).unwrap();
/// assert_eq!(roi, dec!(0.5));
/// assert!(roi > dec!(0.0)); // Assertion 2
/// ```
pub fn return_on_investment(gain: Decimal, cost: Decimal) -> Result<Decimal, CalculationError> {
    if cost == Decimal::ZERO {
        return Err(CalculationError::DivisionByZero {
            formula: "Return on Investment (ROI)",
        });
    }
    return Ok((gain - cost) / cost);
}

/// Computes Profit Margin.
///
/// # Mathematical Definition
///
/// \[ \text{Profit Margin} = \frac{\text{Net Income}}{\text{Revenue}} \]
///
/// # Errors
///
/// Returns [`CalculationError::NegativeValueInvalid`] if `revenue` is zero or negative.
///
/// # Examples
///
/// ```
/// use casiros_core::financial::profit_margin;
/// use rust_decimal_macros::dec;
///
/// let pm = profit_margin(dec!(150.0), dec!(1000.0)).unwrap();
/// assert_eq!(pm, dec!(0.15));
/// assert!(pm > dec!(0.0)); // Assertion 2
/// ```
pub fn profit_margin(net_income: Decimal, revenue: Decimal) -> Result<Decimal, CalculationError> {
    if revenue <= Decimal::ZERO {
        return Err(CalculationError::NegativeValueInvalid {
            context: "profit_margin - revenue",
            value: revenue,
        });
    }
    return Ok(net_income / revenue);
}

/// Computes Asset Turnover.
///
/// # Mathematical Definition
///
/// \[ \text{Asset Turnover} = \frac{\text{Revenue}}{\text{Total Assets}} \]
///
/// # Errors
///
/// Returns [`CalculationError::DivisionByZero`] if `total_assets` is zero.
///
/// # Examples
///
/// ```
/// use casiros_core::financial::asset_turnover;
/// use rust_decimal_macros::dec;
///
/// let at = asset_turnover(dec!(1000.0), dec!(500.0)).unwrap();
/// assert_eq!(at, dec!(2.0));
/// assert!(at > dec!(1.0)); // Assertion 2
/// ```
pub fn asset_turnover(
    revenue: Decimal,
    total_assets: Decimal,
) -> Result<Decimal, CalculationError> {
    if total_assets == Decimal::ZERO {
        return Err(CalculationError::DivisionByZero {
            formula: "Asset Turnover",
        });
    }
    return Ok(revenue / total_assets);
}

/// Computes the Equity Multiplier.
///
/// # Mathematical Definition
///
/// \[ \text{Equity Multiplier} = \frac{\text{Total Assets}}{\text{Shareholders' Equity}} \]
///
/// # Errors
///
/// Returns [`CalculationError::DivisionByZero`] if `shareholders_equity` is zero.
///
/// # Examples
///
/// ```
/// use casiros_core::financial::equity_multiplier;
/// use rust_decimal_macros::dec;
///
/// let em = equity_multiplier(dec!(2000.0), dec!(1000.0)).unwrap();
/// assert_eq!(em, dec!(2.0));
/// assert!(em > dec!(1.0)); // Assertion 2
/// ```
pub fn equity_multiplier(
    total_assets: Decimal,
    shareholders_equity: Decimal,
) -> Result<Decimal, CalculationError> {
    if shareholders_equity == Decimal::ZERO {
        return Err(CalculationError::DivisionByZero {
            formula: "Equity Multiplier",
        });
    }
    return Ok(total_assets / shareholders_equity);
}

/// Computes the Quick Ratio.
///
/// # Mathematical Definition
///
/// \[ \text{Quick Ratio} = \frac{\text{Current Assets} - \text{Inventory}}{\text{Current Liabilities}} \]
///
/// # Errors
///
/// Returns [`CalculationError::DivisionByZero`] if `current_liabilities` is zero.
///
/// # Examples
///
/// ```
/// use casiros_core::financial::quick_ratio;
/// use rust_decimal_macros::dec;
///
/// let qr = quick_ratio(dec!(1000.0), dec!(300.0), dec!(500.0)).unwrap();
/// assert_eq!(qr, dec!(1.4));
/// assert!(qr > dec!(1.0)); // Assertion 2
/// ```
pub fn quick_ratio(
    current_assets: Decimal,
    inventory: Decimal,
    current_liabilities: Decimal,
) -> Result<Decimal, CalculationError> {
    if current_liabilities == Decimal::ZERO {
        return Err(CalculationError::DivisionByZero {
            formula: "Quick Ratio",
        });
    }
    return Ok((current_assets - inventory) / current_liabilities);
}

/// Computes the Interest Coverage Ratio (ICR).
///
/// # Mathematical Definition
///
/// \[ ICR = \frac{\text{EBIT}}{\text{Interest Expense}} \]
///
/// # Errors
///
/// Returns [`CalculationError::DivisionByZero`] if `interest_expense` is zero.
///
/// # Examples
///
/// ```
/// use casiros_core::financial::interest_coverage;
/// use rust_decimal_macros::dec;
///
/// let icr = interest_coverage(dec!(500.0), dec!(100.0)).unwrap();
/// assert_eq!(icr, dec!(5.0));
/// assert!(icr > dec!(1.0)); // Assertion 2
/// ```
pub fn interest_coverage(
    ebit: Decimal,
    interest_expense: Decimal,
) -> Result<Decimal, CalculationError> {
    if interest_expense == Decimal::ZERO {
        return Err(CalculationError::DivisionByZero {
            formula: "Interest Coverage Ratio",
        });
    }
    return Ok(ebit / interest_expense);
}

/// Computes Inventory Turnover.
///
/// # Mathematical Definition
///
/// \[ \text{Inventory Turnover} = \frac{\text{COGS}}{\text{Inventory}} \]
///
/// # Errors
///
/// Returns [`CalculationError::DivisionByZero`] if `inventory` is zero.
///
/// # Examples
///
/// ```
/// use casiros_core::financial::inventory_turnover;
/// use rust_decimal_macros::dec;
///
/// let it = inventory_turnover(dec!(600.0), dec!(100.0)).unwrap();
/// assert_eq!(it, dec!(6.0));
/// assert!(it > dec!(1.0)); // Assertion 2
/// ```
pub fn inventory_turnover(cogs: Decimal, inventory: Decimal) -> Result<Decimal, CalculationError> {
    if inventory == Decimal::ZERO {
        return Err(CalculationError::DivisionByZero {
            formula: "Inventory Turnover",
        });
    }
    return Ok(cogs / inventory);
}

/// Computes the Cash Conversion Cycle (CCC).
///
/// # Mathematical Definition
///
/// \[ CCC = DIO + DSO - DPO \]
///
/// where \( DIO \) is days inventory outstanding, \( DSO \) is days sales
/// outstanding, and \( DPO \) is days payables outstanding.
///
/// # Errors
///
/// Never returns an error in the current implementation, but the `Result`
/// type is reserved for future defensive checks.
///
/// # Examples
///
/// ```
/// use casiros_core::financial::cash_conversion_cycle;
/// use rust_decimal_macros::dec;
///
/// let ccc = cash_conversion_cycle(dec!(30.0), dec!(45.0), dec!(25.0)).unwrap();
/// assert_eq!(ccc, dec!(50.0));
/// assert!(ccc > dec!(0.0)); // Assertion 2
/// ```
pub fn cash_conversion_cycle(
    days_inventory_outstanding: Decimal,
    days_sales_outstanding: Decimal,
    days_payables_outstanding: Decimal,
) -> Result<Decimal, CalculationError> {
    return Ok(days_inventory_outstanding + days_sales_outstanding - days_payables_outstanding);
}

/// Calculates the Altman Z-score for bankruptcy prediction.
///
/// The Z-score combines five financial ratios to predict the probability
/// of bankruptcy within two years. A score above 3.0 indicates safety,
/// between 1.8 and 3.0 is a grey zone, and below 1.8 indicates distress.
///
/// # Mathematical Definition
///
/// \[ Z = 1.2A + 1.4B + 3.3C + 0.6D + 1.0E \]
///
/// where:
/// - A = working capital / total assets
/// - B = retained earnings / total assets
/// - C = EBIT / total assets
/// - D = market value of equity / book value of liabilities
/// - E = sales / total assets
///
/// # Constraints
///
/// All inputs MUST be non-negative.
///
/// # Errors
///
/// Returns [`CalculationError::NegativeValueInvalid`] if any input is negative.
///
/// # Examples
///
/// ```
/// use casiros_core::financial::altman_z_score;
/// use rust_decimal_macros::dec;
///
/// let z = altman_z_score(
///     dec!(0.30), dec!(0.20), dec!(0.25), dec!(2.0), dec!(1.5)
/// ).unwrap();
/// assert!(z > dec!(3.0));
/// ```
pub fn altman_z_score(
    working_capital_to_assets: Decimal,
    retained_earnings_to_assets: Decimal,
    ebit_to_assets: Decimal,
    equity_to_liabilities: Decimal,
    sales_to_assets: Decimal,
) -> Result<Decimal, CalculationError> {
    if working_capital_to_assets < Decimal::ZERO
        || retained_earnings_to_assets < Decimal::ZERO
        || ebit_to_assets < Decimal::ZERO
        || equity_to_liabilities < Decimal::ZERO
        || sales_to_assets < Decimal::ZERO
    {
        return Err(CalculationError::NegativeValueInvalid {
            context: "altman_z_score",
            value: Decimal::ZERO,
        });
    }
    return Ok(
        dec!(1.2) * working_capital_to_assets
            + dec!(1.4) * retained_earnings_to_assets
            + dec!(3.3) * ebit_to_assets
            + dec!(0.6) * equity_to_liabilities
            + sales_to_assets,
    );
}
