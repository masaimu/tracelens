#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Run the tracelens local acceptance pipeline.

Usage:
  tools/run_local_acceptance.sh [--mode manual|pre-commit]

The pipeline runs quality checks, installs tracelens into .local/tracelens,
and executes the installed binary against representative fixtures.
USAGE
}

MODE="manual"
while [[ $# -gt 0 ]]; do
  case "$1" in
    --mode)
      MODE="${2:-}"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

if [[ "$MODE" != "manual" && "$MODE" != "pre-commit" ]]; then
  echo "--mode must be manual or pre-commit" >&2
  exit 2
fi

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

TIMESTAMP="$(date '+%Y%m%d-%H%M%S')"
RESULTS_DIR="$ROOT_DIR/acceptance-results/$TIMESTAMP"
LOG_DIR="$RESULTS_DIR/logs"
SUMMARY_FILE="$RESULTS_DIR/summary.md"
INSTALL_ROOT="$ROOT_DIR/.local/tracelens"
TRACELENS_BIN="$INSTALL_ROOT/bin/tracelens"
STEP_INDEX=0

mkdir -p "$LOG_DIR"

cat >"$SUMMARY_FILE" <<SUMMARY
# tracelens Local Acceptance Summary

- mode: \`$MODE\`
- created_at: \`$TIMESTAMP\`
- install_root: \`.local/tracelens\`

| step | status | seconds | log |
| --- | --- | ---: | --- |
SUMMARY

slugify() {
  printf '%s' "$1" \
    | tr '[:upper:]' '[:lower:]' \
    | tr -cs 'a-z0-9._-' '-'
}

append_summary() {
  local name="$1"
  local status="$2"
  local seconds="$3"
  local log_name="$4"
  printf '| %s | %s | %s | [log](logs/%s) |\n' "$name" "$status" "$seconds" "$log_name" >>"$SUMMARY_FILE"
}

print_command() {
  printf '$'
  printf ' %q' "$@"
  printf '\n'
}

run_step() {
  local name="$1"
  shift
  STEP_INDEX=$((STEP_INDEX + 1))
  local log_name
  log_name="$(printf '%02d-%s.log' "$STEP_INDEX" "$(slugify "$name")")"
  local log_file="$LOG_DIR/$log_name"
  local start_time
  start_time="$(date +%s)"

  echo
  echo "==> [$STEP_INDEX] $name"
  print_command "$@" | tee "$log_file"

  set +e
  "$@" 2>&1 | tee -a "$log_file"
  local status=${PIPESTATUS[0]}
  set -e

  local end_time
  end_time="$(date +%s)"
  local seconds=$((end_time - start_time))

  if [[ "$status" -eq 0 ]]; then
    append_summary "$name" "pass" "$seconds" "$log_name"
    return 0
  fi

  append_summary "$name" "fail" "$seconds" "$log_name"
  echo
  echo "Local acceptance failed at step: $name"
  echo "Summary: $SUMMARY_FILE"
  exit "$status"
}

run_shell_step() {
  local name="$1"
  local script="$2"
  STEP_INDEX=$((STEP_INDEX + 1))
  local log_name
  log_name="$(printf '%02d-%s.log' "$STEP_INDEX" "$(slugify "$name")")"
  local log_file="$LOG_DIR/$log_name"
  local start_time
  start_time="$(date +%s)"

  echo
  echo "==> [$STEP_INDEX] $name"
  printf '$ %s\n' "$script" | tee "$log_file"

  set +e
  bash -o pipefail -c "$script" 2>&1 | tee -a "$log_file"
  local status=${PIPESTATUS[0]}
  set -e

  local end_time
  end_time="$(date +%s)"
  local seconds=$((end_time - start_time))

  if [[ "$status" -eq 0 ]]; then
    append_summary "$name" "pass" "$seconds" "$log_name"
    return 0
  fi

  append_summary "$name" "fail" "$seconds" "$log_name"
  echo
  echo "Local acceptance failed at step: $name"
  echo "Summary: $SUMMARY_FILE"
  exit "$status"
}

echo "tracelens local acceptance pipeline"
echo "mode: $MODE"
echo "results: $RESULTS_DIR"

run_step "cargo fmt check" cargo fmt --check
run_step "cargo test" cargo test
run_step "cargo clippy" cargo clippy --all-targets -- -D warnings
run_step "cargo build" cargo build
run_step "cargo install local binary" cargo install --path . --force --root "$INSTALL_ROOT"

export TRACELENS_BIN

run_step "installed version" "$TRACELENS_BIN" --version
run_step "schema help" "$TRACELENS_BIN" schema --help
run_step "schema text" "$TRACELENS_BIN" schema --command detect --output text
run_step "validate otlp json" "$TRACELENS_BIN" --color never validate tests/fixtures/otlp-basic.json
run_step "validate otlp jsonl" "$TRACELENS_BIN" --color never validate tests/fixtures/otlp-basic.jsonl
run_shell_step "strict validation exit code" 'set +e; output=$("$TRACELENS_BIN" --color never validate tests/fixtures/otlp-invalid-time.json --strict 2>&1); status=$?; set -e; printf "%s\n" "$output"; test "$status" -eq 1; grep -q "Status: failed" <<<"$output"'
run_shell_step "usage error exit code" 'set +e; output=$("$TRACELENS_BIN" --definitely-invalid 2>&1); status=$?; set -e; printf "%s\n" "$output"; test "$status" -eq 2; grep -q "unexpected argument" <<<"$output"'
run_step "summary" "$TRACELENS_BIN" --color never summary tests/fixtures/otlp-basic.json
run_step "list traces" "$TRACELENS_BIN" --color never list-traces tests/fixtures/otlp-basic.json --limit 2
run_step "tree" "$TRACELENS_BIN" --color never tree tests/fixtures/otlp-basic.json --trace-id 5B8EFFF798038103D269B633813FC60C
run_step "services" "$TRACELENS_BIN" --color never services tests/fixtures/otlp-basic.json --trace-id 5B8EFFF798038103D269B633813FC60C
run_shell_step "tree cross-service edges" '"$TRACELENS_BIN" --color never tree tests/fixtures/otlp-concurrent.json --trace-id CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC 2>&1 | grep -q "跨服务边"'
run_shell_step "services cross-service edges" '"$TRACELENS_BIN" --color never services tests/fixtures/otlp-concurrent.json --trace-id CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC 2>&1 | grep -q "跨服务调用边"'
run_shell_step "tree json cross-service edges" '"$TRACELENS_BIN" --color never tree tests/fixtures/otlp-concurrent.json --trace-id CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC --output json 2>&1 | grep -q cross_service_edges'
run_step "critical path" "$TRACELENS_BIN" --color never critical-path tests/fixtures/otlp-concurrent.json --trace-id CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC
run_step "timeline" "$TRACELENS_BIN" --color never timeline tests/fixtures/otlp-concurrent.json --trace-id CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC
run_step "timeline flame" "$TRACELENS_BIN" --color never timeline tests/fixtures/otlp-concurrent.json --trace-id CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC --mode flame
run_step "timeline collapse" "$TRACELENS_BIN" --color never timeline tests/fixtures/otlp-n-plus-one.json --trace-id 88888888888888888888888888888888 --max-rows 3
run_shell_step "timeline flame json smoke" '"$TRACELENS_BIN" --color always timeline tests/fixtures/otlp-concurrent.json --trace-id CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC --mode flame --output json >/dev/null'
run_step "detect" "$TRACELENS_BIN" --color never detect tests/fixtures/otlp-n-plus-one.json --limit 5

run_shell_step "detect json smoke" '"$TRACELENS_BIN" --color always detect tests/fixtures/otlp-n-plus-one.json --limit 5 --output json >/dev/null'
run_shell_step "timeline json smoke" '"$TRACELENS_BIN" --color always timeline tests/fixtures/otlp-concurrent.json --trace-id CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC --output json >/dev/null'
run_shell_step "schema json smoke" '"$TRACELENS_BIN" schema --output json >/dev/null'
run_shell_step "report html smoke" 'report_dir="$(mktemp -d)"; report_path="$report_dir/report.html"; "$TRACELENS_BIN" --color never report tests/fixtures/otlp-semantic-annotations.json --trace-id DDDDDDDDDDDDDDDDDDDDDDDDDDDDDDDD --html "$report_path" >/dev/null; grep -q "<!DOCTYPE html>" "$report_path"; grep -q "跨服务调用边" "$report_path"; grep -q "frontend-service" "$report_path"; rm -rf "$report_dir"'

cat >>"$SUMMARY_FILE" <<'SUMMARY'

Result: pass
SUMMARY

echo
echo "Local acceptance passed."
echo "Summary: $SUMMARY_FILE"
