#!/usr/bin/env bash
# Prune Cargo target/ directories, which cargo never garbage-collects.
#
# Three escalating passes per root, most-targeted first:
#   1. artifacts from uninstalled toolchains
#   2. artifacts untouched for more than SWEEP_TIME_DAYS (default 14)
#   3. BACKSTOP: cap each target/ at SWEEP_MAX_SIZE (default 120GB), oldest first
#
# With kache wired as the rustc wrapper (tracked .cargo/config.toml), swept
# artifacts come back as link-restores rather than recompiles, and a lean
# target/ keeps kache's per-crate keying fast (~100x slower on huge trees).
# Decisions and sizing evidence: docs/kache-strategy.md.
#
# Usage: sweep.sh [ROOT ...]   (default: the enclosing git worktree root)
set -euo pipefail

time_days="${SWEEP_TIME_DAYS:-14}"
max_size="${SWEEP_MAX_SIZE:-120GB}"

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
