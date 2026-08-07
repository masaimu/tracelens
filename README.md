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

It is built for the moments when you have a trace file, not a running trace backend. Give `tracelens` an OTLP JSON or JSONL export, and it helps you validate the file, list traces, inspect span trees, explain service self time, analyze critical paths, detect slow/error candidates, and produce script-friendly JSON output.

The project is still early. The current codebase is a local analysis CLI, not a full trace backend.

## Why Developers Reach For It

- **Local-first:** inspect OTLP JSON or JSONL files directly from disk.
- **Explainable:** understand service self time, critical path segments, concurrency, suspicious timing, and semantic annotations.
- **Proactive triage:** surface slow trace candidates and error-signal candidates with confidence markers.
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
- Detect MVP for slow trace candidates, service candidates, and error-signal candidates.
- Client/server, async work, messaging, and linked span annotations in tree and critical-path output.
- Root span, orphan span, missing parent, duplicate span ID, multiple root, no root, and suspicious timing diagnostics.
- Text output for humans.
- Semantic colored text output with `--color auto|always|never`.
- JSON output for scripts with `--output json`.
- Basic trace listing and sorting.

Current commands:

```text
tracelens validate <file>
tracelens summary <file>
tracelens list-traces <file>
tracelens tree <file> --trace-id <id>
tracelens services <file> --trace-id <id>
tracelens critical-path <file> --trace-id <id>
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

Detect slow trace and error candidates:

```bash
tracelens detect tests/fixtures/otlp-detect.json --limit 3
```

Produce JSON output:

```bash
tracelens detect tests/fixtures/otlp-detect.json --output json
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
- Serial, concurrent, nested, and suspicious span classification.
- Detect MVP for slow trace candidates and error-signal candidates.
- Client/server span pair annotation.
- Async work, messaging, and linked span annotation.
- Validation diagnostics.
- Semantic colored text output and JSON output.

Not implemented yet:

- N+1 detection.
- ASCII timeline or flame graph.
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

Agent-facing project rules are documented in [AGENTS.md](AGENTS.md).

## License

Apache License 2.0. See [LICENSE](LICENSE).
