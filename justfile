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

# Every package area in this monorepo that owns tests.
#
# This list is what `check-canonical` validates and what `_orchestrate`,
# `changed-areas`, and `install` iterate. It used to be a verbatim copy of the
# 21 `ci: true` records in `.github/ci/areas.json`, which made the canonical
# recipe guard structurally blind to every excluded area — so the six areas
# below could sit at `ci: false` with "blocked on the canonical recipe set" as
# the recorded reason and nothing ever reported the gap. An area is listed here
# because it has tests worth gating, NOT because it is already promoted to CI;
# promotion remains a separate, deliberate decision in `areas.json`.
#
# Deliberately absent: `visualizer`, `reaper`, `agent-sandbox`, and `tabby`.
# Those four carry zero or one test, so a canonical recipe set would gate
# nothing (`agent-sandbox` and `tabby` have no justfile at all). Add each one
# here at the same time it gains a suite worth running.
areas := "biscuit-hash biscuit-location biscuit-speaks biscuit-terminal biscuit-tui schematic biscuit-file unchained-ai playa tree-hugger darkmatter sniff model-citizen claudine research queue homelab biscuit-contract biscuit-icon renderable worktree tools biscuit-test-harness biscuit-browser-harness messenger biscuit-visualized biscuit-clipboard"
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
        # A top-level ``VAR := `cmd` `` is evaluated whenever just LOADS the
        # justfile, for every recipe. If `cmd` is a repo-built CLI, `just lint`
        # in CI dies at parse time on a tool CI never installs -- which is how
        # homelab's `sniff`-backed INTEGRATIONS broke its lint job. Only tools
        # present on a bare runner may appear there; anything else belongs
        # inside the recipe that needs it. (`--dry-run` does NOT evaluate these,
        # so it cannot be used to test this.)
        portable='^(cat|date|echo|printf|pwd|uname|basename|dirname|git)([ `]|$)'
        while IFS= read -r assignment; do
            command_word="${assignment#*\`}"
            if ! grep -Eq "$portable" <<< "$command_word"; then
                echo -e "  {{ RED }}❌ Load-time backtick runs a non-portable tool:{{ RESET }} ${assignment}"
                echo    "     Move it into the recipe that needs it; every recipe pays this cost."
                missing+=("<load-time-backtick>")
            fi
        done < <(grep -E '^[A-Za-z_]+ *:?=.*`' "$area/justfile" || true)

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
# (under CI: a plain `git commit` with the message passed as the argument)
commit *args="":
    #!/usr/bin/env bash
    set -euo pipefail

    # A runner has no TTY, no model credentials, and no audio device, and CI must
    # not depend on a network round-trip to write a commit message. Under CI this
    # is therefore a plain, deterministic commit whose message the caller supplies
    # — no LLM, no `_speak`, no network. Local behavior is unchanged.
    if [[ -n "${CI:-}" ]]; then
        message={{ quote(args) }}
        if [[ -z "$message" ]]; then
            echo "just commit: under CI the commit message must be passed as the argument" >&2
            echo "  e.g. just commit \"chore(ci): regenerate catalog\"" >&2
            exit 1
        fi
        if git diff --cached --quiet; then
            echo "No Staged Files! Nothing to do ..."
            exit 1
        fi
        git commit -m "$message"
        exit 0
    fi

    echo ""
    echo -e "Committing staged changes in the {{ BOLD }}Rusty Biscuit{{ RESET }} monorepo to git"
    echo -e "{{ DIM }}{{ ITALIC }}- using the {{ RESET }}{{ ITALIC }}${COMMIT_MODEL:-${MODEL:-minimax-coding-plan/MiniMax-M3}} {{ DIM }}model{{ RESET }}"
    echo ""
    echo -e "{{ BOLD }}{{ BLUE }}Staged Files:{{ RESET }}"
    sniff repo staged-files || ( echo "No Staged Files! Nothing to do ..." && exit 1 )
    claudine compose "@prompts/commit.md" --opencode --op "commit" --quiet --model "${COMMIT_MODEL:-${MODEL:-minimax-coding-plan/MiniMax-M3}}" -y {{ args }}
    just _speak "git commits completed in rusty-biscuit monorepo"
    sniff repo git-status 2>/dev/null || exit 0
    echo

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
init: _ensure-build-deps _ensure-native-libs _ensure-kache _ensure-nextest _ensure-gitnexus
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

# ensure the system libraries areas declare in `.github/ci/areas.json` are present
# (no argument = every area's libraries, for `just init`; an area name = only that
# area's, which is what CI runs before an area's build/test/lint commands)
_ensure-native-libs area="":
    #!/usr/bin/env bash
    set -euo pipefail

    case "$(uname -s)" in
        Linux)  runner_key="ubuntu-latest" ;;
        Darwin) runner_key="macos-latest" ;;
        *)      runner_key="windows-latest" ;;
    esac

    sudo_cmd=""
    if [[ "$(id -u)" -ne 0 ]]; then
        sudo_cmd="sudo"
    fi

    pm=""
    for candidate in apt-get dnf pacman apk brew; do
        if command -v "$candidate" &> /dev/null; then
            pm="$candidate"
            break
        fi
    done

    install_packages() {
        case "$pm" in
            apt-get) $sudo_cmd apt-get update -qq && $sudo_cmd apt-get install -y -qq "$@" ;;
            dnf)     $sudo_cmd dnf install -y "$@" ;;
            pacman)  $sudo_cmd pacman -S --noconfirm "$@" ;;
            apk)     $sudo_cmd apk add "$@" ;;
            brew)    brew install "$@" ;;
        esac
    }

    if ! command -v jq &> /dev/null; then
        if [[ -z "$pm" ]]; then
            echo "Could not detect a package manager; install jq, then run just init again." >&2
            exit 1
        fi
        echo "Installing jq (needed to read .github/ci/areas.json)..."
        install_packages jq
    fi

    # `.github/ci/areas.json` declares EVERY area -- gating or not -- and is the
    # single source of truth for native OS packages; this recipe is the single
    # installer. A requirement is declared once, on the area that needs it, and
    # reaches both developer hosts and CI from there. Declarations are per-OS, so
    # a Linux-only dependency never touches a macOS or Windows host.
    areas_json="{{ justfile_directory() }}/.github/ci/areas.json"
    area="{{ area }}"

    if [[ -n "$area" ]] && ! jq -e --arg a "$area" 'any(.[]; .area == $a)' "$areas_json" > /dev/null; then
        echo "No area named '$area' in .github/ci/areas.json" >&2
        exit 1
    fi

    # An empty area selects every record, which is what `just init` and the
    # workspace-wide coverage job need: non-gating areas carry declarations too
    # and have no area name to scope to.
    declared=$(jq -r --arg k "$runner_key" --arg a "$area" \
        '[.[] | select($a == "" or .area == $a) | (.native // {})[$k] // []] | add // [] | unique | .[]' \
        "$areas_json")

    if [[ -z "$declared" ]]; then
        echo "no native prerequisites declared for ${area:-all areas} on $runner_key"
        exit 0
    fi

    # areas.json names packages the way the CI runner does (apt on Linux, brew on
    # macOS). Each row adds the pkg-config module that proves the library is
    # installed — the same check the failing `-sys` build scripts perform — plus
    # the equivalent package name on the other Linux package managers.
    #   ci-name|pkg-config module|dnf|pacman|apk
    native_map="
    libasound2-dev|alsa|alsa-lib-devel|alsa-lib|alsa-lib-dev
    libpulse-dev|libpulse|pulseaudio-libs-devel|libpulse|pulseaudio-dev
    libgtk-3-dev|gtk+-3.0|gtk3-devel|gtk3|gtk+3.0-dev
    libwebkit2gtk-4.1-dev|webkit2gtk-4.1|webkit2gtk4.1-devel|webkit2gtk-4.1|webkit2gtk-4.1-dev
    libdbus-1-dev|dbus-1|dbus-devel|dbus|dbus-dev
    "

    row_for() {
        awk -F'|' -v n="$1" '{ gsub(/[ \t]/, "") } $1 == n { print; exit }' <<< "$native_map"
    }

    is_installed() {
        local pkg="$1" module="$2"
        if [[ -n "$module" ]] && command -v pkg-config &> /dev/null; then
            pkg-config --exists "$module"
            return
        fi
        if command -v dpkg &> /dev/null; then
            dpkg -s "$pkg" &> /dev/null
            return
        fi
        if command -v brew &> /dev/null; then
            brew list --versions "$pkg" &> /dev/null
            return
        fi
        return 1
    }

    missing=()
    for pkg in $declared; do
        if ! is_installed "$pkg" "$(row_for "$pkg" | cut -d'|' -f2)"; then
            missing+=("$pkg")
        fi
    done

    if [[ ${#missing[@]} -eq 0 ]]; then
        exit 0
    fi

    echo -e "{{ RED }}Missing native libraries{{ RESET }}: ${missing[*]}"

    case "$pm" in
        apt-get|brew) field=1 ;;
        dnf)          field=3 ;;
        pacman)       field=4 ;;
        apk)          field=5 ;;
        *)            field=0 ;;
    esac

    packages=()
    unresolved=()
    for pkg in "${missing[@]}"; do
        name="$pkg"
        if [[ "$field" -gt 1 ]]; then
            name="$(row_for "$pkg" | cut -d'|' -f"$field")"
        fi
        if [[ "$field" -eq 0 || -z "$name" ]]; then
            unresolved+=("$pkg")
        else
            packages+=("$name")
        fi
    done

    if [[ ${#unresolved[@]} -gt 0 ]]; then
        echo "No package name is known for this host: ${unresolved[*]}" >&2
        echo "Install the equivalent development headers, then run just init again." >&2
        echo "Add the mapping to _ensure-native-libs so the next host is handled." >&2
        exit 1
    fi

    # The `-sys` build scripts locate these libraries through pkg-config, so a
    # host that has the headers but not pkg-config still fails to build.
    if ! command -v pkg-config &> /dev/null; then
        case "$pm" in
            pacman|apk) packages+=("pkgconf") ;;
            *)          packages+=("pkg-config") ;;
        esac
    fi

    echo "Installing native prerequisites: ${packages[*]}"
    install_packages "${packages[@]}"
    echo "Native libraries installed."

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

# ensure the test runner every tier above L1 depends on is available
#
# Only `_test` (L1) degrades to `cargo test` when nextest is absent. `_test_l2`,
# `_test_l3`, `_test_browser`, `_test_real`, and `_sanity` invoke `cargo nextest
# run` unconditionally — they need its `-E` filtersets to select a tier — and
# `.config/nextest.toml` carries the retry, slow-timeout, and leak-timeout policy
# that `cargo test` has no equivalent for. A host missing nextest therefore fails
# every tier above L1 outright rather than running them unprotected.
_ensure-nextest:
    #!/usr/bin/env bash
    set -euo pipefail
    [[ -f "$HOME/.cargo/env" ]] && source "$HOME/.cargo/env"

    if cargo nextest --version &> /dev/null; then
        exit 0
    fi

    if command -v cargo-binstall &> /dev/null; then
        RUSTC_WRAPPER="" cargo binstall --no-confirm cargo-nextest
    else
        RUSTC_WRAPPER="" cargo install --locked cargo-nextest
    fi

    cargo nextest --version

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
