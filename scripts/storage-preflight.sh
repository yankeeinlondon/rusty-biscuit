#!/usr/bin/env bash
# Reclaim rebuildable Cargo artifacts before refusing storage-heavy Windows work.
set -euo pipefail

case "$(uname -s)" in
    CYGWIN*|MINGW*|MSYS*) ;;
    *) exit 0 ;;
esac

minimum_gib="${BISCUIT_BUILD_MIN_FREE_GIB:-50}"
if [[ ! "$minimum_gib" =~ ^[0-9]+$ ]]; then
    echo "BISCUIT_BUILD_MIN_FREE_GIB must be a non-negative integer, got: $minimum_gib" >&2
    exit 2
fi
if (( minimum_gib == 0 )); then
    exit 0
fi

auto_sweep="${BISCUIT_BUILD_AUTO_SWEEP:-1}"
if [[ ! "$auto_sweep" =~ ^[01]$ ]]; then
    echo "BISCUIT_BUILD_AUTO_SWEEP must be 0 or 1, got: $auto_sweep" >&2
    exit 2
fi

sweep_max_gb="${BISCUIT_BUILD_SWEEP_MAX_GB:-80}"
if [[ ! "$sweep_max_gb" =~ ^[0-9]+$ ]] || (( sweep_max_gb < 1 || sweep_max_gb > 1000 )); then
    echo "BISCUIT_BUILD_SWEEP_MAX_GB must be an integer from 1 to 1000, got: $sweep_max_gb" >&2
    exit 2
fi

if ! command -v jq >/dev/null 2>&1; then
    echo "Storage preflight requires jq; run 'just init' first." >&2
    exit 2
fi

target_dir="$(cargo metadata --no-deps --format-version 1 | jq -r '.target_directory')"
if command -v cygpath >/dev/null 2>&1; then
    probe_path="$(cygpath -u "$target_dir")"
else
    probe_path="$target_dir"
fi

# A clean checkout may not have created target/ yet; inspect its nearest
# existing parent because that is the filesystem Cargo will consume.
while [[ ! -e "$probe_path" ]]; do
    parent="$(dirname "$probe_path")"
    if [[ "$parent" == "$probe_path" ]]; then
        echo "Cannot resolve an existing parent for Cargo target directory: $target_dir" >&2
        exit 2
    fi
    probe_path="$parent"
done

measure_space() {
    read -r target_fs total_kib used_kib free_kib < <(
        df -Pk "$probe_path" | awk 'END { print $1, $2, $3, $4 }'
    )
    if [[ ! "$free_kib" =~ ^[0-9]+$ ]]; then
        echo "Cannot determine free space for Cargo target directory: $target_dir" >&2
        exit 2
    fi
}

measure_space

# BasePath is the only authority for a relocated distribution; `wsl --list`
# never reports it. MSYS_NO_PATHCONV is load-bearing -- without it Git Bash
# rewrites reg.exe's `/s` switch into a Windows path and the query fails.
list_wsl2_distributions() {
    MSYS_NO_PATHCONV=1 reg.exe query \
        'HKEY_CURRENT_USER\Software\Microsoft\Windows\CurrentVersion\Lxss' /s 2> /dev/null \
        | tr -d '\r' \
        | awk '
            function flush() {
                if (name != "" && base != "" && ver == "0x2") printf "%s\t%s\n", name, base
                name = ""; base = ""; ver = ""
            }
            /^HKEY_/ { flush(); next }
            $1 == "DistributionName" { $1=""; $2=""; sub(/^[ \t]+/, ""); name = $0; next }
            $1 == "BasePath"         { $1=""; $2=""; sub(/^[ \t]+/, ""); base = $0; next }
            $1 == "Version"          { ver = $3; next }
            END { flush() }
        ' || true
}

# A WSL2 ext4.vhdx grows to its guest's high-water mark and never shrinks, so it
# can consume the target volume in a way `just sweep` structurally cannot
# reclaim. Naming it here is what stops a failure from being misread as a Cargo
# problem. Runs only on the failure path, so the gate stays one `df` when it
# passes. Guest usage requires the distribution running, and is reported only
# when that is already true -- a diagnostic must not boot a VM as a side effect.
report_wsl_vhdx() {
    command -v reg.exe > /dev/null 2>&1 || return 0
    command -v cygpath > /dev/null 2>&1 || return 0
    command -v wsl.exe > /dev/null 2>&1 || return 0

    local running name base vhdx size_bytes used_bytes
    running="$(WSL_UTF8=1 wsl.exe --list --running --quiet 2> /dev/null | tr -d '\r' || true)"

    while IFS=$'\t' read -r name base; do
        [[ -n "$base" ]] || continue
        vhdx="$(cygpath -u "${base%\\}\\ext4.vhdx" 2> /dev/null || true)"
        [[ -n "$vhdx" && -f "$vhdx" ]] || continue

        # Only the volume Cargo is about to build on is relevant.
        [[ "$(df -Pk "$vhdx" 2> /dev/null | awk 'END { print $1 }')" == "$target_fs" ]] || continue

        size_bytes="$(stat -c %s "$vhdx" 2> /dev/null || echo 0)"
        (( size_bytes >= 10737418240 )) || continue

        used_bytes=""
        if printf '%s\n' "$running" | grep -qxF "$name"; then
            # MSYS_NO_PATHCONV again: Git Bash rewrites the guest's `/` into the
            # Windows path of its own install root before wsl.exe ever sees it.
            used_bytes="$(
                MSYS_NO_PATHCONV=1 WSL_UTF8=1 \
                    wsl.exe -d "$name" -u root --exec /bin/df -B1 / 2> /dev/null \
                    | awk 'END { print $3 }' || true
            )"
        fi

        echo
        if [[ "$used_bytes" =~ ^[0-9]+$ ]] && (( used_bytes < size_bytes )); then
            awk -v n="$name" -v s="$size_bytes" -v u="$used_bytes" 'BEGIN {
                printf "  %s ext4.vhdx: %.1f GiB on disk, %.1f GiB in use by the guest\n",
                       n, s / 1073741824, u / 1073741824
                printf "    ~%.1f GiB reclaimable -- run '\''just wsl-compact'\''\n",
                       (s - u) / 1073741824
            }'
        else
            awk -v n="$name" -v s="$size_bytes" -v t="$total_kib" 'BEGIN {
                printf "  %s ext4.vhdx: %.1f GiB on disk, %.0f%% of this volume\n",
                       n, s / 1073741824, (s / 1024) / t * 100
                printf "    check reclaimable space with '\''just wsl-compact-status'\''\n"
            }'
        fi
    done < <(list_wsl2_distributions)
}

required_kib=$(( minimum_gib * 1024 * 1024 ))
reclaim_report=""
if (( free_kib < required_kib && auto_sweep == 1 )); then
    script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
    sweep_script="$script_dir/windows-cargo-sweep.ps1"
    if command -v cygpath >/dev/null 2>&1; then
        sweep_script="$(cygpath -w "$sweep_script")"
    fi

    echo "Cargo target volume is below ${minimum_gib} GiB free; attempting the ${sweep_max_gb} GB Windows artifact cap." >&2
    if powershell.exe -NoProfile -ExecutionPolicy Bypass \
        -File "$sweep_script" -Operation run -MaxSizeGB "$sweep_max_gb"; then
        reclaim_report="Automatic Cargo artifact reclaim completed, but did not restore the required headroom."
    else
        reclaim_exit=$?
        reclaim_report="Automatic Cargo artifact reclaim failed with exit ${reclaim_exit}."
    fi
    measure_space
    if (( free_kib >= required_kib )); then
        awk -v f="$free_kib" -v m="$minimum_gib" 'BEGIN {
            printf "Cargo storage preflight restored %.1f GiB free (required: %d GiB).\n",
                   f / 1048576, m
        }' >&2
        exit 0
    fi
fi

if (( free_kib < required_kib )); then
    read -r total_gib used_gib free_gib < <(
        awk -v t="$total_kib" -v u="$used_kib" -v f="$free_kib" 'BEGIN {
            printf "%.1f %.1f %.1f\n", t / 1048576, u / 1048576, f / 1048576
        }'
    )
    wsl_report="$(report_wsl_vhdx || true)"
    cat >&2 <<EOF
Cargo storage preflight failed.
  target:   $target_dir
  volume:   ${total_gib} GiB total, ${used_gib} GiB used, ${free_gib} GiB free
  required: ${minimum_gib} GiB
${wsl_report}
${reclaim_report}

'just windows-sweep' trims the native Cargo target to its 80 GB cap. Automatic
reclaim can be disabled with BISCUIT_BUILD_AUTO_SWEEP=0. Set
BISCUIT_BUILD_MIN_FREE_GIB=0 only for an intentional emergency override.
EOF
    exit 1
fi
