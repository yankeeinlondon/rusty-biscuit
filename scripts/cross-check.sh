#!/usr/bin/env bash
# Cross-OS pre-push smoke: run one package's L1 suite on the standing
# build-host clones (real Linux, native Windows, WSL2) against the LOCAL
# tree — committed-but-unpushed and uncommitted changes included.
#
# Hosts come from the environment, never from the script: BUILD_LINUX,
# BUILD_WIN, and BUILD_WSL name the SSH destinations (an alias from
# ~/.ssh/config or user@host). Setting a variable is how a machine declares
# that host is available; an unset variable means "no such host here", and
# the default `--host all` simply runs on whichever are set.
#
# Sync model: the remote standing clone is reset to the nearest commit the
# remote can fetch (origin/<current-branch> if pushed, else origin/main),
# then the local tree's difference from that commit — tracked and untracked —
# is shipped as one patch and applied. No local commit or push is required.
#
# The WSL host runs the suite the way CI's wsl2-ubuntu leg does: as a nextest
# ARCHIVE, executed with the builder's target directory hidden, so anything
# that bakes a build-host path at compile time (`env!("CARGO_BIN_EXE_*")`)
# fails here exactly as it fails in CI. Feature flags (--features,
# --all-features, --no-default-features) go to the archive build; every other
# extra arg goes to the run.
#
# The standing clones and their target dirs persist between runs so compile
# caches are warm:
#   $BUILD_LINUX   ~/ci-verification/rusty-biscuit
#   $BUILD_WIN     W:\ci-verification\rusty-biscuit
#   $BUILD_WSL     ~/ci-verification/rusty-biscuit
#
# Usage:
#   scripts/cross-check.sh [--host linux|windows|wsl|all] <package> [nextest args...]
set -euo pipefail

LINUX_HOST="${BUILD_LINUX:-}"
WIN_HOST="${BUILD_WIN:-}"
WSL_HOST="${BUILD_WSL:-}"
LINUX_DIR="ci-verification/rusty-biscuit" # relative to remote $HOME (Linux and WSL)
WIN_DIR='W:\ci-verification\rusty-biscuit'
SSH=(ssh -o BatchMode=yes -o ConnectTimeout=15)

hosts="all"
package=""
extra_args=()
while (($# > 0)); do
    case "$1" in
        --host) shift; hosts="${1:?--host needs linux|windows|wsl|all}" ;;
        --host=*) hosts="${1#--host=}" ;;
        -h | --help)
            sed -n '2,32p' "$0" | sed 's/^# \{0,1\}//'
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
case "${hosts}" in linux | windows | wsl | all) ;; *)
    echo "cross-check: --host must be linux, windows, wsl, or all" >&2
    exit 2
    ;;
esac

# Resolve the requested hosts against what this machine declares available.
# An explicit --host for an undeclared host is an error; `all` narrows to the
# declared set and only errors when that set is empty.
want_linux=0
want_windows=0
want_wsl=0
case "${hosts}" in
    linux)
        if [[ -z "${LINUX_HOST}" ]]; then
            echo "cross-check: --host linux requested but BUILD_LINUX is not set" >&2
            exit 2
        fi
        want_linux=1
        ;;
    windows)
        if [[ -z "${WIN_HOST}" ]]; then
            echo "cross-check: --host windows requested but BUILD_WIN is not set" >&2
            exit 2
        fi
        want_windows=1
        ;;
    wsl)
        if [[ -z "${WSL_HOST}" ]]; then
            echo "cross-check: --host wsl requested but BUILD_WSL is not set" >&2
            exit 2
        fi
        want_wsl=1
        ;;
    all)
        [[ -n "${LINUX_HOST}" ]] && want_linux=1
        [[ -n "${WIN_HOST}" ]] && want_windows=1
        [[ -n "${WSL_HOST}" ]] && want_wsl=1
        if ((want_linux == 0 && want_windows == 0 && want_wsl == 0)); then
            echo "cross-check: no build hosts are declared on this machine (set BUILD_LINUX, BUILD_WIN, and/or BUILD_WSL)" >&2
            exit 2
        fi
        ;;
esac

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

cd "$(git rev-parse --show-toplevel)"
origin_url="$(git remote get-url origin)"
branch="$(git rev-parse --abbrev-ref HEAD)"
if git rev-parse --quiet --verify "origin/${branch}" > /dev/null; then
    base_ref="origin/${branch}"
else
    base_ref="origin/main"
fi
base_sha="$(git rev-parse "${base_ref}")"

# One patch carrying everything local relative to the base: tracked changes,
# then each untracked (non-ignored) file as a new-file hunk.
patch_file="$(mktemp "${TMPDIR:-/tmp}/cross-check.XXXXXX.patch")"
trap 'rm -f "${patch_file}"' EXIT
git diff --binary "${base_sha}" > "${patch_file}"
while IFS= read -r untracked; do
    [[ -n "${untracked}" ]] || continue
    git diff --no-index --binary /dev/null "${untracked}" >> "${patch_file}" || true
done < <(git ls-files --others --exclude-standard)

patch_lines="$(wc -l < "${patch_file}" | tr -d ' ')"
echo "cross-check: ${package} @ ${base_ref} (${base_sha:0:9}) + ${patch_lines} patch line(s)"
resolved=""
((want_linux)) && resolved+="linux(${LINUX_HOST}) "
((want_windows)) && resolved+="windows(${WIN_HOST}) "
((want_wsl)) && resolved+="wsl(${WSL_HOST})"
echo "hosts: ${resolved}  extra nextest args: ${extra_args[*]:-<none>}"

run_linux() {
    "${SSH[@]}" "${LINUX_HOST}" "set -e
        mkdir -p \"\$HOME/ci-verification\"
        if [ ! -d \"\$HOME/${LINUX_DIR}/.git\" ]; then
            git clone --quiet '${origin_url}' \"\$HOME/${LINUX_DIR}\"
        fi
        cd \"\$HOME/${LINUX_DIR}\"
        git fetch --quiet origin
        git reset --quiet --hard
        git clean -fdq
        git checkout --quiet --detach '${base_sha}'"
    if [[ "${patch_lines}" != "0" ]]; then
        "${SSH[@]}" "${LINUX_HOST}" "cd \"\$HOME/${LINUX_DIR}\" && git apply" < "${patch_file}"
    fi
    "${SSH[@]}" "${LINUX_HOST}" "cd \"\$HOME/${LINUX_DIR}\" && cargo nextest run -p '${package}' --no-fail-fast ${extra_args[*]@Q}"
}

run_windows() {
    # The remote shell is Windows PowerShell 5: no '&&', and a native
    # command's failure does not fail the session, so exit codes are
    # propagated explicitly via \$LASTEXITCODE.
    "${SSH[@]}" "${WIN_HOST}" "if (-not (Test-Path '${WIN_DIR}\\.git')) { git clone --quiet '${origin_url}' '${WIN_DIR}'; if (\$LASTEXITCODE -ne 0) { exit \$LASTEXITCODE } }
        Set-Location '${WIN_DIR}'
        git fetch --quiet origin
        git reset --quiet --hard
        git clean -fdq
        git checkout --quiet --detach '${base_sha}'; if (\$LASTEXITCODE -ne 0) { exit \$LASTEXITCODE }
        exit \$LASTEXITCODE"
    if [[ "${patch_lines}" != "0" ]]; then
        scp -q -o BatchMode=yes "${patch_file}" "${WIN_HOST}:W:/ci-verification/cross-check.patch"
        "${SSH[@]}" "${WIN_HOST}" "Set-Location '${WIN_DIR}'; git apply 'W:\\ci-verification\\cross-check.patch'; exit \$LASTEXITCODE"
    fi
    "${SSH[@]}" "${WIN_HOST}" "Set-Location '${WIN_DIR}'; cargo nextest run -p '${package}' --no-fail-fast ${extra_args[*]@Q}; exit \$LASTEXITCODE"
}

run_wsl() {
    # Same sync as Linux, then CI's archive mode. The guest's login shell is
    # what puts cargo on PATH, so the script goes in over stdin to `bash -ls`
    # rather than fighting nested quoting through `bash -lc`.
    "${SSH[@]}" "${WSL_HOST}" 'bash -ls' <<EOF
        set -e
        mkdir -p "\$HOME/ci-verification"
        if [ ! -d "\$HOME/${LINUX_DIR}/.git" ]; then
            git clone --quiet '${origin_url}' "\$HOME/${LINUX_DIR}"
        fi
        cd "\$HOME/${LINUX_DIR}"
        git fetch --quiet origin
        git reset --quiet --hard
        git clean -fdq
        git checkout --quiet --detach '${base_sha}'
EOF
    if [[ "${patch_lines}" != "0" ]]; then
        "${SSH[@]}" "${WSL_HOST}" "cd \"\$HOME/${LINUX_DIR}\" && git apply" < "${patch_file}"
    fi
    "${SSH[@]}" "${WSL_HOST}" 'bash -ls' <<EOF
        set -e
        cd "\$HOME/${LINUX_DIR}"
        archive="\$HOME/ci-verification/${package}.tar.zst"
        extract="\$(mktemp -d "\$HOME/ci-verification/extract.XXXXXX")"
        cargo nextest archive -p '${package}' --archive-file "\$archive" ${archive_args[*]@Q}
        # Hide the builder's target dir so a compile-time build-host path
        # cannot resolve; restore it on every exit so the cache survives.
        mv target target.hold
        trap 'cd "\$HOME/${LINUX_DIR}" && mv target.hold target; rm -rf "\$extract"' EXIT
        cargo nextest run --archive-file "\$archive" \
            --workspace-remap "\$HOME/${LINUX_DIR}" --extract-to "\$extract" \
            --no-fail-fast ${run_args[*]@Q}
EOF
}

declare -A results=()
if ((want_linux)); then
    echo
    echo "== linux (${LINUX_HOST}) =="
    if run_linux; then results[linux]="pass"; else results[linux]="FAIL"; fi
fi
if ((want_windows)); then
    echo
    echo "== windows (${WIN_HOST}) =="
    if run_windows; then results[windows]="pass"; else results[windows]="FAIL"; fi
fi
if ((want_wsl)); then
    echo
    echo "== wsl (${WSL_HOST}, archive mode) =="
    if run_wsl; then results[wsl]="pass"; else results[wsl]="FAIL"; fi
fi

echo
echo "cross-check summary for ${package}:"
status=0
for os in linux windows wsl; do
    [[ -n "${results[${os}]:-}" ]] || continue
    printf '  %-8s %s\n' "${os}" "${results[${os}]}"
    [[ "${results[${os}]}" == "pass" ]] || status=1
done
exit "${status}"
