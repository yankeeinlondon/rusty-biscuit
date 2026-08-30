#!/usr/bin/env bash
# Cross-OS pre-push smoke: run one package's L1 suite on the standing
# build-host clones (real Linux via `build-linux`, native Windows via
# `build-win-native`) against the LOCAL tree — committed-but-unpushed and
# uncommitted changes included.
#
# Sync model: the remote standing clone is reset to the nearest commit the
# remote can fetch (origin/<current-branch> if pushed, else origin/main),
# then the local tree's difference from that commit — tracked and untracked —
# is shipped as one patch and applied. No local commit or push is required.
#
# The standing clones and their target dirs persist between runs so compile
# caches are warm:
#   build-linux       ~/ci-verification/rusty-biscuit
#   build-win-native  W:\ci-verification\rusty-biscuit
#
# Usage:
#   scripts/cross-check.sh [--host linux|windows|all] <package> [nextest args...]
set -euo pipefail

LINUX_HOST="build-linux"
WIN_HOST="build-win-native"
LINUX_DIR="ci-verification/rusty-biscuit" # relative to remote $HOME
WIN_DIR='W:\ci-verification\rusty-biscuit'
SSH=(ssh -o BatchMode=yes -o ConnectTimeout=15)

hosts="all"
package=""
extra_args=()
while (($# > 0)); do
    case "$1" in
        --host) shift; hosts="${1:?--host needs linux|windows|all}" ;;
        --host=*) hosts="${1#--host=}" ;;
        -h | --help)
            sed -n '2,19p' "$0" | sed 's/^# \{0,1\}//'
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
case "${hosts}" in linux | windows | all) ;; *)
    echo "cross-check: --host must be linux, windows, or all" >&2
    exit 2
    ;;
esac

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
echo "hosts: ${hosts}   extra nextest args: ${extra_args[*]:-<none>}"

run_linux() {
    "${SSH[@]}" "${LINUX_HOST}" "set -e
        mkdir -p \"\$HOME/ci-verification\"
        if [ ! -d \"\$HOME/${LINUX_DIR}/.git\" ]; then
            git clone --quiet '${origin_url}' \"\$HOME/${LINUX_DIR}\"
        fi
        cd \"\$HOME/${LINUX_DIR}\"
        git fetch --quiet origin
        git checkout --quiet --detach '${base_sha}'
        git reset --quiet --hard
        git clean -fdq"
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
        git checkout --quiet --detach '${base_sha}'; if (\$LASTEXITCODE -ne 0) { exit \$LASTEXITCODE }
        git reset --quiet --hard
        git clean -fdq
        exit \$LASTEXITCODE"
    if [[ "${patch_lines}" != "0" ]]; then
        scp -q -o BatchMode=yes "${patch_file}" "${WIN_HOST}:W:/ci-verification/cross-check.patch"
        "${SSH[@]}" "${WIN_HOST}" "Set-Location '${WIN_DIR}'; git apply 'W:\\ci-verification\\cross-check.patch'; exit \$LASTEXITCODE"
    fi
    "${SSH[@]}" "${WIN_HOST}" "Set-Location '${WIN_DIR}'; cargo nextest run -p '${package}' --no-fail-fast ${extra_args[*]@Q}; exit \$LASTEXITCODE"
}

declare -A results=()
if [[ "${hosts}" == "linux" || "${hosts}" == "all" ]]; then
    echo
    echo "== linux (${LINUX_HOST}) =="
    if run_linux; then results[linux]="pass"; else results[linux]="FAIL"; fi
fi
if [[ "${hosts}" == "windows" || "${hosts}" == "all" ]]; then
    echo
    echo "== windows (${WIN_HOST}) =="
    if run_windows; then results[windows]="pass"; else results[windows]="FAIL"; fi
fi

echo
echo "cross-check summary for ${package}:"
status=0
for os in linux windows; do
    [[ -n "${results[${os}]:-}" ]] || continue
    printf '  %-8s %s\n' "${os}" "${results[${os}]}"
    [[ "${results[${os}]}" == "pass" ]] || status=1
done
exit "${status}"
