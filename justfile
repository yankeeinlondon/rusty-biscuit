set dotenv-load
set positional-arguments

# List of areas in this monorepo
areas := "biscuit-hash biscuit-speaks biscuit-terminal schematic biscuit-file unchained-ai playa so-you-say tree-hugger darkmatter sniff model-citizen claudine research queue homelab"

BOLD := '\033[1m'
RESET := '\033[0m'

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

# start Claude Code CLI in yolo mode
cc *args="":
  @clear
  @echo "Starting Claude Code in Yolo mode"
  @echo
  @claude --dangerously-skip-permissions {{args}}

# start Opencode CLI
oc *args="":
  @clear
  @echo  "Starting Opencode"
  @echo
  @opencode {{args}}

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
                just -f "$area/justfile" test {{args}}  || ( playa effect error-2 2>/dev/null && so-you-say "The ${area} package had failed tests" 2>/dev/null && exit 1 )
            else
                echo "- no TEST command for the area **$area**" >&2
            fi
        else
            echo "- no justfile for the area **$area**" >&2
        fi
    done
    @playa crowd-applause-recital 2>/dev/null || exit 0

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
                echo "Installing from $area..."
                just -f "$area/justfile" install || ( so-you-say "The ${area} package failed during an attempt to install all packages!" && exit 1 )
            else
                if just -f "$area/justfile" --summary 2>/dev/null | grep -qw "build"; then
                    echo "No INSTALL command for $area, doing release build..."
                    just -f "$area/justfile" build --release || ( so-you-say "The ${area} package failed to build while attempting a install on all packages." && exit 1 )
                else
                    echo "- no INSTALL command for the area **$area**" >&2
                fi
            fi
        else
            echo "- no justfile for the area **$area**" >&2
        fi
    done
    if command -v so-you-say >/dev/null 2>&1; then \
        so-you-say "all apps in the monorepo have been rebuilt and installed"; \
    fi

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
                    so-you-say "The ${area} package does not define a lint command" 2>/dev/null || return 0
                else
                    echo "- no lint command for the area **$area**" >&2
                fi
            fi
        else
            echo "- no justfile for the area **$area**" >&2
        fi
    done

# Use GPT 5.2 to commit staged changes
commit:
    @echo "Using Opencode with {{BOLD}}gpt-5.2-codex{{RESET}} to {{BOLD}}commit{{RESET}} staged changes"
    @echo
    @opencode run "/commit -f ; remember you must use 'conventional commits' naming conventions and that unless all staged files are strongly related to the same thing be sure to make multiple commits by package or related activity." --model "openai/gpt-5.2-codex"
    @if command -v so-you-say >/dev/null 2>&1; then \
        so-you-say "git commits completed in rusty-biscuit monorepo"; \
    fi


# Update docs and then update the skill for the Schematic package
skill-schematic:
    @echo "Fixing documentation drift and rebuilding the skill for the {{BOLD}}Schematic{{RESET}} Package"
    @echo "-------------------------------------------------------------------------------"
    @unset ANTHROPIC_API_KEY && claude --dangerously-skip-permissions --model opus -p \'"$(cat docs/skills-schematic.md)"\'
    @echo ""
    @echo "Schematic Skill has been rebuilt"
    @echo ""
    @if command -v so-you-say >/dev/null 2>&1; then \
        so-you-say "The Schematic package has synced it's documents for drift and updated the skill tree."; \
    fi

# Update docs and then update the skill for the darkmatter package
skill-darkmatter:
    @echo "Fixing documentation drift and rebuilding the skill for the {{BOLD}}Darkmatter{{RESET}} Package"
    @echo "-------------------------------------------------------------------------------"
    @unset ANTHROPIC_API_KEY && claude --dangerously-skip-permissions --model opus -p \'"$(cat docs/skills-darkmatter.md)"\'
    @echo ""
    @echo "Darkmatter Skill has been rebuilt"
    @echo ""
    @if command -v so-you-say >/dev/null 2>&1; then \
        so-you-say "The darkmatter package has synced it's documents for drift and updated the skill tree."; \
    fi

# Update docs and then update the skill for the Sniff package
skill-sniff:
    @echo "Fixing documentation drift and rebuilding the skill for the {{BOLD}}Sniff{{RESET}} Package"
    @echo "-------------------------------------------------------------------------------"
    @unset ANTHROPIC_API_KEY && claude --dangerously-skip-permissions --model opus -p \'"$(cat docs/skills-sniff.md)"\'
    @echo ""
    @echo "Sniff skill has been rebuilt"
    @echo ""
    @if command -v so-you-say >/dev/null 2>&1; then \
        so-you-say "The Sniff package has synced it's documents for drift and updated the skill tree."; \
    fi
# Update docs and then update the skill for the Playa package
skill-playa:
    @echo "Fixing documentation drift and rebuilding the skill for the {{BOLD}}Playa{{RESET}} Package"
    @echo "-------------------------------------------------------------------------------"
    @unset ANTHROPIC_API_KEY && claude --dangerously-skip-permissions --model opus -p \'"$(cat docs/skills-playa.md)"\'
    @echo ""
    @echo "The Playa skill has been rebuilt"
    @echo ""
    @if command -v so-you-say >/dev/null 2>&1; then \
        @so-you-say "The playa package has synced it's documents for drift and updated the skill tree."; \
    fi

# Update docs and then update the skill for the Queue package
skill-queue:
    @echo "Fixing documentation drift and rebuilding the skill for the {{BOLD}}Queue{{RESET}} Package"
    @echo "-------------------------------------------------------------------------------"
    @unset ANTHROPIC_API_KEY && claude --dangerously-skip-permissions --model opus -p \'"$(cat docs/skills-queue.md)"\'
    @echo ""
    @echo "The Queue skill has been rebuilt"
    @echo ""
    @if command -v so-you-say >/dev/null 2>&1; then \
        so-you-say "The Queue package has synced it's documents for drift and updated the skill tree."; \
    fi

# Update docs and then update the skill for the biscuit-speak package
skill-biscuit-speaks:
    @echo "Fixing documentation drift and rebuilding the skill for the {{BOLD}}biscuit-speaks{{RESET}} Package"
    @echo "---------------------------------------------------------------------------------"
    @unset ANTHROPIC_API_KEY && claude --dangerously-skip-permissions --model opus -p \'"$(cat docs/skills-biscuit-speaks.md)"\'
    @echo ""
    @echo "The {{BOLD}}biscuit-speaks{{RESET}} skill has been rebuilt"
    @echo ""
    @if command -v so-you-say >/dev/null 2>&1; then \
        so-you-say "The biscuit-speaks package has synced it's documents for drift and updated the skill tree."; \
    fi

# Update docs and then update the skill for the biscuit-hash package
skill-biscuit-hash:
    @echo "Fixing documentation drift and rebuilding the skill for the {{BOLD}}biscuit-hash{{RESET}} Package"
    @echo "---------------------------------------------------------------------------------"
    @unset ANTHROPIC_API_KEY && claude --dangerously-skip-permissions --model opus -p \'"$(cat docs/skills-biscuit-hash.md)"\'
    @echo ""
    @echo "The {{BOLD}}biscuit-hash{{RESET}} skill has been rebuilt"
    @echo ""
    @if command -v so-you-say >/dev/null 2>&1; then \
        so-you-say "The biscuit-hash package has synced it's documents for drift and updated the skill tree."; \
    fi

# Update docs and then update the skill for the biscuit-terminal package
skill-biscuit-terminal:
    @echo "Fixing documentation drift and rebuilding the skill for the {{BOLD}}biscuit-terminal{{RESET}} Package"
    @echo "---------------------------------------------------------------------------------"
    @unset ANTHROPIC_API_KEY && claude --dangerously-skip-permissions --model opus -p \'"$(cat docs/skills-biscuit-terminal.md)"\'
    @echo ""
    @echo "The {{BOLD}}biscuit-terminal{{RESET}} skill has been rebuilt"
    @echo ""
    @if command -v so-you-say >/dev/null 2>&1; then \
        so-you-say "The biscuit-terminal package has synced it's documents for drift and updated the skill tree."; \
    fi

# Update docs and then update the skill for the tree-huger package
skill-tree-hugger:
    @echo "Fixing documentation drift and rebuilding the skill for the {{BOLD}}tree-hugger{{RESET}} package"
    @echo "-------------------------------------------------------------------------------"
    @unset ANTHROPIC_API_KEY && claude --dangerously-skip-permissions --model opus -p \'"$(cat docs/skills-tree-hugger.md)"\'
    @echo ""
    @echo "The {{BOLD}}tree-hugger{{RESET}} skill has been rebuilt"
    @echo ""
    @if command -v so-you-say >/dev/null 2>&1; then \
        so-you-say "The tree-hugger package has synced it's documents for drift and updated the skill tree."; \
    fi


# Update docs and then update the skill for the Research package
skill-research:
    @echo "Fixing documentation drift and rebuilding the skill for the {{BOLD}}research{{RESET}} package"
    @echo "-------------------------------------------------------------------------------"
    @unset ANTHROPIC_API_KEY && claude --dangerously-skip-permissions --model opus -p \'"$(cat docs/skills-research.md)"\'
    @echo ""
    @echo "The {{BOLD}}research{{RESET}} skill has been rebuilt"
    @echo ""
    @if command -v so-you-say >/dev/null 2>&1; then \
        so-you-say "The research package has synced it's documents for drift and updated the skill tree."; \
    fi

# Update docs and then update the skill for the so-you-say package
skill-so-you-say:
    @echo "Fixing documentation drift and rebuilding the skill for the {{BOLD}}so-you-say{{RESET}} package"
    @echo "-------------------------------------------------------------------------------"
    @unset ANTHROPIC_API_KEY && claude --dangerously-skip-permissions --model opus -p \'"$(cat docs/skills-so-you-say.md)"\'
    @echo ""
    @echo "The {{BOLD}}so-you-say{{RESET}} skill has been rebuilt"
    @echo ""
    @if command -v so-you-say >/dev/null 2>&1; then \
        so-you-say "The so-you-say package has synced it's documents for drift and updated the skill tree."; \
    fi

# Update docs and then update the skill for the unchained-ai package
skill-unchained-ai:
    @echo "Fixing documentation drift and rebuilding the skill for the {{BOLD}}unchained-ai{{RESET}} package"
    @echo "-------------------------------------------------------------------------------"
    @unset ANTHROPIC_API_KEY && claude --dangerously-skip-permissions --model opus -p \'"$(cat docs/skills-unchained-ai.md)"\'
    @echo ""
    @echo "The {{BOLD}}unchained-ai{{RESET}} skill has been rebuilt"
    @echo ""
    @if command -v unchained-ai >/dev/null 2>&1; then \
        unchained-ai "The unchained-ai package has synced it's documents for drift and updated the skill tree."; \
    fi

# Update docs and then update the skill for the claudine package
skill-claudine:
    @echo "Fixing documentation drift and rebuilding the skill for the {{BOLD}}claudine{{RESET}} package"
    @echo "-------------------------------------------------------------------------------"
    @unset ANTHROPIC_API_KEY && claude --dangerously-skip-permissions --model opus -p \'"$(cat docs/skills-claudine.md)"\'
    @echo ""
    @echo "The {{BOLD}}claudine{{RESET}} skill has been rebuilt"
    @echo ""
    @if command -v claudine >/dev/null 2>&1; then \
        claudine "The claudine package has synced it's documents for drift and updated the skill tree."; \
    fi
