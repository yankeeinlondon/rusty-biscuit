#!/usr/bin/env bash
# check-error-transport.sh — boundary guard against typed errors collapsing to
# strings at the Darkmatter→Claudine / BlockError transport boundary.
#
# Flags Rust lines that bind a lower-layer error in `map_err(|e| …)` and then
# collapse it into a String — either as a bare `e.to_string()` or by
# interpolating `{e}` / `{e:?}` into a `format!` that feeds a String-carrying
# error variant. Such a collapse discards the typed `#[source]` cause, so the
# CLI's error walker can no longer render the real root cause (the whole point
# of the real-errors feature).
#
# This is a heuristic grep-style review guard, not an AST analysis. It keys on
# the `|e|` closure-binding convention used throughout the composition boundary.
# Intentional or not-yet-converted sites are listed in the companion allowlist
# (`scripts/check-error-transport.allow`), matched on the exact trimmed source
# line so an entry survives line-number drift. Membership is by trimmed text
# only (not file path): two files sharing an identical collapse line are
# covered by one entry — acceptable for a guard scoped to the composition
# boundary.
#
# Output: one finding per violation on stderr, `file:line` plus the offending
# line. Exit status is 1 when any non-allowlisted collapse is found, else 0.
#
# Usage:
#   scripts/check-error-transport.sh [path ...]
#
# Default path is `claudine/lib/src/composition` (the real-errors Phase 6
# boundary). Multiple paths may be supplied. `target/` is pruned automatically.

set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "${script_dir}/.." && pwd)"
allow_file="${script_dir}/check-error-transport.allow"

if [[ $# -eq 0 ]]; then
    targets=("${repo_root}/claudine/lib/src/composition")
else
    targets=("$@")
fi

files=()
while IFS= read -r -d '' f; do
    files+=("$f")
done < <(
    find "${targets[@]}" \
        -type d -name target -prune \
        -o -type f -name '*.rs' -print0
)

if [[ ${#files[@]} -eq 0 ]]; then
    echo "error-transport guard: no Rust files found under: ${targets[*]}" >&2
    exit 0
fi

# A line is a collapse candidate when it binds `|e|` in `map_err` and then
# stringifies that `e` (bare to_string or `{e}` / `{e:` inside a format).
is_collapse() {
    local s="$1"
    [[ "$s" == *"map_err(|e|"* ]] || return 1
    [[ "$s" == *"e.to_string()"* || "$s" == *'{e}'* || "$s" == *'{e:'* ]]
}

allowed() {
    [[ -f "${allow_file}" ]] || return 1
    grep -Fxq -- "$1" "${allow_file}"
}

violations=0
for f in "${files[@]}"; do
    rel="${f#"${repo_root}/"}"
    lineno=0
    while IFS= read -r line || [[ -n "$line" ]]; do
        lineno=$((lineno + 1))
        # ltrim
        trimmed="${line#"${line%%[![:space:]]*}"}"
        # Skip line comments (`//`, `///`, `//!`).
        [[ "${trimmed}" == //* ]] && continue
        is_collapse "${trimmed}" || continue
        allowed "${trimmed}" && continue
        printf '%s:%d: typed error collapsed to String at transport boundary\n    %s\n' \
            "${rel}" "${lineno}" "${trimmed}" >&2
        violations=$((violations + 1))
    done < "${f}"
done

if [[ ${violations} -gt 0 ]]; then
    {
        echo
        echo "error-transport guard: ${violations} new typed-error→String collapse(s) at the boundary."
        echo "Preserve the typed cause as a #[source] field on the error variant, or — if the"
        echo "collapse is intentional — add the exact trimmed line to"
        echo "scripts/check-error-transport.allow under a reason."
    } >&2
    exit 1
fi

echo "error-transport guard: no un-allowlisted typed-error→String collapses."
