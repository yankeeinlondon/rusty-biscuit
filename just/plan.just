# set allow-duplicate-variables
set allow-duplicate-recipes
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

ImplementPhasePrompt := "../prompts/implement-phase.md"

plan-implement-review:
    #!/usr/bin/env bash
    set -euo pipefail

    echo -e "Kicking off a {{ bold }}Plan -> Implement -> Review{{ reset }} cycle"
    spec="$(just _choose_spec)"
    spec_file="$(basename "${spec}")"
    area="$(sniff repo package-area)"
    root="$(sniff repo root)"
    dir="$(dirname "${spec}")"
    fq_path="${root}${area}/${dir}"
    plan="$(just _derive_plan_name "${spec_file}")"
    planning_agent="$(just _choose_agent 'Choose the planning Agent')"
    planning_model=""
    planning_model_param=""
    if [[ "${planning_agent}" == "opencode" ]]; then
        planning_model="$(just _choose_model 'Choose the planning Model for opencode')"
        planning_model_param="--model ${planning_model}"
    fi
    implementing_agent="$(just _choose_agent 'Choose the Implementing Agent')"
    implementing_model=""
    implementing_model_param=""
    if [[ "${implementing_agent}" == "opencode" ]]; then
        implementing_model="$(just _choose_model 'Choose the implementing Model for opencode')"
        implementing_model_param="--model ${implementing_model}"
    fi
    reviewing_agent="$(just _choose_agent 'Choose the reviewing Agent')"
    reviewing_model=""
    reviewing_model_param=""
    if [[ "${reviewing_agent}" == "opencode" ]]; then
        reviewing_model="$(just _choose_model 'Choose the reviewing Model for opencode')"
        reviewing_model_param="--model ${reviewing_model}"
    fi
    echo
    echo -e "{{ bold }}Strategy:{{ reset }}"
    echo
    echo -e "1. we will plan the {{ blue }}${spec}{{ reset }} file (with {{ yellow }}${planning_agent}{{ reset }}); save to {{ blue }}${dir}/${plan}{{ reset }}"
    echo -e "2. we will then {{ italic }}implement{{ reset }} the plan using the {{ yellow }}${implementing_agent}{{ reset }} agent"
    echo -e "3. and finally we will review the implementation with {{ yellow }}${reviewing_agent}{{ reset }} agent"
    echo -e "path: ${fq_path}"
    echo
    if [[ "$(just _confirm "Ready to start?")" == "n" ]]; then
        echo
        echo -e "Ok bye"
        exit 0
    fi
    claudine compose @prompts/plan.md -y spec="${spec}" plan="${plan}" --${planning_agent} ${planning_model_param}
    plan_path="${fq_path}/${plan}"
    total_phases="$(md get "${plan_path}" phases)"
    if [[ -z "${total_phases}" ]] || ! [[ "${total_phases}" =~ ^[0-9]+$ ]]; then
        echo -e "{{ red }}{{ bold }}ERROR:{{ reset }} Expected a numeric 'phases' frontmatter field in {{ blue }}${plan_path}{{ reset }} but got: '{{ dim }}${total_phases}{{ reset }}'" >&2
        exit 1
    fi
    echo
    echo -e "Detected {{ yellow }}${total_phases}{{ reset }} phases in the plan"
    echo
    claudine compose @prompts/implement-phase.md -y plan="${dir}/${plan}" --${implementing_agent} ${implementing_model_param} total_phases="${total_phases}"
    claudine compose @prompts/review-feature.md -y spec="${spec}" iteration=1 --${reviewing_agent} ${reviewing_model_param}
    echo
    echo -e "The specification file {{ blue }}${spec}{{ reset }} drove a plan, implementation, and review."
    echo
    echo -e "  - {{ bold }}Review:{{ reset }} $(just _link "${dir}/review-1.md" "$(sniff repo root)/${dir}/review-1.md")"
    echo

implement-review:
    #!/usr/bin/env bash
    set -euo pipefail
    plan="$(just _choose_plan)"
    dir="$(dirname "${plan}")"
    spec="$(just _get_spec_doc "${dir}")"

    implementing_agent="$(just _choose_agent 'Choose the Implementing Agent')"
    implementing_model=""
    implementing_model_param=""
    if [[ "${implementing_agent}" == "opencode" ]]; then
        implementing_model="$(just _choose_model 'Choose the implementing Model for opencode')"
        implementing_model_param="--model ${implementing_model}"
    fi
    reviewing_agent="$(just _choose_agent 'Choose the reviewing Agent')"
    reviewing_model=""
    reviewing_model_param=""
    if [[ "${reviewing_agent}" == "opencode" ]]; then
        reviewing_model="$(just _choose_model 'Choose the reviewing Model for opencode')"
        reviewing_model_param="--model ${reviewing_model}"
    fi
    echo
    echo -e "{{ bold }}Strategy:{{ reset }}"
    echo
    echo -e "1. we will {{ italic }}implement{{ reset }} the plan {{ blue }}${plan}{{ reset }} using the {{ yellow }}${implementing_agent}{{ reset }} agent"
    echo -e "2. and then we will review the implementation with {{ yellow }}${reviewing_agent}{{ reset }} agent"
    echo
    echo -e "Based on the {{ blue }}${spec}{{ reset }} specification"
    echo
    if [[ "$(just _confirm "Ready to start?")" == "n" ]]; then
        echo
        echo -e "Ok bye"
        exit 0
    fi
    total_phases="$(md get "${plan}" phases)"
    if [[ -z "${total_phases}" ]] || ! [[ "${total_phases}" =~ ^[0-9]+$ ]]; then
        echo -e "{{ red }}{{ bold }}ERROR:{{ reset }} Expected a numeric 'phases' frontmatter field in {{ blue }}${plan}{{ reset }} but got: '{{ dim }}${total_phases}{{ reset }}'" >&2
        exit 1
    fi
    memory="$(md get "${plan}" memory)"
    start_phase="$(md get "${plan}" start_phase)"
    [[ -z "${start_phase}" ]] && start_phase=1
    if (( start_phase > total_phases )); then
        echo -e "{{ red }}start_phase (${start_phase}) exceeds total phases (${total_phases}) — aborting.{{ reset }}"
        exit 1
    fi
    echo -e "Starting at {{ yellow }}phase ${start_phase}{{ reset }} of {{ yellow }}${total_phases}{{ reset }}"
    claudine compose @prompts/implement-phase.md -y plan="${plan}" --${implementing_agent} ${implementing_model_param} phase="${start_phase}" total_phases="${total_phases}" memory="${memory}"
    claudine compose @prompts/review-feature.md -y spec="${spec}" iteration=1 --${reviewing_agent} ${reviewing_model_param}
    echo
    echo -e "The specification file {{ blue }}${spec}{{ reset }} drove implementation and review."
    echo
    echo -e "  - {{ bold }}Review:{{ reset }} $(just _link "${dir}/review-1.md" "$(sniff repo root)/${dir}/review-1.md")"
    echo

implement-spec filter="" agent="" model="":
    #!/usr/bin/env bash
    set -euo pipefail

    echo
    echo -e "Preparing to {{ italic }}implement{{ reset }} a {{ bold }}feature specification{{ reset }}. If the specification"
    echo -e "has a {{ bold }}{{ italic }}plan{{ reset }} file then you will be allowed to choose the plan file"
    echo -e "to execute off of a proper plan using the {{ blue }}$(just _link "{{ ImplementPhasePrompt }}"){{ reset }}"
    echo -e "prompt. However if that's what you wanted to do then you should probably "
    echo -e "just use {{ yellow }}just implement-plan{{ reset }}."
    echo
    echo -e "This {{ yellow }}recipe{{ reset }} is more for simple fixes or features which you don't believe"
    echo -e "a formal plan file is not strictly required to complete successfully."
    echo
    choice="$(just _choose_spec_doc_plan {{ filter }})"
    if [[ -z "${choice}" == "None" ]]; then
        echo "Ok bye."
        echo
    elif [[ "${choice}" =~ ^.*plan.*\.md ]]; then
        just implement-plan "${choice}"
    else

    fi
    use_agent="{{ agent }}"
    use_model="{{ model }}"

# implements a plan, phase by phase
implement-plan filter="":
    #!/usr/bin/env bash
    set -euo pipefail

    # If the argument is an existing file path, use it directly
    if [[ -f "{{ filter }}" ]]; then
        plan="{{ filter }}"
    else
        plan="$(just _choose_plan "{{ filter }}")"
    fi
    if [[ -z "${plan}" ]]; then
        echo -e "{{ red }}No plan file selected — aborting.{{ reset }}" >&2
        exit 1
    fi

    echo "plan is: ${plan}"
    phases="$(md get "${plan}" phases)"
    if [[ -z "${phases}" ]] || (( phases < 1 )); then
        echo -e "{{ red }}Plan is missing a numeric 'phases' field — aborting.{{ reset }}" >&2
        exit 1
    fi

    total="${phases}"
    memory="$(md get "${plan}" memory)"
    current="$(md get "${plan}" start_phase)"
    [[ -z "${current}" ]] && current=1
    if (( current > phases )); then
        echo -e "{{ red }}start_phase (${current}) exceeds total phases (${phases}) — aborting.{{ reset }}"
        exit 1
    fi

    link="$(just _link "${plan}" "$(just _abs_path "${plan}")")"

    echo -e "Implementing the {{ blue }}${link}{{ reset }} plan"
    echo -e "- will be run {{ italic }}phase{{ reset }} by {{ italic }}phase{{ reset }} ({{ italic }}{{ dim }}${phases} phases{{ reset }})"
    echo -e "- we will start with phase ${current}"

    agent="$(just _choose_agent)"

    model=""
    if [[ "${agent}" == "opencode" ]];then
        model="--model $(just _choose_model)"
        echo -e "Will call the {{ bold }}${agent}{{ reset }}({{ dim }}${model}{{ reset }}) agent to implement each phase"
    else
        echo -e "Will call the {{ bold }}${agent}{{ reset }} agent to implement each phase"
    fi

    echo
    if [[ "$(just _confirm "Ready to start?")" == "n" ]]; then
        echo
        echo -e "Ok bye"
        exit 0
    fi
    echo
    while true; do
        claudine compose "@prompts/implement-phase.md" plan="${plan}" phase=${current} total_phases=${total} memory="${memory}" --${agent} -y ${model} -v || (
            echo
            echo -e "{{ red }}{{ bold }}ERROR:{{ reset }} the implementation of the plan {{ blue }}$(just _link "${plan}") was unable to complete phase {{ yellow }}${current}{{ reset }}" || echo "failed at phase ${current}"
            md set "${plan}" "start_phase" ${current} || true
            echo
            exit 5
        )
        just _message "Phase ${current} (_of ${total}_) of the plan '${plan}' was implemented"
        echo
        # INSERT HERE
        git add . || true
        just commit || true
        echo
        [[ ${current} -ge ${phases} ]] && break
        ((current++))
    done
    echo
    echo -e "The {{ blue }}${link}{{ reset }} plan has implemented all ({{ dim }}{{ italic }}${phases} phases (of ${total}){{ reset }})"
    echo

# Interactive wrapper: pick an area and target document, then create a plan
plan spec="" design="" review="":
    #!/usr/bin/env bash
    set -euo pipefail

    spec_file="{{ spec }}"
    design_file="{{ design }}"
    review_file="{{ review }}"

    # If no files were provided, interactive selection via fzf
    if [[ -z "${spec_file}" && -z "${design_file}" && -z "${review_file}" ]]; then
        # Step 1: Choose area
        area="$(printf '%s\n' 'Feature' 'Fix' 'Review' 'Feature (unscheduled)' 'Fix (unscheduled)' | fzf --height=10 --border --border-label 'Choose planning area')"
        if [[ -z "${area}" ]]; then
            echo -e "{{ red }}No area selected — aborting.{{ reset }}" >&2
            exit 1
        fi

        # Step 2: Choose target document and delegate to appropriate planner
        case "${area}" in
            Feature)
                doc="$(fd -g '*{spec,design}*.md' features/ --exclude '_completed' | fzf --height=15 --border --border-label 'Choose a spec or design file')"
                if [[ -z "${doc}" ]]; then
                    echo -e "{{ red }}No document selected — aborting.{{ reset }}" >&2
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
                just plan-feature "${spec_file}" "${design_file}"
                ;;
            Fix)
                doc="$(fd -g '*{spec,design}*.md' fixes/ --exclude '_completed' | fzf --height=15 --border --border-label 'Choose a spec or design file')"
                if [[ -z "${doc}" ]]; then
                    echo -e "{{ red }}No document selected — aborting.{{ reset }}" >&2
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
                just plan-feature "${spec_file}" "${design_file}"
                ;;
            Review)
                just plan-review
                ;;
            'Feature (unscheduled)')
                doc="$(fd -g '*{spec,design}*.md' features/_unscheduled/ --exclude '_completed' | fzf --height=15 --border --border-label 'Choose a spec or design file')"
                if [[ -z "${doc}" ]]; then
                    echo -e "{{ red }}No document selected — aborting.{{ reset }}" >&2
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
                just plan-feature "${spec_file}" "${design_file}"
                ;;
            'Fix (unscheduled)')
                doc="$(fd -g '*{spec,design}*.md' fixes/_unscheduled/ --exclude '_completed' | fzf --height=15 --border --border-label 'Choose a spec or design file')"
                if [[ -z "${doc}" ]]; then
                    echo -e "{{ red }}No document selected — aborting.{{ reset }}" >&2
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
                just plan-feature "${spec_file}" "${design_file}"
                ;;
        esac
    else
        # Explicit arguments provided
        if [[ -n "${review_file}" ]]; then
            # Validate review path
            if [[ ! -f "${review_file}" ]]; then
                mapfile -t matches < <(fd -g '*review*.md' reviews/ --exclude '_completed' | rg "${review_file}" || true)
                if [[ ${#matches[@]} -eq 0 ]]; then
                    echo -e "{{ red }}No review files matching '${review_file}' found{{ reset }}" >&2
                    exit 1
                elif [[ ${#matches[@]} -eq 1 ]]; then
                    review_file="${matches[0]}"
                else
                    review_file="$(printf '%s\n' "${matches[@]}" | fzf --height=15 --border --border-label 'Choose matching review')"
                fi
            fi
            just plan-review "${review_file}"
        else
            # Validate spec/design paths — resolve partial matches via filter
            if [[ -n "${spec_file}" && ! -f "${spec_file}" ]]; then
                mapfile -t matches < <(fd -g '*spec*.md' | rg "${spec_file}" || true)
                if [[ ${#matches[@]} -eq 0 ]]; then
                    echo -e "{{ red }}No spec files matching '${spec_file}' found{{ reset }}" >&2
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
                    echo -e "{{ red }}No design files matching '${design_file}' found{{ reset }}" >&2
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

            just plan-feature "${spec_file}" "${design_file}"
        fi
    fi

# create a plan from a feature spec (and optionally design document)
plan-feature spec="" design="":
    #!/usr/bin/env bash
    set -euo pipefail

    spec_file="{{ spec }}"
    design_file="{{ design }}"

    # If neither was provided, let the user pick a spec or design file, then
    # optionally add the complementary doc from the same directory.
    if [[ -z "${spec_file}" && -z "${design_file}" ]]; then
        doc="$(just _choose_feature)"
        if [[ -z "${doc}" ]]; then
            echo -e "{{ red }}No spec or design file selected — aborting.{{ reset }}" >&2
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
                echo -e "{{ red }}No spec files matching '${spec_file}' found{{ reset }}" >&2
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
                echo -e "{{ red }}No design files matching '${design_file}' found{{ reset }}" >&2
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
    echo -e "Creating a detailed {{ bold }}plan{{ reset }} from:"
    if [[ -n "${spec_file}" ]]; then
        echo -e "  Specification: {{ blue }}$(just _link "${spec_file}" "$(just _abs_path "${spec_file}")"){{ reset }}"
        dir="$(dirname "${spec_file}")"
    fi
    if [[ -n "${design_file}" ]]; then
        echo -e "  Design Document: {{ blue }}$(just _link "${design_file}" "$(just _abs_path "${design_file}")"){{ reset }}"
        dir="$(dirname "${design_file}")"
    fi
    echo
    if [[ "$(just _confirm "Ready to start?")" == "n" ]]; then
        echo
        echo -e "Ok bye"
        exit 0
    fi
    echo
    plan="$(just _derive_plan_name "${spec_file}")"
    claudine compose @prompts/plan.md -y --${agent} spec="${spec_file}" design="${design_file}" plan="${plan}" ${model} || (
        echo
        echo "Planning failed for {{ bold }}${dir}{{ reset }}"
        echo
        just _speak "Planning failed in the $(sniff repo package-area) package area"
        exit 1
    )
    echo
    echo -e "🎉 planning is complete!"

    # Offer to implement the newly created plan
    plan_path="${dir}/${plan}"
    if [[ -f "${plan_path}" ]]; then
        echo
        echo -e "A plan file was created at {{ blue }}$(just _link "${plan_path}" "$(just _abs_path "${plan_path}")"){{ reset }}"
        if [[ "$(just _confirm "Do you want to implement this plan phase by phase now?")" == "y" ]]; then
            echo
            just implement-plan "${plan_path}"
        else
            echo
            echo -e "Ok. You can implement it later with: {{ yellow }}just implement-plan ${plan_path}{{ reset }}"
        fi
    fi
    echo

# create a plan from a review
plan-review review="":
    #!/usr/bin/env bash
    set -euo pipefail

    review_file="{{ review }}"

    # If not provided, let the user pick a review file
    if [[ -z "${review_file}" ]]; then
        review_file="$(fd -g '*review*.md' reviews/ --exclude '_completed' | fzf --height=15 --border --border-label 'Choose a review file')"
        if [[ -z "${review_file}" ]]; then
            echo -e "{{ red }}No review file selected — aborting.{{ reset }}" >&2
            exit 1
        fi
    else
        # Validate provided path — resolve partial matches via filter
        if [[ ! -f "${review_file}" ]]; then
            mapfile -t matches < <(fd -g '*review*.md' reviews/ --exclude '_completed' | rg "${review_file}" || true)
            if [[ ${#matches[@]} -eq 0 ]]; then
                echo -e "{{ red }}No review files matching '${review_file}' found{{ reset }}" >&2
                exit 1
            elif [[ ${#matches[@]} -eq 1 ]]; then
                review_file="${matches[0]}"
            else
                review_file="$(printf '%s\n' "${matches[@]}" | fzf --height=15 --border --border-label 'Choose matching review')"
            fi
        fi
    fi

    agent="$(just _choose_agent)"
    model=""
    if [[ "${agent}" == "opencode" ]];then
        model="--model $(just _choose_model)"
    fi

    echo
    echo -e "Creating a detailed {{ bold }}plan{{ reset }} from:"
    echo -e "  Review: {{ blue }}$(just _link "${review_file}" "$(just _abs_path "${review_file}")"){{ reset }}"
    dir="$(dirname "${review_file}")"
    echo
    if [[ "$(just _confirm "Ready to start?")" == "n" ]]; then
        echo
        echo -e "Ok bye"
        exit 0
    fi
    echo
    claudine compose "@prompts/plan-review-implementation.md" -y --${agent} review="${review_file}" ${model} || (
        echo
        echo "Planning failed for review {{ bold }}${dir}{{ reset }}"
        echo
        just _speak "Planning failed for review in the $(sniff repo package-area) package area"
        exit 1
    )
    echo
    echo -e "🎉 planning is complete!"

    # Offer to implement the newly created plan
    plan_file="${dir}/plan.md"
    if [[ -f "${plan_file}" ]]; then
        echo
        echo -e "A plan file was created at {{ blue }}$(just _link "${plan_file}" "$(just _abs_path "${plan_file}")"){{ reset }}"
        if [[ "$(just _confirm "Do you want to start implementing this plan now?")" == "y" ]]; then
            echo
            just implement-plan "${plan_file}"
        else
            echo
            echo -e "Ok. You can implement it later with: {{ yellow }}just implement-plan ${plan_file}{{ reset }}"
        fi
    fi
    echo

# No-op recipe: reference color vars so linters won't report them as unused.
# Run `just _colors` if you want to exercise this (it does nothing).
_colors:
    @: {{ bold }} {{ italic }} {{ reset }} {{ red }} {{ green }} {{ yellow }} {{ blue }} {{ magenta }} {{ cyan }} {{ dim }}  >/dev/null || true
