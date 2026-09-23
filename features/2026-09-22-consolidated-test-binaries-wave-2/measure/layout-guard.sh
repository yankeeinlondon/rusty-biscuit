#!/usr/bin/env bash
# Red/green proof for one package's layout gate (plan SPP 4, rulings R9).
#
# Runs the gate three times and writes <out.md>:
#   1. with a planted top-level `tests/stray.rs`   → must fail;
#   2. with an undeclared `tests/l1/orphan.rs`     → must fail;
#   3. with both removed                           → must pass.
# Each planted file holds one test, so the failure is the gate's, not a build
# error. Planted files are removed on exit, even on failure.
#
# Usage (repository root): layout-guard.sh <package> <crate-dir> <out.md> [features]
set -euo pipefail

package="${1:?package}"
crate_dir="${2:?crate dir, e.g. tree-hugger/lib}"
out="${3:?output markdown}"
features="${4:-}"
feature_args=()
[[ -z "${features}" ]] || feature_args=(--features "${features}")

stray="${crate_dir}/tests/stray.rs"
orphan="${crate_dir}/tests/l1/orphan.rs"
for planted in "${stray}" "${orphan}"; do
    [[ ! -e "${planted}" ]] || { echo "${planted} already exists" >&2; exit 2; }
done
trap 'rm -f "${stray}" "${orphan}"' EXIT

gate() {
    cargo nextest run --locked -p "${package}" "${feature_args[@]}" --test l1 --color=never \
        --no-tests=fail every_test_source_is_compiled_by_a_declared_target 2>&1
}

record() {
    local label="$1" expected="$2" status output
    set +e
    output="$(gate)"
    status=$?
    set -e
    local verdict="as expected"
    if [[ "${expected}" == fail && ${status} -eq 0 ]] || [[ "${expected}" == pass && ${status} -ne 0 ]]; then
        verdict="UNEXPECTED"
    fi
    {
        echo "## ${label}"
        echo
        echo "Expected: ${expected}. Exit status: ${status} (${verdict})."
        echo
        echo '```text'
        printf '%s\n' "${output}" | grep -E 'violations|test sources outside|^tests/|stray|orphan|PASS|FAIL|Summary' | head -n 20
        echo '```'
        echo
    } >>"${out}"
    [[ "${verdict}" == "as expected" ]]
}

{
    echo "# Layout gate red/green — \`${package}\`"
    echo
    echo "Command: \`cargo nextest run -p ${package}${features:+ --features ${features}} --test l1 every_test_source_is_compiled_by_a_declared_target\`"
    echo "Revision: \`$(git rev-parse --short HEAD)\` plus the uncommitted move. Script: \`measure/layout-guard.sh\`."
    echo
} >"${out}"

planted_test='#[test]
fn planted() {}'
printf '%s\n' "${planted_test}" >"${stray}"
record "1. Planted stray root \`tests/stray.rs\`" fail
rm -f "${stray}"

printf '%s\n' "${planted_test}" >"${orphan}"
record "2. Undeclared module \`tests/l1/orphan.rs\`" fail
rm -f "${orphan}"

record "3. Both removed" pass
cat "${out}"
