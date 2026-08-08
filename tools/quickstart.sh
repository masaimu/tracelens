#!/usr/bin/env bash
# tracelens one-line quickstart.
#
# Real run (downloads the latest release, verifies, and walks every original
# requirement point of the brief):
#   curl -fsSL https://raw.githubusercontent.com/masaimu/tracelens/main/tools/quickstart.sh | bash
#
# Dry run (no network; prints the platform probe, the asset it would fetch, and
# the requirement-point tour it would execute):
#   bash tools/quickstart.sh --dry-run
set -euo pipefail

OWNER="masaimu"
REPO="tracelens"
RAW="https://raw.githubusercontent.com/${OWNER}/${REPO}/main"
API="https://api.github.com/repos/${OWNER}/${REPO}"
UA="tracelens-quickstart"

FIXTURES_BASE="${RAW}/tests/fixtures"
SAMPLE_URL="${RAW}/samples/traces.json"

DRY_RUN=0
for arg in "$@"; do
    case "$arg" in
        --dry-run) DRY_RUN=1 ;;
        -h|--help)
            sed -n '2,14p' "$0"
            exit 0
            ;;
        *)
            echo "unknown argument: $arg" >&2
            exit 2
            ;;
    esac
done

# -------- platform -> release target + file naming --------
OS="$(uname -s)"
ARCH="$(uname -m 2>/dev/null || echo unknown)"
case "$OS" in
    Darwin)
        case "$ARCH" in
            arm64|aarch64) TARGET="aarch64-apple-darwin" ;;
            *) TARGET="x86_64-apple-darwin" ;;
        esac
        OPEN=(open)
        VERIFY=(shasum -a 256 -c)
        ;;
    Linux)
        TARGET="x86_64-unknown-linux-gnu"
        OPEN=(xdg-open)
        VERIFY=(sha256sum -c)
        ;;
    MINGW*|MSYS*|CYGWIN*)
        TARGET="x86_64-pc-windows-msvc"
        EXE=".exe"
        OPEN=(start)
        VERIFY=(sha256sum -c)
        ;;
    *)
        echo "unsupported platform: $OS" >&2
        echo "download artifacts by hand from https://github.com/${OWNER}/${REPO}/releases/latest" >&2
        exit 1
        ;;
esac
EXE="${EXE:-}"
BINARY="tracelens"

# -------- requirement-point tour --------
# Each step prints "original requirement: <point> -> satisfied by <capability>",
# then runs the command. Step 6 also prints a timed wall-clock of `detect`.
tour() {
    local bin="$1"; shift
    local basic="$1"; shift
    local missing="$1"; shift
    local npo="$1"; shift
    local concurrent="$1"; shift
    local samples="$1"; shift

    step() { printf '\n========================================================\n'; echo "$1"; }

    step "1. original requirement: input + scale (OTLP JSON ~5k span) -> tracelens summary"
    "$bin" summary "$samples"

    step "2. original requirement: basic parse (parent_span_id missing / cross-service / orphan) -> validate + tree"
    "$bin" validate "$missing"
    "$bin" tree "$basic" --trace-id 5B8EFFF798038103D269B633813FC60C

    step "3. original requirement: key metrics (end-to-end / critical path / per-service self-time ratio / serial vs concurrent) -> services + critical-path"
    "$bin" services "$basic" --trace-id 5B8EFFF798038103D269B633813FC60C
    "$bin" critical-path "$basic" --trace-id 5B8EFFF798038103D269B633813FC60C

    step "4. original requirement: anomaly detection (slow p99/p999 / error propagation / N+1) -> detect"
    "$bin" detect "$npo" --limit 2
    "$bin" detect "$basic"
    "$bin" detect "$samples" --limit 8

    step "5. original requirement: visualization (single-page HTML report) -> report --html"
    "$bin" report "$concurrent" --trace-id CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC --html tracelens-demo.html

    step "6. original requirement: engineering (subcommand layering / unit tests / sample P95 < 2s) -> --help + timed detect"
    "$bin" --help
    echo "timing detect on the 5k sample set (expect wall-clock well below 2s):"
    /usr/bin/time -p "$bin" detect "$samples" --limit 8 >/dev/null 2>timing.tmp || true
    cat timing.tmp 2>/dev/null || true
    rm -f timing.tmp
    echo "unit tests: clone https://github.com/${OWNER}/${REPO} and run 'cargo test' to replay the suite."

    printf '\nHTML report written to: %s/tracelens-demo.html\n' "$PWD"
    if [ -f tracelens-demo.html ]; then
        "${OPEN[@]}" tracelens-demo.html 2>/dev/null || true
    fi
}

tour_dry() {
    echo "[dry-run] platform: $OS/$ARCH"
    echo "[dry-run] release target: $TARGET"
    echo "[dry-run] asset name pattern: tracelens-<latest-tag>-${TARGET}${EXE} (+ .sha256)"

    step() { printf '\n========================================================\n'; echo "$1"; }
    step "1. original requirement: input + scale (OTLP JSON ~5k span) -> tracelens summary"
    echo "  tracelens summary samples/traces.json"

    step "2. original requirement: basic parse (parent_span_id missing / cross-service / orphan) -> validate + tree"
    echo "  tracelens validate tests/fixtures/otlp-missing-parent.json"
    echo "  tracelens tree tests/fixtures/otlp-basic.json --trace-id 5B8EFFF798038103D269B633813FC60C"

    step "3. original requirement: key metrics (end-to-end / critical path / per-service self-time ratio / serial vs concurrent) -> services + critical-path"
    echo "  tracelens services tests/fixtures/otlp-basic.json --trace-id 5B8EFFF798038103D269B633813FC60C"
    echo "  tracelens critical-path tests/fixtures/otlp-basic.json --trace-id 5B8EFFF798038103D269B633813FC60C"

    step "4. original requirement: anomaly detection (slow p99/p999 / error propagation / N+1) -> detect"
    echo "  tracelens detect tests/fixtures/otlp-n-plus-one.json --limit 2"
    echo "  tracelens detect tests/fixtures/otlp-basic.json"
    echo "  tracelens detect samples/traces.json --limit 8"

    step "5. original requirement: visualization (single-page HTML report) -> report --html"
    echo "  tracelens report tests/fixtures/otlp-concurrent.json --trace-id CCCCCCCCCCCCCCCCCCCCCCCCCCCCCCCC --html tracelens-demo.html"

    step "6. original requirement: engineering (subcommand layering / unit tests / sample P95 < 2s) -> --help + timed detect"
    echo "  tracelens --help"
    echo "  /usr/bin/time -p tracelens detect samples/traces.json --limit 8"
    echo "  clone repo -> cargo test"

    echo
    echo "[dry-run] optional 50k stress: clone repo then"
    echo "  python3 tools/generate_synthetic_traces.py --output big.json --spans 50000 ... ; tracelens detect big.json"
}

if [ "$DRY_RUN" -eq 1 ]; then
    tour_dry
    exit 0
fi

# -------- real run --------
WORK="$(mktemp -d 2>/dev/null || mktemp -d -t tracelens)"
cleanup() { rm -rf "$WORK"; }
trap cleanup EXIT

echo "Fetching latest release metadata..."
JSON="$(curl -fsSL -H "User-Agent: ${UA}" "${API}/releases/latest")"
TAG="$(printf '%s' "$JSON" | grep -oE '"tag_name"[[:space:]]*:[[:space:]]*"[^"]+"' | head -1 | sed -E 's/.*:"([^"]+)"/\1/')"
if [ -z "${TAG:-}" ]; then
    echo "could not determine latest release tag" >&2
    exit 1
fi
echo "Latest release: $TAG"

BIN_NAME="${BINARY}-${TAG}-${TARGET}${EXE}"
SHA_NAME="${BIN_NAME}.sha256"

URLS="$(printf '%s' "$JSON" | grep -oE '"browser_download_url"[[:space:]]*:[[:space:]]*"https://[^"]+"' | sed -E 's/.*"(https:\/\/[^"]+)"/\1/')"
BIN_URL="$(printf '%s\n' "$URLS" | grep -E -- "/${BIN_NAME}\$")"
SHA_URL="$(printf '%s\n' "$URLS" | grep -E -- "/${SHA_NAME}\$")"

if [ -z "${BIN_URL:-}" ] || [ -z "${SHA_URL:-}" ]; then
    echo "could not locate artifact for $TARGET in release $TAG" >&2
    echo "available artifacts:" >&2
    printf '%s\n' "$URLS" >&2
    exit 1
fi

echo "Downloading $BIN_NAME ..."
curl -fsSL -o "$WORK/$BIN_NAME" "$BIN_URL"
curl -fsSL -o "$WORK/$SHA_NAME" "$SHA_URL"

# Make the checksum sidecar point at the bare binary name so -c works inside WORK.
( cd "$WORK" && "${VERIFY[@]}" "$(basename "$SHA_NAME")" )

chmod +x "$WORK/$BIN_NAME"
if [ "$OS" = "Darwin" ]; then
    xattr -d com.apple.quarantine "$WORK/$BIN_NAME" 2>/dev/null || true
fi
BIN="$WORK/$BIN_NAME"
echo "Ready: $($BIN --version)"

echo "Fetching example fixtures + sample dataset..."
basic="$WORK/otlp-basic.json";            curl -fsSL -o "$basic"       "${FIXTURES_BASE}/otlp-basic.json"
missing="$WORK/otlp-missing-parent.json"; curl -fsSL -o "$missing"    "${FIXTURES_BASE}/otlp-missing-parent.json"
npo="$WORK/otlp-n-plus-one.json";         curl -fsSL -o "$npo"        "${FIXTURES_BASE}/otlp-n-plus-one.json"
concurrent="$WORK/otlp-concurrent.json";  curl -fsSL -o "$concurrent" "${FIXTURES_BASE}/otlp-concurrent.json"
samples="$WORK/traces.json";             curl -fsSL -o "$samples"    "$SAMPLE_URL"

tour "$BIN" "$basic" "$missing" "$npo" "$concurrent" "$samples"

echo
echo "Binary location: $BIN"
echo "Re-run any step by re-invoking the binary; uninstall by deleting it."
