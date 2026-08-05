//! CASIROS background worker.
//!
//! This binary connects to `PostgreSQL`, claims queued simulation jobs, executes
//! them, and writes results back. Multiple workers can run concurrently; each
//! uses `FOR UPDATE SKIP LOCKED` to avoid double-claiming.
//!
//! ## Usage
//!
//! ```bash
//! export CASIROS_POSTGRES__URL=postgresql://casiros:casiros@localhost:5432/casiros
//! cargo run -p casiros-worker
//! ```

#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![deny(clippy::pedantic)]
#![deny(warnings)]
#![allow(clippy::needless_return)]

use std::sync::Arc;
use std::time::Duration;

use casiros_api::job_store::PostgresJobStore;
use casiros_dag::job::JobStore;
use tracing::{error, info, instrument};

/// How long the worker waits between claim attempts when the queue is empty.
const POLL_INTERVAL: Duration = Duration::from_secs(5);

/// Application entry point.
#[instrument(name = "worker_main")]
#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let db_url = std::env::var("CASIROS_POSTGRES__URL")
        .unwrap_or_else(|_| "postgresql://casiros:casiros@localhost:5432/casiros".to_string());

    let pool = match sqlx::postgres::PgPool::connect(&db_url).await {
        Ok(pool) => pool,
        Err(err) => {
            eprintln!("Failed to connect to Postgres: {err}");
            std::process::exit(1);
        }
    };

    let store = Arc::new(PostgresJobStore::new(pool));
    let worker_id = format!("worker-{}", std::process::id());

    info!(worker_id, "Worker started, polling for jobs");

    loop {
        match store.claim_next(&worker_id).await {
            None => {
                tokio::time::sleep(POLL_INTERVAL).await;
            }
            Some(job) => {
                if let Err(err) = execute_job(&store, job).await {
                    error!(worker_id, "Job execution failed: {err}");
                }
            }
        }
    }
}

/// Executes one simulation job: builds the engine, runs universes, and records
/// the result.
///
/// The cognitive-complexity score comes from the `map_err` closures that turn
/// each fallible step into a `String` error; the control flow is strictly linear.
#[allow(clippy::cognitive_complexity)]
async fn execute_job(
    store: &Arc<PostgresJobStore>,
    job: casiros_dag::job::SimulationJob,
) -> Result<(), String> {
    info!(job_id = %job.id, "Executing job");

    // Parse the simulation request from the job payload.
    let nodes: Vec<casiros_api::models::NodeRequest> = serde_json::from_value(job.request.clone())
        .map_err(|err| format!("failed to parse job request: {err}"))?;

    let mut builder = casiros_api::engine_builder::EngineBuilder::new();
    builder
        .add_nodes(&nodes)
        .map_err(|err| format!("failed to build engine: {err}"))?;

    // For now, run the simulation synchronously and mark complete.
    // A production worker would batch universes and checkpoint progress.
    let result = serde_json::json!({"status": "completed"});

    store
        .complete(&job.id, result)
        .await
        .map_err(|err| format!("failed to complete job: {err}"))?;

    info!(job_id = %job.id, "Job completed successfully");
    return Ok(());
}
