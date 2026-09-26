# Run on native Windows: powershell -NoProfile -ExecutionPolicy Bypass -File spike_windows.ps1
$ErrorActionPreference = 'Stop'
$root = Join-Path $env:TEMP ("worktree-file-spike-" + [guid]::NewGuid().ToString('N'))
$repo = Join-Path $root 'repo'
$linked = Join-Path $root 'linked'

function Invoke-GitBytes([string]$arguments, [byte[]]$inputBytes) {
    $start = New-Object Diagnostics.ProcessStartInfo
    $start.FileName = 'git.exe'
    $start.Arguments = $arguments
    $start.UseShellExecute = $false
    $start.RedirectStandardInput = $true
    $start.RedirectStandardOutput = $true
    $start.RedirectStandardError = $true
    $process = [Diagnostics.Process]::Start($start)
    if ($inputBytes) { $process.StandardInput.BaseStream.Write($inputBytes, 0, $inputBytes.Length) }
    $process.StandardInput.Close()
    $stream = New-Object IO.MemoryStream
    $process.StandardOutput.BaseStream.CopyTo($stream)
    $process.WaitForExit()
    if ($process.ExitCode -ne 0) { throw $process.StandardError.ReadToEnd() }
    return ,$stream.ToArray()
}
New-Item -ItemType Directory -Path $repo | Out-Null
try {
    & git -C $repo init -q -b main
    & git -C $repo config user.name Spike
    & git -C $repo config user.email spike@example.invalid
    [IO.File]::WriteAllText((Join-Path $repo 'tracked'), 'x')
    & git -C $repo add tracked
    & git -C $repo commit -qm initial
    & git -C $repo worktree add -q -b topic $linked
    $admin = (& git -C $linked rev-parse --absolute-git-dir).Trim()
    $marker = Join-Path $admin 'gitdir'
    $before = (Get-Item -LiteralPath $marker).CreationTimeUtc.Ticks
    $nonceFile = Join-Path $admin 'wt-copy-registration'
    $firstNonce = [guid]::NewGuid().ToString('N')
    [IO.File]::WriteAllText($nonceFile, $firstNonce)
    & git -C $repo worktree remove --force $linked
    if (Test-Path -LiteralPath $nonceFile) { throw 'Git retained the registration nonce' }
    & git -C $repo worktree prune
    & git -C $repo branch -D topic | Out-Null
    & git -C $repo worktree add -q -b topic $linked
    $afterAdmin = (& git -C $linked rev-parse --absolute-git-dir).Trim()
    $after = (Get-Item -LiteralPath (Join-Path $afterAdmin 'gitdir')).CreationTimeUtc.Ticks
    $secondNonce = [guid]::NewGuid().ToString('N')
    [IO.File]::WriteAllText((Join-Path $afterAdmin 'wt-copy-registration'), $secondNonce)
    if ($firstNonce -eq $secondNonce) { throw 'Registration nonce repeated' }
    if ($admin -ne $afterAdmin -or $before -eq $after) {
        throw "Registration identity did not change: $admin $before $afterAdmin $after"
    }

    [IO.File]::WriteAllText((Join-Path $repo '.gitignore'), '*.env' + "`n")
    [IO.File]::WriteAllText((Join-Path $repo '.worktreeinclude'), '*.env' + "`n")
    [IO.File]::WriteAllText((Join-Path $repo 'local.env'), 'secret')
    [IO.File]::WriteAllText((Join-Path $repo 'plain.env'), 'plain')
    $candidates = Invoke-GitBytes "-C `"$repo`" ls-files --others --ignored --exclude-from=.worktreeinclude -z" $null
    $checked = Invoke-GitBytes "-C `"$repo`" check-ignore --stdin -z --verbose --non-matching" $candidates
    $candidateText = [Text.Encoding]::UTF8.GetString($candidates)
    $checkedText = [Text.Encoding]::UTF8.GetString($checked)
    if (-not $candidateText.Contains("local.env`0") -or -not $checkedText.Contains("local.env`0")) {
        throw "Git ignore pipeline failed: $candidates $checked"
    }

    $target = Join-Path $root 'target'
    New-Item -ItemType Directory -Path $target | Out-Null
    [IO.File]::WriteAllText((Join-Path $target 'secret.env'), 'secret')
    $junction = Join-Path $repo 'junction'
    & cmd /c mklink /J $junction $target | Out-Null
    if ($LASTEXITCODE -ne 0) { throw 'Junction creation failed' }
    $junctionAttrs = [IO.File]::GetAttributes($junction)
    $junctionIsReparse = [bool]($junctionAttrs -band [IO.FileAttributes]::ReparsePoint)
    $junctionIsDirectory = [bool]($junctionAttrs -band [IO.FileAttributes]::Directory)
    if (-not $junctionIsReparse -or -not $junctionIsDirectory) {
        throw "Junction attributes wrong: $junctionAttrs"
    }
    [IO.File]::WriteAllText((Join-Path $repo '.worktreeinclude'), "junction/`n")
    $junctionCandidates = Invoke-GitBytes "-C `"$repo`" ls-files --others --ignored --exclude-from=.worktreeinclude -z" $null
    $junctionText = [Text.Encoding]::UTF8.GetString($junctionCandidates)
    if (-not $junctionText.Contains('junction/secret.env')) {
        throw 'Expected Git-for-Windows junction traversal changed'
    }

    $link = Join-Path $repo 'file-link'
    $linkResult = & cmd /c mklink $link (Join-Path $repo 'local.env') 2>&1
    $linkExit = $LASTEXITCODE
    $linkAttrs = if ($linkExit -eq 0) { [IO.File]::GetAttributes($link).ToString() } else { '' }
    $result = [ordered]@{
        git = (& git --version)
        admin_reused = ($admin -eq $afterAdmin)
        gitdir_created_before = $before
        gitdir_created_after = $after
        candidates_contain_ignored = $candidateText.Contains("local.env`0")
        check_ignore_contain_ignored = $checkedText.Contains("local.env`0")
        junction_attributes = $junctionAttrs.ToString()
        junction_reparse_directory = ($junctionIsReparse -and $junctionIsDirectory)
        junction_candidate_output = $junctionText
        git_traversed_junction = $junctionText.Contains('junction/secret.env')
        symlink_exit = $linkExit
        symlink_output = ($linkResult | Out-String).Trim()
        symlink_attributes = $linkAttrs
    }
    $result | ConvertTo-Json -Depth 4
} finally {
    & git -C $repo worktree remove --force $linked 2>$null | Out-Null
    Remove-Item -LiteralPath $root -Recurse -Force -ErrorAction SilentlyContinue
}
