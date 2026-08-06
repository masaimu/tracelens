use std::collections::BTreeSet;

use crate::graph::trace_graph::{TraceCollection, TraceGraph};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileSummary {
    pub trace_count: usize,
    pub span_count: usize,
    pub service_count: usize,
    pub error_span_count: usize,
    pub start_ns: Option<u64>,
    pub end_ns: Option<u64>,
    pub slowest_traces: Vec<TraceSummary>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceSummary {
    pub trace_id: String,
    pub span_count: usize,
    pub service_count: usize,
    pub error_span_count: usize,
    pub start_ns: Option<u64>,
    pub end_ns: Option<u64>,
    pub duration_ns: Option<u64>,
}

pub fn summarize(collection: &TraceCollection) -> FileSummary {
    let mut services = BTreeSet::new();
    let mut error_span_count = 0;
    let mut start_ns = None;
    let mut end_ns = None;
    let mut trace_summaries = Vec::new();

    for trace in collection.traces.values() {
        for service in trace.service_names() {
            services.insert(service.to_string());
        }
        error_span_count += trace.error_span_count();
        start_ns = min_option(start_ns, trace.start_ns());
        end_ns = max_option(end_ns, trace.end_ns());
        trace_summaries.push(summarize_trace(trace));
    }

    trace_summaries.sort_by(|left, right| {
        right
            .duration_ns
            .cmp(&left.duration_ns)
            .then(left.trace_id.cmp(&right.trace_id))
    });
    trace_summaries.truncate(10);

    FileSummary {
        trace_count: collection.traces.len(),
        span_count: collection.span_count(),
        service_count: services.len(),
        error_span_count,
        start_ns,
        end_ns,
        slowest_traces: trace_summaries,
    }
}

fn summarize_trace(trace: &TraceGraph) -> TraceSummary {
    TraceSummary {
        trace_id: trace.trace_id.clone(),
        span_count: trace.spans.len(),
        service_count: trace.service_names().len(),
        error_span_count: trace.error_span_count(),
        start_ns: trace.start_ns(),
        end_ns: trace.end_ns(),
        duration_ns: trace.duration_ns(),
    }
}

fn min_option(left: Option<u64>, right: Option<u64>) -> Option<u64> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.min(right)),
        (Some(value), None) | (None, Some(value)) => Some(value),
        (None, None) => None,
    }
}

fn max_option(left: Option<u64>, right: Option<u64>) -> Option<u64> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.max(right)),
        (Some(value), None) | (None, Some(value)) => Some(value),
        (None, None) => None,
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::analysis::summary::summarize;
    use crate::graph::trace_graph::TraceCollection;
    use crate::input::otlp_json::parse_otlp_file;

    #[test]
    fn summarizes_trace_file() {
        let data = parse_otlp_file(Path::new("tests/fixtures/otlp-basic.json"))
            .expect("fixture should parse");
        let collection = TraceCollection::build(data);
        let summary = summarize(&collection);

        assert_eq!(summary.trace_count, 2);
        assert_eq!(summary.span_count, 4);
        assert_eq!(summary.service_count, 3);
        assert_eq!(summary.error_span_count, 1);
        assert_eq!(summary.slowest_traces[0].duration_ns, Some(100_000_000));
    }
}
