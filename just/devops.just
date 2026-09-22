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

# Preserve enough headroom for Cargo and the Windows linker before entering a
# storage-heavy recipe. Other hosts return immediately. Workspaces that merely
# import these recipes (e.g. the test-toolkit staging fixtures) are not git
# checkouts and have no scripts/ dir, so the preflight skips them; a real
# preflight failure still fails the recipe.
_storage_preflight:
    @repo_root="$(git rev-parse --show-toplevel 2>/dev/null || true)"; if [[ -n "$repo_root" && -f "$repo_root/scripts/storage-preflight.sh" ]]; then bash "$repo_root/scripts/storage-preflight.sh"; fi

# Build the compiler-work counter and print the bindings ONE measured command
# needs, for `fixes/2026-09-12-single-os-compile/plan.md`'s Phase 1 baselines.
#
# The three cross-platform traps this recipe exists to hold in one place:
# Windows needs the `.exe` suffix and a native path (`pwd -W`) because Cargo
# hands `RUSTC_WRAPPER` to the OS loader, not to git-bash; the counter
# directory has to be emptied before the measured command so a retried step
# does not accumulate two runs' events; and the bindings must reach the caller
# as STEP outputs, never `$GITHUB_ENV`, because a job-wide `RUSTC_WRAPPER`
# would wrap the runner-tool builds too and mix wrapped and unwrapped units in
# one `target/`.
#
# `$GITHUB_OUTPUT` is written when present and the values are always echoed, so
# the recipe is usable on a developer host for the same measurement.
_ci_build_counter label:
    #!/usr/bin/env bash
    set -euo pipefail

    # The counter's own build is instrumentation cost, not the cell's work, so
    # it is timed and reported as its own stage rather than folded into setup.
    counter_started="$(date +%s)"
    # `build-tools`, not the default feature set: the counter needs
    # biscuit-hash and biscuit-terminal, and nothing here should compile
    # sniff's duckdb and gix closure to time a build.
    cargo build --quiet --manifest-path scripts/Cargo.toml \
        --no-default-features --features build-tools --bin ci-build
    counter_seconds=$(( $(date +%s) - counter_started ))

    suffix=""
    root="$PWD"
    if [[ -n "${MSYSTEM:-}" || "${OS:-}" == "Windows_NT" ]]; then
        suffix=".exe"
        root="$(pwd -W)"
    fi
    wrapper="${root}/target/debug/ci-build${suffix}"
    [[ -x "${wrapper}" ]] || { echo "ci-build not built at ${wrapper}" >&2; exit 1; }

    counter_dir="${RUNNER_TEMP:-${TMPDIR:-/tmp}}/ci-build-events"
    "${wrapper}" reset --counter-dir "${counter_dir}" >/dev/null

    echo -e "  {{ dim }}measuring compiler work for {{ label }} (counter built in ${counter_seconds}s){{ reset }}"
    if [[ -n "${GITHUB_OUTPUT:-}" ]]; then
        {
            echo "wrapper=${wrapper}"
            echo "wrap=1"
            echo "counter_dir=${counter_dir}"
            echo "setup_seconds=${counter_seconds}"
        } >>"${GITHUB_OUTPUT}"
    fi

# Render the recorded compiler work and write the measurement document for one
# `{package, environment, gate}` cell.
#
# The document is its OWN artifact, never a new field inside `status.json`. The
# rollup reads statuses to decide cell outcomes, and a baseline measurement must
# not be able to change — or even appear to change — what a cell concluded. It
# is keyed the same way every other store is, `{package, environment, gate}`,
# so it joins the existing evidence without inventing a second identity.
#
# Stage seconds are whatever the caller measured; the recipe records only what
# it was given. `queued_seconds` is deliberately NOT computed here. Two reasons:
# parsing an ISO run-start timestamp needs GNU `date -d` on Linux and BSD
# `date -jf` on macOS, and the run's own start is a property of the RUN, which
# the GitHub API reports exactly. The document carries the job's start epoch
# plus the run id and attempt, and the baseline reader joins them once, in
# Python, on one host.
#
# Never fails the cell: an instrumentation fault must not turn a green gate red.
# A missing or unreadable counter reports itself and leaves `compiler_work`
# null rather than embedding a half-measurement.
_ci_build_report wrapper counter_dir out package environment gate:
    #!/usr/bin/env bash
    set -euo pipefail

    compiler_work="null"
    if [[ -x "{{ wrapper }}" ]]; then
        "{{ wrapper }}" report --counter-dir "{{ counter_dir }}" || true
        raw="$("{{ wrapper }}" report --counter-dir "{{ counter_dir }}" --json 2>/dev/null || true)"
        if [[ -n "${raw}" ]]; then
            compiler_work="$(printf '%s' "${raw}" | jq -c . 2>/dev/null || echo null)"
        fi
    else
        echo "ci-build is not available at '{{ wrapper }}'; no compiler work reported" >&2
    fi

    timings="{}"
    if [[ -n "${BISCUIT_CI_STAGE_SECONDS:-}" ]]; then
        timings="$(printf '%s' "${BISCUIT_CI_STAGE_SECONDS}" | jq -c . 2>/dev/null || echo '{}')"
    fi

    mkdir -p "$(dirname "{{ out }}")"
    jq -n \
        --arg package "{{ package }}" \
        --arg environment "{{ environment }}" \
        --arg gate "{{ gate }}" \
        --arg run_id "${GITHUB_RUN_ID:-}" \
        --arg run_attempt "${GITHUB_RUN_ATTEMPT:-}" \
        --arg job_started_epoch "${BISCUIT_CI_JOB_STARTED_EPOCH:-}" \
        --arg runner_os "${RUNNER_OS:-}" \
        --arg runner_image "${ImageOS:-}" \
        --argjson timings "${timings}" \
        --argjson compiler_work "${compiler_work}" \
        '{schema_version: 1, package: $package, environment: $environment, gate: $gate,
          run_id: $run_id, run_attempt: $run_attempt, job_started_epoch: $job_started_epoch,
          runner_os: $runner_os, runner_image: $runner_image,
          timings: $timings, compiler_work: $compiler_work}' \
        >"{{ out }}"

# Spell a path the way the HOST's own programs understand it.
#
# Under Git Bash — which is what `shell: bash` and every `#!/usr/bin/env bash`
# recipe get on a Windows runner — `$PWD` is `/d/a/repo`, an MSYS spelling no
# native program can open. Every consumer of an archive-mode path is a native
# program: `cargo-nextest`'s `--workspace-remap`, insta's workspace root, the
# JUnit staging root, and the verifier itself. `cygpath -m` answers the
# drive-qualified forward-slash spelling, which is the one form both MSYS and
# Win32 accept and which carries no `\\?\` verbatim prefix (the spelling that
# broke nextest's own argument grammar in Phase 3). On every other OS, and
# wherever `cygpath` is absent, the path is already native and is echoed back.
_native_path path:
    #!/usr/bin/env bash
    set -euo pipefail
    if [[ -n "${MSYSTEM:-}" || "${OS:-}" == "Windows_NT" ]] && command -v cygpath >/dev/null 2>&1; then
        cygpath -m -- '{{ path }}'
    else
        printf '%s\n' '{{ path }}'
    fi

# Refuse an `--archive-file` that names nothing, and say why it might.
#
# A tier recipe pastes its `*args` raw into `forwarded=({{ args }})`, so bash
# both word-splits the list AND processes backslash escapes: a native Windows
# spelling (`C:\Users\ken\…`) arrives as `C:Usersken…` and nextest reports a
# missing file for a path nobody typed. Every archive-mode caller therefore
# hands the tier recipes what `_native_path` answers. This recipe is itself
# backslash-safe — its one argument is interpolated inside single quotes — so it
# can name the mangled value back to the reader.
_archive_file_check file:
    #!/usr/bin/env bash
    set -euo pipefail
    if [[ -f '{{ file }}' ]]; then
        exit 0
    fi
    echo -e "❌ {{ red }}--archive-file names no readable file{{ reset }}" >&2
    echo "   '{{ file }}'" >&2
    echo "   Nothing here may compile a replacement: a consumer runs what a producer built." >&2
    if [[ -n "${MSYSTEM:-}" || "${OS:-}" == "Windows_NT" ]]; then
        echo "   On Windows a backslash spelling does not survive a recipe's argument list." >&2
        echo -e "   Pass what {{ bold }}just _native_path <path>{{ reset }} answers." >&2
    fi
    exit 1

# Which build record one `{environment, gate}` cell consumes.
#
# `builds` is the package's own slice of the resolved plan's build records, as
# `affected_scope.py` projected it. One recipe rather than four copies of the
# same `jq`, because the L1, L2, browser, and WSL2 legs must agree exactly about
# which record a cell belongs to.
#
# A cell reaching here without a record is a DISAGREEMENT between the plan and
# the workflow, and it refuses. Every executing L1, L2, or browser cell
# references exactly one build; a job runs only for a cell the plan left
# executing. Answering "compile in place" instead would be the build fallback
# the specification exists to remove — and on a toolchain-free consumer it would
# surface minutes later as a missing compiler rather than here as a plan that
# does not describe this run.
_ci_build_consumer builds environment gate:
    #!/usr/bin/env bash
    set -euo pipefail

    record="$(jq -c --arg env '{{ environment }}' --arg gate '{{ gate }}' \
        '[.[] | select(any(.consumers[]; .environment == $env and .gate == $gate))] | first // empty' \
        <<<'{{ builds }}')"

    if [[ -z "${record}" ]]; then
        echo -e "❌ {{ red }}{{ environment }}/{{ gate }} is executing and the plan names no build for it{{ reset }}" >&2
        echo "   Every executing L1/L2/browser cell references exactly one build record." >&2
        echo "   This consumer will not compile a replacement; fix the plan." >&2
        exit 1
    fi

    artifact="$(jq -r '.artifact' <<<"${record}")"
    key="$(jq -r '.key' <<<"${record}")"
    producer="$(jq -r '.producer' <<<"${record}")"
    echo -e "  {{ dim }}{{ environment }}/{{ gate }} runs build ${key} produced on ${producer}{{ reset }}"
    if [[ -n "${GITHUB_OUTPUT:-}" ]]; then
        {
            echo "archive=1"
            echo "artifact=${artifact}"
            echo "key=${key}"
            echo "producer=${producer}"
        } >>"${GITHUB_OUTPUT}"
    fi

# Verify a downloaded build and print the bindings its consumer needs.
#
# Strict by construction: the verifier is the one the producer shipped beside
# the archive, it is run BEFORE anything is extracted, and a rejection exits
# non-zero so the tier never starts. It compiles nothing — `--nextest` names the
# standalone `cargo-nextest` binary precisely so listing the archive's inventory
# cannot reach Cargo.
#
# Emits `archive_file`, `sidecar_dir`, `workspace`, `key`, and `digest` as step
# outputs. The sidecar directory is the producer's answer to every compile-time
# tool a consumer used to build for itself: the messenger desktop stubs,
# darkmatter's `md` fixture, and the L2 harness broker all arrive here, verified.
# `workspace` is this checkout in the host's own spelling — the value the tier
# recipe remaps the archive onto — computed once here rather than three times in
# three gate steps that must agree.
#
# It also emits this consumer's pre-execution stage costs: `download_seconds`
# (from `BISCUIT_CI_TRANSFER_STARTED_EPOCH`, set by the step that began the
# artifact download), `verify_seconds` for this recipe's own window, and
# `extract_ms` — the verifier's full extraction of the archive, which is the
# stage the reporting contract requires to be visible apart from test time.
# Seconds where a portable `date +%s` is the only clock every consumer shares
# (macOS and Git Bash have no `%3N`), milliseconds where the tool itself
# measured.
_ci_build_verify dir artifact environment plan="":
    #!/usr/bin/env bash
    set -euo pipefail

    started="$(date +%s)"
    suffix=""
    if [[ -n "${MSYSTEM:-}" || "${OS:-}" == "Windows_NT" ]]; then
        suffix=".exe"
    fi
    # `$RUNNER_TEMP` is a Windows path even inside Git Bash, so what arrives
    # here is `D:\a\_temp/build`; every path derived from it is handed to a
    # native program or exported to a later step.
    dir="$(just _native_path '{{ dir }}')"
    workspace="$(just _native_path "$PWD")"
    tool="${dir}/tools/ci-build${suffix}"
    manifest="${dir}/{{ artifact }}.manifest.json"
    if [[ ! -f "${tool}" ]]; then
        echo -e "❌ {{ red }}the build artifact carries no verifier at ${tool}{{ reset }}" >&2
        exit 1
    fi
    # An artifact download does not preserve the executable bit.
    chmod +x "${tool}" 2>/dev/null || true

    verdict="${dir}/{{ artifact }}.verdict.json"
    verify_args=(verify --manifest "${manifest}" --environment '{{ environment }}'
        --workspace "${workspace}" --nextest "cargo-nextest nextest"
        --verdict-out "${verdict}")
    if [[ -n '{{ plan }}' ]]; then
        verify_args+=(--plan '{{ plan }}')
    fi
    "${tool}" "${verify_args[@]}"

    archive_file="${dir}/$(jq -r '.archive.file' "${manifest}")"
    sidecar_dir="${dir}/{{ artifact }}-sidecars"
    if [[ -d "${sidecar_dir}" ]]; then
        chmod +x "${sidecar_dir}"/* 2>/dev/null || true
    fi
    download_seconds=""
    if [[ -n "${BISCUIT_CI_TRANSFER_STARTED_EPOCH:-}" ]]; then
        download_seconds="$(( started - BISCUIT_CI_TRANSFER_STARTED_EPOCH ))"
    fi
    if [[ -n "${GITHUB_OUTPUT:-}" ]]; then
        {
            echo "archive_file=${archive_file}"
            echo "sidecar_dir=${sidecar_dir}"
            echo "workspace=${workspace}"
            echo "key=$(jq -r '.key' "${manifest}")"
            echo "digest=$(jq -r '.digest' "${manifest}")"
            echo "download_seconds=${download_seconds:-0}"
            echo "verify_seconds=$(( $(date +%s) - started ))"
            # An unreadable verdict reports no extraction rather than failing a
            # verification that already succeeded.
            echo "extract_ms=$(jq -r '.timings.extract_ms // 0' "${verdict}" 2>/dev/null || echo 0)"
        } >>"${GITHUB_OUTPUT}"
    fi

# Worker budget: keep two slots for other host work, except on small CI
# runners where reserving them would discard half or more of the capacity.
# This limits test processes, not CPU affinity or Cargo build jobs.
_test_threads:
    #!/usr/bin/env bash
    set -euo pipefail
    cores=""
    if command -v sniff >/dev/null 2>&1 && command -v jq >/dev/null 2>&1; then
        cores="$(sniff cpu --json 2>/dev/null | jq -er '.logical_cores | select(type == "number" and . > 0)' 2>/dev/null)" || cores=""
    fi
    # Bootstrap and archive-only CI environments may not have Sniff installed.
    if [[ ! "$cores" =~ ^[1-9][0-9]*$ ]]; then
        cores="$(getconf _NPROCESSORS_ONLN 2>/dev/null || sysctl -n hw.ncpu 2>/dev/null || nproc 2>/dev/null || echo "${NUMBER_OF_PROCESSORS:-1}")"
    fi
    [[ "$cores" =~ ^[1-9][0-9]*$ ]] || cores=1
    if [[ "${CI:-}" == "true" || "${GITHUB_ACTIONS:-}" == "true" || -n "${BISCUIT_CI_ENVIRONMENT:-}" ]] && (( cores <= 4 )); then
        echo "$cores"
    else
        echo "$(( cores > 2 ? cores - 2 : 1 ))"
    fi

# Build the package
_build package pkg_name *args="": _storage_preflight
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
    cargo install --locked --path {{ path }} {{ args }} || ( just _play error-2 && just _speak "Problems encountered when trying to install {{ pkg_name }}" && exit 1 )
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
_lint pkg: _storage_preflight
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

# Refresh the GitNexus index without rewriting agent instruction files.
gitnexus:
    #!/usr/bin/env bash
    set -euo pipefail

    status_output=""
    status_exit=0
    status_output="$(gitnexus status 2>&1)" || status_exit=$?
    printf '%s\n' "${status_output}"

    if (( status_exit == 0 )) && grep -Eq '^Status: .*up-to-date[[:space:]]*$' <<< "${status_output}"; then
        exit 0
    fi

    gitnexus analyze --skip-agents-md --force

# ---------------------------------------------------------------------------
# JUnit report staging
#
# `.config/nextest.toml` writes every `ci`-profile run to the SAME
# `<target>/nextest/ci/test-results.xml`, so a multi-package area recipe used to
# upload only whichever package ran last. These two recipes bracket each nextest
# invocation and preserve one XML document per invocation:
#
#     $STAGE/<tier>/<package>.xml   verbatim copy of that invocation's report
#     $STAGE/manifest.jsonl         one JSON record per invocation, appended
#
# `$STAGE` is `$BISCUIT_JUNIT_STAGE_DIR` — absolute, or relative to the
# workspace root — and defaults to `<workspace-root>/target/nextest/ci-reports`.
# `<tier>` is one of `L1`, `L2`, `L3`, `browser`, `real`, `sanity`.
#
# The workspace root and target directory normally come from `cargo metadata`.
# `BISCUIT_JUNIT_WORKSPACE_ROOT` / `BISCUIT_JUNIT_TARGET_DIR` supply them
# directly, for environments that have no Cargo at all — the wsl2-ubuntu leg runs
# a `nextest archive` inside a guest with only `just` and a standalone
# `cargo-nextest`. When neither the overrides nor `cargo metadata` are available
# staging still cannot fail the run, but it says so loudly; see
# `_junit_staging_unavailable`.
#
# Each manifest record carries exactly these keys:
#
#     tier package xml exit_code environment duration_s report_present
#
# `package` is the recipe argument — the package is the unit of result
# identity. `environment` comes from `$BISCUIT_CI_ENVIRONMENT` and is `""` when
# unset (local dev) — never invented. `report_present` is `false` when nextest
# emitted no XML at all (a compile failure, say). A MISSING report is signal
# for the rollup, so the record is still written.
#
# A `provenance.json` file sits beside `manifest.jsonl` when `GITHUB_SHA` is
# present at staging time, carrying immutable CI/source metadata from GitHub
# Actions environment variables:
#
#     sha runId runNumber runAttempt ref event workflow
#
# Absent provenance records the tree as `local` evidence; present provenance
# makes the tree `ci` evidence only when all counted samples carry it, all
# share one source revision, and run numbers are consecutive. Staging never
# fails when provenance is unavailable — it is recorded as absent rather than
# causing the invocation to exit non-zero.
#
# `_stage_junit_reset` runs BEFORE the invocation rather than deleting the
# report after copying it, for two reasons: a run that never reaches the report
# stage cannot then be credited with its predecessor's XML, and the last
# invocation's report survives at the legacy `target/nextest/ci/test-results.xml`
# path that `_package-ci.yml` still uploads.
# ---------------------------------------------------------------------------

# Announce, unmissably, that JUnit staging is about to no-op.
#
# Staging must never change a test invocation's outcome, so both staging recipes
# exit 0 when they cannot find their paths. Exiting 0 *silently* is the danger:
# the run executes, stages nothing, and the rollup reports MISSING for a cell
# that actually passed — indistinguishable from a job that never ran. The
# wsl2-ubuntu leg hits this by construction (no cargo in the guest), which is why
# `BISCUIT_JUNIT_WORKSPACE_ROOT` / `BISCUIT_JUNIT_TARGET_DIR` exist.
_junit_staging_unavailable recipe missing:
    #!/usr/bin/env bash
    set -uo pipefail
    echo >&2
    echo -e "⚠️  {{ yellow }}{{ bold }}JUnit staging skipped{{ reset }}{{ yellow }} in {{ recipe }}: {{ missing }} is unavailable{{ reset }}" >&2
    echo "   No report and no manifest record will be produced for this invocation," >&2
    echo "   so a CI rollup will score this cell MISSING even though the tests ran." >&2
    echo -e "   Set {{ bold }}BISCUIT_JUNIT_WORKSPACE_ROOT{{ reset }} and {{ bold }}BISCUIT_JUNIT_TARGET_DIR{{ reset }} to skip the" >&2
    echo "   cargo-metadata derivation entirely (see fixes/2026-07-27-refactor/wsl-archive-requirements.md)." >&2
    echo >&2
    exit 0

# Discard any report left behind by a previous nextest invocation, so a run that
# emits none is recorded as MISSING instead of inheriting a stale document.
_stage_junit_reset:
    #!/usr/bin/env bash
    # Never let staging change a test invocation's outcome: this recipe always
    # exits 0, including when cargo/jq are unavailable.
    set -uo pipefail
    target_dir="${BISCUIT_JUNIT_TARGET_DIR:-}"
    if [[ -z "${target_dir}" ]]; then
        meta_file="$(mktemp "${TMPDIR:-/tmp}/biscuit-junit-reset.XXXXXX")" || exit 0
        trap 'rm -f "${meta_file}"' EXIT
        if ! cargo metadata --no-deps --format-version 1 > "${meta_file}" 2>/dev/null; then
            just _junit_staging_unavailable _stage_junit_reset "cargo metadata"
            exit 0
        fi
        target_dir="$(jq -r '.target_directory // empty' "${meta_file}" 2>/dev/null)" || target_dir=""
    fi
    if [[ -z "${target_dir}" ]]; then
        just _junit_staging_unavailable _stage_junit_reset "the target directory"
        exit 0
    fi
    rm -f "${target_dir%/}/nextest/${NEXTEST_PROFILE:-default}/test-results.xml" 2>/dev/null || true
    exit 0

# Copy this invocation's JUnit report into `$STAGE/<tier>/<pkg>.xml` and append
# its manifest record. See the block comment above for the full contract.
_stage_junit tier pkg exit_code duration:
    #!/usr/bin/env bash
    set -uo pipefail

    exit_code='{{ exit_code }}'
    duration='{{ duration }}'
    [[ "${exit_code}" =~ ^-?[0-9]+$ ]] || exit_code=0
    [[ "${duration}" =~ ^[0-9]+$ ]] || duration=0

    workspace_root="${BISCUIT_JUNIT_WORKSPACE_ROOT:-}"
    target_dir="${BISCUIT_JUNIT_TARGET_DIR:-}"
    if [[ -z "${workspace_root}" || -z "${target_dir}" ]]; then
        meta_file="$(mktemp "${TMPDIR:-/tmp}/biscuit-junit-stage.XXXXXX")" || exit 0
        trap 'rm -f "${meta_file}"' EXIT
        if ! cargo metadata --no-deps --format-version 1 > "${meta_file}" 2>/dev/null; then
            just _junit_staging_unavailable _stage_junit "cargo metadata"
            exit 0
        fi
        [[ -n "${workspace_root}" ]] || workspace_root="$(jq -r '.workspace_root // empty' "${meta_file}" 2>/dev/null)"
        [[ -n "${target_dir}" ]] || target_dir="$(jq -r '.target_directory // empty' "${meta_file}" 2>/dev/null)"
    fi
    if [[ -z "${workspace_root}" || -z "${target_dir}" ]]; then
        just _junit_staging_unavailable _stage_junit "the workspace root / target directory"
        exit 0
    fi
    if ! command -v jq >/dev/null 2>&1; then
        just _junit_staging_unavailable _stage_junit "jq"
        exit 0
    fi

    stage="${BISCUIT_JUNIT_STAGE_DIR:-target/nextest/ci-reports}"
    case "${stage}" in
        /* | ?:[/\\]*) ;;
        *) stage="${workspace_root%/}/${stage}" ;;
    esac
    mkdir -p "${stage}/{{ tier }}" 2>/dev/null || exit 0

    relative_xml="{{ tier }}/{{ pkg }}.xml"
    report="${target_dir%/}/nextest/${NEXTEST_PROFILE:-default}/test-results.xml"
    report_present=false
    if [[ -f "${report}" ]] && cp "${report}" "${stage}/${relative_xml}" 2>/dev/null; then
        report_present=true
    fi

    jq -c -n \
        --arg tier '{{ tier }}' \
        --arg package '{{ pkg }}' \
        --arg xml "${relative_xml}" \
        --argjson exit_code "${exit_code}" \
        --arg environment "${BISCUIT_CI_ENVIRONMENT:-}" \
        --argjson duration_s "${duration}" \
        --argjson report_present "${report_present}" \
        '{tier: $tier, package: $package, xml: $xml, exit_code: $exit_code, environment: $environment, duration_s: $duration_s, report_present: $report_present}' \
        >> "${stage}/manifest.jsonl" 2>/dev/null || exit 0

    # Write provenance.json once per staging tree when CI environment is present.
    # Missing provenance records the tree as local rather than failing the invocation.
    if [[ ! -f "${stage}/provenance.json" && -n "${GITHUB_SHA:-}" ]]; then
        jq -c -n \
            --arg sha "${GITHUB_SHA}" \
            --arg runId "${GITHUB_RUN_ID:-}" \
            --argjson runNumber "${GITHUB_RUN_NUMBER:-0}" \
            --argjson runAttempt "${GITHUB_RUN_ATTEMPT:-0}" \
            --arg ref "${GITHUB_REF:-}" \
            --arg event "${GITHUB_EVENT_NAME:-}" \
            --arg workflow "${GITHUB_WORKFLOW:-}" \
            '{sha: $sha, runId: $runId, runNumber: $runNumber, runAttempt: $runAttempt, ref: $ref, event: $event, workflow: $workflow}' \
            > "${stage}/provenance.json" 2>/dev/null || true
    fi
    exit 0

# Run one canonical tier recipe across several packages, continuing past a
# failing package instead of abandoning the ones after it.
#
# `just` aborts a recipe at its first failing line, so the old
# `@just _test a` / `@just _test b` form silently never tested `b` once `a`
# failed — and the run carried no evidence of the gap.
#
# `specs` is a whitespace-separated list of package names, or — when it contains
# a `;` — a `;`-separated list of `<package> [extra args…]` entries, which is how
# an area keeps a per-package flag (`sniff --features remote`). `args` are
# appended to every package.
#
# Exits 0 when every package passed, 1 when any failed, and 130 immediately on
# Ctrl+C so `just check-test-interrupts` keeps holding.
_run_all recipe specs *args="":
    #!/usr/bin/env bash
    set -uo pipefail

    passed_packages=()
    failed_packages=()
    failed_codes=()
    current_package=""

    print_summary() {
        echo
        echo "================================================"
        echo "{{ recipe }} summary"
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
        echo "Interrupted (Ctrl+C)${current_package:+ while running {{ recipe }} for ${current_package}}; remaining packages were not run."
        print_summary
        exit 130
    }
    trap on_interrupt INT

    specs='{{ specs }}'
    entries=()
    if [[ "${specs}" == *";"* ]]; then
        old_ifs="$IFS"
        IFS=';'
        read -r -a raw_entries <<< "${specs}"
        IFS="$old_ifs"
        for entry in "${raw_entries[@]}"; do
            entry="${entry#"${entry%%[![:space:]]*}"}"
            entry="${entry%"${entry##*[![:space:]]}"}"
            [[ -n "${entry}" ]] && entries+=("${entry}")
        done
    else
        for entry in ${specs}; do
            entries+=("${entry}")
        done
    fi

    if (( ${#entries[@]} == 0 )); then
        echo "No packages given to {{ recipe }}." >&2
        exit 1
    fi

    # Give every invocation in this run the SAME `-p` set, so Cargo resolves
    # features once for the whole area instead of once per package.
    #
    # Feature unification is computed over the selected packages. Running
    # `-p claudine` and then `-p claudine-cli` therefore configures shared
    # dependencies differently — claudine-cli's dev-dependency enables
    # `darkmatter/effects-instrumentation`, and `claudine-catalog-types` takes
    # `chrono` with `default-features = false` while its siblings do not — so
    # each invocation rebuilt crates the previous one had already built under a
    # different configuration. Measured on run 30562649123: `claudine` and
    # `claudine-catalog-types` compiled 3x per job. It is invisible locally
    # because Cargo caches every variant; a cold CI cache pays for each.
    #
    # `_test` narrows back to one package with a `package()` filter, which is
    # exactly what the archive path already does, so per-package JUnit staging
    # and the pass/fail summary are unchanged.
    #
    # A per-package `--features X` is NOT a reason to give up the shared scope:
    # Cargo accepts package-qualified features in a multi-package invocation, so
    # `sniff --features remote; sniff-cli --features test-fixtures` becomes
    # `-p sniff -p sniff-cli -F sniff/remote -F sniff-cli/test-fixtures` —
    # identical semantics, one feature resolution.
    #
    # `--all-features` and `--no-default-features` have no package-qualified
    # form; in a multi-package invocation they apply to EVERY selected package,
    # which would change what the siblings are built with rather than merely
    # when. Those areas keep the per-package loop.
    #
    # Only `_test` opts in. `_lint` and `_sanity` share this engine but do not
    # read the exported scope, so they must keep receiving their per-package
    # flags directly.
    build_scope=""
    build_features=""
    scope_translatable=1
    for entry in "${entries[@]}"; do
        # shellcheck disable=SC2206
        scope_parts=(${entry})
        scope_pkg="${scope_parts[0]}"
        build_scope="${build_scope}${build_scope:+ }${scope_pkg}"
        idx=1
        while (( idx < ${#scope_parts[@]} )); do
            scope_arg="${scope_parts[$idx]}"
            scope_vals=""
            case "${scope_arg}" in
                --features|-F)
                    (( idx++ ))
                    scope_vals="${scope_parts[$idx]:-}"
                    ;;
                --features=*) scope_vals="${scope_arg#--features=}" ;;
                *)            scope_translatable=0 ;;
            esac
            if [[ -n "${scope_vals}" ]]; then
                old_ifs="$IFS"
                IFS=','
                read -r -a scope_feats <<< "${scope_vals}"
                IFS="${old_ifs}"
                for scope_feat in "${scope_feats[@]}"; do
                    [[ -n "${scope_feat}" ]] || continue
                    build_features="${build_features}${build_features:+ }${scope_pkg}/${scope_feat}"
                done
            fi
            (( idx++ ))
        done
    done

    shared_scope=0
    if [[ "{{ recipe }}" == "_test" ]] && (( scope_translatable == 1 )) && (( ${#entries[@]} > 1 )); then
        shared_scope=1
        export BISCUIT_BUILD_SCOPE="${build_scope}"
        export BISCUIT_BUILD_FEATURES="${build_features}"
    fi

    for entry in "${entries[@]}"; do
        # Word-splitting is the point: an entry is `<package> [extra args…]`.
        # shellcheck disable=SC2206
        parts=(${entry})
        package="${parts[0]}"
        current_package="${package}"

        if (( shared_scope == 1 )); then
            # Per-package flags were hoisted into BISCUIT_BUILD_FEATURES above;
            # passing them again would apply an unqualified `--features` to a
            # multi-package selection.
            just {{ recipe }} "${package}" {{ args }}
        elif (( ${#parts[@]} > 1 )); then
            just {{ recipe }} "${package}" "${parts[@]:1}" {{ args }}
        else
            just {{ recipe }} "${package}" {{ args }}
        fi
        exit_code=$?

        if (( exit_code == 130 )); then
            on_interrupt
        elif (( exit_code == 0 )); then
            passed_packages+=("${package}")
        else
            failed_packages+=("${package}")
            failed_codes+=("${exit_code}")
        fi
        current_package=""
    done

    print_summary

    if (( ${#failed_packages[@]} > 0 )); then
        exit 1
    fi

# Level-1 tests across several packages. Local development uses one Nextest
# scheduler across the area; CI retains the per-package loop for independent
# JUnit staging and package-level summaries.
_test_all specs *args="":
    #!/usr/bin/env bash
    set -euo pipefail
    if [[ "${NEXTEST_PROFILE:-}" == "ci" || -n "${BISCUIT_CI_ENVIRONMENT:-}" ]]; then
        just _run_all _test '{{ specs }}' {{ args }}
    else
        just _test_local_all '{{ specs }}' {{ args }}
    fi

# Run one L1 nextest invocation across an entire package area. This is the
# local-development engine used by `_test_all`: Nextest schedules all selected
# test binaries concurrently. `specs` has `_run_all`'s shape, so an area can
# retain package-specific feature contracts without splitting the run.
_test_local_all specs *args="": _storage_preflight
    #!/usr/bin/env bash
    set -euo pipefail

    read -r -a nextest_bin <<< "${BISCUIT_NEXTEST_BIN:-cargo nextest}"
    test_threads="${NEXTEST_TEST_THREADS:-$(just _test_threads)}"
    export NEXTEST_TEST_THREADS="$test_threads"
    forwarded=({{ args }})

    specs='{{ specs }}'
    entries=()
    if [[ "${specs}" == *";"* ]]; then
        old_ifs="$IFS"
        IFS=';'
        read -r -a raw_entries <<< "${specs}"
        IFS="$old_ifs"
        for entry in "${raw_entries[@]}"; do
            entry="${entry#"${entry%%[![:space:]]*}"}"
            entry="${entry%"${entry##*[![:space:]]}"}"
            [[ -n "${entry}" ]] && entries+=("${entry}")
        done
    else
        for entry in ${specs}; do
            entries+=("${entry}")
        done
    fi

    selected=()
    package_names=()
    for entry in "${entries[@]}"; do
        # Word-splitting is the point: an entry is `<package> [feature flags]`.
        # shellcheck disable=SC2206
        parts=(${entry})
        package="${parts[0]}"
        package_names+=("${package}")
        selected+=(-p "${package}")

        idx=1
        while (( idx < ${#parts[@]} )); do
            arg="${parts[$idx]}"
            values=""
            case "${arg}" in
                --features|-F)
                    idx=$((idx + 1))
                    values="${parts[$idx]:-}"
                    ;;
                --features=*) values="${arg#--features=}" ;;
                --all-features)
                    values="$(cargo metadata --no-deps --format-version 1 \
                        | jq -r --arg package "${package}" '
                            .packages[]
                            | select(.name == $package)
                            | .features
                            | keys
                            | join(",")
                        ')"
                    ;;
                *)
                    echo "_test_local_all: unsupported per-package argument for ${package}: ${arg}" >&2
                    echo "Only --features/-F and --all-features are supported; pass common Nextest arguments after specs." >&2
                    exit 2
                    ;;
            esac
            old_ifs="$IFS"
            IFS=','
            read -r -a features <<< "${values}"
            IFS="$old_ifs"
            for feature in "${features[@]}"; do
                [[ -n "${feature}" ]] && selected+=(-F "${package}/${feature}")
            done
            idx=$((idx + 1))
        done
    done
    if (( ${#selected[@]} == 0 )); then
        echo "No packages given to _test_local_all." >&2
        exit 1
    fi

    echo
    echo -e "Testing {{ bold }}${package_names[*]}{{ reset }} in one local invocation"
    echo

    if "${nextest_bin[@]}" --version >/dev/null 2>&1; then
        filter="$(just _tier_filter L1)"
        if [[ -z "${BISCUIT_TEST_FILTER:-}" ]] && printf '%s\n' "${package_names[@]}" | grep -Fxq worktree-cli; then
            filter="(${filter}) & !(package(worktree-cli) & test(/(^|::)perf_/))"
        fi
        "${nextest_bin[@]}" run "${selected[@]}" -E "${filter}" --no-tests=pass \
            ${forwarded[@]+"${forwarded[@]}"}
    else
        cargo test "${selected[@]}" ${forwarded[@]+"${forwarded[@]}"} -- \
            --skip level2_ --skip level3_ --skip browser_ --skip real_ --skip slow_ --skip perf_
    fi

# Sanity tier across several packages, continuing past failures. See `_run_all`.
_sanity_all specs *args="":
    @just _run_all _sanity "{{ specs }}" {{ args }}

# Level-2 tier across several packages, continuing past failures. See `_run_all`.
#
# Owns the backend-execution-proof bracket for the WHOLE tier: `reset` runs once
# here, before the first package, and `verify` once after the last. Exporting
# `BISCUIT_BACKEND_PROOF_OWNER` tells the `_test_l2` invocations underneath not
# to bracket themselves — a per-package `reset` would erase the evidence of
# every package that already ran, making the verdict depend on which package
# happened to go last. See `_backend_proof`.
_test_l2_all specs *args="":
    #!/usr/bin/env bash
    set -uo pipefail

    required="${BISCUIT_TEST_REQUIRED_BACKENDS:-}"
    proof_owner=0
    if [[ -n "${required//[[:space:]]/}" ]]; then
        proof_owner=1
        export BISCUIT_BACKEND_PROOF_OWNER=1
        just _backend_proof reset || exit $?
    fi

    exit_code=0
    just _run_all _test_l2 "{{ specs }}" {{ args }} || exit_code=$?

    # Ctrl+C: the run is incomplete, so an unproven backend says nothing. Keep
    # 130 so `just check-test-interrupts` holds.
    if (( exit_code == 130 )); then
        exit 130
    fi

    if (( proof_owner == 1 )); then
        proof_code=0
        just _backend_proof verify || proof_code=$?
        if (( proof_code != 0 )); then
            echo
            echo -e "❌ {{ red }}Level-2 backend proof failed{{ reset }}"
            # A real test failure keeps its own status; the proof only promotes
            # an otherwise-clean run to a failure.
            if (( exit_code == 0 )); then
                exit_code="${proof_code}"
            fi
        fi
    fi

    exit "${exit_code}"

# Run one `backend-proof` subcommand against the shared evidence file.
#
# `BISCUIT_TEST_REQUIRED_BACKENDS` only hard-fails a backend that is *missing*.
# A backend that is installed but never exercised still yields a green tier that
# verified nothing — an installed `tmux` plus zero tmux tests is not evidence.
# `require_level!` therefore appends one execution record per gated test to
# `$STAGE/backend-executions.jsonl`, `reset` discards the previous run's records
# before the tier (stale evidence would satisfy the check), and `verify` fails
# when a required backend produced no executed test.
#
# `--stage-dir` / `--required` are not passed: the checker reads the same
# `BISCUIT_JUNIT_STAGE_DIR` and `BISCUIT_TEST_REQUIRED_BACKENDS` the tests do.
#
# Exit codes: 0 proved (or nothing required), 1 a required backend is unproven,
# 2 bad configuration or unreadable evidence.
_backend_proof command:
    #!/usr/bin/env bash
    set -euo pipefail
    # `BISCUIT_BACKEND_PROOF_BIN` is the archive-mode path: the checker is a
    # `backend-proof` build sidecar the producer compiled and the consumer
    # verified. A consumer with no Cargo cannot build one, and building it here
    # is exactly what archive mode exists to avoid.
    if [[ -n "${BISCUIT_BACKEND_PROOF_BIN:-}" ]]; then
        exec "${BISCUIT_BACKEND_PROOF_BIN}" {{ command }}
    fi
    cargo run -q -p test-toolkit --features backend-proof --bin backend-proof -- {{ command }}

# Print the nextest filter expression a tier selects with.
#
# Single source of truth. `_expected_manifest` has to reproduce a tier's
# selection exactly — expected and observed are only comparable if they were
# chosen by the same expression — and a hand-maintained second copy of these
# strings is precisely the drift that would make an expected-test manifest lie.
#
# `pkg` matters only for L1: `worktree-cli` additionally excludes its `perf_` SLA
# tests, which assert wall-clock budgets and belong to `just test-perf`.
# `BISCUIT_TEST_FILTER` replaces the L1 expression wholesale — only areas whose
# L1 contract genuinely differs should set it. `BISCUIT_L1_INCLUDE_SLOW=1`
# keeps `slow_` tests inside L1 instead of excluding them, which is darkmatter's
# declared L1 contract (`l1-include-slow` in `[package.metadata.ci.tests]`).
# The explicit `BISCUIT_TEST_FILTER` override wins over both.
#
# ## A tier marker is a prefix, not a substring
#
# `test(/…/)` is an unanchored regex search over the *test name*
# (`module::path::test_name` — nextest does not match it against the binary id),
# so a bare `test(/browser_/)` also matched `unresolved_browser_feature_…` and
# `image_emits_typed_browser_attributes`. Those are ordinary in-process unit
# tests; the marker was never meant to be readable mid-word. Every predicate
# therefore anchors to a path-segment boundary with `(^|::)`. `renderable` lost
# 8 tests to this — excluded from L1, and its `test-browser` is a stub, so they
# ran nowhere. `just check-tier-coverage` now fails on that combination.
_tier_filter tier pkg="":
    #!/usr/bin/env bash
    set -euo pipefail
    base='!(test(/(^|::)level2_/) + test(/(^|::)level3_/) + test(/(^|::)browser_/) + test(/(^|::)real_/) + test(/(^|::)slow_/))'
    case '{{ tier }}' in
        L1|l1)
            filter="${base}"
            if [[ '{{ pkg }}' == "worktree-cli" ]]; then
                filter='!(test(/(^|::)level2_/) + test(/(^|::)level3_/) + test(/(^|::)browser_/) + test(/(^|::)real_/) + test(/(^|::)slow_/) + test(/(^|::)perf_/))'
            elif [[ "${BISCUIT_L1_INCLUDE_SLOW:-}" == "1" ]]; then
                filter='!(test(/(^|::)level2_/) + test(/(^|::)level3_/) + test(/(^|::)browser_/) + test(/(^|::)real_/))'
            fi
            printf '%s\n' "${BISCUIT_TEST_FILTER:-${filter}}"
            ;;
        sanity)  printf '%s\n' "${base}" ;;
        L2|l2)       printf '%s\n' 'test(/(^|::)level2_/)' ;;
        L3|l3)       printf '%s\n' 'test(/(^|::)level3_/)' ;;
        browser)     printf '%s\n' 'test(/(^|::)browser_/)' ;;
        real)        printf '%s\n' 'test(/(^|::)real_/)' ;;
        *)
            echo "_tier_filter: unknown tier '{{ tier }}' (expected L1, L2, L3, browser, real, sanity)" >&2
            exit 1
            ;;
    esac

# Print the arguments that survive an archive-mode nextest invocation, one per
# line; report the dropped ones on stderr.
#
# `--archive-file` is mutually exclusive with *every* Cargo build flag, because
# the binaries already exist — nextest rejects the combination outright rather
# than ignoring it. An area that carries a per-package build flag
# (`_test_all "messenger --features desktop; messenger-cli"`) would therefore be
# unable to use the archive path at all. The features are not lost by dropping
# them here: they were baked into the archive at build time, which is why the
# package's declared features (`[package.metadata.ci.tests]`) also reach
# `_wsl-ci.yml`'s `cargo nextest archive` invocation.
_archive_drop_build_flags *args="":
    #!/usr/bin/env bash
    set -euo pipefail
    supplied=({{ args }})
    kept=()
    dropped=()
    skip_next=0
    for arg in ${supplied[@]+"${supplied[@]}"}; do
        if (( skip_next == 1 )); then
            skip_next=0
            dropped+=("${arg}")
            continue
        fi
        case "${arg}" in
            --features|--target|--target-dir|--cargo-profile|--build-jobs)
                skip_next=1
                dropped+=("${arg}")
                ;;
            --features=*|--target=*|--target-dir=*|--cargo-profile=*|--build-jobs=*)
                dropped+=("${arg}")
                ;;
            --all-features|--no-default-features|--release|--lib|--bins|--tests|--benches|--examples|--all-targets)
                dropped+=("${arg}")
                ;;
            *)
                kept+=("${arg}")
                ;;
        esac
    done
    if (( ${#dropped[@]} > 0 )); then
        echo -e "  {{ dim }}archive mode: dropped build-only flag(s) ${dropped[*]} — already baked into the archive{{ reset }}" >&2
    fi
    for arg in ${kept[@]+"${kept[@]}"}; do
        printf '%s\n' "${arg}"
    done

# Run tests with `cargo test` or `cargo nextest run`
#
# ## Archive mode
#
# Passing `--archive-file` (with `--workspace-remap`) runs pre-built binaries
# from a `cargo nextest archive` instead of compiling. The wsl2-ubuntu leg builds
# `x86_64-unknown-linux-gnu` once on `ubuntu-latest` and executes the archive
# inside a WSL2 guest that has no rustup, no cargo, and no rustc — only `just`
# and a standalone `cargo-nextest`. Two things follow:
#
#   * `BISCUIT_NEXTEST_BIN` (default `cargo nextest`) drives BOTH the
#     availability probe and the invocation. Without it the probe fails in the
#     guest, the recipe falls back to `cargo test`, and the leg dies with
#     `cargo: command not found`.
#   * `-p <pkg>` cannot be used with `--archive-file`, so the package selection
#     moves into the filterset. See `_archive_drop_build_flags` for the build
#     flags that go the same way.
#
# A `cargo test` fallback cannot consume an archive at all — it would recompile —
# so a missing nextest in archive mode is a hard error, never a fallback.
_build_messenger_desktop_stubs:
    #!/usr/bin/env bash
    set -euo pipefail

    # CI and archive runners supply a verified fixture directory. In local
    # builds, compile every fixture in one Cargo process before nextest starts
    # its isolated per-test processes.
    if [[ -n "${MESSENGER_STUB_BIN_DIR:-}" ]]; then
        exit 0
    fi

    cargo build --features desktop -p messenger \
        --bin stub_dunstify \
        --bin stub_notify_send \
        --bin stub_snoretoast \
        --bin stub_burnttoast \
        --bin stub_terminal_notifier \
        --bin stub_alerter

_test pkg *args="": _storage_preflight
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

    # One overridable nextest invocation for both the probe and the run.
    read -r -a nextest_bin <<< "${BISCUIT_NEXTEST_BIN:-cargo nextest}"
    test_threads="${NEXTEST_TEST_THREADS:-$(just _test_threads)}"
    export NEXTEST_TEST_THREADS="$test_threads"

    forwarded=({{ args }})
    archive_mode=0
    archive_file=""
    want_archive=0
    for arg in ${forwarded[@]+"${forwarded[@]}"}; do
        if (( want_archive )); then archive_file="${arg}"; want_archive=0; continue; fi
        case "${arg}" in
            --archive-file) archive_mode=1; want_archive=1 ;;
            --archive-file=*) archive_mode=1; archive_file="${arg#--archive-file=}" ;;
        esac
    done

    if (( archive_mode == 0 )) && [[ "{{ pkg }}" == "messenger" ]]; then
        just _build_messenger_desktop_stubs
    fi

    if (( archive_mode == 1 )) && ! "${nextest_bin[@]}" --version >/dev/null 2>&1; then
        echo -e "❌ {{ red }}--archive-file requires cargo-nextest{{ reset }}" >&2
        echo "   '${nextest_bin[*]} --version' failed, and the cargo test fallback cannot" >&2
        echo "   consume an archive — it recompiles, which is the whole thing an archive avoids." >&2
        echo -e "   Set {{ bold }}BISCUIT_NEXTEST_BIN{{ reset }} (the WSL leg uses \"cargo-nextest nextest\")." >&2
        exit 1
    fi
    if (( archive_mode == 1 )); then
        just _archive_file_check "${archive_file}"
    fi

    just _stage_junit_reset

    exit_code=0
    if "${nextest_bin[@]}" --version >/dev/null 2>&1; then
        echo -e "ℹ️ using {{ green }}${nextest_bin[*]} run{{ reset }}"
        filter="$(just _tier_filter L1 '{{ pkg }}')"
        selection=(-p '{{ pkg }}')
        run_args=(${forwarded[@]+"${forwarded[@]}"})
        # Shared build scope (set by `_run_all`): select every package in the
        # area so Cargo resolves features once, then narrow to this package with
        # `package()`. Same shape as the archive branch below.
        if (( archive_mode == 0 )) && [[ -n "${BISCUIT_BUILD_SCOPE:-}" ]]; then
            selection=()
            for scope_pkg in ${BISCUIT_BUILD_SCOPE}; do
                selection+=(-p "${scope_pkg}")
            done
            # Package-qualified, so each package still gets exactly the features
            # its own spec asked for.
            for scope_feat in ${BISCUIT_BUILD_FEATURES:-}; do
                selection+=(-F "${scope_feat}")
            done
            filter="package({{ pkg }}) & (${filter})"
        fi
        if (( archive_mode == 1 )); then
            selection=()
            filter="package({{ pkg }}) & (${filter})"
            run_args=()
            while IFS= read -r kept_arg; do
                run_args+=("${kept_arg}")
            done < <(just _archive_drop_build_flags ${forwarded[@]+"${forwarded[@]}"})
        fi
        "${nextest_bin[@]}" run ${selection[@]+"${selection[@]}"} -E "$filter" --no-tests=pass ${run_args[@]+"${run_args[@]}"} || exit_code=$?
    else
        echo -e "ℹ️ using {{ green }}cargo test{{ reset }}"
        skip_args=(--skip level2_ --skip level3_ --skip browser_ --skip real_ --skip slow_)
        if [[ "{{ pkg }}" == "worktree-cli" ]]; then
            skip_args+=(--skip perf_)
        fi
        cargo test -p {{ pkg }} {{ args }} -- "${skip_args[@]}" || exit_code=$?
    fi

    duration=$SECONDS
    just _stage_junit L1 "{{ pkg }}" "${exit_code}" "${duration}"
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

# Emit the expected-test manifest `scripts/ci/completion.py` compares against.
#
# A JUnit report cannot distinguish a test that does not exist on this
# environment (compile-time `#[cfg]`) from a test that silently stopped running:
# both are simply absent. Absence must therefore never be *inferred* from the
# report. `cargo nextest list` is what tells a producer what the compiled
# binaries on this environment actually contain, which is why the manifest has
# to be generated ON the execution target, from the same binaries the gate ran.
#
# The hazard it closes is live: `_test` passes `--no-tests=pass`, so a tier that
# selects zero tests exits 0 and stages a valid, empty report — a drifted filter
# expression would quietly downgrade a real package to "nothing to run" with
# nothing anywhere failing. Comparing against the manifest turns that into N
# absent tests.
#
# `tier` is `L1` / `L2` / `L3` / `browser` / `real` / `sanity`. `specs` follows
# `_run_all`: whitespace-separated package names, or — when it contains a `;` —
# `;`-separated `<package> [extra args…]` entries. The filter comes from
# `_tier_filter`, the same source the tier recipe reads, so expected and observed
# are selected by one expression rather than two that can drift apart.
#
# Writes `$STAGE/expected-<tier>.json`, where `$STAGE` is the staging directory
# `_stage_junit` uses. `environment` comes from `$BISCUIT_CI_ENVIRONMENT` — the
# same value the manifest records carry, because expectations are keyed by
# `{environment, tier}` and there is no other way to join the two.
#
# ## Schema version 2
#
# Version 1 recorded only the identities the tier selected, so an identity that
# never appeared in the report could be a `cfg`-absent test, an `#[ignore]`d
# one, another tier's, or a lost one — four different verdicts wearing the same
# silence. Version 2 splits them and records where the listing came from:
#
# * `tests` / `ignored` / `excluded` — the three disjoint identity sets, from
#   `filter-match.status` and its `reason` (S2 proved `ignored` and `expression`
#   are distinguishable at the resolved version). A `cfg`-excluded test is in
#   none of them: it is absent from the listing entirely, which is the correct
#   answer for this target and the reason `target` is recorded.
# * `target` — read from the listing's own `rust-build-meta`, not from `rustc`.
#   An archive consumer has no compiler, and in archive mode this is the
#   PRODUCER's target, which is exactly the one `cfg` was evaluated against.
# * `nextest_version` — R4: the repository pins no version, so the guarantee is
#   "the binary in this job", and the resolved string is recorded so a
#   cross-host mismatch is visible after the fact.
# * `selection` — the expression, profile, and test arguments applied. An ad hoc
#   `-E 'test(one_name)'` shrinks the expected and the observed sets together
#   and is invisible to a set comparison; recording what was selected with is
#   what lets `completion.py` refuse it.
# * `backends` / `companion_suites` — optional, and only what this job was TOLD
#   to require (`$BISCUIT_TEST_REQUIRED_BACKENDS`,
#   `$BISCUIT_CI_COMPANION_SUITES`). The plan remains the source of truth;
#   `completion.py` cross-checks these against it, so a job provisioned for
#   different work than the plan scheduled says so.
#
# ## Listing runs the test binaries, so provisioning comes first
#
# `nextest list` executes each test binary to enumerate its tests. A binary
# that needs a native library, a sidecar, or a pinned toolchain fails to start
# here exactly as it would in the gate — so the caller must provision native
# prerequisites, sidecars, backends, and (for `requires-toolchain` packages) the
# toolchain BEFORE calling this recipe. A consumer needs no compiler to list:
# archive mode runs the binaries the producer already built. A listing failure
# is reported with that order named, because an unprovisioned listing looks
# identical to a drifted filter otherwise.
_expected_manifest tier specs *args="":
    #!/usr/bin/env bash
    set -euo pipefail

    environment="${BISCUIT_CI_ENVIRONMENT:-}"
    if [[ -z "${environment}" ]]; then
        echo "_expected_manifest: BISCUIT_CI_ENVIRONMENT must be set." >&2
        echo "  The manifest is keyed by {environment, tier} and asserts what THIS" >&2
        echo "  environment compiled; an unlabelled one cannot be joined to any cell." >&2
        exit 1
    fi
    if ! command -v jq >/dev/null 2>&1; then
        echo "_expected_manifest: jq is required." >&2
        exit 1
    fi

    read -r -a nextest_bin <<< "${BISCUIT_NEXTEST_BIN:-cargo nextest}"

    # No `_archive_file_check` here: this runs at STAGING time, after the tier
    # has already read the archive. It only needs to know that it did.
    forwarded=({{ args }})
    archive_mode=0
    for arg in ${forwarded[@]+"${forwarded[@]}"}; do
        case "${arg}" in
            --archive-file|--archive-file=*) archive_mode=1 ;;
        esac
    done

    workspace_root="${BISCUIT_JUNIT_WORKSPACE_ROOT:-}"
    if [[ -z "${workspace_root}" ]]; then
        meta_file="$(mktemp "${TMPDIR:-/tmp}/biscuit-expected-meta.XXXXXX")"
        cargo metadata --no-deps --format-version 1 > "${meta_file}"
        workspace_root="$(jq -r '.workspace_root // empty' "${meta_file}")"
        rm -f "${meta_file}"
    fi
    stage="${BISCUIT_JUNIT_STAGE_DIR:-target/nextest/ci-reports}"
    case "${stage}" in
        /* | ?:[/\\]*) ;;
        *) stage="${workspace_root%/}/${stage}" ;;
    esac
    mkdir -p "${stage}"

    specs='{{ specs }}'
    entries=()
    if [[ "${specs}" == *";"* ]]; then
        old_ifs="$IFS"
        IFS=';'
        read -r -a raw_entries <<< "${specs}"
        IFS="$old_ifs"
        for entry in "${raw_entries[@]}"; do
            entry="${entry#"${entry%%[![:space:]]*}"}"
            entry="${entry%"${entry##*[![:space:]]}"}"
            [[ -n "${entry}" ]] && entries+=("${entry}")
        done
    else
        for entry in ${specs}; do
            entries+=("${entry}")
        done
    fi
    if (( ${#entries[@]} == 0 )); then
        echo "_expected_manifest: no packages given." >&2
        exit 1
    fi

    # R4: no version is pinned anywhere in this repository, so the guarantee is
    # "the binary this job installed". Record its first `--version` line; the
    # remaining lines are recoverable from it.
    # Captured whole, then trimmed: `… --version | head -n 1` closes the pipe
    # after one line, and under `pipefail` the resulting EPIPE would fail the
    # recipe on a host whose pipe buffer is smaller than the version banner.
    version_banner="$("${nextest_bin[@]}" --version 2>/dev/null || true)"
    nextest_version="${version_banner%%$'\n'*}"
    if [[ -z "${nextest_version}" ]]; then
        echo "_expected_manifest: '${nextest_bin[*]} --version' produced nothing." >&2
        echo "  The manifest records which nextest listed it, so a cross-host" >&2
        echo "  mismatch is visible after the fact; an unlabelled listing is not." >&2
        exit 1
    fi

    parts_file="$(mktemp "${TMPDIR:-/tmp}/biscuit-expected-parts.XXXXXX")"
    listing="$(mktemp "${TMPDIR:-/tmp}/biscuit-expected-list.XXXXXX")"
    trap 'rm -f "${parts_file}" "${listing}"' EXIT

    target=""
    applied_filter=""
    for entry in "${entries[@]}"; do
        # Word-splitting is the point: an entry is `<package> [extra args…]`.
        # shellcheck disable=SC2206
        parts=(${entry})
        package="${parts[0]}"
        extra=(${parts[@]+"${parts[@]:1}"})

        filter="$(just _tier_filter '{{ tier }}' "${package}")"
        selection=(-p "${package}")
        list_args=(${extra[@]+"${extra[@]}"} ${forwarded[@]+"${forwarded[@]}"})
        if (( archive_mode == 1 )); then
            selection=()
            filter="package(${package}) & (${filter})"
            list_args=()
            while IFS= read -r kept_arg; do
                list_args+=("${kept_arg}")
            done < <(just _archive_drop_build_flags ${extra[@]+"${extra[@]}"} ${forwarded[@]+"${forwarded[@]}"})
        fi
        # One package: the expression actually applied. Several: the tier's own,
        # because there is no single applied expression to honestly name.
        if (( ${#entries[@]} == 1 )); then
            applied_filter="${filter}"
        else
            applied_filter="$(just _tier_filter '{{ tier }}')"
        fi

        echo -e "🧾 Listing {{ bold }}{{ tier }}{{ reset }} tests for {{ bold }}${package}{{ reset }} on {{ bold }}${environment}{{ reset }}"
        if ! "${nextest_bin[@]}" list ${selection[@]+"${selection[@]}"} -E "${filter}" \
            --message-format json ${list_args[@]+"${list_args[@]}"} > "${listing}"; then
            echo >&2
            echo "_expected_manifest: listing ${package} failed." >&2
            echo "  Listing RUNS each test binary to enumerate it, so this fails for the" >&2
            echo "  same reasons the gate would: an unprovisioned native prerequisite, a" >&2
            echo "  missing sidecar or backend, or an absent pinned toolchain. Provision" >&2
            echo "  before listing; do not narrow the filter to get past it." >&2
            exit 1
        fi

        # `list` reports EVERY discovered test with a `filter-match` verdict, and
        # v2 keeps all three verdicts apart: `matches` must produce a result,
        # `mismatch`+`ignored` was excused by `#[ignore]`, and `mismatch`+
        # anything else is another tier's. Identity is `<binary-id>::<test
        # name>`, which is how a JUnit `<testsuite name>` + `<testcase name>`
        # rebuilds it. `test-count` counts discoveries, never matches (S2).
        jq -c --arg pkg "${package}" '
            def cases:
                .["rust-suites"] | to_entries[] as $e
                | select($e.value["package-name"] == $pkg)
                | (($e.value.testcases) // {}) | to_entries[]
                | { id: ((($e.value["binary-id"]) // $e.key) + "::" + .key),
                    m: (.value["filter-match"]) };
            { ($pkg): {
                tests:    ([ cases | select(.m.status == "matches") | .id ] | unique),
                ignored:  ([ cases | select(.m.status != "matches" and .m.reason == "ignored") | .id ] | unique),
                excluded: ([ cases | select(.m.status != "matches" and .m.reason != "ignored") | .id ] | unique) } }
        ' "${listing}" >> "${parts_file}"

        if [[ -z "${target}" ]]; then
            # The listing's own build metadata, so no consumer needs `rustc`.
            # `target-platforms` is the compiled-for triple; the host platform
            # is the fallback for a listing that names no explicit target.
            target="$(jq -r '
                .["rust-build-meta"]
                | (.["target-platforms"][0].triple?
                   // .platforms.host.platform.triple?
                   // .["target-platform"]?
                   // empty)
            ' "${listing}")"
        fi
    done

    if [[ -z "${target}" ]]; then
        echo "_expected_manifest: the listing named no target triple." >&2
        echo "  A listing from another target proves nothing about this one —" >&2
        echo "  cfg-compiled tests differ — so an unlabelled manifest is refused." >&2
        exit 1
    fi

    from_archive=false
    if (( archive_mode == 1 )); then
        from_archive=true
    fi

    # Optional, and only what THIS job was told to require. The plan stays the
    # source of truth; `completion.py` refuses a disagreement rather than
    # preferring either side.
    backends_json='[]'
    if [[ -n "${BISCUIT_TEST_REQUIRED_BACKENDS:-}" ]]; then
        backends_json="$(printf '%s' "${BISCUIT_TEST_REQUIRED_BACKENDS}" \
            | jq -R 'split(",") | map(gsub("^\\s+|\\s+$"; "")) | map(select(length > 0)) | unique')"
    fi
    companions_json='[]'
    if [[ -n "${BISCUIT_CI_COMPANION_SUITES:-}" ]]; then
        companions_json="$(printf '%s' "${BISCUIT_CI_COMPANION_SUITES}" \
            | jq -R 'split(",") | map(gsub("^\\s+|\\s+$"; "")) | map(select(length > 0)) | unique')"
    fi

    out="${stage}/expected-{{ tier }}.json"
    jq -s \
        --arg environment "${environment}" \
        --arg tier '{{ tier }}' \
        --arg target "${target}" \
        --arg nextest_version "${nextest_version}" \
        --argjson from_archive "${from_archive}" \
        --arg filter "${applied_filter}" \
        --arg profile "${NEXTEST_PROFILE:-default}" \
        --arg test_args '{{ args }}' \
        --argjson backends "${backends_json}" \
        --argjson companion_suites "${companions_json}" \
        '{schema_version: 2, environment: $environment, tier: $tier, target: $target,
          nextest_version: $nextest_version, from_archive: $from_archive,
          selection: {filter: $filter, profile: $profile, test_args: $test_args},
          backends: $backends, companion_suites: $companion_suites,
          packages: (add // {})}' \
        "${parts_file}" > "${out}"

    total="$(jq '[.packages[].tests | length] | add // 0' "${out}")"
    other="$(jq '[.packages[] | (.ignored | length) + (.excluded | length)] | add // 0' "${out}")"
    echo -e "✅ {{ green }}expected-test manifest{{ reset }} ${out} {{ dim }}(${total} expected, ${other} ignored/excluded, ${target}){{ reset }}"

# Run Level-1 tests for every workspace package, continuing after failures.
#
# Package discovery comes from Cargo metadata rather than the curated package-area
# list. Optional selectors may be package names or package-area paths.
_test_workspace *selectors="": _storage_preflight
    #!/usr/bin/env bash
    set -euo pipefail

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
    echo "Running Level-1 tests for ${package_count} workspace packages in one invocation..."
    echo "------------------------------------------------"

    # CI features are the existing package-level coverage contract. A package
    # may provide `local-features` when CI additionally builds helpers for a
    # higher tier; presence (including an empty array) overrides `features`.
    specs=""
    while IFS= read -r package; do
        features="$(jq -r --arg package "${package}" '
            .packages[]
            | select(.name == $package)
            | . as $record
            | (.metadata.ci.tests // {}) as $tests
            | if ($tests | has("local-features"))
              then ($tests["local-features"] // [])
              elif ($tests["all-features"] // false)
              then ($record.features | keys)
              else ($tests.features // [])
              end
            | join(",")
        ' "${metadata_file}")"
        entry="${package}"
        if [[ -n "${features}" ]]; then
            entry="${entry} --features ${features}"
        fi
        if [[ -n "${specs}" ]]; then
            specs="${specs}; ${entry}"
        else
            specs="${entry}"
        fi
    done < "${selected_file}"

    # One scheduler sees every selected test binary. `--no-fail-fast` is baked in
    # rather than left to nextest's default because this is the repository's
    # broadest local scope, and re-running it is expensive enough to sit on the CI
    # side of the fail-fast rule's cost-of-next-run test (`rust-testing` skill).
    # At the repository root the caller cannot re-specify it — every argument is
    # consumed as a selector above — so selector-narrowed runs inherit it by
    # design. Area `test` recipes forward their arguments to nextest and keep the
    # fail-fast default.
    just _test_local_all "${specs}" --no-fail-fast

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

# Fail when a tier filter strands tests behind a stub recipe.
#
# A tier marker takes a test OUT of L1 unconditionally; only the area's own
# `test-<tier>` recipe can put it back IN. When that recipe is a stub —
# `@echo "…: not applicable…"` — the two halves disagree and every test the
# filter caught runs in no tier at all. Nothing else reports that: the L1 run is
# green because it never saw them, and the stub exits 0 because it does nothing.
#
# `renderable` sat in exactly that state with 16 tests, 8 of them ordinary unit
# tests the then-unanchored filter matched mid-name (`…_browser_…`). This guard
# would have failed the day the first one was written.
#
# Selection comes from `_tier_filter`, so the guard asks precisely the question
# the tier recipe would have asked.
#
# Cost is why this is its own recipe and not part of `test`, `lint`, or a
# pre-push hook: answering it needs `cargo nextest list`, which builds the area's
# test binaries. Areas that stub no tier are skipped before any cargo runs, and a
# stubbing area is listed only for the tiers it stubs. All of an area's packages
# go into ONE `list` per tier, which also pins the feature resolution — listing
# `-p biscuit-icon` and `-p biscuit-icon-cli` separately unifies features
# differently each time and rebuilds the world between them.
#
# Known blind spot: the listing uses default features, because an area's real
# feature flags live in its own recipe arguments and are not recoverable from
# here. A stranded test behind a non-default feature is invisible. That only
# hides a violation when EVERY stranded test in a tier is feature-gated; one
# ungated sibling still fails the area.
#
# Optional `args` restrict the run to named area directories.
_check_tier_coverage *args="":
    #!/usr/bin/env bash
    set -uo pipefail

    if ! command -v jq >/dev/null 2>&1; then
        echo "_check_tier_coverage: jq is required." >&2
        exit 2
    fi

    # A stub is a recipe whose every non-blank body line says "not applicable".
    # The body runs from the header to the first line back at column 0.
    #
    # The pipeline's exit status is deliberately ignored: `sed … q` closes the
    # pipe as soon as it sees that line, so under `pipefail` the whole
    # substitution reports its upstream's SIGPIPE (141) on a perfectly good read.
    # The captured text is the only signal here.
    is_stub_recipe() {
        local justfile="$1" recipe="$2" body
        body="$(sed -n "/^${recipe}[ :]/,\$p" "${justfile}" \
            | tail -n +2 \
            | sed '/^[^[:space:]]/q' \
            | sed '/^[^[:space:]]/d' \
            | grep -v '^[[:space:]]*$')"
        [[ -n "${body}" ]] || return 1
        ! grep -qv 'not applicable' <<< "${body}"
    }

    declare -a target_areas=()
    if [[ -n "{{ args }}" ]]; then
        read -r -a target_areas <<< "{{ args }}"
    else
        while IFS= read -r justfile; do
            area="${justfile%/justfile}"
            # The root justfile has no directory component and owns no packages.
            [[ "${area}" == "${justfile}" ]] && continue
            target_areas+=("${area}")
        done < <(rg --files -g justfile | sort)
    fi

    meta_file="$(mktemp "${TMPDIR:-/tmp}/biscuit-tier-meta.XXXXXX")"
    listing="$(mktemp "${TMPDIR:-/tmp}/biscuit-tier-list.XXXXXX")"
    # bash runs an inherited EXIT trap when a `$(…)` subshell finishes, so an
    # unguarded `rm` here deletes both scratch files partway through the first
    # command substitution and the rest of the run reads empty ones. `BASHPID`
    # is per-process where `$$` is not, so it distinguishes the two.
    main_pid="${BASHPID}"
    trap '[[ "${BASHPID}" == "${main_pid}" ]] && rm -f "${meta_file}" "${listing}"' EXIT
    cargo metadata --no-deps --format-version 1 > "${meta_file}" || exit 2
    workspace_root="$(jq -r '.workspace_root' "${meta_file}")"

    failures=()
    checked=()
    skipped=()

    for area in "${target_areas[@]}"; do
        justfile="${area}/justfile"
        [[ -f "${justfile}" ]] || continue

        stubbed=()
        for tier in l2 l3 browser real; do
            if is_stub_recipe "${justfile}" "test-${tier}"; then
                stubbed+=("${tier}")
            fi
        done
        if (( ${#stubbed[@]} == 0 )); then
            skipped+=("${area}")
            continue
        fi

        packages=()
        while IFS= read -r pkg; do
            [[ -n "${pkg}" ]] && packages+=("${pkg}")
        done < <(jq -r --arg prefix "${workspace_root%/}/${area}/" \
            '.packages[] | select(.manifest_path | startswith($prefix)) | .name' "${meta_file}")
        if (( ${#packages[@]} == 0 )); then
            skipped+=("${area}")
            continue
        fi

        selection=()
        for package in "${packages[@]}"; do
            selection+=(-p "${package}")
        done

        echo -e "🔎 {{ bold }}${area}{{ reset }} {{ dim }}stubs: ${stubbed[*]} — listing ${packages[*]}{{ reset }}"
        for tier in "${stubbed[@]}"; do
            filter="$(just _tier_filter "${tier}")"
            if ! cargo nextest list "${selection[@]}" -E "${filter}" \
                --message-format json > "${listing}" 2>/dev/null; then
                echo -e "  {{ red }}❌ could not list ${area} for ${tier}{{ reset }}"
                failures+=("${area} (${tier}: listing failed)")
                continue
            fi
            stranded=()
            while IFS= read -r line; do
                [[ -n "${line}" ]] && stranded+=("${line}")
            done < <(jq -r '
                .["rust-suites"][]?
                | ."binary-id" as $binary
                | (.testcases // {})
                | to_entries[]
                | select(.value["filter-match"].status == "matches")
                | "\($binary)::\(.key)"
            ' "${listing}")
            (( ${#stranded[@]} == 0 )) && continue
            echo -e "  {{ red }}❌ ${#stranded[@]} test(s) match the ${tier} tier, but test-${tier} is a stub{{ reset }}"
            printf '      %s\n' "${stranded[@]}"
            failures+=("${area} (${tier}: ${#stranded[@]} stranded)")
        done
        checked+=("${area}")
    done

    echo
    echo "Tier coverage"
    echo "================================================"
    echo -e "{{ dim }}No stubbed tier (not listed): ${#skipped[@]}{{ reset }}"
    echo -e "Listed (${#checked[@]}): ${checked[*]:-(none)}"
    echo -e "{{ red }}Stranded{{ reset }} (${#failures[@]}): ${failures[*]:-(none)}"
    echo "================================================"
    echo

    if (( ${#failures[@]} > 0 )); then
        echo "A stranded test runs in no tier. Either rename it so it carries no tier"
        echo "marker, or give the area a real test-<tier> recipe."
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
_sanity pkg *args="": _storage_preflight
    #!/usr/bin/env bash
    set -euo pipefail
    if [[ "{{ pkg }}" == "messenger" ]]; then
        just _build_messenger_desktop_stubs
    fi
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
    SECONDS=0
    just _stage_junit_reset
    read -r -a nextest_bin <<< "${BISCUIT_NEXTEST_BIN:-cargo nextest}"
    test_threads="${NEXTEST_TEST_THREADS:-$(just _test_threads)}"
    export NEXTEST_TEST_THREADS="$test_threads"
    exit_code=0
    "${nextest_bin[@]}" run -p {{ pkg }} "${targets[@]}" \
        -E "$(just _tier_filter sanity '{{ pkg }}')" \
        --no-tests=pass {{ args }} || exit_code=$?
    just _stage_junit sanity "{{ pkg }}" "${exit_code}" "${SECONDS}"
    exit "${exit_code}"

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
_test_l2 pkg *args="": _storage_preflight
    #!/usr/bin/env bash
    set -euo pipefail
    echo
    echo -e "🖥  Level-2 tests for {{ bold }}{{ pkg }}{{ reset }}"
    echo

    read -r -a nextest_bin <<< "${BISCUIT_NEXTEST_BIN:-cargo nextest}"
    test_threads="${NEXTEST_TEST_THREADS:-$(just _test_threads)}"
    export NEXTEST_TEST_THREADS="$test_threads"

    # Archive mode, exactly as `_test` spells it: `--archive-file` is mutually
    # exclusive with `-p` and with every Cargo build flag, so the package
    # selection moves into the filterset and the build flags are dropped (they
    # were baked in at archive time).
    forwarded=({{ args }})
    archive_mode=0
    archive_file=""
    want_archive=0
    for arg in ${forwarded[@]+"${forwarded[@]}"}; do
        if (( want_archive )); then archive_file="${arg}"; want_archive=0; continue; fi
        case "${arg}" in
            --archive-file) archive_mode=1; want_archive=1 ;;
            --archive-file=*) archive_mode=1; archive_file="${arg#--archive-file=}" ;;
        esac
    done

    filter="$(just _tier_filter L2)"
    selection=(-p '{{ pkg }}')
    run_args=(${forwarded[@]+"${forwarded[@]}"})
    if (( archive_mode == 1 )); then
        if ! "${nextest_bin[@]}" --version >/dev/null 2>&1; then
            echo -e "❌ {{ red }}--archive-file requires cargo-nextest{{ reset }}" >&2
            echo "   '${nextest_bin[*]} --version' failed, and there is no fallback that can" >&2
            echo "   consume an archive — every alternative recompiles." >&2
            exit 1
        fi
        just _archive_file_check "${archive_file}"
        selection=()
        filter="package({{ pkg }}) & (${filter})"
        run_args=()
        while IFS= read -r kept_arg; do
            run_args+=("${kept_arg}")
        done < <(just _archive_drop_build_flags ${forwarded[@]+"${forwarded[@]}"})
    fi

    # Every nextest invocation below is bracketed so its JUnit report is staged
    # before the next one can overwrite it — including a failing invocation.
    run_l2() {
        just _stage_junit_reset
        local code=0
        SECONDS=0
        "$@" || code=$?
        just _stage_junit L2 "{{ pkg }}" "${code}" "${SECONDS}"
        return "${code}"
    }

    # Backend-execution proof. Scope is the TIER, not the package: when
    # `_test_l2_all` drives several packages it owns the bracket and exports
    # `BISCUIT_BACKEND_PROOF_OWNER`, and this recipe leaves it alone. See
    # `_backend_proof` for what is being proved.
    #
    # Entirely inert when no backend is required — the guard is on the env var,
    # so a local run pays neither the `cargo run` startup nor any file I/O.
    required_backends="${BISCUIT_TEST_REQUIRED_BACKENDS:-}"
    proof_owner=0
    if [[ -n "${required_backends//[[:space:]]/}" && -z "${BISCUIT_BACKEND_PROOF_OWNER:-}" ]]; then
        proof_owner=1
        just _backend_proof reset
    fi

    # Single exit point for every branch below, so the proof runs exactly once
    # however the tier finished. A genuine test failure keeps its own exit code
    # and is never masked; an unproven backend promotes an otherwise-clean run
    # to a failure. Ctrl+C (130) skips the proof — the run is incomplete.
    finish() {
        local code="${1}"
        if (( proof_owner == 1 )) && (( code != 130 )); then
            local proof_code=0
            just _backend_proof verify || proof_code=$?
            if (( proof_code != 0 )); then
                echo
                echo -e "❌ {{ red }}Level-2 backend proof failed{{ reset }} for {{ bold }}{{ pkg }}{{ reset }}"
                if (( code == 0 )); then
                    code="${proof_code}"
                fi
            fi
        fi
        exit "${code}"
    }

    # Parallel self-spawn mode: no shared pane, no broker, `-j N`. Every
    # test self-isolates via `shared_or_spawn()`'s owned fallback, so
    # there is nothing to contend for. See the recipe comment above.
    l2_threads="${BISCUIT_L2_THREADS:-1}"
    if [[ "${l2_threads}" != "1" ]]; then
        echo -e "  {{ dim }}parallel self-spawn mode: -j ${l2_threads}, no shared pane{{ reset }}"
        echo
        exit_code=0
        run_l2 "${nextest_bin[@]}" run ${selection[@]+"${selection[@]}"} -E "$filter" --no-tests=pass -j "${l2_threads}" ${run_args[@]+"${run_args[@]}"} || exit_code=$?
        finish "${exit_code}"
    fi

    # Append `.exe` on Windows so the broker resolves under git-bash;
    # `MSYSTEM` is set by MSYS2 / git-bash. macOS and Linux ignore it.
    bin_suffix=""
    if [[ -n "${MSYSTEM:-}" || "${OS:-}" == "Windows_NT" ]]; then
        bin_suffix=".exe"
    fi
    # `BISCUIT_HARNESS_BROKER_BIN` is the archive-mode path: the broker is a
    # `harness-broker` build sidecar the producer compiled and the consumer
    # verified, because a consumer with no Cargo cannot build one. When it is
    # unset, this is a local run and the broker is built here as before —
    # except in archive mode, where building it is exactly what must not happen.
    broker="${BISCUIT_HARNESS_BROKER_BIN:-}"
    if [[ -z "${broker}" ]] && (( archive_mode == 0 )); then
        cargo build -p biscuit-test-harness --bin biscuit-harness-broker --quiet
        # Read cargo metadata via a tempfile to avoid the `bash -c`
        # subprocess hang documented in `_sanity` above. The portable
        # `mktemp` template form works on macOS BSD, Linux GNU, and
        # Windows git-bash; `-t` has incompatible semantics across them.
        meta_file="$(mktemp "${TMPDIR:-/tmp}/biscuit-test-l2-meta.XXXXXX")"
        cargo metadata --no-deps --format-version 1 > "${meta_file}"
        target_dir="$(jq -r '.target_directory' "${meta_file}")"
        rm -f "${meta_file}"
        broker="${target_dir}/debug/biscuit-harness-broker${bin_suffix}"
    fi
    if [[ -z "${broker}" || ! -x "${broker}" ]]; then
        echo -e "{{ yellow }}warning: ${broker:-no broker} not executable; falling back to per-test spawning{{ reset }}"
        exit_code=0
        run_l2 "${nextest_bin[@]}" run ${selection[@]+"${selection[@]}"} -E "$filter" --no-tests=pass ${run_args[@]+"${run_args[@]}"} || exit_code=$?
        finish "${exit_code}"
    fi

    # Registry of (backend id) pairs the trap uses to tear down panes
    # the broker spawned. The trap consumes the registry on EXIT.
    SHARED_REGISTRY="$(mktemp "${TMPDIR:-/tmp}/biscuit-shared.XXXXXX")"
    trap 'while read -r b i; do "${broker}" kill "${b}" "${i}" >/dev/null 2>&1 || true; done < "${SHARED_REGISTRY}"; rm -f "${SHARED_REGISTRY}"' EXIT

    # The broker exits as soon as it has spawned, so a pane tagged with its
    # pid looks leaked to every reaper on the host — including a concurrent
    # L2 run in another worktree, whose first spawn killed this run's shared
    # tmux session under the tests (2026-09-18). Tag the panes with this
    # recipe's pid instead: it lives exactly as long as the run does.
    export BISCUIT_HARNESS_OWNER_PID="$$"

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

    exit_code=0
    run_l2 "${nextest_bin[@]}" run ${selection[@]+"${selection[@]}"} -E "$filter" --no-tests=pass -j 1 ${run_args[@]+"${run_args[@]}"} || exit_code=$?
    finish "${exit_code}"

# Level 3 tier: OS keyboard/mouse injection tests. Always opt-in via
# RUN_LEVEL3=1; the recipe sets the variable so the gate trips cleanly.
#
# This tier TAKES OVER THE DESKTOP: it raises a GUI terminal to frontmost and
# synthesizes real keyboard events, which land in whatever window owns focus.
# Anything the operator types during the run interleaves with the injection.
# The recipe therefore refuses to start unattended — an agent, hook, or CI job
# that shells out to it gets a hard error instead of silently stealing focus
# from whoever is at the keyboard. Set BISCUIT_L3_TAKE_FOCUS=1 to authorize a
# non-interactive run (a human deciding the machine is free), or answer the
# prompt when running from a terminal.
_test_l3 pkg *args="": _storage_preflight
    #!/usr/bin/env bash
    set -euo pipefail
    if [ "${BISCUIT_L3_TAKE_FOCUS:-}" != "1" ]; then
        if [ ! -t 0 ]; then
            echo -e "⛔ {{ bold }}just test-l3 refuses to run unattended{{ reset }}" >&2
            echo >&2
            echo "   Level-3 tests raise a GUI terminal over every other window and" >&2
            echo "   inject real OS keystrokes into whatever holds focus. Running this" >&2
            echo "   without a human present hijacks an active desktop session." >&2
            echo >&2
            echo -e "   Run it from a terminal, or set {{ bold }}BISCUIT_L3_TAKE_FOCUS=1{{ reset }} to" >&2
            echo "   confirm the machine is free." >&2
            exit 1
        fi
        echo
        echo -e "⚠️  {{ bold }}Level-3 tests will take over this desktop{{ reset }}"
        echo -e "   {{ dim }}A terminal window is raised to frontmost and real keystrokes are{{ reset }}"
        echo -e "   {{ dim }}injected into it. Do not type until the run finishes.{{ reset }}"
        echo
        read -r -p "   Proceed? [y/N] " reply
        case "${reply}" in
            [yY]) ;;
            *) echo "   Aborted."; exit 1 ;;
        esac
    fi
    echo
    echo -e "🎹 Level-3 tests for {{ bold }}{{ pkg }}{{ reset }} {{ dim }}(RUN_LEVEL3=1){{ reset }}"
    echo
    # Run one test at a time (`-j 1`). These tests inject real OS keyboard/pointer
    # events and so compete for the single global frontmost window + pointer;
    # `#[serial]` cannot serialize them because nextest runs each test in a
    # separate process — mirror the browser tier and serialize the tier itself.
    SECONDS=0
    just _stage_junit_reset
    exit_code=0
    read -r -a nextest_bin <<< "${BISCUIT_NEXTEST_BIN:-cargo nextest}"
    RUN_LEVEL3=1 "${nextest_bin[@]}" run -p {{ pkg }} -j 1 -E "$(just _tier_filter L3)" --no-tests=pass {{ args }} || exit_code=$?
    just _stage_junit L3 "{{ pkg }}" "${exit_code}" "${SECONDS}"
    exit "${exit_code}"

# Browser tier: headless browser tests. Skip-clean if Chrome absent unless
# BISCUIT_BROWSER_REQUIRED=1 is set in the environment.
#
# Accepts the same archive-mode inputs as `_test` and `_test_l2`; see `_test`'s
# comment for why `-p` and the Cargo build flags have to move.
_test_browser pkg *args="": _storage_preflight
    #!/usr/bin/env bash
    set -euo pipefail
    echo
    echo -e "🌐 Browser tests for {{ bold }}{{ pkg }}{{ reset }}"
    echo
    # Run one test at a time (`-j 1`). Each browser test spawns its own headless
    # Chrome; `#[serial(browser)]` cannot serialize them because nextest runs
    # each test in a separate process. Concurrent Chromes overwhelm teardown and
    # trip nextest's leak detector — mirror the L2 tier and serialize the tier.
    SECONDS=0
    just _stage_junit_reset
    exit_code=0
    read -r -a nextest_bin <<< "${BISCUIT_NEXTEST_BIN:-cargo nextest}"

    forwarded=({{ args }})
    archive_mode=0
    archive_file=""
    want_archive=0
    for arg in ${forwarded[@]+"${forwarded[@]}"}; do
        if (( want_archive )); then archive_file="${arg}"; want_archive=0; continue; fi
        case "${arg}" in
            --archive-file) archive_mode=1; want_archive=1 ;;
            --archive-file=*) archive_mode=1; archive_file="${arg#--archive-file=}" ;;
        esac
    done

    filter="$(just _tier_filter browser)"
    selection=(-p '{{ pkg }}')
    run_args=(${forwarded[@]+"${forwarded[@]}"})
    if (( archive_mode == 1 )); then
        if ! "${nextest_bin[@]}" --version >/dev/null 2>&1; then
            echo -e "❌ {{ red }}--archive-file requires cargo-nextest{{ reset }}" >&2
            echo "   '${nextest_bin[*]} --version' failed, and there is no fallback that can" >&2
            echo "   consume an archive — every alternative recompiles." >&2
            exit 1
        fi
        just _archive_file_check "${archive_file}"
        selection=()
        filter="package({{ pkg }}) & (${filter})"
        run_args=()
        while IFS= read -r kept_arg; do
            run_args+=("${kept_arg}")
        done < <(just _archive_drop_build_flags ${forwarded[@]+"${forwarded[@]}"})
    fi

    "${nextest_bin[@]}" run ${selection[@]+"${selection[@]}"} -j 1 -E "$filter" --no-tests=pass ${run_args[@]+"${run_args[@]}"} || exit_code=$?
    just _stage_junit browser "{{ pkg }}" "${exit_code}" "${SECONDS}"
    exit "${exit_code}"

# Real-resource tier: external services/devices. Selecting the set is harmless;
# tests themselves are responsible for skipping when the resource is absent.
_test_real pkg *args="": _storage_preflight
    #!/usr/bin/env bash
    set -euo pipefail
    echo
    echo -e "🔌 Real-resource tests for {{ bold }}{{ pkg }}{{ reset }}"
    echo
    SECONDS=0
    just _stage_junit_reset
    exit_code=0
    read -r -a nextest_bin <<< "${BISCUIT_NEXTEST_BIN:-cargo nextest}"
    test_threads="${NEXTEST_TEST_THREADS:-$(just _test_threads)}"
    export NEXTEST_TEST_THREADS="$test_threads"
    "${nextest_bin[@]}" run -p {{ pkg }} -E "$(just _tier_filter real)" --no-tests=pass {{ args }} || exit_code=$?
    just _stage_junit real "{{ pkg }}" "${exit_code}" "${SECONDS}"
    exit "${exit_code}"

# Doctests. Nextest does not run doctests, so this recipe shells out to
# `cargo test --doc`. Skips packages that have no lib target (bin-only
# crates have no doctests).
_doctest pkg *args="": _storage_preflight
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
_bench pkg *args="": _storage_preflight
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
_coverage pkg *args="": _storage_preflight
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
_fuzz fuzz_dir target *args="": _storage_preflight
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

#   just status                 full overview of this package area
#   just status <subject>       fuzzy-match one feature/fix and list it
#   just status --recent        only listings touched in the last 7 days
#   just status --recent-all    the same, every package area, grouped
#
# The summary sits last because that is the only line `just --list` shows.
# feature and fix status; --recent / --recent-all narrow to the last 7 days,
# skipping listings dated older than that whose spec says `implemented: true`
status subject="":
    #!/usr/bin/env bash
    set -euo pipefail

    case "{{ subject }}" in
        --recent)     just _status_recent area; exit 0 ;;
        --recent-all) just _status_recent all;  exit 0 ;;
    esac

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
        eza "${scheduled[0]}" -lga --icons=always --hyperlink
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
    eza "${matched}" -lga --icons=always --hyperlink

# Active feature/fix listings holding a file touched inside the recent window.
# `scope` is `area` for the current package area, `all` for every package area
# in the repo, each under its own heading and silent when it has nothing.
#
# Listings dated more than the window ago whose spec frontmatter says
# `implemented: true` are skipped: another spec's implementation touching
# their files is reporting noise, not activity.
#
# "Touched" comes from git, not filesystem mtimes: `git worktree add` and
# branch checkouts rewrite mtimes wholesale, which made every listing in a
# fresh checkout report as recent. A file counts when a commit on HEAD touched
# it inside the window, or it is dirty/untracked with an mtime inside the
# window.
_status_recent scope="area":
    #!/usr/bin/env python3
    import datetime
    import os
    import re
    import subprocess
    import sys
    import time

    RECENT_DAYS = 7
    SCOPE = "{{ scope }}"

    BOLD = "\033[1m"
    DIM = "\033[2m"
    ITALIC = "\033[3m"
    UNDERLINE = "\033[4m"
    RESET = "\033[0m"
    BLUE = "\033[34m"
    CYAN = "\033[36m"

    now = time.time()
    today = datetime.date.today()
    cutoff = now - RECENT_DAYS * 86400
    window = "last " + str(RECENT_DAYS) + " days"

    root = sniff("root")
    if not root:
        done = subprocess.run(
            ["git", "rev-parse", "--show-toplevel"], capture_output=True, text=True
        )
        root = done.stdout.strip() if done.returncode == 0 else os.getcwd()

    def sniff(*args):
        try:
            done = subprocess.run(
                ["sniff", "repo", *args], capture_output=True, text=True
            )
        except OSError:
            return ""
        return done.stdout.strip() if done.returncode == 0 else ""

    def git(*args):
        try:
            done = subprocess.run(
                ["git", "-C", root, *args], capture_output=True, text=True
            )
        except OSError:
            return ""
        return done.stdout if done.returncode == 0 else ""

    def age(mtime):
        secs = max(0, int(now - mtime))
        if secs < 3600:
            return str(max(1, secs // 60)) + "m"
        if secs < 86400:
            return str(secs // 3600) + "h"
        return str(secs // 86400) + "d"

    def hyperlink(path, label):
        target = "file://" + os.path.abspath(path)
        return "\033]8;;" + target + "\033\\" + label + "\033]8;;\033\\"

    LISTING_DATE = re.compile(r"^(\d{4}-\d{2}-\d{2})-")

    def listing_date(name):
        match = LISTING_DATE.match(name)
        if not match:
            return None
        try:
            return datetime.date.fromisoformat(match.group(1))
        except ValueError:
            return None

    def implemented(listing_dir):
        spec = os.path.join(listing_dir, "spec.md")
        try:
            with open(spec, encoding="utf-8", errors="replace") as handle:
                if handle.readline().strip() != "---":
                    return False
                for line in handle:
                    stripped = line.strip()
                    if stripped == "---":
                        return False
                    if stripped.startswith("implemented:"):
                        return stripped.split(":", 1)[1].strip().lower() == "true"
        except OSError:
            return False
        return False

    # Every cell is padded on its own so `·` and `ago` line up down the whole
    # report, not just within one heading: (name, count, unit, age).
    def cells(row):
        return (row[1], str(row[3]), "file" if row[3] == 1 else "files", age(row[4]))

    def scan(area_dir):
        rows = []
        for kind in ("features", "fixes"):
            base = os.path.join(area_dir, kind)
            if not os.path.isdir(base):
                continue
            for entry in os.scandir(base):
                if not entry.is_dir() or entry.name.startswith(("_", ".")):
                    continue
                # Excluded only when BOTH hold: the dated name is older than
                # the window, and the spec says `implemented: true`. Either
                # alone keeps the listing in the report.
                started = listing_date(entry.name)
                if (
                    started is not None
                    and (today - started).days > RECENT_DAYS
                    and implemented(entry.path)
                ):
                    continue
                count = 0
                newest = 0.0
                newest_file = ""
                for root, dirs, files in os.walk(entry.path):
                    dirs[:] = [d for d in dirs if not d.startswith(".")]
                    for name in files:
                        if name.startswith("."):
                            continue
                        path = os.path.join(root, name)
                        try:
                            mtime = os.stat(path).st_mtime
                        except OSError:
                            continue
                        if mtime < cutoff:
                            continue
                        count += 1
                        if mtime > newest:
                            newest = mtime
                            newest_file = os.path.relpath(path, entry.path)
                if count:
                    rows.append(
                        (kind, entry.name, entry.path, count, newest, newest_file)
                    )
        rows.sort(key=lambda row: row[4], reverse=True)
        return rows

    def widths(rows):
        return [max(len(cells(row)[i]) for row in rows) for i in range(4)]

    def block(rows, widths, indent, heading_style, spaced):
        name_w, count_w, unit_w, age_w = widths
        for kind, label in (("features", "Features"), ("fixes", "Fixes")):
            group = [row for row in rows if row[0] == kind]
            if not group:
                continue
            print(indent + heading_style + label + RESET)
            if spaced:
                print()
            for row in group:
                name, count, unit, since = cells(row)
                meta = (
                    count.rjust(count_w) + " " + unit.ljust(unit_w)
                    + " · " + since.rjust(age_w) + " ago"
                )
                print(
                    indent + "  "
                    + hyperlink(row[2], name) + " " * (name_w - len(name))
                    + "   " + DIM + meta + "   " + row[5] + RESET
                )
            if spaced:
                print()

    def title(subject):
        print()
        print(
            "  📅 " + BOLD + "Recent" + RESET
            + " " + DIM + "· " + window + " ·" + RESET
            + " " + BOLD + subject + RESET
        )
        print()

    def nothing_recent(indent):
        print(indent + DIM + ITALIC + "nothing touched in the " + window + RESET)
        print()

    if SCOPE == "area":
        rows = scan(os.getcwd())
        title(sniff("area") or os.path.basename(os.getcwd()))
        if rows:
            block(rows, widths(rows), "  ", BOLD + BLUE + UNDERLINE, True)
        else:
            nothing_recent("  ")
        sys.exit(0)

    if SCOPE != "all":
        print("_status_recent: scope must be `area` or `all`", file=sys.stderr)
        sys.exit(2)

    root = sniff("root") or os.getcwd()
    areas = [a for a in sniff("package-areas", "--list").splitlines() if a.strip()]
    if not areas:
        print("_status_recent: `sniff repo package-areas` found none", file=sys.stderr)
        sys.exit(1)

    # Scanned in full before anything prints: the columns are sized across
    # every area so one heading's long name does not shift its neighbours.
    found = []
    for area in areas:
        rows = scan(root if area == "root" else os.path.join(root, area))
        if rows:
            found.append((area, rows))

    title("all package areas")
    if not found:
        nothing_recent("  ")
        sys.exit(0)

    shared = widths([row for _, rows in found for row in rows])
    for index, pair in enumerate(found):
        if index:
            print()
        print("  " + BOLD + CYAN + "▌ " + pair[0] + RESET)
        block(pair[1], shared, "    ", BOLD + BLUE, False)

    listings = sum(len(rows) for _, rows in found)
    noun = " listing across " if listings == 1 else " listings across "
    areas_noun = " package area" if len(found) == 1 else " package areas"
    print()
    print("  " + DIM + str(listings) + noun + str(len(found)) + areas_noun + RESET)
    print()

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

# ---------------------------------------------------------------------------
# CI comparison
# ---------------------------------------------------------------------------

# `main` carries known-red cells, so "is the run green" is not a question this
# repo can ask. The only decidable one is whether a branch fails something its
# base does not, which needs both runs' results side by side. Doing that by
# hand — locate two runs, download their artifacts, diff two failure sets — is
# the manual triage this recipe exists to delete.
#
# `ref` is a PR number or a branch name; it defaults to the current branch.
#
# A run's results are its per-area `ci-results-<slug>` slices, one per area
# rollup; there is no whole-run document (the global verdict job that wrote
# one is gone). `ci-rollup compare` folds each side's slices by concatenation
# and applies no policy.
#
# Runs that scheduled no package area upload no slice and are skipped on both
# sides. That case is not rare: any change confined to `.github/` or docs
# produces a fully green run with nothing in it, and comparing against one
# would report every cell as not comparable.
#
# Exit 2 means the branch regressed. Exit 0 means it did not — NOT that CI is
# green.
#
# Compare a branch's CI failures against main's (exit 2 = it regressed)
ci-diff ref="":
    #!/usr/bin/env bash
    set -euo pipefail

    target="{{ ref }}"
    if [[ -z "${target}" ]]; then
        target="$(git rev-parse --abbrev-ref HEAD)"
    fi

    # A PR number resolves to its head branch; anything else is one already.
    if [[ "${target}" =~ ^[0-9]+$ ]]; then
        branch="$(gh pr view "${target}" --json headRefName --jq .headRefName)"
        echo -e "PR {{ bold }}#${target}{{ reset }} → branch {{ bold }}${branch}{{ reset }}"
    else
        branch="${target}"
    fi

    if [[ "${branch}" == "main" ]]; then
        echo -e "{{ red }}Nothing to compare: {{ bold }}main{{ reset }}{{ red }} is the base.{{ reset }}" >&2
        exit 1
    fi

    # Newest completed run for a branch that actually uploaded a slice.
    #
    # The artifact names are captured before matching rather than piped into
    # `grep -q`. Under `pipefail` that pipeline reports the producer's SIGPIPE
    # (141) whenever `grep` exits on an early match before `gh` has finished
    # writing — so a match intermittently reads as a miss, depending on whether
    # the run's artifact list happens to fit the pipe buffer.
    latest_run_with_results() {
        local for_branch="$1" id names
        for id in $(gh run list --workflow ci.yml --branch "${for_branch}" \
                        --status completed --limit 15 --json databaseId \
                        --jq '.[].databaseId'); do
            if ! names="$(gh api "repos/{owner}/{repo}/actions/runs/${id}/artifacts" \
                              --jq '.artifacts[].name' 2>/dev/null)"; then
                continue
            fi
            if grep -q '^ci-results-' <<<"${names}"; then
                echo "${id}"
                return 0
            fi
        done
        return 1
    }

    echo -e "Locating runs…"
    base_run="$(latest_run_with_results main)" || {
        echo -e "{{ red }}No completed {{ bold }}main{{ reset }}{{ red }} run has a ci-results-* slice.{{ reset }}" >&2
        exit 1
    }
    head_run="$(latest_run_with_results "${branch}")" || {
        echo -e "{{ red }}No completed run on {{ bold }}${branch}{{ reset }}{{ red }} has a ci-results-* slice.{{ reset }}" >&2
        echo -e "{{ dim }}A run that scheduled no package area uploads none.{{ reset }}" >&2
        exit 1
    }

    workdir="$(mktemp -d)"
    trap 'rm -rf "${workdir}"' EXIT

    # Each slice unpacks into its own artifact-named directory.
    gh run download "${base_run}" -p 'ci-results-*' -D "${workdir}/base" >/dev/null
    gh run download "${head_run}" -p 'ci-results-*' -D "${workdir}/head" >/dev/null

    compare_args=()
    for slice in "${workdir}"/base/ci-results-*/ci-results-*.json; do
        compare_args+=(--base "${slice}")
    done
    for slice in "${workdir}"/head/ci-results-*/ci-results-*.json; do
        compare_args+=(--head "${slice}")
    done

    # Same flags CI builds it with; --no-default-features keeps this off gix,
    # duckdb, and the terminal renderer.
    cargo build --release --quiet -p repo-deps \
        --no-default-features --bin ci-rollup

    echo -e "base {{ bold }}main{{ reset }} run ${base_run} $(gh run view "${base_run}" --json createdAt --jq .createdAt)"
    echo -e "head {{ bold }}${branch}{{ reset }} run ${head_run} $(gh run view "${head_run}" --json createdAt --jq .createdAt)"

    status=0
    ./target/release/ci-rollup compare "${compare_args[@]}" || status=$?

    if [[ "${status}" -eq 2 ]]; then
        echo -e "{{ red }}❌ {{ bold }}${branch}{{ reset }}{{ red }} fails something main does not.{{ reset }}"
    elif [[ "${status}" -eq 0 ]]; then
        echo -e "{{ green }}✅ {{ bold }}${branch}{{ reset }}{{ green }} adds no failure main does not already have.{{ reset }}"
    fi
    exit "${status}"
