//! End-to-end smoke tests for CASIROS.
//!
//! This crate has no runtime code. It exists to host integration tests in
//! `tests/` that launch the compiled `casiros-api` and `casiros-worker`
//! binaries and drive them over real HTTP.
//!
//! ## Why a separate crate
//!
//! The other test suites build an Actix `App` in-process and call handlers
//! directly. That is fast and precise, but it rebuilds the application by hand
//! and so cannot see anything that lives in `main.rs`: which storage backend
//! got selected, whether an environment variable was spelled in a form the
//! config crate recognises, whether a route was actually registered, or whether
//! the API and the worker agree on where jobs live.
//!
//! Every defect these tests were written to catch passed the in-process suites.

#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![deny(clippy::pedantic)]
#![deny(warnings)]
