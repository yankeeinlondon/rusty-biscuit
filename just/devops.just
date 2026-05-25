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
[arg("agent", long="agent")]
[arg("model", long="model")]
[arg("yolo", long="yolo")]
commit yolo="true" agent="opencode" model="":
    #!/usr/bin/env bash
    set -euo pipefail
    model=""
    if [[ ! -z "{{ model }}" ]]; then
        model="--model {{ model }}"
    else
        if [[ "{{ agent }}" == "opencode" ]]; then
            if [[ -z "{{ model }}" ]]; then
                model="--model minimax/MiniMax-M2.7-highspeed"
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
    cargo clippy -p {{ pkg }} -- -D warnings || exit_code=$?

    duration=$SECONDS
    commit_hash=$(git rev-parse HEAD 2>/dev/null || echo "unknown")
    if [[ $exit_code -eq 0 ]]; then
        echo "{\"duration\": $duration, \"result\": \"pass\", \"last_commit\": \"$commit_hash\"}" >> "lint-{{ pkg }}-timing.jsonl"
    else
        echo "{\"duration\": $duration, \"result\": \"fail\", \"last_commit\": \"$commit_hash\"}" >> "lint-{{ pkg }}-timing.jsonl"
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
    sniff repo deps "$(sniff repo package-area)"

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
        cargo nextest run -p {{ pkg }} {{ args }} || exit_code=$?
    else
        echo -e "ℹ️ using {{ green }}cargo test{{ reset }}"
        cargo test -p {{ pkg }} {{ args }} || exit_code=$?
    fi

    duration=$SECONDS
    commit_hash=$(git rev-parse HEAD 2>/dev/null || echo "unknown")

    if [[ $exit_code -eq 0 ]]; then
        result="pass"
    else
        result="fail"
    fi

    if [[ -n "{{ args }}" ]]; then
        params_json=$(to_json_array {{ args }})
        echo "{\"duration\": $duration, \"result\": \"$result\", \"last_commit\": \"$commit_hash\", \"params\": $params_json}" >> "test-{{ pkg }}-timing.jsonl"
    else
        echo "{\"duration\": $duration, \"result\": \"$result\", \"last_commit\": \"$commit_hash\"}" >> "test-{{ pkg }}-timing.jsonl"
    fi

    if [[ $exit_code -ne 0 ]]; then
        echo -e "\n❌ The {{ bold }}{{ red }}{{ pkg }}{{ reset }} has failing tests!"
        exit 1
    fi

    echo
    echo -e "✅ {{ green }} all tests passing in {{ bold }}{{ pkg }}{{ reset }}{{ green }} package{{ reset }}"
    echo

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
    meta="$(cargo metadata --no-deps --format-version 1)"
    has_lib="$(jq -r --arg p '{{ pkg }}' \
        '.packages[] | select(.name==$p) | .targets[] | select(.kind[]=="lib") | .name' \
        <<< "$meta" | head -n 1)"
    has_bin="$(jq -r --arg p '{{ pkg }}' \
        '.packages[] | select(.name==$p) | .targets[] | select(.kind[]=="bin") | .name' \
        <<< "$meta" | head -n 1)"
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
# Skip-clean by default; pass BISCUIT_TEST_LEVEL_REQUIRED=2 to make missing
# harnesses hard-fail.
_test_l2 pkg *args="":
    #!/usr/bin/env bash
    set -euo pipefail
    echo
    echo -e "🖥  Level-2 tests for {{ bold }}{{ pkg }}{{ reset }}"
    echo
    cargo nextest run -p {{ pkg }} -E 'test(/level2_/)' --no-tests=pass {{ args }}

# Level 3 tier: OS keyboard/mouse injection tests. Always opt-in via
# RUN_LEVEL3=1; the recipe sets the variable so the gate trips cleanly.
_test_l3 pkg *args="":
    #!/usr/bin/env bash
    set -euo pipefail
    echo
    echo -e "🎹 Level-3 tests for {{ bold }}{{ pkg }}{{ reset }} {{ dim }}(RUN_LEVEL3=1){{ reset }}"
    echo
    RUN_LEVEL3=1 cargo nextest run -p {{ pkg }} -E 'test(/level3_/)' --no-tests=pass {{ args }}

# Browser tier: headless browser tests. Skip-clean if Chrome absent unless
# BISCUIT_BROWSER_REQUIRED=1 is set in the environment.
_test_browser pkg *args="":
    #!/usr/bin/env bash
    set -euo pipefail
    echo
    echo -e "🌐 Browser tests for {{ bold }}{{ pkg }}{{ reset }}"
    echo
    cargo nextest run -p {{ pkg }} -E 'test(/browser_/)' --no-tests=pass {{ args }}

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
    meta="$(cargo metadata --no-deps --format-version 1)"
    has_lib="$(jq -r --arg p '{{ pkg }}' \
        '.packages[] | select(.name==$p) | .targets[] | select(.kind[]=="lib") | .name' \
        <<< "$meta" | head -n 1)"
    if [[ -z "$has_lib" ]]; then
        echo -e "{{ yellow }}skipping doctests for {{ pkg }}: no lib target{{ reset }}"
        exit 0
    fi
    cargo test --doc -p {{ pkg }} {{ args }}

# Criterion benches. Packages that opt out via
# `[package.metadata.benchmarks] required = false` should make their
# package-level `bench` recipe a documented no-op rather than calling this.
_bench pkg *args="":
    #!/usr/bin/env bash
    set -euo pipefail
    echo
    echo -e "⚙️  Benches for {{ bold }}{{ pkg }}{{ reset }}"
    echo
    cargo bench -p {{ pkg }} {{ args }}

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
