use std::collections::{BTreeMap, BTreeSet};

use crate::input::otlp_json::ParsedTraceData;
use crate::model::diagnostic::Diagnostic;
use crate::model::span::CanonicalSpan;

#[derive(Clone, Debug)]
pub struct TraceCollection {
    pub traces: BTreeMap<String, TraceGraph>,
    pub diagnostics: Vec<Diagnostic>,
}

impl TraceCollection {
    pub fn build(data: ParsedTraceData) -> Self {
        let mut grouped: BTreeMap<String, Vec<CanonicalSpan>> = BTreeMap::new();
        for span in data.spans {
            grouped.entry(span.trace_id.clone()).or_default().push(span);
        }

        let mut diagnostics = data.diagnostics;
        let mut traces = BTreeMap::new();

        for (trace_id, spans) in grouped {
            let trace = TraceGraph::build(trace_id.clone(), spans);
            diagnostics.extend(trace.diagnostics.clone());
            traces.insert(trace_id, trace);
        }

        Self {
            traces,
            diagnostics,
        }
    }

    pub fn span_count(&self) -> usize {
        self.traces.values().map(|trace| trace.spans.len()).sum()
    }
}

/// Aggregated cross-service call edge (parent service -> child service) within one trace.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CrossServiceEdge {
    /// Service name of the parent span on this edge.
    pub from_service: String,
    /// Service name of the child span on this edge.
    pub to_service: String,
    /// Number of parent->child calls aggregated into this edge direction.
    pub span_count: usize,
    /// Number of calls on this edge where the parent is a client span and the child is a server span.
    pub client_server_pair_count: usize,
    /// Sample child span ID retained from the first call on this edge.
    pub sample_span_id: String,
    /// Sample parent span ID retained from the first call on this edge.
    pub sample_parent_span_id: String,
}

/// Whether the OTLP span kind is CLIENT (3). Inlined in the graph module to avoid
/// a graph -> analysis reverse dependency on the annotations SpanRole helper.
fn is_client_kind(kind: Option<i64>) -> bool {
    matches!(kind, Some(3))
}

/// Whether the OTLP span kind is SERVER (2). Inlined in the graph module to avoid
/// a graph -> analysis reverse dependency on the annotations SpanRole helper.
fn is_server_kind(kind: Option<i64>) -> bool {
    matches!(kind, Some(2))
}

#[derive(Clone, Debug)]
pub struct TraceGraph {
    pub trace_id: String,
    pub spans: Vec<CanonicalSpan>,
    pub children_by_parent: BTreeMap<String, Vec<usize>>,
    pub root_indices: Vec<usize>,
    pub orphan_indices: Vec<usize>,
    pub duplicate_span_ids: BTreeSet<String>,
    pub diagnostics: Vec<Diagnostic>,
    pub cross_service_edges: Vec<CrossServiceEdge>,
}

impl TraceGraph {
    fn build(trace_id: String, mut spans: Vec<CanonicalSpan>) -> Self {
        spans.sort_by(|left, right| {
            left.start_ns
                .cmp(&right.start_ns)
                .then(left.end_ns.cmp(&right.end_ns))
                .then(left.span_id.cmp(&right.span_id))
        });

        let mut diagnostics = Vec::new();
        let mut id_to_indices: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        for (index, span) in spans.iter().enumerate() {
            id_to_indices
                .entry(span.span_id.clone())
                .or_default()
                .push(index);
        }

        let mut duplicate_span_ids = BTreeSet::new();
        for (span_id, indices) in &id_to_indices {
            if indices.len() > 1 {
                duplicate_span_ids.insert(span_id.clone());
                diagnostics.push(
                    Diagnostic::error(
                        "duplicate_span_id",
                        format!("spanId appears {} times in the same trace", indices.len()),
                    )
                    .with_trace_id(trace_id.clone())
                    .with_span_id(span_id.clone()),
                );
            }
        }

        let mut children_by_parent: BTreeMap<String, Vec<usize>> = BTreeMap::new();
        let mut root_indices = Vec::new();
        let mut orphan_indices = Vec::new();

        for (index, span) in spans.iter().enumerate() {
            let Some(parent_span_id) = &span.parent_span_id else {
                root_indices.push(index);
                continue;
            };

            if id_to_indices.contains_key(parent_span_id) {
                children_by_parent
                    .entry(parent_span_id.clone())
                    .or_default()
                    .push(index);

                if let Some(parent) = id_to_indices
                    .get(parent_span_id)
                    .and_then(|indices| indices.first())
                    .and_then(|parent_index| spans.get(*parent_index))
                    && (span.start_ns < parent.start_ns || span.end_ns > parent.end_ns)
                {
                    diagnostics.push(
                        Diagnostic::warning(
                            "child_outside_parent",
                            "child span starts before parent or ends after parent",
                        )
                        .with_trace_id(trace_id.clone())
                        .with_span_id(span.span_id.clone()),
                    );
                }
            } else {
                orphan_indices.push(index);
                diagnostics.push(
                    Diagnostic::warning(
                        "missing_parent",
                        format!("parentSpanId {parent_span_id} was not found in this trace"),
                    )
                    .with_trace_id(trace_id.clone())
                    .with_span_id(span.span_id.clone()),
                );
            }
        }

        for children in children_by_parent.values_mut() {
            children.sort_by(|left, right| {
                spans[*left]
                    .start_ns
                    .cmp(&spans[*right].start_ns)
                    .then(spans[*left].span_id.cmp(&spans[*right].span_id))
            });
        }
        root_indices.sort_by(|left, right| {
            spans[*left]
                .start_ns
                .cmp(&spans[*right].start_ns)
                .then(spans[*left].span_id.cmp(&spans[*right].span_id))
        });
        orphan_indices.sort_by(|left, right| {
            spans[*left]
                .start_ns
                .cmp(&spans[*right].start_ns)
                .then(spans[*left].span_id.cmp(&spans[*right].span_id))
        });

        if root_indices.len() > 1 {
            diagnostics.push(
                Diagnostic::warning(
                    "multiple_root_spans",
                    format!("trace has {} root spans", root_indices.len()),
                )
                .with_trace_id(trace_id.clone()),
            );
        } else if root_indices.is_empty() && !spans.is_empty() {
            diagnostics.push(
                Diagnostic::warning("no_root_span", "trace has no root span")
                    .with_trace_id(trace_id.clone()),
            );
        }

        let mut edges_by_direction: BTreeMap<(String, String), CrossServiceEdge> = BTreeMap::new();
        for (parent_span_id, children) in &children_by_parent {
            let Some(parent_index) = id_to_indices
                .get(parent_span_id)
                .and_then(|indices| indices.first())
                .copied()
            else {
                continue;
            };
            let parent_span = &spans[parent_index];
            for &child_index in children {
                let child_span = &spans[child_index];
                if parent_span.service_name == child_span.service_name {
                    continue;
                }
                let edge = edges_by_direction
                    .entry((
                        parent_span.service_name.clone(),
                        child_span.service_name.clone(),
                    ))
                    .or_insert_with(|| CrossServiceEdge {
                        from_service: parent_span.service_name.clone(),
                        to_service: child_span.service_name.clone(),
                        span_count: 0,
                        client_server_pair_count: 0,
                        sample_span_id: child_span.span_id.clone(),
                        sample_parent_span_id: parent_span.span_id.clone(),
                    });
                edge.span_count += 1;
                if is_client_kind(parent_span.kind) && is_server_kind(child_span.kind) {
                    edge.client_server_pair_count += 1;
                }
            }
        }

        let mut cross_service_edges: Vec<CrossServiceEdge> =
            edges_by_direction.into_values().collect();
        cross_service_edges.sort_by(|left, right| {
            right
                .span_count
                .cmp(&left.span_count)
                .then(left.from_service.cmp(&right.from_service))
                .then(left.to_service.cmp(&right.to_service))
        });

        Self {
            trace_id,
            spans,
            children_by_parent,
            root_indices,
            orphan_indices,
            duplicate_span_ids,
            diagnostics,
            cross_service_edges,
        }
    }

    pub fn start_ns(&self) -> Option<u64> {
        self.spans.iter().map(|span| span.start_ns).min()
    }

    pub fn end_ns(&self) -> Option<u64> {
        self.spans.iter().map(|span| span.end_ns).max()
    }

    pub fn duration_ns(&self) -> Option<u64> {
        Some(self.end_ns()?.saturating_sub(self.start_ns()?))
    }

    pub fn error_span_count(&self) -> usize {
        self.spans.iter().filter(|span| span.is_error()).count()
    }

    pub fn service_names(&self) -> BTreeSet<&str> {
        self.spans
            .iter()
            .map(|span| span.service_name.as_str())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::Path;

    use crate::graph::trace_graph::TraceCollection;
    use crate::input::otlp_json::{ParsedTraceData, parse_otlp_file};
    use crate::model::span::CanonicalSpan;

    #[test]
    fn detects_missing_parent() {
        let data = parse_otlp_file(Path::new("tests/fixtures/otlp-missing-parent.json"))
            .expect("fixture should parse");
        let collection = TraceCollection::build(data);

        assert!(
            collection
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "missing_parent")
        );
    }

    #[test]
    fn detects_duplicate_span_ids() {
        let data = parse_otlp_file(Path::new("tests/fixtures/otlp-duplicate-span.json"))
            .expect("fixture should parse");
        let collection = TraceCollection::build(data);

        assert!(
            collection
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "duplicate_span_id")
        );
    }

    #[test]
    fn detects_multiple_roots() {
        let data = parse_otlp_file(Path::new("tests/fixtures/otlp-multiple-roots.json"))
            .expect("fixture should parse");
        let collection = TraceCollection::build(data);

        assert!(
            collection
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "multiple_root_spans")
        );
    }

    #[test]
    fn detects_no_root() {
        let data = parse_otlp_file(Path::new("tests/fixtures/otlp-no-root.json"))
            .expect("fixture should parse");
        let collection = TraceCollection::build(data);

        assert!(
            collection
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "no_root_span")
        );
    }

    #[test]
    fn detects_child_outside_parent() {
        let data = parse_otlp_file(Path::new("tests/fixtures/otlp-child-outside-parent.json"))
            .expect("fixture should parse");
        let collection = TraceCollection::build(data);

        assert!(
            collection
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "child_outside_parent")
        );
    }

    #[test]
    fn aggregates_cross_service_edges_sorted_by_span_count_desc() {
        // root (checkout) -> c1 (cart), c2 (payment), c3 (cart); c1 -> g1 (inventory).
        // Expected edges: checkout->cart (span_count 2), cart->inventory (1),
        // checkout->payment (1). Sorted by span_count desc then (from, to) asc.
        let collection = collection_with(vec![
            span("root", None, "checkout-service", None, 0, 100),
            span("c1", Some("root"), "cart-service", None, 10, 20),
            span("c2", Some("root"), "payment-service", None, 30, 40),
            span("c3", Some("root"), "cart-service", None, 50, 60),
            span("g1", Some("c1"), "inventory-service", None, 12, 15),
        ]);
        let trace = collection
            .traces
            .values()
            .next()
            .expect("trace should be present");

        let edges = &trace.cross_service_edges;
        assert_eq!(edges.len(), 3);
        let order: Vec<(usize, &str, &str, usize)> = edges
            .iter()
            .map(|edge| {
                (
                    edge.span_count,
                    edge.from_service.as_str(),
                    edge.to_service.as_str(),
                    edge.client_server_pair_count,
                )
            })
            .collect();
        assert_eq!(
            order,
            vec![
                (2, "checkout-service", "cart-service", 0),
                (1, "cart-service", "inventory-service", 0),
                (1, "checkout-service", "payment-service", 0),
            ]
        );
        // sample retained from the first call on the busiest edge.
        assert_eq!(edges[0].sample_span_id, "c1");
        assert_eq!(edges[0].sample_parent_span_id, "root");
    }

    #[test]
    fn single_service_trace_has_no_cross_service_edges() {
        let collection = collection_with(vec![
            span("root", None, "checkout-service", None, 0, 100),
            span("child", Some("root"), "checkout-service", None, 10, 20),
        ]);
        let trace = collection
            .traces
            .values()
            .next()
            .expect("trace should be present");

        assert!(trace.cross_service_edges.is_empty());
    }

    #[test]
    fn aggregates_same_direction_calls_and_counts_client_server_pairs() {
        // root (server) -> mid (client, same service as root: no edge). mid ->
        // two cart-service server spans (client/server pairs) and one
        // discount-service client span (cross-service but not a pair).
        let collection = collection_with(vec![
            span("root", None, "checkout-service", Some(2), 0, 100),
            span("mid", Some("root"), "checkout-service", Some(3), 10, 90),
            span("a", Some("mid"), "cart-service", Some(2), 20, 30),
            span("b", Some("mid"), "cart-service", Some(2), 40, 50),
            span("c", Some("mid"), "discount-service", Some(3), 60, 70),
        ]);
        let trace = collection
            .traces
            .values()
            .next()
            .expect("trace should be present");

        let edges = &trace.cross_service_edges;
        assert_eq!(edges.len(), 2);

        let cart = edges
            .iter()
            .find(|edge| edge.to_service == "cart-service")
            .expect("cart-service edge should exist");
        assert_eq!(cart.from_service, "checkout-service");
        assert_eq!(cart.span_count, 2);
        assert_eq!(cart.client_server_pair_count, 2);

        let discount = edges
            .iter()
            .find(|edge| edge.to_service == "discount-service")
            .expect("discount-service edge should exist");
        assert_eq!(discount.from_service, "checkout-service");
        assert_eq!(discount.span_count, 1);
        assert_eq!(discount.client_server_pair_count, 0);
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
        kind: Option<i64>,
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
            kind,
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
