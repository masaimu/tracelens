use std::collections::BTreeSet;

use crate::analysis::critical_path::{CriticalPathAnalysis, CriticalPathStatus};
use crate::graph::trace_graph::TraceGraph;

pub const DEFAULT_TIMELINE_WIDTH: usize = 48;
pub const MIN_TIMELINE_WIDTH: usize = 40;
pub const MAX_TIMELINE_WIDTH: usize = 160;

/// Default upper bound on the number of timeline rows before middle rows are
/// collapsed into summary marker rows. `--max-rows 0` disables collapse.
pub const DEFAULT_TIMELINE_MAX_ROWS: usize = 40;

/// Sentinel span ID used for synthetic collapse marker rows. It is a valid
/// 16-hex-char span ID so it still matches the JSON schema span pattern, but it
/// is far enough from realistic generated IDs that downstream consumers can rely
/// on `is_collapse_marker` to distinguish it.
const COLLAPSE_MARKER_SPAN_ID: &str = "fffffffffffffffe";

/// Timeline layout mode. `Bar` keeps the existing horizontal time axis; `Flame`
/// renders a vertically indented ASCII flame graph. The mode only changes
/// rendering, never the underlying trace analysis.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TimelineMode {
    Bar,
    Flame,
}

impl TimelineMode {
    pub fn label(&self) -> &'static str {
        match self {
            TimelineMode::Bar => "bar",
            TimelineMode::Flame => "flame",
        }
    }
}

/// Summary of the timeline collapse behavior applied to the rows.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimelineCollapse {
    /// Whether collapse is enabled (`true` when `--max-rows` is greater than 0).
    pub enabled: bool,
    /// Configured maximum number of rows before collapse; `0` disables collapse.
    pub max_rows: usize,
    /// Number of non-preserved rows omitted and replaced by collapse markers.
    pub omitted_rows: usize,
    /// Categories of rows preserved during collapse (e.g. `critical_path`).
    pub preserved_reasons: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TimelineAnalysis {
    pub trace_id: String,
    pub start_ns: u64,
    pub end_ns: u64,
    pub duration_ns: u64,
    pub width: usize,
    pub mode: TimelineMode,
    pub max_rows: usize,
    pub rows: Vec<TimelineRow>,
    pub collapsed: TimelineCollapse,
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
    /// `true` for synthetic rows that summarize omitted spans during collapse
    /// instead of representing a real span.
    pub is_collapse_marker: bool,
}

struct TimelineBuildContext<'a> {
    trace: &'a TraceGraph,
    trace_start_ns: u64,
    trace_duration_ns: u64,
    width: usize,
    critical_span_ids: &'a BTreeSet<String>,
    orphan_indices: &'a BTreeSet<usize>,
}

#[allow(clippy::too_many_arguments)]
pub fn analyze_timeline(
    trace: &TraceGraph,
    critical_path: &CriticalPathAnalysis,
    width: usize,
    mode: TimelineMode,
    max_rows: usize,
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

    let (rows, collapsed) = collapse_rows(rows, max_rows);

    TimelineAnalysis {
        trace_id: trace.trace_id.clone(),
        start_ns,
        end_ns,
        duration_ns,
        width,
        mode,
        max_rows,
        rows,
        collapsed,
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
        is_collapse_marker: false,
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

/// Collapse non-essential timeline rows when the row count exceeds `max_rows`.
///
/// Rows are preserved when they are on the critical path, are errors, are
/// orphans, are unattached, or sit at the trace boundaries. Each contiguous run
/// of omitted rows is replaced by a single synthetic collapse marker row so the
/// omission is visible rather than silent. `max_rows == 0` disables collapse.
fn collapse_rows(rows: Vec<TimelineRow>, max_rows: usize) -> (Vec<TimelineRow>, TimelineCollapse) {
    if max_rows == 0 {
        return (
            rows,
            TimelineCollapse {
                enabled: false,
                max_rows,
                omitted_rows: 0,
                preserved_reasons: Vec::new(),
            },
        );
    }

    let last = rows.len().saturating_sub(1);

    if rows.len() <= max_rows {
        return (
            rows,
            TimelineCollapse {
                enabled: true,
                max_rows,
                omitted_rows: 0,
                // nothing was omitted, so there were no "preserved" rows to
                // categorize; keep the field empty until collapse actually runs.
                preserved_reasons: Vec::new(),
            },
        );
    }

    let mut collapsed = Vec::new();
    let mut omitted = 0usize;
    let mut run_count = 0usize;
    let mut run_depth = 0usize;

    for (index, row) in rows.iter().enumerate() {
        let preserved = index == 0
            || index == last
            || row.is_critical_path
            || row.is_error
            || row.is_orphan
            || row.is_unattached;

        if preserved {
            if run_count > 0 {
                collapsed.push(collapse_marker_row(run_count, run_depth));
                omitted += run_count;
                run_count = 0;
            }
            collapsed.push(row.clone());
        } else {
            if run_count == 0 {
                run_depth = row.depth;
            }
            run_count += 1;
        }
    }
    if run_count > 0 {
        collapsed.push(collapse_marker_row(run_count, run_depth));
        omitted += run_count;
    }

    let preserved_reasons = if omitted > 0 {
        collect_preserved_reasons(&rows, last)
    } else {
        Vec::new()
    };

    (
        collapsed,
        TimelineCollapse {
            enabled: true,
            max_rows,
            omitted_rows: omitted,
            preserved_reasons,
        },
    )
}

fn collapse_marker_row(omitted: usize, depth: usize) -> TimelineRow {
    TimelineRow {
        depth,
        span_id: COLLAPSE_MARKER_SPAN_ID.to_string(),
        parent_span_id: None,
        service_name: String::new(),
        name: format!("... collapsed: {omitted} rows omitted ..."),
        start_offset_ns: 0,
        end_offset_ns: 0,
        duration_ns: 0,
        bar_start: 0,
        bar_width: 0,
        is_error: false,
        is_critical_path: false,
        is_orphan: false,
        is_unattached: false,
        is_collapse_marker: true,
    }
}

fn collect_preserved_reasons(rows: &[TimelineRow], last: usize) -> Vec<String> {
    let mut reasons: BTreeSet<String> = BTreeSet::new();
    for (index, row) in rows.iter().enumerate() {
        if index == 0 || index == last {
            reasons.insert("boundary".to_string());
        }
        if row.is_critical_path {
            reasons.insert("critical_path".to_string());
        }
        if row.is_error {
            reasons.insert("error".to_string());
        }
        if row.is_orphan {
            reasons.insert("orphan".to_string());
        }
        if row.is_unattached {
            reasons.insert("unattached".to_string());
        }
    }
    reasons.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::analysis::critical_path::analyze_critical_path;
    use crate::analysis::timeline::{
        DEFAULT_TIMELINE_MAX_ROWS, DEFAULT_TIMELINE_WIDTH, TimelineMode, analyze_timeline,
    };
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

        let timeline = analyze_timeline(
            trace,
            &critical_path,
            DEFAULT_TIMELINE_WIDTH,
            TimelineMode::Bar,
            0,
        );

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

        let timeline = analyze_timeline(
            trace,
            &critical_path,
            DEFAULT_TIMELINE_WIDTH,
            TimelineMode::Bar,
            0,
        );

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

    #[test]
    fn flame_mode_produces_same_rows_as_bar_mode() {
        // mode only changes rendering; the analysis rows must be identical when
        // collapse is disabled.
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

        let bar = analyze_timeline(
            trace,
            &critical_path,
            DEFAULT_TIMELINE_WIDTH,
            TimelineMode::Bar,
            0,
        );
        let flame = analyze_timeline(
            trace,
            &critical_path,
            DEFAULT_TIMELINE_WIDTH,
            TimelineMode::Flame,
            0,
        );

        assert_eq!(flame.mode, TimelineMode::Flame);
        assert_eq!(bar.mode, TimelineMode::Bar);
        assert_eq!(flame.rows, bar.rows);
        assert_eq!(
            flame.rows.iter().map(|row| row.depth).collect::<Vec<_>>(),
            vec![0, 1, 1]
        );
        // critical-path markings are identical between modes.
        assert_eq!(
            flame
                .rows
                .iter()
                .map(|row| (row.span_id.as_str(), row.is_critical_path))
                .collect::<Vec<_>>(),
            bar.rows
                .iter()
                .map(|row| (row.span_id.as_str(), row.is_critical_path))
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn collapse_preserves_critical_error_and_boundary_rows() {
        // one root + seven concurrent children; only the root and the latest
        // ending child are on the critical path. one of the non-critical
        // children is an error span.
        let mut spans = vec![span("root", None, "svc", 0, 200)];
        for i in 1..=7usize {
            let span_id = format!("c{i}");
            let end = if i == 7 { 190 } else { 30 };
            let mut s = span(&span_id, Some("root"), "svc", 10, end);
            if i == 3 {
                s.status_code = Some(2);
            }
            spans.push(s);
        }
        let collection = collection_with(spans);
        let trace = collection
            .traces
            .values()
            .next()
            .expect("trace should be present");
        let critical_path = analyze_critical_path(trace);

        let timeline = analyze_timeline(
            trace,
            &critical_path,
            DEFAULT_TIMELINE_WIDTH,
            TimelineMode::Bar,
            4,
        );

        assert!(timeline.collapsed.enabled);
        assert!(timeline.collapsed.omitted_rows > 0);
        assert!(
            timeline
                .collapsed
                .preserved_reasons
                .contains(&"critical_path".to_string())
        );
        assert!(
            timeline
                .collapsed
                .preserved_reasons
                .contains(&"error".to_string())
        );
        assert!(
            timeline
                .collapsed
                .preserved_reasons
                .contains(&"boundary".to_string())
        );

        let markers = timeline
            .rows
            .iter()
            .filter(|row| row.is_collapse_marker)
            .count();
        assert!(markers > 0, "expected at least one collapse marker row");

        // critical-path rows and the error row must survive collapse.
        assert!(
            timeline
                .rows
                .iter()
                .any(|row| row.span_id == "root" && row.is_critical_path)
        );
        assert!(
            timeline
                .rows
                .iter()
                .any(|row| row.span_id == "c7" && row.is_critical_path)
        );
        assert!(
            timeline
                .rows
                .iter()
                .any(|row| row.span_id == "c3" && row.is_error)
        );
    }

    #[test]
    fn max_rows_zero_disables_collapse() {
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

        let timeline = analyze_timeline(
            trace,
            &critical_path,
            DEFAULT_TIMELINE_WIDTH,
            TimelineMode::Bar,
            0,
        );

        assert!(!timeline.collapsed.enabled);
        assert_eq!(timeline.collapsed.omitted_rows, 0);
        assert!(timeline.rows.iter().all(|row| !row.is_collapse_marker));
        assert_eq!(timeline.max_rows, 0);
    }

    #[test]
    fn default_max_rows_constant_is_set() {
        assert_eq!(DEFAULT_TIMELINE_MAX_ROWS, 40);
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
            trace_state: None,
            flags: None,
            service_name: service_name.to_string(),
            name: span_id.to_string(),
            kind: None,
            start_ns,
            end_ns,
            status_code: None,
            status_message: None,
            attributes: BTreeMap::new(),
            dropped_attributes_count: None,
            resource_attributes: BTreeMap::new(),
            resource_schema_url: None,
            scope_name: None,
            scope_version: None,
            scope_attributes: BTreeMap::new(),
            scope_schema_url: None,
            events: Vec::new(),
            dropped_events_count: None,
            links: Vec::new(),
            dropped_links_count: None,
        }
    }
}
