

# ask AI via Claudine
ask prompt agent="claude":
    #!/usr/bin/env bash
    set -euo pipefail
    claudine {{agent}} "{{prompt}}" --silent 2>/dev/null

# Open Opencode
oc *args="":
    @claudine opencode --include GEMINI_API_KEY --include X_AI_API_KEY --include ZAI_API_KEY --include ZENMUX_API_KEY --include OPEN_ROUTER_API_KEY {{args}}

# Open Claude Code in YOLO mode
cc *args="":
    @claudine claude --yolo {{args}}

# Open codex in YOLO mode
codex *args="":
    @claudine codex --yolo {{args}}

# Open Claude Code
gem *args="":
    @claudine gemini --yolo --include GEMINI_API_KEY {{args}}
