use std::path::PathBuf;
use std::process::Command;

use serde_json::Value;

fn tracelens() -> Command {
    Command::new(env!("CARGO_BIN_EXE_tracelens"))
}

fn fixture(name: &str) -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
        .display()
        .to_string()
}

#[test]
fn validate_basic_fixture() {
    let fixture = fixture("otlp-basic.json");
    let output = tracelens()
        .args(["validate", fixture.as_str()])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("Status: ok"));
    assert!(stdout.contains("Traces: 2"));
    assert!(stdout.contains("Spans: 4"));
}

#[test]
fn summary_lists_slowest_traces() {
    let fixture = fixture("otlp-basic.json");
    let output = tracelens()
        .args(["summary", fixture.as_str()])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("Slowest traces:"));
    assert!(stdout.contains("5b8efff798038103d269b633813fc60c"));
    assert!(stdout.contains("100.000ms"));
}

#[test]
fn tree_accepts_uppercase_trace_id() {
    let fixture = fixture("otlp-basic.json");
    let output = tracelens()
        .args([
            "tree",
            fixture.as_str(),
            "--trace-id",
            "5B8EFFF798038103D269B633813FC60C",
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("[checkout-service] GET /checkout"));
    assert!(stdout.contains("[payment-service] POST /charge"));
    assert!(stdout.contains("ERROR"));
}

#[test]
fn strict_validate_fails_on_invalid_time() {
    let fixture = fixture("otlp-invalid-time.json");
    let output = tracelens()
        .args(["validate", fixture.as_str(), "--strict"])
        .output()
        .expect("command should run");

    assert!(!output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("Status: failed"));
    assert!(stdout.contains("invalid_time_range"));
}

#[test]
fn validate_accepts_jsonl_fixture() {
    let fixture = fixture("otlp-basic.jsonl");
    let output = tracelens()
        .args(["validate", fixture.as_str()])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("Traces: 1"));
    assert!(stdout.contains("Spans: 2"));
}

#[test]
fn list_traces_sorts_by_duration() {
    let fixture = fixture("otlp-basic.json");
    let output = tracelens()
        .args(["list-traces", fixture.as_str()])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    let slow_index = stdout
        .find("5b8efff798038103d269b633813fc60c")
        .expect("slow trace should be listed");
    let fast_index = stdout
        .find("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
        .expect("fast trace should be listed");
    assert!(slow_index < fast_index);
}

#[test]
fn list_traces_respects_limit() {
    let fixture = fixture("otlp-basic.json");
    let output = tracelens()
        .args(["list-traces", fixture.as_str(), "--limit", "1"])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("5b8efff798038103d269b633813fc60c"));
    assert!(!stdout.contains("aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"));
}

#[test]
fn validate_outputs_json() {
    let fixture = fixture("otlp-basic.json");
    let output = tracelens()
        .args(["validate", fixture.as_str(), "--output", "json"])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let value: Value = serde_json::from_slice(&output.stdout).expect("stdout should be json");
    assert_eq!(value["schema_version"], "0.1");
    assert_eq!(value["command"], "validate");
    assert_eq!(value["span_count"], 4);
}

#[test]
fn default_validate_json_status_matches_exit_semantics() {
    let fixture = fixture("otlp-jsonl-invalid-line.jsonl");
    let output = tracelens()
        .args(["validate", fixture.as_str(), "--output", "json"])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let value: Value = serde_json::from_slice(&output.stdout).expect("stdout should be json");
    assert_eq!(value["status"], "ok");
    assert_eq!(value["exit_would_fail"], false);
    assert_eq!(value["error_diagnostic_count"], 1);
}

#[test]
fn summary_outputs_json() {
    let fixture = fixture("otlp-basic.json");
    let output = tracelens()
        .args(["summary", fixture.as_str(), "--output", "json"])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let value: Value = serde_json::from_slice(&output.stdout).expect("stdout should be json");
    assert_eq!(value["schema_version"], "0.1");
    assert_eq!(value["summary"]["trace_count"], 2);
    assert!(value["slowest_traces"].as_array().unwrap().len() >= 2);
}

#[test]
fn tree_outputs_json() {
    let fixture = fixture("otlp-basic.json");
    let output = tracelens()
        .args([
            "tree",
            fixture.as_str(),
            "--trace-id",
            "5B8EFFF798038103D269B633813FC60C",
            "--output",
            "json",
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let value: Value = serde_json::from_slice(&output.stdout).expect("stdout should be json");
    assert_eq!(value["schema_version"], "0.1");
    assert_eq!(value["trace"]["span_count"], 3);
    assert_eq!(value["nodes"].as_array().unwrap()[0]["depth"], 0);
}

#[test]
fn list_traces_outputs_json() {
    let fixture = fixture("otlp-basic.json");
    let output = tracelens()
        .args([
            "list-traces",
            fixture.as_str(),
            "--limit",
            "1",
            "--output",
            "json",
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let value: Value = serde_json::from_slice(&output.stdout).expect("stdout should be json");
    assert_eq!(value["schema_version"], "0.1");
    assert_eq!(value["command"], "list-traces");
    assert_eq!(value["traces"].as_array().unwrap().len(), 1);
}

#[test]
fn strict_validate_fails_on_jsonl_invalid_line() {
    let fixture = fixture("otlp-jsonl-invalid-line.jsonl");
    let output = tracelens()
        .args(["validate", fixture.as_str(), "--strict"])
        .output()
        .expect("command should run");

    assert!(!output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("malformed_jsonl_line"));
}
