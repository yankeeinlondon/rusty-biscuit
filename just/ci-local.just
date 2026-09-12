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
# package declaring `example` or `bench` targets and this recipe does not, so
# an examples-only compile break surfaces in CI rather than pre-push. Widening
# the condition would add a compile pass to every push, which is a cost
# decision, not a drift fix.
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
# `--plan` is the pre-trigger review surface (spec section 4). It resolves the
# canonical plan — every `{package, environment, gate}` cell with its execution,
# origin, and state — prints it, and exits non-zero when a recorded execution
# constraint would still be violated. It runs no gate and builds nothing, so it
# is safe to run before every push. `BISCUIT_CI_CONSTRAINTS_DIR` names the
# constraint store; `--plan-out` keeps the canonical JSON as the machine
# interface alongside the rendered table.
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

    scope_file="$(mktemp "${TMPDIR:-/tmp}/biscuit-ci-local-scope.XXXXXX")"
    plan_file="$(mktemp "${TMPDIR:-/tmp}/biscuit-ci-local-plan.XXXXXX")"
    accepted_file="$(mktemp "${TMPDIR:-/tmp}/biscuit-ci-local-accepted.XXXXXX")"
    rejected_file="$(mktemp "${TMPDIR:-/tmp}/biscuit-ci-local-rejected.XXXXXX")"
    trap 'rm -f "${scope_file}" "${plan_file}" "${accepted_file}" "${rejected_file}"' EXIT

    # One calculation serves both documents: `--plan-out` writes the canonical
    # resolved plan while stdout stays the legacy scope shape the gate loop and
    # `ci.yml` still read. They cannot disagree about what CI will run.
    scope_args=(--plan-out "${plan_file}")
    if [[ -n "${constraints_dir}" ]]; then
        scope_args+=(--constraints "${constraints_dir}")
    fi
    # `--plan` needs no legacy projection: it runs no gate, so the canonical
    # plan is both what it renders and what `--plan-out` writes.
    if (( show_plan == 1 )); then
        scope_args+=(--resolved-plan)
        plan_view="${scope_file}"
    else
        plan_view="${plan_file}"
    fi

    merge_base=""
    if (( select_all == 1 )) || (( ${#selectors[@]} > 0 )); then
        python3 scripts/ci/affected_scope.py "${scope_args[@]}" --all > "${scope_file}"
    else
        if ! merge_base="$(git merge-base "${base}" HEAD 2>/dev/null)"; then
            echo "ci-local: cannot resolve base ref '${base}' (set --base or CI_LOCAL_BASE)." >&2
            exit 2
        fi
        changed="$( { git diff --name-only "${merge_base}"; git ls-files --others --exclude-standard; } | sort -u )"
        if [[ -z "${changed}" ]]; then
            echo "ci-local: no changes relative to ${base} (${merge_base:0:9}); nothing to gate."
            exit 0
        fi
        # The revision pair is part of the plan: a receipt records the head it
        # tested, and a reviewer reconciles the plan against the outgoing work.
        scope_args+=(--base "${merge_base}" --head "$(git rev-parse HEAD)")
        # shellcheck disable=SC2086
        python3 scripts/ci/affected_scope.py "${scope_args[@]}" ${changed} > "${scope_file}"
        echo
        echo "Changed vs ${base} (${merge_base:0:9}): $(wc -l <<< "${changed}" | tr -d ' ') file(s)"
    fi

    # `--plan` resolves published evidence first, so a reviewer sees the cells
    # that are already satisfied rather than the cells that would run without
    # them. A second pass is what turns the accepted set into plan state; it is
    # confined to `--plan` because an ordinary run PRODUCES evidence and has no
    # reason to consume it.
    if (( show_plan == 1 )) && [[ -n "${merge_base}" ]] && [[ -f scripts/ci/local_evidence.py ]]; then
        head_sha="$(git rev-parse HEAD)"
        if python3 scripts/ci/local_evidence.py verify --cells \
            --plan "${plan_view}" --base "${merge_base}" --head "${head_sha}" \
            --rejections "${rejected_file}" > "${accepted_file}" 2>/dev/null; then
            # shellcheck disable=SC2086
            python3 scripts/ci/affected_scope.py "${scope_args[@]}" \
                --accepted-cells "${accepted_file}" \
                --evidence-rejections "${rejected_file}" \
                ${changed} > "${scope_file}"
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
        plan_bin="${BISCUIT_CI_PLAN_BIN:-scripts/target/release/ci-plan}"
        if [[ -x "${plan_bin}" ]] && "${plan_bin}" "${plan_view}"; then
            :
        else
            echo
            echo "Resolved plan (cell / execution / origin / state)"
            echo "------------------------------------------------"
            jq -r '.cells[] | "  \(.package)/\(.environment)/\(.gate)  \(.execution)  \(.origin)  \(.state)  — \(.selection_reason)"' "${plan_view}"
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
    rm -f "${reports_dir}/manifest.jsonl"
    export BISCUIT_JUNIT_STAGE_DIR="${reports_dir}"

    local_environment=""
    if (( run_l2 == 1 )); then
        if ! command -v sniff > /dev/null 2>&1 || ! command -v jq > /dev/null 2>&1; then
            echo "ci-local: --l2 requires sniff and jq; run 'just init'." >&2
            exit 2
        fi
        os_json="$(sniff os --json)"
        case "$(jq -r '.os_type' <<<"${os_json}")" in
            MacOS) local_environment="macos-latest" ;;
            Windows) local_environment="windows-latest" ;;
            Linux)
                if [[ "$(jq -r '.kernel' <<<"${os_json}" | tr '[:upper:]' '[:lower:]')" == *microsoft* ]]; then
                    local_environment="wsl2-ubuntu"
                else
                    local_environment="ubuntu-latest"
                fi
                ;;
            *) echo "ci-local: unsupported host reported by sniff." >&2; exit 2 ;;
        esac
    fi

    # Validate scheduling before treating package results as reusable evidence.
    selftest_log="$(mktemp "${TMPDIR:-/tmp}/biscuit-ci-local-selftest.XXXXXX")"
    # `test_resolved_plan.py` carries the sniff area-drift contract (AC15),
    # which only runs where sniff is installed — a developer host, not a
    # hosted runner. `test_runner_loss.py` derives every producer's composite
    # job label from the shipped workflows, so a rename that would silently
    # kill runner-loss attribution fails here rather than on a dead runner.
    for suite in test_schema.py test_affected_scope.py test_resolved_plan.py test_ci_local.py test_constraints.py test_runner_loss.py; do
        if python3 "scripts/ci/${suite}" > "${selftest_log}" 2>&1; then
            passed+=("ci-infra ${suite}")
        else
            cat "${selftest_log}"
            failed+=("ci-infra ${suite}")
        fi
    done
    rm -f "${selftest_log}"

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
                failed+=("check ${package} (check-only selection)")
            fi
        fi
        if (( run_lint == 1 )) && [[ " ${gates} " == *" lint "* ]]; then
            if just _lint "${package}"; then
                passed+=("lint ${package}")
            else
                failed+=("lint ${package}")
            fi
        fi
        if (( run_test == 1 )) && [[ " ${gates} " == *" test "* ]]; then
            # shellcheck disable=SC2086
            if just _test "${package}" --no-fail-fast ${test_args}; then
                passed+=("test ${package}")
            else
                failed+=("test ${package}")
            fi
        fi
        if (( run_l2 == 1 )) && [[ " ${gates} " == *" test "* ]]; then
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
