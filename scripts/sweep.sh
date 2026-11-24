#!/usr/bin/env bash
# Prune Cargo target/ directories, which cargo never garbage-collects.
#
# Four escalating passes per root, most-targeted first:
#   1. combined incremental caches per target over SWEEP_INCREMENTAL_MAX_GIB (default 15)
#   2. artifacts from uninstalled toolchains
#   3. artifacts untouched for more than SWEEP_TIME_DAYS (default 14)
#   4. cap each target/ at SWEEP_HARD_MAX_SIZE (default 200GB), oldest first;
#      below SWEEP_MIN_FREE_GIB free (default 100), use the tighter
#      SWEEP_MAX_SIZE (default 120GB) instead
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
# Usage: sweep.sh [--dry-run] [ROOT ...]   (default: the enclosing git worktree root)
set -euo pipefail

if [[ "${1:-}" == --dry-run ]]; then
    shift
    script_dir=$(CDPATH= cd -- "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)
    if [[ $# == 0 ]]; then
        set -- "$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
    fi
    for root in "$@"; do
        bash "$script_dir/sweep.sh" --preview-input "$root" | python3 "$script_dir/sweep-report.py" "Not checked"
    done
    exit 0
fi

preview=0
if [[ "${1:-}" == --preview-input ]]; then
    preview=1
    shift
fi

# Manual and scheduled cleanup share the guard. A build can start while the
# census or a prior pass is running, so check again before each mutation.
defer_if_building() {
    if pgrep -x 'cargo|rustc|clippy-driver|cargo-nextest' >/dev/null 2>&1; then
        echo "Sweep deferred because build processes are active."
        exit 0
    fi
}

remove_tree() {
    local diagnostics result attempt
    for attempt in 1 2; do
        defer_if_building
        if diagnostics=$(LC_ALL=C rm -rf -- "$@" 2>&1); then
            return 0
        else
            result=$?
        fi
        if [[ -z "$diagnostics" ]] || grep -qv ': Directory not empty$' <<< "$diagnostics"; then
            printf '%s\n' "$diagnostics" >&2
            return "$result"
        fi
        defer_if_building
        if [[ "$attempt" == 1 ]]; then
            echo "A cleanup directory changed during removal. Retrying once." >&2
        fi
    done
    printf '%s\n' "$diagnostics" >&2
    return "$result"
}

if [[ "$preview" == 0 ]]; then
    defer_if_building
fi

time_days="${SWEEP_TIME_DAYS:-14}"
max_size="${SWEEP_MAX_SIZE:-120GB}"
hard_max_size="${SWEEP_HARD_MAX_SIZE:-200GB}"
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

if [[ "$preview" == 1 ]]; then
    root=$(CDPATH= cd -- "${roots[0]}" && pwd -P)
    roots=("$root")
    printf '%s\0' "$root" "$time_days" "$min_free_gib" "$max_size" "$hard_max_size"
fi

for root in "${roots[@]}"; do
    if [[ ! -d "$root" ]]; then
        echo "sweep: skipping missing root: $root" >&2
        continue
    fi

    if [[ "$preview" == 0 ]]; then
        echo "Sweeping ${root##*/}"
        find "$root" -type d -name target -prune -print0 2>/dev/null \
            | while IFS= read -r -d '' target; do du -sk "$target" 2>/dev/null; done \
            | sort -rn \
            | awk -F '\t' -v root="$root/" 'NR <= 10 {
                if (index($2, root) == 1) $2 = substr($2, length(root) + 1)
                printf "[census] %8.1f GiB  %s\n", $1 / 1048576, $2
            }'
    fi

    # Incremental caches are capped by size because the --time pass below
    # structurally cannot reach them: every build re-touches the incremental
    # directory of each crate it compiles, so these artifacts are perpetually
    # fresh no matter how stale the work behind them is. Removing them costs a
    # single non-incremental rebuild of the workspace crates; dependency
    # artifacts under deps/ are untouched. cargo-sweep does not cover this.
    while IFS= read -r target; do
        incremental_dirs=()
        while IFS= read -r -d '' inc; do
            incremental_dirs+=("$inc")
        done < <(find "$target" -type d \( -name incremental -o -name incremental.kache-auto \) -prune -print0 2>/dev/null)
        [[ -n "${incremental_dirs[0]:-}" ]] || continue
        size_kb=$(du -skc "${incremental_dirs[@]}" 2>/dev/null | awk 'END {print $1}')
        [[ -n "${size_kb:-}" ]] || continue
        if (( size_kb > incremental_max_kb )); then
            if [[ "$preview" == 1 ]]; then
                for inc in "${incremental_dirs[@]}"; do
                    printf 'incremental\0%s\0%s\0' "$target" "$inc"
                done
            else
                awk -v k="$size_kb" -v c="$incremental_max_kb" -v p="${target#"$root"/}" 'BEGIN {
                    printf "[incremental] %.1f GiB > %.1f GiB combined cap -- removing incremental caches under %s\n",
                           k / 1048576, c / 1048576, p
                }'
                remove_tree "${incremental_dirs[@]}"
            fi
        fi
    done < <(find "$root" -type d -name target -prune 2>/dev/null)

    if [[ "$preview" == 1 ]]; then
        continue
    fi
    defer_if_building
    cargo sweep -r --installed "$root"
    defer_if_building
    cargo sweep -r --time "$time_days" "$root"

    # Bound target growth even on roomy volumes; reserve the tighter cap for
    # disk pressure because eviction can discard reusable dependencies.
    free_kb="$(df -Pk "$root" 2>/dev/null | awk 'END { print $4 }')"
    if [[ "$free_kb" =~ ^[0-9]+$ ]] && (( free_kb >= min_free_kb )); then
        awk -v f="$free_kb" -v m="$min_free_kb" 'BEGIN {
            printf "[capacity] %.1f GiB free >= %.1f GiB floor -- applying unconditional cap\n",
                   f / 1048576, m / 1048576
        }'
        defer_if_building
        cargo sweep -r --maxsize "$hard_max_size" "$root"
    else
        if [[ ! "$free_kb" =~ ^[0-9]+$ ]]; then
            echo "[capacity] free space unavailable -- applying maxsize backstop" >&2
        else
            awk -v f="$free_kb" -v m="$min_free_kb" 'BEGIN {
                printf "[capacity] %.1f GiB free < %.1f GiB floor -- applying maxsize backstop\n",
                       f / 1048576, m / 1048576
            }'
        fi
        defer_if_building
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

        if [[ "$preview" == 1 ]]; then
            printf 'orphan\0%s\0' "$resolved"
        else
            size_kb=$(du -sk "$candidate" 2>/dev/null | awk '{print $1}')
            awk -v k="${size_kb:-0}" -v d="$orphan_days" -v p="${candidate##*/}" 'BEGIN {
                printf "[orphan] %.1f GiB  untouched >%s days -- removing %s\n", k / 1048576, d, p
            }'
            remove_tree "$candidate"
        fi
    done < <(find "$orphan_dir" -mindepth 2 -maxdepth 3 -name CACHEDIR.TAG -type f 2>/dev/null)
done

if [[ "$preview" == 1 ]]; then
    exit 0
fi

# APFS local snapshots can pin freed blocks; thin them so the space actually
# returns to the volume. macOS only.
if command -v tmutil &> /dev/null; then
    defer_if_building
    tmutil thinlocalsnapshots / 9999999999 2> /dev/null || true
fi
