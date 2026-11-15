# On native Windows, the bin directory of the bash that runs recipes (Cygwin's
# `bin`, or Git for Windows' / MSYS2's `usr\bin`) goes first on the PATH that
# `just` hands to every recipe. A non-login bash otherwise keeps the Windows
# PATH order, where WinGet links (uutils coreutils), scoop shims, and System32
# can sort earlier and answer for `uname`, `env`, `sort`, `find`, or `cat`
# (uutils `uname -s` prints `Windows_NT`, which no recipe recognizes).
#
# Done in `just` rather than in a BASH_ENV hook because Cygwin's bash skips
# BASH_ENV for a `bash -c` started under SSH (it sources the bashrc files
# instead), which is every linewise recipe on a remote rig.
#
# What this does not reach: `just` resolves the bare `env` at the front of
# `set shell` from the PATH it inherited, so a foreign `env.exe` may still
# launch the recipe shell. That is harmless: it only sets one variable and
# starts `bash`, which it finds through this PATH. Nor does `export` reach the
# shells that evaluate load-time backticks; those stay limited to the tools
# `check-canonical` allows.
#
# `/proc/$$/winexename` is the Windows path of that bash, read with a builtin
# (`read` fails at its unterminated last line but still assigns, hence
# `|| true`). Inside WSL (the "accidental WSL" case `_ensure-native-bash`
# rejects) the file does not exist, the value is empty, and PATH is left alone.
# The backtick holds no backslash because Cygwin re-parses the command line a
# native program such as `just` passes it, collapsing `\\`; `?` stands in for
# the separator before `bash.exe`.
_recipe_bash_bin := if os_family() == "windows" { `exe=""; if [[ -r /proc/$$/winexename ]]; then read -r exe < /proc/$$/winexename || true; fi; printf '%s' "${exe%?bash.exe}"` } else { "" }

export PATH := if _recipe_bash_bin == "" { env("PATH") } else { _recipe_bash_bin + ";" + trim_start_match(env("PATH"), _recipe_bash_bin + ";") }

# Shell for a test recipe to `eval` as its first step, so the run reports its
# wall-clock duration as the last line it prints:
#
#     eval "$(just _test_run_scope)"
#
# Only the outermost recipe of a run owns the report; a recipe nested under one
# gets the no-op, so a run prints one line however many recipes it chains. The
# report is an EXIT trap so a failed or interrupted run reports too and keeps
# its exit code. A recipe that sets its own EXIT trap replaces this one and must
# call `_test_run_finish` from it.
#
# Silent inside a test and on GitHub Actions, where nobody reads the line and
# recipe fixtures would otherwise see it.
_test_run_scope:
    #!/usr/bin/env bash
    set -euo pipefail
    if [[ -n "${BISCUIT_TEST_RUN_STARTED:-}" || -n "${NEXTEST_RUN_ID:-}" || -n "${GITHUB_ACTIONS:-}" ]]; then
        echo '_test_run_finish() { :; }'
        exit 0
    fi
    cat <<'SCOPE'
    BISCUIT_TEST_RUN_STARTED="$(date +%s)"
    export BISCUIT_TEST_RUN_STARTED
    _test_run_finish() {
        local elapsed minutes seconds
        elapsed=$(( $(date +%s) - BISCUIT_TEST_RUN_STARTED ))
        minutes=$(( elapsed / 60 ))
        seconds=$(( elapsed % 60 ))
        printf '🕖 this test run took %d minute%s, %d second%s\n' \
            "${minutes}" "$( (( minutes == 1 )) || echo s )" \
            "${seconds}" "$( (( seconds == 1 )) || echo s )"
    }
    trap _test_run_finish EXIT
    SCOPE
