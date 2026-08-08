use std::collections::BTreeMap;

use crate::graph::trace_graph::TraceGraph;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceDurationAnalysis {
    pub trace_id: String,
    pub wall_clock_duration_ns: Option<u64>,
    pub root_span: Option<RootSpanDuration>,
    pub root_count: usize,
    pub orphan_count: usize,
    pub diagnostics_count: usize,
    pub services: Vec<ServiceDuration>,
    pub spans: Vec<SpanDuration>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RootSpanDuration {
    pub span_id: String,
    pub service_name: String,
    pub name: String,
    pub duration_ns: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServiceDuration {
    pub service_name: String,
    pub self_time_ns: u64,
    pub span_time_ns: u64,
    pub child_covered_time_ns: u64,
    pub span_count: usize,
    pub error_span_count: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpanDuration {
    pub span_id: String,
    pub service_name: String,
    pub name: String,
    pub duration_ns: u64,
    pub child_covered_time_ns: u64,
    pub self_time_ns: u64,
}

#[derive(Default)]
struct ServiceDurationAccumulator {
    self_time_ns: u64,
    span_time_ns: u64,
    child_covered_time_ns: u64,
    span_count: usize,
    error_span_count: usize,
}

pub fn analyze_trace_duration(trace: &TraceGraph) -> TraceDurationAnalysis {
    let root_span = unique_root_span(trace);
    let mut services: BTreeMap<String, ServiceDurationAccumulator> = BTreeMap::new();
    let mut spans = Vec::new();

    for (index, span) in trace.spans.iter().enumerate() {
        let child_covered_time_ns = child_covered_time_ns(trace, index);
        let duration_ns = span.duration_ns();
        let self_time_ns = duration_ns.saturating_sub(child_covered_time_ns);

        let service = services.entry(span.service_name.clone()).or_default();
        service.self_time_ns += self_time_ns;
        service.span_time_ns += duration_ns;
        service.child_covered_time_ns += child_covered_time_ns;
        service.span_count += 1;
        if span.is_error() {
            service.error_span_count += 1;
        }

        spans.push(SpanDuration {
            span_id: span.span_id.clone(),
            service_name: span.service_name.clone(),
            name: span.name.clone(),
            duration_ns,
            child_covered_time_ns,
            self_time_ns,
        });
    }

    let mut services = services
        .into_iter()
        .map(|(service_name, service)| ServiceDuration {
            service_name,
            self_time_ns: service.self_time_ns,
            span_time_ns: service.span_time_ns,
            child_covered_time_ns: service.child_covered_time_ns,
            span_count: service.span_count,
            error_span_count: service.error_span_count,
        })
        .collect::<Vec<_>>();
    services.sort_by(|left, right| {
        right
            .self_time_ns
            .cmp(&left.self_time_ns)
            .then(left.service_name.cmp(&right.service_name))
    });

    TraceDurationAnalysis {
        trace_id: trace.trace_id.clone(),
        wall_clock_duration_ns: trace.duration_ns(),
        root_span,
        root_count: trace.root_indices.len(),
        orphan_count: trace.orphan_indices.len(),
        diagnostics_count: trace.diagnostics.len(),
        services,
        spans,
    }
}

fn unique_root_span(trace: &TraceGraph) -> Option<RootSpanDuration> {
    if trace.root_indices.len() != 1 {
        return None;
    }

    let span = &trace.spans[trace.root_indices[0]];
    Some(RootSpanDuration {
        span_id: span.span_id.clone(),
        service_name: span.service_name.clone(),
        name: span.name.clone(),
        duration_ns: span.duration_ns(),
    })
}

fn child_covered_time_ns(trace: &TraceGraph, span_index: usize) -> u64 {
    let span = &trace.spans[span_index];
    let Some(children) = trace.children_by_parent.get(&span.span_id) else {
        return 0;
    };

    let mut intervals = Vec::new();
    for child_index in children {
        let child = &trace.spans[*child_index];
        let start_ns = child.start_ns.max(span.start_ns);
        let end_ns = child.end_ns.min(span.end_ns);
        if start_ns < end_ns {
            intervals.push((start_ns, end_ns));
        }
    }

    interval_union_duration_ns(&mut intervals)
}

fn interval_union_duration_ns(intervals: &mut [(u64, u64)]) -> u64 {
    intervals.sort_by(|left, right| left.0.cmp(&right.0).then(left.1.cmp(&right.1)));

    let mut total = 0;
    let mut current: Option<(u64, u64)> = None;
    for (start_ns, end_ns) in intervals.iter().copied() {
        match current {
            None => current = Some((start_ns, end_ns)),
            Some((current_start, current_end)) if start_ns <= current_end => {
                current = Some((current_start, current_end.max(end_ns)));
            }
            Some((current_start, current_end)) => {
                total += current_end.saturating_sub(current_start);
                current = Some((start_ns, end_ns));
            }
        }
    }

    if let Some((current_start, current_end)) = current {
        total += current_end.saturating_sub(current_start);
    }

    total
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::analysis::duration::{analyze_trace_duration, interval_union_duration_ns};
    use crate::graph::trace_graph::TraceCollection;
    use crate::input::otlp_json::ParsedTraceData;
    use crate::model::span::CanonicalSpan;

    #[test]
    fn interval_union_does_not_double_count_overlaps() {
        let mut intervals = vec![(10, 60), (40, 80), (90, 100)];

        assert_eq!(interval_union_duration_ns(&mut intervals), 80);
    }

    #[test]
    fn span_self_time_uses_child_interval_union() {
        let collection = collection_with(vec![
            span("root", None, "checkout-service", 0, 100),
            span("child-a", Some("root"), "cart-service", 10, 60),
            span("child-b", Some("root"), "payment-service", 40, 80),
        ]);
        let trace = collection
            .traces
            .values()
            .next()
            .expect("trace should be present");

        let analysis = analyze_trace_duration(trace);
        let root = analysis
            .spans
            .iter()
            .find(|span| span.span_id == "root")
            .expect("root span should be present");

        assert_eq!(root.duration_ns, 100);
        assert_eq!(root.child_covered_time_ns, 70);
        assert_eq!(root.self_time_ns, 30);
    }

    #[test]
    fn aggregates_service_self_time() {
        let collection = collection_with(vec![
            span("root", None, "checkout-service", 0, 100),
            span("child-a", Some("root"), "cart-service", 10, 60),
            span("child-b", Some("root"), "cart-service", 40, 80),
        ]);
        let trace = collection
            .traces
            .values()
            .next()
            .expect("trace should be present");

        let analysis = analyze_trace_duration(trace);
        let cart = analysis
            .services
            .iter()
            .find(|service| service.service_name == "cart-service")
            .expect("cart service should be present");
        let checkout = analysis
            .services
            .iter()
            .find(|service| service.service_name == "checkout-service")
            .expect("checkout service should be present");

        assert_eq!(cart.self_time_ns, 90);
        assert_eq!(cart.span_time_ns, 90);
        assert_eq!(cart.span_count, 2);
        assert_eq!(checkout.self_time_ns, 30);
        assert_eq!(checkout.child_covered_time_ns, 70);
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
