<p align="center">
  <img src="assets/logo.svg" alt="tracelens logo" width="160" />
</p>

<h1 align="center">tracelens</h1>

<p align="center">
  A local-first CLI for inspecting OpenTelemetry traces.
</p>

<p align="center">
  <a href="README.zh-CN.md">中文</a>
</p>

<p align="center">
  <a href="https://github.com/masaimu/tracelens/actions/workflows/ci.yml">
    <img src="https://github.com/masaimu/tracelens/actions/workflows/ci.yml/badge.svg" alt="CI status" />
  </a>
</p>

## What Is tracelens?

`tracelens` is a command-line tool for exploring OpenTelemetry trace exports on your local machine.

It is built for the moments when you have a trace file, not a running trace backend. Give `tracelens` an OTLP JSON or JSONL export, and it helps you validate the file, list traces, inspect span trees, and produce script-friendly JSON output.

The project is still early. The current codebase is a local analysis CLI, not a full trace backend.

## Why It Exists

Trace backends such as Jaeger, Tempo, Zipkin, and vendor platforms are powerful, but they usually assume your data has already been ingested somewhere.

During debugging, interviews, CI checks, incident review, offline analysis, or trace handoff, you may only have a local trace export. `tracelens` focuses on that workflow:

```text
trace file -> parse -> normalize -> build graph -> analyze -> report
```

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

Produce JSON output:

```bash
tracelens summary tests/fixtures/otlp-basic.json --output json
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
- Client/server span pair annotation.
- Async work, messaging, and linked span annotation.
- Validation diagnostics.
- Semantic colored text output and JSON output.

Not implemented yet:

- Slow request detection.
- Error propagation analysis.
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
