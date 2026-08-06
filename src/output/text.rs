use std::collections::BTreeSet;
use std::fmt::Write;
use std::path::Path;

use crate::analysis::duration::{ServiceDuration, TraceDurationAnalysis};
use crate::analysis::summary::{FileSummary, TraceSummary};
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
        for (index, trace) in summary.slowest_traces.iter().take(10).enumerate() {
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

pub fn format_list_traces(path: &Path, traces: &[TraceSummary], limit: usize) -> String {
    let mut output = String::new();

    writeln!(output, "File: {}", path.display()).expect("write to string");
    writeln!(output, "Traces: {}", traces.len()).expect("write to string");
    writeln!(output, "Limit: {}", limit).expect("write to string");
    writeln!(output).expect("write to string");
    writeln!(
        output,
        "trace_id  duration  spans  services  errors  roots  orphans  diagnostics"
    )
    .expect("write to string");

    for trace in traces.iter().take(limit) {
        writeln!(
            output,
            "{}  {}  {}  {}  {}  {}  {}  {}",
            trace.trace_id,
            format_optional_duration(trace.duration_ns),
            trace.span_count,
            trace.service_count,
            trace.error_span_count,
            trace.root_count,
            trace.orphan_count,
            trace.diagnostics_count
        )
        .expect("write to string");
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

pub fn format_services(analysis: &TraceDurationAnalysis, diagnostics: &[Diagnostic]) -> String {
    let mut output = String::new();

    writeln!(output, "Trace 耗时概览").expect("write to string");
    writeln!(output, "trace_id: {}", analysis.trace_id).expect("write to string");
    writeln!(
        output,
        "wall-clock duration: {}",
        format_optional_duration(analysis.wall_clock_duration_ns)
    )
    .expect("write to string");
    writeln!(
        output,
        "说明：wall-clock duration 表示这条 trace 从最早 span 开始到最晚 span 结束的总时间。"
    )
    .expect("write to string");

    match &analysis.root_span {
        Some(root) => {
            writeln!(
                output,
                "root span duration: {}  span_id={}  service={}  name={}",
                format_duration(root.duration_ns),
                root.span_id,
                root.service_name,
                root.name
            )
            .expect("write to string");
            writeln!(
                output,
                "说明：root span duration 表示唯一 root span 的持续时间；它可能和 wall-clock duration 不一致。"
            )
            .expect("write to string");
        }
        None => {
            writeln!(output, "root span duration: unknown").expect("write to string");
            writeln!(
                output,
                "说明：只有存在唯一 root span 时才能确定 root span duration；当前 root 数量为 {}。",
                analysis.root_count
            )
            .expect("write to string");
        }
    }

    writeln!(output, "roots: {}", analysis.root_count).expect("write to string");
    writeln!(output, "orphans: {}", analysis.orphan_count).expect("write to string");
    writeln!(output, "diagnostics: {}", analysis.diagnostics_count).expect("write to string");

    writeln!(output).expect("write to string");
    writeln!(output, "服务耗时贡献").expect("write to string");
    writeln!(
        output,
        "说明：下表按 self_time 从高到低排序。self_time 越高，表示该服务自身在这条 trace 中消耗越多。"
    )
    .expect("write to string");
    write_service_table(&mut output, &analysis.services);

    writeln!(output).expect("write to string");
    writeln!(output, "字段说明：").expect("write to string");
    writeln!(
        output,
        "- self_time：服务自身消耗的时间，已扣除直接子 span 覆盖的时间区间；不同服务并发执行时，各服务 self_time 相加可能大于 wall-clock duration。"
    )
    .expect("write to string");
    writeln!(
        output,
        "- span_time：该服务所有 span 的原始耗时总和；嵌套或并发 span 可能让它大于真实 wall-clock 时间。"
    )
    .expect("write to string");
    writeln!(
        output,
        "- child_covered_time：该服务 span 中被直接子 span 覆盖的时间，重叠 child 只计算一次。"
    )
    .expect("write to string");
    writeln!(output, "- spans：该服务在当前 trace 中包含的 span 数量。").expect("write to string");
    writeln!(
        output,
        "- errors：该服务中 status=error、HTTP 5xx 或 gRPC 非 0 的 span 数量。"
    )
    .expect("write to string");

    if !diagnostics.is_empty() {
        writeln!(output).expect("write to string");
        writeln!(
            output,
            "数据诊断：下面的问题不会被静默忽略，耗时分析需要结合这些诊断一起看。"
        )
        .expect("write to string");
        write_diagnostics(&mut output, diagnostics);
    }

    output
}

fn write_service_table(output: &mut String, services: &[ServiceDuration]) {
    if services.is_empty() {
        writeln!(output, "(no services)").expect("write to string");
        return;
    }

    let service_width = services
        .iter()
        .map(|service| service.service_name.len())
        .max()
        .unwrap_or("service".len())
        .max("service".len());

    writeln!(
        output,
        "{:<service_width$}  {:>12}  {:>12}  {:>18}  {:>5}  {:>6}",
        "service", "self_time", "span_time", "child_covered_time", "spans", "errors"
    )
    .expect("write to string");

    for service in services {
        writeln!(
            output,
            "{:<service_width$}  {:>12}  {:>12}  {:>18}  {:>5}  {:>6}",
            service.service_name,
            format_duration(service.self_time_ns),
            format_duration(service.span_time_ns),
            format_duration(service.child_covered_time_ns),
            service.span_count,
            service.error_span_count
        )
        .expect("write to string");
    }
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
    line.push_str(&format!(
        " kind={} status={}",
        span.kind_label(),
        span.status_label()
    ));

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
        write!(output, " scope={}", diagnostic.scope).expect("write to string");

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
