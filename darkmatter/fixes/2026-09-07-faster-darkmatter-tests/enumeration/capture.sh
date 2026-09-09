#!/usr/bin/env bash
# Capture `cargo nextest list --message-format json` once per package × feature
# selection the darkmatter recipes and CI legs use. Run from the repo root at
# the recorded revision; stderr is kept beside each JSON as <label>.err.
set -uo pipefail
R="$(cd "$(dirname "$0")/../../../.." && pwd)"
E="$R/darkmatter/fixes/2026-09-07-faster-darkmatter-tests/enumeration"
cd "$R"
capture() {
  local label="$1"; shift
  echo "== $label: cargo nextest list --message-format json $*" >&2
  local start=$SECONDS
  cargo nextest list --message-format json "$@" > "$E/$label.json" 2> "$E/$label.err"
  local code=$?
  echo "== $label exit=$code in $((SECONDS-start))s" >&2
  echo "{\"label\":\"$label\",\"args\":\"$*\",\"exit\":$code,\"seconds\":$((SECONDS-start))}" >> "$E/capture-runs.jsonl"
}
: > "$E/capture-runs.jsonl"
capture l1-local-all -p darkmatter -p darkmatter-cli -p dmls -p zed-dmls-cli
capture lib-bare -p darkmatter
capture cli-bare -p darkmatter-cli
capture dmls-bare -p dmls
capture zed-cli-bare -p zed-dmls-cli
capture lib-terminal -p darkmatter --features terminal-tests
capture cli-terminal -p darkmatter-cli --features terminal-tests
capture dmls-terminal -p dmls --features terminal-tests
capture lib-browser -p darkmatter --features browser-tests
capture lib-terminal-browser -p darkmatter --features terminal-tests,browser-tests
capture lib-effects -p darkmatter --features effects-instrumentation
echo "== all captures done" >&2
