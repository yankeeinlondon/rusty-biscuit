#!/usr/bin/env bash
# Build, inspect, and smoke-test ONE distribution target from a tree that
# `scripts/ci/distribution.py stage` already prepared. Settings come from
# `.github/distribution/build-images.json` (plan rules 11-13 of
# `2026-10-28-publish`); the build-env record it writes is consumed by
# `distribution.py provenance`.
#
#   build.sh build <staging-root> <tag> <target> <command> <out-dir>
#       Runs where the target's ABI is defined: on the macOS runner itself, or
#       INSIDE the pinned Ubuntu 22.04 container for Linux (the caller starts
#       the container and sets DIST_CONTAINER_IMAGE to the pinned reference).
#       macOS targets are smoke-tested here too; Linux targets are smoke-tested
#       by `smoke` in a clean runtime container.
#   build.sh smoke <archive-or-extracted-dir> <package> <target> <command> <out-dir>
#       Runs `<command> --version` and `--help` from the archive (and `ldd` on
#       Linux). Needs only bash and coreutils, so it runs in the runtime image.
#   build.sh container <staging-root> <tag> <target> <command> <out-dir>
#   build.sh smoke-container <archive> <package> <target> <command> <out-dir>
#       Local-evidence wrappers (host with Docker): start the pinned image for
#       a Linux target, install the configured packages, and run `build` or
#       `smoke` inside it. DIST_CONTAINER_CACHE=<dir> keeps the Rust toolchain
#       and crate downloads between runs.
#
# Environment: DIST_BIN (default `dist` on PATH), PYTHON (default python3),
# DIST_BUILD_IMAGES (configuration path), DIST_CARGO_HOME (an empty Cargo
# home; default <staging-root>/target/dist-cargo-home), DIST_BUILD_TIMEOUT and
# DIST_STEP_TIMEOUT (seconds), DIST_ALLOW_CROSS=1 (local evidence only: build
# a macOS target on the other architecture; recorded in the build-env).
#
# Non-interactive by design: no prompts, no background processes, and every
# long step is bounded by `timeout` when the host has one.

set -euo pipefail
unset CDPATH

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd -P)
REPO_ROOT=$(cd "$SCRIPT_DIR/../.." && pwd -P)
CONFIG=${DIST_BUILD_IMAGES:-$REPO_ROOT/.github/distribution/build-images.json}
PYTHON=${PYTHON:-python3}
BUILD_TIMEOUT=${DIST_BUILD_TIMEOUT:-5400}
STEP_TIMEOUT=${DIST_STEP_TIMEOUT:-1200}
SMOKE_TIMEOUT=60

die() { printf 'build.sh: error: %s\n' "$*" >&2; exit 1; }
note() { printf 'build.sh: %s\n' "$*" >&2; }

usage() {
    sed -n '/^#   build.sh/,/^#$/p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//' >&2
    exit 2
}

# `timeout` is GNU coreutils on Linux and present on recent macOS; when a host
# lacks it, the workflow's job-level timeout is the bound.
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

# Print a value from build-images.json by dotted path: strings as-is, lists one
# item per line, mappings as KEY=VALUE lines; a missing path prints nothing.
cfg() {
    "$PYTHON" - "$CONFIG" "$1" <<'PY'
import json, sys
value = json.load(open(sys.argv[1], encoding="utf-8"))
for key in sys.argv[2].split("."):
    value = value.get(key) if isinstance(value, dict) else None
if isinstance(value, list):
    print("\n".join(value))
elif isinstance(value, dict):
    print("\n".join(f"{k}={v}" for k, v in value.items() if isinstance(v, str)))
elif value is not None:
    print(value)
PY
}

package_from_tag() {
    "$PYTHON" - "$1" <<'PY'
import re, sys
match = re.match(r"^(.+?)-v([0-9]+\.[0-9]+\.[0-9]+(?:[-+].*)?)$", sys.argv[1])
if not match:
    sys.exit(f"build.sh: error: {sys.argv[1]!r} is not a <package>-v<version> tag")
print(match.group(1))
PY
}

os_release() { sed -n "s/^$1=\"\{0,1\}\([^\"]*\)\"\{0,1\}\$/\1/p" /etc/os-release; }

absolute_dir() { (cd "$1" && pwd -P); }

host_arch() {
    case "$(uname -m)" in
        arm64 | aarch64) echo aarch64 ;;
        x86_64 | amd64) echo x86_64 ;;
        *) uname -m ;;
    esac
}

# Ambient Cargo configuration: an empty CARGO_HOME, and no `.cargo/config*`
# above the staging root (the staged checkout's own is tagged source).
isolate_cargo() {
    local staging=$1 dir
    export CARGO_HOME=${DIST_CARGO_HOME:-$staging/target/dist-cargo-home}
    mkdir -p "$CARGO_HOME"
    for name in config config.toml credentials credentials.toml; do
        [ ! -e "$CARGO_HOME/$name" ] || die "CARGO_HOME $CARGO_HOME holds $name; a build Cargo home has no configuration"
    done
    dir=$(dirname "$staging")
    while :; do
        for name in config config.toml; do
            [ ! -e "$dir/.cargo/$name" ] || die "ambient Cargo configuration $dir/.cargo/$name sits above the staging root"
        done
        [ "$dir" != / ] || break
        dir=$(dirname "$dir")
    done
}

cmd_build() {
    [ $# -eq 5 ] || usage
    local staging tag=$2 target=$3 command=$4 out package os arch runner
    staging=$(absolute_dir "$1") || die "staging root $1 does not exist"
    mkdir -p "$5"
    out=$(absolute_dir "$5")
    [ -f "$CONFIG" ] || die "configuration $CONFIG is missing"
    os=$(cfg "targets.$target.os")
    [ -n "$os" ] || die "target $target is not configured in $CONFIG"
    arch=$(cfg "targets.$target.arch")
    runner=$(cfg "targets.$target.runner")
    package=$(package_from_tag "$tag")
    [ -f "$staging/Cargo.lock" ] || die "$staging has no Cargo.lock"
    [ -f "$staging/rust-toolchain.toml" ] || die "$staging has no rust-toolchain.toml"

    # --- environment -------------------------------------------------------
    local name pair cross=false
    while IFS= read -r name; do
        [ -z "$name" ] || unset "$name"
    done < <(cfg cleared_env)
    while IFS= read -r pair; do
        [ -z "$pair" ] || export "${pair?}"
    done < <(cfg required_env)
    export DO_NOT_USE_BREWFILE=1
    isolate_cargo "$staging"

    local host
    host=$(host_arch)
    case "$os" in
        macos)
            [ "$(uname -s)" = Darwin ] || die "$target builds on macOS only"
            export MACOSX_DEPLOYMENT_TARGET
            MACOSX_DEPLOYMENT_TARGET=$(cfg macos.deployment_target)
            if [ "$host" != "${arch/arm64/aarch64}" ]; then
                [ "${DIST_ALLOW_CROSS:-}" = 1 ] || die "$target is built natively on $runner; this host is $host (DIST_ALLOW_CROSS=1 for local evidence)"
                cross=true
            fi
            ;;
        linux)
            [ "$(uname -s)" = Linux ] || die "$target builds inside the pinned Linux container only"
            [ "$host" = "$arch" ] || die "$target is built natively; this container is $host"
            local image glibc version_id
            image="$(cfg linux.image)@$(cfg linux.index_digest)"
            [ "${DIST_CONTAINER_IMAGE:-}" = "$image" ] ||
                die "DIST_CONTAINER_IMAGE is '${DIST_CONTAINER_IMAGE:-}', expected the pinned $image"
            version_id=$(os_release VERSION_ID)
            [ "$version_id" = "$(cfg linux.os_version_id)" ] || die "container is release $version_id, not $(cfg linux.os_version_id)"
            # awk, not head: under pipefail an early-closing reader can SIGPIPE ldd.
            glibc=$(ldd --version | awk 'NR == 1 {print $NF}')
            [ "$glibc" = "$(cfg max_glibc)" ] || die "container glibc is $glibc, not $(cfg max_glibc)"
            ;;
        *) die "target $target has unknown os $os" ;;
    esac

    # --- toolchain: the tagged rust-toolchain.toml channel -----------------
    local channel rustc_version dist_bin dist_version
    channel=$(awk -F'"' '/^channel *=/ && !found {print $2; found = 1}' "$staging/rust-toolchain.toml")
    [ -n "$channel" ] || die "$staging/rust-toolchain.toml names no channel"
    export RUSTUP_TOOLCHAIN=$channel
    if command -v rustup >/dev/null 2>&1; then
        bounded "$STEP_TIMEOUT" rustup toolchain install "$channel" --profile minimal --target "$target" --no-self-update >&2
    fi
    rustc_version=$(rustc -V)
    case "$rustc_version" in
        "rustc $channel "*) ;;
        *) die "rustc is '$rustc_version', not the tagged $channel" ;;
    esac
    dist_bin=${DIST_BIN:-dist}
    dist_version=$("$dist_bin" --version)
    [ "$dist_version" = "cargo-dist $(cfg dist.version)" ] || die "$dist_bin is '$dist_version', not cargo-dist $(cfg dist.version)"

    # --- lockfile: dist never passes --locked, so prove it around the build -
    local lock_log="$out/cargo-locked-$target.log"
    cp "$staging/Cargo.lock" "$out/Cargo.lock.before"
    (cd "$staging" && bounded "$STEP_TIMEOUT" cargo metadata --locked --format-version 1 >/dev/null) >"$lock_log" 2>&1 || { tail -n 40 "$lock_log" >&2; die "Cargo.lock does not satisfy the staged manifests (--locked)"; }

    # --- dist build ----------------------------------------------------------
    local distrib="$staging/target/distrib" archive="$package-$target.tar.xz"
    local manifest="$out/dist-manifest-$target.json" log="$out/dist-build-$target.log"
    rm -f "$distrib/$archive" "$distrib/$archive.sha256" "$distrib/$target-dist-manifest.json"
    note "dist build $tag for $target (log: $log)"
    if ! (cd "$staging" && bounded "$BUILD_TIMEOUT" "$dist_bin" build \
        --tag="$tag" --artifacts=local --target="$target" --print=linkage --output-format=json \
        >"$manifest" 2>"$log"); then
        tail -n 60 "$log" >&2
        die "dist build failed for $target"
    fi
    cmp -s "$out/Cargo.lock.before" "$staging/Cargo.lock" || die "the build changed Cargo.lock"

    local expected selected
    expected="building $target target, using cargo profile dist --package=$package)"
    if [ "$cross" = true ]; then
        expected="building $target target, from $(rustc -vV | sed -n 's/^host: //p') host, using cargo profile dist --package=$package)"
    fi
    selected=$(grep -E '^building .*using cargo profile' "$log" || true)
    [ "$selected" = "$expected" ] || die "dist build log selects '$selected', expected exactly '$expected'"
    if grep -n -E -e '--workspace|--all-features|--features' "$log" >&2; then
        die "dist build log shows a workspace or feature-selecting cargo invocation"
    fi

    "$PYTHON" - "$manifest" "$package" "$target" "$command" "$archive" <<'PY'
import json, sys
manifest, package, target, command, archive = sys.argv[1:]
data = json.load(open(manifest, encoding="utf-8"))
apps = [release.get("app_name") for release in data.get("releases", [])]
if apps != [package]:
    sys.exit(f"build.sh: error: dist manifest releases {apps}, expected [{package!r}]")
artifact = data.get("artifacts", {}).get(archive)
if not artifact or artifact.get("target_triples") != [target]:
    sys.exit(f"build.sh: error: dist manifest has no {archive} for {target}")
executables = [asset.get("name") for asset in artifact.get("assets", []) if asset.get("kind") == "executable"]
if executables != [command]:
    sys.exit(f"build.sh: error: dist packaged executables {executables}, expected [{command!r}]")
PY
    [ -f "$distrib/$archive" ] && [ -f "$distrib/$archive.sha256" ] || die "dist produced no $archive and sidecar"
    # The global (installer/formula) step reads each target's manifest here.
    cp "$manifest" "$distrib/$target-dist-manifest.json"
    cp "$distrib/$archive" "$distrib/$archive.sha256" "$out/"

    # --- build-env record for provenance ------------------------------------
    local -a image_fields
    if [ "$os" = linux ]; then
        local digest_key
        digest_key=$(cfg "targets.$target.image_digest_key")
        image_fields=(
            "container=$(cfg linux.image)@$(cfg linux.index_digest)"
            "platform=$(cfg "targets.$target.platform")"
            "platform_digest=$(cfg "linux.platform_digests.$digest_key")"
            "os_release=$version_id"
            "glibc=$glibc"
        )
    else
        local xcode sdk macos_version
        xcode=$(bounded 60 xcodebuild -version 2>/dev/null | sed -n 's/^Build version //p' || true)
        sdk=$(bounded 60 xcrun --show-sdk-version 2>/dev/null || true)
        macos_version=$(sw_vers -productVersion 2>/dev/null || true)
        image_fields=(
            "xcode_build=${xcode:-unavailable}"
            "sdk_version=${sdk:-unknown}"
            "macos_version=${macos_version:-unknown}"
            "deployment_target=$MACOSX_DEPLOYMENT_TARGET"
        )
        if [ -z "$xcode" ]; then
            # A Command Line Tools-only host (never a hosted runner) has no xcodebuild.
            image_fields+=("command_line_tools=$(pkgutil --pkg-info=com.apple.pkg.CLTools_Executables 2>/dev/null | sed -n 's/^version: //p')")
        fi
        [ "$cross" = false ] || image_fields+=("cross_compiled_from=$host")
    fi
    image_fields+=(
        "runner=$runner"
        "host=$([ "${GITHUB_ACTIONS:-}" = true ] && echo github-actions || echo local)"
        "image_os=${ImageOS:-local}"
        "image_version=${ImageVersion:-local}"
    )
    "$PYTHON" - "$out/build-env-$target.json" "$target" "$rustc_version" "${image_fields[@]}" <<'PY'
import json, sys
path, target, rustc, *fields = sys.argv[1:]
image = dict(field.split("=", 1) for field in fields)
with open(path, "w", encoding="utf-8") as handle:
    json.dump({"target": target, "rustc": rustc, "image": dict(sorted(image.items()))}, handle, indent=2)
    handle.write("\n")
PY

    # --- inspection and (macOS) smoke ----------------------------------------
    if ! "$PYTHON" "$SCRIPT_DIR/inspect_archive.py" archive "$out/$archive" --package "$package" --command "$command" \
        --target "$target" --tag "$tag" --config "$CONFIG" --staging-root "$staging" \
        --evidence-dir "$out" --report "$out/inspect-$package-$target.json" >/dev/null; then
        die "archive inspection failed; see $out/inspect-$package-$target.json"
    fi
    if [ "$os" = macos ] && [ "$cross" = false ]; then
        cmd_smoke "$out/$archive" "$package" "$target" "$command" "$out"
    else
        note "smoke for $target runs separately (Linux: clean runtime container; cross builds: native runner)"
    fi
    note "built $out/$archive"
}

cmd_smoke() {
    [ $# -eq 5 ] || usage
    local source=$1 package=$2 target=$3 command=$4 out scratch="" dir binary flag status
    mkdir -p "$5"
    out=$(absolute_dir "$5")
    if [ -d "$source" ]; then
        dir=$source
    else
        scratch=$(mktemp -d "${TMPDIR:-/tmp}/dist-smoke.XXXXXX")
        tar -xf "$source" -C "$scratch"
        dir=$scratch
    fi
    binary="$dir/$package-$target/$command"
    [ -f "$binary" ] && [ -x "$binary" ] || die "$binary is not an executable file"
    local report="$out/smoke-$package-$target.txt" output
    : >"$report"
    {
        echo "host: $(uname -s) $(uname -m)"
        [ ! -r /etc/os-release ] || echo "os-release: $(os_release PRETTY_NAME)"
        echo "build tools on PATH: $(for tool in cc gcc ld readelf cargo rustc; do command -v "$tool" >/dev/null 2>&1 && printf '%s ' "$tool"; done; echo)"
    } >>"$report"
    for flag in --version --help; do
        status=0
        output=$(bounded "$SMOKE_TIMEOUT" "$binary" "$flag" </dev/null 2>&1) || status=$?
        printf '\n$ %s %s\n%s\n[exit %s]\n' "$command" "$flag" "$output" "$status" >>"$report"
        [ "$status" -eq 0 ] || die "$command $flag exited $status (see $report)"
        [ -n "$output" ] || die "$command $flag printed nothing"
    done
    if [ "$(uname -s)" = Linux ]; then
        ldd "$binary" >"$out/ldd-$package-$target.txt" 2>&1 || die "ldd failed on $binary"
        ! grep -q 'not found' "$out/ldd-$package-$target.txt" || die "ldd: unresolved library (see $out/ldd-$package-$target.txt)"
    fi
    [ -z "$scratch" ] || rm -rf "$scratch"
    note "smoke passed: $report"
}

# Docker sees a copy of build.sh, inspect_archive.py, and the configuration under the
# output directory: some Docker Desktop backends cannot bind-mount every host
# volume (a checkout on /Volumes, say), and the output directory must be
# mountable anyway.
stage_tools() {
    local tools="$1/.container-tools"
    rm -rf "$tools"
    mkdir -p "$tools"
    cp "$SCRIPT_DIR/build.sh" "$SCRIPT_DIR/inspect_archive.py" "$tools/"
    cp "$CONFIG" "$tools/build-images.json"
    echo "$tools"
}

linux_settings() {
    local target=$1
    [ "$(cfg "targets.$target.os")" = linux ] || die "$target is not a Linux target; build it natively with 'build'"
    PLATFORM=$(cfg "targets.$target.platform")
    IMAGE="$(cfg linux.image)@$(cfg linux.index_digest)"
    command -v docker >/dev/null 2>&1 || die "docker is required for container mode"
}

cmd_container() {
    [ $# -eq 5 ] || usage
    local staging tag=$2 target=$3 command=$4 out package packages dist_url dist_sha cache_args=()
    staging=$(absolute_dir "$1") || die "staging root $1 does not exist"
    mkdir -p "$5"
    out=$(absolute_dir "$5")
    linux_settings "$target"
    package=$(package_from_tag "$tag")
    packages="$(cfg linux.build_packages) $(cfg linux.tool_packages) $(cfg "package_extras.$package.build_packages")"
    dist_url="$(cfg dist.release_url)/cargo-dist-$target.tar.xz"
    dist_sha=$(cfg "dist.sha256.$target")
    if [ -n "${DIST_CONTAINER_CACHE:-}" ]; then
        mkdir -p "$DIST_CONTAINER_CACHE/rustup" "$DIST_CONTAINER_CACHE/cargo-home"
        cache_args=(-v "$(absolute_dir "$DIST_CONTAINER_CACHE")/rustup:/opt/rustup"
            -v "$(absolute_dir "$DIST_CONTAINER_CACHE")/cargo-home:/opt/dist-cargo-home"
            -e DIST_CARGO_HOME=/opt/dist-cargo-home)
    fi
    local tools status=0
    tools=$(stage_tools "$out")
    # shellcheck disable=SC2016 # expanded inside the container
    bounded "$((BUILD_TIMEOUT + STEP_TIMEOUT))" docker run --rm --platform "$PLATFORM" \
        -v "$staging:/work" -v "$out:/out" -v "$tools:/dist-scripts:ro" \
        "${cache_args[@]+"${cache_args[@]}"}" \
        -e DEBIAN_FRONTEND=noninteractive -e DIST_CONTAINER_IMAGE="$IMAGE" \
        -e DIST_BUILD_IMAGES=/dist-scripts/build-images.json -e RUSTUP_HOME=/opt/rustup \
        -e DIST_BUILD_TIMEOUT="$BUILD_TIMEOUT" -e DIST_STEP_TIMEOUT="$STEP_TIMEOUT" \
        -e PACKAGES="$packages" -e DIST_URL="$dist_url" -e DIST_SHA="$dist_sha" \
        -e TAG="$tag" -e TARGET="$target" -e COMMAND="$command" \
        "$IMAGE" bash -euo pipefail -c '
            # shellcheck disable=SC2086
            apt-get update -qq && apt-get install -y -qq --no-install-recommends $PACKAGES >/dev/null
            curl --proto =https --tlsv1.2 -fsSL --max-time 300 -o /tmp/dist.tar.xz "$DIST_URL"
            echo "$DIST_SHA  /tmp/dist.tar.xz" | sha256sum -c - >&2
            mkdir -p /opt/dist && tar -xJf /tmp/dist.tar.xz -C /opt/dist --strip-components=1
            export DIST_BIN=/opt/dist/dist
            # rustup proxies only; the build itself uses an empty CARGO_HOME.
            export CARGO_HOME=/opt/rustup-bin
            init="https://static.rust-lang.org/rustup/dist/$(uname -m)-unknown-linux-gnu/rustup-init"
            curl --proto =https --tlsv1.2 -fsSL --max-time 300 -o /tmp/rustup-init "$init"
            curl --proto =https --tlsv1.2 -fsSL --max-time 60 "$init.sha256" | awk "{print \$1\"  /tmp/rustup-init\"}" | sha256sum -c - >&2
            chmod +x /tmp/rustup-init
            /tmp/rustup-init -y --no-modify-path --profile minimal --default-toolchain none >&2
            export PATH=/opt/rustup-bin/bin:$PATH
            unset CARGO_HOME
            bash /dist-scripts/build.sh build /work "$TAG" "$TARGET" "$COMMAND" /out
        ' || status=$?
    rm -rf "$tools"
    [ "$status" -eq 0 ] || die "container build failed for $target (exit $status)"
}

cmd_smoke_container() {
    [ $# -eq 5 ] || usage
    local archive=$1 package=$2 target=$3 command=$4 out extract runtime
    [ -f "$archive" ] || die "$archive does not exist"
    mkdir -p "$5"
    out=$(absolute_dir "$5")
    linux_settings "$target"
    runtime="$(cfg linux.runtime_packages) $(cfg "package_extras.$package.runtime_packages")"
    local tools status=0
    tools=$(stage_tools "$out")
    extract="$out/.runtime-root"
    rm -rf "$extract"
    mkdir -p "$extract"
    tar -xf "$archive" -C "$extract"
    # Without runtime packages to install, the smoke container has no network.
    local -a network=(--network none)
    [ -z "${runtime// /}" ] || network=()
    # shellcheck disable=SC2016 # expanded inside the container
    bounded "$STEP_TIMEOUT" docker run --rm --platform "$PLATFORM" "${network[@]+"${network[@]}"}" \
        -v "$extract:/dist:ro" -v "$tools:/dist-scripts:ro" -v "$out:/out" \
        -e DEBIAN_FRONTEND=noninteractive -e RUNTIME="$runtime" \
        -e PACKAGE="$package" -e TARGET="$target" -e COMMAND="$command" \
        "$IMAGE" bash -euo pipefail -c '
            if [ -n "${RUNTIME// /}" ]; then
                # shellcheck disable=SC2086
                apt-get update -qq && apt-get install -y -qq --no-install-recommends $RUNTIME >/dev/null
            fi
            bash /dist-scripts/build.sh smoke /dist "$PACKAGE" "$TARGET" "$COMMAND" /out
        ' || status=$?
    rm -rf "$extract" "$tools"
    [ "$status" -eq 0 ] || die "runtime smoke failed for $target (exit $status)"
    "$PYTHON" "$SCRIPT_DIR/inspect_archive.py" ldd "$out/ldd-$package-$target.txt" --package "$package" \
        --target "$target" --config "$CONFIG" --report "$out/ldd-check-$package-$target.json" >/dev/null ||
        die "runtime ldd check failed; see $out/ldd-check-$package-$target.json"
}

main() {
    [ $# -ge 1 ] || usage
    local subcommand=$1
    shift
    case "$subcommand" in
        build) cmd_build "$@" ;;
        smoke) cmd_smoke "$@" ;;
        container) cmd_container "$@" ;;
        smoke-container) cmd_smoke_container "$@" ;;
        -h | --help) usage ;;
        *) usage ;;
    esac
}

main "$@"
