#!/usr/bin/env bash
# kache host decisions, shared by `just init` (_ensure-kache) and
# `just kache-status` (fixes/2026-09-23-ensuring-kache-support, spec §1). Both
# decide cloning through the same `clone_check`: `qualify` runs it from the
# candidate store, `report` from the store `kache doctor` resolves today. The
# two agree only while those stores are the same directory on the same
# filesystem; `report` re-probing is what lets status catch the drift.
#
# Subcommands:
#   qualify           Filesystem qualification for THIS checkout. Resolves the
#                     candidate store directory with the placement cascade's
#                     pure directory logic (a `kache/` directory at the
#                     checkout volume's mount point on macOS when writable,
#                     else the user cache dir when it is on the serving
#                     device, else a user-owned `kache/` directory on that
#                     volume — never a root-owned mount point; Windows first
#                     requires a ReFS checkout volume), resolves the
#                     worktree base the way `wt` does, and runs `clone_check`
#                     from the candidate store into the checkout (and into the
#                     base when configured, reported but never gating the
#                     verdict). The base line, shared with `report`:
#                       kache-host: base=covered|off-device|unconfigured path=P
#                       kache-host: base=invalid reason=R path=P
#                     `invalid` is a setting `wt` itself refuses (R names
#                     the rule, P the offending path or config file); it
#                     does not gate qualification either.
#                     Exit 0 = qualifies, 1 = does not (reason printed),
#                     2 = usage or internal error.
#   report            Post-activation facts: the store path from
#                     `kache doctor --json` (never a reconstruction — the
#                     defect behind the pre-2026-09-23 false verdicts), the
#                     checkout/base device check, the clone check from that
#                     store into the checkout (and into the base when
#                     configured), and the env-passthrough result. Before
#                     asking doctor it reports the daemon state from
#                     `kache daemon --json` and the user config's store pin
#                     and `ignore_env` (see `config-path`), so a doctor
#                     failure still leaves those facts behind:
#                       kache-host: daemon installed=yes|no running=yes|no version=V|-
#                       kache-host: config state=present|absent|unparseable|python3-without-tomllib path=P
#                       kache-host: config ignore_env=true|false|absent
#                       kache-host: config-source scope=cli|daemon state=S selector=K path=P
#                       kache-host: config pin=match|mismatch|absent value=V
#                     `daemon error=…` replaces the daemon line when its state
#                     cannot be read; the pin line follows doctor, because
#                     the pin matches only when `local_store` names the
#                     directory doctor resolves. The config-source lines (see
#                     `config-source`) say whether kache in this environment
#                     and the running daemon load that user config at all.
#                     Exit 0 = report produced, 2 = cannot report.
#   config-path       The user kache config file `_ensure-kache` writes and
#                     `report` reads: `%APPDATA%\kache\config.toml` on
#                     Windows, else `$XDG_CONFIG_HOME/kache/config.toml`
#                     (default `~/.config`).
#   config-source     Whether the store pin in that file reaches kache:
#                       kache-host: config-source scope=cli state=managed|override selector=KACHE_CONFIG|- path=P
#                       kache-host: config-source scope=daemon state=managed|override|unknown selector=daemon_config_path|- path=P|-
#                       kache-host: store=S source=doctor
#                       kache-host: config pin=match|mismatch|absent value=V
#                     `override` names a config file other than the managed
#                     one: a non-empty KACHE_CONFIG in this environment, or
#                     the file the running daemon reports it loaded
#                     (`unknown` when no daemon answers or it names none).
#                     `ignore_env = true` does not stop that selection.
#                     Exit 0 = decided, 2 = kache absent or doctor failed.
#   probe-passthrough Runs a stub compiler through kache with a `DYLD_*`
#                     variable exported and checks whether the variable
#                     arrives: the gate for the macOS ad hoc re-sign, because
#                     a hardened-runtime kache has every `DYLD_*` variable
#                     stripped by dyld at launch and cannot forward what it
#                     never sees. Set KACHE_HOST_BIN to probe a specific kache
#                     binary (e.g. a pristine release fetched to a temp
#                     location) rather than the one on PATH. Exit 0 = pass
#                     (or `n/a` outside macOS), 1 = the variable was
#                     stripped, 2 = the probe could not run.
#   wrapper           The rustc wrapper Cargo would use for THIS checkout, by
#                     Cargo's rules rather than a text search: RUSTC_WRAPPER
#                     wins when set at all (set-but-empty means no wrapper),
#                     then CARGO_BUILD_RUSTC_WRAPPER, then `build.rustc-wrapper`
#                     from `.cargo/config[.toml]` in the working directory and
#                     each of its ancestors (nearer first), then from
#                     `$CARGO_HOME/config[.toml]` (one file per directory,
#                     as Cargo reads it: the legacy `config` whenever it
#                     exists, which hides a `config.toml` beside it; a
#                     `$CARGO_HOME` among the ancestors keeps its place in
#                     that walk, as in Cargo). An
#                     empty value disables wrapping — the neutralized state
#                     `_ensure-kache` writes on failure. One
#                     `kache-host: wrapper-source` line per source that sets
#                     the key, highest precedence first, then the winner:
#                       kache-host: wrapper-source scope=S kind=K source=SRC value=V
#                       kache-host: wrapper=K source=SRC value=V
#                     scope is env|repo|ancestor|host (ancestor: a directory
#                     above the checkout, named by absolute path); kind is
#                     kache (the bare name or a path whose basename is
#                     kache/kache.exe), none (empty), or other; the winner is `wrapper=none source=- value=` when
#                     nothing sets it. A kache winner adds the executable
#                     Cargo would start, resolved by Cargo's rules (bare name
#                     on PATH; a path relative to the working directory for
#                     an environment value, to the directory holding the
#                     config's `.cargo/` for a config value):
#                       kache-host: wrapper-binary state=S path=P certified=C
#                     state is certified (P is the `kache` on PATH, C — the
#                     binary whose version and passthrough are checked),
#                     missing, not-executable, or different; C is `-` when
#                     no kache is on PATH. The name alone certifies nothing:
#                     Cargo fails every build on a missing wrapper.
#                     Exit 0 = decided, 2 = a config file
#                     Cargo would read does not parse (Cargo would fail too)
#                     or python3 predates tomllib (3.11).
#
# Machine-readable facts are one `kache-host: key=value` line each; everything
# else is human-facing prose. Callers parse the `kache-host: ` lines only.
#
# Usage: scripts/kache-host.sh {qualify|report|probe-passthrough|wrapper|config-path|config-source}
set -euo pipefail

PROBE_PREFIX=".kache-host-probe"

usage() {
    echo "usage: $0 {qualify|report|probe-passthrough|wrapper|config-path|config-source}" >&2
    exit 2
}

os_name() {
    case "$(uname -s)" in
        Darwin) echo darwin ;;
        Linux) if grep -qi microsoft /proc/version 2> /dev/null; then echo wsl; else echo linux; fi ;;
        MINGW* | MSYS* | CYGWIN*) echo windows ;;
        *) echo unknown ;;
    esac
}

device_id() {
    case "$(uname -s)" in
        Darwin) stat -f %d "$1" ;;
        *) stat -c %d "$1" ;;
    esac
}

# The mount point of the volume holding $1 (`df -P` puts it last; fields after
# the fifth are joined so a volume name with spaces survives). On Windows it is
# the drive root, in the MSYS spelling (`/b/`) the shell's own tools accept.
mount_point_of() {
    if [[ "$(os_name)" == "windows" ]]; then
        cygpath -u "$(windows_drive "$1"):/"
        return
    fi
    df -P "$1" 2> /dev/null | awk 'NR == 2 { $1 = $2 = $3 = $4 = $5 = ""; sub(/^ +/, ""); print }'
}

windows_drive() {
    cygpath -w "$1" 2> /dev/null | cut -c1
}

# The filesystem type of the Windows volume holding $1, as Get-Volume names it
# (`ReFS`, `NTFS`, …); empty when it cannot be read.
windows_fstype() {
    powershell.exe -NoProfile -NonInteractive -Command \
        "(Get-Volume -DriveLetter $(windows_drive "$1")).FileSystemType" 2> /dev/null | tr -d '\r\n'
}

# The spelling a candidate store is published in: on Windows the one native
# programs (kache.exe, python3) and the MSYS shell both accept (`B:/kache`).
published_path() {
    if [[ "$(os_name)" == "windows" ]]; then
        cygpath -m "$1"
    else
        printf '%s' "$1"
    fi
}

user_config_path() {
    case "$(os_name)" in
        windows) printf '%s/kache/config.toml' "$(cygpath "${APPDATA:?}")" ;;
        *) printf '%s/kache/config.toml' "${XDG_CONFIG_HOME:-$HOME/.config}" ;;
    esac
}

user_cache_dir() {
    case "$(os_name)" in
        darwin) printf '%s/Library/Caches/kache' "$HOME" ;;
        windows) printf '%s/kache' "$(cygpath -u "${LOCALAPPDATA:?}")" ;;
        *) printf '%s/kache' "${XDG_CACHE_HOME:-$HOME/.cache}" ;;
    esac
}

# The worktree base resolved the way `wt` resolves it (worktree/lib/src/
# config.rs `resolve_base_dir`), including every setting `wt` refuses. Sets
# BASE_PATH, BASE_STATE (`configured`, `unconfigured`, or `invalid`), and for
# `invalid` BASE_REASON plus BASE_PATH naming the offending path. A non-empty
# `WT` wins and must exist without a `.git` entry; otherwise
# `~/.worktree.json`, when present, must parse (as serde_json parses it into
# `{ base_dir: String }`) to a path with no `.git` entry that exists. `wt`
# accepts an existing non-directory; kache cannot clone into one, so that is
# `invalid` here too.
worktree_base() {
    BASE_PATH="-"
    BASE_STATE="unconfigured"
    BASE_REASON=""
    local config="$HOME/.worktree.json" base rc=0 prefix
    if [[ -n "${WT:-}" ]]; then
        base="$WT" prefix="wt"
        if [[ ! -e "$base" ]]; then
            BASE_REASON="wt-path-missing"
        elif [[ -e "$base/.git" ]]; then
            BASE_REASON="wt-path-is-a-git-repository"
        fi
    elif [[ -e "$config" ]]; then
        prefix="config-base-dir"
        base="$(python3 - "$config" <<'PY'
import json, sys
try:
    with open(sys.argv[1], "rb") as handle:
        text = handle.read().decode("utf-8")
except (OSError, UnicodeDecodeError):
    sys.exit(4)

class Pairs(list):
    pass

def no_constants(name):
    raise ValueError(name)

# Mirrors serde_json into the derived `WorktreeConfig { base_dir: String }`:
# NaN/Infinity are not JSON, a repeated `base_dir` is an error while other
# keys are ignored, and a derived struct also accepts the one-element array
# form.
try:
    config = json.loads(text, object_pairs_hook=Pairs, parse_constant=no_constants)
    if isinstance(config, Pairs):
        values = [value for key, value in config if key == "base_dir"]
        base = values[0] if len(values) == 1 else None
    elif isinstance(config, list) and len(config) == 1:
        base = config[0]
    else:
        base = None
    if not isinstance(base, str):
        raise ValueError("base_dir")
    base.encode("utf-8")
except (ValueError, UnicodeEncodeError):
    sys.exit(3)
print(base)
PY
)" || rc=$?
        case "$rc" in
            0)
                # wt checks `.git` before existence here, and an empty
                # base_dir joins `.git` onto the working directory.
                if [[ -e "${base:+$base/}.git" ]]; then
                    BASE_REASON="config-base-dir-is-a-git-repository"
                elif [[ ! -e "$base" ]]; then
                    BASE_REASON="config-base-dir-missing"
                fi
                ;;
            3) base="$config" BASE_REASON="config-malformed" ;;
            4) base="$config" BASE_REASON="config-unreadable" ;;
            *) base="$config" BASE_REASON="config-undecidable" ;;
        esac
    else
        return 0
    fi
    [[ -z "$BASE_REASON" && ! -d "$base" ]] && BASE_REASON="${prefix}-not-a-directory"
    BASE_PATH="${base:-\"\"}"
    if [[ -n "$BASE_REASON" ]]; then
        BASE_STATE="invalid"
    else
        BASE_STATE="configured"
    fi
}

# Fills BASE_PATH, BASE_DEVICE, BASE_STATE, and BASE_REASON for the checkout
# $1 relative to its device: `covered` (same device), `off-device`,
# `unconfigured`, or `invalid` (a setting `wt` itself refuses; BASE_REASON
# says why). The base never gates the qualification verdict (2026-09-23
# ruling): init-time gating buys no durable protection because the base can
# be re-pointed after activation regardless; `kache-status` polices it once
# configured, and fails on an invalid one.
resolve_base() {
    BASE_DEVICE="-"
    worktree_base
    if [[ "$BASE_STATE" == "configured" ]]; then
        BASE_DEVICE="$(device_id "$BASE_PATH")"
        if [[ "$BASE_DEVICE" == "$(device_id "$1")" ]]; then
            BASE_STATE="covered"
        else
            BASE_STATE="off-device"
        fi
    fi
}

emit_base_lines() {
    echo "kache-host: devices checkout=$1 store=${2:--} base=${BASE_DEVICE}"
    echo "kache-host: base=${BASE_STATE}${BASE_REASON:+ reason=${BASE_REASON}} path=${BASE_PATH}"
}

# One clone probe from directory $1 into directory $2. Clones do not cross
# volumes, so the probe runs from the store side toward where `target/` dirs
# live; a probe inside target/ alone said "clone" for a checkout on a second
# APFS volume that kache restores by copy (2026-09-09). macOS `cp -c` silently
# falls back to a plain copy across volumes and still exits 0, so the device
# check there is part of the probe, not an afterthought. Returns 0 only when
# the copy is a same-device clone.
clone_probe() {
    local src_dir="$1" dst_dir="$2" src dst ok=0
    src="$(mktemp "$src_dir/${PROBE_PREFIX}.XXXXXX" 2> /dev/null)" || return 1
    if ! dst="$(mktemp "$dst_dir/${PROBE_PREFIX}.XXXXXX" 2> /dev/null)"; then
        rm -f "$src"
        return 1
    fi
    rm -f "$dst"
    case "$(uname -s)" in
        Darwin)
            [[ "$(device_id "$src_dir")" == "$(device_id "$dst_dir")" ]] \
                && cp -c "$src" "$dst" 2> /dev/null || ok=1
            ;;
        *)
            cp --reflink=always "$src" "$dst" &> /dev/null || ok=1
            ;;
    esac
    rm -f "$src" "$dst"
    return "$ok"
}

# The clone decision from store directory $1 into directory $2 (whose role,
# `checkout` or `base`, $3 names in the reason). Returns 0 when restores from
# the store into $2 would be clones; otherwise returns 1 with the deciding fact
# in CLONE_REASON. Device identity alone is not proof that the filesystem
# clones blocks (ext4 shares a device with the store yet cannot reflink — the
# WSL negative case), so the probe decides whenever the devices match.
CLONE_REASON=""
clone_check() {
    local src_dir="$1" dst_dir="$2" role="$3" src_device dst_device fstype
    CLONE_REASON=""
    src_device="$(device_id "$src_dir")"
    dst_device="$(device_id "$dst_dir")"
    if [[ "$src_device" != "$dst_device" ]]; then
        CLONE_REASON="store-device-${src_device}-is-not-the-${role}-device-${dst_device}"
        return 1
    fi
    if [[ "$(os_name)" == "windows" ]]; then
        # No userspace reflink probe exists on Windows: the filesystem type
        # of the destination drive is the answer, as in `qualify`.
        fstype="$(windows_fstype "$dst_dir")"
        [[ "$fstype" == "ReFS" ]] && return 0
        CLONE_REASON="filesystem=${fstype:-unknown}-is-not-ReFS"
        return 1
    fi
    clone_probe "$src_dir" "$dst_dir" && return 0
    fstype="$(df -PT "$dst_dir" 2> /dev/null | awk 'NR == 2 { print $2 }')"
    CLONE_REASON="clone-unsupported${fstype:+-on-${fstype}}"
    return 1
}

# The candidate store directory for the volume serving the checkout $1,
# following the spec §2 cascade: macOS tries a `kache/` directory at the
# volume's mount point first (this host's /Volumes/coding/kache rule), then
# every OS shares one cascade — the user cache dir when it is on the serving
# device, else a user-owned `kache/` directory on that volume — at the mount
# point when this user may create it there, else in the highest user-owned
# ancestor of the checkout below the mount point (rule at that step); never
# the mount point itself (/, /home, … are root-owned). On Windows those are
# `%LOCALAPPDATA%\kache` (kache's default), `kache\` at the drive root, then
# the same ancestor rule — the case for a ReFS Dev Drive beside an NTFS
# system drive. Every step goes through `use_candidate`, so a placement that
# already exists but is unwritable or on another device is skipped, untouched,
# and the cascade moves on. Sets CANDIDATE and, when
# this call created the directory, CREATED_CANDIDATE plus CREATED_TOP, the
# outermost directory it created (a non-qualifying verdict removes them again
# while they are still empty). Returns 1 with a reason in
# CASCADE_REASON when no user-writable placement exists.
CANDIDATE=""
CREATED_CANDIDATE=""
CREATED_TOP=""
CASCADE_REASON=""

# Takes directory $1 as the candidate when it is writable and on device $2,
# creating it (and missing parents) when absent. Only a directory this call
# created is recorded in CREATED_CANDIDATE/CREATED_TOP — a pre-existing one is
# never the cleanup's to remove — and a created one that proves unusable is
# removed again at once. Returns 1, changing nothing that existed, otherwise.
use_candidate() {
    local dir="$1" device="$2" created_top=""
    if [[ ! -d "$dir" ]]; then
        [[ -e "$dir" || -L "$dir" ]] && return 1
        created_top="$dir"
        while [[ ! -d "$(dirname "$created_top")" ]]; do
            created_top="$(dirname "$created_top")"
        done
        mkdir -p "$dir" 2> /dev/null || return 1
    fi
    if [[ -w "$dir" && "$(device_id "$dir")" == "$device" ]]; then
        CANDIDATE="$dir"
        if [[ -n "$created_top" ]]; then
            CREATED_CANDIDATE="$dir"
            CREATED_TOP="$created_top"
        fi
        return 0
    fi
    if [[ -n "$created_top" ]]; then
        CREATED_CANDIDATE="$dir"
        CREATED_TOP="$created_top"
        drop_created_candidate
        CREATED_CANDIDATE=""
        CREATED_TOP=""
    fi
    return 1
}

candidate_store() {
    local checkout="$1" cache_anchor cache_dir checkout_device mount_point try
    checkout_device="$(device_id "$checkout")"

    if [[ "$(uname -s)" == "Darwin" ]]; then
        mount_point="$(mount_point_of "$checkout")"
        try="${mount_point%/}/kache"
        if [[ -n "$mount_point" && "$try" != "$mount_point" ]] \
            && use_candidate "$try" "$checkout_device"; then
            return 0
        fi
    fi

    # The user cache dir when it is already on the serving device — the common
    # single-disk btrfs/XFS case, where it usually equals kache's default
    # anyway. The nearest EXISTING ancestor decides the device, so nothing is
    # created before the device is known and a fresh home without ~/.cache
    # (or ~/Library/Caches) still gets this placement.
    cache_dir="$(user_cache_dir)"
    cache_anchor="$(dirname "$cache_dir")"
    while [[ ! -d "$cache_anchor" && "$cache_anchor" != "/" ]]; do
        cache_anchor="$(dirname "$cache_anchor")"
    done
    if [[ "$(device_id "$cache_anchor")" == "$checkout_device" ]] \
        && use_candidate "$cache_dir" "$checkout_device"; then
        return 0
    fi

    # A user-owned directory on the serving volume: `kache/` at the volume's
    # mount point, when the mount point lets this user create it.
    mount_point="$(mount_point_of "$checkout")"
    try="${mount_point%/}/kache"
    if [[ -n "$mount_point" && "$try" != "$mount_point" ]] \
        && use_candidate "$try" "$checkout_device"; then
        return 0
    fi

    # Else `kache/` in the HIGHEST ancestor of the checkout that lies strictly
    # below the mount point, on the checkout's device, is owned and writable
    # by this user, and is not inside a Git working tree (neither it nor an
    # ancestor below the mount point holds `.git`). Starting above the
    # checkout keeps the store out of the checkout and its `target/`; the
    # highest such directory is the one every checkout beside this one on
    # the volume shares (`/data/src/kache` for a root-owned `/data`).
    [[ -n "$mount_point" ]] || mount_point="/"
    local ancestors=() dir index
    dir="$(dirname "$checkout")"
    while [[ "$dir" != "${mount_point%/}" && "$dir" != "/" && "$dir" != "." ]] \
        && [[ "$(device_id "$dir")" == "$checkout_device" ]]; do
        ancestors+=("$dir")
        dir="$(dirname "$dir")"
    done
    for ((index = ${#ancestors[@]} - 1; index >= 0; index--)); do
        dir="${ancestors[index]}"
        [[ -e "$dir/.git" ]] && break
        [[ -O "$dir" && -w "$dir" ]] || continue
        use_candidate "$dir/kache" "$checkout_device" && return 0
    done

    CASCADE_REASON="no-user-writable-store-location-on-the-checkout-volume"
    return 1
}

# Drop a candidate the cascade created for this probe when the verdict turned
# out to be no: it was a probe artifact, not a placement decision, and only
# empties go — anything that appeared inside it is left strictly alone.
drop_created_candidate() {
    [[ -n "$CREATED_CANDIDATE" && -d "$CREATED_CANDIDATE" ]] || return 0
    [[ -z "$(ls -A "$CREATED_CANDIDATE" 2> /dev/null)" ]] && rmdir "$CREATED_CANDIDATE" 2> /dev/null || return 0
    # Parents `mkdir -p` created on the way (a fresh home's ~/.cache), up to
    # and including CREATED_TOP, go too while empty.
    local dir="$CREATED_CANDIDATE"
    while [[ -n "$CREATED_TOP" && "$dir" != "$CREATED_TOP" ]]; do
        dir="$(dirname "$dir")"
        rmdir "$dir" 2> /dev/null || return 0
    done
}

do_qualify() {
    local checkout candidate
    checkout="$(git rev-parse --show-toplevel 2> /dev/null || pwd)"

    case "$(os_name)" in
        darwin | linux | wsl)
            if ! candidate_store "$checkout"; then
                drop_created_candidate
                echo "kache-host: verdict=no-qualify reason=${CASCADE_REASON}"
                echo "  no placement in the cascade (mount-point kache/, user cache dir, a user-owned"
                echo "  ancestor of the checkout on the volume) is writable by this user; nothing was installed."
                exit 1
            fi
            candidate="$CANDIDATE"
            ;;
        windows)
            # No userspace reflink probe exists on Windows, so the filesystem
            # type IS the answer: ReFS clones blocks, NTFS restores by copy.
            # Checked before the cascade so an NTFS host gets no directory
            # created; the candidate then passes the same `clone_check`.
            local fstype
            fstype="$(windows_fstype "$checkout")"
            if [[ "$fstype" != "ReFS" ]]; then
                echo "kache-host: verdict=no-qualify reason=filesystem=${fstype:-unknown}-is-not-ReFS"
                echo "  Windows earns kache on ReFS only; ${fstype:-an unknown filesystem} restores by copy."
                exit 1
            fi
            if ! candidate_store "$checkout"; then
                drop_created_candidate
                echo "kache-host: verdict=no-qualify reason=${CASCADE_REASON}"
                echo "  no placement in the cascade (user cache dir, kache/ at the drive root, a"
                echo "  user-owned ancestor of the checkout) is writable by this user; nothing was installed."
                exit 1
            fi
            candidate="$CANDIDATE"
            ;;
        *)
            echo "kache-host: verdict=no-qualify reason=unsupported-os-$(uname -s)"
            exit 1
            ;;
    esac

    local store_device checkout_device
    checkout_device="$(device_id "$checkout")"
    store_device="$(device_id "$candidate")"

    if ! clone_check "$candidate" "$checkout" checkout; then
        drop_created_candidate
        echo "kache-host: verdict=no-qualify reason=${CLONE_REASON}"
        echo "  candidate store $candidate cannot clone into $checkout; restores would be copies."
        exit 1
    fi

    resolve_base "$checkout"
    echo "kache-host: verdict=qualify candidate=$(published_path "$candidate")"
    emit_base_lines "$checkout_device" "$store_device"

    if [[ "$BASE_STATE" == "covered" ]]; then
        if clone_check "$candidate" "$BASE_PATH" base; then
            echo "  worktree base $BASE_PATH is on the serving device; clone check store -> base: clone"
        else
            echo "  worktree base $BASE_PATH is on the serving device; clone check store -> base: FAILED (${CLONE_REASON})"
        fi
    elif [[ "$BASE_STATE" == "off-device" ]]; then
        echo "  worktree base $BASE_PATH (device ${BASE_DEVICE}) is on ANOTHER device than the store"
        echo "  (device ${store_device}): no placement can serve both; kache-status reports this."
    elif [[ "$BASE_STATE" == "invalid" ]]; then
        echo "  worktree base setting is INVALID (${BASE_REASON}: ${BASE_PATH}) — wt refuses it;"
        echo "  not covered by this verdict, and kache-status fails until it is fixed."
    else
        echo "  worktree base unconfigured — not covered by this verdict."
    fi
    exit 0
}

do_report() {
    command -v kache &> /dev/null || { echo "kache-host: error=kache-absent"; exit 2; }
    local checkout store config
    checkout="$(git rev-parse --show-toplevel 2> /dev/null || pwd)"
    config="$(user_config_path)"
    emit_daemon_line
    emit_config_lines "$config"
    emit_config_source_lines "$config"
    # The store is whatever kache says it is: kache-status once reconstructed
    # kache's resolution rules itself (KACHE_DIR, then the default cache dir)
    # and reported a false verdict for weeks — doctor is the only authority.
    store="$(doctor_store)" || { echo "kache-host: error=doctor-failed"; exit 2; }
    [[ -n "$store" ]] || { echo "kache-host: error=doctor-no-store"; exit 2; }

    resolve_base "$checkout"
    echo "kache-host: store=${store} source=doctor"
    emit_config_lines "$config" "$store"
    emit_base_lines "$(device_id "$checkout")" "$(device_id "$store")"
    if clone_check "$store" "$checkout" checkout; then
        echo "kache-host: clone checkout=clone"
    else
        echo "kache-host: clone checkout=copy reason=${CLONE_REASON}"
    fi
    if [[ "$BASE_STATE" == "unconfigured" || "$BASE_STATE" == "invalid" ]]; then
        echo "kache-host: clone base=- reason=${BASE_STATE}"
    elif clone_check "$store" "$BASE_PATH" base; then
        echo "kache-host: clone base=clone"
    else
        echo "kache-host: clone base=copy reason=${CLONE_REASON}"
    fi

    if [[ "$(os_name)" == "darwin" ]]; then
        local probe_code=0
        run_passthrough_probe || probe_code=$?
        if [[ "$probe_code" -eq 0 ]]; then
            echo "kache-host: passthrough=pass"
        elif [[ "$probe_code" -eq 2 ]]; then
            echo "kache-host: passthrough=error"
        else
            echo "kache-host: passthrough=fail"
        fi
    else
        echo "kache-host: passthrough=n/a reason=not-macos"
    fi
    exit 0
}

# The store `kache doctor --json` resolves in this environment. kache 0.26.3
# appends ` (will be created on first build)` to the detail while the
# directory does not exist yet; that note is not part of the path.
doctor_store() {
    kache doctor --json 2> /dev/null | python3 -c '
import json, sys
try:
    doctor = json.load(sys.stdin)
except Exception:
    raise SystemExit(1)
for check in doctor.get("checks", []):
    if check.get("label") == "Cache dir" and check.get("pass"):
        print(check.get("detail", "").removesuffix(" (will be created on first build)"))
        raise SystemExit(0)
raise SystemExit(1)
'
}

do_config_source() {
    command -v kache &> /dev/null || { echo "kache-host: error=kache-absent"; exit 2; }
    local config store
    config="$(user_config_path)"
    emit_config_source_lines "$config"
    store="$(doctor_store)" || { echo "kache-host: error=doctor-failed"; exit 2; }
    [[ -n "$store" ]] || { echo "kache-host: error=doctor-no-store"; exit 2; }
    echo "kache-host: store=${store} source=doctor"
    emit_config_lines "$config" "$store"
    exit 0
}

# Which config file the CLI in this environment and the running daemon load,
# against the managed one ($1). Measured on kache 0.26.3 (2026-09-24): a
# non-empty KACHE_CONFIG selects its file — a missing one leaves kache on
# built-in defaults rather than falling back — and `ignore_env = true` in the
# managed file does not gate that selection; an empty KACHE_CONFIG is ignored.
# `kache doctor --json` names no config path, so the CLI side is read from the
# selector; the daemon reports the file it loaded as `daemon_config_path` in
# `kache daemon --json` (null when no daemon answers). Paths compare after
# `~` expansion and symlink resolution.
emit_config_source_lines() {
    local daemon_json
    daemon_json="$(kache daemon --json 2> /dev/null)" || true
    python3 - "$1" "${KACHE_CONFIG:-}" "$daemon_json" << 'PY'
import json, os, sys

managed, selected, daemon_json = sys.argv[1:4]
same = lambda p: os.path.normcase(os.path.realpath(os.path.expanduser(p)))

def line(scope, path, selector):
    state = "managed" if same(path) == same(managed) else "override"
    print(f"kache-host: config-source scope={scope} state={state} selector={selector} path={path}")

if selected:
    line("cli", selected, "KACHE_CONFIG")
else:
    print(f"kache-host: config-source scope=cli state=managed selector=- path={managed}")

try:
    daemon = json.loads(daemon_json)
except ValueError:
    daemon = None
if not isinstance(daemon, dict):
    daemon = {}
loaded = daemon.get("daemon_config_path")
if daemon.get("daemon_running") and isinstance(loaded, str) and loaded:
    line("daemon", loaded, "daemon_config_path")
else:
    print("kache-host: config-source scope=daemon state=unknown selector=- path=-")
PY
}

# The daemon as `kache daemon --json` reports it — the source `_ensure-kache`
# restarts from — because doctor's `Daemon version` check is optional and
# folds "not installed" and "not running" into one failure.
emit_daemon_line() {
    local json state
    json="$(kache daemon --json 2> /dev/null)" || true
    if state="$(python3 -c '
import json, sys
try:
    d = json.load(sys.stdin)
except Exception:
    raise SystemExit(1)
version = d.get("daemon_version")
print("installed=" + ("yes" if d.get("service_installed") else "no")
      + " running=" + ("yes" if d.get("daemon_running") else "no")
      + " version=" + (version if isinstance(version, str) and version else "-"))
' <<< "$json")"; then
        echo "kache-host: daemon $state"
    else
        echo "kache-host: daemon error=daemon-status-unreadable"
    fi
}

# The user config's `[cache]` pin, read with tomllib. Without a store ($2),
# prints the state and `ignore_env` lines; with one, prints only the pin line,
# comparing `local_store` to it after `~` expansion and symlink resolution.
emit_config_lines() {
    python3 - "$@" << 'PY'
import os, sys

path = sys.argv[1]
store = sys.argv[2] if len(sys.argv) > 2 else None
cache = {}
state = "absent"
if os.path.isfile(path):
    try:
        import tomllib
    except ImportError:
        tomllib = None
        state = "python3-without-tomllib"
    if tomllib:
        try:
            with open(path, "rb") as handle:
                cache = tomllib.load(handle).get("cache", {})
            state = "present"
        except (OSError, tomllib.TOMLDecodeError):
            state = "unparseable"
if not isinstance(cache, dict):
    cache = {}

if store is None:
    print(f"kache-host: config state={state} path={path}")
    ignore_env = cache.get("ignore_env")
    if ignore_env is None:
        print("kache-host: config ignore_env=absent")
    else:
        print("kache-host: config ignore_env=" + ("true" if ignore_env is True else "false"))
    sys.exit(0)

value = cache.get("local_store")
if not isinstance(value, str) or not value:
    print("kache-host: config pin=absent value=")
    sys.exit(0)
same = lambda p: os.path.realpath(os.path.expanduser(p))
print(f"kache-host: config pin={'match' if same(value) == same(store) else 'mismatch'} value={value}")
PY
}

# 0 = the DYLD_* variable arrived at the wrapped compiler, 1 = it was
# stripped, 2 = the probe could not run.
#
# The observer is a copy of /usr/bin/printenv re-signed ad hoc, standing in
# for rustc, and the variable NAME rides in as the compiler argument:
# `kache rustc DYLD_FALLBACK_LIBRARY_PATH` makes the wrapped "compiler" print
# the variable's value exactly when it survived kache. Both parts of that
# construction are load-bearing, measured 2026-09-23: an unmodified observer
# can never see the variable — dyld strips DYLD_* at the exec of every Apple
# platform binary (bash, env, printenv itself), and bash additionally expunges
# DYLD_* from its own environment at startup — and kache rejects a bare
# `kache rustc` with no compiler arguments. A control invocation with a
# non-DYLD variable's name proves the stub ran, separating "stripped" from
# "the probe never executed".
run_passthrough_probe() {
    local kache_bin scratch ctl dy
    kache_bin="${KACHE_HOST_BIN:-kache}"
    command -v "$kache_bin" &> /dev/null || { echo "kache-host: error=kache-binary-missing" >&2; return 2; }
    command -v codesign &> /dev/null || { echo "kache-host: error=codesign-missing" >&2; return 2; }
    [[ -x /usr/bin/printenv ]] || { echo "kache-host: error=printenv-missing" >&2; return 2; }
    scratch="$(mktemp -d "${TMPDIR:-/tmp}/kache-passthrough.XXXXXX")" || return 2
    trap 'rm -rf "$scratch"' RETURN
    cp /usr/bin/printenv "$scratch/rustc" || return 2
    codesign --force -s - "$scratch/rustc" &> /dev/null || return 2

    run_stub() {
        (unset KACHE_DISABLED; export KACHE_HOST_PROBE_CONTROL="kache-host-probe-ran" \
            DYLD_FALLBACK_LIBRARY_PATH=".kache-host-dyld-survivor" PATH="$scratch:${PATH}"; \
            "$kache_bin" rustc "$1") 2> /dev/null | tr -d '\n' || true
    }
    ctl="$(run_stub KACHE_HOST_PROBE_CONTROL)"
    [[ "$ctl" == "kache-host-probe-ran" ]] || { echo "kache-host: error=stub-never-ran" >&2; return 2; }
    dy="$(run_stub DYLD_FALLBACK_LIBRARY_PATH)"
    [[ "$dy" == ".kache-host-dyld-survivor" ]]
}

do_probe_passthrough() {
    if [[ "$(os_name)" != "darwin" ]]; then
        echo "kache-host: passthrough=n/a reason=not-macos"
        exit 0
    fi
    local probe_code=0
    run_passthrough_probe || probe_code=$?
    case "$probe_code" in
        0)
            echo "kache-host: passthrough=pass"
            exit 0
            ;;
        1)
            echo "kache-host: passthrough=fail"
            echo "  the wrapped compiler did not receive DYLD_FALLBACK_LIBRARY_PATH: dyld stripped it"
            echo "  from a hardened-runtime kache (codesign -dvv shows flags=0x10000(runtime)); re-sign"
            echo "  ad hoc with: codesign --force -s - \$(command -v kache)"
            exit 1
            ;;
        *)
            echo "kache-host: error=passthrough-probe-could-not-run"
            exit 2
            ;;
    esac
}

do_wrapper() {
    python3 - "${CARGO_HOME:-$HOME/.cargo}" <<'PY'
import os, shutil, sys

try:
    import tomllib
except ImportError:
    print("kache-host: error=python3-without-tomllib")
    sys.exit(2)

cargo_home = sys.argv[1]

def kind(value):
    if value == "":
        return "none"
    name = value.replace("\\", "/").rsplit("/", 1)[-1].lower()
    return "kache" if name in ("kache", "kache.exe") else "other"

sources = []
for variable in ("RUSTC_WRAPPER", "CARGO_BUILD_RUSTC_WRAPPER"):
    if variable in os.environ:
        sources.append(("env", variable, os.environ[variable]))

# Cargo's own search order: `.cargo/` in the working directory and then in
# every ancestor, nearer first, then `$CARGO_HOME` unless the walk already
# read it. A `$CARGO_HOME` that is an ancestor's `.cargo/` keeps its place in
# the walk but is reported as the host file, under the path callers rewrite.
# `config` is listed last so a `config -> config.toml` symlink is reported
# under the name Cargo reads.
host_files = {os.path.realpath(f"{cargo_home}/{name}"): f"{cargo_home}/{name}"
              for name in ("config.toml", "config")}
directories = [("repo", ".cargo")]
cwd = os.getcwd()
ancestor = os.path.dirname(cwd)
while True:
    directories.append(("ancestor", os.path.join(ancestor, ".cargo")))
    parent = os.path.dirname(ancestor)
    if parent == ancestor:
        break
    ancestor = parent
directories.append(("host", cargo_home))

seen = set()
for scope, directory in directories:
    # Cargo reads ONE file per directory: the legacy extensionless `config`
    # whenever it exists — a `config.toml` beside it is ignored, with only a
    # warning — so the ignored file must not contribute a wrapper here. A
    # `config` that is a directory fails Cargo's read, and fails this one.
    path = f"{directory}/config"
    if not os.path.exists(path):
        path = f"{directory}/config.toml"
        if not os.path.isfile(path):
            continue
    if os.path.realpath(path) in seen:
        continue
    seen.add(os.path.realpath(path))
    if os.path.realpath(path) in host_files:
        scope, path = "host", host_files[os.path.realpath(path)]
    try:
        with open(path, "rb") as handle:
            build = tomllib.load(handle).get("build", {})
    except (OSError, tomllib.TOMLDecodeError):
        print(f"kache-host: error=unparseable-cargo-config path={path}")
        sys.exit(2)
    value = build.get("rustc-wrapper") if isinstance(build, dict) else None
    if value is None:
        continue
    if not isinstance(value, str):
        print(f"kache-host: error=non-string-rustc-wrapper path={path}")
        sys.exit(2)
    sources.append((scope, path, value))

for scope, source, value in sources:
    print(f"kache-host: wrapper-source scope={scope} kind={kind(value)} source={source} value={value}")
if not sources:
    print("kache-host: wrapper=none source=- value=")
    sys.exit(0)
scope, source, value = sources[0]
print(f"kache-host: wrapper={kind(value)} source={source} value={value}")
if kind(value) != "kache":
    sys.exit(0)

# A kache-named wrapper is certified only when it is the executable Cargo
# would start AND the `kache` on PATH — the binary install-kache re-signed and
# whose version and DYLD_* passthrough the recipes check. Cargo's resolution:
# a bare name is looked up on PATH; a value with a path separator is joined
# onto the working directory (environment) or onto the directory holding the
# config file's `.cargo/` (config), which leaves an absolute path unchanged.
if "/" in value or "\\" in value:
    root = os.getcwd() if scope == "env" else os.path.dirname(os.path.dirname(source))
    resolved = os.path.join(root, value)
    # Rust's Command on Windows appends `.exe` to an extensionless program.
    if (os.name == "nt" and not os.path.splitext(resolved)[1]
            and not os.path.isfile(resolved) and os.path.isfile(resolved + ".exe")):
        resolved += ".exe"
else:
    resolved = shutil.which(value) or value
certified = shutil.which("kache")
if not os.path.isfile(resolved):
    state = "missing"
elif not os.access(resolved, os.X_OK):
    state = "not-executable"
elif certified and os.path.normcase(os.path.realpath(resolved)) == os.path.normcase(os.path.realpath(certified)):
    state = "certified"
else:
    state = "different"
print(f"kache-host: wrapper-binary state={state} path={resolved} certified={certified or '-'}")
PY
}

case "${1:-}" in
    qualify) do_qualify ;;
    report) do_report ;;
    probe-passthrough) do_probe_passthrough ;;
    wrapper) do_wrapper ;;
    config-path) user_config_path ;;
    config-source) do_config_source ;;
    *) usage ;;
esac
