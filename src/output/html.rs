//! Single-page offline HTML report renderer for `tracelens`.
//!
//! The report only reuses the already-computed analysis model
//! (`TraceGraph`, `TraceDurationAnalysis`, `CriticalPathAnalysis`,
//! `DetectAnalysis`). It never recomputes durations, critical paths,
//! cross-service edges, or detect candidates. Color and heat mapping is a
//! pure data -> CSS class mapping over already-computed values.

use std::collections::HashSet;

use crate::analysis::critical_path::{
    CriticalPathAnalysis, CriticalPathRootSpan, CriticalPathStatus,
};
use crate::analysis::detect::{
    Confidence, DetectAnalysis, ErrorPropagationChain, NPlusOneCandidate, NPlusOneSpanRef,
};
use crate::analysis::duration::{RootSpanDuration, TraceDurationAnalysis};
use crate::graph::trace_graph::TraceGraph;
use crate::model::diagnostic::Severity;
use crate::output::text::format_duration;

const CSS: &str = r#"
body { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif; max-width: 1100px; margin: 2rem auto; padding: 0 1rem; line-height: 1.5; color: #1f2328; background: #ffffff; }
h1 { font-size: 1.4rem; margin-bottom: 0.25rem; }
h2 { font-size: 1.1rem; margin-top: 2rem; border-bottom: 1px solid #d0d7de; padding-bottom: 0.3rem; }
h3 { font-size: 1rem; margin: 1rem 0 0.25rem; }
.meta { color: #57606a; font-size: 0.85rem; margin-top: 0; }
.nav { display:flex; flex-wrap:wrap; gap:0.4rem; margin:0.5rem 0 1.5rem; font-size:0.85rem; }
.nav a { color:#0969da; text-decoration:none; padding:0.15rem 0.5rem; border:1px solid #d0d7de; border-radius:999px; }
.nav a:hover { background:#f6f8fa; }
section { scroll-margin-top: 1rem; }
table { border-collapse: collapse; width: 100%; margin: 0.5rem 0; font-size: 0.9rem; }
th, td { border: 1px solid #d0d7de; padding: 0.35rem 0.5rem; text-align: left; vertical-align: top; }
thead th { background: #f6f8fa; }
code { background: #f6f8fa; padding: 0.05rem 0.3rem; border-radius: 4px; font-family: ui-monospace, SFMono-Regular, Menlo, monospace; }
.mono { font-family: ui-monospace, SFMono-Regular, Menlo, monospace; }
.placeholder { color: #57606a; font-style: italic; border: 1px dashed #d0d7de; padding: 0.6rem 1rem; border-radius: 6px; margin: 0.5rem 0; }
.notes { color: #57606a; font-size: 0.85rem; margin: 0.4rem 0; }
.footer { color: #57606a; font-size: 0.8rem; margin-top: 2rem; border-top: 1px solid #d0d7de; padding-top: 0.6rem; }
.num { text-align: right; }
.badge { border-radius: 10px; padding: 0.05rem 0.5rem; font-size: 0.78rem; font-weight: 600; white-space: nowrap; }
.badge-red { background:#ffe0e0; color:#cf222e; }
.badge-amber { background:#fff1d0; color:#9a6700; }
.badge-green { background:#dcf5dc; color:#1a7f37; }
.badge-muted { background:#f6f8fa; color:#57606a; }
.heat-0 { background:#ffffff; }
.heat-1 { background:#fff8e1; }
.heat-2 { background:#ffe0a3; }
.heat-3 { background:#ffb347; }
.heat-4 { background:#ff8a5b; }
tr.critical-seg { background:#eaf2fb; }
tr.critical-seg td:first-child { border-left:3px solid #2f81f7; }
tr.error-row { background:#ffe5e5; }
td.sev-error { color:#cf222e; font-weight:600; }
td.sev-warning { color:#9a6700; font-weight:600; }
.err-mark { color:#cf222e; font-weight:600; }
"#;

const N_PLUS_ONE_HIGH_THRESHOLD: usize = 10;
const DEFAULT_DETECT_LIMIT: usize = 50;

pub fn detect_limit_for_report() -> usize {
    DEFAULT_DETECT_LIMIT
}

/// Render a single-page, zero-dependency, offline HTML report.
pub fn render_html_report(
    trace: &TraceGraph,
    duration: &TraceDurationAnalysis,
    critical_path: &CriticalPathAnalysis,
    detect: &DetectAnalysis,
) -> String {
    let mut html = String::new();
    html.push_str("<!DOCTYPE html>\n<html lang=\"zh-CN\">\n<head>\n");
    html.push_str("<meta charset=\"utf-8\">\n");
    html.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n");
    html.push_str("<title>tracelens trace report</title>\n<style>\n");
    html.push_str(CSS);
    html.push_str("</style>\n</head>\n<body>\n");

    html.push_str("<h1>tracelens trace report</h1>\n");
    html.push_str(&format!(
        "<p class=\"meta\">trace_id: <code>{}</code> \u{00b7} critical path: {}</p>\n",
        escape_html(&trace.trace_id),
        escape_html(critical_path.status.label())
    ));

    write_nav(&mut html);

    let error_span_ids = collect_error_span_ids(trace);
    write_overview(&mut html, trace, duration, critical_path, detect);
    write_services(&mut html, duration);
    write_critical_path(&mut html, critical_path, &error_span_ids);
    write_cross_service_edges(&mut html, trace);
    write_error_propagation(&mut html, detect, &trace.trace_id);
    write_n_plus_one(&mut html, detect, &trace.trace_id);
    write_diagnostics(&mut html, trace);

    html.push_str("<p class=\"footer\">本报告复用 tracelens 的 services / critical-path / tree / detect 分析模型，未重复计算耗时、关键路径或候选；颜色语义与终端 `--color` 一致。</p>\n");
    html.push_str("</body>\n</html>\n");
    html
}

fn write_nav(html: &mut String) {
    html.push_str("<nav class=\"nav\">\n");
    for (id, label) in [
        ("overview", "Trace 概览"),
        ("services", "服务耗时分布"),
        ("critical-path", "关键路径"),
        ("cross-service-edges", "跨服务调用边"),
        ("error-propagation", "错误传播链"),
        ("n-plus-one", "N+1 候选"),
        ("diagnostics", "Diagnostics"),
    ] {
        html.push_str(&format!(
            "<a href=\"#{}\">{}</a>\n",
            escape_html(id),
            escape_html(label)
        ));
    }
    html.push_str("</nav>\n");
}

fn collect_error_span_ids(trace: &TraceGraph) -> HashSet<String> {
    trace
        .spans
        .iter()
        .filter(|span| span.is_error())
        .map(|span| span.span_id.clone())
        .collect()
}

fn escape_html(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
    out
}

fn row(label: &str, value: &str) -> String {
    format!(
        "<tr><th>{}</th><td>{}</td></tr>\n",
        escape_html(label),
        value
    )
}

fn fmt_ns(ns: u64) -> String {
    escape_html(&format_duration(ns))
}

fn fmt_opt_ns(ns: Option<u64>) -> String {
    match ns {
        Some(value) => fmt_ns(value),
        None => escape_html("unknown"),
    }
}

fn fmt_ratio(per_mille: u16) -> String {
    escape_html(&format!("{:.1}%", per_mille as f64 / 10.0))
}

fn fmt_root_span(root: &RootSpanDuration) -> String {
    format!(
        "{} [{}] <code>{}</code> {}",
        escape_html(&root.name),
        escape_html(&root.service_name),
        escape_html(&root.span_id),
        fmt_ns(root.duration_ns)
    )
}

fn fmt_critical_root_span(root: &CriticalPathRootSpan) -> String {
    format!(
        "{} [{}] <code>{}</code> {}",
        escape_html(&root.name),
        escape_html(&root.service_name),
        escape_html(&root.span_id),
        fmt_ns(root.duration_ns)
    )
}

fn confidence_badge(confidence: Confidence) -> String {
    let (cls, label) = match confidence {
        Confidence::Low => ("badge-muted", "low"),
        Confidence::Medium => ("badge-amber", "medium"),
        Confidence::High => ("badge-red", "high"),
    };
    format!("<span class=\"badge {cls}\">{label}</span>")
}

fn severity_cell(severity: &Severity) -> String {
    let (cls, label) = match severity {
        Severity::Warning => ("sev-warning", "warning"),
        Severity::Error => ("sev-error", "error"),
    };
    format!("<td class=\"{cls}\">{label}</td>")
}

fn heat_class(value: u64, max: u64) -> &'static str {
    if max == 0 || value == 0 {
        return "heat-0";
    }
    let ratio = value as f64 / max as f64;
    if ratio >= 0.75 {
        "heat-4"
    } else if ratio >= 0.5 {
        "heat-3"
    } else if ratio >= 0.25 {
        "heat-2"
    } else {
        "heat-1"
    }
}

fn n_plus_one_calls_badge(count: usize) -> String {
    if count >= N_PLUS_ONE_HIGH_THRESHOLD {
        format!("<span class=\"badge badge-red\">{count}</span>")
    } else if count >= 5 {
        format!("<span class=\"badge badge-amber\">{count}</span>")
    } else {
        count.to_string()
    }
}

fn write_overview(
    html: &mut String,
    trace: &TraceGraph,
    duration: &TraceDurationAnalysis,
    critical_path: &CriticalPathAnalysis,
    detect: &DetectAnalysis,
) {
    html.push_str("<section id=\"overview\">\n<h2>Trace 概览</h2>\n<table>\n<tbody>\n");
    html.push_str(&row(
        "trace_id",
        &format!("<code>{}</code>", escape_html(&trace.trace_id)),
    ));
    html.push_str(&row(
        "wall-clock duration",
        &fmt_opt_ns(duration.wall_clock_duration_ns),
    ));
    html.push_str(&row(
        "root span",
        &match &duration.root_span {
            Some(root) => fmt_root_span(root),
            None => escape_html("none"),
        },
    ));
    html.push_str(&row("spans", &trace.spans.len().to_string()));
    html.push_str(&row("roots", &duration.root_count.to_string()));
    html.push_str(&row("orphans", &duration.orphan_count.to_string()));
    html.push_str(&row(
        "duplicate span IDs",
        &trace.duplicate_span_ids.len().to_string(),
    ));
    let errors_cell = if duration.diagnostics_count > 0
        || trace
            .diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error)
    {
        format!(
            "<span class=\"err-mark\">{}</span>",
            trace.diagnostics.len()
        )
    } else {
        trace.diagnostics.len().to_string()
    };
    html.push_str(&row("diagnostics", &errors_cell));
    let critical_cell = match &critical_path.status {
        CriticalPathStatus::Available => format!(
            "<span class=\"badge badge-green\">{}</span> \u{00b7} {}",
            escape_html(critical_path.status.label()),
            fmt_ns(critical_path.total_duration_ns)
        ),
        CriticalPathStatus::Unavailable { reason } => format!(
            "<span class=\"badge badge-muted\">{}</span> \u{00b7} {}",
            escape_html(critical_path.status.label()),
            escape_html(reason)
        ),
    };
    html.push_str(&row("critical path", &critical_cell));
    if let Some(slow) = detect
        .slow_traces
        .iter()
        .find(|c| c.trace_id == trace.trace_id)
    {
        let rank_cell = format!(
            "{} / {} {}",
            slow.rank,
            fmt_ns(slow.duration_ns),
            confidence_badge(slow.confidence)
        );
        html.push_str(&row("慢请求候选", &rank_cell));
    }
    html.push_str("</tbody>\n</table>\n</section>\n");
}

fn write_services(html: &mut String, duration: &TraceDurationAnalysis) {
    html.push_str("<section id=\"services\">\n<h2>服务耗时分布</h2>\n");
    if duration.services.is_empty() {
        html.push_str("<p class=\"placeholder\">(no services)</p>\n</section>\n");
        return;
    }
    let max_self = duration
        .services
        .iter()
        .map(|s| s.self_time_ns)
        .max()
        .unwrap_or(0);
    html.push_str("<table>\n<thead>\n<tr>");
    for header in [
        "service",
        "self_time",
        "span_time",
        "child_covered_time",
        "spans",
        "errors",
    ] {
        html.push_str(&format!("<th>{}</th>", escape_html(header)));
    }
    html.push_str("</tr>\n</thead>\n<tbody>\n");
    for service in &duration.services {
        let heat = heat_class(service.self_time_ns, max_self);
        let errors_cell = if service.error_span_count > 0 {
            format!(
                "<span class=\"badge badge-red\">{}</span>",
                service.error_span_count
            )
        } else {
            service.error_span_count.to_string()
        };
        html.push_str(&format!(
            "<tr><td class=\"mono\">{}</td><td class=\"num {}\">{}</td><td class=\"num\">{}</td><td class=\"num\">{}</td><td class=\"num\">{}</td><td class=\"num\">{}</td></tr>\n",
            escape_html(&service.service_name),
            heat,
            fmt_ns(service.self_time_ns),
            fmt_ns(service.span_time_ns),
            fmt_ns(service.child_covered_time_ns),
            service.span_count,
            errors_cell,
        ));
    }
    html.push_str("</tbody>\n</table>\n");
    html.push_str("<p class=\"notes\">按 self_time 从高到低排序；self_time 列按相对最大值热力着色（0 最浅、最大最深），已扣除直接子 span 覆盖的时间区间。</p>\n");
    html.push_str("</section>\n");
}

fn write_critical_path(
    html: &mut String,
    critical_path: &CriticalPathAnalysis,
    error_span_ids: &HashSet<String>,
) {
    html.push_str("<section id=\"critical-path\">\n<h2>关键路径</h2>\n");
    match &critical_path.status {
        CriticalPathStatus::Unavailable { reason } => {
            html.push_str(&format!(
                "<p class=\"placeholder\">critical path unavailable: {}</p>\n",
                escape_html(reason)
            ));
        }
        CriticalPathStatus::Available => {
            if let Some(root) = &critical_path.root_span {
                html.push_str(&format!(
                    "<p class=\"meta\">root span: {} \u{00b7} total: {}</p>\n",
                    fmt_critical_root_span(root),
                    fmt_ns(critical_path.total_duration_ns)
                ));
            }
            html.push_str("<table>\n<thead>\n<tr>");
            for header in ["offset", "duration", "service", "name", "span_id"] {
                html.push_str(&format!("<th>{}</th>", escape_html(header)));
            }
            html.push_str("</tr>\n</thead>\n<tbody>\n");
            for seg in &critical_path.segments {
                let is_error = error_span_ids.contains(&seg.span_id);
                let row_class = if is_error {
                    " error-row"
                } else {
                    " critical-seg"
                };
                let error_mark = if is_error {
                    " <span class=\"err-mark\">ERROR</span>"
                } else {
                    ""
                };
                html.push_str(&format!(
                    "<tr class=\"{row_class}\"><td class=\"num\">{}</td><td class=\"num\">{}</td><td class=\"mono\">{}</td><td>{}{error_mark}</td><td class=\"mono\">{}</td></tr>\n",
                    fmt_ns(seg.offset_ns),
                    fmt_ns(seg.duration_ns),
                    escape_html(&seg.service_name),
                    escape_html(&seg.name),
                    escape_html(&seg.span_id),
                ));
            }
            html.push_str("</tbody>\n</table>\n");

            html.push_str("<h3>关键路径 span 汇总</h3>\n");
            if critical_path.span_totals.is_empty() {
                html.push_str("<p class=\"placeholder\">(no span totals)</p>\n");
            } else {
                let max_total = critical_path
                    .span_totals
                    .iter()
                    .map(|t| t.total_ns)
                    .max()
                    .unwrap_or(0);
                html.push_str("<table>\n<thead>\n<tr><th>total</th><th>service</th><th>name</th><th>span_id</th></tr>\n</thead>\n<tbody>\n");
                for total in &critical_path.span_totals {
                    let heat = heat_class(total.total_ns, max_total);
                    html.push_str(&format!(
                        "<tr><td class=\"num {heat}\">{}</td><td class=\"mono\">{}</td><td>{}</td><td class=\"mono\">{}</td></tr>\n",
                        fmt_ns(total.total_ns),
                        escape_html(&total.service_name),
                        escape_html(&total.name),
                        escape_html(&total.span_id),
                    ));
                }
                html.push_str("</tbody>\n</table>\n");
            }
        }
    }

    if !critical_path.notes.is_empty() {
        html.push_str("<p class=\"notes\">\n");
        for note in &critical_path.notes {
            html.push_str(&format!("- {}\n", escape_html(note)));
        }
        html.push_str("</p>\n");
    }
    html.push_str("</section>\n");
}

fn write_cross_service_edges(html: &mut String, trace: &TraceGraph) {
    html.push_str("<section id=\"cross-service-edges\">\n<h2>跨服务调用边</h2>\n");
    if trace.cross_service_edges.is_empty() {
        html.push_str("<p class=\"placeholder\">(no cross-service edges)</p>\n</section>\n");
        return;
    }
    html.push_str("<table>\n<thead>\n<tr><th>from</th><th>to</th><th>calls</th><th>client/server pair</th></tr>\n</thead>\n<tbody>\n");
    for edge in &trace.cross_service_edges {
        html.push_str(&format!(
            "<tr><td class=\"mono\">{}</td><td class=\"mono\">{}</td><td class=\"num\">{}</td><td class=\"num\">{}</td></tr>\n",
            escape_html(&edge.from_service),
            escape_html(&edge.to_service),
            n_plus_one_calls_badge(edge.span_count),
            edge.client_server_pair_count,
        ));
    }
    html.push_str("</tbody>\n</table>\n");
    html.push_str("<p class=\"notes\">按 parent \u{2192} child 方向聚合；同方向多次调用合并为一条边；calls \u{2265} 10 标红、\u{2265} 5 标橙；client/server pair 仅在 parent kind=client \u{2192} child kind=server 时计数。</p>\n");
    html.push_str("</section>\n");
}

fn write_error_propagation(html: &mut String, detect: &DetectAnalysis, trace_id: &str) {
    html.push_str("<section id=\"error-propagation\">\n<h2>错误传播链</h2>\n");
    let chains: Vec<&ErrorPropagationChain> = detect
        .error_propagation_chains
        .iter()
        .filter(|c| c.trace_id == trace_id)
        .collect();
    if chains.is_empty() {
        html.push_str("<p class=\"placeholder\">(no error propagation chains)</p>\n</section>\n");
        return;
    }
    for (idx, chain) in chains.iter().enumerate() {
        html.push_str(&format!(
            "<h3>链 {} {} <code>{}</code></h3>\n",
            idx + 1,
            confidence_badge(chain.confidence),
            escape_html(&chain.trace_id)
        ));
        html.push_str(&format!(
            "<p class=\"meta\">affected services: {} \u{00b7} affected spans: {} \u{00b7} downstream errors: {}</p>\n",
            escape_html(&chain.affected_services.join(", ")),
            chain.affected_span_count,
            chain.downstream_error_span_count,
        ));
        html.push_str(&format!(
            "<p class=\"notes\">{}</p>\n",
            escape_html(&chain.explanation)
        ));
        html.push_str("<p class=\"notes\">root \u{2192} earliest error 路径：</p>\n");
        html.push_str("<table>\n<thead>\n<tr><th>depth</th><th>service</th><th>name</th><th>span_id</th><th>duration</th></tr>\n</thead>\n<tbody>\n");
        for step in &chain.path_to_earliest_error {
            html.push_str(&format!(
                "<tr{}><td class=\"num\">{}</td><td class=\"mono\">{}</td><td>{}{}</td><td class=\"mono\">{}</td><td class=\"num\">{}</td></tr>\n",
                if step.is_error { " class=\"error-row\"" } else { "" },
                step.depth,
                escape_html(&step.service_name),
                escape_html(&step.name),
                if step.is_error { " <span class=\"err-mark\">ERROR</span>" } else { "" },
                escape_html(&step.span_id),
                fmt_ns(step.duration_ns),
            ));
        }
        html.push_str("</tbody>\n</table>\n");
        if !chain.downstream_error_spans.is_empty() {
            html.push_str("<p class=\"notes\">top error span 下游错误证据：</p>\n<table>\n<thead>\n<tr><th>depth</th><th>service</th><th>name</th><th>span_id</th><th>duration</th></tr>\n</thead>\n<tbody>\n");
            for step in &chain.downstream_error_spans {
                html.push_str(&format!(
                    "<tr class=\"error-row\"><td class=\"num\">{}</td><td class=\"mono\">{}</td><td>{}</td><td class=\"mono\">{}</td><td class=\"num\">{}</td></tr>\n",
                    step.depth,
                    escape_html(&step.service_name),
                    escape_html(&step.name),
                    escape_html(&step.span_id),
                    fmt_ns(step.duration_ns),
                ));
            }
            html.push_str("</tbody>\n</table>\n");
        }
    }
    html.push_str("</section>\n");
}

fn write_n_plus_one(html: &mut String, detect: &DetectAnalysis, trace_id: &str) {
    html.push_str("<section id=\"n-plus-one\">\n<h2>N+1 候选</h2>\n");
    let candidates: Vec<&NPlusOneCandidate> = detect
        .n_plus_one_candidates
        .iter()
        .filter(|c| c.trace_id == trace_id)
        .collect();
    if candidates.is_empty() {
        html.push_str("<p class=\"placeholder\">(no n+1 candidates)</p>\n</section>\n");
        return;
    }
    for (idx, candidate) in candidates.iter().enumerate() {
        html.push_str(&format!(
            "<h3>候选 {} {} repeated={} serial_ratio={}</h3>\n",
            idx + 1,
            confidence_badge(candidate.confidence),
            n_plus_one_calls_badge(candidate.repeated_count),
            fmt_ratio(candidate.serial_ratio_per_mille),
        ));
        html.push_str(&format!(
            "<p class=\"notes\">{}</p>\n",
            escape_html(&candidate.reason)
        ));
        let parent = &candidate.parent_span;
        html.push_str("<p class=\"meta\">parent span：</p>\n");
        html.push_str(&format_span_ref_table(parent, false));
        let group = &candidate.child_group;
        html.push_str("<p class=\"meta\">child group：</p>\n<table>\n<tbody>\n");
        html.push_str(&row("service", &escape_html(&group.service_name)));
        html.push_str(&row(
            "normalized name",
            &escape_html(&group.normalized_name),
        ));
        if let Some(db) = &group.db_system {
            html.push_str(&row("db.system", &escape_html(db)));
        }
        if let Some(op) = &group.db_operation {
            html.push_str(&row("db.operation", &escape_html(op)));
        }
        if let Some(rpc) = &group.rpc_system {
            html.push_str(&row("rpc.system", &escape_html(rpc)));
        }
        if let Some(m) = &group.http_method {
            html.push_str(&row("http.method", &escape_html(m)));
        }
        if let Some(r) = &group.http_route {
            html.push_str(&row("http.route", &escape_html(r)));
        }
        html.push_str("</tbody>\n</table>\n");
        if !candidate.example_child_spans.is_empty() {
            html.push_str("<p class=\"meta\">示例 child span：</p>\n");
            for child in candidate.example_child_spans.iter().take(3) {
                html.push_str(&format_span_ref_table(child, false));
            }
        }
    }
    html.push_str("</section>\n");
}

fn format_span_ref_table(span: &NPlusOneSpanRef, _error: bool) -> String {
    format!(
        "<table>\n<tbody>\n<tr><th>depth</th><td>{}</td></tr>\n<tr><th>service</th><td class=\"mono\">{}</td></tr>\n<tr><th>name</th><td>{}</td></tr>\n<tr><th>span_id</th><td class=\"mono\">{}</td></tr>\n<tr><th>duration</th><td class=\"num\">{}</td></tr>\n</tbody>\n</table>\n",
        span.depth,
        escape_html(&span.service_name),
        escape_html(&span.name),
        escape_html(&span.span_id),
        fmt_ns(span.duration_ns),
    )
}

fn write_diagnostics(html: &mut String, trace: &TraceGraph) {
    html.push_str("<section id=\"diagnostics\">\n<h2>Diagnostics</h2>\n");
    if trace.diagnostics.is_empty() {
        html.push_str("<p class=\"placeholder\">(no diagnostics)</p>\n</section>\n");
        return;
    }
    html.push_str("<table>\n<thead>\n<tr><th>severity</th><th>scope</th><th>code</th><th>message</th><th>span_id</th><th>location</th></tr>\n</thead>\n<tbody>\n");
    for diagnostic in &trace.diagnostics {
        let span_id = diagnostic
            .span_id
            .as_deref()
            .map(|s| format!("<code>{}</code>", escape_html(s)))
            .unwrap_or_else(|| escape_html("-"));
        let location = diagnostic
            .location
            .as_deref()
            .map(escape_html)
            .unwrap_or_else(|| escape_html("-"));
        html.push_str(&format!(
            "<tr>{}<td>{}</td><td class=\"mono\">{}</td><td>{}</td><td class=\"mono\">{}</td><td>{}</td></tr>\n",
            severity_cell(&diagnostic.severity),
            escape_html(&diagnostic.scope.to_string()),
            escape_html(diagnostic.code),
            escape_html(&diagnostic.message),
            span_id,
            location,
        ));
    }
    html.push_str("</tbody>\n</table>\n");
    html.push_str("<p class=\"notes\">diagnostics severity 着色：warning 黄、error 红；这些是数据质量与分析前置条件的提示，不静默忽略。</p>\n");
    html.push_str("</section>\n");
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::critical_path::{
        CriticalPathAnalysis, CriticalPathRootSpan, CriticalPathSegment, CriticalPathStatus,
    };
    use crate::analysis::detect::{
        DetectAnalysis, DetectSummary, NPlusOneCandidate, NPlusOneChildGroup, NPlusOneSpanRef,
    };
    use crate::analysis::duration::{RootSpanDuration, ServiceDuration, TraceDurationAnalysis};
    use crate::graph::trace_graph::{CrossServiceEdge, TraceGraph};
    use std::collections::{BTreeMap, BTreeSet};

    fn sample_trace() -> TraceGraph {
        let edge = CrossServiceEdge {
            from_service: "frontend-service".into(),
            to_service: "inventory-service".into(),
            span_count: 10,
            client_server_pair_count: 1,
            sample_span_id: "1000000000000003".into(),
            sample_parent_span_id: "1000000000000002".into(),
        };
        TraceGraph {
            trace_id: "dddddddddddddddddddddddddddddddd".into(),
            spans: Vec::new(),
            children_by_parent: BTreeMap::new(),
            root_indices: Vec::new(),
            orphan_indices: Vec::new(),
            duplicate_span_ids: BTreeSet::new(),
            diagnostics: Vec::new(),
            cross_service_edges: vec![edge],
        }
    }

    fn sample_duration() -> TraceDurationAnalysis {
        let hot = ServiceDuration {
            service_name: "frontend-service".into(),
            self_time_ns: 80_000_000,
            self_time_ratio: Some(0.8),
            span_time_ns: 100_000_000,
            child_covered_time_ns: 0,
            span_count: 1,
            error_span_count: 0,
        };
        let cold = ServiceDuration {
            service_name: "inventory-service".into(),
            self_time_ns: 10_000_000,
            self_time_ratio: Some(0.1),
            span_time_ns: 10_000_000,
            child_covered_time_ns: 0,
            span_count: 1,
            error_span_count: 1,
        };
        TraceDurationAnalysis {
            trace_id: "dddddddddddddddddddddddddddddddd".into(),
            wall_clock_duration_ns: Some(100_000_000),
            root_span: Some(RootSpanDuration {
                span_id: "1000000000000001".into(),
                service_name: "frontend-service".into(),
                name: "GET /checkout".into(),
                duration_ns: 100_000_000,
            }),
            root_count: 1,
            orphan_count: 0,
            diagnostics_count: 0,
            services: vec![hot, cold],
            spans: Vec::new(),
        }
    }

    fn sample_critical_path() -> CriticalPathAnalysis {
        CriticalPathAnalysis {
            status: CriticalPathStatus::Available,
            root_span_id: Some("1000000000000001".into()),
            root_span: Some(CriticalPathRootSpan {
                span_id: "1000000000000001".into(),
                service_name: "frontend-service".into(),
                name: "GET /checkout".into(),
                duration_ns: 100_000_000,
            }),
            total_duration_ns: 100_000_000,
            segments: vec![CriticalPathSegment {
                span_id: "1000000000000001".into(),
                service_name: "frontend-service".into(),
                name: "GET /checkout".into(),
                offset_ns: 0,
                duration_ns: 100_000_000,
            }],
            span_totals: Vec::new(),
            notes: Vec::new(),
        }
    }

    fn empty_detect() -> DetectAnalysis {
        DetectAnalysis {
            summary: DetectSummary {
                trace_count: 0,
                span_count: 0,
                diagnostics_count: 0,
                sample_count: 0,
                sample_quality: crate::analysis::detect::SampleQuality::Insufficient,
                p95_duration_ns: None,
                slow_trace_candidate_count: 0,
                error_trace_candidate_count: 0,
                error_propagation_chain_count: 0,
                n_plus_one_candidate_count: 0,
                service_latency_distribution_count: 0,
                error_span_count: 0,
            },
            limit: DEFAULT_DETECT_LIMIT,
            slow_traces: Vec::new(),
            error_traces: Vec::new(),
            error_propagation_chains: Vec::new(),
            n_plus_one_candidates: Vec::new(),
            service_latency_distribution: Vec::new(),
            notes: Vec::new(),
        }
    }

    fn sample_n_plus_one() -> NPlusOneCandidate {
        NPlusOneCandidate {
            trace_id: "dddddddddddddddddddddddddddddddd".into(),
            parent_span: NPlusOneSpanRef {
                span_id: "1000000000000001".into(),
                parent_span_id: None,
                service_name: "frontend-service".into(),
                name: "GET /checkout".into(),
                depth: 0,
                start_ns: 0,
                duration_ns: 100_000_000,
            },
            child_group: NPlusOneChildGroup {
                service_name: "inventory-service".into(),
                normalized_name: "get /inventory/{id}".into(),
                db_system: None,
                db_operation: None,
                rpc_system: None,
                http_method: Some("GET".into()),
                http_route: Some("/inventory/{id}".into()),
                signature: String::new(),
            },
            repeated_count: 10,
            serial_ratio_per_mille: 1000,
            confidence: Confidence::High,
            reason: "high confidence".into(),
            example_child_spans: Vec::new(),
        }
    }

    #[test]
    fn renders_doctype_nav_and_blocks() {
        let trace = sample_trace();
        let detect = empty_detect();
        let html = render_html_report(&trace, &sample_duration(), &sample_critical_path(), &detect);
        assert!(html.starts_with("<!DOCTYPE html>"));
        assert!(html.contains("<nav class=\"nav\">"));
        for anchor in [
            "#overview",
            "#services",
            "#critical-path",
            "#cross-service-edges",
            "#error-propagation",
            "#n-plus-one",
            "#diagnostics",
        ] {
            assert!(html.contains(anchor), "nav should link to {anchor}");
        }
        assert!(html.contains("Trace 概览"));
        assert!(html.contains("服务耗时分布"));
        assert!(html.contains("关键路径"));
        assert!(html.contains("跨服务调用边"));
    }

    #[test]
    fn empty_three_blocks_show_placeholders() {
        let trace = sample_trace();
        let detect = empty_detect();
        let html = render_html_report(&trace, &sample_duration(), &sample_critical_path(), &detect);
        assert!(html.contains("(no error propagation chains)"));
        assert!(html.contains("(no n+1 candidates)"));
        // sample_trace has no diagnostics
        assert!(html.contains("(no diagnostics)"));
    }

    #[test]
    fn services_heatmap_marks_hot_and_cold() {
        let trace = sample_trace();
        let detect = empty_detect();
        let html = render_html_report(&trace, &sample_duration(), &sample_critical_path(), &detect);
        // hottest self_time (80ms) should get the deepest heat class
        assert!(html.contains("heat-4"));
        // cold service (10ms) should not get heat-4
        assert!(html.contains("heat-1"));
        // error_span_count>0 renders a red badge
        assert!(html.contains("badge-red"));
    }

    #[test]
    fn cross_service_edge_high_calls_badge() {
        let trace = sample_trace(); // edge.span_count = 10
        let detect = empty_detect();
        let html = render_html_report(&trace, &sample_duration(), &sample_critical_path(), &detect);
        assert!(html.contains("badge-red\">10</span>"));
    }

    #[test]
    fn n_plus_one_block_renders_real_candidate() {
        let mut detect = empty_detect();
        detect.n_plus_one_candidates = vec![sample_n_plus_one()];
        let trace = sample_trace();
        let html = render_html_report(&trace, &sample_duration(), &sample_critical_path(), &detect);
        assert!(html.contains("N+1 候选"));
        assert!(html.contains("repeated="));
        assert!(html.contains("inventory-service"));
        assert!(!html.contains("(no n+1 candidates)"));
    }

    #[test]
    fn escapes_unsafe_service_name() {
        let mut trace = sample_trace();
        trace.cross_service_edges[0].from_service = "<script>alert(1)</script>".into();
        let detect = empty_detect();
        let html = render_html_report(&trace, &sample_duration(), &sample_critical_path(), &detect);
        assert!(!html.contains("<script>alert(1)</script>"));
        assert!(html.contains("&lt;script&gt;"));
    }

    #[test]
    fn renders_unavailable_critical_path() {
        let trace = sample_trace();
        let critical_path = CriticalPathAnalysis {
            status: CriticalPathStatus::Unavailable {
                reason: "no root span".into(),
            },
            root_span_id: None,
            root_span: None,
            total_duration_ns: 0,
            segments: Vec::new(),
            span_totals: Vec::new(),
            notes: Vec::new(),
        };
        let html = render_html_report(&trace, &sample_duration(), &critical_path, &empty_detect());
        assert!(html.contains("critical path unavailable"));
        assert!(html.contains("no root span"));
    }
}
