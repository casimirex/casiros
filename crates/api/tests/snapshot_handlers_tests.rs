//! Integration tests for snapshot persistence endpoints.

use actix_web::{App, test};
use casiros_api::repositories::{InMemorySnapshotRepository, SnapshotRepo};
use casiros_api::snapshot_handlers;

const FV_REQUEST: &str = r#"{
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
  "id": "fv-demo"
}"#;

#[actix_web::test]
async fn save_and_load_snapshot_round_trips() {
    let repo = SnapshotRepo::new(InMemorySnapshotRepository::new());
    let app = test::init_service(
        App::new()
            .app_data(actix_web::web::Data::new(repo))
            .route(
                "/snapshots",
                actix_web::web::post().to(snapshot_handlers::save_snapshot),
            )
            .route(
                "/snapshots",
                actix_web::web::get().to(snapshot_handlers::list_snapshots),
            )
            .route(
                "/snapshots/{id}",
                actix_web::web::get().to(snapshot_handlers::load_snapshot),
            )
            .route(
                "/snapshots/{id}",
                actix_web::web::delete().to(snapshot_handlers::delete_snapshot),
            ),
    )
    .await;

    let save_request = test::TestRequest::post()
        .uri("/snapshots")
        .set_payload(FV_REQUEST)
        .insert_header(("Content-Type", "application/json"))
        .to_request();
    let save_response = test::call_service(&app, save_request).await;
    assert!(save_response.status().is_success());

    let load_request = test::TestRequest::get()
        .uri("/snapshots/fv-demo")
        .to_request();
    let load_response = test::call_service(&app, load_request).await;
    assert!(load_response.status().is_success());

    let body = test::read_body(load_response).await;
    let text = String::from_utf8(body.to_vec()).expect("response is valid UTF-8");
    assert!(text.contains("fv-demo"));
    assert!(text.contains("future_value"));
}

#[actix_web::test]
async fn list_snapshots_includes_saved_id() {
    let repo = SnapshotRepo::new(InMemorySnapshotRepository::new());
    let app = test::init_service(
        App::new()
            .app_data(actix_web::web::Data::new(repo))
            .route(
                "/snapshots",
                actix_web::web::post().to(snapshot_handlers::save_snapshot),
            )
            .route(
                "/snapshots",
                actix_web::web::get().to(snapshot_handlers::list_snapshots),
            )
            .route(
                "/snapshots/{id}",
                actix_web::web::get().to(snapshot_handlers::load_snapshot),
            )
            .route(
                "/snapshots/{id}",
                actix_web::web::delete().to(snapshot_handlers::delete_snapshot),
            ),
    )
    .await;

    let save_request = test::TestRequest::post()
        .uri("/snapshots")
        .set_payload(FV_REQUEST)
        .insert_header(("Content-Type", "application/json"))
        .to_request();
    test::call_service(&app, save_request).await;

    let list_request = test::TestRequest::get().uri("/snapshots").to_request();
    let list_response = test::call_service(&app, list_request).await;
    assert!(list_response.status().is_success());

    let body = test::read_body(list_response).await;
    let text = String::from_utf8(body.to_vec()).expect("response is valid UTF-8");
    assert!(text.contains("fv-demo"));
}

#[actix_web::test]
async fn delete_snapshot_removes_it() {
    let repo = SnapshotRepo::new(InMemorySnapshotRepository::new());
    let app = test::init_service(
        App::new()
            .app_data(actix_web::web::Data::new(repo))
            .route(
                "/snapshots",
                actix_web::web::post().to(snapshot_handlers::save_snapshot),
            )
            .route(
                "/snapshots",
                actix_web::web::get().to(snapshot_handlers::list_snapshots),
            )
            .route(
                "/snapshots/{id}",
                actix_web::web::get().to(snapshot_handlers::load_snapshot),
            )
            .route(
                "/snapshots/{id}",
                actix_web::web::delete().to(snapshot_handlers::delete_snapshot),
            ),
    )
    .await;

    let save_request = test::TestRequest::post()
        .uri("/snapshots")
        .set_payload(FV_REQUEST)
        .insert_header(("Content-Type", "application/json"))
        .to_request();
    test::call_service(&app, save_request).await;

    let delete_request = test::TestRequest::delete()
        .uri("/snapshots/fv-demo")
        .to_request();
    let delete_response = test::call_service(&app, delete_request).await;
    assert!(delete_response.status().is_success());

    let load_request = test::TestRequest::get()
        .uri("/snapshots/fv-demo")
        .to_request();
    let load_response = test::call_service(&app, load_request).await;
    assert_eq!(
        load_response.status(),
        actix_web::http::StatusCode::NOT_FOUND
    );
}

#[actix_web::test]
async fn missing_snapshot_returns_not_found() {
    let repo = SnapshotRepo::new(InMemorySnapshotRepository::new());
    let app = test::init_service(
        App::new()
            .app_data(actix_web::web::Data::new(repo))
            .route(
                "/snapshots",
                actix_web::web::post().to(snapshot_handlers::save_snapshot),
            )
            .route(
                "/snapshots",
                actix_web::web::get().to(snapshot_handlers::list_snapshots),
            )
            .route(
                "/snapshots/{id}",
                actix_web::web::get().to(snapshot_handlers::load_snapshot),
            )
            .route(
                "/snapshots/{id}",
                actix_web::web::delete().to(snapshot_handlers::delete_snapshot),
            ),
    )
    .await;

    let request = test::TestRequest::get()
        .uri("/snapshots/does-not-exist")
        .to_request();
    let response = test::call_service(&app, request).await;
    assert_eq!(response.status(), actix_web::http::StatusCode::NOT_FOUND);
}
