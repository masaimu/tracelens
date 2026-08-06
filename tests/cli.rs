use std::path::PathBuf;
use std::process::Command;

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
