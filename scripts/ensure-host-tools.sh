#!/usr/bin/env bash
set -euo pipefail

case "$(uname -s)" in
    MINGW*|MSYS*|CYGWIN*)
        winget_links="$(cygpath -u "${LOCALAPPDATA:?}")/Microsoft/WinGet/Links"
        cargo_bin="${CARGO_HOME:-$HOME/.cargo}/bin"
        if [[ -d "$winget_links" ]]; then
            export PATH="$cargo_bin:$winget_links:$PATH"
        fi
        ;;
esac

if (( BASH_VERSINFO[0] < 4 )); then
    echo "Bash 4 or newer is required by shared recipes; found $BASH_VERSION." >&2
    echo "Install a current Bash and ensure it precedes /bin/bash on PATH." >&2
    exit 1
fi

# These commands are the portable shell baseline used by repository scripts.
required=(
    awk basename bash cat chmod cp curl cut date df dirname du env find git grep
    head jq ln mkdir mktemp mv readlink realpath rm sed sort tar tee touch tr uname wc xargs
)

missing=()
for tool in "${required[@]}"; do
    if ! command -v "$tool" >/dev/null 2>&1; then
        missing+=("$tool")
    fi
done

if (( ${#missing[@]} > 0 )); then
    echo "Missing required shell utilities: ${missing[*]}" >&2
    case "$(uname -s)" in
        MINGW*|MSYS*|CYGWIN*)
            echo "Repair Git for Windows, or install Cygwin packages coreutils, findutils, gawk, grep, sed, and tar." >&2
            ;;
        Darwin)
            echo "Install the missing utilities with Homebrew, then re-run 'just init'." >&2
            ;;
        *)
            echo "Install the missing utilities with the host package manager, then re-run 'just init'." >&2
            ;;
    esac
    exit 1
fi

install_cargo_binary() {
    local binary="$1" package="$2"
    if command -v "$binary" >/dev/null 2>&1; then
        return
    fi

    echo "Installing $binary..."
    if command -v cargo-binstall >/dev/null 2>&1; then
        RUSTC_WRAPPER="" cargo binstall --no-confirm "$package" \
            || RUSTC_WRAPPER="" cargo install --locked "$package"
    else
        RUSTC_WRAPPER="" cargo install --locked "$package"
    fi
    command -v "$binary" >/dev/null 2>&1
}

install_fzf() {
    if fzf --version >/dev/null 2>&1; then
        return
    fi

    echo "Installing fzf..."
    case "$(uname -s)" in
        MINGW*|MSYS*|CYGWIN*)
            if ! command -v winget >/dev/null 2>&1; then
                echo "fzf is missing and winget is unavailable; install junegunn/fzf and re-run 'just init'." >&2
                exit 1
            fi
            winget install --id junegunn.fzf --exact --silent \
                --accept-source-agreements --accept-package-agreements || true
            fzf_link="$winget_links/fzf.exe"
            fzf_target="$(readlink -f "$fzf_link" 2>/dev/null || true)"
            if [[ -n "$fzf_target" ]] && ! "$fzf_link" --version >/dev/null 2>&1; then
                cargo_bin="${CARGO_HOME:-$HOME/.cargo}/bin"
                mkdir -p "$cargo_bin"
                cp "$fzf_target" "$cargo_bin/fzf.exe"
                export PATH="$cargo_bin:$PATH"
            fi
            ;;
        Darwin)
            brew install fzf
            ;;
        Linux)
            elevate=()
            if [[ "$(id -u)" -ne 0 ]]; then
                elevate=(sudo)
            fi
            if command -v apt-get >/dev/null 2>&1; then
                "${elevate[@]}" apt-get update -qq && "${elevate[@]}" apt-get install -y -qq fzf
            elif command -v dnf >/dev/null 2>&1; then
                "${elevate[@]}" dnf install -y fzf
            elif command -v pacman >/dev/null 2>&1; then
                "${elevate[@]}" pacman -S --noconfirm fzf
            elif command -v apk >/dev/null 2>&1; then
                "${elevate[@]}" apk add fzf
            else
                echo "fzf is missing and no supported package manager was found." >&2
                exit 1
            fi
            ;;
    esac
    hash -r

    if ! fzf --version >/dev/null 2>&1; then
        echo "fzf was installed but is not visible on PATH; open a new terminal and re-run 'just init'." >&2
        exit 1
    fi
}

install_cargo_binary fd fd-find
install_cargo_binary rg ripgrep
install_cargo_binary eza eza
install_fzf

fd --version >/dev/null
rg --version >/dev/null
eza --version >/dev/null
fzf --version >/dev/null
printf 'Host utilities ready: core shell tools, fd, rg, fzf, eza\n'
