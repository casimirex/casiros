//! Integration tests for the CASIROS HTTP API.

use actix_web::{App, test, web};
use casiros_api::handlers;
use rust_decimal_macros::dec;
use serde_json::json;

#[actix_web::test]
async fn healthz_returns_ok() {
    let app =
        test::init_service(App::new().route("/healthz", web::get().to(handlers::healthz))).await;
    let req = test::TestRequest::get().uri("/healthz").to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());
}

#[actix_web::test]
async fn evaluate_future_value() {
    let app =
        test::init_service(App::new().route("/evaluate", web::post().to(handlers::evaluate))).await;

    let payload = json!({
        "nodes": [
            { "input": { "name": "principal" } },
            { "input": { "name": "rate" } },
            { "formula": {
                "name": "fv",
                "kind": {
                    "formula": "future_value",
                    "present_value": { "node": "principal" },
                    "rate": { "node": "rate" },
                    "periods": 10
                }
            }}
        ],
        "edges": [
            { "dependency": "principal", "dependent": "fv" },
            { "dependency": "rate", "dependent": "fv" }
        ],
        "inputs": {
            "principal": "100.0",
            "rate": "0.05"
        }
    });

    let req = test::TestRequest::post()
        .uri("/evaluate")
        .set_json(&payload)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert!(resp.status().is_success());

    let body: serde_json::Value = test::read_body_json(resp).await;
    let fv = body["outputs"]["fv"]
        .as_str()
        .unwrap()
        .parse::<rust_decimal::Decimal>()
        .unwrap();
    assert_eq!(fv.round_dp(4), dec!(162.8895));
}
