#!/usr/bin/env bash
# check-comments.sh - heuristic comment quality checker for Rust files.
#
# Flags suspicious comment patterns defined in `docs/comment-quality.md`:
#
#   - long-doc-short-fn   /// blocks > 15 lines on functions with bodies < 10 lines
#   - arguments-block     literal `## Arguments` inside a /// block
#   - heavy-example       fenced ``` example inside /// > 20 lines
#   - redundant-accessor  /// block attached to `pub fn ... -> T { self.field }`
#
# Output: one finding per line, formatted as `file:line:category`.
# Exit status is always 0 — this is warn-only tooling, not a CI gate.
#
# Usage:
#   scripts/check-comments.sh [path ...]
#
# Default path is `claudine/`. Multiple paths may be supplied. Test
# directories (`tests/`, `target/`) are skipped automatically.

set -euo pipefail

if [[ $# -eq 0 ]]; then
    targets=("claudine")
else
    targets=("$@")
fi

files=()
while IFS= read -r -d '' f; do
    files+=("$f")
done < <(
    find "${targets[@]}" \
        -type d \( -name target -o -name tests -o -name benches -o -name examples \) -prune \
        -o -type f -name '*.rs' -print0
)

if [[ ${#files[@]} -eq 0 ]]; then
    exit 0
fi

awk '
function emit(cat, line) {
    printf "%s:%d:%s\n", FILENAME, line, cat
}

function count_braces(s,   tmp, o, c) {
    tmp = s
    o = gsub(/\{/, "{", tmp)
    tmp = s
    c = gsub(/\}/, "}", tmp)
    return o - c
}

function reset_state() {
    in_doc = 0
    doc_block_start = 0
    doc_block_lines = 0
    doc_all_short = 1
    in_example = 0
    example_lines = 0
    example_start = 0
    pending_doc_lines = 0
    pending_doc_start = 0
    pending_doc_all_short = 0
    watching_fn = 0
    fn_brace_depth = 0
    fn_body_lines = 0
    fn_body_text = ""
    fn_is_accessor_candidate = 0
    watching_doc_lines = 0
    watching_doc_start = 0
    watching_doc_all_short = 0
    in_signature = 0
    sig_doc_lines = 0
    sig_doc_start = 0
    sig_doc_all_short = 0
    sig_is_accessor_candidate = 0
}

function single_line_accessor_check(body, doc_lines, doc_start, doc_all_short) {
    if (body ~ /\{[ \t]*&?self\.[A-Za-z_][A-Za-z0-9_]*[ \t]*\}/) {
        if (doc_lines > 0 && doc_all_short) {
            emit("redundant-accessor", doc_start)
        }
    }
}

function enter_body(line, doc_lines, doc_start, doc_all_short, is_accessor,   delta) {
    delta = count_braces(line)
    if (delta <= 0) {
        if (doc_lines > 15) emit("long-doc-short-fn", doc_start)
        if (is_accessor) single_line_accessor_check(line, doc_lines, doc_start, doc_all_short)
        return
    }
    watching_fn = 1
    fn_brace_depth = delta
    fn_body_lines = 0
    fn_body_text = ""
    fn_is_accessor_candidate = is_accessor
    watching_doc_lines = doc_lines
    watching_doc_start = doc_start
    watching_doc_all_short = doc_all_short
}

BEGIN { reset_state() }
FNR == 1 { reset_state() }

{
    line = $0
    trimmed = line
    sub(/^[ \t]+/, "", trimmed)

    is_doc = (trimmed ~ /^\/\/\//)

    if (is_doc) {
        if (!in_doc) {
            in_doc = 1
            doc_block_start = FNR
            doc_block_lines = 0
            doc_all_short = 1
            in_example = 0
            example_lines = 0
        }
        doc_block_lines++

        content = trimmed
        sub(/^\/\/\/[ \t]?/, "", content)

        if (length(content) >= 60) doc_all_short = 0

        if (content ~ /^##[ \t]+Arguments[ \t]*$/) emit("arguments-block", FNR)

        if (content ~ /^```/) {
            if (in_example) {
                if (example_lines > 20) emit("heavy-example", example_start)
                in_example = 0
                example_lines = 0
            } else {
                in_example = 1
                example_lines = 0
                example_start = FNR
            }
        } else if (in_example) {
            example_lines++
        }

        next
    }

    if (in_doc) {
        if (in_example && example_lines > 20) emit("heavy-example", example_start)
        pending_doc_lines = doc_block_lines
        pending_doc_start = doc_block_start
        pending_doc_all_short = doc_all_short
        in_doc = 0
        in_example = 0
        example_lines = 0
        doc_block_lines = 0
    }

    if (watching_fn) {
        delta = count_braces(line)
        fn_brace_depth += delta
        if (trimmed != "") fn_body_lines++

        # Track candidate accessor body: a single non-brace expression line
        # of the form `self.field`, `&self.field`, `self.field.method()`, etc.
        body_only = trimmed
        sub(/[ \t]*}[ \t]*$/, "", body_only)
        sub(/[ \t]*\{[ \t]*$/, "", body_only)
        if (body_only != "" && body_only !~ /^\}$/ && body_only !~ /^\{$/) {
            if (fn_body_text == "") fn_body_text = body_only
            else fn_body_text = fn_body_text " " body_only
        }

        if (fn_brace_depth <= 0) {
            if (watching_doc_lines > 15 && fn_body_lines < 10) {
                emit("long-doc-short-fn", watching_doc_start)
            }
            if (fn_is_accessor_candidate && watching_doc_lines > 0 && watching_doc_all_short) {
                if (fn_body_text ~ /^&?self\.[A-Za-z_][A-Za-z0-9_]*(\.[A-Za-z_][A-Za-z0-9_]*(\(\))?)*$/) {
                    emit("redundant-accessor", watching_doc_start)
                }
            }
            watching_fn = 0
            fn_brace_depth = 0
            fn_body_lines = 0
            fn_body_text = ""
            fn_is_accessor_candidate = 0
            watching_doc_lines = 0
        }
        next
    }

    # Continuing a multi-line signature; look for the `{` that opens the body.
    if (in_signature) {
        if (line ~ /->/) sig_is_accessor_candidate = 1
        if (line ~ /\{/) {
            in_signature = 0
            enter_body(line, sig_doc_lines, sig_doc_start, sig_doc_all_short, sig_is_accessor_candidate)
        }
        next
    }

    if (trimmed == "" || trimmed ~ /^#\[/ || trimmed ~ /^#!\[/) next

    if (line ~ /^[ \t]*(pub[ \t]*(\([^)]*\))?[ \t]+)?(async[ \t]+)?(const[ \t]+)?(unsafe[ \t]+)?fn[ \t]+[A-Za-z_][A-Za-z0-9_]*/) {
        is_accessor_candidate = (line ~ /->/)

        if (line ~ /\{/) {
            enter_body(line, pending_doc_lines, pending_doc_start, pending_doc_all_short, is_accessor_candidate)
        } else {
            in_signature = 1
            sig_doc_lines = pending_doc_lines
            sig_doc_start = pending_doc_start
            sig_doc_all_short = pending_doc_all_short
            sig_is_accessor_candidate = is_accessor_candidate
        }

        pending_doc_lines = 0
        next
    }

    pending_doc_lines = 0
}
' "${files[@]}"

exit 0
