#!/usr/bin/env bash
set -euo pipefail

source scripts/cargo-path.sh

case "$(uname -s)" in
    MINGW*|MSYS*|CYGWIN*)
        winget_links="$(cygpath -u "${LOCALAPPDATA:?}")/Microsoft/WinGet/Links"
        if [[ -d "$winget_links" ]]; then
            export PATH="$winget_links:$PATH"
        fi
        ;;
esac

install_cargo_binary() {
    local binary="$1" package="$2"
    shift 2
    if command -v "$binary" >/dev/null 2>&1; then
        return
    fi

    echo "Installing $binary..."
    if command -v cargo-binstall >/dev/null 2>&1; then
        RUSTC_WRAPPER="" cargo binstall --no-confirm "$package" \
            || RUSTC_WRAPPER="" cargo install --locked "$package" "$@"
    else
        RUSTC_WRAPPER="" cargo install --locked "$package" "$@"
    fi
    command -v "$binary" >/dev/null 2>&1
}

activate_windows_python() {
    local python_root python_exe python_dir
    python_root="$(cygpath -u "${LOCALAPPDATA:?}")/Programs/Python"
    python_exe="$(find "$python_root" -maxdepth 2 -name python.exe -print 2>/dev/null | sort | tail -n1)"
    if [[ -z "$python_exe" ]]; then
        return
    fi

    python_dir="$(dirname "$python_exe")"
    # The Windows installer exposes python.exe, while repository scripts use
    # the cross-platform python3 command name.
    if [[ ! -e "$python_dir/python3.exe" ]]; then
        cp "$python_exe" "$python_dir/python3.exe"
    fi
    export PATH="$python_dir:$PATH"
    if [[ -d /usr/bin && -w /usr/bin ]]; then
        printf '#!/usr/bin/env bash\nexec %q "$@"\n' "$python_exe" > /usr/bin/python3
        chmod +x /usr/bin/python3
    fi
}

ensure_python() {
    if python3 -c 'import sys; assert sys.version_info >= (3, 10)' >/dev/null 2>&1; then
        return
    fi

    case "$(uname -s)" in
        MINGW*|MSYS*|CYGWIN*)
            activate_windows_python
            if python3 -c 'import sys; assert sys.version_info >= (3, 10)' >/dev/null 2>&1; then
                return
            fi
            if ! command -v winget >/dev/null 2>&1; then
                echo "Python 3.10+ is required by CI/CD scripts and winget is unavailable." >&2
                exit 1
            fi
            winget install --id Python.Python.3.13 --exact --silent \
                --accept-source-agreements --accept-package-agreements || true
            activate_windows_python
            ;;
        Darwin)
            brew install python
            ;;
        Linux)
            elevate=()
            if [[ "$(id -u)" -ne 0 ]]; then
                elevate=(sudo)
            fi
            if command -v apt-get >/dev/null 2>&1; then
                "${elevate[@]}" apt-get update -qq && "${elevate[@]}" apt-get install -y -qq python3
            elif command -v dnf >/dev/null 2>&1; then
                "${elevate[@]}" dnf install -y python3
            elif command -v pacman >/dev/null 2>&1; then
                "${elevate[@]}" pacman -S --noconfirm python
            elif command -v apk >/dev/null 2>&1; then
                "${elevate[@]}" apk add python3
            fi
            ;;
    esac

    if ! python3 -c 'import sys; assert sys.version_info >= (3, 10)' >/dev/null 2>&1; then
        echo "Python 3.10+ was not found on PATH; open a new terminal and re-run 'just init'." >&2
        exit 1
    fi
}

ensure_gh() {
    if command -v gh >/dev/null 2>&1; then
        return
    fi

    case "$(uname -s)" in
        MINGW*|MSYS*|CYGWIN*)
            if ! command -v winget >/dev/null 2>&1; then
                echo "GitHub CLI is required and winget is unavailable." >&2
                exit 1
            fi
            winget install --id GitHub.cli --exact --silent \
                --accept-source-agreements --accept-package-agreements || true
            ;;
        Darwin)
            brew install gh
            ;;
        Linux)
            elevate=()
            if [[ "$(id -u)" -ne 0 ]]; then
                elevate=(sudo)
            fi
            if command -v apt-get >/dev/null 2>&1; then
                "${elevate[@]}" apt-get update -qq && "${elevate[@]}" apt-get install -y -qq gh
            elif command -v dnf >/dev/null 2>&1; then
                "${elevate[@]}" dnf install -y gh
            elif command -v pacman >/dev/null 2>&1; then
                "${elevate[@]}" pacman -S --noconfirm github-cli
            elif command -v apk >/dev/null 2>&1; then
                "${elevate[@]}" apk add github-cli
            else
                echo "Install GitHub CLI from https://cli.github.com/ and re-run 'just init'." >&2
                exit 1
            fi
            ;;
    esac
    hash -r
    command -v gh >/dev/null 2>&1
}

ensure_python
ensure_gh

ensure_pnpm() {
    if command -v pnpm >/dev/null 2>&1; then
        return
    fi

    echo "Installing pnpm..."
    # `npm install --global` writes to npm's prefix, which for a distro or
    # Homebrew Node is root-owned (`/usr/local/lib/node_modules`). Unelevated it
    # dies EACCES, and under `set -e` that aborts `just init` outright — leaving
    # pnpm uninstalled, so homelab's frontend lint and test recipes self-skip on
    # every later run. Attempt unelevated first so a user-prefix or nvm/fnm Node
    # (and Windows, which has no sudo) never pulls in a needless sudo prompt.
    if ! npm install --global pnpm 2>/dev/null; then
        if [[ "$(id -u)" -ne 0 ]] && command -v sudo >/dev/null 2>&1; then
            echo "npm's global prefix is not writable; retrying with sudo..."
            sudo npm install --global pnpm
        else
            echo "Could not install pnpm into npm's global prefix ($(npm config get prefix))." >&2
            echo "Install it manually (https://pnpm.io/installation) and re-run 'just init'." >&2
            exit 1
        fi
    fi
    hash -r

    if ! command -v pnpm >/dev/null 2>&1; then
        echo "pnpm was installed but is not visible on PATH; open a new terminal and re-run 'just init'." >&2
        exit 1
    fi
}

# Node is not auto-installed: version managers (nvm, fnm, volta) own the node
# binary on most developer hosts, and most distro `nodejs` packages predate 22.
# A package-manager install would shadow or downgrade a working toolchain, so
# this stops with the concrete remedies instead.
if ! command -v node >/dev/null 2>&1 || ! command -v npm >/dev/null 2>&1; then
    echo "Node.js 22+ and npm are required: pnpm installs through npm, and homelab's" >&2
    echo "'just lint' and 'just test' drive a Vue suite through pnpm." >&2
    echo "Install from https://nodejs.org/, or via nvm/fnm/volta, then re-run 'just init'." >&2
    exit 1
fi
if (( $(node -p 'Number(process.versions.node.split(".")[0])') < 22 )); then
    echo "Node.js 22+ is required; found $(node --version)." >&2
    echo "Upgrade via https://nodejs.org/ or your version manager, then re-run 'just init'." >&2
    exit 1
fi
ensure_pnpm

install_cargo_binary cargo-llvm-cov cargo-llvm-cov
rustup component add llvm-tools-preview
install_cargo_binary release-plz release-plz

# These workflows run only on Unix hosts; cargo-fuzz is unsupported on Windows.
case "$(uname -s)" in
    Linux|Darwin)
        rustup toolchain install nightly --profile minimal
        if ! command -v cargo-fuzz >/dev/null 2>&1; then
            RUSTC_WRAPPER="" cargo +nightly install --locked cargo-fuzz
        fi
        ;;
esac

# Cross-compilation and Bencher upload are currently Linux-owned workflows.
if [[ "$(uname -s)" == "Linux" ]]; then
    install_cargo_binary cross cross
    if ! command -v bencher >/dev/null 2>&1; then
        RUSTC_WRAPPER="" cargo install --git https://github.com/bencherdev/bencher \
            --branch main --locked --force bencher_cli
    fi
fi

python3 -c 'import sys; assert sys.version_info >= (3, 10)'
gh --version >/dev/null
node --version >/dev/null
npm --version >/dev/null
pnpm --version >/dev/null
cargo llvm-cov --version >/dev/null
release-plz --version >/dev/null
case "$(uname -s)" in
    Linux|Darwin) cargo fuzz --version >/dev/null ;;
esac
if [[ "$(uname -s)" == "Linux" ]]; then
    cross --version >/dev/null
    bencher --version >/dev/null
fi

printf 'CI/CD tools ready for this host\n'
