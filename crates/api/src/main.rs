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

use actix_web::{App, HttpServer, middleware, web};
use casiros_api::auth::{AuthConfig, RateLimiter, auth_middleware};
use casiros_api::handlers;
use casiros_api::openapi;
use tracing::{info, instrument};

/// Application entry point.
///
/// Initializes structured logging and starts the Actix-Web server.
#[actix_web::main]
#[instrument(name = "main")]
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

    let auth_config = Arc::new(AuthConfig::from_env());
    let rate_limiter = Arc::new(RateLimiter::new());

    HttpServer::new(move || {
        let auth_config = Arc::clone(&auth_config);
        let rate_limiter = Arc::clone(&rate_limiter);

        App::new()
            .wrap(middleware::Logger::default())
            .wrap(middleware::from_fn(move |req, next| {
                let auth_config = Arc::clone(&auth_config);
                let rate_limiter = Arc::clone(&rate_limiter);
                auth_middleware(req, next, auth_config, rate_limiter)
            }))
            .service(openapi::swagger_ui())
            .route("/healthz", web::get().to(handlers::healthz))
            .route("/evaluate", web::post().to(handlers::evaluate))
            .route("/simulate", web::post().to(handlers::simulate))
    })
    .bind(bind_addr)?
    .run()
    .await
}
