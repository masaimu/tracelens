#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

git config core.hooksPath .githooks
chmod +x .githooks/pre-commit tools/run_local_acceptance.sh tools/setup_local_hooks.sh

echo "Configured git core.hooksPath=.githooks"
echo "pre-commit will run tools/run_local_acceptance.sh before each local commit."
