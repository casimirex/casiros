//! Integration tests for the typed CASIROS API client.

use std::time::Duration;

use actix_web::App;
use casiros_api::handlers;
use casiros_api::models::{
    EvaluateRequest, HealthzResponse, SaveSnapshotRequest, SnapshotListResponse, SnapshotResponse,
};
use casiros_api::snapshot_handlers;
use casiros_api_client::CasirosClient;

/// Starts a local test server and returns its base URL.
async fn test_server() -> String {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("free port");
    let port = listener.local_addr().unwrap().port();

    let server = actix_web::HttpServer::new(move || {
        let repo = casiros_api::repositories::SnapshotRepo::new(
            casiros_api::repositories::InMemorySnapshotRepository::new(),
        );

        App::new()
            .app_data(actix_web::web::Data::new(repo))
            .route("/healthz", actix_web::web::get().to(handlers::healthz))
            .route("/evaluate", actix_web::web::post().to(handlers::evaluate))
            .route("/simulate", actix_web::web::post().to(handlers::simulate))
            .route(
                "/snapshots",
                actix_web::web::post().to(snapshot_handlers::save_snapshot),
            )
            .route(
                "/snapshots/{id}",
                actix_web::web::get().to(snapshot_handlers::load_snapshot),
            )
            .route(
                "/snapshots/{id}",
                actix_web::web::delete().to(snapshot_handlers::delete_snapshot),
            )
            .route(
                "/snapshots",
                actix_web::web::get().to(snapshot_handlers::list_snapshots),
            )
    })
    .listen(listener)
    .unwrap()
    .run();

    tokio::spawn(server);

    // Give the server a moment to start accepting connections.
    tokio::time::sleep(Duration::from_millis(50)).await;

    format!("http://127.0.0.1:{port}")
}

/// The client can call `GET /healthz` and decode the response.
#[tokio::test]
async fn client_can_call_healthz() {
    let base_url = test_server().await;
    let client = CasirosClient::new(&base_url).expect("valid url");

    let response: HealthzResponse = client.healthz().await.expect("healthz succeeds");
    assert_eq!(response.status, "ok");
}

/// The client reports an API error when the server returns a bad request.
#[tokio::test]
async fn client_reports_api_error() {
    let base_url = test_server().await;
    let client = CasirosClient::new(&base_url).expect("valid url");

    let request = EvaluateRequest {
        nodes: (0..101)
            .map(|i| casiros_api::models::NodeRequest::Input {
                name: format!("x_{i}"),
            })
            .collect(),
        edges: vec![],
        inputs: std::collections::HashMap::new(),
    };

    let err = client
        .evaluate(&request)
        .await
        .expect_err("too many nodes is invalid");
    assert!(err.to_string().contains("API error"));
    assert!(err.to_string().contains("Too many nodes"));
}

/// The client can save, load, list, and delete snapshots using the in-memory repo.
#[tokio::test]
async fn client_snapshot_round_trips() {
    let base_url = test_server().await;
    let client = CasirosClient::new(&base_url).expect("valid url");

    let request = SaveSnapshotRequest {
        id: "client-round-trip".to_string(),
        nodes: vec![casiros_api::models::NodeRequest::Input {
            name: "x".to_string(),
        }],
        edges: vec![],
    };

    let save_response = client.save_snapshot(&request).await.expect("save succeeds");
    assert_eq!(save_response.id, "client-round-trip");

    let loaded: SnapshotResponse = client
        .load_snapshot("client-round-trip")
        .await
        .expect("load succeeds");
    assert_eq!(loaded.id, "client-round-trip");

    let listed: SnapshotListResponse = client.list_snapshots().await.expect("list succeeds");
    assert!(listed.snapshots.iter().any(|s| s.id == "client-round-trip"));

    client
        .delete_snapshot("client-round-trip")
        .await
        .expect("delete succeeds");
}
