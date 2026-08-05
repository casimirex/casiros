//! Simulation job store backends.
//!
//! Infrastructure-layer implementations of the
//! [`casiros_dag::job::JobStore`] trait declared in the Application Layer.

pub mod in_memory;
pub use in_memory::InMemoryJobStore;
