#!/usr/bin/env bash
# Builds and runs the S1/S2 spike harness (src/main.rs) as a standalone crate
# in a temporary directory, so no Cargo manifest ever lives in the repository.
# It resolves dependencies with the repository's own Cargo.lock.
#
# Usage: run.sh <output dir>   (run from anywhere inside the repository)
set -euo pipefail
out=$(cd "$(dirname "${1:?output directory}")" && pwd)/$(basename "$1")
here=$(cd "$(dirname "$0")" && pwd)
root=$(git -C "$here" rev-parse --show-toplevel)
crate=$(mktemp -d "${TMPDIR:-/tmp}/wt-graph-render-topologies.XXXXXX")
mkdir "$crate/src"
cp "$here/src/main.rs" "$crate/src/main.rs"
cp "$root/Cargo.lock" "$crate/Cargo.lock"
cat > "$crate/Cargo.toml" <<TOML
[package]
name = "wt-graph-render-topologies-spike"
version = "0.0.0"
edition = "2024"
publish = false

[dependencies]
biscuit-terminal = { path = "$root/biscuit-terminal/lib", features = ["image"] }
mermaid-rs-renderer = { version = "=0.3.1", default-features = false, features = ["png"] }

[workspace]
TOML
CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-${TMPDIR:-/tmp}/wt-graph-spike-target}" \
  cargo run --release --offline --manifest-path "$crate/Cargo.toml" -- "$out"
