//! CASIROS background worker.
//!
//! This binary connects to `PostgreSQL`, claims queued simulation jobs, executes
//! them in batches with progress checkpointing, and writes results back. Multiple
//! workers can run concurrently; each uses `FOR UPDATE SKIP LOCKED` to avoid
//! double-claiming.
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

use casiros_api::engine_builder::{EngineBuilder, distribution_from_request};
use casiros_api::job_store::PostgresJobStore;
use casiros_core::job::{JobProgress, JobStatus};
use casiros_dag::job::JobStore;
use casiros_simulator::simulation::MonteCarloConfig;
use tracing::{error, info, instrument, warn};

/// How long the worker waits between claim attempts when the queue is empty.
const POLL_INTERVAL: Duration = Duration::from_secs(5);

/// Number of universes to simulate in a single batch before checkpointing.
const BATCH_SIZE: usize = 100;

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

/// Executes one simulation job in batches with progress checkpointing.
///
/// The control flow is strictly linear; the cognitive-complexity score comes
/// from the `map_err` closures that turn each fallible step into a `String`
/// error.
#[allow(clippy::cognitive_complexity)]
async fn execute_job(
    store: &Arc<PostgresJobStore>,
    job: casiros_dag::job::SimulationJob,
) -> Result<(), String> {
    info!(job_id = %job.id, "Executing job");

    // Parse the simulation request from the job payload.
    let request: casiros_api::models::CreateJobRequest =
        serde_json::from_value(job.request.clone())
            .map_err(|err| format!("failed to parse job request: {err}"))?;

    // Build the engine from the request.
    let mut builder = EngineBuilder::new();
    builder
        .add_nodes(&request.nodes)
        .map_err(|err| format!("failed to build engine: {err}"))?;
    builder
        .add_edges(&request.edges)
        .map_err(|err| format!("failed to add edges: {err}"))?;

    let Some(target_id) = builder.node_id(&request.target) else {
        store
            .fail(
                &job.id,
                format!("target node '{}' not found", request.target),
            )
            .await
            .ok();
        return Err(format!("target node '{}' not found", request.target));
    };

    // Build the Monte Carlo config.
    let mut config = MonteCarloConfig::new(request.universe_count, request.seed.unwrap_or(42))
        .map_err(|err| format!("failed to create config: {err}"))?;

    for binding in &request.bindings {
        let Some(node_id) = builder.node_id(&binding.node) else {
            store
                .fail(
                    &job.id,
                    format!("binding node '{}' not found", binding.node),
                )
                .await
                .ok();
            return Err(format!("binding node '{}' not found", binding.node));
        };
        config.bind(node_id, distribution_from_request(&binding.distribution));
    }

    let engine = builder.build();
    let total = request.universe_count;
    let mut completed = 0;
    let mut all_values: Vec<rust_decimal::Decimal> = Vec::with_capacity(total);

    while completed < total {
        // Check for cancellation before each batch.
        if let Ok(current) = store.get(&job.tenant_id, &job.workspace_id, &job.id).await
            && current.status == JobStatus::Cancelled
        {
            info!(job_id = %job.id, "Job was cancelled");
            return Ok(());
        }

        let batch = BATCH_SIZE.min(total - completed);
        match config.run_batch(&engine, target_id, completed, batch) {
            Ok(values) => {
                all_values.extend(values);
                completed += batch;

                let progress = JobProgress {
                    universes_total: total,
                    universes_completed: completed,
                    last_checkpoint_at: Some(time::OffsetDateTime::now_utc()),
                };
                store
                    .update_progress(&job.id, &progress)
                    .await
                    .map_err(|err| format!("failed to update progress: {err}"))?;

                info!(
                    job_id = %job.id,
                    completed,
                    total,
                    "Batch completed"
                );
            }
            Err(err) => {
                warn!(job_id = %job.id, error = %err, "Batch failed");
                store.fail(&job.id, err.to_string()).await.ok();
                return Err(format!("simulation failed: {err}"));
            }
        }
    }

    // Aggregate and complete.
    let result = MonteCarloConfig::aggregate(all_values)
        .map_err(|err| format!("failed to aggregate results: {err}"))?;
    let result_json =
        serde_json::to_value(result).map_err(|err| format!("failed to serialize result: {err}"))?;

    store
        .complete(&job.id, result_json)
        .await
        .map_err(|err| format!("failed to complete job: {err}"))?;

    info!(job_id = %job.id, "Job completed successfully");
    return Ok(());
}
