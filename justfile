set dotenv-load
set positional-arguments
set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

# List of areas in this monorepo
areas := "biscuit-hash biscuit-speaks biscuit-terminal schematic biscuit-file unchained-ai playa so-you-say tree-hugger darkmatter sniff model-citizen claudine research queue homelab"

BOLD := '\033[1m'
DIM := '\033[2m'
ITALIC := '\033[3m'
RESET := '\033[0m'
RED := '\033[31m'

default:
    #!/usr/bin/env bash
    set -euo pipefail

    if command -v md &> /dev/null; then
        md just.md
    else
        echo "Rusty Biscuit Monorepo"
        echo "======================"
    fi
    echo ""
    just --list | grep -v 'default'
    echo

_notify message title="Rusty Biscuit":
    #!/usr/bin/env bash
    if command -v osascript &> /dev/null; then
        osascript -e 'display notification "message" with title "{{title}}"'
    elif command -v notify-send &> /dev/null; then
        notify-send '{{message}}'
    fi

# play a sound effect if `playa` CLI is installed (never errors)
_play effect background="":
    @playa "{{effect}}" "{{background}}" >/dev/null 2>&1 || exit 0

# start Claude Code CLI in yolo mode
cc *args="":
    @claudine claude --yolo {{args}}

# Open codex in YOLO mode
codex *args="":
    @claudine codex --yolo {{args}}

# Open Claude Code
gem *args="":
    @claudine gemini --yolo --include GEMINI_API_KEY {{args}}

# start Opencode CLI
oc *args="":
	@claudine opencode --include GEMINI_API_KEY --include X_AI_API_KEY --include ZAI_API_KEY --include ZENMUX_API_KEY --include OPEN_ROUTER_API_KEY {{args}}

# Uses TTS if available but never error
_speak *args:
    @so-you-say "{{args}}" --background >/dev/null 2>&1 || exit 0



_ask_opencode prompt:
    #!/usr/bin/env bash
    if command -v opencode &> /dev/null; then
        local MODEL
        MODEL="${MODEL:-minimax/MiniMax-M2.5-highspeed}"
        echo
        echo -e "{{DIM}}{{ITALIC}}- {{RESET}}Note: {{DIM}}using the {{BOLD}}${MODEL}{{DIM}} model with {{BOLD}}OpenCode CLI{{DIM}}.{{RESET}}"
        echo -e "{{DIM}}{{ITALIC}}- you can choose a different model by setting the {{BOLD}}MODEL{{DIM}} env variable"
        echo
        opencode run "{{prompt}}" --model "$MODEL"  || (
            echo
            echo -e "Attempt to use {{RED}}{{BOLD}}OpenCode{{RESET}} for query failed! Will try using Claude instead"
            echo
            just _ask_claude "{{prompt}}"
        )
    else
        echo
        echo -e "{{RED}}{{BOLD}}ERROR:{{RESET}} the attempt to ask the {{BOLD}}OpenCode{{RESET}} CLI is not possible as the host system does not have it installed!"
        echo
        exit 1
    fi

_ask_claude prompt:
    #!/usr/bin/env bash
    if command -v claude &> /dev/null; then
        echo
        echo -e "{{DIM}}{{ITALIC}}- {{RESET}}Note: {{DIM}}{{ITALIC}}using {{BOLD}}Claude Code{{DIM}} with whatever is setup as the default model.{{RESET}}"
        echo

        unset ANTHROPIC_API_KEY && claude -p "{{prompt}}"
    else
        echo
        echo -e "{{RED}}{{BOLD}}ERROR:{{RESET}} the attempt to ask the {{BOLD}}Claude{{RESET}} CLI is not possible as the host system does not have it installed!"
        echo
        exit 1
    fi

_ask_codex prompt:
    #!/usr/bin/env bash
    if command -v codex &> /dev/null; then
        echo
        echo -e "{{DIM}}{{ITALIC}}- {{RESET}}Note: {{DIM}}{{ITALIC}}using {{BOLD}}Codex CLI{{DIM}} with whatever is setup as the default model.{{RESET}}"
        echo

        codex exec "{{prompt}}"  || (
            echo
            echo -e "Attempt to use {{RED}}{{BOLD}}Codex CLI{{RESET}} for query failed! Will try using Claude instead"
            echo
            just _ask_claude "{{prompt}}"
        )
    else
        echo
        echo -e "{{RED}}{{BOLD}}ERROR:{{RESET}} the attempt to ask the {{BOLD}}Codex{{RESET}} CLI is not possible as the host system does not have it installed!"
        echo
        exit 1
    fi


# use an AI agent to respond to a question (uses `AGENT`)
ask prompt:
    #!/usr/bin/env bash
    if [[ "${AGENT,,}" == "opencode" ]]; then
        just _ask_opencode "{{prompt}}" || ( echo -e "- OpenCode failed so will try using Claude Code instead ..." && just _ask_claude "{{prompt}}" || exit 1 )
    elif [[ "${AGENT,,}" == "codex"  ]]; then
        just _ask_codex "{{prompt}}" || ( echo -e "- Codex failed so will try using Claude Code instead ..." && just _ask_claude "{{prompt}}" || exit 1 )
    else
        just _ask_claude "{{prompt}}" || ( echo -e "- will try using Opencode instead ..." && just _ask_opencode "{{prompt}}" || exit 1 )
    fi

# Ask Claude Code in a non-interactive session
_claude PROMPT:
    @unset ANTHROPIC_API_KEY && cd .. && claude --dangerously-skip-permissions -p \'{{PROMPT}}\'

# Ask Claude Code via a prompt file in a non-interactive session
_claude_file PROMPT_FILE:
    @unset ANTHROPIC_API_KEY && cd .. && claude --dangerously-skip-permissions -p "$(cat {{PROMPT_FILE}})"

modules:
  @cargo modules structure

# build all areas that have a build target
build *args="":
    #!/usr/bin/env bash
    set -euo pipefail
    echo ""
    echo "Building all areas..."
    echo "---------------------"
    echo ""
    for area in {{areas}}; do
        if [ -f "$area/justfile" ]; then
            if just -f "$area/justfile" --summary 2>/dev/null | grep -qw "build"; then
                echo "Building $area..."
                just -f "$area/justfile" build {{args}} || ( playa effect error-2 2>/dev/null && so-you-say "The ${area} package failed to build" 2>/dev/null && exit 1 )
            else
                echo "- no BUILD command for the area **$area**" >&2
            fi
        else
            echo "- no justfile for the area **$area**" >&2
        fi
    done

# test all areas that have a test target
test *args="":
    #!/usr/bin/env bash
    set -euo pipefail
    echo ""
    echo "Testing all areas..."
    echo "--------------------"
    echo ""
    for area in {{areas}}; do
        if [ -f "$area/justfile" ]; then
            if just -f "$area/justfile" --summary 2>/dev/null | grep -qw "test"; then
                echo "Testing $area..."
                just -f "$area/justfile" test {{args}}  || ( playa effect error-2 2>/dev/null && just _speak "The ${area} package had failed tests" && exit 1 )
            else
                echo "- no TEST command for the area **$area**" >&2
            fi
        else
            echo "- no justfile for the area **$area**" >&2
        fi
    done
    @playa crowd-applause-recital 2>/dev/null || exit 0

# run doctests (all workspace crates, or specific areas: just doctest claudine playa)
doctest *args="":
    #!/usr/bin/env bash
    set -euo pipefail
    if [[ -z "{{args}}" ]]; then
        echo ""
        echo "Running doctests for all workspace libraries..."
        echo "------------------------------------------------"
        echo ""
        cargo test --doc --workspace
    else
        IFS=', ' read -ra areas <<< "{{args}}"
        echo ""
        echo "Running doctests for: ${areas[*]}"
        echo "------------------------------------------------"
        echo ""
        all_pkgs=$(cargo metadata --no-deps --format-version 1 | jq -r '.packages[].name')
        pkg_args=""
        for area in "${areas[@]}"; do
            if grep -qx "$area" <<< "$all_pkgs"; then
                pkg_args="$pkg_args -p $area"
            else
                matched=$(echo "$all_pkgs" | grep "^${area}-" || true)
                if [[ -n "$matched" ]]; then
                    while IFS= read -r pkg; do
                        pkg_args="$pkg_args -p $pkg"
                    done <<< "$matched"
                else
                    echo "Warning: no packages found for area '$area', skipping"
                fi
            fi
        done
        cargo test --doc $pkg_args
    fi

# install the Claudine CLI
install_claudine:
    @just -f claudine/justfile install

# install the Claudine CLI
install_darkmatter:
    @just -f claudine/darkmatter install

# install binaries from all areas that have an install target
install:
    #!/usr/bin/env bash
    set -euo pipefail
    echo ""
    echo "Installing from all areas..."
    echo "----------------------------"
    echo ""
    for area in {{areas}}; do
        if [ -f "$area/justfile" ]; then
            if just -f "$area/justfile" --summary 2>/dev/null | grep -qw "install"; then
                echo
                echo "Installing from $area..."
                just -f "$area/justfile" install || ( just _speak "The ${area} package failed during an attempt to install all packages!" && exit 1 )
            else
                if just -f "$area/justfile" --summary 2>/dev/null | grep -qw "build"; then
                    echo
                    echo "No INSTALL command for $area, doing release build..."
                    just -f "$area/justfile" build --release || ( just _speak "The ${area} package failed to build while attempting a install on all packages." && exit 1 )
                else
                    echo
                    echo "- no INSTALL command for the area **$area**" >&2
                fi
            fi
        else
            echo "- no justfile for the area **$area**" >&2
        fi
    done
    just _speak "all apps in the Rusty Biscuit monorepo have been rebuilt and installed"


# executes the latest Darkmatter CLI code in debug mode
md *args="":
    #!/usr/bin/env bash
    set -euo pipefail
    echo ""
    echo -e "{{BOLD}}Darkmatter CLI{{RESET}} (latest debug build)"
    echo -e "----------------------------------------------------"
    cargo run -p darkmatter-cli --bin md -- {{args}}

# executes the latest Research CLI code in debug mode
research *args="":
    #!/usr/bin/env bash
    set -euo pipefail
    echo ""
    echo -e "{{BOLD}}Research CLI{{RESET}} (latest debug build)"
    echo "----------------------------------------------"
    cargo run -p research-cli -- {{args}}

# generate provider model enums from APIs
gen-models *args="":
    #!/usr/bin/env bash
    set -euo pipefail
    echo ""
    echo -e "{{BOLD}}Gen Models{{RESET}} (latest debug build)"
    echo "---------------------------------"
    cargo run -p unchained-ai-gen -- {{args}}

# generate models for a specific provider
gen-models-for provider:
    @cargo run -p unchained-ai-gen -- --providers {{provider}}

# show the Documentation for crates.io for the Darkmatter package
darkmatter-docs:
    @cargo clean --doc && cargo doc --no-deps -p darkmatter --lib --open

# check what release-plz would do (dry run)
release-check:
    @release-plz update --dry-run

# generate/update changelogs locally (without releasing)
release-update:
    @release-plz update

# install release-plz CLI locally
install-release-plz:
    @cargo install release-plz --locked

# run the latest debug build of the `sniff` CLI
sniff *args="":
    @cargo run -p sniff-cli -- {{args}}

# show workspace package dependencies
repo-deps:
    @cargo run --manifest-path scripts/Cargo.toml --bin repo-deps

lint:
  #!/usr/bin/env bash
    set -euo pipefail
    echo ""
    echo "Linting all packages..."
    echo "----------------------------"
    echo ""
    for area in {{areas}}; do
        if [ -f "$area/justfile" ]; then
            if just -f "$area/justfile" --summary 2>/dev/null | grep -qw "lint"; then
                echo "Linting $area..."
                just -f "$area/justfile" lint || ( so-you-say "The ${area} package has lint errors." )
            else
                if just -f "$area/justfile" --summary 2>/dev/null | grep -qw "lint"; then
                    echo "No lint command for $area"
                    so-you-say "The ${area} package does not define a lint command" 2>/dev/null || exit 0
                else
                    echo "- no lint command for the area **$area**" >&2
                fi
            fi
        else
            echo -e "- no {{ITALIC}}justfile{{RESET}} for the package {{BOLD}}$area{{RESET}}" >&2

        fi
    done

# commits all the staged changes using model from MODEL or COMMIT_MODEL in OpenCode
commit:
    @echo ""
    @echo -e "Committing staged changes in the {{BOLD}}Rusty Biscuit{{RESET}} monorepo to git"
    @echo -e "{{DIM}}{{ITALIC}}- using the {{RESET}}{{ITALIC}}${MODEL:-${COMMIT_MODEL:-minimax/MiniMax-M2.7-highspeed}} {{DIM}}model{{RESET}}"
    @echo ""
    @echo -e "{{BOLD}}{{BLUE}}Staged Files:{{RESET}}"
    @sniff repo staged-files || ( echo "No Staged Files! Nothing to do ..." && exit 1 )
    @claudine compose "@prompts/commit.md" --opencode --op "commit" --quiet --model "${COMMIT_MODEL:-${MODEL:-minimax/MiniMax-M2.7-highspeed}}"
    @just _speak "git commits completed in rusty-biscuit monorepo"
    @sniff repo git-status 2>/dev/null || exit 0
    @echo

# stages all files in package area and then commits and pushes
cp:
    @echo ""
    @echo "Staging all {{BOLD}}modified{{RESET}} or {{BOLD}}untracked{{RESET}} files across the {{RED}}rusty-biscuit{{RESET}} monorepo."
    @echo ""
    @git add . ||
    @echo ""
    @echo "Files have been added"
    @echo ""
    @just commit
    @echo ""
    @git push
    @echo ""
    @just _play select-4
    @echo "All committed files from {{BOLD}}rusty-biscuit{{RESET}} monorepo have now been pushed to remote."
    @echo

# install rusty-biscuit CLI's which are used in devops
init: _ensure-build-deps
    #!/usr/bin/env bash
    set -euo pipefail
    # Source cargo env in case _ensure-build-deps just installed Rust
    [[ -f "$HOME/.cargo/env" ]] && source "$HOME/.cargo/env"
    echo -e "Initializing the {{RED}}rusty-biscuit{{RESET}} monorepo"
    echo
    echo -e "First step is to ensure CLI's used for devops are installed"
    echo
    (cd biscuit-terminal && just install)
    (cd darkmatter && just install)
    (cd sniff && just install)
    (cd playa && just install)
    (cd biscuit-speaks && just install)

# ensure Rust, cargo, and C build tools are available
_ensure-build-deps:
    #!/usr/bin/env bash
    set -euo pipefail

    # Check for Rust and cargo
    if ! command -v rustc &> /dev/null || ! command -v cargo &> /dev/null; then
        echo -e "{{RED}}Missing Rust toolchain{{RESET}} (rustc/cargo not found)"
        echo "Installing Rust via rustup..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        source "$HOME/.cargo/env"
        echo "Rust toolchain installed."
    fi

    # Check for C compiler/linker
    if command -v cc &> /dev/null; then
        exit 0
    fi
    echo -e "{{RED}}Missing build dependencies{{RESET}} (cc linker not found)"
    echo "Installing build essentials..."
    if command -v apt-get &> /dev/null; then
        sudo apt-get update -qq && sudo apt-get install -y -qq build-essential pkg-config libssl-dev
    elif command -v dnf &> /dev/null; then
        sudo dnf install -y gcc gcc-c++ make pkg-config openssl-devel
    elif command -v pacman &> /dev/null; then
        sudo pacman -S --noconfirm base-devel pkg-config openssl
    elif command -v apk &> /dev/null; then
        sudo apk add build-base pkgconf openssl-dev
    else
        echo "Could not detect package manager. Please install a C compiler (gcc/clang) manually."
        exit 1
    fi
    echo "Build dependencies installed."

# sync a just recipe from one justfile to all others that have it
sync-recipe recipe source:
    @./scripts/sync-recipe.sh "{{recipe}}" "{{source}}"
