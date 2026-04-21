# set allow-duplicate-variables
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



# moves items from {fixes|features}/_unscheduled to {fixes|features}/YYYY-MM-DD-<name>
#
# Usage:
#   just schedule          — fzf picker across features/_unscheduled and fixes/_unscheduled
#   just schedule <name>   — auto-match <name> under either _unscheduled folder if unique
#
schedule name="":
    #!/usr/bin/env bash
    set -euo pipefail

    name="{{name}}"
    selected=""

    if [[ -n "${name}" ]]; then
        mapfile -t matches < <(for dir in features/_unscheduled fixes/_unscheduled; do
            [[ -d "${dir}" ]] || continue
            fd --type d --max-depth 1 . "${dir}" | while read -r d; do
                base="$(basename "${d}")"
                if [[ "${base}" == *"${name}"* ]]; then
                    echo "${d}"
                fi
            done
        done)
        if [[ ${#matches[@]} -eq 0 ]]; then
            echo -e "{{red}}No directory matching{{reset}} {{blue}}*${name}*{{reset}} {{red}}found in _unscheduled folders{{reset}}"
            exit 1
        elif [[ ${#matches[@]} -eq 1 ]]; then
            selected="${matches[0]}"
        else
            echo -e "{{yellow}}Multiple matches for{{reset}} {{blue}}${name}{{reset}}:"
            printf '  %s\n' "${matches[@]}"
            echo -e "{{yellow}}Please be more specific.{{reset}}"
            exit 1
        fi
    fi

    if [[ -z "${selected}" ]]; then
        mapfile -t dirs < <(for dir in features/_unscheduled fixes/_unscheduled; do
            [[ -d "${dir}" ]] || continue
            fd --type d --max-depth 1 . "${dir}" | while read -r d; do
                base="$(basename "${d}")"
                [[ "${base}" == _* ]] && continue
                echo "${d}"
            done
        done)
        if [[ ${#dirs[@]} -eq 0 ]]; then
            echo -e "{{red}}No unscheduled features or fixes to schedule.{{reset}}"
            exit 1
        fi
        selected="$( ( echo "None"; printf '%s\n' "${dirs[@]}" ) | fzf --height=15 --border --border-label 'Choose an unscheduled feature or fix (or None to abort)' )"
        if [[ "${selected}" == "None" || -z "${selected}" ]]; then
            echo "Aborted."
            exit 1
        fi
    fi

    base="$(basename "${selected}")"
    parent="$(dirname "${selected}")"
    category_root="$(dirname "${parent}")"
    category="$(basename "${category_root}")"
    today="$(date +%F)"
    dest="${category}/${today}-${base}"

    if [[ -e "${dest}" ]]; then
        echo -e "{{red}}Destination already exists:{{reset}} {{blue}}${dest}{{reset}}"
        exit 1
    fi

    mv "${selected}" "${dest}"
    abs_dest="$(just _abs_path "${dest}")"
    link_dest="$(just _link "${dest}" "${abs_dest}")"
    title="$(just _title_case "${base}")"
    echo
    echo -e "📅 scheduled {{bold}}{{yellow}}${title}{{reset}} for {{green}}${today}{{reset}}"
    echo -e "    {{dim}}→{{reset}} {{blue}}${link_dest}{{reset}}"
    echo




# Mark a scheduled feature or fix as completed.
#
# Usage:
#   just complete          — fzf picker across features/ and fixes/
#   just complete <name>   — auto-match YYYY-MM-DD-<name> if unique
#
complete name="":
    #!/usr/bin/env bash
    set -euo pipefail

    name="{{name}}"
    selected=""

    if [[ -n "${name}" ]]; then
        if [[ "${name}" == "fixes" || "${name}" == "features" ]]; then
            mapfile -t matches < <(fd --type d --max-depth 1 --exclude '_completed' --exclude '_unscheduled' . "${name}" | while read -r d; do
                base="$(basename "${d}")"
                [[ "${base}" == ????-??-??-* ]] && echo "${d}"
            done)
        else
            mapfile -t matches < <(for dir in features fixes; do
                [[ -d "${dir}" ]] && fd --type d --max-depth 1 --exclude '_completed' --exclude '_unscheduled' . "${dir}" | while read -r d; do
                    base="$(basename "${d}")"
                    if [[ "${base}" == ????-??-??-*"${name}"* ]]; then
                        echo "${d}"
                    fi
                done
            done)
        fi
        if [[ ${#matches[@]} -eq 0 ]]; then
            echo -e "{{red}}No directory matching{{reset}} {{blue}}*${name}{{reset}} {{red}}found under features/ or fixes/{{reset}}"
            exit 1
        elif [[ ${#matches[@]} -eq 1 ]]; then
            selected="${matches[0]}"
        else
            selected="$(printf '%s\n' "${matches[@]}" | fzf --height=15 --border --border-label 'Multiple matches found. Choose one')"
        fi
    fi

    if [[ -z "${selected}" ]]; then
        mapfile -t dirs < <(for dir in features fixes; do
            [[ -d "${dir}" ]] || continue
            fd --type d --max-depth 1 . "${dir}" | while read -r d; do
                base="$(basename "${d}")"
                [[ "${base}" == _* ]] && continue
                echo "${d}"
            done
        done)
        if [[ ${#dirs[@]} -eq 0 ]]; then
            echo -e "{{red}}No scheduled features or fixes to complete.{{reset}}"
            exit 1
        fi
        selected="$(printf '%s\n' "${dirs[@]}" | fzf --height=15 --border --border-label 'Choose a feature or fix to complete')"
        if [[ -z "${selected}" ]]; then
            echo "Aborted."
            exit 1
        fi
    fi

    base="$(basename "${selected}")"
    parent="$(dirname "${selected}")"
    category="$(basename "${parent}")"
    completed_dir="${category}/_completed"

    file_count="$(fd --type f . "${selected}" 2>/dev/null | wc -l | tr -d ' ')"

    if [[ "${file_count}" -eq 0 ]]; then
        echo -e "{{yellow}}Directory{{reset}} {{blue}}${selected}{{reset}} {{yellow}}is empty.{{reset}}"
        ans="$(just _confirm "Remove it?")"
        if [[ "${ans}" == "y" ]]; then
            rmdir "${selected}"
            echo -e "{{green}}Removed empty directory{{reset}} {{blue}}${selected}{{reset}}"
        else
            echo "Left as-is."
        fi
        exit 0
    fi

    mkdir -p "${completed_dir}"
    dest="${completed_dir}/${base}"

    if [[ -e "${dest}" ]]; then
        rm -rf "${selected}"
        abs_dest="$(just _abs_path "${dest}")"
        link_dest="$(just _link "${dest}" "${abs_dest}")"
        title="$(just _title_case "${base#????-??-??-}")"
        echo
        echo -e "{{green}}Already completed:{{reset}} {{bold}}{{yellow}}${title}{{reset}}"
        echo -e "  {{dim}}→{{reset}} {{blue}}${link_dest}{{reset}}"
        echo
        exit 0
    fi

    mv "${selected}" "${dest}"
    abs_dest="$(just _abs_path "${dest}")"
    link_dest="$(just _link "${dest}" "${abs_dest}")"
    title="$(just _title_case "${base#????-??-??-}")"
    echo
    echo -e "{{green}}Completed{{reset}} {{bold}}{{yellow}}${title}{{reset}}"
    echo -e "  {{dim}}→{{reset}} {{blue}}${link_dest}{{reset}}"
    echo


# No-op recipe: reference color vars so linters won't report them as unused.
# Run `just _colors` if you want to exercise this (it does nothing).
_colors:
    @: {{bold}} {{italic}} {{reset}} {{red}} {{green}} {{yellow}} {{blue}} {{magenta}} {{cyan}} {{dim}}  >/dev/null || true
