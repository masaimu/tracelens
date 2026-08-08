# Comparison: tracelens vs Jaeger / Tempo / Zipkin / vendor platforms

`tracelens` is not a replacement for trace backends. It exists for the moment when you have a trace file but no backend.

## The core distinction

| | tracelens | Jaeger / Tempo / Zipkin / vendor platforms |
| --- | --- | --- |
| Primary input | A local OTLP JSON / JSONL export file | Data already ingested into a backend |
| Requires running infrastructure | No (local CLI) | Yes (server, storage, collector, often a UI) |
| Lifetime of data | The file on disk | Persisted in a backend store |
| Typical interaction | `tracelens critical-path traces.json` on your machine | Query a UI or API against stored data |

`tracelens` reads a file and leaves. It does not collect, store, query online, or serve a web UI.

## When you would still use a backend

A backend wins when you need things a single file cannot give you:

- long-lived retention and historical trends
- cross-service latency percentiles over many days
- live tailing and alerting
- multi-user collaboration in a UI
- retention and search across large trace corpora

`tracelens` deliberately does none of these.

## When tracelens is the right tool

- You received an exported trace file and want to understand it now, locally.
- You are doing offline debugging, CI checks, incident review, or trace handoff, and the data is already a file.
- You need script-friendly, schema-backed JSON output for agents or automation, without standing up a backend.
- You want an explainable view of one trace's structure, self time, critical path, and anomalies.

## Complementary, not competitive

`tracelens` and trace backends solve different stages of the same workflow. A backend ingests and stores traces; `tracelens` explains a single exported trace file on your machine. In incident review or trace handoff, you often export one trace from your backend and hand it to `tracelens` for a focused, explainable local analysis.

`tracelens` does not claim feature parity with Jaeger, Tempo, Zipkin, or any vendor platform. It is a local file analyzer that covers the gap those platforms do not target: you have the file, not the backend.
