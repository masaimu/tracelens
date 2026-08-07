use std::collections::BTreeSet;
use std::fmt::Write;
use std::path::Path;

use crate::analysis::annotations::{
    AnnotationNote, ClientServerPair, SpanAnnotation, TraceAnnotations,
};
use crate::analysis::classification::{ParentRelation, SiblingRelation, TraceClassification};
use crate::analysis::critical_path::{
    CriticalPathAnalysis, CriticalPathSegment, CriticalPathSpanTotal, CriticalPathStatus,
};
use crate::analysis::detect::{
    Confidence, DetectAnalysis, ErrorSpanCandidate, SampleQuality, ServiceSlowCandidate,
    SlowTraceCandidate,
};
use crate::analysis::duration::{ServiceDuration, TraceDurationAnalysis};
use crate::analysis::summary::{FileSummary, TraceSummary};
use crate::graph::trace_graph::{TraceCollection, TraceGraph};
use crate::model::diagnostic::{Diagnostic, Severity};
use crate::model::span::CanonicalSpan;
use crate::output::style::TextStyle;

pub fn format_validate(
    path: &Path,
    collection: &TraceCollection,
    strict: bool,
    style: TextStyle,
) -> String {
    let mut output = String::new();
    let error_count = collection
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.severity == Severity::Error)
        .count();
    let status_failed = strict && error_count > 0;

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
        if status_failed {
            style.error("failed")
        } else {
            style.ok("ok")
        }
    )
    .expect("write to string");
    writeln!(output, "Traces: {}", collection.traces.len()).expect("write to string");
    writeln!(output, "Spans: {}", collection.span_count()).expect("write to string");
    writeln!(output, "Diagnostics: {}", collection.diagnostics.len()).expect("write to string");

    if !collection.diagnostics.is_empty() {
        writeln!(output).expect("write to string");
        write_diagnostics(&mut output, &collection.diagnostics, style);
    }

    output
}

pub fn format_summary(
    path: &Path,
    summary: &FileSummary,
    collection: &TraceCollection,
    style: TextStyle,
) -> String {
    let mut output = String::new();

    writeln!(output, "File: {}", path.display()).expect("write to string");
    writeln!(output, "Traces: {}", summary.trace_count).expect("write to string");
    writeln!(output, "Spans: {}", summary.span_count).expect("write to string");
    writeln!(output, "Services: {}", summary.service_count).expect("write to string");
    writeln!(
        output,
        "Error spans: {}",
        style_count_by_risk(style, summary.error_span_count)
    )
    .expect("write to string");
    writeln!(
        output,
        "Time range: {}",
        format_range_styled(style, summary.start_ns, summary.end_ns)
    )
    .expect("write to string");
    writeln!(
        output,
        "Diagnostics: {}",
        style_count_by_risk(style, collection.diagnostics.len())
    )
    .expect("write to string");

    if !summary.slowest_traces.is_empty() {
        writeln!(output).expect("write to string");
        writeln!(output, "{}", style.section("Slowest traces:")).expect("write to string");
        for (index, trace) in summary.slowest_traces.iter().take(10).enumerate() {
            writeln!(
                output,
                "{}. {}  {}  {} spans  {} services  {} errors",
                index + 1,
                style.identifier(&trace.trace_id),
                format_optional_duration_styled(style, trace.duration_ns),
                trace.span_count,
                trace.service_count,
                style_count_by_risk(style, trace.error_span_count)
            )
            .expect("write to string");
        }
    }

    output
}

pub fn format_list_traces(
    path: &Path,
    traces: &[TraceSummary],
    limit: usize,
    style: TextStyle,
) -> String {
    let mut output = String::new();

    writeln!(output, "File: {}", path.display()).expect("write to string");
    writeln!(output, "Traces: {}", traces.len()).expect("write to string");
    writeln!(output, "Limit: {}", limit).expect("write to string");
    writeln!(output).expect("write to string");
    writeln!(
        output,
        "{}",
        style.table_header(
            "trace_id  duration  spans  services  errors  roots  orphans  diagnostics"
        )
    )
    .expect("write to string");

    for trace in traces.iter().take(limit) {
        writeln!(
            output,
            "{}  {}  {}  {}  {}  {}  {}  {}",
            style.identifier(&trace.trace_id),
            format_optional_duration_styled(style, trace.duration_ns),
            trace.span_count,
            trace.service_count,
            style_count_by_risk(style, trace.error_span_count),
            trace.root_count,
            style_count_by_risk(style, trace.orphan_count),
            style_count_by_risk(style, trace.diagnostics_count)
        )
        .expect("write to string");
    }

    output
}

pub fn format_tree(trace: &TraceGraph, annotations: &TraceAnnotations, style: TextStyle) -> String {
    let mut output = String::new();

    writeln!(output, "Trace: {}", style.identifier(&trace.trace_id)).expect("write to string");
    writeln!(
        output,
        "Duration: {}",
        format_optional_duration_styled(style, trace.duration_ns())
    )
    .expect("write to string");
    writeln!(output, "Spans: {}", trace.spans.len()).expect("write to string");
    writeln!(output, "Roots: {}", trace.root_indices.len()).expect("write to string");
    writeln!(output, "Orphans: {}", trace.orphan_indices.len()).expect("write to string");
    writeln!(
        output,
        "Duplicate span IDs: {}",
        style_count_by_risk(style, trace.duplicate_span_ids.len())
    )
    .expect("write to string");
    writeln!(
        output,
        "Diagnostics: {}",
        style_count_by_risk(style, trace.diagnostics.len())
    )
    .expect("write to string");
    writeln!(output).expect("write to string");

    let mut visited = BTreeSet::new();
    for index in &trace.root_indices {
        write_span_tree(
            &mut output,
            trace,
            annotations,
            *index,
            0,
            &mut visited,
            style,
        );
    }

    if !trace.orphan_indices.is_empty() {
        writeln!(output).expect("write to string");
        writeln!(output, "{}", style.warning("Orphan spans:")).expect("write to string");
        for index in &trace.orphan_indices {
            write_span_tree(
                &mut output,
                trace,
                annotations,
                *index,
                1,
                &mut visited,
                style,
            );
        }
    }

    let mut wrote_unattached_header = false;
    for index in 0..trace.spans.len() {
        if !visited.contains(&index) {
            if !wrote_unattached_header {
                writeln!(output).expect("write to string");
                writeln!(output, "{}", style.warning("Unattached spans:"))
                    .expect("write to string");
                wrote_unattached_header = true;
            }
            write_span_tree(
                &mut output,
                trace,
                annotations,
                index,
                0,
                &mut visited,
                style,
            );
        }
    }

    if has_any_annotations(annotations) {
        writeln!(output).expect("write to string");
        write_annotations_summary(&mut output, annotations, style);
    }

    if !trace.diagnostics.is_empty() {
        writeln!(output).expect("write to string");
        write_diagnostics(&mut output, &trace.diagnostics, style);
    }

    output
}

pub fn format_services(
    analysis: &TraceDurationAnalysis,
    diagnostics: &[Diagnostic],
    style: TextStyle,
) -> String {
    let mut output = String::new();

    writeln!(output, "{}", style.section("Trace 耗时概览")).expect("write to string");
    writeln!(output, "trace_id: {}", style.identifier(&analysis.trace_id))
        .expect("write to string");
    writeln!(
        output,
        "wall-clock duration: {}",
        format_optional_duration_styled(style, analysis.wall_clock_duration_ns)
    )
    .expect("write to string");
    writeln!(
        output,
        "{}",
        style.muted(
            "说明：wall-clock duration 表示这条 trace 从最早 span 开始到最晚 span 结束的总时间。"
        )
    )
    .expect("write to string");

    match &analysis.root_span {
        Some(root) => {
            writeln!(
                output,
                "root span duration: {}  span_id={}  service={}  name={}",
                format_duration_styled(style, root.duration_ns),
                style.identifier(&root.span_id),
                style.service(&root.service_name),
                root.name
            )
            .expect("write to string");
            writeln!(
                output,
                "{}",
                style.muted("说明：root span duration 表示唯一 root span 的持续时间；它可能和 wall-clock duration 不一致。")
            )
            .expect("write to string");
        }
        None => {
            writeln!(output, "root span duration: {}", style.muted("unknown"))
                .expect("write to string");
            writeln!(
                output,
                "{}",
                style.muted(format!(
                    "说明：只有存在唯一 root span 时才能确定 root span duration；当前 root 数量为 {}。",
                    analysis.root_count
                ))
            )
            .expect("write to string");
        }
    }

    writeln!(output, "roots: {}", analysis.root_count).expect("write to string");
    writeln!(output, "orphans: {}", analysis.orphan_count).expect("write to string");
    writeln!(
        output,
        "diagnostics: {}",
        style_count_by_risk(style, analysis.diagnostics_count)
    )
    .expect("write to string");

    writeln!(output).expect("write to string");
    writeln!(output, "{}", style.section("服务耗时贡献")).expect("write to string");
    writeln!(
        output,
        "{}",
        style.muted("说明：下表按 self_time 从高到低排序。self_time 越高，表示该服务自身在这条 trace 中消耗越多。")
    )
    .expect("write to string");
    write_service_table(&mut output, &analysis.services, style);

    writeln!(output).expect("write to string");
    writeln!(output, "{}", style.section("字段说明：")).expect("write to string");
    writeln!(
        output,
        "{}",
        style.muted("- self_time：服务自身消耗的时间，已扣除直接子 span 覆盖的时间区间；不同服务并发执行时，各服务 self_time 相加可能大于 wall-clock duration。")
    )
    .expect("write to string");
    writeln!(
        output,
        "{}",
        style.muted("- span_time：该服务所有 span 的原始耗时总和；嵌套或并发 span 可能让它大于真实 wall-clock 时间。")
    )
    .expect("write to string");
    writeln!(
        output,
        "{}",
        style.muted(
            "- child_covered_time：该服务 span 中被直接子 span 覆盖的时间，重叠 child 只计算一次。"
        )
    )
    .expect("write to string");
    writeln!(
        output,
        "{}",
        style.muted("- spans：该服务在当前 trace 中包含的 span 数量。")
    )
    .expect("write to string");
    writeln!(
        output,
        "{}",
        style.muted("- errors：该服务中 status=error、HTTP 5xx 或 gRPC 非 0 的 span 数量。")
    )
    .expect("write to string");

    if !diagnostics.is_empty() {
        writeln!(output).expect("write to string");
        writeln!(
            output,
            "{}",
            style.warning("数据诊断：下面的问题不会被静默忽略，耗时分析需要结合这些诊断一起看。")
        )
        .expect("write to string");
        write_diagnostics(&mut output, diagnostics, style);
    }

    output
}

pub fn format_detect(
    path: &Path,
    analysis: &DetectAnalysis,
    collection: &TraceCollection,
    style: TextStyle,
) -> String {
    let mut output = String::new();

    writeln!(output, "File: {}", path.display()).expect("write to string");
    writeln!(output, "{}", style.section("Detect 检测概览")).expect("write to string");
    writeln!(
        output,
        "traces: {}  spans: {}  diagnostics: {}  limit: {}",
        analysis.summary.trace_count,
        analysis.summary.span_count,
        style_count_by_risk(style, analysis.summary.diagnostics_count),
        analysis.limit
    )
    .expect("write to string");
    writeln!(
        output,
        "样本数: {}  样本质量: {}  p95 耗时参考: {}",
        analysis.summary.sample_count,
        style_sample_quality(style, analysis.summary.sample_quality),
        format_optional_duration_styled(style, analysis.summary.p95_duration_ns)
    )
    .expect("write to string");
    writeln!(
        output,
        "慢请求候选: {}  错误 trace 候选: {}  错误 span: {}",
        style_count_by_risk(style, analysis.summary.slow_trace_candidate_count),
        style_count_by_risk(style, analysis.summary.error_trace_candidate_count),
        style_count_by_risk(style, analysis.summary.error_span_count)
    )
    .expect("write to string");
    writeln!(
        output,
        "{}",
        style.muted(
            "说明：detect 输出的是候选问题和证据，不是最终根因结论；样本数不足时会降低 confidence。"
        )
    )
    .expect("write to string");

    if !analysis.notes.is_empty() {
        writeln!(output).expect("write to string");
        writeln!(output, "{}", style.section("注意事项")).expect("write to string");
        for note in &analysis.notes {
            writeln!(output, "- {}", style.warning(localize_detect_note(note)))
                .expect("write to string");
        }
    }

    writeln!(output).expect("write to string");
    writeln!(output, "{}", style.section("慢请求候选")).expect("write to string");
    writeln!(
        output,
        "{}",
        style.muted("说明：这里按 trace 的端到端耗时排序，并列出每条慢 trace 内最值得优先查看的 service candidates。")
    )
    .expect("write to string");
    write_slow_trace_candidates(&mut output, &analysis.slow_traces, style);

    writeln!(output).expect("write to string");
    writeln!(output, "{}", style.section("错误传播候选")).expect("write to string");
    writeln!(
        output,
        "{}",
        style.muted("说明：这里会识别 OTLP status=ERROR、HTTP 5xx、gRPC/RPC 非 OK，以及 exception event；earliest 表示时间上最早看到的错误，top 表示拓扑上更高层的错误 span。")
    )
    .expect("write to string");
    write_error_trace_candidates(&mut output, &analysis.error_traces, style);

    writeln!(output).expect("write to string");
    writeln!(output, "{}", style.section("字段说明：")).expect("write to string");
    writeln!(
        output,
        "{}",
        style.muted("- confidence：当前检测结果的置信度。low 通常表示样本太少，只能作为线索；medium/high 可以作为优先排查依据。")
    )
    .expect("write to string");
    writeln!(
        output,
        "{}",
        style.muted("- service candidates：慢 trace 中按 span_time 排序的服务候选，帮助定位下一步该看哪个服务。")
    )
    .expect("write to string");
    writeln!(
        output,
        "{}",
        style.muted("- signals：错误判定信号，例如 status_code_error、http_5xx、grpc_non_zero、rpc_non_ok、exception_event。")
    )
    .expect("write to string");

    if !collection.diagnostics.is_empty() {
        writeln!(output).expect("write to string");
        writeln!(
            output,
            "{}",
            style.warning("数据诊断：下面的问题会影响检测结果的可信度，需要结合候选问题一起看。")
        )
        .expect("write to string");
        write_diagnostics(&mut output, &collection.diagnostics, style);
    }

    output
}

pub fn format_critical_path(
    duration: &TraceDurationAnalysis,
    critical_path: &CriticalPathAnalysis,
    classification: &TraceClassification,
    annotations: &TraceAnnotations,
    trace: &TraceGraph,
    style: TextStyle,
) -> String {
    let mut output = String::new();

    writeln!(output, "{}", style.section("Trace 耗时概览")).expect("write to string");
    writeln!(output, "trace_id: {}", style.identifier(&duration.trace_id))
        .expect("write to string");
    writeln!(
        output,
        "wall-clock duration: {}",
        format_optional_duration_styled(style, duration.wall_clock_duration_ns)
    )
    .expect("write to string");
    match &critical_path.root_span {
        Some(root) => {
            writeln!(
                output,
                "root span duration: {}  span_id={}  service={}  name={}",
                format_duration_styled(style, root.duration_ns),
                style.identifier(&root.span_id),
                style.service(&root.service_name),
                root.name
            )
            .expect("write to string");
        }
        None => {
            writeln!(output, "root span duration: {}", style.muted("unknown"))
                .expect("write to string");
        }
    }

    writeln!(output).expect("write to string");
    writeln!(output, "{}", style.section("关键路径")).expect("write to string");
    writeln!(
        output,
        "{}",
        style.muted("说明：关键路径把 root span 的时间区间完整切分到具体 span；并发 child 同时执行时，该窗口归因给结束最晚的 child。")
    )
    .expect("write to string");
    for note in &critical_path.notes {
        writeln!(
            output,
            "注意：{}",
            style.warning(localize_critical_path_note(note))
        )
        .expect("write to string");
    }

    match &critical_path.status {
        CriticalPathStatus::Available => {
            writeln!(
                output,
                "critical path duration: {}",
                style.critical(format_duration(critical_path.total_duration_ns))
            )
            .expect("write to string");
            write_critical_path_segments(&mut output, &critical_path.segments, style);

            writeln!(output).expect("write to string");
            writeln!(output, "{}", style.section("关键路径 span 汇总")).expect("write to string");
            writeln!(
                output,
                "{}",
                style.muted("说明：下表按 span 在关键路径上的累计时间从高到低排序，表示每个 span 对端到端阻塞的贡献。")
            )
            .expect("write to string");
            write_critical_path_totals(&mut output, &critical_path.span_totals, style);
        }
        CriticalPathStatus::Unavailable { reason } => {
            writeln!(output, "critical path: {}", style.warning("unavailable"))
                .expect("write to string");
            writeln!(output, "原因：{}。", localize_critical_path_reason(reason))
                .expect("write to string");
        }
    }

    writeln!(output).expect("write to string");
    writeln!(output, "{}", style.section("Span 执行分类")).expect("write to string");
    writeln!(
        output,
        "serial: {}  concurrent: {}  nested: {}  suspicious: {}",
        classification.counts.serial,
        style.concurrent(classification.counts.concurrent),
        classification.counts.nested,
        style.warning(classification.counts.suspicious)
    )
    .expect("write to string");
    writeln!(
        output,
        "{}",
        style.muted("说明：serial/concurrent 描述 span 与同层 sibling 的时间关系；nested/suspicious 描述 span 与 parent 的时间关系，suspicious 表示 span 超出了 parent 的时间范围。")
    )
    .expect("write to string");
    write_classification_details(&mut output, classification, style);

    writeln!(output).expect("write to string");
    write_annotations_summary(&mut output, annotations, style);

    if !trace.diagnostics.is_empty() {
        writeln!(output).expect("write to string");
        write_diagnostics(&mut output, &trace.diagnostics, style);
    }

    output
}

fn write_slow_trace_candidates(
    output: &mut String,
    candidates: &[SlowTraceCandidate],
    style: TextStyle,
) {
    if candidates.is_empty() {
        writeln!(output, "(no slow trace candidates)").expect("write to string");
        return;
    }

    writeln!(
        output,
        "{}",
        style.table_header(
            "rank  trace_id                          duration  confidence  spans  services  errors  diagnostics"
        )
    )
    .expect("write to string");

    for candidate in candidates {
        writeln!(
            output,
            "{:>4}  {}  {:>10}  {:>10}  {:>5}  {:>8}  {:>6}  {:>11}",
            candidate.rank,
            style.identifier(&candidate.trace_id),
            style.duration(format_duration(candidate.duration_ns)),
            style_confidence(style, candidate.confidence),
            candidate.span_count,
            candidate.service_count,
            style_count_by_risk(style, candidate.error_span_count),
            style_count_by_risk(style, candidate.diagnostics_count)
        )
        .expect("write to string");
        writeln!(
            output,
            "      解释：{}",
            slow_candidate_explanation(candidate, style)
        )
        .expect("write to string");
        write_service_candidates(output, &candidate.service_candidates, style);
    }
}

fn write_service_candidates(
    output: &mut String,
    candidates: &[ServiceSlowCandidate],
    style: TextStyle,
) {
    if candidates.is_empty() {
        return;
    }

    writeln!(output, "      service candidates:").expect("write to string");
    for service in candidates {
        writeln!(
            output,
            "      - [{}] span_time={} max_span={} spans={} errors={}",
            style.service(&service.service_name),
            style.duration(format_duration(service.span_time_ns)),
            style.duration(format_duration(service.max_span_duration_ns)),
            service.span_count,
            style_count_by_risk(style, service.error_span_count)
        )
        .expect("write to string");
    }
}

fn write_error_trace_candidates(
    output: &mut String,
    candidates: &[crate::analysis::detect::ErrorTraceCandidate],
    style: TextStyle,
) {
    if candidates.is_empty() {
        writeln!(output, "(no error trace candidates)").expect("write to string");
        return;
    }

    for candidate in candidates {
        writeln!(
            output,
            "- trace_id={}  error_spans={}  confidence={}",
            style.identifier(&candidate.trace_id),
            style_count_by_risk(style, candidate.error_span_count),
            style_confidence(style, candidate.confidence)
        )
        .expect("write to string");
        writeln!(
            output,
            "  earliest: {}",
            format_error_span_candidate(&candidate.earliest_error_span, style)
        )
        .expect("write to string");
        writeln!(
            output,
            "  top:      {}",
            format_error_span_candidate(&candidate.top_error_span, style)
        )
        .expect("write to string");
        writeln!(
            output,
            "  signals: {}",
            aggregate_error_signals(candidate, style)
        )
        .expect("write to string");
        writeln!(output, "  error spans:").expect("write to string");
        for span in &candidate.error_spans {
            writeln!(output, "  - {}", format_error_span_candidate(span, style))
                .expect("write to string");
        }
    }
}

fn format_error_span_candidate(candidate: &ErrorSpanCandidate, style: TextStyle) -> String {
    format!(
        "[{}] {} span_id={} depth={} duration={} signals={}",
        style.service(&candidate.service_name),
        candidate.name,
        style.identifier(&candidate.span_id),
        candidate.depth,
        style.duration(format_duration(candidate.duration_ns)),
        candidate
            .signals
            .iter()
            .map(|signal| localize_error_signal(signal))
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn slow_candidate_explanation(candidate: &SlowTraceCandidate, style: TextStyle) -> String {
    if candidate.sample_count < 5 {
        return style.warning(format!(
            "当前样本数只有 {}，这条 trace 耗时最高，但只能低置信度作为排查线索。",
            candidate.sample_count
        ));
    }

    match candidate.p95_duration_ns {
        Some(p95) if candidate.duration_ns >= p95 => format!(
            "耗时 {} 已达到或超过当前样本 p95 参考值 {}。",
            style.duration(format_duration(candidate.duration_ns)),
            style.duration(format_duration(p95))
        ),
        Some(p95) => format!(
            "按耗时排序进入候选；当前样本 p95 参考值为 {}。",
            style.duration(format_duration(p95))
        ),
        None => "当前没有可用 p95 参考值，只按耗时排序。".to_string(),
    }
}

fn style_confidence(style: TextStyle, confidence: Confidence) -> String {
    match confidence {
        Confidence::Low => style.warning(confidence.label()),
        Confidence::Medium => style.concurrent(confidence.label()),
        Confidence::High => style.error(confidence.label()),
    }
}

fn style_sample_quality(style: TextStyle, quality: SampleQuality) -> String {
    match quality {
        SampleQuality::Insufficient => style.warning(quality.label()),
        SampleQuality::Limited => style.concurrent(quality.label()),
        SampleQuality::Broad => style.ok(quality.label()),
    }
}

fn localize_detect_note(note: &str) -> String {
    match note {
        "trace sample count is below 5; slow-trace findings are low-confidence candidates" => {
            "trace 样本数少于 5；慢请求检测只能作为低置信度候选。".to_string()
        }
        "trace sample count is below 20; percentile references are useful but still limited" => {
            "trace 样本数少于 20；p95 参考值已经可用，但仍需要谨慎解读。".to_string()
        }
        "N+1 detection is intentionally deferred to the next M5 step to avoid early false positives" => {
            "N+1 检测会在 M5 的下一步实现；第一版先避免过早引入误报。".to_string()
        }
        _ => note.to_string(),
    }
}

fn localize_error_signal(signal: &str) -> String {
    match signal {
        "status_code_error" => "status_code_error(OTLP ERROR)".to_string(),
        "http_5xx" => "http_5xx(HTTP 5xx)".to_string(),
        "grpc_non_zero" => "grpc_non_zero(gRPC 非 0)".to_string(),
        "rpc_non_ok" => "rpc_non_ok(RPC 非 OK)".to_string(),
        "exception_event" => "exception_event(exception 事件)".to_string(),
        _ => signal.to_string(),
    }
}

fn aggregate_error_signals(
    candidate: &crate::analysis::detect::ErrorTraceCandidate,
    style: TextStyle,
) -> String {
    let signals = candidate
        .error_spans
        .iter()
        .flat_map(|span| span.signals.iter())
        .collect::<BTreeSet<_>>();

    signals
        .into_iter()
        .map(|signal| style.warning(localize_error_signal(signal)))
        .collect::<Vec<_>>()
        .join(",")
}

fn write_critical_path_segments(
    output: &mut String,
    segments: &[CriticalPathSegment],
    style: TextStyle,
) {
    if segments.is_empty() {
        writeln!(output, "(no segments)").expect("write to string");
        return;
    }

    let service_width = segments
        .iter()
        .map(|segment| segment.service_name.len())
        .max()
        .unwrap_or("service".len())
        .max("service".len());
    let name_width = segments
        .iter()
        .map(|segment| segment.name.len())
        .max()
        .unwrap_or("name".len())
        .max("name".len());

    writeln!(
        output,
        "{}",
        style.table_header(format!(
            "{:>12}  {:>12}  {:<service_width$}  {:<name_width$}  span_id",
            "offset", "duration", "service", "name"
        ))
    )
    .expect("write to string");

    for segment in segments {
        let offset = format!("{:>12}", format_duration(segment.offset_ns));
        let duration = format!("{:>12}", format_duration(segment.duration_ns));
        let service = format!("{:<service_width$}", segment.service_name);
        let name = format!("{:<name_width$}", segment.name);
        writeln!(
            output,
            "{}  {}  {}  {}  {}",
            style.duration(offset),
            style.duration(duration),
            style.service(service),
            style.critical(name),
            style.identifier(&segment.span_id)
        )
        .expect("write to string");
    }
}

fn localize_critical_path_note(note: &str) -> String {
    if let Some(count) = note.strip_prefix("trace has ").and_then(|remaining| {
        remaining.strip_suffix(" root spans; the critical path only covers the longest root span")
    }) {
        return format!("trace 有 {count} 个 root span；关键路径只覆盖 duration 最长的 root span");
    }

    if note
        == "wall-clock duration exceeds the root span interval; the critical path only covers the root span interval"
    {
        return "wall-clock duration 大于被选中 root span 的时间区间；关键路径只覆盖该 root span 区间".to_string();
    }

    note.to_string()
}

fn localize_critical_path_reason(reason: &str) -> String {
    if reason == "trace has no root span" {
        return "trace 没有 root span，无法计算关键路径".to_string();
    }

    reason.to_string()
}

fn write_critical_path_totals(
    output: &mut String,
    totals: &[CriticalPathSpanTotal],
    style: TextStyle,
) {
    if totals.is_empty() {
        writeln!(output, "(no spans)").expect("write to string");
        return;
    }

    let service_width = totals
        .iter()
        .map(|total| total.service_name.len())
        .max()
        .unwrap_or("service".len())
        .max("service".len());
    let name_width = totals
        .iter()
        .map(|total| total.name.len())
        .max()
        .unwrap_or("name".len())
        .max("name".len());

    writeln!(
        output,
        "{}",
        style.table_header(format!(
            "{:>12}  {:<service_width$}  {:<name_width$}  span_id",
            "total", "service", "name"
        ))
    )
    .expect("write to string");

    for total in totals {
        let duration = format!("{:>12}", format_duration(total.total_ns));
        let service = format!("{:<service_width$}", total.service_name);
        let name = format!("{:<name_width$}", total.name);
        writeln!(
            output,
            "{}  {}  {}  {}",
            style.duration(duration),
            style.service(service),
            style.critical(name),
            style.identifier(&total.span_id)
        )
        .expect("write to string");
    }
}

fn write_classification_details(
    output: &mut String,
    classification: &TraceClassification,
    style: TextStyle,
) {
    let concurrent: Vec<_> = classification
        .spans
        .iter()
        .filter(|span| span.sibling_relation == SiblingRelation::Concurrent)
        .collect();
    if !concurrent.is_empty() {
        writeln!(output).expect("write to string");
        writeln!(output, "{}", style.concurrent("并发 span：")).expect("write to string");
        for span in concurrent {
            writeln!(
                output,
                "- [{}] {} span_id={}",
                style.service(&span.service_name),
                span.name,
                style.identifier(&span.span_id)
            )
            .expect("write to string");
        }
    }

    let suspicious: Vec<_> = classification
        .spans
        .iter()
        .filter(|span| span.parent_relation == Some(ParentRelation::Suspicious))
        .collect();
    if !suspicious.is_empty() {
        writeln!(output).expect("write to string");
        writeln!(
            output,
            "{}",
            style.warning("可疑 span（超出 parent 时间范围）：")
        )
        .expect("write to string");
        for span in suspicious {
            writeln!(
                output,
                "- [{}] {} span_id={}",
                style.service(&span.service_name),
                style.warning(&span.name),
                style.identifier(&span.span_id)
            )
            .expect("write to string");
        }
    }
}

fn write_service_table(output: &mut String, services: &[ServiceDuration], style: TextStyle) {
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
        "{}",
        style.table_header(format!(
            "{:<service_width$}  {:>12}  {:>12}  {:>18}  {:>5}  {:>6}",
            "service", "self_time", "span_time", "child_covered_time", "spans", "errors"
        ))
    )
    .expect("write to string");

    for service in services {
        let service_name = format!("{:<service_width$}", service.service_name);
        let self_time = format!("{:>12}", format_duration(service.self_time_ns));
        let span_time = format!("{:>12}", format_duration(service.span_time_ns));
        let child_covered_time = format!("{:>18}", format_duration(service.child_covered_time_ns));
        let errors = format!("{:>6}", service.error_span_count);
        writeln!(
            output,
            "{}  {}  {}  {}  {:>5}  {}",
            style.service(service_name),
            style.duration(self_time),
            style.duration(span_time),
            style.duration(child_covered_time),
            service.span_count,
            if service.error_span_count > 0 {
                style.error(errors)
            } else {
                errors
            }
        )
        .expect("write to string");
    }
}

fn write_span_tree(
    output: &mut String,
    trace: &TraceGraph,
    annotations: &TraceAnnotations,
    index: usize,
    depth: usize,
    visited: &mut BTreeSet<usize>,
    style: TextStyle,
) {
    if !visited.insert(index) {
        return;
    }

    let span = &trace.spans[index];
    writeln!(
        output,
        "{}{}",
        "  ".repeat(depth),
        format_span_line(span, annotations.spans.get(index), style)
    )
    .expect("write to string");

    if let Some(children) = trace.children_by_parent.get(&span.span_id) {
        for child_index in children {
            write_span_tree(
                output,
                trace,
                annotations,
                *child_index,
                depth + 1,
                visited,
                style,
            );
        }
    }
}

fn format_span_line(
    span: &CanonicalSpan,
    annotation: Option<&SpanAnnotation>,
    style: TextStyle,
) -> String {
    let mut line = format!(
        "[{}] {} {} span_id={}",
        style.service(&span.service_name),
        span.name,
        format_duration_styled(style, span.duration_ns()),
        style.identifier(&span.span_id)
    );
    line.push_str(&format!(
        " kind={} status={}",
        span.kind_label(),
        if span.is_error() {
            style.error(span.status_label())
        } else {
            style.ok(span.status_label())
        }
    ));

    if span.is_error() {
        line.push(' ');
        line.push_str(&style.error("ERROR"));
    }

    if let Some(annotation) = annotation
        && annotation.has_annotations()
    {
        line.push(' ');
        line.push_str(&format_inline_annotation(annotation, style));
    }

    line
}

fn format_inline_annotation(annotation: &SpanAnnotation, style: TextStyle) -> String {
    let mut labels = Vec::new();
    if !annotation.client_server_peers.is_empty() {
        let peers = annotation
            .client_server_peers
            .iter()
            .map(|peer| format!("{}:{}", peer.relationship.label(), peer.service_name))
            .collect::<Vec<_>>()
            .join(",");
        labels.push(format!("{}({peers})", style.concurrent("client/server")));
    }
    if annotation.async_work {
        labels.push(style.warning("async").to_string());
    }
    if annotation.messaging {
        labels.push(style.concurrent("messaging").to_string());
    }
    if annotation.linked_span_count > 0 {
        labels.push(style.warning(format!("linked({})", annotation.linked_span_count)));
    }

    format!("标注={}", labels.join(","))
}

fn has_any_annotations(annotations: &TraceAnnotations) -> bool {
    annotations.counts.client_server_pairs > 0
        || annotations.counts.async_span_count > 0
        || annotations.counts.linked_span_count > 0
        || annotations.counts.messaging_span_count > 0
}

fn write_annotations_summary(
    output: &mut String,
    annotations: &TraceAnnotations,
    style: TextStyle,
) {
    writeln!(output, "{}", style.section("Span 语义标注")).expect("write to string");
    writeln!(
        output,
        "client/server pairs: {}  async spans: {}  linked spans: {}  messaging spans: {}",
        style.concurrent(annotations.counts.client_server_pairs),
        style.warning(annotations.counts.async_span_count),
        style.warning(annotations.counts.linked_span_count),
        style.concurrent(annotations.counts.messaging_span_count)
    )
    .expect("write to string");
    writeln!(
        output,
        "{}",
        style.muted("说明：client/server pair 表示 client span 直接调用 server span；tracelens 只做标注，不合并两段耗时。")
    )
    .expect("write to string");
    writeln!(
        output,
        "{}",
        style.muted("说明：async/linked 表示 producer、consumer、messaging 属性或 span links 关联的工作；当前不会因为 links 或 messaging 额外把它们强行计入阻塞关键路径。若这类 span 本身已经是普通 parent-child 子节点，关键路径仍按现有拓扑处理。")
    )
    .expect("write to string");

    write_client_server_pairs(output, &annotations.client_server_pairs, style);
    write_async_linked_spans(output, annotations, style);
}

fn write_client_server_pairs(output: &mut String, pairs: &[ClientServerPair], style: TextStyle) {
    if pairs.is_empty() {
        return;
    }

    writeln!(output).expect("write to string");
    writeln!(output, "{}", style.concurrent("client/server pair：")).expect("write to string");
    for pair in pairs {
        writeln!(
            output,
            "- client [{}] {} span_id={} -> server [{}] {} span_id={}",
            style.service(&pair.client_service_name),
            pair.client_name,
            style.identifier(&pair.client_span_id),
            style.service(&pair.server_service_name),
            pair.server_name,
            style.identifier(&pair.server_span_id)
        )
        .expect("write to string");
    }
}

fn write_async_linked_spans(output: &mut String, annotations: &TraceAnnotations, style: TextStyle) {
    let spans = annotations
        .spans
        .iter()
        .filter(|span| span.is_async_related())
        .collect::<Vec<_>>();
    if spans.is_empty() {
        return;
    }

    writeln!(output).expect("write to string");
    writeln!(output, "{}", style.warning("async / linked span：")).expect("write to string");
    for span in spans {
        writeln!(
            output,
            "- [{}] {} span_id={} role={} 标注={}{}",
            style.service(&span.service_name),
            span.name,
            style.identifier(&span.span_id),
            span.role.label(),
            format_annotation_notes(span, style),
            format_link_targets(span, style)
        )
        .expect("write to string");
    }
}

fn format_annotation_notes(span: &SpanAnnotation, style: TextStyle) -> String {
    let notes = span
        .notes
        .iter()
        .filter(|note| **note != AnnotationNote::ClientServerPair)
        .map(|note| match note {
            AnnotationNote::AsyncKind => style.warning("async-kind"),
            AnnotationNote::MessagingAttributes => style.concurrent("messaging"),
            AnnotationNote::SpanLinks => style.warning("links"),
            AnnotationNote::ClientServerPair => style.concurrent("client/server"),
        })
        .collect::<Vec<_>>();

    if notes.is_empty() {
        style.muted("none")
    } else {
        notes.join(",")
    }
}

fn format_link_targets(span: &SpanAnnotation, style: TextStyle) -> String {
    if span.linked_spans.is_empty() {
        return String::new();
    }

    let links = span
        .linked_spans
        .iter()
        .map(|link| {
            let trace_id = link.trace_id.as_deref().unwrap_or("unknown-trace");
            let span_id = link.span_id.as_deref().unwrap_or("unknown-span");
            let location = if link.target_in_trace {
                "current-trace"
            } else if link.same_trace {
                "same-trace-missing"
            } else {
                "external-trace"
            };
            format!(
                "{}:{}({location})",
                style.identifier(trace_id),
                style.identifier(span_id)
            )
        })
        .collect::<Vec<_>>()
        .join(",");

    format!(" links=[{links}]")
}

fn write_diagnostics(output: &mut String, diagnostics: &[Diagnostic], style: TextStyle) {
    writeln!(output, "{}", style.section("Diagnostics:")).expect("write to string");
    for diagnostic in diagnostics {
        let severity = match diagnostic.severity {
            Severity::Error => style.error(&diagnostic.severity),
            Severity::Warning => style.warning(&diagnostic.severity),
        };
        write!(
            output,
            "- [{}] {}: {}",
            severity,
            style.warning(diagnostic.code),
            diagnostic.message
        )
        .expect("write to string");
        write!(output, " scope={}", diagnostic.scope).expect("write to string");

        if let Some(trace_id) = &diagnostic.trace_id {
            write!(output, " trace_id={}", style.identifier(trace_id)).expect("write to string");
        }
        if let Some(span_id) = &diagnostic.span_id {
            write!(output, " span_id={}", style.identifier(span_id)).expect("write to string");
        }
        if let Some(location) = &diagnostic.location {
            write!(output, " location={location}").expect("write to string");
        }
        writeln!(output).expect("write to string");
    }
}

fn format_range_styled(style: TextStyle, start_ns: Option<u64>, end_ns: Option<u64>) -> String {
    match (start_ns, end_ns) {
        (Some(start_ns), Some(end_ns)) => {
            format!(
                "{} ({})",
                style.identifier(format!("{start_ns}..{end_ns}")),
                format_duration_styled(style, end_ns - start_ns)
            )
        }
        _ => style.muted("unknown"),
    }
}

fn format_optional_duration_styled(style: TextStyle, duration_ns: Option<u64>) -> String {
    duration_ns
        .map(|duration_ns| format_duration_styled(style, duration_ns))
        .unwrap_or_else(|| style.muted("unknown"))
}

fn format_duration_styled(style: TextStyle, duration_ns: u64) -> String {
    style.duration(format_duration(duration_ns))
}

fn style_count_by_risk(style: TextStyle, count: usize) -> String {
    if count == 0 {
        count.to_string()
    } else {
        style.warning(count)
    }
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
