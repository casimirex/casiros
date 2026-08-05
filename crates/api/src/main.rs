//! CASIROS HTTP API server.
//!
//! This is the infrastructure-layer entry point. It exposes the core financial
//! engine via a REST API and is responsible for observability, routing, and
//! external service adaptation.
//!
//! ## Layer
//!
//! Infrastructure Layer — depends on the Application Layer crates
//! ([`casiros_dag`], [`casiros_simulator`]) and the Domain Layer
//! ([`casiros_core`]).

#![forbid(unsafe_code)]
#![deny(missing_docs)]
#![deny(clippy::pedantic)]
#![deny(warnings)]
#![allow(clippy::needless_return)]

use std::sync::Arc;

use actix_cors::Cors;
use actix_files::Files;
use actix_web::{App, HttpServer, middleware, web};
use casiros_api::admin_handlers;
use casiros_api::audit::{AuditSink, InMemoryAuditLog, PostgresAuditLog};
use casiros_api::audit_handlers;
use casiros_api::audit_middleware::audit_middleware;
use casiros_api::auth::{AuthConfig, RateLimiter, auth_middleware, build_tenant_resolver};
use casiros_api::config::AppConfig;
use casiros_api::handlers;
use casiros_api::job_handlers;
use casiros_api::job_store::InMemoryJobStore;
use casiros_api::job_ws_handlers;
use casiros_api::metrics;
use casiros_api::metrics_middleware;
use casiros_api::openapi;
use casiros_api::repositories::{
    InMemorySnapshotRepository, PostgresSnapshotRepository, SnapshotRepo,
};
use casiros_api::snapshot_handlers;
use casiros_api::streaming_handlers;
use casiros_api::tenant::TenantResolver;
use casiros_api::tracing_middleware::TracingMiddleware;
use casiros_api::websocket_handlers;
#[cfg(feature = "otel")]
use opentelemetry::trace::TracerProvider as _;
#[cfg(feature = "otel")]
use opentelemetry_otlp::WithExportConfig;
use tracing::{info, instrument};

/// Application entry point.
///
/// Initializes structured logging and starts the Actix-Web server.
#[actix_web::main]
#[instrument(name = "main")]
async fn main() -> std::io::Result<()> {
    let app_config = AppConfig::load().map_err(|err| {
        eprintln!("Failed to load configuration: {err}");
        std::io::Error::new(std::io::ErrorKind::InvalidInput, err)
    })?;

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new(&app_config.log_level)),
        )
        .init();

    let bind_addr = app_config.bind_addr.clone();

    // Initialise OpenTelemetry when CASIROS_OTLP_ENDPOINT is set.
    // Uses HTTP/protobuf transport (reqwest) to avoid the tower 0.4.x
    // compatibility issue with the gRPC/tonic transport.
    #[cfg(feature = "otel")]
    if let Ok(endpoint) = std::env::var("CASIROS_OTLP_ENDPOINT") {
        let exporter = opentelemetry_otlp::SpanExporter::builder()
            .with_http()
            .with_endpoint(&endpoint)
            .build()
            .expect("OTel exporter must build");
        let provider = opentelemetry_sdk::trace::TracerProvider::builder()
            .with_simple_exporter(exporter)
            .build();
        let _tracer = provider.tracer("casiros-api");
        info!(otel_endpoint = %endpoint, "OpenTelemetry initialised");
    }

    info!("CASIROS API starting on {}", bind_addr);
    metrics::init_metrics();

    let auth_config = Arc::new(AuthConfig::from_env());
    let tenant_resolver: Arc<dyn TenantResolver> = build_tenant_resolver();
    let rate_limiter = Arc::new(RateLimiter::new());
    let job_store = Arc::new(InMemoryJobStore::new());
    let Backends {
        snapshot_repo,
        audit_sink,
    } = build_backends(&app_config, tenant_resolver.as_ref())
        .await
        .map_err(std::io::Error::other)?;

    HttpServer::new(move || {
        let auth_config = Arc::clone(&auth_config);
        let tenant_resolver: Arc<dyn TenantResolver> = Arc::clone(&tenant_resolver);
        let rate_limiter = Arc::clone(&rate_limiter);
        let audit_for_middleware = Arc::clone(&audit_sink);

        let job_store = Arc::clone(&job_store);

        App::new()
            .app_data(web::Data::from(Arc::clone(&snapshot_repo)))
            .app_data(web::Data::from(Arc::clone(&audit_sink)))
            .app_data(web::Data::from(Arc::clone(&job_store)))
            .wrap(middleware::from_fn(metrics_middleware::metrics_middleware))
            .wrap(Cors::permissive())
            .wrap(TracingMiddleware::new())
            // Wrappers run outermost-last, so this audit layer executes inside
            // the auth layer below and therefore sees the resolved principal.
            .wrap(middleware::from_fn(move |req, next| {
                let sink = Arc::clone(&audit_for_middleware);
                audit_middleware(req, next, sink)
            }))
            .wrap(middleware::from_fn(move |req, next| {
                let auth_config = Arc::clone(&auth_config);
                let tenant_resolver: Arc<dyn TenantResolver> = Arc::clone(&tenant_resolver);
                let rate_limiter = Arc::clone(&rate_limiter);
                auth_middleware(req, next, auth_config, tenant_resolver, rate_limiter)
            }))
            .service(openapi::swagger_ui())
            .service(Files::new("/dashboard", "web").index_file("index.html"))
            .route("/healthz", web::get().to(handlers::healthz))
            .route("/metrics", web::get().to(handlers::metrics))
            .route("/evaluate", web::post().to(handlers::evaluate))
            .route("/simulate", web::post().to(handlers::simulate))
            .route(
                "/simulate/stream",
                web::post().to(streaming_handlers::simulate_stream),
            )
            .route(
                "/ws/simulate",
                web::get().to(websocket_handlers::simulate_ws),
            )
            .route(
                "/snapshots",
                web::post().to(snapshot_handlers::save_snapshot),
            )
            .route(
                "/snapshots",
                web::get().to(snapshot_handlers::list_snapshots),
            )
            .route(
                "/snapshots/{id}",
                web::get().to(snapshot_handlers::load_snapshot),
            )
            .route(
                "/snapshots/{id}",
                web::delete().to(snapshot_handlers::delete_snapshot),
            )
            .route("/audit", web::get().to(audit_handlers::list_audit_events))
            .route("/simulate/jobs", web::post().to(job_handlers::create_job))
            .route("/simulate/jobs/{id}", web::get().to(job_handlers::get_job))
            .route(
                "/simulate/jobs/{id}/cancel",
                web::post().to(job_handlers::cancel_job),
            )
            .route("/ws/jobs/{id}", web::get().to(job_ws_handlers::job_ws))
            // Admin endpoints (protected by CASIROS_ADMIN_KEY).
            .route(
                "/admin/tenants",
                web::get().to(admin_handlers::list_tenants),
            )
            .route(
                "/admin/tenants",
                web::post().to(admin_handlers::provision_tenant),
            )
            .route(
                "/admin/tenants/{id}/stats",
                web::get().to(admin_handlers::tenant_stats),
            )
            .route("/admin/keys", web::post().to(admin_handlers::create_key))
            .route(
                "/admin/keys/{id}/revoke",
                web::post().to(admin_handlers::revoke_key),
            )
    })
    .bind(bind_addr)?
    .run()
    .await
}

/// The persistence backends shared by every worker thread.
struct Backends {
    /// Snapshot persistence.
    snapshot_repo: Arc<SnapshotRepo>,

    /// Audit trail persistence.
    audit_sink: Arc<AuditSink>,
}

/// Builds the snapshot repository and audit log from configuration.
///
/// Both share a single connection pool when the Postgres backend is selected,
/// so the process opens one pool rather than one per subsystem.
async fn build_backends(
    app_config: &AppConfig,
    resolver: &dyn TenantResolver,
) -> Result<Backends, String> {
    if app_config.snapshot.backend.as_str() == "postgres" {
        return build_postgres_backends(&app_config.postgres.url, resolver).await;
    }

    info!("Using in-memory snapshot and audit backends");
    return Ok(Backends {
        snapshot_repo: Arc::new(SnapshotRepo::new(InMemorySnapshotRepository::new())),
        audit_sink: Arc::new(AuditSink::new(InMemoryAuditLog::new())),
    });
}

/// Connects to Postgres, migrates, and provisions the configured tenants.
///
/// The control flow is strictly linear; the cognitive-complexity score comes
/// entirely from the `map_err` closures that turn each `sqlx` failure into a
/// startup diagnostic, so the lint is suppressed rather than the steps split
/// into fragments that would obscure the startup sequence.
#[allow(clippy::cognitive_complexity)]
async fn build_postgres_backends(
    url: &str,
    resolver: &dyn TenantResolver,
) -> Result<Backends, String> {
    let pool = sqlx::postgres::PgPool::connect(url)
        .await
        .map_err(|err| format!("failed to connect to postgres: {err}"))?;

    let repo = PostgresSnapshotRepository::new(pool.clone());
    repo.migrate()
        .await
        .map_err(|err| format!("failed to run migrations: {err}"))?;

    let audit_log = PostgresAuditLog::new(pool);
    provision_known_tenants(&audit_log, resolver).await?;

    info!("Using Postgres snapshot and audit backends");
    return Ok(Backends {
        snapshot_repo: Arc::new(SnapshotRepo::new(repo)),
        audit_sink: Arc::new(AuditSink::new(audit_log)),
    });
}

/// Creates tenant/workspace rows for every principal the resolver knows about.
///
/// Snapshots, jobs, and audit events all carry foreign keys into `tenants` and
/// `workspaces`, so these rows must exist before the first write.
async fn provision_known_tenants(
    audit_log: &PostgresAuditLog,
    resolver: &dyn TenantResolver,
) -> Result<(), String> {
    for principal in resolver.known_principals() {
        audit_log
            .provision_tenant(&principal)
            .await
            .map_err(|err| format!("failed to provision tenant: {err}"))?;
    }
    return Ok(());
}
