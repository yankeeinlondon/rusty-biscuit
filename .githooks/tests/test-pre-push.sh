#!/usr/bin/env bash
# Level 1 tests for .githooks/pre-push.
#
# Every test runs the hook inside a fixture repository (a `main` on a bare
# `origin`, one feature commit as HEAD, the real `local_evidence.py`/`schema.py`
# and `fixtures/affected_scope_stub.py` as `scripts/ci/affected_scope.py`,
# since no Cargo workspace exists there), with a fake `just` on PATH that
# answers only `pre-push`, so we can assert exit codes and stderr/stdout
# content for every documented behavior in R3 of
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
# and its review-2: the plan is resolved from the OUTGOING REVISION's committed
# tree — never the working tree — so a committed change masked by an unstaged
# revert is still reviewed, and an unstaged policy edit never reaches the plan
# or the scope receipt. The stub planner logs the tree it ran in.
#
# and its review-3: EVERY branch update on stdin is reviewed, in order, under
# its own revision, base, remote branch, and the remote Git names — a
# prohibition on a branch that is not checked out still blocks, a renamed
# refspec binds records under either name, and deletions and tags are skipped
# by name.
#
# and its review-4: each branch update is planned against the base of EVERY
# run it triggers — the remote's main (`github.event.before`) for a push to
# main; for any other branch, the CURRENT REMOTE TIP of the target branch of
# each open pull request whose head it is (listed through a stub `gh` on the
# harness PATH, since the bare fixture remote can hold no pull request), or a
# provisional plan against the remote's main when nothing is open. A stacked
# pull request whose child restores what its parent changed is the review's
# reproduction: against main it selects nothing, against the parent it selects
# the package.
#
# and its review-5: when the SAME push also updates a pull request's target,
# the server may apply the two updates in either order, so the run is reviewed
# in BOTH states — against the target's current remote tip and against the
# incoming revision — whichever order the ref lines arrive in; a target the
# push deletes leaves the pull request no base, and that update is refused.
#
# and ruling D1 of fixes/2026-09-11-cicd-cleanup: a COMPLETE failing run is
# published in both modes — strict blocks the branch only after the note is on
# the remote, so a later `--no-verify` push of the same tree lets CI reuse the
# known failure.
#
# and ruling D2 / audit W1, W10, W14 of the same fix: on a clean checkout the
# hook feeds HEAD's reviewed, evidence-overlaid plan to `just pre-push`, so
# the REAL `ci-local` recipe (imported by a harness justfile, its gate recipes
# answered by the fake `just`) runs only the cells prior passing evidence does
# not cover, reruns a cell whose newest evidence is a failure, never runs the
# planner's selection again, and the receipt is recorded against that plan and
# the reviewed base — a stacked pull request's target tip, not a merge base
# with origin/main — for the cells that actually ran.
#
# and the evidence-retention contract of review-1: a published receipt names
# a report directory that still exists after the hook exits, with every report
# it lists readable there, and a failed copy publishes no receipt.
#
# and the scope-evidence contract of fixes/2026-09-10-local-affected-scope
# (R1/R2): every mode publishes the COMMITTED scope of the outgoing head on
# refs/notes/ci-local/scope before any gate, dirty tree or not, against the
# base the CI event will compare with; a publication failure is named as such
# and never changes the exit code.
#
# Run directly: ./.githooks/tests/test-pre-push.sh
# PRE_PUSH_HOOK_UNDER_TEST=<path> runs the suite against another copy of the
# hook (used to prove a new fixture fails against the hook it was written for);
# CI_LOCAL_RECIPE_UNDER_TEST=<path> does the same for the `ci-local` recipe the
# plan-fed fixtures run.
# The suite must stay runnable under Bash 3.2, the stock macOS /bin/bash: no
# mapfile, no associative arrays, and an empty array expands only through
# ${a[@]+"${a[@]}"} under `set -u`.

set -u
set -o pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
HOOK="${PRE_PUSH_HOOK_UNDER_TEST:-$REPO_ROOT/.githooks/pre-push}"
FIXTURES="$REPO_ROOT/.githooks/tests/fixtures"
# The plan-fed fixtures run the repository's real `ci-local` recipe through
# the real `just`; the fake `just` on the harness PATH delegates to it by
# absolute path and answers the recipe's gate calls itself.
REAL_JUST="$(command -v just || true)"
CI_LOCAL_RECIPE="${CI_LOCAL_RECIPE_UNDER_TEST:-$REPO_ROOT/just/ci-local.just}"

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
# `pre-push`. Any other subcommand exits 99 so unexpected calls show up as
# test failures — the hook resolves the trigger plan itself, through the stub
# planner, and never through `just ci-local`. `plan_fixture` — by default a
# plan that EXECUTES a wsl2-ubuntu cell — is what the stub planner emits for
# the hook's resolution (handed over as TEST_PLANNER_PLAN by the runners).
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
    *)
        echo "fake just: unexpected subcommand: \$*" >&2
        exit 99
        ;;
esac
EOF
    chmod +x "$tmpdir/just"
    printf '%s' "$plan_fixture" >"$tmpdir/planner-plan.path"
}

# A repository with `main` on a bare `origin`, one feature commit as HEAD, and
# the CI scripts committed so the hook's `scripts/ci/*.py` resolve in the
# committed tree. The planner is the stub: the real one needs the Cargo
# workspace it lives in. `main` also carries the stub's policy file and a
# source file for the masked-change fixture.
#
# Usage: make_publishing_repo <tmpdir> [feature_file] [feature_content]
make_publishing_repo() {
    local tmpdir="$1" feature_file="${2:-work.txt}" feature_content="${3:-work}"
    local repo="$tmpdir/repo" bare="$tmpdir/origin.git"
    local commit=(git -c user.name=hook-test -c user.email=hook@example.com -c commit.gpgsign=false)
    git init -q -b main "$repo"
    mkdir -p "$repo/scripts/ci" "$repo/.github/ci" "$repo/pkg/alpha/src"
    cp "$REPO_ROOT"/scripts/ci/*.py "$repo/scripts/ci/"
    cp "$FIXTURES/affected_scope_stub.py" "$repo/scripts/ci/affected_scope.py"
    # As in the real repository: the scripts' bytecode cache must not dirty
    # the tree, or the clean-tree guard would withhold every receipt.
    echo "__pycache__/" >"$repo/.gitignore"
    echo "base" >"$repo/README.md"
    echo "base lib" >"$repo/pkg/alpha/src/lib.rs"
    printf '{"preflight_reason": "committed policy"}\n' >"$repo/.github/ci/policy.json"
    "${commit[@]}" -C "$repo" add -A
    "${commit[@]}" -C "$repo" commit -q -m "base"
    git init -q --bare "$bare"
    git -C "$repo" remote add origin "$bare"
    git -C "$repo" push -q origin main
    git -C "$repo" checkout -q -b feature
    mkdir -p "$(dirname "$repo/$feature_file")"
    echo "$feature_content" >"$repo/$feature_file"
    "${commit[@]}" -C "$repo" add -A
    "${commit[@]}" -C "$repo" commit -q -m "head"
}

# The tools every run needs on its PATH besides the fake `just`: a fake
# `sniff` reporting macOS, and the real `jq` and `python3`.
stage_fixture_tools() {
    local tmpdir="$1"
    printf '#!/bin/sh\necho "{\\"os_type\\":\\"MacOS\\",\\"kernel\\":\\"Darwin\\"}"\n' >"$tmpdir/sniff"
    chmod +x "$tmpdir/sniff"
    ln -s "$(command -v jq)" "$tmpdir/jq"
    ln -s "$(command -v python3)" "$tmpdir/python3"
    mkdir -p "$tmpdir/home"
}

# Run the hook from the fixture repo as Git would: the remote's name and URL
# as arguments and the given ref line(s) — newline-separated — on stdin. The
# stub planner logs its invocations to planner.log and emits the plan fixture
# `make_fake_just` recorded, if any.
#
# Usage: run_hook_as_remote <tmpdir> <mode|__UNSET__> <remote_name> <remote_url> <ref_lines> [env...]
#
# Writes stdout to $tmpdir/out and stderr to $tmpdir/err. Always returns 0
# itself so set -e in callers does not abort on a non-zero hook exit; the
# real exit code is written to $tmpdir/exit.
run_hook_as_remote() {
    local tmpdir="$1" mode="$2" remote_name="$3" remote_url="$4" ref_line="$5"
    shift 5
    local repo="$tmpdir/repo"
    if [ ! -d "$repo" ]; then
        make_publishing_repo "$tmpdir"
        stage_fixture_tools "$tmpdir"
    fi
    # Both home variables: `constraints.py` reads the default store under
    # `Path.home()`, which is HOME on Unix and USERPROFILE on native Windows.
    local env_args=(
        "PATH=$tmpdir:/usr/bin:/bin"
        "HOME=$tmpdir/home"
        "USERPROFILE=$tmpdir/home"
        "TEST_PLANNER_LOG=$tmpdir/planner.log"
    )
    if [ "$mode" != "__UNSET__" ]; then
        env_args+=("RUSTY_BISCUIT_PRE_PUSH=$mode")
    fi
    if [ -f "$tmpdir/planner-plan.path" ]; then
        env_args+=("TEST_PLANNER_PLAN=$(cat "$tmpdir/planner-plan.path")")
    fi
    # Use `env -i` so the hook does not inherit the developer's
    # RUSTY_BISCUIT_PRE_PUSH_* values from the surrounding shell.
    (
        cd "$repo" || exit 97
        printf '%s\n' "$ref_line" \
            | env -i "${env_args[@]}" "$@" \
                "$HOOK" "$remote_name" "$remote_url" >"$tmpdir/out" 2>"$tmpdir/err"
        echo $? >"$tmpdir/exit"
    )
}

# The common case: a push to `origin`.
#
# Usage: run_hook_with_ref_line <tmpdir> <mode|__UNSET__> <ref_lines> [env...]
run_hook_with_ref_line() {
    local tmpdir="$1" mode="$2" ref_line="$3"
    shift 3
    run_hook_as_remote "$tmpdir" "$mode" origin "$tmpdir/origin.git" "$ref_line" "$@"
}

# Push HEAD to a remote ref whose current sha the caller chooses; the remote
# ref and sha decide the scope base.
#
# Usage: run_hook_in_repo <tmpdir> <mode> <remote_ref> <remote_sha> [env...]
run_hook_in_repo() {
    local tmpdir="$1" mode="$2" remote_ref="$3" remote_sha="$4"
    shift 4
    if [ ! -d "$tmpdir/repo" ]; then
        make_publishing_repo "$tmpdir"
        stage_fixture_tools "$tmpdir"
    fi
    local head
    head="$(git -C "$tmpdir/repo" rev-parse HEAD)"
    run_hook_with_ref_line "$tmpdir" "$mode" \
        "refs/heads/feature $head $remote_ref $remote_sha" "$@"
}

# A push of HEAD creating `feature` on the remote, in the given mode.
#
# Usage: run_hook <tmpdir> <mode> [areas_override]
run_hook() {
    local tmpdir="$1"
    local mode="$2"
    local areas="${3-__UNSET__}"
    local env_args=()
    if [ "$areas" != "__UNSET__" ]; then
        env_args+=("RUSTY_BISCUIT_PRE_PUSH_AREAS=$areas")
    fi
    # `"${env_args[@]}"` alone is an unbound-variable abort under Bash 3.2
    # with `set -u` when the array is empty.
    run_hook_in_repo "$tmpdir" "$mode" refs/heads/feature \
        "0000000000000000000000000000000000000000" ${env_args[@]+"${env_args[@]}"}
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

# `needle` is matched at the start of a logged call, i.e. as the subcommand.
assert_log_lacks() {
    local label="$1" tmpdir="$2" needle="$3"
    if [ ! -f "$tmpdir/just.log" ] || ! grep -q -- "^$needle" "$tmpdir/just.log"; then
        return 0
    fi
    echo "  expected fake-just NOT to be called with: $needle" >&2
    sed 's/^/  /' "$tmpdir/just.log" >&2
    return 1
}

# The plan was resolved: the stub planner was invoked to write a plan.
assert_planner_ran() {
    local label="$1" tmpdir="$2"
    if [ -f "$tmpdir/planner.log" ] && grep -qF -- "--plan-out" "$tmpdir/planner.log"; then
        return 0
    fi
    echo "  expected the planner to have resolved a plan ($label)" >&2
    if [ -f "$tmpdir/planner.log" ]; then
        sed 's/^/  /' "$tmpdir/planner.log" >&2
    else
        echo "  (planner.log was never written — the hook never ran the planner)" >&2
    fi
    return 1
}

# The tree the stub planner lived in and ran from, as it logged them.
planner_root() {
    sed -n 's/^root \(.*\) cwd .*$/\1/p' "$1/planner.log" | head -n 1
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

test_scope_only_runs_no_gate_but_still_resolves_scope() {
    local tmpdir="$1"
    make_fake_just "$tmpdir" 7
    run_hook "$tmpdir" "scope-only"
    assert_exit "scope-only" "$tmpdir" 0 || return 1
    # The distinction the mode exists for: scope is calculated and reviewable,
    # no gate runs, and a failing gate exit code cannot be inherited.
    assert_planner_ran "plan resolved" "$tmpdir" || return 1
    assert_log_lacks "no gate" "$tmpdir" "pre-push" || return 1
}

test_off_mode_is_a_deprecated_alias_of_scope_only() {
    local tmpdir="$1"
    make_fake_just "$tmpdir" 7
    run_hook "$tmpdir" "off"
    assert_exit "off" "$tmpdir" 0 || return 1
    assert_contains "off deprecation notice" "$tmpdir/err" "deprecated" || return 1
    assert_contains "off names its replacement" "$tmpdir/err" "scope-only" || return 1
    assert_planner_ran "plan resolved" "$tmpdir" || return 1
    assert_log_lacks "no gate" "$tmpdir" "pre-push" || return 1
}

test_invalid_mode_exits_one() {
    local tmpdir="$1"
    make_fake_just "$tmpdir" 0
    run_hook "$tmpdir" "bogus"
    assert_exit "invalid" "$tmpdir" 1 || return 1
    assert_contains "invalid mode message" "$tmpdir/err" "Unknown RUSTY_BISCUIT_PRE_PUSH" || return 1
    assert_contains "invalid mode red SGR prefix" "$tmpdir/err" $'\033[31mUnknown RUSTY_BISCUIT_PRE_PUSH' || return 1
    assert_contains "valid values listed" "$tmpdir/err" "scope-only, warn, or strict" || return 1
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
    run_hook_in_repo "$tmpdir" "__UNSET__" refs/heads/feature \
        "0000000000000000000000000000000000000000"
    assert_exit "default strict" "$tmpdir" 7 || return 1
}

test_default_delegates_scope_to_pre_push() {
    local tmpdir="$1"
    make_fake_just "$tmpdir" 0
    run_hook "$tmpdir" "warn"
    assert_exit "default" "$tmpdir" 0 || return 1
    # The hook resolves the trigger plan through the planner itself, then
    # makes exactly one `just` call, and it is `pre-push` with no arguments.
    assert_planner_ran "plan resolved" "$tmpdir" || return 1
    assert_log_has "pre-push called" "$tmpdir" "pre-push" || return 1
    if [ "$(wc -l <"$tmpdir/just.log" | tr -d ' ')" != "1" ] \
        || [ "$(sed -n 1p "$tmpdir/just.log")" != "pre-push" ]; then
        echo "  expected exactly 'pre-push' as the only just call, got:" >&2
        sed 's/^/  /' "$tmpdir/just.log" >&2
        return 1
    fi
}

# Harness self-check: with no override, run_hook must hand the hook no
# selection variable at all, not an empty one, and it must do so under the
# default shell. run_hook_with_ref_line reads HOOK dynamically, so a local
# rebinding routes the run to a stub that records its environment.
test_run_hook_forwards_no_selection_without_an_override() {
    local tmpdir="$1"
    printf '#!/usr/bin/env bash\nenv >"%s/hook.env"\n' "$tmpdir" >"$tmpdir/env-dump"
    chmod +x "$tmpdir/env-dump"
    local HOOK="$tmpdir/env-dump"
    run_hook "$tmpdir" "warn"
    assert_exit "stub hook ran" "$tmpdir" 0 || return 1
    assert_contains "mode forwarded" "$tmpdir/hook.env" "RUSTY_BISCUIT_PRE_PUSH=warn" || return 1
    assert_not_contains "no selection variable" "$tmpdir/hook.env" "RUSTY_BISCUIT_PRE_PUSH_AREAS" || return 1
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
    if grep -qF -- "biscuit-file" "$tmpdir/planner.log"; then
        echo "  the plan resolution was narrowed by the selection override:" >&2
        sed 's/^/  /' "$tmpdir/planner.log" >&2
        return 1
    fi
}

# ---------- execution constraints (fixes/2026-09-11-cicd-cleanup, AC17) ------
#
# Phase 2 froze these while the hook still read no constraint store; Phase 4
# built it, so they now run directly. The stub planner writes a plan that
# executes a wsl2-ubuntu cell unless a test hands it another fixture, so each
# refusal below is decided from a resolved plan, exactly as a real push is.

# Run the hook in strict mode with a passing fake `just` plus extra
# environment assignments.
run_hook_with_env() {
    local tmpdir="$1"
    shift
    run_hook_in_repo "$tmpdir" strict refs/heads/feature \
        "0000000000000000000000000000000000000000" "$@"
}

# The tests below name the store through BISCUIT_CI_CONSTRAINTS_DIR, the
# override; the default location, `<home>/.rusty-biscuit/ci-constraints/
# <repository>/`, is covered by the fresh-session test further down.
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
    assert_planner_ran "plan resolved" "$tmpdir" || return 1
    assert_log_lacks "no gate after refusal" "$tmpdir" "pre-push" || return 1
}

test_the_constraint_is_decided_in_scope_only_mode_too() {
    local tmpdir="$1"
    make_fake_just "$tmpdir" 0 "$FIXTURES/plan-wsl-executing.json"
    write_prohibition "$tmpdir/constraints" "wsl2-ubuntu" "2099-01-01"
    run_hook_in_repo "$tmpdir" scope-only refs/heads/feature \
        "0000000000000000000000000000000000000000" \
        "BISCUIT_CI_CONSTRAINTS_DIR=$tmpdir/constraints"
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
    assert_contains "review named as the failing stage" "$tmpdir/err" "Trigger review failed" || return 1
    assert_log_lacks "no gate after refusal" "$tmpdir" "pre-push" || return 1
}

# A resolution that fails (planner error, unresolvable base, missing python3,
# ...) is not a plan that schedules nothing: it blocks, names the stage, and —
# since the reviewed plan IS the scope receipt — publishes no scope either.
test_a_failed_plan_resolution_blocks_and_publishes_no_scope() {
    local tmpdir="$1"
    make_fake_just "$tmpdir" 0
    write_prohibition "$tmpdir/constraints" "wsl2-ubuntu" "2099-01-01"
    run_hook_with_env "$tmpdir" "BISCUIT_CI_CONSTRAINTS_DIR=$tmpdir/constraints" \
        "TEST_PLANNER_FAIL=1"
    assert_exit "failed resolution" "$tmpdir" 2 || return 1
    assert_contains "stage named" "$tmpdir/err" "the planner could not resolve the committed scope" || return 1
    assert_log_lacks "no gate after failure" "$tmpdir" "pre-push" || return 1
    if scope_receipt "$tmpdir" >/dev/null; then
        echo "  a scope receipt was attached although the plan resolution failed" >&2
        return 1
    fi
}

# The hook is the single decision point: the planner is handed no store, so it
# can neither refuse on a record scoped to another repository or branch nor
# hide an unsatisfied cell behind a `prohibited` state.
test_the_plan_resolution_is_handed_no_constraint_store() {
    local tmpdir="$1"
    make_fake_just "$tmpdir" 0
    write_prohibition "$tmpdir/constraints" "wsl2-ubuntu" "2099-01-01"
    run_hook_with_env "$tmpdir" "BISCUIT_CI_CONSTRAINTS_DIR=$tmpdir/constraints"
    assert_planner_ran "plan resolved" "$tmpdir" || return 1
    assert_not_contains "store hidden from planner" "$tmpdir/planner.log" "--constraints" || return 1
    assert_not_contains "store hidden from planner" "$tmpdir/planner.log" "$tmpdir/constraints" || return 1
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

# The fixtures are what the stub planner hands the hook; a fixture the planner
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
# (spec section 3.5) — but this fixture's tree is dirtied first, so nothing is
# published. The distinction the fixture guards is that the failure text is
# printed AFTER the block rather than short-circuiting past it, which is what
# makes a complete failing cell publishable at all.
test_a_failing_warn_run_reaches_the_publication_block() {
    local tmpdir="$1"
    make_fake_just "$tmpdir" 1
    make_publishing_repo "$tmpdir"
    stage_fixture_tools "$tmpdir"
    echo "edited" >>"$tmpdir/repo/README.md"
    run_hook "$tmpdir" "warn"
    assert_exit "warn failure" "$tmpdir" 0 || return 1
    assert_contains "failure still reported" "$tmpdir/out" "Pre-push validation failed" || return 1
    # A dirty tree is not exact-tree evidence, so the guard declines before
    # any validation note is written.
    assert_not_contains "no evidence published" "$tmpdir/out" "cell(s) of evidence" || return 1
}

# The gate actually stages a complete report, but an override withholds it.
test_publication_requires_a_clean_exact_tree_and_no_override() {
    local tmpdir="$1"
    make_publishing_repo "$tmpdir"
    run_publishing_hook "$tmpdir" "RUSTY_BISCUIT_PRE_PUSH_AREAS=alpha"
    assert_exit "override" "$tmpdir" 0 || return 1
    assert_contains "override reason" "$tmpdir/err" "narrowed the gates to 'alpha'" || return 1
    if git -C "$tmpdir/origin.git" show-ref --verify --quiet refs/notes/ci-local/macos-latest; then
        echo "  overridden run published validation evidence" >&2
        return 1
    fi
}

# A gate has already written passing evidence when the hook receives TERM.
# The child sends the signal at that boundary, avoiding a timing-based race.
test_an_interrupted_hook_never_publishes_staged_evidence() {
    local tmpdir="$1"
    make_publishing_repo "$tmpdir"
    stage_publishing_tools "$tmpdir"
    cat >>"$tmpdir/pre-push.hook" <<'EOF'
kill -TERM "$PPID"
exit 0
EOF
    run_hook_in_repo "$tmpdir" strict refs/heads/feature "$ZERO_SHA"
    assert_exit "interrupted" "$tmpdir" 143 || return 1
    assert_contains "interruption explained" "$tmpdir/err" "Pre-push interrupted" || return 1
    if git -C "$tmpdir/origin.git" show-ref --verify --quiet refs/notes/ci-local/macos-latest; then
        echo "  interrupted run published validation evidence" >&2
        return 1
    fi
}

test_a_gate_exiting_130_never_publishes_staged_evidence() {
    local tmpdir="$1"
    make_publishing_repo "$tmpdir"
    stage_publishing_tools "$tmpdir"
    echo 'exit 130' >>"$tmpdir/pre-push.hook"
    run_hook_in_repo "$tmpdir" strict refs/heads/feature "$ZERO_SHA"
    assert_exit "gate interrupted" "$tmpdir" 130 || return 1
    assert_contains "interruption explained" "$tmpdir/err" "validation was interrupted (exit 130)" || return 1
    if git -C "$tmpdir/origin.git" show-ref --verify --quiet refs/notes/ci-local/macos-latest; then
        echo "  interrupted gate published validation evidence" >&2
        return 1
    fi
}

# ---------- retained reports (review-1: receipts must name reports that exist) --
#
# The hook stages JUnit reports in a temp dir it deletes on exit, and the
# receipt's `host.report_dir` must name where they still are afterwards. These
# fixtures satisfy the whole publication guard for real: a pushed head, a clean
# tree, a resolvable `origin/main`, a fake `sniff`, and the real `jq`/`python3`.

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

# The tools a publishing run needs: a passing fake `just` that stages one
# macOS L1 report, with the stub planner emitting the matching one-cell plan.
stage_publishing_tools() {
    local tmpdir="$1"
    make_fake_just "$tmpdir" 0 "$FIXTURES/plan-macos-executing.json"
    make_staging_pre_push "$tmpdir"
    stage_fixture_tools "$tmpdir"
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

# ---------- complete failing runs (fixes/2026-09-11-cicd-cleanup, D1; AC6/AC7) --
#
# A COMPLETE failing run is evidence too. The hook publishes it in BOTH modes:
# warn continues the push with the note already on the remote, and strict
# blocks the branch AFTER the note is on the remote, so the `--no-verify` push
# of the unchanged tree that follows lets CI reuse the known failure instead
# of re-running it. These fixtures satisfy the whole publication guard for
# real, like the retained-reports ones, but the gate fails.

# The gate stages one L1 report for `alpha` with a failing test and records a
# non-zero exit for the cell, so `record-cells` measures a COMPLETE failure
# (a failure whose report names no failing test would be `partial`).
make_failing_staging_pre_push() {
    local tmpdir="$1"
    cat >"$tmpdir/pre-push.hook" <<EOF
cp "$FIXTURES/plan-macos-executing.json" "\$BISCUIT_CI_PLAN_OUT"
mkdir -p "\$BISCUIT_CI_REPORTS_OUT/L1"
printf '<?xml version="1.0"?><testsuites><testsuite name="alpha" tests="2" failures="1" errors="0" skipped="0"><testcase classname="alpha" name="one"/><testcase classname="alpha" name="two"><failure message="assertion failed">boom</failure></testcase></testsuite></testsuites>' >"\$BISCUIT_CI_REPORTS_OUT/L1/alpha.xml"
printf '{"tier":"L1","package":"alpha","xml":"L1/alpha.xml","exit_code":1,"environment":"macos-latest","duration_s":3,"report_present":true}\\n' >"\$BISCUIT_CI_REPORTS_OUT/manifest.jsonl"
EOF
}

# A failing run in the given mode, pushing a branch that does not yet exist on
# the remote: the fake `just` stages the failing report and exits 1.
#
# Usage: run_failing_publishing_hook <tmpdir> <mode> [env...]
run_failing_publishing_hook() {
    local tmpdir="$1" mode="$2"
    shift 2
    make_fake_just "$tmpdir" 1 "$FIXTURES/plan-macos-executing.json"
    make_failing_staging_pre_push "$tmpdir"
    stage_fixture_tools "$tmpdir"
    run_hook_in_repo "$tmpdir" "$mode" refs/heads/feature \
        "0000000000000000000000000000000000000000" "$@"
}

# The macOS receipt for `head` as it sits on the bare remote, read from the
# notes tree itself: the blocked branch never reached the remote, so the
# commit is not there to `notes show` against, but the note is.
remote_macos_receipt() {
    local bare="$1/origin.git" head="$2"
    git -C "$bare" cat-file -p "refs/notes/ci-local/macos-latest:$head" 2>/dev/null
}

assert_remote_receipt_is_a_complete_failure() {
    local label="$1" tmpdir="$2" head="$3"
    local receipt
    if ! receipt="$(remote_macos_receipt "$tmpdir" "$head")"; then
        echo "  no macos-latest note for $head reached the remote ($label)" >&2
        git -C "$tmpdir/origin.git" show-ref >&2 || true
        return 1
    fi
    if ! printf '%s' "$receipt" | jq -e --arg head "$head" \
        '.head == $head and (.cells | length) == 1 and (.cells[0] | .package == "alpha" and .gate == "L1" and .outcome == "fail" and .completion == "complete")' \
        >/dev/null; then
        echo "  the remote receipt does not record a complete failing alpha/L1 cell for $head ($label):" >&2
        printf '%s' "$receipt" | jq '{head, cells}' | sed 's/^/  /' >&2
        return 1
    fi
}

# T2 (D1, AC7): strict blocks the push, the note is already on the remote, and
# after the developer's `--no-verify` push of the same tree a fresh clone
# verifying against that note accepts the failed cell and the planner's
# overlay resolves it to reuse — the failure is rolled up, not re-run.
test_a_blocked_strict_failure_publishes_its_note_and_ci_reuses_the_failure() {
    local tmpdir="$1"
    make_publishing_repo "$tmpdir"
    run_failing_publishing_hook "$tmpdir" strict "BISCUIT_CI_EVIDENCE_DIR=$tmpdir/evidence"
    assert_exit "strict failure" "$tmpdir" 1 || return 1
    assert_contains "push blocked" "$tmpdir/err" "Push blocked by strict mode" || return 1
    assert_contains "note published before blocking" "$tmpdir/out" "Published 1 macos-latest cell(s)" || return 1

    local repo="$tmpdir/repo" bare="$tmpdir/origin.git" head base
    head="$(git -C "$repo" rev-parse HEAD)"
    base="$(git -C "$repo" rev-parse origin/main)"
    # The hook pushes the notes ref and nothing else: the branch is still
    # blocked.
    if git -C "$bare" rev-parse --verify -q refs/heads/feature >/dev/null; then
        echo "  the blocked branch reached the remote" >&2
        return 1
    fi
    assert_remote_receipt_is_a_complete_failure "strict" "$tmpdir" "$head" || return 1

    # The `--no-verify` push of the unchanged tree, then CI's view of it: a
    # fresh clone that fetches the notes and verifies with the committed
    # `local_evidence.py`, knowing nothing of the developer's checkout.
    git -C "$repo" push -q --no-verify origin feature || return 1
    git clone -q "$bare" "$tmpdir/ci" || return 1
    git -C "$tmpdir/ci" fetch -q origin '+refs/notes/ci-local/*:refs/notes/ci-local/*' || return 1
    if ! (cd "$tmpdir/ci" && python3 scripts/ci/local_evidence.py verify --cells \
            --plan "$FIXTURES/plan-macos-executing.json" --base "$base" --head "$head" \
            --rejections "$tmpdir/rejections.json") >"$tmpdir/accepted.json" 2>"$tmpdir/verify.err"; then
        echo "  verify --cells failed in the clone:" >&2
        sed 's/^/  /' "$tmpdir/verify.err" >&2
        return 1
    fi
    if ! jq -e 'length == 1 and (.[0] | .package == "alpha" and .environment == "macos-latest" and .gate == "L1" and .outcome == "fail" and .completion == "complete" and .origin == "local")' \
            "$tmpdir/accepted.json" >/dev/null; then
        echo "  verification did not accept the failed cell with its outcome retained:" >&2
        sed 's/^/  /' "$tmpdir/accepted.json" >&2
        echo "  --- rejections ---" >&2
        sed 's/^/  /' "$tmpdir/rejections.json" >&2
        return 1
    fi
    # The real overlay (`--apply-to` reads nothing from the checkout) resolves
    # the cell to reuse of the failure; nothing is left to execute.
    if ! python3 "$REPO_ROOT/scripts/ci/affected_scope.py" --apply-to "$FIXTURES/plan-macos-executing.json" \
            --accepted-cells "$tmpdir/accepted.json" --resolved-plan >"$tmpdir/applied.json" 2>"$tmpdir/apply.err"; then
        echo "  --apply-to failed:" >&2
        sed 's/^/  /' "$tmpdir/apply.err" >&2
        return 1
    fi
    if ! jq -e '(.cells | length) == 1 and (.cells[0] | .execution == "reuse" and .origin == "local" and .state == "reused" and .evidence.outcome == "fail") and ([.cells[] | select(.execution == "execute")] | length) == 0' \
            "$tmpdir/applied.json" >/dev/null; then
        echo "  the overlaid plan does not reuse the failed cell:" >&2
        jq '.cells' "$tmpdir/applied.json" | sed 's/^/  /' >&2
        return 1
    fi
}

# T1 (AC6): the same complete failure under warn continues the push, and the
# note is on the remote before the hook returns.
test_a_complete_warn_failure_publishes_its_note() {
    local tmpdir="$1"
    make_publishing_repo "$tmpdir"
    run_failing_publishing_hook "$tmpdir" warn "BISCUIT_CI_EVIDENCE_DIR=$tmpdir/evidence"
    assert_exit "warn failure" "$tmpdir" 0 || return 1
    assert_contains "failure still reported" "$tmpdir/out" "Pre-push validation failed" || return 1
    assert_contains "note published" "$tmpdir/out" "Published 1 macos-latest cell(s)" || return 1
    assert_remote_receipt_is_a_complete_failure "warn" "$tmpdir" "$(git -C "$tmpdir/repo" rev-parse HEAD)" || return 1
}

# ---------- scope evidence (fixes/2026-09-10-local-affected-scope, R1/R2) ----
#
# The scope receipt binds the base the CI event will compare with. A push to
# main is compared with the remote's current main (`github.event.before`), the
# remote sha Git hands the hook; every other push runs once per open pull
# request, against that pull request's target tip on the remote. The bare
# fixture remote is not GitHub, so no pull request can be open on it and the
# hook reviews a provisional plan against the remote's main — which is also
# what a pull request opened against main next would run.

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
    # against it: a pull request run compares against its target's tip, and
    # with nothing open the provisional plan is against the remote's main.
    assert_contains "base kind announced" "$tmpdir/out" "provisional: no open pull request" || return 1
    assert_scope_receipt_base "feature push" "$tmpdir" "$main_sha" || return 1
}

test_a_branch_creating_push_records_the_remote_main_tip_and_says_so() {
    local tmpdir="$1"
    make_publishing_repo "$tmpdir"
    run_publishing_hook "$tmpdir" "BISCUIT_CI_EVIDENCE_DIR=$tmpdir/evidence"
    assert_exit "new branch" "$tmpdir" 0 || return 1
    assert_contains "base kind announced" "$tmpdir/out" "provisional: no open pull request" || return 1
    assert_contains "provisional trigger explained" "$tmpdir/out" \
        "a pull request opened against main before main advances will run this plan" || return 1
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
    # The remote stays readable — the review needs its main — and refuses only
    # the scope notes ref, so exactly the transfer fails.
    mkdir -p "$tmpdir/origin.git/hooks"
    cat >"$tmpdir/origin.git/hooks/pre-receive" <<'EOF'
#!/bin/sh
while read -r old new ref; do
    case "$ref" in
        refs/notes/ci-local/scope) echo "scope notes refused by the fixture remote" >&2; exit 1 ;;
    esac
done
exit 0
EOF
    chmod +x "$tmpdir/origin.git/hooks/pre-receive"
    run_hook_in_repo "$tmpdir" scope-only refs/heads/feature "0000000000000000000000000000000000000000"
    assert_exit "unpublishable scope" "$tmpdir" 0 || return 1
    assert_contains "publication failure named" "$tmpdir/err" "Scope evidence PUBLICATION failed" || return 1
    assert_not_contains "not a recording failure" "$tmpdir/err" "RECORDING failed" || return 1
    assert_not_contains "not announced as published" "$tmpdir/out" "Published scope evidence" || return 1
    # Calculated and recorded locally; only the transfer failed.
    assert_scope_receipt_base "unpublishable scope" "$tmpdir" "$(git -C "$tmpdir/repo" rev-parse origin/main)" || return 1
}

# ---------- the outgoing revision's committed tree (review-2, finding 1) ------
#
# The plan the hook reviews must be the plan CI resolves for the push: the
# committed base..head path set, planned by the committed tree's planner,
# manifests, and policy. The working tree is not that tree whenever it is
# dirty, and the two failure shapes below are exactly the ones the review
# reproduced. The stub planner logs `root <dir> cwd <dir>` so a test can see
# which tree planned.

# No temporary worktree survives the hook, whichever way it exited.
assert_no_leftover_worktree() {
    local label="$1" tmpdir="$2"
    local count
    count="$(git -C "$tmpdir/repo" worktree list | wc -l | tr -d ' ')"
    if [ "$count" = "1" ]; then
        return 0
    fi
    echo "  a temporary worktree outlived the hook ($label):" >&2
    git -C "$tmpdir/repo" worktree list | sed 's/^/  /' >&2
    return 1
}

test_a_committed_change_masked_by_an_unstaged_revert_is_still_reviewed() {
    local tmpdir="$1"
    # The feature commit changes only the source file; restoring its base
    # contents unstaged leaves the working tree identical to the base, which
    # is what a working-tree preview reports as "nothing to gate".
    make_publishing_repo "$tmpdir" "pkg/alpha/src/lib.rs" "changed lib"
    stage_fixture_tools "$tmpdir"
    make_fake_just "$tmpdir" 0 "$FIXTURES/plan-macos-executing.json"
    local repo="$tmpdir/repo"
    echo "base lib" >"$repo/pkg/alpha/src/lib.rs"
    if [ -n "$(git -C "$repo" diff --name-only "$(git -C "$repo" merge-base origin/main HEAD)")" ]; then
        echo "  fixture error: the working tree still differs from the base" >&2
        return 1
    fi
    write_prohibition "$tmpdir/constraints" "macos-latest" "2099-01-01"
    run_hook_with_env "$tmpdir" "BISCUIT_CI_CONSTRAINTS_DIR=$tmpdir/constraints"
    # The push sends the committed change, so CI would schedule the prohibited
    # cell: the hook must see it and refuse.
    assert_exit "masked committed change" "$tmpdir" 1 || return 1
    assert_contains "prohibited cell named" "$tmpdir/err" "macos-latest" || return 1
    assert_contains "reason stated" "$tmpdir/err" "do not rerun WSL for this branch" || return 1
    assert_contains "committed path planned" "$tmpdir/planner.log" "-- pkg/alpha/src/lib.rs" || return 1
    assert_log_lacks "no gate after refusal" "$tmpdir" "pre-push" || return 1
    # Constraints are decided before the receipt is published, as they always
    # were: a blocked push leaves no scope note behind.
    if scope_receipt "$tmpdir" >/dev/null; then
        echo "  a scope receipt was published for a push the constraint blocked" >&2
        return 1
    fi
    assert_contains "committed tree materialized" "$tmpdir/out" "temporary worktree" || return 1
    assert_no_leftover_worktree "masked committed change" "$tmpdir" || return 1
}

test_an_unstaged_policy_edit_never_reaches_the_committed_plan_or_receipt() {
    local tmpdir="$1"
    make_publishing_repo "$tmpdir"
    stage_publishing_tools "$tmpdir"
    local repo="$tmpdir/repo"
    printf '{"preflight_reason": "working-tree policy"}\n' >"$repo/.github/ci/policy.json"
    run_hook_in_repo "$tmpdir" scope-only refs/heads/feature "0000000000000000000000000000000000000000"
    assert_exit "dirty policy" "$tmpdir" 0 || return 1
    assert_scope_receipt_base "dirty policy" "$tmpdir" "$(git -C "$repo" rev-parse origin/main)" || return 1
    local receipt
    receipt="$(scope_receipt "$tmpdir")"
    if [ "$(printf '%s' "$receipt" | jq -r '.plan.preflight_reason')" != "committed policy" ] \
        || [ "$(printf '%s' "$receipt" | jq -r '.scope.preflight_reason')" != "committed policy" ]; then
        echo "  the scope receipt describes the working tree's policy, not the committed one:" >&2
        printf '%s' "$receipt" | jq '{plan: .plan.preflight_reason, scope: .scope.preflight_reason}' | sed 's/^/  /' >&2
        return 1
    fi
    # The planner that produced it lived in the committed tree, not the checkout.
    local root
    root="$(planner_root "$tmpdir")"
    if [ -z "$root" ] || [ "$root" = "$(cd "$repo" && pwd -P)" ]; then
        echo "  the planner ran from the checkout ('$root') although its tree was dirty" >&2
        return 1
    fi
    assert_contains "committed tree materialized" "$tmpdir/out" "temporary worktree" || return 1
    assert_no_leftover_worktree "dirty policy" "$tmpdir" || return 1
}

test_a_clean_checkout_is_planned_in_place() {
    local tmpdir="$1"
    make_publishing_repo "$tmpdir"
    stage_publishing_tools "$tmpdir"
    run_hook_in_repo "$tmpdir" scope-only refs/heads/feature "0000000000000000000000000000000000000000"
    assert_exit "clean checkout" "$tmpdir" 0 || return 1
    # A clean checkout IS the committed tree; no worktree is paid for.
    if [ "$(planner_root "$tmpdir")" != "$(cd "$tmpdir/repo" && pwd -P)" ]; then
        echo "  a clean checkout was not planned in place; the planner ran from '$(planner_root "$tmpdir")'" >&2
        return 1
    fi
    assert_not_contains "no worktree announced" "$tmpdir/out" "temporary worktree" || return 1
    assert_no_leftover_worktree "clean checkout" "$tmpdir" || return 1
}

test_a_push_that_does_not_carry_head_reviews_the_pushed_revision() {
    local tmpdir="$1"
    make_publishing_repo "$tmpdir"
    stage_publishing_tools "$tmpdir"
    local repo="$tmpdir/repo" pushed
    local commit=(git -c user.name=hook-test -c user.email=hook@example.com -c commit.gpgsign=false)
    pushed="$(git -C "$repo" rev-parse HEAD)"
    echo "more" >"$repo/more.txt"
    "${commit[@]}" -C "$repo" add -A
    "${commit[@]}" -C "$repo" commit -q -m "more"
    run_hook_with_ref_line "$tmpdir" scope-only \
        "refs/heads/feature $pushed refs/heads/feature 0000000000000000000000000000000000000000"
    assert_exit "push of an older revision" "$tmpdir" 0 || return 1
    assert_contains "pushed revision planned" "$tmpdir/planner.log" "--head $pushed -- work.txt" || return 1
    assert_not_contains "HEAD-only path excluded" "$tmpdir/planner.log" "more.txt" || return 1
    assert_contains "reason announced" "$tmpdir/out" "is not HEAD" || return 1
    # Scope evidence binds HEAD, and HEAD is not going anywhere.
    if scope_receipt "$tmpdir" >/dev/null; then
        echo "  a scope receipt was attached to HEAD for a push that does not carry it" >&2
        return 1
    fi
    assert_no_leftover_worktree "push of an older revision" "$tmpdir" || return 1
}

test_a_deletion_only_push_triggers_nothing_and_is_not_reviewed() {
    local tmpdir="$1"
    make_fake_just "$tmpdir" 0
    run_hook_with_ref_line "$tmpdir" scope-only \
        "(delete) 0000000000000000000000000000000000000000 refs/heads/feature $(git -C "$tmpdir/repo" rev-parse HEAD 2>/dev/null || echo 0)"
    assert_exit "deletion" "$tmpdir" 0 || return 1
    assert_contains "deletion named" "$tmpdir/out" "deletion of refs/heads/feature" || return 1
    assert_contains "nothing to review" "$tmpdir/out" "triggers no run and there is no plan to review" || return 1
    if [ -f "$tmpdir/planner.log" ]; then
        echo "  the planner ran for a push that triggers no run" >&2
        return 1
    fi
}

# ---------- every pushed branch, under its own identity (review-3) ----------
#
# Git hands the hook one line per ref update plus the remote it is pushing to.
# The review must follow each branch update — its own revision, base, remote
# branch, and remote — never the checkout. The review's two reproductions were
# a prohibition on `other` ignored while `feature` was checked out, and a
# second branch's source change never planned.

# A second committed branch, `other`, forked from main with a source change;
# `feature` is checked out again afterwards. Prints the new branch's sha.
#
# Usage: add_other_branch <tmpdir>
add_other_branch() {
    local repo="$1/repo"
    local commit=(git -c user.name=hook-test -c user.email=hook@example.com -c commit.gpgsign=false)
    git -C "$repo" checkout -q -b other main
    echo "other lib" >"$repo/pkg/alpha/src/lib.rs"
    "${commit[@]}" -C "$repo" add -A
    "${commit[@]}" -C "$repo" commit -q -m "other"
    git -C "$repo" checkout -q feature
    git -C "$repo" rev-parse other
}

# A wsl2-ubuntu prohibition scoped by one optional field, whose reason names
# the scope so a refusal can be traced to the record that produced it.
#
# Usage: write_scoped_prohibition <dir> <name> <field> <value>
write_scoped_prohibition() {
    local dir="$1" name="$2" field="$3" value="$4"
    mkdir -p "$dir"
    cat >"$dir/$name.json" <<EOF
{
  "environment": "wsl2-ubuntu",
  "reason": "do not rerun WSL for $field $value",
  "owner": "ken",
  "expiry": "2099-01-01",
  "$field": "$value"
}
EOF
}

# The revisions the stub planner resolved a plan for, in call order (the
# evidence overlay logs too, but carries no `--head`).
planned_heads() {
    sed -n 's/.*--head \([0-9a-f]*\) --.*/\1/p' "$1/planner.log" | tr '\n' ' ' | sed 's/ $//'
}

# The branch updates the hook announced reviewing, in output order.
reviewed_updates() {
    sed -n 's/^Reviewing \(refs\/heads\/[^ ]*\) .*/\1/p' "$1/out" | tr '\n' ' ' | sed 's/ $//'
}

# Forget one run's outputs so the same fixture can be pushed again.
reset_run_outputs() {
    rm -f "$1/planner.log" "$1/out" "$1/err" "$1/exit" "$1/just.log"
}

ZERO_SHA="0000000000000000000000000000000000000000"

test_a_prohibition_on_the_pushed_branch_blocks_whatever_is_checked_out() {
    local tmpdir="$1"
    make_publishing_repo "$tmpdir"
    stage_fixture_tools "$tmpdir"
    make_fake_just "$tmpdir" 0
    local other
    other="$(add_other_branch "$tmpdir")"
    write_scoped_prohibition "$tmpdir/constraints" other branch other
    # `feature` is checked out; only `other` is pushed.
    run_hook_with_ref_line "$tmpdir" strict \
        "refs/heads/other $other refs/heads/other $ZERO_SHA" \
        "BISCUIT_CI_CONSTRAINTS_DIR=$tmpdir/constraints"
    assert_exit "prohibition on the pushed branch" "$tmpdir" 1 || return 1
    assert_contains "record named" "$tmpdir/err" "do not rerun WSL for branch other" || return 1
    assert_contains "update named" "$tmpdir/err" "refs/heads/other -> refs/heads/other" || return 1
    assert_contains "pushed revision planned" "$tmpdir/planner.log" "--head $other -- pkg/alpha/src/lib.rs" || return 1
    assert_log_lacks "no gate after refusal" "$tmpdir" "pre-push" || return 1
    assert_no_leftover_worktree "prohibition on the pushed branch" "$tmpdir" || return 1
}

test_a_prohibition_on_the_checked_out_branch_does_not_bind_another_branch_push() {
    local tmpdir="$1"
    make_publishing_repo "$tmpdir"
    stage_fixture_tools "$tmpdir"
    make_fake_just "$tmpdir" 0
    local other
    other="$(add_other_branch "$tmpdir")"
    write_scoped_prohibition "$tmpdir/constraints" feature branch feature
    run_hook_with_ref_line "$tmpdir" strict \
        "refs/heads/other $other refs/heads/other $ZERO_SHA" \
        "BISCUIT_CI_CONSTRAINTS_DIR=$tmpdir/constraints"
    # The checked-out branch is not what CI validates for this push.
    assert_exit "prohibition on the checkout only" "$tmpdir" 0 || return 1
    assert_not_contains "no refusal" "$tmpdir/err" "Push blocked" || return 1
    assert_contains "identity is the pushed branch" "$tmpdir/out" "Branch identity: other" || return 1
    assert_log_has "gates ran" "$tmpdir" "pre-push" || return 1
}

test_every_pushed_branch_is_planned_with_its_own_path_set() {
    local tmpdir="$1"
    make_publishing_repo "$tmpdir"
    stage_fixture_tools "$tmpdir"
    make_fake_just "$tmpdir" 0
    local repo="$tmpdir/repo" feature other
    feature="$(git -C "$repo" rev-parse HEAD)"
    other="$(add_other_branch "$tmpdir")"
    write_scoped_prohibition "$tmpdir/constraints" other branch other
    run_hook_with_ref_line "$tmpdir" strict \
        "refs/heads/feature $feature refs/heads/feature $ZERO_SHA"$'\n'"refs/heads/other $other refs/heads/other $ZERO_SHA" \
        "BISCUIT_CI_CONSTRAINTS_DIR=$tmpdir/constraints"
    # `feature` passes; the second update is what the prohibition binds.
    assert_exit "second branch prohibited" "$tmpdir" 1 || return 1
    assert_contains "record named" "$tmpdir/err" "do not rerun WSL for branch other" || return 1
    assert_contains "blocked update named" "$tmpdir/err" "Blocked while reviewing refs/heads/other -> refs/heads/other" || return 1
    assert_contains "feature planned with its paths" "$tmpdir/planner.log" "--head $feature -- work.txt" || return 1
    assert_contains "other planned with its paths" "$tmpdir/planner.log" "--head $other -- pkg/alpha/src/lib.rs" || return 1
    if [ "$(planned_heads "$tmpdir")" != "$feature $other" ]; then
        echo "  expected both revisions planned in order, got: $(planned_heads "$tmpdir")" >&2
        return 1
    fi
    assert_log_lacks "no gate after refusal" "$tmpdir" "pre-push" || return 1
    # Blocked before publication: the passing HEAD update left no scope note.
    if scope_receipt "$tmpdir" >/dev/null; then
        echo "  a scope receipt was published although a later update was blocked" >&2
        return 1
    fi
    assert_no_leftover_worktree "second branch prohibited" "$tmpdir" || return 1
}

test_ref_updates_are_reviewed_in_the_order_git_supplies_them() {
    local tmpdir="$1"
    make_publishing_repo "$tmpdir"
    stage_fixture_tools "$tmpdir"
    make_fake_just "$tmpdir" 0
    local repo="$tmpdir/repo" feature other
    feature="$(git -C "$repo" rev-parse HEAD)"
    other="$(add_other_branch "$tmpdir")"
    local feature_line="refs/heads/feature $feature refs/heads/feature $ZERO_SHA"
    local other_line="refs/heads/other $other refs/heads/other $ZERO_SHA"

    run_hook_with_ref_line "$tmpdir" scope-only "$feature_line"$'\n'"$other_line"
    assert_exit "feature then other" "$tmpdir" 0 || return 1
    if [ "$(reviewed_updates "$tmpdir")" != "refs/heads/feature refs/heads/other" ] \
        || [ "$(planned_heads "$tmpdir")" != "$feature $other" ]; then
        echo "  feature-first input was reviewed as: $(reviewed_updates "$tmpdir"); planned: $(planned_heads "$tmpdir")" >&2
        return 1
    fi

    reset_run_outputs "$tmpdir"
    run_hook_with_ref_line "$tmpdir" scope-only "$other_line"$'\n'"$feature_line"
    assert_exit "other then feature" "$tmpdir" 0 || return 1
    if [ "$(reviewed_updates "$tmpdir")" != "refs/heads/other refs/heads/feature" ] \
        || [ "$(planned_heads "$tmpdir")" != "$other $feature" ]; then
        echo "  other-first input was reviewed as: $(reviewed_updates "$tmpdir"); planned: $(planned_heads "$tmpdir")" >&2
        return 1
    fi

    # The same prohibition blocks whichever position the bound update holds.
    write_scoped_prohibition "$tmpdir/constraints" other branch other
    reset_run_outputs "$tmpdir"
    run_hook_with_ref_line "$tmpdir" scope-only "$other_line"$'\n'"$feature_line" \
        "BISCUIT_CI_CONSTRAINTS_DIR=$tmpdir/constraints"
    assert_exit "other first, prohibited" "$tmpdir" 1 || return 1
    assert_contains "record named" "$tmpdir/err" "do not rerun WSL for branch other" || return 1
    reset_run_outputs "$tmpdir"
    run_hook_with_ref_line "$tmpdir" scope-only "$feature_line"$'\n'"$other_line" \
        "BISCUIT_CI_CONSTRAINTS_DIR=$tmpdir/constraints"
    assert_exit "other last, prohibited" "$tmpdir" 1 || return 1
    assert_contains "record named" "$tmpdir/err" "do not rerun WSL for branch other" || return 1
    assert_no_leftover_worktree "ordering" "$tmpdir" || return 1
}

test_the_repository_identity_is_the_remote_being_pushed_to() {
    local tmpdir="$1"
    make_publishing_repo "$tmpdir"
    stage_fixture_tools "$tmpdir"
    make_fake_just "$tmpdir" 0
    local repo="$tmpdir/repo" head
    head="$(git -C "$repo" rev-parse HEAD)"
    git init -q --bare "$tmpdir/upstream.git"
    git -C "$repo" remote add upstream "$tmpdir/upstream.git"
    git -C "$repo" push -q upstream main
    local line="refs/heads/feature $head refs/heads/feature $ZERO_SHA"

    # Scoped to upstream: binds a push to upstream even though origin differs.
    write_scoped_prohibition "$tmpdir/constraints" upstream repository "$tmpdir/upstream.git"
    run_hook_as_remote "$tmpdir" scope-only upstream "$tmpdir/upstream.git" "$line" \
        "BISCUIT_CI_CONSTRAINTS_DIR=$tmpdir/constraints"
    assert_exit "push to the constrained remote" "$tmpdir" 1 || return 1
    assert_contains "record named" "$tmpdir/err" "do not rerun WSL for repository $tmpdir/upstream.git" || return 1
    assert_contains "remote announced" "$tmpdir/out" "Remote: upstream ($tmpdir/upstream.git)" || return 1

    # Scoped to origin: a push to upstream is not a push to origin.
    rm -f "$tmpdir/constraints/upstream.json"
    write_scoped_prohibition "$tmpdir/constraints" origin repository "$tmpdir/origin.git"
    reset_run_outputs "$tmpdir"
    run_hook_as_remote "$tmpdir" scope-only upstream "$tmpdir/upstream.git" "$line" \
        "BISCUIT_CI_CONSTRAINTS_DIR=$tmpdir/constraints"
    assert_exit "push to another remote" "$tmpdir" 0 || return 1
    assert_not_contains "no refusal" "$tmpdir/err" "Push blocked" || return 1
}

test_a_renamed_refspec_binds_a_record_under_either_branch_name() {
    local tmpdir="$1"
    make_publishing_repo "$tmpdir"
    stage_fixture_tools "$tmpdir"
    make_fake_just "$tmpdir" 0
    local repo="$tmpdir/repo" head
    head="$(git -C "$repo" rev-parse HEAD)"
    # `git push origin feature:other`
    local line="refs/heads/feature $head refs/heads/other $ZERO_SHA"

    write_scoped_prohibition "$tmpdir/constraints" other branch other
    run_hook_with_ref_line "$tmpdir" scope-only "$line" "BISCUIT_CI_CONSTRAINTS_DIR=$tmpdir/constraints"
    assert_exit "record under the remote name" "$tmpdir" 1 || return 1
    assert_contains "record named" "$tmpdir/err" "do not rerun WSL for branch other" || return 1
    assert_contains "both names printed" "$tmpdir/out" "Branch identity: other (pushed from feature" || return 1

    rm -f "$tmpdir/constraints/other.json"
    write_scoped_prohibition "$tmpdir/constraints" feature branch feature
    reset_run_outputs "$tmpdir"
    run_hook_with_ref_line "$tmpdir" scope-only "$line" "BISCUIT_CI_CONSTRAINTS_DIR=$tmpdir/constraints"
    assert_exit "record under the local name" "$tmpdir" 1 || return 1
    assert_contains "record named" "$tmpdir/err" "do not rerun WSL for branch feature" || return 1

    rm -f "$tmpdir/constraints/feature.json"
    write_scoped_prohibition "$tmpdir/constraints" main branch main
    reset_run_outputs "$tmpdir"
    run_hook_with_ref_line "$tmpdir" scope-only "$line" "BISCUIT_CI_CONSTRAINTS_DIR=$tmpdir/constraints"
    assert_exit "record under neither name" "$tmpdir" 0 || return 1
}

test_deletions_and_tags_among_branch_updates_are_skipped_by_name() {
    local tmpdir="$1"
    make_publishing_repo "$tmpdir"
    stage_publishing_tools "$tmpdir"
    local repo="$tmpdir/repo" head main_sha
    head="$(git -C "$repo" rev-parse HEAD)"
    main_sha="$(git -C "$repo" rev-parse origin/main)"
    run_hook_with_ref_line "$tmpdir" scope-only \
        "(delete) $ZERO_SHA refs/heads/stale $main_sha"$'\n'"refs/heads/feature $head refs/heads/feature $ZERO_SHA"$'\n'"refs/tags/v1 $head refs/tags/v1 $ZERO_SHA"
    assert_exit "mixed push" "$tmpdir" 0 || return 1
    assert_contains "deletion named" "$tmpdir/out" "deletion of refs/heads/stale" || return 1
    assert_contains "tag named" "$tmpdir/out" "refs/tags/v1 -> refs/tags/v1: only a branch update triggers a run" || return 1
    # Exactly the branch update was planned, and HEAD's scope still went out.
    if [ "$(planned_heads "$tmpdir")" != "$head" ]; then
        echo "  expected only the branch update planned, got: $(planned_heads "$tmpdir")" >&2
        return 1
    fi
    assert_contains "scope published for HEAD" "$tmpdir/out" "Published scope evidence" || return 1
    assert_scope_receipt_base "mixed push" "$tmpdir" "$main_sha" || return 1
}

# ---------- the base of every run the update triggers (review-4) ----------
#
# ci.yml fires `push` for main only and `pull_request` for every target
# branch, and a pull_request run diffs from the TARGET BRANCH TIP at event
# time. The hook must therefore plan a feature branch against each open pull
# request's target as it stands on the remote, never against origin/main by
# assumption. The bare fixture remote cannot hold a pull request, so a stub
# `gh` on the harness PATH answers `pr list` from a fixture and logs what it
# was asked, while `origin` keeps pointing at the bare repository the hook
# fetches from; the GitHub-shaped URL is what Git would report as `$2`.

GITHUB_URL="git@github.com:acme/widgets.git"

# A `gh` that logs its argv to gh.log and answers `pr list` with the given
# `<number> <base branch>` rows (none: no open pull request). A row with a
# third field, `<number> <base branch> <head branch>`, is answered only when
# `pr list` asks for that `--head`, so a push carrying several branches can
# hold a pull request from one of them. A `gh-fail` marker makes it fail the
# way an unauthenticated `gh` does.
#
# Usage: stage_fake_gh <tmpdir> [row...]
stage_fake_gh() {
    local tmpdir="$1"
    shift
    local json="[" sep="" row number rest base head entry existing
    rm -f "$tmpdir"/gh-reply-*.json
    for row in "$@"; do
        number="${row%% *}"
        rest="${row#* }"
        base="${rest%% *}"
        head=""
        case "$rest" in *" "*) head="${rest#* }" ;; esac
        entry="{\"number\":$number,\"baseRefName\":\"$base\"}"
        if [ -z "$head" ]; then
            json="$json$sep$entry"
            sep=","
        elif [ -s "$tmpdir/gh-reply-$head.json" ]; then
            existing="$(sed 's/]$//' "$tmpdir/gh-reply-$head.json")"
            printf '%s,%s]' "$existing" "$entry" >"$tmpdir/gh-reply-$head.json"
        else
            printf '[%s]' "$entry" >"$tmpdir/gh-reply-$head.json"
        fi
    done
    printf '%s]' "$json" >"$tmpdir/gh-reply.json"
    cat >"$tmpdir/gh" <<EOF
#!/bin/sh
echo "\$*" >>"$tmpdir/gh.log"
if [ -f "$tmpdir/gh-fail" ]; then
    echo "gh: To get started with GitHub CLI, please run: gh auth login" >&2
    exit 4
fi
head=""
while [ \$# -gt 0 ]; do
    if [ "\$1" = "--head" ] && [ \$# -gt 1 ]; then
        head="\$2"
    fi
    shift
done
if [ -n "\$head" ] && [ -f "$tmpdir/gh-reply-\$head.json" ]; then
    cat "$tmpdir/gh-reply-\$head.json"
else
    cat "$tmpdir/gh-reply.json"
fi
EOF
    chmod +x "$tmpdir/gh"
}

# The review's reproduction: `parent` (from main) changes pkg/alpha/src/lib.rs
# and is on the remote; `child` (from parent) restores the base contents and
# is checked out as HEAD. With `behind`, the remote's `parent` instead still
# holds main's commit — the parent's change is an update THIS push carries
# (review-5). Sets MAIN_SHA, PARENT_SHA, and CHILD_SHA.
#
# Usage: make_stacked_repo <tmpdir> [pushed|behind]
make_stacked_repo() {
    local tmpdir="$1" repo="$1/repo" remote_parent="${2:-pushed}"
    local commit=(git -c user.name=hook-test -c user.email=hook@example.com -c commit.gpgsign=false)
    make_publishing_repo "$tmpdir"
    stage_fixture_tools "$tmpdir"
    git -C "$repo" checkout -q -b parent main
    echo "parent lib" >"$repo/pkg/alpha/src/lib.rs"
    "${commit[@]}" -C "$repo" add -A
    "${commit[@]}" -C "$repo" commit -q -m "parent"
    if [ "$remote_parent" = "behind" ]; then
        git -C "$repo" push -q origin main:refs/heads/parent
    else
        git -C "$repo" push -q origin parent
    fi
    git -C "$repo" checkout -q -b child parent
    echo "base lib" >"$repo/pkg/alpha/src/lib.rs"
    "${commit[@]}" -C "$repo" add -A
    "${commit[@]}" -C "$repo" commit -q -m "child"
    MAIN_SHA="$(git -C "$repo" rev-parse main)"
    PARENT_SHA="$(git -C "$repo" rev-parse parent)"
    CHILD_SHA="$(git -C "$repo" rev-parse child)"
}

# Push `child` to the GitHub-shaped remote in the given mode.
run_stacked_push() {
    local tmpdir="$1" mode="$2"
    shift 2
    run_hook_as_remote "$tmpdir" "$mode" origin "$GITHUB_URL" \
        "refs/heads/child $CHILD_SHA refs/heads/child $ZERO_SHA" "$@"
}

# A `--base <base> --head <head> --` planner call with NOTHING after the `--`.
assert_planned_with_no_paths() {
    local label="$1" tmpdir="$2" base="$3" head="$4"
    if grep -q -- "--base $base --head $head --\$" "$tmpdir/planner.log"; then
        return 0
    fi
    echo "  expected the planner to be handed an empty path set for base $base ($label):" >&2
    sed 's/^/  /' "$tmpdir/planner.log" >&2
    return 1
}

test_a_stacked_pull_request_is_planned_against_its_target_branch_tip() {
    local tmpdir="$1"
    make_stacked_repo "$tmpdir"
    make_fake_just "$tmpdir" 0 "$FIXTURES/plan-wsl-executing.json"
    stage_fake_gh "$tmpdir" "12 parent"
    write_prohibition "$tmpdir/constraints" "wsl2-ubuntu" "2099-01-01"
    run_stacked_push "$tmpdir" strict "BISCUIT_CI_CONSTRAINTS_DIR=$tmpdir/constraints"
    # CI diffs parent's tip against child, which selects alpha; the WSL cell
    # would execute, so the prohibition blocks.
    assert_exit "stacked pull request" "$tmpdir" 1 || return 1
    assert_contains "planned against the target's tip" "$tmpdir/planner.log" \
        "--base $PARENT_SHA --head $CHILD_SHA -- pkg/alpha/src/lib.rs" || return 1
    assert_contains "trigger named" "$tmpdir/out" "pull request #12 into parent" || return 1
    assert_contains "gh asked about this head" "$tmpdir/gh.log" "--repo acme/widgets --head child" || return 1
    assert_contains "reason stated" "$tmpdir/err" "do not rerun WSL for this branch" || return 1
    assert_log_lacks "no gate after refusal" "$tmpdir" "pre-push" || return 1
    assert_no_leftover_worktree "stacked pull request" "$tmpdir" || return 1
}

# The inverse, and what the old hook did for every branch: against main the
# child's tree is unchanged, so nothing is selected and nothing blocks.
test_a_pull_request_into_main_from_a_restoring_child_selects_nothing() {
    local tmpdir="$1"
    make_stacked_repo "$tmpdir"
    make_fake_just "$tmpdir" 0 "$FIXTURES/plan-wsl-executing.json"
    stage_fake_gh "$tmpdir" "12 main"
    write_prohibition "$tmpdir/constraints" "wsl2-ubuntu" "2099-01-01"
    local main_sha
    main_sha="$(git -C "$tmpdir/repo" rev-parse origin/main)"
    run_stacked_push "$tmpdir" strict "BISCUIT_CI_CONSTRAINTS_DIR=$tmpdir/constraints"
    assert_exit "pull request into main" "$tmpdir" 0 || return 1
    assert_planned_with_no_paths "into main" "$tmpdir" "$main_sha" "$CHILD_SHA" || return 1
    assert_contains "trigger named" "$tmpdir/out" "pull request #12 into main" || return 1
    assert_not_contains "no refusal" "$tmpdir/err" "Push blocked" || return 1
    assert_log_has "gates ran" "$tmpdir" "pre-push" || return 1
}

test_an_advanced_target_branch_is_read_from_the_remote_and_records_exact_scope() {
    local tmpdir="$1"
    make_publishing_repo "$tmpdir"
    stage_fixture_tools "$tmpdir"
    make_fake_just "$tmpdir" 0
    stage_fake_gh "$tmpdir" "7 main"
    local repo="$tmpdir/repo" head new_main
    local commit=(git -c user.name=hook-test -c user.email=hook@example.com -c commit.gpgsign=false)
    head="$(git -C "$repo" rev-parse HEAD)"
    # main advances on the remote through another clone, touching a source
    # file; the checkout's origin/main is left STALE on purpose.
    git clone -q "$tmpdir/origin.git" "$tmpdir/second"
    echo "advanced lib" >"$tmpdir/second/pkg/alpha/src/lib.rs"
    "${commit[@]}" -C "$tmpdir/second" add -A
    "${commit[@]}" -C "$tmpdir/second" commit -q -m "advance main"
    git -C "$tmpdir/second" push -q origin main
    new_main="$(git -C "$tmpdir/second" rev-parse main)"
    if [ "$(git -C "$repo" rev-parse origin/main)" = "$new_main" ]; then
        echo "  fixture error: the tracking ref is not stale, so the test cannot tell the remote from it" >&2
        return 1
    fi
    run_hook_as_remote "$tmpdir" scope-only origin "$GITHUB_URL" \
        "refs/heads/feature $head refs/heads/feature $ZERO_SHA"
    assert_exit "advanced target" "$tmpdir" 0 || return 1
    # CI's two-dot diff from the advanced tip includes main's own change.
    assert_contains "planned against the remote's tip" "$tmpdir/planner.log" \
        "--base $new_main --head $head -- pkg/alpha/src/lib.rs work.txt" || return 1
    assert_contains "trigger named" "$tmpdir/out" "pull request #7 into main" || return 1
    scope_receipt "$tmpdir" >"$tmpdir/receipt.json"
    jq -e --arg base "$new_main" '.base == $base' "$tmpdir/receipt.json" >/dev/null || return 1
    assert_contains "scope published" "$tmpdir/out" "Published scope evidence" || return 1
}

test_a_github_branch_with_no_open_pull_request_gets_a_provisional_plan() {
    local tmpdir="$1"
    make_publishing_repo "$tmpdir"
    stage_fixture_tools "$tmpdir"
    make_fake_just "$tmpdir" 0
    stage_fake_gh "$tmpdir"
    local repo="$tmpdir/repo" head main_sha
    head="$(git -C "$repo" rev-parse HEAD)"
    main_sha="$(git -C "$repo" rev-parse origin/main)"
    local line="refs/heads/feature $head refs/heads/feature $ZERO_SHA"

    # Constrained all the same: the pull request opened next is the trigger.
    write_prohibition "$tmpdir/constraints" "wsl2-ubuntu" "2099-01-01"
    run_hook_as_remote "$tmpdir" scope-only origin "$GITHUB_URL" "$line" \
        "BISCUIT_CI_CONSTRAINTS_DIR=$tmpdir/constraints"
    assert_exit "provisional plan, prohibited" "$tmpdir" 1 || return 1
    assert_contains "gh asked" "$tmpdir/gh.log" "--repo acme/widgets --head feature" || return 1
    assert_contains "provisional named" "$tmpdir/out" \
        "provisional: no open pull request from feature on acme/widgets" || return 1
    assert_contains "provisional trigger explained" "$tmpdir/out" \
        "a pull request opened against main before main advances will run this plan" || return 1
    assert_contains "reason stated" "$tmpdir/err" "do not rerun WSL for this branch" || return 1

    reset_run_outputs "$tmpdir"
    run_hook_as_remote "$tmpdir" scope-only origin "$GITHUB_URL" "$line"
    assert_exit "provisional plan" "$tmpdir" 0 || return 1
    assert_contains "planned against the remote's main" "$tmpdir/planner.log" \
        "--base $main_sha --head $head -- work.txt" || return 1
    assert_scope_receipt_base "provisional plan" "$tmpdir" "$main_sha" || return 1
}

# Review-5: a record written in one session must bind a fresh one that never
# set BISCUIT_CI_CONSTRAINTS_DIR (`env -i` above guarantees it is unset). The
# default store is derived from the remote being pushed to — the GitHub URL
# here, so the directory is `github.com/acme/widgets` — and a record filed
# under another repository's directory does not bind.
test_a_record_in_the_default_store_binds_a_fresh_session() {
    local tmpdir="$1"
    make_publishing_repo "$tmpdir"
    stage_fixture_tools "$tmpdir"
    make_fake_just "$tmpdir" 0
    stage_fake_gh "$tmpdir"
    local repo="$tmpdir/repo" head store="$tmpdir/home/.rusty-biscuit/ci-constraints"
    head="$(git -C "$repo" rev-parse HEAD)"
    local line="refs/heads/feature $head refs/heads/feature $ZERO_SHA"

    write_scoped_prohibition "$store/github.com/other/repo" other repository "github.com/other/repo"
    write_prohibition "$store/github.com/acme/widgets" "wsl2-ubuntu" "2099-01-01"
    run_hook_as_remote "$tmpdir" strict origin "$GITHUB_URL" "$line"
    assert_exit "fresh session, record in the default store" "$tmpdir" 1 || return 1
    assert_contains "record named" "$tmpdir/err" "do not rerun WSL for this branch" || return 1
    assert_log_lacks "no gate ran" "$tmpdir" "pre-push" || return 1

    # Only the other repository's record remains; it lives outside this
    # repository's directory, so a fresh session is not bound by it.
    rm -f "$store/github.com/acme/widgets/current.json"
    reset_run_outputs "$tmpdir"
    run_hook_as_remote "$tmpdir" strict origin "$GITHUB_URL" "$line"
    assert_exit "another repository's record" "$tmpdir" 0 || return 1
    assert_not_contains "no refusal" "$tmpdir/err" "Push blocked" || return 1
}

test_two_open_pull_requests_from_one_head_are_each_reviewed() {
    local tmpdir="$1"
    make_stacked_repo "$tmpdir"
    make_fake_just "$tmpdir" 0 "$FIXTURES/plan-wsl-executing.json"
    stage_fake_gh "$tmpdir" "12 main" "13 parent"
    local repo="$tmpdir/repo" main_sha receipt
    main_sha="$(git -C "$repo" rev-parse origin/main)"

    run_stacked_push "$tmpdir" scope-only
    assert_exit "two pull requests" "$tmpdir" 0 || return 1
    if [ "$(grep -c '^Comparison base for ' "$tmpdir/out")" != "2" ]; then
        echo "  expected one comparison base per open pull request:" >&2
        grep '^Comparison base for ' "$tmpdir/out" | sed 's/^/  /' >&2
        return 1
    fi
    assert_contains "first trigger named" "$tmpdir/out" "pull request #12 into main" || return 1
    assert_contains "second trigger named" "$tmpdir/out" "pull request #13 into parent" || return 1
    assert_planned_with_no_paths "into main" "$tmpdir" "$main_sha" "$CHILD_SHA" || return 1
    assert_contains "planned against parent" "$tmpdir/planner.log" \
        "--base $PARENT_SHA --head $CHILD_SHA -- pkg/alpha/src/lib.rs" || return 1
    # One note holds one receipt: the first context's, and the hook says so.
    assert_contains "receipt scope announced" "$tmpdir/out" "binds its first comparison base only" || return 1
    if ! receipt="$(scope_receipt "$tmpdir")"; then
        echo "  no scope receipt is attached to HEAD" >&2
        return 1
    fi
    if [ "$(printf '%s' "$receipt" | jq -r '.base + " " + .head + " " + .plan.base')" != "$main_sha $CHILD_SHA $main_sha" ]; then
        echo "  the scope receipt does not bind the first context (main $main_sha):" >&2
        printf '%s' "$receipt" | jq '{base, head, plan: .plan.base}' | sed 's/^/  /' >&2
        return 1
    fi

    # A prohibition only the second context violates still blocks.
    write_prohibition "$tmpdir/constraints" "wsl2-ubuntu" "2099-01-01"
    reset_run_outputs "$tmpdir"
    run_stacked_push "$tmpdir" scope-only "BISCUIT_CI_CONSTRAINTS_DIR=$tmpdir/constraints"
    assert_exit "second context prohibited" "$tmpdir" 1 || return 1
    assert_contains "first context passed" "$tmpdir/out" "pull request #12 into main" || return 1
    assert_contains "second context named" "$tmpdir/out" "pull request #13 into parent" || return 1
    assert_contains "blocked update named" "$tmpdir/err" "Blocked while reviewing refs/heads/child -> refs/heads/child" || return 1
    assert_no_leftover_worktree "two pull requests" "$tmpdir" || return 1
}

test_a_github_remote_without_gh_blocks_and_names_it() {
    local tmpdir="$1"
    make_publishing_repo "$tmpdir"
    stage_fixture_tools "$tmpdir"
    make_fake_just "$tmpdir" 0
    local head
    head="$(git -C "$tmpdir/repo" rev-parse HEAD)"
    # No `gh` on the private PATH: the pull requests cannot be listed, so the
    # plan cannot be known, so the push is refused rather than guessed.
    run_hook_as_remote "$tmpdir" scope-only origin "$GITHUB_URL" \
        "refs/heads/feature $head refs/heads/feature $ZERO_SHA"
    assert_exit "no gh" "$tmpdir" 2 || return 1
    assert_contains "gh named" "$tmpdir/err" "\`gh\` is not installed" || return 1
    assert_contains "bypass named" "$tmpdir/err" "--no-verify" || return 1
    if [ -f "$tmpdir/planner.log" ]; then
        echo "  a plan was resolved without knowing which run it stands for" >&2
        return 1
    fi
}

test_a_failing_gh_blocks_and_names_the_failure() {
    local tmpdir="$1"
    make_publishing_repo "$tmpdir"
    stage_fixture_tools "$tmpdir"
    make_fake_just "$tmpdir" 0
    stage_fake_gh "$tmpdir"
    : >"$tmpdir/gh-fail"
    local head
    head="$(git -C "$tmpdir/repo" rev-parse HEAD)"
    run_hook_as_remote "$tmpdir" scope-only origin "$GITHUB_URL" \
        "refs/heads/feature $head refs/heads/feature $ZERO_SHA"
    assert_exit "gh failed" "$tmpdir" 2 || return 1
    assert_contains "command named" "$tmpdir/err" "\`gh pr list\` failed with: gh: To get started with GitHub CLI, please run: gh auth login" || return 1
    assert_contains "bypass named" "$tmpdir/err" "--no-verify" || return 1
    if [ -f "$tmpdir/planner.log" ]; then
        echo "  a plan was resolved although the pull requests could not be listed" >&2
        return 1
    fi
}

test_a_non_github_remote_never_asks_gh() {
    local tmpdir="$1"
    make_publishing_repo "$tmpdir"
    stage_fixture_tools "$tmpdir"
    make_fake_just "$tmpdir" 0
    stage_fake_gh "$tmpdir" "99 main"
    run_hook_in_repo "$tmpdir" scope-only refs/heads/feature "$ZERO_SHA"
    assert_exit "bare remote" "$tmpdir" 0 || return 1
    if [ -f "$tmpdir/gh.log" ]; then
        echo "  gh was consulted for a remote that can hold no pull request:" >&2
        sed 's/^/  /' "$tmpdir/gh.log" >&2
        return 1
    fi
    assert_contains "provisional named" "$tmpdir/out" \
        "provisional: no open pull request, since $tmpdir/origin.git is not a GitHub remote" || return 1
    assert_scope_receipt_base "bare remote" "$tmpdir" "$(git -C "$tmpdir/repo" rev-parse origin/main)" || return 1
}

# ---------- the target branch travels in the same push (review-5) ----------
#
# A push can carry a pull request's head AND its target. The server applies
# the two updates in an order this hook cannot know, so the pull request's run
# may compare against the target's current tip or against the incoming
# revision. The review's reproduction is a child that restores the ORIGINAL
# library over a parent that changed it: against the remote's parent (still
# main's tree) the child selects nothing, against the incoming parent it
# selects the package. The old hook read the target from the remote alone and
# let a child-only prohibition through in either ref order.

# The two ref lines of that push, in the given order.
#
# Usage: simultaneous_ref_lines <parent first|child first>
simultaneous_ref_lines() {
    local parent_line="refs/heads/parent $PARENT_SHA refs/heads/parent $MAIN_SHA"
    local child_line="refs/heads/child $CHILD_SHA refs/heads/child $ZERO_SHA"
    if [ "$1" = "parent first" ]; then
        printf '%s\n%s' "$parent_line" "$child_line"
    else
        printf '%s\n%s' "$child_line" "$parent_line"
    fi
}

test_a_target_updated_in_the_same_push_is_reviewed_at_its_incoming_revision() {
    local tmpdir="$1"
    make_stacked_repo "$tmpdir" behind
    make_fake_just "$tmpdir" 0 "$FIXTURES/plan-wsl-executing.json"
    stage_fake_gh "$tmpdir" "12 parent child"
    write_scoped_prohibition "$tmpdir/constraints" child branch child
    local order
    for order in "parent first" "child first"; do
        reset_run_outputs "$tmpdir"
        run_hook_as_remote "$tmpdir" strict origin "$GITHUB_URL" "$(simultaneous_ref_lines "$order")" \
            "BISCUIT_CI_CONSTRAINTS_DIR=$tmpdir/constraints"
        assert_exit "$order" "$tmpdir" 1 || return 1
        # The state the old hook stopped at: the remote's parent is main's
        # tree, so the child selects nothing there ...
        assert_planned_with_no_paths "$order, current parent" "$tmpdir" "$MAIN_SHA" "$CHILD_SHA" || return 1
        # ... and the state it never reviewed: the incoming parent, against
        # which the child changes the package and the WSL cell would execute.
        assert_contains "$order, incoming parent planned" "$tmpdir/planner.log" \
            "--base $PARENT_SHA --head $CHILD_SHA -- pkg/alpha/src/lib.rs" || return 1
        assert_contains "$order, incoming state named" "$tmpdir/out" \
            "pull request #12 into parent, the revision this push also writes to parent" || return 1
        assert_contains "$order, record named" "$tmpdir/err" "do not rerun WSL for branch child" || return 1
        assert_contains "$order, blocked update named" "$tmpdir/err" \
            "Blocked while reviewing refs/heads/child -> refs/heads/child" || return 1
        assert_log_lacks "$order, no gate after refusal" "$tmpdir" "pre-push" || return 1
        assert_no_leftover_worktree "$order" "$tmpdir" || return 1
    done
}

# The control: the same push with nothing prohibited passes in either order,
# with the child's run reviewed in both states and the parent's provisional
# plan once. The receipt keeps binding HEAD's first context — the current
# remote parent — so the run that sees the incoming parent misses it and CI
# calculates that scope itself.
test_a_simultaneous_push_with_no_prohibited_work_passes_in_either_order() {
    local tmpdir="$1"
    make_stacked_repo "$tmpdir" behind
    make_fake_just "$tmpdir" 0 "$FIXTURES/plan-wsl-executing.json"
    stage_fake_gh "$tmpdir" "12 parent child"
    local order short_child receipt
    short_child="$(git -C "$tmpdir/repo" rev-parse --short=9 "$CHILD_SHA")"
    for order in "parent first" "child first"; do
        reset_run_outputs "$tmpdir"
        run_hook_as_remote "$tmpdir" scope-only origin "$GITHUB_URL" "$(simultaneous_ref_lines "$order")"
        assert_exit "$order" "$tmpdir" 0 || return 1
        if [ "$(grep -c "^Comparison base for $short_child: " "$tmpdir/out")" != "2" ]; then
            echo "  expected the child's run to be reviewed in two states ($order):" >&2
            grep '^Comparison base for ' "$tmpdir/out" | sed 's/^/  /' >&2
            return 1
        fi
        assert_contains "$order, both states announced" "$tmpdir/out" \
            "parent is updated by this push too, so the run of pull request #12 may see either its current tip or the incoming revision as its base" || return 1
        assert_contains "$order, current state named" "$tmpdir/out" \
            "pull request #12 into parent, the tip of parent on origin" || return 1
        assert_contains "$order, incoming state named" "$tmpdir/out" \
            "pull request #12 into parent, the revision this push also writes to parent" || return 1
        assert_planned_with_no_paths "$order, current parent" "$tmpdir" "$MAIN_SHA" "$CHILD_SHA" || return 1
        assert_contains "$order, incoming parent planned" "$tmpdir/planner.log" \
            "--base $PARENT_SHA --head $CHILD_SHA -- pkg/alpha/src/lib.rs" || return 1
        # The parent has no pull request of its own: one provisional plan.
        assert_contains "$order, parent planned provisionally" "$tmpdir/planner.log" \
            "--base $MAIN_SHA --head $PARENT_SHA -- pkg/alpha/src/lib.rs" || return 1
        assert_not_contains "$order, no refusal" "$tmpdir/err" "Blocked while reviewing" || return 1
        assert_contains "$order, receipt scope announced" "$tmpdir/out" "binds its first comparison base only" || return 1
        if ! receipt="$(scope_receipt "$tmpdir")"; then
            echo "  no scope receipt is attached to HEAD ($order)" >&2
            return 1
        fi
        if [ "$(printf '%s' "$receipt" | jq -r '.base + " " + .head')" != "$MAIN_SHA $CHILD_SHA" ]; then
            echo "  the scope receipt does not bind the current remote parent ($order):" >&2
            printf '%s' "$receipt" | jq '{base, head}' | sed 's/^/  /' >&2
            return 1
        fi
        assert_no_leftover_worktree "$order" "$tmpdir" || return 1
    done
}

# A pull request into a branch this push DELETES has no base the hook can
# establish — the run may fire against the old tip before the deletion lands,
# and the pull request has no base at all afterwards — so the update is
# refused by name, in either order, before any plan is resolved.
test_a_pull_request_into_a_branch_this_push_deletes_is_refused() {
    local tmpdir="$1"
    make_stacked_repo "$tmpdir"
    make_fake_just "$tmpdir" 0 "$FIXTURES/plan-wsl-executing.json"
    stage_fake_gh "$tmpdir" "12 parent child"
    local delete_line="(delete) $ZERO_SHA refs/heads/parent $PARENT_SHA"
    local child_line="refs/heads/child $CHILD_SHA refs/heads/child $ZERO_SHA"
    local order lines
    for order in "deletion first" "child first"; do
        if [ "$order" = "deletion first" ]; then
            lines="$delete_line"$'\n'"$child_line"
        else
            lines="$child_line"$'\n'"$delete_line"
        fi
        reset_run_outputs "$tmpdir"
        run_hook_as_remote "$tmpdir" scope-only origin "$GITHUB_URL" "$lines"
        assert_exit "$order" "$tmpdir" 2 || return 1
        assert_contains "$order, pull request named" "$tmpdir/err" \
            "the base of pull request #12 into parent cannot be established" || return 1
        assert_contains "$order, deletion named" "$tmpdir/err" \
            "this push also deletes parent (refs/heads/parent)" || return 1
        assert_contains "$order, bypass named" "$tmpdir/err" "--no-verify" || return 1
        if [ -f "$tmpdir/planner.log" ]; then
            echo "  a plan was resolved for a pull request whose base cannot be established ($order):" >&2
            sed 's/^/  /' "$tmpdir/planner.log" >&2
            return 1
        fi
        if [ "$order" = "deletion first" ]; then
            # The deletion itself is still skipped by name, as before.
            assert_contains "deletion skipped" "$tmpdir/out" "Not reviewing the deletion of refs/heads/parent" || return 1
        fi
        assert_no_leftover_worktree "$order" "$tmpdir" || return 1
    done
}

# ---------- plan-fed local gates (ruling D2; audit W1, W10, W14) ----------
#
# These fixtures run the REAL `just/ci-local.just` from the hook: the fake
# `just` answers `pre-push` by invoking the real `just` on a harness justfile
# that imports the repository's recipe, and answers the recipe's own gate
# calls (`_lint`, `_test`) by logging them and staging a JUnit report the way
# the canonical tier recipes do. The harness planner logs every call with a
# `ci-local` prefix, answers `--apply-to` (the projection, which reads nothing
# from the checkout) with the real planner, and refuses selection outright:
# a recipe that replanned would fail the run rather than pass unnoticed.

stage_ci_local_harness() {
    local tmpdir="$1" harness="$1/harness"
    if [ -z "$REAL_JUST" ]; then
        echo "  the plan-fed fixtures need the real \`just\` on PATH" >&2
        return 1
    fi
    mkdir -p "$harness/scripts/ci"
    cp "$CI_LOCAL_RECIPE" "$harness/ci-local.just"
    printf 'red := ""\ngreen := ""\nreset := ""\nimport "ci-local.just"\n' >"$harness/justfile"
    cp "$REPO_ROOT/scripts/ci/constraints.py" "$harness/scripts/ci/constraints.py"
    # The recipe's self-test loop: no-op stubs, since this fixture tests the
    # recipe's scheduling, not those suites.
    local suite
    for suite in test_schema.py test_affected_scope.py test_resolved_plan.py test_ci_local.py test_constraints.py test_publish_gaps.py test_runner_loss.py; do
        : >"$harness/scripts/ci/$suite"
    done
    cat >"$harness/scripts/ci/affected_scope.py" <<EOF
import os, subprocess, sys
with open(os.environ["TEST_PLANNER_LOG"], "a", encoding="utf-8") as log:
    log.write("ci-local " + " ".join(sys.argv[1:]) + "\n")
if "--apply-to" not in sys.argv:
    raise SystemExit("ci-local selected scope although the hook fed it a plan")
raise SystemExit(subprocess.run([sys.executable, "$REPO_ROOT/scripts/ci/affected_scope.py", *sys.argv[1:]], check=False).returncode)
EOF
    # `ci-local.just` is a Bash script; the harness PATH would otherwise hand
    # it the stock macOS Bash 3.2.
    ln -s "$(command -v bash)" "$tmpdir/bash"
}

# A fake `just` whose `pre-push` is the real recipe. `_test <pkg>` stages a
# passing report, or — while "$tmpdir/gate-fail-<pkg>" exists — a report with
# one failing test and a non-zero exit, so `record-cells` measures a COMPLETE
# failure for that cell.
#
# Usage: make_ci_local_just <tmpdir> [plan_fixture]
make_ci_local_just() {
    local tmpdir="$1"
    local plan_fixture="${2:-$FIXTURES/plan-macos-two-packages.json}"
    cat >"$tmpdir/just" <<EOF
#!/usr/bin/env bash
echo "\$*" >>"$tmpdir/just.log"
case "\$1" in
    pre-push)
        shift
        exec "$REAL_JUST" --justfile "$tmpdir/harness/justfile" --working-directory "$tmpdir/harness" ci-local --l2 "\$@"
        ;;
    _lint)
        exit 0
        ;;
    _test)
        pkg="\$2"
        if [ -f "$tmpdir/gate-interrupt-\$pkg" ]; then
            if [ "\$(cat "$tmpdir/gate-interrupt-\$pkg")" = malformed ]; then
                printf '{"tier":' >>"\$BISCUIT_JUNIT_STAGE_DIR/manifest.jsonl"
                exit 1
            fi
            if [ "\$(cat "$tmpdir/gate-interrupt-\$pkg")" = 143 ]; then
                printf '{"tier":"L1","package":"%s","xml":"L1/%s.xml","exit_code":143,"report_present":false}\n' "\$pkg" "\$pkg" >>"\$BISCUIT_JUNIT_STAGE_DIR/manifest.jsonl"
                exit 1
            fi
            exit 130
        fi
        mkdir -p "\$BISCUIT_JUNIT_STAGE_DIR/L1"
        if [ -f "$tmpdir/gate-fail-\$pkg" ]; then
            printf '<?xml version="1.0"?><testsuites><testsuite name="%s" tests="2" failures="1" errors="0" skipped="0"><testcase classname="%s" name="one"/><testcase classname="%s" name="two"><failure message="assertion failed">boom</failure></testcase></testsuite></testsuites>' "\$pkg" "\$pkg" "\$pkg" >"\$BISCUIT_JUNIT_STAGE_DIR/L1/\$pkg.xml"
            printf '{"tier":"L1","package":"%s","xml":"L1/%s.xml","exit_code":1,"environment":"macos-latest","duration_s":3,"report_present":true}\n' "\$pkg" "\$pkg" >>"\$BISCUIT_JUNIT_STAGE_DIR/manifest.jsonl"
            exit 1
        fi
        printf '<?xml version="1.0"?><testsuites><testsuite name="%s" tests="2" failures="0" errors="0" skipped="0"><testcase classname="%s" name="one"/><testcase classname="%s" name="two"/></testsuite></testsuites>' "\$pkg" "\$pkg" "\$pkg" >"\$BISCUIT_JUNIT_STAGE_DIR/L1/\$pkg.xml"
        printf '{"tier":"L1","package":"%s","xml":"L1/%s.xml","exit_code":0,"environment":"macos-latest","duration_s":3,"report_present":true}\n' "\$pkg" "\$pkg" >>"\$BISCUIT_JUNIT_STAGE_DIR/manifest.jsonl"
        exit 0
        ;;
    *)
        echo "fake just: unexpected subcommand: \$*" >&2
        exit 99
        ;;
esac
EOF
    chmod +x "$tmpdir/just"
    printf '%s' "$plan_fixture" >"$tmpdir/planner-plan.path"
}

# The repository, tools, harness, and fake `just` for a plan-fed run: the
# feature commit adds beta's source beside alpha's, so both packages of the
# two-package plan exist at HEAD.
stage_plan_fed_fixture() {
    local tmpdir="$1"
    make_publishing_repo "$tmpdir" "pkg/beta/src/lib.rs" "beta lib"
    stage_fixture_tools "$tmpdir"
    stage_ci_local_harness "$tmpdir" || return 1
    make_ci_local_just "$tmpdir"
}

# Commit one more change on the fixture's feature branch.
add_follow_up_commit() {
    local repo="$1/repo" file="$2" content="$3"
    local commit=(git -c user.name=hook-test -c user.email=hook@example.com -c commit.gpgsign=false)
    mkdir -p "$(dirname "$repo/$file")"
    echo "$content" >>"$repo/$file"
    "${commit[@]}" -C "$repo" add -A
    "${commit[@]}" -C "$repo" commit -q -m "follow-up: $file"
}

# The macOS receipt attached to `head` in the fixture repository.
local_macos_receipt() {
    git -C "$1/repo" notes --ref refs/notes/ci-local/macos-latest show "$2" 2>/dev/null
}

# The receipt on `head` records exactly `expected_cells` (a jq array literal
# of "package/gate" strings, in order) against `expected_base`.
assert_receipt_records() {
    local label="$1" tmpdir="$2" head="$3" expected_base="$4" expected_cells="$5"
    local receipt
    if ! receipt="$(local_macos_receipt "$tmpdir" "$head")"; then
        echo "  no macos-latest receipt is attached to $head ($label)" >&2
        return 1
    fi
    if ! printf '%s' "$receipt" | jq -e --arg base "$expected_base" --argjson cells "$expected_cells" \
        '.base == $base and ([.cells[] | "\(.package)/\(.gate)"] == $cells)' >/dev/null; then
        echo "  the receipt on $head does not record cells $expected_cells against base $expected_base ($label):" >&2
        printf '%s' "$receipt" | jq -c '{base, cells: [.cells[] | "\(.package)/\(.gate)/\(.outcome)"]}' | sed 's/^/  /' >&2
        return 1
    fi
}

# The identity `schema.py` gives the plan a scope receipt carries — what a
# validation receipt bound to that scope must store as `scope_identity`.
scope_receipt_plan_identity() {
    python3 -c 'import json, sys; sys.path.insert(0, sys.argv[2]); import schema; print(schema.identity(schema.canonical(json.load(open(sys.argv[1]))["plan"])))' "$1" "$REPO_ROOT/scripts/ci"
}

# `ci-local` ran the planner exactly once, for the projection, never selecting.
assert_ci_local_never_selected() {
    local label="$1" tmpdir="$2"
    local calls
    calls="$(grep -c '^ci-local ' "$tmpdir/planner.log" 2>/dev/null || true)"
    if [ "$calls" = "1" ] && grep -q '^ci-local --apply-to ' "$tmpdir/planner.log"; then
        return 0
    fi
    echo "  expected ci-local's one planner call to be the projection ($label); planner.log:" >&2
    sed 's/^/  /' "$tmpdir/planner.log" >&2
    return 1
}

# W14: commit A is validated and published; commit B changes only a document.
# alpha declares its build closure, so A's alpha/L1 cell qualifies for B under
# gate-input equivalence and is not rerun; beta declares none, so it reruns.
# B's receipt then records beta only.
test_a_docs_only_follow_up_skips_the_cells_prior_evidence_covers() {
    local tmpdir="$1"
    stage_plan_fed_fixture "$tmpdir" || return 1
    local repo="$tmpdir/repo" main_sha head_a head_b
    main_sha="$(git -C "$repo" rev-parse origin/main)"
    head_a="$(git -C "$repo" rev-parse HEAD)"
    run_hook_in_repo "$tmpdir" strict refs/heads/feature "$ZERO_SHA" "BISCUIT_CI_EVIDENCE_DIR=$tmpdir/evidence"
    assert_exit "commit A" "$tmpdir" 0 || return 1
    assert_contains "A is fed the reviewed plan" "$tmpdir/out" "Running pre-push validation from the reviewed plan" || return 1
    assert_log_has "A ran alpha" "$tmpdir" "_test alpha" || return 1
    assert_log_has "A ran beta" "$tmpdir" "_test beta" || return 1
    assert_ci_local_never_selected "commit A" "$tmpdir" || return 1
    assert_receipt_records "commit A" "$tmpdir" "$head_a" "$main_sha" '["alpha/L1","beta/L1"]' || return 1

    add_follow_up_commit "$tmpdir" README.md "more documentation"
    head_b="$(git -C "$repo" rev-parse HEAD)"
    reset_run_outputs "$tmpdir"
    run_hook_in_repo "$tmpdir" strict refs/heads/feature "$ZERO_SHA" "BISCUIT_CI_EVIDENCE_DIR=$tmpdir/evidence"
    assert_exit "commit B" "$tmpdir" 0 || return 1
    assert_log_lacks "alpha not rerun" "$tmpdir" "_test alpha" || return 1
    assert_log_has "beta rerun (no declared closure)" "$tmpdir" "_test beta" || return 1
    assert_log_has "lint still runs" "$tmpdir" "_lint alpha" || return 1
    assert_contains "alpha named as skipped" "$tmpdir/out" "skip   alpha/L1" || return 1
    assert_ci_local_never_selected "commit B" "$tmpdir" || return 1
    assert_receipt_records "commit B" "$tmpdir" "$head_b" "$main_sha" '["beta/L1"]' || return 1
}

# The counterpart: B changes alpha's source, so its gate inputs differ from
# A's and alpha/L1 is rerun.
test_a_follow_up_changing_a_packages_inputs_reruns_its_cell() {
    local tmpdir="$1"
    stage_plan_fed_fixture "$tmpdir" || return 1
    local repo="$tmpdir/repo" main_sha head_b
    main_sha="$(git -C "$repo" rev-parse origin/main)"
    run_hook_in_repo "$tmpdir" strict refs/heads/feature "$ZERO_SHA" "BISCUIT_CI_EVIDENCE_DIR=$tmpdir/evidence"
    assert_exit "commit A" "$tmpdir" 0 || return 1

    add_follow_up_commit "$tmpdir" pkg/alpha/src/lib.rs "changed alpha"
    head_b="$(git -C "$repo" rev-parse HEAD)"
    reset_run_outputs "$tmpdir"
    run_hook_in_repo "$tmpdir" strict refs/heads/feature "$ZERO_SHA" "BISCUIT_CI_EVIDENCE_DIR=$tmpdir/evidence"
    assert_exit "commit B" "$tmpdir" 0 || return 1
    assert_log_has "alpha rerun" "$tmpdir" "_test alpha" || return 1
    assert_contains "inputs named as changed" "$tmpdir/out" "gate-inputs-changed: alpha/macos-latest/L1" || return 1
    assert_ci_local_never_selected "commit B" "$tmpdir" || return 1
    assert_receipt_records "commit B" "$tmpdir" "$head_b" "$main_sha" '["alpha/L1","beta/L1"]' || return 1
}

# D2's implementation assumption: A's note records a COMPLETE failure for
# alpha/L1; B (docs only) reruns it, and B's passing note supersedes A's.
test_a_cell_whose_prior_evidence_is_a_failure_is_rerun_and_superseded() {
    local tmpdir="$1"
    stage_plan_fed_fixture "$tmpdir" || return 1
    local repo="$tmpdir/repo" main_sha head_a head_b
    main_sha="$(git -C "$repo" rev-parse origin/main)"
    head_a="$(git -C "$repo" rev-parse HEAD)"
    : >"$tmpdir/gate-fail-alpha"
    run_hook_in_repo "$tmpdir" warn refs/heads/feature "$ZERO_SHA" "BISCUIT_CI_EVIDENCE_DIR=$tmpdir/evidence"
    assert_exit "commit A, warn" "$tmpdir" 0 || return 1
    assert_receipt_records "commit A" "$tmpdir" "$head_a" "$main_sha" '["alpha/L1","beta/L1"]' || return 1
    if ! local_macos_receipt "$tmpdir" "$head_a" | jq -e '.cells[] | select(.package == "alpha") | .outcome == "fail" and .completion == "complete"' >/dev/null; then
        echo "  A's receipt does not record a complete alpha/L1 failure" >&2
        return 1
    fi

    rm -f "$tmpdir/gate-fail-alpha"
    add_follow_up_commit "$tmpdir" README.md "more documentation"
    head_b="$(git -C "$repo" rev-parse HEAD)"
    reset_run_outputs "$tmpdir"
    run_hook_in_repo "$tmpdir" strict refs/heads/feature "$ZERO_SHA" "BISCUIT_CI_EVIDENCE_DIR=$tmpdir/evidence"
    assert_exit "commit B" "$tmpdir" 0 || return 1
    assert_log_has "alpha rerun" "$tmpdir" "_test alpha" || return 1
    assert_contains "rerun named with its reason" "$tmpdir/out" "rerun  alpha/L1" || return 1
    assert_receipt_records "commit B" "$tmpdir" "$head_b" "$main_sha" '["alpha/L1","beta/L1"]' || return 1
    # Newest wins: verifying against B's plan resolves alpha/L1 to B's pass,
    # not A's failure.
    git -C "$repo" notes --ref refs/notes/ci-local/scope show "$head_b" | jq '.plan' >"$tmpdir/plan-b.json"
    if ! (cd "$repo" && python3 scripts/ci/local_evidence.py verify --cells \
            --plan "$tmpdir/plan-b.json" --base "$main_sha" --head "$head_b") >"$tmpdir/accepted.json" 2>"$tmpdir/verify.err"; then
        sed 's/^/  /' "$tmpdir/verify.err" >&2
        return 1
    fi
    if ! jq -e --arg b "$head_b" '[.[] | select(.package == "alpha")] | length == 1 and (.[0] | .outcome == "pass" and .evidence.commit == $b)' "$tmpdir/accepted.json" >/dev/null; then
        echo "  B's pass did not supersede A's failure for alpha/L1:" >&2
        sed 's/^/  /' "$tmpdir/accepted.json" >&2
        return 1
    fi
}

# W1/W10: a stacked pull request's run compares against its target's tip, so
# that is the base the receipt records and the plan it is bound to — not a
# merge base with origin/main. The receipt's scope identity is the identity
# of the plan the scope receipt carries.
test_a_stacked_pull_request_records_its_target_tip_as_the_receipt_base() {
    local tmpdir="$1"
    make_stacked_repo "$tmpdir"
    stage_ci_local_harness "$tmpdir" || return 1
    make_ci_local_just "$tmpdir"
    stage_fake_gh "$tmpdir" "12 parent"
    run_stacked_push "$tmpdir" strict "BISCUIT_CI_EVIDENCE_DIR=$tmpdir/evidence"
    assert_exit "stacked pull request" "$tmpdir" 0 || return 1
    assert_contains "trigger named" "$tmpdir/out" "pull request #12 into parent" || return 1
    assert_ci_local_never_selected "stacked pull request" "$tmpdir" || return 1
    local repo="$tmpdir/repo" merge_base
    merge_base="$(git -C "$repo" merge-base origin/main "$CHILD_SHA")"
    if [ "$merge_base" = "$PARENT_SHA" ]; then
        echo "  fixture error: the merge base with main equals the parent tip, so the test cannot tell them apart" >&2
        return 1
    fi
    assert_receipt_records "stacked pull request" "$tmpdir" "$CHILD_SHA" "$PARENT_SHA" '["alpha/L1","beta/L1"]' || return 1
    local recorded expected
    recorded="$(local_macos_receipt "$tmpdir" "$CHILD_SHA" | jq -r '.scope_identity')"
    git -C "$repo" notes --ref refs/notes/ci-local/scope show "$CHILD_SHA" >"$tmpdir/scope-receipt.json"
    expected="$(scope_receipt_plan_identity "$tmpdir/scope-receipt.json")"
    if [ -z "$expected" ] || [ "$recorded" != "$expected" ]; then
        echo "  the validation receipt's scope_identity ($recorded) is not the scope receipt's plan identity ($expected)" >&2
        return 1
    fi
}

test_an_advanced_non_main_target_reuses_prior_cells_and_publishes_exact_receipts() {
    local tmpdir="$1"
    stage_plan_fed_fixture "$tmpdir" || return 1
    local repo="$tmpdir/repo" base head advanced
    base="$(git -C "$repo" rev-parse origin/main)"
    git -C "$repo" push -q origin "$base:refs/heads/parent"
    stage_fake_gh "$tmpdir" "12 parent"
    head="$(git -C "$repo" rev-parse HEAD)"
    run_hook_as_remote "$tmpdir" strict origin "$GITHUB_URL" \
        "refs/heads/feature $head refs/heads/feature $ZERO_SHA" "BISCUIT_CI_EVIDENCE_DIR=$tmpdir/evidence"
    assert_exit "initial validation" "$tmpdir" 0 || return 1
    assert_log_has "initial alpha" "$tmpdir" "_test alpha" || return 1

    git clone -q "$tmpdir/origin.git" "$tmpdir/second"
    git -C "$tmpdir/second" checkout -q parent
    echo "advanced target" >"$tmpdir/second/README.md"
    git -C "$tmpdir/second" add README.md
    git -C "$tmpdir/second" -c user.name=hook-test -c user.email=hook@example.com -c commit.gpgsign=false commit -q -m "advance parent"
    git -C "$tmpdir/second" push -q origin parent
    advanced="$(git -C "$tmpdir/second" rev-parse HEAD)"
    add_follow_up_commit "$tmpdir" docs.md "head documentation"
    head="$(git -C "$repo" rev-parse HEAD)"
    reset_run_outputs "$tmpdir"
    run_hook_as_remote "$tmpdir" strict origin "$GITHUB_URL" \
        "refs/heads/feature $head refs/heads/feature $ZERO_SHA" "BISCUIT_CI_EVIDENCE_DIR=$tmpdir/evidence"
    assert_exit "advanced comparison" "$tmpdir" 0 || return 1
    assert_log_lacks "alpha not rerun" "$tmpdir" "_test alpha" || return 1
    assert_log_has "beta ran" "$tmpdir" "_test beta" || return 1
    assert_log_has "lint ran" "$tmpdir" "_lint alpha" || return 1
    assert_ci_local_never_selected "advanced comparison" "$tmpdir" || return 1
    assert_receipt_records "advanced comparison" "$tmpdir" "$head" "$advanced" '["beta/L1"]' || return 1
    git -C "$tmpdir/origin.git" notes --ref refs/notes/ci-local/scope show "$head" >"$tmpdir/remote-scope.json"
    jq -e --arg base "$advanced" '.base == $base' "$tmpdir/remote-scope.json" >/dev/null || return 1
    git -C "$tmpdir/origin.git" notes --ref refs/notes/ci-local/macos-latest show "$head" >"$tmpdir/remote-validation.json"
    jq -e --arg base "$advanced" '.base == $base and .cells[0].outcome == "pass"' "$tmpdir/remote-validation.json" >/dev/null || return 1
    local identity
    identity="$(scope_receipt_plan_identity "$tmpdir/remote-scope.json")"
    jq -e --arg identity "$identity" '.scope_identity == $identity' "$tmpdir/remote-validation.json" >/dev/null || return 1
}

test_a_real_recipe_interruption_withholds_earlier_reports() {
    local tmpdir="$1"
    stage_plan_fed_fixture "$tmpdir" || return 1
    local interrupted_status
    for interrupted_status in malformed 130 143; do
        echo "$interrupted_status" >"$tmpdir/gate-interrupt-beta"
        reset_run_outputs "$tmpdir"
        run_hook_in_repo "$tmpdir" strict refs/heads/feature "$ZERO_SHA"
        if [ "$(cat "$tmpdir/exit")" = 0 ]; then
            echo "  interrupted real recipe passed" >&2
            return 1
        fi
        assert_log_has "earlier report staged" "$tmpdir" "_test alpha" || return 1
        assert_log_has "beta interrupted" "$tmpdir" "_test beta" || return 1
        assert_contains "marker survived recipe" "$tmpdir/err" "validation marked this run interrupted" || return 1
        if [ "$interrupted_status" = malformed ]; then
            assert_contains "unreadable manifest explained" "$tmpdir/err" "cannot establish run completion from the staged manifest" || return 1
        fi
        if git -C "$tmpdir/origin.git" show-ref --verify --quiet refs/notes/ci-local/macos-latest; then
            echo "  interrupted real recipe published its earlier passing report" >&2
            return 1
        fi
    done
}

# ---------- runner ----------

echo ""
echo "${C_DIM}Running pre-push hook tests against${C_RESET} $HOOK"
echo ""

run_test "real recipe interruption withholds earlier evidence" test_a_real_recipe_interruption_withholds_earlier_reports
run_test "advanced non-main target reuses and publishes exact receipts" test_an_advanced_non_main_target_reuses_prior_cells_and_publishes_exact_receipts
run_test "scope-only runs no gate but resolves scope"                 test_scope_only_runs_no_gate_but_still_resolves_scope
run_test "off is a deprecated alias of scope-only"                    test_off_mode_is_a_deprecated_alias_of_scope_only
run_test "invalid mode exits 1 with helpful message"                  test_invalid_mode_exits_one
run_test "warn + passing → exit 0"                                    test_warn_passing_tests_exits_zero
run_test "warn + failing → exit 0 and mentions --no-verify"           test_warn_failing_tests_exits_zero_with_no_verify_hint
run_test "strict + passing → exit 0"                                  test_strict_passing_tests_exits_zero
run_test "strict + failing → propagate exit and mention --no-verify"  test_strict_failing_tests_exits_nonzero_with_no_verify_hint
run_test "unset default is strict"                                    test_unset_default_is_strict
run_test "default path calls 'just pre-push' with no selection"       test_default_delegates_scope_to_pre_push
run_test "run_hook forwards no selection without an override"        test_run_hook_forwards_no_selection_without_an_override
run_test "RUSTY_BISCUIT_PRE_PUSH_AREAS override is forwarded verbatim" test_areas_override_is_passed_through
run_test "a failing warn run reaches the publication block"           test_a_failing_warn_run_reaches_the_publication_block
run_test "interrupted gate withholds staged evidence" test_a_gate_exiting_130_never_publishes_staged_evidence
run_test "interrupted hook withholds staged evidence" test_an_interrupted_hook_never_publishes_staged_evidence
run_test "publication requires a clean, exact, unoverridden tree"     test_publication_requires_a_clean_exact_tree_and_no_override
run_test "a persisted prohibition blocks the push"                    test_prohibition_blocks_the_push
run_test "a prohibition refusal states its reason"                    test_prohibition_is_explained
run_test "an expired prohibition does not block"                      test_expired_prohibition_does_not_block
run_test "a reused prohibited cell does not block"                    test_a_reused_prohibited_cell_does_not_block
run_test "an absent prohibited environment does not block"            test_an_absent_prohibited_environment_does_not_block
run_test "an executing prohibited cell blocks with its reason"        test_an_executing_prohibited_cell_blocks_with_its_reason
run_test "the constraint is decided in scope-only mode too"           test_the_constraint_is_decided_in_scope_only_mode_too
run_test "an unreadable plan blocks"                                  test_an_unreadable_plan_blocks
run_test "a failed plan resolution blocks and publishes no scope"     test_a_failed_plan_resolution_blocks_and_publishes_no_scope
run_test "the plan resolution is handed no constraint store"          test_the_plan_resolution_is_handed_no_constraint_store
run_test "a constraint for another branch does not block"             test_a_constraint_for_another_branch_does_not_block
run_test "the plan fixtures are valid resolved plans"                 test_the_plan_fixtures_are_valid_resolved_plans
run_test "a published receipt names retained, readable reports"       test_a_published_receipt_names_retained_readable_reports
run_test "reports that cannot be retained publish no receipt"         test_reports_that_cannot_be_retained_publish_no_receipt
run_test "a blocked strict failure publishes its note; CI reuses it"  test_a_blocked_strict_failure_publishes_its_note_and_ci_reuses_the_failure
run_test "a complete warn failure publishes its note"                 test_a_complete_warn_failure_publishes_its_note
run_test "scope-only publishes committed scope for a push to main"    test_scope_only_publishes_committed_scope_for_a_push_to_main
run_test "a feature push records the PR base, not its previous tip"   test_a_feature_branch_push_records_the_pull_request_base_not_its_previous_tip
run_test "a branch-creating push records the remote's main tip, says so" test_a_branch_creating_push_records_the_remote_main_tip_and_says_so
run_test "a dirty strict run publishes scope but no validation"       test_a_dirty_strict_run_publishes_scope_but_no_validation_receipt
run_test "a failed scope publication is named; exit code unchanged"   test_a_failed_scope_publication_is_named_and_leaves_the_exit_code_alone
run_test "a committed change masked by an unstaged revert is reviewed" test_a_committed_change_masked_by_an_unstaged_revert_is_still_reviewed
run_test "an unstaged policy edit never reaches the plan or receipt"  test_an_unstaged_policy_edit_never_reaches_the_committed_plan_or_receipt
run_test "a clean checkout is planned in place"                       test_a_clean_checkout_is_planned_in_place
run_test "a push that does not carry HEAD reviews the pushed revision" test_a_push_that_does_not_carry_head_reviews_the_pushed_revision
run_test "a deletion-only push triggers nothing and is not reviewed"  test_a_deletion_only_push_triggers_nothing_and_is_not_reviewed
run_test "a prohibition on the pushed branch blocks, whatever is checked out" test_a_prohibition_on_the_pushed_branch_blocks_whatever_is_checked_out
run_test "a prohibition on the checked-out branch does not bind another push" test_a_prohibition_on_the_checked_out_branch_does_not_bind_another_branch_push
run_test "every pushed branch is planned with its own path set"       test_every_pushed_branch_is_planned_with_its_own_path_set
run_test "ref updates are reviewed in the order Git supplies them"    test_ref_updates_are_reviewed_in_the_order_git_supplies_them
run_test "the repository identity is the remote being pushed to"      test_the_repository_identity_is_the_remote_being_pushed_to
run_test "a renamed refspec binds a record under either branch name"  test_a_renamed_refspec_binds_a_record_under_either_branch_name
run_test "deletions and tags among branch updates are skipped by name" test_deletions_and_tags_among_branch_updates_are_skipped_by_name
run_test "a stacked pull request is planned against its target's tip"  test_a_stacked_pull_request_is_planned_against_its_target_branch_tip
run_test "a pull request into main from a restoring child selects nothing" test_a_pull_request_into_main_from_a_restoring_child_selects_nothing
run_test "an advanced target records its exact remote comparison"      test_an_advanced_target_branch_is_read_from_the_remote_and_records_exact_scope
run_test "no open pull request on GitHub → a provisional plan"          test_a_github_branch_with_no_open_pull_request_gets_a_provisional_plan
run_test "a record in the default store binds a fresh session"         test_a_record_in_the_default_store_binds_a_fresh_session
run_test "two open pull requests from one head are each reviewed"     test_two_open_pull_requests_from_one_head_are_each_reviewed
run_test "a GitHub remote without gh blocks and names it"             test_a_github_remote_without_gh_blocks_and_names_it
run_test "a failing gh blocks and names the failure"                  test_a_failing_gh_blocks_and_names_the_failure
run_test "a non-GitHub remote never asks gh"                          test_a_non_github_remote_never_asks_gh
run_test "a target updated in the same push is reviewed at its incoming revision" test_a_target_updated_in_the_same_push_is_reviewed_at_its_incoming_revision
run_test "a simultaneous push with nothing prohibited passes in either order" test_a_simultaneous_push_with_no_prohibited_work_passes_in_either_order
run_test "a pull request into a branch this push deletes is refused"  test_a_pull_request_into_a_branch_this_push_deletes_is_refused
run_test "a docs-only follow-up skips the cells prior evidence covers"  test_a_docs_only_follow_up_skips_the_cells_prior_evidence_covers
run_test "a follow-up changing a package's inputs reruns its cell"     test_a_follow_up_changing_a_packages_inputs_reruns_its_cell
run_test "a cell whose prior evidence is a failure is rerun, superseded" test_a_cell_whose_prior_evidence_is_a_failure_is_rerun_and_superseded
run_test "a stacked pull request records its target tip as the base"  test_a_stacked_pull_request_records_its_target_tip_as_the_receipt_base

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
