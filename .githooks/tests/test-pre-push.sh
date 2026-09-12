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
# (AC17): the hook resolves the outgoing plan first and decides every
# prohibition from it — an executing prohibited cell blocks and states its
# reason, a reused or absent one does not, an expired record is announced,
# and an unreadable plan blocks.
#
# and the evidence-retention contract of its review-1: a published receipt names
# a report directory that still exists after the hook exits, with every report
# it lists readable there, and a failed copy publishes no receipt.
#
# and the scope-evidence contract of fixes/2026-09-10-local-affected-scope
# (R1/R2): every mode publishes the COMMITTED scope of the outgoing head on
# refs/notes/ci-local/scope before any gate, dirty tree or not, against the
# base the CI event will compare with; a publication failure is named as such
# and never changes the exit code. The planner inside those fixture
# repositories is `fixtures/affected_scope_stub.py` (no Cargo workspace there);
# the receipt and note plumbing is the real `local_evidence.py`.
#
# Run directly: ./.githooks/tests/test-pre-push.sh

set -u
set -o pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
HOOK="$REPO_ROOT/.githooks/pre-push"
FIXTURES="$REPO_ROOT/.githooks/tests/fixtures"

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
# Usage: make_fake_just <tmpdir> <pre_push_exit_code> [plan_fixture]
#
# The fake records every invocation (one per line) to "$tmpdir/just.log"
# with the arguments space-joined. It exits with the requested code for
# `pre-push`. `ci-local` (the plan resolution, which runs no gate) records the
# store directory it was handed in "$tmpdir/ci-local.env", then exits with the
# code in "$tmpdir/ci-local.exit" if a test wrote one; otherwise it copies
# `plan_fixture` — by default a plan that EXECUTES a wsl2-ubuntu cell — to the
# path named by `--plan-out` and exits 0. Any other subcommand exits 99 so
# unexpected calls show up as test failures.
make_fake_just() {
    local tmpdir="$1"
    local pre_push_exit="$2"
    local plan_fixture="${3:-$FIXTURES/plan-wsl-executing.json}"

    cat >"$tmpdir/just" <<EOF
#!/usr/bin/env bash
echo "\$*" >>"$tmpdir/just.log"
case "\$1" in
    pre-push)
        # A test that needs the gate to leave something behind (a plan, staged
        # reports) drops a script here; it runs with the hook's exports.
        if [ -f "$tmpdir/pre-push.hook" ]; then
            . "$tmpdir/pre-push.hook"
        fi
        exit $pre_push_exit
        ;;
    ci-local)
        echo "BISCUIT_CI_CONSTRAINTS_DIR=\${BISCUIT_CI_CONSTRAINTS_DIR-<unset>}" >"$tmpdir/ci-local.env"
        if [ -f "$tmpdir/ci-local.exit" ]; then
            exit "\$(cat "$tmpdir/ci-local.exit")"
        fi
        while [ \$# -gt 0 ]; do
            if [ "\$1" = "--plan-out" ]; then
                cp "$plan_fixture" "\$2"
            fi
            shift
        done
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
    # RUSTY_BISCUIT_PRE_PUSH_* values from the surrounding shell. Git feeds a
    # hook its ref lines on stdin; an empty stdin models a push of no ref, and
    # an inherited one can leave the hook blocked in its ref loop.
    env -i "${env_args[@]}" "$HOOK" </dev/null >"$tmpdir/out" 2>"$tmpdir/err"
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

# `needle` is matched at the start of a logged call, i.e. as the subcommand:
# a temp path such as biscuit-pre-push-review.XXXXXX may appear in any call.
assert_log_lacks() {
    local label="$1" tmpdir="$2" needle="$3"
    if [ ! -f "$tmpdir/just.log" ] || ! grep -q -- "^$needle" "$tmpdir/just.log"; then
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
    env -i "PATH=$tmpdir:/usr/bin:/bin" "$HOOK" </dev/null >"$tmpdir/out" 2>"$tmpdir/err"
    echo $? >"$tmpdir/exit"
    assert_exit "default strict" "$tmpdir" 7 || return 1
}

test_default_delegates_scope_to_pre_push() {
    local tmpdir="$1"
    make_fake_just "$tmpdir" 0
    run_hook "$tmpdir" "warn"
    assert_exit "default" "$tmpdir" 0 || return 1
    # The hook computes no scope of its own: the plan resolution, then exactly
    # one gate call, and it is `pre-push` with no arguments.
    assert_log_has "pre-push called" "$tmpdir" "pre-push" || return 1
    if [ "$(wc -l <"$tmpdir/just.log" | tr -d ' ')" != "2" ] \
        || ! sed -n 1p "$tmpdir/just.log" | grep -q '^ci-local --plan --plan-out ' \
        || [ "$(sed -n 2p "$tmpdir/just.log")" != "pre-push" ]; then
        echo "  expected 'ci-local --plan --plan-out <file>' then exactly 'pre-push', got:" >&2
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
    # The override narrows the local gates only; the plan a push triggers is
    # CI's computed scope, so the constraint decision must not see it.
    if grep -F -- "ci-local" "$tmpdir/just.log" | grep -qF -- "biscuit-file"; then
        echo "  the plan resolution was narrowed by the selection override:" >&2
        sed 's/^/  /' "$tmpdir/just.log" >&2
        return 1
    fi
}

# ---------- execution constraints (fixes/2026-09-11-cicd-cleanup, AC17) ------
#
# Phase 2 froze these while the hook still read no constraint store; Phase 4
# built it, so they now run directly. The fake `just` writes a plan that
# executes a wsl2-ubuntu cell unless a test hands it another fixture, so each
# refusal below is decided from a resolved plan, exactly as a real push is.

# Run the hook with a passing fake `just` plus extra environment assignments.
run_hook_with_env() {
    local tmpdir="$1"
    shift
    env -i "PATH=$tmpdir:/usr/bin:/bin" "RUSTY_BISCUIT_PRE_PUSH=strict" "$@" \
        "$HOOK" </dev/null >"$tmpdir/out" 2>"$tmpdir/err"
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

# The finding of review-1: a prohibition is decided from the RESOLVED plan, so
# a forbidden environment whose cells are already reused, or which the scope
# never reaches, does not block. The store here forbids wsl2-ubuntu throughout.

test_a_reused_prohibited_cell_does_not_block() {
    local tmpdir="$1"
    make_fake_just "$tmpdir" 0 "$FIXTURES/plan-wsl-reused.json"
    write_prohibition "$tmpdir/constraints" "wsl2-ubuntu" "2099-01-01"
    run_hook_with_env "$tmpdir" "BISCUIT_CI_CONSTRAINTS_DIR=$tmpdir/constraints"
    assert_exit "reused prohibited cell" "$tmpdir" 0 || return 1
    assert_not_contains "no refusal" "$tmpdir/err" "Push blocked" || return 1
    assert_log_has "gates still ran" "$tmpdir" "pre-push" || return 1
}

test_an_absent_prohibited_environment_does_not_block() {
    local tmpdir="$1"
    make_fake_just "$tmpdir" 0 "$FIXTURES/plan-wsl-absent.json"
    write_prohibition "$tmpdir/constraints" "wsl2-ubuntu" "2099-01-01"
    run_hook_with_env "$tmpdir" "BISCUIT_CI_CONSTRAINTS_DIR=$tmpdir/constraints"
    assert_exit "absent prohibited environment" "$tmpdir" 0 || return 1
    assert_not_contains "no refusal" "$tmpdir/err" "Push blocked" || return 1
}

test_an_executing_prohibited_cell_blocks_with_its_reason() {
    local tmpdir="$1"
    make_fake_just "$tmpdir" 0 "$FIXTURES/plan-wsl-executing.json"
    write_prohibition "$tmpdir/constraints" "wsl2-ubuntu" "2099-01-01"
    run_hook_with_env "$tmpdir" "BISCUIT_CI_CONSTRAINTS_DIR=$tmpdir/constraints"
    assert_exit "executing prohibited cell" "$tmpdir" 1 || return 1
    assert_contains "environment named" "$tmpdir/err" "wsl2-ubuntu" || return 1
    assert_contains "reason stated" "$tmpdir/err" "do not rerun WSL for this branch" || return 1
    # Decided from the plan, before any gate: the refusal must precede pre-push.
    assert_log_has "plan resolved" "$tmpdir" "ci-local --plan --plan-out" || return 1
    assert_log_lacks "no gate after refusal" "$tmpdir" "pre-push" || return 1
}

test_the_constraint_is_decided_in_scope_only_mode_too() {
    local tmpdir="$1"
    make_fake_just "$tmpdir" 0 "$FIXTURES/plan-wsl-executing.json"
    write_prohibition "$tmpdir/constraints" "wsl2-ubuntu" "2099-01-01"
    env -i "PATH=$tmpdir:/usr/bin:/bin" "RUSTY_BISCUIT_PRE_PUSH=scope-only" \
        "BISCUIT_CI_CONSTRAINTS_DIR=$tmpdir/constraints" \
        "$HOOK" </dev/null >"$tmpdir/out" 2>"$tmpdir/err"
    echo $? >"$tmpdir/exit"
    assert_exit "scope-only refusal" "$tmpdir" 1 || return 1
    assert_contains "reason stated" "$tmpdir/err" "do not rerun WSL for this branch" || return 1
}

test_an_unreadable_plan_blocks() {
    local tmpdir="$1"
    printf '{not json' >"$tmpdir/garbage.json"
    make_fake_just "$tmpdir" 0 "$tmpdir/garbage.json"
    write_prohibition "$tmpdir/constraints" "wsl2-ubuntu" "2099-01-01"
    run_hook_with_env "$tmpdir" "BISCUIT_CI_CONSTRAINTS_DIR=$tmpdir/constraints"
    if [ "$(cat "$tmpdir/exit")" = "0" ]; then
        echo "an unreadable plan proves nothing, yet the push proceeded" >&2
        return 1
    fi
    assert_contains "plan named" "$tmpdir/err" "cannot read the resolved plan" || return 1
    assert_log_lacks "no gate after refusal" "$tmpdir" "pre-push" || return 1
}

test_a_failed_plan_resolution_blocks() {
    local tmpdir="$1"
    make_fake_just "$tmpdir" 0
    # A resolution that fails (unresolvable base ref, missing python3, ...)
    # is not a plan that schedules nothing.
    echo 2 >"$tmpdir/ci-local.exit"
    write_prohibition "$tmpdir/constraints" "wsl2-ubuntu" "2099-01-01"
    run_hook_with_env "$tmpdir" "BISCUIT_CI_CONSTRAINTS_DIR=$tmpdir/constraints"
    assert_exit "failed resolution" "$tmpdir" 2 || return 1
    assert_log_lacks "no gate after failure" "$tmpdir" "pre-push" || return 1
}

# The hook is the single decision point: the planner is handed no store, so it
# can neither refuse on a record scoped to another repository or branch nor
# hide an unsatisfied cell behind a `prohibited` state.
test_the_plan_resolution_is_handed_no_constraint_store() {
    local tmpdir="$1"
    make_fake_just "$tmpdir" 0
    write_prohibition "$tmpdir/constraints" "wsl2-ubuntu" "2099-01-01"
    run_hook_with_env "$tmpdir" "BISCUIT_CI_CONSTRAINTS_DIR=$tmpdir/constraints"
    assert_contains "store hidden from planner" "$tmpdir/ci-local.env" "BISCUIT_CI_CONSTRAINTS_DIR=" || return 1
    assert_not_contains "store hidden from planner" "$tmpdir/ci-local.env" "$tmpdir/constraints" || return 1
}

test_a_constraint_for_another_branch_does_not_block() {
    local tmpdir="$1"
    make_fake_just "$tmpdir" 0
    mkdir -p "$tmpdir/constraints"
    cat >"$tmpdir/constraints/current.json" <<EOF
{
  "environment": "wsl2-ubuntu",
  "reason": "do not rerun WSL for this branch",
  "owner": "ken",
  "expiry": "2099-01-01",
  "branch": "not-the-branch-under-test-$$"
}
EOF
    run_hook_with_env "$tmpdir" "BISCUIT_CI_CONSTRAINTS_DIR=$tmpdir/constraints"
    assert_exit "other branch" "$tmpdir" 0 || return 1
}

# The fixtures are what the fake `just` hands the hook; a fixture the planner
# would never write proves nothing about the real plan.
test_the_plan_fixtures_are_valid_resolved_plans() {
    local tmpdir="$1"
    python3 - "$REPO_ROOT" "$FIXTURES" <<'PYCHECK'
import json, sys
from pathlib import Path
root, fixtures = Path(sys.argv[1]), Path(sys.argv[2])
sys.path.insert(0, str(root / "scripts" / "ci"))
import schema
failed = False
for path in sorted(fixtures.glob("plan-*.json")):
    problems = schema.validate_resolved_plan(json.loads(path.read_text(encoding="utf-8")))
    if problems:
        failed = True
        print(f"  {path.name}: {problems}", file=sys.stderr)
sys.exit(1 if failed else 0)
PYCHECK
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

# ---------- retained reports (review-1: receipts must name reports that exist) --
#
# The hook stages JUnit reports in a temp dir it deletes on exit, and the
# receipt's `host.report_dir` must name where they still are afterwards. These
# fixtures satisfy the whole publication guard for real: a pushed head, a clean
# tree, a resolvable `origin/main`, a fake `sniff`, and the real `jq`/`python3`.

# A repository with `main` on a bare `origin`, one feature commit as HEAD, and
# the CI scripts committed so the hook's `$REPO_ROOT/scripts/ci/*.py` resolve.
# The planner is the stub: the real one needs the Cargo workspace it lives in.
make_publishing_repo() {
    local tmpdir="$1"
    local repo="$tmpdir/repo" bare="$tmpdir/origin.git"
    local commit=(git -c user.name=hook-test -c user.email=hook@example.com -c commit.gpgsign=false)
    git init -q -b main "$repo"
    mkdir -p "$repo/scripts/ci"
    cp "$REPO_ROOT"/scripts/ci/*.py "$repo/scripts/ci/"
    cp "$FIXTURES/affected_scope_stub.py" "$repo/scripts/ci/affected_scope.py"
    # As in the real repository: the scripts' bytecode cache must not dirty
    # the tree, or the clean-tree guard would withhold every receipt.
    echo "__pycache__/" >"$repo/.gitignore"
    echo "base" >"$repo/README.md"
    "${commit[@]}" -C "$repo" add -A
    "${commit[@]}" -C "$repo" commit -q -m "base"
    git init -q --bare "$bare"
    git -C "$repo" remote add origin "$bare"
    git -C "$repo" push -q origin main
    git -C "$repo" checkout -q -b feature
    echo "work" >"$repo/work.txt"
    "${commit[@]}" -C "$repo" add -A
    "${commit[@]}" -C "$repo" commit -q -m "head"
}

# The gate stages one passing L1 report for `alpha` and writes the plan the
# receipt is recorded against, exactly where the hook told it to.
make_staging_pre_push() {
    local tmpdir="$1"
    cat >"$tmpdir/pre-push.hook" <<EOF
cp "$FIXTURES/plan-macos-executing.json" "\$BISCUIT_CI_PLAN_OUT"
mkdir -p "\$BISCUIT_CI_REPORTS_OUT/L1"
printf '<?xml version="1.0"?><testsuites><testsuite name="alpha" tests="2" failures="0" errors="0" skipped="0"><testcase classname="alpha" name="one"/><testcase classname="alpha" name="two"/></testsuite></testsuites>' >"\$BISCUIT_CI_REPORTS_OUT/L1/alpha.xml"
printf '{"tier":"L1","package":"alpha","xml":"L1/alpha.xml","exit_code":0,"environment":"macos-latest","duration_s":3,"report_present":true}\\n' >"\$BISCUIT_CI_REPORTS_OUT/manifest.jsonl"
EOF
}

# The tools a publishing run needs on its PATH: a passing fake `just` that
# stages one macOS L1 report, a fake `sniff`, and the real `jq`, `python3`,
# `xargs`, and `grep`. The stub planner logs its invocations to planner.log.
stage_publishing_tools() {
    local tmpdir="$1"
    make_fake_just "$tmpdir" 0 "$FIXTURES/plan-macos-executing.json"
    make_staging_pre_push "$tmpdir"
    printf '#!/bin/sh\necho "{\\"os_type\\":\\"MacOS\\",\\"kernel\\":\\"Darwin\\"}"\n' >"$tmpdir/sniff"
    chmod +x "$tmpdir/sniff"
    ln -s "$(command -v jq)" "$tmpdir/jq"
    ln -s "$(command -v python3)" "$tmpdir/python3"
    mkdir -p "$tmpdir/home"
}

# Run the hook from the fixture repo as Git would: `origin` as the remote
# argument and one ref line on stdin whose local sha is HEAD, so PUSHES_HEAD
# is 1. The remote ref and sha are the caller's: they decide the scope base.
#
# Usage: run_hook_in_repo <tmpdir> <mode> <remote_ref> <remote_sha> [env...]
run_hook_in_repo() {
    local tmpdir="$1" mode="$2" remote_ref="$3" remote_sha="$4"
    shift 4
    local repo="$tmpdir/repo"
    local head
    head="$(git -C "$repo" rev-parse HEAD)"
    (
        cd "$repo" || exit 97
        printf 'refs/heads/feature %s %s %s\n' "$head" "$remote_ref" "$remote_sha" \
            | env -i "PATH=$tmpdir:/usr/bin:/bin" "HOME=$tmpdir/home" \
                "TEST_PLANNER_LOG=$tmpdir/planner.log" \
                "RUSTY_BISCUIT_PRE_PUSH=$mode" "$@" \
                "$HOOK" origin "$tmpdir/origin.git" >"$tmpdir/out" 2>"$tmpdir/err"
        echo $? >"$tmpdir/exit"
    )
}

# A strict run pushing a branch that does not yet exist on the remote.
run_publishing_hook() {
    local tmpdir="$1"
    shift
    stage_publishing_tools "$tmpdir"
    run_hook_in_repo "$tmpdir" strict refs/heads/feature \
        "0000000000000000000000000000000000000000" "$@"
}

test_a_published_receipt_names_retained_readable_reports() {
    local tmpdir="$1"
    make_publishing_repo "$tmpdir"
    run_publishing_hook "$tmpdir" "BISCUIT_CI_EVIDENCE_DIR=$tmpdir/evidence"
    assert_exit "publishing run" "$tmpdir" 0 || return 1
    assert_contains "receipt published" "$tmpdir/out" "Published 1 macos-latest cell(s)" || return 1

    local repo="$tmpdir/repo" head receipt report_dir
    head="$(git -C "$repo" rev-parse HEAD)"
    if ! receipt="$(git -C "$repo" notes --ref refs/notes/ci-local/macos-latest show "$head" 2>/dev/null)"; then
        echo "  no receipt note was attached to $head" >&2
        return 1
    fi
    report_dir="$(printf '%s' "$receipt" | jq -r '.host.report_dir')"
    # The hook's own staging directory is gone by now; the receipt must name
    # the durable copy under the overridden root, keyed by head and environment.
    if [ "$report_dir" != "$tmpdir/evidence/$head/macos-latest" ]; then
        echo "  receipt names report_dir '$report_dir', expected '$tmpdir/evidence/$head/macos-latest'" >&2
        return 1
    fi
    if [ ! -d "$report_dir" ]; then
        echo "  receipt names report_dir '$report_dir', which does not exist after the hook exited" >&2
        return 1
    fi
    local report
    while IFS= read -r report; do
        if [ ! -r "$report_dir/$report" ]; then
            echo "  receipt cell names report '$report', not readable under $report_dir" >&2
            ls -R "$report_dir" >&2
            return 1
        fi
    done <<<"$(printf '%s' "$receipt" | jq -r '.cells[].report')"
    # The whole staging directory travels, manifest included, so the retained
    # copy is what `record-cells` read rather than a subset of it.
    [ -r "$report_dir/manifest.jsonl" ] || { echo "  manifest.jsonl was not retained" >&2; return 1; }
}

test_reports_that_cannot_be_retained_publish_no_receipt() {
    local tmpdir="$1"
    make_publishing_repo "$tmpdir"
    # A regular file where the evidence root's parent must be a directory makes
    # every mkdir under it fail.
    : >"$tmpdir/blocker"
    run_publishing_hook "$tmpdir" "BISCUIT_CI_EVIDENCE_DIR=$tmpdir/blocker/evidence"
    # The gates passed; only the evidence is withheld, and the hook says why.
    assert_exit "unretained run" "$tmpdir" 0 || return 1
    assert_contains "refusal explained" "$tmpdir/err" "no receipt is published" || return 1
    assert_contains "refusal names the location" "$tmpdir/err" "$tmpdir/blocker/evidence" || return 1
    assert_not_contains "nothing published" "$tmpdir/out" "cell(s) of evidence" || return 1
    local repo="$tmpdir/repo"
    if [ -n "$(git -C "$repo" notes --ref refs/notes/ci-local/macos-latest list 2>/dev/null)" ]; then
        echo "  a receipt was attached although its reports were not retained" >&2
        return 1
    fi
}

# ---------- scope evidence (fixes/2026-09-10-local-affected-scope, R1/R2) ----
#
# The scope receipt binds the base the CI event will compare with. A push to
# main is compared with the remote's current main (`github.event.before`), the
# remote sha Git hands the hook; every other push is validated by a pull
# request against the tip of main, which the merge base with origin/main is
# until main advances.

scope_receipt() {
    git -C "$1/repo" notes --ref refs/notes/ci-local/scope show "$(git -C "$1/repo" rev-parse HEAD)" 2>/dev/null
}

assert_scope_receipt_base() {
    local label="$1" tmpdir="$2" expected="$3"
    local receipt
    if ! receipt="$(scope_receipt "$tmpdir")"; then
        echo "  no scope receipt is attached to HEAD ($label)" >&2
        return 1
    fi
    local repo="$tmpdir/repo" head
    head="$(git -C "$repo" rev-parse HEAD)"
    if [ "$(printf '%s' "$receipt" | jq -r '.base')" != "$expected" ] \
        || [ "$(printf '%s' "$receipt" | jq -r '.head')" != "$head" ] \
        || [ "$(printf '%s' "$receipt" | jq -r '.tree')" != "$(git -C "$repo" rev-parse "$head^{tree}")" ]; then
        echo "  the scope receipt does not bind base=$expected head=$head ($label):" >&2
        printf '%s' "$receipt" | jq '{base, head, tree}' | sed 's/^/  /' >&2
        return 1
    fi
    # Both documents ride along and the plan names the same pair.
    if [ "$(printf '%s' "$receipt" | jq -r '.plan.base + " " + .plan.head')" != "$expected $head" ] \
        || [ "$(printf '%s' "$receipt" | jq -r '.scope.packages | join(",")')" != "alpha" ]; then
        echo "  the scope receipt carries the wrong plan or projection ($label)" >&2
        return 1
    fi
}

test_scope_only_publishes_committed_scope_for_a_push_to_main() {
    local tmpdir="$1"
    make_publishing_repo "$tmpdir"
    stage_publishing_tools "$tmpdir"
    local repo="$tmpdir/repo" main_sha head
    main_sha="$(git -C "$repo" rev-parse origin/main)"
    head="$(git -C "$repo" rev-parse HEAD)"
    run_hook_in_repo "$tmpdir" scope-only refs/heads/main "$main_sha"
    assert_exit "scope-only push to main" "$tmpdir" 0 || return 1
    assert_log_lacks "no gate" "$tmpdir" "pre-push" || return 1
    assert_contains "base kind announced" "$tmpdir/out" "the push event's before" || return 1
    assert_contains "publication announced" "$tmpdir/out" "Published scope evidence" || return 1
    assert_scope_receipt_base "push to main" "$tmpdir" "$main_sha" || return 1
    # Calculated exactly as CI's scope step invokes the planner: options, then
    # `--`, then the COMMITTED path set of base..head.
    assert_contains "planner invoked as CI does" "$tmpdir/planner.log" \
        "--base $main_sha --head $head -- work.txt" || return 1
    # And the ref reached the remote, which is where CI reads it.
    if ! git -C "$tmpdir/origin.git" rev-parse --verify -q refs/notes/ci-local/scope >/dev/null; then
        echo "  refs/notes/ci-local/scope was not pushed to origin" >&2
        return 1
    fi
}

test_a_feature_branch_push_records_the_pull_request_base_not_its_previous_tip() {
    local tmpdir="$1"
    make_publishing_repo "$tmpdir"
    stage_publishing_tools "$tmpdir"
    local repo="$tmpdir/repo" previous main_sha
    local commit=(git -c user.name=hook-test -c user.email=hook@example.com -c commit.gpgsign=false)
    git -C "$repo" push -q origin feature
    previous="$(git -C "$repo" rev-parse HEAD)"
    echo "more" >"$repo/more.txt"
    "${commit[@]}" -C "$repo" add -A
    "${commit[@]}" -C "$repo" commit -q -m "more"
    main_sha="$(git -C "$repo" rev-parse origin/main)"
    run_hook_in_repo "$tmpdir" scope-only refs/heads/feature "$previous"
    assert_exit "feature push" "$tmpdir" 0 || return 1
    # The previous tip is what Git hands the hook, but no CI event compares
    # against it: the pull request run compares against main.
    assert_contains "base kind announced" "$tmpdir/out" "merge base with origin/main" || return 1
    assert_scope_receipt_base "feature push" "$tmpdir" "$main_sha" || return 1
}

test_a_branch_creating_push_records_the_merge_base_and_says_so() {
    local tmpdir="$1"
    make_publishing_repo "$tmpdir"
    run_publishing_hook "$tmpdir" "BISCUIT_CI_EVIDENCE_DIR=$tmpdir/evidence"
    assert_exit "new branch" "$tmpdir" 0 || return 1
    assert_contains "base kind announced" "$tmpdir/out" "merge base with origin/main" || return 1
    assert_scope_receipt_base "new branch" "$tmpdir" "$(git -C "$tmpdir/repo" rev-parse origin/main)" || return 1
    # Scope goes out BEFORE the gates run, whatever they then do.
    if [ "$(grep -n 'Published scope evidence' "$tmpdir/out" | cut -d: -f1)" -gt \
         "$(grep -n 'Published 1 macos-latest' "$tmpdir/out" | cut -d: -f1)" ]; then
        echo "  scope evidence was published after the validation receipt" >&2
        return 1
    fi
}

test_a_dirty_strict_run_publishes_scope_but_no_validation_receipt() {
    local tmpdir="$1"
    make_publishing_repo "$tmpdir"
    stage_publishing_tools "$tmpdir"
    local repo="$tmpdir/repo"
    echo "edited" >>"$repo/README.md"
    echo "untracked" >"$repo/untracked.txt"
    run_hook_in_repo "$tmpdir" strict refs/heads/feature \
        "0000000000000000000000000000000000000000" "BISCUIT_CI_EVIDENCE_DIR=$tmpdir/evidence"
    assert_exit "dirty strict" "$tmpdir" 0 || return 1
    assert_log_has "gates still ran" "$tmpdir" "pre-push" || return 1
    assert_scope_receipt_base "dirty strict" "$tmpdir" "$(git -C "$repo" rev-parse origin/main)" || return 1
    # R1: the committed path set, never the worktree's.
    assert_not_contains "unstaged edit excluded" "$tmpdir/planner.log" "README.md" || return 1
    assert_not_contains "untracked file excluded" "$tmpdir/planner.log" "untracked.txt" || return 1
    assert_contains "committed path included" "$tmpdir/planner.log" "-- work.txt" || return 1
    if [ -n "$(git -C "$repo" notes --ref refs/notes/ci-local/macos-latest list 2>/dev/null)" ]; then
        echo "  a validation receipt was attached although the tree was dirty" >&2
        return 1
    fi
    assert_not_contains "no validation claim" "$tmpdir/out" "Published 1 macos-latest" || return 1
}

test_a_failed_scope_publication_is_named_and_leaves_the_exit_code_alone() {
    local tmpdir="$1"
    make_publishing_repo "$tmpdir"
    stage_publishing_tools "$tmpdir"
    git -C "$tmpdir/repo" remote set-url origin "$tmpdir/no-such-origin.git"
    run_hook_in_repo "$tmpdir" scope-only refs/heads/feature "0000000000000000000000000000000000000000"
    assert_exit "unpublishable scope" "$tmpdir" 0 || return 1
    assert_contains "publication failure named" "$tmpdir/err" "Scope evidence PUBLICATION failed" || return 1
    assert_not_contains "not a calculation failure" "$tmpdir/err" "CALCULATION failed" || return 1
    assert_not_contains "not announced as published" "$tmpdir/out" "Published scope evidence" || return 1
    # Calculated and recorded locally; only the transfer failed.
    assert_scope_receipt_base "unpublishable scope" "$tmpdir" "$(git -C "$tmpdir/repo" rev-parse origin/main)" || return 1
}

test_a_failed_scope_calculation_is_named_and_leaves_the_exit_code_alone() {
    local tmpdir="$1"
    make_publishing_repo "$tmpdir"
    stage_publishing_tools "$tmpdir"
    run_hook_in_repo "$tmpdir" scope-only refs/heads/feature \
        "0000000000000000000000000000000000000000" "TEST_PLANNER_FAIL=1"
    assert_exit "uncalculable scope" "$tmpdir" 0 || return 1
    assert_contains "calculation failure named" "$tmpdir/err" "Scope evidence CALCULATION failed" || return 1
    assert_not_contains "not a publication failure" "$tmpdir/err" "PUBLICATION failed" || return 1
    if scope_receipt "$tmpdir" >/dev/null; then
        echo "  a scope receipt was attached although the calculation failed" >&2
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
run_test "a reused prohibited cell does not block"                    test_a_reused_prohibited_cell_does_not_block
run_test "an absent prohibited environment does not block"            test_an_absent_prohibited_environment_does_not_block
run_test "an executing prohibited cell blocks with its reason"        test_an_executing_prohibited_cell_blocks_with_its_reason
run_test "the constraint is decided in scope-only mode too"           test_the_constraint_is_decided_in_scope_only_mode_too
run_test "an unreadable plan blocks"                                  test_an_unreadable_plan_blocks
run_test "a failed plan resolution blocks"                            test_a_failed_plan_resolution_blocks
run_test "the plan resolution is handed no constraint store"          test_the_plan_resolution_is_handed_no_constraint_store
run_test "a constraint for another branch does not block"             test_a_constraint_for_another_branch_does_not_block
run_test "the plan fixtures are valid resolved plans"                 test_the_plan_fixtures_are_valid_resolved_plans
run_test "a published receipt names retained, readable reports"       test_a_published_receipt_names_retained_readable_reports
run_test "reports that cannot be retained publish no receipt"         test_reports_that_cannot_be_retained_publish_no_receipt
run_test "scope-only publishes committed scope for a push to main"    test_scope_only_publishes_committed_scope_for_a_push_to_main
run_test "a feature push records the PR base, not its previous tip"   test_a_feature_branch_push_records_the_pull_request_base_not_its_previous_tip
run_test "a branch-creating push records the merge base and says so"  test_a_branch_creating_push_records_the_merge_base_and_says_so
run_test "a dirty strict run publishes scope but no validation"       test_a_dirty_strict_run_publishes_scope_but_no_validation_receipt
run_test "a failed scope publication is named; exit code unchanged"   test_a_failed_scope_publication_is_named_and_leaves_the_exit_code_alone
run_test "a failed scope calculation is named; exit code unchanged"   test_a_failed_scope_calculation_is_named_and_leaves_the_exit_code_alone

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
