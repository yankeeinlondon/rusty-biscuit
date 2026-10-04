#!/usr/bin/env zsh
# Multi-file fixture: Zsh.

typeset -g PROMPT_STYLE="plain"

render_prompt() {
    local style="$1"
    print -- "${style:-$PROMPT_STYLE}"
}

function reset_prompt {
    PROMPT_STYLE="plain"
}
