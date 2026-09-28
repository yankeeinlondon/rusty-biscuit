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

    descriptive="$(just _title_case "{{ feat }}")"
    dasherized="$(just _dasherize "{{ feat }}")"

    dir="features/_unscheduled/${dasherized}"
    file="${dir}/spec.md"

    if [[ -f "${file}" ]]; then
        file_link="$(just _link "${descriptive}" "${file}")"
        echo
        echo -e "A {{ italic }}spec{{ reset }} file for the feature {{ blue }}${file_link}{{ reset }} already exists inside the {{ italic }}_unscheduled{{ reset }} folder"
        echo
        eza -lhga --git --icons=always --hyperlink --group "${file}" 2>/dev/null
        echo

        if just _file_has_content "${file}" 2>/dev/null; then
            if [[ "$(just _confirm "Would you like to {{ bold }}{{ italic }}clarify{{ reset }} the requirements in this spec file?")" == y ]]; then
                clarify_agent="$(just _choose_agent)"
                clarify_model=""
                if [[ "${clarify_agent}" == "opencode" ]];then
                    clarify_model="--model $(just _choose_model)"
                fi
                abs_spec="$(just _abs_path "${file}")"
                claudine compose @prompts/clarify.md --${clarify_agent} -y -i doc="${abs_spec}" ${clarify_model}
            fi
            if [[ "$(just _confirm "Do you want to create a {{ bold }}{{ italic }}technical design{{ reset }} now from the specification file?")" == y ]]; then
                just design "${dasherized}"
            else
                echo "Ok. Bye."
                exit 0
            fi
        else
            if [[ "$(just _confirm "The spec file is {{ italic }}empty{{ reset }}; do you want to edit the specification now?")" == y ]]; then
                if md edit "${file}" 2>/dev/null; then
                    echo
                    if [[ "$(just _confirm "Do you want to run a {{ bold }}{{ italic }}clarifying{{ reset }} agent to make the specification clearer?")" == y ]]; then
                        clarify_agent="$(just _choose_agent)"
                        clarify_model=""
                        if [[ "${clarify_agent}" == "opencode" ]];then
                            clarify_model="--model $(just _choose_model)"
                        fi
                        abs_spec="$(just _abs_path "${file}")"
                        claudine compose @prompts/clarify.md --${clarify_agent} -y -i doc="${abs_spec}" ${clarify_model}
                    fi
                    if [[ "$(just _confirm "Do you want to create a {{ bold }}{{ italic }}technical design{{ reset }} now from the specification file?")" == y ]]; then
                        just design "${dasherized}"
                    else
                        echo "Ok. Bye."
                        exit 0
                    fi
                fi
            else
                echo "Ok. Empty spec file left in place."
                exit 0
            fi
        fi
        exit 0
    fi

    mkdir -p "${dir}"

    if md edit "${file}"; then
        link="$(just _link "${file}")"
        echo -e "The spec file has been created at {{ blue }}${link}{{ reset }}"

        if [[ "$(just _confirm "Do you want to run a clarify session on the spec?")" == "y" ]]; then
            clarify_agent="$(just _choose_agent)"
            clarify_model=""
            if [[ "${clarify_agent}" == "opencode" ]];then
                clarify_model="--model $(just _choose_model)"
            fi
            abs_spec="$(just _abs_path "${file}")"
            claudine compose @prompts/clarify.md --${clarify_agent} -y -i doc="${abs_spec}" ${clarify_model}
        fi
        if [[ "$(just _confirm "Do you want to create a {{ bold }}{{ italic }}technical design{{ reset }} now from the specification file?")" == "y" ]]; then
            just design "${dasherized}"
        else
            echo "Ok. Bye."
            exit 0
        fi
    else
        # rm -rf "${dir}"
        echo -e "No content was added to the spec file for {{ bold }}${descriptive}{{ reset }}!"
    fi

# adds a complimentary tech design to the specification file
[arg("dir")]
[arg("spec")]
design filter="" spec="" dir="":
    #!/usr/bin/env bash
    set -euo pipefail

    file="{{ spec }}"
    use_dir="{{ dir }}"

    # ensure that `file` is set
    if [[ -z "${file}" ]]; then
        if [[ -z "{{ filter }}" ]]; then
            file="$(just _choose_spec)"
        else
            file="$(just _choose_spec "{{ filter }}")"
        fi
    fi

    if [[ -z "${use_dir}" ]]; then
        use_dir="$(dirname ${file})"
    fi

    use_agent="$(just _choose_agent)"
    model=""
    if [[ "${use_agent}" == "opencode" ]];then
        model="--model $(just _choose_model)"
    fi

    echo
    echo -e "Creating a technical design from the spec file: {{ blue }}${file}{{ reset }}. Using {{ bold }}${use_agent}{{ reset }} agent."
    claudine compose @prompts/design.md --${use_agent} -y spec="$(basename "${file}")" dir="${use_dir}" ${model} || (
        echo
        echo -e "Failed to complete the design for {{ blue }}$(just _link "${file}"){{ reset }} using the {{ yellow }}${use_agent}{{ reset }} agent"
        echo
        just _speak "Attempt to create a technical design in the $(sniff repo package-area) package area has failed!"
        exit 1
    )
    echo
    link="$(just _link "${use_dir}/tech-design.md")"
    echo -e "We now have a {{ bold }}technical design{{ reset }} document for {{ blue }}${link}{{ reset }}"

clarify filter="" agent="":
    #!/usr/bin/env bash
    set -euo pipefail

    if [[ -z "{{ filter }}" ]]; then
        file="$(fd -g '*{spec,design}*.md' --exclude '_completed' | fzf --height=15 --border --border-label 'Choose a spec or design file' )"
    else
        file="$(fd -g '*{spec,design}*.md' --exclude '_completed' | rg "{{ filter }}" | fzf --height=15 --border --border-label 'Choose a spec or design file' )"
    fi

    use_agent="{{ agent }}"
    if [[ -z "${use_agent}" ]]; then
        use_agent="$(just _choose_agent)"
    fi
    model=""
    if [[ "${use_agent}" == "opencode" ]];then
        model="--model $(just _choose_model)"
    fi
    doc_path="${file}"
    link="$(just _link "${file}")"
    claudine compose "@prompts/clarify.md" --${use_agent} -y -i doc="${doc_path}" ${model} || (
        echo
        echo -e "Failed to complete the {{ yellow }}Clarify{{ reset }} prompt for ${link}"
        echo
        just _speak "While trying to clarify requirements in $(sniff repo package-area), a problem was encountered and the clarify prompt has failed"
        exit 1
    )
    echo -e "The {{ blue }}${link}{{ reset }} file has been {{ italic }}clarified{{ reset }}"
    if [[ "${file}" == *spec* ]]; then
        spec_dir="$(dirname "${file}")"
        if [[ "$(just _confirm "Do you want to create a {{ bold }}{{ italic }}technical design{{ reset }} for this spec?")" == "y" ]]; then
            just design --spec="${file}" --dir="${spec_dir}"
        fi
    fi

# No-op recipe: reference color vars so linters won't report them as unused.

# Run `just _colors` if you want to exercise this (it does nothing).
_colors:
    @: {{ bold }} {{ italic }} {{ reset }} {{ red }} {{ green }} {{ yellow }} {{ blue }} {{ magenta }} {{ cyan }} {{ dim }}  >/dev/null || true
