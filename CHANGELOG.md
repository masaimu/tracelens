# Changelog

This file records user-visible changes to `tracelens`. It is the source for release notes once GitHub Releases are wired up. Versions follow the rule in [docs/versioning.md](docs/versioning.md).

The JSON output `schema_version` is tracked separately and remains at `0.1` until the JSON contract is declared stable at `1.0`.

## 0.1.0 (unreleased — pending GitHub Releases workflow)

Local OpenTelemetry trace analysis CLI. The first focused release line covers everything needed to understand a single exported trace file on your machine.

### Capabilities

#### M0 — project skeleton
- Rust CLI project that builds and runs; design documents and range-control mechanism in place.

#### M1 — OTLP input parsing
- OTLP JSON and JSONL input.
- Canonical span model with OpenTelemetry metadata preserved (schema URL, trace state, flags, status message, dropped counts, scope attributes, nested `arrayValue` / `kvlistValue`).
- Lenient default parsing with diagnostics plus `--strict` validation mode.
- Trace/span ID case normalization and all-zero ID diagnostics.
- Timestamp string and numeric form compatibility.
- Validated on 5k and 50k spans synthetic datasets.

#### M2 — trace index and graph
- Trace grouping by `trace_id`, parent-child edges, root/orphan/duplicate/missing-parent and timing diagnostics.
- Cross-service edge aggregation in `tree` and `services` output (one aggregated edge per parent_service → child_service direction, with call count and client/server pair count).

#### M3 — core CLI analysis commands
- `validate`, `summary`, `list-traces`, `tree`, `services` commands.
- `--output json` with `schema_version: "0.1"`.
- `tracelens schema` CLI-discoverable, description-backed JSON Schema for agents and automation.

#### M4 — duration analysis and critical path
- Wall-clock duration and unique root span duration.
- Span self time with child interval union to avoid double counting.
- Critical path analysis based on parent-child topology and time intervals; duplicate span IDs do not incorrectly collapse.
- Serial / concurrent / nested / suspicious span classification.
- Client/server, async work, messaging, and linked span annotations.

#### M5 — pattern detection
- `detect`: slow trace candidates, service latency distribution, error-signal candidates, error propagation chains, and N+1 candidates with confidence markers.
- Conservative N+1 heuristics (repeated >= 5 medium, >= 10 and mostly serial high confidence).

#### M6 — terminal visualization
- Semantic colored terminal output with `--color auto|always|never`.
- ASCII `timeline` in two layouts: horizontal time bar (`--mode bar`) and vertical flame view (`--mode flame`), with `--max-rows` folding for large traces.

#### M7 — performance, stability, and automation interface
- Exit-code contract (`0` success, `1` business/input failure, `2` clap usage error) and end-to-end tests.
- GitHub Actions CI, security checks, automatic and manual performance benchmark workflows with Actions summaries.
- Local acceptance pipeline and pre-commit hook.
- Synthetic fixture generator and benchmark runner; 5k/50k spans JSON/JSONL smoke and 50k `detect` P95 benchmark.

#### M8 — single-page HTML report
- `report <file> --trace-id <id> --html out.html` generates a single-page offline HTML report.
- Renders trace overview, service timing, critical path, cross-service edges, error propagation chains, N+1 candidates, and diagnostics.
- Color heat mapping for slow services, error spans, high-count N+1 edges, and diagnostic severity; in-report anchor navigation.

#### M9 — release and distribution (in progress)
- `tracelens --version` aligns with `Cargo.toml` (`0.1.0`).
- `tools/build_release.sh` produces a local stripped binary plus a sha256 checksum for the current host.
- Installation documentation, comparison document, and this changelog.
- Remote download from GitHub Releases and cross-platform artifacts are pending.

### Known limits in 0.1.0
- No remote download of prebuilt binaries yet; local artifact build or `cargo install` only.
- No cross-platform prebuilt artifacts (linux/windows/mac x86_64) yet.
- JSON `schema_version` is still `0.1`, not stable at `1.0`.
- No multi-day latency trends, live tailing, or trace backend ingestion.
