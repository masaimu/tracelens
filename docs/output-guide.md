# Output Guide

This guide explains the most important terms and fields in `tracelens` output.

The goal is to make trace output understandable without requiring the reader to know the implementation details.

## Common Trace Fields

### `trace_id`

The OpenTelemetry trace ID being inspected.

Most single-trace commands require:

```bash
--trace-id <trace-id>
```

IDs are normalized to lowercase hex in default parsing mode.

### `span_id`

The OpenTelemetry span ID.

In text output, span IDs are shown so you can map `tracelens` output back to the original OTLP data.

### `service`

The service name, usually read from resource attribute:

```text
service.name
```

If the attribute is missing, `tracelens` uses a fallback service name and emits a diagnostic.

## OpenTelemetry Metadata in JSON

`tree --output json` includes a canonical span object for each node.

Common OpenTelemetry metadata fields include:

- `trace_state`: OTLP `traceState`, if present.
- `flags`: OTLP span flags, if present.
- `status_message`: OTLP status message.
- `resource_schema_url`: `ResourceSpans.schemaUrl`.
- `scope_name`: instrumentation scope name.
- `scope_version`: instrumentation scope version.
- `scope_attributes`: instrumentation scope attributes.
- `scope_schema_url`: `ScopeSpans.schemaUrl`.
- `dropped_attributes_count`: span dropped attributes count.
- `dropped_events_count`: span dropped events count.
- `dropped_links_count`: span dropped links count.

Events and links also preserve their own dropped attribute counts. Links additionally preserve `trace_state` and `flags`.

Nested OTLP `arrayValue` and `kvlistValue` attributes are preserved as JSON strings inside the current string-based attribute maps. This keeps the data available to scripts and agents while the public attribute model remains simple.

## Duration Fields

### `wall-clock duration`

The time from the earliest span start to the latest span end in the trace.

Use it to answer:

```text
How long did the whole observed trace window last?
```

This can differ from root span duration when a trace has multiple roots, orphan spans, or async work outside the selected root span.

### `root span duration`

The duration of the root span used for trace-level analysis.

When there is exactly one root span, this is straightforward. When a trace has multiple roots, `critical-path` selects the longest root span and prints a note.

### `span_time`

The raw sum of span durations for a service.

Nested and concurrent spans can make this larger than wall-clock time.

### `child_covered_time`

The time range inside a span that is covered by direct child spans.

Overlapping child spans are counted once by interval union.

### `self_time`

The time a service spent in its own spans after subtracting directly covered child intervals.

Use it to answer:

```text
Which service contributed the most own time in this trace?
```

Example:

```text
service              self_time     span_time  child_covered_time  spans  errors
cart-service          50.000ms      50.000ms                 0ns      1       0
payment-service       40.000ms      40.000ms                 0ns      1       1
checkout-service      10.000ms     100.000ms            90.000ms      1       0
```

Here, `checkout-service` owns the root span, but most of its time is covered by child spans.

## Critical Path

`critical-path` explains how the selected root span interval is attributed to spans.

```bash
tracelens critical-path traces.json --trace-id <trace-id>
```

### `critical path duration`

The duration of the selected root span interval.

This is not always the same as wall-clock duration.

### `segments`

Critical path segments split the selected root span interval into attributed windows:

```text
offset       duration    service           name
100.000ms    300.000ms   cart-service      GET /cart
650.000ms    200.000ms   redis             SET cache
```

Columns:

- `offset`: when this segment begins relative to the trace start
- `duration`: how much time this segment contributes
- `service`: owning service
- `name`: span name
- `span_id`: original span ID

When concurrent child spans overlap, `tracelens` attributes that window to the child that ends latest. This keeps the selected root span interval fully covered without double-counting overlapping child time.

### `span_totals`

The total critical path contribution grouped by span.

Use this to answer:

```text
Which spans contributed the most blocking time?
```

## Timeline

`timeline` draws a terminal-friendly time view for one trace.

```bash
tracelens timeline traces.json --trace-id <trace-id>
```

It keeps the parent-child order from the trace graph. There are two layouts:

- `--mode bar` (default): horizontal bars show when each span starts and how long it runs, with concurrent overlap readable across the time axis.
- `--mode flame`: a vertical flame view. Each span gets one row indented by call depth, with the parent above and children directly below indented, so the widest and deepest call stacks are visible at a glance.

Both layouts reuse the same analysis model and carry the same `*` / `!` / `?` markers, so the choice is about reading preference, not different data.

```bash
tracelens timeline traces.json --trace-id <trace-id> --mode flame
```

### Folding large traces

When a single trace has many spans, the default output can fill more than a screen. Use `--max-rows <n>` to fold the middle rows:

```bash
tracelens timeline traces.json --trace-id <trace-id> --max-rows 40
```

Folding keeps the first and last rows as boundaries, plus any rows on the critical path, any error spans, and any orphan or unattached spans. Omitted rows are reported as a single collapse marker row (`... collapsed: N rows omitted ...`), never silently truncated. Pass `--max-rows 0` to show every row.

### Markers

Text output uses stable ASCII markers:

- `*`: the span appears on the critical path
- `!`: the span is an error span
- `?`: the span is orphan or unattached

Bar characters also carry meaning:

- `=`: critical path span
- `!`: error span
- `#`: ordinary span

Colors can highlight these meanings, but the symbols remain readable with:

```bash
tracelens --color never timeline traces.json --trace-id <trace-id>
```

### `bar_width`

The width of the ASCII time bar, not the full terminal line.

Default:

```text
48
```

Allowed range:

```text
40..=160
```

### `start`

The span start offset relative to the trace start.

### `duration`

The span's own duration.

Concurrent spans can overlap, so span durations in the timeline should not be summed as wall-clock time.

### `timeline.rows`

JSON output includes structured rows:

```json
{
  "depth": 1,
  "span_id": "0000000000000003",
  "service_name": "payment-service",
  "name": "POST /charge",
  "start_offset_ns": 500000000,
  "duration_ns": 400000000,
  "bar_start": 21,
  "bar_width": 17,
  "is_critical_path": true,
  "is_error": false,
  "is_orphan": false,
  "is_unattached": false,
  "mode": "bar",
  "is_collapse_marker": false
}
```

`bar_start` and `bar_width` are scaled positions inside the configured time bar. If two rows have overlapping bar ranges, those spans overlap in time.

## Span Execution Classification

`critical-path` also classifies span execution relationships.

```text
serial: 3  concurrent: 4  nested: 5  suspicious: 1
```

### `serial`

A span whose sibling group does not overlap in time.

### `concurrent`

A span that overlaps at least one sibling in the same parent group.

### `nested`

A span fully contained within its parent time range.

### `suspicious`

A span that starts before its parent or ends after its parent.

This does not always mean the trace is wrong, but it is worth checking instrumentation or export quality.

## Semantic Annotations

`tracelens` annotates span semantics in `tree` and `critical-path` output.

```text
Span 语义标注
client/server pairs: 1  async spans: 2  linked spans: 1  messaging spans: 2
```

### Client/server Pair

A direct parent-child edge where:

- parent kind is `client`
- child kind is `server`

`tracelens` marks the pair but does not merge the two spans into one timing node.

This conservative behavior avoids hiding instrumentation differences across service boundaries.

### Async Work

A span is marked as async-related when it is:

- kind `producer`
- kind `consumer`
- carrying `messaging.*` attributes
- carrying span links

### Linked Span

If a span has links, `tracelens` prints the linked trace/span target and whether that target exists in the current trace.

Span links are not converted into parent-child edges.

### Messaging Span

If a span has `messaging.*` attributes, it is marked as messaging-related.

Messaging spans are not automatically treated as blocking causal paths because messaging semantics vary by system and instrumentation.

## Cross-service Edges

`tracelens tree` and `tracelens services` print a cross-service edge summary for a trace.

A cross-service edge is a direct `parent -> child` relationship where the parent span's `service` differs from the child span's `service`. Same-direction repeated calls collapse into one edge, and `span_count` is accumulated.

```text
跨服务边
checkout-service  →  postgres-service  calls=10  (client/server pair: 0)
frontend-service  →  inventory-service  calls=1  (client/server pair: 1)
```

The edge list is sorted by `span_count` descending, with a stable `(from_service, to_service)` tie-breaker, so the busiest cross-service call sits on top.

### `calls`

The number of `parent -> child` spans aggregated into this directional edge. Multiple calls between the same two services in the same direction collapse into one row.

### `client/server pair`

A subset of `calls` where the parent span kind is `client` and the child span kind is `server`. This is stricter than the example-implementation annotations pair: it only counts edges whose instrumentation already declares a client/server boundary in `span.kind`.

When the same trace records a genuine client/server hop, the graph-level pair count equals the annotations pair count. When spans cross service boundaries without an explicit client/server kind, `client/server pair` stays `0` while `calls` still reflects the relational hop.

`(no cross-service edges)` is printed when every span in the trace belongs to the same service.

### JSON

Both `tree --output json` and `services --output json` expose the same top-level `cross_service_edges` array:

```json
"cross_service_edges": [
  {
    "from_service": "frontend-service",
    "to_service": "inventory-service",
    "span_count": 1,
    "client_server_pair_count": 1,
    "sample_parent_span_id": "1000000000000002",
    "sample_span_id": "1000000000000003"
  }
]
```

`sample_parent_span_id` and `sample_span_id` point to one representative parent/child pair, so you can jump back into `tree` for the exact spans behind an edge.

## Detect Candidates

`detect` suggests where to look first across a trace file.

```bash
tracelens detect traces.json --limit 5
```

The output is a candidate list, not a final root-cause verdict.

### `sample_count`

The number of traces with enough timing information to participate in duration-based detection.

Low sample counts reduce confidence. For example, a trace can be the slowest one in a file with only a few samples, but that does not mean it is globally abnormal.

### `sample_quality`

The current sample-size label:

- `insufficient`: fewer than 5 timed traces
- `limited`: fewer than 20 timed traces
- `broad`: 20 or more timed traces

### `p95_duration_ns`

The nearest-rank p95 duration reference for the current file.

It is a local-file reference, not a production latency SLO.

### `confidence`

The confidence marker for a candidate:

- `low`: useful as a hint only
- `medium`: useful for prioritizing the next inspection step
- `high`: strong signal in the current trace file

### `slow_traces`

Slow trace candidates are ranked by wall-clock duration.

Each candidate includes:

- `trace_id`
- `rank`
- `duration_ns`
- `p95_duration_ns`
- `sample_count`
- `confidence`
- `service_candidates`

### `service_candidates`

Services inside a slow trace ranked by `span_time_ns`.

This helps answer:

```text
Which service should I inspect first inside this slow trace?
```

`span_time_ns` is the sum of span durations for that service in the trace. It is a triage hint, not the same as service self time.

### `service_latency_distribution`

Service latency distribution is aggregated across the current file.

Each service includes:

- `service_name`
- `trace_count`
- `span_count`
- `error_span_count`
- `total_span_time_ns`
- `p50_duration_ns`
- `p95_duration_ns`
- `max_span_duration_ns`
- `slow_span_samples`

This helps answer:

```text
Which service looks slow across this trace file?
```

The distribution uses span duration, not service self time. It is useful for triage, while `tracelens services <file> --trace-id <id>` remains the better command for precise self-time analysis inside one trace.

### `error_traces`

Error trace candidates are traces where `tracelens` finds error signals.

Current signals include:

- OTLP `status.code == ERROR`
- HTTP 5xx
- gRPC/RPC non-OK status
- exception events

Each error trace includes:

- `error_span_count`
- `earliest_error_span`
- `top_error_span`
- `error_spans`
- `confidence`

`earliest_error_span` is the first error span by start time. `top_error_span` is the highest-level error span by trace topology. `error_spans` keeps the full evidence list so later error signals are not hidden when the earliest and top spans are the same.

### `error_propagation_chains`

Error propagation chains show observable parent-child evidence for traces with error signals.

Each chain includes:

- `trace_id`
- `confidence`
- `earliest_error_span`
- `top_error_span`
- `path_to_earliest_error`
- `downstream_error_spans`
- `downstream_error_span_count`
- `affected_span_count`
- `affected_services`

`path_to_earliest_error` follows parent links from the visible root or orphan entry point to the earliest visible error span. It may be short when the root span itself is already marked as error.

`downstream_error_spans` lists error spans below `top_error_span`. This helps show whether a high-level failure also appears in child services such as payment, inventory, database, or messaging work.

This is still a candidate explanation, not a root-cause proof. Missing parents, orphan spans, async work, and instrumentation behavior can all affect what the chain can show.

### `n_plus_one_candidates`

N+1 candidates are groups of similar direct child spans under the same parent span.

Current detection is intentionally conservative:

- only direct parent-child relationships are considered
- span links and messaging relationships are not treated as parent-child
- repeated child count must be at least `5`
- high confidence requires repeated count at least `10` and `serial_ratio >= 0.8`

Each candidate includes:

- `trace_id`
- `parent_span`
- `child_group`
- `repeated_count`
- `serial_ratio`
- `confidence`
- `reason`
- `example_child_spans`

### `child_group`

The normalized group signature for repeated child spans.

The grouping uses:

- service name
- normalized span name
- `db.system`
- `db.operation`
- `rpc.system`
- `http.method`
- `http.route`

Numeric parts of span names are normalized. For example:

```text
SELECT product 1
SELECT product 2
```

becomes:

```text
select product {num}
```

### `serial_ratio`

The ratio of adjacent repeated child spans that run sequentially.

If a group has 10 spans and every next span starts after the previous span ends, the ratio is `1.0`.

If repeated child spans mostly overlap, the ratio is lower. This keeps concurrent fan-out from being incorrectly upgraded to high-confidence N+1.

## Diagnostics

Diagnostics are warnings or errors about input quality or trace structure.

Examples:

- `missing_resource_spans`
- `missing_service_name`
- `malformed_jsonl_line`
- `invalid_trace_id`
- `invalid_span_id`
- `invalid_timestamp`
- `invalid_time_range`
- `missing_parent`
- `duplicate_span_id`
- `multiple_root_spans`
- `no_root_span`
- `child_outside_parent`

Diagnostics are intentionally visible. Trace analysis is only useful when you can also see the data quality caveats.

## JSON Output

Most commands support:

```bash
--output json
```

JSON output includes:

```json
{
  "schema_version": "0.1",
  "command": "critical-path"
}
```

The schema is still version `0.1`, so it can change before the project reaches a stable `1.0`.

The published JSON Schema is:

```text
schemas/tracelens-output.schema.json
```

The installed CLI can print the same schema and its field descriptions:

```bash
tracelens schema --output json
tracelens schema --output text
tracelens schema --command detect --output text
```

Use `tracelens schema --output text` when you need to understand what fields such as `self_time_ns`, `critical_path.segments`, `timeline.rows`, `confidence`, or `n_plus_one_candidates` mean. Use `tracelens schema --output json` when a script or agent needs the full machine-readable contract.

See [JSON Schema](json-schema.md) for Agent and automation consumption guidance.

Useful top-level JSON areas:

- `summary`
- `trace`
- `nodes`
- `services`
- `critical_path`
- `timeline`
- `classification`
- `annotations`
- `slow_traces`
- `service_latency_distribution`
- `error_traces`
- `error_propagation_chains`
- `cross_service_edges`
- `n_plus_one_candidates`
- `notes`
- `diagnostics`

## HTML Report

`tracelens report <file> --trace-id <id> --html out.html` writes a single-page offline HTML file.

```bash
tracelens report traces.json --trace-id <trace-id> --html out.html
```

The report is a file, not stdout JSON, so `report` is intentionally not part of `tracelens schema` and does not accept `--output`. It reuses the services / critical-path / tree analysis and renders:

- trace overview
- service timing
- critical path segments and span totals
- cross-service edges
- placeholder blocks for error propagation chains, N+1 candidates, and diagnostics (filled in a later iteration)

`stdout` prints the output path, the trace id, and any warnings; the HTML body is written to the `--html` path. The file is self-contained (inline CSS, no external resources) and opens offline in any browser.

## Exit Codes

`tracelens` uses this first-version exit-code contract:

| code | meaning |
| ---: | --- |
| `0` | command completed and output is usable |
| `1` | input failure, strict validation failure, invalid analysis request, or unmet analysis precondition |
| `2` | CLI usage error, such as an unknown option or missing required argument |

Candidate findings are not command failures. For example, `detect` can find slow/error/N+1 candidates and still exit `0`.

Default `validate` can report diagnostics and still exit `0`. Use `validate --strict` when diagnostics should fail CI.

See [CI integration](ci-integration.md) for practical shell and GitHub Actions examples.

## Color Output

Text output supports:

```bash
--color auto
--color always
--color never
```

Use `--color never` for logs, CI, and file redirection.

JSON output never includes ANSI color escapes.
