//! Integration tests for the WebSocket `/ws/simulate` endpoint.

use std::net::TcpListener;

use actix_web::{App, HttpServer, web};
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use tokio_tungstenite::connect_async;

use casiros_api::websocket_handlers;

/// Starts the API on a random local port and returns its bound address.
fn spawn_test_server() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("failed to bind");
    let port = listener.local_addr().unwrap().port();

    let server = HttpServer::new(move || {
        App::new().route(
            "/ws/simulate",
            web::get().to(websocket_handlers::simulate_ws),
        )
    })
    .listen(listener)
    .expect("failed to listen")
    .run();

    tokio::spawn(server);
    format!("ws://127.0.0.1:{port}/ws/simulate")
}

#[tokio::test]
async fn ws_simulate_streams_progress_and_result() {
    let url = spawn_test_server();

    let payload = json!({
        "nodes": [
            { "input": { "name": "x" } },
            {
                "formula": {
                    "name": "doubled",
                    "kind": {
                        "formula": "future_value",
                        "present_value": { "node": "x" },
                        "rate": 0,
                        "periods": 1
                    }
                }
            }
        ],
        "edges": [{ "dependency": "x", "dependent": "doubled" }],
        "bindings": [
            { "node": "x", "distribution": { "kind": "fixed", "value": 42.0 } }
        ],
        "target": "doubled",
        "universe_count": 100,
        "seed": 42
    });

    let (mut socket, _response) = connect_async(&url)
        .await
        .expect("failed to connect websocket");

    socket
        .send(tokio_tungstenite::tungstenite::Message::Text(
            payload.to_string().into(),
        ))
        .await
        .expect("failed to send request frame");

    let mut saw_result = false;

    let timeout = std::time::Duration::from_secs(10);
    let deadline = tokio::time::Instant::now() + timeout;

    while tokio::time::Instant::now() < deadline {
        let msg = tokio::time::timeout(std::time::Duration::from_millis(500), socket.next()).await;

        if msg.is_err() {
            continue;
        }

        let Some(Ok(item)) = msg.unwrap() else {
            continue;
        };

        let text = match item {
            tokio_tungstenite::tungstenite::Message::Text(text) => text,
            tokio_tungstenite::tungstenite::Message::Close(_) => break,
            _ => continue,
        };

        let event: serde_json::Value = serde_json::from_str(&text).unwrap();
        match event["type"].as_str() {
            Some("progress") => {
                assert_eq!(event["total"], 100);
            }
            Some("result") => {
                saw_result = true;
                assert_eq!(event["result"]["count"], 100);
                break;
            }
            Some("error") => panic!("unexpected error frame: {event}"),
            _ => {}
        }
    }

    let _ = socket.close(None).await;
    assert!(saw_result, "did not receive result frame");
}
