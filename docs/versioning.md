# Versioning

`tracelens` follows a simple, deliberate versioning rule during its first release line.

## Where the version comes from

- The single source of truth is `version` in the `[package]` table of `Cargo.toml`.
- `tracelens --version` is rendered by clap from `CARGO_PKG_VERSION`, which cargo fills from that same `Cargo.toml` field.
- Therefore `tracelens --version` always prints `tracelens <Cargo.toml version>`. The CLI test `version_command_reports_pkg_version` pins this invariant: the two must never drift.

To change the version:

1. Bump `version` in `Cargo.toml`.
2. Run `tracelens --version` (or `cargo run -- --version`) and confirm it prints the new value.
3. The CLI test above fails if they drift, so no separate manual reconciliation is needed.

## Pre-1.0 semantics (`0.1.x`)

`tracelens` is currently at `0.1.0`. Pre-`1.0`:

- The `minor` component (the `1` in `0.1.0`) may carry behavior or output-structure changes that are not strictly backward compatible.
- The `patch` component is reserved for fixes and minor, non-breaking improvements.
- The JSON output `schema_version` is **separate** from the crate version. It stays at `0.1` and remains adjustable until the JSON contract is declared stable at `1.0`. See [JSON Schema](json-schema.md).

Do not read `0.1.0` as a mature stable API. It means: a locally usable CLI whose terminal/JSON output is still allowed to evolve before the first `1.0` signal.

## After `1.0`

Once `tracelens` reaches `1.0`:

- The crate version will follow stricter semver: incompatible changes require a major bump.
- The JSON `schema_version` will graduate to `1.0` and then follow its own semver line, decoupled from the crate version.
- Cross-platform release artifacts and GitHub Releases are expected by that point (see [Project milestones](../design/milestones.md)).
