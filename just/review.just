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
# implemented spec until a review marks it production ready (at most 5 cycles).
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

    implemented="$(md get --raw "${spec}" implemented 2>/dev/null || true)"
    if [[ "${implemented}" != "true" ]]; then
        echo -e "{{ yellow }}$(just _link "${spec}"){{ reset }} is not marked {{ italic }}implemented{{ reset }} (implemented: '${implemented}'); implement it before running the review/fix cycle." >&2
        exit 1
    fi

    dir="$(dirname "${spec}")"

    # The spec has no `ready` property of its own: readiness is the latest
    # review's `ready`, and a ready review also sets the spec's `completed`.
    latest_review() {
        local iterations
        iterations="$(md get --raw "${spec}" review_iterations 2>/dev/null || true)"
        [[ "${iterations}" =~ ^[0-9]+$ ]] && (( iterations > 0 )) && echo "${dir}/review-${iterations}.md"
        return 0
    }

    review="$(latest_review)"
    if [[ -n "${review}" && "$(md get --raw "${review}" ready 2>/dev/null || true)" == "true" ]] \
        || [[ "$(md get --raw "${spec}" completed 2>/dev/null || true)" == "true" ]]; then
        echo -e "$(just _link "${spec}") is already {{ green }}production ready{{ reset }}; nothing to do."
        exit 0
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

    cycle=0
    recurring=()
    while (( cycle < max_cycles )); do
        (( ++cycle ))
        review="$(latest_review)"

        # Without an unimplemented review there are no findings to implement,
        # so the cycle starts with its review.
        if [[ -n "${review}" && -f "${review}" && "$(md get --raw "${review}" implemented 2>/dev/null || true)" != "true" ]]; then
            echo -e "🛠  cycle ${cycle}/${max_cycles}: implementing the findings of $(just _link "${review}")"
            claudine compose "@prompts/_implement/implement-suggestions.md" -y --${implement_agent} --model "${implement_model}" spec="${spec}"
        fi

        echo -e "🔎 cycle ${cycle}/${max_cycles}: reviewing $(just _link "${spec}")"
        claudine compose "@prompts/_reviews/feature-review.md" -y --${review_agent} --model "${review_model}" spec="${spec}"

        review="$(latest_review)"
        ready="$(md get --raw "${review}" ready 2>/dev/null || true)"
        if [[ "${ready}" == "true" ]]; then
            echo -e "👍 $(just _link "${review}") marked the spec {{ green }}production ready{{ reset }} after ${cycle} cycle(s)"
            exit 0
        fi
        if [[ "${ready}" != "false" ]]; then
            echo -e "{{ red }}ERROR:{{ reset }} ${review} did not set a boolean 'ready' frontmatter property" >&2
            exit 1
        fi
        # A recurring finding class does not stop the loop: the review already
        # swept every sibling site, so the next cycle implements the full list.
        if [[ "$(md get --raw "${review}" recurrence 2>/dev/null || true)" == "true" ]]; then
            recurring+=("${review}")
            echo -e "{{ yellow }}NOTE:{{ reset }} $(just _link "${review}") repeats a finding class an earlier review raised; its sweep list is implemented next."
        fi
    done

    echo -e "{{ red }}ERROR:{{ reset }} ${max_cycles} cycles ran without a production-ready review; latest is $(just _link "$(latest_review)")" >&2
    if (( ${#recurring[@]} > 0 )); then
        echo -e "    reviews reporting a recurring finding class (read their {{ italic }}Recurrence{{ reset }} section first):" >&2
        for r in "${recurring[@]}"; do echo -e "    - $(just _link "${r}")" >&2; done
    fi
    exit 1

# No-op recipe: reference color vars so linters won't report them as unused.
# Run `just _colors` if you want to exercise this (it does nothing).
_colors:
    @: {{ bold }} {{ italic }} {{ reset }} {{ red }} {{ green }} {{ yellow }} {{ blue }} {{ magenta }} {{ cyan }} {{ dim }}  >/dev/null || true

