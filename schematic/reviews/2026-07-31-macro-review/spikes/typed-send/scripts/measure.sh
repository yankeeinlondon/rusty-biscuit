#!/usr/bin/env bash
# Q4 compile-time measurement. Unwrapped rustc (kache off), own target dir.
# Usage: scripts/measure.sh <target-dir> [runs]
set -euo pipefail
unset CDPATH
SPIKE_DIR="$(cd -P -- "$(dirname -- "$0")/.." >/dev/null && pwd)"
cd "$SPIKE_DIR"
[[ -f "$SPIKE_DIR/src/synthetic_new.rs" ]] || { echo "not the spike dir: $SPIKE_DIR" >&2; exit 1; }
export CARGO_TARGET_DIR="$1"; RUNS="${2:-3}"
export RUSTC_WRAPPER="" CARGO_BUILD_RUSTC_WRAPPER="" CARGO_INCREMENTAL=1
cargo build --lib --features synthetic-old,synthetic-new-calls -q   # deps warm
t() { local s e; s=$(python3 -c 'import time;print(time.time())'); "$@" -q; e=$(python3 -c 'import time;print(time.time())'); python3 -c "print(f'{$e-$s:.2f}')"; }
for cfg in "base:" "old:synthetic-old" "new:synthetic-new" "new+calls:synthetic-new-calls"; do
  name="${cfg%%:*}"; feat="${cfg#*:}"
  file=src/api.rs; [[ "$name" == old ]] && file=src/synthetic_old.rs; [[ "$name" == new* ]] && file=src/synthetic_new.rs
  ff=(); [[ -n "$feat" ]] && ff=(--features "$feat")
  clean=(); check=(); incr=()
  for _ in $(seq "$RUNS"); do
    cargo clean -p typed-send-spike -q
    clean+=("$(t cargo build --lib "${ff[@]}")")
  done
  for _ in $(seq "$RUNS"); do
    cargo clean -p typed-send-spike -q
    check+=("$(t cargo check --lib "${ff[@]}")")
  done
  cargo build --lib "${ff[@]}" -q
  cp "$file" "$file.bak"
  for i in $(seq "$RUNS"); do
    # A one-line body edit (not just a touch): change one endpoint's path.
    python3 - "$file" "$i" <<'PY'
import sys, re
p, i = sys.argv[1], sys.argv[2]
s = open(p).read()
s, n = re.subn(r'/e7(_\d+)?/', f'/e7_{i}/', s, count=1)
if n == 0:
    s, n = re.subn(r'"/v1/chat\d*"', f'"/v1/chat{i}"', s, count=1)
assert n == 1
open(p, 'w').write(s)
PY
    incr+=("$(t cargo build --lib "${ff[@]}")")
  done
  mv "$file.bak" "$file"
  echo "$name | clean build: ${clean[*]} s | clean check: ${check[*]} s | incremental build: ${incr[*]} s"
done
