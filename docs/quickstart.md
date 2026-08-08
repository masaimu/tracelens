# Quickstart

The single fastest way to experience every original capability of `tracelens` is the one-line quickstart.

## One-line quickstart

```bash
curl -fsSL https://raw.githubusercontent.com/masaimu/tracelens/main/tools/quickstart.sh | bash
```

This script:

- detects your platform and selects the matching release binary (macOS arm64/x86_64, Linux x86_64, Windows git-bash),
- downloads the latest release and its `.sha256` sidecar and verifies the checksum,
- on macOS clears the Gatekeeper quarantine attribute,
- pulls the committed sample dataset `samples/traces.json` plus a few small fixtures,
- walks one command per original requirement point of the brief,
- ends by opening `tracelens-demo.html`, the single-page HTML report.

## Requirement-point tour

Each tour step prints the original requirement it answers, then runs the command.

| # | Original requirement | Command |
|---|---|---|
| 1 | input + scale (OTLP JSON ~5k span) | `tracelens summary samples/traces.json` |
| 2 | basic parse (missing parent / cross-service / orphan) | `tracelens validate`, `tracelens tree` |
| 3 | key metrics (duration / critical path / self-time ratio / serial vs concurrent) | `tracelens services`, `tracelens critical-path` |
| 4 | anomaly detection (slow p99/p999 / error propagation / N+1) | `tracelens detect` |
| 5 | visualization (single-page HTML report) | `tracelens report --html` |
| 6 | engineering (subcommands / unit tests / sample P95 < 2s) | `tracelens --help`, timed `detect` |

## Dry run

No network, no download — print exactly what the script would do:

```bash
bash tools/quickstart.sh --dry-run
```

## Samples dataset

`samples/traces.json` is the deterministic ~5k-span / 8-service sample committed in the repository, which the original brief asked for as `traces.json`. It is also what makes `detect` show real (non-null) `p99`/`p999` values, since each service carries ~625 spans.

## Platform notes

- macOS (arm64 and x86_64), Linux x86_64, and Windows under git-bash are supported.
- A native PowerShell quickstart is not provided in this line; Windows users can still run the git-bash path or download artifacts directly from [Releases](https://github.com/masaimu/tracelens/releases/latest).
- Optional 50k stress: clone the repo and run `python3 tools/generate_synthetic_traces.py --output big.json --spans 50000 ...` then `tracelens detect big.json`.
