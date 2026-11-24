param([string]$RepoRoot = $PWD.Path, [ValidateSet('plain', 'bold', 'color')][string]$Style = 'plain')

$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
$Culture = [Globalization.CultureInfo]::InvariantCulture
$Units = @{ B = 1; KiB = 1KB; MiB = 1MB; GiB = 1GB; TiB = 1TB; PiB = 1PB }

function Format-Style([string]$Text, [string]$Code) {
    if ($Style -eq 'plain') { return $Text }
    if ($Style -eq 'bold') {
        if ($Code -notmatch '^1(;|$)') { return $Text }
        $Code = '1'
    }
    return "$([char]27)[$($Code)m$Text$([char]27)[0m"
}

function Format-Heading([string]$Text) {
    return Format-Style "$($Text):" '1;36'
}

function Format-Size([double]$Bytes, [int]$Width = 12, [switch]$Estimated) {
    $unit = 'B'
    foreach ($candidate in 'GiB', 'MiB', 'KiB') {
        if ($Bytes -ge $Units[$candidate]) {
            $unit = $candidate
            break
        }
    }
    $amount = ($Bytes / $Units[$unit]).ToString('N1', $Culture)
    if ($Estimated) { $amount = '~' + $amount }
    return $amount.PadLeft($Width - 4) + ' ' + $unit.PadRight(3)
}

$savings = @{}
$estimateComplete = $true
$sweeper = Join-Path $env:USERPROFILE '.cargo/bin/cargo-sweep.exe'
if (Test-Path -LiteralPath $sweeper) {
    $results = & $sweeper sweep --dry-run --recursive --maxsize 80GB $RepoRoot
    $estimateComplete = $LASTEXITCODE -eq 0
    foreach ($line in $results) {
        if ($line -match '^\[ERROR\]') {
            $estimateComplete = $false
        }
        if ($line -match 'Would clean: ([0-9.]+) (\w+) from "([A-Za-z]):' -and $Units.ContainsKey($Matches[2])) {
            $drive = $Matches[3]
            $savings[$drive] += [double]::Parse($Matches[1], $Culture) * $Units[$Matches[2]]
        } elseif ($line -match 'Would clean:' -and $line -notmatch 'nothing from') {
            $estimateComplete = $false
        }
    }
} else {
    $estimateComplete = $false
}

Write-Output ("{0} {1}" -f (Format-Heading 'Sweep preview'), (Format-Style $env:COMPUTERNAME '1'))
Write-Output ''
Write-Output (Format-Heading 'Disk space')
$row = '  {0,-20} {1,12} {2,12} {3,12} {4,14}'
Write-Output (Format-Style ($row -f 'Volume', 'Total', 'Used', 'Free now', 'Free after') '1')
$total = 0
foreach ($disk in Get-PSDrive -PSProvider FileSystem | Where-Object { $_.Used + $_.Free -gt 0 }) {
    $reclaim = [Math]::Min([double]$savings[$disk.Name], $disk.Used)
    $total += $reclaim
    $after = if ($estimateComplete) {
        Format-Style ((Format-Size ($disk.Free + $reclaim) 14 -Estimated).TrimEnd()) '1;32'
    } else {
        Format-Style ('Incomplete'.PadLeft(14)) '33'
    }
    $capacity = Format-Size ($disk.Used + $disk.Free)
    $used = Format-Size $disk.Used
    $free = Format-Size $disk.Free
    Write-Output ("  {0,-20} {1} {2} {3} {4}" -f ($disk.Name + ':'), $capacity, $used, $free, $after)
}

$bucket = 'Cargo size cap (80 GB)'
Write-Output ''
Write-Output (Format-Heading 'Expected to reclaim')
if ($total -gt 0) {
    $amount = (Format-Size $total -Estimated).TrimEnd()
    Write-Output ('  {0,-36} {1}' -f $bucket, $amount)
    Write-Output ('  {0} {1}' -f (Format-Style ('Estimated reclaim'.PadRight(36)) '1'),
        (Format-Style $amount '1;32'))
} else {
    Write-Output '  None'
}

Write-Output ''
Write-Output (Format-Heading 'No savings expected')
if ($estimateComplete -and $total -eq 0) { Write-Output "  $bucket" } else { Write-Output '  None' }
if (-not $estimateComplete) { Write-Output 'Estimate incomplete for the Cargo size cap.' }

$name = 'RustyBiscuit-CargoSweep'
$task = Get-ScheduledTask -TaskName $name -ErrorAction SilentlyContinue
$schedule = 'Not installed'
if ($task) {
    $info = Get-ScheduledTaskInfo -TaskName $name
    $schedule = "$($task.State)"
    if ($info.NextRunTime -gt [datetime]::MinValue) { $schedule += " | Next $($info.NextRunTime)" }
    $taskRoot = $task.Actions | Select-Object -First 1 -ExpandProperty WorkingDirectory
    if ($taskRoot -and -not (Test-Path -LiteralPath $taskRoot)) { $schedule += ' | Checkout missing' }
    elseif ($info.LastTaskResult) { $schedule += ' | Last result 0x{0:X8}' -f $info.LastTaskResult }
}
$parts = $schedule -split ' \| '
$statusColor = if ($schedule -match '^Not |Checkout missing|Last result') { '33' } else { '32' }
$details = ($parts | Select-Object -Skip 1 | ForEach-Object { ' ' + (Format-Style '|' '2') + ' ' + $_ }) -join ''
Write-Output ''
Write-Output ('{0} {1}{2}' -f (Format-Heading 'Schedule'), (Format-Style $parts[0] $statusColor), $details)
Write-Output (Format-Style 'Estimated from Cargo artifact sizes. Compression and shared blocks can reduce actual savings.' '2')
