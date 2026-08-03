//! Integration tests for the `casiros-cli convert` command.

use std::fs;
use std::io::Write;

use assert_cmd::Command;
use rust_decimal_macros::dec;
use serde_json::json;
use tempfile::tempdir;

fn write_file(dir: &tempfile::TempDir, name: &str, content: &str) -> std::path::PathBuf {
    let path = dir.path().join(name);
    let mut file = fs::File::create(&path).unwrap();
    file.write_all(content.as_bytes()).unwrap();
    path
}

#[test]
fn convert_csv_to_json_inputs_map() {
    let dir = tempdir().unwrap();
    let csv = write_file(
        &dir,
        "inputs.csv",
        "node,value\nprincipal,100.0\nrate,0.05\n",
    );
    let json_out = dir.path().join("outputs.json");

    let mut cmd = Command::cargo_bin("casiros-cli").unwrap();
    cmd.arg("convert").arg(&csv).arg(&json_out);
    cmd.assert().success();

    let text = fs::read_to_string(&json_out).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(parsed["principal"], json!(dec!(100.0)));
    assert_eq!(parsed["rate"], json!(dec!(0.05)));
}

#[test]
fn convert_json_outputs_to_csv() {
    let dir = tempdir().unwrap();
    let json = write_file(
        &dir,
        "response.json",
        r#"{"outputs":{"fv":"162.8895","pv":"100"}}"#,
    );
    let csv_out = dir.path().join("out.csv");

    let mut cmd = Command::cargo_bin("casiros-cli").unwrap();
    cmd.arg("convert").arg(&json).arg(&csv_out);
    cmd.assert().success();

    let text = fs::read_to_string(&csv_out).unwrap();
    assert!(text.contains("node,value"));
    assert!(text.contains("fv,162.8895"));
    assert!(text.contains("pv,100"));
}

#[test]
fn convert_json_simulate_response_to_csv() {
    let dir = tempdir().unwrap();
    let json = write_file(
        &dir,
        "simulate.json",
        r#"{"count":1000,"mean":"10.5","median":"10.2","min":"5.0","max":"20.0"}"#,
    );
    let csv_out = dir.path().join("out.csv");

    let mut cmd = Command::cargo_bin("casiros-cli").unwrap();
    cmd.arg("convert").arg(&json).arg(&csv_out);
    cmd.assert().success();

    let text = fs::read_to_string(&csv_out).unwrap();
    assert!(text.contains("metric,value"));
    assert!(text.contains("count,1000"));
    assert!(text.contains("mean,10.5"));
}

#[test]
fn convert_excel_round_trip_inputs_map() {
    let dir = tempdir().unwrap();
    let json = write_file(
        &dir,
        "inputs.json",
        r#"{"spot":"100","strike":"95","volatility":"0.2"}"#,
    );
    let xlsx_out = dir.path().join("out.xlsx");

    let mut export = Command::cargo_bin("casiros-cli").unwrap();
    export.arg("convert").arg(&json).arg(&xlsx_out);
    export.assert().success();

    let json_in = dir.path().join("roundtrip.json");
    let mut import = Command::cargo_bin("casiros-cli").unwrap();
    import.arg("convert").arg(&xlsx_out).arg(&json_in);
    import.assert().success();

    let text = fs::read_to_string(&json_in).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(parsed["spot"], json!(dec!(100)));
    assert_eq!(parsed["strike"], json!(dec!(95)));
    assert_eq!(parsed["volatility"], json!(dec!(0.2)));
}
