# OpenTelemetry Compatibility

`tracelens` is a local OpenTelemetry trace analysis CLI. It reads OTLP trace exports from disk, normalizes them into a canonical span model, and runs local analysis.

This page explains what part of OpenTelemetry is supported today.

References:

- [OTLP specification](https://opentelemetry.io/docs/specs/otlp/)
- [OTLP file exporter specification](https://opentelemetry.io/docs/specs/otel/protocol/file-exporter/)
- [OpenTelemetry trace proto](https://github.com/open-telemetry/opentelemetry-proto/blob/main/opentelemetry/proto/trace/v1/trace.proto)
- [OpenTelemetry common proto](https://github.com/open-telemetry/opentelemetry-proto/blob/main/opentelemetry/proto/common/v1/common.proto)

## Supported Input Shapes

| Input | Status | Notes |
| --- | --- | --- |
| OTLP JSON traces | Supported | Reads top-level `resourceSpans` objects. |
| OTLP JSONL traces | Supported | Reads one OTLP JSON object per non-empty line. |
| `resourceSpans[].scopeSpans[].spans[]` | Supported | This is the primary trace shape. |
| Unknown JSON fields | Supported | Unknown fields are ignored. |
| Metrics, logs, profiles | Not supported | `tracelens` is trace-only today. |
| Binary protobuf | Not supported | Only JSON/JSONL files are accepted. |
| OTLP/gRPC or OTLP/HTTP receiver | Not supported | `tracelens` is not an ingestion server. |

## OTLP JSON Mapping

| OTLP JSON behavior | tracelens behavior |
| --- | --- |
| lowerCamelCase field names such as `traceId` and `resourceSpans` | Supported. |
| 64-bit integers encoded as decimal strings | Supported for timestamps and numeric fields currently parsed by the CLI. |
| 64-bit integers encoded as JSON numbers | Supported for timestamps and numeric fields currently parsed by the CLI. |
| trace/span IDs encoded as hex strings | Supported. IDs are normalized to lowercase. |
| Uppercase hex trace/span IDs | Accepted and normalized. |
| All-zero trace/span IDs | Rejected as invalid IDs. |
| Enum numeric values | Supported for span kind and status code. |
| Enum name strings | Accepted in the current lenient parser for status code, but OTLP/JSON specifies integer enum values. Treat this as compatibility leniency, not strict OTLP output. |

## Preserved Trace Metadata

`tracelens` currently preserves:

- resource attributes
- resource `schemaUrl`
- scope name
- scope version
- scope attributes
- scope `schemaUrl`
- span `traceState`
- span `flags`
- span kind
- span status code
- span status message
- span attributes
- span dropped attributes count
- events
- event dropped attributes count
- links
- link `traceState`
- link `flags`
- link dropped attributes count
- span dropped events count
- span dropped links count

These fields are visible in `tree --output json` under each span object.

## AnyValue Handling

OpenTelemetry attributes use `AnyValue`.

`tracelens` currently stores canonical attributes as:

```text
BTreeMap<String, String>
```

Supported `AnyValue` forms:

| AnyValue form | Behavior |
| --- | --- |
| `stringValue` | Stored as string. |
| `boolValue` | Stored as `true` or `false`. |
| `intValue` | Stored as decimal string. |
| `doubleValue` | Stored as decimal string when finite. |
| `bytesValue` | Stored as the encoded string. |
| `arrayValue` | Stored as a JSON string. |
| `kvlistValue` | Stored as a JSON object string. |

This preserves nested values without changing the public attribute model yet. A future typed attribute model can build on this, but it is not part of the current milestone.

## Validation and Diagnostics

Default mode is lenient:

```bash
tracelens validate traces.json
```

It tries to keep valid spans while emitting diagnostics for malformed parts.

Strict mode is stricter:

```bash
tracelens validate traces.json --strict
```

Strict validation returns a nonzero exit code when error diagnostics are present.

Current diagnostics include:

- missing `resourceSpans`
- malformed JSONL line
- missing required span fields
- invalid trace/span/parent IDs
- all-zero trace/span IDs
- invalid timestamps
- invalid time range
- missing service name
- missing parent
- duplicate span ID
- multiple root spans
- no root span
- child outside parent time range

## Not Supported Yet

The following are outside the current first-version scope:

- `.json.gz`
- Zipkin adapter
- Jaeger adapter
- W3C Trace Context as a standalone input file
- OTLP binary protobuf
- OTLP/gRPC or OTLP/HTTP ingestion
- full lossless OTLP round-trip
- metrics/logs/profiles analysis
- long-term trace storage
- live tailing

These can be considered later only after they are added to the milestone document.
