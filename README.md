<p align="center">
  <img src="assets/logo.svg" alt="tracelens logo" width="160" />
</p>

<h1 align="center">tracelens</h1>

<p align="center">
  Understand slow OpenTelemetry traces locally, without running a trace backend.
</p>

<p align="center">
  <a href="README.zh-CN.md">中文</a>
</p>

<p align="center">
  <a href="https://github.com/masaimu/tracelens/actions/workflows/ci.yml">
    <img src="https://github.com/masaimu/tracelens/actions/workflows/ci.yml/badge.svg" alt="CI status" />
  </a>
  <a href="https://github.com/masaimu/tracelens/actions/workflows/benchmark.yml">
    <img src="https://github.com/masaimu/tracelens/actions/workflows/benchmark.yml/badge.svg" alt="Benchmark status" />
  </a>
  <a href="https://github.com/masaimu/tracelens/actions/workflows/security.yml">
    <img src="https://github.com/masaimu/tracelens/actions/workflows/security.yml/badge.svg" alt="Security status" />
  </a>
</p>

## What Is tracelens?

`tracelens` is a command-line tool for exploring OpenTelemetry trace exports on your local machine.

It is built for the moments when you have a trace file, not a running trace backend. Give `tracelens` an OTLP JSON or JSONL export, and it helps you validate the file, list traces, inspect span trees, explain service self time, analyze critical paths, draw an ASCII timeline, detect slow/error/N+1 candidates, explain observable error propagation, and produce script-friendly JSON output.

The project is still early. The current codebase is a local analysis CLI, not a full trace backend.

## Why Developers Reach For It

- **Local-first:** inspect OTLP JSON or JSONL files directly from disk.
- **Explainable:** understand service self time, critical path segments, timeline overlap, concurrency, suspicious timing, and semantic annotations.
- **Proactive triage:** surface slow trace, service latency distribution, error propagation, and N+1 candidates with confidence markers.
- **Automation-friendly:** use `--output json` and `--color never` in scripts, CI, and agent workflows.
- **Conservative semantics:** client/server pairs are annotated, not merged; span links are not converted into parent-child edges.

## Why It Exists

Trace backends such as Jaeger, Tempo, Zipkin, and vendor platforms are powerful, but they usually assume your data has already been ingested somewhere.

During debugging, interviews, CI checks, incident review, offline analysis, or trace handoff, you may only have a local trace export. `tracelens` focuses on that workflow:

```text
trace file -> parse -> normalize -> build graph -> analyze -> report
```

## Guides

- [Why tracelens?](docs/why-tracelens.md)
- [Use cases](docs/use-cases.md)
- [Examples](docs/examples.md)
- [Output guide](docs/output-guide.md)
- [JSON Schema](docs/json-schema.md)
- [OpenTelemetry compatibility](docs/opentelemetry-compatibility.md)
- [Performance](docs/performance.md)

## Current Capabilities

`tracelens` currently supports:

- OTLP JSON input.
- OTLP JSONL input.
- Lenient default parsing with diagnostics.
- Strict validation mode with `--strict`.
- Trace grouping by `trace_id`.
- Parent-child span graph construction.
- Service-level self time analysis.
- Critical path analysis and span execution classification.
- ASCII timeline output for a single trace, including critical path, error, orphan, and overlap markers.
- Detect output for slow trace candidates, service latency distribution, error propagation chains, error-signal candidates, and N+1 candidates.
- Client/server, async work, messaging, and linked span annotations in tree and critical-path output.
- OpenTelemetry metadata preservation for schema URLs, trace state, flags, status messages, dropped counts, and nested attribute values.
- Root span, orphan span, missing parent, duplicate span ID, multiple root, no root, and suspicious timing diagnostics.
- Text output for humans.
- Semantic colored text output with `--color auto|always|never`.
- JSON output for scripts and agents with `--output json`.
- A published JSON Schema for current `--output json` structures.
- Basic trace listing and sorting.

Current commands:

```text
tracelens validate <file>
tracelens summary <file>
tracelens list-traces <file>
tracelens tree <file> --trace-id <id>
tracelens services <file> --trace-id <id>
tracelens critical-path <file> --trace-id <id>
tracelens timeline <file> --trace-id <id>
tracelens detect <file>
```

## Installation

The project does not publish release artifacts yet. For now, install from a local checkout:

```bash
cargo install --path .
```

After installation, verify:

```bash
tracelens --version
tracelens --help
```

You can also run the debug binary without installing:

```bash
cargo build
./target/debug/tracelens --help
```

## Quick Start

Validate an OTLP JSON file:

```bash
tracelens validate tests/fixtures/otlp-basic.json
```

Summarize the file:

```bash
tracelens summary tests/fixtures/otlp-basic.json
```

List traces sorted by duration:

```bash
tracelens list-traces tests/fixtures/otlp-basic.json --limit 10
```

Inspect one trace tree:

```bash
tracelens tree tests/fixtures/otlp-basic.json --trace-id 5B8EFFF798038103D269B633813FC60C
```

Explain service-level self time for one trace:

```bash
tracelens services tests/fixtures/otlp-basic.json --trace-id 5B8EFFF798038103D269B633813FC60C
```

Show the critical path and span execution classification for one trace:

```bash
tracelens critical-path tests/fixtures/otlp-concurrent.json --trace-id CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC
```

Draw an ASCII timeline for one trace:

```bash
tracelens timeline tests/fixtures/otlp-concurrent.json --trace-id CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC
```

Detect slow trace, error, and N+1 candidates:

```bash
tracelens detect tests/fixtures/otlp-n-plus-one.json --limit 3
```

Produce JSON output:

```bash
tracelens detect tests/fixtures/otlp-n-plus-one.json --output json
```

Read the output schema:

```text
schemas/tracelens-output.schema.json
```

Control terminal colors:

```bash
tracelens --color always critical-path tests/fixtures/otlp-concurrent.json --trace-id CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC
tracelens --color never summary tests/fixtures/otlp-basic.json
```

Validate JSONL:

```bash
tracelens validate tests/fixtures/otlp-basic.jsonl
```

Use strict mode:

```bash
tracelens validate tests/fixtures/otlp-basic.json --strict
```

## Supported Input

Currently supported:

| Format | Status | Notes |
| --- | --- | --- |
| OTLP JSON | Supported | `resourceSpans[].scopeSpans[].spans[]` |
| OTLP JSONL | Supported | One OTLP object per line |

See [OpenTelemetry compatibility](docs/opentelemetry-compatibility.md) for the exact supported, partially supported, and unsupported OTLP behaviors.

Not supported yet:

- `.json.gz` compressed input.
- Zipkin JSON.
- Jaeger JSON.
- W3C Trace Context as a standalone trace file.

## Project Status

`tracelens` is in early development.

Implemented:

- Foundation CLI.
- OTLP JSON and JSONL parsing.
- Basic trace graph construction.
- Service-level self time analysis.
- Critical path analysis based on parent-child topology and time intervals.
- ASCII timeline output for trace time structure.
- Serial, concurrent, nested, and suspicious span classification.
- Detect output for slow trace candidates, service latency distribution, error propagation chains, error-signal candidates, and N+1 candidates.
- Client/server span pair annotation.
- Async work, messaging, and linked span annotation.
- Validation diagnostics.
- Semantic colored text output and JSON output.
- JSON Schema for agent and automation consumers.
- OpenTelemetry compatibility documentation.

Not implemented yet:

- ASCII flame graph.
- HTML report.
- Release artifacts for remote download.

See:

- [Project milestones](design/milestones.md)
- [Current progress](design/progress.md)

## Development

Run the standard checks:

```bash
cargo fmt
cargo test
cargo clippy --all-targets -- -D warnings
cargo build
```

Enable the local pre-commit acceptance pipeline once per checkout:

```bash
tools/setup_local_hooks.sh
```

After setup, every `git commit` installs `tracelens` into `.local/tracelens` and runs the local acceptance command suite before the commit is created. You can run it manually with:

```bash
tools/run_local_acceptance.sh
```

See [Local acceptance pipeline](docs/local-acceptance-pipeline.md).

Agent-facing project rules are documented in [AGENTS.md](AGENTS.md).

## License

Apache License 2.0. See [LICENSE](LICENSE).
