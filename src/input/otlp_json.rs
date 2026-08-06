use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use serde_json::Value;

use crate::model::diagnostic::Diagnostic;
use crate::model::span::{
    CanonicalSpan, SPAN_ID_LEN, SpanEvent, SpanLink, TRACE_ID_LEN, normalize_hex_id,
};

#[derive(Clone, Debug)]
pub struct ParsedTraceData {
    pub spans: Vec<CanonicalSpan>,
    pub diagnostics: Vec<Diagnostic>,
}

impl ParsedTraceData {
    fn empty() -> Self {
        Self {
            spans: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    fn merge(&mut self, mut other: ParsedTraceData) {
        self.spans.append(&mut other.spans);
        self.diagnostics.append(&mut other.diagnostics);
    }
}

pub fn parse_otlp_file(path: &Path) -> Result<ParsedTraceData> {
    let contents =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;

    match serde_json::from_str::<Value>(&contents) {
        Ok(value) => Ok(parse_otlp_value(&value, None)),
        Err(json_error) => Ok(parse_jsonl(&contents, &json_error)),
    }
}

fn parse_jsonl(contents: &str, json_error: &serde_json::Error) -> ParsedTraceData {
    let mut data = ParsedTraceData::empty();
    let mut saw_line = false;

    for (index, line) in contents.lines().enumerate() {
        let line_number = index + 1;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        saw_line = true;
        match serde_json::from_str::<Value>(trimmed) {
            Ok(value) => data.merge(parse_otlp_value(&value, Some(line_number))),
            Err(error) => data.diagnostics.push(
                Diagnostic::error(
                    "malformed_jsonl_line",
                    format!("failed to parse JSONL line {line_number}: {error}"),
                )
                .with_location(format!("line {line_number}")),
            ),
        }
    }

    if !saw_line {
        data.diagnostics.push(Diagnostic::error(
            "empty_input",
            format!("input is empty or not valid OTLP JSON: {json_error}"),
        ));
    }

    data
}

fn parse_otlp_value(value: &Value, line_number: Option<usize>) -> ParsedTraceData {
    let mut data = ParsedTraceData::empty();
    let location_prefix = line_number
        .map(|line| format!("line {line}"))
        .unwrap_or_else(|| "root".to_string());

    let Some(resource_spans) = value.get("resourceSpans").and_then(Value::as_array) else {
        data.diagnostics.push(
            Diagnostic::error("missing_resource_spans", "missing resourceSpans array")
                .with_location(location_prefix),
        );
        return data;
    };

    for (resource_index, resource_span) in resource_spans.iter().enumerate() {
        let resource_location = format!("{location_prefix}.resourceSpans[{resource_index}]");
        let resource_attributes = resource_span
            .get("resource")
            .and_then(|resource| resource.get("attributes"))
            .map(parse_attributes)
            .unwrap_or_default();
        let service_name = resource_attributes
            .get("service.name")
            .cloned()
            .unwrap_or_else(|| {
                data.diagnostics.push(
                    Diagnostic::warning(
                        "missing_service_name",
                        "resource is missing service.name; using unknown-service",
                    )
                    .with_location(resource_location.clone()),
                );
                "unknown-service".to_string()
            });

        let Some(scope_spans) = resource_span.get("scopeSpans").and_then(Value::as_array) else {
            data.diagnostics.push(
                Diagnostic::warning(
                    "missing_scope_spans",
                    "resourceSpan is missing scopeSpans array",
                )
                .with_location(resource_location),
            );
            continue;
        };

        for (scope_index, scope_span) in scope_spans.iter().enumerate() {
            let scope_location = format!(
                "{location_prefix}.resourceSpans[{resource_index}].scopeSpans[{scope_index}]"
            );
            let scope_name = scope_span
                .get("scope")
                .and_then(|scope| scope.get("name"))
                .and_then(Value::as_str)
                .map(ToString::to_string);
            let scope_version = scope_span
                .get("scope")
                .and_then(|scope| scope.get("version"))
                .and_then(Value::as_str)
                .map(ToString::to_string);

            let Some(spans) = scope_span.get("spans").and_then(Value::as_array) else {
                data.diagnostics.push(
                    Diagnostic::warning("missing_spans", "scopeSpan is missing spans array")
                        .with_location(scope_location),
                );
                continue;
            };

            for (span_index, span_value) in spans.iter().enumerate() {
                let span_location = format!(
                    "{location_prefix}.resourceSpans[{resource_index}].scopeSpans[{scope_index}].spans[{span_index}]"
                );
                if let Some(span) = parse_span(
                    span_value,
                    &service_name,
                    &resource_attributes,
                    scope_name.clone(),
                    scope_version.clone(),
                    &span_location,
                    &mut data.diagnostics,
                ) {
                    data.spans.push(span);
                }
            }
        }
    }

    data
}

fn parse_span(
    value: &Value,
    service_name: &str,
    resource_attributes: &BTreeMap<String, String>,
    scope_name: Option<String>,
    scope_version: Option<String>,
    location: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<CanonicalSpan> {
    let raw_trace_id = required_string(value, "traceId", location, diagnostics)?;
    let trace_id = match normalize_hex_id(raw_trace_id, TRACE_ID_LEN) {
        Ok(trace_id) => trace_id,
        Err(message) => {
            diagnostics.push(
                Diagnostic::error("invalid_trace_id", format!("invalid traceId: {message}"))
                    .with_location(location),
            );
            return None;
        }
    };

    let raw_span_id = required_string(value, "spanId", location, diagnostics)?;
    let span_id = match normalize_hex_id(raw_span_id, SPAN_ID_LEN) {
        Ok(span_id) => span_id,
        Err(message) => {
            diagnostics.push(
                Diagnostic::error("invalid_span_id", format!("invalid spanId: {message}"))
                    .with_trace_id(trace_id)
                    .with_location(location),
            );
            return None;
        }
    };

    let parent_span_id = value
        .get("parentSpanId")
        .and_then(Value::as_str)
        .and_then(|raw_parent| {
            let trimmed = raw_parent.trim();
            if trimmed.is_empty() {
                None
            } else {
                match normalize_hex_id(trimmed, SPAN_ID_LEN) {
                    Ok(parent_span_id) => Some(parent_span_id),
                    Err(message) => {
                        diagnostics.push(
                            Diagnostic::error(
                                "invalid_parent_span_id",
                                format!("invalid parentSpanId: {message}"),
                            )
                            .with_trace_id(trace_id.clone())
                            .with_span_id(span_id.clone())
                            .with_location(location),
                        );
                        None
                    }
                }
            }
        });

    let name = value
        .get("name")
        .and_then(Value::as_str)
        .filter(|name| !name.trim().is_empty())
        .unwrap_or("<unnamed>")
        .to_string();

    let start_ns = parse_required_timestamp(value, "startTimeUnixNano", location, diagnostics)?;
    let end_ns = parse_required_timestamp(value, "endTimeUnixNano", location, diagnostics)?;

    if end_ns < start_ns {
        diagnostics.push(
            Diagnostic::error(
                "invalid_time_range",
                "endTimeUnixNano is earlier than startTimeUnixNano",
            )
            .with_trace_id(trace_id)
            .with_span_id(span_id)
            .with_location(location),
        );
        return None;
    }

    Some(CanonicalSpan {
        trace_id,
        span_id,
        parent_span_id,
        service_name: service_name.to_string(),
        name,
        kind: value.get("kind").and_then(parse_i64),
        start_ns,
        end_ns,
        status_code: value
            .get("status")
            .and_then(|status| status.get("code"))
            .and_then(parse_status_code),
        attributes: value
            .get("attributes")
            .map(parse_attributes)
            .unwrap_or_default(),
        resource_attributes: resource_attributes.clone(),
        scope_name,
        scope_version,
        events: value.get("events").map(parse_events).unwrap_or_default(),
        links: value.get("links").map(parse_links).unwrap_or_default(),
    })
}

fn required_string<'a>(
    value: &'a Value,
    field: &'static str,
    location: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<&'a str> {
    match value.get(field).and_then(Value::as_str) {
        Some(raw) if !raw.trim().is_empty() => Some(raw),
        _ => {
            diagnostics.push(
                Diagnostic::error(
                    "missing_required_field",
                    format!("missing required field {field}"),
                )
                .with_location(location),
            );
            None
        }
    }
}

fn parse_required_timestamp(
    value: &Value,
    field: &'static str,
    location: &str,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<u64> {
    match value.get(field).and_then(parse_u64) {
        Some(timestamp) => Some(timestamp),
        None => {
            diagnostics.push(
                Diagnostic::error("invalid_timestamp", format!("invalid or missing {field}"))
                    .with_location(location),
            );
            None
        }
    }
}

fn parse_attributes(value: &Value) -> BTreeMap<String, String> {
    let mut attributes = BTreeMap::new();
    let Some(items) = value.as_array() else {
        return attributes;
    };

    for item in items {
        let Some(key) = item.get("key").and_then(Value::as_str) else {
            continue;
        };
        let Some(value) = item.get("value").and_then(any_value_to_string) else {
            continue;
        };

        attributes.insert(key.to_string(), value);
    }

    attributes
}

fn parse_events(value: &Value) -> Vec<SpanEvent> {
    let Some(items) = value.as_array() else {
        return Vec::new();
    };

    items
        .iter()
        .map(|item| SpanEvent {
            name: item
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("<unnamed>")
                .to_string(),
            time_unix_nano: item.get("timeUnixNano").and_then(parse_u64),
            attributes: item
                .get("attributes")
                .map(parse_attributes)
                .unwrap_or_default(),
        })
        .collect()
}

fn parse_links(value: &Value) -> Vec<SpanLink> {
    let Some(items) = value.as_array() else {
        return Vec::new();
    };

    items
        .iter()
        .map(|item| SpanLink {
            trace_id: item
                .get("traceId")
                .and_then(Value::as_str)
                .map(|value| value.trim().to_ascii_lowercase()),
            span_id: item
                .get("spanId")
                .and_then(Value::as_str)
                .map(|value| value.trim().to_ascii_lowercase()),
            attributes: item
                .get("attributes")
                .map(parse_attributes)
                .unwrap_or_default(),
        })
        .collect()
}

fn any_value_to_string(value: &Value) -> Option<String> {
    if let Some(value) = value.get("stringValue").and_then(Value::as_str) {
        return Some(value.to_string());
    }
    if let Some(value) = value.get("intValue").and_then(parse_i64) {
        return Some(value.to_string());
    }
    if let Some(value) = value.get("doubleValue").and_then(Value::as_f64) {
        return Some(value.to_string());
    }
    if let Some(value) = value.get("boolValue").and_then(Value::as_bool) {
        return Some(value.to_string());
    }
    if let Some(value) = value.get("bytesValue").and_then(Value::as_str) {
        return Some(value.to_string());
    }

    None
}

fn parse_u64(value: &Value) -> Option<u64> {
    match value {
        Value::Number(number) => number.as_u64(),
        Value::String(text) => text.parse::<u64>().ok(),
        _ => None,
    }
}

fn parse_i64(value: &Value) -> Option<i64> {
    match value {
        Value::Number(number) => number.as_i64(),
        Value::String(text) => text.parse::<i64>().ok(),
        _ => None,
    }
}

fn parse_status_code(value: &Value) -> Option<i64> {
    if let Some(code) = parse_i64(value) {
        return Some(code);
    }

    let text = value.as_str()?.trim();
    match text {
        "STATUS_CODE_UNSET" | "UNSET" => Some(0),
        "STATUS_CODE_OK" | "OK" => Some(1),
        "STATUS_CODE_ERROR" | "ERROR" => Some(2),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::parse_otlp_file;
    use std::path::Path;

    #[test]
    fn parses_basic_otlp_json() {
        let data = parse_otlp_file(Path::new("tests/fixtures/otlp-basic.json"))
            .expect("fixture should parse");

        assert_eq!(data.spans.len(), 4);
        assert!(
            data.spans
                .iter()
                .any(|span| span.service_name == "checkout-service")
        );
    }

    #[test]
    fn reports_invalid_time_range() {
        let data = parse_otlp_file(Path::new("tests/fixtures/otlp-invalid-time.json"))
            .expect("fixture should parse with diagnostics");

        assert!(data.spans.is_empty());
        assert!(
            data.diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "invalid_time_range")
        );
    }

    #[test]
    fn parses_jsonl_and_preserves_scope_events_and_links() {
        let data = parse_otlp_file(Path::new("tests/fixtures/otlp-basic.jsonl"))
            .expect("fixture should parse");

        assert_eq!(data.spans.len(), 2);
        assert!(data.diagnostics.is_empty());

        let root = data
            .spans
            .iter()
            .find(|span| span.span_id == "8888888888888888")
            .expect("root span should exist");
        assert_eq!(root.scope_name.as_deref(), Some("test.frontend"));
        assert_eq!(root.scope_version.as_deref(), Some("1.0.0"));
        assert_eq!(
            root.resource_attributes.get("service.name").unwrap(),
            "frontend-service"
        );
        assert_eq!(root.events.len(), 1);

        let child = data
            .spans
            .iter()
            .find(|span| span.span_id == "9999999999999999")
            .expect("child span should exist");
        assert_eq!(child.links.len(), 1);
    }

    #[test]
    fn parses_jsonl_with_empty_lines() {
        let data = parse_otlp_file(Path::new(
            "tests/fixtures/otlp-jsonl-with-empty-lines.jsonl",
        ))
        .expect("fixture should parse");

        assert_eq!(data.spans.len(), 2);
        assert!(data.diagnostics.is_empty());
    }

    #[test]
    fn reports_jsonl_invalid_line_and_keeps_valid_spans() {
        let data = parse_otlp_file(Path::new("tests/fixtures/otlp-jsonl-invalid-line.jsonl"))
            .expect("fixture should parse with diagnostics");

        assert_eq!(data.spans.len(), 2);
        assert!(
            data.diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "malformed_jsonl_line"
                    && diagnostic.location.as_deref() == Some("line 2"))
        );
    }
}
