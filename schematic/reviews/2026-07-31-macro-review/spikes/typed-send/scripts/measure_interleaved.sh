#!/usr/bin/env bash
# Q4, noise-resistant: interleave configs each round; report min and median.
# Usage: scripts/measure_interleaved.sh <target-dir> [rounds]
set -euo pipefail
unset CDPATH
SPIKE_DIR="$(cd -P -- "$(dirname -- "$0")/.." >/dev/null && pwd)"
cd "$SPIKE_DIR"
[[ -f "$SPIKE_DIR/src/synthetic_new.rs" ]] || { echo "not the spike dir" >&2; exit 1; }
export CARGO_TARGET_DIR="$1"; ROUNDS="${2:-5}"
export RUSTC_WRAPPER="" CARGO_BUILD_RUSTC_WRAPPER="" CARGO_INCREMENTAL=1
now() { python3 -c 'import time;print(time.time())'; }
declare -A R
for r in $(seq "$ROUNDS"); do
  for cfg in "base:" "old:synthetic-old" "new:synthetic-new" "calls:synthetic-new-calls"; do
    name="${cfg%%:*}"; feat="${cfg#*:}"; ff=(); [[ -n "$feat" ]] && ff=(--features "$feat")
    for mode in build check; do
      cargo clean -p typed-send-spike -q
      s=$(now); cargo "$mode" --lib "${ff[@]}" -q; e=$(now)
      R["$name.$mode"]+="$(python3 -c "print(f'{$e-$s:.2f}')") "
    done
  done
  echo "round $r done (load: $(sysctl -n vm.loadavg))" >&2
done
for k in base.build old.build new.build calls.build base.check old.check new.check calls.check; do
  python3 - "$k" ${R[$k]} <<'PY'
import sys, statistics
k, xs = sys.argv[1], sorted(map(float, sys.argv[2:]))
print(f"{k:12} min {xs[0]:6.2f}  median {statistics.median(xs):6.2f}  all {xs}")
PY
done
