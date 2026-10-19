set allow-duplicate-variables
# set allow-duplicate-recipes

import "./util.just"

bold := '\033[1m'
dim := '\033[2m'
italic := '\033[3m'
reset := '\033[0m'
red := '\033[31m'
green := '\033[32m'
yellow := '\033[33m'
blue := '\033[34m'
magenta := '\033[35m'
cyan := '\033[36m'

# Stand-in for the coming sequence support: alternate implement → review on an
# implemented spec until a review marks it production ready (at most 5 cycles),
# then run one follow-up pass for that review's open medium/low findings and
# observations. Rerunning on a ready spec runs only a still-open follow-up pass.
# Pass `--use-defaults` to skip the agent/model prompts and use the defaults.
[positional-arguments]
fix +args:
    #!/usr/bin/env bash
    set -euo pipefail

    max_cycles=5
    use_defaults=false
    spec=""
    for arg in "$@"; do
        case "${arg}" in
            --use-defaults) use_defaults=true ;;
            -*)
                echo -e "{{ red }}ERROR:{{ reset }} unknown option '${arg}' (supported: --use-defaults)" >&2
                exit 1
                ;;
            *)
                if [[ -n "${spec}" ]]; then
                    echo -e "{{ red }}ERROR:{{ reset }} expected one spec, got '${spec}' and '${arg}'" >&2
                    exit 1
                fi
                spec="${arg}"
                ;;
        esac
    done
    if [[ -z "${spec}" ]]; then
        echo -e "{{ red }}ERROR:{{ reset }} no spec given; usage: just fix <spec> [--use-defaults]" >&2
        exit 1
    fi

    if [[ ! -f "${spec}" ]]; then
        dirs=()
        for d in features fixes; do [[ -d "${d}" ]] && dirs+=("${d}"); done
        if [[ ${#dirs[@]} -eq 0 ]]; then
            echo -e "{{ red }}ERROR:{{ reset }} no features/ or fixes/ directory in $(pwd)" >&2
            exit 1
        fi
        matches="$(fd -g '*spec*.md' "${dirs[@]}" --exclude '_completed' --exclude '_unscheduled' | rg -F -- "${spec}" || true)"
        if [[ -z "${matches}" ]]; then
            echo -e "{{ red }}ERROR:{{ reset }} no active spec in ${dirs[*]} matches '${spec}'" >&2
            exit 1
        fi
        spec="$(printf '%s\n' "${matches}" | fzf --select-1 --height=15 --border --border-label 'Choose the spec to fix')"
    fi

    if ! implemented="$(md get --raw "${spec}" implemented)"; then
        echo -e "{{ red }}ERROR:{{ reset }} could not read the frontmatter of $(just _link "${spec}"); fix the error above before running the review/fix cycle." >&2
        exit 1
    fi
    if [[ "${implemented}" != "true" ]]; then
        echo -e "{{ yellow }}$(just _link "${spec}"){{ reset }} is not marked {{ italic }}implemented{{ reset }} (implemented: '${implemented}'); implement it before running the review/fix cycle." >&2
        exit 1
    fi

    dir="$(dirname "${spec}")"

    # The spec has no `ready` property of its own: readiness is the latest
    # review's `ready`, and a ready review also sets the spec's `completed`.
    latest_review() {
        local iterations
        iterations="$(md get --raw "${spec}" review_iterations)" || return 1
        [[ "${iterations}" =~ ^[0-9]+$ ]] && (( iterations > 0 )) && echo "${dir}/review-${iterations}.md"
        return 0
    }

    # A ready review's medium/low findings and observations, until its one
    # follow-up pass marks it `implemented`.
    open_followups() {
        local review="$1" implemented findings observations
        implemented="$(md get --raw "${review}" implemented)" || return 1
        if [[ "${implemented}" == "true" ]]; then echo 0; return 0; fi
        findings="$(md get --raw "${review}" findings)" || return 1
        observations="$(md get --raw "${review}" observations)" || return 1
        printf '%s\n%s\n' "${findings}" "${observations}" | grep -c . || true
    }

    review="$(latest_review)"
    review_ready=""
    if [[ -n "${review}" && -f "${review}" ]]; then
        review_ready="$(md get --raw "${review}" ready)"
    fi
    completed="$(md get --raw "${spec}" completed)"
    followups_only=false
    if [[ "${review_ready}" == "true" || "${completed}" == "true" ]]; then
        if [[ -n "${review}" && -f "${review}" ]] && (( $(open_followups "${review}") > 0 )); then
            followups_only=true
        else
            echo -e "$(just _link "${spec}") is already {{ green }}production ready{{ reset }}; nothing to do."
            exit 0
        fi
    fi

    choose_agent() {
        local label="$1" default="$2"
        if [[ "${use_defaults}" == "true" ]]; then echo "${default}"; return; fi
        { echo "${default}"; echo -e "{{ agents }}" | rg -vx -- "${default}" | rg -v '^$'; } \
            | fzf --height=12 --border --border-label "${label} agent (default: ${default})"
    }
    choose_model() {
        local label="$1" default="$2" answer
        if [[ "${use_defaults}" == "true" ]]; then echo "${default}"; return; fi
        read -r -p "$(echo -e "${label} model [{{ bold }}${default}{{ reset }}]: ")" answer
        echo "${answer:-${default}}"
    }

    review_agent="$(choose_agent "Review" codex)"
    review_model="$(choose_model "Review" gpt-6.1-sol)"
    implement_agent="$(choose_agent "Implementation" claude)"
    implement_model="$(choose_model "Implementation" opus)"

    echo
    echo -e "Review/fix cycle for $(just _link "${spec}") (at most ${max_cycles} cycles)"
    echo -e "    - review:         {{ yellow }}${review_agent}{{ reset }}/{{ green }}${review_model}{{ reset }}"
    echo -e "    - implementation: {{ yellow }}${implement_agent}{{ reset }}/{{ green }}${implement_model}{{ reset }}"
    echo

    # One pass, and no review after it: fixes the small medium/low findings
    # and files the rest in the area's `fixes/_unscheduled/` backlog.
    run_followups() {
        local review="$1" open
        open="$(open_followups "${review}")"
        (( open > 0 )) || return 0
        echo -e "🧹 follow-up pass for ${open} open item(s) in $(just _link "${review}")"
        claudine compose "@prompts/_implement/implement-suggestions.md" -y --${implement_agent} --model "${implement_model}" spec="${spec}"
    }

    if [[ "${followups_only}" == "true" ]]; then
        run_followups "${review}"
        exit 0
    fi

    cycle=0
    while (( cycle < max_cycles )); do
        (( ++cycle ))
        review="$(latest_review)"

        # Without an unimplemented review there are no findings to implement,
        # so the cycle starts with its review.
        review_implemented="true"
        if [[ -n "${review}" && -f "${review}" ]]; then
            review_implemented="$(md get --raw "${review}" implemented)"
        fi
        if [[ "${review_implemented}" != "true" ]]; then
            echo -e "🛠  cycle ${cycle}/${max_cycles}: implementing the findings of $(just _link "${review}")"
            claudine compose "@prompts/_implement/implement-suggestions.md" -y --${implement_agent} --model "${implement_model}" spec="${spec}"
        fi

        echo -e "🔎 cycle ${cycle}/${max_cycles}: reviewing $(just _link "${spec}")"
        claudine compose "@prompts/_reviews/feature-review.md" -y --${review_agent} --model "${review_model}" spec="${spec}"

        review="$(latest_review)"
        ready="$(md get --raw "${review}" ready)"
        if [[ "${ready}" == "true" ]]; then
            echo -e "👍 $(just _link "${review}") marked the spec {{ green }}production ready{{ reset }} after ${cycle} cycle(s)"
            run_followups "${review}"
            exit 0
        fi
        if [[ "${ready}" != "false" ]]; then
            echo -e "{{ red }}ERROR:{{ reset }} ${review} did not set a boolean 'ready' frontmatter property" >&2
            exit 1
        fi
    done

    echo -e "{{ red }}ERROR:{{ reset }} ${max_cycles} cycles ran without a production-ready review; latest is $(just _link "$(latest_review)")" >&2
    exit 1

# No-op recipe: reference color vars so linters won't report them as unused.
# Run `just _colors` if you want to exercise this (it does nothing).
_colors:
    @: {{ bold }} {{ italic }} {{ reset }} {{ red }} {{ green }} {{ yellow }} {{ blue }} {{ magenta }} {{ cyan }} {{ dim }}  >/dev/null || true

