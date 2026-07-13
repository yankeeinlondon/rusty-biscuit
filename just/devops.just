import 'notify.just'
mod util

bold := '\033[1m'
dim := '\033[2m'
reset := '\033[0m'
italic := '\033[3m'
red := '\033[31m'
green := '\033[32m'
yellow := '\033[33m'
blue := '\033[34m'
magenta := '\033[35m'
underline := '\033[4m'

# dim := '\033[2m'

# Build the package
_build package pkg_name *args="":
    #!/usr/bin/env bash
    set -euo pipefail
    echo -e "🏃‍♂️ Building {{ bold }}{{ pkg_name }}{{ reset }} {{ italic }}on {{ reset }}{{ os() }}"
    echo
    cargo build -p {{ package }} {{ args }} || ( echo -e "{{ red }}ERROR:{{ reset }} Failed to build {{ bold }}{{ pkg_name }}{{ reset }}" && just _speak "The {pkg_name}} failed to build!" && exit 1)
    echo
    echo -e "✅ {{ bold }}{{ green }}{{ pkg_name }}{{ reset }} {{ italic }}built successfully!{{ reset }}"
    echo

_install path pkg_name binary="" *args="":
    #!/usr/bin/env bash
    set -euo pipefail

    echo
    if [[ -z "{{ binary }}" ]]; then
        echo -e "Installing {{ yellow }}{{ bold }}{{ pkg_name }}{{ reset }}"
    else
        echo -e "Installing {{ yellow }}{{ bold }}{{ pkg_name }}{{ reset }} ({{ italic }}{{ blue }}{{ binary }}{{ reset }})"
    fi

    echo
    cargo install --path {{ path }} {{ args }} || ( just _play error-2 && just _speak "Problems encountered when trying to install {{ pkg_name }}" && exit 1 )
    echo
    if [[ -z "{{ binary }}" ]]; then
        echo -e "✅ {{ bold }}{{ green }}{{ pkg_name }} installed!{{ reset }}"
    else
        echo -e "✅ {{ bold }}{{ green }}{{ pkg_name }} installed!{{ reset }} ({{ italic }}{{ blue }}{{ binary }}{{ reset }})"
    fi
    just _play drop-4 || exit 0

# commits all the staged changes with OpenCode (model is MODEL or COMMIT_MODEL)
[arg("agent")]
[arg("model")]
[arg("yolo")]
commit yolo="true" agent="opencode" model="":
    #!/usr/bin/env bash
    set -euo pipefail
    model=""
    if [[ ! -z "{{ model }}" ]]; then
        model="--model {{ model }}"
    else
        if [[ "{{ agent }}" == "opencode" ]]; then
            if [[ -z "{{ model }}" ]]; then
                model="--model minimax/MiniMax-M3"
            fi
        fi
    fi
    yolo=""
    if [[ "{{ yolo }}" == "true" ]]; then
        yolo="-y"
    fi
    echo ""
    echo -e "Committing staged changes in the {{ bold }}Rusty Biscuit{{ reset }} monorepo to git"
    if [[ -z "$model" ]]; then
        echo -e "{{ dim }}  - using the {{ italic }}default{{ reset }}{{ dim }}model for the agent{{ reset }}"
    else
        echo -e "{{ dim }}  - using the {{ italic }}{{ blue }}${model}{{ reset }}{{ dim }}model{{ reset }}"
    fi
    echo ""
    echo -e "{{ bold }}{{ blue }}Staged Files:{{ reset }}"
    sniff repo staged-files || ( echo "No Staged Files! Nothing to do ..." )
    claudine compose --{{ agent }} "@prompts/commit.md" --op "commit" --quiet ${yolo} ${model}
    just _speak "git commits completed in the $(sniff repo package-area) package area of rusty biscuit"
    sniff repo git-status --compact 2>/dev/null || exit 0
    echo

# Lint a package
_lint pkg:
    #!/usr/bin/env bash
    set -euo pipefail

    SECONDS=0

    echo
    echo -e "Linting {{ bold }}{{ pkg }}{{ reset }}"
    echo

    exit_code=0
    cargo clippy -p {{ pkg }} --all-targets -- -D warnings || exit_code=$?

    duration=$SECONDS
    commit_hash=$(git rev-parse HEAD 2>/dev/null || echo "unknown")
    if [[ "${BISCUIT_LINT_RECORD_TIMING:-0}" == "1" ]]; then
        if [[ $exit_code -eq 0 ]]; then
            echo "{\"duration\": $duration, \"result\": \"pass\", \"last_commit\": \"$commit_hash\"}" >> "lint-{{ pkg }}-timing.jsonl"
        else
            echo "{\"duration\": $duration, \"result\": \"fail\", \"last_commit\": \"$commit_hash\"}" >> "lint-{{ pkg }}-timing.jsonl"
        fi
    fi

    if [[ $exit_code -ne 0 ]]; then
        just _speak "The {{ pkg }} package has failing lints"
        echo -e "\n❌ The {{ bold }}{{ red }}{{ pkg }}{{ reset }} has failing lints!"
        exit 1
    fi

_fix_lint pkg agent="claude" *args="":
    #!/usr/bin/env bash
    set -euo pipefail

    echo
    echo -e "Fixing the lint problems in {{ bold }}{{ pkg }}{{ reset }}"
    echo
    claudine {{ agent }} "The package '{{ pkg }}' in rusty-biscuit monorepo has lint errors. Run 'just lint' and fix all lint warnings/errors. Provide a full explanation for what was done." -y {{ args }} || (just _speak "Failure trying to fix the lint errors in the package {{ pkg }}" && exit 1)
    echo
    echo -e "✅ {{ green }}The {{ bold }}{{ pkg }}{{ reset }}{{ green }} has had all lints now fixed!"
    echo
    sniff repo git-status

# Show dependencies for this package area
deps:
    #!/usr/bin/env bash
    set -euo pipefail

    echo
    sniff repo package-dependencies "$(sniff repo package-area)"

# Run tests with `cargo test` or `cargo nextest run`
_test pkg *args="":
    #!/usr/bin/env bash
    set -euo pipefail

    to_json_array() {
        local first=1
        local json="["
        for arg in "$@"; do
            if [[ $first -eq 1 ]]; then
                first=0
            else
                json="${json},"
            fi
            local escaped
            escaped=$(printf '%s' "$arg" | sed 's/\\/\\\\/g; s/"/\\"/g')
            json="${json}\"${escaped}\""
        done
        json="${json}]"
        printf '%s' "$json"
    }

    SECONDS=0

    echo
    echo -e "Testing {{ bold }}{{ pkg }}{{ reset }} package"
    echo

    exit_code=0
    if cargo nextest --version >/dev/null 2>&1; then
        echo -e "ℹ️ using {{ green }}cargo nextest run{{ reset }}"
        cargo nextest run -p {{ pkg }} -E '!(test(/level2_/) + test(/level3_/) + test(/browser_/) + test(/real_/) + test(/slow_/))' --no-tests=pass {{ args }} || exit_code=$?
    else
        echo -e "ℹ️ using {{ green }}cargo test{{ reset }}"
        cargo test -p {{ pkg }} {{ args }} -- --skip level2_ --skip level3_ --skip browser_ --skip real_ --skip slow_ || exit_code=$?
    fi

    duration=$SECONDS
    commit_hash=$(git rev-parse HEAD 2>/dev/null || echo "unknown")

    case $exit_code in
        0) result="pass" ;;
        130) result="interrupted" ;;
        *) result="fail" ;;
    esac

    if [[ "${BISCUIT_TEST_RECORD_TIMING:-0}" == "1" ]]; then
        if [[ -n "{{ args }}" ]]; then
            params_json=$(to_json_array {{ args }})
            echo "{\"duration\": $duration, \"result\": \"$result\", \"last_commit\": \"$commit_hash\", \"params\": $params_json}" >> "test-{{ pkg }}-timing.jsonl"
        else
            echo "{\"duration\": $duration, \"result\": \"$result\", \"last_commit\": \"$commit_hash\"}" >> "test-{{ pkg }}-timing.jsonl"
        fi
    fi

    if [[ $exit_code -eq 130 ]]; then
        echo
        echo -e "{{ yellow }}Testing {{ bold }}{{ pkg }}{{ reset }}{{ yellow }} interrupted (Ctrl+C){{ reset }}"
        exit 130
    fi

    if [[ $exit_code -ne 0 ]]; then
        echo -e "\n❌ The {{ bold }}{{ red }}{{ pkg }}{{ reset }} has failing tests!"
        exit 1
    fi

    echo
    echo -e "✅ {{ green }} all tests passing in {{ bold }}{{ pkg }}{{ reset }}{{ green }} package{{ reset }}"
    echo

# Run Level-1 tests for every workspace package, continuing after failures.
#
# Package discovery comes from Cargo metadata rather than the curated package-area
# list. Optional selectors may be package names or package-area paths.
_test_workspace *selectors="":
    #!/usr/bin/env bash
    set -euo pipefail

    passed_packages=()
    failed_packages=()
    failed_codes=()
    current_package=""

    print_summary() {
        echo
        echo "================================================"
        echo "Workspace Test Summary"
        echo "================================================"
        echo -e "{{ green }}Passed{{ reset }} (${#passed_packages[@]}): ${passed_packages[*]:-(none)}"
        echo -e "{{ red }}Failed{{ reset }} (${#failed_packages[@]}):"
        if (( ${#failed_packages[@]} == 0 )); then
            echo "  (none)"
        else
            local i
            for ((i = 0; i < ${#failed_packages[@]}; i++)); do
                echo "  - ${failed_packages[$i]} (exit ${failed_codes[$i]})"
            done
        fi
        echo "================================================"
        echo
    }

    on_interrupt() {
        trap - INT
        echo
        echo "Interrupted (Ctrl+C)${current_package:+ while testing ${current_package}}; remaining packages were not run."
        print_summary
        exit 130
    }
    trap on_interrupt INT

    metadata_file="$(mktemp "${TMPDIR:-/tmp}/biscuit-workspace-tests.XXXXXX")"
    selected_file="$(mktemp "${TMPDIR:-/tmp}/biscuit-workspace-selected.XXXXXX")"
    trap 'rm -f "${metadata_file}" "${selected_file}"' EXIT

    if ! cargo metadata --no-deps --format-version 1 > "${metadata_file}"; then
        echo "Unable to discover workspace packages with cargo metadata." >&2
        exit 1
    fi

    workspace_root="$(jq -r '.workspace_root // empty' "${metadata_file}")"
    workspace_root="${workspace_root:-${PWD}}"
    workspace_root="${workspace_root%/}"

    requested=({{ selectors }})
    while IFS=$'\t' read -r package manifest_path; do
        include=0
        if (( ${#requested[@]} == 0 )); then
            include=1
        else
            relative_manifest="${manifest_path#"${workspace_root}/"}"
            package_area="${relative_manifest%/Cargo.toml}"
            for selector in "${requested[@]}"; do
                selector="${selector%,}"
                if [[ "${package}" == "${selector}" || "${package_area}" == "${selector}" || "${package_area}" == "${selector}/"* ]]; then
                    include=1
                    break
                fi
            done
        fi

        if (( include == 1 )) && ! grep -Fxq "${package}" "${selected_file}"; then
            printf '%s\n' "${package}" >> "${selected_file}"
        fi
    done < <(
        jq -r '
            .workspace_members as $members
            | .packages[]
            | select(.id as $id | $members | index($id))
            | [.name, .manifest_path]
            | @tsv
        ' "${metadata_file}" | sort
    )

    if [[ ! -s "${selected_file}" ]]; then
        echo "No workspace packages matched: ${requested[*]:-(no selectors)}" >&2
        exit 1
    fi

    package_count="$(wc -l < "${selected_file}" | tr -d ' ')"
    echo
    echo "Running Level-1 tests for ${package_count} workspace packages..."
    echo "------------------------------------------------"

    while IFS= read -r package; do
        current_package="${package}"
        package_args=()

        # Messenger's default feature set omits the desktop implementation its
        # package-area test contract covers.
        if [[ "${package}" == "messenger" ]]; then
            package_args+=(--features desktop)
        fi

        echo
        echo "Testing ${package}..."
        set +e
        just _test "${package}" "${package_args[@]}"
        exit_code=$?
        set -e

        if [[ ${exit_code} -eq 130 ]]; then
            on_interrupt
        elif [[ ${exit_code} -eq 0 ]]; then
            passed_packages+=("${package}")
        else
            failed_packages+=("${package}")
            failed_codes+=("${exit_code}")
        fi
        current_package=""
    done < "${selected_file}"

    print_summary

    if (( ${#failed_packages[@]} > 0 )); then
        exit 1
    fi

# Verify that every package-area `test` recipe preserves Ctrl+C as exit 130.
_check_test_interrupts:
    #!/usr/bin/env bash
    set -euo pipefail

    cargo() {
        if [[ "${1:-} ${2:-}" == "nextest --version" ]]; then
            return 0
        fi
        return 130
    }
    export -f cargo
    export BISCUIT_TEST_RECORD_TIMING=0

    passed_areas=()
    failed_areas=()

    while IFS= read -r justfile; do
        area="${justfile%/justfile}"
        [[ "${area}" == "${justfile}" ]] && continue

        if ! (CDPATH= cd "./${area}" && just --summary 2>/dev/null) | tr ' ' '\n' | grep -qx test; then
            continue
        fi

        set +e
        (CDPATH= cd "./${area}" && just test >/dev/null 2>&1)
        exit_code=$?
        set -e

        if [[ ${exit_code} -eq 130 ]]; then
            passed_areas+=("${area}")
        else
            failed_areas+=("${area} (exit ${exit_code})")
        fi
    done < <(rg --files -g justfile | sort)

    echo
    echo "Ctrl+C test contract"
    echo "================================================"
    echo -e "{{ green }}Passed{{ reset }} (${#passed_areas[@]}): ${passed_areas[*]:-(none)}"
    echo -e "{{ red }}Failed{{ reset }} (${#failed_areas[@]}): ${failed_areas[*]:-(none)}"
    echo "================================================"
    echo

    if (( ${#failed_areas[@]} > 0 )); then
        exit 1
    fi

# ---------------------------------------------------------------------------
# Canonical lifecycle recipes (testing-best-practices spec, Phase 2).
#
# Each package-area justfile exposes the canonical 12-recipe set
# (sanity, test, test-l2, test-l3, test-browser, test-real, lint, bench,
# coverage, doctest, fuzz, all) and delegates the per-package boilerplate to
# the shared `_*` recipes below. Tier filter expressions are documented in
# `.config/nextest.toml`. Recipes that don't apply to a package are explicit
# no-ops with an explanatory comment in the package's own justfile.
# ---------------------------------------------------------------------------

# Sanity tier: fast-confidence subset (≤15s). Lib + bin tests only, with every
# non-default tier excluded via the `slow` filter expression. Doctests live in
# `_doctest`; lint lives in `_lint`. Auto-detects which of `--lib` / `--bins`
# applies so the recipe works for lib-only crates, bin-only crates, and the
# common lib+CLI combo without per-package customization.
_sanity pkg *args="":
    #!/usr/bin/env bash
    set -euo pipefail
    echo
    echo -e "🩺 Sanity check {{ bold }}{{ pkg }}{{ reset }} {{ dim }}(lib + bins, !slow){{ reset }}"
    echo
    # Capture cargo metadata to a tempfile and have jq read it from
    # the file. The previous `"$(cargo metadata)" <<< | jq | head`
    # pipeline deadlocked from `bash -c` in sub-workspace dirs
    # (e.g. `schematic/`, `tree-hugger/`, `unchained-ai/`) because
    # `head -n 1` closing its stdin early left the `jq` end of the
    # pipe in a state the enclosing command-substitution couldn't
    # reap. Using a file plus `first(...)` in jq removes both the pipe
    # and the early-close behaviour. `first(...)` short-circuits on
    # the first match so we don't pay for full enumeration either.
    #
    # `mktemp` here uses the template form (no `-t`) so the same line
    # works on macOS (BSD mktemp), Linux (GNU mktemp), and Windows
    # git-bash — `-t` has incompatible semantics across implementations.
    meta_file="$(mktemp "${TMPDIR:-/tmp}/biscuit-sanity-meta.XXXXXX")"
    trap 'rm -f "${meta_file}"' EXIT
    cargo metadata --no-deps --format-version 1 > "${meta_file}"
    has_lib="$(jq -r --arg p '{{ pkg }}' \
        'first(.packages[] | select(.name==$p) | .targets[] | select(.kind[]=="lib") | .name) // ""' \
        "${meta_file}")"
    has_bin="$(jq -r --arg p '{{ pkg }}' \
        'first(.packages[] | select(.name==$p) | .targets[] | select(.kind[]=="bin") | .name) // ""' \
        "${meta_file}")"
    targets=()
    [[ -n "$has_lib" ]] && targets+=("--lib")
    [[ -n "$has_bin" ]] && targets+=("--bins")
    if (( ${#targets[@]} == 0 )); then
        echo -e "{{ yellow }}skipping sanity for {{ pkg }}: no lib or bin targets{{ reset }}"
        exit 0
    fi
    cargo nextest run -p {{ pkg }} "${targets[@]}" \
        -E '!(test(/level2_/) + test(/level3_/) + test(/browser_/) + test(/real_/) + test(/slow_/))' \
        --no-tests=pass {{ args }}

# Level 2 tier: real terminal / PTY / harness tests.
#
# Skip-clean by default; pass BISCUIT_TEST_LEVEL_REQUIRED=2 to make
# missing harnesses hard-fail.
#
# ## Shared harness amortization
#
# `cargo nextest run` executes each test in its own child process, so a
# process-local `SharedHarness` static cannot reuse a terminal pane
# across tests. To realise the 2-3 s per-spawn savings the spec
# promises, this recipe:
#
#   1. Builds `biscuit-harness-broker` and uses it to spawn ONE pane /
#      window / session per backend (WezTerm, kitty, tmux, Apple
#      Terminal) on the host. Each spawn's id is exported via a
#      `BISCUIT_SHARED_*` env var so each nextest child can attach via
#      `<Backend>Harness::shared_or_spawn()` instead of spawning fresh.
#
#   2. Runs nextest with `-j 1` so tests share the single pane without
#      contending for it. L2 tests are `#[serial(level2)]`-tagged
#      anyway and only one `question` / `bt` process can sensibly own a
#      pane at a time.
#
#   3. Tears every spawned pane down in a trap after nextest exits
#      (success or failure) via `biscuit-harness-broker kill`.
#
# Backends whose tooling is missing return exit code 2 from
# `broker spawn`; the recipe simply doesn't export that env var and
# tests fall back to per-process spawning (which themselves skip
# cleanly via `require_level!`). This means the recipe degrades
# gracefully on hosts without every terminal installed.
#
# ## Parallel self-spawn mode (`BISCUIT_L2_THREADS`)
#
# The shared-pane + `-j 1` model above serializes the WHOLE tier for the
# sake of the (often small) subset of tests that attach to the shared
# pane via `<Backend>::shared_or_spawn()`. An area whose L2 suite is
# dominated by self-isolating tests (each spawns its OWN tmux session /
# PTY — e.g. claudine) pays a steep serial tax for no isolation benefit.
#
# Setting `BISCUIT_L2_THREADS` to a value other than `1` switches this
# recipe into parallel self-spawn mode: it skips the broker entirely and
# exports NO `BISCUIT_SHARED_*` var, so EVERY `shared_or_spawn()` call
# takes its fallback branch and spawns an OWNED, `Drop`-cleaned pane of
# its own. With no shared resource, nextest runs at `-j N` safely. The
# default (`1`, or unset) preserves the shared-pane serial path verbatim
# for every other package area.
_test_l2 pkg *args="":
    #!/usr/bin/env bash
    set -euo pipefail
    echo
    echo -e "🖥  Level-2 tests for {{ bold }}{{ pkg }}{{ reset }}"
    echo

    # Parallel self-spawn mode: no shared pane, no broker, `-j N`. Every
    # test self-isolates via `shared_or_spawn()`'s owned fallback, so
    # there is nothing to contend for. See the recipe comment above.
    l2_threads="${BISCUIT_L2_THREADS:-1}"
    if [[ "${l2_threads}" != "1" ]]; then
        echo -e "  {{ dim }}parallel self-spawn mode: -j ${l2_threads}, no shared pane{{ reset }}"
        echo
        cargo nextest run -p {{ pkg }} -E 'test(/level2_/)' --no-tests=pass -j "${l2_threads}" {{ args }}
        exit $?
    fi

    # Build the broker once (no-op if already up to date) and resolve
    # its absolute path through cargo metadata.
    cargo build -p biscuit-test-harness --bin biscuit-harness-broker --quiet
    # Read cargo metadata via a tempfile to avoid the `bash -c`
    # subprocess hang documented in `_sanity` above. The portable
    # `mktemp` template form works on macOS BSD, Linux GNU, and
    # Windows git-bash; `-t` has incompatible semantics across them.
    meta_file="$(mktemp "${TMPDIR:-/tmp}/biscuit-test-l2-meta.XXXXXX")"
    cargo metadata --no-deps --format-version 1 > "${meta_file}"
    target_dir="$(jq -r '.target_directory' "${meta_file}")"
    rm -f "${meta_file}"
    # Append `.exe` on Windows so the broker resolves under git-bash;
    # `MSYSTEM` is set by MSYS2 / git-bash. macOS and Linux ignore it.
    bin_suffix=""
    if [[ -n "${MSYSTEM:-}" || "${OS:-}" == "Windows_NT" ]]; then
        bin_suffix=".exe"
    fi
    broker="${target_dir}/debug/biscuit-harness-broker${bin_suffix}"
    if [[ ! -x "${broker}" ]]; then
        echo -e "{{ yellow }}warning: ${broker} not executable; falling back to per-test spawning{{ reset }}"
        cargo nextest run -p {{ pkg }} -E 'test(/level2_/)' --no-tests=pass {{ args }}
        exit $?
    fi

    # Registry of (backend id) pairs the trap uses to tear down panes
    # the broker spawned. The trap consumes the registry on EXIT.
    SHARED_REGISTRY="$(mktemp "${TMPDIR:-/tmp}/biscuit-shared.XXXXXX")"
    trap 'while read -r b i; do "${broker}" kill "${b}" "${i}" >/dev/null 2>&1 || true; done < "${SHARED_REGISTRY}"; rm -f "${SHARED_REGISTRY}"' EXIT

    # spawn one shared pane per backend. Backends whose tooling is
    # missing exit non-zero; we silently skip them. The corresponding
    # BISCUIT_SHARED_* env var stays unset and tests fall back to
    # per-process spawning (which also skip cleanly).
    if id="$("${broker}" spawn wezterm 2>/dev/null)" && [[ -n "${id}" ]]; then
        export BISCUIT_SHARED_WEZTERM_PANE_ID="${id}"
        echo "wezterm ${id}" >> "${SHARED_REGISTRY}"
        echo -e "  {{ dim }}spawned wezterm pane ${id}{{ reset }}"
    fi
    if id="$("${broker}" spawn kitty 2>/dev/null)" && [[ -n "${id}" ]]; then
        export BISCUIT_SHARED_KITTY_WINDOW_ID="${id}"
        echo "kitty ${id}" >> "${SHARED_REGISTRY}"
        echo -e "  {{ dim }}spawned kitty window ${id}{{ reset }}"
    fi
    if id="$("${broker}" spawn tmux 2>/dev/null)" && [[ -n "${id}" ]]; then
        export BISCUIT_SHARED_TMUX_SESSION="${id}"
        echo "tmux ${id}" >> "${SHARED_REGISTRY}"
        echo -e "  {{ dim }}spawned tmux session ${id}{{ reset }}"
    fi
    if id="$("${broker}" spawn apple-terminal 2>/dev/null)" && [[ -n "${id}" ]]; then
        export BISCUIT_SHARED_APPLE_TERMINAL_WINDOW_ID="${id}"
        echo "apple-terminal ${id}" >> "${SHARED_REGISTRY}"
        echo -e "  {{ dim }}spawned apple-terminal window ${id}{{ reset }}"
    fi

    cargo nextest run -p {{ pkg }} -E 'test(/level2_/)' --no-tests=pass -j 1 {{ args }}

# Level 3 tier: OS keyboard/mouse injection tests. Always opt-in via
# RUN_LEVEL3=1; the recipe sets the variable so the gate trips cleanly.
_test_l3 pkg *args="":
    #!/usr/bin/env bash
    set -euo pipefail
    echo
    echo -e "🎹 Level-3 tests for {{ bold }}{{ pkg }}{{ reset }} {{ dim }}(RUN_LEVEL3=1){{ reset }}"
    echo
    # Run one test at a time (`-j 1`). These tests inject real OS keyboard/pointer
    # events and so compete for the single global frontmost window + pointer;
    # `#[serial]` cannot serialize them because nextest runs each test in a
    # separate process — mirror the browser tier and serialize the tier itself.
    RUN_LEVEL3=1 cargo nextest run -p {{ pkg }} -j 1 -E 'test(/level3_/)' --no-tests=pass {{ args }}

# Browser tier: headless browser tests. Skip-clean if Chrome absent unless
# BISCUIT_BROWSER_REQUIRED=1 is set in the environment.
_test_browser pkg *args="":
    #!/usr/bin/env bash
    set -euo pipefail
    echo
    echo -e "🌐 Browser tests for {{ bold }}{{ pkg }}{{ reset }}"
    echo
    # Run one test at a time (`-j 1`). Each browser test spawns its own headless
    # Chrome; `#[serial(browser)]` cannot serialize them because nextest runs
    # each test in a separate process. Concurrent Chromes overwhelm teardown and
    # trip nextest's leak detector — mirror the L2 tier and serialize the tier.
    cargo nextest run -p {{ pkg }} -j 1 -E 'test(/browser_/)' --no-tests=pass {{ args }}

# Real-resource tier: external services/devices. Selecting the set is harmless;
# tests themselves are responsible for skipping when the resource is absent.
_test_real pkg *args="":
    #!/usr/bin/env bash
    set -euo pipefail
    echo
    echo -e "🔌 Real-resource tests for {{ bold }}{{ pkg }}{{ reset }}"
    echo
    cargo nextest run -p {{ pkg }} -E 'test(/real_/)' --no-tests=pass {{ args }}

# Doctests. Nextest does not run doctests, so this recipe shells out to
# `cargo test --doc`. Skips packages that have no lib target (bin-only
# crates have no doctests).
_doctest pkg *args="":
    #!/usr/bin/env bash
    set -euo pipefail
    echo
    echo -e "📚 Doctests for {{ bold }}{{ pkg }}{{ reset }}"
    echo
    # See `_sanity` above for why we use a tempfile + `first(...)` jq
    # filter instead of `cargo metadata | jq | head` piping, and for
    # the portable `mktemp` template form.
    meta_file="$(mktemp "${TMPDIR:-/tmp}/biscuit-doctest-meta.XXXXXX")"
    trap 'rm -f "${meta_file}"' EXIT
    cargo metadata --no-deps --format-version 1 > "${meta_file}"
    has_lib="$(jq -r --arg p '{{ pkg }}' \
        'first(.packages[] | select(.name==$p) | .targets[] | select(.kind[]=="lib") | .name) // ""' \
        "${meta_file}")"
    if [[ -z "$has_lib" ]]; then
        echo -e "{{ yellow }}skipping doctests for {{ pkg }}: no lib target{{ reset }}"
        exit 0
    fi
    cargo test --doc -p {{ pkg }} {{ args }}

# Print a stable, host-derived baseline name for cargo bench.
# Format: {host8}--{arch}--{os}--{cores}c--{mem}g
#
# host8 is an 8-hex-char xxHash prefix (via bh) of a stable machine
# identifier — IOPlatformUUID on macOS, /etc/machine-id on Linux, hostname
# fallback. Benchmark baselines are thereby self-segregating: results from
# one machine cannot be silently compared against another.
#
# Requires `sniff`, `bh`, and `jq` on PATH.
_bench_id:
    #!/usr/bin/env bash
    set -euo pipefail
    case "$(uname -s)" in
        Darwin)
            machine_id="$(ioreg -d2 -c IOPlatformExpertDevice 2>/dev/null \
                | awk -F'"' '/IOPlatformUUID/{print $4; exit}')"
            ;;
        Linux)
            machine_id="$(cat /etc/machine-id 2>/dev/null \
                || cat /var/lib/dbus/machine-id 2>/dev/null \
                || hostname)"
            ;;
        *)
            machine_id="$(hostname)"
            ;;
    esac
    [[ -z "${machine_id:-}" ]] && machine_id="$(hostname)"
    host8="$(printf '%016x' "$(bh "$machine_id")" | cut -c1-8)"

    cpu_json="$(sniff cpu --json)"
    os_json="$(sniff os --json)"
    mem_json="$(sniff memory --json)"

    arch="$(echo "$cpu_json"   | jq -r .arch)"
    cores="$(echo "$cpu_json"  | jq -r .physical_cores)"
    os="$(echo "$os_json"      | jq -r .distribution)"
    mem_gb="$(echo "$mem_json" | jq -r '.total_bytes / 1073741824 | floor')"

    echo "${host8}--${arch}--${os}--${cores}c--${mem_gb}g"

# Warn (and gate) when the host is in a state that invalidates bench results.
# Checks: on battery, < 30% memory available, 1-min load avg > cores/2.
#
# Interactive TTY: prints warnings, prompts ENTER to run / Q to quit.
# Non-interactive (CI=true or BENCH_NONINTERACTIVE=1): refuses unless
# BENCH_YES=1.
_bench_preflight:
    #!/usr/bin/env bash
    set -euo pipefail
    warnings=()

    case "$(uname -s)" in
        Darwin)
            if pmset -g batt 2>/dev/null | head -1 | grep -q "Battery Power"; then
                warnings+=("on battery power — thermal throttling likely")
            fi
            load1="$(sysctl -n vm.loadavg 2>/dev/null | awk '{print $2}')"
            ;;
        Linux)
            if [[ -r /sys/class/power_supply/AC/online ]] \
                && [[ "$(cat /sys/class/power_supply/AC/online)" == "0" ]]; then
                warnings+=("on battery power — thermal throttling likely")
            fi
            load1="$(awk '{print $1}' /proc/loadavg)"
            ;;
        *)
            load1="0"
            ;;
    esac

    mem_json="$(sniff memory --json)"
    avail_pct="$(jq -r '(.available_bytes / .total_bytes * 100) | floor' <<< "$mem_json")"
    (( avail_pct < 30 )) && warnings+=("only ${avail_pct}% memory available (< 30%)")

    cores="$(sniff cpu --json | jq -r .physical_cores)"
    threshold="$(awk -v c="$cores" 'BEGIN{print c/2}')"
    if awk -v l="${load1:-0}" -v t="$threshold" 'BEGIN{exit !(l>t)}'; then
        warnings+=("1-min load avg ${load1} > ${threshold} (cores/2)")
    fi

    [[ ${#warnings[@]} -eq 0 ]] && exit 0

    echo
    echo -e "{{ yellow }}⚠  Host is in a constrained state — bench results may not be comparable:{{ reset }}"
    for w in "${warnings[@]}"; do
        echo -e "   • $w"
    done
    echo

    if [[ -n "${CI:-}" || -n "${BENCH_NONINTERACTIVE:-}" ]]; then
        if [[ -z "${BENCH_YES:-}" ]]; then
            echo -e "{{ red }}Refusing to run benches in non-interactive mode.{{ reset }}"
            echo -e "{{ dim }}Set BENCH_YES=1 to override.{{ reset }}"
            exit 1
        fi
        echo -e "{{ dim }}BENCH_YES=1 set — proceeding despite constrained state.{{ reset }}"
        exit 0
    fi

    if [[ -n "${BENCH_YES:-}" ]]; then
        echo -e "{{ dim }}BENCH_YES=1 set — proceeding despite constrained state.{{ reset }}"
        exit 0
    fi

    read -r -p "Press ENTER to run anyway, Q to quit: " reply
    case "${reply:-}" in
        q|Q) echo "Aborted."; exit 130 ;;
        *)   exit 0 ;;
    esac

# Criterion benches. Packages that opt out via
# `[package.metadata.benchmarks] required = false` should make their
# package-level `bench` recipe a documented no-op rather than calling this.
#
# For the canonical before/after workflow use `_bench_save` and
# `_bench_compare` instead.
_bench pkg *args="":
    #!/usr/bin/env bash
    set -euo pipefail
    just _bench_preflight
    echo
    echo -e "⚙️  Benches for {{ bold }}{{ pkg }}{{ reset }}"
    echo
    cargo bench -p {{ pkg }} {{ args }}

# Save current run as the baseline for this host. Run before your change.
_bench_save pkg *args="":
    #!/usr/bin/env bash
    set -euo pipefail
    just _bench_preflight
    name="$(just _bench_id)"
    echo
    echo -e "💾 Saving bench baseline {{ bold }}${name}{{ reset }} for {{ bold }}{{ pkg }}{{ reset }}"
    echo
    cargo bench -p {{ pkg }} {{ args }} -- --save-baseline "$name"

# Compare current run against this host's saved baseline. Run after your change.
_bench_compare pkg *args="":
    #!/usr/bin/env bash
    set -euo pipefail
    just _bench_preflight
    name="$(just _bench_id)"
    target_dir="$(cargo metadata --no-deps --format-version 1 | jq -r .target_directory)"
    if ! find "${target_dir}/criterion" -type d -name "$name" 2>/dev/null | grep -q .; then
        echo -e "{{ red }}No baseline named '${name}' found.{{ reset }}" >&2
        echo -e "{{ dim }}Run 'just bench-save {{ pkg }}' before your change first.{{ reset }}" >&2
        exit 1
    fi
    echo
    echo -e "📊 Comparing {{ bold }}{{ pkg }}{{ reset }} against baseline {{ bold }}${name}{{ reset }}"
    echo
    cargo bench -p {{ pkg }} {{ args }} -- --baseline "$name"

# Per-package LCOV coverage. Writes a `lcov-<pkg>.info` artifact next to the
# package's own working directory.
_coverage pkg *args="":
    #!/usr/bin/env bash
    set -euo pipefail
    echo
    echo -e "📊 Coverage for {{ bold }}{{ pkg }}{{ reset }}"
    echo
    cargo llvm-cov -p {{ pkg }} --lcov --output-path "lcov-{{ pkg }}.info" {{ args }}

# Run a cargo-fuzz target. Requires nightly toolchain pinned in the
# package's `fuzz/rust-toolchain.toml`.
#
# Usage: `just _fuzz <pkg-fuzz-dir> <target> [-- libfuzzer-args]`
_fuzz fuzz_dir target *args="":
    #!/usr/bin/env bash
    set -euo pipefail
    echo
    echo -e "🦠 Fuzzing {{ bold }}{{ target }}{{ reset }} {{ dim }}({{ fuzz_dir }}){{ reset }}"
    echo
    cd "{{ fuzz_dir }}"
    cargo +nightly fuzz run "{{ target }}" {{ args }}

# Composite "everything that is part of the PR gate" runner.
# Order: sanity → lint → doctest → test → test-l2 → test-browser (D7).
# Tiers that skip cleanly when their resource is absent do not fail `_all`.
_all pkg:
    #!/usr/bin/env bash
    set -euo pipefail
    echo
    echo -e "🚦 Running {{ bold }}all{{ reset }} canonical tiers for {{ bold }}{{ pkg }}{{ reset }}"
    echo
    just _sanity      "{{ pkg }}"
    just _lint        "{{ pkg }}"
    just _doctest     "{{ pkg }}"
    just _test        "{{ pkg }}"
    just _test_l2     "{{ pkg }}"
    just _test_browser "{{ pkg }}"

# Validate that the *current* package-area justfile exposes the canonical
# recipe set (D6). Run from inside a package area:  `just _check_canonical`.
_check_canonical:
    #!/usr/bin/env bash
    set -euo pipefail
    required=(sanity test test-l2 test-l3 test-browser test-real lint bench coverage doctest fuzz all)
    summary="$(just --summary 2>/dev/null | tr ' ' '\n')"
    missing=()
    for r in "${required[@]}"; do
        if ! grep -qx "$r" <<< "$summary"; then
            missing+=("$r")
        fi
    done
    if (( ${#missing[@]} > 0 )); then
        echo -e "{{ red }}❌ Missing canonical recipes:{{ reset }} ${missing[*]}"
        echo -e "   {{ dim }}see just/devops.just for the shared `_*` templates{{ reset }}"
        exit 1
    fi
    echo -e "{{ green }}✅ Justfile defines all {{ bold }}${#required[@]}{{ reset }}{{ green }} canonical recipes{{ reset }}"

# Show all README.md files in the package
_readme_files:
    @sniff docs --readme --json "$(sniff repo package-area)"

status subject="":
    #!/usr/bin/env bash
    set -euo pipefail

    # No subject — show full overview (original behaviour)
    if [[ -z "{{ subject }}" ]]; then
        echo
        echo -e "{{ bold }}{{ blue }}{{ underline }}Features{{ reset }}"
        echo
        if [[ -d features ]]; then
            eza features -lhga --git --icons=always --hyperlink --group -I '_*' || echo -e "  {{ italic }}no features found{{ reset }}"
        else
            echo -e "  {{ italic }}no features directory{{ reset }}"
        fi
        echo
        echo -e "{{ bold }}Unscheduled{{ reset }}"
        echo
        if [[ -d features/_unscheduled ]]; then
            eza features/_unscheduled -lhga --git --icons=always --hyperlink --group -s name || echo -e "  {{ italic }}no unscheduled features{{ reset }}"
        else
            echo -e "  {{ italic }}no unscheduled directory{{ reset }}"
        fi
        echo
        echo -e "{{ bold }}{{ blue }}{{ underline }}Fixes{{ reset }}"
        echo
        if [[ -d fixes ]]; then
            eza fixes -lDga -I '_*' --icons=always --hyperlink || echo -e "  {{ italic }}no fixes found{{ reset }}"
        else
            echo -e "  {{ italic }}no fixes directory{{ reset }}"
        fi
        echo
        echo -e "{{ bold }}{{ magenta }}{{ underline }}Recent Reviews{{ reset }}"
        echo
        if [[ -d reviews ]]; then
            mapfile -t recent_reviews < <(find reviews -maxdepth 1 -mindepth 1 -type d -mtime -3 2>/dev/null | sort)
            if [[ ${#recent_reviews[@]} -gt 0 ]]; then
                eza "${recent_reviews[@]}" -ldga --icons=always --hyperlink
            else
                echo -e "  {{ italic }}no reviews updated in the last 3 days{{ reset }}"
            fi
        else
            echo -e "  {{ italic }}no reviews directory{{ reset }}"
        fi
        exit 0
    fi

    # Subject provided — match against features and fixes
    subj="{{ subject }}"
    tmpfile=$(mktemp)
    trap 'rm -f "${tmpfile}"' EXIT

    # First pass: scheduled items only
    mapfile -t scheduled < <(
        for d in features fixes; do
            [[ -d "${d}" ]] || continue
            find "${d}" -maxdepth 1 -mindepth 1 -type d ! -name '_*' -print0 2>/dev/null | while IFS= read -r -d '' dir; do
                basename="${dir##*/}"
                if [[ "${basename}" == *"${subj}"* ]]; then
                    printf '%s\n' "${dir}"
                fi
            done
        done
    )

    # Single scheduled match → immediate display
    if [[ ${#scheduled[@]} -eq 1 && -n "${scheduled[0]}" ]]; then
        eza "${scheduled[0]}" -lga ---icons=always --hyperlink
        exit 0
    fi

    # Build full candidate list (scheduled + unscheduled fallback)
    (
        if [[ ${#scheduled[@]} -gt 0 ]]; then
            printf '%s\n' "${scheduled[@]}"
        fi
        for d in features/_unscheduled fixes/_unscheduled; do
            [[ -d "${d}" ]] || continue
            find "${d}" -maxdepth 1 -mindepth 1 -type d ! -name '_*' -print0 2>/dev/null | while IFS= read -r -d '' dir; do
                basename="${dir##*/}"
                if [[ "${basename}" == *"${subj}"* ]]; then
                    printf '%s\n' "${dir}"
                fi
            done
        done
    ) > "${tmpfile}"

    # Remove duplicates while preserving order
    awk '!seen[$0]++' "${tmpfile}" > "${tmpfile}.uniq"
    mv "${tmpfile}.uniq" "${tmpfile}"

    if [[ ! -s "${tmpfile}" ]]; then
        echo -e "{{ red }}No matches found for '{{ bold }}{{ subject }}{{ reset }}{{ red }}'{{ reset }}" >&2
        exit 1
    fi

    matched="$(just util::_fzf_select "Select matching item" "${tmpfile}")"
    eza "${matched}" -lga ---icons=always --hyperlink

# show the directory structure of the given directory
_structure dir:
    @eza -lDTL 4 --icons=always --hyperlink {{ dir }}

# produce documentation for a package (without dependencies included)
_doc package open="false":
    #!/usr/bin/env bash
    set -euo pipefail
    echo
    echo -e "Creating documentation for {{ bold }}{{ package }}{{ reset }}."
    echo
    cargo doc -p {{ package }} --no-deps
    echo
    if [[ "{{ open }}" == "true" ]]; then
        open "../target/doc/{{ package }}/index.html"
    fi
