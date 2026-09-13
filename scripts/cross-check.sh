#!/usr/bin/env bash
# Cross-OS pre-push smoke: run one package's L1 suite on the standing
# build-host clones (real Linux, native Windows, WSL2, macOS) against the
# LOCAL tree — committed-but-unpushed and uncommitted changes included.
#
# Hosts come from the environment, never from the script: BUILD_LINUX,
# BUILD_WIN, BUILD_WSL, and BUILD_MACOS name the SSH destinations (an alias
# from ~/.ssh/config or user@host). Setting a variable is how a machine
# declares that host is available; an unset variable means "no such host
# here".
#
# `--os all` (the default) means every declared OS EXCEPT the one this
# script is running on: the local suite already covers the local OS. An
# explicit `--os <name>` runs that one OS even when it matches the local one.
#
# Sync model: the remote standing clone is reset to the nearest commit the
# remote can fetch (origin/<current-branch> if pushed, else origin/main),
# then the local tree's difference from that commit — tracked and untracked —
# is shipped as one patch and applied. No local commit or push is required.
#
# Concurrency: the hosts are shared. Each remote run holds a per-host lock
# (`ci-verification/.cross-check.lock`, a directory created atomically) for
# the whole reset/apply/test sequence, so overlapping runs from other people
# or agents queue instead of clobbering the clone. A waiter prints the lock
# owner and gives up after 30 minutes with exit 75. A lock left behind by a
# dead run is reported, never removed by this script.
#
# The WSL host runs the suite the way CI's wsl2-ubuntu leg does: as a nextest
# ARCHIVE, executed with the builder's target directory hidden, so anything
# that bakes a build-host path at compile time (`env!("CARGO_BIN_EXE_*")`)
# fails here exactly as it fails in CI. Feature flags (--features,
# --all-features, --no-default-features) go to the archive build; every other
# extra arg goes to the run.
#
# The remote run never reads the developer's `~/.config`. On the WSL guest
# that directory is a CIFS mount of the Synology NAS, and when the NAS is down
# `git` dies on its global config and cargo's package-file listing (gitoxide,
# honoring the global excludes at `$XDG_CONFIG_HOME/git/ignore`) dies with
# "Host is down" — before a single test runs (2026-09-10). The Unix preamble
# therefore sets GIT_CONFIG_GLOBAL=/dev/null (fetch/reset/clean/checkout/apply
# need no identity) and points XDG_CONFIG_HOME at an empty local directory.
#
# A WSL run whose tested tree IS the outgoing head's tree, on a clean remote
# worktree, with no test filter, becomes a published `wsl2-ubuntu` validation
# receipt — so "prior WSL evidence" is an ordinary receipt CI can reuse rather
# than a log someone remembers. Every other run publishes nothing and prints
# the reason: this script ships the developer's LOCAL tree, uncommitted work
# included, so most of its runs test something no head names.
#
# BISCUIT_TEST_REQUIRED_BACKENDS is forwarded from the caller's environment to
# every remote run when set. Without it a Level 2 test whose backend is absent
# on the host skips, and nextest prints PASS in ~0.02 s — indistinguishable
# from evidence unless you read the duration. Name the backend you expect
# (e.g. `BISCUIT_TEST_REQUIRED_BACKENDS=tmux just cross-check --os wsl …
# --features terminal-tests level2_`) and the skip becomes a failure.
#
# The standing clones and their target dirs persist between runs so compile
# caches are warm:
#   $BUILD_LINUX   ~/ci-verification/rusty-biscuit
#   $BUILD_WIN     W:\ci-verification\rusty-biscuit
#   $BUILD_WSL     ~/ci-verification/rusty-biscuit
#   $BUILD_MACOS   ~/ci-verification/rusty-biscuit
#
# Usage:
#   scripts/cross-check.sh [--os linux|windows|wsl|macos|all] <package> [nextest args...]
set -euo pipefail

LOCK_WAIT_SECS=1800
UNIX_DIR="ci-verification/rusty-biscuit" # relative to remote $HOME (linux, wsl, macos)
WIN_BASE='W:\ci-verification'
WIN_DIR="${WIN_BASE}\\rusty-biscuit"
SSH=(ssh -o BatchMode=yes -o ConnectTimeout=15)
SCP=(scp -q -o BatchMode=yes -o ConnectTimeout=15)

ORDER=(linux windows wsl macos)
declare -A HOST=([linux]="${BUILD_LINUX:-}" [windows]="${BUILD_WIN:-}" [wsl]="${BUILD_WSL:-}" [macos]="${BUILD_MACOS:-}")
declare -A VAR=([linux]=BUILD_LINUX [windows]=BUILD_WIN [wsl]=BUILD_WSL [macos]=BUILD_MACOS)

local_os() {
    case "$(uname -s)" in
        Darwin) echo macos ;;
        Linux) if grep -qi microsoft /proc/version 2> /dev/null; then echo wsl; else echo linux; fi ;;
        MINGW* | MSYS* | CYGWIN*) echo windows ;;
        *) echo unknown ;;
    esac
}

os_sel="all"
package=""
extra_args=()
while (($# > 0)); do
    case "$1" in
        --os) shift; os_sel="${1:?--os needs linux|windows|wsl|macos|all}" ;;
        --os=*) os_sel="${1#--os=}" ;;
        --host | --host=*)
            echo "cross-check: --host was renamed to --os (linux|windows|wsl|macos|all)" >&2
            exit 2
            ;;
        -h | --help)
            awk 'NR > 1 && !/^#/ { exit } NR > 1 { sub(/^# ?/, ""); print }' "$0"
            exit 0
            ;;
        -*)
            if [[ -z "${package}" ]]; then
                echo "cross-check: unknown flag $1" >&2
                exit 2
            fi
            extra_args+=("$1")
            ;;
        *)
            if [[ -z "${package}" ]]; then package="$1"; else extra_args+=("$1"); fi
            ;;
    esac
    shift
done
if [[ -z "${package}" ]]; then
    echo "cross-check: a package name is required (e.g. 'just cross-check biscuit-file')" >&2
    exit 2
fi
case "${os_sel}" in linux | windows | wsl | macos | all) ;; *)
    echo "cross-check: --os must be linux, windows, wsl, macos, or all" >&2
    exit 2
    ;;
esac

# Resolve the requested OSes against what this machine declares available.
# An explicit --os for an undeclared host is an error; `all` narrows to the
# declared set minus the local OS and only errors when that set is empty.
here="$(local_os)"
declare -A want=()
skipped_local=""
if [[ "${os_sel}" == "all" ]]; then
    for os in "${ORDER[@]}"; do
        [[ -n "${HOST[${os}]}" ]] || continue
        if [[ "${os}" == "${here}" ]]; then skipped_local="${os}"; continue; fi
        want[${os}]=1
    done
    if ((${#want[@]} == 0)); then
        echo "cross-check: no build host for an OS other than this one (${here}) is declared (set BUILD_LINUX, BUILD_WIN, BUILD_WSL, and/or BUILD_MACOS)" >&2
        exit 2
    fi
else
    if [[ -z "${HOST[${os_sel}]}" ]]; then
        echo "cross-check: --os ${os_sel} requested but ${VAR[${os_sel}]} is not set" >&2
        exit 2
    fi
    want[${os_sel}]=1
fi

# Archive mode splits the extra args: cargo build flags belong to `nextest
# archive`, everything else (filters, --no-fail-fast, ...) to `nextest run`.
archive_args=()
run_args=()
take_next=0
for arg in "${extra_args[@]}"; do
    if ((take_next)); then archive_args+=("${arg}"); take_next=0; continue; fi
    case "${arg}" in
        --features) archive_args+=("${arg}"); take_next=1 ;;
        --features=* | --all-features | --no-default-features) archive_args+=("${arg}") ;;
        *) run_args+=("${arg}") ;;
    esac
done

# Forwarded verbatim into each remote script, so it is validated here rather
# than quoted for two shells: comma-separated backend identifiers only.
required_backends="${BISCUIT_TEST_REQUIRED_BACKENDS:-}"
if [[ -n "${required_backends}" && ! "${required_backends}" =~ ^[A-Za-z0-9_,-]+$ ]]; then
    echo "cross-check: BISCUIT_TEST_REQUIRED_BACKENDS must be comma-separated backend names, got: ${required_backends}" >&2
    exit 2
fi

cd "$(git rev-parse --show-toplevel)"
origin_url="$(git remote get-url origin)"
branch="$(git rev-parse --abbrev-ref HEAD)"
if git rev-parse --quiet --verify "origin/${branch}" > /dev/null; then
    base_ref="origin/${branch}"
else
    base_ref="origin/main"
fi
base_sha="$(git rev-parse "${base_ref}")"

# Per-run identity: names the shipped files on the host (unique, so two runs
# can stage concurrently before one of them wins the lock) and labels the lock.
short_host="$(hostname)"; short_host="${short_host%%.*}"
run_id="${short_host}-$$-$(date +%s)"
owner="${USER:-unknown}@${short_host} ${branch} ${base_sha:0:9} started $(date -u +%Y-%m-%dT%H:%M:%SZ)"

# One patch carrying everything local relative to the base: tracked changes,
# then each untracked (non-ignored) file as a new-file hunk.
work_dir="$(mktemp -d "${TMPDIR:-/tmp}/cross-check.XXXXXX")"
trap 'rm -rf "${work_dir}"' EXIT
patch_file="${work_dir}/cross-check-${run_id}.patch"
git diff --binary "${base_sha}" > "${patch_file}"
while IFS= read -r untracked; do
    [[ -n "${untracked}" ]] || continue
    git diff --no-index --binary /dev/null "${untracked}" >> "${patch_file}" || true
done < <(git ls-files --others --exclude-standard)

patch_lines="$(wc -l < "${patch_file}" | tr -d ' ')"
echo "cross-check: ${package} @ ${base_ref} (${base_sha:0:9}) + ${patch_lines} patch line(s)"
resolved=""
for os in "${ORDER[@]}"; do
    [[ -n "${want[${os}]:-}" ]] && resolved+="${os}(${HOST[${os}]}) "
done
[[ -n "${skipped_local}" ]] && resolved+=" (${skipped_local} skipped: local OS)"
echo "os: ${resolved}  extra nextest args: ${extra_args[*]:-<none>}"

# Remote scripts are generated locally, shipped next to the patch, and run in
# one SSH session each so the lock covers the whole sequence. Remote-side
# variables are escaped (\$); everything else is substituted here.

# The lock and clone sync shared by linux, wsl, and macos. `cleanup` is the
# single EXIT trap for the whole remote run; mode-specific steps append to
# `extra_cleanup` rather than installing their own trap.
unix_prelude() {
    cat <<EOF
set -euo pipefail
base="\$HOME/ci-verification"
repo="\$HOME/${UNIX_DIR}"
# Never touch the developer's ~/.config (a network mount on the WSL guest).
export GIT_CONFIG_GLOBAL=/dev/null
export XDG_CONFIG_HOME="\$base/.xdg-empty"
mkdir -p "\$XDG_CONFIG_HOME"
${required_backends:+export BISCUIT_TEST_REQUIRED_BACKENDS='${required_backends}'}
lock="\$base/.cross-check.lock"
patch="\$base/cross-check-${run_id}.patch"
self="\$base/cross-check-${run_id}.sh"
# Until the lock is won only the shipped files are ours to remove; an
# interrupted waiter must not touch the holder's lock.
trap 'rm -f "\$patch" "\$self"' EXIT
waited=0
until mkdir "\$lock" 2> /dev/null; do
    if [ "\$waited" -eq 0 ]; then
        echo "cross-check: waiting for lock on \$(hostname -s); held by: \$(cat "\$lock/owner" 2> /dev/null || echo unknown)"
    fi
    if [ "\$waited" -ge ${LOCK_WAIT_SECS} ]; then
        echo "cross-check: gave up after ${LOCK_WAIT_SECS}s; \$lock still held by: \$(cat "\$lock/owner" 2> /dev/null || echo unknown)" >&2
        echo "cross-check: if that run is dead its owner removes the lock by hand; never remove someone else's" >&2
        exit 75
    fi
    sleep 5
    waited=\$((waited + 5))
done
echo "${owner}" > "\$lock/owner"
extra_cleanup() { :; }
cleanup() {
    extra_cleanup
    rm -rf "\$lock" "\$patch" "\$self"
}
trap cleanup EXIT
if [ ! -d "\$repo/.git" ]; then
    git clone --quiet '${origin_url}' "\$repo"
fi
cd "\$repo"
git fetch --quiet origin
git reset --quiet --hard
git clean -fdq
git checkout --quiet --detach '${base_sha}'
if [ -s "\$patch" ]; then
    git apply "\$patch"
fi
EOF
}

unix_run_native() {
    cat <<EOF
cargo nextest run -p '${package}' --no-fail-fast ${extra_args[*]@Q}
EOF
}

unix_run_archive() {
    cat <<EOF
archive="\$base/${package}.tar.zst"
extract="\$(mktemp -d "\$base/extract.XXXXXX")"
report="\$base/cross-check-${run_id}-report.xml"
# Hide the builder's target dir so a compile-time build-host path cannot
# resolve; restore it on every exit so the cache survives.
extra_cleanup() {
    cd "\$repo" 2> /dev/null && if [ -d target.hold ]; then mv target.hold target; fi
    rm -rf "\$extract" "\$archive"
}
# What the remote actually tested. Printed rather than assumed: a receipt for
# this run is only publishable when this tree IS the outgoing head's tree and
# the worktree was clean (spec section 3.8).
echo "cross-check-tree: \$(git rev-parse HEAD^{tree})"
echo "cross-check-dirty: \$(git status --porcelain | wc -l | tr -d ' ')"
export NEXTEST_PROFILE="\${NEXTEST_PROFILE:-ci}"
started=\$(date +%s)
cargo nextest archive -p '${package}' --archive-file "\$archive" ${archive_args[*]@Q}
mv target target.hold
set +e
cargo nextest run --archive-file "\$archive" \\
    --workspace-remap "\$repo" --extract-to "\$extract" \\
    --no-fail-fast ${run_args[*]@Q}
run_code=\$?
set -e
echo "cross-check-exit: \$run_code"
echo "cross-check-duration: \$(( \$(date +%s) - started ))"
# The archive run's target directory is the extraction root, so that is where
# the JUnit report lands. The builder's own target dir is checked too, because
# a future nextest could place it there. If neither exists the run publishes
# nothing and says so locally — it never invents a measurement.
for candidate in "\$extract/target/nextest/\$NEXTEST_PROFILE/test-results.xml" \\
    "\$repo/target.hold/nextest/\$NEXTEST_PROFILE/test-results.xml"; do
    if [ -f "\$candidate" ]; then cp "\$candidate" "\$report"; break; fi
done
exit \$run_code
EOF
}

run_unix() {
    local os="$1" host="${HOST[$1]}"
    local script="${work_dir}/cross-check-${run_id}.sh"
    {
        unix_prelude
        if [[ "${os}" == "wsl" ]]; then unix_run_archive; else unix_run_native; fi
    } > "${script}"
    local -a ship=("${script}")
    [[ "${patch_lines}" != "0" ]] && ship+=("${patch_file}")
    "${SSH[@]}" "${host}" 'mkdir -p "$HOME/ci-verification"'
    "${SCP[@]}" "${ship[@]}" "${host}:ci-verification/"
    # A login shell is what puts cargo on PATH on every Unix host. The WSL leg's
    # output is teed so its `cross-check-*:` markers can be read afterwards
    # without hiding the run from the terminal.
    if [[ "${os}" == "wsl" ]]; then
        "${SSH[@]}" "${host}" "bash -l \"\$HOME/ci-verification/cross-check-${run_id}.sh\"" \
            2>&1 | tee "${work_dir}/wsl.log"
        return "${PIPESTATUS[0]}"
    fi
    "${SSH[@]}" "${host}" "bash -l \"\$HOME/ci-verification/cross-check-${run_id}.sh\""
}

# Turn a qualifying WSL archive run into a `wsl2-ubuntu` validation receipt.
#
# "Qualifying" is `local_evidence.py cross-check`'s decision, not this script's:
# exact outgoing tree, clean remote worktree, unfiltered run, and a real report.
# Anything else prints the reason and publishes nothing (spec section 3.8).
publish_wsl_receipt() {
    local log="${work_dir}/wsl.log"
    [[ -f "${log}" ]] || return 0
    local tested_tree dirty exit_code duration
    tested_tree="$(sed -n 's/^cross-check-tree: //p' "${log}" | tail -n1)"
    dirty="$(sed -n 's/^cross-check-dirty: //p' "${log}" | tail -n1)"
    exit_code="$(sed -n 's/^cross-check-exit: //p' "${log}" | tail -n1)"
    duration="$(sed -n 's/^cross-check-duration: //p' "${log}" | tail -n1)"

    local report="${work_dir}/wsl-report.xml"
    if ! "${SCP[@]}" "${HOST[wsl]}:ci-verification/cross-check-${run_id}-report.xml" \
        "${report}" 2> /dev/null; then
        echo "cross-check: publishing no receipt — the remote run produced no JUnit report"
        return 0
    fi

    local plan="${work_dir}/plan.json" receipt="${work_dir}/receipt.json"
    local head_sha merge_base
    head_sha="$(git rev-parse HEAD)"
    if ! merge_base="$(git merge-base origin/main "${head_sha}" 2> /dev/null)"; then
        echo "cross-check: publishing no receipt — no merge base with origin/main"
        return 0
    fi
    local -a changed=()
    while IFS= read -r changed_path; do
        [[ -n "${changed_path}" ]] && changed+=("${changed_path}")
    done < <(git diff --name-only "${merge_base}")
    if ! python3 scripts/ci/affected_scope.py --resolved-plan \
        --base "${merge_base}" --head "${head_sha}" \
        "${changed[@]}" > "${plan}" 2> /dev/null; then
        echo "cross-check: publishing no receipt — the resolved plan could not be calculated"
        return 0
    fi

    local -a extra=()
    [[ -n "${run_args[*]:-}" ]] && extra=(--run-filters "${run_args[*]}")
    [[ "${dirty:-1}" != "0" ]] && extra+=(--remote-dirty)
    if ! python3 scripts/ci/local_evidence.py cross-check \
        --plan "${plan}" --package "${package}" --report "${report}" \
        --exit-code "${exit_code:-1}" --duration "${duration:-0}" \
        --tested-tree "${tested_tree:-}" --base "${merge_base}" --head "${head_sha}" \
        --host-label "WSL2 (${HOST[wsl]}, cross-check archive mode)" \
        "${extra[@]}" > "${receipt}"; then
        return 0
    fi
    git notes --ref refs/notes/ci-local/wsl2-ubuntu add -f -F "${receipt}" "${head_sha}"
    if git push --no-verify origin \
        refs/notes/ci-local/wsl2-ubuntu:refs/notes/ci-local/wsl2-ubuntu > /dev/null 2>&1; then
        echo "cross-check: published a wsl2-ubuntu receipt for ${package} at ${head_sha:0:9}."
    else
        echo "cross-check: recorded a wsl2-ubuntu receipt locally; it could not be pushed."
    fi
}

run_windows() {
    local host="${HOST[windows]}"
    local script="${work_dir}/cross-check-${run_id}.ps1"
    # The remote shell is Windows PowerShell 5: no '&&', and a native
    # command's failure does not fail the session, so every step checks
    # \$LASTEXITCODE and the lock is released in `finally`.
    cat > "${script}" <<EOF
\$base = '${WIN_BASE}'
\$repo = '${WIN_DIR}'
${required_backends:+\$env:BISCUIT_TEST_REQUIRED_BACKENDS = '${required_backends}'}
\$lock = "\$base\\.cross-check.lock"
\$patch = "\$base\\cross-check-${run_id}.patch"
\$self = "\$base\\cross-check-${run_id}.ps1"
\$waited = 0
while (\$true) {
    try { New-Item -ItemType Directory -Path \$lock -ErrorAction Stop | Out-Null; break }
    catch {
        \$owner = Get-Content "\$lock\\owner" -ErrorAction SilentlyContinue
        if (\$waited -eq 0) { Write-Host "cross-check: waiting for lock on \$env:COMPUTERNAME; held by: \$owner" }
        if (\$waited -ge ${LOCK_WAIT_SECS}) {
            Write-Host "cross-check: gave up after ${LOCK_WAIT_SECS}s; \$lock still held by: \$owner"
            Write-Host "cross-check: if that run is dead its owner removes the lock by hand; never remove someone else's"
            Remove-Item -Force \$patch, \$self -ErrorAction SilentlyContinue
            exit 75
        }
        Start-Sleep -Seconds 5
        \$waited += 5
    }
}
Set-Content -Path "\$lock\\owner" -Value '${owner}'
function Invoke-CrossCheck {
    if (-not (Test-Path "\$repo\\.git")) {
        git clone --quiet '${origin_url}' \$repo
        if (\$LASTEXITCODE -ne 0) { return \$LASTEXITCODE }
    }
    Set-Location \$repo
    git fetch --quiet origin
    if (\$LASTEXITCODE -ne 0) { return \$LASTEXITCODE }
    git reset --quiet --hard
    if (\$LASTEXITCODE -ne 0) { return \$LASTEXITCODE }
    git clean -fdq
    if (\$LASTEXITCODE -ne 0) { return \$LASTEXITCODE }
    git checkout --quiet --detach '${base_sha}'
    if (\$LASTEXITCODE -ne 0) { return \$LASTEXITCODE }
    if (Test-Path \$patch) {
        git apply \$patch
        if (\$LASTEXITCODE -ne 0) { return \$LASTEXITCODE }
    }
    cargo nextest run -p '${package}' --no-fail-fast ${extra_args[*]@Q}
    return \$LASTEXITCODE
}
try { \$code = Invoke-CrossCheck }
finally { Remove-Item -Recurse -Force \$lock, \$patch, \$self -ErrorAction SilentlyContinue }
exit \$code
EOF
    local -a ship=("${script}")
    [[ "${patch_lines}" != "0" ]] && ship+=("${patch_file}")
    "${SSH[@]}" "${host}" "New-Item -ItemType Directory -Force -Path '${WIN_BASE}' | Out-Null"
    "${SCP[@]}" "${ship[@]}" "${host}:W:/ci-verification/"
    "${SSH[@]}" "${host}" "powershell -NoProfile -ExecutionPolicy Bypass -File '${WIN_BASE}\\cross-check-${run_id}.ps1'; exit \$LASTEXITCODE"
}

declare -A results=()
for os in "${ORDER[@]}"; do
    [[ -n "${want[${os}]:-}" ]] || continue
    echo
    case "${os}" in
        windows) echo "== windows (${HOST[windows]}) ==" ;;
        wsl) echo "== wsl (${HOST[wsl]}, archive mode) ==" ;;
        *) echo "== ${os} (${HOST[${os}]}) ==" ;;
    esac
    if [[ "${os}" == "windows" ]]; then
        if run_windows; then results[windows]="pass"; else results[windows]="FAIL"; fi
    else
        if run_unix "${os}"; then results[${os}]="pass"; else results[${os}]="FAIL"; fi
    fi
done

if [[ -n "${want[wsl]:-}" ]]; then
    publish_wsl_receipt || true
fi

echo
echo "cross-check summary for ${package}:"
status=0
for os in "${ORDER[@]}"; do
    [[ -n "${results[${os}]:-}" ]] || continue
    printf '  %-8s %s\n' "${os}" "${results[${os}]}"
    [[ "${results[${os}]}" == "pass" ]] || status=1
done
exit "${status}"
