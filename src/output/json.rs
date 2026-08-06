use serde_json::{Value, json};

use crate::analysis::duration::{RootSpanDuration, ServiceDuration, TraceDurationAnalysis};
use crate::analysis::summary::{FileSummary, TraceSummary};
use crate::graph::trace_graph::{TraceCollection, TraceGraph};
use crate::model::diagnostic::Diagnostic;
use crate::model::span::{CanonicalSpan, SpanEvent, SpanLink};

const SCHEMA_VERSION: &str = "0.1";

pub fn format_validate_json(collection: &TraceCollection, strict: bool) -> String {
    let error_count = collection
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity.to_string() == "error")
        .count();
    let exit_would_fail = strict && error_count > 0;
    let status = if exit_would_fail { "failed" } else { "ok" };

    to_pretty(json!({
        "schema_version": SCHEMA_VERSION,
        "command": "validate",
        "mode": if strict { "strict" } else { "default" },
        "status": status,
        "error_diagnostic_count": error_count,
        "exit_would_fail": exit_would_fail,
        "trace_count": collection.traces.len(),
        "span_count": collection.span_count(),
        "diagnostics": diagnostics_to_json(&collection.diagnostics),
    }))
}

pub fn format_summary_json(summary: &FileSummary, collection: &TraceCollection) -> String {
    to_pretty(json!({
        "schema_version": SCHEMA_VERSION,
        "command": "summary",
        "summary": {
            "trace_count": summary.trace_count,
            "span_count": summary.span_count,
            "service_count": summary.service_count,
            "error_span_count": summary.error_span_count,
            "start_ns": summary.start_ns,
            "end_ns": summary.end_ns,
            "duration_ns": duration(summary.start_ns, summary.end_ns),
        },
        "slowest_traces": trace_summaries_to_json(&summary.slowest_traces),
        "diagnostics": diagnostics_to_json(&collection.diagnostics),
    }))
}

pub fn format_list_traces_json(traces: &[TraceSummary], limit: usize) -> String {
    let limited: Vec<Value> = traces
        .iter()
        .take(limit)
        .map(trace_summary_to_json)
        .collect();

    to_pretty(json!({
        "schema_version": SCHEMA_VERSION,
        "command": "list-traces",
        "limit": limit,
        "traces": limited,
    }))
}

pub fn format_tree_json(trace: &TraceGraph) -> String {
    to_pretty(json!({
        "schema_version": SCHEMA_VERSION,
        "command": "tree",
        "trace": {
            "trace_id": trace.trace_id,
            "duration_ns": trace.duration_ns(),
            "span_count": trace.spans.len(),
            "root_count": trace.root_indices.len(),
            "orphan_count": trace.orphan_indices.len(),
            "duplicate_span_id_count": trace.duplicate_span_ids.len(),
            "diagnostics_count": trace.diagnostics.len(),
        },
        "nodes": tree_nodes_to_json(trace),
        "diagnostics": diagnostics_to_json(&trace.diagnostics),
    }))
}

pub fn format_services_json(analysis: &TraceDurationAnalysis, trace: &TraceGraph) -> String {
    to_pretty(json!({
        "schema_version": SCHEMA_VERSION,
        "command": "services",
        "trace": {
            "trace_id": analysis.trace_id,
            "wall_clock_duration_ns": analysis.wall_clock_duration_ns,
            "root_span": analysis.root_span.as_ref().map(root_span_to_json),
            "root_count": analysis.root_count,
            "orphan_count": analysis.orphan_count,
            "diagnostics_count": analysis.diagnostics_count,
        },
        "services": analysis
            .services
            .iter()
            .map(service_duration_to_json)
            .collect::<Vec<_>>(),
        "diagnostics": diagnostics_to_json(&trace.diagnostics),
    }))
}

fn root_span_to_json(root: &RootSpanDuration) -> Value {
    json!({
        "span_id": root.span_id,
        "service_name": root.service_name,
        "name": root.name,
        "duration_ns": root.duration_ns,
    })
}

fn service_duration_to_json(service: &ServiceDuration) -> Value {
    json!({
        "service_name": service.service_name,
        "self_time_ns": service.self_time_ns,
        "span_time_ns": service.span_time_ns,
        "child_covered_time_ns": service.child_covered_time_ns,
        "span_count": service.span_count,
        "error_span_count": service.error_span_count,
    })
}

fn tree_nodes_to_json(trace: &TraceGraph) -> Vec<Value> {
    let mut nodes = Vec::new();
    let mut visited = vec![false; trace.spans.len()];

    for index in &trace.root_indices {
        push_tree_node(trace, *index, 0, &mut visited, &mut nodes);
    }

    for index in &trace.orphan_indices {
        push_tree_node(trace, *index, 0, &mut visited, &mut nodes);
    }

    for index in 0..trace.spans.len() {
        if !visited[index] {
            push_tree_node(trace, index, 0, &mut visited, &mut nodes);
        }
    }

    nodes
}

fn push_tree_node(
    trace: &TraceGraph,
    index: usize,
    depth: usize,
    visited: &mut [bool],
    nodes: &mut Vec<Value>,
) {
    if visited[index] {
        return;
    }
    visited[index] = true;

    let span = &trace.spans[index];
    nodes.push(json!({
        "depth": depth,
        "span": span_to_json(span),
    }));

    if let Some(children) = trace.children_by_parent.get(&span.span_id) {
        for child_index in children {
            push_tree_node(trace, *child_index, depth + 1, visited, nodes);
        }
    }
}

fn trace_summaries_to_json(traces: &[TraceSummary]) -> Vec<Value> {
    traces.iter().map(trace_summary_to_json).collect()
}

fn trace_summary_to_json(trace: &TraceSummary) -> Value {
    json!({
        "trace_id": trace.trace_id,
        "span_count": trace.span_count,
        "service_count": trace.service_count,
        "error_span_count": trace.error_span_count,
        "root_count": trace.root_count,
        "orphan_count": trace.orphan_count,
        "diagnostics_count": trace.diagnostics_count,
        "start_ns": trace.start_ns,
        "end_ns": trace.end_ns,
        "duration_ns": trace.duration_ns,
    })
}

fn span_to_json(span: &CanonicalSpan) -> Value {
    json!({
        "trace_id": span.trace_id,
        "span_id": span.span_id,
        "parent_span_id": span.parent_span_id,
        "service_name": span.service_name,
        "name": span.name,
        "kind": span.kind,
        "kind_label": span.kind_label(),
        "start_ns": span.start_ns,
        "end_ns": span.end_ns,
        "duration_ns": span.duration_ns(),
        "status_code": span.status_code,
        "status_label": span.status_label(),
        "is_error": span.is_error(),
        "attributes": span.attributes,
        "resource_attributes": span.resource_attributes,
        "scope_name": span.scope_name,
        "scope_version": span.scope_version,
        "events": span.events.iter().map(event_to_json).collect::<Vec<_>>(),
        "links": span.links.iter().map(link_to_json).collect::<Vec<_>>(),
    })
}

fn event_to_json(event: &SpanEvent) -> Value {
    json!({
        "name": event.name,
        "time_unix_nano": event.time_unix_nano,
        "attributes": event.attributes,
    })
}

fn link_to_json(link: &SpanLink) -> Value {
    json!({
        "trace_id": link.trace_id,
        "span_id": link.span_id,
        "attributes": link.attributes,
    })
}

fn diagnostics_to_json(diagnostics: &[Diagnostic]) -> Vec<Value> {
    diagnostics
        .iter()
        .map(|diagnostic| {
            json!({
                "scope": diagnostic.scope.to_string(),
                "severity": diagnostic.severity.to_string(),
                "code": diagnostic.code,
                "message": diagnostic.message,
                "trace_id": diagnostic.trace_id,
                "span_id": diagnostic.span_id,
                "location": diagnostic.location,
            })
        })
        .collect()
}

fn duration(start_ns: Option<u64>, end_ns: Option<u64>) -> Option<u64> {
    Some(end_ns?.saturating_sub(start_ns?))
}

fn to_pretty(value: Value) -> String {
    let mut output = serde_json::to_string_pretty(&value).expect("JSON value should serialize");
    output.push('\n');
    output
}
