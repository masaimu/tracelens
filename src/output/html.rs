//! Single-page offline HTML report renderer for `tracelens`.
//!
//! The report only reuses the already-computed analysis model
//! (`TraceGraph`, `TraceDurationAnalysis`, `CriticalPathAnalysis`).
//! It never recomputes durations, critical paths, or cross-service edges.

use crate::analysis::critical_path::{
    CriticalPathAnalysis, CriticalPathRootSpan, CriticalPathStatus,
};
use crate::analysis::duration::{RootSpanDuration, TraceDurationAnalysis};
use crate::graph::trace_graph::TraceGraph;
use crate::output::text::format_duration;

const CSS: &str = r#"
body { font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif; max-width: 1100px; margin: 2rem auto; padding: 0 1rem; line-height: 1.5; color: #1f2328; background: #ffffff; }
h1 { font-size: 1.4rem; margin-bottom: 0.25rem; }
h2 { font-size: 1.1rem; margin-top: 2rem; border-bottom: 1px solid #d0d7de; padding-bottom: 0.3rem; }
.meta { color: #57606a; font-size: 0.85rem; margin-top: 0; }
table { border-collapse: collapse; width: 100%; margin: 0.5rem 0; font-size: 0.9rem; }
th, td { border: 1px solid #d0d7de; padding: 0.35rem 0.5rem; text-align: left; vertical-align: top; }
thead th { background: #f6f8fa; }
code { background: #f6f8fa; padding: 0.05rem 0.3rem; border-radius: 4px; font-family: ui-monospace, SFMono-Regular, Menlo, monospace; }
.mono { font-family: ui-monospace, SFMono-Regular, Menlo, monospace; }
.placeholder { color: #57606a; font-style: italic; border: 1px dashed #d0d7de; padding: 0.6rem 1rem; border-radius: 6px; margin: 0.5rem 0; }
.notes { color: #57606a; font-size: 0.85rem; margin: 0.4rem 0; }
.footer { color: #57606a; font-size: 0.8rem; margin-top: 2rem; border-top: 1px solid #d0d7de; padding-top: 0.6rem; }
.num { text-align: right; }
"#;

/// Render a single-page, zero-dependency, offline HTML report.
///
/// `trace`, `duration` and `critical_path` must already be computed by the
/// existing analysis model. The renderer only maps analysis data to HTML,
/// it never recomputes durations, critical paths, or cross-service edges.
pub fn render_html_report(
    trace: &TraceGraph,
    duration: &TraceDurationAnalysis,
    critical_path: &CriticalPathAnalysis,
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

    write_overview(&mut html, trace, duration, critical_path);
    write_services(&mut html, duration);
    write_critical_path(&mut html, critical_path);
    write_cross_service_edges(&mut html, trace);
    write_placeholder(&mut html, "错误传播链", "error propagation chains");
    write_placeholder(&mut html, "N+1 候选", "n+1 candidates");
    write_placeholder(&mut html, "Diagnostics", "diagnostics");

    html.push_str("<p class=\"footer\">本报告复用 tracelens 的 services / critical-path / tree 分析模型，未重复计算耗时或关键路径。错误传播链、N+1 候选与完整 diagnostics 表格将在第二十四期补齐。</p>\n");
    html.push_str("</body>\n</html>\n");
    html
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

fn write_overview(
    html: &mut String,
    trace: &TraceGraph,
    duration: &TraceDurationAnalysis,
    critical_path: &CriticalPathAnalysis,
) {
    html.push_str("<h2>Trace 概览</h2>\n<table>\n<tbody>\n");
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
    html.push_str(&row("diagnostics", &duration.diagnostics_count.to_string()));
    html.push_str(&row(
        "critical path",
        &match &critical_path.status {
            CriticalPathStatus::Available => {
                format!(
                    "{} \u{00b7} {}",
                    escape_html(critical_path.status.label()),
                    fmt_ns(critical_path.total_duration_ns)
                )
            }
            CriticalPathStatus::Unavailable { reason } => {
                format!(
                    "{} \u{00b7} {}",
                    escape_html(critical_path.status.label()),
                    escape_html(reason)
                )
            }
        },
    ));
    html.push_str("</tbody>\n</table>\n");
}

fn write_services(html: &mut String, duration: &TraceDurationAnalysis) {
    html.push_str("<h2>服务耗时分布</h2>\n");
    if duration.services.is_empty() {
        html.push_str("<p class=\"placeholder\">(no services)</p>\n");
        return;
    }
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
        html.push_str(&format!(
            "<tr><td class=\"mono\">{}</td><td class=\"num\">{}</td><td class=\"num\">{}</td><td class=\"num\">{}</td><td class=\"num\">{}</td><td class=\"num\">{}</td></tr>\n",
            escape_html(&service.service_name),
            fmt_ns(service.self_time_ns),
            fmt_ns(service.span_time_ns),
            fmt_ns(service.child_covered_time_ns),
            service.span_count,
            service.error_span_count,
        ));
    }
    html.push_str("</tbody>\n</table>\n");
    html.push_str("<p class=\"notes\">按 self_time 从高到低排序；self_time 已扣除直接子 span 覆盖的时间区间。</p>\n");
}

fn write_critical_path(html: &mut String, critical_path: &CriticalPathAnalysis) {
    html.push_str("<h2>关键路径</h2>\n");
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
                html.push_str(&format!(
                    "<tr><td class=\"num\">{}</td><td class=\"num\">{}</td><td class=\"mono\">{}</td><td>{}</td><td class=\"mono\">{}</td></tr>\n",
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
                html.push_str("<table>\n<thead>\n<tr><th>total</th><th>service</th><th>name</th><th>span_id</th></tr>\n</thead>\n<tbody>\n");
                for total in &critical_path.span_totals {
                    html.push_str(&format!(
                        "<tr><td class=\"num\">{}</td><td class=\"mono\">{}</td><td>{}</td><td class=\"mono\">{}</td></tr>\n",
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
}

fn write_cross_service_edges(html: &mut String, trace: &TraceGraph) {
    html.push_str("<h2>跨服务调用边</h2>\n");
    if trace.cross_service_edges.is_empty() {
        html.push_str("<p class=\"placeholder\">(no cross-service edges)</p>\n");
        return;
    }
    html.push_str("<table>\n<thead>\n<tr><th>from</th><th>to</th><th>calls</th><th>client/server pair</th></tr>\n</thead>\n<tbody>\n");
    for edge in &trace.cross_service_edges {
        html.push_str(&format!(
            "<tr><td class=\"mono\">{}</td><td class=\"mono\">{}</td><td class=\"num\">{}</td><td class=\"num\">{}</td></tr>\n",
            escape_html(&edge.from_service),
            escape_html(&edge.to_service),
            edge.span_count,
            edge.client_server_pair_count,
        ));
    }
    html.push_str("</tbody>\n</table>\n");
    html.push_str("<p class=\"notes\">按 parent \u{2192} child 方向聚合；同方向多次调用合并为一条边；client/server pair 仅在 parent kind=client \u{2192} child kind=server 时计数。</p>\n");
}

fn write_placeholder(html: &mut String, title: &str, english: &str) {
    html.push_str(&format!("<h2>{}</h2>\n", escape_html(title)));
    html.push_str(&format!(
        "<div class=\"placeholder\">该区块的 {} 渲染将在第二十四期补充，本期未呈现具体证据。</div>\n",
        escape_html(english)
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analysis::critical_path::{
        CriticalPathAnalysis, CriticalPathRootSpan, CriticalPathSegment, CriticalPathStatus,
    };
    use crate::analysis::duration::{RootSpanDuration, ServiceDuration, TraceDurationAnalysis};
    use crate::graph::trace_graph::{CrossServiceEdge, TraceGraph};
    use std::collections::{BTreeMap, BTreeSet};

    fn sample_trace() -> TraceGraph {
        let edge = CrossServiceEdge {
            from_service: "frontend-service".into(),
            to_service: "inventory-service".into(),
            span_count: 1,
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
        let service = ServiceDuration {
            service_name: "frontend-service".into(),
            self_time_ns: 10_000_000,
            span_time_ns: 100_000_000,
            child_covered_time_ns: 0,
            span_count: 1,
            error_span_count: 0,
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
            services: vec![service],
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

    #[test]
    fn renders_doctype_and_four_core_blocks() {
        let trace = sample_trace();
        let duration = sample_duration();
        let critical_path = sample_critical_path();
        let html = render_html_report(&trace, &duration, &critical_path);
        assert!(html.starts_with("<!DOCTYPE html>"));
        assert!(html.contains("Trace 概览"));
        assert!(html.contains("服务耗时分布"));
        assert!(html.contains("关键路径"));
        assert!(html.contains("跨服务调用边"));
        assert!(html.contains("frontend-service"));
        assert!(html.contains("inventory-service"));
        assert!(html.contains("100.000ms"));
    }

    #[test]
    fn renders_empty_cross_service_edges_placeholder() {
        let mut trace = sample_trace();
        trace.cross_service_edges.clear();
        let duration = sample_duration();
        let critical_path = sample_critical_path();
        let html = render_html_report(&trace, &duration, &critical_path);
        assert!(html.contains("(no cross-service edges)"));
    }

    #[test]
    fn escapes_unsafe_service_name() {
        let mut trace = sample_trace();
        trace.cross_service_edges[0].from_service = "<script>alert(1)</script>".into();
        let duration = sample_duration();
        let critical_path = sample_critical_path();
        let html = render_html_report(&trace, &duration, &critical_path);
        assert!(!html.contains("<script>alert(1)</script>"));
        assert!(html.contains("&lt;script&gt;"));
    }

    #[test]
    fn renders_unavailable_critical_path() {
        let trace = sample_trace();
        let duration = sample_duration();
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
        let html = render_html_report(&trace, &duration, &critical_path);
        assert!(html.contains("critical path unavailable"));
        assert!(html.contains("no root span"));
    }
}
