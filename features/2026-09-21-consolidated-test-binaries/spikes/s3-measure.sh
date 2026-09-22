#!/usr/bin/env bash
# Spike S3 / R6 measurement series for the claudine-cli pilot.
#
# The SAME script produces the before series (Phase 1, unmigrated tree) and
# the after series (Phase 3, migrated tree); only <series> and the checkout
# differ. Do not edit the commands between the two series — record any change
# as a protocol deviation in measurements.md instead.
#
# Usage (repository root of the checkout being measured):
#   s3-measure.sh <series-label> <out-dir> [trials]
#
# Environment knobs (defaults are the R6 protocol):
#   S3_FEATURES   feature set for every build   (default: terminal-tests)
#   S3_EDIT_FILE  file touched per edit trial    (default: resolved from S3_EDIT_TEST)
#   S3_EDIT_TEST  positional test-name filter    (default: handle_rejects_present_non_absolute_agent_cwd)
set -euo pipefail

series="${1:?series label}"
out_dir="${2:?out dir}"
trials="${3:-5}"
features="${S3_FEATURES:-terminal-tests}"
edit_test="${S3_EDIT_TEST:-handle_rejects_present_non_absolute_agent_cwd}"
package="claudine-cli"

repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"
mkdir -p "${out_dir}"
unset RUSTC_WRAPPER CARGO_BUILD_RUSTC_WRAPPER BISCUIT_TEST_FILTER BISCUIT_L1_INCLUDE_SLOW

# The test moves between series (tests/agent_cwd.rs → tests/l1/agent_cwd.rs);
# locate it by content so the recipe itself never changes.
edit_file="${S3_EDIT_FILE:-$(grep -rl --include='*.rs' "fn ${edit_test}(" claudine/cli/tests | head -n 1)}"
[[ -n "${edit_file}" ]] || { echo "cannot find fn ${edit_test}" >&2; exit 1; }

target_dir="$(mktemp -d "${TMPDIR:-/tmp}/s3-${series}-target.XXXXXX")"
filter="$(just _tier_filter L1 "${package}")"
log="${out_dir}/${series}.log"
: >"${log}"

snapshot() {
    {
        echo "## snapshot: $1 ($(date -u +%FT%TZ))"
        echo "uptime: $(uptime)"
        sniff cpu --json 2>/dev/null || true
        sniff memory --json 2>/dev/null || true
    } >>"${out_dir}/${series}-host.txt"
}

timed() {
    # /usr/bin/time -l reports wall time and the maximum resident set size of
    # the largest waited-for descendant (rustc / the linker), in bytes.
    local label="$1"; shift
    local tfile="${out_dir}/${series}-${label}.time"
    /usr/bin/time -l "$@" >>"${log}" 2>"${tfile}.stderr" || { cat "${tfile}.stderr" >&2; return 1; }
    awk '/real/ {print "wall_seconds=" $1} /maximum resident set size/ {print "peak_rss_bytes=" $1}' "${tfile}.stderr" >"${tfile}"
    echo "${label}: $(tr '\n' ' ' <"${tfile}")"
}

{
    echo "series=${series}"
    echo "revision=$(git rev-parse HEAD)"
    echo "dirty=$(git status --porcelain -- claudine/cli | wc -l | tr -d ' ')"
    echo "toolchain=$(rustc -V)"
    echo "cargo=$(cargo -V)"
    echo "nextest=$(cargo nextest --version | head -n 1)"
    echo "host=$(rustc -vV | awk '/^host:/ {print $2}')"
    echo "profile=test (dev)"
    echo "features=${features}"
    echo "jobs=$(sysctl -n hw.ncpu 2>/dev/null || nproc)"
    echo "linker=default ($(cc --version 2>/dev/null | head -n 1))"
    echo "rustc_wrapper=${RUSTC_WRAPPER:-unset}"
    echo "target_dir=${target_dir} (fresh)"
    echo "edit_file=${edit_file}"
    echo "edit_test=${edit_test}"
    echo "l1_filter=${filter}"
} >"${out_dir}/${series}-conditions.txt"

snapshot "start"

# 1. Clean test build in a fresh target dir (what a producer does).
timed clean-build cargo test --no-run --locked -p "${package}" --features "${features}" \
    --target-dir "${target_dir}" --color=never

# 2. Produced test targets and their on-disk size (this package's executables only).
cargo test --no-run --locked -p "${package}" --features "${features}" --target-dir "${target_dir}" \
    --color=never --message-format json 2>/dev/null \
    | python3 -c '
import json, os, sys
count = size = 0
for line in sys.stdin:
    try:
        msg = json.loads(line)
    except ValueError:
        continue
    if msg.get("reason") != "compiler-artifact" or not msg.get("executable"):
        continue
    if msg["target"]["kind"] != ["test"] or not msg["package_id"].split("#")[-1].startswith("claudine-cli@"):
        continue
    count += 1
    size += os.path.getsize(msg["executable"])
print(f"test_targets={count}")
print(f"test_executable_bytes={size}")
' >"${out_dir}/${series}-targets.txt"
echo "targets: $(tr '\n' ' ' <"${out_dir}/${series}-targets.txt")"
echo "target_dir_bytes=$(/usr/bin/du -sk "${target_dir}" | awk '{print $1 * 1024}')" >>"${out_dir}/${series}-targets.txt"

# 3. Warm edit-to-one-test loop: one warm-up, then <trials> timed trials.
# `touch` changes no bytes, so every trial rebuilds exactly the target that
# owns the file and nothing else.
run_one=(cargo nextest run --locked -p "${package}" --features "${features}" --target-dir "${target_dir}"
    --color=never -E "${filter}" --no-tests=fail "${edit_test}")
touch "${edit_file}"
timed edit-warmup "${run_one[@]}"
for trial in $(seq 1 "${trials}"); do
    touch "${edit_file}"
    timed "edit-trial-${trial}" "${run_one[@]}"
done

snapshot "end"
echo "done; artifacts in ${out_dir}; remove ${target_dir} when the series is recorded"
