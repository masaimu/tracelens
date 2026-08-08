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
    pub error_propagation_chain_count: usize,
    pub n_plus_one_candidate_count: usize,
    pub service_latency_distribution_count: usize,
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
pub struct ErrorPropagationStep {
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub service_name: String,
    pub name: String,
    pub depth: usize,
    pub start_ns: u64,
    pub duration_ns: u64,
    pub is_error: bool,
    pub signals: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ErrorPropagationChain {
    pub trace_id: String,
    pub confidence: Confidence,
    pub earliest_error_span: ErrorSpanCandidate,
    pub top_error_span: ErrorSpanCandidate,
    pub path_to_earliest_error: Vec<ErrorPropagationStep>,
    pub downstream_error_spans: Vec<ErrorPropagationStep>,
    pub downstream_error_span_count: usize,
    pub affected_span_count: usize,
    pub affected_services: Vec<String>,
    pub explanation: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NPlusOneSpanRef {
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub service_name: String,
    pub name: String,
    pub depth: usize,
    pub start_ns: u64,
    pub duration_ns: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NPlusOneChildGroup {
    pub service_name: String,
    pub normalized_name: String,
    pub db_system: Option<String>,
    pub db_operation: Option<String>,
    pub rpc_system: Option<String>,
    pub http_method: Option<String>,
    pub http_route: Option<String>,
    pub signature: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NPlusOneCandidate {
    pub trace_id: String,
    pub parent_span: NPlusOneSpanRef,
    pub child_group: NPlusOneChildGroup,
    pub repeated_count: usize,
    pub serial_ratio_per_mille: u16,
    pub confidence: Confidence,
    pub reason: String,
    pub example_child_spans: Vec<NPlusOneSpanRef>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServiceLatencySpanSample {
    pub trace_id: String,
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub name: String,
    pub start_ns: u64,
    pub duration_ns: u64,
    pub is_error: bool,
    pub signals: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServiceLatencyDistribution {
    pub service_name: String,
    pub trace_count: usize,
    pub span_count: usize,
    pub error_span_count: usize,
    pub total_span_time_ns: u64,
    pub p50_duration_ns: u64,
    pub p95_duration_ns: u64,
    pub p99_duration_ns: Option<u64>,
    pub p999_duration_ns: Option<u64>,
    pub max_span_duration_ns: u64,
    pub slow_span_samples: Vec<ServiceLatencySpanSample>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DetectAnalysis {
    pub limit: usize,
    pub summary: DetectSummary,
    pub slow_traces: Vec<SlowTraceCandidate>,
    pub error_traces: Vec<ErrorTraceCandidate>,
    pub error_propagation_chains: Vec<ErrorPropagationChain>,
    pub n_plus_one_candidates: Vec<NPlusOneCandidate>,
    pub service_latency_distribution: Vec<ServiceLatencyDistribution>,
    pub notes: Vec<String>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct NPlusOneGroupKey {
    service_name: String,
    normalized_name: String,
    db_system: Option<String>,
    db_operation: Option<String>,
    rpc_system: Option<String>,
    http_method: Option<String>,
    http_route: Option<String>,
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
    let error_propagation_chains = error_propagation_chains(collection, limit);
    let n_plus_one_candidates = n_plus_one_candidates(collection, limit);
    let service_latency_distribution = service_latency_distribution(collection, limit);
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
        "N+1 detection uses same-parent direct child span heuristics; inspect candidates before treating them as root cause"
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
            error_propagation_chain_count: error_propagation_chains.len(),
            n_plus_one_candidate_count: n_plus_one_candidates.len(),
            service_latency_distribution_count: service_latency_distribution.len(),
            error_span_count,
        },
        slow_traces,
        error_traces,
        error_propagation_chains,
        n_plus_one_candidates,
        service_latency_distribution,
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

fn error_propagation_chains(
    collection: &TraceCollection,
    limit: usize,
) -> Vec<ErrorPropagationChain> {
    let mut chains = collection
        .traces
        .values()
        .filter_map(error_propagation_chain)
        .collect::<Vec<_>>();

    chains.sort_by(|left, right| {
        confidence_rank(right.confidence)
            .cmp(&confidence_rank(left.confidence))
            .then(
                right
                    .downstream_error_span_count
                    .cmp(&left.downstream_error_span_count),
            )
            .then(right.affected_span_count.cmp(&left.affected_span_count))
            .then(left.trace_id.cmp(&right.trace_id))
    });
    chains.truncate(limit);
    chains
}

fn error_propagation_chain(trace: &TraceGraph) -> Option<ErrorPropagationChain> {
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

    let path_indices = path_to_span(trace, earliest_index);
    let path_to_earliest_error = path_indices
        .iter()
        .map(|index| error_propagation_step(trace, *index))
        .collect::<Vec<_>>();

    let descendant_indices = descendant_indices(trace, &trace.spans[top_index].span_id);
    let affected_span_count = descendant_indices.len().saturating_add(1);
    let mut downstream_error_indices = descendant_indices
        .into_iter()
        .filter(|index| *index != top_index && !error_signals(&trace.spans[*index]).is_empty())
        .collect::<Vec<_>>();
    downstream_error_indices.sort_by(|left, right| {
        trace.spans[*left]
            .start_ns
            .cmp(&trace.spans[*right].start_ns)
            .then(span_depth(trace, *left).cmp(&span_depth(trace, *right)))
            .then(trace.spans[*left].span_id.cmp(&trace.spans[*right].span_id))
    });

    let downstream_error_span_count = downstream_error_indices.len();
    let downstream_error_spans = downstream_error_indices
        .iter()
        .take(10)
        .map(|index| error_propagation_step(trace, *index))
        .collect::<Vec<_>>();

    let mut affected_services = BTreeSet::new();
    affected_services.insert(trace.spans[top_index].service_name.clone());
    for index in path_indices.iter().chain(downstream_error_indices.iter()) {
        affected_services.insert(trace.spans[*index].service_name.clone());
    }

    Some(ErrorPropagationChain {
        trace_id: trace.trace_id.clone(),
        confidence,
        earliest_error_span: error_span_candidate(trace, earliest_index),
        top_error_span: error_span_candidate(trace, top_index),
        path_to_earliest_error,
        downstream_error_spans,
        downstream_error_span_count,
        affected_span_count,
        affected_services: affected_services.into_iter().collect(),
        explanation: "path_to_earliest_error follows parent-child topology from the visible root or orphan entry point to the first error signal; downstream_error_spans lists later error evidence below top_error_span".to_string(),
    })
}

fn path_to_span(trace: &TraceGraph, index: usize) -> Vec<usize> {
    let id_to_index = first_span_id_to_index(trace);
    let mut path = Vec::new();
    let mut cursor_index = index;
    let mut visited = BTreeSet::new();

    loop {
        let span = &trace.spans[cursor_index];
        if !visited.insert(span.span_id.as_str()) {
            break;
        }
        path.push(cursor_index);

        let Some(parent_span_id) = span.parent_span_id.as_deref() else {
            break;
        };
        let Some(parent_index) = id_to_index.get(parent_span_id).copied() else {
            break;
        };
        cursor_index = parent_index;
    }

    path.reverse();
    path
}

fn descendant_indices(trace: &TraceGraph, span_id: &str) -> Vec<usize> {
    let mut descendants = Vec::new();
    let mut stack = trace
        .children_by_parent
        .get(span_id)
        .cloned()
        .unwrap_or_default();
    let mut visited = BTreeSet::new();

    while let Some(index) = stack.pop() {
        if !visited.insert(index) {
            continue;
        }
        descendants.push(index);
        let child_span_id = &trace.spans[index].span_id;
        if let Some(children) = trace.children_by_parent.get(child_span_id) {
            stack.extend(children.iter().copied());
        }
    }

    descendants.sort_by(|left, right| {
        trace.spans[*left]
            .start_ns
            .cmp(&trace.spans[*right].start_ns)
            .then(span_depth(trace, *left).cmp(&span_depth(trace, *right)))
            .then(trace.spans[*left].span_id.cmp(&trace.spans[*right].span_id))
    });
    descendants
}

fn error_propagation_step(trace: &TraceGraph, index: usize) -> ErrorPropagationStep {
    let span = &trace.spans[index];
    let signals = error_signals(span);
    ErrorPropagationStep {
        span_id: span.span_id.clone(),
        parent_span_id: span.parent_span_id.clone(),
        service_name: span.service_name.clone(),
        name: span.name.clone(),
        depth: span_depth(trace, index),
        start_ns: span.start_ns,
        duration_ns: span.duration_ns(),
        is_error: !signals.is_empty(),
        signals,
    }
}

fn service_latency_distribution(
    collection: &TraceCollection,
    limit: usize,
) -> Vec<ServiceLatencyDistribution> {
    #[derive(Default)]
    struct ServiceLatencyAccumulator {
        trace_ids: BTreeSet<String>,
        durations: Vec<u64>,
        error_span_count: usize,
        total_span_time_ns: u64,
        slow_span_samples: Vec<ServiceLatencySpanSample>,
    }

    let mut by_service: BTreeMap<String, ServiceLatencyAccumulator> = BTreeMap::new();

    for trace in collection.traces.values() {
        for span in &trace.spans {
            let entry = by_service.entry(span.service_name.clone()).or_default();
            let duration_ns = span.duration_ns();
            let signals = error_signals(span);

            entry.trace_ids.insert(trace.trace_id.clone());
            entry.durations.push(duration_ns);
            entry.total_span_time_ns = entry.total_span_time_ns.saturating_add(duration_ns);
            if !signals.is_empty() {
                entry.error_span_count += 1;
            }
            entry.slow_span_samples.push(ServiceLatencySpanSample {
                trace_id: trace.trace_id.clone(),
                span_id: span.span_id.clone(),
                parent_span_id: span.parent_span_id.clone(),
                name: span.name.clone(),
                start_ns: span.start_ns,
                duration_ns,
                is_error: !signals.is_empty(),
                signals,
            });
        }
    }

    let mut distributions = by_service
        .into_iter()
        .filter_map(|(service_name, mut accumulator)| {
            accumulator.durations.sort_unstable();
            let p50_duration_ns = percentile_nearest_rank(&accumulator.durations, 50)?;
            let p95_duration_ns = percentile_nearest_rank(&accumulator.durations, 95)?;
            let p99_duration_ns = (accumulator.durations.len() >= 20)
                .then(|| percentile_nearest_rank(&accumulator.durations, 99))
                .flatten();
            let p999_duration_ns = (accumulator.durations.len() >= 100)
                .then(|| percentile_nearest_rank(&accumulator.durations, 999))
                .flatten();
            let max_span_duration_ns = accumulator.durations.last().copied()?;
            accumulator.slow_span_samples.sort_by(|left, right| {
                right
                    .duration_ns
                    .cmp(&left.duration_ns)
                    .then(left.trace_id.cmp(&right.trace_id))
                    .then(left.span_id.cmp(&right.span_id))
            });
            accumulator.slow_span_samples.truncate(3);

            Some(ServiceLatencyDistribution {
                service_name,
                trace_count: accumulator.trace_ids.len(),
                span_count: accumulator.durations.len(),
                error_span_count: accumulator.error_span_count,
                total_span_time_ns: accumulator.total_span_time_ns,
                p50_duration_ns,
                p95_duration_ns,
                p99_duration_ns,
                p999_duration_ns,
                max_span_duration_ns,
                slow_span_samples: accumulator.slow_span_samples,
            })
        })
        .collect::<Vec<_>>();

    distributions.sort_by(|left, right| {
        right
            .p95_duration_ns
            .cmp(&left.p95_duration_ns)
            .then(right.max_span_duration_ns.cmp(&left.max_span_duration_ns))
            .then(right.total_span_time_ns.cmp(&left.total_span_time_ns))
            .then(left.service_name.cmp(&right.service_name))
    });
    distributions.truncate(limit);
    distributions
}

fn n_plus_one_candidates(collection: &TraceCollection, limit: usize) -> Vec<NPlusOneCandidate> {
    let mut candidates = collection
        .traces
        .values()
        .flat_map(trace_n_plus_one_candidates)
        .collect::<Vec<_>>();

    candidates.sort_by(|left, right| {
        confidence_rank(right.confidence)
            .cmp(&confidence_rank(left.confidence))
            .then(right.repeated_count.cmp(&left.repeated_count))
            .then(
                right
                    .serial_ratio_per_mille
                    .cmp(&left.serial_ratio_per_mille),
            )
            .then(left.trace_id.cmp(&right.trace_id))
            .then(left.parent_span.span_id.cmp(&right.parent_span.span_id))
            .then(left.child_group.signature.cmp(&right.child_group.signature))
    });
    candidates.truncate(limit);
    candidates
}

fn trace_n_plus_one_candidates(trace: &TraceGraph) -> Vec<NPlusOneCandidate> {
    let span_id_to_index = first_span_id_to_index(trace);
    let mut candidates = Vec::new();

    for (parent_span_id, child_indices) in &trace.children_by_parent {
        let Some(parent_index) = span_id_to_index.get(parent_span_id.as_str()).copied() else {
            continue;
        };

        let mut groups: BTreeMap<NPlusOneGroupKey, Vec<usize>> = BTreeMap::new();
        for child_index in child_indices {
            let child = &trace.spans[*child_index];
            groups
                .entry(n_plus_one_group_key(child))
                .or_default()
                .push(*child_index);
        }

        for (group_key, mut group_child_indices) in groups {
            if group_child_indices.len() < 5 {
                continue;
            }

            group_child_indices.sort_by(|left, right| {
                trace.spans[*left]
                    .start_ns
                    .cmp(&trace.spans[*right].start_ns)
                    .then(trace.spans[*left].span_id.cmp(&trace.spans[*right].span_id))
            });
            let repeated_count = group_child_indices.len();
            let serial_ratio_per_mille = serial_ratio_per_mille(trace, &group_child_indices);
            let confidence = n_plus_one_confidence(repeated_count, serial_ratio_per_mille);

            candidates.push(NPlusOneCandidate {
                trace_id: trace.trace_id.clone(),
                parent_span: n_plus_one_span_ref(trace, parent_index),
                child_group: n_plus_one_child_group(group_key),
                repeated_count,
                serial_ratio_per_mille,
                confidence,
                reason: n_plus_one_reason(repeated_count, serial_ratio_per_mille, confidence),
                example_child_spans: group_child_indices
                    .iter()
                    .take(3)
                    .map(|index| n_plus_one_span_ref(trace, *index))
                    .collect(),
            });
        }
    }

    candidates
}

fn n_plus_one_confidence(repeated_count: usize, serial_ratio_per_mille: u16) -> Confidence {
    if repeated_count >= 10 && serial_ratio_per_mille >= 800 {
        Confidence::High
    } else {
        Confidence::Medium
    }
}

fn n_plus_one_reason(
    repeated_count: usize,
    serial_ratio_per_mille: u16,
    confidence: Confidence,
) -> String {
    match confidence {
        Confidence::High => format!(
            "repeated child spans >= 10 and serial ratio is {} per mille",
            serial_ratio_per_mille
        ),
        Confidence::Medium if repeated_count >= 10 => format!(
            "repeated child spans >= 10 but serial ratio is only {} per mille",
            serial_ratio_per_mille
        ),
        Confidence::Medium => format!("repeated child spans >= 5 ({repeated_count})"),
        Confidence::Low => "below N+1 threshold".to_string(),
    }
}

fn serial_ratio_per_mille(trace: &TraceGraph, child_indices: &[usize]) -> u16 {
    if child_indices.len() <= 1 {
        return 1_000;
    }

    let serial_pairs = child_indices
        .windows(2)
        .filter(|window| trace.spans[window[1]].start_ns >= trace.spans[window[0]].end_ns)
        .count();
    let adjacent_pairs = child_indices.len() - 1;
    ((serial_pairs * 1_000 + adjacent_pairs / 2) / adjacent_pairs) as u16
}

fn n_plus_one_group_key(span: &CanonicalSpan) -> NPlusOneGroupKey {
    NPlusOneGroupKey {
        service_name: span.service_name.clone(),
        normalized_name: normalize_span_name_for_n_plus_one(&span.name),
        db_system: attribute_value(span, "db.system"),
        db_operation: attribute_value(span, "db.operation"),
        rpc_system: attribute_value(span, "rpc.system"),
        http_method: attribute_value(span, "http.method"),
        http_route: attribute_value(span, "http.route"),
    }
}

fn n_plus_one_child_group(key: NPlusOneGroupKey) -> NPlusOneChildGroup {
    let signature = n_plus_one_signature(&key);
    NPlusOneChildGroup {
        service_name: key.service_name,
        normalized_name: key.normalized_name,
        db_system: key.db_system,
        db_operation: key.db_operation,
        rpc_system: key.rpc_system,
        http_method: key.http_method,
        http_route: key.http_route,
        signature,
    }
}

fn n_plus_one_signature(key: &NPlusOneGroupKey) -> String {
    [
        format!("service={}", key.service_name),
        format!("name={}", key.normalized_name),
        format_option("db.system", key.db_system.as_deref()),
        format_option("db.operation", key.db_operation.as_deref()),
        format_option("rpc.system", key.rpc_system.as_deref()),
        format_option("http.method", key.http_method.as_deref()),
        format_option("http.route", key.http_route.as_deref()),
    ]
    .into_iter()
    .filter(|part| !part.ends_with("=<none>"))
    .collect::<Vec<_>>()
    .join(" ")
}

fn format_option(key: &str, value: Option<&str>) -> String {
    format!("{key}={}", value.unwrap_or("<none>"))
}

fn n_plus_one_span_ref(trace: &TraceGraph, index: usize) -> NPlusOneSpanRef {
    let span = &trace.spans[index];
    NPlusOneSpanRef {
        span_id: span.span_id.clone(),
        parent_span_id: span.parent_span_id.clone(),
        service_name: span.service_name.clone(),
        name: span.name.clone(),
        depth: span_depth(trace, index),
        start_ns: span.start_ns,
        duration_ns: span.duration_ns(),
    }
}

fn normalize_span_name_for_n_plus_one(name: &str) -> String {
    let name = name.trim().to_ascii_lowercase();
    let name = name.split('?').next().unwrap_or(name.as_str());
    let mut output = String::new();
    let mut in_number = false;
    let mut previous_whitespace = false;

    for character in name.chars() {
        if character.is_ascii_digit() {
            if !in_number {
                output.push_str("{num}");
                in_number = true;
            }
            previous_whitespace = false;
            continue;
        }

        in_number = false;
        if character.is_whitespace() {
            if !previous_whitespace {
                output.push(' ');
                previous_whitespace = true;
            }
        } else {
            output.push(character);
            previous_whitespace = false;
        }
    }

    output.trim().to_string()
}

fn attribute_value(span: &CanonicalSpan, key: &str) -> Option<String> {
    span.attributes
        .get(key)
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn first_span_id_to_index(trace: &TraceGraph) -> BTreeMap<&str, usize> {
    let mut map = BTreeMap::new();
    for (index, span) in trace.spans.iter().enumerate() {
        map.entry(span.span_id.as_str()).or_insert(index);
    }
    map
}

fn confidence_rank(confidence: Confidence) -> u8 {
    match confidence {
        Confidence::Low => 0,
        Confidence::Medium => 1,
        Confidence::High => 2,
    }
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

    use std::collections::BTreeMap;

    use super::{Confidence, analyze_detect};
    use crate::graph::trace_graph::TraceCollection;
    use crate::input::otlp_json::{ParsedTraceData, parse_otlp_file};
    use crate::model::span::CanonicalSpan;

    #[test]
    fn detects_slow_and_error_trace_candidates() {
        let data = parse_otlp_file(Path::new("tests/fixtures/otlp-detect.json"))
            .expect("fixture should parse");
        let collection = TraceCollection::build(data);
        let analysis = analyze_detect(&collection, 3);

        assert_eq!(analysis.summary.sample_count, 6);
        assert_eq!(analysis.summary.p95_duration_ns, Some(900_000_000));
        assert_eq!(analysis.summary.error_propagation_chain_count, 1);
        assert_eq!(analysis.summary.service_latency_distribution_count, 3);
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

        let chain = analysis
            .error_propagation_chains
            .iter()
            .find(|chain| chain.trace_id == "66666666666666666666666666666666")
            .expect("error propagation chain should be detected");
        assert_eq!(chain.path_to_earliest_error.len(), 1);
        assert_eq!(chain.downstream_error_span_count, 3);
        assert_eq!(chain.affected_span_count, 4);
        assert!(
            chain
                .affected_services
                .contains(&"payment-service".to_string())
        );

        let checkout = analysis
            .service_latency_distribution
            .iter()
            .find(|service| service.service_name == "checkout-service")
            .expect("checkout latency distribution should be present");
        assert_eq!(checkout.p95_duration_ns, 900_000_000);
        assert_eq!(checkout.max_span_duration_ns, 900_000_000);
        assert_eq!(checkout.error_span_count, 1);
    }

    #[test]
    fn detects_n_plus_one_candidates_conservatively() {
        let data = parse_otlp_file(Path::new("tests/fixtures/otlp-n-plus-one.json"))
            .expect("fixture should parse");
        let collection = TraceCollection::build(data);
        let analysis = analyze_detect(&collection, 5);

        assert_eq!(analysis.summary.n_plus_one_candidate_count, 2);

        let high = analysis
            .n_plus_one_candidates
            .iter()
            .find(|candidate| candidate.trace_id == "77777777777777777777777777777777")
            .expect("serial repeated database calls should be detected");
        assert_eq!(high.repeated_count, 10);
        assert_eq!(high.serial_ratio_per_mille, 1_000);
        assert_eq!(high.confidence, Confidence::High);
        assert_eq!(high.child_group.normalized_name, "select product {num}");
        assert_eq!(high.child_group.db_system.as_deref(), Some("postgresql"));

        let concurrent = analysis
            .n_plus_one_candidates
            .iter()
            .find(|candidate| candidate.trace_id == "88888888888888888888888888888888")
            .expect("concurrent repeated calls should still be a possible candidate");
        assert_eq!(concurrent.repeated_count, 6);
        assert_eq!(concurrent.serial_ratio_per_mille, 0);
        assert_eq!(concurrent.confidence, Confidence::Medium);
    }

    #[test]
    fn service_latency_distribution_reports_p99_p999_when_sample_large_enough() {
        let collection = latency_collection(120);
        let analysis = analyze_detect(&collection, 10);
        let svc = analysis
            .service_latency_distribution
            .iter()
            .find(|distribution| distribution.service_name == "svc")
            .expect("svc latency distribution should be present");
        assert_eq!(svc.span_count, 120);
        assert_eq!(svc.p99_duration_ns, Some(1_000));
        assert_eq!(svc.p999_duration_ns, Some(1_000));
    }

    #[test]
    fn service_latency_distribution_p99_p999_null_for_small_samples() {
        let medium = analyze_detect(&latency_collection(25), 10)
            .service_latency_distribution
            .into_iter()
            .find(|distribution| distribution.service_name == "svc")
            .expect("svc latency distribution should be present");
        assert_eq!(medium.span_count, 25);
        assert_eq!(medium.p99_duration_ns, Some(1_000));
        assert_eq!(medium.p999_duration_ns, None);

        let small = analyze_detect(&latency_collection(5), 10)
            .service_latency_distribution
            .into_iter()
            .find(|distribution| distribution.service_name == "svc")
            .expect("svc latency distribution should be present");
        assert_eq!(small.span_count, 5);
        assert_eq!(small.p99_duration_ns, None);
        assert_eq!(small.p999_duration_ns, None);
    }

    fn latency_collection(span_count: usize) -> TraceCollection {
        let spans = (0..span_count)
            .map(|index| latency_span(&format!("s{index}"), "svc", 0, 1_000))
            .collect::<Vec<_>>();
        TraceCollection::build(ParsedTraceData {
            spans,
            diagnostics: Vec::new(),
        })
    }

    fn latency_span(
        span_id: &str,
        service_name: &str,
        start_ns: u64,
        end_ns: u64,
    ) -> CanonicalSpan {
        CanonicalSpan {
            trace_id: "trace".to_string(),
            span_id: span_id.to_string(),
            parent_span_id: None,
            trace_state: None,
            flags: None,
            service_name: service_name.to_string(),
            name: "op".to_string(),
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
