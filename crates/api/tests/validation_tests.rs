//! Tests for API request validation and security limits.

use actix_web::{App, test, web};
use casiros_api::handlers;
use serde_json::json;

#[actix_web::test]
async fn evaluate_rejects_too_many_nodes() {
    let app =
        test::init_service(App::new().route("/evaluate", web::post().to(handlers::evaluate))).await;

    let nodes: Vec<serde_json::Value> = (0..=100)
        .map(|i| json!({ "input": { "name": format!("x{i}") } }))
        .collect();
    let payload = json!({ "nodes": nodes, "edges": [], "inputs": {} });

    let req = test::TestRequest::post()
        .uri("/evaluate")
        .set_json(&payload)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400);
}

#[actix_web::test]
async fn simulate_rejects_too_many_universes() {
    let app =
        test::init_service(App::new().route("/simulate", web::post().to(handlers::simulate))).await;

    let payload = json!({
        "nodes": [{ "input": { "name": "x" } }],
        "edges": [],
        "bindings": [
            { "node": "x", "distribution": { "kind": "fixed", "value": 1.0 } }
        ],
        "target": "x",
        "universe_count": 100_001
    });

    let req = test::TestRequest::post()
        .uri("/simulate")
        .set_json(&payload)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400);
}

#[actix_web::test]
async fn evaluate_rejects_excessive_depth() {
    let app =
        test::init_service(App::new().route("/evaluate", web::post().to(handlers::evaluate))).await;

    let mut nodes = vec![json!({ "input": { "name": "x" } })];
    let mut edges = Vec::new();

    let mut previous = String::from("x");
    for i in 0..55 {
        let name = format!("f{i}");
        nodes.push(json!({
            "formula": {
                "name": &name,
                "kind": {
                    "formula": "future_value",
                    "present_value": { "node": &previous },
                    "rate": { "node": "x" },
                    "periods": 1
                }
            }
        }));
        edges.push(json!({ "dependency": &previous, "dependent": &name }));
        edges.push(json!({ "dependency": "x", "dependent": &name }));
        previous = name;
    }

    let payload = json!({
        "nodes": nodes,
        "edges": edges,
        "inputs": { "x": "1.0" }
    });

    let req = test::TestRequest::post()
        .uri("/evaluate")
        .set_json(&payload)
        .to_request();
    let resp = test::call_service(&app, req).await;
    assert_eq!(resp.status(), 400);
}
