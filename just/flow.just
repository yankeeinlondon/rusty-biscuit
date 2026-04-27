set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

import "./util.just"

# Read a frontmatter value from the feature spec.
_read_config key spec="spec.md":
    #!/usr/bin/env bash
    set -euo pipefail
    value="$(md get --raw "{{ spec }}" "{{ key }}" 2>/dev/null || true)"
    printf '%s\n' "${value}"

# Write a frontmatter value to the feature spec.
_write_config key value spec="spec.md":
    #!/usr/bin/env bash
    set -euo pipefail
    md set --save "{{ spec }}" "{{ key }}" "{{ value }}"

# Return true when design.md exists in the directory and has non-whitespace content.
_has_design dir:
    #!/usr/bin/env bash
    set -euo pipefail
    file="{{ dir }}/design.md"
    if [[ -f "${file}" ]] && grep -q '[^[:space:]]' "${file}"; then
        echo true
    else
        echo false
    fi

# Return true when plan.md exists in the directory and has non-whitespace content.
_has_plan dir:
    #!/usr/bin/env bash
    set -euo pipefail
    file="{{ dir }}/plan.md"
    if [[ -f "${file}" ]] && grep -q '[^[:space:]]' "${file}"; then
        echo true
    else
        echo false
    fi

# Read the current flow iteration from spec frontmatter, defaulting to 0.
_get_flow_iteration spec="spec.md":
    #!/usr/bin/env bash
    set -euo pipefail
    value="$(md get --raw "{{ spec }}" flow_iteration 2>/dev/null || true)"
    if [[ "${value}" =~ ^[0-9]+$ ]]; then
        echo "${value}"
    else
        echo 0
    fi

# Run the feature lifecycle flow through interactive setup and clarification.
flow filter="":
    #!/usr/bin/env bash
    set -euo pipefail

    red=$'\033[31m'
    green=$'\033[32m'
    yellow=$'\033[33m'
    blue=$'\033[34m'
    bold=$'\033[1m'
    reset=$'\033[0m'

    require_command() {
        local cmd="$1"
        if ! command -v "${cmd}" >/dev/null 2>&1; then
            printf '%sERROR:%s required command `%s` was not found.\n' "${red}${bold}" "${reset}" "${cmd}" >&2
            exit 1
        fi
    }

    read_config() {
        md get --raw "$1" "$2" 2>/dev/null || true
    }

    write_config() {
        md set --save "$1" "$2" "$3" >/dev/null
    }

    choose_from() {
        local label="$1"
        shift
        printf '%s\n' "$@" | fzf --height=15 --border --border-label "${label}"
    }

    available_agents() {
        local all=(claude codex opencode gemini qwen roo kimi)
        local found=()
        local agent
        for agent in "${all[@]}"; do
            if command -v "${agent}" >/dev/null 2>&1; then
                found+=("${agent}")
            fi
        done
        if [[ ${#found[@]} -eq 0 ]]; then
            printf '%s\n' "${all[@]}"
        else
            printf '%s\n' "${found[@]}"
        fi
    }

    choose_agent() {
        local stage="$1"
        mapfile -t agents < <(available_agents)
        choose_from "Choose ${stage} agent" "${agents[@]}"
    }

    choose_model() {
        local stage="$1"
        if command -v opencode >/dev/null 2>&1; then
            mapfile -t models < <(opencode models 2>/dev/null | sed '/^[[:space:]]*$/d' || true)
        else
            models=()
        fi
        if [[ ${#models[@]} -eq 0 ]]; then
            models=(
                'zai-coding-plan/glm-5.1'
                'minimax/MiniMax-M2.7-highspeed'
                'openrouter/qwen/qwen3.6-plus'
                'openrouter/qwen/qwen3-max'
                'kimi-for-coding/k2p6'
            )
        fi
        choose_from "Choose ${stage} Opencode model" "${models[@]}"
    }

    resolve_spec() {
        local filter="$1"
        local roots=()
        [[ -d claudine/features ]] && roots+=(claudine/features)
        [[ -d claudine/fixes ]] && roots+=(claudine/fixes)
        [[ -d features ]] && roots+=(features)
        [[ -d fixes ]] && roots+=(fixes)
        if [[ ${#roots[@]} -eq 0 ]]; then
            printf '%sERROR:%s no feature or fix directories were found.\n' "${red}${bold}" "${reset}" >&2
            exit 1
        fi

        if command -v fd >/dev/null 2>&1; then
            mapfile -t files < <(fd -g 'spec.md' "${roots[@]}" --exclude '_completed' --exclude '_unscheduled' | sort)
        else
            mapfile -t files < <(find "${roots[@]}" -path '*/_completed/*' -prune -o -path '*/_unscheduled/*' -prune -o -name spec.md -print | sort)
        fi

        if [[ -n "${filter}" ]]; then
            mapfile -t files < <(printf '%s\n' "${files[@]}" | rg --fixed-strings "${filter}" || true)
        fi

        case "${#files[@]}" in
            0)
                printf '%sERROR:%s no matching spec.md files found.\n' "${red}${bold}" "${reset}" >&2
                exit 1
                ;;
            1)
                printf '%s\n' "${files[0]}"
                ;;
            *)
                require_command fzf
                printf '%s\n' "${files[@]}" | fzf --height=15 --border --border-label 'Choose feature spec'
                ;;
        esac
    }

    spec_body_has_content() {
        awk '
            BEGIN { in_fm = 0; seen_fm = 0; body = 0 }
            NR == 1 && $0 == "---" { in_fm = 1; seen_fm = 1; next }
            in_fm && $0 == "---" { in_fm = 0; body = 1; next }
            !seen_fm { body = 1 }
            body && $0 ~ /[^[:space:]]/ { found = 1 }
            END { exit found ? 0 : 1 }
        ' "$1"
    }

    ensure_stage_config() {
        local spec="$1"
        local stage="$2"
        local agent_key="$3"
        local model_key="$4"
        local agent
        local model

        agent="$(read_config "${spec}" "${agent_key}")"
        if [[ -z "${agent}" || "${agent}" == "null" ]]; then
            require_command fzf
            agent="$(choose_agent "${stage}")"
            write_config "${spec}" "${agent_key}" "${agent}"
            printf '%s%s%s agent: %s\n' "${blue}" "${stage}" "${reset}" "${agent}"
        fi

        if [[ "${agent}" == "opencode" ]]; then
            model="$(read_config "${spec}" "${model_key}")"
            if [[ -z "${model}" || "${model}" == "null" ]]; then
                require_command fzf
                model="$(choose_model "${stage}")"
                write_config "${spec}" "${model_key}" "${model}"
                printf '%s%s%s model: %s\n' "${blue}" "${stage}" "${reset}" "${model}"
            fi
        fi
    }

    require_command md
    require_command claudine

    spec="$(resolve_spec "{{ filter }}")"
    feature_dir="$(dirname "${spec}")"
    abs_spec="$(cd "$(dirname "${spec}")" && pwd)/$(basename "${spec}")"

    printf '\n%sFlow%s %s\n' "${bold}" "${reset}" "${feature_dir}"

    ensure_stage_config "${spec}" "Clarify" clarify_agent clarify_model
    ensure_stage_config "${spec}" "Design" design_agent design_model
    ensure_stage_config "${spec}" "Plan" planning_agent planning_model
    ensure_stage_config "${spec}" "Implement" implementation_agent implementation_model
    ensure_stage_config "${spec}" "Review" review_agent review_model

    if ! spec_body_has_content "${spec}"; then
        printf '\n%sERROR:%s %s has no non-empty requirements body.\n' "${red}${bold}" "${reset}" "${spec}" >&2
        printf 'Add the feature requirements below the frontmatter before running the flow.\n' >&2
        exit 1
    fi

    clarify_agent="$(read_config "${spec}" clarify_agent)"
    clarify_model="$(read_config "${spec}" clarify_model)"
    model_args=()
    if [[ "${clarify_agent}" == "opencode" && -n "${clarify_model}" && "${clarify_model}" != "null" ]]; then
        model_args=(--model "${clarify_model}")
    fi

    printf '\n%sClarify%s launching %s for %s\n' "${green}${bold}" "${reset}" "${clarify_agent}" "${spec}"
    claudine compose @prompts/clarify.md "--${clarify_agent}" -y -i "doc=${abs_spec}" "${model_args[@]}"

    printf '\n%sPhase 1 complete -> entering autonomous mode%s\n' "${green}${bold}" "${reset}"
