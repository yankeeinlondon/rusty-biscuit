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
# Sync model: the local tree — tracked changes and untracked, non-ignored files
# alike — is committed locally as one throwaway commit over the nearest base the
# remote can fetch (origin/<current-branch> if pushed, else origin/main), and
# that commit object is shipped as a `git bundle`. Every host checks out that
# exact revision, clean. The developer's branch, index, and stash stack are
# untouched, and no push is required. See "THE TESTED REVISION" below for why a
# patch applied over the base cannot serve here.
#
# Layout: nothing about a host's disk is assumed here. Each host keeps one
# standing clone per originating checkout, at
# `<coding dir>/<local host>--<worktree>/rusty-biscuit`, where the coding dir is
# the HOST's `CODING_DIR` (read from its login environment), else `~/coding`
# (`%USERPROFILE%\coding` on Windows), and `<worktree>` is the local linked
# worktree's directory name, or `main` for the main checkout. The path is
# resolved over SSH before anything is shipped. Separate checkouts therefore
# never share a clone or its target dir.
#
# Stale clones: before any leg starts, each selected host drops the clones this
# origin host left there for worktrees that no longer exist here; see
# `prepare_host`.
#
# Concurrency: two runs from the same checkout still share a clone. Each remote
# run holds that clone's lock (`<clone dir>/.cross-check.lock`, a directory
# created atomically) for the whole reset/checkout/test sequence, so
# overlapping runs queue instead of clobbering it. A waiter prints the lock
# owner and gives up after 30 minutes with exit 75. A lock left behind by a
# dead run is reported, never removed by this script.
#
# Execution model: every host runs the suite the way CI runs it, as a
# producer/consumer pair rather than one `cargo nextest run`.
#
#   1. a resolved plan is computed HERE, once, for the tested revision and
#      shipped with the bundle; the planned build key in it is the key CI
#      computes for that same head;
#   2. on the host, `ci-build produce` writes the archive, its sidecars, and a
#      manifest for exactly that key, and the `ci-build` verifier is staged
#      beside them as CI stages it;
#   3. the archive, manifest, sidecars, and verifier are moved into a directory
#      the consumer owns — the transfer, on one host — and a SECOND checkout is
#      materialized as a `git worktree` at another path;
#   4. `just _ci_build_verify` refuses a mismatched, corrupt, or incompatible
#      input before anything is extracted;
#   5. the producer's target directory is hidden and the canonical tier recipe
#      runs in archive mode, so anything that bakes a build-host path at compile
#      time (`env!("CARGO_BIN_EXE_*")`) fails here exactly as it fails in CI.
#
# The `--os wsl` leg produces the `ubuntu-latest` record and consumes it as
# `wsl2-ubuntu`, which is the same record and the same key CI's Linux producer
# writes. It does not prove the machine-to-machine transfer — that edge is
# CI's, and `_wsl-ci.yml` is where it is proven; what this proves is the
# archive contract and relocation on a real WSL2 guest.
#
# Cargo build flags (--features, --all-features, --no-default-features) cannot
# be honored in archive mode: the plan's declared feature arguments ARE the
# archive's, and overriding them would change the build key the run reports.
# Passing one therefore selects the older native path (`cargo nextest run`) for
# every host, which is announced and publishes no receipt.
#
# The remote run never reads the developer's `~/.config`. On the WSL guest
# that directory is a CIFS mount of the Synology NAS, and when the NAS is down
# `git` dies on its global config and cargo's package-file listing (gitoxide,
# honoring the global excludes at `$XDG_CONFIG_HOME/git/ignore`) dies with
# "Host is down" — before a single test runs (2026-09-10). The Unix preamble
# therefore sets GIT_CONFIG_GLOBAL=/dev/null (fetch/reset/clean/checkout need
# no identity) and points XDG_CONFIG_HOME at an empty local directory.
#
# A WSL run whose tested tree IS the outgoing head's tree, on a clean remote
# worktree, with no test filter, becomes a published `wsl2-ubuntu` validation
# receipt — so "prior WSL evidence" is an ordinary receipt CI can reuse rather
# than a log someone remembers. Every other run publishes nothing and prints
# the reason: this script ships the developer's LOCAL tree, uncommitted work
# included, so most of its runs test something no head names. A qualifying
# receipt records the build key and realized digest the remote actually ran, so
# the evidence names the program it came from.
#
# BISCUIT_TEST_REQUIRED_BACKENDS is forwarded from the caller's environment to
# every remote run when set. Without it a Level 2 test whose backend is absent
# on the host skips, and nextest prints PASS in ~0.02 s — indistinguishable
# from evidence unless you read the duration. Name the backend you expect
# (e.g. `BISCUIT_TEST_REQUIRED_BACKENDS=tmux just cross-check --os wsl …
# --features terminal-tests level2_`) and the skip becomes a failure.
#
# The standing clones and their target dirs persist between runs so compile
# caches are warm. Nothing sweeps them; see the `os` skill's build-hosts notes.
#
# Usage:
#   scripts/cross-check.sh [--os linux|windows|wsl|macos|all] <package> [nextest args...]
set -euo pipefail

LOCK_WAIT_SECS=1800
SSH=(ssh -o BatchMode=yes -o ConnectTimeout=15)
SCP=(scp -q -o BatchMode=yes -o ConnectTimeout=15)

# Bash 3.2 (macOS /bin/bash) supports indexed arrays only.
readonly linux=0 windows=1 wsl=2 macos=3
ORDER=(linux windows wsl macos)
declare -a HOST=([linux]="${BUILD_LINUX:-}" [windows]="${BUILD_WIN:-}" [wsl]="${BUILD_WSL:-}" [macos]="${BUILD_MACOS:-}")
declare -a VAR=([linux]=BUILD_LINUX [windows]=BUILD_WIN [wsl]=BUILD_WSL [macos]=BUILD_MACOS)
# Which plan environment each host IS, and which native producer owns its
# archive. Mirrors `.github/ci/environments.json`: only the WSL2 guest consumes
# another environment's record.
declare -a ENVIRONMENT=([linux]=ubuntu-latest [windows]=windows-latest [wsl]=wsl2-ubuntu [macos]=macos-latest)
declare -a PRODUCER=([linux]=ubuntu-latest [windows]=windows-latest [wsl]=ubuntu-latest [macos]=macos-latest)
# Each host's clone directory, resolved on that host (`prepare_host`).
declare -a CLONE_DIR=("" "" "" "")

shell_args() {
    local arg
    for arg in "$@"; do printf '%q ' "$arg"; done
}

powershell_quote() {
    printf "'%s'" "${1//\'/\'\'}"
}

powershell_args() {
    local arg
    for arg in "$@"; do
        arg="${arg//\'/\'\'}"
        printf "'%s' " "$arg"
    done
}

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
declare -a want=("" "" "" "")
skipped_local=""
if [[ "${os_sel}" == "all" ]]; then
    for os in "${ORDER[@]}"; do
        [[ -n "${HOST[${os}]}" ]] || continue
        if [[ "${os}" == "${here}" ]]; then skipped_local="${os}"; continue; fi
        want[${os}]=1
    done
    if [[ -z "${want[*]// /}" ]]; then
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

# Cargo build flags are what decides the mode. In archive mode the plan's
# declared feature arguments are already baked into the archive and honoring an
# override here would change the build key the run reports, so a caller who
# passes one gets the native path for every host instead — announced, and never
# publishable as evidence.
build_flags=()
run_args=()
take_next=0
for arg in ${extra_args[@]+"${extra_args[@]}"}; do
    if ((take_next)); then build_flags+=("${arg}"); take_next=0; continue; fi
    case "${arg}" in
        --features) build_flags+=("${arg}"); take_next=1 ;;
        --features=* | --all-features | --no-default-features) build_flags+=("${arg}") ;;
        *) run_args+=("${arg}") ;;
    esac
done
mode="archive"
if ((${#build_flags[@]} > 0)); then
    mode="native"
fi

# Forwarded verbatim into each remote script, so it is validated here rather
# than quoted for two shells: comma-separated backend identifiers only.
required_backends="${BISCUIT_TEST_REQUIRED_BACKENDS:-}"
if [[ -n "${required_backends}" && ! "${required_backends}" =~ ^[A-Za-z0-9_,-]+$ ]]; then
    echo "cross-check: BISCUIT_TEST_REQUIRED_BACKENDS must be comma-separated backend names, got: ${required_backends}" >&2
    exit 2
fi

# `_storage_preflight` refuses a Windows host below its Cargo-target floor. An
# archive consumer compiles nothing, so a bounded override is sometimes the
# honest answer on a rig whose free space is held by something unrelated — but
# it is the CALLER's decision, forwarded verbatim, never this script's default.
min_free_gib="${BISCUIT_BUILD_MIN_FREE_GIB:-}"
if [[ -n "${min_free_gib}" && ! "${min_free_gib}" =~ ^[0-9]+$ ]]; then
    echo "cross-check: BISCUIT_BUILD_MIN_FREE_GIB must be a non-negative integer, got: ${min_free_gib}" >&2
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
# The id also names a Git ref below, and a host whose name carries `~`, `:`, or
# a space would make `git update-ref` reject it.
short_host="${short_host//[^A-Za-z0-9-]/-}"
run_id="${short_host}-$$-$(date +%s)"
# One component of a clone directory's name. Lowercased: the Windows host's
# file system is case-insensitive, and two spellings of one checkout must not
# become two clones elsewhere.
offset_part() {
    local value="${1//[^A-Za-z0-9._-]/-}"
    printf '%s' "${value}" | tr '[:upper:]' '[:lower:]'
}
if [[ "$(git rev-parse --path-format=absolute --git-dir)" \
    == "$(git rev-parse --path-format=absolute --git-common-dir)" ]]; then
    worktree="main"
else
    worktree="$(offset_part "$(basename "$(git rev-parse --show-toplevel)")")"
fi
# Runs of `-` collapse so the host part never contains `--`: the first `--` in
# a clone directory's name is always the host/worktree separator, even for a
# worktree whose own name contains one.
origin_host="$(offset_part "${short_host}" | tr -s '-')"
clone_offset="${origin_host}--${worktree}"
# Every worktree of this repository that still exists here, spelled as its
# clone directory's `<worktree>` part. The main checkout is listed first; an
# entry git calls `prunable` has lost its directory and is gone.
live_worktrees=()
first_entry=1
pending=""
while IFS= read -r line; do
    case "${line}" in
        "worktree "*)
            if ((first_entry)); then pending="main"; first_entry=0
            else pending="$(offset_part "$(basename "${line#worktree }")")"; fi
            ;;
        prunable*) pending="" ;;
        "")
            [[ -n "${pending}" ]] && live_worktrees+=("${pending}")
            pending=""
            ;;
    esac
done < <(git worktree list --porcelain; echo)
owner="${USER:-unknown}@${short_host} ${branch} ${base_sha:0:9} started $(date -u +%Y-%m-%dT%H:%M:%SZ)"

# THE TESTED REVISION
#
# `ci-build produce` binds an archive to `plan.head` and refuses any checkout
# whose `HEAD` differs or whose tracked tree is dirty (`build-source-mismatch`).
# Shipping a patch and applying it over the base therefore cannot work: the
# host's HEAD would be the base and its tree would be dirty, while the plan
# named something else. So this run's tested content is committed HERE, once,
# as a real commit object, and that object — not a patch — is what every host
# receives.
#
# The commit is built from a throwaway index: the base tree overlaid with the
# working tree's tracked changes and its untracked, non-ignored files, exactly
# the content the patch used to carry. The developer's branch, index, and stash
# stack are untouched, nothing is signed (signing would prompt), and the only
# ref created is deleted on exit.
#
# It is SHIPPED rather than recreated per host — a `git bundle` whose one
# prerequisite is the base every host already fetches — so all four hosts run
# the same commit id by construction, with no dependence on `git apply`, CRLF
# translation, or clean filters agreeing across Linux, macOS, and Windows.
#
# When the working tree matches the base exactly there is nothing to commit and
# the base IS the tested revision; no bundle is shipped.
work_dir="$(mktemp -d "${TMPDIR:-/tmp}/cross-check.XXXXXX")"
synthetic_ref="refs/cross-check/${run_id}"
trap 'rm -rf "${work_dir}"; git update-ref -d "${synthetic_ref}" 2> /dev/null || true' EXIT

index_file="${work_dir}/index"
GIT_INDEX_FILE="${index_file}" git read-tree "${base_sha}"
GIT_INDEX_FILE="${index_file}" git add --all
tested_tree="$(GIT_INDEX_FILE="${index_file}" git write-tree)"

bundle_file="${work_dir}/cross-check-${run_id}.bundle"
if [[ "${tested_tree}" == "$(git rev-parse "${base_sha}^{tree}")" ]]; then
    tested_revision="${base_sha}"
    bundle_file=""
    echo "cross-check: ${package} @ ${base_ref} (${base_sha:0:9}), no local changes"
else
    tested_revision="$(
        GIT_AUTHOR_NAME="cross-check" GIT_AUTHOR_EMAIL="cross-check@invalid" \
            GIT_COMMITTER_NAME="cross-check" GIT_COMMITTER_EMAIL="cross-check@invalid" \
            git -c commit.gpgsign=false commit-tree "${tested_tree}" \
            -p "${base_sha}" -m "cross-check ${run_id}"
    )"
    git update-ref "${synthetic_ref}" "${tested_revision}"
    git bundle create --quiet "${bundle_file}" "${base_sha}..${synthetic_ref}"
    changed="$(git diff --name-only "${base_sha}" "${tested_revision}" | wc -l | tr -d ' ')"
    echo "cross-check: ${package} @ ${base_ref} (${base_sha:0:9}) + ${changed} changed file(s)" \
        "as ${tested_revision:0:9}"
fi
resolved=""
for os in "${ORDER[@]}"; do
    [[ -n "${want[${os}]:-}" ]] && resolved+="${os}(${HOST[${os}]}) "
done
[[ -n "${skipped_local}" ]] && resolved+=" (${skipped_local} skipped: local OS)"
echo "os: ${resolved}  extra nextest args: ${extra_args[*]:-<none>}"

# The plan is resolved HERE and shipped, not recomputed on each host: one
# calculation is what makes every host report the same planned key, and the key
# is the planner's — computed through `ci-build` over the environment table's
# build contracts — not something this script invents. `--all` selects every
# package so a smoke run is never limited to what happens to be in scope.
plan_file="${work_dir}/cross-check-${run_id}-plan.json"
declare -a BUILD_KEY=("" "" "" "")
declare -a BUILD_ARTIFACT=("" "" "" "")
if [[ "${mode}" == "archive" ]]; then
    if ! python3 scripts/ci/affected_scope.py --all --resolved-plan \
        --base "${base_sha}" --head "${tested_revision}" > "${plan_file}"; then
        echo "cross-check: the resolved plan could not be calculated; nothing can name a build key" >&2
        exit 2
    fi
    for os in "${ORDER[@]}"; do
        [[ -n "${want[${os}]:-}" ]] || continue
        producer="${PRODUCER[${os}]}"
        key="$(jq -r --arg p "${package}" --arg pr "${producer}" \
            '[.builds[] | select(.package == $p and .producer == $pr) | .key]
             | if length == 1 then .[0] else "" end' "${plan_file}")"
        if [[ -z "${key}" ]]; then
            echo "cross-check: the plan resolves no single ${producer} build for ${package}," >&2
            echo "  so ${os} cannot run an archive it can name. Re-run with a build flag" >&2
            echo "  (e.g. --all-features) to take the native path deliberately." >&2
            exit 2
        fi
        BUILD_KEY[${os}]="${key}"
        BUILD_ARTIFACT[${os}]="build-${package}-${producer}-${key}"
    done
    echo "mode: archive — planned keys:"
    for os in "${ORDER[@]}"; do
        [[ -n "${want[${os}]:-}" ]] || continue
        printf '  %-8s %s produced on %s, consumed as %s\n' \
            "${os}" "${BUILD_KEY[${os}]}" "${PRODUCER[${os}]}" "${ENVIRONMENT[${os}]}"
    done
else
    echo "mode: native — ${build_flags[*]} overrides the plan's declared features, so no"
    echo "  archive can carry the key this run would report; nothing here is publishable."
fi

# Prepare one host before any leg does work: remove the clones this origin host
# left there for worktrees that no longer exist, then resolve (and create) this
# checkout's clone directory and record it in CLONE_DIR.
#
# The coding dir is the host's own setting, so it is read on the host: a login
# shell on Unix (the same environment the run itself gets) and the user
# environment on Windows.
#
# Only `<this host>--*` directories are candidates, and one is removed only
# when its worktree is not live here and its lock is not held. Removal is a
# rename to `.cross-check-trash--<this host>--…` (instant, and the clone's name
# is free at once) followed by a detached delete, because a Rust clone's
# `target/` takes minutes to delete and no leg should wait on it. Nothing
# reports whether that delete finishes, so every preparation also re-deletes
# this origin's leftover trash. A rename that fails is reported and does not
# stop the run.
unix_prepare_script() {
    cat <<EOF
set -u
coding="\${CODING_DIR:-\$HOME/coding}"
mkdir -p "\$coding" && cd "\$coding" || exit 1
coding="\$(pwd)"
live=' ${live_worktrees[*]} '
for dir in "\$coding"/'${origin_host}'--*/; do
    [ -d "\$dir" ] || continue
    dir="\${dir%/}"
    tree="\${dir##*/}"
    tree="\${tree#'${origin_host}'--}"
    case "\$live" in *" \$tree "*) continue ;; esac
    if [ -d "\$dir/.cross-check.lock" ]; then echo "stale-locked: \$dir"; continue; fi
    if mv "\$dir" "\$coding/.cross-check-trash--\${dir##*/}-\$\$-\$(date +%s)"; then
        echo "stale-removed: \$dir"
    else
        echo "stale-failed: \$dir"
    fi
done
# Detached, so the SSH session ends without waiting for it.
for trash in "\$coding"/.cross-check-trash--'${origin_host}'--*/; do
    [ -d "\$trash" ] || continue
    nohup rm -rf "\${trash%/}" < /dev/null > /dev/null 2>&1 &
done
dir="\$coding/${clone_offset}"
mkdir -p "\$dir" || exit 1
echo "clone-dir: \$dir"
EOF
}

# The trash delete is started through WMI rather than as a child: Windows
# OpenSSH ends a session's child processes with it. `\\?\` lifts the
# 260-character path limit a deep `target\` exceeds.
windows_prepare_script() {
    local live="" name
    for name in ${live_worktrees[@]+"${live_worktrees[@]}"}; do live+="'${name}',"; done
    cat <<EOF
\$ProgressPreference = 'SilentlyContinue'
\$coding = if (\$env:CODING_DIR) { \$env:CODING_DIR } else { Join-Path \$env:USERPROFILE 'coding' }
New-Item -ItemType Directory -Force -Path \$coding -ErrorAction Stop | Out-Null
\$coding = (Resolve-Path -LiteralPath \$coding).Path
\$live = @(${live%,})
\$prefix = '${origin_host}--'
foreach (\$entry in Get-ChildItem -LiteralPath \$coding -Directory) {
    if (-not \$entry.Name.StartsWith(\$prefix, [StringComparison]::OrdinalIgnoreCase)) { continue }
    \$dir = \$entry.FullName
    if (\$live -contains \$entry.Name.Substring(\$prefix.Length)) { continue }
    if (Test-Path -LiteralPath (Join-Path \$dir '.cross-check.lock')) { Write-Output "stale-locked: \$dir"; continue }
    \$trash = ".cross-check-trash--\$(\$entry.Name)-\$PID-\$([DateTimeOffset]::UtcNow.ToUnixTimeSeconds())"
    try { Rename-Item -LiteralPath \$dir -NewName \$trash -ErrorAction Stop; Write-Output "stale-removed: \$dir" }
    catch { Write-Output "stale-failed: \$dir" }
}
foreach (\$entry in Get-ChildItem -LiteralPath \$coding -Directory -Force) {
    if (-not \$entry.Name.StartsWith(".cross-check-trash--\$prefix", [StringComparison]::OrdinalIgnoreCase)) { continue }
    \$line = 'cmd.exe /c rd /s /q "\\\\?\\' + \$entry.FullName + '"'
    Invoke-CimMethod -ClassName Win32_Process -MethodName Create -Arguments @{ CommandLine = \$line } | Out-Null
}
\$dir = Join-Path \$coding '${clone_offset}'
New-Item -ItemType Directory -Force -Path \$dir -ErrorAction Stop | Out-Null
Write-Output "clone-dir: \$dir"
EOF
}

prepare_host() {
    local os="$1" host="${HOST[$1]}" output dir line
    if [[ "${os}" == "windows" ]]; then
        # Encoded rather than inline: the command crosses the remote shell once
        # more, and `$` does not reliably survive that on this host.
        local encoded
        encoded="$(windows_prepare_script | iconv -f UTF-8 -t UTF-16LE | base64 | tr -d '\n')"
        output="$("${SSH[@]}" "${host}" "powershell -NoProfile -EncodedCommand ${encoded}")" || return 1
    else
        output="$(unix_prepare_script | "${SSH[@]}" "${host}" "bash -ls")" || return 1
    fi
    output="$(printf '%s\n' "${output}" | tr -d '\r')"
    while IFS= read -r line; do
        case "${line}" in
            "stale-removed: "*) echo "cross-check: removing stale clone ${host}:${line#stale-removed: } in the background (its worktree no longer exists here)" ;;
            "stale-locked: "*) echo "cross-check: kept stale clone ${host}:${line#stale-locked: } — its lock is held" ;;
            "stale-failed: "*) echo "cross-check: could not fully remove stale clone ${host}:${line#stale-failed: }; remove it by hand" >&2 ;;
        esac
    done <<< "${output}"
    # The last marker line wins, so a chatty login profile cannot corrupt it.
    dir="$(printf '%s\n' "${output}" | sed -n 's/^clone-dir: //p' | tail -n1)"
    if [[ "${os}" == "windows" && ! "${dir}" =~ ^[A-Za-z]:\\ ]] || [[ "${os}" != "windows" && "${dir}" != /* ]]; then
        echo "cross-check: could not resolve the clone directory on ${host} (got: '${dir}')" >&2
        return 1
    fi
    CLONE_DIR[${os}]="${dir}"
    echo "clone: ${host}:${dir}"
}

# Remote scripts are generated locally, shipped next to the bundle, and run in
# one SSH session each so the lock covers the whole sequence. Remote-side
# variables are escaped (\$); everything else is substituted here.

# The lock and clone sync shared by linux, wsl, and macos. `cleanup` is the
# single EXIT trap for the whole remote run; mode-specific steps append to
# `extra_cleanup` rather than installing their own trap.
unix_prelude() {
    cat <<EOF
set -euo pipefail
base=$(printf '%q' "$1")
repo="\$base/rusty-biscuit"
# Never touch the developer's ~/.config (a network mount on the WSL guest).
export GIT_CONFIG_GLOBAL=/dev/null
export XDG_CONFIG_HOME="\$base/.xdg-empty"
mkdir -p "\$XDG_CONFIG_HOME"
${required_backends:+export BISCUIT_TEST_REQUIRED_BACKENDS='${required_backends}'}
${min_free_gib:+export BISCUIT_BUILD_MIN_FREE_GIB='${min_free_gib}'}
lock="\$base/.cross-check.lock"
bundle="\$base/cross-check-${run_id}.bundle"
plan="\$base/cross-check-${run_id}-plan.json"
self="\$base/cross-check-${run_id}.sh"
# Until the lock is won only the shipped files are ours to remove; an
# interrupted waiter must not touch the holder's lock.
trap 'rm -f "\$bundle" "\$plan" "\$self"' EXIT
waited=0
until mkdir "\$lock" 2> /dev/null; do
    if [ "\$waited" -eq 0 ]; then
        echo "cross-check: waiting for lock on \$(hostname -s):\$base; held by: \$(cat "\$lock/owner" 2> /dev/null || echo unknown)"
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
    rm -rf "\$lock" "\$bundle" "\$plan" "\$self"
}
trap cleanup EXIT
if [ ! -d "\$repo/.git" ]; then
    git clone --quiet '${origin_url}' "\$repo"
fi
cd "\$repo"
git fetch --quiet origin
git reset --quiet --hard
git clean -fdq
# The bundle's only prerequisite is the base the fetch above just supplied.
if [ -s "\$bundle" ]; then
    git fetch --quiet "\$bundle" '${synthetic_ref}'
fi
# Nothing is applied on top: a checkout that is not this revision, or is dirty,
# is what \`ci-build\` refuses, so it must fail here instead.
git checkout --quiet --detach '${tested_revision}'
EOF
}

unix_run_native() {
    cat <<EOF
cargo nextest run -p '${package}' --no-fail-fast $(shell_args ${extra_args[@]+"${extra_args[@]}"})
EOF
}

# The producer/consumer sequence, identical on linux, wsl, and macos. Only the
# planned key, its producer, and the consuming environment differ, and all three
# come from the plan shipped with this script.
unix_run_archive() {
    local os="$1"
    local producer="${PRODUCER[${os}]}" environment="${ENVIRONMENT[${os}]}"
    local key="${BUILD_KEY[${os}]}" artifact="${BUILD_ARTIFACT[${os}]}"
    cat <<EOF
plan="\$base/cross-check-${run_id}-plan.json"
out="\$base/cross-check-${run_id}-build"
consume="\$base/cross-check-${run_id}-consume"
src="\$consume/src"
reports="\$consume/reports"
report="\$base/cross-check-${run_id}-report.xml"
rm -rf "\$out" "\$consume"
mkdir -p "\$out" "\$consume" "\$reports"
# Hide the producer's target dir so a compile-time build-host path cannot
# resolve; restore it on every exit so the cache survives. The second checkout
# is a git worktree, so removing it is a git operation, not an rm.
extra_cleanup() {
    cd "\$repo" 2> /dev/null || return 0
    git worktree remove --force "\$src" > /dev/null 2>&1 || true
    git worktree prune > /dev/null 2>&1 || true
    if [ -d target.hold ]; then rm -rf target; mv target.hold target; fi
    rm -rf "\$consume" "\$out"
}
# What the remote actually tested. Printed rather than assumed: a receipt for
# this run is only publishable when this tree IS the outgoing head's tree and
# the worktree was clean (spec section 3.8).
echo "cross-check-tree: \$(git rev-parse HEAD^{tree})"
echo "cross-check-dirty: \$(git status --porcelain | wc -l | tr -d ' ')"
export NEXTEST_PROFILE="\${NEXTEST_PROFILE:-ci}"
started=\$(date +%s)

# 1. The producer tool, built exactly as CI's owner leg builds it: the feature
#    split that keeps sniff, duckdb, and gix out of a hashing helper.
cargo build --release --manifest-path scripts/Cargo.toml \\
    --no-default-features --features build-tools --bin ci-build
tool="\$repo/target/release/ci-build"

# 2. One record, named by the plan. \`produce\` refuses a key this toolchain is
#    not the planned host for, before it compiles anything.
"\$tool" produce --plan "\$plan" --producer '${producer}' --key '${key}' --out-dir "\$out"

# 3. The verifier travels with the thing it verifies, as it does in CI: the
#    consumer must not have to build the tool that judges its inputs.
mkdir -p "\$out/tools"
cp "\$tool" "\$out/tools/"

# 4. The transfer. The consumer is handed its own directory and never reads the
#    producer's, so a path that resolved only because the two were the same
#    directory fails here.
cp -R "\$out" "\$consume/build"

# 5. A different checkout at a different path, and the producer's target tree
#    hidden. A worktree rather than a copy: the same tree, cheaply, with no
#    build outputs to inherit.
git worktree prune > /dev/null 2>&1 || true
git worktree add --detach --quiet "\$src" '${tested_revision}'
mv target target.hold

# 6. Verification, before anything is extracted. A rejection exits non-zero and
#    the tier never starts; nothing here may compile a replacement.
cd "\$src"
just _ci_build_verify "\$consume/build" '${artifact}' '${environment}' "\$plan"
manifest="\$consume/build/${artifact}.manifest.json"
archive_file="\$consume/build/\$(jq -r '.archive.file' "\$manifest")"
echo "cross-check-key: \$(jq -r '.key' "\$manifest")"
echo "cross-check-digest: \$(jq -r '.digest' "\$manifest")"

# 7. The canonical tier recipe, in archive mode, with the same bindings CI's
#    consumer steps export. \`cargo-nextest nextest\` is the standalone driver:
#    a consumer that reached \`cargo\` would be compiling.
export BISCUIT_NEXTEST_BIN='cargo-nextest nextest'
export BISCUIT_JUNIT_WORKSPACE_ROOT="\$src"
export BISCUIT_JUNIT_TARGET_DIR="\$src/target"
export INSTA_WORKSPACE_ROOT="\$src"
export BISCUIT_JUNIT_STAGE_DIR="\$reports"
export BISCUIT_CI_ENVIRONMENT='${environment}'
sidecar_dir="\$consume/build/${artifact}-sidecars"
if [ -d "\$sidecar_dir" ]; then
    export PATH="\$sidecar_dir:\$PATH"
fi
set +e
just _test '${package}' --no-fail-fast \\
    --archive-file "\$archive_file" --workspace-remap "\$src" $(shell_args ${run_args[@]+"${run_args[@]}"})
run_code=\$?
set -e
echo "cross-check-exit: \$run_code"
echo "cross-check-duration: \$(( \$(date +%s) - started ))"
# Staged by the tier recipe itself, at a path this run owns. If it is absent the
# run publishes nothing and says so locally — it never invents a measurement.
if [ -f "\$reports/L1/${package}.xml" ]; then
    cp "\$reports/L1/${package}.xml" "\$report"
fi
exit \$run_code
EOF
}

run_unix() {
    local os="$1" host="${HOST[$1]}"
    local script="${work_dir}/cross-check-${run_id}.sh"
    local base="${CLONE_DIR[${os}]}"
    local remote_script
    remote_script="$(printf '%q' "${base}/cross-check-${run_id}.sh")"
    {
        unix_prelude "${base}"
        if [[ "${mode}" == "archive" ]]; then unix_run_archive "${os}"; else unix_run_native; fi
    } > "${script}"
    local -a ship=("${script}")
    [[ -n "${bundle_file}" ]] && ship+=("${bundle_file}")
    [[ "${mode}" == "archive" ]] && ship+=("${plan_file}")
    "${SCP[@]}" "${ship[@]}" "${host}:${base}/"
    # A login shell is what puts cargo on PATH on every Unix host, but the run
    # script is handed to a NON-login `bash` inside it. A login shell that runs
    # a script file reports the status of the last command in the host's
    # `~/.bash_logout` instead of the script's: on the WSL guest that file ends
    # with `clear_console -q`, which fails without a tty, so every green run
    # came back as exit 1 (the "wsl FAIL with cross-check-exit: 0" the `os`
    # skill recorded on 2026-09-15; diagnosed 2026-09-22). `bash -lc` does not
    # do this, and the inner shell inherits the exported PATH.
    #
    # The WSL leg's output is teed so its `cross-check-*:` markers can be read
    # afterwards without hiding the run from the terminal.
    if [[ "${os}" == "wsl" ]]; then
        "${SSH[@]}" "${host}" "bash -lc 'bash \"\$0\"' ${remote_script}" \
            2>&1 | tee "${work_dir}/wsl.log"
        return "${PIPESTATUS[0]}"
    fi
    local status=0
    "${SSH[@]}" "${host}" "bash -lc 'bash \"\$0\"' ${remote_script}" || status=$?
    # The staged JUnit report outlives the remote run on purpose — only the WSL
    # leg's receipt fetches one — so every other host has to be told to drop it,
    # or a shared rig accumulates one per run forever.
    discard_remote_report "${os}"
    return "${status}"
}

# Remove this run's staged JUnit report from a Unix host.
discard_remote_report() {
    "${SSH[@]}" "${HOST[$1]}" "rm -f $(printf '%q' "${CLONE_DIR[$1]}/cross-check-${run_id}-report.xml")" \
        > /dev/null 2>&1 || true
}

# Turn a qualifying WSL archive run into a `wsl2-ubuntu` validation receipt.
#
# "Qualifying" is `local_evidence.py cross-check`'s decision, not this script's:
# exact outgoing tree, clean remote worktree, unfiltered run, and a real report.
# Anything else prints the reason and publishes nothing (spec section 3.8).
publish_wsl_receipt() {
    local log="${work_dir}/wsl.log"
    [[ -f "${log}" ]] || return 0
    local tested_tree dirty exit_code duration build_key build_digest
    tested_tree="$(sed -n 's/^cross-check-tree: //p' "${log}" | tail -n1)"
    dirty="$(sed -n 's/^cross-check-dirty: //p' "${log}" | tail -n1)"
    exit_code="$(sed -n 's/^cross-check-exit: //p' "${log}" | tail -n1)"
    duration="$(sed -n 's/^cross-check-duration: //p' "${log}" | tail -n1)"
    # The remote's own manifest, not the local plan's expectation: a receipt
    # that named a key nothing verified would be a claim rather than evidence.
    build_key="$(sed -n 's/^cross-check-key: //p' "${log}" | tail -n1)"
    build_digest="$(sed -n 's/^cross-check-digest: //p' "${log}" | tail -n1)"

    local report="${work_dir}/wsl-report.xml"
    local fetched=0
    "${SCP[@]}" "${HOST[wsl]}:${CLONE_DIR[wsl]}/cross-check-${run_id}-report.xml" \
        "${report}" 2> /dev/null && fetched=1
    # Dropped here, not on each exit path below: the local copy is made, and
    # every reason this function declines to publish leaves the same litter.
    discard_remote_report wsl
    if (( fetched == 0 )); then
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
    [[ -n "${build_key}" ]] && extra+=(--build-key "${build_key}")
    [[ -n "${build_digest}" ]] && extra+=(--build-digest "${build_digest}")
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

# The native-Windows body of `Invoke-CrossCheck`: the same seven steps the Unix
# archive script performs, in PowerShell 5's idiom.
#
# Every path this hands to `just` goes through `_native_path` first. `just`
# pastes a recipe's `*args` list raw into a bash array, so bash processes
# backslash escapes and `C:\Users\…\x.tar.zst` reaches nextest as
# `C:Usersken…` — a missing-file error for a path nobody typed (measured on
# `build-win-native`, 2026-09-14). `_native_path` is the shipped rule for the
# one spelling both layers accept; restating it here would be a second copy of
# it to keep in step.
windows_run_archive() {
    local producer="${PRODUCER[windows]}" environment="${ENVIRONMENT[windows]}"
    local key="${BUILD_KEY[windows]}" artifact="${BUILD_ARTIFACT[windows]}"
    cat <<EOF
    \$env:NEXTEST_PROFILE = \$(if (\$env:NEXTEST_PROFILE) { \$env:NEXTEST_PROFILE } else { 'ci' })
    \$started = [int][double]::Parse((Get-Date -UFormat %s))
    Write-Host "cross-check-tree: \$(git rev-parse 'HEAD^{tree}')"
    Write-Host "cross-check-dirty: \$((git status --porcelain | Measure-Object -Line).Lines)"

    cargo build --release --manifest-path scripts/Cargo.toml --no-default-features --features build-tools --bin ci-build
    if (\$LASTEXITCODE -ne 0) { \$script:code = \$LASTEXITCODE; return }
    # The scripts crate is a member of the root workspace, so its binaries land
    # in the workspace target dir: the one Move-Item hides below, not a target
    # dir of its own. Never exercised until 2026-09-22, because the storage
    # preflight refused this leg before it could reach the tool.
    \$tool = "\$repo\\target\\release\\ci-build.exe"

    & \$tool produce --plan \$plan --producer '${producer}' --key '${key}' --out-dir \$out
    if (\$LASTEXITCODE -ne 0) { \$script:code = \$LASTEXITCODE; return }

    New-Item -ItemType Directory -Force -Path "\$out\\tools" | Out-Null
    Copy-Item \$tool "\$out\\tools\\" -Force
    New-Item -ItemType Directory -Force -Path \$consume | Out-Null
    Copy-Item \$out "\$consume\\build" -Recurse -Force
    New-Item -ItemType Directory -Force -Path \$reports | Out-Null

    git worktree prune 2>&1 | Out-Null
    git worktree add --detach --quiet \$src '${tested_revision}'
    if (\$LASTEXITCODE -ne 0) { \$script:code = \$LASTEXITCODE; return }
    Move-Item "\$repo\\target" "\$repo\\target.hold"

    Set-Location \$src
    \$nativeSrc = (just _native_path \$src | Select-Object -Last 1)
    \$nativeBuild = (just _native_path "\$consume\\build" | Select-Object -Last 1)
    just _ci_build_verify \$nativeBuild '${artifact}' '${environment}' \$plan
    if (\$LASTEXITCODE -ne 0) { \$script:code = \$LASTEXITCODE; return }
    \$manifest = "\$consume\\build\\${artifact}.manifest.json"
    \$archiveName = (Get-Content \$manifest -Raw | ConvertFrom-Json).archive.file
    \$archiveFile = (just _native_path "\$consume\\build\\\$archiveName" | Select-Object -Last 1)
    \$parsed = (Get-Content \$manifest -Raw | ConvertFrom-Json)
    Write-Host "cross-check-key: \$(\$parsed.key)"
    Write-Host "cross-check-digest: \$(\$parsed.digest)"

    \$env:BISCUIT_NEXTEST_BIN = 'cargo-nextest nextest'
    \$env:BISCUIT_JUNIT_WORKSPACE_ROOT = \$nativeSrc
    \$env:BISCUIT_JUNIT_TARGET_DIR = "\$nativeSrc/target"
    \$env:INSTA_WORKSPACE_ROOT = \$nativeSrc
    \$env:BISCUIT_JUNIT_STAGE_DIR = (just _native_path \$reports | Select-Object -Last 1)
    \$env:BISCUIT_CI_ENVIRONMENT = '${environment}'
    \$sidecarDir = "\$consume\\build\\${artifact}-sidecars"
    if (Test-Path \$sidecarDir) { \$env:PATH = "\$sidecarDir;\$env:PATH" }
    just _test '${package}' --no-fail-fast --archive-file \$archiveFile --workspace-remap \$nativeSrc $(powershell_args ${run_args[@]+"${run_args[@]}"})
    \$runCode = \$LASTEXITCODE
    Write-Host "cross-check-exit: \$runCode"
    Write-Host "cross-check-duration: \$([int][double]::Parse((Get-Date -UFormat %s)) - \$started)"
    if (Test-Path "\$reports\\L1\\${package}.xml") {
        Copy-Item "\$reports\\L1\\${package}.xml" \$report -Force
    }
    \$script:code = \$runCode
EOF
}

windows_run_native() {
    cat <<EOF
    cargo nextest run -p '${package}' --no-fail-fast $(powershell_args ${extra_args[@]+"${extra_args[@]}"})
    \$script:code = \$LASTEXITCODE
EOF
}

run_windows() {
    local host="${HOST[windows]}"
    local script="${work_dir}/cross-check-${run_id}.ps1"
    local base="${CLONE_DIR[windows]}"
    local windows_body
    if [[ "${mode}" == "archive" ]]; then
        windows_body="$(windows_run_archive)"
    else
        windows_body="$(windows_run_native)"
    fi
    # The remote shell is Windows PowerShell 5: no '&&', and a native
    # command's failure does not fail the session, so every step checks
    # \$LASTEXITCODE and the lock is released in `finally`.
    #
    # The run's exit code is a SCRIPT-SCOPED variable, never `Invoke-CrossCheck`'s
    # return value. A PowerShell function's output stream carries everything the
    # native commands inside it wrote to stdout, so `$code = Invoke-CrossCheck`
    # binds an array whose first element is a git message and `exit $code`
    # reports that — a tier that exited 1 was summarized `windows  pass` on
    # `build-win-native`, 2026-09-14. `| Out-Host` keeps the function's output
    # on the console, where the log needs it, without binding it to anything.
    cat > "${script}" <<EOF
\$base = $(powershell_quote "${base}")
\$repo = "\$base\\rusty-biscuit"
${required_backends:+\$env:BISCUIT_TEST_REQUIRED_BACKENDS = '${required_backends}'}
${min_free_gib:+\$env:BISCUIT_BUILD_MIN_FREE_GIB = '${min_free_gib}'}
\$lock = "\$base\\.cross-check.lock"
\$bundle = "\$base\\cross-check-${run_id}.bundle"
\$plan = "\$base\\cross-check-${run_id}-plan.json"
\$out = "\$base\\cross-check-${run_id}-build"
\$consume = "\$base\\cross-check-${run_id}-consume"
\$src = "\$consume\\src"
\$reports = "\$consume\\reports"
\$report = "\$base\\cross-check-${run_id}-report.xml"
\$self = "\$base\\cross-check-${run_id}.ps1"
\$waited = 0
while (\$true) {
    try { New-Item -ItemType Directory -Path \$lock -ErrorAction Stop | Out-Null; break }
    catch {
        \$owner = Get-Content "\$lock\\owner" -ErrorAction SilentlyContinue
        if (\$waited -eq 0) { Write-Host "cross-check: waiting for lock on \$(\$env:COMPUTERNAME):\$base; held by: \$owner" }
        if (\$waited -ge ${LOCK_WAIT_SECS}) {
            Write-Host "cross-check: gave up after ${LOCK_WAIT_SECS}s; \$lock still held by: \$owner"
            Write-Host "cross-check: if that run is dead its owner removes the lock by hand; never remove someone else's"
            Remove-Item -Force \$bundle, \$plan, \$self -ErrorAction SilentlyContinue
            exit 75
        }
        Start-Sleep -Seconds 5
        \$waited += 5
    }
}
Set-Content -Path "\$lock\\owner" -Value '${owner}'
\$script:code = 1
function Invoke-CrossCheck {
    if (-not (Test-Path "\$repo\\.git")) {
        git clone --quiet '${origin_url}' \$repo
        if (\$LASTEXITCODE -ne 0) { \$script:code = \$LASTEXITCODE; return }
    }
    Set-Location \$repo
    # Git for Windows refuses a path over 260 characters unless told otherwise,
    # whatever the OS allows, and this repository has paths deep enough to hit
    # that from a clone directory of ordinary length (measured 2026-09-22:
    # a darkmatter benchmark sample under the run's second checkout).
    git config core.longpaths true
    git fetch --quiet origin
    if (\$LASTEXITCODE -ne 0) { \$script:code = \$LASTEXITCODE; return }
    git reset --quiet --hard
    if (\$LASTEXITCODE -ne 0) { \$script:code = \$LASTEXITCODE; return }
    git clean -fdq
    if (\$LASTEXITCODE -ne 0) { \$script:code = \$LASTEXITCODE; return }
    # The bundle's only prerequisite is the base the fetch above just supplied.
    if (Test-Path \$bundle) {
        git fetch --quiet \$bundle '${synthetic_ref}'
        if (\$LASTEXITCODE -ne 0) { \$script:code = \$LASTEXITCODE; return }
    }
    # Nothing is applied on top: a checkout that is not this revision, or is
    # dirty, is what \`ci-build\` refuses, so it must fail here instead.
    git checkout --quiet --detach '${tested_revision}'
    if (\$LASTEXITCODE -ne 0) { \$script:code = \$LASTEXITCODE; return }
${windows_body}
}
try { Invoke-CrossCheck | Out-Host }
finally {
    Set-Location \$repo -ErrorAction SilentlyContinue
    git worktree remove --force \$src 2>&1 | Out-Null
    git worktree prune 2>&1 | Out-Null
    if (Test-Path "\$repo\\target.hold") {
        Remove-Item -Recurse -Force "\$repo\\target" -ErrorAction SilentlyContinue
        Move-Item "\$repo\\target.hold" "\$repo\\target" -ErrorAction SilentlyContinue
    }
    Remove-Item -Recurse -Force \$consume, \$out -ErrorAction SilentlyContinue
    Remove-Item -Recurse -Force \$lock, \$bundle, \$plan, \$self -ErrorAction SilentlyContinue
}
exit \$script:code
EOF
    local -a ship=("${script}")
    [[ -n "${bundle_file}" ]] && ship+=("${bundle_file}")
    [[ "${mode}" == "archive" ]] && ship+=("${plan_file}")
    # scp takes the forward-slash spelling of a Windows path.
    "${SCP[@]}" "${ship[@]}" "${host}:${base//\\//}/"
    local status=0
    "${SSH[@]}" "${host}" "powershell -NoProfile -ExecutionPolicy Bypass -File $(powershell_quote "${base}\\cross-check-${run_id}.ps1"); exit \$LASTEXITCODE" || status=$?
    # Nothing fetches a Windows report; see `discard_remote_report`.
    "${SSH[@]}" "${host}" "Remove-Item -Force $(powershell_quote "${base}\\cross-check-${run_id}-report.xml") -ErrorAction SilentlyContinue" \
        > /dev/null 2>&1 || true
    return "${status}"
}

declare -a results=("" "" "" "")
# Every selected host is pruned and prepared before any leg starts; a host that
# cannot be prepared fails its leg without running it.
echo
for os in "${ORDER[@]}"; do
    [[ -n "${want[${os}]:-}" ]] || continue
    prepare_host "${os}" || results[${os}]="FAIL"
done
for os in "${ORDER[@]}"; do
    [[ -n "${want[${os}]:-}" && -z "${results[${os}]}" ]] || continue
    echo
    if [[ "${mode}" == "archive" ]]; then
        echo "== ${os} (${HOST[${os}]}, archive ${BUILD_KEY[${os}]} from ${PRODUCER[${os}]}) =="
    else
        echo "== ${os} (${HOST[${os}]}, native) =="
    fi
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
