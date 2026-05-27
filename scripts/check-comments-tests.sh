#!/usr/bin/env bash
# check-comments-tests.sh - Level 1 fixture tests for `check-comments.sh`.
#
# Writes temporary Rust fixtures, runs the checker, and asserts on the
# emitted findings and exit status. Exits 1 on any failure.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CHECKER="$SCRIPT_DIR/check-comments.sh"

if [[ ! -x "$CHECKER" ]]; then
    echo "ERROR: $CHECKER is not executable" >&2
    exit 1
fi

PASS=0
FAIL=0
FAILED_TESTS=()

run_case() {
    local name="$1"
    local fixture="$2"
    local expected_findings="$3"
    local expected_exit="${4:-0}"

    local tmpdir
    tmpdir=$(mktemp -d)
    trap "rm -rf '$tmpdir'" RETURN

    local rs_file="$tmpdir/fixture.rs"
    printf '%s' "$fixture" > "$rs_file"

    local actual_output
    local actual_exit=0
    actual_output=$("$CHECKER" "$tmpdir" 2>&1) || actual_exit=$?

    # Replace the tmpdir path so expected strings can be path-agnostic.
    local normalized
    normalized=$(printf '%s' "$actual_output" | sed "s|$tmpdir/||g")

    if [[ "$normalized" == "$expected_findings" && "$actual_exit" == "$expected_exit" ]]; then
        PASS=$((PASS + 1))
        printf '  ok    %s\n' "$name"
    else
        FAIL=$((FAIL + 1))
        FAILED_TESTS+=("$name")
        printf '  FAIL  %s\n' "$name"
        printf '        expected exit=%s findings=%q\n' "$expected_exit" "$expected_findings"
        printf '        got      exit=%s findings=%q\n' "$actual_exit" "$normalized"
    fi
}

# ---------------------------------------------------------------------------
# Test: single-line function with a long doc block flags long-doc-short-fn.
# ---------------------------------------------------------------------------
read -r -d '' FIX_SINGLE_LINE_LONG_DOC <<'RUST' || true
/// line 1
/// line 2
/// line 3
/// line 4
/// line 5
/// line 6
/// line 7
/// line 8
/// line 9
/// line 10
/// line 11
/// line 12
/// line 13
/// line 14
/// line 15
/// line 16
pub fn single() -> bool { false }
RUST
run_case \
    "single-line function with long doc flags long-doc-short-fn" \
    "$FIX_SINGLE_LINE_LONG_DOC" \
    "fixture.rs:1:long-doc-short-fn" \
    0

# ---------------------------------------------------------------------------
# Test: multi-line signature with a long doc flags long-doc-short-fn.
# ---------------------------------------------------------------------------
read -r -d '' FIX_MULTI_LINE_SIG <<'RUST' || true
/// line 1
/// line 2
/// line 3
/// line 4
/// line 5
/// line 6
/// line 7
/// line 8
/// line 9
/// line 10
/// line 11
/// line 12
/// line 13
/// line 14
/// line 15
/// line 16
pub fn accessor(
    &self,
) -> bool {
    self.enabled
}
RUST
run_case \
    "multi-line signature with long doc flags long-doc-short-fn" \
    "$FIX_MULTI_LINE_SIG" \
    "fixture.rs:1:long-doc-short-fn
fixture.rs:1:redundant-accessor" \
    0

# ---------------------------------------------------------------------------
# Test: short function with a short doc is not flagged.
# ---------------------------------------------------------------------------
read -r -d '' FIX_SHORT_DOC_SHORT_FN <<'RUST' || true
/// Compute the answer.
pub fn answer() -> u32 {
    42
}
RUST
run_case \
    "short doc on short function is not flagged" \
    "$FIX_SHORT_DOC_SHORT_FN" \
    "" \
    0

# ---------------------------------------------------------------------------
# Test: long function body with a long doc is not flagged (body >= 10).
# ---------------------------------------------------------------------------
read -r -d '' FIX_LONG_FN <<'RUST' || true
/// line 1
/// line 2
/// line 3
/// line 4
/// line 5
/// line 6
/// line 7
/// line 8
/// line 9
/// line 10
/// line 11
/// line 12
/// line 13
/// line 14
/// line 15
/// line 16
pub fn long() -> u32 {
    let a = 1;
    let b = 2;
    let c = 3;
    let d = 4;
    let e = 5;
    let f = 6;
    let g = 7;
    let h = 8;
    let i = 9;
    let j = 10;
    a + b + c + d + e + f + g + h + i + j
}
RUST
run_case \
    "long doc on long function (body >= 10) is not flagged" \
    "$FIX_LONG_FN" \
    "" \
    0

# ---------------------------------------------------------------------------
# Test: literal `## Arguments` block inside /// flags arguments-block.
# ---------------------------------------------------------------------------
read -r -d '' FIX_ARGUMENTS_BLOCK <<'RUST' || true
/// Do the thing.
///
/// ## Arguments
///
/// * `x` - the input
pub fn thing(x: u32) -> u32 {
    x
}
RUST
run_case \
    "literal ## Arguments inside /// flags arguments-block" \
    "$FIX_ARGUMENTS_BLOCK" \
    "fixture.rs:3:arguments-block" \
    0

# ---------------------------------------------------------------------------
# Test: heavy-example flags fenced block longer than 20 lines.
# ---------------------------------------------------------------------------
heavy_example_fixture() {
    printf '%s\n' '/// Example below.'
    printf '%s\n' '///'
    printf '%s\n' '/// ```'
    local i
    for i in $(seq 1 21); do
        printf '/// let v%s = %s;\n' "$i" "$i"
    done
    printf '%s\n' '/// ```'
    printf '%s\n' 'pub fn run() {}'
}
FIX_HEAVY_EXAMPLE=$(heavy_example_fixture)
run_case \
    "fenced example > 20 lines flags heavy-example" \
    "$FIX_HEAVY_EXAMPLE" \
    "fixture.rs:3:heavy-example
fixture.rs:1:long-doc-short-fn" \
    0

# ---------------------------------------------------------------------------
# Test: redundant-accessor flags `pub fn x() -> T { self.x }`.
# ---------------------------------------------------------------------------
read -r -d '' FIX_REDUNDANT_ACCESSOR <<'RUST' || true
/// Whether enabled.
pub fn enabled(&self) -> bool { self.enabled }
RUST
run_case \
    "single-line accessor with short doc flags redundant-accessor" \
    "$FIX_REDUNDANT_ACCESSOR" \
    "fixture.rs:1:redundant-accessor" \
    0

# ---------------------------------------------------------------------------
# Test: multi-line accessor body with short doc flags redundant-accessor.
# ---------------------------------------------------------------------------
read -r -d '' FIX_REDUNDANT_ACCESSOR_MULTI <<'RUST' || true
/// Whether enabled.
pub fn enabled(&self) -> bool {
    self.enabled
}
RUST
run_case \
    "multi-line accessor body with short doc flags redundant-accessor" \
    "$FIX_REDUNDANT_ACCESSOR_MULTI" \
    "fixture.rs:1:redundant-accessor" \
    0

# ---------------------------------------------------------------------------
# Test: non-accessor body (more than just `self.field`) is NOT flagged.
# ---------------------------------------------------------------------------
read -r -d '' FIX_NON_ACCESSOR <<'RUST' || true
/// Run a computation.
pub fn compute(&self) -> u32 {
    self.value + 1
}
RUST
run_case \
    "non-accessor body is not flagged as redundant-accessor" \
    "$FIX_NON_ACCESSOR" \
    "" \
    0

# ---------------------------------------------------------------------------
# Test: exit status is always 0 even when findings exist (warn-only).
# ---------------------------------------------------------------------------
run_case \
    "exit status is 0 even with findings (warn-only)" \
    "$FIX_REDUNDANT_ACCESSOR" \
    "fixture.rs:1:redundant-accessor" \
    0

# ---------------------------------------------------------------------------
# Test: clean fixture produces no findings.
# ---------------------------------------------------------------------------
read -r -d '' FIX_CLEAN <<'RUST' || true
//! Module doc.

/// Add two numbers together.
pub fn add(a: u32, b: u32) -> u32 {
    a + b
}
RUST
run_case \
    "clean fixture produces no findings" \
    "$FIX_CLEAN" \
    "" \
    0

# ---------------------------------------------------------------------------
echo
echo "----------------------------------------"
echo "passed: $PASS  failed: $FAIL"
if [[ $FAIL -gt 0 ]]; then
    echo
    echo "failed tests:"
    for t in "${FAILED_TESTS[@]}"; do
        echo "  - $t"
    done
    exit 1
fi
