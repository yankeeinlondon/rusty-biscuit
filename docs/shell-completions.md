# Shell Completions

If you want to add a conditional block to your `.zshrc` or `.bashrc` that will conditionally add all the CLI's from this monorepo which you have installed on your computer you can add the following:

```bash
# has_command <cmd>
#
# checks whether a particular program passed in via $1 is installed
# on the OS or not (at least within the $PATH)
function has_command() {
    local -r cmd="${1:?cmd is missing}"

    if command -v "${cmd}" &> /dev/null; then
        return 0
    else
        return 1
    fi
}

# colorize <content>
#
# Looks for tags which represent formatting instructions -- `{{RED}}`, `{{RESET}}`,
# etc. -- and converts them using a variable of the same name.
colorize() {
    local -r content="${*:-}"
    local rest="$content"
    local result=""
    local tag

    while [[ "$rest" == *"{{"* ]]; do
        result+="${rest%%\{\{*}"
        rest="${rest#*\{\{}"

        if [[ "$rest" != *"}}"* ]]; then
            result+="{{${rest}"
            rest=""
            break
        fi

        tag="${rest%%\}\}*}"
        rest="${rest#*\}\}}"

        # Ensure tag is a valid variable name to prevent injection/errors
        if [[ ! "$tag" =~ ^[a-zA-Z_][a-zA-Z0-9_]*$ ]]; then
            result+="{{${tag}}}"
            continue
        fi

        if [ -n "${ZSH_VERSION:-}" ]; then
            if (( ${+parameters[$tag]} )); then
                # shellcheck disable=SC2296
                result+="${(P)tag}"
            else
                result+="{{${tag}}}"
            fi
        else
            # Bash: Use eval to safely handle indirection and avoid Zsh parsing errors
            if eval "[[ \${!tag+x} ]]"; then
                local val
                eval 'val="${!tag}"'
                result+="$val"
            else
                result+="{{${tag}}}"
            fi
        fi
    done

    result+="$rest"

    printf '%s' "$result"
}

function setup_colors() {
    export AD_COLORS_SETUP="true"
    export BLACK=$'\033[30m'
    export RED=$'\033[31m'
    export GREEN=$'\033[32m'
    export YELLOW=$'\033[33m'
    export BLUE=$'\033[34m'
    export MAGENTA=$'\033[35m'
    export CYAN=$'\033[36m'
    export WHITE=$'\033[37m'

    export BRIGHT_BLACK=$'\033[90m'
    export BRIGHT_RED=$'\033[91m'
    export BRIGHT_GREEN=$'\033[92m'
    export BRIGHT_YELLOW=$'\033[93m'
    export BRIGHT_BLUE=$'\033[94m'
    export BRIGHT_MAGENTA=$'\033[95m'
    export BRIGHT_CYAN=$'\033[96m'
    export BRIGHT_WHITE=$'\033[97m'

    # font weights
    export BOLD=$'\033[1m'
    export NORMAL=$'\033[22m'
    export DIM=$'\033[2m'

    export ITALIC=$'\033[3m'
    export NO_ITALIC=$'\033[23m'
    export STRIKE=$'\033[9m'
    export NO_STRIKE=$'\033[29m'
    export REVERSE=$'\033[7m'
    export NO_REVERSE=$'\033[27m'
    export UNDERLINE=$'\033[4m'
    export NO_UNDERLINE=$'\033[24m'
    export BLINK=$'\033[5m'
    export NO_BLINK=$'\033[25m'

    export BG_BLACK=$'\033[40m'
    export BG_RED=$'\033[41m'
    export BG_GREEN=$'\033[42m'
    export BG_YELLOW=$'\033[43m'
    export BG_BLUE=$'\033[44m'
    export BG_MAGENTA=$'\033[45m'
    export BG_CYAN=$'\033[46m'
    export BG_WHITE=$'\033[47m'

    export BG_BRIGHT_BLACK=$'\033[100m'
    export BG_BRIGHT_RED=$'\033[101m'
    export BG_BRIGHT_GREEN=$'\033[102m'
    export BG_BRIGHT_YELLOW=$'\033[103m'
    export BG_BRIGHT_BLUE=$'\033[104m'
    export BG_BRIGHT_MAGENTA=$'\033[105m'
    export BG_BRIGHT_CYAN=$'\033[106m'
    export BG_BRIGHT_WHITE=$'\033[107m'

    export RESET=$'\033[0m' # RESET ALL ATTRIBUTES
    export DEF_FG=$'\033[39m' # RESET the Foreground Color
    export DEF_BG=$'\033[49m' # RESET the Background Color
    export DEF_COLOR="${DEFAULT_FG}${DEFAULT_BG}" # RESET Foreground and Background

    export SAVE_POSITION=$'\033[s'
    export RESTORE_POSITION=$'\033[u'
    export CLEAR_SCREEN=$'\033[2J'
}

# logc <content> <content> <...>
#
# Logs all passed parameters to STDERR.
#   - unlike `log` function this will run the
#   - parameters through `colorize()` to that
#   the caller doesn't need to bother with the
function logc() {
    setup_colors

    local reset="${RESET:-}"
    [[ -z "$reset" ]] && reset=$'\033[0m'

    content="$(colorize "${*}")"
    printf "%b\\n" "${content}${reset}" >&2

    remove_colors
}

if has_command "just"; then

    if is_zsh; then
        if file_exists "${HOME}/.zsh/completion/_just"; then
            logc "- {{BOLD}}just{{RESET}} completions loaded"
        else
            logc "- {{ITALIC}}adding {{RESET}}{{BOLD}}just{{RESET}} completions"
            just --completions zsh > "${HOME}/.zsh/completion/_just"
            add_to_fpath "_just"
            ensure_autoload
        fi
    elif is_bash; then
        if file_exists "${HOME}/.local/share/bash-completion/completions"; then
            if file_contains "${HOME}/.local/share/bash-completion/completions" "_just() {"; then
                logc "- {{BOLD}}just{{RESET}} completions loaded"
            else
                logc "- {{ITALIC}}adding {{RESET}}{{BOLD}}just{{RESET}} completions"
                just --completions bash >> "${HOME}/.local/share/bash-completion/completions"
            fi
        fi
    fi
fi

if has_command "hug"; then

    if is_zsh; then
        if file_exists "${HOME}/.zsh/completion/_hug"; then
            logc "- {{BOLD}}hug{{RESET}} ({{DIM}}{{ITALIC}}tree-hugger{{RESET}}) completions loaded"
        else
            logc "- {{ITALIC}}adding {{RESET}}{{BOLD}}hug{{RESET}} ({{DIM}}{{ITALIC}}tree-hugger{{RESET}}) completions"
            hug completions zsh > "${HOME}/.zsh/completion/_hug"
            add_to_fpath "_hug"
            ensure_autoload
        fi
    elif is_bash; then
        if file_exists "${HOME}/.local/share/bash-completion/completions"; then
            if file_contains "${HOME}/.local/share/bash-completion/completions" "_hug() {"; then
                logc "- {{BOLD}}hug{{RESET}} completions loaded"
            else
                logc "- {{ITALIC}}adding {{RESET}}{{BOLD}}hug{{RESET}} ({{DIM}}{{ITALIC}}tree-hugger{{RESET}}) completions"
                hug completions bash >> "${HOME}/.local/share/bash-completion/completions"
            fi
        fi
    fi
fi
if has_command "homey"; then

    if is_zsh; then
        source <(COMPLETE=zsh homey)
        logc "- {{BOLD}}homey{{RESET}} ({{DIM}}{{ITALIC}}homelab{{RESET}}) completions loaded"
    elif is_bash; then
        source <(COMPLETE=bash homey)
        logc "- {{BOLD}}homey{{RESET}} ({{DIM}}{{ITALIC}}homelab{{RESET}}) completions loaded"
    fi
fi

if has_command "messenger"; then

    if is_zsh; then
        source <(COMPLETE=zsh messenger)
        logc "- {{BOLD}}messenger{{RESET}} completions loaded"
    elif is_bash; then
        source <(COMPLETE=bash messenger)
        logc "- {{BOLD}}messenger{{RESET}} completions loaded"
    fi
fi

if has_command "md"; then

    if is_zsh; then
        source <(COMPLETE=zsh md)
        logc "- {{BOLD}}md{{RESET}} ({{DIM}}{{ITALIC}}darkmatter{{RESET}}) completions loaded"
    elif is_bash; then
        source <(COMPLETE=bash md)
        logc "- {{BOLD}}md{{RESET}} ({{DIM}}{{ITALIC}}darkmatter{{RESET}}) completions loaded"
    fi
fi

if has_command "bt"; then

    if is_zsh; then
        source <(COMPLETE=zsh bt)
        logc "- {{BOLD}}bt{{RESET}} ({{DIM}}{{ITALIC}}biscuit-terminal{{RESET}}) completions loaded"
    elif is_bash; then
        source <(COMPLETE=bash bt)
        logc "- {{BOLD}}bt{{RESET}} ({{DIM}}{{ITALIC}}biscuit-terminal{{RESET}}) completions loaded"
    fi
fi

if has_command "sniff"; then

    if is_zsh; then
        source <(COMPLETE=zsh sniff)
        logc "- {{BOLD}}sniff{{RESET}} completions loaded"
    elif is_bash; then
        source <(COMPLETE=bash sniff)
        logc "- {{BOLD}}sniff{{RESET}} completions loaded"
    fi
fi

if has_command "wt"; then

    if is_zsh; then
        source <(wt --completions zsh)
        logc "- {{BOLD}}wt{{RESET}} ({{DIM}}worktree{{RESET}}) completions loaded"
    elif is_bash; then
        source <(wt --completions bash)
        logc "- {{BOLD}}wt{{RESET}} ({{DIM}}worktree{{RESET}}) completions loaded"
    fi
fi

if has_command "playa"; then

    if is_zsh; then
        source <(COMPLETE=zsh playa)
        logc "- {{BOLD}}playa{{RESET}} completions loaded"
    elif is_bash; then
        source <(COMPLETE=bash playa)
        logc "- {{BOLD}}playa{{RESET}} completions loaded"
    fi
fi

if has_command "so-you-say"; then

    if is_zsh; then
        source <(COMPLETE=zsh so-you-say)
        logc "- {{BOLD}}so-you-say{{RESET}} ({{DIM}}{{ITALIC}}biscuit-speaks{{RESET}}) completions loaded"
    elif is_bash; then
        source <(COMPLETE=bash so-you-say)
        logc "- {{BOLD}}so-you-say{{RESET}} ({{DIM}}{{ITALIC}}biscuit-speaks{{RESET}}) completions loaded"
    fi
fi

if has_command "model"; then

    if is_zsh; then
        source <(COMPLETE=zsh model)
        logc "- {{BOLD}}model{{RESET}} ({{DIM}}{{ITALIC}}model-citizen{{RESET}}) completions loaded"
    elif is_bash; then
        source <(COMPLETE=bash model)
        logc "- {{BOLD}}model{{RESET}} ({{DIM}}{{ITALIC}}model-citizen{{RESET}}) completions loaded"
    fi
fi

if has_command "bh"; then

    if is_zsh; then
        source <(COMPLETE=zsh bh)
        logc "- {{BOLD}}bh{{RESET}} ({{DIM}}{{ITALIC}}biscuit-hash{{RESET}}) completions loaded"
    elif is_bash; then
        source <(COMPLETE=bash bh)
        logc "- {{BOLD}}bh{{RESET}} ({{DIM}}{{ITALIC}}biscuit-hash{{RESET}}) completions loaded"
    fi
fi

if has_command "claudine"; then

    if is_zsh; then
        source <(claudine completions zsh)
        logc "- {{BOLD}}claudine{{RESET}} completions loaded"
    elif is_bash; then
        source <(claudine completions bash)
        logc "- {{BOLD}}claudine{{RESET}} completions loaded"
    fi
fi
```

> Note: you can conserve a lot of characters by removing colors, I moved this code in from my base shell scripts and it had colors and ... i'm lazy.

