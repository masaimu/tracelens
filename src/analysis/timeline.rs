use std::collections::BTreeSet;

use crate::analysis::critical_path::{CriticalPathAnalysis, CriticalPathStatus};
use crate::graph::trace_graph::TraceGraph;

pub const DEFAULT_TIMELINE_WIDTH: usize = 48;
pub const MIN_TIMELINE_WIDTH: usize = 40;
pub const MAX_TIMELINE_WIDTH: usize = 160;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimelineAnalysis {
    pub trace_id: String,
    pub start_ns: u64,
    pub end_ns: u64,
    pub duration_ns: u64,
    pub width: usize,
    pub rows: Vec<TimelineRow>,
    pub notes: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimelineRow {
    pub depth: usize,
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub service_name: String,
    pub name: String,
    pub start_offset_ns: u64,
    pub end_offset_ns: u64,
    pub duration_ns: u64,
    pub bar_start: usize,
    pub bar_width: usize,
    pub is_error: bool,
    pub is_critical_path: bool,
    pub is_orphan: bool,
    pub is_unattached: bool,
}

struct TimelineBuildContext<'a> {
    trace: &'a TraceGraph,
    trace_start_ns: u64,
    trace_duration_ns: u64,
    width: usize,
    critical_span_ids: &'a BTreeSet<String>,
    orphan_indices: &'a BTreeSet<usize>,
}

pub fn analyze_timeline(
    trace: &TraceGraph,
    critical_path: &CriticalPathAnalysis,
    width: usize,
) -> TimelineAnalysis {
    let start_ns = trace.start_ns().unwrap_or(0);
    let end_ns = trace.end_ns().unwrap_or(start_ns);
    let duration_ns = end_ns.saturating_sub(start_ns);
    let critical_span_ids = critical_span_ids(critical_path);
    let orphan_indices = trace
        .orphan_indices
        .iter()
        .copied()
        .collect::<BTreeSet<_>>();
    let context = TimelineBuildContext {
        trace,
        trace_start_ns: start_ns,
        trace_duration_ns: duration_ns,
        width,
        critical_span_ids: &critical_span_ids,
        orphan_indices: &orphan_indices,
    };
    let mut rows = Vec::new();
    let mut visited = vec![false; trace.spans.len()];

    for index in &trace.root_indices {
        push_row(&context, *index, 0, false, &mut visited, &mut rows);
    }

    for index in &trace.orphan_indices {
        push_row(&context, *index, 0, false, &mut visited, &mut rows);
    }

    for index in 0..trace.spans.len() {
        if !visited[index] {
            push_row(&context, index, 0, true, &mut visited, &mut rows);
        }
    }

    let mut notes = Vec::new();
    if duration_ns == 0 && !trace.spans.is_empty() {
        notes.push("trace duration is zero; timeline bars are pinned to the first column".into());
    }
    if matches!(critical_path.status, CriticalPathStatus::Unavailable { .. }) {
        notes.push("critical path is unavailable; timeline rows are not marked as critical".into());
    }
    if !trace.orphan_indices.is_empty() {
        notes.push("orphan spans are shown with '?' because their parent span is missing".into());
    }

    TimelineAnalysis {
        trace_id: trace.trace_id.clone(),
        start_ns,
        end_ns,
        duration_ns,
        width,
        rows,
        notes,
    }
}

fn push_row(
    context: &TimelineBuildContext<'_>,
    index: usize,
    depth: usize,
    is_unattached: bool,
    visited: &mut [bool],
    rows: &mut Vec<TimelineRow>,
) {
    if visited[index] {
        return;
    }
    visited[index] = true;

    let trace = context.trace;
    let span = &trace.spans[index];
    let start_offset_ns = span.start_ns.saturating_sub(context.trace_start_ns);
    let end_offset_ns = span.end_ns.saturating_sub(context.trace_start_ns);
    let bar_start = scale_to_width(start_offset_ns, context.trace_duration_ns, context.width);
    let bar_end = scale_to_width(end_offset_ns, context.trace_duration_ns, context.width);
    let bar_width = span_bar_width(bar_start, bar_end, context.width);
    rows.push(TimelineRow {
        depth,
        span_id: span.span_id.clone(),
        parent_span_id: span.parent_span_id.clone(),
        service_name: span.service_name.clone(),
        name: span.name.clone(),
        start_offset_ns,
        end_offset_ns,
        duration_ns: span.duration_ns(),
        bar_start,
        bar_width,
        is_error: span.is_error(),
        is_critical_path: context.critical_span_ids.contains(&span.span_id),
        is_orphan: context.orphan_indices.contains(&index),
        is_unattached,
    });

    if let Some(children) = trace.children_by_parent.get(&span.span_id) {
        for child_index in children {
            push_row(
                context,
                *child_index,
                depth + 1,
                is_unattached,
                visited,
                rows,
            );
        }
    }
}

fn critical_span_ids(critical_path: &CriticalPathAnalysis) -> BTreeSet<String> {
    if !matches!(critical_path.status, CriticalPathStatus::Available) {
        return BTreeSet::new();
    }

    critical_path
        .segments
        .iter()
        .map(|segment| segment.span_id.clone())
        .collect()
}

fn scale_to_width(offset_ns: u64, duration_ns: u64, width: usize) -> usize {
    if width == 0 || duration_ns == 0 {
        return 0;
    }

    let max_column = width - 1;
    let scaled = (offset_ns as u128 * max_column as u128) / duration_ns as u128;
    scaled.min(max_column as u128) as usize
}

fn span_bar_width(start: usize, end: usize, width: usize) -> usize {
    if width == 0 {
        return 0;
    }

    let end = end.max(start + 1).min(width);
    end.saturating_sub(start).max(1)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::analysis::critical_path::analyze_critical_path;
    use crate::analysis::timeline::{DEFAULT_TIMELINE_WIDTH, analyze_timeline};
    use crate::graph::trace_graph::TraceCollection;
    use crate::input::otlp_json::ParsedTraceData;
    use crate::model::span::CanonicalSpan;

    #[test]
    fn lays_out_rows_by_tree_order_and_scaled_offsets() {
        let collection = collection_with(vec![
            span("root", None, "checkout-service", 100, 200),
            span("a", Some("root"), "cart-service", 110, 140),
            span("b", Some("root"), "payment-service", 150, 190),
        ]);
        let trace = collection
            .traces
            .values()
            .next()
            .expect("trace should be present");
        let critical_path = analyze_critical_path(trace);

        let timeline = analyze_timeline(trace, &critical_path, DEFAULT_TIMELINE_WIDTH);

        assert_eq!(timeline.duration_ns, 100);
        assert_eq!(
            timeline
                .rows
                .iter()
                .map(|row| (row.depth, row.span_id.as_str(), row.bar_start))
                .collect::<Vec<_>>(),
            vec![(0, "root", 0), (1, "a", 4), (1, "b", 23)]
        );
        assert!(timeline.rows[0].is_critical_path);
        assert!(timeline.rows[1].is_critical_path);
    }

    #[test]
    fn marks_orphan_rows() {
        let collection = collection_with(vec![span("orphan", Some("missing"), "worker", 0, 10)]);
        let trace = collection
            .traces
            .values()
            .next()
            .expect("trace should be present");
        let critical_path = analyze_critical_path(trace);

        let timeline = analyze_timeline(trace, &critical_path, DEFAULT_TIMELINE_WIDTH);

        assert_eq!(timeline.rows.len(), 1);
        assert!(timeline.rows[0].is_orphan);
        assert!(!timeline.rows[0].is_critical_path);
        assert!(
            timeline
                .notes
                .iter()
                .any(|note| note.contains("orphan spans"))
        );
    }

    fn collection_with(spans: Vec<CanonicalSpan>) -> TraceCollection {
        TraceCollection::build(ParsedTraceData {
            spans,
            diagnostics: Vec::new(),
        })
    }

    fn span(
        span_id: &str,
        parent_span_id: Option<&str>,
        service_name: &str,
        start_ns: u64,
        end_ns: u64,
    ) -> CanonicalSpan {
        CanonicalSpan {
            trace_id: "trace".to_string(),
            span_id: span_id.to_string(),
            parent_span_id: parent_span_id.map(str::to_string),
            service_name: service_name.to_string(),
            name: span_id.to_string(),
            kind: None,
            start_ns,
            end_ns,
            status_code: None,
            attributes: BTreeMap::new(),
            resource_attributes: BTreeMap::new(),
            scope_name: None,
            scope_version: None,
            events: Vec::new(),
            links: Vec::new(),
        }
    }
}
