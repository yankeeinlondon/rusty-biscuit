set dotenv-load
set positional-arguments

# set allow-duplicate-recipes

set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

import "./just/lifecycle.just"
import "./just/plan.just"
import "./just/review.just"
import "./just/notify.just"
import "./just/ai.just"
import "./just/devops.just"
import "./just/spec.just"

# List of areas in this monorepo

areas := "biscuit-hash biscuit-location biscuit-speaks biscuit-terminal biscuit-tui schematic biscuit-file unchained-ai playa tree-hugger darkmatter sniff model-citizen claudine research queue homelab biscuit-contract biscuit-icon renderable worktree"
BOLD := '\033[1m'
DIM := '\033[2m'
ITALIC := '\033[3m'
RESET := '\033[0m'
RED := '\033[31m'
GREEN := '\033[32m'
# Single kache version authority, shared with GitHub Actions via
# `.github/kache-version` (D2). Both sides read the same file, so they cannot
# drift to different versions.
KACHE_VERSION := trim(`cat .github/kache-version`)

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

modules:
    @cargo modules structure

# Run Level-1 tests for all Cargo workspace packages. Optional selectors may
# name packages or package-area paths: `just test claudine darkmatter`.
test *args="":
    @just _test_workspace {{ args }}

# Verify that every package-area test recipe preserves Ctrl+C as exit 130.
check-test-interrupts:
    @just _check_test_interrupts

# run the test suite, then sweep for child processes that outlived it
#
# Wraps `just test` in the cross-platform `leak-sweep` detector (tools/test-toolkit).
# nextest's per-test LEAK status only catches children still holding a test's
# stdout/stderr; this also catches detached orphans (exit code 99 if any survive,
# rooted at the repo). Pass `--warn-only` semantics by running leak-sweep directly.
test-leaks *args="":
    @cargo run -q -p test-toolkit --features leak-sweep --bin leak-sweep -- just test {{ args }}

# detect which monorepo areas have changed files compared to the upstream branch
#
# `[no-cd]` lets callers (and Level 1 tests) invoke this recipe inside a
# different git working tree without `just` resetting the shell cwd back
# to the justfile's directory. In production the pre-push hook already
# runs at the repo root, so the attribute is a no-op there.
[no-cd]
changed-areas:
    #!/usr/bin/env bash
    set -euo pipefail
    upstream=$(git rev-parse --abbrev-ref --symbolic-full-name @{u} 2>/dev/null || true)
    if [[ -z "${upstream:-}" ]]; then
        # No upstream branch; return empty so caller can fall back
        exit 0
    fi
    changed=$(git diff --name-only "$upstream..HEAD" | cut -d/ -f1 | sort -u)
    matched=""
    for area in {{ areas }}; do
        if echo "$changed" | grep -qx "$area"; then
            matched="$matched $area"
        fi
    done
    # Trim leading space
    echo "${matched# }"

# pre-push hook entry point (default areas: claudine darkmatter)
pre-push *areas="claudine darkmatter":
    @just test {{ areas }}

# run Level 1 tests for the .githooks/pre-push shell hook itself
test-pre-push-hook:
    @./.githooks/tests/test-pre-push.sh

# run Level 1 tests for the `changed-areas` recipe heuristic itself
test-changed-areas:
    @./.githooks/tests/test-changed-areas.sh

# run Level 1 tests for both the pre-push hook and the changed-areas recipe
test-githooks: test-pre-push-hook test-changed-areas

# run doctests (all workspace crates, or specific areas: just doctest claudine playa)
doctest *args="":
    #!/usr/bin/env bash
    set -euo pipefail
    if [[ -z "{{ args }}" ]]; then
        echo ""
        echo "Running doctests for all workspace libraries..."
        echo "------------------------------------------------"
        echo ""
        cargo test --doc --workspace
    else
        IFS=', ' read -ra areas <<< "{{ args }}"
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
    @(cd claudine && just install)

# install the Claudine CLI
install_darkmatter:
    @(cd darkmatter && just install)

# install binaries from all areas that have an install target
install:
    #!/usr/bin/env bash
    set -euo pipefail
    echo ""
    echo "Installing from all areas..."
    echo "----------------------------"
    echo ""
    for area in {{ areas }}; do
        if [ -f "$area/justfile" ]; then
            if (cd "$area" && just --summary 2>/dev/null) | grep -qw "install"; then
                echo
                echo "Installing from $area..."
                (cd "$area" && just install) || ( just _speak "The ${area} package failed during an attempt to install all packages!" && exit 1 )
            else
                if (cd "$area" && just --summary 2>/dev/null) | grep -qw "build"; then
                    echo
                    echo "No INSTALL command for $area, doing release build..."
                    (cd "$area" && just build --release) || ( just _speak "The ${area} package failed to build while attempting a install on all packages." && exit 1 )
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
    echo -e "{{ BOLD }}Darkmatter CLI{{ RESET }} (latest debug build)"
    echo -e "----------------------------------------------------"
    cargo run -p darkmatter-cli --bin md -- {{ args }}

# executes the latest Research CLI code in debug mode
research *args="":
    #!/usr/bin/env bash
    set -euo pipefail
    echo ""
    echo -e "{{ BOLD }}Research CLI{{ RESET }} (latest debug build)"
    echo "----------------------------------------------"
    cargo run -p research-cli -- {{ args }}

# generate provider model enums from APIs
gen-models *args="":
    #!/usr/bin/env bash
    set -euo pipefail
    echo ""
    echo -e "{{ BOLD }}Gen Models{{ RESET }} (latest debug build)"
    echo "---------------------------------"
    cargo run -p unchained-ai-gen -- {{ args }}

# generate models for a specific provider
gen-models-for provider:
    @cargo run -p unchained-ai-gen -- --providers {{ provider }}

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
    @cargo run -p sniff-cli -- {{ args }}

# show workspace package dependencies
repo-deps:
    @cargo run --manifest-path scripts/Cargo.toml --bin repo-deps

lint:
    @just _orchestrate lint

# run sanity checks (all areas, or specific areas: just sanity claudine darkmatter)
sanity *args="":
    @just _orchestrate sanity {{ args }}

# build all areas, or specific areas: just build claudine darkmatter
build *args="":
    @just _orchestrate build {{ args }}

# run benchmarks (all areas, or specific areas: just bench claudine darkmatter)
bench *args="":
    @just _orchestrate bench {{ args }}

# run coverage (all areas, or specific areas: just coverage claudine darkmatter)
coverage *args="":
    @just _orchestrate coverage {{ args }}

# run fuzz targets (all areas, or specific areas: just fuzz claudine darkmatter)
fuzz *args="":
    @just _orchestrate fuzz {{ args }}

# run all canonical tiers (all areas, or specific areas: just all claudine darkmatter)
all *args="":
    @just _orchestrate all {{ args }}

# validate that every curated package area defines the canonical 12-recipe set
#
# Parses each area's `justfile` directly with grep rather than spawning a
# nested `just --summary` per area. The nested form is prone to hangs on
# large workspaces (observed timing out at `homelab` under cold caches),
# and even when it does not hang it spawns 17+ `just` parser processes
# that each re-walk shared imports. Direct parsing keeps this gate fast
# (~50 ms) and deterministic in CI.
check-canonical *args="":
    #!/usr/bin/env bash
    set -euo pipefail
    failed_areas=()
    passed_areas=()
    required=(sanity test test-l2 test-l3 test-browser test-real lint bench coverage doctest fuzz all)

    if [[ -z "{{ args }}" ]]; then
        target_areas=({{ areas }})
    else
        IFS=', ' read -ra target_areas <<< "{{ args }}"
    fi

    echo ""
    echo "Validating canonical recipe set for: ${target_areas[*]}"
    echo "------------------------------------------------"
    echo ""

    for area in "${target_areas[@]}"; do
        if [ ! -f "$area/justfile" ]; then
            echo -e "{{ RED }}❌ $area:{{ RESET }} no justfile"
            failed_areas+=("$area")
            continue
        fi
        echo "Checking $area..."
        missing=()
        for r in "${required[@]}"; do
            # A recipe definition in just starts at column 0 with the recipe
            # name followed by optional `*args=""` / parameters and a `:`.
            # We deliberately do not invoke `just` here — see recipe header.
            if ! grep -Eq "^${r}( |:|\$)" "$area/justfile"; then
                missing+=("$r")
            fi
        done
        if (( ${#missing[@]} > 0 )); then
            echo -e "  {{ RED }}❌ Missing canonical recipes:{{ RESET }} ${missing[*]}"
            failed_areas+=("$area")
        else
            echo -e "  {{ GREEN }}✅ Justfile defines all ${#required[@]} canonical recipes{{ RESET }}"
            passed_areas+=("$area")
        fi
    done

    echo ""
    echo "================================================"
    echo "check-canonical summary"
    echo "================================================"
    echo -e "{{ GREEN }}Passed{{ RESET }} (${#passed_areas[@]}): ${passed_areas[*]:-(none)}"
    if [[ ${#failed_areas[@]} -gt 0 ]]; then
        echo -e "{{ RED }}Failed{{ RESET }} (${#failed_areas[@]}): ${failed_areas[*]}"
    else
        echo -e "{{ RED }}Failed{{ RESET }} (${#failed_areas[@]}): (none)"
    fi
    echo "================================================"
    echo ""

    if [[ ${#failed_areas[@]} -gt 0 ]]; then
        exit 1
    fi

# commits all the staged changes using model from COMMIT_MODEL or MODEL in OpenCode
commit *args="":
    @echo ""
    @echo -e "Committing staged changes in the {{ BOLD }}Rusty Biscuit{{ RESET }} monorepo to git"
    @echo -e "{{ DIM }}{{ ITALIC }}- using the {{ RESET }}{{ ITALIC }}${COMMIT_MODEL:-${MODEL:-minimax-coding-plan/MiniMax-M3}} {{ DIM }}model{{ RESET }}"
    @echo ""
    @echo -e "{{ BOLD }}{{ BLUE }}Staged Files:{{ RESET }}"
    @sniff repo staged-files || ( echo "No Staged Files! Nothing to do ..." && exit 1 )
    @claudine compose "@prompts/commit.md" --opencode --op "commit" --quiet --model "${COMMIT_MODEL:-${MODEL:-minimax-coding-plan/MiniMax-M3}}" -y {{ args }}
    @just _speak "git commits completed in rusty-biscuit monorepo"
    @sniff repo git-status 2>/dev/null || exit 0
    @echo

# stages all files in package area and then commits and pushes
cp:
    @echo ""
    @echo "Staging all {{ BOLD }}modified{{ RESET }} or {{ BOLD }}untracked{{ RESET }} files across the {{ RED }}rusty-biscuit{{ RESET }} monorepo."
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
    @echo "All committed files from {{ BOLD }}rusty-biscuit{{ RESET }} monorepo have now been pushed to remote."
    @echo

# install rusty-biscuit and third-party CLIs used for development
init: _ensure-build-deps _ensure-kache _ensure-gitnexus
    #!/usr/bin/env bash
    set -euo pipefail
    # Source cargo env in case _ensure-build-deps just installed Rust
    [[ -f "$HOME/.cargo/env" ]] && source "$HOME/.cargo/env"
    echo -e "Initializing the {{ RED }}rusty-biscuit{{ RESET }} monorepo"
    echo
    echo -e "First step is to ensure CLIs used for development are installed"
    echo
    (cd sniff && just install)
    sniff runtime
    kache doctor
    (cd biscuit-terminal && just install)
    (cd darkmatter && just install)
    (cd playa && just install)
    (cd biscuit-speaks && just install)

# ensure Rust, cargo, and C build tools are available
_ensure-build-deps:
    #!/usr/bin/env bash
    set -euo pipefail

    # Check for Rust and cargo
    if ! command -v rustc &> /dev/null || ! command -v cargo &> /dev/null; then
        echo -e "{{ RED }}Missing Rust toolchain{{ RESET }} (rustc/cargo not found)"
        echo "Installing Rust via rustup..."
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        source "$HOME/.cargo/env"
        echo "Rust toolchain installed."
    fi

    # Check for C compiler/linker
    if command -v cc &> /dev/null; then
        exit 0
    fi
    echo -e "{{ RED }}Missing build dependencies{{ RESET }} (cc linker not found)"
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

# ensure the repository-pinned Rust compiler cache is available
_ensure-kache:
    #!/usr/bin/env bash
    set -euo pipefail
    [[ -f "$HOME/.cargo/env" ]] && source "$HOME/.cargo/env"

    installed_version=""
    if command -v kache &> /dev/null; then
        installed_version=$(kache --version | awk '{print $2}')
    fi

    if [[ "$installed_version" == "{{ KACHE_VERSION }}" ]]; then
        exit 0
    fi

    if command -v cargo-binstall &> /dev/null; then
        RUSTC_WRAPPER="" cargo binstall \
            --no-confirm \
            --version "{{ KACHE_VERSION }}" \
            kache
    else
        RUSTC_WRAPPER="" cargo install \
            --locked \
            --force \
            --version "{{ KACHE_VERSION }}" \
            kache
    fi

    kache --version

# ensure GitNexus and its native parser are available for this host
_ensure-gitnexus:
    #!/usr/bin/env bash
    set -euo pipefail

    if ! command -v node &> /dev/null || ! command -v npm &> /dev/null; then
        echo "GitNexus requires Node.js 22 or newer and npm." >&2
        exit 1
    fi

    node_major=$(node -p 'Number(process.versions.node.split(".")[0])')
    if (( node_major < 22 )); then
        echo "GitNexus requires Node.js 22 or newer; found $(node --version)." >&2
        exit 1
    fi

    npm_global_root_native=$(npm root --global)
    npm_global_root="$npm_global_root_native"
    if command -v cygpath &> /dev/null; then
        npm_global_root=$(cygpath --unix "$npm_global_root_native")
    fi
    gitnexus_root="$npm_global_root/gitnexus"
    gitnexus_root_for_node="$gitnexus_root"
    if command -v cygpath &> /dev/null; then
        gitnexus_root_for_node=$(cygpath --windows "$gitnexus_root")
    fi

    run_global_npm() {
        if [[ -w "$npm_global_root" || (-d "$gitnexus_root" && -w "$gitnexus_root") ]]; then
            npm "$@"
        elif command -v sudo &> /dev/null; then
            sudo npm "$@"
        else
            echo "The npm global package directory is not writable: $npm_global_root" >&2
            echo "Configure a user-writable npm prefix, then run just init again." >&2
            return 1
        fi
    }

    if ! command -v gitnexus &> /dev/null || [[ ! -f "$gitnexus_root/package.json" ]]; then
        echo "Installing GitNexus..."
        run_global_npm install --global gitnexus@latest
    fi

    if [[ ! -d "$gitnexus_root/node_modules/tree-sitter" ]]; then
        echo "GitNexus installation is missing its tree-sitter dependency." >&2
        exit 1
    fi

    if ! GITNEXUS_TREE_SITTER="$gitnexus_root_for_node/node_modules/tree-sitter" \
        node -e 'require(process.env.GITNEXUS_TREE_SITTER)' &> /dev/null; then
        echo "Building GitNexus tree-sitter support for $(node -p '`${process.platform}-${process.arch}, Node ${process.versions.node}`')..."
        run_global_npm rebuild tree-sitter \
            --prefix "$gitnexus_root" \
            --build-from-source
    fi

    GITNEXUS_TREE_SITTER="$gitnexus_root_for_node/node_modules/tree-sitter" \
        node -e 'require(process.env.GITNEXUS_TREE_SITTER)'
    gitnexus --version
    echo "GitNexus is ready."

# report the active compiler-cache configuration and health
cache-status:
    @sniff runtime
    @kache doctor
    @kache stats
    @kache daemon

# install kache's optional login service for remote caching
cache-daemon-install:
    @kache daemon install
    @kache daemon

# sync a just recipe from one justfile to all others that have it
sync-recipe recipe source:
    @./scripts/sync-recipe.sh "{{ recipe }}" "{{ source }}"

# heuristic check for comment quality anti-patterns (warn-only)
check-comments *args="":
    @./scripts/check-comments.sh {{ args }}

# run fixture tests for the comment-quality heuristic checker
check-comments-test:
    @./scripts/check-comments-tests.sh

# Internal helper: run a named recipe across all curated areas (or specific areas).
_orchestrate recipe *args="":
    #!/usr/bin/env bash
    set -euo pipefail
    failed_areas=()
    passed_areas=()

    if [[ -z "{{ args }}" ]]; then
        echo ""
        echo "Running {{ recipe }} for all areas..."
        echo "------------------------------------------------"
        echo ""
        for area in {{ areas }}; do
            if [ -f "$area/justfile" ]; then
                if (cd "./$area" && just --summary 2>/dev/null) | grep -qw "{{ recipe }}"; then
                    echo
                    echo "{{ recipe }} $area..."
                    if (cd "./$area" && just {{ recipe }}); then
                        passed_areas+=("$area")
                    else
                        failed_areas+=("$area")
                        just _message "{{ recipe }} failed in $area"
                    fi
                else
                    echo "Error: area '$area' has no {{ recipe }} recipe" >&2
                    failed_areas+=("$area (no {{ recipe }} recipe)")
                fi
            else
                echo "Error: area '$area' has no justfile" >&2
                failed_areas+=("$area (no justfile)")
            fi
        done
    else
        IFS=', ' read -ra areas <<< "{{ args }}"
        echo ""
        echo "Running {{ recipe }} for: ${areas[*]}"
        echo "------------------------------------------------"
        echo ""
        for area in "${areas[@]}"; do
            if [ -d "$area" ] && [ -f "$area/justfile" ]; then
                if (cd "./$area" && just --summary 2>/dev/null) | grep -qw "{{ recipe }}"; then
                    echo
                    echo "{{ recipe }} $area..."
                    if (cd "./$area" && just {{ recipe }}); then
                        passed_areas+=("$area")
                    else
                        failed_areas+=("$area")
                        just _message "{{ recipe }} failed in $area"
                    fi
                else
                    echo "Error: area '$area' has no {{ recipe }} recipe" >&2
                    failed_areas+=("$area (no {{ recipe }} recipe)")
                fi
            else
                echo "Error: area '$area' not found or has no justfile" >&2
                failed_areas+=("$area (not found / no justfile)")
            fi
        done
    fi

    echo ""
    echo "================================================"
    echo "{{ recipe }} summary"
    echo "================================================"
    echo -e "{{ GREEN }}Passed{{ RESET }} (${#passed_areas[@]}): ${passed_areas[*]:-(none)}"
    if [[ ${#failed_areas[@]} -gt 0 ]]; then
        echo -e "{{ RED }}Failed{{ RESET }} (${#failed_areas[@]}): ${failed_areas[*]}"
    else
        echo -e "{{ RED }}Failed{{ RESET }} (${#failed_areas[@]}): (none)"
    fi
    echo "================================================"
    echo ""

    if [[ ${#failed_areas[@]} -gt 0 ]]; then
        exit 1
    fi

audio-reset:
    sudo killall coreaudiod
