#!/usr/bin/env bash
# Alternating before/after measurement driver for review-1 of
# fixes/2026-09-07-faster-sniff-tests.
#
# Usage: run.sh <run-id> <tree-label> <tree-path> <cohort>
#
# Records, for one measured run: sustained CPU idle before, load averages
# before/after, `/usr/bin/time -p` wall clock, and full stdout+stderr.

set -uo pipefail

RUN_ID="$1"
TREE_LABEL="$2"
TREE="$3"
COHORT="$4"

OUT=/Users/ken/.claudine/worktrees/rusty-biscuit/fix-cli-slow-tests/sniff/fixes/2026-09-07-faster-sniff-tests/measurement/review1-alternating
LOG="$OUT/run-${RUN_ID}-${TREE_LABEL}-${COHORT}.log"
JSONL="$OUT/runs.jsonl"

top_idle() {
    /usr/bin/top -l 2 -s 3 -n 0 2>/dev/null | grep "CPU usage" | tail -1
}

load_avg() {
    uptime | sed 's/.*load averages*: //'
}

IDLE_BEFORE="$(top_idle)"
LOAD_BEFORE="$(load_avg)"
STARTED="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

{
    echo "### run_id=${RUN_ID} tree=${TREE_LABEL} cohort=${COHORT}"
    echo "### tree_path=${TREE}"
    echo "### started_utc=${STARTED}"
    echo "### top_idle_before=${IDLE_BEFORE}"
    echo "### load_before=${LOAD_BEFORE}"
    echo "### command=just --justfile ${TREE}/sniff/justfile --working-directory ${TREE}/sniff ${COHORT}"
    echo "###"
} > "$LOG"

TIMEFILE="$(mktemp)"
/usr/bin/time -p just --justfile "${TREE}/sniff/justfile" \
    --working-directory "${TREE}/sniff" "${COHORT}" \
    >> "$LOG" 2>&1
EXIT=$?

# /usr/bin/time -p writes to stderr, which is already in $LOG.
rm -f "$TIMEFILE"

LOAD_AFTER="$(load_avg)"
ENDED="$(date -u +%Y-%m-%dT%H:%M:%SZ)"
REAL="$(grep -E '^real ' "$LOG" | tail -1 | awk '{print $2}')"
SUMMARY="$(grep -E '^ *Summary \[' "$LOG" | sed 's/^ *//' | paste -sd '|' -)"
SUMMARIES="$(grep -cE '^ *Summary \[' "$LOG")"
COMPILED="$(grep -cE '^ *Compiling ' "$LOG")"

{
    echo "###"
    echo "### ended_utc=${ENDED}"
    echo "### load_after=${LOAD_AFTER}"
    echo "### exit_code=${EXIT}"
    echo "### wall_real_seconds=${REAL}"
    echo "### compiling_lines=${COMPILED}"
} >> "$LOG"

printf '{"run_id":"%s","tree":"%s","cohort":"%s","started_utc":"%s","ended_utc":"%s","wall_real_s":%s,"exit_code":%d,"compiling_lines":%d,"summary_lines":%d,"summary":"%s","top_idle_before":"%s","load_before":"%s","load_after":"%s"}\n' \
    "$RUN_ID" "$TREE_LABEL" "$COHORT" "$STARTED" "$ENDED" "${REAL:-null}" "$EXIT" \
    "$COMPILED" "$SUMMARIES" \
    "$(echo "$SUMMARY" | sed 's/"/\\"/g')" \
    "$(echo "$IDLE_BEFORE" | sed 's/"/\\"/g')" "$LOAD_BEFORE" "$LOAD_AFTER" \
    >> "$JSONL"

echo "[${RUN_ID} ${TREE_LABEL} ${COHORT}] real=${REAL}s exit=${EXIT} compiling=${COMPILED} :: ${SUMMARY}"
exit 0
