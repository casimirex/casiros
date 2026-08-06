//! Integration tests for API key authentication and tenant-scoped rate limiting.

use std::collections::HashMap;
use std::sync::Arc;

use actix_web::{App, test};
use casiros_api::auth::{AuthConfig, RateLimiter, auth_middleware};
use casiros_api::handlers;
use casiros_api::openapi;
use casiros_api::repositories::{InMemorySnapshotRepository, SnapshotRepo};
use casiros_api::snapshot_handlers;
use casiros_api::tenant::InMemoryTenantResolver;
use casiros_core::tenant::{Principal, TenantId, WorkspaceId};

fn auth_config_with_key(key: &str) -> Arc<AuthConfig> {
    Arc::new(AuthConfig::with_keys(
        std::collections::HashSet::from([key.to_string()]),
        2,
    ))
}

fn default_resolver() -> Arc<dyn casiros_api::tenant::TenantResolver> {
    Arc::new(InMemoryTenantResolver::default_for_any_key())
}

fn isolated_resolver() -> Arc<dyn casiros_api::tenant::TenantResolver> {
    let mut mapping = HashMap::new();
    mapping.insert(
        "key_a".to_string(),
        Principal::new(
            TenantId::new("tenant_a").unwrap(),
            WorkspaceId::new("workspace_a").unwrap(),
            "key_a",
        ),
    );
    mapping.insert(
        "key_b".to_string(),
        Principal::new(
            TenantId::new("tenant_b").unwrap(),
            WorkspaceId::new("workspace_b").unwrap(),
            "key_b",
        ),
    );
    Arc::new(InMemoryTenantResolver::new(mapping))
}

/// Public paths must remain accessible without an API key.
#[actix_web::test]
async fn public_paths_skip_auth() {
    let config = Arc::new(AuthConfig::default());
    let resolver = default_resolver();
    let limiter = Arc::new(RateLimiter::new());

    let app = test::init_service(
        App::new()
            .wrap(actix_web::middleware::from_fn(move |req, next| {
                let config = Arc::clone(&config);
                let resolver: Arc<dyn casiros_api::tenant::TenantResolver> = Arc::clone(&resolver);
                let limiter = Arc::clone(&limiter);
                auth_middleware(req, next, config, resolver, limiter)
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
    let resolver = default_resolver();
    let limiter = Arc::new(RateLimiter::new());

    let app = test::init_service(
        App::new()
            .wrap(actix_web::middleware::from_fn(move |req, next| {
                let config = Arc::clone(&config);
                let resolver: Arc<dyn casiros_api::tenant::TenantResolver> = Arc::clone(&resolver);
                let limiter = Arc::clone(&limiter);
                auth_middleware(req, next, config, resolver, limiter)
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
    let resolver = default_resolver();
    let limiter = Arc::new(RateLimiter::new());

    let app = test::init_service(
        App::new()
            .wrap(actix_web::middleware::from_fn(move |req, next| {
                let config = Arc::clone(&config);
                let resolver: Arc<dyn casiros_api::tenant::TenantResolver> = Arc::clone(&resolver);
                let limiter = Arc::clone(&limiter);
                auth_middleware(req, next, config, resolver, limiter)
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
    let resolver = default_resolver();
    let limiter = Arc::new(RateLimiter::new());

    let app = test::init_service(
        App::new()
            .wrap(actix_web::middleware::from_fn(move |req, next| {
                let config = Arc::clone(&config);
                let resolver: Arc<dyn casiros_api::tenant::TenantResolver> = Arc::clone(&resolver);
                let limiter = Arc::clone(&limiter);
                auth_middleware(req, next, config, resolver, limiter)
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
    let resolver = default_resolver();
    let limiter = Arc::new(RateLimiter::new());

    let app = test::init_service(
        App::new()
            .wrap(actix_web::middleware::from_fn(move |req, next| {
                let config = Arc::clone(&config);
                let resolver: Arc<dyn casiros_api::tenant::TenantResolver> = Arc::clone(&resolver);
                let limiter = Arc::clone(&limiter);
                auth_middleware(req, next, config, resolver, limiter)
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

/// Snapshots saved by one tenant cannot be read by another tenant.
#[actix_web::test]
async fn tenant_isolation_prevents_cross_tenant_snapshot_access() {
    let config = Arc::new(AuthConfig::with_keys(
        std::collections::HashSet::from(["key_a".to_string(), "key_b".to_string()]),
        100,
    ));
    let resolver = isolated_resolver();
    let limiter = Arc::new(RateLimiter::new());
    let repo = SnapshotRepo::new(InMemorySnapshotRepository::new());

    let app = test::init_service(
        App::new()
            .app_data(actix_web::web::Data::new(repo))
            .wrap(actix_web::middleware::from_fn(move |req, next| {
                let config = Arc::clone(&config);
                let resolver: Arc<dyn casiros_api::tenant::TenantResolver> = Arc::clone(&resolver);
                let limiter = Arc::clone(&limiter);
                auth_middleware(req, next, config, resolver, limiter)
            }))
            .route(
                "/snapshots",
                actix_web::web::post().to(snapshot_handlers::save_snapshot),
            )
            .route(
                "/snapshots/{id}",
                actix_web::web::get().to(snapshot_handlers::load_snapshot),
            ),
    )
    .await;

    let payload = r#"{
        "id": "isolated",
        "nodes": [{"input": {"name": "x"}}],
        "edges": []
    }"#;

    let save_request = test::TestRequest::post()
        .uri("/snapshots")
        .insert_header(("X-API-Key", "key_a"))
        .insert_header(("Content-Type", "application/json"))
        .set_payload(payload)
        .to_request();
    let save_response = test::call_service(&app, save_request).await;
    assert!(save_response.status().is_success());

    let load_a = test::TestRequest::get()
        .uri("/snapshots/isolated")
        .insert_header(("X-API-Key", "key_a"))
        .to_request();
    let response_a = test::call_service(&app, load_a).await;
    assert!(response_a.status().is_success());

    let load_b = test::TestRequest::get()
        .uri("/snapshots/isolated")
        .insert_header(("X-API-Key", "key_b"))
        .to_request();
    let response_b = test::call_service(&app, load_b).await;
    assert_eq!(response_b.status(), 404);
}
