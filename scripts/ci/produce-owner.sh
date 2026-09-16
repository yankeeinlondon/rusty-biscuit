#!/usr/bin/env bash
set -euo pipefail
args=()
if [ "${MEASURE:-false}" = true ]; then
    args+=(--counter-dir "$RUNNER_TEMP/build-counters")
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
