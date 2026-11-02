# shellcheck shell=bash
# shellcheck disable=SC2034 # sets globals (want, skipped_local, worktree, PRUNED_CLONE_DIR, …) for the scripts that source it
# Shared by `cross-check.sh` and `cross-check-refresh.sh`: the build hosts this
# machine declares, the calling host's identity, and the one rule set that
# decides which clone directories on a build host are no longer needed.
# Sourced, never executed.
#
# Clone layout on a build host: `<coding dir>/<calling host>--<worktree>/`,
# holding the `rusty-biscuit` clone and its `target/`. The coding dir is the
# host's `CODING_DIR`, else `~/coding` (`%USERPROFILE%\coding` on Windows).
#
# A clone's last use is the time of `rusty-biscuit/.git/FETCH_HEAD`: every
# cross-check run starts with `git fetch`, which rewrites it, and nothing else
# does in normal use. It is language-neutral and costs one file check, where
# build-output dates would need a walk of the whole tree and do not move on a
# run that compiles nothing. A clone that never got as far as a fetch falls
# back to its directory's own time.
#
# A clone is retired when either rule holds and its `.cross-check.lock` is not
# held:
#
#   stale    its prefix is the calling host and its worktree no longer exists
#            on the calling host (only the calling host can know this);
#   expired  its last use is at least CROSS_CHECK_MAX_AGE_DAYS (default 3) days
#            ago, whatever its prefix, so clones from other or renamed calling
#            hosts age out too.
#
# The clone a cross-check run is about to use is never retired. Only
# directories that look like clones (a `rusty-biscuit` child) are candidates,
# so unrelated folders in the coding dir are left alone.
#
# A third rule covers standalone clones: a Git clone of this repository (its
# `origin` URL ends in `rusty-biscuit`) sitting directly in the coding dir,
# outside the `<host>--<worktree>` layout, such as a second checkout made by
# hand. Such a clone is
#
#   abandoned  when nothing in it has been used for CROSS_CHECK_ABANDONED_DAYS
#              (default 7) days, judged by the newest of `.git/HEAD`,
#              `.git/FETCH_HEAD`, and the HEAD reflog of the clone and each of
#              its linked worktrees; and nothing in it exists only here: no
#              stash, no branch or worktree HEAD with commits missing from
#              every remote-tracking ref, and no uncommitted or untracked
#              file in the clone or any linked worktree.
#
# An abandoned clone is retired together with its linked worktrees. One that
# is idle but fails a check is kept and reported with the reason; a check that
# cannot run keeps the clone too. A local run (`cross-check-refresh` on this
# machine's own coding dir) never considers the checkout it was started from.
#
# Retiring is a rename to `.cross-check-trash--…` (instant, and the clone's
# name is free at once) followed by a detached delete, because a Rust clone's
# `target/` takes minutes to delete. Nothing reports whether that delete
# finishes, so every pass re-deletes all leftover trash.

readonly linux=0 windows=1 wsl=2 macos=3
ORDER=(linux windows wsl macos)
declare -a HOST=([linux]="${BUILD_LINUX:-}" [windows]="${BUILD_WIN:-}" [wsl]="${BUILD_WSL:-}" [macos]="${BUILD_MACOS:-}")
declare -a VAR=([linux]=BUILD_LINUX [windows]=BUILD_WIN [wsl]=BUILD_WSL [macos]=BUILD_MACOS)
SSH=(ssh -o BatchMode=yes -o ConnectTimeout=15)
# OpenSSH 10 warns on every connection to a host without post-quantum key
# exchange, which the build hosts do not offer yet. Older clients reject the
# option, so it is added only when this client accepts it (`ssh -G` resolves
# the configuration locally, without connecting).
if ssh -G -o BatchMode=yes -o WarnWeakCrypto=no localhost > /dev/null 2>&1; then
    SSH+=(-o WarnWeakCrypto=no)
fi

PRUNE_MAX_AGE_DAYS="${CROSS_CHECK_MAX_AGE_DAYS:-3}"
if [[ ! "${PRUNE_MAX_AGE_DAYS}" =~ ^[1-9][0-9]*$ ]]; then
    echo "CROSS_CHECK_MAX_AGE_DAYS must be a positive whole number of days, got: ${PRUNE_MAX_AGE_DAYS}" >&2
    exit 2
fi
ABANDONED_MAX_DAYS="${CROSS_CHECK_ABANDONED_DAYS:-7}"
if [[ ! "${ABANDONED_MAX_DAYS}" =~ ^[1-9][0-9]*$ ]]; then
    echo "CROSS_CHECK_ABANDONED_DAYS must be a positive whole number of days, got: ${ABANDONED_MAX_DAYS}" >&2
    exit 2
fi

powershell_quote() {
    printf "'%s'" "${1//\'/\'\'}"
}

local_os() {
    case "$(uname -s)" in
        Darwin) echo macos ;;
        Linux) if grep -qi microsoft /proc/version 2> /dev/null; then echo wsl; else echo linux; fi ;;
        MINGW* | MSYS* | CYGWIN*) echo windows ;;
        *) echo unknown ;;
    esac
}

# One component of a clone directory's name. Lowercased: the Windows host's
# file system is case-insensitive, and two spellings of one checkout must not
# become two clones elsewhere.
offset_part() {
    local value="${1//[^A-Za-z0-9._-]/-}"
    printf '%s' "${value}" | tr '[:upper:]' '[:lower:]'
}

# Sets `short_host`, `origin_host`, `worktree`, `live_worktrees` and
# `protected_checkout` (the main checkout of the caller's repository, which a
# local prune never retires) for the checkout the caller is in.
calling_host_identity() {
    protected_checkout="$(dirname "$(git rev-parse --path-format=absolute --git-common-dir)")"
    short_host="$(hostname)"
    short_host="${short_host%%.*}"
    # The id also names a Git ref, and a host whose name carries `~`, `:`, or
    # a space would make `git update-ref` reject it.
    short_host="${short_host//[^A-Za-z0-9-]/-}"
    if [[ "$(git rev-parse --path-format=absolute --git-dir)" \
        == "$(git rev-parse --path-format=absolute --git-common-dir)" ]]; then
        worktree="main"
    else
        worktree="$(offset_part "$(basename "$(git rev-parse --show-toplevel)")")"
    fi
    # Runs of `-` collapse so the host part never contains `--`: the first
    # `--` in a clone directory's name is always the host/worktree separator,
    # even for a worktree whose own name contains one.
    origin_host="$(offset_part "${short_host}" | tr -s '-')"
    # Every worktree of this repository that still exists here, spelled as its
    # clone directory's `<worktree>` part. The main checkout is listed first;
    # an entry git calls `prunable` has lost its directory and is gone.
    live_worktrees=()
    local first_entry=1 pending="" line
    while IFS= read -r line; do
        case "${line}" in
            "worktree "*)
                if ((first_entry)); then pending="main"; first_entry=0
                else pending="$(offset_part "$(basename "${line#worktree }")")"; fi
                ;;
            prunable*) pending="" ;;
            "")
                [[ -n "${pending}" ]] && live_worktrees+=("${pending}")
                pending=""
                ;;
        esac
    done < <(git worktree list --porcelain; echo)
}

# select_hosts <program> <linux|windows|wsl|macos|all> [optional]: fills `want`.
# An explicit OS for an undeclared host is an error; `all` narrows to the
# declared set minus the local OS and errors when that set is empty, unless
# `optional` is given (a caller that also works on this machine).
select_hosts() {
    local program="$1" os_sel="$2" optional="${3:-}" here os
    here="$(local_os)"
    want=("" "" "" "")
    skipped_local=""
    case "${os_sel}" in linux | windows | wsl | macos | all) ;; *)
        echo "${program}: --os must be linux, windows, wsl, macos, or all" >&2
        exit 2
        ;;
    esac
    if [[ "${os_sel}" == "all" ]]; then
        for os in "${ORDER[@]}"; do
            [[ -n "${HOST[${os}]}" ]] || continue
            if [[ "${os}" == "${here}" ]]; then skipped_local="${os}"; continue; fi
            want[${os}]=1
        done
        # Joined first: `${want[*]// /}` strips each element before joining,
        # so empty elements still join to spaces and the check never fired.
        local joined="${want[*]}"
        if [[ -z "${joined// /}" && -z "${optional}" ]]; then
            echo "${program}: no build host for an OS other than this one (${here}) is declared (set BUILD_LINUX, BUILD_WIN, BUILD_WSL, and/or BUILD_MACOS)" >&2
            exit 2
        fi
    else
        if [[ -z "${HOST[${os_sel}]}" ]]; then
            echo "${program}: --os ${os_sel} requested but ${VAR[${os_sel}]} is not set" >&2
            exit 2
        fi
        want[${os_sel}]=1
    fi
}

# The rules above as a host-side script, in one of two modes:
#
#   prepare  a cross-check run: `own` is the clone it is about to use, which
#            is never retired and is resolved, created and printed as
#            `clone-dir:`. Trash is deleted detached, so no leg waits.
#   refresh  `cross-check-refresh`: nothing is skipped as `own`. Every clone is
#            sized, the volume is measured before and after, and trash is
#            deleted before the after-measurement so it reports what came back.
#
# Lines the caller reads (sizes in bytes):
#   coding: <coding dir>                  refresh only
#   volume-before: <free> <total>         refresh only
#   clone: <bytes> <days unused> <dir>    refresh only, one per clone
#   <stale|expired>-<removed|would|locked|failed>: <dir>[ (N days unused)]
#   abandoned-<removed|would|failed>: <dir>[ (N days unused)]
#   abandoned-kept: <dir> (<why it was kept>)
#   volume-after: <free>                  refresh only
#   clone-dir: <dir>                      prepare only
#
# `protect` is the checkout a local run was started from; it is empty on a
# build host, where the caller's checkout does not live.
unix_prune_script() {
    local mode="$1" own="$2" dry_run="$3" protect="${4:-}" live="" name
    for name in ${live_worktrees[@]+"${live_worktrees[@]}"}; do live+=" ${name}"; done
    cat <<EOF
set -u
coding="\${CODING_DIR:-\$HOME/coding}"
mkdir -p "\$coding" && cd "\$coding" || exit 1
coding="\$(pwd)"
mode='${mode}'
own='${own}'
dry=${dry_run}
max_age=$((PRUNE_MAX_AGE_DAYS * 86400))
abandoned_max=$((ABANDONED_MAX_DAYS * 86400))
protect=$(printf '%q' "${protect}")
now=\$(date +%s)
live='${live} '
mtime() { stat -c %Y "\$1" 2> /dev/null || stat -f %m "\$1"; }
volume() { df -Pk "\$coding" | awk 'NR == 2 { print \$4 * 1024, \$2 * 1024 }'; }
[ "\$mode" = refresh ] && echo "coding: \$coding" && echo "volume-before: \$(volume)"
for dir in "\$coding"/*--*/; do
    [ -d "\$dir" ] || continue
    dir="\${dir%/}"
    name="\${dir##*/}"
    [ "\$name" = "\$own" ] && continue
    [ -d "\$dir/rusty-biscuit" ] || continue
    used="\$dir/rusty-biscuit/.git/FETCH_HEAD"
    [ -f "\$used" ] || used="\$dir"
    age=\$((now - \$(mtime "\$used")))
    if [ "\$mode" = refresh ]; then
        echo "clone: \$(( \$(du -sk "\$dir" | cut -f1) * 1024 )) \$((age / 86400)) \$dir"
    fi
    reason=""
    detail=""
    case "\$name" in
        '${origin_host}'--*)
            case "\$live" in *" \${name#'${origin_host}'--} "*) ;; *) reason=stale ;; esac
            ;;
    esac
    if [ -z "\$reason" ] && [ "\$age" -ge "\$max_age" ]; then
        reason=expired
        detail=" (\$((age / 86400)) days unused)"
    fi
    [ -n "\$reason" ] || continue
    if [ -d "\$dir/.cross-check.lock" ]; then echo "\$reason-locked: \$dir"; continue; fi
    if [ "\$dry" = 1 ]; then echo "\$reason-would: \$dir\$detail"; continue; fi
    if mv "\$dir" "\$coding/.cross-check-trash--\$name-\$\$-\$(date +%s)"; then
        echo "\$reason-removed: \$dir\$detail"
    else
        echo "\$reason-failed: \$dir"
    fi
done
$(unix_abandoned_block)
if [ "\$dry" != 1 ]; then
    for trash in "\$coding"/.cross-check-trash--*/; do
        [ -d "\$trash" ] || continue
        if [ "\$mode" = refresh ]; then
            rm -rf "\${trash%/}"
        else
            # Detached, so the SSH session ends without waiting for it.
            nohup rm -rf "\${trash%/}" < /dev/null > /dev/null 2>&1 &
        fi
    done
fi
[ "\$mode" = refresh ] && echo "volume-after: \$(volume | cut -d' ' -f1)"
if [ -n "\$own" ]; then
    dir="\$coding/\$own"
    mkdir -p "\$dir" || exit 1
    echo "clone-dir: \$dir"
fi
exit 0
EOF
}

# The abandoned-clone rule as host-script text, spliced into
# `unix_prune_script` (quoted, so nothing in it expands here). It reads the
# script's `coding`, `mode`, `dry`, `now`, `abandoned_max`, `protect`, and
# `mtime`.
unix_abandoned_block() {
    cat << 'EOF'
# Prints why the clone whose worktrees (clone first) are listed in $trees must
# be kept, or nothing when every check passed. A check that cannot run keeps it.
abandoned_reason() {
    out="$(git -C "$1" stash list 2>&1)" || { echo "could not read its stashes"; return; }
    [ -z "$out" ] || { echo "it has stashed changes"; return; }
    out="$(git -C "$1" rev-list -n 1 --branches --not --remotes 2>&1)" || { echo "could not read its branches"; return; }
    [ -z "$out" ] || { echo "a branch has commits on no remote"; return; }
    while IFS= read -r tree; do
        [ -n "$tree" ] || continue
        out="$(git -C "$tree" --no-optional-locks status --porcelain 2>&1)" || { echo "could not read the status of $tree"; return; }
        [ -z "$out" ] || { echo "$tree has uncommitted changes"; return; }
        out="$(git -C "$tree" rev-list -n 1 HEAD --not --remotes 2>&1)" || { echo "could not read the history of $tree"; return; }
        [ -z "$out" ] || { echo "$tree has commits on no remote"; return; }
    done << TREES
$trees
TREES
}
for dir in "$coding"/*/; do
    [ -d "$dir" ] || continue
    dir="${dir%/}"
    name="${dir##*/}"
    case "$name" in *--*) continue ;; esac
    [ -d "$dir/.git" ] || continue
    if [ -n "$protect" ] && [ "$dir" -ef "$protect" ]; then continue; fi
    origin="$(git -C "$dir" config --get remote.origin.url 2> /dev/null)"
    case "${origin%/}" in */rusty-biscuit | */rusty-biscuit.git | *:rusty-biscuit | *:rusty-biscuit.git) ;; *) continue ;; esac
    last=0
    for used in "$dir/.git/HEAD" "$dir/.git/FETCH_HEAD" "$dir/.git/logs/HEAD" "$dir"/.git/worktrees/*/HEAD "$dir"/.git/worktrees/*/logs/HEAD; do
        [ -f "$used" ] || continue
        t=$(mtime "$used")
        [ "$t" -gt "$last" ] && last=$t
    done
    age=$((now - last))
    if [ "$mode" = refresh ]; then
        echo "clone: $(( $(du -sk "$dir" | cut -f1) * 1024 )) $((age / 86400)) $dir"
    fi
    [ "$age" -ge "$abandoned_max" ] || continue
    detail=" ($((age / 86400)) days unused)"
    # Every worktree git still knows, the clone itself first; one marked
    # prunable has lost its directory and holds nothing.
    if ! listing="$(git -C "$dir" worktree list --porcelain 2>&1)"; then
        echo "abandoned-kept: $dir (could not list its worktrees)"
        continue
    fi
    trees="$(printf '%s\n\n' "$listing" | awk '/^worktree /{p=substr($0,10)} /^prunable/{p=""} /^$/{if(p!="")print p; p=""}')"
    why="$(abandoned_reason "$dir")"
    if [ -n "$why" ]; then echo "abandoned-kept: $dir ($why)"; continue; fi
    if [ "$dry" = 1 ]; then echo "abandoned-would: $dir$detail"; continue; fi
    stamp="$$-$(date +%s)"
    failed=""
    i=0
    while IFS= read -r tree; do
        i=$((i + 1))
        # The first entry is the clone itself, moved last.
        [ "$i" -gt 1 ] && [ -n "$tree" ] || continue
        parent="$(dirname "$tree")"
        if mv "$tree" "$coding/.cross-check-trash--$name-worktree$i-$stamp"; then
            rmdir "$parent" 2> /dev/null
        else
            failed="$tree"
            break
        fi
    done << TREES
$trees
TREES
    if [ -n "$failed" ]; then echo "abandoned-failed: $failed"; continue; fi
    if mv "$dir" "$coding/.cross-check-trash--$name-$stamp"; then
        echo "abandoned-removed: $dir$detail"
    else
        echo "abandoned-failed: $dir"
    fi
done
EOF
}

# The same rules in PowerShell 5. The trash delete is started through WMI
# rather than as a child: Windows OpenSSH ends a session's child processes
# with it. `\\?\` lifts the 260-character path limit a deep `target\` exceeds.
windows_prune_script() {
    local mode="$1" own="$2" dry_run="$3" protect="${4:-}" live="" name dry='$false'
    for name in ${live_worktrees[@]+"${live_worktrees[@]}"}; do live+="'${name}',"; done
    [[ "${dry_run}" == 1 ]] && dry='$true'
    cat <<EOF
\$ProgressPreference = 'SilentlyContinue'
\$coding = if (\$env:CODING_DIR) { \$env:CODING_DIR } else { Join-Path \$env:USERPROFILE 'coding' }
New-Item -ItemType Directory -Force -Path \$coding -ErrorAction Stop | Out-Null
\$coding = (Resolve-Path -LiteralPath \$coding).Path
\$mode = '${mode}'
\$own = $(powershell_quote "${own}")
\$dry = ${dry}
\$maxAge = [TimeSpan]::FromDays(${PRUNE_MAX_AGE_DAYS})
\$abandonedMax = [TimeSpan]::FromDays(${ABANDONED_MAX_DAYS})
\$protect = $(powershell_quote "${protect}")
\$now = [DateTime]::UtcNow
# .NET enumeration reads sizes from the directory listing; Get-ChildItem
# builds an object per file and is several times slower on a target tree. It
# is the fallback when an unreadable entry stops enumeration.
function Get-TreeBytes(\$dir) {
    \$bytes = [long]0
    try {
        foreach (\$file in (New-Object System.IO.DirectoryInfo \$dir).EnumerateFiles('*', [System.IO.SearchOption]::AllDirectories)) { \$bytes += \$file.Length }
    } catch {
        \$bytes = [long](Get-ChildItem -LiteralPath \$dir -Recurse -File -Force -ErrorAction SilentlyContinue | Measure-Object -Property Length -Sum).Sum
    }
    return [long]\$bytes
}
\$live = @(${live%,})
\$prefix = '${origin_host}--'
\$drive = New-Object System.IO.DriveInfo ([System.IO.Path]::GetPathRoot(\$coding))
if (\$mode -eq 'refresh') { Write-Output "coding: \$coding"; Write-Output "volume-before: \$(\$drive.AvailableFreeSpace) \$(\$drive.TotalSize)" }
foreach (\$entry in Get-ChildItem -LiteralPath \$coding -Directory -Force) {
    \$name = \$entry.Name
    if (\$name.StartsWith('.') -or \$name.IndexOf('--') -lt 1) { continue }
    if (\$own -and \$name -ieq \$own) { continue }
    \$dir = \$entry.FullName
    if (-not (Test-Path -LiteralPath (Join-Path \$dir 'rusty-biscuit'))) { continue }
    \$used = Join-Path \$dir 'rusty-biscuit\\.git\\FETCH_HEAD'
    \$touched = if (Test-Path -LiteralPath \$used) { (Get-Item -LiteralPath \$used -Force).LastWriteTimeUtc } else { \$entry.LastWriteTimeUtc }
    \$age = \$now - \$touched
    \$days = [int][math]::Floor(\$age.TotalDays)
    if (\$mode -eq 'refresh') { Write-Output "clone: \$(Get-TreeBytes \$dir) \$days \$dir" }
    \$reason = ''
    \$detail = ''
    if (\$name.StartsWith(\$prefix, [StringComparison]::OrdinalIgnoreCase) -and -not (\$live -contains \$name.Substring(\$prefix.Length))) {
        \$reason = 'stale'
    } elseif (\$age -ge \$maxAge) {
        \$reason = 'expired'
        \$detail = " (\$days days unused)"
    }
    if (-not \$reason) { continue }
    if (Test-Path -LiteralPath (Join-Path \$dir '.cross-check.lock')) { Write-Output "\$reason-locked: \$dir"; continue }
    if (\$dry) { Write-Output "\$reason-would: \$dir\$detail"; continue }
    \$trash = ".cross-check-trash--\$name-\$PID-\$([DateTimeOffset]::UtcNow.ToUnixTimeSeconds())"
    try { Rename-Item -LiteralPath \$dir -NewName \$trash -ErrorAction Stop; Write-Output "\$reason-removed: \$dir\$detail" }
    catch { Write-Output "\$reason-failed: \$dir" }
}
$(windows_abandoned_block)
if (-not \$dry) {
    foreach (\$entry in Get-ChildItem -LiteralPath \$coding -Directory -Force) {
        if (-not \$entry.Name.StartsWith('.cross-check-trash--', [StringComparison]::OrdinalIgnoreCase)) { continue }
        \$line = 'cmd.exe /c rd /s /q "\\\\?\\' + \$entry.FullName + '"'
        \$started = Invoke-CimMethod -ClassName Win32_Process -MethodName Create -Arguments @{ CommandLine = \$line }
        # A refresh measures the volume afterwards, so it waits for the delete.
        if (\$mode -eq 'refresh' -and \$started.ProcessId) { Wait-Process -Id \$started.ProcessId -ErrorAction SilentlyContinue }
    }
}
if (\$mode -eq 'refresh') {
    \$drive = New-Object System.IO.DriveInfo ([System.IO.Path]::GetPathRoot(\$coding))
    Write-Output "volume-after: \$(\$drive.AvailableFreeSpace)"
}
if (\$own) {
    \$dir = Join-Path \$coding \$own
    New-Item -ItemType Directory -Force -Path \$dir -ErrorAction Stop | Out-Null
    Write-Output "clone-dir: \$dir"
}
EOF
}

# The abandoned-clone rule in PowerShell 5, spliced into
# `windows_prune_script` (quoted, so nothing in it expands here). It reads the
# script's `$coding`, `$mode`, `$dry`, `$now`, `$abandonedMax`, `$protect`, and
# `Get-TreeBytes`. Without `git` on the host the rule is skipped, never guessed.
windows_abandoned_block() {
    cat << 'EOF'
# Why the clone (first of $trees) must be kept, or '' when every check passed.
# A check that cannot run keeps it.
function Get-AbandonedReason($trees) {
    $clone = $trees[0]
    $out = & git -C $clone stash list 2>&1
    if ($LASTEXITCODE -ne 0) { return 'could not read its stashes' }
    if ($out) { return 'it has stashed changes' }
    $out = & git -C $clone rev-list -n 1 --branches --not --remotes 2>&1
    if ($LASTEXITCODE -ne 0) { return 'could not read its branches' }
    if ($out) { return 'a branch has commits on no remote' }
    foreach ($tree in $trees) {
        $out = & git -C $tree --no-optional-locks status --porcelain 2>&1
        if ($LASTEXITCODE -ne 0) { return "could not read the status of $tree" }
        if ($out) { return "$tree has uncommitted changes" }
        $out = & git -C $tree rev-list -n 1 HEAD --not --remotes 2>&1
        if ($LASTEXITCODE -ne 0) { return "could not read the history of $tree" }
        if ($out) { return "$tree has commits on no remote" }
    }
    return ''
}
$protectFull = if ($protect) { [System.IO.Path]::GetFullPath($protect).TrimEnd('\') } else { '' }
$entries = if (Get-Command git -CommandType Application -ErrorAction SilentlyContinue) { @(Get-ChildItem -LiteralPath $coding -Directory -Force) } else { @() }
foreach ($entry in $entries) {
    $name = $entry.Name
    if ($name.StartsWith('.') -or $name.Contains('--')) { continue }
    $dir = $entry.FullName
    if (-not (Test-Path -LiteralPath (Join-Path $dir '.git') -PathType Container)) { continue }
    if ($protectFull -and ($dir.TrimEnd('\') -ieq $protectFull)) { continue }
    $origin = (& git -C $dir config --get remote.origin.url 2>$null | Select-Object -First 1)
    if (-not ($origin -match '[/\\:]rusty-biscuit(\.git)?[/\\]?$')) { continue }
    $last = [DateTime]::MinValue
    $used = @('HEAD', 'FETCH_HEAD', 'logs\HEAD') | ForEach-Object { Join-Path $dir ".git\$_" }
    $linked = Join-Path $dir '.git\worktrees'
    if (Test-Path -LiteralPath $linked) {
        foreach ($admin in Get-ChildItem -LiteralPath $linked -Directory -Force) { $used += (Join-Path $admin.FullName 'HEAD'); $used += (Join-Path $admin.FullName 'logs\HEAD') }
    }
    foreach ($file in $used) {
        if (Test-Path -LiteralPath $file -PathType Leaf) {
            $t = (Get-Item -LiteralPath $file -Force).LastWriteTimeUtc
            if ($t -gt $last) { $last = $t }
        }
    }
    $age = $now - $last
    $days = [int][math]::Floor($age.TotalDays)
    if ($mode -eq 'refresh') { Write-Output "clone: $(Get-TreeBytes $dir) $days $dir" }
    if ($age -lt $abandonedMax) { continue }
    $detail = " ($days days unused)"
    # Every worktree git still knows, the clone itself first; one marked
    # prunable has lost its directory and holds nothing.
    $listing = & git -C $dir worktree list --porcelain 2>$null
    if ($LASTEXITCODE -ne 0) { Write-Output "abandoned-kept: $dir (could not list its worktrees)"; continue }
    $trees = @()
    $pending = $null
    foreach ($line in @($listing) + '') {
        if ($line.StartsWith('worktree ')) { $pending = $line.Substring(9) }
        elseif ($line.StartsWith('prunable')) { $pending = $null }
        elseif ($line -eq '') { if ($pending) { $trees += $pending }; $pending = $null }
    }
    if ($trees.Count -eq 0) { Write-Output "abandoned-kept: $dir (could not list its worktrees)"; continue }
    $why = Get-AbandonedReason $trees
    if ($why) { Write-Output "abandoned-kept: $dir ($why)"; continue }
    if ($dry) { Write-Output "abandoned-would: $dir$detail"; continue }
    $stamp = "$PID-$([DateTimeOffset]::UtcNow.ToUnixTimeSeconds())"
    $failed = ''
    # The first entry is the clone itself, moved last.
    for ($i = 1; $i -lt $trees.Count; $i++) {
        $tree = [System.IO.Path]::GetFullPath($trees[$i])
        $parent = Split-Path -Parent $tree
        try {
            Move-Item -LiteralPath $tree -Destination (Join-Path $coding ".cross-check-trash--$name-worktree$i-$stamp") -ErrorAction Stop
            # Non-recursive: deletes the parent only when the move left it empty.
            try { [System.IO.Directory]::Delete($parent) } catch {}
        } catch { $failed = $tree; break }
    }
    if ($failed) { Write-Output "abandoned-failed: $failed"; continue }
    try { Rename-Item -LiteralPath $dir -NewName ".cross-check-trash--$name-$stamp" -ErrorAction Stop; Write-Output "abandoned-removed: $dir$detail" }
    catch { Write-Output "abandoned-failed: $dir" }
}
EOF
}

# Bytes as a short human figure (GiB with one decimal, or MiB below 1 GiB).
human_bytes() {
    awk -v b="$1" 'BEGIN { if (b >= 1073741824) printf "%.1f GiB", b / 1073741824; else printf "%.0f MiB", b / 1048576 }'
}

# prune_host <program> <os|local> <prepare|refresh> <own clone or ""> <dry_run 0|1>
# Runs the rules on one host and reports each outcome; a refresh also reports
# the before and after state of the volume. `local` runs them on this machine's
# own coding dir instead, without SSH, protecting the caller's checkout. Sets
# PRUNED_CLONE_DIR to the `clone-dir:` line (empty for a refresh). Fails only
# when the host could not be reached or the script could not run.
prune_host() {
    local program="$1" os="$2" mode="$3" own="$4" dry_run="$5" host output
    PRUNED_CLONE_DIR=""
    if [[ "${os}" == "local" ]]; then
        host="local"
        if [[ "$(local_os)" == "windows" ]]; then
            local encoded
            encoded="$(windows_prune_script "${mode}" "${own}" "${dry_run}" "${protected_checkout}" | iconv -f UTF-8 -t UTF-16LE | base64 | tr -d '\n')"
            output="$(powershell.exe -NoProfile -EncodedCommand "${encoded}")" || return 1
        else
            output="$(unix_prune_script "${mode}" "${own}" "${dry_run}" "${protected_checkout}" | bash -s)" || return 1
        fi
        report_prune "${program}" "${host}" "${mode}" "${dry_run}" "${output}"
        return 0
    fi
    # Indexed only after the `local` case: `local` would read as arithmetic.
    host="${HOST[$2]}"
    if [[ "${os}" == "windows" ]]; then
        # Encoded rather than inline: the command crosses the remote shell once
        # more, and `$` does not reliably survive that on this host. Windows
        # caps a command line at 32,767 characters; the encoded script was
        # about 22,000 when the abandoned-clone rule was added.
        local encoded
        encoded="$(windows_prune_script "${mode}" "${own}" "${dry_run}" | iconv -f UTF-8 -t UTF-16LE | base64 | tr -d '\n')"
        output="$("${SSH[@]}" "${host}" "powershell -NoProfile -EncodedCommand ${encoded}")" || return 1
    else
        output="$(unix_prune_script "${mode}" "${own}" "${dry_run}" | "${SSH[@]}" "${host}" "bash -ls")" || return 1
    fi
    report_prune "${program}" "${host}" "${mode}" "${dry_run}" "${output}"
    # The last marker line wins, so a chatty login profile cannot corrupt it.
    PRUNED_CLONE_DIR="$(printf '%s\n' "${output}" | tr -d '\r' | sed -n 's/^clone-dir: //p' | tail -n1)"
}

# report_prune <program> <host> <prepare|refresh> <dry_run 0|1> <host output>
# Turns a host script's lines into the report: one line per outcome for a
# cross-check run, and a before/after report for a refresh. Paths are compared
# as plain bash strings, never handed to awk or sed: a Windows path's
# backslashes would be read as escapes and match nothing.
report_prune() {
    local program="$1" host="$2" mode="$3" dry_run="$4" output line rest bytes days dir i
    local coding="" free_before=0 total=0 free_after=0 removed_bytes=0 all_bytes=0
    local -a clone_bytes=() clone_days=() clone_dirs=()
    local -a removed_dirs=() removed_sizes=() removed_why=() kept_locked=() kept_idle=() failed=()
    output="$(printf '%s\n' "$5" | tr -d '\r')"

    while IFS= read -r line; do
        rest="${line#*: }"
        case "${line}" in
            "coding: "*) coding="${rest}" ;;
            "volume-before: "*) read -r free_before total <<< "${rest}" ;;
            "volume-after: "*) free_after="${rest}" ;;
            "clone: "*)
                read -r bytes days dir <<< "${rest}"
                clone_bytes+=("${bytes}")
                clone_days+=("${days}")
                clone_dirs+=("${dir}")
                all_bytes=$((all_bytes + bytes))
                ;;
            *"-removed: "* | *"-would: "*)
                dir="${rest% (* days unused)}"
                bytes=0
                for i in "${!clone_dirs[@]}"; do
                    if [[ "${clone_dirs[i]}" == "${dir}" ]]; then bytes="${clone_bytes[i]}"; break; fi
                done
                removed_dirs+=("${dir}")
                removed_sizes+=("${bytes}")
                removed_bytes=$((removed_bytes + bytes))
                days="${rest##* (}"
                case "${line}" in
                    stale-*) removed_why+=("worktree no longer exists") ;;
                    abandoned-*) removed_why+=("standalone clone unused ${days% days unused)} days, nothing unpushed") ;;
                    *) removed_why+=("unused ${days% days unused)} days") ;;
                esac
                ;;
            *"-locked: "*) kept_locked+=("${rest}") ;;
            "abandoned-kept: "*) kept_idle+=("${rest}") ;;
            *"-failed: "*) failed+=("${rest}") ;;
        esac
    done <<< "${output}"

    if [[ "${mode}" != "refresh" ]]; then
        for i in "${!removed_dirs[@]}"; do
            echo "${program}: removing clone ${host}:${removed_dirs[i]} in the background (${removed_why[i]})"
        done
        for dir in ${kept_locked[@]+"${kept_locked[@]}"}; do echo "${program}: kept clone ${host}:${dir}; its lock is held"; done
        for line in ${kept_idle[@]+"${kept_idle[@]}"}; do
            echo "${program}: kept idle clone ${host}:${line% (*}; $(kept_reason "${line}")"
        done
        for dir in ${failed[@]+"${failed[@]}"}; do echo "${program}: could not remove clone ${host}:${dir}; remove it by hand" >&2; done
        return 0
    fi

    echo
    printf 'BEFORE  %s free of %s (%s used) in %s\n' "$(human_bytes "${free_before}")" "$(human_bytes "${total}")" "$(percent_used "${free_before}" "${total}")" "${coding:-?}"
    echo
    if ((${#clone_dirs[@]} == 0)); then
        echo "  no clones"
    else
        printf '  %10s  %6s  %s\n' SIZE UNUSED CLONE
        # Largest first; the index rides along so the path is never parsed.
        for i in $(for i in "${!clone_bytes[@]}"; do echo "${clone_bytes[i]} ${i}"; done | sort -rn | cut -d' ' -f2); do
            printf '  %10s  %5sd  %s\n' "$(human_bytes "${clone_bytes[i]}")" "${clone_days[i]}" "$(clone_name "${clone_dirs[i]}" "${coding}")"
        done
        printf '  %10s\n' "----------"
        printf '  %10s  %6s  %s\n' "$(human_bytes "${all_bytes}")" "" "$(plural ${#clone_dirs[@]} clone)"
    fi

    echo
    if [[ "${dry_run}" == 1 ]]; then echo "AFTER (dry run: nothing was removed)"; else echo "AFTER"; fi
    echo
    if ((${#removed_dirs[@]} == 0)); then
        echo "  nothing to remove; $(human_bytes "${free_before}") free ($(percent_used "${free_before}" "${total}") used)"
        print_kept
        return 0
    else
        local heading="REMOVED" width name
        [[ "${dry_run}" == 1 ]] && heading="WOULD REMOVE"
        width=${#heading}
        for dir in "${removed_dirs[@]}"; do
            name="$(clone_name "${dir}" "${coding}")"
            ((${#name} > width)) && width=${#name}
        done
        printf '  %10s  %-*s  %s\n' SIZE "${width}" "${heading}" WHY
        for i in $(for i in "${!removed_sizes[@]}"; do echo "${removed_sizes[i]} ${i}"; done | sort -rn | cut -d' ' -f2); do
            printf '  %10s  %-*s  %s\n' "$(human_bytes "${removed_sizes[i]}")" "${width}" "$(clone_name "${removed_dirs[i]}" "${coding}")" "${removed_why[i]}"
        done
        printf '  %10s\n' "----------"
        printf '  %10s  %s\n' "$(human_bytes "${removed_bytes}")" "$(plural ${#removed_dirs[@]} clone)"
    fi
    ((${#kept_locked[@]} + ${#kept_idle[@]} > 0)) && echo
    print_kept
    for dir in ${failed[@]+"${failed[@]}"}; do
        printf '  could not remove %s; remove it by hand\n' "${dir}" >&2
    done
    echo
    if [[ "${dry_run}" == 1 ]]; then
        printf '  free space  %s -> about %s (%s used)\n' "$(human_bytes "${free_before}")" \
            "$(human_bytes $((free_before + removed_bytes)))" "$(percent_used $((free_before + removed_bytes)) "${total}")"
    else
        printf '  free space  %s -> %s (%s used)\n' "$(human_bytes "${free_before}")" "$(human_bytes "${free_after}")" "$(percent_used "${free_after}" "${total}")"
        # Free space is measured on a live volume, so other writes can hide a
        # small gain; never report a negative reclaim.
        if ((free_after > free_before)); then
            printf '  reclaimed   %s\n' "$(human_bytes $((free_after - free_before)))"
        else
            printf '  reclaimed   none measurable (other writes on the volume offset it)\n'
        fi
    fi
}

# The kept clones of the report being built (reads report_prune's locals).
print_kept() {
    local dir line
    for dir in ${kept_locked[@]+"${kept_locked[@]}"}; do
        printf '  kept %s: in use (its lock is held)\n' "$(clone_name "${dir}" "${coding}")"
    done
    for line in ${kept_idle[@]+"${kept_idle[@]}"}; do
        printf '  kept %s: %s\n' "$(clone_name "${line% (*}" "${coding}")" "$(kept_reason "${line}")"
    done
}

# kept_reason "<dir> (<why>)": the why of an `abandoned-kept:` line, as a
# phrase that follows the clone's name.
kept_reason() {
    local why="${1##* (}"
    printf 'unused, but %s' "${why%)}"
}

# A clone's directory name, without the coding dir it lives in.
clone_name() {
    local name="${1#"$2"}"
    name="${name#/}"
    printf '%s' "${name#\\}"
}

# percent_used <free bytes> <total bytes>
percent_used() {
    if (($2 > 0)); then echo "$(( ($2 - $1) * 100 / $2 ))%"; else echo "?%"; fi
}

# plural <count> <noun>
plural() {
    if (($1 == 1)); then echo "1 $2"; else echo "$1 $2s"; fi
}
