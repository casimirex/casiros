//! Integration tests for API key authentication and rate limiting.

use std::sync::Arc;

use actix_web::{App, test};
use casiros_api::auth::{AuthConfig, RateLimiter, auth_middleware};
use casiros_api::handlers;
use casiros_api::openapi;

fn auth_config_with_key(key: &str) -> Arc<AuthConfig> {
    Arc::new(AuthConfig::with_keys(
        std::collections::HashSet::from([key.to_string()]),
        2,
    ))
}

/// Public paths must remain accessible without an API key.
#[actix_web::test]
async fn public_paths_skip_auth() {
    let config = Arc::new(AuthConfig::default());
    let limiter = Arc::new(RateLimiter::new());

    let app = test::init_service(
        App::new()
            .wrap(actix_web::middleware::from_fn(move |req, next| {
                auth_middleware(req, next, Arc::clone(&config), Arc::clone(&limiter))
            }))
            .service(openapi::swagger_ui())
            .route("/healthz", actix_web::web::get().to(handlers::healthz)),
    )
    .await;

    let request = test::TestRequest::get().uri("/healthz").to_request();
    let response = test::call_service(&app, request).await;
    assert!(response.status().is_success());
}

/// A missing key on a protected path returns 401.
#[actix_web::test]
async fn missing_key_returns_unauthorized() {
    let config = auth_config_with_key("secret");
    let limiter = Arc::new(RateLimiter::new());

    let app = test::init_service(
        App::new()
            .wrap(actix_web::middleware::from_fn(move |req, next| {
                auth_middleware(req, next, Arc::clone(&config), Arc::clone(&limiter))
            }))
            .route("/evaluate", actix_web::web::post().to(handlers::evaluate)),
    )
    .await;

    let request = test::TestRequest::post().uri("/evaluate").to_request();
    let response = test::call_service(&app, request).await;
    assert_eq!(response.status(), 401);
}

/// An invalid key on a protected path returns 401.
#[actix_web::test]
async fn invalid_key_returns_unauthorized() {
    let config = auth_config_with_key("secret");
    let limiter = Arc::new(RateLimiter::new());

    let app = test::init_service(
        App::new()
            .wrap(actix_web::middleware::from_fn(move |req, next| {
                auth_middleware(req, next, Arc::clone(&config), Arc::clone(&limiter))
            }))
            .route("/evaluate", actix_web::web::post().to(handlers::evaluate)),
    )
    .await;

    let request = test::TestRequest::post()
        .uri("/evaluate")
        .insert_header(("X-API-Key", "wrong"))
        .to_request();
    let response = test::call_service(&app, request).await;
    assert_eq!(response.status(), 401);
}

/// A valid key allows access to a protected path.
#[actix_web::test]
async fn valid_key_allows_access() {
    let config = auth_config_with_key("secret");
    let limiter = Arc::new(RateLimiter::new());

    let app = test::init_service(
        App::new()
            .wrap(actix_web::middleware::from_fn(move |req, next| {
                auth_middleware(req, next, Arc::clone(&config), Arc::clone(&limiter))
            }))
            .route("/evaluate", actix_web::web::post().to(handlers::evaluate)),
    )
    .await;

    let request = test::TestRequest::post()
        .uri("/evaluate")
        .insert_header(("X-API-Key", "secret"))
        .to_request();
    let response = test::call_service(&app, request).await;
    assert_eq!(response.status(), 400); // Bad request, but passed auth.
}

/// Exceeding the per-minute rate limit returns 429.
#[actix_web::test]
async fn rate_limit_returns_too_many_requests() {
    let config = auth_config_with_key("secret");
    let limiter = Arc::new(RateLimiter::new());

    let app = test::init_service(
        App::new()
            .wrap(actix_web::middleware::from_fn(move |req, next| {
                auth_middleware(req, next, Arc::clone(&config), Arc::clone(&limiter))
            }))
            .route("/evaluate", actix_web::web::post().to(handlers::evaluate)),
    )
    .await;

    for _ in 0..2 {
        let request = test::TestRequest::post()
            .uri("/evaluate")
            .insert_header(("X-API-Key", "secret"))
            .to_request();
        let response = test::call_service(&app, request).await;
        assert_eq!(response.status(), 400);
    }

    let request = test::TestRequest::post()
        .uri("/evaluate")
        .insert_header(("X-API-Key", "secret"))
        .to_request();
    let response = test::call_service(&app, request).await;
    assert_eq!(response.status(), 429);
}
