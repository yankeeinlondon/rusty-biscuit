#!/usr/bin/env bash
# Phase 2 visual evidence: plans the observation and long-label fixtures with
# `GitGraph::plan` (real biscuit-visualized measurement) and renders each plan
# to PNG through `MermaidDiagram::render`, the production path. Built as a
# standalone crate in a temporary directory, so no Cargo manifest lives in the
# repository; resolves dependencies with the repository's Cargo.lock.
#
# Usage: run.sh <output dir>   (run from anywhere inside the repository)
set -euo pipefail
mkdir -p "${1:?output directory}"
out=$(cd "$1" && pwd)
here=$(cd "$(dirname "$0")" && pwd)
root=$(git -C "$here" rev-parse --show-toplevel)
crate=$(mktemp -d "${TMPDIR:-/tmp}/wt-graph-phase2-renders.XXXXXX")
mkdir "$crate/src"
cp "$here/src/main.rs" "$crate/src/main.rs"
cp "$root/Cargo.lock" "$crate/Cargo.lock"
cat > "$crate/Cargo.toml" <<TOML
[package]
name = "wt-graph-phase2-renders"
version = "0.0.0"
edition = "2024"
publish = false

[dependencies]
biscuit-terminal = { path = "$root/biscuit-terminal/lib", features = ["image"] }
biscuit-visualized = { path = "$root/biscuit-visualized/src", features = ["image"] }

[workspace]
TOML
CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-${TMPDIR:-/tmp}/wt-graph-spike-target}" \
  BISCUIT_VISUALIZED_CACHE_DIR="$crate/cache" \
  cargo run --release --offline --manifest-path "$crate/Cargo.toml" -- "$out"
