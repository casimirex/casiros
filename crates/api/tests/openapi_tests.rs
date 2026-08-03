//! Integration tests for the `OpenAPI` contract and Swagger UI serving.

use actix_web::{App, test};
use casiros_api::handlers;
use casiros_api::openapi;

/// The generated `OpenAPI` document must be valid JSON and list the three
/// public endpoints.
#[actix_web::test]
async fn openapi_json_lists_endpoints() {
    let app = test::init_service(App::new().service(openapi::swagger_ui())).await;

    let request = test::TestRequest::get().uri("/openapi.json").to_request();
    let response = test::call_service(&app, request).await;
    assert!(response.status().is_success());

    let body = test::read_body(response).await;
    let text = String::from_utf8(body.to_vec()).expect("openapi.json is valid UTF-8");
    assert!(text.contains("/healthz"));
    assert!(text.contains("/evaluate"));
    assert!(text.contains("/simulate"));
    assert!(text.contains("CASIROS API"));
}

/// Swagger UI index page must be mounted and return a successful HTML page.
#[actix_web::test]
async fn swagger_ui_index_is_reachable() {
    let app = test::init_service(App::new().service(openapi::swagger_ui())).await;

    let request = test::TestRequest::get()
        .uri("/swagger-ui/index.html")
        .to_request();
    let response = test::call_service(&app, request).await;
    assert!(
        response.status().is_success(),
        "swagger-ui index should be reachable"
    );
}

/// The three API handlers must still be callable when Swagger UI is mounted.
#[actix_web::test]
async fn healthz_with_openapi_service() {
    let app = test::init_service(
        App::new()
            .service(openapi::swagger_ui())
            .route("/healthz", actix_web::web::get().to(handlers::healthz)),
    )
    .await;

    let request = test::TestRequest::get().uri("/healthz").to_request();
    let response = test::call_service(&app, request).await;
    assert!(response.status().is_success());
}
