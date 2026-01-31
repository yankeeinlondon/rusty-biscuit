set dotenv-load
set positional-arguments

# List of areas in this monorepo
areas := "biscuit-hash biscuit-speaks biscuit-terminal playa playa-cli ai-pipeline research so-you-say tree-hugger tree-hugger-cli darkmatter sniff"

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
                just -f "$area/justfile" build {{args}}
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
                just -f "$area/justfile" test {{args}}
            else
                echo "- no TEST command for the area **$area**" >&2
            fi
        else
            echo "- no justfile for the area **$area**" >&2
        fi
    done

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
                just -f "$area/justfile" install
            else
                if just -f "$area/justfile" --summary 2>/dev/null | grep -qw "build"; then
                    echo "No INSTALL command for $area, doing release build..."
                    just -f "$area/justfile" build --release
                else
                    echo "- no INSTALL command for the area **$area**" >&2
                fi
            fi
        else
            echo "- no justfile for the area **$area**" >&2
        fi
    done

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
    cargo run -p ai-pipeline-gen -- {{args}}

# generate models for a specific provider
gen-models-for provider:
    @cargo run -p ai-pipeline-gen -- --providers {{provider}}

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

lint *args="":
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
                just -f "$area/justfile" lint
            else
                if just -f "$area/justfile" --summary 2>/dev/null | grep -qw "lint"; then
                    echo "No lint command for $area"
                else
                    echo "- no lint command for the area **$area**" >&2
                fi
            fi
        else
            echo "- no justfile for the area **$area**" >&2
        fi
    done

# commits all the staged changes using GPT 5.2 (via Opencode)
commit:
    @echo "Committing staged changes in the Rusty Biscuit monorepo to git"
    @echo ""
    @opencode run commit "-f" --model "opencode/gpt-5.2"

# Update docs and then update the skill for the Schematic package
skill-schematic:
    @echo "Fixing documentation drift and rebuilding the skill for the {{BOLD}}Schematic{{RESET}} Package"
    @echo "-------------------------------------------------------------------------------"
    claude --dangerously-skip-permissions --model opus -p \'"$(cat docs/skills-schematic.md)"\'
    @echo ""
    @echo "Schematic Skill has been rebuilt"
    @echo ""
    @so-you-say "The Schematic package has synced it's documents for drift and updated the skill tree."

# Update docs and then update the skill for the darkmatter package
skill-darkmatter:
    @echo "Fixing documentation drift and rebuilding the skill for the {{BOLD}}Darkmatter{{RESET}} Package"
    @echo "-------------------------------------------------------------------------------"
    @claude --dangerously-skip-permissions --model opus -p \'"$(cat docs/skills-darkmatter.md)"\'
    @echo ""
    @echo "Darkmatter Skill has been rebuilt"
    @echo ""
    @so-you-say "The darkmatter package has synced it's documents for drift and updated the skill tree."

# Update docs and then update the skill for the Sniff package
skill-sniff:
    @echo "Fixing documentation drift and rebuilding the skill for the {{BOLD}}Sniff{{RESET}} Package"
    @echo "-------------------------------------------------------------------------------"
    @unset ANTHROPIC_API_KEY
    @claude --dangerously-skip-permissions --model opus -p \'"$(cat docs/skills-sniff.md)"\'
    @echo ""
    @echo "Sniff skill has been rebuilt"
    @echo ""
    @so-you-say "The Sniff package has synced it's documents for drift and updated the skill tree."

# Update docs and then update the skill for the Playa package
skill-playa:
    @echo "Fixing documentation drift and rebuilding the skill for the {{BOLD}}Playa{{RESET}} Package"
    @echo "-------------------------------------------------------------------------------"
    @unset ANTHROPIC_API_KEY
    @claude --dangerously-skip-permissions --model opus -p \'"$(cat docs/skills-playa.md)"\'
    @echo ""
    @echo "The Playa skill has been rebuilt"
    @echo ""
    @so-you-say "The playa package has synced it's documents for drift and updated the skill tree."

# Update docs and then update the skill for the Queue package
skill-queue:
    @echo "Fixing documentation drift and rebuilding the skill for the {{BOLD}}Queue{{RESET}} Package"
    @echo "-------------------------------------------------------------------------------"
    @unset ANTHROPIC_API_KEY
    @claude --dangerously-skip-permissions --model opus -p \'"$(cat docs/skills-queue.md)"\'
    @echo ""
    @echo "The Queue skill has been rebuilt"
    @echo ""
    @so-you-say "The Queue package has synced it's documents for drift and updated the skill tree."

# Update docs and then update the skill for the biscuit-speak package
skill-biscuit-speaks:
    @echo "Fixing documentation drift and rebuilding the skill for the {{BOLD}}biscuit-speaks{{RESET}} Package"
    @echo "---------------------------------------------------------------------------------"
    @unset ANTHROPIC_API_KEY
    @claude --dangerously-skip-permissions --model opus -p \'"$(cat docs/skills-biscuit-speaks.md)"\'
    @echo ""
    @echo "The {{BOLD}}biscuit-speaks{{RESET}} skill has been rebuilt"
    @echo ""
    @so-you-say "The biscuit-speaks package has synced it's documents for drift and updated the skill tree."

# Update docs and then update the skill for the biscuit-hash package
skill-biscuit-hash:
    @echo "Fixing documentation drift and rebuilding the skill for the {{BOLD}}biscuit-hash{{RESET}} Package"
    @echo "---------------------------------------------------------------------------------"
    @unset ANTHROPIC_API_KEY
    @claude --dangerously-skip-permissions --model opus -p \'"$(cat docs/skills-biscuit-hash.md)"\'
    @echo ""
    @echo "The {{BOLD}}biscuit-hash{{RESET}} skill has been rebuilt"
    @echo ""
    @so-you-say "The biscuit-hash package has synced it's documents for drift and updated the skill tree."

# Update docs and then update the skill for the tree-huger package
skill-tree-hugger:
    @echo "Fixing documentation drift and rebuilding the skill for the {{BOLD}}tree-hugger{{RESET}} Package"
    @echo "-------------------------------------------------------------------------------"
    @unset ANTHROPIC_API_KEY
    @claude --dangerously-skip-permissions --model opus -p \'"$(cat docs/skills-tree-hugger.md)"\'
    @echo ""
    @echo "The {{BOLD}}tree-hugger{{RESET}} skill has been rebuilt"
    @echo ""
    @so-you-say "The tree-hugger package has synced it's documents for drift and updated the skill tree."

# Update docs and then update the skill for the Research package
skill-research:
    @echo "Fixing documentation drift and rebuilding the skill for the {{BOLD}}research{{RESET}} Package"
    @echo "-------------------------------------------------------------------------------"
    @unset ANTHROPIC_API_KEY
    @claude --dangerously-skip-permissions --model opus -p \'"$(cat docs/skills-research.md)"\'
    @echo ""
    @echo "The {{BOLD}}research{{RESET}} skill has been rebuilt"
    @echo ""
    @so-you-say "The research package has synced it's documents for drift and updated the skill tree."
