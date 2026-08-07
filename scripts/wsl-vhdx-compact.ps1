[CmdletBinding()]
param(
    [ValidateSet("run", "install", "uninstall", "status")]
    [string]$Operation = "run",
    [string[]]$Distribution = @(),
    [ValidateRange(1, 1000)]
    [int]$MinSlackGiB = $(if ($env:BISCUIT_WSL_MIN_SLACK_GIB) { $env:BISCUIT_WSL_MIN_SLACK_GIB } else { 20 }),
    [string]$LogPath = $env:BISCUIT_WSL_COMPACT_LOG,
    [switch]$DryRun
)

$ErrorActionPreference = "Stop"
$taskName = "RustyBiscuit-WslVhdxCompact"
$scriptPath = $MyInvocation.MyCommand.Path

# WSL emits UTF-16LE by default, which turns every parsed field into
# NUL-separated characters. Every wsl.exe call below assumes this is set.
$env:WSL_UTF8 = "1"

if (-not $LogPath) {
    $LogPath = Join-Path $env:LOCALAPPDATA "rusty-biscuit\logs\wsl-vhdx-compact.log"
}

# BasePath is the only authority for a relocated distro. `wsl --list` never
# reports it, and this repo's host keeps Ubuntu on W: rather than under
# %LOCALAPPDATA%\Packages.
function Get-WslDistribution {
    Get-ChildItem 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Lxss' -ErrorAction SilentlyContinue |
        ForEach-Object { Get-ItemProperty $_.PSPath } |
        Where-Object { $_.Version -eq 2 -and $_.BasePath } |
        ForEach-Object {
            $base = $_.BasePath -replace '^\\\\\?\\', ''
            [PSCustomObject]@{
                Name = $_.DistributionName
                Vhdx = Join-Path $base "ext4.vhdx"
            }
        } |
        Where-Object { Test-Path -LiteralPath $_.Vhdx }
}

# Slack is what compaction can actually return: blocks the VHDX occupies on
# NTFS that the guest filesystem no longer considers allocated. Measuring it
# first is what keeps a scheduled run from rewriting a 100 GiB file for nothing.
function Get-VhdxSlackGiB {
    param([string]$Name, [string]$Vhdx)

    $onDisk = (Get-Item -LiteralPath $Vhdx).Length
    $df = & wsl.exe -d $Name -u root --exec /bin/df -B1 /
    if ($LASTEXITCODE -ne 0) { return $null }

    $fields = ($df | Select-Object -Last 1) -split '\s+'
    if ($fields.Count -lt 3) { return $null }
    $used = [int64]$fields[2]

    [PSCustomObject]@{
        OnDiskGiB = [math]::Round($onDisk / 1GB, 1)
        UsedGiB   = [math]::Round($used / 1GB, 1)
        SlackGiB  = [math]::Round(($onDisk - $used) / 1GB, 1)
    }
}

function Test-WslBusy {
    param([string]$Name)

    # pgrep's pattern is an ERE, and -x anchors it to the whole process name.
    & wsl.exe -d $Name -u root --exec /bin/sh -c "pgrep -x 'cargo|rustc|rust-analyzer|cc1|cc1plus|ld|node|pytest' >/dev/null 2>&1" | Out-Null
    return ($LASTEXITCODE -eq 0)
}

if ($Operation -eq "install") {
    $powerShell = (Get-Command powershell.exe -ErrorAction Stop).Source
    $arguments = @(
        "-NoProfile"
        "-ExecutionPolicy Bypass"
        "-File `"$scriptPath`""
        "-Operation run"
        "-MinSlackGiB $MinSlackGiB"
        "-LogPath `"$LogPath`""
    ) -join " "
    $action = New-ScheduledTaskAction -Execute $powerShell -Argument $arguments
    $trigger = New-ScheduledTaskTrigger -Weekly -WeeksInterval 1 -DaysOfWeek Saturday -At 3am
    $settings = New-ScheduledTaskSettingsSet -StartWhenAvailable -MultipleInstances IgnoreNew -ExecutionTimeLimit (New-TimeSpan -Hours 2)
    $userId = [System.Security.Principal.WindowsIdentity]::GetCurrent().Name
    # RunLevel Highest is load-bearing: `diskpart compact vdisk` fails without
    # elevation, and Task Scheduler will not prompt.
    $principal = New-ScheduledTaskPrincipal -UserId $userId -LogonType S4U -RunLevel Highest
    Register-ScheduledTask -TaskName $taskName -Action $action -Trigger $trigger -Settings $settings -Principal $principal -Description "Reclaim WSL2 ext4.vhdx slack, which never shrinks on its own." -Force | Out-Null
    Write-Output "Installed scheduled task '$taskName' (Saturday at 03:00, min slack ${MinSlackGiB} GiB)."
    Write-Output "This task runs 'wsl --shutdown'; it will end any WSL session live at that hour."
    exit 0
}

if ($Operation -eq "uninstall") {
    Unregister-ScheduledTask -TaskName $taskName -Confirm:$false -ErrorAction Stop
    Write-Output "Removed scheduled task '$taskName'."
    exit 0
}

if ($Operation -eq "status") {
    # Out-String renders now rather than leaving objects for the formatter to
    # lay out at pipeline end -- the `exit` below discards anything still
    # buffered there, which silently swallows the whole report.
    Get-ScheduledTask -TaskName $taskName -ErrorAction SilentlyContinue |
        Select-Object TaskName, State, Description | Format-List | Out-String
    Get-ScheduledTaskInfo -TaskName $taskName -ErrorAction SilentlyContinue |
        Select-Object LastRunTime, LastTaskResult, NextRunTime | Format-List | Out-String
    foreach ($d in Get-WslDistribution) {
        $m = Get-VhdxSlackGiB -Name $d.Name -Vhdx $d.Vhdx
        if ($m) {
            Write-Output ("{0}: on-disk {1} GiB, used {2} GiB, reclaimable {3} GiB  ({4})" -f $d.Name, $m.OnDiskGiB, $m.UsedGiB, $m.SlackGiB, $d.Vhdx)
        }
    }
    exit 0
}

$identity = [System.Security.Principal.WindowsIdentity]::GetCurrent()
if (-not (New-Object System.Security.Principal.WindowsPrincipal($identity)).IsInRole([System.Security.Principal.WindowsBuiltInRole]::Administrator)) {
    throw "Compaction requires an elevated shell; 'diskpart compact vdisk' cannot run unelevated."
}

$busy = Get-Process -Name cargo, rustc, clippy-driver, link, lld-link -ErrorAction SilentlyContinue
if ($busy) {
    $names = ($busy.ProcessName | Sort-Object -Unique) -join ", "
    Write-Output "Compaction skipped because build processes are active: $names"
    exit 0
}

$logDir = Split-Path -Parent $LogPath
New-Item -ItemType Directory -Force -Path $logDir | Out-Null

& {
    $distros = @(Get-WslDistribution)
    if ($Distribution.Count -gt 0) {
        $distros = @($distros | Where-Object { $Distribution -contains $_.Name })
    }
    if ($distros.Count -eq 0) {
        Write-Output "[$(Get-Date -Format o)] no WSL2 distributions with an ext4.vhdx found"
        return
    }

    # Trim every candidate before shutting anything down: fstrim needs the guest
    # running, compaction needs it stopped, so the two phases cannot interleave.
    $candidates = @()
    foreach ($d in $distros) {
        if (Test-WslBusy -Name $d.Name) {
            Write-Output "[$(Get-Date -Format o)] $($d.Name): skipped, work in progress inside the distribution"
            continue
        }

        & wsl.exe -d $d.Name -u root --exec /bin/sh -c "PATH=/usr/sbin:/sbin:`$PATH fstrim -av"

        $m = Get-VhdxSlackGiB -Name $d.Name -Vhdx $d.Vhdx
        if (-not $m) {
            Write-Output "[$(Get-Date -Format o)] $($d.Name): skipped, could not measure guest usage"
            continue
        }
        Write-Output "[$(Get-Date -Format o)] $($d.Name): on-disk $($m.OnDiskGiB) GiB, used $($m.UsedGiB) GiB, reclaimable $($m.SlackGiB) GiB"

        if ($m.SlackGiB -lt $MinSlackGiB) {
            Write-Output "[$(Get-Date -Format o)] $($d.Name): below the ${MinSlackGiB} GiB threshold, leaving it alone"
            continue
        }
        $candidates += $d
    }

    if ($candidates.Count -eq 0) { return }
    if ($DryRun) {
        Write-Output "[$(Get-Date -Format o)] dry run, would compact: $(($candidates.Name) -join ', ')"
        return
    }

    & wsl.exe --shutdown

    foreach ($d in $candidates) {
        $drive = [System.IO.Path]::GetPathRoot($d.Vhdx).TrimEnd([char[]]@(':', '\'))
        $before = Get-PSDrive -Name $drive
        $script = [System.IO.Path]::GetTempFileName()
        @(
            "select vdisk file=`"$($d.Vhdx)`""
            "attach vdisk readonly"
            "compact vdisk"
            "detach vdisk"
            "exit"
        ) | Set-Content -LiteralPath $script -Encoding ascii

        try {
            & diskpart.exe /s $script
            if ($LASTEXITCODE -ne 0) {
                throw "diskpart failed with exit $LASTEXITCODE for $($d.Vhdx)"
            }
        } finally {
            Remove-Item -LiteralPath $script -Force -ErrorAction SilentlyContinue
        }

        $after = Get-PSDrive -Name $drive
        $freed = [math]::Round(($after.Free - $before.Free) / 1GB, 1)
        Write-Output ("[{0}] {1}: compacted, {2}: freed {3} GiB, now {4} GiB free" -f (Get-Date -Format o), $d.Name, $drive, $freed, [math]::Round($after.Free / 1GB, 1))
    }
} *>&1 | Tee-Object -FilePath $LogPath -Append
