# Local Acceptance Pipeline

This guide explains the local acceptance pipeline that should run before committing `tracelens` changes.

The goal is to verify the installed CLI, not only the source tree. The pipeline builds the project, installs `tracelens` into a project-local directory, then runs a representative command suite against real fixtures.

## Quick Start

Enable the local Git hook once per checkout:

```bash
tools/setup_local_hooks.sh
```

After that, every local commit runs:

```bash
tools/run_local_acceptance.sh --mode pre-commit
```

You can also run the pipeline manually:

```bash
tools/run_local_acceptance.sh
```

## What It Runs

The pipeline runs:

```text
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
cargo build
cargo install --path . --force --root .local/tracelens
```

Then it executes the installed binary:

```text
.local/tracelens/bin/tracelens --version
.local/tracelens/bin/tracelens validate tests/fixtures/otlp-basic.json
.local/tracelens/bin/tracelens validate tests/fixtures/otlp-basic.jsonl
.local/tracelens/bin/tracelens summary tests/fixtures/otlp-basic.json
.local/tracelens/bin/tracelens list-traces tests/fixtures/otlp-basic.json --limit 2
.local/tracelens/bin/tracelens tree tests/fixtures/otlp-basic.json --trace-id 5B8EFFF798038103D269B633813FC60C
.local/tracelens/bin/tracelens services tests/fixtures/otlp-basic.json --trace-id 5B8EFFF798038103D269B633813FC60C
.local/tracelens/bin/tracelens critical-path tests/fixtures/otlp-concurrent.json --trace-id CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC
.local/tracelens/bin/tracelens timeline tests/fixtures/otlp-concurrent.json --trace-id CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC
.local/tracelens/bin/tracelens detect tests/fixtures/otlp-n-plus-one.json --limit 5
```

It also runs JSON smoke checks for `detect` and `timeline`.

`cargo test` includes JSON Schema validation for every supported `--output json` command, so schema drift is covered by the same local pipeline.

## Output

Each run writes:

```text
acceptance-results/<timestamp>/
  summary.md
  logs/
```

The command output is shown in the terminal and captured in logs. This makes the pipeline useful both for automated pre-commit checks and for manual local review.

These directories are ignored by Git:

```text
.local/
acceptance-results/
```

## Why Setup Is Required

Git does not automatically enable hooks from a repository after clone. This is a security boundary: a repository should not be able to make arbitrary local scripts run automatically without an explicit local opt-in.

That means each developer must run:

```bash
tools/setup_local_hooks.sh
```

This command configures:

```bash
git config core.hooksPath .githooks
```

Once configured, `.githooks/pre-commit` runs the acceptance pipeline before each local commit.

## Failure Behavior

If any step fails:

- the pipeline exits nonzero
- the pre-commit hook fails
- the commit is blocked
- the failure log is available under `acceptance-results/<timestamp>/logs/`

Fix the issue and commit again.

## Agent Rule

Agents must not rely on a hook being active. Before committing, an Agent must either:

- confirm the hook is enabled and let `git commit` trigger it, or
- run `tools/run_local_acceptance.sh` manually.

The implementation report must mention whether the local acceptance pipeline passed.
