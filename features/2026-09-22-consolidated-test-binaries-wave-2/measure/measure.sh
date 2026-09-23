#!/usr/bin/env bash
# Lightweight before/after observation for one package (rulings R8, plan SPP 1
# and 8). The SAME command runs against the base worktree (before) and the
# migrated tree (after); only the checkout and the series label differ.
#
# It records two things into <out-dir>/<package>-<series>.txt:
#   1. a clean `cargo test --no-run --locked` into a fresh target dir, with the
#      package's test executables counted and their bytes summed; and
#   2. one warm edit-to-one-module observation: touch the file defining
#      <edit-test> (located by content, so it follows the move), then
#      `cargo nextest run -p <pkg> -E "$(just _tier_filter L1 <pkg>)" <edit-test>`.
#      One untimed warm-up precedes the timed run.
#
# kache is off: `unset RUSTC_WRAPPER` does not override `~/.cargo/config.toml`,
# only an explicitly empty value does, and kache's cc shims leave PATH too
# (`os` skill, build-hosts.md).
#
# Usage (from the root of the checkout being measured):
#   measure.sh <package> <tests-dir> <series> <out-dir> <edit-test> [features]
set -euo pipefail

package="${1:?package}"
tests_dir="${2:?tests dir, e.g. tree-hugger/lib/tests}"
series="${3:?series label (before|after)}"
out_dir="${4:?out dir}"
edit_test="${5:?positional test-name filter}"
features="${6:-}"

unset CARGO_BUILD_RUSTC_WRAPPER BISCUIT_TEST_FILTER BISCUIT_L1_INCLUDE_SLOW
export RUSTC_WRAPPER=""
PATH="$(printf '%s' "${PATH}" | tr ':' '\n' | grep -v '/kache/shims' | paste -sd: -)"
export PATH

mkdir -p "${out_dir}"
out="${out_dir}/${package}-${series}.txt"
target_dir="$(mktemp -d "${TMPDIR:-/tmp}/w2-measure-${package}-${series}.XXXXXX")"
feature_args=()
[[ -z "${features}" ]] || feature_args=(--features "${features}")

edit_file="$(grep -rl --include='*.rs' "fn ${edit_test}(" "${tests_dir}" | head -n 1)"
[[ -n "${edit_file}" ]] || { echo "cannot find fn ${edit_test} under ${tests_dir}" >&2; exit 1; }

{
    echo "package: ${package}"
    echo "series: ${series}"
    echo "revision: $(git rev-parse --short HEAD) (dirty paths under ${tests_dir%/tests}: $(git status --porcelain -- "${tests_dir%/tests}" | wc -l | tr -d ' '))"
    echo "features: ${features:-none}"
    echo "rustc: $(rustc --version)"
    echo "nextest: $(cargo nextest --version 2>/dev/null | head -n 1)"
    echo "wrapper: RUSTC_WRAPPER='' (kache off)"
    echo "load at start: $(uptime | sed 's/.*load averages*: //')"
} >"${out}"

start=$(date +%s)
cargo test --no-run --locked -p "${package}" "${feature_args[@]}" --target-dir "${target_dir}" \
    --message-format=json 2>/dev/null |
    python3 -c '
import json, os, sys
package = sys.argv[1]
count = 0
total = 0
for line in sys.stdin:
    try:
        message = json.loads(line)
    except ValueError:
        continue
    if message.get("reason") != "compiler-artifact" or message["target"]["kind"] != ["test"]:
        continue
    # Cargo drops the name from a package ID whose directory has the same name
    # (…/darkmatter/dmls#0.1.0), so match both spellings.
    package_id = message["package_id"]
    if "#" + package + "@" not in package_id and not package_id.split("#")[0].endswith("/" + package):
        continue
    executable = message.get("executable")
    if executable:
        count += 1
        total += os.path.getsize(executable)
print(f"test executables: {count}")
print(f"test executable bytes: {total}")
' "${package}" >>"${out}"
echo "clean test build wall: $(( $(date +%s) - start )) s" >>"${out}"

filter="$(just _tier_filter L1 "${package}")"
run_one=(cargo nextest run --locked -p "${package}" "${feature_args[@]}" --target-dir "${target_dir}"
    --color=never -E "${filter}" --no-tests=fail "${edit_test}")
touch "${edit_file}"
"${run_one[@]}" >/dev/null 2>&1
touch "${edit_file}"
start_ns=$(python3 -c 'import time; print(time.monotonic_ns())')
"${run_one[@]}" >"${out_dir}/${package}-${series}-edit.log" 2>&1
end_ns=$(python3 -c 'import time; print(time.monotonic_ns())')
{
    echo "edit file: ${edit_file}"
    echo "edit command: ${run_one[*]}"
    echo "warm edit-to-one-module wall: $(python3 -c "print(f'{(${end_ns} - ${start_ns}) / 1e9:.2f}')") s"
    echo "load at end: $(uptime | sed 's/.*load averages*: //')"
} >>"${out}"
rm -rf "${target_dir}"
cat "${out}"
