#!/usr/bin/env bash
set -euo pipefail
# Unset: one `ci-build produce` over every record this owner holds, as a local
# caller runs it; nothing is uploaded. `record`: one child of the hosted
# coordinator (`scripts/ci/artifacts/produce.cjs`), which passes `--key` and
# owns the owner's timing itself.
case "${PRODUCE_MODE:-}" in
    "" | record) ;;
    *)
        echo "produce-owner.sh: unknown PRODUCE_MODE '$PRODUCE_MODE' (expected unset or 'record')" >&2
        exit 2
        ;;
esac
out="${CI_BUILD_OUT:-$RUNNER_TEMP/build}"
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
        --arg output "$out" \
        '["python3", $script, "--out", $out, "--target", $target, "--output", $output]')")
fi
# All records assigned to this owner share Cargo's target directory, whether
# they are produced in this one invocation or one key per invocation.
start=$(date +%s)
status=0
"${CI_BUILD_BIN:-./target/release/ci-build}" produce \
    --plan "${CI_BUILD_PLAN:-ci-artifacts/ci-resolved-plan/resolved-plan.json}" \
    --producer "$PRODUCER" \
    --out-dir "$out" \
    ${args[@]+"${args[@]}"} "$@" || status=$?
if [ "${MEASURE:-false}" = true ] && [ -z "${PRODUCE_MODE:-}" ]; then
    mkdir -p "$out"
    printf '{"produce_wall_seconds": %s}\n' "$(( $(date +%s) - start ))" > "$out/owner-timing.json"
fi
exit "$status"
