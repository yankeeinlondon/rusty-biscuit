#!/usr/bin/env bash
# Build-weight information (R-S6, D29): an uncached release build of the spike
# binary per renderer variant, each in its own fresh target dir with no compiler
# cache wrapper. Writes <out>/build-weight.json.
#
# Usage: scripts/build_weight.sh <scratch-dir>
set -euo pipefail

spike="$(cd "$(dirname "$0")/.." && pwd)"
scratch="$1"
out="$scratch/build-weight.json"
unset RUSTC_WRAPPER CARGO_BUILD_RUSTC_WRAPPER
export RUSTC_WRAPPER=""

variants=(
  "current|--no-default-features --features current"
  "merman|--no-default-features --features merman"
  "merman-elk|--no-default-features --features elk"
  "both|"
)

echo "[" > "$out"
first=1
for entry in "${variants[@]}"; do
  name="${entry%%|*}"
  flags="${entry#*|}"
  target="$scratch/weight-$name"
  rm -rf "$target"
  # Fetch first so download time is not counted.
  (cd "$spike" && cargo fetch -q)
  start=$(date +%s)
  # shellcheck disable=SC2086
  (cd "$spike" && CARGO_TARGET_DIR="$target" cargo build --release -q $flags)
  end=$(date +%s)
  binary="$target/release/s1-comparison"
  size=$(stat -f %z "$binary" 2>/dev/null || stat -c %s "$binary")
  cp "$binary" "$target/stripped" && strip "$target/stripped"
  stripped=$(stat -f %z "$target/stripped" 2>/dev/null || stat -c %s "$target/stripped")
  # shellcheck disable=SC2086
  packages=$(cd "$spike" && cargo tree -e normal,build --prefix none $flags | sed 's/ (\*)//' | sort -u | wc -l | tr -d ' ')
  [ $first -eq 1 ] || echo "," >> "$out"
  first=0
  printf '  {"variant": "%s", "flags": "%s", "build_seconds": %d, "binary_bytes": %d, "stripped_bytes": %d, "unique_packages": %d}' \
    "$name" "$flags" $((end - start)) "$size" "$stripped" "$packages" >> "$out"
  echo "$name: $((end - start)) s, $size bytes ($stripped stripped), $packages packages"
  rm -rf "$target"
done
echo "" >> "$out"
echo "]" >> "$out"
