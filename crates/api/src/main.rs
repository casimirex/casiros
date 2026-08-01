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

use actix_web::{App, HttpResponse, HttpServer, Responder, middleware, web};
use tracing::{info, instrument};

/// Health check endpoint for liveness and readiness probes.
#[instrument(name = "healthz")]
async fn healthz() -> impl Responder {
    info!("Health check requested");
    return HttpResponse::Ok().json(serde_json::json!({ "status": "ok" }));
}

/// Application entry point.
///
/// Initializes structured logging and starts the Actix-Web server.
#[actix_web::main]
async fn main() -> std::io::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let bind_addr =
        std::env::var("CASIROS_BIND_ADDR").unwrap_or_else(|_| "127.0.0.1:8080".to_string());

    info!("CASIROS API starting on {}", bind_addr);

    HttpServer::new(|| {
        App::new()
            .wrap(middleware::Logger::default())
            .route("/healthz", web::get().to(healthz))
    })
    .bind(bind_addr)?
    .run()
    .await
}
