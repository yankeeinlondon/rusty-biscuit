bold := '\033[1m'
reset := '\033[0m'
blue := '\033[34m'
red := '\033[31m'

# Emit an OSC8 hyperlink: `$(just _link "text" "https://example.com")`
# or `$(just _link "text" "./README.md")`
#
# The link parameter accepts:
#   - An HTTP(S) URL (used as-is)
#   - A local file path (resolved to absolute and prefixed with file://)
#
# Uses %b for display so embedded `\033[..m` SGR codes are interpreted.
# Uses BEL (\a) as the OSC string terminator instead of ESC+backslash so the
# captured output can be safely interpolated into later `echo -e` calls —
# a trailing backslash would otherwise merge with a following `\033...` and
# be consumed by echo's backslash interpretation.
_link display link="":
    #!/usr/bin/env bash
    set -euo pipefail
    dp="{{ display }}"
    lk="{{ link }}"
    if [[ -z "${lk}" ]]; then
        if [[ -e "${dp}" ]]; then
            if [[ "${dp}" = /* ]]; then
                lk="${dp}"
            else
                lk="$(pwd)/${dp#./}"
            fi
        else
            printf '%b' "${dp}"
            exit 0
        fi
    fi
    if [[ "${lk}" == http://* || "${lk}" == https://* ]]; then
        uri="${lk}"
    elif [[ -e "${lk}" ]]; then
        if [[ "${lk}" = /* ]]; then
            uri="file://${lk}"
        else
            uri="file://$(pwd)/${lk#./}"
        fi
    else
        printf '%b' "The {{ bold }}link{{ reset }} passed into {{ blue }}_link{{ reset }} helper recipe was neither a valid HTTP url nor a valid local file reference!\n" >&2
        exit 1
    fi
    printf '\033]8;;%s\a%b\033]8;;\a' "${uri}" "${dp}"

# Resolve a path to an absolute path; works for non-existent paths.
# If the path is already absolute, returns it as-is.
# `$(just _abs_path ./foo)` or `$(just _abs_path /tmp/foo)`
_abs_path path:
    #!/usr/bin/env bash
    set -euo pipefail
    p="{{ path }}"
    if [[ "${p}" = /* ]]; then
        printf '%s\n' "${p}"
    else
        printf '%s/%s\n' "$(pwd)" "${p#./}"
    fi

_title_case str:
    #!/usr/bin/env bash
    set -euo pipefail
    s="{{ str }}"
    s="${s//-/ }"
    echo "${s}" | awk '{for(i=1;i<=NF;i++) $i=toupper(substr($i,1,1)) tolower(substr($i,2))}1'

# check whether file passed in has non-whitespace content in it
_file_has_content file:
    #!/usr/bin/env bash
    set -euo pipefail
    grep -q '[^[:space:]]' "{{ file }}"

# Prompt the user with a yes/no question. Prints `y` or `n` to stdout and
# always exits 0 so callers can branch with `[[ $(just _confirm "...") == y ]]`
# without triggering just's "Recipe failed" error banner on a "no" answer.
_confirm question:
    #!/usr/bin/env bash
    set -euo pipefail
    printf '%b [Y/n] ' "{{ question }}" >&2
    read -n 1 ans; echo >&2
    [[ $ans =~ ^[Nn]$ ]] && echo n || echo y

agents := "claude\ncodex\nopencode\ngemini\nqwen\nroo\nkimi\n"

# chooses a spec or design file to identify the feature
_choose_feature feat="":
    #!/usr/bin/env bash
    set -euo pipefail
    file="$(fd -g '*{spec,design}*\.md' features/ --exclude '_completed' | {{ if feat == "" { "cat" } else { "rg " + feat } }} |  fzf --height=15 --border --border-label 'Choose a spec or design file')"
    echo "${file}"

# chooses a spec or design file to identify the fix
_choose_fix feat="":
    #!/usr/bin/env bash
    set -euo pipefail
    file="$(fd -g '*{spec,design}*\.md' fixes/ --exclude '_completed' | {{ if feat == "" { "cat" } else { "rg " + feat } }} |  fzf --height=15 --border --border-label 'Choose a spec or design file')"
    echo "${file}"

# chooses a spec or design file to identify the feature
_choose_feature_or_fix feat="":
    #!/usr/bin/env bash
    set -euo pipefail
    file="$(fd -g '*{spec,design}*\.md' --exclude '_completed' | {{ if feat == "" { "cat" } else { "rg " + feat } }} |  fzf --height=15 --border --border-label 'Choose a spec or design file')"
    echo "${file}"

_choose_agent question="Choose the Agent to use":
    #!/usr/bin/env bash
    set -euo pipefail
    agent="$(echo "{{ agents }}" | fzf --height=15 --border --border-label '{{ question }}')"
    echo "${agent}"

# ask user to choose a spec file
_choose_spec filter="":
    #!/usr/bin/env bash
    set -euo pipefail
    if [[ -z "{{ filter }}" ]]; then
        mapfile -t files < <(fd -g '*spec*\.md' --exclude '_completed')
    else
        mapfile -t files < <(fd -g '*spec*\.md' --exclude '_completed' | rg "{{ filter }}")
    fi
    if [[ ${#files[@]} -eq 1 ]]; then
        echo "${files[0]}"
    elif [[ ${#files[@]} -gt 1 ]]; then
        printf '%s\n' "${files[@]}" | fzf --height=15 --border --border-label 'Choose a spec file'
    else
        echo -e "{{ red }}no spec files found!{{ reset }}" >&2
        exit 1
    fi

# ask user to choose a plan file
_choose_plan filter="":
    #!/usr/bin/env bash
    set -euo pipefail
    if [[ -z "{{ filter }}" ]]; then
        file="$(fd -g '*plan*\.md' --exclude '_completed' | fzf --height=15 --border --border-label 'Choose a plan file' )"
    else
        file="$(fd -g '*plan*\.md' --exclude '_completed' | rg "{{ filter }}" | fzf --height=15 --border --border-label 'Choose the plan file' )"
    fi
    echo "${file}"

_has_spec_file dir:
    #!/usr/bin/env bash
    set -euo pipefail
    count=$(fd -g '*spec*.md' "{{ dir }}" 2>/dev/null | wc -l)
    (( count > 0 )) && echo true || echo false

_has_design_file dir:
    #!/usr/bin/env bash
    set -euo pipefail
    count=$(fd -g '*design*.md' "{{ dir }}" 2>/dev/null | wc -l)
    (( count > 0 )) && echo true || echo false

_get_design_doc dir:
    #!/usr/bin/env bash
    set -euo pipefail

    mapfile -t files < <(fd -g '*design*.md' "{{ dir }}" 2>/dev/null)
    if [[ ${#files[@]} -eq 1 ]]; then
        design="${files[0]}"
    elif [[ ${#files[@]} -eq 0 ]]; then
        design=""
    else
        design="$( ( echo "NONE"; fd -g '*design*.md' "{{ dir }}" 2>/dev/null ) | fzf --height=5 --border --border-label 'Choose matching design (or NONE to skip)' )"
        [[ "${design}" == "NONE" ]] && design=""
    fi
    echo "${design}"

_choose_model question="Choose a Model":
    #!/usr/bin/env bash
    set -euo pipefail
    model="$(printf '%s\n' 'zai-coding-plan/glm-5.2' 'minimax/MiniMax-M3' 'kimi-for-coding/k3' 'omlx/Qwen3.6-35B-A3B-oQ6' 'omlx/Qwen3.6-27B-nvfp4' | fzf --border --border-label '{{ question }}')"
    echo "${model}"

# return a list of all `*review*.md` files found features, fixes, and reviews
_choose_review filter="":
    #!/usr/bin/env bash
    set -euo pipefail
    mapfile -t files < <(for d in reviews/; do [[ -d "${d}" ]] && fd -g '*review*.md' "${d}" --exclude '_completed'; done {{ if filter == "" { "" } else { "| rg \"" + filter + "\"" } }})
    if [[ ${#files[@]} -eq 1 ]]; then
        echo "${files[0]}"
    elif [[ ${#files[@]} -gt 1 ]]; then
        printf '%s\n' "${files[@]}" | fzf --height=15 --border --border-label 'Choose a review file'
    else
        echo -e "{{ red }}no reviews found!{{ reset }}" >&2
        exit 1
    fi

_choose_spec_review filter="":
    #!/usr/bin/env bash
    set -euo pipefail
    mapfile -t files < <(for d in fixes/ features/; do [[ -d "${d}" ]] && fd -g '*review*.md' "${d}"; done {{ if filter == "" { "" } else { "| rg \"" + filter + "\"" } }})
    if [[ ${#files[@]} -eq 1 ]]; then
        echo "${files[0]}"
    elif [[ ${#files[@]} -gt 1 ]]; then
        printf '%s\n' "${files[@]}" | fzf --height=15 --border --border-label 'Choose a review file'
    else
        echo -e "{{ red }}no reviews found!{{ reset }}" >&2
        exit 1
    fi

_get_spec_doc dir:
    #!/usr/bin/env bash
    set -euo pipefail

    mapfile -t files < <(fd -g '*spec*.md' "{{ dir }}" 2>/dev/null)
    if [[ ${#files[@]} -eq 1 ]]; then
        spec="${files[0]}"
    elif [[ ${#files[@]} -eq 0 ]]; then
        spec=""
    else
        spec="$( ( echo "NONE"; fd -g '*spec*.md' "{{ dir }}" 2>/dev/null ) | fzf --height=5 --border --border-label 'Choose matching spec (or NONE to skip)' )"
        [[ "${spec}" == "NONE" ]] && spec=""
    fi
    echo "${spec}"

# Derive a plan filename from a spec filename.
# If the spec basename ends with `spec-N.md`, returns `plan-N.md`.
# If it ends with `something-spec-N.md`, returns `something-plan-N.md`.
# Otherwise returns `plan.md`.
_derive_plan_name spec_file:
    #!/usr/bin/env bash
    set -euo pipefail
    spec_basename="$(basename "{{ spec_file }}")"
    if [[ "${spec_basename}" =~ (.*)spec(-[0-9]+)\.md$ ]]; then
        echo "${BASH_REMATCH[1]}plan${BASH_REMATCH[2]}.md"
    else
        echo "plan.md"
    fi

# Convert the passed string to kebab-case: "hello_world" or "helloWorld" → "hello-world"
_dasherize str:
    #!/usr/bin/env bash
    set -euo pipefail
    s="{{ str }}"
    s="${s//_/-}"
    s="${s// /-}"
    echo "${s}" | tr '[:upper:]' '[:lower:]' | sed -E 's/[-_ ]+/-/g' | sed -E 's/^-//'

# Present options from a file via fzf with a NONE fallback.
# Reads one option per line from `options_file`, appends NONE, and presents
# them in fzf with the given label.
# Exits 1 if NONE is chosen, fzf is cancelled, or fzf is not installed.
# Outputs the selected option to stdout on success.
_fzf_select label options_file:
    #!/usr/bin/env bash
    set -euo pipefail
    label="{{ label }}"
    file="{{ options_file }}"
    if ! command -v fzf >/dev/null 2>&1; then
        echo -e "{{ red }}fzf is not installed{{ reset }}" >&2
        exit 1
    fi
    result="$( ( cat "${file}"; echo "NONE" ) | fzf --height=15 --border --border-label "${label}")"
    if [[ -z "${result}" || "${result}" == "NONE" ]]; then
        exit 1
    fi
    echo "${result}"
