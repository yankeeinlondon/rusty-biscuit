#!/usr/bin/env bash
# Prune Cargo target/ directories, which cargo never garbage-collects.
#
# Four escalating passes per root, most-targeted first:
#   1. incremental caches over SWEEP_INCREMENTAL_MAX_GIB (default 15)
#   2. artifacts from uninstalled toolchains
#   3. artifacts untouched for more than SWEEP_TIME_DAYS (default 14)
#   4. LOW-SPACE BACKSTOP: when the target filesystem has less than
#      SWEEP_MIN_FREE_GIB free (default 100), cap each target/ at
#      SWEEP_MAX_SIZE (default 120GB), oldest first
#
# Then one pass over the whole host:
#   5. out-of-tree target dirs untouched for more than SWEEP_ORPHAN_DAYS,
#      under the SWEEP_ORPHAN_DIRS locations (default ~/.cache), removed whole
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
min_free_gib="${SWEEP_MIN_FREE_GIB:-100}"
incremental_max_kb=$(( ${SWEEP_INCREMENTAL_MAX_GIB:-15} * 1024 * 1024 ))

if [[ ! "$min_free_gib" =~ ^[0-9]+$ ]]; then
    echo "SWEEP_MIN_FREE_GIB must be a non-negative integer, got: $min_free_gib" >&2
    exit 2
fi
min_free_kb=$(( min_free_gib * 1024 * 1024 ))

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

    # Artifact mtimes describe when rustc produced a file, not when Cargo last
    # reused it. An unconditional oldest-first cap therefore evicts healthy,
    # reusable dependencies from an active worktree and turns the next local
    # test into a cold build. Reserve that tradeoff for actual capacity pressure.
    free_kb="$(df -Pk "$root" 2>/dev/null | awk 'END { print $4 }')"
    if [[ "$free_kb" =~ ^[0-9]+$ ]] && (( free_kb >= min_free_kb )); then
        awk -v f="$free_kb" -v m="$min_free_kb" 'BEGIN {
            printf "[capacity] %.1f GiB free >= %.1f GiB floor -- skipping maxsize backstop\n",
                   f / 1048576, m / 1048576
        }'
    else
        if [[ ! "$free_kb" =~ ^[0-9]+$ ]]; then
            echo "[capacity] free space unavailable -- applying maxsize backstop" >&2
        else
            awk -v f="$free_kb" -v m="$min_free_kb" 'BEGIN {
                printf "[capacity] %.1f GiB free < %.1f GiB floor -- applying maxsize backstop\n",
                       f / 1048576, m / 1048576
            }'
        fi
        cargo sweep -r --maxsize "$max_size" "$root"
    fi
done

# Out-of-tree target directories, swept once for the host rather than per root.
#
# The passes above can only reach target dirs *inside* a root, and only ones
# literally named "target". A build invoked with an explicit --target-dir
# elsewhere -- what agent sessions and one-off benchmark runs do routinely --
# lands outside every root under an arbitrary name, so no pass ever sees it and
# it grows without bound. Three such directories reached 119 GiB on one WSL host
# while `just sweep` reported success every time it ran.
#
# These are orphans by construction: no Cargo project points at them, so a stale
# one will never be reused and there is nothing worth sweeping incrementally --
# the whole directory goes. Cargo writes a CACHEDIR.TAG naming itself into every
# target dir it creates, which is what identifies a candidate here.
orphan_days="${SWEEP_ORPHAN_DAYS:-$time_days}"
IFS=: read -r -a orphan_dirs <<< "${SWEEP_ORPHAN_DIRS:-${XDG_CACHE_HOME:-$HOME/.cache}}"

# A target dir that a root or the environment actively points at is not an
# orphan even when it sits inside a scanned location.
protected=()
for root in "${roots[@]}"; do
    [[ -d "$root" ]] && protected+=("$(cd "$root" && pwd -P)")
done
if [[ -n "${CARGO_TARGET_DIR:-}" && -d "${CARGO_TARGET_DIR}" ]]; then
    protected+=("$(cd "$CARGO_TARGET_DIR" && pwd -P)")
fi

for orphan_dir in "${orphan_dirs[@]}"; do
    [[ -d "$orphan_dir" ]] || continue

    # -mindepth 2 keeps the scanned location itself from ever being a candidate.
    while IFS= read -r tag; do
        grep -qi 'created by cargo' "$tag" 2> /dev/null || continue
        candidate=$(dirname "$tag")
        resolved=$(cd "$candidate" 2> /dev/null && pwd -P) || continue

        skip=""
        for guarded in "${protected[@]}"; do
            if [[ "$resolved" == "$guarded" || "$resolved" == "$guarded"/* ]]; then
                skip=1
                break
            fi
        done
        [[ -n "$skip" ]] && continue

        # Build churn lands in the profile subtrees, so freshness is decided
        # there; -quit makes the still-in-use case stop at the first hit instead
        # of walking a tree that can hold millions of files.
        if [[ -n "$(find "$candidate" -maxdepth 3 -mtime "-${orphan_days}" -print -quit 2> /dev/null)" ]]; then
            continue
        fi

        size_kb=$(du -sk "$candidate" 2> /dev/null | awk '{print $1}')
        awk -v k="${size_kb:-0}" -v d="$orphan_days" -v p="$candidate" 'BEGIN {
            printf "[orphan] %.1f GiB  untouched >%s days -- removing %s\n", k / 1048576, d, p
        }'
        rm -rf "$candidate"
    done < <(find "$orphan_dir" -mindepth 2 -maxdepth 3 -name CACHEDIR.TAG -type f 2> /dev/null)
done

# APFS local snapshots can pin freed blocks; thin them so the space actually
# returns to the volume. macOS only.
if command -v tmutil &> /dev/null; then
    tmutil thinlocalsnapshots / 9999999999 2> /dev/null || true
fi

echo "sweep: done"
