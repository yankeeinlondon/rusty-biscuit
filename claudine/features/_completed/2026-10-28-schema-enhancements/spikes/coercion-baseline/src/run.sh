#!/bin/bash
# Runs the throwaway coercion-baseline recorder. Proxies point at a closed port
# as a belt-and-braces guard against any accidental network request.
set -e
cd /Volumes/coding/personal/rusty-biscuit/.claude/worktrees/agent-a27591efd1c6158e9
OUT=${1:-/private/tmp/claude-501/-Volumes-coding-wt-rusty-biscuit-feat-schema-enhancement/a04574a7-d2f4-4613-8cbc-6d56a98594e7/scratchpad/out}
HTTPS_PROXY=http://127.0.0.1:9 HTTP_PROXY=http://127.0.0.1:9 ALL_PROXY=http://127.0.0.1:9 \
  cargo run -q -p darkmatter --example spike_coercion_baseline -- claudine/docs/schemas/partials/functions.yaml "$OUT" 2>&1 | grep -v "^warning" | tail -3
