set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

import "./util.just"

# Read a frontmatter value from the feature spec.
_read_config key spec="spec.md":
    #!/usr/bin/env bash
    set -euo pipefail
    value="$(md get --raw "{{spec}}" "{{key}}" 2>/dev/null || true)"
    printf '%s\n' "${value}"

# Write a frontmatter value to the feature spec.
_write_config key value spec="spec.md":
    #!/usr/bin/env bash
    set -euo pipefail
    md set --save "{{spec}}" "{{key}}" "{{value}}"

# Return true when design.md exists in the directory and has non-whitespace content.
_has_design dir:
    #!/usr/bin/env bash
    set -euo pipefail
    file="{{dir}}/design.md"
    if [[ -f "${file}" ]] && grep -q '[^[:space:]]' "${file}"; then
        echo true
    else
        echo false
    fi

# Return true when plan.md exists in the directory and has non-whitespace content.
_has_plan dir:
    #!/usr/bin/env bash
    set -euo pipefail
    file="{{dir}}/plan.md"
    if [[ -f "${file}" ]] && grep -q '[^[:space:]]' "${file}"; then
        echo true
    else
        echo false
    fi

# Read the current flow iteration from spec frontmatter, defaulting to 0.
_get_flow_iteration spec="spec.md":
    #!/usr/bin/env bash
    set -euo pipefail
    value="$(md get --raw "{{spec}}" flow_iteration 2>/dev/null || true)"
    if [[ "${value}" =~ ^[0-9]+$ ]]; then
        echo "${value}"
    else
        echo 0
    fi
