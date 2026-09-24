#!/usr/bin/env bash
# Phase 6 final sweep: the same recipes as `baseline/pre-existing.sh`, on the
# migrated tree, so the two summaries compare row for row.
# One log per {area, recipe}; a summary line per run in summary.tsv.
set -uo pipefail
repo=$(git rev-parse --show-toplevel)
out="$repo/features/2026-09-22-consolidated-test-binaries-wave-2/acceptance/final-sweep-logs"
mkdir -p "$out"
: > "$out/summary.tsv"
printf 'rev\t%s\n' "$(git -C "$repo" rev-parse HEAD)" >> "$out/summary.tsv"
export INSTA_UPDATE=no
areas=(tree-hugger claudine sniff biscuit-file schematic biscuit-terminal darkmatter biscuit-tui)
for area in "${areas[@]}"; do
    for recipe in test test-l2 lint; do
        start=$(date +%s)
        (cd "$repo/$area" && just "$recipe") > "$out/$area.$recipe.log" 2>&1 < /dev/null
        code=$?
        printf '%s\t%s\t%s\t%ss\n' "$area" "$recipe" "$code" "$(( $(date +%s) - start ))" >> "$out/summary.tsv"
    done
    start=$(date +%s)
    (cd "$repo" && just check-tier-coverage "$area") > "$out/$area.check-tier-coverage.log" 2>&1 < /dev/null
    code=$?
    printf '%s\t%s\t%s\t%ss\n' "$area" "check-tier-coverage" "$code" "$(( $(date +%s) - start ))" >> "$out/summary.tsv"
done
(cd "$repo" && just check-canonical "${areas[*]}") > "$out/all.check-canonical.log" 2>&1 < /dev/null
printf '%s\t%s\t%s\n' "all" "check-canonical" "$?" >> "$out/summary.tsv"
echo done >> "$out/summary.tsv"
