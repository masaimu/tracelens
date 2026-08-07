use std::collections::{BTreeMap, BTreeSet};

use crate::graph::trace_graph::TraceGraph;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CriticalPathAnalysis {
    pub status: CriticalPathStatus,
    pub root_span_id: Option<String>,
    pub root_span: Option<CriticalPathRootSpan>,
    pub total_duration_ns: u64,
    pub segments: Vec<CriticalPathSegment>,
    pub span_totals: Vec<CriticalPathSpanTotal>,
    pub notes: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CriticalPathStatus {
    Available,
    Unavailable { reason: String },
}

impl CriticalPathStatus {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Available => "available",
            Self::Unavailable { .. } => "unavailable",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CriticalPathSegment {
    pub span_id: String,
    pub service_name: String,
    pub name: String,
    pub offset_ns: u64,
    pub duration_ns: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CriticalPathRootSpan {
    pub span_id: String,
    pub service_name: String,
    pub name: String,
    pub duration_ns: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CriticalPathSpanTotal {
    pub span_id: String,
    pub service_name: String,
    pub name: String,
    pub total_ns: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct RawCriticalPathSegment {
    span_index: usize,
    start_ns: u64,
    end_ns: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ChildInterval {
    span_index: usize,
    span_id: String,
    start_ns: u64,
    end_ns: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ActiveChild {
    end_ns: u64,
    span_id: String,
    span_index: usize,
}

impl Ord for ActiveChild {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.end_ns
            .cmp(&other.end_ns)
            // For the same end time, the smallest span ID is the dominant
            // child. Reverse the string comparison so next_back() picks it.
            .then_with(|| other.span_id.cmp(&self.span_id))
            .then_with(|| other.span_index.cmp(&self.span_index))
    }
}

impl PartialOrd for ActiveChild {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

pub fn analyze_critical_path(trace: &TraceGraph) -> CriticalPathAnalysis {
    let Some(root_index) = select_root(trace) else {
        return CriticalPathAnalysis {
            status: CriticalPathStatus::Unavailable {
                reason: "trace has no root span".to_string(),
            },
            root_span_id: None,
            root_span: None,
            total_duration_ns: 0,
            segments: Vec::new(),
            span_totals: Vec::new(),
            notes: Vec::new(),
        };
    };

    let mut notes = Vec::new();
    if trace.root_indices.len() > 1 {
        notes.push(format!(
            "trace has {} root spans; the critical path only covers the longest root span",
            trace.root_indices.len()
        ));
    }

    let root = &trace.spans[root_index];
    let root_span = CriticalPathRootSpan {
        span_id: root.span_id.clone(),
        service_name: root.service_name.clone(),
        name: root.name.clone(),
        duration_ns: root.duration_ns(),
    };
    if let (Some(start_ns), Some(end_ns)) = (trace.start_ns(), trace.end_ns()) {
        let wall_clock_ns = end_ns.saturating_sub(start_ns);
        if wall_clock_ns > root.duration_ns() {
            notes.push(
                "wall-clock duration exceeds the root span interval; the critical path only covers the root span interval"
                    .to_string(),
            );
        }
    }

    let trace_start_ns = trace.start_ns().unwrap_or(root.start_ns);
    let mut raw_segments = Vec::new();
    let mut stack = BTreeSet::new();
    collect_segments(
        trace,
        root_index,
        root.start_ns,
        root.end_ns,
        &mut stack,
        &mut raw_segments,
    );
    let raw_segments = coalesce_raw_segments(raw_segments);
    let span_totals = aggregate_span_totals(trace, &raw_segments);
    let segments = critical_path_segments(trace, trace_start_ns, raw_segments);

    CriticalPathAnalysis {
        status: CriticalPathStatus::Available,
        root_span_id: Some(root.span_id.clone()),
        root_span: Some(root_span),
        total_duration_ns: root.duration_ns(),
        segments,
        span_totals,
        notes,
    }
}

fn select_root(trace: &TraceGraph) -> Option<usize> {
    trace.root_indices.iter().copied().max_by(|left, right| {
        let left_span = &trace.spans[*left];
        let right_span = &trace.spans[*right];
        left_span
            .duration_ns()
            .cmp(&right_span.duration_ns())
            // Prefer the earliest start, then the smallest span ID, on ties.
            .then(right_span.start_ns.cmp(&left_span.start_ns))
            .then(right_span.span_id.cmp(&left_span.span_id))
    })
}

fn collect_segments(
    trace: &TraceGraph,
    span_index: usize,
    range_start: u64,
    range_end: u64,
    stack: &mut BTreeSet<usize>,
    segments: &mut Vec<RawCriticalPathSegment>,
) {
    if range_start >= range_end {
        return;
    }

    // Cycle guard: a span already on the recursion stack absorbs the range
    // directly instead of recursing forever on cyclic parent-child edges.
    if !stack.insert(span_index) {
        segments.push(RawCriticalPathSegment {
            span_index,
            start_ns: range_start,
            end_ns: range_end,
        });
        return;
    }

    let span = &trace.spans[span_index];
    let children = trace
        .children_by_parent
        .get(&span.span_id)
        .map(Vec::as_slice)
        .unwrap_or(&[]);

    let mut split_points = vec![range_start, range_end];
    let mut child_intervals = Vec::new();
    for child_index in children {
        let child = &trace.spans[*child_index];
        let start_ns = child.start_ns.max(range_start);
        let end_ns = child.end_ns.min(range_end);
        if start_ns < end_ns {
            split_points.push(start_ns);
            split_points.push(end_ns);
            child_intervals.push(ChildInterval {
                span_index: *child_index,
                span_id: child.span_id.clone(),
                start_ns,
                end_ns,
            });
        }
    }
    split_points.sort_unstable();
    split_points.dedup();
    child_intervals.sort_by(|left, right| {
        left.start_ns
            .cmp(&right.start_ns)
            .then(left.end_ns.cmp(&right.end_ns))
            .then(left.span_id.cmp(&right.span_id))
            .then(left.span_index.cmp(&right.span_index))
    });

    let mut next_child = 0;
    let mut active_children = BTreeSet::new();
    for window in split_points.windows(2) {
        let (window_start, window_end) = (window[0], window[1]);
        while next_child < child_intervals.len()
            && child_intervals[next_child].start_ns <= window_start
        {
            let child = &child_intervals[next_child];
            if child.end_ns > window_start {
                active_children.insert(ActiveChild {
                    end_ns: child.end_ns,
                    span_id: child.span_id.clone(),
                    span_index: child.span_index,
                });
            }
            next_child += 1;
        }

        while let Some(child) = active_children.iter().next().cloned() {
            if child.end_ns > window_start {
                break;
            }
            active_children.remove(&child);
        }

        if let Some(child) = active_children.iter().next_back().cloned() {
            collect_segments(
                trace,
                child.span_index,
                window_start,
                window_end,
                stack,
                segments,
            );
        } else {
            segments.push(RawCriticalPathSegment {
                span_index,
                start_ns: window_start,
                end_ns: window_end,
            });
        }
    }

    stack.remove(&span_index);
}

fn coalesce_raw_segments(raw_segments: Vec<RawCriticalPathSegment>) -> Vec<RawCriticalPathSegment> {
    let mut merged: Vec<RawCriticalPathSegment> = Vec::new();
    for segment in raw_segments {
        match merged.last_mut() {
            Some(last)
                if last.span_index == segment.span_index && last.end_ns == segment.start_ns =>
            {
                last.end_ns = segment.end_ns;
            }
            _ => merged.push(segment),
        }
    }
    merged
}

fn critical_path_segments(
    trace: &TraceGraph,
    trace_start_ns: u64,
    raw_segments: Vec<RawCriticalPathSegment>,
) -> Vec<CriticalPathSegment> {
    raw_segments
        .into_iter()
        .map(|segment| {
            let span = &trace.spans[segment.span_index];
            CriticalPathSegment {
                span_id: span.span_id.clone(),
                service_name: span.service_name.clone(),
                name: span.name.clone(),
                offset_ns: segment.start_ns.saturating_sub(trace_start_ns),
                duration_ns: segment.end_ns.saturating_sub(segment.start_ns),
            }
        })
        .collect()
}

fn aggregate_span_totals(
    trace: &TraceGraph,
    raw_segments: &[RawCriticalPathSegment],
) -> Vec<CriticalPathSpanTotal> {
    let mut totals: BTreeMap<usize, u64> = BTreeMap::new();
    for segment in raw_segments {
        *totals.entry(segment.span_index).or_default() +=
            segment.end_ns.saturating_sub(segment.start_ns);
    }

    let mut span_totals: Vec<CriticalPathSpanTotal> = totals
        .into_iter()
        .filter_map(|(span_index, total_ns)| {
            let span = trace.spans.get(span_index)?;
            Some(CriticalPathSpanTotal {
                span_id: span.span_id.clone(),
                service_name: span.service_name.clone(),
                name: span.name.clone(),
                total_ns,
            })
        })
        .collect();
    span_totals.sort_by(|left, right| {
        right
            .total_ns
            .cmp(&left.total_ns)
            .then(left.span_id.cmp(&right.span_id))
            .then(left.service_name.cmp(&right.service_name))
            .then(left.name.cmp(&right.name))
    });
    span_totals
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::analysis::critical_path::{CriticalPathStatus, analyze_critical_path};
    use crate::graph::trace_graph::TraceCollection;
    use crate::input::otlp_json::ParsedTraceData;
    use crate::model::span::CanonicalSpan;

    #[test]
    fn covers_root_interval_with_concurrent_children() {
        let collection = collection_with(vec![
            span("root", None, "checkout-service", 100, 200),
            span("a", Some("root"), "cart-service", 110, 140),
            span("b", Some("root"), "payment-service", 150, 190),
            span("c", Some("root"), "inventory-service", 150, 180),
            span("b1", Some("b"), "postgres", 155, 170),
            span("b2", Some("b"), "redis", 165, 185),
        ]);
        let trace = collection
            .traces
            .values()
            .next()
            .expect("trace should be present");

        let analysis = analyze_critical_path(trace);

        assert_eq!(analysis.status, CriticalPathStatus::Available);
        assert_eq!(analysis.total_duration_ns, 100);
        let covered: u64 = analysis
            .segments
            .iter()
            .map(|segment| segment.duration_ns)
            .sum();
        assert_eq!(covered, 100);

        let segments: Vec<(&str, u64, u64)> = analysis
            .segments
            .iter()
            .map(|segment| {
                (
                    segment.span_id.as_str(),
                    segment.offset_ns,
                    segment.duration_ns,
                )
            })
            .collect();
        assert_eq!(
            segments,
            vec![
                ("root", 0, 10),
                ("a", 10, 30),
                ("root", 40, 10),
                ("b", 50, 5),
                ("b1", 55, 10),
                ("b2", 65, 20),
                ("b", 85, 5),
                ("root", 90, 10),
            ]
        );

        let totals: BTreeMap<&str, u64> = analysis
            .span_totals
            .iter()
            .map(|total| (total.span_id.as_str(), total.total_ns))
            .collect();
        assert_eq!(
            totals,
            BTreeMap::from([("root", 30), ("a", 30), ("b", 10), ("b1", 10), ("b2", 20),])
        );
        assert!(analysis.notes.is_empty());
    }

    #[test]
    fn merges_adjacent_segments_of_same_span() {
        let collection = collection_with(vec![
            span("root", None, "checkout-service", 0, 100),
            span("a", Some("root"), "cart-service", 10, 40),
            span("b", Some("root"), "payment-service", 40, 60),
        ]);
        let trace = collection
            .traces
            .values()
            .next()
            .expect("trace should be present");

        let analysis = analyze_critical_path(trace);
        let segments: Vec<(&str, u64, u64)> = analysis
            .segments
            .iter()
            .map(|segment| {
                (
                    segment.span_id.as_str(),
                    segment.offset_ns,
                    segment.duration_ns,
                )
            })
            .collect();
        assert_eq!(
            segments,
            vec![
                ("root", 0, 10),
                ("a", 10, 30),
                ("b", 40, 20),
                ("root", 60, 40)
            ]
        );
    }

    #[test]
    fn clamps_children_outside_root_interval() {
        let collection = collection_with(vec![
            span("root", None, "checkout-service", 100, 200),
            span("late", Some("root"), "notify-service", 190, 210),
        ]);
        let trace = collection
            .traces
            .values()
            .next()
            .expect("trace should be present");

        let analysis = analyze_critical_path(trace);

        assert_eq!(analysis.total_duration_ns, 100);
        let covered: u64 = analysis
            .segments
            .iter()
            .map(|segment| segment.duration_ns)
            .sum();
        assert_eq!(covered, 100);
        assert!(
            analysis
                .notes
                .iter()
                .any(|note| note.contains("wall-clock duration exceeds"))
        );
    }

    #[test]
    fn picks_longest_root_for_multiple_roots() {
        let collection = collection_with(vec![
            span("short-root", None, "checkout-service", 0, 10),
            span("long-root", None, "cart-service", 20, 120),
            span("child", Some("long-root"), "payment-service", 30, 80),
        ]);
        let trace = collection
            .traces
            .values()
            .next()
            .expect("trace should be present");

        let analysis = analyze_critical_path(trace);

        assert_eq!(analysis.status, CriticalPathStatus::Available);
        assert_eq!(analysis.root_span_id.as_deref(), Some("long-root"));
        assert_eq!(
            analysis
                .root_span
                .as_ref()
                .map(|root| root.span_id.as_str()),
            Some("long-root")
        );
        assert_eq!(analysis.total_duration_ns, 100);
        assert!(
            analysis
                .notes
                .iter()
                .any(|note| note.contains("root spans"))
        );
    }

    #[test]
    fn duplicate_span_ids_are_not_merged_in_totals() {
        let collection = collection_with(vec![
            span("root", None, "checkout-service", 0, 100),
            span("dup", Some("root"), "cart-service", 10, 40),
            span("dup", Some("root"), "payment-service", 40, 70),
        ]);
        let trace = collection
            .traces
            .values()
            .next()
            .expect("trace should be present");

        let analysis = analyze_critical_path(trace);

        let duplicate_totals: Vec<_> = analysis
            .span_totals
            .iter()
            .filter(|total| total.span_id == "dup")
            .collect();
        assert_eq!(duplicate_totals.len(), 2);
        assert!(
            duplicate_totals
                .iter()
                .any(|total| total.service_name == "cart-service" && total.total_ns == 30)
        );
        assert!(
            duplicate_totals
                .iter()
                .any(|total| total.service_name == "payment-service" && total.total_ns == 30)
        );
    }

    #[test]
    fn unavailable_without_root() {
        let collection = collection_with(vec![
            span("orphan-a", Some("missing"), "cart-service", 10, 40),
            span("orphan-b", Some("missing"), "payment-service", 50, 90),
        ]);
        let trace = collection
            .traces
            .values()
            .next()
            .expect("trace should be present");

        let analysis = analyze_critical_path(trace);

        assert_eq!(
            analysis.status,
            CriticalPathStatus::Unavailable {
                reason: "trace has no root span".to_string()
            }
        );
        assert!(analysis.segments.is_empty());
        assert!(analysis.span_totals.is_empty());
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
