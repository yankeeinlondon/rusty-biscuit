#!/usr/bin/env bash
set -euo pipefail
args=()
if [ "${MEASURE:-false}" = true ]; then
    args+=(--counter-dir "$RUNNER_TEMP/build-counters")
fi
# The hosted job names a diagnostics directory on Ubuntu only; local callers,
# including the real-Cargo fixture, leave it unset and sample nothing.
if [ -n "${BUILD_DIAGNOSTICS:-}" ]; then
    here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
    args+=(--sample-command "$(jq -nc \
        --arg script "$here/storage_sample.py" \
        --arg out "$BUILD_DIAGNOSTICS/storage.jsonl" \
        --arg target "${CARGO_TARGET_DIR:-$PWD/target}" \
        --arg output "$RUNNER_TEMP/build" \
        '["python3", $script, "--out", $out, "--target", $target, "--output", $output]')")
fi
# All records assigned to this owner share Cargo's target directory. This
# entry point is also exercised by the real-Cargo shared-dependency fixture.
start=$(date +%s)
status=0
"${CI_BUILD_BIN:-./target/release/ci-build}" produce \
    --plan "${CI_BUILD_PLAN:-ci-artifacts/ci-resolved-plan/resolved-plan.json}" \
    --producer "$PRODUCER" \
    --out-dir "$RUNNER_TEMP/build" \
    ${args[@]+"${args[@]}"} "$@" || status=$?
if [ "${MEASURE:-false}" = true ]; then
    mkdir -p "$RUNNER_TEMP/build"
    printf '{"produce_wall_seconds": %s}\n' "$(( $(date +%s) - start ))" > "$RUNNER_TEMP/build/owner-timing.json"
fi
exit "$status"
