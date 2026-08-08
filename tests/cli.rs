use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicUsize, Ordering};

static REPORT_TMP_COUNTER: AtomicUsize = AtomicUsize::new(0);

use serde_json::Value;

const EXIT_SUCCESS: i32 = 0;
const EXIT_FAILURE: i32 = 1;
const EXIT_USAGE: i32 = 2;

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

fn contains_ansi(value: &str) -> bool {
    value.contains("\x1b[")
}

fn assert_exit_code(output: &Output, expected: i32) {
    assert_eq!(
        output.status.code(),
        Some(expected),
        "unexpected exit code\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn output_schema() -> Value {
    serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/schemas/tracelens-output.schema.json"
    )))
    .expect("schema should be valid JSON")
}

fn schema_pointer<'a>(schema: &'a Value, pointer: &str) -> &'a str {
    schema
        .pointer(pointer)
        .and_then(Value::as_str)
        .unwrap_or_else(|| panic!("schema pointer should be a string: {pointer}"))
}

fn collect_schema_properties_missing_descriptions(
    node: &Value,
    path: &str,
    missing: &mut Vec<String>,
) {
    if let Some(properties) = node.get("properties").and_then(Value::as_object) {
        for (name, value) in properties {
            let field_path = if path.is_empty() {
                name.to_string()
            } else {
                format!("{path}.{name}")
            };

            let has_description = value
                .get("description")
                .and_then(Value::as_str)
                .is_some_and(|description| !description.trim().is_empty());
            if !has_description {
                missing.push(field_path.clone());
            }

            collect_schema_properties_missing_descriptions(value, &field_path, missing);
        }
    }

    if let Some(items) = node.get("items") {
        let item_path = if path.is_empty() {
            "[]".to_string()
        } else {
            format!("{path}[]")
        };
        collect_schema_properties_missing_descriptions(items, &item_path, missing);
    }

    for keyword in ["oneOf", "allOf", "anyOf"] {
        if let Some(values) = node.get(keyword).and_then(Value::as_array) {
            for value in values {
                collect_schema_properties_missing_descriptions(value, path, missing);
            }
        }
    }
}

fn assert_matches_output_schema(value: &Value) {
    let schema = output_schema();
    assert!(
        jsonschema::meta::is_valid(&schema),
        "tracelens output schema should be a valid JSON Schema"
    );

    let validator = jsonschema::validator_for(&schema).expect("schema should compile");
    let errors = validator
        .iter_errors(value)
        .map(|error| format!("{} at {}", error, error.instance_path()))
        .collect::<Vec<_>>();
    assert!(
        errors.is_empty(),
        "output did not match schema:\n{}\nvalue:\n{}",
        errors.join("\n"),
        serde_json::to_string_pretty(value).expect("value should pretty print")
    );
}

fn run_json(args: &[&str]) -> Value {
    let output = tracelens().args(args).output().expect("command should run");

    assert!(
        output.status.success(),
        "command should succeed: {:?}\nstderr:\n{}",
        args,
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("stdout should be json")
}

#[test]
fn help_mentions_schema_discovery() {
    let output = tracelens()
        .arg("--help")
        .output()
        .expect("command should run");

    assert_exit_code(&output, EXIT_SUCCESS);
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("Output schema"));
    assert!(stdout.contains("tracelens schema --output json"));
    assert!(stdout.contains("field descriptions"));
    assert!(stdout.contains("schema"));
}

#[test]
fn usage_errors_exit_two() {
    let output = tracelens()
        .arg("--definitely-invalid")
        .output()
        .expect("command should run");

    assert_exit_code(&output, EXIT_USAGE);
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(stderr.contains("unexpected argument"));
}

#[test]
fn schema_help_explains_field_descriptions() {
    let output = tracelens()
        .args(["schema", "--help"])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("Print the JSON output schema and field descriptions"));
    assert!(stdout.contains("--command"));
    assert!(stdout.contains("--output"));
}

#[test]
fn schema_outputs_json_with_descriptions() {
    let value = run_json(&["schema", "--output", "json"]);

    assert!(
        jsonschema::meta::is_valid(&value),
        "schema command should print a valid JSON Schema"
    );
    assert_eq!(
        schema_pointer(&value, "/$defs/schemaVersion/description"),
        "Output contract version. Current value is \"0.1\"; the contract can change before a stable 1.0 release."
    );
    assert!(
        schema_pointer(&value, "/$defs/diagnostic/properties/code/description")
            .contains("Stable diagnostic code")
    );
    assert!(
        schema_pointer(
            &value,
            "/$defs/serviceDuration/properties/self_time_ns/description"
        )
        .contains("Service self time")
    );
    assert!(
        schema_pointer(
            &value,
            "/$defs/criticalPath/properties/segments/description"
        )
        .contains("Critical-path segments")
    );
    assert!(
        schema_pointer(
            &value,
            "/$defs/timelineOutput/properties/timeline/properties/rows/description"
        )
        .contains("Timeline rows")
    );
    assert!(
        schema_pointer(
            &value,
            "/$defs/detectOutput/properties/slow_traces/description"
        )
        .contains("Slow trace candidates")
    );
    assert!(
        schema_pointer(
            &value,
            "/$defs/detectOutput/properties/n_plus_one_candidates/description"
        )
        .contains("N+1-like candidates")
    );
}

#[test]
fn schema_outputs_text_field_reference() {
    let output = tracelens()
        .args(["schema", "--output", "text"])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("tracelens JSON Output Reference"));
    assert!(stdout.contains("schema_version"));
    assert!(stdout.contains("diagnostics"));
    assert!(stdout.contains("self_time_ns"));
    assert!(stdout.contains("critical_path.segments"));
    assert!(stdout.contains("timeline.rows"));
    assert!(stdout.contains("slow_traces"));
    assert!(stdout.contains("confidence"));
    assert!(stdout.contains("n_plus_one_candidates"));
}

#[test]
fn schema_text_filter_limits_to_selected_command() {
    let output = tracelens()
        .args(["schema", "--command", "detect", "--output", "text"])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("command filter: detect"));
    assert!(stdout.contains("[detect]"));
    assert!(stdout.contains("slow_traces"));
    assert!(stdout.contains("n_plus_one_candidates"));
    assert!(!stdout.contains("[timeline]"));
}

#[test]
fn output_schema_properties_have_descriptions() {
    let schema = output_schema();
    let mut missing = Vec::new();

    let definitions = schema["$defs"]
        .as_object()
        .expect("schema should contain definitions");
    for (name, definition) in definitions {
        let has_definition_description = definition
            .get("description")
            .and_then(Value::as_str)
            .is_some_and(|description| !description.trim().is_empty());
        assert!(
            has_definition_description,
            "schema definition should have description: {name}"
        );
        collect_schema_properties_missing_descriptions(definition, name, &mut missing);
    }

    assert!(
        missing.is_empty(),
        "schema properties are missing descriptions:\n{}",
        missing.join("\n")
    );
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

    assert_exit_code(&output, EXIT_FAILURE);
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

    assert_exit_code(&output, EXIT_SUCCESS);
    assert!(output.status.success());
    let value: Value = serde_json::from_slice(&output.stdout).expect("stdout should be json");
    assert_eq!(value["schema_version"], "0.1");
    assert_eq!(value["command"], "validate");
    assert_eq!(value["span_count"], 4);
}

#[test]
fn strict_validate_json_failure_exit_contract_matches_payload() {
    let fixture = fixture("otlp-invalid-time.json");
    let output = tracelens()
        .args(["validate", fixture.as_str(), "--strict", "--output", "json"])
        .output()
        .expect("command should run");

    assert_exit_code(&output, EXIT_FAILURE);
    let value: Value = serde_json::from_slice(&output.stdout).expect("stdout should be json");
    assert_eq!(value["status"], "failed");
    assert_eq!(value["exit_would_fail"], true);
    assert_eq!(value["error_diagnostic_count"], 1);
}

#[test]
fn summary_without_valid_spans_exits_one() {
    let fixture = fixture("otlp-all-zero-id.json");
    let output = tracelens()
        .args(["summary", fixture.as_str()])
        .output()
        .expect("command should run");

    assert_exit_code(&output, EXIT_FAILURE);
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(stderr.contains("no valid spans found"));
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
fn services_outputs_chinese_explanations() {
    let fixture = fixture("otlp-basic.json");
    let output = tracelens()
        .args([
            "services",
            fixture.as_str(),
            "--trace-id",
            "5B8EFFF798038103D269B633813FC60C",
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("Trace 耗时概览"));
    assert!(stdout.contains("服务耗时贡献"));
    assert!(stdout.contains("字段说明"));
    assert!(stdout.contains("self_time"));
    assert!(stdout.contains("说明：wall-clock duration"));
    assert!(stdout.contains("checkout-service"));
    assert!(stdout.contains("10.000ms"));
}

#[test]
fn services_outputs_json() {
    let fixture = fixture("otlp-basic.json");
    let output = tracelens()
        .args([
            "services",
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
    assert_eq!(value["command"], "services");
    assert_eq!(value["trace"]["wall_clock_duration_ns"], 100_000_000);
    assert_eq!(value["trace"]["root_span"]["duration_ns"], 100_000_000);

    let services = value["services"]
        .as_array()
        .expect("services should be array");
    let checkout = services
        .iter()
        .find(|service| service["service_name"] == "checkout-service")
        .expect("checkout service should be present");
    assert_eq!(checkout["self_time_ns"], 10_000_000);
    assert_eq!(checkout["span_time_ns"], 100_000_000);
    assert_eq!(checkout["child_covered_time_ns"], 90_000_000);
}

#[test]
fn detect_outputs_chinese_explanations() {
    let fixture = fixture("otlp-detect.json");
    let output = tracelens()
        .args([
            "--color",
            "never",
            "detect",
            fixture.as_str(),
            "--limit",
            "2",
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("Detect 检测概览"));
    assert!(stdout.contains("样本数: 6"));
    assert!(stdout.contains("慢请求候选"));
    assert!(stdout.contains("service candidates"));
    assert!(stdout.contains("checkout-service"));
    assert!(stdout.contains("服务耗时分布"));
    assert!(stdout.contains("slow span samples"));
    assert!(stdout.contains("错误传播候选"));
    assert!(stdout.contains("earliest:"));
    assert!(stdout.contains("top:"));
    assert!(stdout.contains("错误传播链"));
    assert!(stdout.contains("path: root -> earliest error"));
    assert!(stdout.contains("downstream errors"));
    assert!(stdout.contains("status_code_error(OTLP ERROR)"));
    assert!(stdout.contains("http_5xx(HTTP 5xx)"));
    assert!(stdout.contains("grpc_non_zero(gRPC 非 0)"));
    assert!(stdout.contains("exception_event(exception 事件)"));
    assert!(stdout.contains("N+1 检测"));
}

#[test]
fn detect_outputs_json() {
    let fixture = fixture("otlp-detect.json");
    let output = tracelens()
        .args(["detect", fixture.as_str(), "--output", "json"])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let value: Value = serde_json::from_slice(&output.stdout).expect("stdout should be json");

    assert_eq!(value["schema_version"], "0.1");
    assert_eq!(value["command"], "detect");
    assert_eq!(value["summary"]["sample_count"], 6);
    assert_eq!(value["summary"]["sample_quality"], "limited");
    assert_eq!(value["summary"]["p95_duration_ns"], 900_000_000_u64);
    assert_eq!(value["summary"]["error_propagation_chain_count"], 1);
    assert_eq!(value["summary"]["service_latency_distribution_count"], 5);
    assert_eq!(
        value["slow_traces"][0]["trace_id"],
        "66666666666666666666666666666666"
    );
    assert_eq!(value["slow_traces"][0]["confidence"], "medium");
    assert!(
        value["slow_traces"][0]["service_candidates"]
            .as_array()
            .expect("service_candidates should be array")
            .iter()
            .any(|service| service["service_name"] == "checkout-service")
    );
    assert_eq!(value["error_traces"][0]["error_span_count"], 4);
    assert_eq!(
        value["error_traces"][0]["top_error_span"]["span_id"],
        "6600000000000001"
    );
    let signals = value["error_traces"][0]["earliest_error_span"]["signals"]
        .as_array()
        .expect("signals should be array");
    assert!(signals.iter().any(|signal| signal == "status_code_error"));
    assert_eq!(
        value["error_propagation_chains"][0]["trace_id"],
        "66666666666666666666666666666666"
    );
    assert_eq!(
        value["error_propagation_chains"][0]["path_to_earliest_error"]
            .as_array()
            .expect("path_to_earliest_error should be array")
            .len(),
        1
    );
    assert_eq!(
        value["error_propagation_chains"][0]["downstream_error_span_count"],
        3
    );
    assert!(
        value["error_propagation_chains"][0]["downstream_error_spans"]
            .as_array()
            .expect("downstream_error_spans should be array")
            .iter()
            .any(|span| span["span_id"] == "6600000000000002")
    );
    let service_distribution = value["service_latency_distribution"]
        .as_array()
        .expect("service_latency_distribution should be array");
    let checkout = service_distribution
        .iter()
        .find(|service| service["service_name"] == "checkout-service")
        .expect("checkout distribution should exist");
    assert_eq!(checkout["p95_duration_ns"], 900_000_000_u64);
    assert_eq!(checkout["max_span_duration_ns"], 900_000_000_u64);
    assert_eq!(
        checkout["slow_span_samples"][0]["span_id"],
        "6600000000000001"
    );
    assert_eq!(value["summary"]["n_plus_one_candidate_count"], 0);
    assert!(
        value["n_plus_one_candidates"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[test]
fn detect_outputs_n_plus_one_candidates_with_chinese_explanations() {
    let fixture = fixture("otlp-n-plus-one.json");
    let output = tracelens()
        .args([
            "--color",
            "never",
            "detect",
            fixture.as_str(),
            "--limit",
            "5",
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("N+1 候选"));
    assert!(stdout.contains("repeated=10"));
    assert!(stdout.contains("serial_ratio=100.0%"));
    assert!(stdout.contains("confidence=high"));
    assert!(stdout.contains("select product {num}"));
    assert!(stdout.contains("repeated=6"));
    assert!(stdout.contains("serial_ratio=0.0%"));
    assert!(stdout.contains("confidence=medium"));
    assert!(stdout.contains("possible N+1"));
}

#[test]
fn detect_outputs_n_plus_one_json() {
    let fixture = fixture("otlp-n-plus-one.json");
    let output = tracelens()
        .args([
            "detect",
            fixture.as_str(),
            "--limit",
            "5",
            "--output",
            "json",
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let value: Value = serde_json::from_slice(&output.stdout).expect("stdout should be json");
    assert_eq!(value["summary"]["n_plus_one_candidate_count"], 2);

    let candidates = value["n_plus_one_candidates"]
        .as_array()
        .expect("n_plus_one_candidates should be array");
    let high = candidates
        .iter()
        .find(|candidate| candidate["trace_id"] == "77777777777777777777777777777777")
        .expect("high confidence candidate should exist");
    assert_eq!(high["repeated_count"], 10);
    assert_eq!(high["serial_ratio_per_mille"], 1000);
    assert_eq!(high["serial_ratio"], 1.0);
    assert_eq!(high["confidence"], "high");
    assert_eq!(
        high["child_group"]["normalized_name"],
        "select product {num}"
    );
    assert_eq!(high["child_group"]["db_system"], "postgresql");

    let possible = candidates
        .iter()
        .find(|candidate| candidate["trace_id"] == "88888888888888888888888888888888")
        .expect("possible candidate should exist");
    assert_eq!(possible["repeated_count"], 6);
    assert_eq!(possible["serial_ratio_per_mille"], 0);
    assert_eq!(possible["confidence"], "medium");
}

#[test]
fn detect_rejects_zero_limit() {
    let fixture = fixture("otlp-detect.json");
    let output = tracelens()
        .args(["detect", fixture.as_str(), "--limit", "0"])
        .output()
        .expect("command should run");

    assert_exit_code(&output, EXIT_FAILURE);
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(stderr.contains("--limit must be greater than 0"));
}

#[test]
fn strict_validate_fails_on_jsonl_invalid_line() {
    let fixture = fixture("otlp-jsonl-invalid-line.jsonl");
    let output = tracelens()
        .args(["validate", fixture.as_str(), "--strict"])
        .output()
        .expect("command should run");

    assert_exit_code(&output, EXIT_FAILURE);
    assert!(!output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("malformed_jsonl_line"));
}

#[test]
fn critical_path_outputs_segments_and_classification() {
    let fixture = fixture("otlp-concurrent.json");
    let output = tracelens()
        .args([
            "critical-path",
            fixture.as_str(),
            "--trace-id",
            "CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC",
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("关键路径"));
    assert!(stdout.contains("critical path duration: 1.000s"));
    assert!(stdout.contains("wall-clock duration: 1.100s"));
    assert!(stdout.contains("checkout-service"));
    assert!(stdout.contains("redis"));
    assert!(stdout.contains("SET cache"));
    assert!(stdout.contains("serial: 3  concurrent: 4  nested: 5  suspicious: 1"));
    assert!(stdout.contains("并发 span："));
    assert!(stdout.contains("可疑 span"));
    assert!(stdout.contains("POST /notify"));
}

#[test]
fn critical_path_outputs_json() {
    let fixture = fixture("otlp-concurrent.json");
    let output = tracelens()
        .args([
            "critical-path",
            fixture.as_str(),
            "--trace-id",
            "cccccccccccccccccccccccccccccccc",
            "--output",
            "json",
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    let json: Value = serde_json::from_str(&stdout).expect("stdout should be valid json");

    assert_eq!(json["schema_version"], "0.1");
    assert_eq!(json["command"], "critical-path");
    assert_eq!(json["critical_path"]["status"], "available");
    assert_eq!(
        json["critical_path"]["total_duration_ns"],
        1_000_000_000_u64
    );
    assert_eq!(json["trace"]["wall_clock_duration_ns"], 1_100_000_000_u64);
    assert_eq!(json["classification"]["counts"]["serial"], 3);
    assert_eq!(json["classification"]["counts"]["concurrent"], 4);
    assert_eq!(json["classification"]["counts"]["nested"], 5);
    assert_eq!(json["classification"]["counts"]["suspicious"], 1);

    let segments = json["critical_path"]["segments"]
        .as_array()
        .expect("segments should be an array");
    let covered: u64 = segments
        .iter()
        .map(|segment| segment["duration_ns"].as_u64().expect("duration_ns"))
        .sum();
    assert_eq!(covered, 1_000_000_000_u64);

    let totals = json["critical_path"]["span_totals"]
        .as_array()
        .expect("span_totals should be an array");
    assert_eq!(totals[0]["span_id"], "0000000000000002");
    assert_eq!(totals[0]["total_ns"], 300_000_000_u64);

    let notes = json["critical_path"]["notes"]
        .as_array()
        .expect("notes should be an array");
    assert!(notes.iter().any(|note| {
        note.as_str()
            .expect("note")
            .contains("wall-clock duration exceeds")
    }));
}

#[test]
fn timeline_outputs_ascii_bars_and_chinese_explanations() {
    let fixture = fixture("otlp-concurrent.json");
    let output = tracelens()
        .args([
            "--color",
            "never",
            "timeline",
            fixture.as_str(),
            "--trace-id",
            "CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC",
            "--width",
            "48",
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(!contains_ansi(&stdout));
    assert!(stdout.contains("Trace Timeline"));
    assert!(stdout.contains("横轴表示从 trace start 到 trace end 的相对时间"));
    assert!(stdout.contains("* 表示该 span 出现在关键路径中"));
    assert!(stdout.contains("axis: 0ns |------------------------------------------------| 1.100s"));
    assert!(stdout.contains("bar_width: 48"));
    assert!(stdout.contains("GET /checkout"));
    assert!(stdout.contains("POST /charge"));
    assert!(stdout.contains("GET /stock"));
    assert!(stdout.contains("=============="));
    assert!(stdout.contains("#############"));
    assert!(stdout.contains("注意：wall-clock duration 大于被选中 root span 的时间区间"));
    assert!(stdout.contains("字段说明"));
}

#[test]
fn timeline_outputs_json_without_ansi_and_preserves_overlap() {
    let fixture = fixture("otlp-concurrent.json");
    let output = tracelens()
        .args([
            "--color",
            "always",
            "timeline",
            fixture.as_str(),
            "--trace-id",
            "cccccccccccccccccccccccccccccccc",
            "--width",
            "48",
            "--output",
            "json",
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(!contains_ansi(&stdout));
    let json: Value = serde_json::from_str(&stdout).expect("stdout should be valid json");

    assert_eq!(json["schema_version"], "0.1");
    assert_eq!(json["command"], "timeline");
    assert_eq!(json["timeline"]["width"], 48);
    assert_eq!(json["trace"]["wall_clock_duration_ns"], 1_100_000_000_u64);
    assert_eq!(json["critical_path"]["status"], "available");

    let rows = json["timeline"]["rows"]
        .as_array()
        .expect("timeline rows should be an array");
    assert_eq!(rows.len(), 7);

    let checkout = rows
        .iter()
        .find(|row| row["span_id"] == "0000000000000001")
        .expect("checkout row should exist");
    assert_eq!(checkout["is_critical_path"], true);
    assert_eq!(checkout["depth"], 0);

    let charge = rows
        .iter()
        .find(|row| row["span_id"] == "0000000000000003")
        .expect("charge row should exist");
    let stock = rows
        .iter()
        .find(|row| row["span_id"] == "0000000000000006")
        .expect("stock row should exist");
    assert_eq!(charge["bar_start"], stock["bar_start"]);
    assert_eq!(charge["start_offset_ns"], stock["start_offset_ns"]);
    assert_eq!(stock["is_critical_path"], false);
}

#[test]
fn timeline_rejects_invalid_width() {
    let fixture = fixture("otlp-concurrent.json");
    let output = tracelens()
        .args([
            "timeline",
            fixture.as_str(),
            "--trace-id",
            "cccccccccccccccccccccccccccccccc",
            "--width",
            "12",
        ])
        .output()
        .expect("command should run");

    assert_exit_code(&output, EXIT_FAILURE);
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(stderr.contains("--width must be between 40 and 160"));
}

#[test]
fn timeline_flame_mode_renders_indented_rows_without_axis() {
    let fixture = fixture("otlp-concurrent.json");
    let output = tracelens()
        .args([
            "--color",
            "never",
            "timeline",
            fixture.as_str(),
            "--trace-id",
            "CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC",
            "--mode",
            "flame",
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(!contains_ansi(&stdout));
    assert!(stdout.contains("Trace Timeline (flame)"));
    assert!(stdout.contains("flame 视图按调用深度纵向缩进"));
    // flame layout has no horizontal time axis or ASCII bar fills.
    assert!(!stdout.contains("axis: 0ns"));
    assert!(!stdout.contains("=============="));
    assert!(!stdout.contains("#############"));
    // child spans still appear, indented by depth.
    assert!(stdout.contains("GET /checkout"));
    assert!(stdout.contains("GET /cart"));
    // critical-path marking semantics are unchanged in flame mode.
    assert!(stdout.contains("* 表示该 span 出现在关键路径中"));
}

#[test]
fn timeline_flame_mode_outputs_json_with_mode_and_collapse_fields() {
    let json = run_json(&[
        "--color",
        "never",
        "timeline",
        fixture("otlp-concurrent.json").as_str(),
        "--trace-id",
        "cccccccccccccccccccccccccccccccc",
        "--mode",
        "flame",
        "--output",
        "json",
    ]);

    assert_eq!(json["schema_version"], "0.1");
    assert_eq!(json["timeline"]["mode"], "flame");
    assert_eq!(json["timeline"]["collapsed"]["enabled"], true);
    assert_eq!(json["timeline"]["collapsed"]["omitted_rows"], 0);
    assert!(json["timeline"]["collapsed"]["preserved_reasons"].is_array());

    let rows = json["timeline"]["rows"]
        .as_array()
        .expect("timeline rows should be an array");
    assert!(!rows.is_empty());
    for row in rows {
        assert_eq!(row["mode"], "flame");
        assert_eq!(row["is_collapse_marker"], false);
    }

    // the flame JSON must still satisfy the published output schema, which now
    // requires the per-row mode/is_collapse_marker and top-level collapsed.
    assert_matches_output_schema(&json);
}

#[test]
fn timeline_max_rows_collapses_middle_rows() {
    // trace 88888888 in otlp-n-plus-one.json is a concurrent fan-out (GET /cart
    // + 6 GET /inventory children); only the root and one child are on the
    // critical path, so the non-critical middle children can be collapsed.
    // The serial N+1 trace (77777777...) has every span on the critical path
    // and cannot collapse, so it is intentionally not used here.
    let fixture = fixture("otlp-n-plus-one.json");
    let text_output = tracelens()
        .args([
            "--color",
            "never",
            "timeline",
            fixture.as_str(),
            "--trace-id",
            "88888888888888888888888888888888",
            "--max-rows",
            "6",
        ])
        .output()
        .expect("command should run");
    assert!(text_output.status.success());
    let text_stdout = String::from_utf8(text_output.stdout).expect("stdout should be utf8");
    assert!(text_stdout.contains("collapsed"));
    assert!(text_stdout.contains("omitted: 4"));

    let json = run_json(&[
        "--color",
        "never",
        "timeline",
        fixture.as_str(),
        "--trace-id",
        "88888888888888888888888888888888",
        "--max-rows",
        "6",
        "--output",
        "json",
    ]);

    assert_eq!(json["timeline"]["collapsed"]["enabled"], true);
    assert_eq!(json["timeline"]["collapsed"]["omitted_rows"], 4);
    assert!(
        json["timeline"]["collapsed"]["preserved_reasons"]
            .as_array()
            .expect("preserved_reasons should be an array")
            .iter()
            .any(|reason| reason == "critical_path")
    );

    let rows = json["timeline"]["rows"]
        .as_array()
        .expect("timeline rows should be an array");
    assert!(
        rows.iter().any(|row| row["is_collapse_marker"] == true),
        "expected a collapse marker row"
    );
    // critical-path rows are preserved through collapse.
    assert!(
        rows.iter()
            .any(|row| row["name"] == "GET /cart" && row["is_critical_path"] == true)
    );
    assert!(
        rows.iter()
            .any(|row| row["name"] == "GET /inventory/1" && row["is_critical_path"] == true)
    );

    assert_matches_output_schema(&json);
}

#[test]
fn timeline_max_rows_zero_keeps_all_rows() {
    let fixture = fixture("otlp-n-plus-one.json");
    let args = |max_rows: &str| {
        vec![
            "--color".to_string(),
            "never".to_string(),
            "timeline".to_string(),
            fixture.clone(),
            "--trace-id".to_string(),
            "88888888888888888888888888888888".to_string(),
            "--max-rows".to_string(),
            max_rows.to_string(),
            "--output".to_string(),
            "json".to_string(),
        ]
    };

    let zero = run_json(&args("0").iter().map(String::as_str).collect::<Vec<_>>());
    let default = run_json(&args("40").iter().map(String::as_str).collect::<Vec<_>>());

    assert_eq!(zero["timeline"]["collapsed"]["enabled"], false);
    assert_eq!(zero["timeline"]["collapsed"]["omitted_rows"], 0);
    assert_eq!(
        zero["timeline"]["collapsed"]["preserved_reasons"]
            .as_array()
            .map(Vec::len),
        Some(0)
    );
    let zero_rows = zero["timeline"]["rows"]
        .as_array()
        .expect("timeline rows should be an array");
    let default_rows = default["timeline"]["rows"]
        .as_array()
        .expect("timeline rows should be an array");
    assert_eq!(zero_rows.len(), default_rows.len());
    assert!(
        zero_rows
            .iter()
            .all(|row| row["is_collapse_marker"] == false)
    );
    // all spans are still present without collapse.
    for name in [
        "GET /cart",
        "GET /inventory/1",
        "GET /inventory/2",
        "GET /inventory/3",
        "GET /inventory/4",
        "GET /inventory/5",
        "GET /inventory/6",
    ] {
        assert!(
            zero_rows.iter().any(|row| row["name"] == name),
            "expected span {name} to remain without collapse"
        );
    }

    assert_matches_output_schema(&zero);
}

#[test]
fn tree_outputs_semantic_annotations_with_chinese_explanations() {
    let fixture = fixture("otlp-semantic-annotations.json");
    let output = tracelens()
        .args([
            "--color",
            "never",
            "tree",
            fixture.as_str(),
            "--trace-id",
            "DDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDD",
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("标注=client/server"));
    assert!(stdout.contains("linked(1)"));
    assert!(stdout.contains("Span 语义标注"));
    assert!(stdout.contains("client/server pair 表示 client span 直接调用 server span"));
    assert!(stdout.contains("async/linked 表示 producer、consumer、messaging 属性或 span links"));
    assert!(stdout.contains("client [frontend-service] GET inventory"));
    assert!(stdout.contains("server [inventory-service] GET /stock"));
}

#[test]
fn tree_json_outputs_semantic_annotations() {
    let fixture = fixture("otlp-semantic-annotations.json");
    let output = tracelens()
        .args([
            "tree",
            fixture.as_str(),
            "--trace-id",
            "dddddddddddddddddddddddddddddddd",
            "--output",
            "json",
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be json");

    assert_eq!(json["annotations"]["counts"]["client_server_pairs"], 1);
    assert_eq!(json["annotations"]["counts"]["linked_span_count"], 1);
    let nodes = json["nodes"].as_array().expect("nodes should be array");
    assert!(nodes.iter().any(|node| {
        node["annotations"]["client_server_peers"]
            .as_array()
            .is_some_and(|peers| !peers.is_empty())
    }));
}

#[test]
fn critical_path_outputs_semantic_annotations() {
    let fixture = fixture("otlp-semantic-annotations.json");
    let output = tracelens()
        .args([
            "--color",
            "never",
            "critical-path",
            fixture.as_str(),
            "--trace-id",
            "dddddddddddddddddddddddddddddddd",
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("Span 语义标注"));
    assert!(
        stdout.contains(
            "client/server pairs: 1  async spans: 2  linked spans: 1  messaging spans: 2"
        )
    );
    assert!(stdout.contains("async / linked span："));
    assert!(stdout.contains("publish checkout event"));
    assert!(stdout.contains("consume checkout event"));
    assert!(
        stdout.contains("links=[dddddddddddddddddddddddddddddddd:1000000000000004(current-trace)]")
    );
}

#[test]
fn critical_path_json_outputs_semantic_annotations() {
    let fixture = fixture("otlp-semantic-annotations.json");
    let output = tracelens()
        .args([
            "critical-path",
            fixture.as_str(),
            "--trace-id",
            "dddddddddddddddddddddddddddddddd",
            "--output",
            "json",
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(!contains_ansi(&stdout));
    let json: Value = serde_json::from_str(&stdout).expect("stdout should be valid json");

    assert_eq!(json["annotations"]["counts"]["client_server_pairs"], 1);
    assert_eq!(json["annotations"]["counts"]["async_span_count"], 2);
    assert_eq!(json["annotations"]["counts"]["linked_span_count"], 1);
    assert_eq!(json["annotations"]["counts"]["messaging_span_count"], 2);
    assert_eq!(
        json["annotations"]["client_server_pairs"][0]["client"]["span_id"],
        "1000000000000002"
    );
    assert_eq!(
        json["annotations"]["client_server_pairs"][0]["server"]["span_id"],
        "1000000000000003"
    );
    assert_eq!(
        json["annotations"]["linked_spans"][0]["linked_spans"][0]["span_id"],
        "1000000000000004"
    );
    assert_eq!(
        json["annotations"]["linked_spans"][0]["linked_spans"][0]["target_in_trace"],
        true
    );
    assert_eq!(json["critical_path"]["status"], "available");

    let segment_span_ids = json["critical_path"]["segments"]
        .as_array()
        .expect("segments should be an array")
        .iter()
        .map(|segment| segment["span_id"].as_str().expect("span_id"))
        .collect::<Vec<_>>();
    assert!(segment_span_ids.contains(&"1000000000000002"));
    assert!(segment_span_ids.contains(&"1000000000000003"));
}

#[test]
fn critical_path_reports_selected_root_for_multiple_roots() {
    let fixture = fixture("otlp-multiple-roots.json");
    let output = tracelens()
        .args([
            "critical-path",
            fixture.as_str(),
            "--trace-id",
            "12121212121212121212121212121212",
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("root span duration: 50.000ms"));
    assert!(stdout.contains("span_id=1212121212121212"));
    assert!(stdout.contains("注意：trace 有 2 个 root span"));
    assert!(!stdout.contains("root span duration: unknown"));
}

#[test]
fn critical_path_json_reports_selected_root_for_multiple_roots() {
    let fixture = fixture("otlp-multiple-roots.json");
    let output = tracelens()
        .args([
            "critical-path",
            fixture.as_str(),
            "--trace-id",
            "12121212121212121212121212121212",
            "--output",
            "json",
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let json: Value = serde_json::from_slice(&output.stdout).expect("stdout should be valid json");

    assert_eq!(json["trace"]["root_span"]["span_id"], "1212121212121212");
    assert_eq!(json["trace"]["root_span"]["duration_ns"], 50_000_000_u64);
    assert_eq!(
        json["critical_path"]["root_span"]["span_id"],
        "1212121212121212"
    );
    assert_eq!(json["critical_path"]["root_span_id"], "1212121212121212");
}

#[test]
fn critical_path_reports_unavailable_without_root() {
    let fixture = fixture("otlp-no-root.json");
    let output = tracelens()
        .args([
            "critical-path",
            fixture.as_str(),
            "--trace-id",
            "90909090909090909090909090909090",
            "--output",
            "json",
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    let json: Value = serde_json::from_str(&stdout).expect("stdout should be valid json");

    assert_eq!(json["critical_path"]["status"], "unavailable");
    assert_eq!(
        json["critical_path"]["unavailable_reason"],
        "trace has no root span"
    );
    assert_eq!(
        json["critical_path"]["segments"]
            .as_array()
            .expect("segments should be an array")
            .len(),
        0
    );
}

#[test]
fn critical_path_fails_for_unknown_trace_id() {
    let fixture = fixture("otlp-concurrent.json");
    let output = tracelens()
        .args([
            "critical-path",
            fixture.as_str(),
            "--trace-id",
            "dddddddddddddddddddddddddddddddd",
        ])
        .output()
        .expect("command should run");

    assert_exit_code(&output, EXIT_FAILURE);
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).expect("stderr should be utf8");
    assert!(stderr.contains("trace_id not found"));
}

#[test]
fn color_always_adds_ansi_to_text_output() {
    let fixture = fixture("otlp-basic.json");
    let output = tracelens()
        .args(["--color", "always", "validate", fixture.as_str()])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(contains_ansi(&stdout));
    assert!(stdout.contains("\x1b[32mok\x1b[0m"));
}

#[test]
fn color_never_keeps_text_output_plain() {
    let fixture = fixture("otlp-concurrent.json");
    let output = tracelens()
        .args([
            "--color",
            "never",
            "critical-path",
            fixture.as_str(),
            "--trace-id",
            "CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC",
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(!contains_ansi(&stdout));
    assert!(stdout.contains("关键路径"));
    assert!(stdout.contains("critical path duration: 1.000s"));
}

#[test]
fn json_output_ignores_color_always() {
    let fixture = fixture("otlp-basic.json");
    let output = tracelens()
        .args([
            "--color",
            "always",
            "validate",
            fixture.as_str(),
            "--output",
            "json",
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(!contains_ansi(&stdout));
    let value: Value = serde_json::from_str(&stdout).expect("stdout should be valid json");
    assert_eq!(value["command"], "validate");
}

#[test]
fn no_color_disables_auto_color() {
    let fixture = fixture("otlp-invalid-time.json");
    let output = tracelens()
        .env("NO_COLOR", "1")
        .args(["--color", "auto", "validate", fixture.as_str()])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(!contains_ansi(&stdout));
}

#[test]
fn json_outputs_match_the_published_schema() {
    let basic = fixture("otlp-basic.json");
    let concurrent = fixture("otlp-concurrent.json");
    let detect = fixture("otlp-detect.json");
    let n_plus_one = fixture("otlp-n-plus-one.json");
    let trace_id = "5b8efff798038103d269b633813fc60c";
    let concurrent_trace_id = "cccccccccccccccccccccccccccccccc";

    let commands = [
        vec!["validate", basic.as_str(), "--output", "json"],
        vec!["summary", basic.as_str(), "--output", "json"],
        vec![
            "list-traces",
            basic.as_str(),
            "--limit",
            "2",
            "--output",
            "json",
        ],
        vec![
            "tree",
            basic.as_str(),
            "--trace-id",
            trace_id,
            "--output",
            "json",
        ],
        vec![
            "services",
            basic.as_str(),
            "--trace-id",
            trace_id,
            "--output",
            "json",
        ],
        vec![
            "critical-path",
            concurrent.as_str(),
            "--trace-id",
            concurrent_trace_id,
            "--output",
            "json",
        ],
        vec![
            "timeline",
            concurrent.as_str(),
            "--trace-id",
            concurrent_trace_id,
            "--width",
            "48",
            "--output",
            "json",
        ],
        vec!["detect", detect.as_str(), "--output", "json"],
        vec![
            "detect",
            n_plus_one.as_str(),
            "--limit",
            "5",
            "--output",
            "json",
        ],
    ];

    for command in commands {
        let value = run_json(&command);
        assert_matches_output_schema(&value);
    }
}

#[test]
fn tree_json_preserves_otlp_compatibility_metadata() {
    let fixture = fixture("otlp-compatibility.json");
    let json = run_json(&[
        "tree",
        fixture.as_str(),
        "--trace-id",
        "ABCDEF0123456789ABCDEF0123456789",
        "--output",
        "json",
    ]);

    assert_matches_output_schema(&json);

    let root = json["nodes"]
        .as_array()
        .expect("nodes should be array")
        .iter()
        .find(|node| node["span"]["span_id"] == "abcdefabcdefabcd")
        .expect("root span node should exist");
    let span = &root["span"];

    assert_eq!(span["trace_id"], "abcdef0123456789abcdef0123456789");
    assert_eq!(span["trace_state"], "rojo=00f067aa0ba902b7");
    assert_eq!(span["flags"], 1);
    assert_eq!(span["status_message"], "checkout failed");
    assert_eq!(span["dropped_attributes_count"], 3);
    assert_eq!(
        span["resource_schema_url"],
        "https://opentelemetry.io/schemas/1.28.0"
    );
    assert_eq!(span["scope_name"], "compat.instrumentation");
    assert_eq!(span["scope_version"], "1.2.3");
    assert_eq!(span["scope_attributes"]["scope.mode"], "test");
    assert_eq!(
        span["scope_schema_url"],
        "https://opentelemetry.io/schemas/1.28.0"
    );
    assert_eq!(span["events"][0]["dropped_attributes_count"], 1);
    assert_eq!(span["links"][0]["trace_state"], "link=1");
    assert_eq!(span["links"][0]["flags"], 1);
    assert_eq!(span["links"][0]["dropped_attributes_count"], 2);
    assert_eq!(span["dropped_events_count"], 4);
    assert_eq!(span["dropped_links_count"], 5);

    let request_tags: Value = serde_json::from_str(
        span["attributes"]["request.tags"]
            .as_str()
            .expect("request.tags should be a JSON string"),
    )
    .expect("request.tags should parse as JSON");
    assert_eq!(request_tags, serde_json::json!(["vip", 42, false]));
}

#[test]
fn tree_outputs_cross_service_edges_section() {
    let fixture = fixture("otlp-concurrent.json");
    let output = tracelens()
        .args([
            "--color",
            "never",
            "tree",
            fixture.as_str(),
            "--trace-id",
            "cccccccccccccccccccccccccccccccc",
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("跨服务边"));
    assert!(stdout.contains("checkout-service  →  cart-service  calls=1"));
    assert!(stdout.contains("payment-service  →  postgres  calls=1"));
    assert!(!contains_ansi(&stdout));
}

#[test]
fn services_outputs_cross_service_edges_section() {
    let fixture = fixture("otlp-concurrent.json");
    let output = tracelens()
        .args([
            "--color",
            "never",
            "services",
            fixture.as_str(),
            "--trace-id",
            "cccccccccccccccccccccccccccccccc",
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("跨服务调用边"));
    assert!(stdout.contains("checkout-service  →  cart-service  calls=1"));
    assert!(stdout.contains("payment-service  →  redis  calls=1"));
    assert!(!contains_ansi(&stdout));
}

#[test]
fn tree_json_outputs_cross_service_edges() {
    let fixture = fixture("otlp-concurrent.json");
    let json = run_json(&[
        "tree",
        fixture.as_str(),
        "--trace-id",
        "cccccccccccccccccccccccccccccccc",
        "--output",
        "json",
    ]);

    let edges = json["cross_service_edges"]
        .as_array()
        .expect("cross_service_edges should be an array");
    assert!(
        !edges.is_empty(),
        "concurrent trace should have cross-service edges"
    );
    for edge in edges {
        assert!(edge["from_service"].is_string());
        assert!(edge["to_service"].is_string());
        assert!(edge["span_count"].is_u64());
        assert!(edge["client_server_pair_count"].is_u64());
        assert!(edge["sample_span_id"].is_string());
        assert!(edge["sample_parent_span_id"].is_string());
    }

    let checkout_cart = edges
        .iter()
        .find(|edge| {
            edge["from_service"] == "checkout-service" && edge["to_service"] == "cart-service"
        })
        .expect("checkout-service -> cart-service edge should be present");
    assert_eq!(checkout_cart["span_count"], 1);
    assert_eq!(checkout_cart["client_server_pair_count"], 0);
    assert_eq!(checkout_cart["sample_parent_span_id"], "0000000000000001");
    assert_eq!(checkout_cart["sample_span_id"], "0000000000000002");

    assert_matches_output_schema(&json);
}

#[test]
fn services_json_outputs_cross_service_edges() {
    let fixture = fixture("otlp-concurrent.json");
    let json = run_json(&[
        "services",
        fixture.as_str(),
        "--trace-id",
        "cccccccccccccccccccccccccccccccc",
        "--output",
        "json",
    ]);

    let edges = json["cross_service_edges"]
        .as_array()
        .expect("cross_service_edges should be an array");
    assert!(!edges.is_empty());
    assert!(
        edges.iter().any(|edge| {
            edge["from_service"] == "payment-service" && edge["to_service"] == "redis"
        }),
        "payment-service -> redis edge should be present"
    );

    assert_matches_output_schema(&json);
}

#[test]
fn tree_cross_service_edges_empty_for_single_service_trace() {
    let fixture = fixture("otlp-basic.json");
    let output = tracelens()
        .args([
            "--color",
            "never",
            "tree",
            fixture.as_str(),
            "--trace-id",
            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        ])
        .output()
        .expect("command should run");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    assert!(stdout.contains("跨服务边"));
    assert!(stdout.contains("(no cross-service edges)"));
    assert!(!stdout.contains("calls="));

    let json = run_json(&[
        "tree",
        fixture.as_str(),
        "--trace-id",
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "--output",
        "json",
    ]);
    let edges = json["cross_service_edges"]
        .as_array()
        .expect("cross_service_edges should be an array");
    assert!(edges.is_empty());
    assert_matches_output_schema(&json);
}

#[test]
fn tree_cross_service_edge_counts_client_server_pair() {
    let fixture = fixture("otlp-semantic-annotations.json");
    let json = run_json(&[
        "tree",
        fixture.as_str(),
        "--trace-id",
        "dddddddddddddddddddddddddddddddd",
        "--output",
        "json",
    ]);

    let edges = json["cross_service_edges"]
        .as_array()
        .expect("cross_service_edges should be an array");
    let pair = edges
        .iter()
        .find(|edge| {
            edge["from_service"] == "frontend-service" && edge["to_service"] == "inventory-service"
        })
        .expect("frontend-service -> inventory-service edge should be present");
    assert_eq!(pair["span_count"], 1);
    assert_eq!(pair["client_server_pair_count"], 1);
    assert_eq!(pair["sample_parent_span_id"], "1000000000000002");
    assert_eq!(pair["sample_span_id"], "1000000000000003");

    // The graph-layer client/server pair count matches the annotations
    // client_server_pairs count on this trace: both arrive at 1 because the only
    // client(kind=3) -> server(kind=2) parent-child pair happens to be
    // cross-service (frontend-service -> inventory-service).
    assert_eq!(json["annotations"]["counts"]["client_server_pairs"], 1);

    assert_matches_output_schema(&json);
}

fn run_report(fixture_name: &str, trace_id: &str) -> (String, String) {
    let fixture = fixture(fixture_name);
    let n = REPORT_TMP_COUNTER.fetch_add(1, Ordering::SeqCst);
    let out_path = std::env::temp_dir().join(format!("tracelens-report-{n}.html"));
    let output = tracelens()
        .args([
            "report",
            fixture.as_str(),
            "--trace-id",
            trace_id,
            "--html",
            out_path.to_str().expect("temp path should be utf8"),
        ])
        .output()
        .expect("command should run");
    assert_exit_code(&output, 0);
    let stdout = String::from_utf8(output.stdout).expect("stdout should be utf8");
    let html = std::fs::read_to_string(&out_path)
        .unwrap_or_else(|_| panic!("report html should be written to {:?}", out_path));
    let _ = std::fs::remove_file(&out_path);
    (html, stdout)
}

#[test]
fn report_generates_html_with_four_core_blocks() {
    let (html, stdout) = run_report(
        "otlp-semantic-annotations.json",
        "dddddddddddddddddddddddddddddddd",
    );
    assert!(stdout.contains("wrote"));
    assert!(stdout.contains("dddddddddddddddddddddddddddddddd"));
    assert!(html.starts_with("<!DOCTYPE html>"));
    assert!(html.contains("Trace 概览"));
    assert!(html.contains("服务耗时分布"));
    assert!(html.contains("关键路径"));
    assert!(html.contains("跨服务调用边"));
    // cross-service call edge with a client/server pair renders a row.
    assert!(html.contains("frontend-service"));
    assert!(html.contains("inventory-service"));
    // placeholder blocks are visible but don't render evidence bodies.
    assert!(html.contains("错误传播链"));
    assert!(html.contains("N+1 候选"));
    assert!(html.contains("Diagnostics"));
    // no inline evidence rendered for placeholder blocks yet.
    assert!(!html.contains("<script>"));
}

#[test]
fn report_aggregates_n_plus_one_cross_service_edge() {
    let (html, _stdout) = run_report("otlp-n-plus-one.json", "77777777777777777777777777777777");
    assert!(html.starts_with("<!DOCTYPE html>"));
    assert!(html.contains("跨服务调用边"));
    assert!(html.contains("checkout-service"));
    assert!(html.contains("postgres-service"));
    // the 10 repeated calls collapse into one edge row.
    let count_marker = "<td class=\"num\">10</td>";
    assert!(html.contains(count_marker));
}

#[test]
fn report_empty_cross_service_edges_for_single_service() {
    let (html, _stdout) = run_report(
        "otlp-missing-parent.json",
        "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
    );
    assert!(html.starts_with("<!DOCTYPE html>"));
    assert!(html.contains("跨服务调用边"));
    assert!(html.contains("(no cross-service edges)"));
}

#[test]
fn report_concurrent_trace_renders_critical_path() {
    let (html, _stdout) = run_report("otlp-concurrent.json", "cccccccccccccccccccccccccccccccc");
    assert!(html.starts_with("<!DOCTYPE html>"));
    assert!(html.contains("关键路径"));
    assert!(html.contains("cart-service"));
    assert!(html.contains("notify-service"));
}

#[test]
fn report_n_plus_one_block_renders_real_candidate() {
    let (html, _stdout) = run_report("otlp-n-plus-one.json", "77777777777777777777777777777777");
    assert!(!html.contains("(no n+1 candidates)"));
    assert!(html.contains("repeated="));
    assert!(html.contains("badge-red\">10</span>"));
}

#[test]
fn report_diagnostics_block_warns_on_missing_parent() {
    let (html, _stdout) = run_report(
        "otlp-missing-parent.json",
        "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
    );
    assert!(html.contains("sev-warning"));
    assert!(html.contains("Diagnostics"));
}

#[test]
fn report_error_propagation_renders_on_error_trace() {
    let (html, _stdout) = run_report("otlp-basic.json", "5B8EFFF798038103D269B633813FC60C");
    // payment-service is an error span; the report marks errors in red.
    assert!(html.contains("err-mark"));
    assert!(html.contains("badge-red\">1</span>"));
}

#[test]
fn report_renders_nav_heatmap_and_slow_badge() {
    let (html, _stdout) = run_report("otlp-concurrent.json", "cccccccccccccccccccccccccccccccc");
    assert!(html.contains("<nav class=\"nav\">"));
    assert!(html.contains("critical-seg"));
    assert!(html.contains("慢请求候选"));
    assert!(html.contains("heat-4"));
}
