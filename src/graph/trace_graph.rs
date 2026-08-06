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

#[derive(Clone, Debug)]
pub struct TraceGraph {
    pub trace_id: String,
    pub spans: Vec<CanonicalSpan>,
    pub children_by_parent: BTreeMap<String, Vec<usize>>,
    pub root_indices: Vec<usize>,
    pub orphan_indices: Vec<usize>,
    pub duplicate_span_ids: BTreeSet<String>,
    pub diagnostics: Vec<Diagnostic>,
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

        Self {
            trace_id,
            spans,
            children_by_parent,
            root_indices,
            orphan_indices,
            duplicate_span_ids,
            diagnostics,
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
    use std::path::Path;

    use crate::graph::trace_graph::TraceCollection;
    use crate::input::otlp_json::parse_otlp_file;

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
}
