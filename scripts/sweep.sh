#!/usr/bin/env bash
# Prune Cargo target/ directories, which cargo never garbage-collects.
#
# Four escalating passes per root, most-targeted first:
#   1. incremental caches over SWEEP_INCREMENTAL_MAX_GIB (default 15)
#   2. artifacts from uninstalled toolchains
#   3. artifacts untouched for more than SWEEP_TIME_DAYS (default 14)
#   4. BACKSTOP: cap each target/ at SWEEP_MAX_SIZE (default 120GB), oldest first
#
# Swept artifacts come back as link-restores rather than recompiles when a host
# has opted into kache as the rustc wrapper; activation is a per-host decision
# and nothing here assumes it. Sweep remains required either way because Cargo
# does not garbage-collect target directories, and a lean target/ keeps kache's
# per-crate keying fast (~100x slower on huge trees).
# Decisions and sizing evidence: docs/kache-strategy.md.
#
# Usage: sweep.sh [ROOT ...]   (default: the enclosing git worktree root)
set -euo pipefail

time_days="${SWEEP_TIME_DAYS:-14}"
max_size="${SWEEP_MAX_SIZE:-120GB}"
incremental_max_kb=$(( ${SWEEP_INCREMENTAL_MAX_GIB:-15} * 1024 * 1024 ))

if [[ $# -gt 0 ]]; then
    roots=("$@")
else
    roots=("$(git rev-parse --show-toplevel 2>/dev/null || pwd)")
fi

for root in "${roots[@]}"; do
    if [[ ! -d "$root" ]]; then
        echo "sweep: skipping missing root: $root" >&2
        continue
    fi

    echo "==> $root"

    # Census: the 10 largest target/ dirs before sweeping, so cap sizing is
    # driven by the log rather than guesses.
    find "$root" -type d -name target -prune 2>/dev/null \
        | while IFS= read -r target; do du -sk "$target" 2>/dev/null; done \
        | sort -rn \
        | head -10 \
        | awk '{printf "[census] %8.1f GiB  %s\n", $1 / 1048576, $2}'

    # Incremental caches are capped by size because the --time pass below
    # structurally cannot reach them: every build re-touches the incremental
    # directory of each crate it compiles, so these artifacts are perpetually
    # fresh no matter how stale the work behind them is. Removing one costs a
    # single non-incremental rebuild of the workspace crates; dependency
    # artifacts under deps/ are untouched. cargo-sweep does not cover this.
    find "$root" -type d -name target -prune 2>/dev/null \
        | while IFS= read -r target; do
            find "$target" -type d -name incremental -prune 2>/dev/null
        done \
        | while IFS= read -r inc; do
            size_kb=$(du -sk "$inc" 2>/dev/null | awk '{print $1}')
            [[ -n "${size_kb:-}" ]] || continue
            if (( size_kb > incremental_max_kb )); then
                awk -v k="$size_kb" -v c="$incremental_max_kb" -v p="$inc" 'BEGIN {
                    printf "[incremental] %.1f GiB > %.1f GiB cap -- removing %s\n",
                           k / 1048576, c / 1048576, p
                }'
                rm -rf "$inc"
            fi
        done

    cargo sweep -r --installed "$root"
    cargo sweep -r --time "$time_days" "$root"
    cargo sweep -r --maxsize "$max_size" "$root"
done

# APFS local snapshots can pin freed blocks; thin them so the space actually
# returns to the volume. macOS only.
if command -v tmutil &> /dev/null; then
    tmutil thinlocalsnapshots / 9999999999 2> /dev/null || true
fi

echo "sweep: done"
