#!/usr/bin/env bash
# ensure-repo-cli-tests.sh - Level 1 tests for the keep-or-reinstall rule in
# `ensure-repo-cli.sh`, which `just init` applies to each monorepo CLI.
#
# Fake binaries with explicit ages stand on PATH, and a stub `just` records
# which area it was asked to install instead of building. Exits 1 on any
# failure.

set -euo pipefail

SCRIPT_DIR="$(CDPATH="" cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
SUBJECT="${SCRIPT_DIR}/ensure-repo-cli.sh"
PASS=0
FAIL=0
FAILED_TESTS=()

scratch="$(mktemp -d)"
trap 'rm -rf "${scratch}"' EXIT

bin_dir="${scratch}/bin"
mkdir -p "${bin_dir}" "${scratch}/area"
log="${scratch}/installs.log"
printf '#!/usr/bin/env bash\necho "install $(basename "$PWD")" >> "%s"\n' "${log}" > "${bin_dir}/just"
chmod +x "${bin_dir}/just"

# make_binary <name> <days old>
make_binary() {
    printf '#!/usr/bin/env bash\n' > "${bin_dir}/$1"
    chmod +x "${bin_dir}/$1"
    local stamp
    case "$(uname -s)" in
        Darwin) stamp="$(date -v-"$2"d '+%Y%m%d%H%M')" ;;
        *)      stamp="$(date -d "$2 days ago" '+%Y%m%d%H%M')" ;;
    esac
    touch -t "${stamp}" "${bin_dir}/$1"
}

# check <name> <expect: install|keep> <stderr pattern> -- <script args...>
check() {
    local name="$1" expect="$2" pattern="$3"
    shift 4
    : > "${log}"
    local stderr
    stderr="$(cd "${scratch}" && PATH="${bin_dir}:/usr/bin:/bin" bash "${SUBJECT}" "$@" 2>&1 >/dev/null)" || {
        FAIL=$((FAIL + 1)); FAILED_TESTS+=("${name}: exited non-zero: ${stderr}"); return
    }
    local installed=keep
    [[ -s "${log}" ]] && installed=install
    if [[ "${installed}" != "${expect}" ]]; then
        FAIL=$((FAIL + 1)); FAILED_TESTS+=("${name}: expected ${expect}, got ${installed}")
    elif ! grep -qE -e "${pattern}" <<< "${stderr}"; then
        FAIL=$((FAIL + 1)); FAILED_TESTS+=("${name}: stderr did not match '${pattern}': ${stderr}")
    else
        PASS=$((PASS + 1))
    fi
}

make_binary fresh 2
make_binary edge 30
make_binary stale 31

check "missing binary installs"        install "not found on PATH"            -- absent area
check "fresh binary is kept"           keep    "keeping .*fresh \(built .*2 days ago\)" -- fresh area
check "30-day-old binary is kept"      keep    "30 days ago"                  -- edge area
check "31-day-old binary reinstalls"   install "31 days ago\), older than 30" -- stale area
check "--rebuild replaces fresh"       install "--rebuild requested"          -- --rebuild fresh area

if PATH="${bin_dir}:/usr/bin:/bin" bash "${SUBJECT}" only-one >/dev/null 2>&1; then
    FAIL=$((FAIL + 1)); FAILED_TESTS+=("wrong arity: expected a usage error")
else
    PASS=$((PASS + 1))
fi

echo "ensure-repo-cli: ${PASS} passed, ${FAIL} failed"
for failed in "${FAILED_TESTS[@]}"; do
    echo "  FAIL ${failed}" >&2
done
(( FAIL == 0 ))
