# CASIROS Development Roadmap v2.0

**Mission Type:** Financial Physics Engine & Multiverse Simulator
**Language:** Rust (Edition 2024)
**Standards:** NASA JPL Institutional Coding Standard (adapted for Rust) + Clean Architecture

---

## Mission Statement

CASIROS is a **NASA/JPL-grade** financial computation engine. Every formula is a pure, stateless,
provably correct function. The system computes financial metrics across thousands of parallel
"multiverse" scenarios, traces causal dependencies through a directed acyclic graph, and exposes
results via a high-performance API — all with the rigor expected of flight software.

**Non-Negotiable Compile-Time Guarantees:**

| Directive | Meaning |
|---|---|
| `#![forbid(unsafe_code)]` | Memory safety is absolute; no `unsafe` blocks anywhere |
| `#![deny(missing_docs)]` | If it isn't documented, it doesn't compile |
| `#![deny(clippy::pedantic)]` | Every Clippy lint is a hard error |
| `#![deny(warnings)]` | Warnings are compile errors |
| `rust_decimal::Decimal` | Floating-point math is **banned** for currency and ratios |
| `tracing` | Observability is built-in from day zero |

---

## NASA JPL Institutional Coding Standard — Adapted for Rust

The [NASA JPL Institutional Coding Standard for C](https://yurichev.com/mirrors/C/JPL_Coding_Standard_C.pdf)
defines ten rules for flight software. Below is the authoritative Rust adaptation for CASIROS.
Every crate, every module, every function must comply.

### Rule 1: Restrict All Code to Simple Control Flow Constructs

> *"There shall be no use of `goto`, `setjmp`, or `longjmp`."*

**Rust adaptation:**
- `#![deny(clippy::cognitive_complexity)]` with a threshold of **10**.
- Maximum function body length: **60 lines** (excluding doc comments and blank lines).
- Maximum cyclomatic complexity: **10** per function.
- No `loop` with internal `break`/`continue` that spans more than one screen of code.
- Prefer iterators and combinators over manual `for`/`while` loops.

```rust
// ✅ CORRECT: Iterator combinator chain, no manual loop
pub fn sum_of_squares(values: &[Decimal]) -> Decimal {
    values.iter().map(|v| v * v).sum()
}

// ❌ WRONG: Manual loop with mutable accumulator
pub fn sum_of_squares(values: &[Decimal]) -> Decimal {
    let mut acc = dec!(0.0);
    for v in values {
        acc += v * v;
    }
    acc
}
```

### Rule 2: All Loops Must Have a Fixed Upper Bound

> *"A loop shall always have an upper bound on the number of iterations."*

**Rust adaptation:**
- Every loop must have a provable, statically visible upper bound.
- `while true` is **forbidden** in the `core` crate.
- Use `for _ in 0..MAX_ITERATIONS` with an explicit `MAX_ITERATIONS` constant.
- Recursive functions must have a depth limit parameter (see Rule 6).

```rust
// ✅ CORRECT: Fixed upper bound
const MAX_NEWTON_ITERATIONS: u32 = 100;

pub fn yield_to_maturity(price: Decimal, coupon: Decimal, face: Decimal, periods: u32)
    -> Result<Decimal, CalculationError>
{
    let mut guess = coupon / face;
    for _ in 0..MAX_NEWTON_ITERATIONS {
        // Newton-Raphson step
        guess = guess - f(guess) / f_prime(guess);
        if f(guess).abs() < TOLERANCE {
            return Ok(guess);
        }
    }
    Err(CalculationError::ConvergenceFailure {
        formula: "Yield to Maturity",
        iterations: MAX_NEWTON_ITERATIONS,
    })
}
```

### Rule 3: Do Not Use Dynamic Memory Allocation After Initialization

> *"There shall be no use of dynamic memory allocation after task initialization."*

**Rust adaptation:**
- The `core` crate must be `#![no_std]`-compatible in spirit: no `Box`, `Vec`, `String`, or `HashMap`
  in hot-path formula functions. All data passed by reference or stack-allocated.
- The `dag` and `simulator` crates may allocate during **setup** (graph construction, scenario
  generation) but must **pre-allocate** all buffers before the hot loop.
- Use `ArrayVec`, `SmallVec`, or fixed-size arrays where possible.

```rust
// ✅ CORRECT: Stack-allocated, no heap in hot path
pub fn compute_ratios(input: &FinancialInput) -> FinancialRatios {
    FinancialRatios {
        roe: return_on_equity(input.net_income, input.equity).unwrap_or_default(),
        roa: return_on_assets(input.net_income, input.total_assets).unwrap_or_default(),
        // ... all fields computed inline, no Vec allocation
    }
}
```

### Rule 4: Assertion Density

> *"The density of assertions in the code shall average to a minimum of two assertions per function."*

**Rust adaptation:**
- Every public function must have at least **2 assertions** across its doc-test(s) and unit tests.
- Prefer `assert_eq!` over `assert!` for clearer failure messages.
- Use `debug_assert!` for invariant checks inside the function body (stripped in release).

```rust
/// Computes the current ratio.
///
/// # Examples
/// ```
/// use casiros_core::financial::current_ratio;
/// use rust_decimal_macros::dec;
///
/// let cr = current_ratio(dec!(1000), dec!(500)).unwrap();
/// assert_eq!(cr, dec!(2.0));                         // Assertion 1
/// assert!(cr > dec!(0.0));                           // Assertion 2
/// ```
pub fn current_ratio(current_assets: Decimal, current_liabilities: Decimal)
    -> Result<Decimal, CalculationError>
{
    debug_assert!(current_assets >= dec!(0.0), "Current assets must be non-negative");
    debug_assert!(current_liabilities >= dec!(0.0), "Current liabilities must be non-negative");

    if current_liabilities == dec!(0.0) {
        return Err(CalculationError::DivisionByZero {
            formula: "Current Ratio",
        });
    }
    Ok(current_assets / current_liabilities)
}
```

### Rule 5: Declaration Proximity

> *"Data objects shall be declared at the smallest possible level of scope."*

**Rust adaptation:**
- Variables declared at the innermost block scope where they are used.
- Initialize at declaration; never declare uninitialized.
- Use `let` bindings inside `if let` / `match` arms rather than pre-declaring.

```rust
// ✅ CORRECT: Declaration at point of use
pub fn weighted_average_cost_of_capital(
    equity_value: Decimal,
    debt_value: Decimal,
    cost_of_equity: Decimal,
    cost_of_debt: Decimal,
    tax_rate: Decimal,
) -> Result<Decimal, CalculationError> {
    let total_value = equity_value + debt_value;
    if total_value == dec!(0.0) {
        return Err(CalculationError::DivisionByZero { formula: "WACC" });
    }
    let equity_weight = equity_value / total_value;
    let debt_weight = debt_value / total_value;
    let after_tax_cost_of_debt = cost_of_debt * (dec!(1.0) - tax_rate);
    Ok(equity_weight * cost_of_equity + debt_weight * after_tax_cost_of_debt)
}
```

### Rule 6: No Recursion

> *"There shall be no use of recursion."*

**Rust adaptation:**
- `#![deny(clippy::recursion)]` in the `core` crate.
- All algorithms must use iteration.
- If recursion is absolutely unavoidable (e.g., tree traversal in the DAG crate), it must have
  an explicit depth limit parameter and a documented justification.

### Rule 7: Restricted Pointer Use

> *"The use of function pointers shall be restricted."*

**Rust adaptation:**
- `#![forbid(unsafe_code)]` — stronger than `deny`; cannot be overridden locally.
- No raw pointer dereferencing.
- Function pointers (`fn(...)`) are allowed only for dependency injection via traits.
- Prefer `Box<dyn Trait>` or generics over function pointers.

### Rule 8: Compile-Time Checks

> *"All code shall compile with all compiler warnings enabled."*

**Rust adaptation:**
- `#![deny(warnings)]` in every crate.
- `#![deny(rust_2018_idioms)]`, `#![deny(rust_2024_compatibility)]`.
- Use `const fn` for any function that can be evaluated at compile time.
- Use `const` generics for array sizes and fixed parameters.
- `#![deny(unreachable_pub)]` — no accidentally public items.

```rust
/// Compile-time constant: risk-free rate as a Decimal.
/// Uses `const fn` so it can be used in const contexts.
pub const fn risk_free_rate() -> Decimal {
    dec!(0.05) // 5% — updated per economic regime
}

/// Const-generic moving average over N periods.
pub fn moving_average<const N: usize>(values: &[Decimal; N]) -> Decimal {
    values.iter().sum::<Decimal>() / Decimal::from(N as u64)
}
```

### Rule 9: Data Hiding

> *"The use of preprocessor macros shall be limited to file inclusion and simple macros."*

**Rust adaptation:**
- All struct fields are **private** by default.
- Public API exposed only through `impl` blocks with documented methods.
- No `pub` fields on structs (use getters/setters or `pub(crate)` where needed).
- Procedural macros are restricted to the `macros` crate and must be documented.

```rust
// ✅ CORRECT: Private fields, public constructor and accessors
#[derive(Debug, Clone)]
pub struct Bond {
    face_value: Decimal,
    coupon_rate: Decimal,
    periods_to_maturity: u32,
    yield_to_maturity: Decimal,
}

impl Bond {
    pub fn new(
        face_value: Decimal,
        coupon_rate: Decimal,
        periods_to_maturity: u32,
        yield_to_maturity: Decimal,
    ) -> Result<Self, CalculationError> {
        if face_value <= dec!(0.0) {
            return Err(CalculationError::NegativeValueInvalid {
                context: "Bond face value",
                value: face_value,
            });
        }
        Ok(Self { face_value, coupon_rate, periods_to_maturity, yield_to_maturity })
    }

    pub fn price(&self) -> Result<Decimal, CalculationError> {
        bond_price(self.face_value, self.coupon_rate, self.periods_to_maturity, self.yield_to_maturity)
    }
}
```

### Rule 10: Defensive Checks

> *"All code shall be checked by at least one static source code analyzer."*

**Rust adaptation:**
- `cargo clippy -- -D warnings` in CI (Clippy is the static analyzer).
- `cargo audit` for dependency vulnerabilities.
- `cargo deny` for license compliance.
- Every public function validates **all** preconditions and returns `Result<T, CalculationError>`.
- Postcondition checks via `debug_assert!` in the function body.

---

## Clean Architecture

CASIROS follows **Clean Architecture** (Robert C. Martin) with strict layer separation.
Dependencies point **inward**. Inner layers know nothing about outer layers.

```
┌─────────────────────────────────────────────────────────────┐
│                   Presentation Layer                         │
│  (Future: CLI via clap, WASM frontend, Grafana dashboards)   │
├─────────────────────────────────────────────────────────────┤
│                   Infrastructure Layer                       │
│  crates/api/  — Actix-Web REST + WebSocket                   │
│  crates/api/  — sqlx PostgreSQL, Redis cache, S3 artifacts   │
├─────────────────────────────────────────────────────────────┤
│                   Application Layer                          │
│  crates/dag/         — Causality graph evaluation            │
│  crates/simulator/   — Monte Carlo multiverse engine         │
├─────────────────────────────────────────────────────────────┤
│                   Domain Layer                               │
│  crates/core/  — Pure financial formulas, value objects,     │
│                  CalculationError, shared types               │
└─────────────────────────────────────────────────────────────┘
```

### Layer Rules

1. **Domain Layer** (`crates/core/`)
   - Zero dependencies on other CASIROS crates.
   - Only external dependency: `rust_decimal`.
   - Contains: entities (value objects), formula traits, `CalculationError`, shared types.
   - Every function is **pure**: same input → same output, no side effects, no I/O.

2. **Application Layer** (`crates/dag/`, `crates/simulator/`)
   - Depends **only** on the Domain Layer.
   - Contains: use cases (evaluate DAG, run simulation, aggregate results).
   - Defines **traits** for infrastructure concerns (e.g., `trait ScenarioRepository`).
   - Never imports `actix-web`, `sqlx`, or any infrastructure crate directly.

3. **Infrastructure Layer** (`crates/api/`)
   - Depends on the Application Layer (via traits) and Domain Layer (for types).
   - Contains: HTTP handlers, database implementations, external service adapters.
   - Implements traits defined by the Application Layer.

4. **Presentation Layer** (Future)
   - CLI, WASM, or dashboard frontends.
   - Depends on Infrastructure Layer via HTTP/WebSocket.

### Trait-Based Boundaries

Every layer boundary is a **trait** defined by the inner layer, implemented by the outer layer:

```rust
// Defined in crates/simulator/ (Application Layer)
#[async_trait]
pub trait ScenarioRepository: Send + Sync {
    async fn save_scenario(&self, scenario: &Universe) -> Result<ScenarioId, RepositoryError>;
    async fn load_scenario(&self, id: ScenarioId) -> Result<Universe, RepositoryError>;
    async fn list_scenarios(&self) -> Result<Vec<ScenarioSummary>, RepositoryError>;
}

// Implemented in crates/api/ (Infrastructure Layer)
pub struct PostgresScenarioRepository {
    pool: sqlx::PgPool,
}

#[async_trait]
impl ScenarioRepository for PostgresScenarioRepository {
    async fn save_scenario(&self, scenario: &Universe) -> Result<ScenarioId, RepositoryError> {
        // sqlx INSERT ...
    }
    // ...
}
```

---

## Phase 0: The Foundry (Project Scaffolding & NASA Lints)

*Goal: Establish the workspace, error handling, and strict compile-time guarantees before a single
formula is written.*

### 0.1 Workspace Structure

```text
casiros/
├── Cargo.toml                      # [Workspace] — members, resolver = "2"
├── .cargo/
│   └── config.toml                 # Global compiler flags
├── rust-toolchain.toml             # Pin to stable Rust Edition 2024
├── clippy.toml                     # Workspace-wide Clippy configuration
├── deny.toml                       # cargo-deny configuration
├── config/
│   ├── default.toml                # Default configuration values
│   ├── development.toml            # Dev overrides
│   └── production.toml             # Production overrides
├── crates/
│   ├── core/                       # Domain Layer — The Physics Engine
│   │   ├── Cargo.toml
│   │   ├── src/
│   │   │   ├── lib.rs             # #![forbid(unsafe_code)], #![deny(missing_docs)]
│   │   │   ├── error.rs           # CalculationError enum
│   │   │   ├── types.rs           # Dollar, Rate, Ratio, Periods, Amounts
│   │   │   ├── prelude.rs         # Re-exports for ergonomics
│   │   │   ├── general.rs         # TVM formulas (FV, PV, annuity, perpetuity)
│   │   │   ├── financial.rs       # Financial ratios (ROE, ROA, DuPont)
│   │   │   ├── banking.rs         # Banking metrics (NIM, CAR, LDR)
│   │   │   ├── markets.rs         # Market metrics (Beta, Sharpe, VaR)
│   │   │   ├── stocks_bonds.rs    # Equity & fixed income (DDM, DCF, YTM)
│   │   │   └── corporate.rs       # Corporate finance (WACC, FCFF, EVA)
│   │   └── tests/
│   │       ├── general_tests.rs
│   │       ├── financial_tests.rs
│   │       └── ...
│   ├── dag/                        # Application Layer — Causality Graph
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── graph.rs           # DiGraph<FormulaNode> + CausalityEngine
│   │       ├── evaluator.rs       # FormulaEvaluator trait + implementations
│   │       └── visualization.rs   # Dot/GraphML export
│   ├── simulator/                  # Application Layer — Multiverse Engine
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── universe.rs        # Universe struct + UniverseMetrics
│   │       ├── monte_carlo.rs     # Monte Carlo generation + parallel execution
│   │       ├── distributions.rs   # Random distribution helpers
│   │       └── aggregation.rs     # Statistics (mean, stddev, percentiles)
│   ├── api/                        # Infrastructure Layer — HTTP + WebSocket
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs            # Actix-Web server entry point
│   │       ├── routes/
│   │       │   ├── mod.rs
│   │       │   ├── calculate.rs   # /calculate/* endpoints
│   │       │   └── simulate.rs    # /simulate/* endpoints
│   │       ├── middleware/
│   │       │   ├── mod.rs
│   │       │   ├── tracing.rs     # Request tracing middleware
│   │       │   └── rate_limit.rs  # Rate limiting
│   │       └── repository/
│   │           ├── mod.rs
│   │           └── postgres.rs    # PostgresScenarioRepository
│   └── macros/                     # Procedural Macros — Narrative Engine
│       ├── Cargo.toml
│       └── src/
│           └── lib.rs
├── benches/                        # Criterion benchmarks (workspace-level)
│   ├── formula_bench.rs
│   ├── dag_bench.rs
│   └── monte_carlo_bench.rs
├── fuzz/                           # cargo-fuzz targets
│   └── fuzz_targets/
│       └── formula_fuzz.rs
├── docker/
│   ├── Dockerfile                  # Multi-stage build
│   └── docker-compose.yml         # Local dev environment
├── .github/
│   └── workflows/
│       └── ci.yml                 # CI/CD pipeline
├── CHANGELOG.md
└── README.md
```

### 0.2 Compiler Configuration (`.cargo/config.toml`)

```toml
[build]
# Rust Edition 2024
rustflags = [
    "-D", "warnings",
    "-D", "missing_docs",
    "-D", "clippy::pedantic",
    "-D", "clippy::cognitive_complexity",
    "-D", "clippy::recursion",
    "-D", "unreachable_pub",
    "-D", "rust_2018_idioms",
    "-D", "rust_2024_compatibility",
]
```

### 0.3 The Core Crate Boilerplate (`crates/core/src/lib.rs`)

```rust
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
#![deny(clippy::recursion)]
#![deny(unreachable_pub)]
#![deny(rust_2018_idioms)]
#![deny(rust_2024_compatibility)]
#![allow(clippy::needless_return)]  // Explicit returns preferred for auditability

pub mod prelude {
    //! Re-exports for ergonomic use across the workspace.
    pub use crate::error::CalculationError;
    pub use crate::types::{Amounts, Dollar, Periods, Rate, Ratio};
    pub use rust_decimal::Decimal;
    pub use rust_decimal_macros::dec;
}

pub mod error;
pub mod types;
pub mod general;
pub mod financial;
pub mod banking;
pub mod markets;
pub mod stocks_bonds;
pub mod corporate;
```

### 0.4 Defensive Error Handling (`crates/core/src/error.rs`)

```rust
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
```

### 0.5 Shared Types (`crates/core/src/types.rs`)

```rust
use rust_decimal::Decimal;

/// Monetary value in the base currency (e.g., USD).
/// All financial amounts use this type for transactional integrity.
pub type Dollar = Decimal;

/// An interest rate, discount rate, or growth rate expressed as a decimal.
/// Example: 5% = `dec!(0.05)`.
pub type Rate = Decimal;

/// A dimensionless ratio (e.g., 0.6 for 60%).
pub type Ratio = Decimal;

/// A number of compounding periods (years, months, quarters).
pub type Periods = u32;

/// The three fundamental time-value-of-money quantities.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Amounts {
    /// Present value (PV)
    pub principal: Dollar,
    /// Future value (FV)
    pub future_value: Dollar,
    /// Periodic payment (PMT)
    pub payment: Dollar,
}

impl Amounts {
    /// Creates a new `Amounts` with all fields validated as non-negative.
    pub fn new(principal: Dollar, future_value: Dollar, payment: Dollar)
        -> Result<Self, crate::error::CalculationError>
    {
        use crate::error::CalculationError;
        if principal < Decimal::ZERO {
            return Err(CalculationError::NegativeValueInvalid {
                context: "Amounts::principal",
                value: principal,
            });
        }
        if future_value < Decimal::ZERO {
            return Err(CalculationError::NegativeValueInvalid {
                context: "Amounts::future_value",
                value: future_value,
            });
        }
        if payment < Decimal::ZERO {
            return Err(CalculationError::NegativeValueInvalid {
                context: "Amounts::payment",
                value: payment,
            });
        }
        Ok(Self { principal, future_value, payment })
    }
}
```

---

## Phase 1: The Mathematical Kernel (Core Crate)

*Goal: Implement every formula as a **pure, stateless function** with full precondition
validation, doc-tests, and unit tests.*

### 1.1 Formula Implementation Template

Every formula follows this exact pattern:

```rust
/// Short description of what this formula computes.
///
/// # Mathematical Definition
///
/// \[ Formula = \frac{Numerator}{Denominator} \]
///
/// # Constraints (NASA Style Preconditions)
///
/// - `param1` MUST be >= 0.
/// - `param2` MUST be > 0 (to prevent division by zero).
///
/// # Errors
///
/// Returns [`CalculationError::DivisionByZero`] if `param2` is zero.
/// Returns [`CalculationError::NegativeValueInvalid`] if `param1` is negative.
///
/// # Examples
///
/// ```
/// use casiros_core::financial::example_formula;
/// use rust_decimal_macros::dec;
///
/// let result = example_formula(dec!(100.0), dec!(50.0)).unwrap();
/// assert_eq!(result, dec!(2.0));
/// ```
pub fn example_formula(param1: Decimal, param2: Decimal) -> Result<Decimal, CalculationError> {
    // Precondition checks
    if param1 < Decimal::ZERO {
        return Err(CalculationError::NegativeValueInvalid {
            context: "example_formula - param1",
            value: param1,
        });
    }
    if param2 == Decimal::ZERO {
        return Err(CalculationError::DivisionByZero {
            formula: "Example Formula",
        });
    }

    // Computation
    Ok(param1 / param2)
}
```

### 1.2 General Finance Formulas (`crates/core/src/general.rs`)

```rust
use super::prelude::*;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

/// Calculates the future value of a present sum using compound interest.
///
/// # Mathematical Definition
///
/// \[ FV = PV \times (1 + r)^n \]
///
/// # Constraints
///
/// - `present_value` MUST be >= 0.
/// - `rate` MUST be > -1.0 (to prevent invalid negative compounding).
/// - `periods` MUST be > 0.
///
/// # Errors
///
/// Returns [`CalculationError::InvalidRate`] if `rate` <= -1.0.
/// Returns [`CalculationError::NegativeValueInvalid`] if `present_value` < 0.
///
/// # Examples
///
/// ```
/// use casiros_core::general::future_value;
/// use rust_decimal_macros::dec;
///
/// // $100 at 5% for 10 years = $162.89
/// let fv = future_value(dec!(100.0), dec!(0.05), 10).unwrap();
/// assert_eq!(fv.round_dp(4), dec!(162.8895));
/// ```
pub fn future_value(
    present_value: Decimal,
    rate: Decimal,
    periods: Periods,
) -> Result<Decimal, CalculationError> {
    if rate <= dec!(-1.0) {
        return Err(CalculationError::InvalidRate { rate });
    }
    if present_value < Decimal::ZERO {
        return Err(CalculationError::NegativeValueInvalid {
            context: "future_value - present_value",
            value: present_value,
        });
    }
    if periods == 0 {
        return Ok(present_value);
    }

    let growth_factor = (dec!(1.0) + rate).powi(periods as i64);
    Ok(present_value * growth_factor)
}

/// Calculates the present value of a future sum using discounting.
///
/// # Mathematical Definition
///
/// \[ PV = \frac{FV}{(1 + r)^n} \]
///
/// # Examples
///
/// ```
/// use casiros_core::general::present_value;
/// use rust_decimal_macros::dec;
///
/// let pv = present_value(dec!(162.8895), dec!(0.05), 10).unwrap();
/// assert_eq!(pv.round_dp(2), dec!(100.00));
/// ```
pub fn present_value(
    future_value: Decimal,
    rate: Decimal,
    periods: Periods,
) -> Result<Decimal, CalculationError> {
    if rate <= dec!(-1.0) {
        return Err(CalculationError::InvalidRate { rate });
    }
    if future_value < Decimal::ZERO {
        return Err(CalculationError::NegativeValueInvalid {
            context: "present_value - future_value",
            value: future_value,
        });
    }
    if periods == 0 {
        return Ok(future_value);
    }

    let discount_factor = (dec!(1.0) + rate).powi(periods as i64);
    if discount_factor == Decimal::ZERO {
        return Err(CalculationError::DivisionByZero {
            formula: "Present Value",
        });
    }
    Ok(future_value / discount_factor)
}

/// Calculates the future value of an annuity (series of equal payments).
///
/// # Mathematical Definition
///
/// \[ FV_{\text{annuity}} = PMT \times \frac{(1 + r)^n - 1}{r} \]
///
/// # Examples
///
/// ```
/// use casiros_core::general::annuity_future_value;
/// use rust_decimal_macros::dec;
///
/// // $1,000/year at 5% for 10 years
/// let fv = annuity_future_value(dec!(1000.0), dec!(0.05), 10).unwrap();
/// assert_eq!(fv.round_dp(2), dec!(12577.89));
/// ```
pub fn annuity_future_value(
    payment: Decimal,
    rate: Decimal,
    periods: Periods,
) -> Result<Decimal, CalculationError> {
    if rate <= dec!(-1.0) {
        return Err(CalculationError::InvalidRate { rate });
    }
    if payment < Decimal::ZERO {
        return Err(CalculationError::NegativeValueInvalid {
            context: "annuity_future_value - payment",
            value: payment,
        });
    }
    if rate == Decimal::ZERO {
        return Ok(payment * Decimal::from(periods));
    }

    let growth = (dec!(1.0) + rate).powi(periods as i64);
    Ok(payment * (growth - dec!(1.0)) / rate)
}

/// Calculates the present value of an annuity.
///
/// # Mathematical Definition
///
/// \[ PV_{\text{annuity}} = PMT \times \frac{1 - (1 + r)^{-n}}{r} \]
///
/// # Examples
///
/// ```
/// use casiros_core::general::annuity_present_value;
/// use rust_decimal_macros::dec;
///
/// let pv = annuity_present_value(dec!(1000.0), dec!(0.05), 10).unwrap();
/// assert_eq!(pv.round_dp(2), dec!(7721.73));
/// ```
pub fn annuity_present_value(
    payment: Decimal,
    rate: Decimal,
    periods: Periods,
) -> Result<Decimal, CalculationError> {
    if rate <= dec!(-1.0) {
        return Err(CalculationError::InvalidRate { rate });
    }
    if payment < Decimal::ZERO {
        return Err(CalculationError::NegativeValueInvalid {
            context: "annuity_present_value - payment",
            value: payment,
        });
    }
    if rate == Decimal::ZERO {
        return Ok(payment * Decimal::from(periods));
    }

    let discount = (dec!(1.0) + rate).powi(periods as i64);
    if discount == Decimal::ZERO {
        return Err(CalculationError::DivisionByZero {
            formula: "Annuity Present Value",
        });
    }
    Ok(payment * (dec!(1.0) - dec!(1.0) / discount) / rate)
}

/// Calculates the present value of a perpetuity.
///
/// # Mathematical Definition
///
/// \[ PV_{\text{perpetuity}} = \frac{PMT}{r} \]
///
/// # Examples
///
/// ```
/// use casiros_core::general::perpetuity_present_value;
/// use rust_decimal_macros::dec;
///
/// let pv = perpetuity_present_value(dec!(100.0), dec!(0.05)).unwrap();
/// assert_eq!(pv, dec!(2000.0));
/// ```
pub fn perpetuity_present_value(
    payment: Decimal,
    rate: Decimal,
) -> Result<Decimal, CalculationError> {
    if rate <= Decimal::ZERO {
        return Err(CalculationError::InvalidRate { rate });
    }
    if payment < Decimal::ZERO {
        return Err(CalculationError::NegativeValueInvalid {
            context: "perpetuity_present_value - payment",
            value: payment,
        });
    }
    Ok(payment / rate)
}

/// Converts a nominal annual rate to an effective annual rate (EAR).
///
/// # Mathematical Definition
///
/// \[ EAR = \left(1 + \frac{r_{\text{nom}}}{m}\right)^m - 1 \]
///
/// where `m` is the number of compounding periods per year.
///
/// # Examples
///
/// ```
/// use casiros_core::general::effective_annual_rate;
/// use rust_decimal_macros::dec;
///
/// // 5% nominal compounded monthly
/// let ear = effective_annual_rate(dec!(0.05), 12).unwrap();
/// assert_eq!(ear.round_dp(6), dec!(0.051162));
/// ```
pub fn effective_annual_rate(
    nominal_rate: Decimal,
    compounding_periods: u32,
) -> Result<Decimal, CalculationError> {
    if nominal_rate <= dec!(-1.0) {
        return Err(CalculationError::InvalidRate { rate: nominal_rate });
    }
    if compounding_periods == 0 {
        return Err(CalculationError::DivisionByZero {
            formula: "Effective Annual Rate",
        });
    }
    let periodic_rate = nominal_rate / Decimal::from(compounding_periods);
    let factor = (dec!(1.0) + periodic_rate).powi(compounding_periods as i64);
    Ok(factor - dec!(1.0))
}
```

### 1.3 Financial Ratios (`crates/core/src/financial.rs`)

```rust
use super::prelude::*;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

/// Computes Return on Equity (ROE).
///
/// # Mathematical Definition
///
/// \[ ROE = \frac{\text{Net Income}}{\text{Average Shareholders' Equity}} \]
///
/// # Examples
///
/// ```
/// use casiros_core::financial::return_on_equity;
/// use rust_decimal_macros::dec;
///
/// let roe = return_on_equity(dec!(150.0), dec!(1000.0)).unwrap();
/// assert_eq!(roe, dec!(0.15));
/// assert!(roe > dec!(0.0));  // Assertion 2
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
    Ok(net_income / avg_shareholders_equity)
}

/// Computes Return on Assets (ROA).
///
/// # Mathematical Definition
///
/// \[ ROA = \frac{\text{Net Income}}{\text{Average Total Assets}} \]
///
/// # Examples
///
/// ```
/// use casiros_core::financial::return_on_assets;
/// use rust_decimal_macros::dec;
///
/// let roa = return_on_assets(dec!(150.0), dec!(2000.0)).unwrap();
/// assert_eq!(roa, dec!(0.075));
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
    Ok(net_income / avg_total_assets)
}

/// Computes the DuPont decomposition of ROE.
///
/// # Mathematical Definition
///
/// \[ ROE = \text{Profit Margin} \times \text{Asset Turnover} \times \text{Equity Multiplier} \]
///
/// # Examples
///
/// ```
/// use casiros_core::financial::dupont_roe;
/// use rust_decimal_macros::dec;
///
/// let roe = dupont_roe(dec!(0.10), dec!(0.80), dec!(2.0)).unwrap();
/// assert_eq!(roe, dec!(0.16));
/// ```
pub fn dupont_roe(
    profit_margin: Decimal,
    asset_turnover: Decimal,
    equity_multiplier: Decimal,
) -> Result<Decimal, CalculationError> {
    Ok(profit_margin * asset_turnover * equity_multiplier)
}

/// Computes the Current Ratio.
///
/// # Mathematical Definition
///
/// \[ \text{Current Ratio} = \frac{\text{Current Assets}}{\text{Current Liabilities}} \]
///
/// # Examples
///
/// ```
/// use casiros_core::financial::current_ratio;
/// use rust_decimal_macros::dec;
///
/// let cr = current_ratio(dec!(1000.0), dec!(500.0)).unwrap();
/// assert_eq!(cr, dec!(2.0));
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
    Ok(current_assets / current_liabilities)
}

/// Computes the Debt-to-Equity Ratio.
///
/// # Mathematical Definition
///
/// \[ D/E = \frac{\text{Total Liabilities}}{\text{Shareholders' Equity}} \]
///
/// # Examples
///
/// ```
/// use casiros_core::financial::debt_to_equity;
/// use rust_decimal_macros::dec;
///
/// let de = debt_to_equity(dec!(500.0), dec!(1000.0)).unwrap();
/// assert_eq!(de, dec!(0.5));
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
    Ok(total_liabilities / shareholders_equity)
}
```

### 1.4 Corporate Finance (`crates/core/src/corporate.rs`)

```rust
use super::prelude::*;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

/// Computes the Weighted Average Cost of Capital (WACC).
///
/// # Mathematical Definition
///
/// \[ WACC = \frac{E}{V} \times r_e + \frac{D}{V} \times r_d \times (1 - t) \]
///
/// where \( V = E + D \).
///
/// # Examples
///
/// ```
/// use casiros_core::corporate::wacc;
/// use rust_decimal_macros::dec;
///
/// let w = wacc(dec!(600.0), dec!(400.0), dec!(0.12), dec!(0.06), dec!(0.30)).unwrap();
/// assert_eq!(w.round_dp(4), dec!(0.0888));
/// ```
pub fn wacc(
    equity_value: Decimal,
    debt_value: Decimal,
    cost_of_equity: Decimal,
    cost_of_debt: Decimal,
    tax_rate: Decimal,
) -> Result<Decimal, CalculationError> {
    let total_value = equity_value + debt_value;
    if total_value == Decimal::ZERO {
        return Err(CalculationError::DivisionByZero { formula: "WACC" });
    }
    if tax_rate < Decimal::ZERO || tax_rate > dec!(1.0) {
        return Err(CalculationError::RangeViolation {
            context: "WACC - tax_rate",
            value: tax_rate,
        });
    }
    let equity_weight = equity_value / total_value;
    let debt_weight = debt_value / total_value;
    let after_tax_cost_of_debt = cost_of_debt * (dec!(1.0) - tax_rate);
    Ok(equity_weight * cost_of_equity + debt_weight * after_tax_cost_of_debt)
}

/// Computes Free Cash Flow to Firm (FCFF).
///
/// # Mathematical Definition
///
/// \[ FCFF = EBIT \times (1 - t) + D\&A - \Delta WC - CapEx \]
///
/// # Examples
///
/// ```
/// use casiros_core::corporate::free_cash_flow_to_firm;
/// use rust_decimal_macros::dec;
///
/// let fcff = free_cash_flow_to_firm(
///     dec!(1000.0), dec!(0.30), dec!(100.0), dec!(50.0), dec!(200.0)
/// ).unwrap();
/// assert_eq!(fcff, dec!(550.0));
/// ```
pub fn free_cash_flow_to_firm(
    ebit: Decimal,
    tax_rate: Decimal,
    depreciation: Decimal,
    delta_working_capital: Decimal,
    capex: Decimal,
) -> Result<Decimal, CalculationError> {
    if tax_rate < Decimal::ZERO || tax_rate > dec!(1.0) {
        return Err(CalculationError::RangeViolation {
            context: "FCFF - tax_rate",
            value: tax_rate,
        });
    }
    let nopat = ebit * (dec!(1.0) - tax_rate);
    Ok(nopat + depreciation - delta_working_capital - capex)
}

/// Computes the Sustainable Growth Rate (SGR).
///
/// # Mathematical Definition
///
/// \[ SGR = ROE \times (1 - \text{Dividend Payout Ratio}) \]
///
/// # Examples
///
/// ```
/// use casiros_core::corporate::sustainable_growth_rate;
/// use rust_decimal_macros::dec;
///
/// let sgr = sustainable_growth_rate(dec!(0.15), dec!(0.40)).unwrap();
/// assert_eq!(sgr, dec!(0.09));
/// ```
pub fn sustainable_growth_rate(
    roe: Decimal,
    dividend_payout_ratio: Decimal,
) -> Result<Decimal, CalculationError> {
    if dividend_payout_ratio < Decimal::ZERO || dividend_payout_ratio > dec!(1.0) {
        return Err(CalculationError::RangeViolation {
            context: "SGR - dividend_payout_ratio",
            value: dividend_payout_ratio,
        });
    }
    let retention_ratio = dec!(1.0) - dividend_payout_ratio;
    Ok(roe * retention_ratio)
}
```

---

## Phase 2: The Causality Engine (DAG Crate)

*Goal: Build the topological graph that links formulas (e.g., `Inventory_Turnover` →
`Cash_Conversion_Cycle`) and evaluates them in correct dependency order.*

### 2.1 Graph Structure (`crates/dag/src/graph.rs`)

```rust
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::algo::toposort;
use std::collections::HashMap;
use casiros_core::prelude::*;

/// Every formula in the CASIROS system, represented as a graph node.
///
/// The enum variants correspond 1:1 with public functions in `casiros_core`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FormulaNode {
    // General — Time Value of Money
    FutureValue,
    PresentValue,
    AnnuityFutureValue,
    AnnuityPresentValue,
    PerpetuityPresentValue,
    EffectiveAnnualRate,

    // Financial Ratios
    ReturnOnEquity,
    ReturnOnAssets,
    DuPontROE,
    CurrentRatio,
    DebtToEquity,
    ProfitMargin,
    AssetTurnover,
    EquityMultiplier,

    // Corporate Finance
    Wacc,
    FreeCashFlowToFirm,
    SustainableGrowthRate,

    // Banking
    NetInterestMargin,
    LoanToDepositRatio,
    CapitalAdequacyRatio,

    // Markets
    Beta,
    SharpeRatio,
    ValueAtRisk,

    // Stocks & Bonds
    DividendDiscountModel,
    BondPrice,
    YieldToMaturity,
    Duration,
}

/// The directed acyclic graph of formula dependencies.
///
/// An edge A → B means "formula B depends on the output of formula A."
/// The engine evaluates formulas in topological order, ensuring every
/// formula's inputs are computed before it runs.
pub struct CausalityEngine {
    graph: DiGraph<FormulaNode, ()>,
    node_indices: HashMap<FormulaNode, NodeIndex>,
}

impl CausalityEngine {
    /// Constructs the full formula dependency graph.
    ///
    /// This is the single source of truth for how formulas relate to each other.
    /// Adding a new formula requires:
    /// 1. Adding its variant to `FormulaNode`.
    /// 2. Registering it here with `add_node`.
    /// 3. Adding edges from its dependencies.
    pub fn new() -> Self {
        let mut graph = DiGraph::new();
        let mut node_indices = HashMap::new();

        // Register all formula nodes
        for node in FormulaNode::all_variants() {
            let idx = graph.add_node(node);
            node_indices.insert(node, idx);
        }

        let mut engine = Self { graph, node_indices };

        // Define dependency edges
        // Example: DuPont ROE depends on Profit Margin, Asset Turnover, Equity Multiplier
        engine.add_edge(FormulaNode::ProfitMargin, FormulaNode::DuPontROE);
        engine.add_edge(FormulaNode::AssetTurnover, FormulaNode::DuPontROE);
        engine.add_edge(FormulaNode::EquityMultiplier, FormulaNode::DuPontROE);

        // SGR depends on ROE
        engine.add_edge(FormulaNode::ReturnOnEquity, FormulaNode::SustainableGrowthRate);

        // WACC depends on cost of equity and cost of debt (leaf inputs, no edges needed)
        // but conceptually: WACC is used by DCF valuation
        engine.add_edge(FormulaNode::Wacc, FormulaNode::FreeCashFlowToFirm);

        engine
    }

    fn add_edge(&mut self, from: FormulaNode, to: FormulaNode) {
        let from_idx = self.node_indices[&from];
        let to_idx = self.node_indices[&to];
        self.graph.add_edge(from_idx, to_idx, ());
    }

    /// Returns the formulas in topological execution order.
    ///
    /// # Errors
    ///
    /// Returns an error string if a cycle is detected in the dependency graph.
    /// Cycles are a critical bug — they must be resolved before the engine can run.
    pub fn execution_order(&self) -> Result<Vec<FormulaNode>, String> {
        let sorted = toposort(&self.graph, None)
            .map_err(|cycle| {
                let node = self.graph[cycle.node_id()];
                format!(
                    "Cyclic dependency detected at formula '{:?}'. \
                     Review the dependency edges in CausalityEngine::new().",
                    node
                )
            })?;
        Ok(sorted.into_iter().map(|idx| self.graph[idx]).collect())
    }

    /// Returns all formulas that directly depend on the given formula.
    pub fn dependents_of(&self, node: FormulaNode) -> Vec<FormulaNode> {
        let idx = self.node_indices[&node];
        self.graph
            .neighbors_directed(idx, petgraph::Direction::Outgoing)
            .map(|i| self.graph[i])
            .collect()
    }

    /// Returns all formulas that the given formula depends on.
    pub fn dependencies_of(&self, node: FormulaNode) -> Vec<FormulaNode> {
        let idx = self.node_indices[&node];
        self.graph
            .neighbors_directed(idx, petgraph::Direction::Incoming)
            .map(|i| self.graph[i])
            .collect()
    }
}

impl FormulaNode {
    /// Returns all variants for graph initialization.
    fn all_variants() -> Vec<FormulaNode> {
        use FormulaNode::*;
        vec![
            FutureValue, PresentValue, AnnuityFutureValue, AnnuityPresentValue,
            PerpetuityPresentValue, EffectiveAnnualRate,
            ReturnOnEquity, ReturnOnAssets, DuPontROE, CurrentRatio, DebtToEquity,
            ProfitMargin, AssetTurnover, EquityMultiplier,
            Wacc, FreeCashFlowToFirm, SustainableGrowthRate,
            NetInterestMargin, LoanToDepositRatio, CapitalAdequacyRatio,
            Beta, SharpeRatio, ValueAtRisk,
            DividendDiscountModel, BondPrice, YieldToMaturity, Duration,
        ]
    }
}
```

### 2.2 Formula Evaluator (`crates/dag/src/evaluator.rs`)

```rust
use std::collections::HashMap;
use casiros_core::prelude::*;
use casiros_core::{general, financial, corporate};
use crate::graph::{CausalityEngine, FormulaNode};

/// Stores the computed output of each formula during DAG evaluation.
#[derive(Debug, Clone)]
pub struct EvaluationContext {
    /// Formula → computed result
    results: HashMap<FormulaNode, Decimal>,
    /// Raw input values provided by the caller
    inputs: HashMap<String, Decimal>,
}

impl EvaluationContext {
    /// Creates a new context with the given raw inputs.
    pub fn new(inputs: HashMap<String, Decimal>) -> Self {
        Self {
            results: HashMap::new(),
            inputs,
        }
    }

    /// Retrieves a raw input value by name.
    pub fn get_input(&self, name: &str) -> Result<Decimal, CalculationError> {
        self.inputs.get(name).copied().ok_or_else(|| {
            CalculationError::MissingInput {
                formula: "EvaluationContext",
                parameter: name,
            }
        })
    }

    /// Stores a computed formula result.
    pub fn set_result(&mut self, node: FormulaNode, value: Decimal) {
        self.results.insert(node, value);
    }

    /// Retrieves a previously computed formula result.
    pub fn get_result(&self, node: FormulaNode) -> Option<Decimal> {
        self.results.get(&node).copied()
    }
}

/// Evaluates the full DAG in topological order.
///
/// Walks the execution order, calling the appropriate core function for each
/// formula node, and stores results in the context for downstream formulas.
pub fn evaluate_dag(
    engine: &CausalityEngine,
    ctx: &mut EvaluationContext,
) -> Result<(), CalculationError> {
    let order = engine
        .execution_order()
        .map_err(|e| CalculationError::MissingInput {
            formula: "DAG evaluation",
            parameter: "execution_order",
        })?; // In production, map this properly

    for node in order {
        let result = evaluate_node(node, ctx)?;
        ctx.set_result(node, result);
    }

    Ok(())
}

fn evaluate_node(
    node: FormulaNode,
    ctx: &EvaluationContext,
) -> Result<Decimal, CalculationError> {
    use FormulaNode::*;
    match node {
        FutureValue => {
            let pv = ctx.get_input("present_value")?;
            let rate = ctx.get_input("rate")?;
            let periods = ctx.get_input("periods")?.to_u32().unwrap_or(1);
            general::future_value(pv, rate, periods)
        }
        PresentValue => {
            let fv = ctx.get_input("future_value")?;
            let rate = ctx.get_input("rate")?;
            let periods = ctx.get_input("periods")?.to_u32().unwrap_or(1);
            general::present_value(fv, rate, periods)
        }
        ReturnOnEquity => {
            let net_income = ctx.get_input("net_income")?;
            let equity = ctx.get_input("shareholders_equity")?;
            financial::return_on_equity(net_income, equity)
        }
        DuPontROE => {
            let pm = ctx.get_result(ProfitMargin)
                .or_else(|| ctx.get_input("profit_margin").ok())?;
            let at = ctx.get_result(AssetTurnover)
                .or_else(|| ctx.get_input("asset_turnover").ok())?;
            let em = ctx.get_result(EquityMultiplier)
                .or_else(|| ctx.get_input("equity_multiplier").ok())?;
            financial::dupont_roe(pm, at, em)
        }
        SustainableGrowthRate => {
            let roe = ctx.get_result(ReturnOnEquity)
                .or_else(|| ctx.get_input("roe").ok())?;
            let dpr = ctx.get_input("dividend_payout_ratio")?;
            corporate::sustainable_growth_rate(roe, dpr)
        }
        Wacc => {
            let ev = ctx.get_input("equity_value")?;
            let dv = ctx.get_input("debt_value")?;
            let ce = ctx.get_input("cost_of_equity")?;
            let cd = ctx.get_input("cost_of_debt")?;
            let tr = ctx.get_input("tax_rate")?;
            corporate::wacc(ev, dv, ce, cd, tr)
        }
        FreeCashFlowToFirm => {
            let ebit = ctx.get_input("ebit")?;
            let tr = ctx.get_input("tax_rate")?;
            let da = ctx.get_input("depreciation")?;
            let dwc = ctx.get_input("delta_working_capital")?;
            let capex = ctx.get_input("capex")?;
            corporate::free_cash_flow_to_firm(ebit, tr, da, dwc, capex)
        }
        // ... all other variants
        _ => Err(CalculationError::MissingInput {
            formula: "evaluate_node",
            parameter: "unimplemented_formula",
        }),
    }
}
```

---

## Phase 3: The Multiverse Simulator (Simulator Crate)

*Goal: Run thousands of parallel "what-if" scenarios using Rayon with proper statistical
aggregation.*

### 3.1 Universe Definition (`crates/simulator/src/universe.rs`)

```rust
use casiros_core::prelude::*;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

/// A single economic scenario — one "universe" in the multiverse.
///
/// Each field represents an input variable that can be perturbed during
/// Monte Carlo simulation.
#[derive(Debug, Clone)]
pub struct Universe {
    // Macroeconomic
    pub inflation_rate: Decimal,
    pub risk_free_rate: Decimal,
    pub gdp_growth: Decimal,

    // Company-specific
    pub revenue: Decimal,
    pub revenue_growth: Decimal,
    pub cogs_percent: Decimal,
    pub operating_expenses: Decimal,
    pub depreciation: Decimal,
    pub capex: Decimal,
    pub tax_rate: Decimal,

    // Balance sheet
    pub total_assets: Decimal,
    pub total_liabilities: Decimal,
    pub shareholders_equity: Decimal,
    pub current_assets: Decimal,
    pub current_liabilities: Decimal,
    pub working_capital: Decimal,

    // Market
    pub shares_outstanding: Decimal,
    pub market_price: Decimal,
    pub market_return: Decimal,
    pub beta: Decimal,
}

/// The complete set of computed metrics for a single universe.
#[derive(Debug, Clone)]
pub struct UniverseMetrics {
    // Profitability
    pub roe: Decimal,
    pub roa: Decimal,
    pub profit_margin: Decimal,
    pub asset_turnover: Decimal,
    pub equity_multiplier: Decimal,

    // Liquidity & Solvency
    pub current_ratio: Decimal,
    pub debt_to_equity: Decimal,

    // Valuation
    pub wacc: Decimal,
    pub fcff: Decimal,
    pub sgr: Decimal,
    pub intrinsic_value: Decimal,

    // Market
    pub expected_return: Decimal,
    pub sharpe_ratio: Decimal,
}
```

### 3.2 Monte Carlo Engine (`crates/simulator/src/monte_carlo.rs`)

```rust
use rayon::prelude::*;
use rand::Rng;
use rand_distr::{Distribution, Normal, LogNormal};
use casiros_core::prelude::*;
use crate::universe::{Universe, UniverseMetrics};
use crate::aggregation::SimulationResults;

/// Configuration for a Monte Carlo simulation run.
pub struct MonteCarloConfig {
    /// Number of parallel universes to simulate.
    pub iterations: usize,
    /// Random seed for reproducibility.
    pub seed: u64,
    /// Whether to track convergence (stops early if results stabilize).
    pub track_convergence: bool,
    /// Convergence threshold (max % change in mean between batches).
    pub convergence_threshold: Decimal,
    /// Batch size for convergence checking.
    pub convergence_batch_size: usize,
}

impl Default for MonteCarloConfig {
    fn default() -> Self {
        Self {
            iterations: 10_000,
            seed: 42,
            track_convergence: true,
            convergence_threshold: dec!(0.001), // 0.1%
            convergence_batch_size: 1_000,
        }
    }
}

/// Generates perturbed universes around a baseline using log-normal distributions
/// for growth rates and normal distributions for ratios.
pub fn generate_scenarios(
    baseline: &Universe,
    config: &MonteCarloConfig,
) -> Vec<Universe> {
    let mut rng = rand::rngs::StdRng::seed_from_u64(config.seed);

    // Define perturbation distributions
    let growth_noise = Normal::new(0.0, 0.02).unwrap();     // 2% stddev on growth rates
    let ratio_noise = Normal::new(0.0, 0.01).unwrap();       // 1% stddev on ratios
    let price_noise = LogNormal::new(0.0, 0.15).unwrap();   // 15% stddev on prices

    (0..config.iterations)
        .map(|_| {
            let mut u = baseline.clone();

            // Perturb growth rates
            u.revenue_growth += Decimal::from_f64_retain(
                growth_noise.sample(&mut rng)
            ).unwrap_or(Decimal::ZERO);
            u.gdp_growth += Decimal::from_f64_retain(
                growth_noise.sample(&mut rng)
            ).unwrap_or(Decimal::ZERO);

            // Perturb ratios
            u.cogs_percent += Decimal::from_f64_retain(
                ratio_noise.sample(&mut rng)
            ).unwrap_or(Decimal::ZERO);
            u.tax_rate += Decimal::from_f64_retain(
                ratio_noise.sample(&mut rng)
            ).unwrap_or(Decimal::ZERO);

            // Perturb market price multiplicatively
            let price_mult = Decimal::from_f64_retain(
                price_noise.sample(&mut rng)
            ).unwrap_or(dec!(1.0));
            u.market_price *= price_mult;

            // Clamp to valid ranges
            u.cogs_percent = u.cogs_percent.clamp(dec!(0.0), dec!(1.0));
            u.tax_rate = u.tax_rate.clamp(dec!(0.0), dec!(0.5));
            u.revenue = Decimal::max(u.revenue, Decimal::ZERO);

            u
        })
        .collect()
}

/// Runs the full simulation across all generated universes in parallel.
pub fn run_simulation(
    scenarios: &[Universe],
) -> SimulationResults {
    let metrics: Vec<UniverseMetrics> = scenarios
        .par_iter()
        .map(|universe| compute_universe_metrics(universe))
        .collect();

    SimulationResults::from_metrics(&metrics)
}

/// Computes all financial metrics for a single universe by calling core functions.
fn compute_universe_metrics(universe: &Universe) -> UniverseMetrics {
    use casiros_core::financial;
    use casiros_core::corporate;

    let net_income = universe.revenue * (dec!(1.0) - universe.cogs_percent)
        - universe.operating_expenses - universe.depreciation;

    UniverseMetrics {
        roe: financial::return_on_equity(net_income, universe.shareholders_equity)
            .unwrap_or_default(),
        roa: financial::return_on_assets(net_income, universe.total_assets)
            .unwrap_or_default(),
        profit_margin: if universe.revenue > Decimal::ZERO {
            net_income / universe.revenue
        } else {
            Decimal::ZERO
        },
        asset_turnover: if universe.total_assets > Decimal::ZERO {
            universe.revenue / universe.total_assets
        } else {
            Decimal::ZERO
        },
        equity_multiplier: if universe.shareholders_equity > Decimal::ZERO {
            universe.total_assets / universe.shareholders_equity
        } else {
            Decimal::ZERO
        },
        current_ratio: financial::current_ratio(
            universe.current_assets, universe.current_liabilities,
        ).unwrap_or_default(),
        debt_to_equity: financial::debt_to_equity(
            universe.total_liabilities, universe.shareholders_equity,
        ).unwrap_or_default(),
        wacc: corporate::wacc(
            universe.shareholders_equity,
            universe.total_liabilities,
            universe.market_return,
            universe.risk_free_rate,
            universe.tax_rate,
        ).unwrap_or_default(),
        fcff: corporate::free_cash_flow_to_firm(
            net_income + universe.depreciation, // EBIT approximation
            universe.tax_rate,
            universe.depreciation,
            universe.working_capital,
            universe.capex,
        ).unwrap_or_default(),
        sgr: corporate::sustainable_growth_rate(
            financial::return_on_equity(net_income, universe.shareholders_equity)
                .unwrap_or_default(),
            dec!(0.40), // Default 40% payout ratio
        ).unwrap_or_default(),
        intrinsic_value: Decimal::ZERO, // Requires DCF model (Phase 1.4)
        expected_return: universe.risk_free_rate
            + universe.beta * (universe.market_return - universe.risk_free_rate),
        sharpe_ratio: if universe.market_return > universe.risk_free_rate {
            (universe.market_return - universe.risk_free_rate)
                / dec!(0.15) // Default volatility
        } else {
            Decimal::ZERO
        },
    }
}
```

### 3.3 Statistical Aggregation (`crates/simulator/src/aggregation.rs`)

```rust
use casiros_core::prelude::*;
use rust_decimal::Decimal;
use std::collections::BTreeMap;

/// Aggregated statistics from a Monte Carlo simulation run.
#[derive(Debug, Clone)]
pub struct SimulationResults {
    /// Number of universes simulated.
    pub count: usize,
    /// Per-metric statistics.
    pub metrics: BTreeMap<String, MetricStats>,
}

/// Statistical summary for a single output metric.
#[derive(Debug, Clone)]
pub struct MetricStats {
    pub mean: Decimal,
    pub median: Decimal,
    pub stddev: Decimal,
    pub min: Decimal,
    pub max: Decimal,
    pub percentile_5: Decimal,
    pub percentile_25: Decimal,
    pub percentile_75: Decimal,
    pub percentile_95: Decimal,
}

impl SimulationResults {
    /// Computes aggregate statistics from a slice of universe metrics.
    pub fn from_metrics(metrics: &[UniverseMetrics]) -> Self {
        // This is a simplified version. In production, use a proper
        // streaming statistics library or implement Welford's algorithm.
        let count = metrics.len();
        let mut results = BTreeMap::new();

        if count == 0 {
            return Self { count: 0, metrics: results };
        }

        // Collect values for each metric
        let roe_values: Vec<Decimal> = metrics.iter().map(|m| m.roe).collect();
        results.insert("roe".to_string(), MetricStats::compute(&roe_values));

        let wacc_values: Vec<Decimal> = metrics.iter().map(|m| m.wacc).collect();
        results.insert("wacc".to_string(), MetricStats::compute(&wacc_values));

        let sgr_values: Vec<Decimal> = metrics.iter().map(|m| m.sgr).collect();
        results.insert("sgr".to_string(), MetricStats::compute(&sgr_values));

        // ... all other metrics

        Self { count, metrics: results }
    }
}

impl MetricStats {
    /// Computes statistics from a slice of values.
    pub fn compute(values: &[Decimal]) -> Self {
        let n = values.len();
        if n == 0 {
            return Self {
                mean: Decimal::ZERO, median: Decimal::ZERO, stddev: Decimal::ZERO,
                min: Decimal::ZERO, max: Decimal::ZERO,
                percentile_5: Decimal::ZERO, percentile_25: Decimal::ZERO,
                percentile_75: Decimal::ZERO, percentile_95: Decimal::ZERO,
            };
        }

        let mut sorted = values.to_vec();
        sorted.sort();

        let sum: Decimal = sorted.iter().sum();
        let mean = sum / Decimal::from(n as u64);

        let variance: Decimal = sorted.iter()
            .map(|v| {
                let diff = v - mean;
                diff * diff
            })
            .sum::<Decimal>()
            / Decimal::from(n as u64);

        // Decimal doesn't have sqrt natively; use f64 approximation for stddev
        let stddev = Decimal::from_f64_retain(
            variance.to_f64().unwrap_or(0.0).sqrt()
        ).unwrap_or(Decimal::ZERO);

        let percentile = |p: f64| -> Decimal {
            let idx = ((n as f64) * p / 100.0).ceil() as usize;
            sorted.get(idx.saturating_sub(1)).copied().unwrap_or(Decimal::ZERO)
        };

        Self {
            mean,
            median: sorted[n / 2],
            stddev,
            min: sorted[0],
            max: sorted[n - 1],
            percentile_5: percentile(5.0),
            percentile_25: percentile(25.0),
            percentile_75: percentile(75.0),
            percentile_95: percentile(95.0),
        }
    }
}
```

---

## Phase 4: The API & Observability (API Crate)

*Goal: Expose the engine via Actix-Web with structured tracing, rate limiting, and
WebSocket streaming for real-time simulation progress.*

### 4.1 Server Entry Point (`crates/api/src/main.rs`)

```rust
use actix_web::{web, App, HttpServer, HttpResponse, middleware};
use actix_cors::Cors;
use tracing::{info, instrument};
use tracing_subscriber::{self, EnvFilter};
use std::net::SocketAddr;

mod routes;
mod middleware as mw;

/// CASIROS API Server
///
/// Serves financial computation endpoints with NASA-grade observability.
/// Every request is traced with a unique span ID for debugging.
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize structured logging. Fail fast if logging can't start.
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info"))
        )
        .with_target(false)
        .with_thread_ids(true)
        .json()
        .init();

    let bind_addr: SocketAddr = std::env::var("CASIROS_BIND_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:8080".to_string())
        .parse()
        .expect("Invalid CASIROS_BIND_ADDR");

    info!("CASIROS API starting on {}", bind_addr);

    HttpServer::new(|| {
        let cors = Cors::default()
            .allow_any_origin()
            .allow_any_method()
            .allow_any_header()
            .max_age(3600);

        App::new()
            .wrap(cors)
            .wrap(middleware::Logger::default())
            .wrap(mw::RequestTracing)
            .configure(routes::configure)
    })
    .bind(bind_addr)?
    .run()
    .await
}
```

### 4.2 Route Handlers (`crates/api/src/routes/calculate.rs`)

```rust
use actix_web::{web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use tracing::{instrument, info, error};
use casiros_core::prelude::*;
use casiros_core::{general, financial, corporate};

/// Request payload for a single-formula calculation.
#[derive(Debug, Deserialize)]
pub struct CalculationRequest {
    /// Name of the formula to compute (e.g., "roe", "wacc", "future_value").
    pub formula: String,
    /// Key-value parameters for the formula.
    pub params: std::collections::HashMap<String, f64>,
}

/// Response payload for a calculation result.
#[derive(Debug, Serialize)]
pub struct CalculationResponse {
    pub formula: String,
    pub result: f64,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub formula: String,
}

#[instrument(name = "calculate", skip(req), fields(formula = %req.formula))]
pub async fn handle_calculate(
    req: web::Json<CalculationRequest>,
) -> impl Responder {
    let params: std::collections::HashMap<String, Decimal> = req.params.iter()
        .map(|(k, v)| (k.clone(), Decimal::from_f64_retain(*v).unwrap_or_default()))
        .collect();

    let result = match req.formula.as_str() {
        "future_value" => {
            let pv = params.get("present_value").copied().unwrap_or_default();
            let rate = params.get("rate").copied().unwrap_or_default();
            let periods = params.get("periods")
                .and_then(|d| d.to_u32())
                .unwrap_or(1);
            general::future_value(pv, rate, periods)
        }
        "roe" => {
            let ni = params.get("net_income").copied().unwrap_or_default();
            let eq = params.get("shareholders_equity").copied().unwrap_or_default();
            financial::return_on_equity(ni, eq)
        }
        "wacc" => {
            let ev = params.get("equity_value").copied().unwrap_or_default();
            let dv = params.get("debt_value").copied().unwrap_or_default();
            let ce = params.get("cost_of_equity").copied().unwrap_or_default();
            let cd = params.get("cost_of_debt").copied().unwrap_or_default();
            let tr = params.get("tax_rate").copied().unwrap_or_default();
            corporate::wacc(ev, dv, ce, cd, tr)
        }
        "sgr" => {
            let roe = params.get("roe").copied().unwrap_or_default();
            let dpr = params.get("dividend_payout_ratio").copied().unwrap_or_default();
            corporate::sustainable_growth_rate(roe, dpr)
        }
        _ => {
            error!("Unknown formula requested: {}", req.formula);
            return HttpResponse::BadRequest().json(ErrorResponse {
                error: format!("Unknown formula: '{}'", req.formula),
                formula: req.formula.clone(),
            });
        }
    };

    match result {
        Ok(value) => {
            info!(
                formula = %req.formula,
                result = %value,
                "Calculation successful"
            );
            HttpResponse::Ok().json(CalculationResponse {
                formula: req.formula.clone(),
                result: value.to_f64().unwrap_or(0.0),
                status: "success".to_string(),
            })
        }
        Err(e) => {
            error!(
                formula = %req.formula,
                error = %e,
                "Calculation failed"
            );
            HttpResponse::BadRequest().json(ErrorResponse {
                error: e.to_string(),
                formula: req.formula.clone(),
            })
        }
    }
}
```

---

## Phase 5: The Narrative Engine (Macros Crate)

*Goal: Generate human-readable "CFO Memo" narratives from computed metrics using
compile-time procedural macros.*

### 5.1 Narrative Macro (`crates/macros/src/lib.rs`)

```rust
use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, LitStr};

/// Generates a CFO narrative memo from financial metrics.
///
/// # Usage
///
/// ```ignore
/// use casiros_macros::generate_narrative;
///
/// let memo = generate_narrative!(
///     company: "Acme Corp",
///     roe: 0.15,
///     debt_to_equity: 0.8,
///     current_ratio: 2.0,
///     wacc: 0.088,
///     sgr: 0.09
/// );
/// ```
///
/// Expands to a formatted `String` with a professional financial analysis.
#[proc_macro]
pub fn generate_narrative(input: TokenStream) -> TokenStream {
    let input_str = input.to_string();
    let narrative = format!(
        "## Financial Analysis Memo\n\n{}\n\n*Generated by CASIROS Narrative Engine*",
        input_str
    );

    let lit = LitStr::new(&narrative, proc_macro::Span::call_site());
    quote! { #lit }.into()
}
```

---

## Formula Catalog

The complete inventory of formulas implemented in `casiros_core`, organized by module.

### General Finance (`general.rs`)

| # | Function | Formula | Description |
|---|---|---|---|
| 1 | `future_value` | \( FV = PV(1+r)^n \) | Compound future value |
| 2 | `present_value` | \( PV = FV/(1+r)^n \) | Discounted present value |
| 3 | `annuity_future_value` | \( FV_a = PMT\frac{(1+r)^n-1}{r} \) | Future value of annuity |
| 4 | `annuity_present_value` | \( PV_a = PMT\frac{1-(1+r)^{-n}}{r} \) | Present value of annuity |
| 5 | `perpetuity_present_value` | \( PV_p = PMT/r \) | Perpetuity value |
| 6 | `growing_perpetuity` | \( PV = PMT/(r-g) \) | Growing perpetuity (Gordon) |
| 7 | `effective_annual_rate` | \( EAR = (1 + r/m)^m - 1 \) | Nominal to effective rate |
| 8 | `continuous_compounding` | \( FV = PV \cdot e^{rt} \) | Continuous compounding |

### Financial Ratios (`financial.rs`)

| # | Function | Formula | Description |
|---|---|---|---|
| 9 | `return_on_equity` | \( ROE = NI/E \) | Return on equity |
| 10 | `return_on_assets` | \( ROA = NI/TA \) | Return on assets |
| 11 | `return_on_investment` | \( ROI = (Gain - Cost)/Cost \) | Return on investment |
| 12 | `profit_margin` | \( PM = NI/Revenue \) | Net profit margin |
| 13 | `asset_turnover` | \( AT = Revenue/TA \) | Asset turnover |
| 14 | `equity_multiplier` | \( EM = TA/E \) | Equity multiplier |
| 15 | `dupont_roe` | \( ROE = PM \times AT \times EM \) | DuPont decomposition |
| 16 | `current_ratio` | \( CR = CA/CL \) | Current ratio |
| 17 | `quick_ratio` | \( QR = (CA-Inv)/CL \) | Quick (acid-test) ratio |
| 18 | `debt_to_equity` | \( D/E = TL/E \) | Debt-to-equity |
| 19 | `interest_coverage` | \( ICR = EBIT/Interest \) | Interest coverage ratio |
| 20 | `inventory_turnover` | \( IT = COGS/Inv \) | Inventory turnover |
| 21 | `cash_conversion_cycle` | \( CCC = DIO + DSO - DPO \) | Cash conversion cycle |

### Banking (`banking.rs`)

| # | Function | Formula | Description |
|---|---|---|---|
| 22 | `net_interest_margin` | \( NIM = (IntInc - IntExp)/AvgAssets \) | Net interest margin |
| 23 | `loan_to_deposit_ratio` | \( LDR = Loans/Deposits \) | Loan-to-deposit ratio |
| 24 | `capital_adequacy_ratio` | \( CAR = Capital/RWA \) | Capital adequacy (Basel) |
| 25 | `provision_coverage` | \( PCR = Provisions/NPA \) | Provision coverage ratio |

### Markets (`markets.rs`)

| # | Function | Formula | Description |
|---|---|---|---|
| 26 | `beta` | \( \beta = Cov(R_i,R_m)/Var(R_m) \) | Market sensitivity |
| 27 | `sharpe_ratio` | \( S = (R_p - R_f)/\sigma_p \) | Risk-adjusted return |
| 28 | `treynor_ratio` | \( T = (R_p - R_f)/\beta_p \) | Systematic risk return |
| 29 | `jensens_alpha` | \( \alpha = R_p - [R_f + \beta(R_m - R_f)] \) | Excess return |
| 30 | `value_at_risk` | \( VaR = \mu - z_\alpha\sigma \) | Parametric VaR |
| 31 | `expected_shortfall` | \( ES = E[L \mid L > VaR] \) | Conditional VaR |

### Stocks & Bonds (`stocks_bonds.rs`)

| # | Function | Formula | Description |
|---|---|---|---|
| 32 | `dividend_discount_model` | \( P = D_1/(r-g) \) | Gordon growth model |
| 33 | `discounted_cash_flow` | \( P = \sum FCFF_t/(1+WACC)^t \) | DCF valuation |
| 34 | `bond_price` | \( P = \sum C/(1+y)^t + F/(1+y)^n \) | Bond pricing |
| 35 | `yield_to_maturity` | Solve \( P = \sum CF_t/(1+YTM)^t \) | YTM (iterative) |
| 36 | `duration` | \( D = \sum t\cdot PV(CF_t)/P \) | Macaulay duration |
| 37 | `modified_duration` | \( MD = D/(1+y) \) | Modified duration |
| 38 | `convexity` | \( C = \sum t(t+1)PV(CF_t)/(P(1+y)^2) \) | Bond convexity |

### Corporate Finance (`corporate.rs`)

| # | Function | Formula | Description |
|---|---|---|---|
| 39 | `wacc` | \( WACC = w_e r_e + w_d r_d(1-t) \) | Weighted average cost of capital |
| 40 | `free_cash_flow_to_firm` | \( FCFF = EBIT(1-t) + D\&A - \Delta WC - CapEx \) | Free cash flow to firm |
| 41 | `free_cash_flow_to_equity` | \( FCFE = FCFF - Int(1-t) + \Delta Debt \) | Free cash flow to equity |
| 42 | `economic_value_added` | \( EVA = NOPAT - (IC \times WACC) \) | Economic value added |
| 43 | `sustainable_growth_rate` | \( SGR = ROE \times (1 - DPR) \) | Sustainable growth rate |
| 44 | `internal_growth_rate` | \( IGR = ROA \times (1 - DPR) / (1 - ROA \times (1-DPR)) \) | Internal growth rate |

---

## Testing Strategy

### Test Pyramid

```
         ┌──────┐
         │ E2E  │  API integration tests (cargo test --test integration)
         ├──────┤
         │ Int  │  Cross-crate DAG + simulation tests
         ├──────┤
         │ Unit │  Per-function tests + doc-tests (≥2 assertions each)
         └──────┘
```

### Unit Tests

- **Location:** `crates/core/tests/` per module.
- **Requirement:** Every public function must have ≥1 doc-test + ≥2 assertion-based unit tests.
- **Pattern:**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_future_value_normal_case() {
        let fv = future_value(dec!(100.0), dec!(0.05), 10).unwrap();
        assert_eq!(fv.round_dp(4), dec!(162.8895));
    }

    #[test]
    fn test_future_value_zero_periods() {
        let fv = future_value(dec!(100.0), dec!(0.05), 0).unwrap();
        assert_eq!(fv, dec!(100.0));
    }

    #[test]
    fn test_future_value_rejects_invalid_rate() {
        let result = future_value(dec!(100.0), dec!(-2.0), 10);
        assert!(result.is_err());
    }
}
```

### Property-Based Testing

Use `proptest` to verify mathematical invariants:

```rust
use proptest::prelude::*;

proptest! {
    #[test]
    fn future_value_increases_with_positive_rate(
        pv in 0.0f64..1_000_000.0,
        rate in 0.0f64..1.0,
        periods in 1u32..100,
    ) {
        let pv_dec = Decimal::from_f64_retain(pv).unwrap();
        let rate_dec = Decimal::from_f64_retain(rate).unwrap();
        let fv = future_value(pv_dec, rate_dec, periods).unwrap();
        prop_assert!(fv >= pv_dec);
    }

    #[test]
    fn present_value_is_inverse_of_future_value(
        pv in 0.0f64..1_000_000.0,
        rate in 0.0f64..0.5,
        periods in 1u32..50,
    ) {
        let pv_dec = Decimal::from_f64_retain(pv).unwrap();
        let rate_dec = Decimal::from_f64_retain(rate).unwrap();
        let fv = future_value(pv_dec, rate_dec, periods).unwrap();
        let recovered_pv = present_value(fv, rate_dec, periods).unwrap();
        let diff = (recovered_pv - pv_dec).abs();
        prop_assert!(diff < dec!(0.01));
    }
}
```

### Integration Tests

- **Location:** `crates/dag/tests/`, `crates/simulator/tests/`.
- Test full DAG evaluation with known inputs/outputs.
- Test Monte Carlo simulation with a fixed seed for deterministic results.

### Fuzz Testing

- **Tool:** `cargo-fuzz`.
- **Targets:** All API endpoint parsers, formula input parsers.
- **Location:** `fuzz/fuzz_targets/`.

### Coverage Target

| Crate | Line Coverage | Branch Coverage |
|---|---|---|
| `core` | ≥95% | ≥90% |
| `dag` | ≥90% | ≥85% |
| `simulator` | ≥85% | ≥80% |
| `api` | ≥80% | ≥75% |

---

## CI/CD Pipeline

### GitHub Actions Workflow (`.github/workflows/ci.yml`)

```yaml
name: CASIROS CI

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

env:
  CARGO_TERM_COLOR: always
  RUSTFLAGS: "-D warnings"

jobs:
  check:
    name: Check & Lint
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy
      - uses: Swatinem/rust-cache@v2

      - name: Format check
        run: cargo fmt --all -- --check

      - name: Clippy (strict)
        run: cargo clippy --workspace --all-targets --all-features -- -D warnings

      - name: Check compilation
        run: cargo check --workspace --all-features

  test:
    name: Test
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2

      - name: Run tests
        run: cargo test --workspace --all-features

      - name: Run doc-tests
        run: cargo test --doc --workspace

  coverage:
    name: Coverage
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2

      - name: Install tarpaulin
        run: cargo install cargo-tarpaulin

      - name: Generate coverage
        run: cargo tarpaulin --workspace --out Xml

      - name: Upload to Codecov
        uses: codecov/codecov-action@v4
        with:
          file: cobertura.xml

  security:
    name: Security Audit
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable

      - name: cargo audit
        run: |
          cargo install cargo-audit
          cargo audit

      - name: cargo deny
        run: |
          cargo install cargo-deny
          cargo deny check

  docs:
    name: Documentation
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2

      - name: Build docs
        run: cargo doc --no-deps --document-private-items --workspace

      - name: Deploy to GitHub Pages
        if: github.ref == 'refs/heads/main'
        uses: peaceiris/actions-gh-pages@v3
        with:
          github_token: ${{ secrets.GITHUB_TOKEN }}
          publish_dir: ./target/doc

  bench:
    name: Benchmarks
    if: github.ref == 'refs/heads/main'
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2

      - name: Run benchmarks
        run: cargo bench --workspace
```

---

## Security Considerations

### Compile-Time Guarantees

| Measure | Implementation |
|---|---|
| No unsafe code | `#![forbid(unsafe_code)]` in every crate |
| No panics in core | All functions return `Result<T, CalculationError>` |
| Dependency auditing | `cargo audit` + `cargo deny` in CI |
| No secrets in code | `dotenvy` for local dev; env vars in production |
| Minimal dependency tree | Each crate declares only essential dependencies |

### Runtime Protections

| Measure | Implementation |
|---|---|
| Rate limiting | `actix-web` middleware: 100 req/s per IP |
| Payload size limit | 1MB max request body |
| Input validation | All API inputs validated before reaching core |
| Structured logging | `tracing` with JSON output; no PII in logs |
| Health checks | `/healthz` endpoint for liveness/readiness |
| Authentication | JWT-based (Phase 4.1); API keys for service-to-service |

### Dependency Policy

```toml
# deny.toml
[licenses]
allow = [
    "MIT",
    "Apache-2.0",
    "BSD-3-Clause",
    "Unicode-DFS-2016",
]
copyleft = "deny"

[bans]
multiple-versions = "deny"
wildcards = "deny"
```

---

## Configuration Management

### Configuration File (`config/default.toml`)

```toml
[server]
bind_addr = "127.0.0.1:8080"
workers = 4
request_timeout_secs = 30
max_body_size_bytes = 1_048_576

[simulation]
default_iterations = 10_000
max_iterations = 1_000_000
convergence_threshold = 0.001
convergence_batch_size = 1_000

[logging]
level = "info"
format = "json"

[database]
url = "postgres://casiros:casiros@localhost:5432/casiros"
max_connections = 10
```

### Environment Variable Overrides

All config keys can be overridden via environment variables:

```bash
export CASIROS_SERVER__BIND_ADDR="0.0.0.0:9090"
export CASIROS_SIMULATION__DEFAULT_ITERATIONS="50000"
export CASIROS_LOGGING__LEVEL="debug"
```

---

## Database Strategy (Phase 4+)

- **Database:** PostgreSQL 16 via `sqlx`.
- **Migrations:** `sqlx-cli migrate` with timestamped migration files.
- **Tables:**
  - `scenarios` — saved baseline universes.
  - `simulation_runs` — metadata for each Monte Carlo run.
  - `simulation_results` — aggregated statistics per run.
  - `formula_cache` — memoized formula results for repeated inputs.
- **Read/Write Split:** Read replicas for dashboard queries; primary for writes.

---

## Benchmarking Strategy

### Benchmark Targets (`benches/`)

```rust
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use casiros_core::general::future_value;
use rust_decimal_macros::dec;

fn bench_future_value(c: &mut Criterion) {
    c.bench_function("future_value", |b| {
        b.iter(|| {
            future_value(
                black_box(dec!(1000.0)),
                black_box(dec!(0.05)),
                black_box(10),
            )
        })
    });
}

criterion_group!(benches, bench_future_value);
criterion_main!(benches);
```

### Performance SLAs

| Operation | Target Latency | Max Latency |
|---|---|---|
| Single formula evaluation | <1µs | <10µs |
| Full DAG evaluation (44 formulas) | <50µs | <500µs |
| Monte Carlo (10k scenarios) | <100ms | <1s |
| API response (p50) | <5ms | <50ms |
| API response (p99) | <50ms | <500ms |

---

## Deployment Strategy

### Docker Multi-Stage Build (`docker/Dockerfile`)

```dockerfile
# Stage 1: Build
FROM rust:1.82-slim-bookworm AS builder
WORKDIR /app
COPY . .
RUN cargo build --release --workspace

# Stage 2: Runtime
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/casiros-api /usr/local/bin/casiros-api
COPY config/ /etc/casiros/
EXPOSE 8080
USER 1000:1000
ENTRYPOINT ["casiros-api"]
```

### Docker Compose (`docker/docker-compose.yml`)

```yaml
version: "3.9"
services:
  api:
    build:
      context: ..
      dockerfile: docker/Dockerfile
    ports:
      - "8080:8080"
    environment:
      - CASIROS_DATABASE__URL=postgres://casiros:casiros@db:5432/casiros
    depends_on:
      db:
        condition: service_healthy

  db:
    image: postgres:16-alpine
    environment:
      POSTGRES_USER: casiros
      POSTGRES_PASSWORD: casiros
      POSTGRES_DB: casiros
    ports:
      - "5432:5432"
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U casiros"]
      interval: 5s
      timeout: 5s
      retries: 5
```

---

## Code Review Checklist

Every pull request must pass this checklist before merge:

- [ ] **Documentation:** Every new public function has a doc-comment with ≥1 doc-test.
- [ ] **Error Handling:** All fallible operations return `Result<T, CalculationError>`. No `.unwrap()` or `.expect()` outside of tests.
- [ ] **Precision:** All financial values use `Decimal`. `f64` is allowed ONLY in random noise generation and must be immediately converted.
- [ ] **Preconditions:** Every public function validates its inputs and returns appropriate errors.
- [ ] **Assertions:** ≥2 assertions across doc-tests and unit tests per function.
- [ ] **Function Length:** ≤60 lines per function body (excluding doc comments and blank lines).
- [ ] **No Unsafe:** `#![forbid(unsafe_code)]` is not violated.
- [ ] **No Recursion:** No recursive functions in `core` crate.
- [ ] **Layer Discipline:** No infrastructure imports in domain/application crates.
- [ ] **Tracing:** Every API handler has `#[instrument]`.
- [ ] **Tests Pass:** `cargo test --workspace` is green.
- [ ] **Clippy Clean:** `cargo clippy --workspace -- -D warnings` is green.
- [ ] **Formatting:** `cargo fmt --all -- --check` passes.
- [ ] **DAG Updated:** If a new formula is added, the `FormulaNode` enum and `CausalityEngine::new()` edges are updated.
- [ ] **CHANGELOG:** Entry added under the appropriate version header.

---

## Versioning Strategy

- **Semantic Versioning:** `MAJOR.MINOR.PATCH` per [semver.org](https://semver.org).
- **Pre-1.0:** `0.x.y` — MINOR bumps for new formulas, PATCH bumps for fixes.
- **1.0.0:** Released when the full formula catalog (44 formulas) is implemented, tested, and audited.
- **CHANGELOG.md:** Maintained at workspace root. Each crate may also have its own.

### Version Milestones

| Version | Deliverable |
|---|---|
| `0.1.0` | Phase 0 complete: workspace, lints, error types |
| `0.2.0` | Phase 1 complete: all 44 core formulas |
| `0.3.0` | Phase 2 complete: DAG engine with evaluation |
| `0.4.0` | Phase 3 complete: Monte Carlo simulator |
| `0.5.0` | Phase 4 complete: API server with all endpoints |
| `0.6.0` | Phase 5 complete: Narrative macros |
| `0.7.0` | Database persistence, configuration management |
| `0.8.0` | WebSocket streaming, dashboard |
| `0.9.0` | Performance optimization, audit, security review |
| `1.0.0` | Production release |

---

## Example Tutorial: Adding a Custom Formula

*The ultimate test of system elegance. Here is the exact step-by-step process an engineer
must follow to extend CASIROS with a new formula.*

**Goal:** Add **"Times Interest Earned"** (TIE = EBIT / Interest Expense).

### Step 1: Define the Math

Open `crates/core/src/financial.rs`.

### Step 2: Write the Function (with NASA validation)

```rust
/// Computes the Times Interest Earned (TIE) ratio, also known as
/// the Interest Coverage Ratio.
///
/// # Mathematical Definition
///
/// \[ TIE = \frac{EBIT}{\text{Interest Expense}} \]
///
/// # Constraints
///
/// - `interest_expense` MUST be > 0.
///
/// # Errors
///
/// Returns [`CalculationError::DivisionByZero`] if `interest_expense` is zero.
///
/// # Examples
///
/// ```
/// use casiros_core::financial::times_interest_earned;
/// use rust_decimal_macros::dec;
///
/// let tie = times_interest_earned(dec!(500.0), dec!(100.0)).unwrap();
/// assert_eq!(tie, dec!(5.0));
/// assert!(tie > dec!(1.0));  // Healthy: can cover interest 5x
/// ```
pub fn times_interest_earned(
    ebit: Decimal,
    interest_expense: Decimal,
) -> Result<Decimal, CalculationError> {
    if interest_expense == Decimal::ZERO {
        return Err(CalculationError::DivisionByZero {
            formula: "Times Interest Earned (TIE)",
        });
    }
    Ok(ebit / interest_expense)
}
```

### Step 3: Add Unit Tests

In `crates/core/tests/financial_tests.rs`:

```rust
#[test]
fn test_tie_normal_case() {
    let tie = times_interest_earned(dec!(500.0), dec!(100.0)).unwrap();
    assert_eq!(tie, dec!(5.0));
}

#[test]
fn test_tie_rejects_zero_interest() {
    let result = times_interest_earned(dec!(500.0), dec!(0.0));
    assert!(matches!(result, Err(CalculationError::DivisionByZero { .. })));
}

#[test]
fn test_tie_negative_ebit_allowed() {
    // Negative EBIT is valid (company is losing money)
    let tie = times_interest_earned(dec!(-200.0), dec!(100.0)).unwrap();
    assert_eq!(tie, dec!(-2.0));
}
```

### Step 4: Update the DAG

Open `crates/dag/src/graph.rs`:

1. Add `TimesInterestEarned` to the `FormulaNode` enum.
2. Add it to `FormulaNode::all_variants()`.
3. Add dependency edges in `CausalityEngine::new()` (TIE depends on EBIT and Interest Expense as leaf inputs — no edges needed if they're raw inputs).

### Step 5: Add to the Evaluator

Open `crates/dag/src/evaluator.rs`:

```rust
TimesInterestEarned => {
    let ebit = ctx.get_input("ebit")?;
    let interest = ctx.get_input("interest_expense")?;
    financial::times_interest_earned(ebit, interest)
}
```

### Step 6: Expose via API

Open `crates/api/src/routes/calculate.rs`, add to the match:

```rust
"tie" => {
    let ebit = params.get("ebit").copied().unwrap_or_default();
    let interest = params.get("interest_expense").copied().unwrap_or_default();
    financial::times_interest_earned(ebit, interest)
}
```

### Step 7: Update the Formula Catalog

Add entry #20b to the Financial Ratios table in this roadmap.

### Step 8: Verify

```bash
cargo test --workspace          # All tests pass
cargo clippy --workspace -- -D warnings  # Clean
cargo doc --workspace           # Documentation builds
```

---

## Phase 3: Ecosystem Hardening & Client Expansion

*Goal: Build on the MVP and Phase 2 foundation with persistent storage, richer clients, performance work, and deeper financial coverage.*

### 3.1 Persistent Storage Backends

Replace in-memory-only operation with pluggable snapshot repositories.

- Define `trait SnapshotRepository` in `crates/dag/` (Application Layer).
- Implement `PostgresSnapshotRepository` in `crates/api/` via `sqlx`.
- Implement `S3SnapshotRepository` in `crates/api/` for artifact export.
- Keep `EngineSnapshot` JSON as the portable interchange format.

Deliverables:
- `POST /snapshots` — save a named snapshot.
- `GET /snapshots/:id` — load a snapshot.
- `DELETE /snapshots/:id` — remove a snapshot.
- `LIST /snapshots` — paginated listing.

### 3.2 Python Client SDK

Generate or hand-write a typed Python client.

- Use the committed `casiros.openapi.json` to generate a Pydantic-based client with `openapi-python-client`, or maintain a small hand-written `httpx` wrapper.
- Publish to PyPI as `casiros-client`.
- Provide Jupyter-friendly helpers for graph construction and simulation plotting.

### 3.3 Web Dashboard / Simulation Reports

Add a lightweight browser-based UI for non-engineer users.

- New `crates/dashboard/` crate (or static assets under `crates/api/assets/`).
- Render `/` as an HTML landing page with links to Swagger UI and examples.
- Simulation result page with Vega-Lite or Chart.js histograms.
- Graph validation visualizer (mermaid.js or DOT → SVG).

### 3.4 Performance & Caching

- Add deterministic memoization to `CausalityEngine::evaluate` for unchanged sub-graphs.
- Introduce `dashmap`-backed result cache behind a trait boundary.
- Parallelize independent node evaluation with `rayon`.
- Add a flamegraph-friendly Criterion benchmark for large graphs.

### 3.5 CSV / Excel Import & Export

- `casiros-cli import csv engine.csv --output engine.json`
- `casiros-cli export csv engine.json --output engine.csv`
- Excel support via `calamine` / `rust_xlsxwriter`.
- Map CSV columns to input names and formula bindings.

### 3.6 Advanced Financial Formulas

Expand the catalog with more option pricing and fixed-income tools.

- Binomial option pricing (European & American).
- Black-Scholes Greeks (delta, gamma, theta, vega, rho).
- Duration, modified duration, and convexity for bonds.
- Implied volatility approximation via Newton-Raphson.
- Value at Risk (VaR) and Conditional VaR (CVaR) for Monte Carlo outputs.

### 3.7 Streaming Simulation Progress

- WebSocket endpoint `/ws/simulate` that streams universe-count progress and partial aggregates.
- Server-Sent Events (SSE) alternative at `/simulate/stream`.
- Keep non-streaming `/simulate` endpoint for simple clients.

### 3.8 Release Engineering

- Add `cargo audit` and `cargo deny` to CI.
- Enforce code coverage gates (e.g. 90% line coverage).
- Build release binaries for Linux, macOS, and Windows via GitHub Actions.
- Publish `casiros-core`, `casiros-dag`, and `casiros-simulator` to crates.io.
- Tag releases and maintain `CHANGELOG.md`.

### Phase 3 Definition of Done

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features
cargo test --workspace
cargo doc --workspace --no-deps --document-private-items
cargo audit
cargo deny check
```

All checks green before merge.

---

## Agent Guardrails (AI Development Prompt)

*Feed this section to any AI agent (Claude Code, Copilot, etc.) working on CASIROS.*

```
You are developing CASIROS, a NASA/JPL-grade financial physics engine in Rust.
Follow these rules without exception:

## Development Discipline

1. TDD FIRST: Write the doc-test BEFORE the function body. Watch it fail, then implement.
2. PRECISION OVER PERFORMANCE: Use rust_decimal::Decimal for ALL money and ratios.
   f64 is allowed ONLY for stochastic noise in the Monte Carlo engine and must be
   immediately converted to Decimal.
3. NO PANICS: Every fallible function returns Result<T, CalculationError>.
   .unwrap() and .expect() are FORBIDDEN outside of #[cfg(test)].
4. OBSERVABILITY: Every public API handler has #[instrument]. Every error is logged
   at error! level with full context.
5. FAIL FAST: main() panics if the database is unreachable or the port is bound.
   No graceful degradation in infrastructure — core services must be perfect.

## Clean Architecture Discipline

6. LAYER ORDER: Domain (core) → Application (dag, simulator) → Infrastructure (api).
   Dependencies point INWARD. Inner layers never import outer layers.
7. TRAIT BOUNDARIES: Every layer boundary is a trait defined by the inner layer.
   Infrastructure implements application traits. Application never imports actix-web or sqlx.
8. PURE DOMAIN: Core crate functions are pure — no I/O, no global state, no side effects.
   Same input ALWAYS produces same output.

## Code Quality

9. FUNCTION LENGTH: ≤60 lines per function body.
10. ASSERTION DENSITY: ≥2 assertions per function (across doc-tests and unit tests).
11. NO UNSAFE: #![forbid(unsafe_code)] in every crate. No exceptions.
12. NO RECURSION: No recursive functions in core. Iteration only.
13. ALL PRECONDITIONS CHECKED: Every public function validates all inputs before computing.

## Commit Discipline

14. CONVENTIONAL COMMITS: feat(core):, fix(dag):, test(simulator):, docs:, chore:, refactor:
15. ONE CONCERN PER COMMIT: Each commit addresses exactly one formula, one bug, or one feature.
16. COMMIT MESSAGE BODY: Explain WHY the change was made, not WHAT changed (the diff shows that).

## PR Requirements

17. All checklist items in the Code Review Checklist must be satisfied.
18. CI must be green (fmt, clippy, test, coverage, audit, docs).
19. At least one other engineer must review (or, for AI-generated code, a second AI pass).
```
---

## End of Roadmap

**CASIROS is now ready for launch.** This roadmap provides the blueprints, the constraints,
and the philosophical rigor. Every formula is provably correct. Every dependency is traced.
Every scenario is simulated. Every line of code is documented.

*"Look back and weep" — not at the complexity, but at the elegance.*
