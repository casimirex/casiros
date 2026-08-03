//! Benchmark the `/ws/simulate` WebSocket streaming endpoint.
#![allow(missing_docs)]

use std::net::TcpListener;
use std::time::{Duration, Instant};

use actix_web::{App, HttpServer, web};
use criterion::{Criterion, criterion_group, criterion_main};
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use tokio_tungstenite::connect_async;

use casiros_api::websocket_handlers;

const UNIVERSE_COUNTS: [usize; 2] = [10_000, 100_000];

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

fn build_request(universe_count: usize) -> serde_json::Value {
    json!({
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
        "universe_count": universe_count,
        "seed": 42
    })
}

async fn run_once(url: &str, request: &serde_json::Value) -> Duration {
    let (mut socket, _) = connect_async(url).await.expect("failed to connect");
    socket
        .send(tokio_tungstenite::tungstenite::Message::Text(
            request.to_string().into(),
        ))
        .await
        .expect("failed to send");

    let start = Instant::now();
    loop {
        let msg = tokio::time::timeout(Duration::from_secs(60), socket.next())
            .await
            .expect("timeout waiting for frame")
            .expect("stream closed")
            .expect("websocket error");

        let text = match msg {
            tokio_tungstenite::tungstenite::Message::Text(text) => text,
            tokio_tungstenite::tungstenite::Message::Close(_) => break,
            _ => continue,
        };

        let event: serde_json::Value = serde_json::from_str(&text).unwrap();
        if event["type"] == "result" || event["type"] == "error" {
            break;
        }
    }
    let _ = socket.close(None).await;
    start.elapsed()
}

fn bench_websocket_simulate(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
    let url = rt.block_on(async { spawn_test_server() });

    for count in UNIVERSE_COUNTS {
        let request = build_request(count);
        c.bench_function(&format!("websocket_simulate_{count}_universes"), |b| {
            b.iter_custom(|iters| {
                let mut total = Duration::ZERO;
                for _ in 0..iters {
                    total += rt.block_on(run_once(&url, &request));
                }
                total
            });
        });
    }
}

criterion_group!(benches, bench_websocket_simulate);
criterion_main!(benches);
