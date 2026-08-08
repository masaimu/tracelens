#!/usr/bin/env bash
# Build a local release artifact for the current host target: a stripped
# `tracelens` binary plus a sha256 checksum sidecar.
#
# Cross-platform artifacts (linux/windows/mac x86_64) and remote publishing to
# GitHub Releases are produced by CI in a later iteration. This script only
# reproduces the macOS arm64 (current host) artifact locally.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

if [[ ! -f Cargo.toml ]]; then
  echo "error: Cargo.toml not found at $REPO_ROOT" >&2
  exit 1
fi

VERSION="$(grep -E '^version[[:space:]]*=' Cargo.toml | head -1 | sed -E 's/.*"([^"]+)".*/\1/')"
if [[ -z "${VERSION:-}" ]]; then
  echo "error: could not read package version from Cargo.toml" >&2
  exit 1
fi

HOST="$(rustc -vV | sed -n 's/^host: //p')"
if [[ -z "${HOST:-}" ]]; then
  echo "error: could not detect rustc host target" >&2
  exit 1
fi

DIST_DIR="$REPO_ROOT/dist"
ARTIFACT="tracelens-${VERSION}-${HOST}"
ARTIFACT_PATH="$DIST_DIR/$ARTIFACT"

echo "building tracelens ${VERSION} for ${HOST}"
cargo build --release

rm -rf "$DIST_DIR"
mkdir -p "$DIST_DIR"
cp target/release/tracelens "$ARTIFACT_PATH"

if command -v strip >/dev/null 2>&1; then
  strip "$ARTIFACT_PATH" || echo "warn: strip failed, keeping unstripped binary"
else
  echo "warn: strip not found, keeping unstripped binary"
fi

( cd "$DIST_DIR" && shasum -a 256 "$ARTIFACT" > "$ARTIFACT.sha256" )

echo "artifact: $ARTIFACT_PATH"
"$ARTIFACT_PATH" --version
echo "checksum: $DIST_DIR/$ARTIFACT.sha256"
echo "verify:    ( cd dist && shasum -a 256 -c $ARTIFACT.sha256 )"
