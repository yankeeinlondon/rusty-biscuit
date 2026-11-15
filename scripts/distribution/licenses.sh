#!/usr/bin/env bash
# Write the third-party notices for one distributed package.
#
#   licenses.sh <checkout-root> <package> <output-file>
#
# The notices cover the package's locked production dependency closure for
# every target in `<checkout-root>/about.toml` (default features): one copy of
# each license text with the crates it covers, rendered by cargo-about 0.8.4,
# followed by what `native_notices.py` adds: crates Cargo links that
# cargo-about's graph omits, and the licenses of native libraries the crates
# vendor.
# `distribution.py stage --notices` takes the result.
#
# Environment: CARGO_ABOUT (default `cargo-about`, must report exactly 0.8.4),
# CARGO (default `cargo`), PYTHON (default `python3`; `native_notices.py` imports
# the pinned scripts/ci/requirements-distribution.txt, which setup-dist provides).
set -euo pipefail

readonly ABOUT_VERSION="0.8.4"

fail() {
    printf 'licenses.sh: %s\n' "$*" >&2
    exit 1
}

if [ "$#" -ne 3 ]; then
    printf 'usage: licenses.sh <checkout-root> <package> <output-file>\n' >&2
    exit 2
fi

script_dir=$(CDPATH='' cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)
root=$(CDPATH='' cd -- "$1" && pwd -P) || fail "checkout root $1 is not a directory"
package=$2
output=$3
about=${CARGO_ABOUT:-cargo-about}
python=${PYTHON:-python3}

command -v "$about" >/dev/null 2>&1 || fail "$about not found; install cargo-about $ABOUT_VERSION"
reported=$("$about" --version 2>/dev/null) || fail "$about --version failed"
[ "$reported" = "cargo-about $ABOUT_VERSION" ] || fail "need cargo-about $ABOUT_VERSION, found '$reported'"
[ -f "$root/Cargo.lock" ] || fail "$root/Cargo.lock is missing"
[ -f "$root/about.toml" ] || fail "$root/about.toml is missing"

manifest=$("$python" "$script_dir/native_notices.py" manifest "$root" "$package") || fail "cannot resolve package $package"

work=$(mktemp -d "${TMPDIR:-/tmp}/licenses.XXXXXX")
trap 'rm -rf "$work"' EXIT
cp "$root/Cargo.lock" "$work/Cargo.lock.before"

# shellcheck disable=SC2016 # the backticks are Markdown, not substitutions
{
    printf '# Third-party notices for %s\n\n' "$package"
    printf 'The binary in this archive is built from the `%s` package and is\n' "$package"
    printf 'licensed under AGPL-3.0-only; see `LICENSE`. It contains the third-party\n'
    printf 'software below, under the licenses shown.\n\n'
} >"$work/header.md"

(cd "$root" && "$about" generate --locked --fail \
    -c "$root/about.toml" -m "$manifest" -o "$work/crates.md" "$script_dir/about.hbs") ||
    fail "cargo-about could not resolve every license for $package"
"$python" "$script_dir/native_notices.py" render "$root" "$package" "$work/crates.md" >"$work/extra.md" ||
    fail "notices for $package are incomplete"

cmp -s "$root/Cargo.lock" "$work/Cargo.lock.before" || fail "Cargo.lock changed while collecting notices"
if ! grep -q '^- `' "$work/crates.md"; then
    fail "cargo-about produced no third-party crates for $package"
fi

mkdir -p "$(dirname "$output")"
if [ -s "$work/extra.md" ]; then
    printf '\n' >"$work/separator"
    cat "$work/header.md" "$work/crates.md" "$work/separator" "$work/extra.md" >"$work/notices.md"
else
    cat "$work/header.md" "$work/crates.md" >"$work/notices.md"
fi
cp "$work/notices.md" "$output.tmp.$$"
mv "$output.tmp.$$" "$output"
