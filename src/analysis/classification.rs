use std::collections::BTreeMap;

use crate::graph::trace_graph::TraceGraph;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceClassification {
    pub counts: ClassificationCounts,
    pub spans: Vec<SpanClassification>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ClassificationCounts {
    pub serial: usize,
    pub concurrent: usize,
    pub nested: usize,
    pub suspicious: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpanClassification {
    pub span_id: String,
    pub service_name: String,
    pub name: String,
    pub sibling_relation: SiblingRelation,
    pub parent_relation: Option<ParentRelation>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SiblingRelation {
    Serial,
    Concurrent,
}

impl SiblingRelation {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Serial => "serial",
            Self::Concurrent => "concurrent",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ParentRelation {
    Nested,
    Suspicious,
}

impl ParentRelation {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Nested => "nested",
            Self::Suspicious => "suspicious",
        }
    }
}

pub fn classify_trace_spans(trace: &TraceGraph) -> TraceClassification {
    let mut is_root = vec![false; trace.spans.len()];
    for index in &trace.root_indices {
        is_root[*index] = true;
    }
    let mut is_orphan = vec![false; trace.spans.len()];
    for index in &trace.orphan_indices {
        is_orphan[*index] = true;
    }

    let sibling_relations = classify_sibling_relations(trace);
    let parent_by_span_id = first_parent_indices(trace);

    let mut spans = Vec::new();
    let mut counts = ClassificationCounts::default();

    for (index, span) in trace.spans.iter().enumerate() {
        let sibling_relation = sibling_relations[index];

        let parent_relation = if is_root[index] || is_orphan[index] {
            None
        } else {
            let parent = span
                .parent_span_id
                .as_deref()
                .and_then(|parent_id| parent_by_span_id.get(parent_id).copied())
                .and_then(|parent_index| trace.spans.get(parent_index));
            parent.map(|parent| {
                if span.start_ns < parent.start_ns || span.end_ns > parent.end_ns {
                    ParentRelation::Suspicious
                } else {
                    ParentRelation::Nested
                }
            })
        };

        match sibling_relation {
            SiblingRelation::Serial => counts.serial += 1,
            SiblingRelation::Concurrent => counts.concurrent += 1,
        }
        match parent_relation {
            Some(ParentRelation::Nested) => counts.nested += 1,
            Some(ParentRelation::Suspicious) => counts.suspicious += 1,
            None => {}
        }

        spans.push(SpanClassification {
            span_id: span.span_id.clone(),
            service_name: span.service_name.clone(),
            name: span.name.clone(),
            sibling_relation,
            parent_relation,
        });
    }

    TraceClassification { counts, spans }
}

fn classify_sibling_relations(trace: &TraceGraph) -> Vec<SiblingRelation> {
    let mut relations = vec![SiblingRelation::Serial; trace.spans.len()];

    mark_concurrent_in_group(trace, &trace.root_indices, &mut relations);

    for children in trace.children_by_parent.values() {
        mark_concurrent_in_group(trace, children, &mut relations);
    }

    let mut orphan_groups: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    for index in &trace.orphan_indices {
        if let Some(parent_id) = trace.spans[*index].parent_span_id.as_deref() {
            orphan_groups.entry(parent_id).or_default().push(*index);
        }
    }
    for group in orphan_groups.values() {
        mark_concurrent_in_group(trace, group, &mut relations);
    }

    relations
}

fn mark_concurrent_in_group(
    trace: &TraceGraph,
    indices: &[usize],
    relations: &mut [SiblingRelation],
) {
    if indices.len() < 2 {
        return;
    }

    let mut sorted = indices.to_vec();
    sorted.sort_by(|left, right| {
        let left_span = &trace.spans[*left];
        let right_span = &trace.spans[*right];
        left_span
            .start_ns
            .cmp(&right_span.start_ns)
            .then(left_span.end_ns.cmp(&right_span.end_ns))
            .then(left_span.span_id.cmp(&right_span.span_id))
    });

    let mut max_end_index = sorted[0];
    let mut max_end_ns = trace.spans[max_end_index].end_ns;

    for index in sorted.into_iter().skip(1) {
        let span = &trace.spans[index];
        if max_end_ns > span.start_ns {
            relations[index] = SiblingRelation::Concurrent;
            relations[max_end_index] = SiblingRelation::Concurrent;
        }
        if span.end_ns > max_end_ns {
            max_end_ns = span.end_ns;
            max_end_index = index;
        }
    }
}

fn first_parent_indices(trace: &TraceGraph) -> BTreeMap<&str, usize> {
    let mut parent_by_span_id = BTreeMap::new();
    for (index, span) in trace.spans.iter().enumerate() {
        parent_by_span_id
            .entry(span.span_id.as_str())
            .or_insert(index);
    }
    parent_by_span_id
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use crate::analysis::classification::{ParentRelation, SiblingRelation, classify_trace_spans};
    use crate::graph::trace_graph::TraceCollection;
    use crate::input::otlp_json::ParsedTraceData;
    use crate::model::span::CanonicalSpan;

    #[test]
    fn classifies_serial_concurrent_nested_and_suspicious() {
        let collection = collection_with(vec![
            span("root", None, "checkout-service", 100, 200),
            span("a", Some("root"), "cart-service", 110, 140),
            span("b", Some("root"), "payment-service", 150, 190),
            span("c", Some("root"), "inventory-service", 150, 180),
            span("d", Some("root"), "notify-service", 190, 210),
            span("b1", Some("b"), "postgres", 155, 170),
            span("b2", Some("b"), "redis", 165, 185),
        ]);
        let trace = collection
            .traces
            .values()
            .next()
            .expect("trace should be present");

        let classification = classify_trace_spans(trace);

        assert_eq!(classification.counts.serial, 3);
        assert_eq!(classification.counts.concurrent, 4);
        assert_eq!(classification.counts.nested, 5);
        assert_eq!(classification.counts.suspicious, 1);

        assert_relation(
            &classification,
            "a",
            SiblingRelation::Serial,
            Some(ParentRelation::Nested),
        );
        assert_relation(
            &classification,
            "b",
            SiblingRelation::Concurrent,
            Some(ParentRelation::Nested),
        );
        assert_relation(
            &classification,
            "c",
            SiblingRelation::Concurrent,
            Some(ParentRelation::Nested),
        );
        assert_relation(
            &classification,
            "b1",
            SiblingRelation::Concurrent,
            Some(ParentRelation::Nested),
        );
        assert_relation(
            &classification,
            "b2",
            SiblingRelation::Concurrent,
            Some(ParentRelation::Nested),
        );
        assert_relation(
            &classification,
            "d",
            SiblingRelation::Serial,
            Some(ParentRelation::Suspicious),
        );
        assert_relation(&classification, "root", SiblingRelation::Serial, None);
    }

    #[test]
    fn multiple_roots_are_siblings() {
        let collection = collection_with(vec![
            span("root-a", None, "checkout-service", 100, 200),
            span("root-b", None, "cart-service", 150, 180),
        ]);
        let trace = collection
            .traces
            .values()
            .next()
            .expect("trace should be present");

        let classification = classify_trace_spans(trace);

        assert_eq!(classification.counts.concurrent, 2);
        assert_eq!(classification.counts.nested, 0);
        assert_relation(&classification, "root-a", SiblingRelation::Concurrent, None);
        assert_relation(&classification, "root-b", SiblingRelation::Concurrent, None);
    }

    #[test]
    fn orphans_with_same_missing_parent_are_siblings() {
        let collection = collection_with(vec![
            span("root", None, "checkout-service", 100, 200),
            span("orphan-a", Some("missing"), "cart-service", 110, 150),
            span("orphan-b", Some("missing"), "cart-service", 140, 180),
            span("orphan-c", Some("other-missing"), "cart-service", 110, 150),
        ]);
        let trace = collection
            .traces
            .values()
            .next()
            .expect("trace should be present");

        let classification = classify_trace_spans(trace);

        assert_relation(
            &classification,
            "orphan-a",
            SiblingRelation::Concurrent,
            None,
        );
        assert_relation(
            &classification,
            "orphan-b",
            SiblingRelation::Concurrent,
            None,
        );
        assert_relation(&classification, "orphan-c", SiblingRelation::Serial, None);
    }

    #[test]
    fn concurrent_group_marking_handles_nested_overlaps() {
        let collection = collection_with(vec![
            span("root", None, "checkout-service", 0, 200),
            span("long", Some("root"), "service-a", 0, 100),
            span("short-a", Some("root"), "service-b", 10, 20),
            span("short-b", Some("root"), "service-c", 30, 40),
        ]);
        let trace = collection
            .traces
            .values()
            .next()
            .expect("trace should be present");

        let classification = classify_trace_spans(trace);

        assert_relation(
            &classification,
            "long",
            SiblingRelation::Concurrent,
            Some(ParentRelation::Nested),
        );
        assert_relation(
            &classification,
            "short-a",
            SiblingRelation::Concurrent,
            Some(ParentRelation::Nested),
        );
        assert_relation(
            &classification,
            "short-b",
            SiblingRelation::Concurrent,
            Some(ParentRelation::Nested),
        );
    }

    fn assert_relation(
        classification: &crate::analysis::classification::TraceClassification,
        span_id: &str,
        sibling: SiblingRelation,
        parent: Option<ParentRelation>,
    ) {
        let span = classification
            .spans
            .iter()
            .find(|span| span.span_id == span_id)
            .expect("span classification should be present");
        assert_eq!(span.sibling_relation, sibling, "span {span_id}");
        assert_eq!(span.parent_relation, parent, "span {span_id}");
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
