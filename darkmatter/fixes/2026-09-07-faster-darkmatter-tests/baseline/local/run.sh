#!/usr/bin/env bash
# One diagnostic warm run per required darkmatter population, serially, from
# the area directory. Each run records wall time, exit code, and load average
# before/after in runs.jsonl beside its verbatim console log. Runs are timed
# with the exact recipe; nothing is built beforehand beyond the listing
# captures, so the first run of a not-yet-built selection carries build time
# in its wall column (the console log separates it: nextest's own Summary
# line is runner elapsed).
set -uo pipefail
R="$(cd "$(dirname "$0")/../../../../.." && pwd)"
L="$R/darkmatter/fixes/2026-09-07-faster-darkmatter-tests/baseline/local"
cd "$R/darkmatter"
now_ms() { node -e 'process.stdout.write(String(Date.now()))'; }
run() {
  local id="$1"; shift
  local log="$L/$id.log"
  local before after start end code
  before="$(sysctl -n vm.loadavg)"
  start="$(now_ms)"
  echo "== $id: $* (start $(date -u +%FT%TZ), load $before)" >&2
  env "$@" > "$log" 2>&1
  code=$?
  end="$(now_ms)"
  after="$(sysctl -n vm.loadavg)"
  echo "== $id: exit $code in $(( (end - start) / 1000 ))s (load $after)" >&2
  printf '{"id":"%s","command":%s,"exitCode":%d,"wallMs":%d,"started":"%s","loadavgBefore":"%s","loadavgAfter":"%s","log":"%s.log"}\n' \
    "$id" "$(printf '%s\n' "$@" | node -e 'const l=require("fs").readFileSync(0,"utf8").trim().split("\n");process.stdout.write(JSON.stringify(l))')" \
    "$code" "$((end - start))" "$(date -u -r $((start/1000)) +%FT%TZ)" "$before" "$after" "$id" >> "$L/runs.jsonl"
}
: > "$L/runs.jsonl"
run sanity just sanity
run test-l1-local just test
run test-l1-include-slow BISCUIT_L1_INCLUDE_SLOW=1 just test
run doctest just doctest
run test-l2 just test-l2
run test-browser just test-browser
run test-l3-optout just test-l3
echo "== all runs done" >&2
