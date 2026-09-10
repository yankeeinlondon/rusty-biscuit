#!/usr/bin/env bash
# Launch-cost probe for Phase 3 decision 1 (fix 2026-09-07-faster-darkmatter-tests).
#
# Times the cargo-built `md` from the two launch directories the CLI tests
# actually use — nextest's ambient CWD (`darkmatter/cli`, inside the monorepo)
# and a directory outside any repository — over the four subcommands that
# account for 315 of the 504 `md` spawn sites by literal-first-token census
# (compose 99, clean 87, hash 66, schema 63), plus `--version` as the pure
# process-launch floor and a document that itself sits inside the checkout
# (the `CARGO_MANIFEST_DIR` fixture shape). Two 16-wide bursts approximate
# nextest's concurrency.
#
# Evidence generator, not a gate: hyperfine aborts on a non-zero exit, so no
# duration recorded here comes from a failed run. Serial timings are
# `--shell=none`; the bursts need a shell for `xargs`.
#
#   attribution/launch-probe.sh      # writes launch-probe-<group>.{json,md} beside itself
#   RUNS=30 attribution/launch-probe.sh
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$HERE/../../../.." && pwd)"
MD="$ROOT/target/debug/md"
PROBE="${PROBE_ROOT:-/tmp/rb-dm-probe-973d1d9}"
DOC_OUT="$PROBE/doc/probe.md"
DOC_IN="$ROOT/target/rb-dm-probe/probe.md"
OUTSIDE="$PROBE/cwd-outside"
PKG="$ROOT/darkmatter/cli"
RUNS="${RUNS:-15}"
export NO_COLOR=1

mkdir -p "$PROBE/doc" "$OUTSIDE" "$(dirname "$DOC_IN")"
cat > "$DOC_OUT" <<'DOC'
---
title: Probe
---

# Probe

A short paragraph with **bold** text and a [link](https://example.com).

- one
- two
DOC
cp "$DOC_OUT" "$DOC_IN"

case "$(cd "$PROBE" && pwd -P)" in
  "$ROOT"/*) echo "probe root $PROBE resolves inside the checkout" >&2; exit 2 ;;
esac
[[ -x "$MD" ]] || { echo "missing $MD (build darkmatter-cli first)" >&2; exit 2; }

{
  echo "revision $(git -C "$ROOT" rev-parse HEAD)"
  echo "md $("$MD" --version)"
  echo "hyperfine $(hyperfine --version)"
  echo "host $(uname -srm)"
  echo "load-before $(uptime | sed 's/.*load averages*: //')"
  echo "runs $RUNS"
} > "$HERE/launch-probe-identity.txt"

serial() { # <group> <cwd> <hyperfine args...>
  local group=$1 cwd=$2; shift 2
  ( cd "$cwd" && hyperfine --warmup 3 --runs "$RUNS" --shell=none \
      --export-json "$HERE/launch-probe-$group.json" \
      --export-markdown "$HERE/launch-probe-$group.md" "$@" )
}

serial outside "$OUTSIDE" \
  -n "version"                 "$MD --version" \
  -n "hash"                    "$MD hash $DOC_OUT" \
  -n "clean"                   "$MD clean $DOC_OUT" \
  -n "compose"                 "$MD compose $DOC_OUT" \
  -n "schema validate"         "$MD schema validate $DOC_OUT" \
  -n "compose doc-in-checkout" "$MD compose $DOC_IN"

serial pkg-cwd "$PKG" \
  -n "hash"                    "$MD hash $DOC_OUT" \
  -n "clean"                   "$MD clean $DOC_OUT" \
  -n "compose"                 "$MD compose $DOC_OUT" \
  -n "schema validate"         "$MD schema validate $DOC_OUT" \
  -n "compose doc-in-checkout" "$MD compose $DOC_IN"

burst() { # <group> <cwd>
  local group=$1 cwd=$2
  ( cd "$cwd" && hyperfine --warmup 1 --runs 5 \
      --export-json "$HERE/launch-probe-burst-$group.json" \
      --export-markdown "$HERE/launch-probe-burst-$group.md" \
      -n "compose x16" "seq 16 | xargs -P 16 -I{} $MD compose $DOC_OUT" \
      -n "hash x16"    "seq 16 | xargs -P 16 -I{} $MD hash $DOC_OUT" )
}

burst outside "$OUTSIDE"
burst pkg-cwd "$PKG"

echo "load-after $(uptime | sed 's/.*load averages*: //')" >> "$HERE/launch-probe-identity.txt"
