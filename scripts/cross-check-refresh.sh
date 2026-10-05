#!/usr/bin/env bash
# Retire the build-host clones `cross-check` no longer needs, and report the
# space each host gets back. Runs nothing else: no fetch, no build, no test.
#
# The rules are exactly the ones every `cross-check` run applies before its
# legs start (see `cross-check-hosts.sh`): a clone is retired when its lock is
# not held and either
#
#   - it is this host's clone and its worktree no longer exists here, or
#   - it has had no cross-check run in CROSS_CHECK_MAX_AGE_DAYS (default 3)
#     days, whatever host created it.
#
# A run only prunes the hosts it selects; this checks every declared host
# (BUILD_LINUX, BUILD_WIN, BUILD_WSL, BUILD_MACOS) except the local OS, so a
# host you have not cross-checked recently is still cleaned.
#
# For each host it reports the state before (free and total space on the
# coding dir's volume, and every clone with its size and days unused) and
# after (each clone removed, its size, and the free space reclaimed). Unlike a
# run, it deletes synchronously so the after figure is real; sizing large
# clones takes minutes, so on a terminal a spinner shows which host is being
# worked on until its report is ready.
#
# Usage:
#   scripts/cross-check-refresh.sh [--os linux|windows|wsl|macos|all] [--dry-run]
#
#   --dry-run   report what would be removed and the space it would free,
#               without removing anything
set -euo pipefail

# shellcheck source=cross-check-hosts.sh
source "$(dirname "${BASH_SOURCE[0]}")/cross-check-hosts.sh"

os_sel="all"
dry_run=0
while (($# > 0)); do
    case "$1" in
        --os) shift; os_sel="${1:?--os needs linux|windows|wsl|macos|all}" ;;
        --os=*) os_sel="${1#--os=}" ;;
        --dry-run) dry_run=1 ;;
        -h | --help)
            awk 'NR > 1 && !/^#/ { exit } NR > 1 { sub(/^# ?/, ""); print }' "$0"
            exit 0
            ;;
        *)
            echo "cross-check-refresh: unknown argument $1 (supported: --os, --dry-run)" >&2
            exit 2
            ;;
    esac
    shift
done

cd "$(git rev-parse --show-toplevel)"
calling_host_identity
select_hosts cross-check-refresh "${os_sel}"

# A spinner on its own line while a host works, only when stdout is a
# terminal. It is erased with a carriage return and spaces (no escape codes)
# before that host's report prints.
spinner_pid=""
spinner_line=""
start_spinner() {
    [[ -t 1 ]] || return 0
    spinner_line="$1"
    (
        trap 'exit 0' TERM
        while :; do
            for frame in '|' '/' '-' '\'; do
                printf '\r  %s %s' "${frame}" "${spinner_line}"
                sleep 0.1
            done
        done
    ) &
    spinner_pid=$!
}
stop_spinner() {
    [[ -n "${spinner_pid}" ]] || return 0
    kill "${spinner_pid}" 2> /dev/null || true
    wait "${spinner_pid}" 2> /dev/null || true
    spinner_pid=""
    printf '\r%*s\r' "$((${#spinner_line} + 4))" ""
}
trap 'stop_spinner' EXIT
trap 'stop_spinner; exit 130' INT TERM

title="Cross Build Refresh"
echo
echo "${title}"
printf '%s\n' "${title//?/=}"

failed=0
report="$(mktemp)"
problems="$(mktemp)"
trap 'stop_spinner; rm -f "${report}" "${problems}"' EXIT
for os in "${ORDER[@]}"; do
    [[ -n "${want[${os}]:-}" ]] || continue
    host="${HOST[${os}]}"
    echo
    start_spinner "${os} (${host}): sizing clones and $([[ "${dry_run}" == 1 ]] && echo "checking what to remove" || echo "removing what is no longer needed")..."
    status=0
    prune_host cross-check-refresh "${os}" refresh "" "${dry_run}" > "${report}" 2> "${problems}" || status=$?
    stop_spinner
    title="${os} (${host})"
    echo "${title}"
    printf '%s\n' "${title//?/-}"
    cat "${report}"
    cat "${problems}" >&2
    if ((status != 0)); then
        echo "  could not reach ${host} or run the refresh there" >&2
        failed=1
    fi
done
echo
exit "${failed}"
