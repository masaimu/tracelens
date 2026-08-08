use serde_json::{Value, json};

use crate::analysis::annotations::{
    ClientServerPair, LinkedSpanRef, SpanAnnotation, TraceAnnotations,
};
use crate::analysis::classification::{SpanClassification, TraceClassification};
use crate::analysis::critical_path::{
    CriticalPathAnalysis, CriticalPathRootSpan, CriticalPathSegment, CriticalPathSpanTotal,
    CriticalPathStatus,
};
use crate::analysis::detect::{
    DetectAnalysis, ErrorPropagationChain, ErrorPropagationStep, ErrorSpanCandidate,
    ErrorTraceCandidate, NPlusOneCandidate, NPlusOneChildGroup, NPlusOneSpanRef,
    ServiceLatencyDistribution, ServiceLatencySpanSample, ServiceSlowCandidate, SlowTraceCandidate,
};
use crate::analysis::duration::{RootSpanDuration, ServiceDuration, TraceDurationAnalysis};
use crate::analysis::summary::{FileSummary, TraceSummary};
use crate::analysis::timeline::{TimelineAnalysis, TimelineRow};
use crate::graph::trace_graph::{CrossServiceEdge, TraceCollection, TraceGraph};
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

pub fn format_tree_json(trace: &TraceGraph, annotations: &TraceAnnotations) -> String {
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
        "nodes": tree_nodes_to_json(trace, annotations),
        "annotations": trace_annotations_to_json(annotations),
        "cross_service_edges": trace
            .cross_service_edges
            .iter()
            .map(cross_service_edge_to_json)
            .collect::<Vec<_>>(),
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
        "cross_service_edges": trace
            .cross_service_edges
            .iter()
            .map(cross_service_edge_to_json)
            .collect::<Vec<_>>(),
        "diagnostics": diagnostics_to_json(&trace.diagnostics),
    }))
}

pub fn format_detect_json(analysis: &DetectAnalysis, collection: &TraceCollection) -> String {
    to_pretty(json!({
        "schema_version": SCHEMA_VERSION,
        "command": "detect",
        "limit": analysis.limit,
        "summary": {
            "trace_count": analysis.summary.trace_count,
            "span_count": analysis.summary.span_count,
            "diagnostics_count": analysis.summary.diagnostics_count,
            "sample_count": analysis.summary.sample_count,
            "sample_quality": analysis.summary.sample_quality.label(),
            "p95_duration_ns": analysis.summary.p95_duration_ns,
            "slow_trace_candidate_count": analysis.summary.slow_trace_candidate_count,
            "error_trace_candidate_count": analysis.summary.error_trace_candidate_count,
            "error_propagation_chain_count": analysis.summary.error_propagation_chain_count,
            "n_plus_one_candidate_count": analysis.summary.n_plus_one_candidate_count,
            "service_latency_distribution_count": analysis.summary.service_latency_distribution_count,
            "error_span_count": analysis.summary.error_span_count,
        },
        "slow_traces": analysis
            .slow_traces
            .iter()
            .map(slow_trace_candidate_to_json)
            .collect::<Vec<_>>(),
        "error_traces": analysis
            .error_traces
            .iter()
            .map(error_trace_candidate_to_json)
            .collect::<Vec<_>>(),
        "error_propagation_chains": analysis
            .error_propagation_chains
            .iter()
            .map(error_propagation_chain_to_json)
            .collect::<Vec<_>>(),
        "n_plus_one_candidates": analysis
            .n_plus_one_candidates
            .iter()
            .map(n_plus_one_candidate_to_json)
            .collect::<Vec<_>>(),
        "service_latency_distribution": analysis
            .service_latency_distribution
            .iter()
            .map(service_latency_distribution_to_json)
            .collect::<Vec<_>>(),
        "notes": analysis.notes,
        "diagnostics": diagnostics_to_json(&collection.diagnostics),
    }))
}

pub fn format_critical_path_json(
    duration: &TraceDurationAnalysis,
    critical_path: &CriticalPathAnalysis,
    classification: &TraceClassification,
    annotations: &TraceAnnotations,
    trace: &TraceGraph,
) -> String {
    let (unavailable_reason, status_label) = match &critical_path.status {
        CriticalPathStatus::Available => (None, critical_path.status.label()),
        CriticalPathStatus::Unavailable { reason } => {
            (Some(reason.as_str()), critical_path.status.label())
        }
    };

    to_pretty(json!({
        "schema_version": SCHEMA_VERSION,
        "command": "critical-path",
        "trace": {
            "trace_id": duration.trace_id,
            "wall_clock_duration_ns": duration.wall_clock_duration_ns,
            "root_span": critical_path.root_span.as_ref().map(critical_path_root_span_to_json),
            "root_count": duration.root_count,
            "orphan_count": duration.orphan_count,
            "diagnostics_count": duration.diagnostics_count,
        },
        "critical_path": {
            "status": status_label,
            "unavailable_reason": unavailable_reason,
            "root_span_id": critical_path.root_span_id,
            "root_span": critical_path.root_span.as_ref().map(critical_path_root_span_to_json),
            "total_duration_ns": critical_path.total_duration_ns,
            "segments": critical_path
                .segments
                .iter()
                .map(critical_path_segment_to_json)
                .collect::<Vec<_>>(),
            "span_totals": critical_path
                .span_totals
                .iter()
                .map(critical_path_span_total_to_json)
                .collect::<Vec<_>>(),
            "notes": critical_path.notes,
        },
        "classification": {
            "counts": {
                "serial": classification.counts.serial,
                "concurrent": classification.counts.concurrent,
                "nested": classification.counts.nested,
                "suspicious": classification.counts.suspicious,
            },
            "spans": classification
                .spans
                .iter()
                .map(span_classification_to_json)
                .collect::<Vec<_>>(),
        },
        "annotations": trace_annotations_to_json(annotations),
        "diagnostics": diagnostics_to_json(&trace.diagnostics),
    }))
}

pub fn format_timeline_json(
    timeline: &TimelineAnalysis,
    critical_path: &CriticalPathAnalysis,
    trace: &TraceGraph,
) -> String {
    let (unavailable_reason, status_label) = match &critical_path.status {
        CriticalPathStatus::Available => (None, critical_path.status.label()),
        CriticalPathStatus::Unavailable { reason } => {
            (Some(reason.as_str()), critical_path.status.label())
        }
    };

    to_pretty(json!({
        "schema_version": SCHEMA_VERSION,
        "command": "timeline",
        "trace": {
            "trace_id": timeline.trace_id,
            "start_ns": timeline.start_ns,
            "end_ns": timeline.end_ns,
            "wall_clock_duration_ns": timeline.duration_ns,
            "span_count": trace.spans.len(),
            "root_count": trace.root_indices.len(),
            "orphan_count": trace.orphan_indices.len(),
            "diagnostics_count": trace.diagnostics.len(),
        },
        "timeline": {
            "mode": timeline.mode.label(),
            "width": timeline.width,
            "rows": timeline
                .rows
                .iter()
                .map(|row| timeline_row_to_json(row, timeline.mode.label()))
                .collect::<Vec<_>>(),
            "collapsed": {
                "enabled": timeline.collapsed.enabled,
                "max_rows": timeline.collapsed.max_rows,
                "omitted_rows": timeline.collapsed.omitted_rows,
                "preserved_reasons": timeline.collapsed.preserved_reasons,
            },
            "notes": timeline.notes,
        },
        "critical_path": {
            "status": status_label,
            "unavailable_reason": unavailable_reason,
            "root_span_id": critical_path.root_span_id,
            "total_duration_ns": critical_path.total_duration_ns,
            "notes": critical_path.notes,
        },
        "diagnostics": diagnostics_to_json(&trace.diagnostics),
    }))
}

fn slow_trace_candidate_to_json(candidate: &SlowTraceCandidate) -> Value {
    json!({
        "trace_id": candidate.trace_id,
        "rank": candidate.rank,
        "duration_ns": candidate.duration_ns,
        "p95_duration_ns": candidate.p95_duration_ns,
        "sample_count": candidate.sample_count,
        "confidence": candidate.confidence.label(),
        "reason": candidate.reason,
        "span_count": candidate.span_count,
        "service_count": candidate.service_count,
        "error_span_count": candidate.error_span_count,
        "diagnostics_count": candidate.diagnostics_count,
        "service_candidates": candidate
            .service_candidates
            .iter()
            .map(service_slow_candidate_to_json)
            .collect::<Vec<_>>(),
    })
}

fn service_slow_candidate_to_json(candidate: &ServiceSlowCandidate) -> Value {
    json!({
        "service_name": candidate.service_name,
        "span_time_ns": candidate.span_time_ns,
        "max_span_duration_ns": candidate.max_span_duration_ns,
        "span_count": candidate.span_count,
        "error_span_count": candidate.error_span_count,
    })
}

fn error_trace_candidate_to_json(candidate: &ErrorTraceCandidate) -> Value {
    json!({
        "trace_id": candidate.trace_id,
        "error_span_count": candidate.error_span_count,
        "confidence": candidate.confidence.label(),
        "earliest_error_span": error_span_candidate_to_json(&candidate.earliest_error_span),
        "top_error_span": error_span_candidate_to_json(&candidate.top_error_span),
        "error_spans": candidate
            .error_spans
            .iter()
            .map(error_span_candidate_to_json)
            .collect::<Vec<_>>(),
        "explanation": candidate.explanation,
    })
}

fn error_span_candidate_to_json(candidate: &ErrorSpanCandidate) -> Value {
    json!({
        "span_id": candidate.span_id,
        "parent_span_id": candidate.parent_span_id,
        "service_name": candidate.service_name,
        "name": candidate.name,
        "depth": candidate.depth,
        "start_ns": candidate.start_ns,
        "duration_ns": candidate.duration_ns,
        "signals": candidate.signals,
    })
}

fn error_propagation_chain_to_json(chain: &ErrorPropagationChain) -> Value {
    json!({
        "trace_id": chain.trace_id,
        "confidence": chain.confidence.label(),
        "earliest_error_span": error_span_candidate_to_json(&chain.earliest_error_span),
        "top_error_span": error_span_candidate_to_json(&chain.top_error_span),
        "path_to_earliest_error": chain
            .path_to_earliest_error
            .iter()
            .map(error_propagation_step_to_json)
            .collect::<Vec<_>>(),
        "downstream_error_spans": chain
            .downstream_error_spans
            .iter()
            .map(error_propagation_step_to_json)
            .collect::<Vec<_>>(),
        "downstream_error_span_count": chain.downstream_error_span_count,
        "affected_span_count": chain.affected_span_count,
        "affected_services": chain.affected_services,
        "explanation": chain.explanation,
    })
}

fn error_propagation_step_to_json(step: &ErrorPropagationStep) -> Value {
    json!({
        "span_id": step.span_id,
        "parent_span_id": step.parent_span_id,
        "service_name": step.service_name,
        "name": step.name,
        "depth": step.depth,
        "start_ns": step.start_ns,
        "duration_ns": step.duration_ns,
        "is_error": step.is_error,
        "signals": step.signals,
    })
}

fn n_plus_one_candidate_to_json(candidate: &NPlusOneCandidate) -> Value {
    json!({
        "trace_id": candidate.trace_id,
        "parent_span": n_plus_one_span_ref_to_json(&candidate.parent_span),
        "child_group": n_plus_one_child_group_to_json(&candidate.child_group),
        "repeated_count": candidate.repeated_count,
        "serial_ratio": candidate.serial_ratio_per_mille as f64 / 1_000.0,
        "serial_ratio_per_mille": candidate.serial_ratio_per_mille,
        "confidence": candidate.confidence.label(),
        "reason": candidate.reason,
        "example_child_spans": candidate
            .example_child_spans
            .iter()
            .map(n_plus_one_span_ref_to_json)
            .collect::<Vec<_>>(),
    })
}

fn n_plus_one_child_group_to_json(group: &NPlusOneChildGroup) -> Value {
    json!({
        "service_name": group.service_name,
        "normalized_name": group.normalized_name,
        "db_system": group.db_system,
        "db_operation": group.db_operation,
        "rpc_system": group.rpc_system,
        "http_method": group.http_method,
        "http_route": group.http_route,
        "signature": group.signature,
    })
}

fn n_plus_one_span_ref_to_json(span: &NPlusOneSpanRef) -> Value {
    json!({
        "span_id": span.span_id,
        "parent_span_id": span.parent_span_id,
        "service_name": span.service_name,
        "name": span.name,
        "depth": span.depth,
        "start_ns": span.start_ns,
        "duration_ns": span.duration_ns,
    })
}

fn service_latency_distribution_to_json(distribution: &ServiceLatencyDistribution) -> Value {
    json!({
        "service_name": distribution.service_name,
        "trace_count": distribution.trace_count,
        "span_count": distribution.span_count,
        "error_span_count": distribution.error_span_count,
        "total_span_time_ns": distribution.total_span_time_ns,
        "p50_duration_ns": distribution.p50_duration_ns,
        "p95_duration_ns": distribution.p95_duration_ns,
        "max_span_duration_ns": distribution.max_span_duration_ns,
        "slow_span_samples": distribution
            .slow_span_samples
            .iter()
            .map(service_latency_span_sample_to_json)
            .collect::<Vec<_>>(),
    })
}

fn service_latency_span_sample_to_json(sample: &ServiceLatencySpanSample) -> Value {
    json!({
        "trace_id": sample.trace_id,
        "span_id": sample.span_id,
        "parent_span_id": sample.parent_span_id,
        "name": sample.name,
        "start_ns": sample.start_ns,
        "duration_ns": sample.duration_ns,
        "is_error": sample.is_error,
        "signals": sample.signals,
    })
}

fn critical_path_segment_to_json(segment: &CriticalPathSegment) -> Value {
    json!({
        "span_id": segment.span_id,
        "service_name": segment.service_name,
        "name": segment.name,
        "offset_ns": segment.offset_ns,
        "duration_ns": segment.duration_ns,
    })
}

fn critical_path_span_total_to_json(total: &CriticalPathSpanTotal) -> Value {
    json!({
        "span_id": total.span_id,
        "service_name": total.service_name,
        "name": total.name,
        "total_ns": total.total_ns,
    })
}

fn critical_path_root_span_to_json(root: &CriticalPathRootSpan) -> Value {
    json!({
        "span_id": root.span_id,
        "service_name": root.service_name,
        "name": root.name,
        "duration_ns": root.duration_ns,
    })
}

fn timeline_row_to_json(row: &TimelineRow, mode: &str) -> Value {
    json!({
        "depth": row.depth,
        "span_id": row.span_id,
        "parent_span_id": row.parent_span_id,
        "service_name": row.service_name,
        "name": row.name,
        "start_offset_ns": row.start_offset_ns,
        "end_offset_ns": row.end_offset_ns,
        "duration_ns": row.duration_ns,
        "bar_start": row.bar_start,
        "bar_width": row.bar_width,
        "is_error": row.is_error,
        "is_critical_path": row.is_critical_path,
        "is_orphan": row.is_orphan,
        "is_unattached": row.is_unattached,
        "mode": mode,
        "is_collapse_marker": row.is_collapse_marker,
    })
}

fn span_classification_to_json(span: &SpanClassification) -> Value {
    json!({
        "span_id": span.span_id,
        "service_name": span.service_name,
        "name": span.name,
        "sibling_relation": span.sibling_relation.label(),
        "parent_relation": span.parent_relation.map(|relation| relation.label()),
    })
}

fn trace_annotations_to_json(annotations: &TraceAnnotations) -> Value {
    json!({
        "counts": {
            "client_server_pairs": annotations.counts.client_server_pairs,
            "client_server_span_count": annotations.counts.client_server_span_count,
            "async_span_count": annotations.counts.async_span_count,
            "linked_span_count": annotations.counts.linked_span_count,
            "messaging_span_count": annotations.counts.messaging_span_count,
        },
        "client_server_pairs": annotations
            .client_server_pairs
            .iter()
            .map(client_server_pair_to_json)
            .collect::<Vec<_>>(),
        "async_spans": annotations
            .spans
            .iter()
            .filter(|span| span.is_async_related())
            .map(span_annotation_to_json)
            .collect::<Vec<_>>(),
        "linked_spans": annotations
            .spans
            .iter()
            .filter(|span| span.linked_span_count > 0)
            .map(span_annotation_to_json)
            .collect::<Vec<_>>(),
        "spans": annotations
            .spans
            .iter()
            .map(span_annotation_to_json)
            .collect::<Vec<_>>(),
    })
}

fn span_annotation_to_json(annotation: &SpanAnnotation) -> Value {
    json!({
        "span_id": annotation.span_id,
        "service_name": annotation.service_name,
        "name": annotation.name,
        "role": annotation.role.label(),
        "client_server_peers": annotation
            .client_server_peers
            .iter()
            .map(|peer| {
                json!({
                    "span_id": peer.span_id,
                    "service_name": peer.service_name,
                    "name": peer.name,
                    "relationship": peer.relationship.label(),
                })
            })
            .collect::<Vec<_>>(),
        "async_work": annotation.async_work,
        "messaging": annotation.messaging,
        "linked_span_count": annotation.linked_span_count,
        "linked_spans": annotation
            .linked_spans
            .iter()
            .map(linked_span_ref_to_json)
            .collect::<Vec<_>>(),
        "notes": annotation
            .notes
            .iter()
            .map(|note| note.label())
            .collect::<Vec<_>>(),
    })
}

fn client_server_pair_to_json(pair: &ClientServerPair) -> Value {
    json!({
        "client": {
            "span_id": pair.client_span_id,
            "service_name": pair.client_service_name,
            "name": pair.client_name,
        },
        "server": {
            "span_id": pair.server_span_id,
            "service_name": pair.server_service_name,
            "name": pair.server_name,
        },
    })
}

fn linked_span_ref_to_json(link: &LinkedSpanRef) -> Value {
    json!({
        "trace_id": link.trace_id,
        "span_id": link.span_id,
        "same_trace": link.same_trace,
        "target_in_trace": link.target_in_trace,
    })
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

fn cross_service_edge_to_json(edge: &CrossServiceEdge) -> Value {
    json!({
        "from_service": edge.from_service,
        "to_service": edge.to_service,
        "span_count": edge.span_count,
        "client_server_pair_count": edge.client_server_pair_count,
        "sample_span_id": edge.sample_span_id,
        "sample_parent_span_id": edge.sample_parent_span_id,
    })
}

fn tree_nodes_to_json(trace: &TraceGraph, annotations: &TraceAnnotations) -> Vec<Value> {
    let mut nodes = Vec::new();
    let mut visited = vec![false; trace.spans.len()];

    for index in &trace.root_indices {
        push_tree_node(trace, annotations, *index, 0, &mut visited, &mut nodes);
    }

    for index in &trace.orphan_indices {
        push_tree_node(trace, annotations, *index, 0, &mut visited, &mut nodes);
    }

    for index in 0..trace.spans.len() {
        if !visited[index] {
            push_tree_node(trace, annotations, index, 0, &mut visited, &mut nodes);
        }
    }

    nodes
}

fn push_tree_node(
    trace: &TraceGraph,
    annotations: &TraceAnnotations,
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
        "annotations": annotations.spans.get(index).map(span_annotation_to_json),
    }));

    if let Some(children) = trace.children_by_parent.get(&span.span_id) {
        for child_index in children {
            push_tree_node(trace, annotations, *child_index, depth + 1, visited, nodes);
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
        "trace_state": span.trace_state,
        "flags": span.flags,
        "service_name": span.service_name,
        "name": span.name,
        "kind": span.kind,
        "kind_label": span.kind_label(),
        "start_ns": span.start_ns,
        "end_ns": span.end_ns,
        "duration_ns": span.duration_ns(),
        "status_code": span.status_code,
        "status_message": span.status_message,
        "status_label": span.status_label(),
        "is_error": span.is_error(),
        "attributes": span.attributes,
        "dropped_attributes_count": span.dropped_attributes_count,
        "resource_attributes": span.resource_attributes,
        "resource_schema_url": span.resource_schema_url,
        "scope_name": span.scope_name,
        "scope_version": span.scope_version,
        "scope_attributes": span.scope_attributes,
        "scope_schema_url": span.scope_schema_url,
        "events": span.events.iter().map(event_to_json).collect::<Vec<_>>(),
        "dropped_events_count": span.dropped_events_count,
        "links": span.links.iter().map(link_to_json).collect::<Vec<_>>(),
        "dropped_links_count": span.dropped_links_count,
    })
}

fn event_to_json(event: &SpanEvent) -> Value {
    json!({
        "name": event.name,
        "time_unix_nano": event.time_unix_nano,
        "attributes": event.attributes,
        "dropped_attributes_count": event.dropped_attributes_count,
    })
}

fn link_to_json(link: &SpanLink) -> Value {
    json!({
        "trace_id": link.trace_id,
        "span_id": link.span_id,
        "trace_state": link.trace_state,
        "flags": link.flags,
        "attributes": link.attributes,
        "dropped_attributes_count": link.dropped_attributes_count,
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
