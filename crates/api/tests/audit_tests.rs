//! Integration tests for the audit trail middleware and `GET /audit`.

use std::sync::Arc;

use actix_web::{App, test, web};
use casiros_api::audit::{AuditSink, InMemoryAuditLog};
use casiros_api::audit_handlers;
use casiros_api::audit_middleware::audit_middleware;
use casiros_api::auth::{AuthConfig, RateLimiter, auth_middleware};
use casiros_api::handlers;
use casiros_api::models::AuditListResponse;
use casiros_api::repositories::{InMemorySnapshotRepository, SnapshotRepo};
use casiros_api::snapshot_handlers;
use casiros_api::tenant::{InMemoryTenantResolver, TenantResolver};

/// Builds a resolver that maps two keys onto two distinct tenants.
fn isolated_resolver() -> Arc<dyn TenantResolver> {
    Arc::new(InMemoryTenantResolver::parse(Some(
        "key_a:tenant_a:workspace_a,key_b:tenant_b:workspace_b",
    )))
}

/// Assembles an app with auth and audit middleware in production order.
macro_rules! audited_app {
    ($sink:expr) => {{
        let config = Arc::new(AuthConfig::with_keys(
            std::collections::HashSet::from(["key_a".to_string(), "key_b".to_string()]),
            1_000,
        ));
        let resolver = isolated_resolver();
        let limiter = Arc::new(RateLimiter::new());
        let sink_for_middleware = Arc::clone(&$sink);
        let sink_for_data = Arc::clone(&$sink);

        test::init_service(
            App::new()
                .app_data(web::Data::from(sink_for_data))
                .app_data(web::Data::new(SnapshotRepo::new(
                    InMemorySnapshotRepository::new(),
                )))
                .wrap(actix_web::middleware::from_fn(move |req, next| {
                    let sink = Arc::clone(&sink_for_middleware);
                    audit_middleware(req, next, sink)
                }))
                .wrap(actix_web::middleware::from_fn(move |req, next| {
                    let config = Arc::clone(&config);
                    let resolver: Arc<dyn TenantResolver> = Arc::clone(&resolver);
                    let limiter = Arc::clone(&limiter);
                    auth_middleware(req, next, config, resolver, limiter)
                }))
                .route("/healthz", web::get().to(handlers::healthz))
                .route("/evaluate", web::post().to(handlers::evaluate))
                .route(
                    "/snapshots",
                    web::get().to(snapshot_handlers::list_snapshots),
                )
                .route("/audit", web::get().to(audit_handlers::list_audit_events)),
        )
        .await
    }};
}

/// A successful request must leave exactly one audit event.
#[actix_web::test]
async fn successful_request_leaves_an_event() {
    let log = InMemoryAuditLog::new();
    let sink = Arc::new(AuditSink::new(log.clone()));
    let app = audited_app!(sink);

    let request = test::TestRequest::get()
        .uri("/snapshots")
        .insert_header(("X-API-Key", "key_a"))
        .to_request();
    let response = test::call_service(&app, request).await;

    assert!(response.status().is_success());
    assert_eq!(log.len(), 1);
}

/// A failing request must still be recorded, with a non-success result.
#[actix_web::test]
async fn failed_request_is_still_audited() {
    let log = InMemoryAuditLog::new();
    let sink = Arc::new(AuditSink::new(log.clone()));
    let app = audited_app!(sink);

    // An empty body fails validation, producing a 400.
    let request = test::TestRequest::post()
        .uri("/evaluate")
        .insert_header(("X-API-Key", "key_a"))
        .to_request();
    let response = test::call_service(&app, request).await;

    assert_eq!(response.status(), 400);
    assert_eq!(log.len(), 1);
}

/// Health checks carry no principal and must not pollute the trail.
#[actix_web::test]
async fn health_checks_are_not_audited() {
    let log = InMemoryAuditLog::new();
    let sink = Arc::new(AuditSink::new(log.clone()));
    let app = audited_app!(sink);

    let request = test::TestRequest::get().uri("/healthz").to_request();
    let response = test::call_service(&app, request).await;

    assert!(response.status().is_success());
    assert!(log.is_empty());
}

/// `GET /audit` must return only the calling tenant's events.
#[actix_web::test]
async fn audit_endpoint_is_tenant_scoped() {
    let log = InMemoryAuditLog::new();
    let sink = Arc::new(AuditSink::new(log.clone()));
    let app = audited_app!(sink);

    // Tenant A and tenant B each perform one snapshot listing.
    for key in ["key_a", "key_b"] {
        let request = test::TestRequest::get()
            .uri("/snapshots")
            .insert_header(("X-API-Key", key))
            .to_request();
        let response = test::call_service(&app, request).await;
        assert!(response.status().is_success());
    }

    let request = test::TestRequest::get()
        .uri("/audit")
        .insert_header(("X-API-Key", "key_a"))
        .to_request();
    let body: AuditListResponse = test::call_and_read_body_json(&app, request).await;

    // Tenant A sees its own snapshot listing but never tenant B's.
    assert!(body.events.iter().all(|e| e.tenant_id == "tenant_a"));
    assert!(!body.events.is_empty());
}

/// Recorded events must carry the HTTP method and status as metadata.
#[actix_web::test]
async fn events_capture_method_and_status() {
    let log = InMemoryAuditLog::new();
    let sink = Arc::new(AuditSink::new(log.clone()));
    let app = audited_app!(sink);

    let request = test::TestRequest::get()
        .uri("/snapshots")
        .insert_header(("X-API-Key", "key_a"))
        .to_request();
    test::call_service(&app, request).await;

    let request = test::TestRequest::get()
        .uri("/audit")
        .insert_header(("X-API-Key", "key_a"))
        .to_request();
    let body: AuditListResponse = test::call_and_read_body_json(&app, request).await;

    let listing = body
        .events
        .iter()
        .find(|e| e.resource == "/snapshots")
        .expect("the snapshot listing must be audited");
    assert_eq!(
        listing.metadata.get("method").map(String::as_str),
        Some("GET")
    );
    assert_eq!(
        listing.metadata.get("status").map(String::as_str),
        Some("200")
    );
}
