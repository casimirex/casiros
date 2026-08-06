//! Integration tests for the `casiros-cli` binary.

use std::io::Write;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::NamedTempFile;

const EVALUATE_REQUEST: &str = r#"{
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

fn write_temp(content: &str) -> NamedTempFile {
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(content.as_bytes()).unwrap();
    file
}

#[test]
fn evaluate_command_prints_outputs() {
    let input = write_temp(EVALUATE_REQUEST);
    let mut cmd = Command::cargo_bin("casiros-cli").unwrap();
    cmd.arg("evaluate").arg(input.path());
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("outputs"))
        .stdout(predicate::str::contains("162.889"));
}

#[test]
fn validate_command_reports_depth() {
    let input = write_temp(EVALUATE_REQUEST);
    let mut cmd = Command::cargo_bin("casiros-cli").unwrap();
    cmd.arg("validate").arg(input.path());
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("\"valid\":true"))
        .stdout(predicate::str::contains("\"depth\":2"));
}

#[test]
fn save_and_load_round_trip() {
    let engine = write_temp(EVALUATE_REQUEST);
    let snapshot = NamedTempFile::new().unwrap();
    let restored = NamedTempFile::new().unwrap();

    let mut save = Command::cargo_bin("casiros-cli").unwrap();
    save.arg("save").arg(engine.path()).arg(snapshot.path());
    save.assert().success();

    let mut load = Command::cargo_bin("casiros-cli").unwrap();
    load.arg("load").arg(snapshot.path()).arg(restored.path());
    load.assert().success();

    let restored_text = std::fs::read_to_string(restored.path()).unwrap();
    assert!(restored_text.contains("principal"));
    assert!(restored_text.contains("future_value"));
}

#[test]
fn missing_file_fails() {
    let mut cmd = Command::cargo_bin("casiros-cli").unwrap();
    cmd.arg("evaluate").arg("/nonexistent/path.json");
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Failed to read"));
}
