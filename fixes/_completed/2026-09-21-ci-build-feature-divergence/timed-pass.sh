#!/usr/bin/env bash
# Task 1.7 — the one local timed pass for the feature-attribution join.
#
# Reproduces the environment owner's build for the ten-package selection, in
# the producer's order (sorted by package name), in one target tree: each
# package's archive compile (`cargo test --no-run`, which is what
# `cargo nextest archive` runs), then its sidecar builds. Builds only; no test
# executes.
#
# To mirror CI, where `Swatinem/rust-cache` restores every third-party artifact
# but never a workspace crate, a warm pass builds everything first, then every
# workspace crate's fingerprint, build output, and incremental state is removed
# before the measured pass. Every rustc invocation of the measured pass is
# timed by the `ci-build` wrapper, labeled with its owner package.
#
# usage: timed-pass.sh <out-dir>
set -euo pipefail
unset CDPATH

repo="$(git rev-parse --show-toplevel)"
out="${1:?usage: timed-pass.sh <out-dir>}"
mkdir -p "$out"
out="$(cd "$out" >/dev/null && pwd)"
target_triple="$(rustc -vV | sed -n 's/^host: //p')"
export CARGO_TARGET_DIR="$repo/target/feature-attribution-timing"
# rust-cache sets this in CI; incremental state would also survive the purge.
export CARGO_INCREMENTAL=0
export KACHE_DISABLED=1

owners=(biscuit-file claudine claudine-cli claudine-gen darkmatter darkmatter-cli dmls repo-deps sniff sniff-cli)

wrapper_target="$repo/target/feature-attribution-wrapper"
cargo build --quiet --manifest-path "$repo/Cargo.toml" --target-dir "$wrapper_target" \
    -p repo-deps --no-default-features --features build-tools --bin ci-build
wrapper="$wrapper_target/debug/ci-build"

ci_features() {
    cargo metadata --manifest-path "$repo/Cargo.toml" --no-deps --format-version 1 |
        python3 -c 'import json,sys
p=[p for p in json.load(sys.stdin)["packages"] if p["name"]==sys.argv[1]][0]
t=((p.get("metadata") or {}).get("ci") or {}).get("tests") or {}
print(",".join(t.get("features",[])))
print(" ".join(t.get("sidecars",[])))' "$1"
}

sidecar_args() {
    python3 -c 'import json,sys
s=json.load(open(sys.argv[1]))["sidecars"][sys.argv[2]]
args=["--package",s["package"]]
for f in s.get("features",[]): args+=["--features",f]
for b in s["bins"]: args+=["--bin",b]
print(" ".join(args))' "$repo/.github/ci/sidecars.json" "$1"
}

build_all() {
    local log="$1"
    for owner in "${owners[@]}"; do
        local meta features sidecars
        meta="$(ci_features "$owner")"
        features="$(sed -n 1p <<<"$meta")"
        sidecars="$(sed -n 2p <<<"$meta")"
        local args=(test --no-run --profile test --target "$target_triple" --package "$owner")
        [[ -n "$features" ]] && args+=(--features "$features")
        local started=$SECONDS
        BISCUIT_CI_BUILD_PACKAGE="$owner" cargo "${args[@]}" --manifest-path "$repo/Cargo.toml" 2>>"$log"
        for sidecar in $sidecars; do
            # shellcheck disable=SC2046
            BISCUIT_CI_BUILD_PACKAGE="$owner" cargo build --profile test --target "$target_triple" \
                --manifest-path "$repo/Cargo.toml" $(sidecar_args "$sidecar") 2>>"$log"
        done
        echo "$owner $((SECONDS - started))" >>"$out/owner-seconds.txt"
    done
}

export RUSTC_WRAPPER="$wrapper" BISCUIT_CI_BUILD_WRAP=1 BISCUIT_CI_BUILD_CONFIGURATION=timed-pass

# Warm pass: third-party artifacts, as the dependency cache would restore them.
export BISCUIT_CI_BUILD_COUNTER_DIR="$out/warm-events"
: >"$out/owner-seconds.txt"
build_all "$out/warm.log"
mv "$out/owner-seconds.txt" "$out/warm-owner-seconds.txt"

# Purge every workspace crate, as a fresh CI checkout would find it.
members="$(cargo metadata --manifest-path "$repo/Cargo.toml" --no-deps --format-version 1 |
    python3 -c 'import json,sys; print(" ".join(p["name"] for p in json.load(sys.stdin)["packages"]))')"
for dir in "$CARGO_TARGET_DIR/$target_triple/debug" "$CARGO_TARGET_DIR/debug"; do
    [[ -d "$dir" ]] || continue
    for name in $members; do
        rm -rf "$dir/.fingerprint/$name-"* "$dir/build/$name-"* "$dir/incremental/${name//-/_}-"*
        rm -f "$dir/deps/lib${name//-/_}-"* "$dir/deps/${name//-/_}-"*
    done
done

# Measured pass.
export BISCUIT_CI_BUILD_COUNTER_DIR="$out/events"
build_all "$out/timed.log"
echo "events: $out/events"
