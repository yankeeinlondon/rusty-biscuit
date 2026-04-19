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
    echo -e "Will call the {{bold}}${agent}{{reset}} agent to implement each phase"
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
            if [[ "$(just _confirm "Do you want to include a matching {{bold}}spec{{reset}} file?")" == "y" ]]; then
                spec_file="$(just _get_spec_doc "${doc_dir}")"
            fi
        else
            spec_file="${doc}"
            if [[ "$(just _confirm "Do you want to include a matching {{bold}}design{{reset}} file?")" == "y" ]]; then
                design_file="$(just _get_design_doc "${doc_dir}")"
            fi
        fi
    fi

    agent="$(just _choose_agent)"

    spec_info=""
    if [[ -n "${spec_file}" ]]; then
        spec_filename="$(basename "${spec_file}")"
        abs_spec_filename="$(just _abs_path "${spec_file}")"
        link_spec="$(just _link "${spec_filename}" "${abs_spec_filename}")"
        spec_info="{{blue}}${link_spec}{{reset}}"
    fi

    design_info=""
    if [[ -n "${design_file}" ]]; then
        design_filename="$(basename "${design_file}")"
        abs_design_filename="$(just _abs_path "${design_file}")"
        link_design="$(just _link "${design_filename}" "${abs_design_filename}")"
        if [[ -n "${spec_info}" ]]; then
            design_info=" {{italic}}and the{{reset}} {{blue}}${link_design}{{reset}}"
        else
            design_info="{{blue}}${link_design}{{reset}}"
        fi
    fi

    echo
    echo -e "🏃‍♂️ creating a detailed {{bold}}plan{{reset}} from the ${spec_info}${design_info}"
    echo
    claudine compose @prompts/plan.md -y --${agent} --set "{ spec: \"${spec_file}\", design: \"${design_file}\" }"
    echo
    echo -e "🎉 planning is complete!"





# No-op recipe: reference color vars so linters won't report them as unused.
# Run `just _colors` if you want to exercise this (it does nothing).
_colors:
    @: {{bold}} {{italic}} {{reset}} {{red}} {{green}} {{yellow}} {{blue}} {{magenta}} {{cyan}} {{dim}}  >/dev/null || true
