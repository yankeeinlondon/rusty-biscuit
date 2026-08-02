#!/usr/bin/env bash
# Refuse storage-heavy Cargo work before Windows runs out of target-volume space.
set -euo pipefail

case "$(uname -s)" in
    CYGWIN*|MINGW*|MSYS*) ;;
    *) exit 0 ;;
esac

minimum_gib="${BISCUIT_BUILD_MIN_FREE_GIB:-50}"
if [[ ! "$minimum_gib" =~ ^[0-9]+$ ]]; then
    echo "BISCUIT_BUILD_MIN_FREE_GIB must be a non-negative integer, got: $minimum_gib" >&2
    exit 2
fi
if (( minimum_gib == 0 )); then
    exit 0
fi

if ! command -v jq >/dev/null 2>&1; then
    echo "Storage preflight requires jq; run 'just init' first." >&2
    exit 2
fi

target_dir="$(cargo metadata --no-deps --format-version 1 | jq -r '.target_directory')"
if command -v cygpath >/dev/null 2>&1; then
    probe_path="$(cygpath -u "$target_dir")"
else
    probe_path="$target_dir"
fi

# A clean checkout may not have created target/ yet; inspect its nearest
# existing parent because that is the filesystem Cargo will consume.
while [[ ! -e "$probe_path" ]]; do
    parent="$(dirname "$probe_path")"
    if [[ "$parent" == "$probe_path" ]]; then
        echo "Cannot resolve an existing parent for Cargo target directory: $target_dir" >&2
        exit 2
    fi
    probe_path="$parent"
done

free_kib="$(df -Pk "$probe_path" | awk 'END { print $4 }')"
if [[ ! "$free_kib" =~ ^[0-9]+$ ]]; then
    echo "Cannot determine free space for Cargo target directory: $target_dir" >&2
    exit 2
fi

required_kib=$(( minimum_gib * 1024 * 1024 ))
if (( free_kib < required_kib )); then
    free_gib="$(awk -v kib="$free_kib" 'BEGIN { printf "%.1f", kib / 1048576 }')"
    cat >&2 <<EOF
Cargo storage preflight failed.
  target:   $target_dir
  free:     ${free_gib} GiB
  required: ${minimum_gib} GiB

Run 'just sweep' before starting another build or gate. On a constrained
Windows host, set SWEEP_MAX_SIZE=80GB for the sweep task. Set
BISCUIT_BUILD_MIN_FREE_GIB=0 only for an intentional emergency override.
EOF
    exit 1
fi
