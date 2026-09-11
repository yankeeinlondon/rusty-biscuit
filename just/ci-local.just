# Local stand-in for the CI package gates.
#
# For every package CI would schedule for this branch's changes, run the same
# gates CI runs, with the same arguments:
#
#   just _lint <pkg>                      clippy, --all-targets, NO features
#   just _test <pkg> --no-fail-fast <test_args>   L1 with the declared CI features
#   just _test_l2 <pkg> <test_args>        declared L2 on a hostable backend
#   cargo check --all-targets -p <pkg>     direct reverse dependencies only
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
# Also replicated: preflight's CI-infra self-test suite
# (scope calculation and local concurrency/routing). These catch a stale scope
# fixture or a serial L2 regression locally before publishing validation evidence.
#
# Not replicated: L3/browser tiers, WSL archives, companion suites, and other
# operating systems. `--l2` adds the source packages' hostable L2 tier.
#
# Usage:
#   just ci-local                       affected packages vs origin/main
#   just ci-local --lint-only           clippy gates only
#   just ci-local --test-only           L1 gates only
#   just ci-local --l2                  add hostable L2 tests
#   just ci-local --base <ref>          diff against another ref (or CI_LOCAL_BASE)
#   just ci-local --all                 every workspace package
#   just ci-local --dry-run             print the scope and gates, run nothing
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
    scope_out="${BISCUIT_CI_SCOPE_OUT:-}"
    selectors=()
    set -- {{ args }}
    while (( $# > 0 )); do
        case "$1" in
            --lint-only) run_test=0; run_check=0; run_l2=0 ;;
            --test-only) run_lint=0; run_check=0; run_l2=0 ;;
            --l2) run_l2=1 ;;
            --scope-out) shift; scope_out="${1:?--scope-out needs a path}" ;;
            --scope-out=*) scope_out="${1#--scope-out=}" ;;
            --all) select_all=1 ;;
            --dry-run) dry_run=1 ;;
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
    trap 'rm -f "${scope_file}"' EXIT

    if (( select_all == 1 )) || (( ${#selectors[@]} > 0 )); then
        python3 scripts/ci/affected_scope.py --all > "${scope_file}"
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
        # shellcheck disable=SC2086
        python3 scripts/ci/affected_scope.py ${changed} > "${scope_file}"
        echo
        echo "Changed vs ${base} (${merge_base:0:9}): $(wc -l <<< "${changed}" | tr -d ' ') file(s)"
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
    echo "CI scope: ${count} package(s) (class=${change_class}, full_scope=${full_scope}${full_gates:+ for ${full_gates}})"
    echo "Gates   : $([[ ${run_lint} == 1 ]] && printf 'lint ')$([[ ${run_check} == 1 ]] && printf 'reverse-check ')$([[ ${run_test} == 1 ]] && printf 'L1 ')$([[ ${run_l2} == 1 ]] && printf 'L2')"
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
    for suite in test_affected_scope.py test_ci_local.py; do
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
        # Source packages carry every gate; direct reverse dependencies carry
        # compile-check only.
        gates="$(jq -r --arg p "${package}" '.matrix[] | select(.package == $p) | .gates // ["lint","check","test"] | join(" ")' "${scope_file}")"
        if (( run_check == 1 )) && [[ " ${gates} " == " check " ]]; then
            check_args="$(jq -r --arg p "${package}" '.matrix[] | select(.package == $p) | .check_args' "${scope_file}")"
            # shellcheck disable=SC2086
            if cargo check --all-targets ${check_args}; then
                passed+=("check ${package} (direct reverse dependency)")
            else
                failed+=("check ${package} (direct reverse dependency)")
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
                elif BISCUIT_L2_THREADS="${l2_threads}" BISCUIT_TEST_REQUIRED_BACKENDS="${required_csv}" just _test_l2 "${package}" ${test_args}; then
                    passed+=("L2 ${package} (${required_csv})")
                else
                    failed+=("L2 ${package} (${required_csv})")
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
