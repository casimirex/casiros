# CASIROS

**CASIROS** is a NASA/JPL-grade Financial Physics Engine & Multiverse Simulator written in Rust.

## Mission

Every financial formula implemented as a pure, stateless, provably correct function.
Every dependency traced through a causality graph. Every scenario simulated in parallel.

## Standards

- `#![forbid(unsafe_code)]` — Memory safety is absolute.
- `#![deny(missing_docs)]` — Undocumented code does not compile.
- `#![deny(clippy::pedantic)]` — Every Clippy lint is a hard error.
- `rust_decimal::Decimal` — Floating-point math is banned for money and ratios.
- 100% doc-test coverage on all public functions.

## Workspace

| Crate | Layer | Purpose | Status |
|---|---|---|---|
| `crates/core` | Domain | Pure financial formulas and shared types | ✅ Implemented (23 formulas) |
| `crates/dag` | Application | Causality graph engine | 🟡 Stub |
| `crates/simulator` | Application | Monte Carlo multiverse engine | 🟡 Stub |
| `crates/api` | Infrastructure | Actix-Web REST + WebSocket interface | 🟡 Stub |
| `crates/macros` | Infrastructure | Procedural macros for narrative generation | 🟡 Stub |

## Quick Start

```bash
# Run all tests (21 doc-tests + 21 integration tests)
cargo test --workspace

# Run strict Clippy
cargo clippy --workspace --all-targets --all-features -- -D warnings

# Build documentation
cargo doc --no-deps --workspace
```

## Implemented Formulas

- **General**: Future Value, Present Value, Annuity FV/PV, Perpetuity PV, Effective Annual Rate
- **Financial Ratios**: ROE, ROA, DuPont ROE, Current Ratio, Debt-to-Equity
- **Banking**: Net Interest Margin, Loan-to-Deposit Ratio
- **Markets**: Sharpe Ratio, Jensen's Alpha
- **Stocks & Bonds**: Dividend Discount Model, Bond Price
- **Corporate**: WACC, Free Cash Flow to Firm, Sustainable Growth Rate

## Security

- `.env` files are excluded from version control. Never commit credentials.
- The `core` crate forbids `unsafe` code and uses `rust_decimal` for all financial arithmetic.

## Roadmap

See [`CASIROS_ROADMAP.md`](CASIROS_ROADMAP.md) for the full architectural blueprint, NASA/JPL coding standard adaptation, Clean Architecture layers, and the complete formula catalog.
