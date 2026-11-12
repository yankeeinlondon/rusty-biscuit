#!/usr/bin/env bash
# Entry point for `just distribution-verify`: verify locally built
# distribution outputs on this host without publishing anything.
#
#   verify.sh <out-dir> --artifacts DIR [--package P] [--version V] [--target T]...
#       1. `verify_binstall.py fixture-check` with the pinned cargo-binstall
#          1.26.2 (CARGO_BINSTALL, then cargo-binstall on PATH, if either
#          reports exactly 1.26.2; otherwise the upstream release asset is
#          downloaded and checked against the SHA-256 pinned in
#          `.github/actions/setup-dist/action.yml`).
#       2. `verify_runtime.py run` over DIR: the archives, `.sha256` files,
#          `sha256.sum`, and hardened `<package>-installer.sh` that the release
#          workflow's Package job produces. Package defaults to
#          biscuit-hash-cli; the version defaults to the installer's
#          APP_VERSION; targets default to all four.
#       3. `<out-dir>/report.json` and `<out-dir>/README.md` combine both.
#
# DIR is required: this script never builds. Produce it the way the workflow
# does (see docs/distribution.md and the Phase 2 evidence README): `distribution.py
# stage`, `build.sh build` (macOS) or `build.sh container` (Linux) per target,
# then `dist build --artifacts=global`, `installers.py harden`, and
# `installers.py check-sums` over the collected files.
#
# Pending infrastructure and disabled channels are reported, never counted as
# passes. Exit status: 0 when nothing failed, 1 on a failed check, 2 on usage
# or unusable input. PYTHON names an interpreter with the pinned requirements
# (default python3). Non-interactive: no prompts, stdin closed for every tool.

set -euo pipefail
unset CDPATH

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)
PYTHON=${PYTHON:-python3}
BINSTALL_VERSION=1.26.2
ACTION="$SCRIPT_DIR/../../.github/actions/setup-dist/action.yml"

die() { printf 'verify.sh: error: %s\n' "$1" >&2; exit "${2:-1}"; }
note() { printf 'verify.sh: %s\n' "$*" >&2; }

usage() {
    sed -n '/^#   verify.sh/,/^#$/p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//' >&2
    exit 2
}

# The asset name and SHA-256 the setup action pins for this host, read from
# the action itself so the two can never disagree.
pinned_asset() {
    local key
    key="$(uname -s)-$(uname -m)"
    awk -v key="$key" '
        /name: Install the pinned cargo-binstall/ { step = 1 }
        step && $1 == key ")" && $2 ~ /^asset=/ { found = 1; sub(/^asset=/, "", $2); asset = $2; next }
        found && /sha=/ { sub(/.*sha=/, ""); sub(/ .*/, ""); print asset, $0; exit }
    ' "$ACTION"
}

binstall_version() { "$1" -V </dev/null 2>/dev/null || true; }

# Prints the path of a cargo-binstall that reports exactly the pinned version,
# downloading it into the directory $1 when no local copy qualifies.
pinned_binstall() {
    local candidate dir=$1 asset sha
    for candidate in "${CARGO_BINSTALL:-}" "$(command -v cargo-binstall || true)"; do
        if [ -n "$candidate" ] && [ "$(binstall_version "$candidate")" = "$BINSTALL_VERSION" ]; then
            echo "$candidate"
            return
        fi
    done
    read -r asset sha < <(pinned_asset) || true
    [ -n "${asset:-}" ] && [ -n "${sha:-}" ] ||
        die "no cargo-binstall $BINSTALL_VERSION pin for $(uname -s) $(uname -m) in $ACTION"
    note "downloading cargo-binstall $BINSTALL_VERSION ($asset)"
    curl --proto '=https' --tlsv1.2 -fsSL --retry 3 --max-time 300 -o "$dir/$asset" \
        "https://github.com/cargo-bins/cargo-binstall/releases/download/v$BINSTALL_VERSION/$asset" </dev/null ||
        die "cannot download cargo-binstall $BINSTALL_VERSION; it is required, not optional"
    (cd "$dir" && echo "$sha  $asset" | shasum -a 256 -c - >&2) ||
        die "cargo-binstall $asset does not match the SHA-256 pinned in $ACTION"
    case "$asset" in
        *.zip) unzip -q -o "$dir/$asset" cargo-binstall -d "$dir" </dev/null ;;
        *) tar -xzf "$dir/$asset" -C "$dir" cargo-binstall ;;
    esac
    [ "$(binstall_version "$dir/cargo-binstall")" = "$BINSTALL_VERSION" ] ||
        die "the downloaded cargo-binstall does not report $BINSTALL_VERSION"
    echo "$dir/cargo-binstall"
}

main() {
    [ $# -ge 1 ] || usage
    case "$1" in -h | --help) usage ;; esac
    local out=$1 artifacts="" package=biscuit-hash-cli version="" command binstall status=0
    local -a targets=()
    shift
    while [ $# -gt 0 ]; do
        case "$1" in
            --artifacts) artifacts=${2:?--artifacts needs a directory}; shift 2 ;;
            --package) package=${2:?--package needs a name}; shift 2 ;;
            --version) version=${2:?--version needs a version}; shift 2 ;;
            --target) targets+=(--target "${2:?--target needs a target}"); shift 2 ;;
            *) usage ;;
        esac
    done
    [ -n "$artifacts" ] || die "no --artifacts directory. This script does not build; produce the Package job's outputs first (see the header of $0)" 2
    [ -d "$artifacts" ] || die "--artifacts $artifacts is not a directory" 2
    command=$("$PYTHON" - "$SCRIPT_DIR/../ci" "$package" <<'PY'
import sys
sys.path.insert(0, sys.argv[1])
import distribution
print(distribution.ROSTER.get(sys.argv[2], ""))
PY
)
    [ -n "$command" ] || die "$package ships no distributed command" 2
    if [ -z "$version" ]; then
        version=$(sed -n 's/^APP_VERSION="\(.*\)"$/\1/p' "$artifacts/$package-installer.sh" 2>/dev/null || true)
        [ -n "$version" ] || die "cannot read APP_VERSION from $artifacts/$package-installer.sh; pass --version" 2
    fi
    mkdir -p "$out"

    BINSTALL_DIR=$(mktemp -d "${TMPDIR:-/tmp}/verify-binstall.XXXXXX")
    trap 'rm -rf "$BINSTALL_DIR"' EXIT
    binstall=$(pinned_binstall "$BINSTALL_DIR")
    note "cargo-binstall: $binstall"
    "$PYTHON" "$SCRIPT_DIR/verify_binstall.py" fixture-check --binstall "$binstall" --out "$out/binstall" </dev/null ||
        status=1
    [ -f "$out/binstall/binstall-report.json" ] || die "verify_binstall.py wrote no report"

    local runtime_status=0
    "$PYTHON" "$SCRIPT_DIR/verify_runtime.py" run --artifacts "$artifacts" --package "$package" \
        --command "$command" --version "$version" --out "$out/runtime" "${targets[@]+"${targets[@]}"}" </dev/null ||
        runtime_status=$?
    [ "$runtime_status" -ne 2 ] || die "verify_runtime.py could not use its input (see above)" 2
    [ "$runtime_status" -eq 0 ] || status=1

    "$PYTHON" "$SCRIPT_DIR/verify_runtime.py" combine --runtime-report "$out/runtime/runtime-report.json" \
        --binstall-report "$out/binstall/binstall-report.json" --out "$out" </dev/null || status=1
    note "report: $out/README.md"
    return "$status"
}

main "$@"
