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
    set -Eeuo pipefail

    red=$'\033[31m'
    green=$'\033[32m'
    yellow=$'\033[33m'
    blue=$'\033[34m'
    bold=$'\033[1m'
    reset=$'\033[0m'
    spec=""
    feature_dir=""

    artifact_link() {
        local label="$1"
        local path="$2"
        local abs
        if [[ "${path}" = /* ]]; then
            abs="${path}"
        else
            abs="$(pwd)/${path}"
        fi
        printf '\033]8;;file://%s\033\\%s\033]8;;\033\\' "${abs}" "${label}"
    }

    status_line() {
        local stage="$1"
        local state="$2"
        local message="$3"
        printf '\n%s[%s]%s %s -- %s\n' "${blue}${bold}" "${stage}" "${reset}" "${state}" "${message}"
    }

    mark_progress() {
        local stage="$1"
        if [[ -n "${spec}" && -f "${spec}" ]]; then
            md set --save "${spec}" flow_stage "${stage}" >/dev/null 2>&1 || true
        fi
    }

    persist_flow_state() {
        if [[ -z "${spec}" || ! -f "${spec}" ]]; then
            return 0
        fi

        local iteration start_phase plan
        iteration="$(md get --raw "${spec}" flow_iteration 2>/dev/null || true)"
        [[ "${iteration}" =~ ^[0-9]+$ ]] || iteration=0
        md set --save "${spec}" flow_iteration "${iteration}" >/dev/null 2>&1 || true

        if [[ -n "${feature_dir}" ]]; then
            plan="${feature_dir}/plan.md"
            if [[ -f "${plan}" ]]; then
                start_phase="$(md get --raw "${plan}" start_phase 2>/dev/null || true)"
                [[ "${start_phase}" =~ ^[0-9]+$ ]] || start_phase=1
                md set --save "${spec}" flow_plan_start_phase "${start_phase}" >/dev/null 2>&1 || true
            fi
        fi
    }

    on_flow_error() {
        local status="$?"
        persist_flow_state
        printf '\n%sERROR:%s flow stopped; progress was saved to %s.\n' "${red}${bold}" "${reset}" "$(artifact_link spec.md "${spec:-spec.md}")" >&2
        exit "${status}"
    }

    trap on_flow_error ERR

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

    file_has_content() {
        local file="$1"
        [[ -f "${file}" ]] && grep -q '[^[:space:]]' "${file}"
    }

    model_args_for() {
        local agent="$1"
        local model="$2"
        if [[ "${agent}" == "opencode" && -n "${model}" && "${model}" != "null" ]]; then
            printf '%s\0%s\0' --model "${model}"
        fi
    }

    commit_flow_changes() {
        local label="$1"
        status_line Commit Running "${label}"
        git add .
        if git diff --cached --quiet; then
            status_line Commit Skipping "no staged changes after ${label}"
            return 0
        fi
        just commit
    }

    run_design_stage() {
        local spec="$1"
        local feature_dir="$2"
        local dir_arg="$3"
        local design_file="${feature_dir}/design.md"
        if file_has_content "${design_file}"; then
            status_line Design Skipping "$(artifact_link design.md "${design_file}") already has content."
            mark_progress design_complete
            return 0
        fi

        local agent model
        agent="$(read_config "${spec}" design_agent)"
        model="$(read_config "${spec}" design_model)"
        mapfile -d '' -t model_args < <(model_args_for "${agent}" "${model}")
        status_line Design Running "${agent} will write $(artifact_link design.md "${design_file}")."
        claudine compose @prompts/design.md "--${agent}" -y dir="${dir_arg}" spec="spec.md" design="design.md" "${model_args[@]}"
        mark_progress design_complete
    }

    run_plan_stage() {
        local spec="$1"
        local feature_dir="$2"
        local dir_arg="$3"
        local plan_file="${feature_dir}/plan.md"
        if file_has_content "${plan_file}"; then
            status_line Plan Skipping "$(artifact_link plan.md "${plan_file}") already has content."
            mark_progress plan_complete
            return 0
        fi

        local agent model
        agent="$(read_config "${spec}" planning_agent)"
        model="$(read_config "${spec}" planning_model)"
        mapfile -d '' -t model_args < <(model_args_for "${agent}" "${model}")
        status_line Plan Running "${agent} will write $(artifact_link plan.md "${plan_file}")."
        claudine compose @prompts/plan.md "--${agent}" -y dir="${dir_arg}" spec="spec.md" design="design.md" plan="plan.md" "${model_args[@]}"
        mark_progress plan_complete
    }

    run_implementation_plan() {
        local spec="$1"
        local feature_dir="$2"
        local plan="${feature_dir}/plan.md"
        local agent model phases current total memory
        if [[ ! -f "${plan}" ]]; then
            printf '\n%sERROR:%s plan file is missing: %s\n' "${red}${bold}" "${reset}" "${plan}" >&2
            exit 1
        fi

        phases="$(md get --raw "${plan}" phases 2>/dev/null || true)"
        if [[ ! "${phases}" =~ ^[0-9]+$ ]] || (( phases < 1 )); then
            printf '\n%sERROR:%s %s is missing numeric phases frontmatter.\n' "${red}${bold}" "${reset}" "${plan}" >&2
            exit 1
        fi
        total="${phases}"

        current="$(md get --raw "${plan}" start_phase 2>/dev/null || true)"
        [[ "${current}" =~ ^[0-9]+$ ]] || current=1
        if (( current > total )); then
            status_line Implement Skipping "$(artifact_link plan.md "${plan}") is already complete."
            mark_progress implementation_complete
            return 0
        fi

        memory="$(md get --raw "${plan}" memory 2>/dev/null || true)"
        agent="$(read_config "${spec}" implementation_agent)"
        model="$(read_config "${spec}" implementation_model)"
        mapfile -d '' -t model_args < <(model_args_for "${agent}" "${model}")

        status_line Implement Running "phases ${current} through ${total} from $(artifact_link plan.md "${plan}") with ${agent}."
        while (( current <= total )); do
            status_line Implement Running "phase ${current}/${total}"
            md set --save "${spec}" flow_plan_start_phase "${current}" >/dev/null
            claudine compose @prompts/implement-phase.md "--${agent}" -y plan="${plan}" phase="${current}" total_phases="${total}" memory="${memory}" "${model_args[@]}"
            (( current++ ))
            md set --save "${plan}" start_phase "${current}" >/dev/null
            md set --save "${spec}" flow_plan_start_phase "${current}" >/dev/null
        done
        mark_progress implementation_complete
    }

    run_review_loop() {
        local spec="$1"
        local feature_dir="$2"
        local dir_arg="$3"
        local agent model iteration review_number review ready
        agent="$(read_config "${spec}" review_agent)"
        model="$(read_config "${spec}" review_model)"
        mapfile -d '' -t review_model_args < <(model_args_for "${agent}" "${model}")

        while true; do
            iteration="$(md get --raw "${spec}" flow_iteration 2>/dev/null || true)"
            [[ "${iteration}" =~ ^[0-9]+$ ]] || iteration=0
            review_number=$((iteration + 1))
            review="${feature_dir}/review-${review_number}.md"

            status_line Review Running "iteration ${review_number} with ${agent}; output $(artifact_link "review-${review_number}.md" "${review}")."
            claudine compose @prompts/review-feature.md "--${agent}" -y dir="${dir_arg}" spec="spec.md" design="design.md" plan="plan.md" iteration="${review_number}" review="review-${review_number}.md" "${review_model_args[@]}"
            mark_progress "review_${review_number}_complete"

            ready="$(md get --raw "${review}" ready 2>/dev/null || true)"
            if [[ "${ready}" == "true" ]]; then
                mark_progress complete
                status_line Flow Complete "$(artifact_link "review-${review_number}.md" "${review}") marked ready."
                return 0
            fi

            if [[ "${ready}" != "false" ]]; then
                printf '\n%sERROR:%s %s did not set ready frontmatter to true or false.\n' "${red}${bold}" "${reset}" "${review}" >&2
                exit 1
            fi

            if (( iteration >= 5 )); then
                printf '\n%sERROR:%s review loop reached the maximum of 5 repair iterations.\n' "${red}${bold}" "${reset}" >&2
                exit 1
            fi

            write_config "${spec}" flow_iteration "${review_number}"
            repair_agent="$(read_config "${spec}" implementation_agent)"
            repair_model="$(read_config "${spec}" implementation_model)"
            mapfile -d '' -t repair_model_args < <(model_args_for "${repair_agent}" "${repair_model}")
            status_line "Review Repair" Running "applying suggestions from $(artifact_link "review-${review_number}.md" "${review}") with ${repair_agent}."
            claudine compose @prompts/implement-feature-review-suggestions.md "--${repair_agent}" -y dir="${dir_arg}" spec="spec.md" design="design.md" plan="plan.md" review="review-${review_number}.md" iteration="${review_number}" "${repair_model_args[@]}"
            mark_progress "review_${review_number}_repair_complete"
            commit_flow_changes "review iteration ${review_number}"
        done
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
                'zai-coding-plan/glm-5.2'
                'minimax-coding-plan/MiniMax-M2.7-highspeed'
                'minimax-coding-plan/MiniMax-M3'
                'kimi-for-coding/k2p7'
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

    status_line Flow Starting "$(artifact_link spec.md "${spec}")"

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

    status_line Clarify Running "launching ${clarify_agent} for $(artifact_link spec.md "${spec}")."
    claudine compose @prompts/clarify.md "--${clarify_agent}" -y -i "doc=${abs_spec}" "${model_args[@]}"
    mark_progress clarify_complete

    printf '\n%sPhase 1 complete -> entering autonomous mode%s\n' "${green}${bold}" "${reset}"

    dir_arg="${feature_dir#claudine/}"

    run_design_stage "${spec}" "${feature_dir}" "${dir_arg}"
    run_plan_stage "${spec}" "${feature_dir}" "${dir_arg}"
    run_implementation_plan "${spec}" "${feature_dir}"
    commit_flow_changes "implementation"
    run_review_loop "${spec}" "${feature_dir}" "${dir_arg}"
