# Build Hosts

Remote build hosts are declared through environment variables, never named in
scripts or docs. A machine that sets a variable is saying that host is
available from it; an unset variable means "not from here". Check with
`env | grep '^BUILD_'` before planning cross-OS evidence, and say which hosts
were declared when you report.

| Variable | Provides | Standing clone | Notes |
|---|---|---|---|
| `BUILD_LINUX` | Native Linux | `~/ci-verification/rusty-biscuit` | Target of `just cross-check --host linux`. |
| `BUILD_WIN` | Native Windows, PowerShell as the remote shell | `W:\ci-verification\rusty-biscuit` | Target of `just cross-check --host windows`. Git's stderr shows as a red `NativeCommandError`; harmless. |
| `BUILD_WSL` | A WSL2 Ubuntu guest | `~/ci-verification/rusty-biscuit` | Target of `just cross-check --host wsl`, which runs CI's archive mode ([wsl.md](wsl.md)). For ad hoc commands, non-login shells lack `~/.cargo/bin`; wrap them in `bash -lc`. |
| `BUILD_MACOS` | A macOS host other than the current one | none yet | Reserved; no recipe consumes it today. |

Each value is an SSH destination: an alias from the developer's
`~/.ssh/config` or `user@host`. Use it as `ssh -o BatchMode=yes "$BUILD_WSL"
...` so a missing key fails fast instead of prompting. The `post-quantum`
warnings SSH prints are noise.

`BUILD_WIN` and `BUILD_WSL` are commonly the same physical machine. If the
WSL guest's sshd hangs while the distro is running, go through the Windows
side: `ssh "$BUILD_WIN" 'wsl.exe -d <distro> -e <argv>'` and strip NULs from
the output.

## `just cross-check`

`scripts/cross-check.sh` (root `just cross-check <package> [--host
linux|windows|wsl|all] [nextest args]`) syncs a standing clone to
`origin/<branch>` (falls back to `origin/main` when the branch is unpushed),
applies the local tree's difference as one patch (tracked and untracked
files), and runs the package's L1 suite there. Compile caches stay warm
between runs. It reads `BUILD_LINUX`, `BUILD_WIN`, and `BUILD_WSL`; the
default `--host all` runs on whichever are set, an explicit `--host` for an
undeclared one is an error, and no declared host at all is an error rather
than a silent pass. The `wsl` host runs CI's nextest archive mode with the
builder target directory hidden, so it catches the class of failure that
only the `wsl2-ubuntu` CI leg sees; feature flags are routed to the archive
build and everything else to the run.

- Verify the banner line `cross-check: <pkg> @ origin/<branch> (<sha>) + N
  patch line(s)` before trusting a result. If in doubt, confirm the remote
  head: `ssh "$BUILD_WIN" "git -C W:\ci-verification\rusty-biscuit log -1
  --oneline"`.
- A clone left dirty by an earlier failed patch used to make the checkout
  fail silently and apply the patch onto the old tree. The script now resets
  and cleans first; recover an older clone by hand with `git reset --hard;
  git clean -fdq; git checkout --detach <sha>` over SSH.
- Positional filter args are nextest test-name substrings. A binary name
  matches nothing ("0 tests run"). `-E 'test(...)'` breaks the recipe's
  unquoted argument line; pass several name substrings instead.
- Child-process stderr is not shown by a remote test failure. A probe that
  must be read back can append to `W:\ci-verification\probe.txt` and be read
  with `ssh "$BUILD_WIN" "Get-Content W:\ci-verification\probe.txt"`.

## Storage rules on the Windows host

The Windows host has a small system drive and a large `W:` volume that holds
the WSL VHDX and every Cargo target. The checkout's `.cargo/config.toml` pins
`target-dir` to `W:`. **Never override `CARGO_TARGET_DIR` there**; doing so
once filled the system drive to zero bytes and froze the host.

- `just` recipes run a storage preflight (`scripts/storage-preflight.sh`)
  that refuses to build below 50 GiB free. It is not bypassable and it is not
  advisory; when it fails, free space or use another host.
- Scratch checkouts and their target directories are outside every sweep's
  scope. Delete your own scratch targets before reporting, on both the
  Windows side and inside the WSL guest.
- Check free space first: `ssh "$BUILD_WIN" "Get-PSDrive C, W"`; inside
  WSL, `ssh "$BUILD_WSL" 'df -h ~'`.

## Remote-process hygiene

- Stopping a local background task does not stop a backgrounded remote
  PowerShell pipeline; kill the parent `powershell.exe` PID or it starts the
  next cargo command.
- Run remote work through `just` gates where one exists rather than raw
  `cargo`, so timing records and preflights apply.
- When copying a single changed file to a standing clone for a quick check,
  `scp` it and then confirm `git status --short` on the remote shows only that
  file before running.
