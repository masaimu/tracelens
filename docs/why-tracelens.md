# Why tracelens?

`tracelens` is built for a simple but common problem:

```text
You have an OpenTelemetry trace file, but you do not have a trace backend running.
```

In that moment, you still need answers:

- Is this trace file valid?
- Which trace is slow?
- Which traces look suspicious enough to inspect first?
- Where is the time going?
- Which service contributes the most self time?
- Which trace contains error signals?
- Which spans are concurrent, suspicious, or outside their parent range?
- Are client/server and async spans being interpreted safely?

`tracelens` turns raw OTLP JSON or JSONL exports into local, explainable clues.

## The Short Version

Use `tracelens` when you want to understand a trace export without first ingesting it into Jaeger, Tempo, Zipkin, or a vendor platform.

```bash
tracelens critical-path traces.json --trace-id <trace-id>
```

It gives you a terminal-first view of the trace:

- file and trace diagnostics
- span tree structure
- service-level self time
- critical path segments
- slow/error candidates with confidence markers
- serial, concurrent, nested, and suspicious span classification
- client/server, async work, messaging, and linked span annotations
- JSON output for scripts and CI

## What Makes It Useful

### Local-first

`tracelens` reads files directly from disk. It does not require a server, database, agent, collector, or browser UI.

That makes it useful for:

- incident review with exported traces
- offline debugging
- CI checks
- local reproductions
- trace handoff between teams
- agent-driven analysis workflows

### Explainable by Default

The CLI output is intentionally explanatory. It does not only print numbers; it also explains what those numbers mean.

For example, service self time is shown separately from raw span time, critical path output explains how concurrent child spans are attributed, and `detect` marks low-sample findings as candidates instead of pretending they are final conclusions.

### Conservative Trace Semantics

Distributed traces are messy. Real data can include missing parents, orphan spans, multiple roots, duplicated span IDs, async work, and span links.

`tracelens` keeps those structures visible instead of forcing every trace into a perfect tree.

Important semantic choices:

- client/server span pairs are annotated, not merged
- span links are not converted into parent-child edges
- messaging and async spans are marked as related work, not automatically treated as blocking causal paths

### Script-friendly

Human-readable output is useful during debugging, but CI and automation need structured data.

Most commands support:

```bash
--output json
```

Text commands also support:

```bash
--color never
```

That keeps logs and scripts clean.

## What tracelens Is Not

`tracelens` is not a trace backend.

It does not try to replace:

- Jaeger
- Tempo
- Zipkin
- OpenTelemetry Collector
- vendor observability platforms

Those systems are designed for ingestion, storage, querying, dashboards, and long-term operation.

`tracelens` is a local analysis lens for trace files. It fits before, beside, or after a backend workflow.

## When To Reach For It

Reach for `tracelens` when:

- a teammate sends you an OTLP JSON export
- a CI job needs to validate a trace fixture
- an incident review needs a quick local explanation
- you want to inspect span relationships without opening a UI
- an automation agent needs JSON output from a trace analysis step
- you want to understand whether a slow trace is dominated by service self time, a critical path span, error signals, or suspicious structure

If your question is "where should we store millions of traces?", use a backend.

If your question is "what does this one trace file tell me?", use `tracelens`.
