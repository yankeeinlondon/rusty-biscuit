#!/usr/bin/env bash
set -euo pipefail

operation="$1"
host="${2:-}"
case "$operation" in
    run | status | install | uninstall) ;;
    *) echo "sweep: unknown operation: $operation" >&2; exit 2 ;;
esac

case "${SWEEP_STATUS_RENDER:-}" in
    plain | bold | color) render="$SWEEP_STATUS_RENDER" ;;
    *)
        render=plain
        if [[ -t 1 && "${TERM:-dumb}" != dumb ]]; then
            render=bold
            if [[ -z "${NO_COLOR:-}" ]]; then render=color; fi
        fi
        ;;
esac

if [[ "$operation" == status && "$host" != all && -t 1 && -t 2 &&
      "${TERM:-dumb}" != dumb && -z "${SWEEP_STATUS_RENDER+x}" ]]; then
    caption="Scanning sweep candidates"
    if [[ -n "$host" ]]; then
        caption+=" on $host"
    fi
    (
        sleep 0.25
        while :; do
            for frame in '|' '/' '-' '\'; do
                printf '\r\033[K%s  %s' "$frame" "$caption" >&2
                sleep 0.12
            done
        done
    ) &
    spinner=$!
    stop_spinner() {
        kill "$spinner" 2>/dev/null || true
        wait "$spinner" 2>/dev/null || true
        printf '\r\033[K' >&2
    }
    trap stop_spinner EXIT
    trap 'exit 130' INT
    trap 'exit 143' TERM
    result=0
    report=$(SWEEP_STATUS_RENDER="$render" bash "${BASH_SOURCE[0]}" "$operation" "$host" 2>&1) || result=$?
    stop_spinner
    trap - EXIT INT TERM
    [[ -z "$report" ]] || printf '%s\n' "$report"
    exit "$result"
fi

if [[ "$operation" == install || "$operation" == uninstall ]]; then
    if [[ -n "$host" ]]; then
        echo "Sweep schedule changes apply to the local host." >&2
        exit 2
    fi
elif [[ "$host" == all ]]; then
    failed=0
    seen=()
    for variable in local BUILD_WIN BUILD_WSL BUILD_MACOS BUILD_LINUX; do
        destination=""
        label="Local"
        if [[ "$variable" != local ]]; then
            destination="${!variable-}"
            [[ -n "$destination" ]] || continue
            label="$destination ($variable)"
            if [[ "$destination" == all ]]; then
                printf '%s: all is reserved for the host group\n' "$variable" >&2
                failed=1
                continue
            fi
            for prior in "${seen[@]+"${seen[@]}"}"; do
                [[ "$prior" != "$destination" ]] || continue 2
            done
            seen+=("$destination")
        fi
        printf '\n-- %s --\n' "$label"
        if ! bash "${BASH_SOURCE[0]}" "$operation" "$destination"; then
            printf 'Sweep %s failed for %s.\n' "$operation" "$label" >&2
            failed=1
        fi
    done
    exit "$failed"
fi

if [[ -n "$host" ]]; then
    source scripts/cross-check-hosts.sh
    calling_host_identity
    clone="$origin_host--$worktree/rusty-biscuit"
    ssh_cmd=("${SSH[@]}" -- "$host")
    # The native Windows rigs use PowerShell as their SSH shell. Also
    # recognize cmd so a Windows SSH alias does not need a BUILD_WIN entry.
    remote_os=$("${ssh_cmd[@]}" 'echo "sweep-powershell=$env:OS sweep-cmd=%OS%"')
    case "$remote_os" in
        *sweep-powershell=Windows_NT* | *sweep-cmd=Windows_NT*)
            # Encode the script so the SSH shell cannot expand PowerShell variables.
            compressed=$({
                cat <<EOF
\$ErrorActionPreference = 'Stop'
\$ProgressPreference = 'SilentlyContinue'
\$coding = if (\$env:CODING_DIR) { \$env:CODING_DIR } else { Join-Path \$env:USERPROFILE 'coding' }
\$repo = Join-Path \$coding 'rusty-biscuit'
if (-not (Test-Path -LiteralPath (Join-Path \$repo 'scripts/windows-cargo-sweep.ps1'))) {
    \$task = Get-ScheduledTask -TaskName 'RustyBiscuit-CargoSweep' -ErrorAction SilentlyContinue
    \$taskRoot = \$task.Actions | Select-Object -First 1 -ExpandProperty WorkingDirectory
    if (\$taskRoot -and (Test-Path -LiteralPath (Join-Path \$taskRoot 'scripts/windows-cargo-sweep.ps1'))) {
        \$repo = \$taskRoot
    } else {
        \$repo = Join-Path \$coding '$clone'
    }
}
if (-not (Test-Path -LiteralPath (Join-Path \$repo 'scripts/windows-cargo-sweep.ps1'))) {
    throw "No sweep checkout found on \$env:COMPUTERNAME."
}
Set-Location -LiteralPath \$repo
if ('$operation' -eq 'run') { Write-Output ("Sweeping " + (Split-Path -Leaf \$repo)) }
EOF
                if [[ "$operation" == status ]]; then
                    printf '& {\n'
                    cat scripts/windows-sweep-preview.ps1
                    printf '} -RepoRoot $repo -Style %s\n' "$render"
                else
                    cat <<EOF
& './scripts/windows-cargo-sweep.ps1' -Operation run
exit \$LASTEXITCODE
EOF
                fi
            } | gzip -c | base64 | tr -d '\n')
            # The SSH command must also fit through cmd's shorter command-line limit.
            payload=$(iconv -f UTF-8 -t UTF-16LE <<EOF | base64 | tr -d '\n'
\$Global:ProgressPreference = 'SilentlyContinue'
\$inputStream = [IO.MemoryStream]::new([Convert]::FromBase64String('$compressed'))
\$reader = [IO.StreamReader]::new([IO.Compression.GZipStream]::new(\$inputStream, [IO.Compression.CompressionMode]::Decompress))
& ([scriptblock]::Create(\$reader.ReadToEnd()))
EOF
            )
            exec "${ssh_cmd[@]}" "powershell.exe -NoProfile -NonInteractive -OutputFormat Text -ExecutionPolicy Bypass -EncodedCommand $payload"
            ;;
        *)
            # Send the dispatcher so the remote checkout only needs the existing
            # cleanup scripts. Resolve the checkout in the remote login shell.
            {
                printf 'clone="%s"\n' "$clone"
                cat <<'EOF'
set -e
unset CDPATH
coding="${CODING_DIR:-$HOME/coding}"
repo="$coding/rusty-biscuit"
if [[ ! -f "$repo/scripts/sweep.sh" ]]; then
    repo="$coding/$clone"
fi
if [[ ! -f "$repo/scripts/sweep.sh" ]]; then
    echo "No sweep checkout found on $(hostname)." >&2
    exit 1
fi
cd -- "$repo"
EOF
                # Run and preview the same policy even if the target checkout
                # predates preview support.
                printf 'sweep_policy() (\n'
                cat scripts/sweep.sh
                printf '\n)\n'
                if [[ "$operation" == status ]]; then
                    cat <<'EOF'
sweep_summary() {
    python3 -c "$(cat <<'SWEEP_REPORT_PY'
EOF
                    cat scripts/sweep-report.py
                    printf '\nSWEEP_REPORT_PY\n)" "$@"\n}\n'
                fi
                cat "${BASH_SOURCE[0]}"
            } | "${ssh_cmd[@]}" "bash -lc 'SWEEP_STATUS_RENDER=$render exec bash -s -- $operation'"
            exit 0
            ;;
    esac
fi

platform=$(uname -s)
case "$platform" in
    CYGWIN* | MINGW* | MSYS*)
        if [[ "$operation" == status ]]; then
            exec powershell.exe -NoProfile -ExecutionPolicy Bypass \
                -File scripts/windows-sweep-preview.ps1 -Style "$render"
        fi
        exec powershell.exe -NoProfile -ExecutionPolicy Bypass \
            -File scripts/windows-cargo-sweep.ps1 -Operation "$operation"
        ;;
    Darwin | Linux) ;;
    *) echo "sweep: unsupported platform: $platform" >&2; exit 2 ;;
esac

if [[ "$operation" == install || "$operation" == uninstall ]]; then
    if [[ "$platform" == Darwin ]]; then
        exec bash scripts/macos-cargo-sweep.sh "$operation"
    fi
    exec bash scripts/linux-cargo-sweep.sh "$operation"
fi

if ! declare -F sweep_policy >/dev/null; then
    sweep_policy() { bash scripts/sweep.sh "$@"; }
fi

if [[ "$operation" == run ]]; then
    just _ensure-cargo-sweep
    source scripts/cargo-path.sh
    sweep_policy
    exit 0
fi

sweep_schedule() {
    local next result directory
    if [[ "$platform" == Darwin ]]; then
        local uid
        uid=$(id -u)
        if launchctl print "gui/$uid/com.ken.rusty-biscuit-sweep" >/dev/null 2>&1 ||
            launchctl print "user/$uid/com.ken.rusty-biscuit-sweep" >/dev/null 2>&1; then
            echo "Launch agent loaded"
        else
            echo "Not installed or not loaded"
        fi
    elif command -v systemctl >/dev/null &&
        [[ "$(systemctl --user show rusty-biscuit-sweep.timer -p LoadState --value 2>/dev/null)" == loaded ]]; then
        next=$(systemctl --user show rusty-biscuit-sweep.timer -p NextElapseUSecRealtime --value)
        result=$(systemctl --user show rusty-biscuit-sweep.service -p Result --value)
        directory=$(systemctl --user show rusty-biscuit-sweep.service -p WorkingDirectory --value)
        printf 'Systemd timer%s' "${next:+ | Next $next}"
        if [[ -n "$directory" && ! -d "$directory" ]]; then
            printf ' | Checkout missing'
        elif [[ -n "$result" && "$result" != success ]]; then
            printf ' | Last result: %s' "$result"
        fi
        printf '\n'
    elif crontab -l 2>/dev/null | grep -F '# rusty-biscuit-sweep (managed' >/dev/null; then
        echo "Cron schedule installed"
    else
        echo "Not installed"
    fi
}

if ! declare -F sweep_summary >/dev/null; then
    sweep_summary() { python3 scripts/sweep-report.py "$@"; }
fi

source scripts/cargo-path.sh
sweep_policy --preview-input | sweep_summary "$(sweep_schedule)"
