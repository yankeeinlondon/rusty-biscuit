#!/usr/bin/env bash
# Level 1 tests for the `just changed-areas` recipe in the root justfile.
#
# Exercises the real recipe (not a shim) so regressions in the upstream
# detection, top-level-segment heuristic, or curated area filter cannot
# silently change which test suites the pre-push hook runs.
#
# Each scenario builds a fresh temporary git repository, optionally
# configures a local upstream, commits files in known paths, then invokes
# the recipe via `just --justfile <REPO_ROOT/justfile>` with the temp
# repo as the shell cwd. The recipe is decorated with `[no-cd]` so the
# shell cwd is honored.
#
# Scenarios covered (match the suggested fix in
# features/2026-05-19-ci-cd/review-1.md):
#
#   - No upstream branch -> empty output (caller falls back)
#   - Single mapped area
#   - Multiple mapped areas -> curated-order output
#   - Unmapped top-level path only -> empty output
#   - Mixed mapped + unmapped -> only the mapped area appears
#
# Run directly: ./.githooks/tests/test-changed-areas.sh

set -u
set -o pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
JUSTFILE="$REPO_ROOT/justfile"

if [ ! -f "$JUSTFILE" ]; then
    echo "FAIL: cannot locate root justfile at $JUSTFILE" >&2
    exit 2
fi
if ! command -v just >/dev/null 2>&1; then
    echo "FAIL: \`just\` is not on PATH" >&2
    exit 2
fi
if ! command -v git >/dev/null 2>&1; then
    echo "FAIL: \`git\` is not on PATH" >&2
    exit 2
fi

PASS=0
FAIL=0
FAILED_TESTS=()

# Colors (only when stdout is a terminal).
if [ -t 1 ]; then
    C_GREEN=$'\033[32m'
    C_RED=$'\033[31m'
    C_DIM=$'\033[2m'
    C_RESET=$'\033[0m'
else
    C_GREEN=""
    C_RED=""
    C_DIM=""
    C_RESET=""
fi

# Seed a temp repo on `main` with the curated area directories present,
# then create (but do NOT set upstream on) a `feature` branch. Individual
# tests configure upstream and commit files as needed.
init_repo() {
    local tmpdir="$1"
    (
        cd "$tmpdir"
        git init -q -b main .
        git config user.email "tests@example.com"
        git config user.name "tests"
        git config commit.gpgsign false
        mkdir -p claudine darkmatter sniff biscuit-file notes
        : >claudine/.keep
        : >darkmatter/.keep
        : >sniff/.keep
        : >biscuit-file/.keep
        : >notes/.keep
        echo "init" >README.md
        git add -A
        git commit -qm "init"
        git checkout -q -b feature
    )
}

# Point `feature` at local `main` as its upstream (no remote required).
configure_upstream() {
    local tmpdir="$1"
    (cd "$tmpdir" && git branch -q --set-upstream-to=main feature)
}

# Commit one file at the given path on the current branch.
commit_file() {
    local tmpdir="$1" path="$2"
    (
        cd "$tmpdir"
        mkdir -p "$(dirname "$path")"
        printf 'content\n' >>"$path"
        git add "$path"
        git commit -qm "add $path"
    )
}

# Invoke the real `changed-areas` recipe with the temp repo as the cwd.
# `HOME` is pointed at the temp dir so the developer's `~/.gitconfig`
# does not leak into the test environment.
run_changed_areas() {
    local tmpdir="$1"
    (cd "$tmpdir" && HOME="$tmpdir" just --justfile "$JUSTFILE" changed-areas)
}

assert_eq() {
    local label="$1" expected="$2" actual="$3"
    if [ "$actual" = "$expected" ]; then
        return 0
    fi
    echo "  $label" >&2
    echo "  expected: '$expected'" >&2
    echo "  actual:   '$actual'" >&2
    return 1
}

run_test() {
    local name="$1"
    shift
    local tmpdir
    tmpdir=$(mktemp -d)
    if "$@" "$tmpdir"; then
        echo "${C_GREEN}PASS${C_RESET} ${name}"
        PASS=$((PASS + 1))
    else
        echo "${C_RED}FAIL${C_RESET} ${name}"
        FAIL=$((FAIL + 1))
        FAILED_TESTS+=("$name")
    fi
    rm -rf "$tmpdir"
}

# ---------- test cases ----------

test_no_upstream_outputs_empty() {
    local tmpdir="$1"
    init_repo "$tmpdir"
    # No upstream configured on `feature`; recipe must exit 0 with no output.
    local out
    out=$(run_changed_areas "$tmpdir")
    assert_eq "no upstream" "" "$out" || return 1
}

test_single_mapped_area() {
    local tmpdir="$1"
    init_repo "$tmpdir"
    configure_upstream "$tmpdir"
    commit_file "$tmpdir" "claudine/src/lib.rs"
    local out
    out=$(run_changed_areas "$tmpdir")
    assert_eq "single mapped area" "claudine" "$out" || return 1
}

test_multiple_mapped_areas_curated_order() {
    local tmpdir="$1"
    init_repo "$tmpdir"
    configure_upstream "$tmpdir"
    commit_file "$tmpdir" "claudine/src/lib.rs"
    commit_file "$tmpdir" "darkmatter/src/lib.rs"
    # The curated `areas` list in the root justfile places `darkmatter`
    # ahead of `claudine`, and the recipe emits matches in curated order.
    local out
    out=$(run_changed_areas "$tmpdir")
    assert_eq "multiple mapped areas" "darkmatter claudine" "$out" || return 1
}

test_unmapped_path_outputs_empty() {
    local tmpdir="$1"
    init_repo "$tmpdir"
    configure_upstream "$tmpdir"
    commit_file "$tmpdir" "notes/random.md"
    commit_file "$tmpdir" "top-level.md"
    local out
    out=$(run_changed_areas "$tmpdir")
    assert_eq "unmapped paths" "" "$out" || return 1
}

test_mixed_paths_returns_only_mapped() {
    local tmpdir="$1"
    init_repo "$tmpdir"
    configure_upstream "$tmpdir"
    commit_file "$tmpdir" "claudine/src/lib.rs"
    commit_file "$tmpdir" "notes/random.md"
    local out
    out=$(run_changed_areas "$tmpdir")
    assert_eq "mixed paths" "claudine" "$out" || return 1
}

# ---------- runner ----------

echo ""
echo "${C_DIM}Running \`just changed-areas\` tests against${C_RESET} $JUSTFILE"
echo ""

run_test "no upstream branch -> empty output"              test_no_upstream_outputs_empty
run_test "single mapped area -> area name"                 test_single_mapped_area
run_test "multiple mapped areas -> curated order"          test_multiple_mapped_areas_curated_order
run_test "unmapped top-level paths -> empty"               test_unmapped_path_outputs_empty
run_test "mixed mapped + unmapped -> only mapped"          test_mixed_paths_returns_only_mapped

echo ""
echo "================================================"
echo "changed-areas test summary"
echo "================================================"
echo "${C_GREEN}Passed${C_RESET}: $PASS"
if [ "$FAIL" -gt 0 ]; then
    echo "${C_RED}Failed${C_RESET}: $FAIL"
    for t in "${FAILED_TESTS[@]}"; do
        echo "  - $t"
    done
    echo "================================================"
    exit 1
fi
echo "${C_RED}Failed${C_RESET}: 0"
echo "================================================"
exit 0
