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
[arg("design")]
[arg("spec")]
review feat="" agent="" spec="" design="":
    #!/usr/bin/env bash
    set -euo pipefail

    provided_spec="{{ spec }}"
    provided_design="{{ design }}"

    if [[ -n "${provided_spec}" || -n "${provided_design}" ]]; then
        # Files provided directly — derive directory and skip feature chooser
        dir=""
        spec_file=""
        design_file=""

        if [[ -n "${provided_spec}" ]]; then
            if [[ ! -f "${provided_spec}" ]]; then
                echo -e "{{ red }}ERROR:{{ reset }} Spec file not found: ${provided_spec}" >&2
                exit 1
            fi
            spec_file="${provided_spec}"
            dir="$(dirname "${provided_spec}")"
        fi

        if [[ -n "${provided_design}" ]]; then
            if [[ ! -f "${provided_design}" ]]; then
                echo -e "{{ red }}ERROR:{{ reset }} Design file not found: ${provided_design}" >&2
                exit 1
            fi
            design_file="${provided_design}"
            [[ -z "${dir}" ]] && dir="$(dirname "${provided_design}")"
        fi

        # Resolve missing companion file(s)
        if [[ -z "${design_file}" ]]; then
            mapfile -t files < <(fd -g '*design*.md' "${dir}")
            if [[ ${#files[@]} -eq 1 ]]; then
                design_file="${files[0]}"
            elif [[ ${#files[@]} -eq 0 ]]; then
                design_file=""
            else
                design_file="$( ( echo "NONE"; fd -g '*design*.md' "${dir}" ) | fzf --height=5 --border --border-label 'Choose matching design (or NONE to skip)' )"
                [[ "${design_file}" == "NONE" ]] && design_file=""
            fi
        fi

        if [[ -z "${spec_file}" ]]; then
            mapfile -t files < <(fd -g '*spec*.md' "${dir}")
            if [[ ${#files[@]} -eq 1 ]]; then
                spec_file="${files[0]}"
            elif [[ ${#files[@]} -eq 0 ]]; then
                spec_file=""
            else
                spec_file="$( ( echo "NONE"; fd -g '*spec*.md' "${dir}" ) | fzf --height=5 --border --border-label 'Choose matching spec (or NONE to skip)' )"
                [[ "${spec_file}" == "NONE" ]] && spec_file=""
            fi
        fi
    else
        # Interactive feature chooser path
        doc="$(just _choose_feature_or_fix "{{ feat }}")"
        dir="$(dirname "${doc}")"
        spec_file=""
        design_file=""
        if [[ "${doc}" == *"design"* ]]; then
            design_file="${doc}"
            mapfile -t files < <(fd -g '*spec*.md' "${dir}")
            if [[ ${#files[@]} -eq 1 ]]; then
                spec_file="${files[0]}"
            elif [[ ${#files[@]} -eq 0 ]]; then
                spec_file=""
            else
                spec_file="$( ( echo "NONE"; fd -g '*spec*.md' "${dir}" ) | fzf --height=5 --border --border-label 'Choose matching spec (or NONE to skip)' )"
                [[ "${spec_file}" == "NONE" ]] && spec_file=""
            fi
        else
            spec_file="${doc}"
            mapfile -t files < <(fd -g '*design*.md' "${dir}")
            if [[ ${#files[@]} -eq 1 ]]; then
                design_file="${files[0]}"
            elif [[ ${#files[@]} -eq 0 ]]; then
                design_file=""
            else
                design_file="$( ( echo "NONE"; fd -g '*design*.md' "${dir}" ) | fzf --height=5 --border --border-label 'Choose matching design (or NONE to skip)' )"
                [[ "${design_file}" == "NONE" ]] && design_file=""
            fi
        fi
    fi
    feature="$(basename "${dir}")"
    feature="$(just _title_case ${feature#????-??-??-})"
    use_agent="{{ agent }}"
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
    # iterate until we find no pre-existing review file
    while [[ -f "${dir}/review-${iteration}.md" ]]; do
        (( iteration++ ))
    done

    full_dir="$(sniff repo package-area)/${dir}"
    full_dir_pretty="{{ dim }}$(sniff repo package-area)/$(dirname "${dir}")/{{ reset }}$(basename "${dir}")"
    PROMPT="@prompts/review-feature.md"
    echo -e "Review for the {{ bold }}{{ yellow }}${feature}{{ reset }} feature"
    echo
    echo -e "    - Feature Directory: {{ dim }}${full_dir_pretty}{{ reset }}"
    [[ -n "${spec_file}" ]] && echo -e "    - Specification File: {{ blue }}{{ dim }}$(just _link "${full_dir}/{{ reset }}{{ blue }}$(basename "${spec_file}"){{ reset }}" "${spec_file}"){{ reset }}"
    [[ -n "${design_file}" ]] && echo -e "    - Design File: {{ blue }}{{ dim }}$(just _link "${full_dir}/{{ reset }}{{ blue }}$(basename "${design_file}"){{ reset }}" "${spec_file}"){{ reset }}"
    echo -e "    - Review Iteration:  {{ dim }}${iteration}{{ reset }}"
    echo -e "    - Prompt File: {{ dim }}${PROMPT}{{ reset }}"
    echo
    saved_to="${full_dir_pretty}/review-${iteration}.md"
    if [[ -z "${model}" ]];then
        echo -e "The {{ yellow }}${use_agent}{{ reset }} agent will be used and saved to ${saved_to}"
    else
        echo -e "The {{ yellow }}${use_agent}{{ reset }} agent will be used {{ italic }}with{{ reset }} the model {{ green }}${model}{{ reset }} and the review saved to ${saved_to}"
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

    claudine compose "${PROMPT}" -y dir="${full_dir}" spec="${spec_name}" design="${design_name}" iteration="${iteration}" --${use_agent} ${use_model} || (
        just _play two-tone
        echo -e "{{ red }}ERROR:{{ reset }} There was a problem trying to create the review for ${feature}"
        exit 1
    )
    review="${dir}/review-${iteration}.md"
    ready="$(md get "${review}" "ready")"

    echo
    if [[ "$ready" == "false" ]]; then
        echo
        echo -e "🎁 The {{ italic }}feature{{ reset }} review for $(just _link "${review}") has completed."
        echo -e "👎 the reviewing agent ({{ dim }}${use_agent}{{ reset }}) believes this feature is {{ red }}not{{ reset }} production ready!"
        echo

    elif [[ "$ready" == "true" ]]; then
        echo
        echo -e "🎁 The {{ italic }}feature{{ reset }} review for $(just _link "${review}") has completed."
        echo -e "👍 the reviewing agent ({{ dim }}${use_agent}{{ reset }}) believes this feature is production ready!"
        echo
    else
        echo
        echo -e "🎁 The {{ italic }}feature{{ reset }} review for $(just _link "${review}") has completed."
        echo -e "🤨 the 'ready' frontmatter property was not set!"
        echo
    fi
    echo
    echo -e "- run the following command when you're ready to implement the suggestions:"
    echo
    echo -e "  {{ blue }}claudine compose prompts/implement-review.md -y spec="{{ spec }}" iteration=${iteration} {{ reset }} when you're ready to implement the"
    echo

# implement a review that was conducted (not a feature review)
implement-review filter="":
    #!/usr/bin/env bash
    set -euo pipefail

    echo
    echo -e "Preparing to implement all {{ italic }}suggestions{{ reset }} found in a Review"

    # Ask user what type of review to implement
    review_type="$(printf '%s\n' 'Feature or Fix Review (features/ or fixes/)' 'General Review (reviews/)' | fzf --height=10 --border --border-label 'What type of review do you want to implement?')"

    if [[ "${review_type}" == "Feature or Fix Review (features/ or fixes/)" ]]; then
        just implement-feature-review "{{ filter }}"
        exit 0
    fi

    # General review path (reviews/ directory)
    review="$(just _choose_review "{{ filter }}")"
    agent="$(just _choose_agent)"
    model=""
    if [[ "${agent}" == "opencode" ]];then
        model="--model $(just _choose_model)"
    fi
    claudine compose prompts/review-implementation.md review="${review}" -y --${agent} ${model} || (
        echo
        echo -e "{{ red }}ERROR:{{ reset }} failed to complete the implementation of the {{ blue }}${review}{{ reset }} review"
        echo
        just _speak "Failed to complete the implementation of a review's suggestions in the $(sniff repo package-area) area"
    )

# implements the suggestions from a review of a feature or fix
[arg("design")]
[arg("review")]
[arg("spec")]
implement-feature-review filter="" review="" spec="" design="" agent="":
    #!/usr/bin/env bash
    set -euo pipefail
    echo -e "Preparing to {{ italic }}implement{{ reset }} all of suggestions from a review."
    review="{{ review }}"
    if [[ -z "${review}" ]]; then
        review="$(just _choose_spec_review "{{ filter }}")"
    fi
    dir="$(dirname "${review}")"
    spec_file="{{ spec }}"
    design_file="{{ design }}"

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

    echo
    echo -e "About to {{ bold }}implement{{ reset }} the {{ blue }}$(just _link "${review}"){{ reset }} review suggestions"
    echo -e "    - Review Iteration:  {{ dim }}${iteration}{{ reset }}"
    echo -e "This review will compare against the {{ italic }}requirements{{ reset }} defined in:"
    if [[ "${has_spec}" == true && -n "${spec_file}" ]]; then
        link="$(just _link "${spec_file}")"
        echo -e "  - {{ bold }}Specification file:{{ reset }} {{ blue }}${link}{{ reset }}"
    fi
    if [[ "${has_design}" == true && -n "${design_file}" ]]; then
        link="$(just _link "${design_file}")"
        echo -e "  - {{ bold }}Design file:{{ reset }} {{ blue }}${link}{{ reset }}"
    fi

    if [[ -z "{{ agent }}" ]]; then
        use_agent="$(just _choose_agent)"
    else
        use_agent="{{ agent }}"
    fi
    model=""
    if [[ "${use_agent}" == "opencode" ]];then
        model="--model $(just _choose_model)"
    fi

    echo
    if [[ -z "${model}" ]];then
        echo -e "The {{ yellow }}${use_agent}{{ reset }} agent will be used for the implementation"
    else
        echo -e "The {{ yellow }}${use_agent}{{ reset }}({{ italic }}${model}{{ reset }}) agent will be used for the implementation"
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

    echo -e "🏃‍♂️ Starting the {{ bold }}implementation{{ reset }} of the review suggestions (#${iteration})"
    if claudine compose "@prompts/implement-feature-review-suggestions.md" -y --${use_agent} dir="${dir}" spec="${spec_name}" design="${design_name}" iteration="${iteration}" ${model}; then
        echo
        echo -e "🎉 Implemented all the suggestions found in the $(just _link "${review}") review!"
        echo

        if [[ "$(just _confirm "Run another review to validate this work?")" == "n" ]]; then
            echo
            echo -e "Ok bye"
            exit 0
        fi
        echo -e "Starting the {{ bold }}review{{ reset }} recipe with:"
        echo -e "  - Specification: $(just _link "${spec_file}")"
    if [[ ! -z "${design_file}" ]]; then
        echo -e "  - Design: $(just _link "${design_file}")"
    fi
    just review --spec="${spec_file}" --design="${design_file}"

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
    @: {{ bold }} {{ italic }} {{ reset }} {{ red }} {{ green }} {{ yellow }} {{ blue }} {{ magenta }} {{ cyan }} {{ dim }}  >/dev/null || true
