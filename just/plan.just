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



# implements a plan, phase by phase
implement-plan filter="":
    #!/usr/bin/env bash
    set -euo pipefail

    plan="$(just _choose_plan "{{filter}}")"
    if [[ -z "${plan}" ]]; then
        echo -e "{{red}}No plan file selected — aborting.{{reset}}" >&2
        exit 1
    fi

    phases="$(md get "${plan}" phases 2>/dev/null || echo "")"
    if [[ ! "${phases}" =~ ^[0-9]+$ ]] || (( phases < 1 )); then
        echo -e "{{red}}Plan is missing a numeric 'phases' field — aborting.{{reset}}" >&2
        exit 1
    fi

    current="$(md get "${plan}" start_phase 2>/dev/null || echo 1)"
    if [[ ! "${current}" =~ ^[0-9]+$ ]]; then
        current=1
    fi
    if (( current > phases )); then
        echo -e "{{red}}start_phase (${current}) exceeds total phases (${phases}) — aborting.{{reset}}" >&2
        exit 1
    fi

    link="$(just _link "${plan}" "$(just _abs_path "${plan}")")"

    echo -e "Implementing the {{blue}}${link}{{reset}} plan"
    echo -e "- will be run {{italic}}phase{{reset}} by {{italic}}phase{{reset}} ({{italic}}{{dim}}${phases} phases{{reset}})"
    echo -e "- we will start with phase ${current}"

    agent="$(just _choose_agent)"

    model=""
    if [[ "${agent}" == "opencode" ]];then
        model="--model $(just _choose_model)"
        echo -e "Will call the {{bold}}${agent}{{reset}}({{dim}}${model}{{reset}}) agent to implement each phase"
    else
        echo -e "Will call the {{bold}}${agent}{{reset}} agent to implement each phase"
    fi

    echo
    while true; do
        claudine compose "@prompts/implement-phase.md" plan="${plan}" phase=${current} --${agent} -y
        echo
        git add . || exit 0
        just commit
        echo
        [[ ${current} -ge ${phases} ]] && break
        ((current++))
    done
    echo
    echo -e "The {{blue}}${link}{{reset}} plan has completed ({{dim}}{{italic}}${phases} phases{{reset}})"
    echo



plan-feature spec="" design="":
    #!/usr/bin/env bash
    set -euo pipefail

    spec_file="{{spec}}"
    design_file="{{design}}"

    # If neither was provided, let the user pick a spec or design file, then
    # optionally add the complementary doc from the same directory.
    if [[ -z "${spec_file}" && -z "${design_file}" ]]; then
        doc="$(just _choose_feature)"
        if [[ -z "${doc}" ]]; then
            echo -e "{{red}}No spec or design file selected — aborting.{{reset}}" >&2
            exit 1
        fi
        doc_dir="$(dirname "${doc}")"
        if [[ "$(basename "${doc}")" == *design* ]]; then
            design_file="${doc}"
            spec_file="$(just _get_spec_doc "${doc_dir}")"
        else
            spec_file="${doc}"
            design_file="$(just _get_design_doc "${doc_dir}")"
        fi
    else
        # Validate provided paths — resolve partial matches via filter
        if [[ -n "${spec_file}" && ! -f "${spec_file}" ]]; then
            mapfile -t matches < <(fd -g '*spec*.md' | rg "${spec_file}" || true)
            if [[ ${#matches[@]} -eq 0 ]]; then
                echo -e "{{red}}No spec files matching '${spec_file}' found{{reset}}" >&2
                exit 1
            elif [[ ${#matches[@]} -eq 1 ]]; then
                spec_file="${matches[0]}"
            else
                spec_file="$(printf '%s\n' "${matches[@]}" | fzf --height=15 --border --border-label 'Choose matching spec')"
            fi
        fi

        if [[ -n "${design_file}" && ! -f "${design_file}" ]]; then
            mapfile -t matches < <(fd -g '*design*.md' | rg "${design_file}" || true)
            if [[ ${#matches[@]} -eq 0 ]]; then
                echo -e "{{red}}No design files matching '${design_file}' found{{reset}}" >&2
                exit 1
            elif [[ ${#matches[@]} -eq 1 ]]; then
                design_file="${matches[0]}"
            else
                design_file="$(printf '%s\n' "${matches[@]}" | fzf --height=15 --border --border-label 'Choose matching design')"
            fi
        fi

        # Auto-find complementary doc from the same directory
        if [[ -n "${spec_file}" && -z "${design_file}" ]]; then
            doc_dir="$(dirname "${spec_file}")"
            design_file="$(just _get_design_doc "${doc_dir}")"
        elif [[ -n "${design_file}" && -z "${spec_file}" ]]; then
            doc_dir="$(dirname "${design_file}")"
            spec_file="$(just _get_spec_doc "${doc_dir}")"
        fi
    fi

    agent="$(just _choose_agent)"
    model=""
    if [[ "${agent}" == "opencode" ]];then
        model="--model $(just _choose_model)"
    fi


    echo
    echo -e "Creating a detailed {{bold}}plan{{reset}} from:"
    if [[ -n "${spec_file}" ]]; then
        echo -e "  Specification: {{blue}}$(just _link "${spec_file}"){{reset}}"
        dir="$(dirname ${spec_file})"
    fi
    if [[ -n "${design_file}" ]]; then
        echo -e "  Design Document: {{blue}}$(just _link "${design_file}"){{reset}}"
        dir="$(dirname ${design_file})"
    fi
    echo
    if [[ "$(just _confirm "Ready to start?")" == "n" ]]; then
        echo
        echo -e "Ok bye"
        exit 0
    fi
    echo
    claudine compose @prompts/plan.md -y --${agent} spec="${spec_file}" design="${design_file}" ${model} || (
        echo
        echo "Planning failed for {{bold}}${dir}{{reset}}"
        echo
        just _speak "Planning failed in the $(sniff repo package-area) package area"
        exit 1
    )
    echo
    echo -e "🎉 planning is complete!"





# No-op recipe: reference color vars so linters won't report them as unused.
# Run `just _colors` if you want to exercise this (it does nothing).
_colors:
    @: {{bold}} {{italic}} {{reset}} {{red}} {{green}} {{yellow}} {{blue}} {{magenta}} {{cyan}} {{dim}}  >/dev/null || true
