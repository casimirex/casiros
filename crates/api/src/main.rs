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
use casiros_api::auth::{AuthConfig, RateLimiter, auth_middleware};
use casiros_api::config::AppConfig;
use casiros_api::handlers;
use casiros_api::openapi;
use casiros_api::repositories::{
    InMemorySnapshotRepository, PostgresSnapshotRepository, SnapshotRepo,
};
use casiros_api::snapshot_handlers;
use casiros_api::streaming_handlers;
use casiros_api::tracing_middleware::TracingMiddleware;
use casiros_api::websocket_handlers;
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

    info!("CASIROS API starting on {}", bind_addr);

    let auth_config = Arc::new(AuthConfig::from_env());
    let rate_limiter = Arc::new(RateLimiter::new());
    let snapshot_repo = build_snapshot_repo(&app_config)
        .await
        .map_err(std::io::Error::other)?;

    HttpServer::new(move || {
        let auth_config = Arc::clone(&auth_config);
        let rate_limiter = Arc::clone(&rate_limiter);

        App::new()
            .app_data(web::Data::from(Arc::clone(&snapshot_repo)))
            .wrap(Cors::permissive())
            .wrap(TracingMiddleware::new())
            .wrap(middleware::from_fn(move |req, next| {
                let auth_config = Arc::clone(&auth_config);
                let rate_limiter = Arc::clone(&rate_limiter);
                auth_middleware(req, next, auth_config, rate_limiter)
            }))
            .service(openapi::swagger_ui())
            .service(Files::new("/dashboard", "web").index_file("index.html"))
            .route("/healthz", web::get().to(handlers::healthz))
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
    })
    .bind(bind_addr)?
    .run()
    .await
}

async fn build_snapshot_repo(app_config: &AppConfig) -> Result<Arc<SnapshotRepo>, String> {
    return match app_config.snapshot.backend.as_str() {
        "postgres" => {
            let pool = sqlx::postgres::PgPool::connect(&app_config.postgres.url)
                .await
                .map_err(|err| format!("failed to connect to postgres: {err}"))?;
            let repo = PostgresSnapshotRepository::new(pool);
            repo.migrate()
                .await
                .map_err(|err| format!("failed to run migrations: {err}"))?;
            Ok(Arc::new(SnapshotRepo::new(repo)))
        }
        _ => Ok(Arc::new(SnapshotRepo::new(
            InMemorySnapshotRepository::new(),
        ))),
    };
}
