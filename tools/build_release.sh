#!/usr/bin/env bash
# Build a release artifact plus a sha256 checksum sidecar, named
# tracelens-<version>-<target>.
#
# Cargo strips symbols at build time via [profile.release] strip = "symbols"
# in Cargo.toml, so this script does not depend on a platform `strip` command.
#
# Optional first argument is an explicit rustc target triple. When omitted,
# the current host triple is used. Passing a target enables cross-compiling,
# e.g. building x86_64-apple-darwin on an aarch64 macOS runner, which avoids
# the supply-constrained macos-13 (Intel) runner.

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

TARGET="${1:-$HOST}"

# Make sure the target's std is installed. Idempotent for the host target;
# required when cross-compiling to a non-host triple.
rustup target add "$TARGET"

# Windows MSVC/GNU targets produce a .exe; every other target is a bare binary.
EXE=""
case "$TARGET" in
  *pc-windows*) EXE=".exe" ;;
esac

DIST_DIR="$REPO_ROOT/dist"
ARTIFACT="tracelens-${VERSION}-${TARGET}${EXE}"

echo "building tracelens ${VERSION} for ${TARGET} (host ${HOST})"
cargo build --release --locked --target "$TARGET"

rm -rf "$DIST_DIR"
mkdir -p "$DIST_DIR"
cp "target/${TARGET}/release/tracelens${EXE}" "$DIST_DIR/$ARTIFACT"

# Compute sha256 across macOS (shasum), Linux (sha256sum), and Windows
# (powershell Get-FileHash), writing a portable "<hash>  <basename>" line so
# `shasum -a 256 -c` and `sha256sum -c` both verify the sidecar.
artifact_path="$DIST_DIR/$ARTIFACT"
if command -v shasum >/dev/null 2>&1; then
  hash="$(shasum -a 256 "$artifact_path" | awk '{print $1}')"
elif command -v sha256sum >/dev/null 2>&1; then
  hash="$(sha256sum "$artifact_path" | awk '{print $1}')"
elif command -v pwsh >/dev/null 2>&1; then
  hash="$(pwsh -NoProfile -Command "(Get-FileHash -LiteralPath '$artifact_path' -Algorithm SHA256).Hash.ToLower()" | tr -d '[:space:]')"
elif command -v powershell.exe >/dev/null 2>&1; then
  hash="$(powershell.exe -NoProfile -Command "(Get-FileHash -LiteralPath '$artifact_path' -Algorithm SHA256).Hash.ToLower()" | tr -d '[:space:]')"
else
  echo "error: no sha256 tool found (shasum / sha256sum / pwsh / powershell.exe)" >&2
  exit 1
fi

printf '%s  %s\n' "$hash" "$ARTIFACT" > "$artifact_path.sha256"

echo "artifact: $artifact_path"
if [[ "$TARGET" == "$HOST" ]]; then
  "$artifact_path" --version
else
  echo "(cross target $TARGET != host $HOST: exec self-check skipped, build artifact + checksum only)"
fi
echo "checksum: $artifact_path.sha256"
echo "verify:    ( cd dist && shasum -a 256 -c $ARTIFACT.sha256 )  # or: sha256sum -c $ARTIFACT.sha256"
