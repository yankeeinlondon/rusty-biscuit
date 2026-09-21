# Local stand-in for the CI package gates.
#
# For every package CI would schedule for this branch's changes, run the same
# gates CI runs, with the same arguments:
#
#   just _lint <pkg>                      clippy, --all-targets, NO features
#   just _test <pkg> --no-fail-fast <test_args>   L1 with the declared CI features
#   just _test_l2 <pkg> <test_args>        declared L2 on a hostable backend
#   cargo check <check_args>               check-ONLY packages (see below)
#
# The `cargo check` branch is CURRENTLY UNREACHABLE and deliberately left that
# way. It fires only for a package whose sole selected gate is `check`, which
# was the old unchanged-reverse-dependency shape. Those are no longer selected
# at all, and every gating package now owns a `lint` cell, so no package can
# carry `check` alone. The live consequence: CI schedules a `check` cell for a
# package declaring `example` or `bench` targets, and on `ubuntu-latest` for a
# package with unchanged direct reverse dependencies (compiled inside that
# cell; Open Question 1, Option B), and this recipe executes neither, so an
# examples-only compile break or a broken unchanged consumer surfaces in CI
# rather than pre-push. On this host's own OS the package's passing L1 receipt
# satisfies the check cell, so such a break surfaces only on the other OSes.
# Widening the condition would add a compile pass to every push, which is a
# cost decision, not a drift fix.
#
# The lint gate deliberately builds without features: that is CI's invocation,
# and it is where incomplete `terminal-tests`/`daemon-tests`-style gating
# surfaces (an ungated test module cannot resolve its optional crate). Run
# `--lint-only` first — it is minutes, not tens of minutes, and it caught four
# of the seven CI rounds of 2026-08-27.
#
# Scope comes from `scripts/ci/affected_scope.py`, the same script CI uses, fed
# the files that differ from the base ref (committed, staged, unstaged, and
# untracked). Global inputs are scoped per GATE there, as in CI: `Cargo.toml`,
# the toolchain, `.cargo/`, and the CI workflows widen every gate; `clippy.toml`
# widens only lint; `.config/nextest.toml` only test; a just file widens a gate
# only when the recipe that changed is one CI's gate recipes can reach (so a
# `pre-push` or `plan.just` edit gates nothing). `--lint-only` keeps a genuine
# full scope tractable.
#
# Also replicated: preflight's CI-infra self-test suite (the contract schemas,
# scope calculation, the resolved plan, local concurrency/routing, and the
# execution-constraint store). These catch a stale scope fixture, a serial L2
# regression, or an unenforced constraint locally, before validation evidence is
# published.
#
# Not replicated: L3/browser tiers, WSL archives, companion suites, and other
# operating systems. `--l2` adds the source packages' hostable L2 tier.
#
# `--plan` is the developer's WORKING-TREE preview of the pre-trigger review
# (spec section 4). It resolves the canonical plan — every `{package,
# environment, gate}` cell with its execution, origin, and state — for the
# files that differ from the base ref in the checkout, prints it, and exits
# non-zero when a recorded execution constraint would still be violated. It
# runs no gate and builds nothing, so it is safe to run before every push.
# The constraint store is `<home>/.rusty-biscuit/ci-constraints/<repository>/`
# (resolved by `scripts/ci/constraints.py directory`) unless
# `BISCUIT_CI_CONSTRAINTS_DIR` or `--constraints` overrides it; `--plan-out`
# keeps the canonical JSON as the machine interface alongside the rendered table.
# The pre-push hook does NOT call `--plan`: it reviews each PUSHED REVISION's
# committed tree with the planner, manifests, and policy committed there,
# against the base of each run the push triggers (an open pull request's
# target tip on the remote — and its incoming revision too, when the same
# push updates the target — or the remote's main when none is open), where
# this preview diffs the checkout against `origin/main`. The two differ when
# the checkout is dirty (a committed change masked by an unstaged revert is
# absent here and present in the hook's review) and when the pull request
# targets another branch or its target has advanced.
#
# `--plan-in <path>` (or `BISCUIT_CI_PLAN_IN`, the form the hook uses) runs the
# gates FROM a resolved plan instead of computing one: the planner's selection
# never runs, the package list, gates, and test arguments are the legacy
# projection of that plan (`affected_scope.py --apply-to`, which reads nothing
# from the checkout), and for this host's environment an L1/L2 cell runs only
# when the plan says it EXECUTES — or when it was reused from prior evidence
# whose outcome is a FAILURE, since a local rerun is cheap and the fresh result
# supersedes the old one (spec ruling D2). Lint stages no report and is never
# reusable, so it runs as always. This is how a clean-checkout push runs only
# the cells prior passing evidence does not already cover: the hook feeds the
# reviewed, evidence-overlaid plan of the outgoing head, then records a receipt
# for the cells that actually ran against that same plan and base. A dirty
# checkout or an RUSTY_BISCUIT_PRE_PUSH_AREAS override keeps the working-tree
# replan below, which is wider feedback and never published as evidence.
#
# Gate execution uses a report-producing local Nextest profile and an unwrapped
# target directory reserved for local CI. Kache restores immutable hardlinks
# into the normal target directory; a later feature-set change can make rustc
# try to overwrite one and fail before any test runs. `BISCUIT_CI_TARGET_DIR`
# may relocate the reserved directory, but it must remain exclusive to
# unwrapped local-CI runs.
#
# That one reserved directory is also this recipe's answer to CI's producer:
# every tier it runs is a child of this process and shares that target tree, so
# the compile CI performs once in `ci-build produce` happens here once too,
# without serializing an archive. Nothing would consume one — there is no second
# machine and no second path. What the run still owes a reader is the planned
# BUILD KEY of the cells it is about to run, printed below, because that key is
# the only thing by which a local result and a CI result can be said to have
# compiled the same program. The separated case — another host, another
# checkout — is `just cross-check`, which uses the same `ci-build` producer,
# manifest, verifier, and archive-mode tier recipes CI uses.
#
# Usage:
#   just ci-local                       affected packages vs origin/main
#   just ci-local --lint-only           clippy gates only
#   just ci-local --test-only           L1 gates only
#   just ci-local --l2                  add hostable L2 tests
#   just ci-local --base <ref>          diff against another ref (or CI_LOCAL_BASE)
#   just ci-local --all                 every workspace package
#   just ci-local --dry-run             print the scope and gates, run nothing
#   just ci-local --plan                review every resolved cell, run nothing
#   just ci-local --plan-out <path>     also write the canonical plan JSON
#   just ci-local --plan-in <path>      run the gates a resolved plan leaves executing
#   just ci-local --constraints <dir>   read execution constraints from another store
#   just ci-local claudine-cli darkmatter   named packages or areas, no scope calc
ci-local *args="":
    #!/usr/bin/env bash
    set -euo pipefail

    base="${CI_LOCAL_BASE:-origin/main}"
    run_lint=1
    run_test=1
    run_check=1
    run_l2=0
    select_all=0
    dry_run=0
    show_plan=0
    scope_out="${BISCUIT_CI_SCOPE_OUT:-}"
    plan_out="${BISCUIT_CI_PLAN_OUT:-}"
    plan_in="${BISCUIT_CI_PLAN_IN:-}"
    constraints_dir="${BISCUIT_CI_CONSTRAINTS_DIR:-}"
    selectors=()
    set -- {{ args }}
    while (( $# > 0 )); do
        case "$1" in
            --lint-only) run_test=0; run_check=0; run_l2=0 ;;
            --test-only) run_lint=0; run_check=0; run_l2=0 ;;
            --l2) run_l2=1 ;;
            --scope-out) shift; scope_out="${1:?--scope-out needs a path}" ;;
            --scope-out=*) scope_out="${1#--scope-out=}" ;;
            --plan-out) shift; plan_out="${1:?--plan-out needs a path}" ;;
            --plan-out=*) plan_out="${1#--plan-out=}" ;;
            --plan-in) shift; plan_in="${1:?--plan-in needs a path}" ;;
            --plan-in=*) plan_in="${1#--plan-in=}" ;;
            --constraints) shift; constraints_dir="${1:?--constraints needs a path}" ;;
            --constraints=*) constraints_dir="${1#--constraints=}" ;;
            --all) select_all=1 ;;
            --dry-run) dry_run=1 ;;
            --plan) show_plan=1 ;;
            --base) shift; base="${1:?--base needs a ref}" ;;
            --base=*) base="${1#--base=}" ;;
            -h|--help) just --list | grep -A1 '^\s*ci-local'; exit 0 ;;
            -*) echo "ci-local: unknown flag $1" >&2; exit 2 ;;
            *) selectors+=("$1") ;;
        esac
        shift
    done
    if ! command -v python3 > /dev/null 2>&1; then
        echo "ci-local: python3 is required (scripts/ci/affected_scope.py); run 'just init'." >&2
        exit 2
    fi
    ci_repo_root="$(git rev-parse --show-toplevel 2>/dev/null || pwd -P)"
    export CARGO_TARGET_DIR="${BISCUIT_CI_TARGET_DIR:-${ci_repo_root}/target/ci-local}"
    export RUSTC_WRAPPER=""
    export NEXTEST_PROFILE="local-evidence"
    # Nextest resolves relative JUnit paths below the workspace target even
    # when Cargo builds into CARGO_TARGET_DIR. Tell the staging helper where
    # that report actually lives so passing cells cannot become partial.
    export BISCUIT_JUNIT_WORKSPACE_ROOT="${ci_repo_root}"
    export BISCUIT_JUNIT_TARGET_DIR="${ci_repo_root}/target"
    # The store is resolved here, at the trigger boundary, and handed to the
    # planner explicitly: the planner has no default of its own, so CI, which
    # never passes `--constraints`, cannot read it. The hook derives the
    # repository from the remote being pushed to; this preview has no push, so
    # it uses origin's URL — empty without one, which is the store root, where
    # every repository's records bind.
    constraints_dir="$(python3 scripts/ci/constraints.py directory \
        --repository "$(git remote get-url origin 2>/dev/null || true)" \
        --directory "${constraints_dir}")"

    scope_file="$(mktemp "${TMPDIR:-/tmp}/biscuit-ci-local-scope.XXXXXX")"
    plan_file="$(mktemp "${TMPDIR:-/tmp}/biscuit-ci-local-plan.XXXXXX")"
    accepted_file="$(mktemp "${TMPDIR:-/tmp}/biscuit-ci-local-accepted.XXXXXX")"
    rejected_file="$(mktemp "${TMPDIR:-/tmp}/biscuit-ci-local-rejected.XXXXXX")"
    trap 'rm -f "${scope_file}" "${plan_file}" "${accepted_file}" "${rejected_file}"' EXIT

    # One calculation serves both documents: `--plan-out` writes the canonical
    # resolved plan while stdout stays the legacy scope shape the gate loop and
    # `ci.yml` still read. They cannot disagree about what CI will run.
    scope_args=(--plan-out "${plan_file}" --constraints "${constraints_dir}")
    # `--plan` needs no legacy projection: it runs no gate, so the canonical
    # plan is both what it renders and what `--plan-out` writes.
    if (( show_plan == 1 )); then
        scope_args+=(--resolved-plan)
        plan_view="${scope_file}"
    else
        plan_view="${plan_file}"
    fi

    merge_base=""
    if [[ -n "${plan_in}" ]]; then
        # A fed plan IS the selection. Its cells already carry the evidence the
        # caller verified, so the planner never runs here: the legacy shape the
        # gate loop reads is projected from the plan by `--apply-to` with no
        # accepted cells, which reads nothing from the checkout.
        if (( select_all == 1 )) || (( ${#selectors[@]} > 0 )); then
            echo "ci-local: --plan-in runs the given plan; it cannot be combined with --all or package selectors." >&2
            exit 2
        fi
        if [[ ! -s "${plan_in}" ]]; then
            echo "ci-local: the plan at '${plan_in}' (--plan-in / BISCUIT_CI_PLAN_IN) is missing or empty." >&2
            exit 2
        fi
        cp "${plan_in}" "${plan_file}"
        python3 scripts/ci/affected_scope.py --apply-to "${plan_file}" > "${scope_file}"
        plan_view="${plan_file}"
        echo
        echo "Plan supplied by the caller: ${plan_in}"
    elif (( select_all == 1 )) || (( ${#selectors[@]} > 0 )); then
        python3 scripts/ci/affected_scope.py "${scope_args[@]}" --all > "${scope_file}"
    else
        if ! merge_base="$(git merge-base "${base}" HEAD 2>/dev/null)"; then
            echo "ci-local: cannot resolve base ref '${base}' (set --base or CI_LOCAL_BASE)." >&2
            exit 2
        fi
        # One diff, two lists. `--name-status` is what separates a deletion
        # from a path that is merely absent, and `diff_scope.py` renders the
        # stream as `--deleted <path>... -- <changed path>...` so the change
        # inventory carries that identity. Untracked files are changed paths
        # no diff reports and are never deletions, so they ride along as the
        # parser's positional arguments — NUL-delimited the whole way, which
        # the old word-split list was not.
        untracked=()
        while IFS= read -r -d '' path; do
            untracked+=("${path}")
        done < <(git ls-files --others --exclude-standard -z)
        diff_arguments="$(mktemp)"
        git diff --name-status -z "${merge_base}" \
            | python3 scripts/ci/diff_scope.py ${untracked[@]+"${untracked[@]}"} > "${diff_arguments}"
        path_args=()
        while IFS= read -r -d '' argument; do
            path_args+=("${argument}")
        done < "${diff_arguments}"
        rm -f "${diff_arguments}"
        # The tail is `--deleted <path>` pairs, then `--`, then the changed
        # paths; stepping the pairs finds the separator without assuming any
        # path's spelling.
        separator=0
        while (( separator < ${#path_args[@]} )) && [[ "${path_args[separator]}" == "--deleted" ]]; do
            separator=$(( separator + 2 ))
        done
        changed_count=$(( ${#path_args[@]} - separator - 1 ))
        if (( changed_count == 0 )); then
            echo "ci-local: no changes relative to ${base} (${merge_base:0:9}); nothing to gate."
            exit 0
        fi
        # The revision pair is part of the plan: a receipt records the head it
        # tested, and a reviewer reconciles the plan against the outgoing work.
        scope_args+=(--base "${merge_base}" --head "$(git rev-parse HEAD)")
        python3 scripts/ci/affected_scope.py "${scope_args[@]}" "${path_args[@]}" > "${scope_file}"
        echo
        echo "Changed vs ${base} (${merge_base:0:9}): ${changed_count} file(s)"
    fi

    # `--plan` resolves published evidence first, so a reviewer sees the cells
    # that are already satisfied rather than the cells that would run without
    # them. The accepted set is overlaid on the plan just resolved — the same
    # operation CI performs on a carried receipt — rather than re-selecting.
    # An ordinary working-tree run does not consume evidence: its tree is not
    # necessarily any committed head's, so nothing can be verified against it.
    # The run that does consume evidence is the one fed a plan (`--plan-in`),
    # whose caller — the hook — verified and overlaid the evidence already, so
    # `merge_base` is empty and this block is skipped for it. `plan_file` is
    # read and rewritten by the one Python process, and stdout goes to the
    # separate `scope_file`.
    if (( show_plan == 1 )) && [[ -n "${merge_base}" ]] && [[ -f scripts/ci/local_evidence.py ]]; then
        head_sha="$(git rev-parse HEAD)"
        if python3 scripts/ci/local_evidence.py verify --cells \
            --plan "${plan_view}" --base "${merge_base}" --head "${head_sha}" \
            --rejections "${rejected_file}" > "${accepted_file}" 2>/dev/null; then
            python3 scripts/ci/affected_scope.py --apply-to "${plan_file}" \
                --accepted-cells "${accepted_file}" \
                --evidence-rejections "${rejected_file}" \
                --resolved-plan --plan-out "${plan_file}" > "${scope_file}"
        fi
    fi

    # Resolve explicit selectors (package names or area directories) against
    # the workspace the same way `just test <selector>` does.
    if (( ${#selectors[@]} > 0 )); then
        workspace_root="$(git rev-parse --show-toplevel)"
        selected=""
        while IFS=$'\t' read -r package manifest_path; do
            relative_manifest="${manifest_path#"${workspace_root}/"}"
            package_area="${relative_manifest%/Cargo.toml}"
            for selector in "${selectors[@]}"; do
                selector="${selector%,}"
                if [[ "${package}" == "${selector}" || "${package_area}" == "${selector}" || "${package_area}" == "${selector}/"* ]]; then
                    selected+="${package}"$'\n'
                    break
                fi
            done
        done < <(cargo metadata --no-deps --format-version 1 \
            | jq -r '.workspace_members as $m | .packages[] | select(.id as $id | $m | index($id)) | [.name, .manifest_path] | @tsv')
        if [[ -z "${selected}" ]]; then
            echo "ci-local: no workspace packages matched: ${selectors[*]}" >&2
            exit 1
        fi
        packages="$(sort -u <<< "${selected}" | sed '/^$/d')"
    else
        packages="$(jq -r '.packages[]' "${scope_file}")"
    fi

    count="$(sed '/^$/d' <<< "${packages}" | wc -l | tr -d ' ')"
    full_scope="$(jq -r '.full_scope' "${scope_file}")"
    change_class="$(jq -r '.change_class' "${scope_file}")"
    full_gates="$(jq -rc '.full_scope_gates | join(",")' "${scope_file}")"
    if [[ -n "${scope_out}" ]]; then
        cp "${scope_file}" "${scope_out}"
    fi
    if [[ -n "${plan_out}" ]]; then
        cp "${plan_view}" "${plan_out}"
    fi

    if (( show_plan == 1 )); then
        # The rich surface is `ci-plan`, a TerminalRenderable Table/Prose
        # renderer. It is a `local-tools` bin rather than something `--plan`
        # compiles on demand: a pre-trigger review must not start a build.
        # Without the prebuilt binary the same cells render as plain lines.
        echo "Constraint store: ${constraints_dir}"
        plan_bin="${BISCUIT_CI_PLAN_BIN:-target/release/ci-plan}"
        if [[ -x "${plan_bin}" ]] && "${plan_bin}" "${plan_view}"; then
            :
        else
            # The same change inventory the renderer and `ci-reporting` show,
            # read from the same plan field so the three cannot disagree about
            # what changed (spec section 7).
            echo
            jq -r '.change_inventory as $i
                | if ($i.diff_available // false) | not then
                    "Change inventory: no diff was consulted — \($i.reason // "the plan recorded no reason")"
                  else
                    "Change inventory: \($i.counts.total // 0) changed path(s)",
                    ($i.paths // {} | to_entries[] | select(.value | length > 0)
                      | "  \(.key) (\(.value | length)) — \(.value | join(", "))")
                  end' "${plan_view}"
            # Parity with `ci-plan`'s `ArchiveGuard::summary`: a reviewer must
            # be able to tell a changed-file scan from a full-tree one on both
            # surfaces, because only the full-tree one is repository-wide proof.
            echo
            jq -r '.archive_guard as $g
                | if ($g.selected // false) | not then
                    "Archive-path guard: not selected — \($g.reason // "no reason recorded")"
                  elif $g.mode == "changed" then
                    "Archive-path guard: changed-file scan of \(($g.paths // []) | length) file(s) — \($g.reason // "")"
                  else
                    "Archive-path guard: full-tree scan of the eligible corpus — \($g.reason // "")"
                  end' "${plan_view}"
            echo
            echo "Resolved plan (cell / execution / origin / state)"
            echo "------------------------------------------------"
            jq -r '.cells[] | "  \(.package)/\(.environment)/\(.gate)  \(.execution)  \(.origin)  \(.state)  — \(.selection_reason)"' "${plan_view}"
            if [[ "$(jq -r '.cells | length' "${plan_view}")" == "0" ]]; then
                echo "  No package test is required: the resolved plan schedules no package cell."
            fi
            if [[ "$(jq -r '(.evidence_rejections // []) | length' "${plan_view}")" != "0" ]]; then
                echo
                echo "Rejected evidence"
                jq -r '(.evidence_rejections // [])[] | "  ✗ \(.)"' "${plan_view}"
            fi
        fi
        prohibited="$(jq -r '.prohibited_cells | length' "${plan_view}")"
        if [[ "${prohibited}" != "0" ]]; then
            echo
            echo -e "{{ red }}${prohibited} cell(s) are prohibited by a recorded execution constraint and have no qualifying evidence:{{ reset }}"
            jq -r '.prohibited_cells[] | "  ⛔ \(.)"' "${plan_view}"
            jq -r '[.cells[] | select(.state == "prohibited") | .prohibition] | unique[] | "     \(.reason) (set by \(.owner), expires \(.expiry))"' "${plan_view}"
            echo
            echo "Triggering CI now would violate that constraint. Reuse qualifying evidence for those cells, or withdraw the constraint." >&2
            exit 1
        fi
        exit 0
    fi

    echo "CI scope: ${count} package(s) (class=${change_class}, full_scope=${full_scope}${full_gates:+ for ${full_gates}})"
    # An affirmative scheduling decision, never a warning: the change reached no
    # gating package, so the run names what it did change and says the gates it
    # is not running are the ones nothing required (spec section 7).
    if (( count == 0 )) && [[ -s "${plan_view}" ]]; then
        jq -r '(.change_inventory.paths // {}) | to_entries[] | select(.value | length > 0)
            | "Changed  : \(.key) (\(.value | length)) — \(.value | join(", "))"' "${plan_view}"
        echo "No package test is required: the resolved plan schedules no package cell."
    fi
    echo "Gates   : $([[ ${run_lint} == 1 ]] && printf 'lint ')$([[ ${run_check} == 1 ]] && printf 'check ')$([[ ${run_test} == 1 ]] && printf 'L1 ')$([[ ${run_l2} == 1 ]] && printf 'L2')"
    echo "------------------------------------------------"
    if (( dry_run == 1 )); then
        while IFS= read -r package; do
            [[ -n "${package}" ]] || continue
            test_args="$(jq -r --arg p "${package}" '.matrix[] | select(.package == $p) | .test_args // ""' "${scope_file}")"
            echo "  ${package}${test_args:+  (test: ${test_args})}"
        done <<< "${packages}"
        exit 0
    fi

    passed=()
    failed=()
    SECONDS=0

    # Per-gate reports for the validation receipt (spec section 3). The canonical
    # tier recipes already stage a JUnit report and a manifest record per
    # invocation; pointing them at a directory this run owns is what makes the
    # manifest describe THIS run. The default staging path accumulates across
    # runs locally — publishing from it would credit a passing cell to a tree
    # that never ran it.
    reports_dir="${BISCUIT_CI_REPORTS_OUT:-}"
    if [[ -z "${reports_dir}" ]]; then
        reports_dir="$(mktemp -d "${TMPDIR:-/tmp}/biscuit-ci-local-reports.XXXXXX")"
        trap 'rm -f "${scope_file}" "${plan_file}" "${accepted_file}" "${rejected_file}"; rm -rf "${reports_dir}"' EXIT
    fi
    mkdir -p "${reports_dir}"
    rm -f \
        "${reports_dir}/manifest.jsonl" \
        "${reports_dir}/gate-backends.jsonl" \
        "${reports_dir}/backend-executions.jsonl" \
        "${reports_dir}/isolation-burn-down.jsonl" \
        "${reports_dir}/spawn-site-burn-down.jsonl"
    export BISCUIT_JUNIT_STAGE_DIR="${reports_dir}"
    rm -f "${reports_dir}/interrupted"
    # Just may normalize a recipe's exit status; the marker preserves the
    # incomplete-run decision across the outer recipe and hook processes.
    abort_if_interrupted() {
        local status="$1"
        # Tier recipes may fold TERM's status into 1 after recording the raw
        # child status. Preserve that interruption decision from the manifest.
        if (( status < 128 )) && [[ -s "${reports_dir}/manifest.jsonl" ]]; then
            if ! status="$(jq -r -s '[.[] | .exit_code // 0 | select(. >= 128)] | max // 0' "${reports_dir}/manifest.jsonl")"; then
                printf '%s\n' "unreadable manifest" > "${reports_dir}/interrupted"
                echo "ci-local: cannot establish run completion from the staged manifest; validation publication is withheld." >&2
                exit 1
            fi
        fi
        if (( status >= 128 )); then
            printf '%s\n' "${status}" > "${reports_dir}/interrupted"
            echo "ci-local: interrupted; later gates and validation publication are skipped." >&2
            exit "${status}"
        fi
    }
    trap 'abort_if_interrupted 130' INT
    trap 'abort_if_interrupted 143' TERM
    trap 'abort_if_interrupted 129' HUP


    # The host's environment names which of a plan's cells are this run's; a
    # fed plan needs it for every tier, `--l2` for the backend proof, and the
    # build-key report below reads it whenever the host can answer. Detection is
    # best-effort for that report and required for the other two, so a host
    # without sniff keeps working and only loses the report.
    local_environment=""
    if command -v sniff > /dev/null 2>&1 && command -v jq > /dev/null 2>&1 \
        && os_json="$(sniff os --json 2>/dev/null)"; then
        case "$(jq -r '.os_type // ""' <<<"${os_json}" 2>/dev/null)" in
            MacOS) local_environment="macos-latest" ;;
            Windows) local_environment="windows-latest" ;;
            Linux)
                if [[ "$(jq -r '.kernel' <<<"${os_json}" | tr '[:upper:]' '[:lower:]')" == *microsoft* ]]; then
                    local_environment="wsl2-ubuntu"
                else
                    local_environment="ubuntu-latest"
                fi
                ;;
        esac
    fi
    if (( run_l2 == 1 )) || [[ -n "${plan_in}" ]]; then
        if ! command -v sniff > /dev/null 2>&1 || ! command -v jq > /dev/null 2>&1; then
            echo "ci-local: --l2 and --plan-in require sniff and jq; run 'just init'." >&2
            exit 2
        fi
        if [[ -z "${local_environment}" ]]; then
            echo "ci-local: unsupported host reported by sniff." >&2
            exit 2
        fi
    fi

    # Every tier below runs in this process tree against one target directory,
    # so the cells sharing a planned build key are exactly the cells sharing
    # that tree. Nothing is archived — there is no second machine to hand one
    # to — but the key is reported, so a local result and a CI result can be
    # compared by what actually decides whether they compiled the same program.
    # Grouped by key rather than by package, so a package that somehow resolved
    # two keys for this environment shows as the two lines it is.
    if [[ -n "${local_environment}" ]] \
        && [[ -s "${plan_file}" ]] \
        && [[ "$(jq -r '(.builds // []) | length' "${plan_file}")" != "0" ]]; then
        echo
        echo "Build keys for ${local_environment} (one local target tree, no archive serialized)"
        echo "  target: ${CARGO_TARGET_DIR}"
        jq -r --arg e "${local_environment}" '
            [.cells[] | select(.environment == $e and (.build // "") != "")]
            | group_by([.package, .build])[]
            | "  \(.[0].package)  \(.[0].build)  \([.[] | .gate] | unique | join(", "))"
        ' "${plan_file}"
        echo "------------------------------------------------"
    fi

    # Validate scheduling before treating package results as reusable evidence.
    selftest_log="$(mktemp "${TMPDIR:-/tmp}/biscuit-ci-local-selftest.XXXXXX")"
    # `test_resolved_plan.py` carries the sniff area-drift contract (AC15),
    # which runs wherever sniff is installed: a developer host, and `ci.yml`'s
    # `area-drift` job, which builds it. `test_runner_loss.py` derives every
    # producer's composite job label from the shipped workflows, so a rename
    # that would silently kill runner-loss attribution fails here rather than
    # on a dead runner.
    # `test_build_key.py` defends the single hashing boundary: a planner that
    # fell back to a second digest would still emit a plan, and every
    # downstream comparison against a producer's manifest would stop meaning
    # anything. It is ~0.1s wherever `scripts/target` already holds `ci-build`.
    for suite in test_schema.py test_affected_scope.py test_resolved_plan.py test_ci_local.py test_constraints.py test_publish_gaps.py test_runner_loss.py test_build_key.py; do
        if python3 "scripts/ci/${suite}" > "${selftest_log}" 2>&1; then
            passed+=("ci-infra ${suite}")
        else
            cat "${selftest_log}"
            failed+=("ci-infra ${suite}")
        fi
    done
    rm -f "${selftest_log}"

    # With a fed plan, the L1/L2 cells this host runs: the ones the plan left
    # executing in this environment, plus the ones reused from a FAILURE, which
    # are rerun (D2). A cell reused from a pass, omitted, or resolved elsewhere
    # is skipped and named. Without a plan every gate of every package runs.
    running_cells=""
    if [[ -n "${plan_in}" ]]; then
        running_cells="$(jq -r --arg e "${local_environment}" '.cells[] | select(.environment == $e and (.gate == "L1" or .gate == "L2")) | select(.execution == "execute" or (.execution == "reuse" and .evidence.outcome == "fail")) | "\(.package)/\(.gate)"' "${plan_file}")"
        echo "Cells for ${local_environment} from the supplied plan:"
        jq -r --arg e "${local_environment}" '.cells[] | select(.environment == $e and (.gate == "L1" or .gate == "L2")) | if .execution == "execute" then "  run    \(.package)/\(.gate)" elif .execution == "reuse" and .evidence.outcome == "fail" then "  rerun  \(.package)/\(.gate)  (prior evidence is a failure; a fresh result supersedes it)" elif .execution == "reuse" then "  skip   \(.package)/\(.gate)  (reused: \(.origin) \(.evidence.outcome // "") on \((.evidence.evidence.commit // .evidence.commit // "this tree")[0:9]))" else "  skip   \(.package)/\(.gate)  (\(.state))" end' "${plan_file}"
        echo "------------------------------------------------"
    fi
    cell_runs_here() {
        [[ -z "${plan_in}" ]] && return 0
        grep -qxF -- "$1/$2" <<< "${running_cells}"
    }

    while IFS= read -r package; do
        [[ -n "${package}" ]] || continue
        test_args="$(jq -r --arg p "${package}" '.matrix[] | select(.package == $p) | .test_args // ""' "${scope_file}")"
        # A package reached only through the check gate. Unreachable today (see
        # the recipe header): every gating package owns a lint cell, so `check`
        # is never a package's sole gate.
        gates="$(jq -r --arg p "${package}" '.matrix[] | select(.package == $p) | .gates // ["lint","check","test"] | join(" ")' "${scope_file}")"
        if (( run_check == 1 )) && [[ " ${gates} " == " check " ]]; then
            check_args="$(jq -r --arg p "${package}" '.matrix[] | select(.package == $p) | .check_args' "${scope_file}")"
            # shellcheck disable=SC2086
            if cargo check ${check_args}; then
                passed+=("check ${package} (check-only selection)")
            else
                abort_if_interrupted "$?"
                failed+=("check ${package} (check-only selection)")
            fi
        fi
        # A companions-only lint cell exists for a companion suite the change
        # selected, not for the package, so its producer skips clippy — see
        # `_package-ci.yml`'s lint job. Linting here anyway would run work CI
        # will not run and report a gate that does not exist.
        companions_only="$(jq -r --arg p "${package}" '.matrix[] | select(.package == $p) | .lint_companions_only // false' "${scope_file}")"
        if (( run_lint == 1 )) && [[ " ${gates} " == *" lint "* ]] && [[ "${companions_only}" != "true" ]]; then
            if just _lint "${package}"; then
                passed+=("lint ${package}")
            else
                abort_if_interrupted "$?"
                failed+=("lint ${package}")
            fi
        fi
        # The archive-path guard, whenever the plan attached it to this
        # package's lint cell. `companion_suites.py` is the same runner CI's
        # lint job uses, so the recipe it runs is the registry's one canonical
        # string rather than a second spelling that could drift from it.
        #
        # `BISCUIT_ARCHIVE_GUARD_PLAN` names the plan THIS run just resolved:
        # the scan covers the files the planner selected, and the recipe
        # computes no diff of its own (the spec's "one scheduler").
        #
        # No receipt machinery is needed to keep this out of CI's evidence: a
        # lint cell is already `reusable=False` in the plan, so nothing here can
        # be reused for the ubuntu-latest execution — which is what the message
        # below states outright on a non-Linux host.
        guard_planned=false
        if [[ -s "${plan_file}" ]]; then
            guard_planned="$(jq -r --arg p "${package}" '[.cells[]? | select(.package == $p and .gate == "lint") | (.companions // [])[] | select(.name == "archive-path-guard")] | length > 0' "${plan_file}")"
        fi
        if (( run_lint == 1 )) && [[ "${guard_planned}" == "true" ]]; then
            guard_note=""
            if [[ "${local_environment}" != "ubuntu-latest" ]]; then
                guard_note=" — the planned ubuntu-latest execution remains OUTSTANDING"
                echo "archive-path guard: this host is ${local_environment:-unidentified}, not ubuntu-latest."
                echo "  A pass here is a local check only; the planned ubuntu-latest execution of"
                echo "  ${package}/ubuntu-latest/lint remains OUTSTANDING and no local result stands in for it."
            fi
            if BISCUIT_ARCHIVE_GUARD_PLAN="${plan_file}" python3 scripts/ci/companion_suites.py \
                --suites '["archive-path-guard"]' \
                --environment ubuntu-latest \
                --gate lint \
                --out "${reports_dir}/archive-path-guard.json"; then
                passed+=("archive-path-guard ${package}${guard_note}")
            else
                abort_if_interrupted "$?"
                failed+=("archive-path-guard ${package}")
            fi
        fi
        if (( run_test == 1 )) && [[ " ${gates} " == *" test "* ]] && cell_runs_here "${package}" L1; then
            # shellcheck disable=SC2086
            if just _test "${package}" --no-fail-fast ${test_args}; then
                passed+=("test ${package}")
            else
                abort_if_interrupted "$?"
                failed+=("test ${package}")
            fi
        fi
        if (( run_l2 == 1 )) && [[ " ${gates} " == *" test "* ]] && cell_runs_here "${package}" L2; then
            has_l2="$(jq -r --arg p "${package}" '.matrix[] | select(.package == $p) | .tiers | index("L2") != null' "${scope_file}")"
            l2_here="$(jq -r --arg p "${package}" --arg e "${local_environment}" '.matrix[] | select(.package == $p) | .l2_environments | index($e) != null' "${scope_file}")"
            if [[ "${has_l2}" == "true" ]]; then
                backends="$(jq -r --arg p "${package}" '.matrix[] | select(.package == $p) | .l2_backends | join(" ")' "${scope_file}")"
                required=()
                if [[ " ${backends} " == *" tmux "* ]] && command -v tmux > /dev/null 2>&1; then
                    required+=(tmux)
                fi
                if [[ " ${backends} " == *" wezterm "* ]] && command -v wezterm > /dev/null 2>&1 && [[ -n "${WEZTERM_UNIX_SOCKET:-}" ]]; then
                    required+=(wezterm)
                fi
                if [[ " ${backends} " == *" kitty "* ]] && command -v kitty > /dev/null 2>&1 && [[ -n "${KITTY_LISTEN_ON:-}" ]]; then
                    required+=(kitty)
                fi
                l2_threads="${BISCUIT_L2_THREADS:-1}"
                parallel_l2="$(jq -r --arg p "${package}" '.matrix[] | select(.package == $p) | (.runner_tools // []) | index("l2-parallel-self-spawn") != null' "${scope_file}")"
                if [[ "${parallel_l2}" == "true" ]]; then
                    l2_threads="${BISCUIT_L2_THREADS:-${NEXTEST_TEST_THREADS:-$(just _test_threads)}}"
                fi
                required_csv="$(IFS=,; echo "${required[*]}")"
                if (( ${#required[@]} == 0 )); then
                    if [[ "${l2_here}" == "true" ]]; then
                        failed+=("L2 ${package} (no non-focusing backend available)")
                    fi
                else
                    # Which backends THIS invocation required. `test-toolkit`
                    # records which of them actually drove a test, but its
                    # records name a test, not a package; the receipt needs
                    # both halves to say a cell was backend-proven.
                    jq -c -n --arg p "${package}" --args '{package: $p, gate: "L2", backends: $ARGS.positional}' \
                        "${required[@]}" >> "${reports_dir}/gate-backends.jsonl"
                    if BISCUIT_L2_THREADS="${l2_threads}" BISCUIT_TEST_REQUIRED_BACKENDS="${required_csv}" just _test_l2 "${package}" ${test_args}; then
                        passed+=("L2 ${package} (${required_csv})")
                    else
                        abort_if_interrupted "$?"
                        failed+=("L2 ${package} (${required_csv})")
                    fi
                fi
            fi
        fi
    done <<< "${packages}"

    echo
    echo "ci-local summary ($((SECONDS / 60))m $((SECONDS % 60))s)"
    echo "------------------------------------------------"
    for gate in "${passed[@]:-}"; do [[ -n "${gate}" ]] && echo -e "  ✅ ${gate}"; done
    for gate in "${failed[@]:-}"; do [[ -n "${gate}" ]] && echo -e "  ❌ {{ red }}${gate}{{ reset }}"; done
    if (( ${#failed[@]} > 0 )); then
        echo
        echo -e "{{ red }}${#failed[@]} gate(s) would fail in CI.{{ reset }}"
        exit 1
    fi
    echo
    echo -e "{{ green }}All ${#passed[@]} gate(s) passed for this host and scope.{{ reset }}"
