#!/usr/bin/env bash
# check-lifecycle-doc-facets.sh — doc guard keeping new lifecycle documentation
# on the faceted `err.*` contract instead of the deprecated legacy aliases.
#
# The real-errors feature added the stable, versioned facet surface
# (`err.code` / `err.category` / `err.disposition` / `err.origin` /
# `err.detail.*`) and kept `err.kind` / `err.variant` / `err.msg` only as
# DEPRECATED ALIASES for backward compatibility. New examples that match on the
# aliases teach stringly handlers that drift across providers, so this guard
# fails if a lifecycle doc references `err.kind` / `err.variant` / `err.msg`
# anywhere OUTSIDE the explicit "Deprecated aliases" documentation context.
#
# Allowance mechanism (so documenting the aliases doesn't trip the guard):
# a single delimited section is exempt. The section starts at a Markdown
# heading whose text contains "Deprecated aliases" (case-insensitive) and ends
# at the next heading line (any `#`-prefixed line) or end of file. Only lines
# inside that window may mention the deprecated aliases. This is a precise,
# self-documenting window — not a blanket per-line skip — so a stray alias in a
# real example block is still caught.
#
# This is a heuristic grep-style review guard, not a Markdown parser.
#
# Output: one finding per violation on stderr, `file:line` plus the offending
# line. Exit status is 1 when any out-of-context alias use is found, else 0.
#
# Usage:
#   scripts/check-lifecycle-doc-facets.sh [file ...]
#
# Default targets are every public lifecycle/composition topic that mentions
# lifecycle `err.*`: `lifecycle.md`, `composition.md`, and
# `frontmatter-properties.md`. Explicit files may be supplied (e.g. when a new
# lifecycle topic doc is added) to override the default set.

set -euo pipefail

# Compute script_dir without letting a `cd` CDPATH match leak to stdout: clear
# CDPATH for the subshell and silence the cd. (A stray CDPATH echo would make
# script_dir a multi-line string and corrupt the path — the bug the companion
# transport guard hit when invoked directly with CDPATH set.)
script_dir="$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")" >/dev/null 2>&1 && pwd)"
repo_root="$(CDPATH= cd -- "${script_dir}/.." >/dev/null 2>&1 && pwd)"

if [[ $# -eq 0 ]]; then
    # Every public lifecycle/composition topic that documents lifecycle `err.*`.
    # The deprecated aliases are allowed only inside an explicit
    # "Deprecated aliases" section (see the window logic below).
    files=(
        "${repo_root}/claudine/docs/topics/lifecycle.md"
        "${repo_root}/claudine/docs/topics/composition.md"
        "${repo_root}/claudine/docs/topics/frontmatter-properties.md"
    )
else
    files=("$@")
fi

# A line mentions a deprecated alias when it references err.kind / err.variant /
# err.msg as a field path (the `.` rules out bare `kind`/`variant`/`msg`).
mentions_alias() {
    local s="$1"
    [[ "$s" == *"err.kind"* || "$s" == *"err.variant"* || "$s" == *"err.msg"* ]]
}

# A line opens/labels the exempt window when it is a Markdown heading whose text
# contains "deprecated aliases" (case-insensitive). `nocasematch` keeps this
# portable to bash 3.2 (no `${var,,}` lowercasing).
is_deprecated_heading() {
    local s="$1"
    [[ "$s" == \#* ]] || return 1
    local rc=0
    shopt -s nocasematch
    [[ "$s" == *"deprecated aliases"* ]] || rc=1
    shopt -u nocasematch
    return $rc
}

violations=0
for f in "${files[@]}"; do
    if [[ ! -f "$f" ]]; then
        echo "lifecycle-doc-facets guard: file not found: $f" >&2
        exit 1
    fi
    rel="${f#"${repo_root}/"}"
    lineno=0
    in_deprecated=0
    while IFS= read -r line || [[ -n "$line" ]]; do
        lineno=$((lineno + 1))
        if [[ "$line" == \#* ]]; then
            # Any heading closes a prior exempt window; a deprecated-aliases
            # heading opens a new one.
            if is_deprecated_heading "$line"; then
                in_deprecated=1
            else
                in_deprecated=0
            fi
            continue
        fi
        [[ ${in_deprecated} -eq 1 ]] && continue
        mentions_alias "$line" || continue
        printf '%s:%d: deprecated err alias used outside the "Deprecated aliases" section\n    %s\n' \
            "${rel}" "${lineno}" "${line}" >&2
        violations=$((violations + 1))
    done < "${f}"
done

if [[ ${violations} -gt 0 ]]; then
    {
        echo
        echo "lifecycle-doc-facets guard: ${violations} deprecated err-alias use(s) outside the allowed section."
        echo "New lifecycle docs/examples must match the faceted contract: err.code, err.category,"
        echo "err.disposition, err.origin, err.detail.* (see the error catalog). Keep err.kind /"
        echo "err.variant / err.msg only inside the '#### Deprecated aliases' section."
    } >&2
    exit 1
fi

echo "✅ lifecycle-doc-facets guard: lifecycle docs use the faceted err.* contract."
