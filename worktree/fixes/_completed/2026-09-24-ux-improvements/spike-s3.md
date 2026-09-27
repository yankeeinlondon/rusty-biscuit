---
spike: S3
date: 2026-09-24
---

# Spike S3: prompts and protocol lines through captured-stdout wrappers

A throwaway probe binary (`inquire` 0.9.4, the version `worktree-cli` locks) printed its stdin, stderr, and
stdout TTY state and `WT_SHELL_WRAPPER` to stderr, asked an `inquire::Confirm`, then printed on stdout a
plain line, `cd:/tmp/s3 dir/café-ü`, and `remove-handoff:0123abcd$(touch /tmp/s3-pwned)`. Candidate wrappers
ran in detached tmux sessions (no window focus) under clean shells: zsh 5.9 (`-f`), bash 5.3
(`--norc --noprofile`), fish 4.9.3 (`-N`, installed on this host with Homebrew for the spike; Phase 3's L2
tests need it too). PowerShell ran on `$BUILD_WIN` under `ssh -tt` (Windows PowerShell 5.1.26100; `pwsh`
is not installed there).

## Results

| Check | zsh | bash | fish | PowerShell 5.1 |
|---|---|---|---|---|
| `inquire` renders on stderr and takes the answer while stdout is captured | yes | yes | yes | stdin and stderr stay TTYs (not driven interactively) |
| Variable visible to the one invocation only | `WT_SHELL_WRAPPER=1 command wt` | same | `WT_SHELL_WRAPPER=1 command wt` (fish 3.1+) | set, then restore in `finally` |
| `cd:` path with a space and non-ASCII characters | exact | exact | exact | **mojibake with the default encoding; exact with UTF-8** |
| Token passed as one argument; `$(…)` in it never runs | yes | yes | yes | n/a (not driven) |
| Failed `cd` stops before the handoff | — | — | yes, exit 1, shell stays put | — |

`inquire` writes through `stderr()` (`inquire-0.9.4/src/terminal/crossterm.rs:97`, `console.rs:21`), which
is why prompts survive capture.

## Findings

1. **PowerShell re-encodes captured native output.** Windows PowerShell 5.1 decodes a native command's
   stdout with `[Console]::OutputEncoding`, which was IBM437 on the build host: `café-ü日` arrived as
   `caf├⌐-├╝µùÑ`. Setting `[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)` for the
   call (restored in `finally`) gave the exact string. **The PowerShell wrapper must do this**, or
   non-ASCII worktree paths break `cd:`. Lines arrive as an array of strings with no trailing CR.
2. **POSIX wrappers must not run the handoff inside a `while read` loop fed by a here-doc.** The second
   `wt remove --handoff` inherited the here-doc as stdin (`stdin_tty=false`). The handoff run asks nothing,
   so this is harmless today, but the cleaner shape collects the `cd:` and `remove-handoff:` values in the
   loop and acts after it, so the handoff runs with the terminal's stdin.
3. **Env scoping** works as the plan expects in all four shells. fish's `VAR=value command` form needs fish
   3.1 (2020); `env WT_SHELL_WRAPPER=1 wt` is the fallback for older fish.
4. **The handoff invocation must also set `WT_SHELL_WRAPPER=1`**, because the second run may print a `cd:`
   line of its own in future and must not be treated as wrapper-less.
5. The wrapper never evaluates output: the token containing `$(touch …)` reached `wt` as one literal
   argument in every shell, and the file was never created.

## Shapes to carry into Phase 2

POSIX (bash, zsh):

```sh
wt() {
  local out line rc dest="" token=""
  out="$(WT_SHELL_WRAPPER=1 command wt "$@")"; rc=$?
  while IFS= read -r line; do
    case "$line" in
      cd:*) dest="${line#cd:}" ;;
      remove-handoff:*) token="${line#remove-handoff:}" ;;
      *) printf '%s\n' "$line" ;;
    esac
  done <<EOF
$out
EOF
  [ "$rc" -eq 0 ] || return "$rc"
  [ -n "$dest" ] && { cd -- "$dest" || return 1; }
  [ -n "$token" ] && WT_SHELL_WRAPPER=1 command wt remove --handoff "$token"
}
```

PowerShell (outline):

```powershell
function wt {
  $prevVar = $env:WT_SHELL_WRAPPER; $prevEnc = [Console]::OutputEncoding
  $env:WT_SHELL_WRAPPER = '1'; [Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)
  try { $out = & wt.exe @args; $rc = $LASTEXITCODE }
  finally { [Console]::OutputEncoding = $prevEnc; if ($null -eq $prevVar) { Remove-Item Env:WT_SHELL_WRAPPER } else { $env:WT_SHELL_WRAPPER = $prevVar } }
  # collect cd:/remove-handoff: from $out; on success Set-Location -LiteralPath $dest and
  # [Environment]::CurrentDirectory = (Get-Location -PSProvider FileSystem).ProviderPath, then run the handoff
}
```

Not verified here: the PowerShell wrapper end to end with a real prompt. That belongs to Phase 3's Windows
L2 test.
