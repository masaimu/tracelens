# CI Integration

This guide explains how to use `tracelens` in CI jobs, scripts, and agent workflows.

The recommended CI posture is:

- use `--color never` for log-friendly text output
- use `--output json` for machine-readable results
- use `validate --strict` when bad trace input should fail the job
- use `tracelens schema --output text|json` when an agent or script needs field meanings

## Exit Codes

`tracelens` uses a small exit-code contract:

| code | meaning | typical cases |
| ---: | --- | --- |
| `0` | success | command completed and output is usable |
| `1` | failure | input failure, strict validation failure, invalid analysis request, or unmet analysis precondition |
| `2` | usage error | invalid CLI syntax, unknown option, missing required argument, or invalid enum value |

`2` follows clap's default CLI parsing behavior.

Important distinction:

- `detect` finding slow/error/N+1 candidates still exits `0`; candidates are analysis results.
- `critical-path` returning `status: "unavailable"` for a valid trace can still exit `0`; unavailability is part of the analysis result.
- `validate` default mode can exit `0` while reporting diagnostics; use `--strict` when diagnostics should block CI.

## Validate Trace Files

Use strict validation when malformed input should fail the pipeline:

```bash
tracelens --color never validate traces.json --strict
```

For machine-readable validation:

```bash
tracelens validate traces.json --strict --output json > tracelens-validate.json
```

When strict validation fails, the command exits `1` and the JSON payload reports:

```json
{
  "status": "failed",
  "exit_would_fail": true,
  "error_diagnostic_count": 1
}
```

Default validation is useful when CI should collect diagnostics without blocking:

```bash
tracelens validate traces.json --output json > tracelens-validate.json
```

In default mode, diagnostics can be present while the command still exits `0`.

## Analyze Without ANSI Color

For CI logs:

```bash
tracelens --color never summary traces.json
tracelens --color never detect traces.json --limit 5
```

For JSON:

```bash
tracelens detect traces.json --limit 5 --output json > tracelens-detect.json
```

JSON output never includes ANSI color escapes, even if `--color always` is set.

## Read the Output Contract

Agents and scripts should not guess field meanings. Use the installed CLI:

```bash
tracelens schema --output text
tracelens schema --command detect --output text
tracelens schema --output json
```

Suggested agent flow:

1. Run `tracelens --help`.
2. Discover the `schema` command.
3. Run `tracelens schema --output text` to read field meanings.
4. Run the target command with `--output json`.
5. Branch on `schema_version` and `command`.
6. Treat `diagnostics` and `notes` as caveats.

## Example GitHub Actions Step

```yaml
- name: Validate trace export
  run: |
    tracelens --color never validate traces.json --strict

- name: Detect trace candidates
  run: |
    tracelens detect traces.json --limit 5 --output json > tracelens-detect.json
```

If the first step exits `1`, the job fails. The second step exits `0` when candidate analysis completes successfully, even if candidates are found.

## Shell Pattern for Optional Diagnostics

Use this when diagnostics should be collected but not fail the job:

```bash
tracelens validate traces.json --output json > tracelens-validate.json
jq '.diagnostics' tracelens-validate.json
```

Use this when strict validation should fail the job:

```bash
tracelens validate traces.json --strict --output json > tracelens-validate.json
```

If you need to inspect the failed JSON while preserving the exit code:

```bash
set +e
tracelens validate traces.json --strict --output json > tracelens-validate.json
status=$?
set -e

cat tracelens-validate.json
exit "$status"
```
