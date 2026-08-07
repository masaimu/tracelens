use std::collections::{BTreeMap, BTreeSet};

use crate::graph::trace_graph::{TraceCollection, TraceGraph};
use crate::model::span::CanonicalSpan;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Confidence {
    Low,
    Medium,
    High,
}

impl Confidence {
    pub fn label(self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SampleQuality {
    Insufficient,
    Limited,
    Broad,
}

impl SampleQuality {
    pub fn label(self) -> &'static str {
        match self {
            Self::Insufficient => "insufficient",
            Self::Limited => "limited",
            Self::Broad => "broad",
        }
    }

    fn from_sample_count(sample_count: usize) -> Self {
        if sample_count < 5 {
            Self::Insufficient
        } else if sample_count < 20 {
            Self::Limited
        } else {
            Self::Broad
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DetectSummary {
    pub trace_count: usize,
    pub span_count: usize,
    pub diagnostics_count: usize,
    pub sample_count: usize,
    pub sample_quality: SampleQuality,
    pub p95_duration_ns: Option<u64>,
    pub slow_trace_candidate_count: usize,
    pub error_trace_candidate_count: usize,
    pub error_span_count: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServiceSlowCandidate {
    pub service_name: String,
    pub span_time_ns: u64,
    pub max_span_duration_ns: u64,
    pub span_count: usize,
    pub error_span_count: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SlowTraceCandidate {
    pub trace_id: String,
    pub rank: usize,
    pub duration_ns: u64,
    pub p95_duration_ns: Option<u64>,
    pub sample_count: usize,
    pub confidence: Confidence,
    pub reason: String,
    pub span_count: usize,
    pub service_count: usize,
    pub error_span_count: usize,
    pub diagnostics_count: usize,
    pub service_candidates: Vec<ServiceSlowCandidate>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ErrorSpanCandidate {
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub service_name: String,
    pub name: String,
    pub depth: usize,
    pub start_ns: u64,
    pub duration_ns: u64,
    pub signals: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ErrorTraceCandidate {
    pub trace_id: String,
    pub error_span_count: usize,
    pub confidence: Confidence,
    pub earliest_error_span: ErrorSpanCandidate,
    pub top_error_span: ErrorSpanCandidate,
    pub error_spans: Vec<ErrorSpanCandidate>,
    pub explanation: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DetectAnalysis {
    pub limit: usize,
    pub summary: DetectSummary,
    pub slow_traces: Vec<SlowTraceCandidate>,
    pub error_traces: Vec<ErrorTraceCandidate>,
    pub notes: Vec<String>,
}

pub fn analyze_detect(collection: &TraceCollection, limit: usize) -> DetectAnalysis {
    let mut durations = collection
        .traces
        .values()
        .filter_map(TraceGraph::duration_ns)
        .collect::<Vec<_>>();
    durations.sort_unstable();

    let sample_count = durations.len();
    let sample_quality = SampleQuality::from_sample_count(sample_count);
    let p95_duration_ns = percentile_nearest_rank(&durations, 95);

    let slow_traces = slow_trace_candidates(collection, limit, p95_duration_ns, sample_count);
    let error_traces = error_trace_candidates(collection, limit);
    let error_span_count = collection
        .traces
        .values()
        .map(detect_error_span_count)
        .sum::<usize>();

    let mut notes = Vec::new();
    if sample_quality == SampleQuality::Insufficient {
        notes.push(
            "trace sample count is below 5; slow-trace findings are low-confidence candidates"
                .to_string(),
        );
    } else if sample_quality == SampleQuality::Limited {
        notes.push(
            "trace sample count is below 20; percentile references are useful but still limited"
                .to_string(),
        );
    }
    notes.push(
        "N+1 detection is intentionally deferred to the next M5 step to avoid early false positives"
            .to_string(),
    );

    DetectAnalysis {
        limit,
        summary: DetectSummary {
            trace_count: collection.traces.len(),
            span_count: collection.span_count(),
            diagnostics_count: collection.diagnostics.len(),
            sample_count,
            sample_quality,
            p95_duration_ns,
            slow_trace_candidate_count: slow_traces.len(),
            error_trace_candidate_count: error_traces.len(),
            error_span_count,
        },
        slow_traces,
        error_traces,
        notes,
    }
}

fn slow_trace_candidates(
    collection: &TraceCollection,
    limit: usize,
    p95_duration_ns: Option<u64>,
    sample_count: usize,
) -> Vec<SlowTraceCandidate> {
    let mut ranked = collection
        .traces
        .values()
        .filter_map(|trace| trace.duration_ns().map(|duration| (trace, duration)))
        .collect::<Vec<_>>();

    ranked.sort_by(
        |(left_trace, left_duration), (right_trace, right_duration)| {
            right_duration
                .cmp(left_duration)
                .then(left_trace.trace_id.cmp(&right_trace.trace_id))
        },
    );

    ranked
        .into_iter()
        .enumerate()
        .take(limit)
        .map(|(index, (trace, duration_ns))| {
            let confidence = slow_confidence(sample_count, duration_ns, p95_duration_ns);
            SlowTraceCandidate {
                trace_id: trace.trace_id.clone(),
                rank: index + 1,
                duration_ns,
                p95_duration_ns,
                sample_count,
                confidence,
                reason: slow_reason(confidence, duration_ns, p95_duration_ns, sample_count),
                span_count: trace.spans.len(),
                service_count: trace.service_names().len(),
                error_span_count: detect_error_span_count(trace),
                diagnostics_count: trace.diagnostics.len(),
                service_candidates: service_slow_candidates(trace, 3),
            }
        })
        .collect()
}

fn slow_confidence(
    sample_count: usize,
    duration_ns: u64,
    p95_duration_ns: Option<u64>,
) -> Confidence {
    if sample_count < 5 {
        return Confidence::Low;
    }
    if p95_duration_ns.is_some_and(|p95| duration_ns < p95) {
        return Confidence::Low;
    }

    if sample_count < 20 {
        Confidence::Medium
    } else {
        Confidence::High
    }
}

fn slow_reason(
    confidence: Confidence,
    duration_ns: u64,
    p95_duration_ns: Option<u64>,
    sample_count: usize,
) -> String {
    match (confidence, p95_duration_ns, sample_count) {
        (Confidence::Low, Some(p95), sample_count) if sample_count < 5 && duration_ns >= p95 => {
            format!("longest trace in a very small sample; duration >= p95 reference ({p95}ns)")
        }
        (Confidence::Low, Some(p95), sample_count) if sample_count < 5 => {
            format!("ranked by duration in a very small sample; below p95 reference ({p95}ns)")
        }
        (Confidence::Low, None, sample_count) if sample_count < 5 => {
            "longest trace in a very small sample".to_string()
        }
        (Confidence::Low, Some(p95), _) if duration_ns < p95 => {
            format!("ranked by duration but below p95 reference ({p95}ns)")
        }
        (_, Some(p95), _) if duration_ns >= p95 => {
            format!("duration is at or above p95 reference ({p95}ns)")
        }
        (_, Some(p95), _) => format!("ranked by duration; p95 reference is {p95}ns"),
        (_, None, _) => "ranked by duration; no percentile reference is available".to_string(),
    }
}

fn service_slow_candidates(trace: &TraceGraph, limit: usize) -> Vec<ServiceSlowCandidate> {
    let mut by_service: BTreeMap<String, ServiceSlowCandidate> = BTreeMap::new();
    for span in &trace.spans {
        let entry = by_service
            .entry(span.service_name.clone())
            .or_insert_with(|| ServiceSlowCandidate {
                service_name: span.service_name.clone(),
                span_time_ns: 0,
                max_span_duration_ns: 0,
                span_count: 0,
                error_span_count: 0,
            });
        let duration_ns = span.duration_ns();
        entry.span_time_ns = entry.span_time_ns.saturating_add(duration_ns);
        entry.max_span_duration_ns = entry.max_span_duration_ns.max(duration_ns);
        entry.span_count += 1;
        if !error_signals(span).is_empty() {
            entry.error_span_count += 1;
        }
    }

    let mut candidates = by_service.into_values().collect::<Vec<_>>();
    candidates.sort_by(|left, right| {
        right
            .span_time_ns
            .cmp(&left.span_time_ns)
            .then(right.max_span_duration_ns.cmp(&left.max_span_duration_ns))
            .then(left.service_name.cmp(&right.service_name))
    });
    candidates.truncate(limit);
    candidates
}

fn error_trace_candidates(collection: &TraceCollection, limit: usize) -> Vec<ErrorTraceCandidate> {
    let mut candidates = collection
        .traces
        .values()
        .filter_map(error_trace_candidate)
        .collect::<Vec<_>>();

    candidates.sort_by(|left, right| {
        right
            .error_span_count
            .cmp(&left.error_span_count)
            .then(left.trace_id.cmp(&right.trace_id))
    });
    candidates.truncate(limit);
    candidates
}

fn error_trace_candidate(trace: &TraceGraph) -> Option<ErrorTraceCandidate> {
    let mut error_indices = trace
        .spans
        .iter()
        .enumerate()
        .filter_map(|(index, span)| {
            if error_signals(span).is_empty() {
                None
            } else {
                Some(index)
            }
        })
        .collect::<Vec<_>>();

    if error_indices.is_empty() {
        return None;
    }

    error_indices.sort_by(|left, right| {
        trace.spans[*left]
            .start_ns
            .cmp(&trace.spans[*right].start_ns)
            .then(trace.spans[*left].span_id.cmp(&trace.spans[*right].span_id))
    });
    let earliest_index = error_indices[0];
    let top_index = topological_highest_error_index(trace, &error_indices);
    let confidence = error_confidence(trace, &error_indices);
    let error_spans = error_indices
        .iter()
        .map(|index| error_span_candidate(trace, *index))
        .collect::<Vec<_>>();

    Some(ErrorTraceCandidate {
        trace_id: trace.trace_id.clone(),
        error_span_count: error_indices.len(),
        confidence,
        earliest_error_span: error_span_candidate(trace, earliest_index),
        top_error_span: error_span_candidate(trace, top_index),
        error_spans,
        explanation: "error signals were found in this trace; inspect earliest_error_span for the first visible signal and top_error_span for the highest-level failing span".to_string(),
    })
}

fn error_confidence(trace: &TraceGraph, error_indices: &[usize]) -> Confidence {
    let has_structured_status = error_indices.iter().any(|index| {
        error_signals(&trace.spans[*index])
            .iter()
            .any(|signal| signal != "exception_event")
    });

    if has_structured_status {
        Confidence::High
    } else {
        Confidence::Medium
    }
}

fn topological_highest_error_index(trace: &TraceGraph, error_indices: &[usize]) -> usize {
    error_indices
        .iter()
        .copied()
        .min_by(|left, right| {
            span_depth(trace, *left)
                .cmp(&span_depth(trace, *right))
                .then(
                    trace.spans[*left]
                        .start_ns
                        .cmp(&trace.spans[*right].start_ns),
                )
                .then(trace.spans[*left].span_id.cmp(&trace.spans[*right].span_id))
        })
        .expect("error_indices is non-empty")
}

fn error_span_candidate(trace: &TraceGraph, index: usize) -> ErrorSpanCandidate {
    let span = &trace.spans[index];
    ErrorSpanCandidate {
        span_id: span.span_id.clone(),
        parent_span_id: span.parent_span_id.clone(),
        service_name: span.service_name.clone(),
        name: span.name.clone(),
        depth: span_depth(trace, index),
        start_ns: span.start_ns,
        duration_ns: span.duration_ns(),
        signals: error_signals(span),
    }
}

fn span_depth(trace: &TraceGraph, index: usize) -> usize {
    let id_to_index = trace
        .spans
        .iter()
        .enumerate()
        .map(|(index, span)| (span.span_id.as_str(), index))
        .collect::<BTreeMap<_, _>>();
    let mut depth = 0;
    let mut cursor = &trace.spans[index];
    let mut visited = BTreeSet::new();

    while let Some(parent_span_id) = cursor.parent_span_id.as_deref() {
        if !visited.insert(cursor.span_id.as_str()) {
            break;
        }
        let Some(parent_index) = id_to_index.get(parent_span_id) else {
            break;
        };
        depth += 1;
        cursor = &trace.spans[*parent_index];
    }

    depth
}

fn detect_error_span_count(trace: &TraceGraph) -> usize {
    trace
        .spans
        .iter()
        .filter(|span| !error_signals(span).is_empty())
        .count()
}

fn error_signals(span: &CanonicalSpan) -> Vec<String> {
    let mut signals = Vec::new();
    if span.status_code == Some(2) {
        push_signal(&mut signals, "status_code_error");
    }
    if attribute_u16(span, "http.status_code").is_some_and(|status| status >= 500) {
        push_signal(&mut signals, "http_5xx");
    }
    if attribute_non_zero_status(span, "rpc.grpc.status_code")
        || attribute_non_zero_status(span, "grpc.status_code")
    {
        push_signal(&mut signals, "grpc_non_zero");
    }
    if attribute_non_zero_status(span, "rpc.status_code") {
        push_signal(&mut signals, "rpc_non_ok");
    }
    if span.events.iter().any(is_exception_event) {
        push_signal(&mut signals, "exception_event");
    }

    signals
}

fn push_signal(signals: &mut Vec<String>, signal: &'static str) {
    if !signals.iter().any(|existing| existing == signal) {
        signals.push(signal.to_string());
    }
}

fn attribute_u16(span: &CanonicalSpan, key: &str) -> Option<u16> {
    span.attributes.get(key)?.parse::<u16>().ok()
}

fn attribute_non_zero_status(span: &CanonicalSpan, key: &str) -> bool {
    let Some(value) = span.attributes.get(key) else {
        return false;
    };
    let normalized = value.trim().to_ascii_lowercase();
    !matches!(
        normalized.as_str(),
        "" | "0" | "ok" | "status_code_ok" | "unset" | "status_code_unset"
    )
}

fn is_exception_event(event: &crate::model::span::SpanEvent) -> bool {
    event.name.to_ascii_lowercase().contains("exception")
        || event
            .attributes
            .keys()
            .any(|key| key.starts_with("exception."))
}

fn percentile_nearest_rank(sorted: &[u64], percentile: usize) -> Option<u64> {
    if sorted.is_empty() {
        return None;
    }

    let rank = (sorted.len() * percentile).div_ceil(100);
    let index = rank.saturating_sub(1).min(sorted.len() - 1);
    Some(sorted[index])
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{Confidence, analyze_detect};
    use crate::graph::trace_graph::TraceCollection;
    use crate::input::otlp_json::parse_otlp_file;

    #[test]
    fn detects_slow_and_error_trace_candidates() {
        let data = parse_otlp_file(Path::new("tests/fixtures/otlp-detect.json"))
            .expect("fixture should parse");
        let collection = TraceCollection::build(data);
        let analysis = analyze_detect(&collection, 3);

        assert_eq!(analysis.summary.sample_count, 6);
        assert_eq!(analysis.summary.p95_duration_ns, Some(900_000_000));
        assert_eq!(
            analysis.slow_traces[0].trace_id,
            "66666666666666666666666666666666"
        );
        assert_eq!(analysis.slow_traces[0].confidence, Confidence::Medium);
        assert!(
            analysis.slow_traces[0]
                .service_candidates
                .iter()
                .any(|service| service.service_name == "checkout-service")
        );

        let error = analysis
            .error_traces
            .iter()
            .find(|trace| trace.trace_id == "66666666666666666666666666666666")
            .expect("error trace should be detected");
        assert_eq!(error.error_span_count, 4);
        assert_eq!(error.confidence, Confidence::High);
        assert_eq!(error.top_error_span.span_id, "6600000000000001");
        assert!(
            error
                .earliest_error_span
                .signals
                .contains(&"status_code_error".to_string())
        );
    }
}
