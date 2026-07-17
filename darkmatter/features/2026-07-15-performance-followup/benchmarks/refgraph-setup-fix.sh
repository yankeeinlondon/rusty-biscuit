#!/usr/bin/env bash
# Bracketed harness for the reference-graph Command-Setup regression.
#
# Measures three pins on the manifest fixtures:
#   base   — the audit commit (the gate's baseline)
#   before — the integrated head (carries the regression)
#   after  — the integrated head + the classify_options baseline-schema fix
#
# Method. `after` is sampled twice per round (after_A / after_B). Those two arms
# are identical code, so their delta is the drift floor the host can resolve; an
# effect smaller than that floor is not established. Rounds are short and every
# pin is sampled in each round, so pooling spreads each pin evenly across the
# whole session — unlike a single long hyperfine block, where a load excursion
# lands entirely inside one arm. That failure mode is why
# `raw/f35-residuals/invalid-run-load46-69/` exists as a negative control.
#
# Usage: refgraph-setup-fix.sh <out-dir> [rounds]
# Expects binaries at /tmp/dmbench/target-{base,before,after}/release/md; build
# them in detached worktrees with isolated CARGO_TARGET_DIR (see summary.md).
set -euo pipefail

OUT="${1:?usage: refgraph-setup-fix.sh <out-dir> [rounds]}"
ROUNDS="${2:-8}"
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$HERE"
mkdir -p "$OUT"

CASES=(compose_trivial compose_schema_transclusion compose_interpolation_heavy compose_transclusion_heavy)
bin() { echo "/tmp/dmbench/target-$1/release/md"; }

for p in base before after; do
  [ -x "$(bin $p)" ] || { echo "missing binary for pin '$p'" >&2; exit 1; }
done

( while true; do echo "$(date +%H:%M:%S) $(uptime | sed 's/.*load averages: //')"; sleep 5; done ) > "$OUT/load.log" 2>&1 &
LPID=$!
trap 'kill $LPID 2>/dev/null || true' EXIT

for round in $(seq 1 "$ROUNDS"); do
  for case in "${CASES[@]}"; do
    NO_COLOR=1 hyperfine --shell=none --warmup 3 --runs 12 --style basic \
      --export-json "$OUT/$case.r$round.json" \
      -n after_A "$(bin after) compose fixtures/$case.md" \
      -n before  "$(bin before) compose fixtures/$case.md" \
      -n base    "$(bin base) compose fixtures/$case.md" \
      -n after_B "$(bin after) compose fixtures/$case.md" >/dev/null 2>&1
  done
  # Controls: render exercises no compose setup; --help exercises no compose
  # code at all, so it isolates fixed process/binary-size cost.
  NO_COLOR=1 hyperfine --shell=none --warmup 3 --runs 12 --style basic \
    --export-json "$OUT/render_basic.r$round.json" \
    -n after_A "$(bin after) render fixtures/render_basic.md" \
    -n before  "$(bin before) render fixtures/render_basic.md" \
    -n base    "$(bin base) render fixtures/render_basic.md" \
    -n after_B "$(bin after) render fixtures/render_basic.md" >/dev/null 2>&1
  NO_COLOR=1 hyperfine --shell=none --warmup 3 --runs 12 --style basic \
    --export-json "$OUT/help.r$round.json" \
    -n after_A "$(bin after) --help" \
    -n before  "$(bin before) --help" \
    -n base    "$(bin base) --help" \
    -n after_B "$(bin after) --help" >/dev/null 2>&1
  echo "round $round/$ROUNDS done"
done

echo "raw vectors in $OUT; analyze with: bun benchmarks/refgraph-setup-fix-report.ts $OUT"
