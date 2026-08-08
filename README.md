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
- **Automation-friendly:** use `--output json`, `--color never`, and `tracelens schema` in scripts, CI, and agent workflows.
- **Conservative semantics:** client/server pairs are annotated, not merged; span links are not converted into parent-child edges.

## Why It Exists

Trace backends such as Jaeger, Tempo, Zipkin, and vendor platforms are powerful, but they usually assume your data has already been ingested somewhere.

During debugging, recorded walkthroughs, CI checks, incident review, offline analysis, or trace handoff, you may only have a local trace export. `tracelens` focuses on that workflow:

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
- [CI integration](docs/ci-integration.md)
- [Comparison](docs/comparison.md)

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
- ASCII timeline output for a single trace, including critical path, error, orphan, and overlap markers, with two layouts: a horizontal time bar (`--mode bar`, default) and a vertical flame view (`--mode flame`). Large traces can be folded with `--max-rows` so the terminal stays readable.
- Detect output for slow trace candidates, service latency distribution, error propagation chains, error-signal candidates, and N+1 candidates.
- Client/server, async work, messaging, and linked span annotations in tree and critical-path output.
- Cross-service edge summary in `tree` and `services` output: one aggregated edge per parent_service → child_service direction, with call count and client/server pair count.
- Single-page offline HTML report for one trace via `report <file> --trace-id <id> --html out.html`; the report reuses the services / critical-path / tree / detect analysis and renders trace overview, service timing, critical path, cross-service edges, error propagation chains, N+1 candidates, and diagnostics, with color heat mapping for slow services, error spans, high-count N+1 edges, and diagnostic severity.
- OpenTelemetry metadata preservation for schema URLs, trace state, flags, status messages, dropped counts, and nested attribute values.
- Root span, orphan span, missing parent, duplicate span ID, multiple root, no root, and suspicious timing diagnostics.
- Text output for humans.
- Semantic colored text output with `--color auto|always|never`.
- JSON output for scripts and agents with `--output json`.
- A published JSON Schema and CLI-discoverable field reference for current `--output json` structures.
- A documented exit-code contract for CI and automation.
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
tracelens report <file> --trace-id <id> --html out.html
tracelens schema
```

## Installation

`tracelens` ships prebuilt binaries per platform. Pick the path that fits you.

### From GitHub Releases

Each version tag (for example `v0.1.0`) publishes prebuilt binaries to GitHub Releases for macOS arm64/x86_64, Linux x86_64, and Windows x86_64. **Until the first tagged release is published**, use one of the local build paths below.

Once a tagged release exists:

1. Download `tracelens-<version>-<host>` for your platform (Windows: the `.exe`) and the matching `.sha256`.
2. Verify the checksum (macOS): `shasum -a 256 -c tracelens-<version>-<host>.sha256` (Linux: `sha256sum -c`; Windows PowerShell: compare `Get-FileHash -Algorithm SHA256`).
3. Run: `./tracelens --version` (Windows: `tracelens.exe --version`).

### Build a local release artifact

A stripped binary plus a sha256 checksum for the current host:

```bash
tools/build_release.sh
```

It prints the artifact path and produces `dist/tracelens-0.1.0-<host>` with a matching `.sha256`. Verify and run it:

```bash
( cd dist && shasum -a 256 -c *.sha256 )
./dist/tracelens-0.1.0-<host> --version
```

### Install from source

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

See [Versioning](docs/versioning.md) for the version rule and tag naming convention.

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

Draw a vertical flame view of the same trace, or fold a large trace:

```bash
tracelens timeline tests/fixtures/otlp-concurrent.json --trace-id CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC --mode flame
tracelens timeline tests/fixtures/otlp-n-plus-one.json --trace-id 88888888888888888888888888888888 --max-rows 10
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

```bash
tracelens schema --output text
tracelens schema --output json
tracelens schema --command detect --output text
```

Generate a single-page offline HTML report for a trace:

```bash
tracelens report tests/fixtures/otlp-concurrent.json --trace-id CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC --html report.html
```

The report renders the trace overview, service timing, critical path, and cross-service edges in one offline HTML file you can open in a browser.

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

Use in CI:

```bash
tracelens --color never validate traces.json --strict
tracelens detect traces.json --limit 5 --output json > tracelens-detect.json
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
- ASCII timeline output for trace time structure, in horizontal bar and vertical flame layouts, with large-trace folding.
- Cross-service edge aggregation in `tree` and `services` output.
- Single-page offline HTML report (trace overview, service timing, critical path, cross-service edges).
- Serial, concurrent, nested, and suspicious span classification.
- Detect output for slow trace candidates, service latency distribution, error propagation chains, error-signal candidates, and N+1 candidates.
- Client/server span pair annotation.
- Async work, messaging, and linked span annotation.
- Validation diagnostics.
- Semantic colored text output and JSON output.
- JSON Schema and CLI-discoverable field descriptions for agent and automation consumers.
- OpenTelemetry compatibility documentation.
- Exit-code and CI integration documentation.
- Cross-platform release workflow (`.github/workflows/release.yml`): a version tag publishes prebuilt binaries with sha256 checksums to GitHub Releases for macOS arm64/x86_64, Linux x86_64, and Windows x86_64.
- Local release artifact build script (`tools/build_release.sh`, also used by CI) producing a stripped binary and a sha256 checksum for the current host.

Not implemented yet:

- Package-manager distribution (Homebrew tap, crates.io, npm wrapper) is not provided yet. GitHub Releases is the distribution channel.

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
