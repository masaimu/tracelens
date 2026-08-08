# JSON Schema

`tracelens` publishes a JSON Schema for command output:

```text
schemas/tracelens-output.schema.json
```

The schema is meant for AI agents, CI jobs, scripts, and downstream tools that consume:

```bash
tracelens <command> <file> --output json
```

## Version

Current output schema version:

```json
{
  "schema_version": "0.1"
}
```

`0.1` means the structure is documented and tested, but not yet promised as a stable `1.0` API. Additive fields can appear before `1.0`. Breaking changes must update the schema, tests, and documentation in the same iteration.

## Covered Commands

The schema covers the current JSON output for:

- `validate`
- `summary`
- `list-traces`
- `tree`
- `services`
- `critical-path`
- `timeline`
- `detect`

The top-level `command` field selects the matching schema branch.

## How Agents Should Consume It

Recommended flow:

1. Run the command with `--output json`.
2. Check `schema_version`.
3. Check `command`.
4. Read command-specific sections such as `summary`, `trace`, `nodes`, `services`, `critical_path`, `timeline`, `annotations`, `slow_traces`, or `diagnostics`.
5. Treat unknown fields as forward-compatible additions.

Example:

```bash
tracelens detect tests/fixtures/otlp-n-plus-one.json --output json
```

Useful stable top-level fields:

- `schema_version`
- `command`
- `diagnostics`
- `notes`

Common analysis sections:

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
- `n_plus_one_candidates`

## Validation in This Repository

The CLI test suite compiles `schemas/tracelens-output.schema.json` and validates real JSON output from every supported JSON command.

Run:

```bash
cargo test
```

The local acceptance pipeline also runs `cargo test`, so schema drift is checked before local commits when the hook is enabled:

```bash
tools/setup_local_hooks.sh
tools/run_local_acceptance.sh
```

## Span Metadata

`tree --output json` includes canonical span objects. These now preserve common OpenTelemetry metadata:

- `trace_state`
- `flags`
- `status_message`
- `resource_schema_url`
- `scope_attributes`
- `scope_schema_url`
- `dropped_attributes_count`
- `dropped_events_count`
- `dropped_links_count`
- event `dropped_attributes_count`
- link `trace_state`
- link `flags`
- link `dropped_attributes_count`

Nested OTLP `arrayValue` and `kvlistValue` attributes are preserved as JSON strings inside the current string-based attributes maps. This keeps information available without changing the attribute model before it is stable.

## Compatibility Policy

Before `schema_version` reaches `1.0`:

- Consumers should tolerate additional fields.
- Consumers should branch on `schema_version` and `command`.
- Consumers should not rely on object key ordering.
- Consumers should treat `diagnostics` as part of the result, not as incidental log text.

When a future change needs to break a documented field, the project should update:

- `schemas/tracelens-output.schema.json`
- `docs/json-schema.md`
- `docs/output-guide.md`
- related CLI tests
- `design/progress.md`
- the current iteration document
