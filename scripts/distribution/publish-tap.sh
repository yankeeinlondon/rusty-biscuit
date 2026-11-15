#!/usr/bin/env bash
# Publish one release's Homebrew formula to the tap (plan rule 16 of
# `2026-10-28-publish`). All logic, flags, and exit codes are tap.py's:
#
#   publish-tap.sh --selection <preflight.json> --assets-dir <dir> [--dry-run] [--report <file>]
#
# Requires HOMEBREW_TAP_TOKEN and HOMEBREW_TAP_AUTHOR; never GITHUB_TOKEN.
set -euo pipefail

exec "${PYTHON:-python3}" "$(dirname "${BASH_SOURCE[0]}")/tap.py" publish "$@"
