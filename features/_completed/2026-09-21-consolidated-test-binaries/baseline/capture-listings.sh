#!/usr/bin/env bash
# Phase 1 before-listing capture (spike S1's validated command, run exactly).
# Evidence tool only: Phase 2's `consolidation.py capture` supersedes it and
# must reproduce these files byte-for-byte on an unmigrated tree (minus the
# locality fields S1 says are never identity).
#
# Usage: capture-listings.sh <out-dir> [package ...]
# Tier expressions come from `just _tier_filter`, never from this script.
set -euo pipefail

out_dir="${1:?out dir}"
shift
repo_root="$(git rev-parse --show-toplevel)"
cd "${repo_root}"
# An inherited override would silently replace the canonical L1 expression.
unset BISCUIT_TEST_FILTER BISCUIT_L1_INCLUDE_SLOW

# Feature sets = every `--features` value the package's canonical recipes pass
# (area justfiles) plus the CI union in `[package.metadata.ci.tests].features`.
feature_sets() {
    case "$1" in
        claudine-cli) printf '%s\n' none daemon-tests terminal-tests daemon-tests,terminal-tests real-tests ;;
        darkmatter) printf '%s\n' none effects-instrumentation terminal-tests browser-tests terminal-tests,browser-tests terminal-tests,browser-tests,effects-instrumentation ;;
        darkmatter-cli) printf '%s\n' none terminal-tests ;;
        biscuit-terminal) printf '%s\n' none image terminal-tests browser-tests image,terminal-tests,browser-tests ;;
        *) echo "unknown package $1" >&2; return 1 ;;
    esac
}

packages=("$@")
if (( ${#packages[@]} == 0 )); then
    packages=(claudine-cli darkmatter darkmatter-cli biscuit-terminal)
fi

host_os="$(uname -s | tr '[:upper:]' '[:lower:]')"
mkdir -p "${out_dir}"

for pkg in "${packages[@]}"; do
    while IFS= read -r fset; do
        feature_args=()
        if [[ "${fset}" != "none" ]]; then
            feature_args=(--features "${fset}")
        fi
        tag="${fset//,/+}"
        # Build once per feature set so every tier listing reads the same binaries.
        cargo nextest list --color=never -p "${pkg}" ${feature_args[@]+"${feature_args[@]}"} \
            --message-format json >/dev/null
        tiers=(L1 sanity L2 L3 browser real)
        for tier in "${tiers[@]}"; do
            filter="$(just _tier_filter "${tier}" "${pkg}")"
            file="${out_dir}/${host_os}__${pkg}__${tag}__${tier}.json"
            cargo nextest list --color=never -p "${pkg}" ${feature_args[@]+"${feature_args[@]}"} \
                --message-format json -E "${filter}" 2>/dev/null >"${file}"
            echo "captured ${file}"
        done
        if [[ "${pkg}" == "darkmatter" ]]; then
            filter="$(BISCUIT_L1_INCLUDE_SLOW=1 just _tier_filter L1 "${pkg}")"
            file="${out_dir}/${host_os}__${pkg}__${tag}__L1-include-slow.json"
            cargo nextest list --color=never -p "${pkg}" ${feature_args[@]+"${feature_args[@]}"} \
                --message-format json -E "${filter}" 2>/dev/null >"${file}"
            echo "captured ${file}"
        fi
    done < <(feature_sets "${pkg}")
done

# Raw listings are ~64 MB; commit them compressed. SHA256SUMS hashes the raw
# JSON so a decompressed file can be proven byte-identical to what nextest
# emitted. `gzip -n` omits name/mtime so re-compression is deterministic.
(
    cd "${out_dir}"
    shasum -a 256 -- *.json >SHA256SUMS
    gzip -9nf -- *.json
)
