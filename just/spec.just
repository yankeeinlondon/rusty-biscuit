
import "./util.just"

Today := `date +%F`
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
                        just clarify "${file}"
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



# adds a new fix topic to the package area
fix name:
    #!/usr/bin/env bash
    set -euo pipefail

    descriptive="$(just _title_case "{{name}}")"
    dir="{{Today}}-$(just _dasherize "{{name}}")"
    relative_dir="fixes/${dir}"
    mkdir -p "${relative_dir}"
    spec="${relative_dir}/spec.md"
    link="$(just _link "${spec}" "$(just _abs_path "${relative_dir}")" )"
    if md edit "${spec}"; then
        echo -e "Fix spec file created at {{blue}}${link}{{reset}}!"
        echo
        agent="$(just _choose_agent)"
        claudine compose @prompts/fix.md dir="${relative_dir}" spec="spec.md" -y --${agent} && (
            mv -r "${relative_dir}" "fixes/_completed/${dir}"
            link="$(just _link "spec.md" "$(just _abs_path "fixes/_completed/${dir}/spec.md")" )"
            echo
            echo -e "The {{bold}}${descriptive}"
        )

    else
        # rm -rf "${relative_dir}"
        echo -e "No content was added to the spec file for {{bold}}$(just _title_case "${name}"){{reset}}!"
    fi



# adds a complimentary tech design to the specification file
design filter="":
    #!/usr/bin/env bash
    set -euo pipefail

    if [[ -z "{{filter}}" ]]; then
        file="$(just _choose_spec)"
    else
        file="$(just _choose_spec "{{filter}}")"
    fi

    use_agent="$(just _choose_agent)"

    dir="$(dirname ${file})"
    echo
    echo -e "Creating a technical design from the spec file: {{blue}}${file}{{reset}}. Using {{bold}}${use_agent}{{reset}} agent."
    claudine compose @prompts/design.md --${use_agent} -y --set "{ spec: \"${file}\", dir: \"${dir}\"  }"
    echo
    link="$()"
    echo -e "We now have a {{bold}}technical design{{reset}} document for this specification"
    if [[ "$(just _confirm "Are you ready to create a plan for this feature?")" == "y" ]]; then
        just plan-feature "${file}" "${dir}/tech-design.md"
    fi

clarify filter="":
    #!/usr/bin/env bash
    set -euo pipefail

    if [[ -z "{{filter}}" ]]; then
        file="$(fd -g '*{spec,design}*.md' | fzf --height=15 --border --border-label 'Choose a spec or design file' )"
    else
        file="$(fd -g '*{spec,design}*.md' | rg "{{filter}}" | fzf --height=15 --border --border-label 'Choose a spec or design file' )"
    fi

    agent="$(just _choose_agent)"
    doc_path="$(sniff repo package-area)/${file}"
    claudine compose "@prompts/clarify.md" --${agent} -y -i --set "{ doc: \"@${doc_path}\" }"
    link="$(just _link $(basename "${file}") "${file}")"
    echo -e "The {{blue}}${link}{{reset}} file has been {{italic}}clarified{{reset}}"


# No-op recipe: reference color vars so linters won't report them as unused.
# Run `just _colors` if you want to exercise this (it does nothing).
_colors:
    @: {{bold}} {{italic}} {{reset}} {{red}} {{green}} {{yellow}} {{blue}} {{magenta}} {{cyan}} {{dim}}  >/dev/null || true
