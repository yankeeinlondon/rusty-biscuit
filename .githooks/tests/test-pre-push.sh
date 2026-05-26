#!/usr/bin/env bash
# Level 1 tests for .githooks/pre-push.
#
# Exercises the hook with a fake `just` on PATH so we can assert exit codes
# and stderr/stdout content for every documented behavior in R3 of
# features/2026-05-19-ci-cd/spec.md:
#
#   - off / warn / strict modes
#   - invalid mode rejection
#   - warn-mode failure exits 0 and mentions --no-verify
#   - strict-mode failure exits non-zero and mentions --no-verify
#   - RUSTY_BISCUIT_PRE_PUSH_AREAS override is honored verbatim
#   - changed-areas empty result falls back to "claudine darkmatter"
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
# Usage: make_fake_just <tmpdir> <pre_push_exit_code> [changed_areas_output]
#
# The fake records every invocation (one per line) to "$tmpdir/just.log"
# with the arguments space-joined. It exits 0 for `changed-areas` and the
# requested exit code for `pre-push`. Any other subcommand exits 99 so
# unexpected calls show up as test failures.
make_fake_just() {
    local tmpdir="$1"
    local pre_push_exit="$2"
    local changed_areas_out="${3:-}"

    cat >"$tmpdir/just" <<EOF
#!/usr/bin/env bash
echo "\$*" >>"$tmpdir/just.log"
case "\$1" in
    changed-areas)
        printf '%s' "$changed_areas_out"
        exit 0
        ;;
    pre-push)
        exit $pre_push_exit
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

test_off_mode_skips_tests() {
    local tmpdir="$1"
    make_fake_just "$tmpdir" 0
    run_hook "$tmpdir" "off"
    assert_exit "off" "$tmpdir" 0 || return 1
    assert_log_absent "off" "$tmpdir" || return 1
}

test_invalid_mode_exits_one() {
    local tmpdir="$1"
    make_fake_just "$tmpdir" 0
    run_hook "$tmpdir" "bogus"
    assert_exit "invalid" "$tmpdir" 1 || return 1
    assert_contains "invalid mode message" "$tmpdir/err" "Unknown RUSTY_BISCUIT_PRE_PUSH" || return 1
    assert_contains "invalid mode red SGR prefix" "$tmpdir/err" $'\033[31mUnknown RUSTY_BISCUIT_PRE_PUSH' || return 1
    assert_contains "valid values listed" "$tmpdir/err" "off, warn, or strict" || return 1
}

test_warn_passing_tests_exits_zero() {
    local tmpdir="$1"
    make_fake_just "$tmpdir" 0
    run_hook "$tmpdir" "warn"
    assert_exit "warn pass" "$tmpdir" 0 || return 1
    assert_contains "warn pass message" "$tmpdir/out" "Pre-push tests passed." || return 1
}

test_warn_failing_tests_exits_zero_with_no_verify_hint() {
    local tmpdir="$1"
    make_fake_just "$tmpdir" 1
    run_hook "$tmpdir" "warn"
    assert_exit "warn fail" "$tmpdir" 0 || return 1
    assert_contains "warn failure header" "$tmpdir/out" "Pre-push tests failed" || return 1
    # R3 requires warn-mode failures to print prominently in red. The hook
    # emits a literal ANSI SGR red prefix (\033[31m) and reset (\033[0m)
    # regardless of TTY status, so a byte-level check on the captured
    # stdout is the contract under test here.
    assert_contains "warn failure red SGR prefix" "$tmpdir/out" $'\033[31mPre-push tests failed' || return 1
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
    assert_contains "strict failure header" "$tmpdir/out" "Pre-push tests failed" || return 1
    # Same red-styling contract as warn mode (see test above for rationale).
    assert_contains "strict failure red SGR prefix" "$tmpdir/out" $'\033[31mPre-push tests failed' || return 1
    assert_contains "strict failure SGR reset" "$tmpdir/out" $'\033[0m' || return 1
    assert_contains "strict no-verify hint" "$tmpdir/err" "--no-verify" || return 1
}

test_areas_override_is_passed_through() {
    local tmpdir="$1"
    make_fake_just "$tmpdir" 0
    run_hook "$tmpdir" "warn" "biscuit-file sniff"
    assert_exit "override" "$tmpdir" 0 || return 1
    # Override path must skip `just changed-areas` entirely and pass the
    # override straight to `just pre-push`.
    assert_not_contains "no changed-areas call" "$tmpdir/just.log" "changed-areas" || return 1
    assert_log_has "override forwarded" "$tmpdir" "pre-push biscuit-file sniff" || return 1
}

test_no_upstream_falls_back_to_curated_defaults() {
    local tmpdir="$1"
    # changed-areas produces empty output (simulates no upstream)
    make_fake_just "$tmpdir" 0 ""
    run_hook "$tmpdir" "warn"
    assert_exit "fallback" "$tmpdir" 0 || return 1
    assert_log_has "changed-areas attempted" "$tmpdir" "changed-areas" || return 1
    assert_log_has "fallback to claudine darkmatter" "$tmpdir" "pre-push claudine darkmatter" || return 1
}

test_changed_areas_result_is_used() {
    local tmpdir="$1"
    make_fake_just "$tmpdir" 0 "darkmatter sniff"
    run_hook "$tmpdir" "warn"
    assert_exit "changed-areas used" "$tmpdir" 0 || return 1
    assert_log_has "changed-areas piped to pre-push" "$tmpdir" "pre-push darkmatter sniff" || return 1
}

# ---------- runner ----------

echo ""
echo "${C_DIM}Running pre-push hook tests against${C_RESET} $HOOK"
echo ""

run_test "off mode skips tests entirely"                              test_off_mode_skips_tests
run_test "invalid mode exits 1 with helpful message"                  test_invalid_mode_exits_one
run_test "warn + passing → exit 0"                                    test_warn_passing_tests_exits_zero
run_test "warn + failing → exit 0 and mentions --no-verify"           test_warn_failing_tests_exits_zero_with_no_verify_hint
run_test "strict + passing → exit 0"                                  test_strict_passing_tests_exits_zero
run_test "strict + failing → propagate exit and mention --no-verify"  test_strict_failing_tests_exits_nonzero_with_no_verify_hint
run_test "RUSTY_BISCUIT_PRE_PUSH_AREAS override is honored verbatim"  test_areas_override_is_passed_through
run_test "empty changed-areas falls back to claudine darkmatter"      test_no_upstream_falls_back_to_curated_defaults
run_test "non-empty changed-areas output is forwarded to pre-push"    test_changed_areas_result_is_used

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
