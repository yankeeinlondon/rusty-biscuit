#!/usr/bin/env bash
# cross-check-hosts-tests.sh - Level 1 tests for the clone-pruning rules in
# `cross-check-hosts.sh` that `cross-check.sh` and `cross-check-refresh.sh`
# share.
#
# Runs the real Unix host script through `prune_host`, with SSH replaced by a
# stub that executes it locally against a temporary coding dir (CODING_DIR).
# Clone ages are set explicitly. The PowerShell twin cannot run here; it is
# exercised on the Windows build host (`just cross-check-refresh --os windows
# --dry-run`). Exits 1 on any failure.

set -euo pipefail

SCRIPT_DIR="$(CDPATH="" cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PASS=0
FAIL=0
FAILED_TESTS=()

scratch="$(mktemp -d)"
trap 'rm -rf "${scratch}"' EXIT

# Stands in for `ssh <host> "bash -ls"`: drops the host and runs stdin here.
stub="${scratch}/fake-ssh"
printf '#!/usr/bin/env bash\nexec bash -s\n' > "${stub}"
chmod +x "${stub}"

# make_clone <name> <days since fetch | nofetch | bare> [dir days] [locked]
#   <n>      a clone whose last `git fetch` (FETCH_HEAD) was n days ago
#   nofetch  a clone that never fetched: the directory's own age decides
#   bare     not a clone at all (no `rusty-biscuit` child)
make_clone() {
    local dir="${CODING_DIR}/$1" kind="$2" dir_days="${3:-0}" locked="${4:-}"
    mkdir -p "${dir}"
    if [[ "${kind}" == "bare" ]]; then
        mkdir -p "${dir}/notes"
    else
        mkdir -p "${dir}/rusty-biscuit/target" "${dir}/rusty-biscuit/.git"
        head -c 4096 /dev/zero > "${dir}/rusty-biscuit/target/blob"
        if [[ "${kind}" != "nofetch" ]]; then
            echo fetched > "${dir}/rusty-biscuit/.git/FETCH_HEAD"
            age_path "${dir}/rusty-biscuit/.git/FETCH_HEAD" "${kind}"
        fi
    fi
    [[ -n "${locked}" ]] && mkdir -p "${dir}/.cross-check.lock"
    age_path "${dir}" "${dir_days}"
}

age_path() {
    python3 -c 'import os, sys, time; t = time.time() - float(sys.argv[2]) * 86400; os.utime(sys.argv[1], (t, t))' "$1" "$2"
}

# run_prune <mode> <own> <dry_run>: runs prune_host in a subshell with the
# shared library, the stub SSH, and the fixture identity; prints its output.
run_prune() {
    (
        # shellcheck source=cross-check-hosts.sh
        source "${SCRIPT_DIR}/cross-check-hosts.sh"
        SSH=("${stub}")
        HOST[linux]="fixture-host"
        origin_host="testhost"
        live_worktrees=(main live1 live2)
        prune_host test linux "$1" "$2" "$3"
        echo "PRUNED_CLONE_DIR=${PRUNED_CLONE_DIR}"
    ) 2>&1
}

check() {
    local name="$1" condition="$2"
    if eval "${condition}"; then
        PASS=$((PASS + 1))
    else
        FAIL=$((FAIL + 1))
        FAILED_TESTS+=("${name}")
        echo "FAIL: ${name}" >&2
        echo "  condition: ${condition}" >&2
        echo "  output:" >&2
        printf '%s\n' "${output}" | sed 's/^/    /' >&2
    fi
}

# The AFTER section of the last report.
after_section() { sed -n '/^AFTER/,$p' <<< "${output}"; }

fresh_fixture() {
    export CODING_DIR="${scratch}/coding-$1"
    rm -rf "${CODING_DIR}"
    mkdir -p "${CODING_DIR}"
    make_clone testhost--gone 0                   # stale: worktree not live
    make_clone testhost--live1 0 10               # fetched today, old directory: kept
    make_clone testhost--live2 5                  # live but unused 5 days: expired
    make_clone other--orphan 4                    # other calling host, old: expired
    make_clone other--recent 1                    # other calling host, recent: kept
    make_clone testhost--nofetch nofetch 4        # never fetched: directory age decides
    make_clone other--locked 9 0 locked           # old but locked: kept
    make_clone not--aclone bare 30                # not a clone: never touched
    mkdir -p "${CODING_DIR}/.cross-check-trash--elsewhere--x-1-2/deep"
}

# --- refresh, for real -------------------------------------------------------
fresh_fixture refresh
output="$(run_prune refresh "" 0)" || true
check "refresh: stale clone removed" '[[ ! -e "${CODING_DIR}/testhost--gone" ]] && grep -qE "^ +[0-9.]+ (MiB|GiB) +testhost--gone +worktree no longer exists$" <<< "$(after_section)"'
check "refresh: live clone unused 5 days expires" '[[ ! -e "${CODING_DIR}/testhost--live2" ]] && grep -qE "^ +[0-9.]+ (MiB|GiB) +testhost--live2 +unused 5 days$" <<< "$(after_section)"'
check "refresh: other host's old clone expires" '[[ ! -e "${CODING_DIR}/other--orphan" ]]'
check "refresh: directory age decides without a fetch" '[[ ! -e "${CODING_DIR}/testhost--nofetch" ]]'
check "refresh: a recent fetch outweighs an old directory" '[[ -d "${CODING_DIR}/testhost--live1" ]]'
check "refresh: recent clone of another host kept" '[[ -d "${CODING_DIR}/other--recent" ]]'
check "refresh: locked clone kept and reported" '[[ -d "${CODING_DIR}/other--locked" ]] && grep -q "^  kept other--locked: in use (its lock is held)$" <<< "${output}"'
check "refresh: non-clone directory untouched" '[[ -d "${CODING_DIR}/not--aclone" ]]'
check "refresh: trash deleted before reporting" '! ls -d "${CODING_DIR}"/.cross-check-trash--* > /dev/null 2>&1'
check "refresh: before state reports free space and every clone" 'grep -qE "^BEFORE  .* free of .* \([0-9]+% used\) in ${CODING_DIR}$" <<< "${output}" && [[ "$(grep -cE "^ +[0-9.]+ (MiB|GiB) +[0-9]+d  [a-z]+--" <<< "${output}")" -eq 7 ]] && grep -qE "^ +[0-9.]+ (MiB|GiB) +7 clones$" <<< "${output}"'
check "refresh: after state reports removals and reclaimed space" 'grep -q "^AFTER$" <<< "${output}" && grep -qE "^ +[0-9.]+ (MiB|GiB)  4 clones$" <<< "$(after_section)" && grep -qE "^  free space  .* -> .* \([0-9]+% used\)$" <<< "${output}" && grep -qE "^  reclaimed   " <<< "${output}"'
check "refresh: after state lists each removed clone with its size" '[[ "$(grep -cE "^ +[0-9.]+ (MiB|GiB)  [a-z]+--[a-z0-9]+ +(unused|worktree)" <<< "$(after_section)")" -eq 4 ]] && grep -qE "^  -{10}$" <<< "$(after_section)"'
check "refresh: no clone dir is claimed" 'grep -q "^PRUNED_CLONE_DIR=$" <<< "${output}"'

# --- refresh, dry run --------------------------------------------------------
fresh_fixture dry
output="$(run_prune refresh "" 1)" || true
check "dry run: nothing removed" '[[ -d "${CODING_DIR}/testhost--gone" && -d "${CODING_DIR}/testhost--live2" && -d "${CODING_DIR}/other--orphan" && -d "${CODING_DIR}/testhost--nofetch" ]]'
check "dry run: trash left alone" 'ls -d "${CODING_DIR}"/.cross-check-trash--* > /dev/null 2>&1'
check "dry run: reports what would go and the space" 'grep -q "^AFTER (dry run: nothing was removed)$" <<< "${output}" && grep -q "WOULD REMOVE" <<< "${output}" && grep -qE "^ +[0-9.]+ (MiB|GiB)  4 clones$" <<< "$(after_section)" && grep -qE "^  free space  .* -> about " <<< "${output}"'

# --- prepare (a cross-check run) ---------------------------------------------
fresh_fixture prepare
output="$(run_prune prepare testhost--live2 0)" || true
check "prepare: the run's own clone is never retired" '[[ -f "${CODING_DIR}/testhost--live2/rusty-biscuit/target/blob" ]] && ! grep -q "testhost--live2 (" <<< "${output}"'
check "prepare: own clone dir is returned" 'grep -q "^PRUNED_CLONE_DIR=${CODING_DIR}/testhost--live2$" <<< "${output}"'
check "prepare: same rules retire the others" '[[ ! -e "${CODING_DIR}/testhost--gone" ]] && grep -q "removing clone fixture-host:.*other--orphan" <<< "${output}"'
check "prepare: no sizing or volume report" '! grep -q " before: " <<< "${output}"'

# --- age threshold -----------------------------------------------------------
fresh_fixture threshold
output="$(CROSS_CHECK_MAX_AGE_DAYS=10 run_prune refresh "" 1)" || true
check "threshold: a longer age keeps 4- and 5-day clones" '! grep -q "other--orphan\|testhost--live2 (" <<< "$(grep "would remove" <<< "${output}")"'
output="$(CROSS_CHECK_MAX_AGE_DAYS=0 run_prune refresh "" 1 || true)"
check "threshold: zero is rejected" 'grep -q "CROSS_CHECK_MAX_AGE_DAYS must be a positive whole number" <<< "${output}"'

# --- Windows report ----------------------------------------------------------
# A Windows host's output: CRLF line endings and backslash paths. The removed
# clones' sizes must still be found, summed, and totaled.
win_output=$'coding: B:\\coding\r
volume-before: 17287243776 322014330880\r
clone: 124770000000 3 B:\\coding\\shazam--feat-schema-enhancement\r
clone: 92560000000 2 B:\\coding\\shazam--fix-magic-globs\r
clone: 6440000000 9 B:\\coding\\mac--fix-sniff\r
expired-would: B:\\coding\\mac--fix-sniff (9 days unused)\r
expired-would: B:\\coding\\shazam--feat-schema-enhancement (3 days unused)\r
volume-after: 17287243776\r'
output="$( (source "${SCRIPT_DIR}/cross-check-hosts.sh"; report_prune test win-host refresh 1 "${win_output}") 2>&1 )" || true
check "windows report: removed sizes found for backslash paths" 'grep -qE "^ +6\.0 GiB  mac--fix-sniff +unused 9 days$" <<< "$(after_section)" && grep -qE "^ +116\.2 GiB  shazam--feat-schema-enhancement +unused 3 days$" <<< "$(after_section)"'
check "windows report: removed sizes are totaled" 'grep -qE "^ +122\.2 GiB  2 clones$" <<< "$(after_section)" && grep -q "^  free space  16.1 GiB -> about 138.3 GiB (53% used)$" <<< "${output}"'
check "windows report: kept clones are listed only in the before table" '[[ "$(grep -c "fix-magic-globs" <<< "${output}")" -eq 1 ]]'

# --- nothing to remove -------------------------------------------------------
output="$( (source "${SCRIPT_DIR}/cross-check-hosts.sh"; report_prune test quiet-host refresh 0 $'coding: /c\nvolume-before: 1073741824 2147483648\nclone: 1048576 0 /c/me--live\nvolume-after: 1073741824') 2>&1 )" || true
check "nothing to remove: one line, no free-space or reclaim noise" 'grep -q "^  nothing to remove; 1.0 GiB free (50% used)$" <<< "${output}" && ! grep -qE "free space|reclaimed" <<< "${output}"'

# --- host selection ----------------------------------------------------------
output="$( (source "${SCRIPT_DIR}/cross-check-hosts.sh"; HOST=("" "" "" ""); select_hosts test all) 2>&1 )" && status=0 || status=$?
check "hosts: none declared is an error, not a silent no-op" '[[ ${status} -eq 2 ]] && grep -q "no build host for an OS other than this one" <<< "${output}"'

echo
echo "cross-check-hosts tests: ${PASS} passed, ${FAIL} failed"
if ((FAIL > 0)); then
    printf '  - %s\n' "${FAILED_TESTS[@]}"
    exit 1
fi
