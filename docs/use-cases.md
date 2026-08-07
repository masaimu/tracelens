# Use Cases

This guide explains where `tracelens` fits in a real engineering workflow.

`tracelens` is most useful when you have a local OpenTelemetry trace export and want a quick, explainable answer without running a trace backend.

## 1. Validate a Trace Export Before Sharing It

Use this when a trace file comes from a test, CI job, collector export, or teammate.

```bash
tracelens validate traces.json
```

What to look for:

- malformed JSON or JSONL lines
- invalid trace/span IDs
- invalid timestamps
- missing `service.name`
- missing parents
- duplicate span IDs

For stricter checks:

```bash
tracelens validate traces.json --strict
```

Why it matters:

Invalid trace structure can make later analysis misleading. `validate` gives you a fast first-pass quality check.

## 2. Find the Slowest Trace in a File

Use this when a file contains many traces and you need to pick the right one to inspect.

```bash
tracelens list-traces traces.json --limit 10
```

What to look for:

- largest duration
- span count
- service count
- error count
- root/orphan counts
- diagnostics count

Why it matters:

You usually do not want to open every trace. Start with the slowest or riskiest candidate.

## 3. Detect Slow and Error Candidates

Use this when you want `tracelens` to suggest where to look first.

```bash
tracelens detect traces.json --limit 5
```

What to look for:

- slow trace candidates
- `sample_count` and `p95` duration reference
- `confidence`
- service candidates inside each slow trace
- error trace candidates
- earliest error span
- topologically higher error span
- error signals such as OTLP ERROR, HTTP 5xx, gRPC/RPC non-OK, or exception events

Why it matters:

`detect` turns raw trace lists into triage hints. It is intentionally conservative: low sample counts lower confidence, and current output is a candidate list rather than a final root-cause verdict.

## 4. Inspect the Span Tree

Use this when you want to understand parent-child structure and service boundaries.

```bash
tracelens tree traces.json --trace-id <trace-id>
```

What to look for:

- root span
- nested calls
- cross-service relationships
- orphan spans
- duplicate span IDs
- client/server annotations
- async, messaging, and linked span annotations

Why it matters:

The tree view helps you see whether the trace shape matches the system behavior you expected.

## 5. Explain Service-level Time

Use this when the main question is "which service contributed most of the time?"

```bash
tracelens services traces.json --trace-id <trace-id>
```

What to look for:

- `wall-clock duration`
- `root span duration`
- service `self_time`
- service `span_time`
- `child_covered_time`
- error span count per service

Why it matters:

Raw span duration can be misleading when spans nest or overlap. `self_time` removes directly covered child intervals so the service contribution is easier to reason about.

## 6. Explain the Critical Path

Use this when the main question is "what blocked the end-to-end request?"

```bash
tracelens critical-path traces.json --trace-id <trace-id>
```

What to look for:

- critical path duration
- segment offsets and durations
- span totals on the critical path
- concurrent sibling spans
- suspicious spans outside the parent range
- notes about multiple roots or wall-clock/root mismatch

Why it matters:

The slowest raw span is not always the most useful answer. The critical path explains how the root span interval is attributed across nested and concurrent child spans.

## 7. Keep CI Logs Clean

Use this when you want trace checks in CI, scripts, or automation.

```bash
tracelens --color never validate traces.json
tracelens critical-path traces.json --trace-id <trace-id> --output json
tracelens detect traces.json --output json
```

What to look for:

- nonzero exit status in strict validation
- JSON fields under `diagnostics`
- JSON fields under `critical_path`
- JSON fields under `slow_traces` and `error_traces`
- JSON fields under `annotations`

Why it matters:

`--color never` avoids ANSI escapes in logs. `--output json` gives scripts stable structured data.

## 8. Review Async and Linked Work Safely

Use this when a trace includes producer/consumer spans, messaging attributes, or span links.

```bash
tracelens critical-path tests/fixtures/otlp-semantic-annotations.json \
  --trace-id DDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDD
```

What to look for:

- client/server pair count
- async span count
- linked span count
- messaging span count
- link target trace/span IDs

Why it matters:

Async traces can be easy to over-interpret. `tracelens` annotates related work without turning span links into parent-child blocking edges.

## Picking the Right Command

| Question | Command |
| --- | --- |
| Is this file usable? | `validate` |
| Which trace should I inspect first? | `summary` or `list-traces` |
| Which traces look slow or erroneous? | `detect` |
| What is the parent-child shape? | `tree` |
| Which service spent the most own time? | `services` |
| What blocked the root span? | `critical-path` |
| Do I need script-friendly output? | add `--output json` |
| Do I need clean logs? | add `--color never` |
