# TODAY := `date +%F`
set allow-duplicate-variables
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

agents := "claude\ncodex\nopencode\ngemini\nqwen\nroo\nkimi\n"


# chooses a spec or design file to identify the feature
_choose_feature feat="":
    #!/usr/bin/env bash
    set -euo pipefail
    file="$(fd -g '*{spec,design}\.md' features/ | {{ if feat == ""  { "cat" } else { "rg " + feat } }} |  fzf --height=15 --border --border-label 'Choose a spec or design file')"
    echo "${file}"

_choose_agent:
    #!/usr/bin/env bash
    set -euo pipefail
    agent="$(echo "{{agents}}" | fzf --height=15 --border --border-label 'Choose the Agent to use')"


# ask user to choose a spec file
_choose_spec filter="":
    #!/usr/bin/env bash
    set -euo pipefail
    if [[ -z "{{filter}}" ]]; then
        file="$(fd -g '*spec*\.md'  |  fzf --height=15 --border --border-label 'Choose a spec or design file' )"
    else
        file="$(fd -g '*spec*\.md' | rg "{{filter}}" | fzf --height=15 --border --border-label 'Choose the spec file' )"
    fi
    echo "${file}"

_get_design_doc dir:
    #!/usr/bin/env bash
    set -euo pipefail

    mapfile -t files < <(fd -g '*design*.md' "{{dir}}")
    if [[ ${#files[@]} -eq 1 ]]; then
        design="${files[0]}"
    elif [[ ${#files[@]} -eq 0 ]]; then
        design=""
    else
        design="$( ( echo "NONE"; fd -g '*design*.md' "{{dir}}" ) | fzf --height=5 --border --border-label 'Choose matching design (or NONE to skip)' )"
        [[ "${design}" == "NONE" ]] && design=""
    fi
    echo "${design}"

# Specify a feature with a spec file, a design file, or both which has now been implemented
# and you want to run a review on.
review-feature feat="":
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

    echo -e "🏃‍♂️ Starting review for the {{bold}}{{yellow}}${feature}{{reset}} feature"
    echo
    [[ -n "${spec}" ]] && echo -e "    - Specification File: {{dim}}$(basename ${spec}){{reset}}"
    [[ -n "${design}" ]] && echo -e "    - Design File: {{dim}}$(basename ${design}){{reset}}"
    echo -e "    - Feature Directory: {{dim}}${dir}{{reset}}"

    claudine compose "@prompts/review-feature.md" -y --set "{ dir: \"${dir}\", spec: \"$(basename ${spec})\", design: \"$(basename ${design})\" }"
    echo
    echo "🎉 review for {{bold}}{{yellow}}${feature}{{reset}} has completed"
    echo


implement-review-suggestions feat="" iteration="1":
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

    review="review-{{iteration}}.md"

    echo -e "🏃‍♂️ Starting the {{italic}}implementation{{reset}} of the review suggestions on the {{bold}}{{yellow}}${feature}{{reset}} feature ({{dim}}{{blue}}${review}{{reset}})"
    echo
    [[ -n "${spec}" ]] && echo -e "    - Specification File: {{dim}}$(basename ${spec}){{reset}}"
    [[ -n "${design}" ]] && echo -e "    - Design File: {{dim}}$(basename ${design}){{reset}}"
    echo -e "    - Design File: {{dim}}$(basename ${review}){{reset}}"
    echo -e "    - Feature Directory: {{dim}}${dir}{{reset}}"

    claudine compose "@prompts/implement-review-suggestions.md" -y --set "{ dir: \"${dir}\", spec: \"$(basename ${spec})\", design: \"$(basename ${design})\" }"
    echo
    echo "🎉 the {{italic}}implementation{{reset}} of the review ({{dim}}{{blue}}${review}{{reset}}) for {{bold}}{{yellow}}${feature}{{reset}} has completed"
    echo

# adds a complimentary tech design to the specification file
design-feature spec="" agent="":
    #!/usr/bin/env bash
    set -euo pipefail

    if [[ -z "{{spec}}" ]]; then
        file="$(just _choose_spec features)"
    else
        file="$(just _choose_spec features "{{spec}}")"
    fi

    use_agent="$(echo "{{agents}}" | fzf --border --border-label "Choose an Agent")"

    dir="$(dirname ${file})"
    echo
    echo -e "Designing the feature from the spec file: {{blue}}${file}{{reset}}. Using {{bold}}{{agent}}{{reset}} agent."
    claudine compose @prompts/design.md --${use_agent} -y --set "{ spec: \"${file}\", dir: \"${dir}\"  }"
    echo
    echo -e "We now have a {{bold}}technical design{{reset}} document for this specification"
    if [[ "$(just _confirm "Are you ready to create a plan for this feature?")" == "y" ]]; then
        plan-feature "${file}" "${dir}/tech-design.md"
    fi

# adds a complimentary tech design to the specification file
design-fix spec="" agent="codex":
    #!/usr/bin/env bash
    set -euo pipefail

    if [[ -z "{{spec}}" ]]; then
        file="$(just _choose_spec fixes)"
    else
        file="$(just _choose_spec fixes "{{spec}}")"
    fi

    use_agent="$(echo "{{agents}}" | fzf --border --border-label "Choose an Agent")"

    dir="$(dirname ${file})"
    echo
    echo -e "Designing the feature from the spec file: {{blue}}${file}{{reset}}. Using {{bold}}{{agent}}{{reset}} agent."
    claudine compose @prompts/design.md --{{agent}} -y --set "{ spec: \"${file}\", dir: \"${dir}\"  }"
    echo
    echo -e "We now have a **technical design** document for this specification"
    if [[ "$(just _confirm "Are you ready to create a plan for this feature?")" == "y" ]]; then
        plan-feature "${file}" "${dir}/tech-design.md"
    fi


# Helps to fill in the details of your specification so that it fully articulates your requirements
clarify-spec spec="":
    #!/usr/bin/env bash
    set -euo pipefail

    if [[ -z "{{spec}}" ]]; then
        file="$(just _choose_spec)"
    else
        file="$(just _choose_spec "{{spec}}")"
    fi

    filename="$(basename "${file}")"
    abs_filename="$(just _abs_path "${file}")"
    link="$(just _link "${filename}" "${abs_filename}")"
    echo
    echo -e "🏃‍♂️ clarifying the requirements in the spec file {{blue}}${link}{{reset}} ..."
    if claudine compose "@prompts/clarify.md" --claude -y -i --set "{ spec: \"{{spec}}\"  }"; then
        echo
        echo -e "We have completed the {{italic}}clarification{{reset}} work on the specification file ({{blue}}${link}{{reset}})"
        echo
        echo -e "With the specification file now clarified, it might be a good time to create a complimentary tech-design document."
        if [[ "$(just _confirm "Start the technical design?")" == "y" ]]; then
            just design-feature "${file}"
        else
            echo
            echo -e "Ok, run {{green}}just design-feature {{dim}}[spec]{{reset}} when you're ready."
            echo
            exit 0
        fi
    else
        echo "there were problems that occurred during the clarification process"
    fi

plan-feature spec="" design="":
    #!/usr/bin/env bash
    set -euo pipefail

    if [[ -z "{{spec}}" ]]; then
        spec_file="$(just _choose_spec)"
    else
        if [[ "{{spec}}" == *.md ]]; then
            spec_file="{{spec}}"
        else
            spec_file=""
        fi
    fi
    dir="$(dirname "${spec_file}")"
    if [[ -z "{{design}}" ]]; then
        design_file="$(just _get_design_doc "${dir}")"
    else
        design_file="{{design}}"
    fi
    agent="$(just _choose_agent)"

    if [[ -z "${design_file}" ]]; then
        design_info=""
    else
        abs_design_filename="$(just _abs_path "${design_file}")"
        link_design="$(just _link "$(basename "${design_file}")" "${abs_design_filename}")"
        design_info=" {{italic}}and the{{reset}} {{blue}}${link_design}{{reset}}"
    fi

    spec_filename="$(basename "${spec_file}")"
    abs_spec_filename="$(just _abs_path "${spec_file}")"
    link_spec="$(just _link "${spec_filename}" "${abs_spec_filename}")"
    echo
    echo -e "🏃‍♂️ creating a detailed {{bold}}plan{{reset}} from the {{blue}}${link_spec}{{reset}}${design_info}"
    echo
    claudine compose @prompts/plan.md -y --${agent} --set "{ spec: \"${spec_file}\", design: \"${design_file}\" }"
    echo
    echo -e "🎉 planning is complete!"


# moves features/_unscheduled/<feat> to features/YYYY-MM-DD-<feat>
schedule-feature feat="":
    #!/usr/bin/env bash
    set -euo pipefail

    src="features/_unscheduled/{{feat}}"
    if [[ ! -d "${src}" || -z "{{feat}}" ]]; then
        echo -e "{{red}}No unscheduled feature found at{{reset}} {{blue}}${src}{{reset}}"
        src="$( ( echo "None"; fd --type d --max-depth 1 . features/_unscheduled ) | fzf --height=15 --border --border-label 'Choose an unscheduled feature (or None to abort)' )"
        if [[ "$src" == "None" ]]; then
            echo "No planning can happen without a valid feature directory!"
            exit 1
        fi
    fi

    today="$(date +%F)"
    name="$(basename "${src}")"
    dest="features/${today}-${name}"

    if [[ -e "${dest}" ]]; then
        echo -e "{{red}}Destination already exists:{{reset}} {{blue}}${dest}{{reset}}"
        exit 1
    fi

    mv "${src}" "${dest}"
    abs_dest="$(just _abs_path "${dest}")"
    link_dest="$(just _link "${dest}" "${abs_dest}")"
    echo
    echo -e "📅 scheduled {{bold}}{{yellow}}${name}{{reset}} for {{green}}${today}{{reset}}"
    echo -e "    {{dim}}→{{reset}} {{blue}}${link_dest}{{reset}}"
    echo

# creates a new feature and then prompts for next steps
feature feat:
    #!/usr/bin/env bash
    set -euo pipefail

    file="features/_unscheduled/{{feat}}/spec.md"
    file_link="$(just _link "{{blue}}{{feat}}{{reset}}" "$(just _abs_path "{{feat}}")")"

    if [[ -f "${file}" ]]; then
        echo
        echo -e "A {{italic}}spec{{reset}} file for the feature {{blue}}${file_link}{{reset}} already exists inside the {{italic}}_unscheduled{{reset}} folder"
        echo
        eza -lhga --git --hyperlink --group "${file}" 2>/dev/null
        echo

        if just _file_has_content "${file}" 2>/dev/null; then
            if [[ "$(just _confirm "Would you like to have us to {{bold}}{{italic}}clarify{{reset}} the requirements in this spec file?")" == y ]]; then
                just clarify-spec "${file}"
            else
                if [[ "$(just _confirm "Do you want to create a {{bold}}{{italic}}technical design{{reset}} now from the specification file?")" == y ]]; then
                    just design-feature "${file}"
                else
                    echo "Ok. Bye."
                    exit 0
                fi
            fi
        else
            if [[ "$(just _confirm "The spec file is {{italic}}empty{{reset}}; do you want to edit the specification now?")" == y ]]; then
                if md edit "${file}" 2>/dev/null; then
                    echo
                    echo "Now that the spec file has content, do you want to run a {{bold}}{{italic}}clarifying{{reset}} agent to"
                    if [[ "$(just _confirm "make the specification clearer before building a technical design?")" == y ]]; then
                        just clarify-spec "${file}"
                    else
                        if [[ "$(just _confirm "Do you want to create a {{bold}}{{italic}}technical design{{reset}} now from the specification file?")" == y ]]; then
                            just design-feature "${file}"
                        else
                            echo "Ok. Bye."
                            exit 0
                        fi
                    fi
                fi

            else
                echo "Ok. Empty spec file left in place."
                exit 0
            fi
        fi
        exit 0
    fi
    if md edit "${file}"; then
        echo -e "The spec file has been created at {{blue}}${file}{{reset}}"

        if [[ "$(just _confirm "make the specification clearer before building a technical design?")" == y ]]; then
            just clarify-spec "${file}"
        else
            if [[ "$(just _confirm "Do you want to create a {{bold}}{{italic}}technical design{{reset}} now from the specification file?")" == y ]]; then
                just design-feature "${file}"
            else
                echo "Ok. Bye."
                exit 0
            fi
        fi
    else
        rm -f "${file}"
        echo -e "the feature had no content so was removed"
    fi

# No-op recipe: reference color vars so linters won't report them as unused.
# Run `just _colors` if you want to exercise this (it does nothing).
_colors:
    @: {{bold}} {{italic}} {{reset}} {{red}} {{green}} {{yellow}} {{blue}} {{magenta}} {{cyan}} {{dim}}  >/dev/null || true
