# Build Hosts

Remote build hosts are declared through environment variables, never named in
scripts or docs. A machine that sets a variable is saying that host is
available from it; an unset variable means "not from here". Check with
`env | grep '^BUILD_'` before planning cross-OS evidence, and say which hosts
were declared when you report.

| Variable | Provides | Notes |
|---|---|---|
| `BUILD_LINUX` | Native Linux | Target of `just cross-check --os linux`. |
| `BUILD_WIN` | Native Windows, PowerShell as the remote shell | Target of `just cross-check --os windows`. Git's stderr shows as a red `NativeCommandError`; harmless. |
| `BUILD_WSL` | A WSL2 Ubuntu guest | Target of `just cross-check --os wsl`, which runs CI's archive mode ([wsl.md](wsl.md)). For ad hoc commands, non-login shells lack `~/.cargo/bin`; wrap them in `bash -lc`. |
| `BUILD_MACOS` | A macOS host other than the current one | Target of `just cross-check --os macos`; same flow as Linux. |

### Standing clone layout

`cross-check` assumes nothing about a host's disk. Each host keeps one
standing clone per originating checkout, at
`<coding dir>/<local host>--<worktree>/rusty-biscuit`:

- `<coding dir>` is the **host's** `CODING_DIR` (its login environment on
  Unix, its user environment on Windows), else `~/coding`
  (`%USERPROFILE%\coding` on Windows). It is resolved over SSH at the start
  of each leg and printed as `clone: <host>:<dir>`.
- `<local host>--<worktree>` is the machine running `cross-check` and the
  linked worktree's directory name, or `main` for the main checkout, lowercased
  (e.g. `shazam--feat-dark-fixes`). Runs of `-` in the host name collapse, so
  the first `--` is always the separator.

On 2026-09-22 `build-win-native` set `CODING_DIR=B:\coding` (user scope; `B:`
is a fixed ReFS volume), and `build-linux` and the WSL guest set none, so they
use `~/coding`. **On Windows, an unset `CODING_DIR` falls back to the small
`C:` system drive**, which has frozen that host once already (see below), so
keep it set there.

A clone is created only when a checkout first runs `cross-check`, so a host
holds one clone (and one `target/`) per origin worktree that has used it, and
no two worktrees ever share one. Before any leg starts, each selected host
removes this origin host's `<host>--*` clones whose worktree no longer exists
here (absent from `git worktree list`, or listed as `prunable`). Removal is a
rename to `.cross-check-trash--<host>--…` followed by a detached delete (on
Windows started through WMI, since OpenSSH kills a session's children, with a
`\\?\` path for deep `target\` trees), so no leg waits on it; each run also
re-deletes its origin's leftover trash. A clone whose lock is held is kept and
reported; a rename that fails is reported and does not stop the run. Hosts not
selected for a run are not pruned by it. The
pre-2026-09-22 shared clones (`~/ci-verification`, `W:\ci-verification`) are
no longer used, and deleting them is the host owner's call.

`cross-check` forwards `BISCUIT_TEST_REQUIRED_BACKENDS` from your shell to the
remote run. Set it whenever you run a Level 2 filter remotely — a backend the
host lacks otherwise skips, and nextest prints PASS in ~0.02 s. The Unix legs
also never read the remote `~/.config` (`GIT_CONFIG_GLOBAL=/dev/null`, empty
`XDG_CONFIG_HOME`), so a down NAS cannot fail them ([wsl.md](wsl.md)).

Each value is an SSH destination: an alias from the developer's
`~/.ssh/config` or `user@host`. Use it as `ssh -o BatchMode=yes "$BUILD_WSL"
...` so a missing key fails fast instead of prompting. The `post-quantum`
warnings SSH prints are noise.

For an ad hoc PowerShell script on `BUILD_WIN`, copy it over and run it with
`-File`; do not pipe it into `powershell -Command -`. Piped input is read
line by line, and on 2026-09-24 a script with multi-line `foreach`/`switch`
blocks printed nothing and reported no error. The remote shell is
PowerShell, so `%TEMP%` stays literal; quote the command for your local shell
and use `$env:TEMP`:

```sh
scp -q probe.ps1 "$BUILD_WIN":AppData/Local/Temp/probe.ps1
ssh -o BatchMode=yes "$BUILD_WIN" 'powershell -NoProfile -ExecutionPolicy Bypass -File $env:TEMP\probe.ps1; Remove-Item $env:TEMP\probe.ps1'
```

`BUILD_WIN` and `BUILD_WSL` are commonly the same physical machine. If the
WSL guest's sshd hangs while the distro is running, go through the Windows
side: `ssh "$BUILD_WIN" 'wsl.exe -d <distro> -e <argv>'` and strip NULs from
the output.

## `just cross-check`

`scripts/cross-check.sh` (root `just cross-check <package> [--os
linux|windows|wsl|macos|all] [nextest args]`) commits your working tree
(tracked edits and untracked, non-ignored files) as one throwaway commit over
`origin/<branch>` (falls back to `origin/main` when the branch is unpushed),
ships that commit as a `git bundle`, checks the standing clone out at it, and
runs the package's L1 suite there. Your branch, index, and stash stack are
untouched and nothing is signed. The commit is the tested revision and the
plan's `head`, which is what lets archive mode run at all: `ci-build` refuses
any checkout that is not `plan.head` or whose tracked tree is dirty. Compile caches stay warm
between runs. It reads `BUILD_LINUX`, `BUILD_WIN`, `BUILD_WSL`, and
`BUILD_MACOS`. The default `--os all` runs on every declared OS except the
one the script is running on, since the local suite already covers it; the
banner names the skipped OS. An explicit `--os` runs that one OS even when it
matches the local one, errors when its host is undeclared, and `all` errors
when nothing but the local OS is declared rather than passing silently. The
`wsl` host runs CI's nextest archive mode with the builder target directory
hidden, so it catches the class of failure that only the `wsl2-ubuntu` CI
leg sees. By default every host builds the package's declared CI features
(`[package.metadata.ci.tests] features`, not `local-features`) and runs the L1
filter. A Cargo build flag (`--features`, `--all-features`,
`--no-default-features`) cannot be honored in archive mode, so passing one
switches *every* host to plain `cargo nextest run` with no tier filter and no
receipt; every other argument goes to the run.

Per-test results appear only as PASS lines in the live output. The run keeps
no JUnit report locally, and `--no-tests=pass` means a green leg does not prove
a `cfg`-gated test ran. To show that a Windows-only test compiled and ran, tee
the output (`./scripts/cross-check.sh <pkg> --os windows 2>&1 | tee <file>`)
and cite its PASS line.

The `just` recipe re-splits its arguments, so a filterset containing spaces
or parentheses (`-E 'binary(a) | binary(b)'`) dies with a shell syntax error
before any host is contacted. Call `./scripts/cross-check.sh` directly for a
filtered run; it passes the quoted filterset through intact.

Two runs from the same checkout share a clone. Each run holds that clone's
lock (`<clone dir>/.cross-check.lock`, an atomically created directory with an
`owner` file naming the user, branch, base SHA, and start time) for the whole
reset, checkout, and test sequence, so overlapping runs queue instead of
clobbering the clone. A waiter prints the owner and gives up after 30 minutes
with exit 75. A lock left by a dead run is reported, never removed by the
script; only its owner removes it by hand. Never work in the standing clone
directly; use your own worktree for ad hoc sessions.

- Verify the banner line `cross-check: <pkg> @ origin/<branch> (<sha>) + N
  changed file(s) as <rev>` before trusting a result. If in doubt, confirm the
  remote head: `ssh "$BUILD_WIN" "git -C <clone dir>\rusty-biscuit log
  -1 --oneline"` — it is `<rev>`.
- A clone left dirty by an earlier run used to make the checkout fail silently
  and test the old tree. The script resets and cleans before checking the
  tested revision out; recover an older clone by hand with `git reset --hard;
  git clean -fdq; git checkout --detach <sha>` over SSH.
- Positional filter args are nextest test-name substrings. A binary name
  matches nothing ("0 tests run"). `-E 'test(...)'` breaks the recipe's
  unquoted argument line (a `|` in it is run as a shell pipe); pass several
  name substrings instead, which nextest ORs.
- Child-process stderr is not shown by a remote test failure. A probe that
  must be read back can append to `<clone dir>\probe.txt` and be read
  with `ssh "$BUILD_WIN" "Get-Content <clone dir>\probe.txt"`.
- `BUILD_WIN` has **no usable `python3`**: the name resolves to a Cygwin shim
  pointing at a deleted `Python313\python.exe`, so a command that merely probes
  for `python3` finds one and then fails on use. A working 3.13 is reachable
  only as `py`. Anything that runs `scripts/ci/*.py` over SSH must spell `py`
  (measured 2026-09-15).
- **`tar` on `BUILD_WIN`'s `PATH` is Cygwin's too** (`/usr/bin/tar`), and it
  cannot open a `C:\...` path ("Cannot open: Input/output error"). To unpack an
  archive copied over with `scp`, call Windows' own
  `& "$env:SystemRoot\System32\tar.exe" -xzf <file> -C <dir>` (2026-09-24).
- **`bash` on `BUILD_WIN`'s `PATH` is Cygwin's** (`C:\cygwin64\bin\bash.exe`,
  `uname -s` = `CYGWIN_NT`, drives under `/cygdrive/b`), not Git Bash. To run a
  repo shell script the way a Git Bash user would, call
  `& "C:\Program Files\Git\bin\bash.exe" /b/<path>` (`MINGW64_NT`, `/b/`).
  `scripts/kache-host.sh qualify` gave the same verdicts under both
  (2026-09-23). `B:` is ReFS and `C:` NTFS, so the host covers both kache
  qualification outcomes; a scratch directory on `B:` plus a scratch
  `LOCALAPPDATA` keeps a probe off the real `B:\kache` and user cache.
- **A login shell reports its `~/.bash_logout`'s status, not the script's.**
  Diagnosed 2026-09-22, and the cause of the `wsl FAIL` with a
  `cross-check-exit: 0` marker recorded on 2026-09-15. The WSL guest's
  `~/.bash_logout` ends with `[ -x /usr/bin/clear_console ] &&
  /usr/bin/clear_console -q`, which fails without a tty, so `ssh <host> 'bash
  -l <script>'` returned 1 for a run whose every test passed. `bash -lc 'bash
  "$0"' <script>` does not do this and still sources the profile, so `cargo`
  stays on `PATH`; that is what `cross-check` now sends. Suspect this shape in
  any remote command that runs a script through a login shell, and check the
  host's `~/.bash_logout` before believing the exit status.
- The `wsl` leg still publishes no receipt for a run whose tree is not the
  outgoing head's, which is most runs; the printed reason says which.
  Separately, the archive run writes its JUnit report to
  `<clone>/target/nextest/ci/test-results.xml`, which is not where
  `publish_wsl_receipt` looks, and the same fresh `target` makes the restoring
  `mv target.hold target` nest the warm cache at `target/target.hold`, so the
  next WSL run rebuilds from cold. Both are `scripts/cross-check.sh`
  bookkeeping, not the package under test.
- **Native Windows, first working run 2026-09-22.** Two faults had been hidden
  behind the storage preflight, which always refused the leg before it got far
  enough: `cross-check` looked for `ci-build.exe` under
  `<clone>\scripts\target\release` (the `scripts` crate is a root-workspace
  member, so it builds into the workspace `target\`), and Git for Windows
  refused this repository's deepest paths (`Filename too long`) until the clone
  set `core.longpaths true` — `LongPathsEnabled` in the registry is not enough,
  since Git keeps its own 260-character cap.

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
- The standing clone's own target under `W:\ci-verification` is **also**
  outside the scheduled sweep: `RustyBiscuit-CargoSweep` is rooted at the
  `C:` checkout. On 2026-09-16 it held 161 GB, `W:` had 143 MB free, and
  `cross-check --os windows` died compiling with `os error 112` (not enough
  space). Running the scheduled task reclaimed nothing. `cross-check` has no
  sweep of its own, so report it rather than deleting the shared target.
  `W:` was freed by 2026-09-15. Later on 2026-09-16 a run failed *before*
  compiling: the patch upload reported `scp: write remote
  "W:/ci-verification/cross-check-….patch": Failure`, then git reported
  `unable to write file …` and `Could not reset index file`. That pattern
  points to write failures on `W:`, not to a patch bug. Check
  `Get-PSDrive W` before debugging the patch.
  On 2026-09-16 `W:` reported 0 GB free, and SSH to `$BUILD_WSL` failed at
  key exchange with `Connection reset by peer`. The guest's VHDX lives on
  `W:`, so treat a WSL reset as the same storage problem, not a network one.
- The standing cross-check clone `W:\ci-verification\rusty-biscuit` does not
  inherit that `target-dir` pin: it builds into its own `target\`, which the
  daily `RustyBiscuit-CargoSweep` (scoped to `W:/rusty-biscuit-target`) never
  touches. On 2026-09-17 that `target\` plus an orphan
  `W:\ci-verification\rb-pr66` (62 GB, 2026-08-30) and the 131 GB WSL VHDX
  left `W:` at 8 KB free while the sweep log reported success with
  `free_gib=0`. A full `W:` fails `--os windows` at the patch upload (`scp
  ... Failure`, `No space left on device`). The same day the WSL guest (its
  VHDX lives on `W:`) reset every SSH connection
  (`kex_exchange_identification: Connection reset`), so suspect a full `W:`
  first when both legs fail together. Freeing `W:` is the owner's call.

A stale lock on a standing clone does not block Linux evidence. Build a
private clone that only *reads* the standing one: `git clone --shared
--no-checkout <clone dir>/rusty-biscuit ~/scratch/<name>`. If the base
commit is missing there, send the gap as a `git bundle` (`<remote tip>..<base>`)
instead of fetching into the standing clone. Apply a
`git diff --cached --binary <base>` built with a temporary `GIT_INDEX_FILE`, so
untracked files come along and the local index is untouched. Run the recipes
through `bash -lc`, which is what puts `just` and `cargo-nextest` on `PATH`.
Delete `~/scratch/<name>` afterward; its `target/` is not swept.
  Measure over SSH with `-EncodedCommand` (UTF-16LE base64): a `$` in an
  inline PowerShell argument does not survive the remote shell.

## Compiler cache on the hosts

The standing `cross-check` clones are **never** built through kache, because CI never is.
Do not export `RUSTC_WRAPPER` in a session that touches them: in hardlink mode (build-linux is
ZFS without a working clone path; the WSL guest is ext4) a restored artifact is a read-only link
into the store, and the next unwrapped rebuild fails with "output file ... is not writeable"
(2026-09-09, 89 such files). Ruling per platform: `docs/kache-strategy.md`.

`unset RUSTC_WRAPPER` does **not** keep kache out; only an explicitly empty
`RUSTC_WRAPPER=""` does (measured 2026-09-21):

- `build-linux` has a `/usr/local/bin/cargo` shim ahead of the rustup proxy on
  `PATH`. It turns kache on for any compile-ish subcommand whose `target/` does
  not exist yet, which is every fresh private `~/scratch` clone, and leaves an
  already-set `RUSTC_WRAPPER` (even empty) alone. A capture into a new clone
  followed by a rebuild after a patch failed with the hardlink error above.
  Export `RUSTC_WRAPPER=""` (or `KACHE_AUTO=0`) before the first cargo command.
- The macOS dev host sets `rustc-wrapper = "kache"` in `~/.cargo/config.toml`
  and puts kache `cc`/`gcc`/`clang` shims (`~/.local/lib/kache/shims`) on
  `PATH`. A "kache off" timing needs `RUSTC_WRAPPER=""` and those shims removed
  from `PATH`. Otherwise the second of two "clean" builds restores the first
  one's dependencies (33.8 s against 114 s for the same package).

## Remote-process hygiene

- Stopping a local background task does not stop a backgrounded remote
  PowerShell pipeline; kill the parent `powershell.exe` PID or it starts the
  next cargo command.
- Run remote work through `just` gates where one exists rather than raw
  `cargo`, so timing records and preflights apply.
- When copying a single changed file to a standing clone for a quick check,
  `scp` it and then confirm `git status --short` on the remote shows only that
  file before running.
- Do not run a `--os windows` leg while a `--os wsl` leg is compiling. The two
  hosts are one physical machine, and on 2026-09-23 a first `claudine` build
  in a new Windows clone ran out of commit memory while the guest built the
  same package: `The paging file is too small for this operation to complete.
  (os error 1455)` on `libstd`/`libtest` rmeta. That clone's `target\` was
  left poisoned, and the retry failed with `E0463: can't find crate for
  claudine`. Run the legs one after the other, and discard (or let `cross-check`
  prune) a clone whose build hit 1455 before trusting its next result.
- In zsh, `"$BUILD_LINUX:scratch/x"` applies the `:s` history modifier and
  mangles the destination (`build-linuxap-…`). Brace the variable:
  `"${BUILD_LINUX}:scratch/x"`.
