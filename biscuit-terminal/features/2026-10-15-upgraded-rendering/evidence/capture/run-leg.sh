#!/usr/bin/env bash
# Usage: run-leg.sh <leg-label> [repo-root]
#
# Renders the frozen Mermaid corpus with the dependency versions currently in
# <repo-root>/Cargo.lock and writes artifacts plus report.md to
# ../leg-<label>/. The capture crate is assembled under $TMPDIR, outside the
# workspace, so it adds no workspace target; the copied lockfile pins it to the
# workspace's resolved mermaid-rs-renderer and resvg.
set -euo pipefail
leg="$1"
here="$(cd "$(dirname "$0")" && pwd)"
repo="${2:-$(cd "$here/../../../../.." && pwd)}"
work="$(mktemp -d)"
out="$here/../leg-$leg"
rm -rf "$out"
mkdir -p "$work/src" "$out"
cat > "$work/Cargo.toml" <<TOML
[package]
name = "upgraded-rendering-capture"
version = "0.0.0"
edition = "2021"
publish = false

[workspace]

[dependencies]
biscuit-visualized = { path = "$repo/biscuit-visualized/src", features = ["image"] }
biscuit-hash = { path = "$repo/biscuit-hash/lib" }
resvg = "*"
TOML
cp "$here/capture.rs" "$work/src/main.rs"
cp "$repo/Cargo.lock" "$work/Cargo.lock"
cp "$repo/rust-toolchain.toml" "$work/rust-toolchain.toml"
(cd "$work" && cargo build --release --quiet)
(cd "$work" && cargo tree -e normal --prefix none 2>/dev/null \
  | grep -E '^(resvg|usvg|mermaid-rs-renderer|fontdb|tiny-skia) ' | sort -u) > "$out/versions.txt"
rustc_version="$(cd "$work" && rustc --version)"
echo "$rustc_version" >> "$out/versions.txt"
BISCUIT_VISUALIZED_CACHE_DIR="$work/cache" "$work/target/release/upgraded-rendering-capture" \
  "$repo/biscuit-visualized/src/tests/fixtures/mermaid-corpus" "$out" "$leg"
echo "artifacts: $out"
