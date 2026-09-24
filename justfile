set dotenv-load
set positional-arguments

# set allow-duplicate-recipes

# The `env CYG_SYS_BASHRC=1` prefix is load-bearing. Cygwin's /etc/bash.bashrc
# opens with `[[ -z ${CYG_SYS_BASHRC} ]]`, an unguarded expansion that the `-u`
# below turns into a fatal "unbound variable" wherever bash sources that file
# non-interactively (a shell exporting BASH_ENV is the usual cause). Pre-setting
# the variable sends that guard down its already-initialized early-return path.
#
# It must reach bash through the environment rather than just's `export`, which
# covers recipe shells but NOT the shells just spawns to evaluate backtick
# assignments — `KACHE_MIN_VERSION` below is one.
set shell := ["env", "CYG_SYS_BASHRC=1", "bash", "-eu", "-o", "pipefail", "-c"]

import "./just/lifecycle.just"
import "./just/plan.just"
import "./just/review.just"
import "./just/notify.just"
import "./just/ai.just"
import "./just/devops.just"
import "./just/ci-local.just"
import "./just/spec.just"

# Every package area in this monorepo that owns tests.
#
# This list is what `check-canonical` validates and what `_orchestrate` and
# `install` iterate. An area is listed here because it has
# tests worth gating, NOT because it is already promoted to CI; promotion is a
# separate, deliberate decision in the package's own manifest
# (`[package.metadata.ci] gates`).
#
# Deliberately absent: `visualizer`, `reaper`, `agent-sandbox`, and `tabby`.
# Those four carry zero or one test, so a canonical recipe set would gate
# nothing (`agent-sandbox` and `tabby` have no justfile at all). Add each one
# here at the same time it gains a suite worth running.
areas := "biscuit-hash biscuit-location biscuit-speaks biscuit-terminal biscuit-tui schematic biscuit-file unchained-ai playa tree-hugger darkmatter sniff model-citizen claudine research queue homelab biscuit-contract biscuit-icon renderable worktree tools biscuit-test-harness biscuit-browser-harness messenger biscuit-visualized biscuit-clipboard"

# The `areas :=` list above is a LOCAL convenience (canonical-recipe validation
# and `_orchestrate` iteration). CI does not read it: CI selects and records
# work by package, with package policy in each manifest's `[package.metadata.ci]`.
BOLD := '\033[1m'
DIM := '\033[2m'
ITALIC := '\033[3m'
RESET := '\033[0m'
RED := '\033[31m'
GREEN := '\033[32m'
YELLOW := '\033[33m'
# Single kache version FLOOR (D2). Hosts run the latest kache; the repository
# only states the minimum it has verified against (0.7.x was silently
# write-only). Read here and by the maintenance audit, so the two cannot drift.
KACHE_MIN_VERSION := trim(`cat .github/kache-min-version`)

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

# Verify no tier filter strands tests behind a stub `test-<tier>` recipe.
#
# Not part of any lifecycle recipe or hook: it has to build each stubbing area's
# test binaries to answer the question. Run it when tier markers or tier recipes
# change. Optional args restrict it to named area directories.
check-tier-coverage *args="":
    @just _check_tier_coverage {{ args }}

# run the test suite, then sweep for child processes that outlived it
#
# Wraps `just test` in the cross-platform `leak-sweep` detector (tools/test-toolkit).
# nextest's per-test LEAK status only catches children still holding a test's
# stdout/stderr; this also catches detached orphans (exit code 99 if any survive,
# rooted at the repo). Pass `--warn-only` semantics by running leak-sweep directly.
test-leaks *args="":
    @cargo run -q -p test-toolkit --features leak-sweep --bin leak-sweep -- just test {{ args }}

# pre-push hook entry point: full local evidence for source-changed packages
#
# Runs lint and L1 plus hostable L2 for source packages. Direct reverse
# dependencies are not compiled here; CI compiles them inside the changed
# package's own Linux check cell. A successful hook can let CI omit this host's
# environment for the exact outgoing tree.
pre-push *selectors="":
    @just ci-local --l2 {{ selectors }}

# run one package's L1 suite on the standing build-host clones (real Linux,
# native Windows, WSL2 in CI's nextest-archive mode, and macOS) against the
# LOCAL tree — no commit or push needed. Hosts come from BUILD_LINUX,
# BUILD_WIN, BUILD_WSL, and BUILD_MACOS (SSH destinations); an unset variable
# means that host is not available here. `--os all` (the default) runs every
# declared OS except the local one. Runs on a host queue behind a per-host
# lock. The expected pre-push step for changes touching path semantics,
# process spawning, or terminal behavior, which the local L1 cannot exercise.
# Usage: just cross-check <package> [--os linux|windows|wsl|macos|all] [nextest args]
cross-check *args:
    @./scripts/cross-check.sh {{ args }}

# run Level 1 tests for the .githooks/pre-push shell hook itself
test-pre-push-hook:
    @./.githooks/tests/test-pre-push.sh

# prove the shared hook directory dispatches to each linked worktree's hook
test-pre-push-dispatcher:
    @./.githooks/tests/test-pre-push-dispatcher.sh

# run Level 1 tests for every versioned git hook
test-githooks: test-pre-push-dispatcher test-pre-push-hook

# run doctests (all workspace crates, or specific areas: just doctest claudine playa)
doctest *args="": _storage_preflight
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

# --- Rust toolchain pin --------------------------------------------------------
# `rust-toolchain.toml` is the only place the toolchain version lives. rustup
# honors it for every local cargo invocation, CI's toolchain step is `rustup
# show` against the same file, and the cross-check build hosts check the repo
# out, so advancing that one line moves everything.

# Show the pinned Rust toolchain next to the current latest stable
toolchain:
    #!/usr/bin/env bash
    set -euo pipefail
    pinned="$(just _toolchain-pinned)"
    latest="$(just _toolchain-latest-stable)"
    echo -e "pinned:        {{ BOLD }}${pinned}{{ RESET }}  (rust-toolchain.toml)"
    echo -e "latest stable: {{ BOLD }}${latest}{{ RESET }}"
    if [[ "$pinned" == "$latest" ]]; then
        echo -e "{{ GREEN }}the pin is at the latest stable{{ RESET }}"
    else
        echo -e "{{ YELLOW }}a newer stable exists{{ RESET }}; advance the pin with {{ BOLD }}just toolchain-upgrade{{ RESET }}"
    fi

# Advance the pin to the latest stable (or the given version) and install it
toolchain-upgrade version="":
    #!/usr/bin/env bash
    set -euo pipefail
    current="$(just _toolchain-pinned)"
    target="{{ version }}"
    if [[ -z "$target" ]]; then
        target="$(just _toolchain-latest-stable)"
    fi
    if [[ ! "$target" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
        echo -e "{{ RED }}error{{ RESET }}: the pin must be an exact version such as 1.98.1, not '${target}'" >&2
        exit 2
    fi
    if [[ "$target" == "$current" ]]; then
        echo -e "rust-toolchain.toml already pins {{ BOLD }}${current}{{ RESET }}"
        exit 0
    fi
    python3 - "$target" <<'PY'
    import re, sys
    path = "rust-toolchain.toml"
    text = open(path).read()
    updated, count = re.subn(r'^channel = "[^"]+"$', f'channel = "{sys.argv[1]}"', text, flags=re.M)
    assert count == 1, "expected exactly one channel line"
    open(path, "w").write(updated)
    PY
    echo -e "pinned {{ BOLD }}${current}{{ RESET }} → {{ BOLD }}${target}{{ RESET }} in rust-toolchain.toml"
    echo "installing it with its components..."
    rustup toolchain install >/dev/null
    rustc --version
    echo
    echo -e "next: {{ BOLD }}just ci-local --lint-only --all{{ RESET }} surfaces new clippy lints or rustfmt drift before you commit the pin."

_toolchain-pinned:
    @sed -n 's/^channel = "\(.*\)"$/\1/p' rust-toolchain.toml

# The stable channel manifest is the same source rustup resolves `stable` from.
_toolchain-latest-stable:
    @curl -fsSL https://static.rust-lang.org/dist/channel-rust-stable.toml | awk '/^\[pkg.rust\]$/ { block = 1; next } block && !found && /^version = / { match($0, /[0-9]+\.[0-9]+\.[0-9]+/); print substr($0, RSTART, RLENGTH); found = 1 }'

# run the latest debug build of the `sniff` CLI
sniff *args="":
    @cargo run -p sniff-cli -- {{ args }}

# show workspace package dependencies
repo-deps:
    @cargo run -p repo-deps --bin repo-deps

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

# install host, CI/CD, and rusty-biscuit tools used by repository recipes
#
# On native Windows, run scripts\init.ps1 instead of invoking this recipe
# directly: just needs bash AND cygpath on PATH before it can run any recipe,
# so the "no shell environment" check has to happen outside just.
init: _ensure-native-bash
    #!/usr/bin/env bash
    set -euo pipefail
    echo -e "Initializing the {{ RED }}rusty-biscuit{{ RESET }} monorepo"
    echo

    echo -e "{{ BOLD }}Host prerequisites{{ RESET }}"
    just _ensure-build-deps
    source scripts/cargo-path.sh
    just _ensure-native-libs
    just _ensure-host-tools
    echo

    echo -e "{{ BOLD }}CI/CD tools{{ RESET }}"
    just _ensure-ci-tools
    echo

    echo -e "{{ BOLD }}Repository and developer tools{{ RESET }}"
    just _ensure-cargo-sweep
    just _ensure-kache
    just _ensure-gitnexus
    just _ensure-git-hooks
    echo
    (cd sniff && just install)
    sniff runtime
    (cd biscuit-hash && just install)
    (cd biscuit-terminal && just install)
    (cd darkmatter && just install)
    (cd playa && just install)
    (cd biscuit-speaks && just install)
    (cd claudine && just install)

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

# ensure repository shell and interactive utility dependencies are available
_ensure-host-tools:
    @bash scripts/ensure-host-tools.sh

# ensure CLIs invoked directly by CI/CD and local reproduction recipes exist
_ensure-ci-tools: _ensure-nextest
    @bash scripts/ensure-ci-tools.sh

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

# ensure the system libraries packages declare in their own manifests
# (`[package.metadata.ci.native]`) are present. No argument = every workspace
# package's libraries (for `just init` and the workspace-wide coverage job);
# arguments = an explicit list of CI-key package names, which is what CI passes
# after the scope job computes a package's dependency-closure union.
_ensure-native-libs *packages="":
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

    declared="{{ packages }}"

    if [[ -z "$declared" ]]; then
        # The whole-workspace form. Declarations live in each package's own
        # manifest, which `cargo metadata` reports; a package's system
        # libraries are a property of the package, so there is one lookup for
        # developer hosts and CI alike. The WSL2 guest has no cargo, but CI
        # never uses this form there — it passes the explicit list the scope
        # job computed.
        if ! command -v cargo &> /dev/null; then
            echo "cargo is required to read package native declarations; pass an" >&2
            echo "explicit package list instead (CI does)." >&2
            exit 1
        fi
        if ! command -v jq &> /dev/null; then
            if [[ "$runner_key" == "windows-latest" ]]; then
                # No apt/brew here; winget is the only system package manager.
                if command -v winget &> /dev/null; then
                    echo "Installing jq (needed to read package native declarations)..."
                    winget install --id jqlang.jq -e --silent \
                        --accept-source-agreements --accept-package-agreements || true
                    hash -r
                fi
                if ! command -v jq &> /dev/null; then
                    echo "Cannot verify Windows native prerequisites: jq is not installed." >&2
                    echo "Install jq with: winget install jqlang.jq — then re-run just init." >&2
                    exit 1
                fi
            elif [[ -z "$pm" ]]; then
                echo "Could not detect a package manager; install jq, then run just init again." >&2
                exit 1
            else
                echo "Installing jq (needed to read package native declarations)..."
                install_packages jq
            fi
        fi
        declared=$(cargo metadata --no-deps --format-version 1 \
            | jq -r --arg k "$runner_key" \
                '[.packages[].metadata.ci.native[$k] // []] | add // [] | unique | .[]')
    fi

    if [[ -z "$declared" ]]; then
        echo "no native prerequisites declared for $runner_key"
        exit 0
    fi

    # Manifests name packages the way the CI runner does (apt on Linux, brew on
    # macOS). Each row adds the pkg-config module that proves the library is
    # installed — the same check the failing `-sys` build scripts perform — plus
    # the equivalent package name on the other Linux package managers.
    # A row may leave the pkg-config module empty. That is the runtime-binary
    # case: espeak-ng is not linked against, it is executed, so no `.pc` file
    # proves what matters and `is_installed` falls through to the package
    # database and finally to `command -v`.
    #   ci-name|pkg-config module|dnf|pacman|apk
    native_map="
    libasound2-dev|alsa|alsa-lib-devel|alsa-lib|alsa-lib-dev
    libpulse-dev|libpulse|pulseaudio-libs-devel|libpulse|pulseaudio-dev
    libgtk-3-dev|gtk+-3.0|gtk3-devel|gtk3|gtk+3.0-dev
    libwebkit2gtk-4.1-dev|webkit2gtk-4.1|webkit2gtk4.1-devel|webkit2gtk-4.1|webkit2gtk-4.1-dev
    libdbus-1-dev|dbus-1|dbus-devel|dbus|dbus-dev
    espeak-ng||espeak-ng|espeak-ng|espeak-ng
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
        # Last resort for a package that ships an executable of the same name.
        # Without this a dnf/pacman/apk host reinstalls it on every `just init`,
        # because nothing above can see it.
        command -v "$pkg" &> /dev/null
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

# `just init` step: own kache end to end on hosts whose filesystem earns it
# (fixes/2026-09-23-ensuring-kache-support). The FILESYSTEM decides — via the
# shared probe `scripts/kache-host.sh qualify`, never the OS name. Qualifying
# hosts run the spec §4 order: install/upgrade to latest, place the store from
# `kache doctor`, write `[cache] local_store` + `ignore_env = true` into the
# user config (the single source of truth — always written, even when it
# equals the default, because the pin is deliberate) and confirm this
# environment's kache loads that file and resolves the pinned store, re-assert
# the macOS DYLD_* passthrough gate, then the daemon lifecycle strictly after
# the config write, confirm the daemon loaded that file too, and activation
# LAST. Any pre-activation failure (a KACHE_CONFIG override included, named
# in the warning) prints WARNING lines,
# leaves kache off (undoing an earlier activation), and `just init` completes
# its other steps. Non-qualifying hosts never get kache installed; a
# below-floor install there needs an interactively confirmed binary-only
# upgrade (a non-interactive init errors instead), and an already-active
# wrapper is reported as drift and left to the human.
_ensure-kache:
    #!/usr/bin/env bash
    set -euo pipefail
    source scripts/cargo-path.sh

    # CI guard (2026-09-23 ruling): CI never installs or upgrades kache, and
    # if a workflow ever runs `just init`, init must not fight the runner's
    # own cache setup.
    if [[ -n "${CI:-}" || -n "${GITHUB_ACTIONS:-}" ]]; then
        echo "kache: CI environment detected — kache management left to the runner; nothing installed or upgraded"
        exit 0
    fi

    warn() { echo -e "{{ YELLOW }}WARNING{{ RESET }}: $*"; }
    dev_id() {
        case "$(uname -s)" in
            Darwin) stat -f %d "$1" ;;
            *)      stat -c %d "$1" ;;
        esac
    }
    cargo_home="${CARGO_HOME:-$HOME/.cargo}"
    # The one host file Cargo reads: a legacy extensionless `config` hides a
    # `config.toml` beside it, so activation and its undo go where Cargo looks.
    host_config="$cargo_home/config.toml"
    [[ -e "$cargo_home/config" ]] && host_config="$cargo_home/config"
    meets_floor() {
        [[ "$(printf '%s\n%s\n' "{{ KACHE_MIN_VERSION }}" "$1" | sort -V | head -1)" == "{{ KACHE_MIN_VERSION }}" ]]
    }
    installed_version() { kache --version 2>/dev/null | cut -d' ' -f2; }

    # Failure contract (spec §4): a pre-activation failure ends the kache
    # sequence with WARNING lines and kache OFF — `just init` itself continues.
    # An activation an earlier run wrote is undone too, because a wrapper
    # sitting on top of a broken kache is exactly the dyld failure this fix
    # exists for. (Activation is the last step, so nothing follows it.)
    # "OFF" is a claim about the wrapper Cargo would EFFECTIVELY use, so it is
    # re-decided after the undo: an inherited RUSTC_WRAPPER or
    # CARGO_BUILD_RUSTC_WRAPPER, an unwritable config, or a config this helper
    # does not edit keeps kache active, and that is reported as a manual-action
    # state instead. It still exits 0: init must finish its other steps, and
    # the fix is the human's (an environment variable in their shell).
    kache_off() {
        warn "$1"
        local wrapper_out wrapper_rc=0 line kind rest src
        wrapper_out="$(./scripts/kache-host.sh wrapper 2>&1)" || wrapper_rc=$?
        if [[ $wrapper_rc -eq 0 ]] \
            && grep -qF "kind=kache source=$host_config value=" <<<"$wrapper_out"; then
            if python3 scripts/kache-config-merge.py "$host_config" build rustc-wrapper ""; then
                warn "an existing activation was neutralized in $host_config (dated backup kept beside it)."
            else
                warn "could not neutralize the existing activation in $host_config."
            fi
            wrapper_rc=0
            wrapper_out="$(./scripts/kache-host.sh wrapper 2>&1)" || wrapper_rc=$?
        fi
        if [[ $wrapper_rc -ne 0 ]]; then
            warn "kache MAY STILL BE ACTIVE — manual action required: the rustc wrapper cannot be decided ($(sed -n 's/^kache-host: error=//p' <<<"$wrapper_out" | head -1))."
            echo "         fix that Cargo config, then confirm with 'just kache-status'; 'just init' continues with its other setup steps."
        elif grep -q '^kache-host: wrapper=kache ' <<<"$wrapper_out"; then
            warn "kache STILL ACTIVE — manual action required; Cargo keeps wrapping rustc with kache through:"
            # Every kache source down to the first one that sets another
            # value: undoing only the winner would expose the next.
            while IFS= read -r line; do
                kind="${line#* kind=}"; kind="${kind%% *}"
                [[ "$kind" == "kache" ]] || break
                rest="${line#* source=}"
                src="${rest%% value=*}"
                if [[ "$line" == "scope=env "* ]]; then
                    echo "         environment $src — undo: 'unset $src' in the shell that runs init, and drop it wherever it is exported"
                else
                    echo "         $src — undo: set [build] rustc-wrapper = \"\" there, or delete that line"
                fi
            done < <(sed -n 's/^kache-host: wrapper-source //p' <<<"$wrapper_out")
            echo "         confirm with 'just kache-status'; 'just init' continues with its other setup steps."
        else
            warn "kache left OFF; 'just init' continues with its other setup steps."
        fi
        exit 0
    }

    # (1) filesystem qualification — needs no kache installed
    probe_out=""
    probe_rc=0
    probe_out="$(./scripts/kache-host.sh qualify 2>&1)" || probe_rc=$?
    echo "$probe_out"
    echo

    if [[ $probe_rc -eq 2 ]]; then
        kache_off "the qualification probe could not run (see above); kache left off."
    fi

    if [[ $probe_rc -ne 0 ]]; then
        # Non-qualifying: the sequence ends at step (1) — one reason line, the
        # floor check when kache is installed, drift reporting; never an
        # install (spec §3).
        reason="$(sed -n 's/^kache-host: verdict=no-qualify reason=//p' <<<"$probe_out" | head -1)"
        echo "kache: this filesystem does not earn kache (${reason:-see probe output above}); kache stays off."
        if command -v kache &> /dev/null; then
            installed="$(installed_version)"
            if meets_floor "$installed"; then
                echo "kache: $installed present and meets the floor {{ KACHE_MIN_VERSION }} — report only."
            elif [[ -t 0 ]]; then
                read -r -p "kache $installed is below the floor {{ KACHE_MIN_VERSION }}. Upgrade to the latest release now, binary-only? [y/N] " answer
                if [[ "${answer:-n}" == "y" || "${answer:-n}" == "Y" ]]; then
                    just install-kache true
                else
                    echo "kache: below-floor install left as is — 'just install-kache true' upgrades it by hand."
                fi
            else
                echo "ERROR: kache $installed is below the floor {{ KACHE_MIN_VERSION }} and this init is non-interactive;" >&2
                echo "       refusing to upgrade or skip silently. Run 'just install-kache true' by hand, then" >&2
                echo "       re-run 'just init'." >&2
                exit 1
            fi
        else
            echo "kache: not installed — init leaves it that way ('just install-kache' installs it by hand)."
        fi
        # The same effective-wrapper decision `kache-status` makes, so an
        # empty (neutralized) or foreign wrapper is not reported as drift.
        wrapper_rc=0
        wrapper_out="$(./scripts/kache-host.sh wrapper 2>&1)" || wrapper_rc=$?
        if [[ $wrapper_rc -ne 0 ]]; then
            warn "could not decide whether kache is active ($(sed -n 's/^kache-host: error=//p' <<<"$wrapper_out" | head -1)). This init changed nothing."
        elif grep -q '^kache-host: wrapper=kache ' <<<"$wrapper_out"; then
            warn "kache is ACTIVE on a filesystem that does not earn it — restores are copies. This init changed nothing."
            echo "         undo, this shell : export RUSTC_WRAPPER=\"\"   (the empty value wins over the config file)"
            echo "         undo, host-wide  : neutralize or remove the rustc-wrapper line in $host_config"
        fi
        exit 0
    fi

    candidate="$(sed -n 's/^kache-host: verdict=qualify candidate=//p' <<<"$probe_out" | head -1)"
    echo "kache: filesystem qualifies — running the init-owned sequence."

    # (2) install or upgrade to the latest release (full mode)
    just install-kache || kache_off "install-kache failed; the kache binary is not verified usable."
    installed="$(installed_version)"

    # (3) placement: `kache doctor` reads the store the runtime resolves
    # today. Already on the serving device → that IS the store (still pinned in
    # the config file; spec §2 ruling); otherwise the probe's cascade
    # candidate. A doctor failure here makes placement undecidable — same skip
    # bucket as any other pre-activation failure.
    report_out=""
    report_rc=0
    report_out="$(./scripts/kache-host.sh report 2>&1)" || report_rc=$?
    if [[ $report_rc -ne 0 ]]; then
        kache_off "kache doctor failed after install ($(sed -n 's/^kache-host: error=//p' <<<"$report_out" | head -1)) — store placement is undecidable."
    fi
    doctor_store="$(sed -n 's/^kache-host: store=\(.*\) source=doctor$/\1/p' <<<"$report_out" | head -1)"
    checkout_dev="$(dev_id "$PWD")"
    if [[ -n "$doctor_store" && "$(dev_id "$doctor_store" 2>/dev/null || echo "?")" == "$checkout_dev" ]]; then
        store="$doctor_store"
        moved="no — already the store kache resolves; pinned deliberately"
    elif [[ -n "$candidate" && "$candidate" != "-" ]]; then
        store="$candidate"
        moved="yes — from ${doctor_store:-the kache default} to $store (placement cascade, spec §2)"
    else
        kache_off "no decidable store placement: doctor resolves '${doctor_store:-(nothing)}' off-device and the probe named no cascade candidate."
    fi

    # (4) write the user config — the single source of truth every process
    # reads (shells, daemon, editors, launchd jobs). Always written, even when
    # it equals the default. The pre/post CONTENT snapshot (not mtime) feeds
    # the daemon-restart trigger in step (6).
    kache_config="$(./scripts/kache-host.sh config-path)"
    snapshot="$(mktemp)"
    trap 'rm -f "$snapshot"' EXIT
    if [[ -f "$kache_config" ]]; then cp "$kache_config" "$snapshot"; else : > "$snapshot"; fi
    python3 scripts/kache-config-merge.py "$kache_config" cache local_store "$store" ignore_env true \
        || kache_off "could not write [cache] local_store/ignore_env into $kache_config."
    config_changed=0
    cmp -s "$snapshot" "$kache_config" || config_changed=1

    # The pin is the single source of truth only while every kache process
    # loads $kache_config. A non-empty KACHE_CONFIG selects another file, and
    # `ignore_env = true` does not gate that selector — so re-read which file
    # this environment's kache (and, after step 6, the daemon) loads, and
    # which store doctor now resolves, before anything is activated.
    # check_config_source cli|daemon
    check_config_source() {
        local scope="$1" out rc=0 line state selector path pin_line
        out="$(./scripts/kache-host.sh config-source 2>&1)" || rc=$?
        if [[ $rc -ne 0 ]]; then
            kache_off "could not confirm which config kache loads after the pin write ($(sed -n 's/^kache-host: error=//p' <<<"$out" | head -1))."
        fi
        line="$(sed -n "s/^kache-host: config-source scope=$scope //p" <<<"$out" | head -1)"
        state="${line#state=}"; state="${state%% *}"
        selector="${line#* selector=}"; selector="${selector%% *}"
        path="${line#* path=}"
        if [[ "$scope" == cli && "$state" == override ]]; then
            warn "kache CONFIG OVERRIDE — this environment exports $selector=$path, so kache reads that file, not $kache_config;"
            echo "         the store pinned there never reaches builds run with it ('ignore_env = true' does not stop this selector)."
            echo "         undo: 'unset $selector' in the shell that runs init and cargo, drop it wherever it is exported, then re-run 'just init'"
            kache_off "the single-source store pin does not hold for this environment."
        elif [[ "$scope" == daemon && "$state" == override ]]; then
            warn "kache CONFIG OVERRIDE — the running kache daemon loaded $path ($selector), not $kache_config;"
            echo "         its launch environment selects another config (KACHE_CONFIG=$path), so it serves another store."
            echo "         undo: remove KACHE_CONFIG from the daemon's service environment, 'kache daemon restart', then re-run 'just init'"
            kache_off "the single-source store pin does not hold for the daemon."
        elif [[ "$state" != managed ]]; then
            kache_off "could not confirm which config the kache $scope loads ('${state:-no report}') — the single-source store pin is unverified."
        fi
        if [[ "$scope" == cli ]]; then
            pin_line="$(sed -n 's/^kache-host: config pin=//p' <<<"$out" | head -1)"
            if [[ "${pin_line%% *}" != match ]]; then
                kache_off "kache resolves the store '$(sed -n 's/^kache-host: store=\(.*\) source=doctor$/\1/p' <<<"$out" | head -1)', not the '$store' just pinned in $kache_config."
            fi
        fi
    }
    check_config_source cli

    # (5) re-assert the macOS passthrough gate — idempotent when install-kache
    # already re-signed; the gate is the verification, not the re-sign.
    if [[ "$(uname -s)" == "Darwin" ]] && ! ./scripts/kache-host.sh probe-passthrough; then
        kache_off "env-passthrough verification failed — a wrapped compiler would not receive DYLD_* variables."
    fi

    # (6) daemon lifecycle — strictly after the config write so the daemon's
    # first read of the store is the ratified one. Install when absent, ensure
    # running, restart when the running daemon's version mismatches the
    # installed binary or step (4) changed the config (content, not mtime).
    read_daemon() {
        daemon_running=no daemon_installed=no daemon_version=- daemon_socket=-
        local json state key value
        json="$(kache daemon --json 2>/dev/null)" || true
        [[ -n "$json" ]] || return 1
        state="$(python3 -c '
    import json, sys
    try:
        d = json.load(sys.stdin)
    except Exception:
        raise SystemExit(1)
    def field(key):
        value = d.get(key)
        return value if isinstance(value, str) and value else "-"
    print("running=" + ("yes" if d.get("daemon_running") else "no"))
    print("installed=" + ("yes" if d.get("service_installed") else "no"))
    print("version=" + field("daemon_version"))
    print("socket=" + field("socket"))
    ' <<<"$json")" || return 1
        while IFS='=' read -r key value; do
            case "$key" in
                running)   daemon_running="$value" ;;
                installed) daemon_installed="$value" ;;
                version)   daemon_version="$value" ;;
                socket)    daemon_socket="$value" ;;
            esac
        done <<<"$state"
    }
    # Ready means running AT THE INSTALLED VERSION, whatever brought it up: a
    # service manager can report a restart as done while the old daemon is
    # still the one answering, and activating on top of it breaks the
    # same-version requirement (spec §3). Bounded: one read per second.
    wait_daemon() {
        local tries="$1" i
        for ((i = 0; i < tries; i++)); do
            read_daemon || true
            [[ "$daemon_running" == "yes" && "$daemon_version" == "$installed" ]] && return 0
            sleep 1
        done
        return 1
    }
    daemon_not_ready() {
        if [[ "$daemon_running" == "yes" ]]; then
            kache_off "the kache daemon still runs version $daemon_version, not the installed $installed, after $1 — it must match the binary before activation."
        fi
        kache_off "the kache daemon is not running after $1."
    }

    read_daemon || kache_off "could not read the daemon state ('kache daemon --json')."
    if [[ "$daemon_installed" != "yes" ]]; then
        kache daemon install >/dev/null || kache_off "'kache daemon install' failed."
        read_daemon || true
    fi
    lifecycle_step="install/start"
    if [[ "$daemon_running" == "yes" ]]; then
        restart_why=""
        if [[ "$daemon_version" != "$installed" ]]; then
            restart_why="binary changed (daemon reports $daemon_version, installed $installed)"
        fi
        if [[ "$config_changed" == "1" ]]; then
            restart_why="${restart_why:+$restart_why; }config changed"
        fi
        if [[ -n "$restart_why" ]]; then
            echo "kache: restarting the daemon — $restart_why."
            kache daemon restart >/dev/null || kache_off "'kache daemon restart' failed."
            lifecycle_step="restart"
        fi
    fi
    if ! wait_daemon 10; then
        # Up but at another version: `start` cannot replace a running daemon.
        [[ "$daemon_running" == "yes" ]] && daemon_not_ready "$lifecycle_step"
        echo "kache: daemon not running — starting it (launchd throttles a relaunch for up to 10s after the binary changed)."
        kache daemon start >/dev/null 2>&1 || true
        wait_daemon 20 || daemon_not_ready "install/start/restart"
    fi

    check_config_source daemon

    # (7) activation — LAST, only after every prior check passed. Host-wide via
    # Cargo home; a tracked .cargo/config.toml wrapper stays forbidden.
    python3 scripts/kache-config-merge.py "$host_config" build rustc-wrapper kache \
        || kache_off "could not write the activation into $host_config."

    # (8) confirm the activation took: an inherited RUSTC_WRAPPER (even set
    # but empty), CARGO_BUILD_RUSTC_WRAPPER, or a repository/ancestor
    # .cargo/config outranks the host file, and then Cargo never runs kache.
    # A kache-NAMED winner counts only when it resolves to the kache on PATH
    # — the binary steps (2)-(6) verified; a missing path, or another
    # executable called kache, is not the kache init certified.
    # The host entry is kept — it is correct and takes effect once the
    # override goes — but the report says "not active" and names each source
    # above it. Still exit 0: init finishes its other steps (spec §4).
    active_out=""
    active_rc=0
    active_out="$(./scripts/kache-host.sh wrapper 2>&1)" || active_rc=$?
    activation_summary="[build] rustc-wrapper = \"kache\" in $host_config"
    active=yes
    if [[ $active_rc -ne 0 ]] || ! grep -q '^kache-host: wrapper=kache ' <<<"$active_out" \
        || ! grep -q '^kache-host: wrapper-binary state=certified ' <<<"$active_out"; then
        active=no
        activation_summary="$activation_summary — written, but NOT in effect (see WARNING below)"
    fi

    # One report block (spec §6)
    devices_line="$(sed -n 's/^kache-host: devices //p' <<<"$probe_out" | head -1)"
    base_line="$(sed -n 's/^kache-host: base=//p' <<<"$probe_out" | head -1)"
    base_state="${base_line%% *}"
    base_path="${base_line#*path=}"
    base_reason="$(sed -n 's/^[^ ]* reason=\([^ ]*\) .*/\1/p' <<<"$base_line")"
    case "$base_state" in
        covered)    base_summary="covered — ${base_path:-?} on the serving device" ;;
        off-device) base_summary="${base_path:-?} is on ANOTHER device than the store — no placement can serve both; kache-status reports this" ;;
        invalid)    base_summary="INVALID setting (${base_reason:-?}: ${base_path:-?}) — wt refuses it; not covered by this verdict, and kache-status fails until it is fixed" ;;
        *)          base_summary="unconfigured — not covered by this verdict" ;;
    esac
    case "$(uname -s)" in
        Darwin) builtin_default="$HOME/Library/Caches/kache" ;;
        *)      builtin_default="${XDG_CACHE_HOME:-$HOME/.cache}/kache" ;;
    esac
    abandoned="none found"
    if [[ "$builtin_default" != "$store" && -d "$builtin_default" ]] \
        && [[ -n "$(ls -A "$builtin_default" 2>/dev/null)" ]]; then
        abandoned="$(du -sh "$builtin_default" 2>/dev/null | cut -f1) at $builtin_default — abandoned; deleting it is left to you"
    fi
    if [[ "$(uname -s)" == "Darwin" ]]; then
        passthrough_summary="DYLD_* reaches the wrapped compiler (probe passed)"
    else
        passthrough_summary="n/a outside macOS"
    fi
    say() { printf '  %-14s %s\n' "$1" "$2"; }
    echo
    if [[ "$active" == "yes" ]]; then
        echo "kache: qualified and set up —"
    else
        echo "kache: qualified, but activation INCOMPLETE — Cargo does not run kache for this checkout —"
    fi
    say "verdict" "qualifies (${devices_line:-devices unknown})"
    say "worktree base" "$base_summary"
    say "version" "$installed (floor {{ KACHE_MIN_VERSION }})"
    say "passthrough" "$passthrough_summary"
    say "store" "$store — moved: $moved"
    say "old store" "$abandoned"
    say "daemon" "running, version $daemon_version, socket $daemon_socket"
    say "activation" "$activation_summary"
    [[ "$active" == "yes" ]] && exit 0

    echo
    if [[ $active_rc -ne 0 ]]; then
        warn "kache NOT CONFIRMED ACTIVE — the rustc wrapper cannot be decided ($(sed -n 's/^kache-host: error=//p' <<<"$active_out" | head -1))."
        echo "         fix that Cargo config, then confirm with 'just kache-status'; 'just init' continues with its other setup steps."
        exit 0
    fi
    binary_line="$(sed -n 's/^kache-host: wrapper-binary //p' <<<"$active_out" | head -1)"
    if [[ -n "$binary_line" ]]; then
        binary_state="${binary_line#state=}"; binary_state="${binary_state%% *}"
        binary_path="${binary_line#* path=}"; binary_path="${binary_path% certified=*}"
        certified="${binary_line##* certified=}"
        winner_source="$(sed -n 's/^kache-host: wrapper=kache source=\(.*\) value=.*$/\1/p' <<<"$active_out" | head -1)"
        case "$binary_state" in
            missing)        why="does not exist — Cargo fails every build" ;;
            not-executable) why="is not executable — Cargo fails every build" ;;
            *)              why="is not the kache init verified ($certified)" ;;
        esac
        warn "kache NOT ACTIVE — the rustc wrapper Cargo would run, $binary_path (set by $winner_source), $why."
        if [[ "$winner_source" == RUSTC_WRAPPER || "$winner_source" == CARGO_BUILD_RUSTC_WRAPPER ]]; then
            echo "         undo: 'unset $winner_source' in the shell that runs cargo, and drop it wherever it is exported"
        else
            [[ "$winner_source" == .cargo/* ]] && winner_source="$PWD/$winner_source"
            echo "         undo: set [build] rustc-wrapper = \"kache\" in $winner_source, or delete that line"
        fi
        echo "         confirm with 'just kache-status'; 'just init' continues with its other setup steps."
        exit 0
    fi
    warn "kache NOT ACTIVE — Cargo uses another rustc wrapper, which outranks the host entry just written:"
    # Every source above the host file: removing only the winner would expose
    # the next one.
    while IFS= read -r line; do
        rest="${line#* source=}"
        src="${rest%% value=*}"
        value="${line#* value=}"
        [[ "$src" == "$host_config" ]] && break
        [[ "$line" == *" kind=kache "* ]] && continue
        if [[ "$line" == "scope=env "* ]]; then
            echo "         environment $src=\"$value\" — undo: 'unset $src' in the shell that runs cargo, and drop it wherever it is exported"
        else
            [[ "$line" == "scope=repo "* ]] && src="$PWD/$src"
            echo "         $src sets \"$value\" — undo: delete its [build] rustc-wrapper line"
        fi
    done < <(sed -n 's/^kache-host: wrapper-source //p' <<<"$active_out")
    echo "         the host entry stays in $host_config and applies once those are gone;"
    echo "         confirm with 'just kache-status'; 'just init' continues with its other setup steps."

# The ownership split (2026-09-23 ruling): this recipe owns the BINARY —
# install/upgrade to latest, the macOS ad hoc re-sign with the passthrough
# verification as the gate, and (full mode only) the default store-cap seed —
# and does no daemon work in either mode. `_ensure-kache` owns everything
# else on qualifying hosts: store placement, the user config write, the daemon
# lifecycle (strictly after the config write), and activation last.

# install or upgrade the Rust compiler cache to the LATEST release (the floor
# in .github/kache-min-version is a check the recipes apply, never a pin). On
# macOS every install or upgrade re-signs the binary ad hoc and is GATED on the
# env-passthrough probe — the binstall release ships with the hardened
# runtime, and dyld strips every DYLD_* variable from a hardened process at
# launch, which is the rust-lld/libLLVM.dylib link failure this fix exists
# for — with a source install (ad hoc-signed by construction) as the fallback.
# binary_only=true ("just install-kache true", passed positionally) performs
# no config writes at all.
install-kache binary_only="false":
    #!/usr/bin/env bash
    set -euo pipefail
    source scripts/cargo-path.sh

    if [[ "{{ binary_only }}" != "true" && "{{ binary_only }}" != "false" ]]; then
        echo "install-kache: binary_only must be exactly 'true' or 'false', passed positionally:" >&2
        echo "  just install-kache        # full: binary (+ default store-cap seed on a config-less host)" >&2
        echo "  just install-kache true   # binary-only: no config writes, no daemon work" >&2
        exit 2
    fi

    # cargo-binstall is the install path on every OS: it fetches a prebuilt
    # kache binary instead of compiling from source. Install it first when
    # absent (that one install is from source). Plain --no-confirm (no
    # --force) still installs or upgrades to the latest release and is a
    # no-op when the latest is already installed — the floor elsewhere is a
    # check, not a pin, and a second `just init` changes nothing.
    if ! command -v cargo-binstall &> /dev/null; then
        echo "Installing cargo-binstall (used to fetch prebuilt kache binaries)..."
        RUSTC_WRAPPER="" cargo install --locked cargo-binstall
    fi
    RUSTC_WRAPPER="" cargo binstall --no-confirm kache

    if [[ "$(uname -s)" == "Darwin" ]]; then
        # The gate is the verification, so the probe runs FIRST: an already
        # re-signed binary passes and is left untouched (re-signing rewrites
        # the file, which the running daemon treats as a binary change and
        # restarts itself over), while any fresh binstall release is
        # hardened, fails the probe, and is re-signed — so no upgrade path
        # can skip the re-sign.
        if ! ./scripts/kache-host.sh probe-passthrough; then
            codesign --force -s - "$(command -v kache)"
            if ! ./scripts/kache-host.sh probe-passthrough; then
                echo "install-kache: the re-signed binary still fails the env-passthrough probe;" >&2
                echo "                falling back to a source install, which is ad hoc-signed by construction" >&2
                # --force is deliberate here and nowhere else: the fallback
                # exists to REPLACE a binary that cannot pass the gate, which
                # cargo install would skip as "already installed".
                RUSTC_WRAPPER="" cargo install --locked --force kache
                ./scripts/kache-host.sh probe-passthrough
            fi
        fi
    fi

    if [[ "{{ binary_only }}" == "false" ]]; then
        # Seed a default store config when the host has none. An uncapped store
        # thrashes: LRU can evict fresh entries before they score a hit. 100 GiB is
        # the agreed starting point (docs/kache-strategy.md); never overwrite an
        # existing config — hosts size against their own volume. A read-only
        # config directory (build-linux mounts ~/.config over CIFS) is reported,
        # not fatal, so `just init` still completes there.
        case "$(uname -s)" in
            MINGW*|MSYS*|CYGWIN*) kache_config_dir="$(cygpath "${APPDATA:?}")/kache" ;;
            *)                    kache_config_dir="${XDG_CONFIG_HOME:-$HOME/.config}/kache" ;;
        esac
        if [[ ! -f "$kache_config_dir/config.toml" ]]; then
            if mkdir -p "$kache_config_dir" 2> /dev/null \
                && printf '[cache]\nlocal_max_size = "100GiB"\n' > "$kache_config_dir/config.toml" 2> /dev/null; then
                echo "Wrote default kache store cap (100GiB) to $kache_config_dir/config.toml"
            else
                echo "WARNING: could not write $kache_config_dir/config.toml; the store cap defaults to kache's own (50GiB)."
            fi
        fi
    fi

    kache --version
    echo
    echo "kache installed or upgraded (above), verified where the OS requires it (macOS: DYLD_* passthrough)."
    if [[ "{{ binary_only }}" == "true" ]]; then
        echo "binary-only mode: no config written, no daemon touched."
    else
        echo "Nothing here activates it and no daemon is touched:"
        echo "  qualifying host : 'just init' owns placement, daemon, and activation (activation is last)"
        echo "  by hand         : 'just kache-status' rules whether THIS filesystem earns kache"
    fi

# The durable guard. Init's verdict is point-in-time, but the worktree base
# can be configured — or re-pointed — after activation and binaries can be
# replaced, so this recipe re-checks the facts through the script init used
# (`scripts/kache-host.sh report`): the store is whatever `kache doctor`
# resolves (never a reconstruction — the pre-2026-09-23 recipe guessed from
# KACHE_DIR and reported false verdicts), plus the checkout/worktree-base
# device check, init's clone check re-run from that store, the macOS
# env-passthrough result, the daemon (service installed, running, same version
# as the binary), and the user config's `local_store` pin and `ignore_env`.
# Exits non-zero on drift while kache is active; with kache not in use every
# fact is still printed, but nothing is judged.

# report whether kache is active here, and whether this filesystem earns it
kache-status:
    #!/usr/bin/env bash
    set -uo pipefail

    say() { printf '  %-12s %s\n' "$1" "$2"; }
    echo "=== kache status — policy: docs/kache-strategy.md ==="

    installed="-" below_floor=0
    if command -v kache &> /dev/null; then
        installed="$(kache --version 2>/dev/null | cut -d' ' -f2)"
        if [[ "$(printf '%s\n%s\n' "{{ KACHE_MIN_VERSION }}" "$installed" | sort -V | head -1)" == "{{ KACHE_MIN_VERSION }}" ]]; then
            say "installed" "$installed (floor {{ KACHE_MIN_VERSION }})"
        else
            below_floor=1
            say "installed" "$installed — BELOW the floor {{ KACHE_MIN_VERSION }}; 'just install-kache' upgrades"
        fi
    else
        say "installed" "no — 'just install-kache' installs it ('just init' owns activation on qualifying hosts)"
    fi

    # The wrapper Cargo would use, decided by the same helper `_ensure-kache`
    # uses. Every source that names kache is listed, not only the winner: a
    # shadowed activation takes over once the one above it is undone. A
    # kache-named winner is healthy only when it resolves to the kache on
    # PATH, the binary whose version and passthrough are checked below.
    cargo_home="${CARGO_HOME:-$HOME/.cargo}"
    active="" wrapper_error="" wrapper_problem=""
    if wrapper_out="$(./scripts/kache-host.sh wrapper 2>&1)"; then
        winner="$(sed -n 's/^kache-host: wrapper=//p' <<<"$wrapper_out" | head -1)"
        winner_kind="${winner%% *}"
        winner_rest="${winner#* source=}"
        winner_source="${winner_rest%% value=*}"
        binary_line="$(sed -n 's/^kache-host: wrapper-binary //p' <<<"$wrapper_out" | head -1)"
        binary_state="${binary_line#state=}"; binary_state="${binary_state%% *}"
        binary_path="${binary_line#* path=}"; binary_path="${binary_path% certified=*}"
        binary_certified="${binary_line##* certified=}"
        case "$binary_state" in
            certified) ;;
            missing)        wrapper_problem="Cargo would run the rustc wrapper $binary_path, which does not exist — every build fails" ;;
            not-executable) wrapper_problem="Cargo would run the rustc wrapper $binary_path, which is not executable — every build fails" ;;
            different)      wrapper_problem="Cargo would run the rustc wrapper $binary_path, not the kache on PATH ($binary_certified) whose version and passthrough are checked here" ;;
            *)              wrapper_problem="the helper did not resolve the kache wrapper to an executable" ;;
        esac
        while IFS= read -r line; do
            scope="${line#*scope=}"; scope="${scope%% *}"
            kind="${line#* kind=}"; kind="${kind%% *}"
            rest="${line#* source=}"
            src="${rest%% value=*}"
            value="${rest#* value=}"
            case "$scope" in
                env)  where="environment $src=$value" ;;
                repo) where="repo $src (tracked wrapper is forbidden here)" ;;
                ancestor) where="$src (a parent directory of this checkout)" ;;
                *)    where="$src (host-wide: every repo on this machine)" ;;
            esac
            if [[ "$src" == "$winner_source" && "$kind" == "kache" && -n "$wrapper_problem" ]]; then
                active="$where"
                say "active" "BROKEN — $where names kache, but resolves to $binary_path ($binary_state)"
            elif [[ "$src" == "$winner_source" && "$kind" == "kache" ]]; then
                active="$where"
                say "active" "YES — $where"
            elif [[ "$src" == "$winner_source" && "$kind" == "none" ]]; then
                say "active" "no — $where sets an empty wrapper, which disables wrapping"
            elif [[ "$src" == "$winner_source" ]]; then
                say "active" "no — $where wraps rustc with '$value', not kache"
            elif [[ "$kind" == "kache" ]]; then
                say "shadowed" "kache in $where — takes effect if $winner_source is undone"
            fi
        done < <(sed -n 's/^kache-host: wrapper-source //p' <<<"$wrapper_out")
        [[ "$winner_kind" == "none" && "$winner_source" == "-" ]] && say "active" "no — nothing sets a rustc wrapper"
    else
        wrapper_error="$(sed -n 's/^kache-host: error=//p' <<<"$wrapper_out" | head -1)"
        say "active" "UNKNOWN — cannot decide the rustc wrapper (${wrapper_error:-wrapper helper failed})"
    fi

    # Daemon, config pin, store, devices, worktree base, clone checks, and
    # passthrough all come from the shared script; the clone check is init's,
    # run from the store kache resolves now, so a store that moved or never
    # cloned shows as drift. The daemon and config lines precede doctor, so
    # they are judged even when doctor fails.
    store="-" store_dev="-" checkout_dev="-" base_state="unknown" base_path="-" base_reason="" passthrough="n/a" report_failed=0
    checkout_clone="-" checkout_clone_reason="" base_clone="-" base_clone_reason=""
    daemon_line="" config_state="" config_path="-" ignore_env="" pin="" pin_value=""
    cli_source_state="" cli_source_path="" daemon_source_state="" daemon_source_path=""
    if command -v kache &> /dev/null; then
        report_out="$(./scripts/kache-host.sh report 2>&1)"
        report_rc=$?
        daemon_line="$(sed -n 's/^kache-host: daemon //p' <<<"$report_out" | head -1)"
        config_line="$(sed -n 's/^kache-host: config state=//p' <<<"$report_out" | head -1)"
        config_state="${config_line%% *}"
        [[ "$config_line" == *" path="* ]] && config_path="${config_line#* path=}"
        ignore_env="$(sed -n 's/^kache-host: config ignore_env=//p' <<<"$report_out" | head -1)"
        pin_line="$(sed -n 's/^kache-host: config pin=//p' <<<"$report_out" | head -1)"
        pin="${pin_line%% *}"
        pin_value="${pin_line#* value=}"
        # Which file kache here and the running daemon actually load: a
        # KACHE_CONFIG selector bypasses the managed file, `ignore_env` or not.
        for scope in cli daemon; do
            source_line="$(sed -n "s/^kache-host: config-source scope=$scope //p" <<<"$report_out" | head -1)"
            source_state="${source_line#state=}"; source_state="${source_state%% *}"
            source_path="${source_line#* path=}"
            printf -v "${scope}_source_state" '%s' "$source_state"
            printf -v "${scope}_source_path" '%s' "$source_path"
        done
        daemon_installed="-" daemon_running="-" daemon_version="-"
        if [[ "$daemon_line" == installed=* ]]; then
            read -r daemon_installed daemon_running daemon_version <<<"$daemon_line"
            daemon_installed="${daemon_installed#installed=}"
            daemon_running="${daemon_running#running=}"
            daemon_version="${daemon_version#version=}"
            say "daemon" "service installed: $daemon_installed, running: $daemon_running, version $daemon_version"
        else
            say "daemon" "UNKNOWN — 'kache daemon --json' gave no readable state"
        fi
        pin_display="unknown (doctor did not report the store)"
        [[ -n "$pin" ]] && pin_display="${pin_value:-unset}"
        case "$config_state" in
            present) say "config" "$config_path — local_store $pin_display, ignore_env ${ignore_env:-?}" ;;
            absent)  say "config" "$config_path — missing" ;;
            *)       say "config" "$config_path — cannot be read (${config_state:-no report})" ;;
        esac
        [[ "$cli_source_state" == "override" ]] \
            && say "override" "KACHE_CONFIG=$cli_source_path — kache in this shell reads that file instead"
        [[ "$daemon_source_state" == "override" ]] \
            && say "override" "the running daemon loaded $daemon_source_path instead"
        if [[ $report_rc -ne 0 ]]; then
            report_failed=1
            say "store" "cannot report ($(sed -n 's/^kache-host: error=//p' <<<"$report_out" | head -1))"
        else
            store="$(sed -n 's/^kache-host: store=\(.*\) source=doctor$/\1/p' <<<"$report_out" | head -1)"
            devices_line="$(sed -n 's/^kache-host: devices //p' <<<"$report_out" | head -1)"
            checkout_dev="$(cut -d' ' -f1 <<<"$devices_line" | cut -d= -f2-)"
            store_dev="$(cut -d' ' -f2 <<<"$devices_line" | cut -d= -f2-)"
            base_line="$(sed -n 's/^kache-host: base=//p' <<<"$report_out" | head -1)"
            base_state="${base_line%% *}"
            base_path="${base_line#*path=}"
            base_reason="$(sed -n 's/^[^ ]* reason=\([^ ]*\) .*/\1/p' <<<"$base_line")"
            passthrough="$(sed -n 's/^kache-host: passthrough=//p' <<<"$report_out" | head -1 | cut -d' ' -f1)"
            checkout_clone_line="$(sed -n 's/^kache-host: clone checkout=//p' <<<"$report_out" | head -1)"
            checkout_clone="${checkout_clone_line%% *}"
            [[ "$checkout_clone_line" == *" reason="* ]] && checkout_clone_reason="${checkout_clone_line#* reason=}"
            base_clone_line="$(sed -n 's/^kache-host: clone base=//p' <<<"$report_out" | head -1)"
            base_clone="${base_clone_line%% *}"
            [[ "$base_clone_line" == *" reason="* ]] && base_clone_reason="${base_clone_line#* reason=}"
            say "store" "$store (from kache doctor)"
            say "devices" "$devices_line"
            say "clone" "store -> checkout: ${checkout_clone:-?}${checkout_clone_reason:+ ($checkout_clone_reason)}"
            case "$base_state" in
                covered)    say "worktree base" "$base_path — same device as the store; store -> base: ${base_clone:-?}${base_clone_reason:+ ($base_clone_reason)}" ;;
                off-device) say "worktree base" "$base_path — ANOTHER device than the store" ;;
                invalid)    say "worktree base" "INVALID setting (${base_reason:-?}: $base_path) — wt refuses it" ;;
                *)          say "worktree base" "unconfigured — not covered by this check" ;;
            esac
            if [[ "$passthrough" == "n/a" ]]; then
                say "passthrough" "n/a outside macOS"
            else
                say "passthrough" "$passthrough (DYLD_* through the wrapped compiler)"
            fi
        fi
    else
        say "store" "unknown — kache is not installed"
    fi

    echo
    if [[ -z "$active" && -z "$wrapper_error" ]]; then
        echo "  VERDICT: not in use. Cargo builds without kache; nothing of kache's to undo."
        exit 0
    fi

    problems=()
    if [[ -n "$wrapper_error" ]]; then
        problems+=("the rustc wrapper Cargo would use is undecidable: $wrapper_error")
    elif ! command -v kache &> /dev/null; then
        problems+=("kache is not installed, but a rustc wrapper is active — every build would fail")
    else
        [[ -n "$wrapper_problem" ]] && problems+=("$wrapper_problem")
        [[ "$below_floor" == "1" ]] \
            && problems+=("kache $installed is below the floor {{ KACHE_MIN_VERSION }} — 'just install-kache' upgrades")
        if [[ "$daemon_line" != installed=* ]]; then
            problems+=("the daemon state is unreadable ('kache daemon --json' failed)")
        else
            [[ "$daemon_installed" == "yes" ]] \
                || problems+=("the kache daemon service is not installed — 'kache daemon install'")
            if [[ "$daemon_running" != "yes" ]]; then
                problems+=("the kache daemon is not running — 'kache daemon start'")
            elif [[ "$daemon_version" != "$installed" ]]; then
                problems+=("the running daemon is kache $daemon_version but the installed binary is $installed — 'kache daemon restart'")
            fi
        fi
        # The pin and ignore_env in the user config are the store's single
        # source of truth (spec §2); 'just init' writes both. The pin can only
        # be judged against the store doctor resolves.
        case "$config_state" in
            present | absent)
                case "$ignore_env" in
                    true)  ;;
                    false) problems+=("[cache] ignore_env is false in $config_path — KACHE_* environment overrides can move the store") ;;
                    *)     problems+=("[cache] ignore_env is missing from $config_path — KACHE_* environment overrides can move the store") ;;
                esac
                if [[ "$report_failed" != "1" ]]; then
                    case "$pin" in
                        match)    ;;
                        mismatch) problems+=("[cache] local_store ($pin_value) in $config_path is not the store kache resolves ($store)") ;;
                        *)        problems+=("[cache] local_store is missing from $config_path — the store is not pinned; 'just init' writes it") ;;
                    esac
                fi
                ;;
            *) problems+=("the kache user config $config_path cannot be read (${config_state:-no report}) — the store pin is undecidable") ;;
        esac
        # The pin is the single source of truth only while every kache
        # process loads that file; `ignore_env` does not gate KACHE_CONFIG.
        case "$cli_source_state" in
            managed)  ;;
            override) problems+=("this shell exports KACHE_CONFIG=$cli_source_path, so kache reads that file, not $config_path — the store pin is bypassed; 'unset KACHE_CONFIG' wherever it is exported") ;;
            *)        problems+=("cannot tell which config file kache loads here (${cli_source_state:-no report})") ;;
        esac
        if [[ "$daemon_running" == "yes" ]]; then
            case "$daemon_source_state" in
                managed)  ;;
                override) problems+=("the running daemon loaded $daemon_source_path, not $config_path — its launch environment selects another config (KACHE_CONFIG); remove it there and 'kache daemon restart'") ;;
                *)        problems+=("the running daemon does not report which config file it loaded — the store pin is unconfirmed for it") ;;
            esac
        fi
        if [[ "$report_failed" == "1" ]]; then
            problems+=("kache doctor could not report the store — the store location is undecidable")
        else
            if [[ "$store_dev" != "-" && "$store_dev" != "$checkout_dev" ]]; then
                problems+=("the store ($store, device $store_dev) is on another device than this checkout (device $checkout_dev) — restores are copies")
            elif [[ "$checkout_clone" != "clone" ]]; then
                problems+=("the store ($store) cannot clone into this checkout: ${checkout_clone_reason:-no clone check reported} — restores are copies")
            fi
            if [[ "$base_state" == "invalid" ]]; then
                problems+=("the worktree base setting is invalid (${base_reason:-?}: $base_path) — wt refuses it, so worktree coverage is undecidable")
            elif [[ "$base_state" == "off-device" ]]; then
                problems+=("the worktree base ($base_path) is on another device than the store — no placement can serve both")
            elif [[ "$base_state" == "covered" && "$base_clone" != "clone" ]]; then
                problems+=("the store ($store) cannot clone into the worktree base ($base_path): ${base_clone_reason:-no clone check reported} — worktrees there restore by copy")
            fi
            case "$passthrough" in
                fail)  problems+=("the wrapped compiler does not receive DYLD_* — re-sign: codesign --force -s - \$(command -v kache)") ;;
                error) problems+=("the env-passthrough probe could not run") ;;
            esac
        fi
    fi
    if [[ ${#problems[@]} -eq 0 ]]; then
        echo "  VERDICT: active on a filesystem that clones blocks — this is the case kache is for."
        exit 0
    fi
    echo "  VERDICT: DRIFT — kache is active but the facts above disagree; failing loudly:"
    for problem in "${problems[@]}"; do
        echo "           - $problem"
    done
    echo "           A target/ is always wrapped or never wrapped."
    if [[ "$store_dev" != "$checkout_dev" && "$store_dev" != "-" ]]; then
        echo "           Do NOT take kache's own first two suggestions here:"
        echo "             windows_hardlink = true        unsafe — Cargo DOES rewrite object outputs"
        echo "             storage_layout_advice = false  silences the signal rather than the cause"
    fi
    echo "           Undo — this shell : export RUSTC_WRAPPER=\"\"   (the empty value overrides the config file)"
    host_config="$cargo_home/config.toml"
    [[ -e "$cargo_home/config" ]] && host_config="$cargo_home/config"
    echo "           Undo — host-wide  : neutralize or remove the rustc-wrapper line in $host_config"
    exit 1

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

# prune Cargo target/ dirs, which cargo never garbage-collects. Passes:
# uninstalled toolchains, untouched >14d, then a 120GB cap only when the target
# filesystem has less than 100 GiB free, then out-of-tree target dirs under
# ~/.cache left behind by --target-dir builds (docs/kache-strategy.md).
# Constrained Windows hosts use the native 80GB policy below. Roots default to
# this repo; override with paths.
sweep *args="": _ensure-cargo-sweep
    @scripts/sweep.sh {{ args }}

# restore the native Cargo target cap if needed, then verify Windows headroom
storage-check: _storage_preflight

# run the native Windows 80 GB sweep policy now
windows-sweep:
    @powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/windows-cargo-sweep.ps1 -Operation run

# install the daily native Windows sweep policy in Task Scheduler
install-windows-sweep:
    @powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/windows-cargo-sweep.ps1 -Operation install

# show the native Windows sweep task's state and next run
windows-sweep-status:
    @powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/windows-cargo-sweep.ps1 -Operation status

# A WSL2 ext4.vhdx grows to its high-water mark and never shrinks, so it can
# starve the target volume of space that sweeping Cargo cannot return — the
# usual cause of a storage-preflight failure that `just sweep` does not fix.
# WSL's own `--set-sparse` remedy is disabled upstream for corruption risk
# (docs/kache-strategy.md), which is why this reclaim is scheduled, not automatic.
#
# reclaim WSL2 vhdx slack now (elevated; ends any running WSL session)
wsl-compact *args="":
    @powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/wsl-vhdx-compact.ps1 -Operation run {{ args }}

# install the weekly WSL2 vhdx compaction task in Task Scheduler
install-wsl-compact:
    @powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/wsl-vhdx-compact.ps1 -Operation install

# show the WSL2 vhdx task's state plus per-distro reclaimable space
wsl-compact-status:
    @powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/wsl-vhdx-compact.ps1 -Operation status

# remove the weekly WSL2 vhdx compaction task
uninstall-wsl-compact:
    @powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/wsl-vhdx-compact.ps1 -Operation uninstall

# Separate from the Windows task above even on WSL2: that one sweeps C:\ and
# cannot see the guest's ext4 filesystem, where target/ actually lives.
#
# install the daily 04:00 Linux sweep schedule (systemd user timer, else cron)
install-linux-sweep *roots="":
    @scripts/linux-cargo-sweep.sh install {{ roots }}

# show the Linux sweep schedule's next run and last log lines
linux-sweep-status:
    @scripts/linux-cargo-sweep.sh status

# remove the Linux sweep schedule
uninstall-linux-sweep:
    @scripts/linux-cargo-sweep.sh uninstall

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

# install the versioned pre-push dispatcher into Git's shared hook directory
#
# Linked worktrees share that directory, so an absolute symlink to one
# checkout's hook executes the wrong protocol from every other worktree. The
# dispatcher resolves the active worktree when Git invokes it. Copying it also
# survives removal of whichever worktree last ran `just init` and preserves
# other unversioned hooks in the directory (e.g. commit-msg).
_ensure-git-hooks:
    #!/usr/bin/env bash
    set -euo pipefail
    hooks_dir="$(git rev-parse --git-path hooks)"
    source="$(git rev-parse --show-toplevel)/.githooks/pre-push-dispatcher"
    target="${hooks_dir}/pre-push"
    if [[ ! -L "${target}" && -e "${target}" ]] && cmp -s "${source}" "${target}"; then
        exit 0
    fi
    mkdir -p "${hooks_dir}"
    staged="${target}.tmp.$$"
    trap 'rm -f "${staged}"' EXIT
    cp "${source}" "${staged}"
    chmod +x "${staged}"
    rm -f "${target}"
    mv "${staged}" "${target}"
    trap - EXIT
    echo "Git hooks: installed worktree-aware pre-push dispatcher at ${target} (host L1/L2 evidence for source changes; RUSTY_BISCUIT_PRE_PUSH=scope-only|warn|strict)"
