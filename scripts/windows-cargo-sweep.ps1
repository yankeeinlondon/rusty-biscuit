[CmdletBinding()]
param(
    [ValidateSet("run", "install", "uninstall", "status")]
    [string]$Operation = "run",
    [string]$RepoRoot = "",
    [ValidateRange(1, 1000)]
    [int]$MaxSizeGB = 80,
    [string]$CargoPath = "",
    [string]$CargoSweepPath = "",
    [string]$LogPath = "",
    [switch]$DryRun
)

$ErrorActionPreference = "Stop"
$taskName = "RustyBiscuit-CargoSweep"
$scriptPath = $MyInvocation.MyCommand.Path

if (-not $RepoRoot) {
    $RepoRoot = Split-Path -Parent (Split-Path -Parent $scriptPath)
}
if (-not $CargoPath) {
    $CargoPath = (Get-Command cargo.exe -ErrorAction Stop).Source
}
if (-not $CargoSweepPath) {
    $CargoSweepPath = Join-Path $env:USERPROFILE ".cargo\bin\cargo-sweep.exe"
}
if (-not $LogPath) {
    $LogPath = Join-Path $env:LOCALAPPDATA "rusty-biscuit\logs\cargo-sweep.log"
}

if ($Operation -eq "install") {
    $powerShell = (Get-Command powershell.exe -ErrorAction Stop).Source
    $arguments = @(
        "-NoProfile"
        "-ExecutionPolicy Bypass"
        "-File `"$scriptPath`""
        "-Operation run"
        "-RepoRoot `"$RepoRoot`""
        "-MaxSizeGB $MaxSizeGB"
        "-CargoPath `"$CargoPath`""
        "-CargoSweepPath `"$CargoSweepPath`""
        "-LogPath `"$LogPath`""
    ) -join " "
    $action = New-ScheduledTaskAction -Execute $powerShell -Argument $arguments -WorkingDirectory $RepoRoot
    $trigger = New-ScheduledTaskTrigger -Weekly -WeeksInterval 1 -DaysOfWeek Sunday, Wednesday -At 4am
    $settings = New-ScheduledTaskSettingsSet -StartWhenAvailable -MultipleInstances IgnoreNew -ExecutionTimeLimit (New-TimeSpan -Hours 2)
    $userId = [System.Security.Principal.WindowsIdentity]::GetCurrent().Name
    $principal = New-ScheduledTaskPrincipal -UserId $userId -LogonType S4U -RunLevel Limited
    Register-ScheduledTask -TaskName $taskName -Action $action -Trigger $trigger -Settings $settings -Principal $principal -Description "Keep rusty-biscuit Cargo artifacts within the Windows storage budget." -Force | Out-Null
    Write-Output "Installed scheduled task '$taskName' (Sunday and Wednesday at 04:00)."
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
    Get-ScheduledTask -TaskName $taskName -ErrorAction Stop |
        Select-Object TaskName, State, Description | Format-List | Out-String
    Get-ScheduledTaskInfo -TaskName $taskName -ErrorAction Stop |
        Select-Object LastRunTime, LastTaskResult, NextRunTime | Format-List | Out-String
    exit 0
}

$busy = Get-Process -Name cargo, rustc, clippy-driver, link, lld-link -ErrorAction SilentlyContinue
if ($busy) {
    $names = ($busy.ProcessName | Sort-Object -Unique) -join ", "
    Write-Output "Sweep skipped because build processes are active: $names"
    exit 0
}

if (-not (Test-Path -LiteralPath $CargoSweepPath)) {
    throw "cargo-sweep is not installed at $CargoSweepPath; run 'just _ensure-cargo-sweep'."
}
if (-not (Test-Path -LiteralPath (Join-Path $RepoRoot "Cargo.toml"))) {
    throw "RepoRoot is not a Cargo workspace: $RepoRoot"
}

$logDir = Split-Path -Parent $LogPath
New-Item -ItemType Directory -Force -Path $logDir | Out-Null
$sweepArgs = @("sweep")
if ($DryRun) {
    $sweepArgs += "--dry-run"
}
$sweepArgs += @("--recursive", "--maxsize", "${MaxSizeGB}GB", $RepoRoot)

& {
    $targetDir = (& $CargoPath metadata --no-deps --format-version 1 | ConvertFrom-Json).target_directory
    $driveName = [System.IO.Path]::GetPathRoot($targetDir).TrimEnd([char[]]@(':', '\'))
    $before = Get-PSDrive -Name $driveName
    Write-Output "[$(Get-Date -Format o)] sweep start target=$targetDir free_gib=$([math]::Round($before.Free / 1GB, 1)) max=${MaxSizeGB}GB dry_run=$DryRun"
    & $CargoSweepPath @sweepArgs
    if ($LASTEXITCODE -ne 0) {
        throw "cargo-sweep failed with exit $LASTEXITCODE"
    }
    $after = Get-PSDrive -Name $driveName
    Write-Output "[$(Get-Date -Format o)] sweep complete target=$targetDir free_gib=$([math]::Round($after.Free / 1GB, 1))"
} *>&1 | Tee-Object -FilePath $LogPath -Append
