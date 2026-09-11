#!/usr/bin/env bash
# Level 1 tests for .githooks/pre-push.
#
# Exercises the hook with a fake `just` on PATH so we can assert exit codes
# and stderr/stdout content for every documented behavior in R3 of
# features/2026-05-19-ci-cd/spec.md:
#
#   - scope-only / warn / strict modes, and `off` as a deprecated alias
#   - invalid mode rejection
#   - warn-mode failure exits 0 and mentions --no-verify
#   - strict-mode failure exits non-zero and mentions --no-verify
#   - the unset default is strict
#   - the default path delegates to `just pre-push` with no selection
#   - RUSTY_BISCUIT_PRE_PUSH_AREAS override is forwarded verbatim
#
# plus the execution-constraint boundary of fixes/2026-09-11-cicd-cleanup
# (AC17): an unexpired prohibition blocks the push and states its reason; an
# expired one is announced and does not.
#
# Run directly: ./.githooks/tests/test-pre-push.sh

set -u
set -o pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
HOOK="$REPO_ROOT/.githooks/pre-push"

if [ ! -x "$HOOK" ]; then
    echo "FAIL: hook not executable at $HOOK" >&2
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

# Create a fake `just` shim in a fresh temp dir for each test case.
#
# Usage: make_fake_just <tmpdir> <pre_push_exit_code>
#
# The fake records every invocation (one per line) to "$tmpdir/just.log"
# with the arguments space-joined. It exits with the requested code for
# `pre-push`, and 0 for `ci-local` (the scope-only path, which runs no gate).
# Any other subcommand exits 99 so unexpected calls show up as test failures.
make_fake_just() {
    local tmpdir="$1"
    local pre_push_exit="$2"

    cat >"$tmpdir/just" <<EOF
#!/usr/bin/env bash
echo "\$*" >>"$tmpdir/just.log"
case "\$1" in
    pre-push)
        exit $pre_push_exit
        ;;
    ci-local)
        exit 0
        ;;
    *)
        echo "fake just: unexpected subcommand: \$*" >&2
        exit 99
        ;;
esac
EOF
    chmod +x "$tmpdir/just"
}

# Run the hook with a controlled PATH and capture exit code + output.
#
# Usage: run_hook <tmpdir> <mode> [areas_override]
#
# Writes stdout to $tmpdir/out and stderr to $tmpdir/err. Always returns 0
# itself so set -e in callers does not abort on a non-zero hook exit; the
# real exit code is written to $tmpdir/exit.
run_hook() {
    local tmpdir="$1"
    local mode="$2"
    local areas="${3-__UNSET__}"

    local env_args=(
        "PATH=$tmpdir:/usr/bin:/bin"
        "RUSTY_BISCUIT_PRE_PUSH=$mode"
    )
    if [ "$areas" != "__UNSET__" ]; then
        env_args+=("RUSTY_BISCUIT_PRE_PUSH_AREAS=$areas")
    fi

    # Use `env -i` so the hook does not inherit the developer's
    # RUSTY_BISCUIT_PRE_PUSH_* values from the surrounding shell.
    env -i "${env_args[@]}" "$HOOK" >"$tmpdir/out" 2>"$tmpdir/err"
    echo $? >"$tmpdir/exit"
}

# Assertion helpers.
assert_exit() {
    local label="$1" tmpdir="$2" expected="$3"
    local actual
    actual=$(cat "$tmpdir/exit")
    if [ "$actual" = "$expected" ]; then
        return 0
    fi
    echo "  expected exit $expected, got $actual" >&2
    echo "  --- stdout ---" >&2
    sed 's/^/  /' "$tmpdir/out" >&2
    echo "  --- stderr ---" >&2
    sed 's/^/  /' "$tmpdir/err" >&2
    return 1
}

assert_contains() {
    local label="$1" file="$2" needle="$3"
    if grep -qF -- "$needle" "$file"; then
        return 0
    fi
    echo "  expected '$file' to contain: $needle" >&2
    echo "  --- contents ---" >&2
    sed 's/^/  /' "$file" >&2
    return 1
}

assert_not_contains() {
    local label="$1" file="$2" needle="$3"
    if ! grep -qF -- "$needle" "$file"; then
        return 0
    fi
    echo "  expected '$file' NOT to contain: $needle" >&2
    return 1
}

assert_log_has() {
    local label="$1" tmpdir="$2" needle="$3"
    if [ -f "$tmpdir/just.log" ] && grep -qF -- "$needle" "$tmpdir/just.log"; then
        return 0
    fi
    echo "  expected fake-just log to contain: $needle" >&2
    if [ -f "$tmpdir/just.log" ]; then
        echo "  --- just.log ---" >&2
        sed 's/^/  /' "$tmpdir/just.log" >&2
    else
        echo "  (just.log was never written — hook did not call just)" >&2
    fi
    return 1
}

assert_log_absent() {
    local label="$1" tmpdir="$2"
    if [ ! -f "$tmpdir/just.log" ] || [ ! -s "$tmpdir/just.log" ]; then
        return 0
    fi
    echo "  expected fake-just to NOT be called, but it was:" >&2
    sed 's/^/  /' "$tmpdir/just.log" >&2
    return 1
}

# Each test is wrapped so a single failure does not abort the whole suite.
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

assert_log_lacks() {
    local label="$1" tmpdir="$2" needle="$3"
    if [ ! -f "$tmpdir/just.log" ] || ! grep -qF -- "$needle" "$tmpdir/just.log"; then
        return 0
    fi
    echo "  expected fake-just NOT to be called with: $needle" >&2
    sed 's/^/  /' "$tmpdir/just.log" >&2
    return 1
}

test_scope_only_runs_no_gate_but_still_resolves_scope() {
    local tmpdir="$1"
    make_fake_just "$tmpdir" 7
    run_hook "$tmpdir" "scope-only"
    assert_exit "scope-only" "$tmpdir" 0 || return 1
    # The distinction the mode exists for: scope is calculated and reviewable,
    # no gate runs, and a failing gate exit code cannot be inherited.
    assert_log_has "plan resolved" "$tmpdir" "ci-local --plan" || return 1
    assert_log_lacks "no gate" "$tmpdir" "pre-push" || return 1
}

test_off_mode_is_a_deprecated_alias_of_scope_only() {
    local tmpdir="$1"
    make_fake_just "$tmpdir" 7
    run_hook "$tmpdir" "off"
    assert_exit "off" "$tmpdir" 0 || return 1
    assert_contains "off deprecation notice" "$tmpdir/err" "deprecated" || return 1
    assert_contains "off names its replacement" "$tmpdir/err" "scope-only" || return 1
    assert_log_has "plan resolved" "$tmpdir" "ci-local --plan" || return 1
    assert_log_lacks "no gate" "$tmpdir" "pre-push" || return 1
}

test_invalid_mode_exits_one() {
    local tmpdir="$1"
    make_fake_just "$tmpdir" 0
    run_hook "$tmpdir" "bogus"
    assert_exit "invalid" "$tmpdir" 1 || return 1
    assert_contains "invalid mode message" "$tmpdir/err" "Unknown RUSTY_BISCUIT_PRE_PUSH" || return 1
    assert_contains "invalid mode red SGR prefix" "$tmpdir/err" $'\033[31mUnknown RUSTY_BISCUIT_PRE_PUSH' || return 1
    assert_contains "valid values listed" "$tmpdir/err" "off, warn, or strict" || return 1
    assert_contains "alias explained" "$tmpdir/err" "scope-only" || return 1
}

test_warn_passing_tests_exits_zero() {
    local tmpdir="$1"
    make_fake_just "$tmpdir" 0
    run_hook "$tmpdir" "warn"
    assert_exit "warn pass" "$tmpdir" 0 || return 1
    assert_contains "warn pass message" "$tmpdir/out" "Pre-push validation passed." || return 1
}

test_warn_failing_tests_exits_zero_with_no_verify_hint() {
    local tmpdir="$1"
    make_fake_just "$tmpdir" 1
    run_hook "$tmpdir" "warn"
    assert_exit "warn fail" "$tmpdir" 0 || return 1
    assert_contains "warn failure header" "$tmpdir/out" "Pre-push validation failed" || return 1
    # R3 requires warn-mode failures to print prominently in red. The hook
    # emits a literal ANSI SGR red prefix (\033[31m) and reset (\033[0m)
    # regardless of TTY status, so a byte-level check on the captured
    # stdout is the contract under test here.
    assert_contains "warn failure red SGR prefix" "$tmpdir/out" $'\033[31mPre-push validation failed' || return 1
    assert_contains "warn failure SGR reset" "$tmpdir/out" $'\033[0m' || return 1
    assert_contains "warn no-verify hint" "$tmpdir/err" "--no-verify" || return 1
}

test_strict_passing_tests_exits_zero() {
    local tmpdir="$1"
    make_fake_just "$tmpdir" 0
    run_hook "$tmpdir" "strict"
    assert_exit "strict pass" "$tmpdir" 0 || return 1
}

test_strict_failing_tests_exits_nonzero_with_no_verify_hint() {
    local tmpdir="$1"
    make_fake_just "$tmpdir" 7
    run_hook "$tmpdir" "strict"
    assert_exit "strict fail" "$tmpdir" 7 || return 1
    assert_contains "strict failure header" "$tmpdir/out" "Pre-push validation failed" || return 1
    # Same red-styling contract as warn mode (see test above for rationale).
    assert_contains "strict failure red SGR prefix" "$tmpdir/out" $'\033[31mPre-push validation failed' || return 1
    assert_contains "strict failure SGR reset" "$tmpdir/out" $'\033[0m' || return 1
    assert_contains "strict no-verify hint" "$tmpdir/err" "--no-verify" || return 1
}

test_unset_default_is_strict() {
    local tmpdir="$1"
    make_fake_just "$tmpdir" 7
    env -i "PATH=$tmpdir:/usr/bin:/bin" "$HOOK" >"$tmpdir/out" 2>"$tmpdir/err"
    echo $? >"$tmpdir/exit"
    assert_exit "default strict" "$tmpdir" 7 || return 1
}

test_default_delegates_scope_to_pre_push() {
    local tmpdir="$1"
    make_fake_just "$tmpdir" 0
    run_hook "$tmpdir" "warn"
    assert_exit "default" "$tmpdir" 0 || return 1
    # The hook computes no scope of its own: exactly one `just` call, and it
    # is `pre-push` with no arguments.
    assert_log_has "pre-push called" "$tmpdir" "pre-push" || return 1
    if [ "$(wc -l <"$tmpdir/just.log" | tr -d ' ')" != "1" ] || [ "$(cat "$tmpdir/just.log")" != "pre-push" ]; then
        echo "  expected exactly one call 'pre-push', got:" >&2
        sed 's/^/  /' "$tmpdir/just.log" >&2
        return 1
    fi
}

test_areas_override_is_passed_through() {
    local tmpdir="$1"
    make_fake_just "$tmpdir" 0
    run_hook "$tmpdir" "warn" "biscuit-file sniff"
    assert_exit "override" "$tmpdir" 0 || return 1
    assert_log_has "override forwarded" "$tmpdir" "pre-push biscuit-file sniff" || return 1
    assert_contains "override announced" "$tmpdir/out" "biscuit-file sniff" || return 1
}

# ---------- execution constraints (fixes/2026-09-11-cicd-cleanup, AC17) ------
#
# Phase 2 froze these while the hook still read no constraint store; Phase 4
# built it, so they now run directly.

# Run the hook with a passing fake `just` plus extra environment assignments.
run_hook_with_env() {
    local tmpdir="$1"
    shift
    env -i "PATH=$tmpdir:/usr/bin:/bin" "RUSTY_BISCUIT_PRE_PUSH=strict" "$@" \
        "$HOOK" >"$tmpdir/out" 2>"$tmpdir/err"
    echo $? >"$tmpdir/exit"
}

# The constraint store's LOCATION is Open Question 2 (recommendation: a
# per-branch file under ~/.rusty-biscuit/ci-constraints/). Only this helper
# depends on that choice; the contracts below assert the observable refusal,
# which holds under every option.
write_prohibition() {
    local dir="$1" environment="$2" expiry="$3"
    mkdir -p "$dir"
    cat >"$dir/current.json" <<EOF
{
  "environment": "$environment",
  "reason": "do not rerun WSL for this branch",
  "owner": "ken",
  "expiry": "$expiry"
}
EOF
}

test_prohibition_blocks_the_push() {
    local tmpdir="$1"
    make_fake_just "$tmpdir" 0
    write_prohibition "$tmpdir/constraints" "wsl2-ubuntu" "2099-01-01"
    run_hook_with_env "$tmpdir" "BISCUIT_CI_CONSTRAINTS_DIR=$tmpdir/constraints"

    if [ "$(cat "$tmpdir/exit")" = "0" ]; then
        echo "the hook never consulted the constraint store: it exited 0 with an" \
             "unexpired wsl2-ubuntu prohibition in place"
        return 1
    fi
    assert_contains "prohibition named" "$tmpdir/err" "wsl2-ubuntu" || return 1
}

test_prohibition_is_explained() {
    local tmpdir="$1"
    make_fake_just "$tmpdir" 0
    write_prohibition "$tmpdir/constraints" "wsl2-ubuntu" "2099-01-01"
    run_hook_with_env "$tmpdir" "BISCUIT_CI_CONSTRAINTS_DIR=$tmpdir/constraints"

    if ! grep -qF -- "do not rerun WSL for this branch" "$tmpdir/err"; then
        echo "the hook never consulted the constraint store: its refusal does not" \
             "state the recorded reason, owner, or affected environment"
        return 1
    fi
}

test_expired_prohibition_does_not_block() {
    local tmpdir="$1"
    make_fake_just "$tmpdir" 0
    write_prohibition "$tmpdir/constraints" "wsl2-ubuntu" "2020-01-01"
    run_hook_with_env "$tmpdir" "BISCUIT_CI_CONSTRAINTS_DIR=$tmpdir/constraints"

    # An expiry that has passed must not block, and the hook must say it
    # ignored an expired constraint rather than silently dropping it.
    if ! grep -qiF -- "expired" "$tmpdir/out" "$tmpdir/err"; then
        echo "the hook never consulted the constraint store: an expired" \
             "prohibition is neither honored nor reported"
        return 1
    fi
    assert_exit "expired prohibition" "$tmpdir" 0 || return 1
}

# A failing run reaches the publication block — that is the Phase 4 change
# (spec section 3.5) — but this hook process pushes no ref, so nothing is
# published. The distinction the fixture guards is that the failure text is
# printed AFTER the block rather than short-circuiting past it, which is what
# makes a complete failing cell publishable at all.
test_a_failing_warn_run_reaches_the_publication_block() {
    local tmpdir="$1"
    make_fake_just "$tmpdir" 1
    run_hook "$tmpdir" "warn"
    assert_exit "warn failure" "$tmpdir" 0 || return 1
    assert_contains "failure still reported" "$tmpdir/out" "Pre-push validation failed" || return 1
    # No outgoing head in this fixture (the hook reads no ref lines on stdin),
    # so the guard declines before any note is written.
    assert_not_contains "no evidence published" "$tmpdir/out" "Published" || return 1
}

# The publication guard is one conjunction; each clause is load-bearing and
# each is asserted in the hook source so a refactor cannot drop one silently.
test_publication_requires_a_clean_exact_tree_and_no_override() {
    local tmpdir="$1"
    local guards=(
        'PUSHES_HEAD" -eq 1'
        '-z "$SELECTION"'
        'git status --porcelain'
        '-s "$PLAN_FILE"'
    )
    local guard
    for guard in "${guards[@]}"; do
        if ! grep -qF -- "$guard" "$HOOK"; then
            echo "  the publication guard no longer requires: $guard" >&2
            return 1
        fi
    done
}

test_a_blocked_strict_push_records_locally_but_publishes_nothing() {
    if ! grep -qF -- 'not published while the push is blocked' "$HOOK"; then
        echo "  strict mode must not push a notes ref for a branch it just blocked" >&2
        return 1
    fi
}

# ---------- runner ----------

echo ""
echo "${C_DIM}Running pre-push hook tests against${C_RESET} $HOOK"
echo ""

run_test "scope-only runs no gate but resolves scope"                 test_scope_only_runs_no_gate_but_still_resolves_scope
run_test "off is a deprecated alias of scope-only"                    test_off_mode_is_a_deprecated_alias_of_scope_only
run_test "invalid mode exits 1 with helpful message"                  test_invalid_mode_exits_one
run_test "warn + passing → exit 0"                                    test_warn_passing_tests_exits_zero
run_test "warn + failing → exit 0 and mentions --no-verify"           test_warn_failing_tests_exits_zero_with_no_verify_hint
run_test "strict + passing → exit 0"                                  test_strict_passing_tests_exits_zero
run_test "strict + failing → propagate exit and mention --no-verify"  test_strict_failing_tests_exits_nonzero_with_no_verify_hint
run_test "unset default is strict"                                    test_unset_default_is_strict
run_test "default path calls 'just pre-push' with no selection"       test_default_delegates_scope_to_pre_push
run_test "RUSTY_BISCUIT_PRE_PUSH_AREAS override is forwarded verbatim" test_areas_override_is_passed_through
run_test "a failing warn run reaches the publication block"           test_a_failing_warn_run_reaches_the_publication_block
run_test "publication requires a clean, exact, unoverridden tree"     test_publication_requires_a_clean_exact_tree_and_no_override
run_test "a blocked strict push records locally, publishes nothing"   test_a_blocked_strict_push_records_locally_but_publishes_nothing
run_test "a persisted prohibition blocks the push"                    test_prohibition_blocks_the_push
run_test "a prohibition refusal states its reason"                    test_prohibition_is_explained
run_test "an expired prohibition does not block"                      test_expired_prohibition_does_not_block

echo ""
echo "================================================"
echo "pre-push hook test summary"
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
