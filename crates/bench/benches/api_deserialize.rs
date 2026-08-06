//! Benchmark JSON request deserialization throughput.
#![allow(missing_docs)]

use casiros_api::models::EvaluateRequest;
use criterion::{Criterion, criterion_group, criterion_main};

const EVALUATE_JSON: &str = r#"{
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
}"#;

fn bench_api_deserialize(c: &mut Criterion) {
    c.bench_function("api_deserialize_evaluate_request", |b| {
        b.iter(|| serde_json::from_str::<EvaluateRequest>(EVALUATE_JSON).unwrap());
    });
}

criterion_group!(benches, bench_api_deserialize);
criterion_main!(benches);
