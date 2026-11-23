#!/usr/bin/env bash
# ensure-repo-cli.sh - keep or (re)install one of the monorepo's own CLIs for
# `just init`.
#
# Usage: ensure-repo-cli.sh [--rebuild] <binary> <area>
#
# An existing `<binary>` on PATH is kept when it is younger than
# MAX_AGE_DAYS: a host may carry a build newer than this checkout, and every
# rebuild costs minutes and gigabytes of target/. Any binary of that name
# counts, whoever built it. An older binary, a missing one, one that cannot
# start, or `--rebuild` runs `just install` in `<area>`. Every decision is reported on stderr.

set -euo pipefail

MAX_AGE_DAYS="${ENSURE_REPO_CLI_MAX_AGE_DAYS:-30}"

rebuild=false
if [[ "${1:-}" == "--rebuild" ]]; then
    rebuild=true
    shift
fi
if (( $# != 2 )); then
    echo "usage: ensure-repo-cli.sh [--rebuild] <binary> <area>" >&2
    exit 2
fi
binary="$1"
area="$2"

# Portable mtime (epoch seconds) of the file a PATH entry resolves to.
mtime() {
    case "$(uname -s)" in
        Darwin) stat -L -f %m "$1" ;;
        *)      stat -L -c %Y "$1" ;;
    esac
}

format_epoch() {
    case "$(uname -s)" in
        Darwin) date -r "$1" '+%Y-%m-%d %H:%M' ;;
        *)      date -d "@$1" '+%Y-%m-%d %H:%M' ;;
    esac
}

install_area() {
    (cd "$area" && just install)
}

if [[ "$rebuild" == true ]]; then
    echo "$binary: --rebuild requested; installing from $area/" >&2
    install_area
    exit 0
fi

if ! found="$(command -v "$binary" 2>/dev/null)" || [[ "$found" != */* ]]; then
    echo "$binary: not found on PATH; installing from $area/" >&2
    install_area
    exit 0
fi

# A recent binary can still be unable to start: one linked against a shared
# library the host has since removed (a Homebrew `pcre2` uninstall, say) is
# killed by the loader before it parses any argument. 127 is the Linux and
# Git Bash loader failure, above 128 a signal (dyld aborts with SIGABRT); a
# CLI that merely rejects `--version` exits with a usage code below either.
# bash reports a signalled child ("Abort trap: 6") on the waiting shell's own
# stderr, so the probe runs in a command substitution whose stderr is closed.
status="$( { "$found" --version >/dev/null 2>&1; echo $?; } 2>/dev/null )"
if (( status == 127 || status > 128 )); then
    echo "$binary: $found does not start (exit $status); reinstalling from $area/" >&2
    install_area
    exit 0
fi

built="$(mtime "$found")"
age_days=$(( ($(date +%s) - built) / 86400 ))
when="$(format_epoch "$built")"
days="$age_days days"
(( age_days == 1 )) && days="1 day"

if (( age_days > MAX_AGE_DAYS )); then
    echo "$binary: $found was built $when ($days ago), older than $MAX_AGE_DAYS days; reinstalling from $area/" >&2
    install_area
else
    echo "$binary: keeping $found (built $when, $days ago); run 'just init --rebuild' to replace it" >&2
fi
