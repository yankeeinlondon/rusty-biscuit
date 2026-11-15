#!/usr/bin/env bash
# Run a command with PYTHON naming a temporary interpreter that holds exactly
# scripts/ci/requirements-distribution.txt, then remove the environment.
#
#   pinned-python.sh <command> [args...]
#
# The distribution scripts import PyYAML and semver at those pinned versions;
# a host interpreter without them, or with other versions, would test or build
# with different code than CI. Run from the repository root.
set -euo pipefail

if [ "$#" -eq 0 ]; then
    printf 'usage: pinned-python.sh <command> [args...]\n' >&2
    exit 2
fi
host_python="$(command -v python3 || true)"
if [ -z "${host_python}" ]; then
    printf 'pinned-python.sh: python3 is required and is not on PATH\n' >&2
    exit 2
fi
venv="$(mktemp -d "${TMPDIR:-/tmp}/distribution-venv.XXXXXX")"
trap 'rm -rf "${venv}"' EXIT
"${host_python}" -m venv "${venv}"
PYTHON="${venv}/bin/python"
# A Windows venv puts its interpreter under Scripts/.
[ -x "${PYTHON}" ] || PYTHON="${venv}/Scripts/python.exe"
"${PYTHON}" -m pip install --quiet --disable-pip-version-check --no-input \
    -r scripts/ci/requirements-distribution.txt
export PYTHON
"$@"
