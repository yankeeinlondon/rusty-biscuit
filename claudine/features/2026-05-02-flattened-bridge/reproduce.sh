#!/usr/bin/env bash
#
# Verification script for terminal escape code bleed in non-interactive
# Claudine wrapper sessions.
#
# Usage:
#   ./reproduce.sh
#
# The historical failure leaked OSC 11 color probes such as:
#   ^[]11;rgb:1a1a/1b1b/2626^[
#
# This script runs a direct non-interactive wrapper dry-run in a pseudo-TTY and
# fails if OSC 11 query or response sequences appear in the captured output.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../../.." && pwd)"
OUTPUT_FILE="${SCRIPT_DIR}/reproduce_output.txt"

echo "=== Terminal Escape Code Bleed Verification ==="
echo ""
echo "Building claudine CLI..."
cd "${REPO_ROOT}"
cargo build -p claudine-cli --bin claudine --quiet

echo ""
echo "Running claudine codex dry-run in pseudo-TTY..."
echo "Output: ${OUTPUT_FILE}"
echo ""

if ! command -v script >/dev/null 2>&1; then
    echo "ERROR: 'script' command not found. Cannot create pseudo-TTY."
    echo "Install util-linux (Linux) or use macOS built-in."
    exit 1
fi

if script -q /dev/null echo "test" >/dev/null 2>&1; then
    # macOS style: script <output> <command> [args...]
    script -q "${OUTPUT_FILE}" \
        "${REPO_ROOT}/target/debug/claudine" codex --dry-run "Say hello" 2>/dev/null || true
else
    # Linux style: script -c <command> <output>
    script -q -c "'${REPO_ROOT}/target/debug/claudine' codex --dry-run 'Say hello' 2>/dev/null" \
        "${OUTPUT_FILE}" || true
fi

echo ""
echo "=== Output Analysis ==="
echo ""

if [ ! -f "${OUTPUT_FILE}" ]; then
    echo "ERROR: Output file not created."
    exit 1
fi

read -r QUERY_COUNT RESPONSE_COUNT ESC_COUNT < <(
    python3 - "${OUTPUT_FILE}" <<'PY'
import sys
from pathlib import Path

data = Path(sys.argv[1]).read_bytes()
print(data.count(b"\x1b]11;?"), data.count(b"\x1b]11;rgb:"), data.count(b"\x1b"))
PY
)

echo "OSC 11 queries detected in output: ${QUERY_COUNT}"
echo "OSC 11 responses detected in output: ${RESPONSE_COUNT}"
echo "Total escape sequences in output: ${ESC_COUNT}"
echo ""

if [ "${QUERY_COUNT}" -gt 0 ] || [ "${RESPONSE_COUNT}" -gt 0 ]; then
    echo "FAILED: OSC 11 sequences found in non-interactive wrapper output."
    echo ""
    echo "Sample sequences from output:"
    grep -o $'\x1b]11;[^\x07]*\x07\|\x1b]11;[^\x1b]*\x1b\\\\' "${OUTPUT_FILE}" 2>/dev/null | head -5 || true
    exit 1
fi

echo "PASSED: no OSC 11 sequences detected in non-interactive wrapper output."
echo ""
echo "=== Verification Complete ==="
