set dotenv-load
set positional-arguments

# List of areas in this monorepo
areas := "shared research so-you-say md sniff"

BOLD := '\033[1m'
RESET := '\033[0m'

default:
    @echo
    @echo "Dockhand Monorepo"
    @echo "================="
    @echo ""
    @just --list | grep -v 'default'
    @echo

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
                echo "- no INSTALL command for the area **$area**" >&2
            fi
        else
            echo "- no justfile for the area **$area**" >&2
        fi
    done

# executes the latest MD CLI code in debug mode
md *args="":
    #!/usr/bin/env bash
    set -euo pipefail
    echo ""
    echo -e "{{BOLD}}MD CLI{{RESET}} (latest debug build)"
    echo -e "--------------------------------------------"
    cd md 2>/dev/null
    cargo run {{args}}

# executes the latest Research CLI code in debug mode
research *args="":
    #!/usr/bin/env bash
    set -euo pipefail
    echo ""
    echo -e "{{BOLD}}Research CLI{{RESET}} (latest debug build)"
    echo "---------------------------------"
    cd research/cli 2>/dev/null
    cargo run {{args}}
