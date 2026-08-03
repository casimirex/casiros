//! CASIROS HTTP API library.
//!
//! This crate is both a library and a binary. The library exposes the request
//! models, engine builder, and HTTP handlers so they can be tested in-process.
//! The binary (`src/main.rs`) wires the handlers into an Actix-Web server.

#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![deny(clippy::pedantic)]
#![deny(warnings)]
#![allow(clippy::needless_return)]

pub mod engine_builder;
pub mod handlers;
pub mod models;
pub mod validation;
