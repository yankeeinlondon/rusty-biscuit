set dotenv-load
set positional-arguments

# List of areas in this monorepo
areas := "biscuit biscuit-hash biscuit-speaks biscuit-terminal playa playa-cli ai-pipeline research so-you-say darkmatter sniff"

BOLD := '\033[1m'
RESET := '\033[0m'

default:
    @echo
    @echo "Dockhand Monorepo"
    @echo "================="
    @echo ""
    @just --list | grep -v 'default'
    @echo

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
    cargo run -p darkmatter --bin md -- {{args}}

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
