//! Smoke tests for the tracing middleware.

use actix_web::{App, test, web};
use casiros_api::tracing_middleware::TracingMiddleware;

async fn ok_handler() -> &'static str {
    "ok"
}

#[actix_web::test]
async fn tracing_middleware_does_not_break_request() {
    let app = test::init_service(
        App::new()
            .wrap(TracingMiddleware::new())
            .route("/ping", web::get().to(ok_handler)),
    )
    .await;

    let req = test::TestRequest::get().uri("/ping").to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let body = test::read_body(resp).await;
    assert_eq!(body, actix_web::web::Bytes::from_static(b"ok"));
}
