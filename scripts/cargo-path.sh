# Source this script; do not execute it.
#
# Puts the host's Cargo bin directory on PATH for the current shell. Every
# `just` recipe runs in a fresh shell, and on a first-time install rustup has
# just written Cargo's bin directory without any parent shell inheriting it.
# On Cygwin / MSYS / Git Bash, $HOME also differs from the Windows profile
# directory rustup actually installs into, so "$HOME/.cargo" is the wrong
# place to look there — %USERPROFILE% is authoritative.

case "$(uname -s)" in
    MINGW*|MSYS*|CYGWIN*)
        if [[ -n "${CARGO_HOME:-}" ]]; then
            cargo_bin="$(cygpath -u "$CARGO_HOME")/bin"
        else
            cargo_bin="$(cygpath -u "${USERPROFILE:-$HOME}")/.cargo/bin"
        fi
        # Appended, not prepended: the shell's own /usr/bin must stay first
        # (just/shell.just), and a `cargo install`ed coreutils in here would
        # otherwise shadow it.
        cargo_path_order=append
        ;;
    *)
        cargo_bin="${CARGO_HOME:-$HOME/.cargo}/bin"
        cargo_path_order=prepend
        ;;
esac

if [[ -d "$cargo_bin" ]] && [[ ":$PATH:" != *":$cargo_bin:"* ]]; then
    if [[ "$cargo_path_order" == append ]]; then
        export PATH="$PATH:$cargo_bin"
    else
        export PATH="$cargo_bin:$PATH"
    fi
fi

# rustup writes this env file on Unix hosts; it is absent on Windows.
if [[ -f "${CARGO_HOME:-$HOME/.cargo}/env" ]]; then
    source "${CARGO_HOME:-$HOME/.cargo}/env"
fi

unset cargo_bin cargo_path_order
