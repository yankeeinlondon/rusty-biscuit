#Requires -Version 5.1
<#
.SYNOPSIS
    Windows preflight for `just init` in the rusty-biscuit monorepo.

.DESCRIPTION
    The monorepo's justfiles run every recipe through bash, and `just` itself
    needs `cygpath` on PATH to translate recipe shebang lines on Windows. When
    either is missing, `just init` dies with an opaque error ("could not find
    `cygpath` executable ...") before any recipe can run — so this check has
    to happen outside `just`.

    Run this instead of `just init` on native Windows:

        powershell -ExecutionPolicy Bypass -File scripts\init.ps1

    Any arguments are forwarded to `just init`.
#>
param(
    [Parameter(ValueFromRemainingArguments = $true)]
    [string[]]$JustArgs
)

$ErrorActionPreference = 'Stop'

# WinGet updates the persisted user PATH without changing an already-open
# terminal. Honor its ordering and retain session-only entries afterwards.
$persistedPaths = @(
    [Environment]::GetEnvironmentVariable('Path', 'Machine') -split ';'
    [Environment]::GetEnvironmentVariable('Path', 'User') -split ';'
) | Where-Object { $_ }
$currentPaths = @($env:Path -split ';') | Where-Object { $_ }
$sessionOnlyPaths = @($currentPaths | Where-Object { $persistedPaths -notcontains $_ })
$env:Path = (@($persistedPaths) + @($sessionOnlyPaths)) -join ';'

$python = Get-ChildItem -LiteralPath "$env:LOCALAPPDATA\Programs\Python" `
    -Filter python.exe -Recurse -ErrorAction SilentlyContinue |
    Sort-Object FullName -Descending |
    Select-Object -First 1
if ($python) {
    $env:Path = "$($python.DirectoryName);$env:Path"
}

function Write-Step([string]$Message) { Write-Host "`n==> $Message" }
function Fail([string]$Message, [string[]]$Remediation) {
    Write-Host "`nERROR: $Message" -ForegroundColor Red
    foreach ($line in $Remediation) { Write-Host "  $line" }
    Write-Host ""
    exit 1
}

# --- 1. just itself -------------------------------------------------------

$just = Get-Command just -ErrorAction SilentlyContinue
if (-not $just) {
    Fail 'the `just` command runner is not installed (or not on PATH).' @(
        'Install it with:  winget install Casey.Just',
        'then open a NEW terminal so PATH updates take effect and re-run this script.'
    )
}

# --- 2. bash + cygpath ----------------------------------------------------

$bash    = Get-Command bash    -ErrorAction SilentlyContinue
$cygpath = Get-Command cygpath -ErrorAction SilentlyContinue

$wslStub = "$env:LOCALAPPDATA\Microsoft\WindowsApps\bash.exe"
$bashIsWslStub = $bash -and ($bash.Source -eq $wslStub)

if ((-not $bash) -or $bashIsWslStub -or (-not $cygpath)) {
    # Best case: a usable environment is installed but simply not on PATH.
    $candidates = @(
        @{ Name = 'Cygwin';          Bin = 'C:\cygwin64\bin' },
        @{ Name = 'Cygwin';          Bin = 'C:\cygwin\bin' },
        @{ Name = 'Git for Windows'; Bin = 'C:\Program Files\Git\bin'; Extra = 'C:\Program Files\Git\usr\bin' },
        @{ Name = 'Git for Windows'; Bin = 'C:\Program Files (x86)\Git\bin'; Extra = 'C:\Program Files (x86)\Git\usr\bin' }
    )
    foreach ($c in $candidates) {
        $cygpathDir = $c.Bin
        if ($c.Extra) { $cygpathDir = $c.Extra }
        $hasBash    = Test-Path (Join-Path $c.Bin 'bash.exe')
        $hasCygpath = Test-Path (Join-Path $cygpathDir 'cygpath.exe')
        if ($hasBash -and $hasCygpath) {
            $dirs = @($c.Bin) + ($(if ($c.Extra) { $c.Extra } else { @() }))
            if ($bashIsWslStub) {
                Fail "$($c.Name) is installed, but `bash` resolves to the WSL launcher stub because WindowsApps sorts earlier on PATH." (@(
                    'just would try to run this repo''s recipes INSIDE WSL instead of native Windows.',
                    'Move the following to the FRONT of your user PATH (System Properties -> Environment Variables):'
                ) + ($dirs | ForEach-Object { "    $_" }) + @(
                    'Then open a NEW terminal and re-run this script.',
                    '',
                    'One-off alternative for THIS PowerShell session only:',
                    ('    $env:PATH = "' + ($dirs -join ';') + ';$env:PATH"')
                ))
            }
            Fail "$($c.Name) is installed but its bin directory is not on PATH." (@(
                'Add the following to your user PATH (System Properties -> Environment Variables):'
            ) + ($dirs | ForEach-Object { "    $_" }) + @(
                'Then open a NEW terminal and re-run this script.',
                '',
                'One-off alternative for THIS PowerShell session only:',
                ('    $env:PATH = "' + ($dirs -join ';') + ';$env:PATH"')
            ))
        }
    }

    if ($bashIsWslStub) {
        Fail 'the only `bash` on PATH is the WSL launcher stub (WindowsApps\bash.exe).' @(
            'just would try to run this repo''s recipes INSIDE WSL instead of native Windows.',
            '',
            'If you meant to work in WSL: run `just init` from a real WSL terminal instead.',
            'If you meant native Windows: install Git for Windows (winget install Git.Git)',
            'or Cygwin, add its bin directory to PATH (it must sort BEFORE WindowsApps),',
            'then open a new terminal and re-run this script.'
        )
    }

    Fail 'no bash/cygpath found. just runs every recipe in this repo through bash.' @(
        'Install ONE of:',
        '  - Git for Windows:  winget install Git.Git',
        '      (then add "C:\Program Files\Git\bin" and "C:\Program Files\Git\usr\bin" to PATH)',
        '  - Cygwin:           https://cygwin.com/install.html',
        '      (then add "C:\cygwin64\bin" to PATH)',
        'Open a NEW terminal afterwards so PATH updates take effect, then re-run this script.'
    )
}

Write-Step "shell environment OK (bash: $($bash.Source))"

# --- 3. hand off to just --------------------------------------------------

& $just.Source init @JustArgs
exit $LASTEXITCODE
