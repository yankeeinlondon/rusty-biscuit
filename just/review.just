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


# Specify a feature with a spec file, a design file, or both which has now been implemented
# and you want to run a review on.
review feat="" agent="":
    #!/usr/bin/env bash
    set -euo pipefail
    doc="$(just _choose_feature {{feat}}'')"
    dir="$(dirname "${doc}")"
    if [[ "${doc}" == *"design"* ]]; then
        design="${doc}"
        mapfile -t files < <(fd -g '*spec*.md' "${dir}")
        if [[ ${#files[@]} -eq 1 ]]; then
            spec="${files[0]}"
        elif [[ ${#files[@]} -eq 0 ]]; then
            spec=""
        else
            spec="$( ( echo "NONE"; fd -g '*spec*.md' "${dir}" ) | fzf --height=5 --border --border-label 'Choose matching spec (or NONE to skip)' )"
            [[ "${spec}" == "NONE" ]] && spec=""
        fi
    else
        spec="${doc}"
        mapfile -t files < <(fd -g '*design*.md' "${dir}")
        if [[ ${#files[@]} -eq 1 ]]; then
            design="${files[0]}"
        elif [[ ${#files[@]} -eq 0 ]]; then
            design=""
        else
            design="$( ( echo "NONE"; fd -g '*design*.md' "${dir}" ) | fzf --height=5 --border --border-label 'Choose matching design (or NONE to skip)' )"
            [[ "${design}" == "NONE" ]] && design=""
        fi
    fi
    feature="$(basename "${dir}")"
    feature="$(just _title_case ${feature#????-??-??-})"
    use_agent="{{agent}}"
    if [[ -z "${use_agent}" ]]; then
        use_agent="$(just _choose_agent)"
    fi
    model=""
    use_model=""
    if [[ "${use_agent}" == "opencode" ]];then
        model="$(just _choose_model)"
        use_model="--model ${model}"
    fi

    iteration=1
    while [[ -f "${dir}/review-${iteration}.md" ]]; do
        (( iteration++ ))
    done

    echo -e "Review for the {{bold}}{{yellow}}${feature}{{reset}} feature"
    echo
    [[ -n "${spec}" ]] && echo -e "    - Specification File: {{dim}}$(basename "${spec}"){{reset}}"
    [[ -n "${design}" ]] && echo -e "    - Design File: {{dim}}$(basename "${design}"){{reset}}"
    echo -e "    - Feature Directory: {{dim}}${dir}{{reset}}"
    echo -e "    - Review Iteration:  {{dim}}${iteration}{{reset}}"
    echo
    if [[ -z "${model}" ]];then
        echo -e "The {{yellow}}${use_agent}{{reset}} agent will be used"
    else
        echo -e "The {{yellow}}${use_agent}{{reset}} agent will be used {{italic}}with{{reset}} the model {{green}}${model}{{reset}}"
    fi

    if [[ "$(just _confirm "Ready to start?")" == "n" ]]; then
        echo
        echo -e "Ok bye"
        exit 0
    fi

    spec_name=""
    [[ -n "${spec}" ]] && spec_name="$(basename "${spec}")"
    design_name=""
    [[ -n "${design}" ]] && design_name="$(basename "${design}")"

    claudine compose "@prompts/review-feature.md" -y dir="${dir}" spec="${spec_name}" design="${design_name}" iteration="${iteration}" --${use_agent} ${use_model} || (
        just _play two-tone
        echo -e "{{red}}ERROR:{{reset}} There was a problem trying to create the review for ${feature}"
    )

    echo
    echo -e "🎉 review for {{bold}}{{yellow}}${feature}{{reset}} has completed: $(just _link "$(sniff repo package-area)/${dir}/review-${iteration}.md")"
    echo

# implement a review that was conducted (not a feature review)
implement-review filter="":
    #!/usr/bin/env bash
    set -euo pipefail

    echo
    echo -e "Preparing to implement all {{italic}}suggestions{{reset}} found in a Review"
    review="$(just _choose_review "{{filter}}")"
    agent="$(just _choose_agent)"
    model=""
    if [[ "${agent}" == "opencode" ]];then
        model="--model $(just _choose_model)"
    fi
    claudine compose prompts/review-implementation.md review="${review}" -y --${agent} ${model} || (
        echo
        echo -e "{{red}}ERROR:{{reset}} failed to complete the implementation of the {{blue}}${review}{{reset}} review"
        echo
        just _speak "Failed to complete the implementation of a review's suggestions in the $(sniff repo package-area) area"
    )


# implements the suggestions from a review of a feature or fix
implement-feature-review filter="" spec="" design="" agent="":
    #!/usr/bin/env bash
    set -euo pipefail
    echo -e "Preparing to {{italic}}implement{{reset}} all of suggestions from a review."
    review="$(just _choose_spec_review "{{filter}}")"
    dir="$(dirname "${review}")"
    spec_file="{{spec}}"
    design_file="{{design}}"

    if [[ -z "${spec_file}" ]] && [[ "$(just _has_spec_file "${dir}")" == true ]]; then
        spec_file="$(just _get_spec_doc "${dir}")"
    fi
    if [[ -z "${design_file}" ]] && [[ "$(just _has_design_file "${dir}")" == true ]]; then
        design_file="$(just _get_design_doc "${dir}")"
    fi

    has_spec="$(just _has_spec_file "${dir}")"
    has_design="$(just _has_design_file "${dir}")"

    iteration=1
    while [[ -f "${dir}/review-${iteration}.md" ]]; do
        (( iteration++ ))
    done
    (( iteration > 1 )) && (( iteration-- ))

    if [[ "${has_spec}" == false && "${has_design}" == false ]]; then
        echo
        echo "No specification or design document available to compare the review against!"
        exit 1
    fi

    if [[ -z "{{agent}}" ]]; then
        use_agent="$(just _choose_agent)"
    else
        use_agent="{{agent}}"
    fi
    model=""
    if [[ "${use_agent}" == "opencode" ]];then
        model="--model $(just _choose_model)"
    fi

    echo
    echo -e "About to {{bold}}implement{{reset}} the {{blue}}$(just _link "${review}"){{reset}} review suggestions"
    echo -e "    - Review Iteration:  {{dim}}${iteration}{{reset}}"
    echo -e "This review will compare against the {{italic}}requirements{{reset}} defined in:"
    if [[ "${has_spec}" == true && -n "${spec_file}" ]]; then
        link="$(just _link "${spec_file}")"
        echo -e "  - {{bold}}Specification file:{{reset}} {{blue}}${link}{{reset}}"
    fi
    if [[ "${has_design}" == true && -n "${design_file}" ]]; then
        link="$(just _link "${design_file}")"
        echo -e "  - {{bold}}Design file:{{reset}} {{blue}}${link}{{reset}}"
    fi

    echo
    if [[ -z "${model}" ]];then
        echo -e "The {{yellow}}${use_agent}{{reset}} agent will be used for the implementation"
    else
        echo -e "The {{yellow}}${use_agent}{{reset}}({{italic}}${model}{{reset}}) agent will be used for the implementation"
    fi
    echo

    if [[ "$(just _confirm "Ready to start?")" == "n" ]]; then
        echo
        echo -e "Ok bye"
        exit 0
    fi

    spec_name=""
    [[ -n "${spec_file}" ]] && spec_name="$(basename "${spec_file}")"
    design_name=""
    [[ -n "${design_file}" ]] && design_name="$(basename "${design_file}")"

    echo -e "🏃‍♂️ Starting the {{bold}}implementation{{reset}} of the review suggestions (#${iteration})"
    if claudine compose "@prompts/implement-feature-review-suggestions.md" -y --${use_agent} dir="${dir}" spec="${spec_name}" design="${design_name}" iteration="${iteration}" ${model}; then

        ready="$(md get "${review}" "ready")"
        if [[ ready == "false" ]]; then
            echo
            echo -e "🎁 Implemented all the suggestions found in the $(just _link "${review}") review!"
            echo -e "👎 the reviewing agent ({{dim}}${use_agent}{{reset}}) believes this feature is {{red}}not{{reset}} production ready!"
            echo

        elif [[ "$ready" == "true" ]]; then
            echo
            echo -e "🎁 Implemented all the suggestions found in the $(just _link "${review}") review!"
            echo -e "🎉 the reviewing agent ({{dim}}${use_agent}{{reset}}) believes this feature is production ready!"
            echo
        else
            echo
            echo -e "🎁 Implemented all the suggestions found in the $(just _link "${review}") review!"
            echo -e "🤨 the 'ready' frontmatter property was not set!"
            echo
        fi
        just _speak "The implementation of review suggestions in "$(sniff repo package-area)" package area has completed"
    else
        echo
        echo -e "❌ Failed to complete the implementation of review suggestions in $(just _link "${review}")"
        echo
        just _speak "An implementation of review suggestions in "$(sniff repo package-area)" package area has failed!"
        exit 1
    fi



# No-op recipe: reference color vars so linters won't report them as unused.
# Run `just _colors` if you want to exercise this (it does nothing).
_colors:
    @: {{bold}} {{italic}} {{reset}} {{red}} {{green}} {{yellow}} {{blue}} {{magenta}} {{cyan}} {{dim}}  >/dev/null || true
