#!/usr/bin/env bash
# Spike S1: builds src/main.rs as a standalone crate in a temporary
# directory (no Cargo manifest lives in the repository), resolving
# dependencies with the repository's Cargo.lock, and runs it over the
# built-in cases plus observed.mmd (the gathered Mermaid text of the
# observed-shape L1 fixture, `observed_sparse_lanes()`).
#
# Usage: run.sh   (run from anywhere inside the repository)
set -euo pipefail
here=$(cd "$(dirname "$0")" && pwd)
root=$(git -C "$here" rev-parse --show-toplevel)
crate=$(mktemp -d "${TMPDIR:-/tmp}/wt-graph-s1-geometry.XXXXXX")
mkdir "$crate/src"
cp "$here/src/main.rs" "$crate/src/main.rs"
cp "$root/Cargo.lock" "$crate/Cargo.lock"
cat > "$crate/Cargo.toml" <<TOML
[package]
name = "wt-graph-s1-geometry"
version = "0.0.0"
edition = "2024"
publish = false

[dependencies]
mermaid-rs-renderer = { version = "=0.3.1", default-features = false }

[workspace]
TOML
CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-${TMPDIR:-/tmp}/wt-graph-spike-target}" \
  cargo run --quiet --release --offline --manifest-path "$crate/Cargo.toml" -- "$here/observed.mmd"
