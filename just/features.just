TODAY := `date +%F`


_title_case str:
    #!/usr/bin/env bash
    set -euo pipefail
    s="{{str}}"
    s="${s//-/ }"
    echo "${s}" | awk '{for(i=1;i<=NF;i++) $i=toupper(substr($i,1,1)) tolower(substr($i,2))}1'

# chooses a spec or design file to identify the feature
_choose_feature feat="":
    #!/usr/bin/env bash
    set -euo pipefail
    file="$(fd -g '*{spec,design}\.md' features/ | {{ if feat == ""  { "cat" } else { "rg " + feat } }} |  fzf --height=15 --border --border-label 'Choose a spec or design file')"
    echo "${file}"


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

    echo -e "🏃‍♂️ Starting review for the {{BOLD}}{{YELLOW}}${feature}{{RESET}} feature"
    echo
    [[ -n "${spec}" ]] && echo -e "    - Specification File: {{DIM}}$(basename ${spec}){{RESET}}"
    [[ -n "${design}" ]] && echo -e "    - Design File: {{DIM}}$(basename ${design}){{RESET}}"
    echo -e "    - Feature Directory: {{DIM}}${dir}{{RESET}}"

    claudine compose "@prompts/review-feature.md" -y --set "{ dir: \"${dir}\", spec: \"$(basename ${spec})\", design: \"$(basename ${design})\" }"
    echo
    echo "🎉 review for {{BOLD}}{{YELLOW}}${feature}{{RESET}} has completed"
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

    echo -e "🏃‍♂️ Starting the {{ITALIC}}implementation{{RESET}} of the review suggestions on the {{BOLD}}{{YELLOW}}${feature}{{RESET}} feature ({{DIM}}{{BLUE}}${review}{{RESET}})"
    echo
    [[ -n "${spec}" ]] && echo -e "    - Specification File: {{DIM}}$(basename ${spec}){{RESET}}"
    [[ -n "${design}" ]] && echo -e "    - Design File: {{DIM}}$(basename ${design}){{RESET}}"
    echo -e "    - Design File: {{DIM}}$(basename ${review}){{RESET}}"
    echo -e "    - Feature Directory: {{DIM}}${dir}{{RESET}}"

    claudine compose "@prompts/implement-review-suggestions.md" -y --set "{ dir: \"${dir}\", spec: \"$(basename ${spec})\", design: \"$(basename ${design})\" }"
    echo
    echo "🎉 the {{ITALIC}}implementation{{RESET}} of the review ({{DIM}}{{BLUE}}${review}{{RESET}}) for {{BOLD}}{{YELLOW}}${feature}{{RESET}} has completed"
    echo


# creates a new feature and then prompts for next steps
feature feat:
    #!/usr/bin/env bash
    file="features/_unscheduled/{{feat}}.md"
    if $(md edit "${file}"); then
        echo "The spec file has been created at {{BLUE}}${file}{{RESET}}"
    else
        rm "${file}" >/dev/null
        echo "the feature had no content so was removed"
    fi

