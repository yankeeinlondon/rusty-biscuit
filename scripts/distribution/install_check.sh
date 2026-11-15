#!/usr/bin/env bash
# Install one published package release with its shell installer and run the
# installed command. The distribution workflow runs this after upload and
# before the Homebrew formula is published: a formula may reference only
# assets that install.
#
#   install_check.sh native <installer-url> <built-installer> <package> <target> <command> <out-dir>
#       Downloads the installer from the Release, requires it to be
#       byte-identical to the one the build produced, installs into an
#       isolated directory, and runs `<command> --version` and `--help`.
#   install_check.sh container <installer-url> <built-installer> <package> <target> <command> <out-dir>
#       The same, inside the pinned Ubuntu 22.04 runtime image (Linux
#       targets): only a downloader, CA certificates, xz, and the package's
#       runtime libraries are installed, never a compiler.
#
# The installation is isolated: a scratch HOME, `<PACKAGE>_UNMANAGED_INSTALL`
# (dist's flat layout, which also disables PATH edits and receipts), and a
# PATH of system directories only, so a Homebrew or Cargo copy on the runner
# cannot satisfy the check. A shell profile written under the scratch HOME
# fails the check. The report is `<out-dir>/install-<package>-<target>.txt`.
#
# Non-interactive: stdin is closed for every command and each step is bounded.

set -euo pipefail
unset CDPATH

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)
REPO_ROOT=$(cd "$SCRIPT_DIR/../.." && pwd -P)
CONFIG=${DIST_BUILD_IMAGES:-$REPO_ROOT/.github/distribution/build-images.json}
PYTHON=${PYTHON:-python3}
STEP_TIMEOUT=${DIST_STEP_TIMEOUT:-600}
SYSTEM_PATH=/usr/bin:/bin:/usr/sbin:/sbin

die() { printf 'install_check.sh: error: %s\n' "$*" >&2; exit 1; }
note() { printf 'install_check.sh: %s\n' "$*" >&2; }

usage() {
    sed -n '/^#   install_check.sh/,/^#$/p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//' >&2
    exit 2
}

bounded() {
    local seconds=$1
    shift
    if command -v timeout >/dev/null 2>&1; then
        timeout -k 30 "$seconds" "$@"
    elif command -v gtimeout >/dev/null 2>&1; then
        gtimeout -k 30 "$seconds" "$@"
    else
        "$@"
    fi
}

cfg() {
    "$PYTHON" - "$CONFIG" "$1" <<'PY'
import json, sys
value = json.load(open(sys.argv[1], encoding="utf-8"))
for key in sys.argv[2].split("."):
    value = value.get(key) if isinstance(value, dict) else None
if isinstance(value, list):
    print("\n".join(value))
elif value is not None:
    print(value)
PY
}

absolute_dir() { (cd "$1" && pwd -P); }

# dist's per-app environment prefix: the package name, upper-cased, `-` as `_`.
env_prefix() { printf '%s' "$1" | tr '[:lower:]-' '[:upper:]_'; }

fetch_installer() {
    local url=$1 built=$2 dest=$3
    case "$url" in
        https://github.com/*/releases/download/*) ;;
        *) die "installer URL $url is not a version-specific GitHub Release download" ;;
    esac
    [ -f "$built" ] || die "built installer $built does not exist"
    bounded 300 curl --proto '=https' --tlsv1.2 -fsSL --retry 3 -o "$dest" "$url" </dev/null ||
        die "cannot download $url"
    cmp -s "$dest" "$built" || die "the Release's installer differs from the one this build produced"
}

# Runs inside the target runtime: the installer file is already verified.
run_install() {
    local installer=$1 package=$2 target=$3 command=$4 out=$5
    local scratch prefix report status output flag listing
    mkdir -p "$out"
    scratch=$(mktemp -d "${TMPDIR:-/tmp}/dist-install.XXXXXX")
    mkdir -p "$scratch/home" "$scratch/root/bin"
    prefix=$(env_prefix "$package")
    report="$out/install-$package-$target.txt"
    {
        echo "host: $(uname -s) $(uname -m)"
        [ ! -r /etc/os-release ] || echo "os-release: $(sed -n 's/^PRETTY_NAME="\{0,1\}\([^"]*\)"\{0,1\}$/\1/p' /etc/os-release)"
        [ "$(uname -s)" != Darwin ] || echo "macos: $(sw_vers -productVersion 2>/dev/null || echo unknown)"
    } >"$report"

    status=0
    output=$(bounded "$STEP_TIMEOUT" env -i PATH="$SYSTEM_PATH" HOME="$scratch/home" \
        XDG_CONFIG_HOME="$scratch/home/.config" "${prefix}_UNMANAGED_INSTALL=$scratch/root/bin" \
        sh "$installer" </dev/null 2>&1) || status=$?
    printf '\n$ sh %s-installer.sh\n%s\n[exit %s]\n' "$package" "$output" "$status" >>"$report"
    [ "$status" -eq 0 ] || die "the installer exited $status (see $report)"

    listing=$(cd "$scratch/root/bin" && find . -mindepth 1 | sed 's|^\./||' | sort)
    printf '\ninstalled: %s\n' "$(echo "$listing" | tr '\n' ' ')" >>"$report"
    [ "$listing" = "$command" ] || die "the installer installed '$listing', expected exactly '$command'"
    for profile in .profile .bashrc .bash_profile .zshrc .zshenv .config/fish/conf.d; do
        [ ! -e "$scratch/home/$profile" ] || die "the installer wrote $profile; an isolated install never edits PATH"
    done

    for flag in --version --help; do
        status=0
        output=$(bounded 60 env -i PATH="$SYSTEM_PATH" HOME="$scratch/home" \
            "$scratch/root/bin/$command" "$flag" </dev/null 2>&1) || status=$?
        printf '\n$ %s %s\n%s\n[exit %s]\n' "$command" "$flag" "$output" "$status" >>"$report"
        [ "$status" -eq 0 ] || die "$command $flag exited $status (see $report)"
        [ -n "$output" ] || die "$command $flag printed nothing"
    done
    rm -rf "$scratch"
    note "install check passed: $report"
}

cmd_native() {
    [ $# -eq 6 ] || usage
    local url=$1 built=$2 package=$3 target=$4 command=$5 out
    mkdir -p "$6"
    out=$(absolute_dir "$6")
    fetch_installer "$url" "$built" "$out/$package-installer.sh"
    run_install "$out/$package-installer.sh" "$package" "$target" "$command" "$out"
}

cmd_container() {
    [ $# -eq 6 ] || usage
    local url=$1 built=$2 package=$3 target=$4 command=$5 out platform image runtime tools status=0
    mkdir -p "$6"
    out=$(absolute_dir "$6")
    [ "$(cfg "targets.$target.os")" = linux ] || die "$target is not a Linux target; use native"
    command -v docker >/dev/null 2>&1 || die "docker is required for container mode"
    platform=$(cfg "targets.$target.platform")
    image="$(cfg linux.image)@$(cfg linux.index_digest)"
    runtime="curl ca-certificates xz-utils $(cfg linux.runtime_packages) $(cfg "package_extras.$package.runtime_packages")"
    fetch_installer "$url" "$built" "$out/$package-installer.sh"
    # Docker sees copies under the output directory (see build.sh stage_tools).
    tools="$out/.install-tools"
    rm -rf "$tools"
    mkdir -p "$tools"
    cp "${BASH_SOURCE[0]}" "$tools/install_check.sh"
    cp "$CONFIG" "$tools/build-images.json"
    # shellcheck disable=SC2016 # expanded inside the container
    bounded "$((STEP_TIMEOUT * 2))" docker run --rm --platform "$platform" \
        -v "$out:/out" -v "$tools:/install-tools:ro" \
        -e DEBIAN_FRONTEND=noninteractive -e RUNTIME="$runtime" \
        -e PACKAGE="$package" -e TARGET="$target" -e COMMAND="$command" \
        "$image" bash -euo pipefail -c '
            # shellcheck disable=SC2086
            apt-get update -qq && apt-get install -y -qq --no-install-recommends $RUNTIME >/dev/null
            for tool in cc gcc rustc cargo; do
                ! command -v "$tool" >/dev/null 2>&1 || { echo "runtime image has $tool" >&2; exit 1; }
            done
            DIST_BUILD_IMAGES=/install-tools/build-images.json bash /install-tools/install_check.sh \
                _run "/out/$PACKAGE-installer.sh" "$PACKAGE" "$TARGET" "$COMMAND" /out
        ' </dev/null || status=$?
    rm -rf "$tools"
    [ "$status" -eq 0 ] || die "the container install check failed for $target (exit $status)"
}

main() {
    [ $# -ge 1 ] || usage
    local subcommand=$1
    shift
    case "$subcommand" in
        native) cmd_native "$@" ;;
        container) cmd_container "$@" ;;
        # Internal: the step `container` runs inside the runtime image.
        _run) [ $# -eq 5 ] || usage; run_install "$@" ;;
        *) usage ;;
    esac
}

main "$@"
