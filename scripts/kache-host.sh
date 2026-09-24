#!/usr/bin/env bash
# kache host decisions, shared by `just init` (_ensure-kache) and
# `just kache-status` so the qualification verdict and the status report can
# never disagree (fixes/2026-09-23-ensuring-kache-support, spec §1).
#
# Subcommands:
#   qualify           Filesystem qualification for THIS checkout. Resolves the
#                     candidate store directory with the placement cascade's
#                     pure directory logic (a `kache/` directory at the
#                     checkout volume's mount point on macOS when writable,
#                     else the user cache dir when it is on the serving
#                     device, else a user-owned `kache/` directory on that
#                     volume — never a root-owned mount point), resolves the
#                     worktree base the way `wt` does, and clone-probes from
#                     the candidate store into the checkout (and into the base
#                     when configured, reported but never gating the verdict).
#                     Exit 0 = qualifies, 1 = does not (reason printed),
#                     2 = usage or internal error.
#   report            Post-activation facts: the store path from
#                     `kache doctor --json` (never a reconstruction — the
#                     defect behind the pre-2026-09-23 false verdicts), the
#                     checkout/base device check, and the env-passthrough
#                     result. Exit 0 = report produced, 2 = cannot report.
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
#
# Machine-readable facts are one `kache-host: key=value` line each; everything
# else is human-facing prose. Callers parse the `kache-host: ` lines only.
#
# Usage: scripts/kache-host.sh {qualify|report|probe-passthrough}
set -euo pipefail

PROBE_PREFIX=".kache-host-probe"

usage() {
    echo "usage: $0 {qualify|report|probe-passthrough}" >&2
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
# the fifth are joined so a volume name with spaces survives).
mount_point_of() {
    df -P "$1" 2> /dev/null | awk 'NR == 2 { $1 = $2 = $3 = $4 = $5 = ""; sub(/^ +/, ""); print }'
}

user_cache_dir() {
    case "$(uname -s)" in
        Darwin) printf '%s/Library/Caches/kache' "$HOME" ;;
        *) printf '%s/kache' "${XDG_CACHE_HOME:-$HOME/.cache}" ;;
    esac
}

# The worktree base resolved the way `wt` resolves it (worktree/lib/src
# config.rs): the WT environment variable first, then `base_dir` in
# ~/.worktree.json. Prints the path and returns 0 only when it names an
# existing directory; the caller decides how to report the alternatives.
worktree_base() {
    local candidate=""
    if [[ -n "${WT:-}" ]]; then
        candidate="$WT"
    elif [[ -f "$HOME/.worktree.json" ]]; then
        candidate="$(python3 - "$HOME/.worktree.json" <<'PY'
import json, sys
try:
    with open(sys.argv[1]) as handle:
        base = json.load(handle).get("base_dir")
    if base:
        print(base)
except Exception:
    pass
PY
)"
    fi
    [[ -n "$candidate" && -d "$candidate" ]] || return 1
    printf '%s' "$candidate"
}

# Fills BASE_PATH, BASE_DEVICE, and BASE_STATE for the checkout $1 relative to
# its device: `covered` (same device), `off-device`, or `unconfigured`. The
# base never gates the qualification verdict (2026-09-23 ruling): init-time
# gating buys no durable protection because the base can be re-pointed after
# activation regardless; `kache-status` polices it once configured.
resolve_base() {
    BASE_PATH="-"
    BASE_DEVICE="-"
    BASE_STATE="unconfigured"
    local base
    if base="$(worktree_base)"; then
        BASE_PATH="$base"
        BASE_DEVICE="$(device_id "$base")"
        if [[ "$BASE_DEVICE" == "$(device_id "$1")" ]]; then
            BASE_STATE="covered"
        else
            BASE_STATE="off-device"
        fi
    fi
}

emit_base_lines() {
    echo "kache-host: devices checkout=$1 store=${2:--} base=${BASE_DEVICE}"
    echo "kache-host: base=${BASE_STATE} path=${BASE_PATH}"
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

# The candidate store directory for the volume serving the checkout $1,
# following the spec §2 cascade: macOS tries a `kache/` directory at the
# volume's mount point first (this host's /Volumes/coding/kache rule), then
# both OSes share one cascade — the user cache dir when it is on the serving
# device, else a user-owned `kache/` directory on that volume; never the
# mount point itself (/, /home, … are root-owned). Sets CANDIDATE and, when
# this call created the directory, CREATED_CANDIDATE (a non-qualifying verdict
# removes it again while it is still empty). Returns 1 with a reason in
# CASCADE_REASON when no user-writable placement exists.
CANDIDATE=""
CREATED_CANDIDATE=""
CASCADE_REASON=""
candidate_store() {
    local checkout="$1" cache_parent cache_dir mount_point try

    if [[ "$(uname -s)" == "Darwin" ]]; then
        mount_point="$(mount_point_of "$checkout")"
        try="${mount_point%/}/kache"
        if [[ -n "$mount_point" && "$try" != "$mount_point" && -d "$try" && -w "$try" ]]; then
            CANDIDATE="$try"
            return 0
        fi
        if [[ -n "$mount_point" && "$try" != "$mount_point" ]] && mkdir -p "$try" 2> /dev/null; then
            CANDIDATE="$try"
            CREATED_CANDIDATE="$try"
            return 0
        fi
    fi

    # The user cache dir when it is already on the serving device — the common
    # single-disk btrfs/XFS case, where it usually equals kache's default
    # anyway. The parent decides the device so nothing is created before the
    # device is known.
    cache_dir="$(user_cache_dir)"
    cache_parent="$(dirname "$cache_dir")"
    if [[ "$cache_parent" == "/" ]]; then
        cache_parent="$cache_dir"
    fi
    if [[ -d "$cache_parent" ]] \
        && [[ "$(device_id "$cache_parent")" == "$(device_id "$checkout")" ]] \
        && mkdir -p "$cache_dir" 2> /dev/null && [[ -w "$cache_dir" ]]; then
        CANDIDATE="$cache_dir"
        return 0
    fi

    # A user-owned directory on the serving volume: `kache/` at the volume's
    # mount point, when the mount point lets this user create it.
    mount_point="$(mount_point_of "$checkout")"
    try="${mount_point%/}/kache"
    if [[ -n "$mount_point" && "$try" != "$mount_point" ]] && mkdir -p "$try" 2> /dev/null; then
        CANDIDATE="$try"
        CREATED_CANDIDATE="$try"
        return 0
    fi

    CASCADE_REASON="no-user-writable-store-location-on-the-checkout-volume"
    return 1
}

# Drop a candidate the cascade created for this probe when the verdict turned
# out to be no: it was a probe artifact, not a placement decision, and only
# empties go — anything that appeared inside it is left strictly alone.
drop_created_candidate() {
    [[ -n "$CREATED_CANDIDATE" && -d "$CREATED_CANDIDATE" ]] || return 0
    [[ -z "$(ls -A "$CREATED_CANDIDATE" 2> /dev/null)" ]] && rmdir "$CREATED_CANDIDATE" 2> /dev/null || true
}

do_qualify() {
    local checkout candidate reason fstype
    checkout="$(git rev-parse --show-toplevel 2> /dev/null || pwd)"

    case "$(os_name)" in
        darwin | linux | wsl)
            if ! candidate_store "$checkout"; then
                drop_created_candidate
                echo "kache-host: verdict=no-qualify reason=${CASCADE_REASON}"
                echo "  no placement in the cascade (mount-point kache/, user cache dir, user-owned"
                echo "  directory on the volume) is writable by this user; nothing was installed."
                exit 1
            fi
            candidate="$CANDIDATE"
            ;;
        windows)
            # No userspace reflink probe exists on Windows, so the filesystem
            # type IS the answer: ReFS clones blocks, NTFS restores by copy
            # (verbatim from the kache-status recipe; shipped unexercised per
            # the 2026-09-23 ruling — no Windows host exists to test on).
            local drive fstype
            drive="$(pwd -W 2> /dev/null | cut -c1)"
            fstype="$(powershell.exe -NoProfile -NonInteractive -Command \
                "(Get-Volume -DriveLetter $drive).FileSystemType" 2> /dev/null | tr -d '\r\n')"
            if [[ "$fstype" == "ReFS" ]]; then
                # Store placement on Windows is decided after install by
                # `kache doctor --json` (a ReFS Dev Drive usually serves the
                # default store already), so no candidate directory here.
                resolve_base "$checkout"
                echo "kache-host: verdict=qualify candidate=-"
                emit_base_lines "$(device_id "$checkout")" "-"
                echo "  filesystem ${drive}: is ReFS; store placement follows kache doctor after install."
                exit 0
            fi
            echo "kache-host: verdict=no-qualify reason=filesystem=${fstype:-unknown}-is-not-ReFS"
            echo "  Windows earns kache on ReFS only; ${fstype:-an unknown filesystem} restores by copy."
            exit 1
            ;;
        *)
            echo "kache-host: verdict=no-qualify reason=unsupported-os-$(uname -s)"
            exit 1
            ;;
    esac

    local store_device checkout_device
    checkout_device="$(device_id "$checkout")"
    store_device="$(device_id "$candidate")"

    # The clone probe is the verdict: device identity alone is not proof that
    # the filesystem clones blocks (ext4 shares a device with the store yet
    # cannot reflink — the WSL negative case).
    if [[ "$store_device" != "$checkout_device" ]] || ! clone_probe "$candidate" "$checkout"; then
        if [[ "$store_device" != "$checkout_device" ]]; then
            reason="store-device-${store_device}-is-not-the-checkout-device-${checkout_device}"
        else
            fstype="$(df -PT "$checkout" 2> /dev/null | awk 'NR == 2 { print $2 }')"
            reason="clone-unsupported${fstype:+-on-${fstype}}"
        fi
        drop_created_candidate
        echo "kache-host: verdict=no-qualify reason=${reason}"
        echo "  candidate store $candidate cannot clone into $checkout; restores would be copies."
        exit 1
    fi

    resolve_base "$checkout"
    echo "kache-host: verdict=qualify candidate=${candidate}"
    emit_base_lines "$checkout_device" "$store_device"

    if [[ "$BASE_STATE" == "covered" ]]; then
        if clone_probe "$candidate" "$BASE_PATH"; then
            echo "  worktree base $BASE_PATH is on the serving device; clone probe store -> base: clone"
        else
            echo "  worktree base $BASE_PATH is on the serving device; clone probe store -> base: FAILED"
        fi
    elif [[ "$BASE_STATE" == "off-device" ]]; then
        echo "  worktree base $BASE_PATH (device ${BASE_DEVICE}) is on ANOTHER device than the store"
        echo "  (device ${store_device}): no placement can serve both; kache-status reports this."
    else
        echo "  worktree base unconfigured — not covered by this verdict."
    fi
    exit 0
}

do_report() {
    command -v kache &> /dev/null || { echo "kache-host: error=kache-absent"; exit 2; }
    local checkout store
    checkout="$(git rev-parse --show-toplevel 2> /dev/null || pwd)"
    # The store is whatever kache says it is: kache-status once reconstructed
    # kache's resolution rules itself (KACHE_DIR, then the default cache dir)
    # and reported a false verdict for weeks — doctor is the only authority.
    store="$(kache doctor --json 2> /dev/null | python3 -c '
import json, sys
try:
    doctor = json.load(sys.stdin)
except Exception:
    raise SystemExit(1)
for check in doctor.get("checks", []):
    if check.get("label") == "Cache dir" and check.get("pass"):
        print(check.get("detail", ""))
        raise SystemExit(0)
raise SystemExit(1)
')" || { echo "kache-host: error=doctor-failed"; exit 2; }
    [[ -n "$store" ]] || { echo "kache-host: error=doctor-no-store"; exit 2; }

    resolve_base "$checkout"
    echo "kache-host: store=${store} source=doctor"
    emit_base_lines "$(device_id "$checkout")" "$(device_id "$store")"

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

case "${1:-}" in
    qualify) do_qualify ;;
    report) do_report ;;
    probe-passthrough) do_probe_passthrough ;;
    *) usage ;;
esac
