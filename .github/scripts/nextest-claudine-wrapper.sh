#!/usr/bin/env bash
set -euo pipefail

if [[ -n "${BISCUIT_CLAUDINE_TEST_BIN:-}" ]]; then
  if [[ ! -x "$BISCUIT_CLAUDINE_TEST_BIN" ]]; then
    printf 'configured Claudine test binary is not executable: %s\n' \
      "$BISCUIT_CLAUDINE_TEST_BIN" >&2
    exit 1
  fi

  export CARGO_BIN_EXE_claudine="$BISCUIT_CLAUDINE_TEST_BIN"
  export NEXTEST_BIN_EXE_claudine="$BISCUIT_CLAUDINE_TEST_BIN"
fi

exec "$@"
