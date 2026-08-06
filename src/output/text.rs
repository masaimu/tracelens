use std::collections::BTreeSet;
use std::fmt::Write;
use std::path::Path;

use crate::analysis::summary::FileSummary;
use crate::graph::trace_graph::{TraceCollection, TraceGraph};
use crate::model::diagnostic::{Diagnostic, Severity};
use crate::model::span::CanonicalSpan;

pub fn format_validate(path: &Path, collection: &TraceCollection, strict: bool) -> String {
    let mut output = String::new();
    let error_count = collection
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Error)
        .count();

    writeln!(output, "File: {}", path.display()).expect("write to string");
    writeln!(
        output,
        "Mode: {}",
        if strict { "strict" } else { "default" }
    )
    .expect("write to string");
    writeln!(
        output,
        "Status: {}",
        if strict && error_count > 0 {
            "failed"
        } else {
            "ok"
        }
    )
    .expect("write to string");
    writeln!(output, "Traces: {}", collection.traces.len()).expect("write to string");
    writeln!(output, "Spans: {}", collection.span_count()).expect("write to string");
    writeln!(output, "Diagnostics: {}", collection.diagnostics.len()).expect("write to string");

    if !collection.diagnostics.is_empty() {
        writeln!(output).expect("write to string");
        write_diagnostics(&mut output, &collection.diagnostics);
    }

    output
}

pub fn format_summary(path: &Path, summary: &FileSummary, collection: &TraceCollection) -> String {
    let mut output = String::new();

    writeln!(output, "File: {}", path.display()).expect("write to string");
    writeln!(output, "Traces: {}", summary.trace_count).expect("write to string");
    writeln!(output, "Spans: {}", summary.span_count).expect("write to string");
    writeln!(output, "Services: {}", summary.service_count).expect("write to string");
    writeln!(output, "Error spans: {}", summary.error_span_count).expect("write to string");
    writeln!(
        output,
        "Time range: {}",
        format_range(summary.start_ns, summary.end_ns)
    )
    .expect("write to string");
    writeln!(output, "Diagnostics: {}", collection.diagnostics.len()).expect("write to string");

    if !summary.slowest_traces.is_empty() {
        writeln!(output).expect("write to string");
        writeln!(output, "Slowest traces:").expect("write to string");
        for (index, trace) in summary.slowest_traces.iter().enumerate() {
            writeln!(
                output,
                "{}. {}  {}  {} spans  {} services  {} errors",
                index + 1,
                trace.trace_id,
                format_optional_duration(trace.duration_ns),
                trace.span_count,
                trace.service_count,
                trace.error_span_count
            )
            .expect("write to string");
        }
    }

    output
}

pub fn format_tree(trace: &TraceGraph) -> String {
    let mut output = String::new();

    writeln!(output, "Trace: {}", trace.trace_id).expect("write to string");
    writeln!(
        output,
        "Duration: {}",
        format_optional_duration(trace.duration_ns())
    )
    .expect("write to string");
    writeln!(output, "Spans: {}", trace.spans.len()).expect("write to string");
    writeln!(output, "Roots: {}", trace.root_indices.len()).expect("write to string");
    writeln!(output, "Orphans: {}", trace.orphan_indices.len()).expect("write to string");
    writeln!(
        output,
        "Duplicate span IDs: {}",
        trace.duplicate_span_ids.len()
    )
    .expect("write to string");
    writeln!(output, "Diagnostics: {}", trace.diagnostics.len()).expect("write to string");
    writeln!(output).expect("write to string");

    let mut visited = BTreeSet::new();
    for index in &trace.root_indices {
        write_span_tree(&mut output, trace, *index, 0, &mut visited);
    }

    if !trace.orphan_indices.is_empty() {
        writeln!(output).expect("write to string");
        writeln!(output, "Orphan spans:").expect("write to string");
        for index in &trace.orphan_indices {
            write_span_tree(&mut output, trace, *index, 1, &mut visited);
        }
    }

    let mut wrote_unattached_header = false;
    for index in 0..trace.spans.len() {
        if !visited.contains(&index) {
            if !wrote_unattached_header {
                writeln!(output).expect("write to string");
                writeln!(output, "Unattached spans:").expect("write to string");
                wrote_unattached_header = true;
            }
            write_span_tree(&mut output, trace, index, 0, &mut visited);
        }
    }

    if !trace.diagnostics.is_empty() {
        writeln!(output).expect("write to string");
        write_diagnostics(&mut output, &trace.diagnostics);
    }

    output
}

fn write_span_tree(
    output: &mut String,
    trace: &TraceGraph,
    index: usize,
    depth: usize,
    visited: &mut BTreeSet<usize>,
) {
    if !visited.insert(index) {
        return;
    }

    let span = &trace.spans[index];
    writeln!(output, "{}{}", "  ".repeat(depth), format_span_line(span)).expect("write to string");

    if let Some(children) = trace.children_by_parent.get(&span.span_id) {
        for child_index in children {
            write_span_tree(output, trace, *child_index, depth + 1, visited);
        }
    }
}

fn format_span_line(span: &CanonicalSpan) -> String {
    let mut line = format!(
        "[{}] {} {} span_id={}",
        span.service_name,
        span.name,
        format_duration(span.duration_ns()),
        span.span_id
    );

    if span.is_error() {
        line.push_str(" ERROR");
    }

    line
}

fn write_diagnostics(output: &mut String, diagnostics: &[Diagnostic]) {
    writeln!(output, "Diagnostics:").expect("write to string");
    for diagnostic in diagnostics {
        write!(
            output,
            "- [{}] {}: {}",
            diagnostic.severity, diagnostic.code, diagnostic.message
        )
        .expect("write to string");

        if let Some(trace_id) = &diagnostic.trace_id {
            write!(output, " trace_id={trace_id}").expect("write to string");
        }
        if let Some(span_id) = &diagnostic.span_id {
            write!(output, " span_id={span_id}").expect("write to string");
        }
        if let Some(location) = &diagnostic.location {
            write!(output, " location={location}").expect("write to string");
        }
        writeln!(output).expect("write to string");
    }
}

fn format_range(start_ns: Option<u64>, end_ns: Option<u64>) -> String {
    match (start_ns, end_ns) {
        (Some(start_ns), Some(end_ns)) => {
            format!(
                "{start_ns}..{end_ns} ({})",
                format_duration(end_ns - start_ns)
            )
        }
        _ => "unknown".to_string(),
    }
}

fn format_optional_duration(duration_ns: Option<u64>) -> String {
    duration_ns
        .map(format_duration)
        .unwrap_or_else(|| "unknown".to_string())
}

pub fn format_duration(duration_ns: u64) -> String {
    if duration_ns >= 1_000_000_000 {
        return format!("{:.3}s", duration_ns as f64 / 1_000_000_000.0);
    }
    if duration_ns >= 1_000_000 {
        return format!("{:.3}ms", duration_ns as f64 / 1_000_000.0);
    }
    if duration_ns >= 1_000 {
        return format!("{:.3}us", duration_ns as f64 / 1_000.0);
    }

    format!("{duration_ns}ns")
}

#[cfg(test)]
mod tests {
    use super::format_duration;

    #[test]
    fn formats_duration_units() {
        assert_eq!(format_duration(42), "42ns");
        assert_eq!(format_duration(42_000), "42.000us");
        assert_eq!(format_duration(42_000_000), "42.000ms");
        assert_eq!(format_duration(1_500_000_000), "1.500s");
    }
}
