# Local stand-in for the CI package gates.
#
# For every package CI would schedule for this branch's changes, run the same
# two gates CI runs, with the same arguments:
#
#   just _lint <pkg>                      clippy, --all-targets, NO features
#   just _test <pkg> --no-fail-fast <test_args>   L1 with the declared CI features
#
# The lint gate deliberately builds without features: that is CI's invocation,
# and it is where incomplete `terminal-tests`/`daemon-tests`-style gating
# surfaces (an ungated test module cannot resolve its optional crate). Run
# `--lint-only` first — it is minutes, not tens of minutes, and it caught four
# of the seven CI rounds of 2026-08-27.
#
# Scope comes from `scripts/ci/affected_scope.py`, the same script CI uses, fed
# the files that differ from the base ref (committed, staged, unstaged, and
# untracked). A change under `just/`, `.github/`, `Cargo.toml`, ... is global
# there too — unless its diff against the base touches only comments and blank
# lines — so a recipe change covers the whole workspace; `--lint-only` keeps
# that tractable.
#
# Also replicated: preflight's CI-infra self-test suite
# (`scripts/ci/test_affected_scope.py`, ~1s on every run). It is what catches
# a stale scope fixture — e.g. a new workspace dependency edge — locally
# instead of on three preflight cells.
#
# Not replicated: L2/L3/browser tiers, WSL archives, companion suites, and the
# Windows runner. Use `just test-l2` inside an area, and `just cross-check
# <pkg>` for the Linux/Windows build hosts. CI stays the proof; this is the
# discovery loop.
#
# Usage:
#   just ci-local                       affected packages vs origin/main
#   just ci-local --lint-only           clippy gates only
#   just ci-local --test-only           L1 gates only
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
    select_all=0
    dry_run=0
    selectors=()
    set -- {{ args }}
    while (( $# > 0 )); do
        case "$1" in
            --lint-only) run_test=0 ;;
            --test-only) run_lint=0 ;;
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
    lock_file="$(mktemp "${TMPDIR:-/tmp}/biscuit-ci-local-lock.XXXXXX")"
    trap 'rm -f "${scope_file}" "${lock_file}"' EXIT

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
        lock_args=()
        if git show "${merge_base}:Cargo.lock" > "${lock_file}" 2>/dev/null; then
            lock_args=(--base-lockfile "${lock_file}")
        fi
        # shellcheck disable=SC2086
        python3 scripts/ci/affected_scope.py "${lock_args[@]}" --base-ref "${merge_base}" ${changed} > "${scope_file}"
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

    count="$(wc -l <<< "${packages}" | tr -d ' ')"
    full_scope="$(jq -r '.full_scope' "${scope_file}")"
    change_class="$(jq -r '.change_class' "${scope_file}")"
    echo "CI scope: ${count} package(s) (class=${change_class}, full_scope=${full_scope})"
    echo "Gates   : $([[ ${run_lint} == 1 ]] && printf 'lint ')$([[ ${run_test} == 1 ]] && printf 'test')"
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

    # Preflight parity: CI runs this exact suite on every OS before any
    # package gate. Quiet on success; full unittest output on failure.
    selftest_log="$(mktemp "${TMPDIR:-/tmp}/biscuit-ci-local-selftest.XXXXXX")"
    if python3 scripts/ci/test_affected_scope.py > "${selftest_log}" 2>&1; then
        passed+=("preflight ci-infra self-test")
    else
        cat "${selftest_log}"
        failed+=("preflight ci-infra self-test")
    fi
    rm -f "${selftest_log}"

    while IFS= read -r package; do
        [[ -n "${package}" ]] || continue
        test_args="$(jq -r --arg p "${package}" '.matrix[] | select(.package == $p) | .test_args // ""' "${scope_file}")"
        if (( run_lint == 1 )); then
            if just _lint "${package}"; then
                passed+=("lint ${package}")
            else
                failed+=("lint ${package}")
            fi
        fi
        if (( run_test == 1 )); then
            # shellcheck disable=SC2086
            if just _test "${package}" --no-fail-fast ${test_args}; then
                passed+=("test ${package}")
            else
                failed+=("test ${package}")
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
    echo -e "{{ green }}All ${#passed[@]} gate(s) passed — CI's lint and L1 jobs should be green for this scope.{{ reset }}"
