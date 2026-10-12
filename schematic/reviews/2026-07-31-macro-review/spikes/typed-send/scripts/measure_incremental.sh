#!/usr/bin/env bash
# Q4 incremental: one-line edit (one endpoint's path) then rebuild; interleaved.
set -euo pipefail
unset CDPATH
SPIKE_DIR="$(cd -P -- "$(dirname -- "$0")/.." >/dev/null && pwd)"
cd "$SPIKE_DIR"; [[ -f src/synthetic_new.rs ]] || exit 1
export CARGO_TARGET_DIR="$1"; ROUNDS="${2:-4}"
export RUSTC_WRAPPER="" CARGO_BUILD_RUSTC_WRAPPER="" CARGO_INCREMENTAL=1
now() { python3 -c 'import time;print(time.time())'; }
trap 'python3 scripts/gen_synthetic.py 550 >/dev/null' EXIT
declare -A R
for r in $(seq "$ROUNDS"); do
  for cfg in "old:synthetic-old:src/synthetic_old.rs" "new:synthetic-new:src/synthetic_new.rs" "calls:synthetic-new-calls:src/synthetic_new.rs"; do
    IFS=: read -r name feat file <<<"$cfg"
    cargo build --lib --features "$feat" -q
    python3 - "$file" "$r$name" <<'PY'
import sys, re
p, tag = sys.argv[1], sys.argv[2]
s = open(p).read()
s, n = re.subn(r'/e7(_\w+)?/', f'/e7_{tag}/', s, count=1)
assert n == 1
open(p, 'w').write(s)
PY
    s=$(now); cargo build --lib --features "$feat" -q; e=$(now)
    R[$name]+="$(python3 -c "print(f'{$e-$s:.2f}')") "
  done
done
for k in old new calls; do echo "$k incremental: ${R[$k]}"; done
