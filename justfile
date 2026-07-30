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
    echo -e "{{ DIM }}{{ ITALIC }}- using the {{ RESET }}{{ ITALIC }}${COMMIT_MODEL:-${MODEL:-minimax/MiniMax-M3}} {{ DIM }}model{{ RESET }}"
    echo ""
    echo -e "{{ BOLD }}{{ BLUE }}Staged Files:{{ RESET }}"
    sniff repo staged-files || ( echo "No Staged Files! Nothing to do ..." && exit 1 )
    claudine compose "@prompts/commit.md" --opencode --op "commit" --quiet --model "${COMMIT_MODEL:-${MODEL:-minimax/MiniMax-M3}}" -y {{ args }}
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
#
# On native Windows, run scripts\init.ps1 instead of invoking this recipe
# directly: just needs bash AND cygpath on PATH before it can run any recipe,
# so the "no shell environment" check has to happen outside just.
init: _ensure-native-bash _ensure-build-deps _ensure-native-libs _ensure-kache _ensure-nextest _ensure-gitnexus
    #!/usr/bin/env bash
    set -euo pipefail
    # Put cargo on PATH in case _ensure-build-deps just installed Rust
    source scripts/cargo-path.sh
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

# "Accidental WSL" guard. Must be a LINEWISE recipe: just runs shebang recipes
# through the cygpath-translated interpreter (Cygwin/Git Bash), but linewise
# recipes through `set shell`'s bare `bash` — which on Windows can resolve to
# the WSL launcher (WindowsApps sorts before a real bash on PATH). Recipes then
# run in Linux against a /mnt/c checkout and fail confusingly ("cargo: command
# not found" even though Rust is installed on the Windows side). Intentional
# WSL development keeps the checkout in the Linux filesystem, so only /mnt/*
# is rejected.
_ensure-native-bash:
    @if grep -qiE 'microsoft|wsl' /proc/version 2>/dev/null && [ "${PWD#/mnt/}" != "$PWD" ]; then \
        echo "just is running recipes inside WSL against a Windows-mounted checkout ($PWD)." >&2; \
        echo "This happens when 'bash' on the Windows PATH resolves to the WSL launcher" >&2; \
        echo "(WindowsApps\bash.exe) instead of Cygwin or Git Bash." >&2; \
        echo "" >&2; \
        echo "If you intended to run this in native Windows (most likely), then:" >&2; \
        echo "  - run scripts\init.ps1 instead of 'just init', or" >&2; \
        echo "  - reorder your PATH so C:\cygwin64\bin (or Git's bin) sorts BEFORE" >&2; \
        echo "    %LOCALAPPDATA%\Microsoft\WindowsApps, then open a new terminal." >&2; \
        echo "If you meant to work in WSL: clone the repo inside the WSL filesystem" >&2; \
        echo "  (e.g. ~/rusty-biscuit) and run 'just init' from there." >&2; \
        exit 1; \
    fi

# ensure Rust, cargo, and C build tools are available
_ensure-build-deps:
    #!/usr/bin/env bash
    set -euo pipefail
    source scripts/cargo-path.sh

    case "$(uname -s)" in
        MINGW*|MSYS*|CYGWIN*)
            # --- native Windows -------------------------------------------
            if ! command -v rustc &> /dev/null || ! command -v cargo &> /dev/null; then
                echo -e "{{ RED }}Missing Rust toolchain{{ RESET }} (rustc/cargo not found)"
                # sh.rustup.rs misdetects under Cygwin/MSYS (it picks the GNU
                # triple and downloads with Cygwin paths a native curl cannot
                # write), so on Windows we fetch rustup-init.exe directly for
                # the host's MSVC triple instead.
                case "$(uname -m)" in
                    x86_64)        host_triple="x86_64-pc-windows-msvc" ;;
                    aarch64|arm64) host_triple="aarch64-pc-windows-msvc" ;;
                    i686)          host_triple="i686-pc-windows-msvc" ;;
                    *)
                        echo "Unsupported CPU architecture for automatic Rust install: $(uname -m)" >&2
                        echo "Download rustup manually from https://rustup.rs and re-run just init." >&2
                        exit 1
                        ;;
                esac
                # TEMP is a native Windows path, writable by both Cygwin's
                # curl and the System32 curl (which cannot write /tmp paths).
                win_tmp="${TEMP:-${TMP:-/tmp}}"
                installer="$win_tmp/rustup-init.exe"
                echo "Downloading rustup for $host_triple..."
                curl --proto '=https' --tlsv1.2 -sSfL -o "$installer" \
                    "https://static.rust-lang.org/rustup/dist/$host_triple/rustup-init.exe"
                echo "Installing Rust (stable toolchain, MSVC host)..."
                "$installer" -y --default-toolchain stable
                rm -f "$installer"
                source scripts/cargo-path.sh
                cargo --version
            fi

            if rustc -vV 2>/dev/null | grep -q 'host: .*windows-gnu'; then
                echo -e "{{ RED }}Warning{{ RESET }}: the active toolchain targets windows-gnu."
                echo "This repo's Windows builds expect the MSVC toolchain; switch with:"
                echo "  rustup default stable-x86_64-pc-windows-msvc"
            fi

            # The Windows linker is link.exe from the Visual Studio C++
            # workload, not `cc`; vswhere is the supported detector. Detect
            # the compiler COMPONENT (present in every SKU from Build Tools
            # to Community) rather than the Build-Tools-only workload ID.
            case "$(uname -m)" in
                aarch64|arm64) vc_component="Microsoft.VisualStudio.Component.VC.Tools.arm64" ;;
                *)             vc_component="Microsoft.VisualStudio.Component.VC.Tools.x86.x64" ;;
            esac
            pf86="$(printenv 'ProgramFiles(x86)' 2>/dev/null || echo 'C:\Program Files (x86)')"
            vswhere="$(cygpath -u "$pf86")/Microsoft Visual Studio/Installer/vswhere.exe"

            have_vc_tools() {
                [[ -x "$vswhere" ]] && [[ -n "$("$vswhere" -latest -products '*' \
                    -requires "$vc_component" \
                    -property installationPath 2>/dev/null | tr -d '\r' | head -n1)" ]]
            }

            if ! have_vc_tools; then
                echo -e "{{ RED }}Missing C++ linker{{ RESET }} (no Visual Studio with the 'Desktop development with C++' workload)"
                echo "Installing Visual Studio 2022 Build Tools (C++ workload)..."
                echo "This is a multi-GB download and will prompt for administrator approval."
                if command -v winget &> /dev/null; then
                    winget install --id Microsoft.VisualStudio.2022.BuildTools \
                        --accept-source-agreements --accept-package-agreements \
                        --override "--quiet --wait --norestart --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
                else
                    win_tmp="${TEMP:-${TMP:-/tmp}}"
                    vs_installer="$win_tmp/vs_BuildTools.exe"
                    curl --proto '=https' --tlsv1.2 -sSfL -o "$vs_installer" \
                        "https://aka.ms/vs/17/release/vs_BuildTools.exe"
                    "$vs_installer" --quiet --wait --norestart \
                        --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended
                    rm -f "$vs_installer"
                fi
            fi

            if ! have_vc_tools; then
                echo -e "{{ RED }}C++ build tools are still not detected.{{ RESET }}" >&2
                echo "Install 'Visual Studio 2022 Build Tools' with the 'Desktop development" >&2
                echo "with C++' workload from an ELEVATED terminal, then re-run just init:" >&2
                echo "  winget install --id Microsoft.VisualStudio.2022.BuildTools --interactive" >&2
                echo "or download https://aka.ms/vs/17/release/vs_BuildTools.exe" >&2
                exit 1
            fi
            echo "C++ build tools found: $("$vswhere" -latest -products '*' \
                -requires "$vc_component" \
                -property installationPath | tr -d '\r' | head -n1)"
            exit 0
            ;;
    esac

    # --- Linux / macOS ------------------------------------------------------
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

    areas_json="{{ justfile_directory() }}/.github/ci/areas.json"

    if ! command -v jq &> /dev/null; then
        if [[ "$runner_key" == "windows-latest" ]]; then
            # No apt/brew here; winget is the only system package manager.
            if command -v winget &> /dev/null; then
                echo "Installing jq (needed to read .github/ci/areas.json)..."
                winget install --id jqlang.jq -e --silent \
                    --accept-source-agreements --accept-package-agreements || true
                hash -r
            fi
            if ! command -v jq &> /dev/null; then
                # No area currently declares windows-latest native packages;
                # scan only the "native" blocks so a declaration added later
                # is not silently skipped on a jq-less host.
                awk_bin="$(command -v awk || command -v gawk || true)"
                if [[ -n "$awk_bin" ]] && "$awk_bin" '
                        /"native"[[:space:]]*:[[:space:]]*\{/ { inblock=1 }
                        inblock && /windows-latest/           { found=1 }
                        inblock && /^[[:space:]]*\}/          { inblock=0 }
                        END { exit(found ? 1 : 0) }' "$areas_json"; then
                    echo "no native prerequisites declared for Windows — skipping"
                    exit 0
                fi
                if [[ -z "$awk_bin" ]]; then
                    echo "Cannot verify Windows native prerequisites: neither jq nor awk/gawk is installed." >&2
                    echo "Install jq (winget install jqlang.jq) or the Cygwin gawk package, then re-run just init." >&2
                else
                    echo "Windows native prerequisites are declared in .github/ci/areas.json but jq is not installed." >&2
                    echo "Install jq with: winget install jqlang.jq — then re-run just init." >&2
                fi
                exit 1
            fi
        elif [[ -z "$pm" ]]; then
            echo "Could not detect a package manager; install jq, then run just init again." >&2
            exit 1
        else
            echo "Installing jq (needed to read .github/ci/areas.json)..."
            install_packages jq
        fi
    fi

    # `.github/ci/areas.json` declares EVERY area -- gating or not -- and is the
    # single source of truth for native OS packages; this recipe is the single
    # installer. A requirement is declared once, on the area that needs it, and
    # reaches both developer hosts and CI from there. Declarations are per-OS, so
    # a Linux-only dependency never touches a macOS or Windows host.
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
    source scripts/cargo-path.sh

    installed_version=""
    if command -v kache &> /dev/null; then
        installed_version=$(kache --version | cut -d' ' -f2)
    fi

    if [[ "$installed_version" != "{{ KACHE_VERSION }}" ]]; then
        # cargo-binstall is the install path on every OS: it fetches a prebuilt
        # kache binary instead of compiling from source. Install it first when
        # absent (that one install is from source).
        if ! command -v cargo-binstall &> /dev/null; then
            echo "Installing cargo-binstall (used to fetch prebuilt kache binaries)..."
            RUSTC_WRAPPER="" cargo install --locked cargo-binstall
        fi

        RUSTC_WRAPPER="" cargo binstall \
            --no-confirm \
            --force \
            --version "{{ KACHE_VERSION }}" \
            kache
    fi

    # Seed a default store config when the host has none. The tracked
    # .cargo/config.toml makes kache the wrapper for every build in this repo,
    # so the store needs a deliberate cap: an uncapped store thrashes (LRU can
    # evict fresh entries before they score a hit). 100 GiB is the agreed
    # starting point (docs/kache-strategy.md); never overwrite an existing
    # config — hosts size against their own volume.
    case "$(uname -s)" in
        MINGW*|MSYS*|CYGWIN*) kache_config_dir="$(cygpath "${APPDATA:?}")/kache" ;;
        *)                    kache_config_dir="${XDG_CONFIG_HOME:-$HOME/.config}/kache" ;;
    esac
    if [[ ! -f "$kache_config_dir/config.toml" ]]; then
        mkdir -p "$kache_config_dir"
        printf '[cache]\nlocal_max_size = "100GiB"\n' > "$kache_config_dir/config.toml"
        echo "Wrote default kache store cap (100GiB) to $kache_config_dir/config.toml"
    fi

    kache --version

# ensure cargo-sweep is available for target/ hygiene (just sweep)
_ensure-cargo-sweep:
    #!/usr/bin/env bash
    set -euo pipefail
    source scripts/cargo-path.sh

    if cargo sweep --version &> /dev/null; then
        exit 0
    fi

    if command -v cargo-binstall &> /dev/null; then
        RUSTC_WRAPPER="" cargo binstall --no-confirm cargo-sweep \
            || RUSTC_WRAPPER="" cargo install --locked cargo-sweep
    else
        RUSTC_WRAPPER="" cargo install --locked cargo-sweep
    fi

# prune Cargo target/ dirs, which cargo never garbage-collects. With kache
# wired, swept artifacts come back as link-restores, not recompiles — and a
# lean target/ keeps kache's per-crate keying fast (~100x slower on huge
# trees). Passes: uninstalled toolchains, untouched >14d, then a 120GB
# backstop cap per root (docs/kache-strategy.md). Roots default to this repo;
# override with `just sweep <path>...`.
sweep *args="": _ensure-cargo-sweep
    @scripts/sweep.sh {{ args }}

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
    source scripts/cargo-path.sh

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
        case "$(uname -s)" in
            MINGW*|MSYS*|CYGWIN*)
                echo "Install it with: winget install OpenJS.NodeJS.LTS" >&2
                echo "Then open a NEW terminal (so PATH updates apply) and re-run just init." >&2
                ;;
        esac
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
        # `-w` on the full path fails for a FRESH npm prefix (node_modules does
        # not exist yet), which used to drop hosts with a writable prefix into
        # the sudo branch. Test the nearest ancestor that actually exists.
        local probe="$npm_global_root"
        while [[ ! -e "$probe" && "$probe" != "/" && "$probe" != "." ]]; do
            probe="$(dirname "$probe")"
        done
        if [[ -w "$probe" || (-d "$gitnexus_root" && -w "$gitnexus_root") ]]; then
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
        # The native toolchain goes in FIRST: node-gyp/node-addon-api are what
        # a source build of tree-sitter needs, and npm's script-approval flags
        # (--allow-scripts) guarantee their install scripts actually run —
        # without them a blocked script leaves a tree-sitter binding that
        # installs cleanly but cannot be require()'d.
        echo "Installing GitNexus build toolchain (node-gyp, node-addon-api, tree-sitter)..."
        run_global_npm install --global \
            node-addon-api node-gyp tree-sitter \
            --allow-scripts node-addon-api \
            --allow-scripts node-gyp \
            --allow-scripts tree-sitter
        echo "Installing GitNexus..."
        run_global_npm install --global gitnexus@latest \
            --allow-scripts gitnexus \
            --allow-scripts tree-sitter
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
