# Build Hosts

Part of the `os` skill; [SKILL.md](SKILL.md) is the index.

Remote build hosts are declared only through environment variables, never
named in scripts or docs. A set variable means "that host is reachable from
here"; an unset one means "not from here". Run `env | grep '^BUILD_'` before
planning cross-OS evidence, and say which hosts were declared when you report.

| Variable | Provides | Notes |
|---|---|---|
| `BUILD_LINUX` | Native Linux | Has `tmux` and `wezterm`, but no `Xvfb`, `kitty`, or `xdotool`, so X11 Level 3 tests skip there. Run those in Docker instead ([macos.md](macos.md)). It has no `rsync` (copy a tree with `tar` over `ssh`) and no Chrome or Chromium, so a Linux browser reading runs in Docker too. Do not install packages on it. |
| `BUILD_WIN` | Native Windows, with PowerShell as the remote shell | Git's stderr appears as a red `NativeCommandError`; ignore it. Redirecting a native program with `*>` wraps each stderr line in a `RemoteException` record hard-wrapped at console width, so a long line splits; read structured output from a file the program writes. `scp` to a non-system drive spells the drive in the path: `"${BUILD_WIN}":/B:/coding/…`. |
| `BUILD_WSL` | A WSL2 Ubuntu guest | `cross-check` runs CI's archive mode here ([wsl.md](wsl.md)). For ad hoc commands, wrap them in `bash -lc`: non-login shells do not have `~/.cargo/bin` on `PATH`. |
| `BUILD_MACOS` | A macOS host other than this one | Same flow as Linux. |

Each value is an SSH destination (an `~/.ssh/config` alias or `user@host`).
Call it as `ssh -o BatchMode=yes "$BUILD_WSL" …` so a missing key fails fast
instead of prompting. OpenSSH 10 prints a `post-quantum key exchange` warning
on every connection; it is noise. `cross-check` passes `-o WarnWeakCrypto=no`
only when the local client supports that option, because older clients
refuse to connect with it. In zsh, `"$BUILD_LINUX:path"` applies the `:s`
history modifier and mangles the destination; write `"${BUILD_LINUX}:path"`.

## `just cross-check`

`just cross-check <package> [--os linux|windows|wsl|macos|all] [nextest args]`
(`scripts/cross-check.sh`) works like this:

1. It commits your working tree, including tracked edits and untracked
   non-ignored files, as one throwaway commit on top of `origin/<branch>`
   (or `origin/main` if the branch is unpushed).
2. It ships that commit as a `git bundle` and checks out a standing clone at
   it.
3. It runs the package's L1 suite with the package's declared CI `features`.

Your branch, index, and stash stay untouched, and nothing gets signed. That
commit is the plan's `head`, which archive mode requires: `ci-build` refuses
any checkout that is not `plan.head`, or whose tracked tree is dirty.

- **OS selection.** `--os` takes exactly one value; a comma list is rejected.
  `all` (the default) skips the local OS and names it in the banner. It
  errors if no other OS is declared. An explicit `--os` runs even when it
  matches the local OS, and errors if that host is undeclared.
- **Archive vs native mode.** The `wsl` leg runs CI's nextest archive mode,
  with the builder's target dir hidden. It catches failures that only the
  `wsl2-ubuntu` CI leg would see. Any Cargo build flag (`--features`,
  `--all-features`, `--no-default-features`) switches *every* host to plain
  `cargo nextest run`, with no tier filter, no receipt, and no published
  observation. In archive mode a target selector (`--test l1`, `--lib`) gets a
  usage error from `--archive-file` and the leg fails.
- **Filtering.** Positional args are nextest test-name substrings, and
  nextest ORs them together. A binary name matches nothing ("0 tests run").
  Arguments are re-split by a shell on each side. A filterset with spaces,
  `|`, or parentheses (`-E 'test(/a|b/)'`) breaks: `just` drops the quotes,
  and the Windows leg's recipes re-split it even when you call the script
  directly. Pass several substrings instead, for example
  `just cross-check claudine --os windows claudine_schemas composition::schema`.
  A filtered run is targeted evidence only.
- **Suites that run Cargo themselves** (trybuild compile-fail, fresh-output
  compiles) are `slow_`, which the L1 recipe's filter excludes, so an archive-mode run
  cannot select them; a crate outside the workspace has no archive at all. Run them in
  native mode with a build flag:
  `./scripts/cross-check.sh --os all schematic-runtime --all-features --test l1 -E 'test(/(^|::)slow_/)'`,
  or `… schematic-schema --all-features --manifest-path schematic/schema/Cargo.toml`
  for the generated crate. Schematic's fresh-output guard builds `--offline`
  against `schematic/schema/Cargo.lock`, so a host whose Cargo cache lacks a
  crate fails it with "failed to download … --offline was specified". That is
  a cache gap: `cargo fetch --manifest-path schematic/schema/Cargo.toml` on
  that host fills it, as CI's build job does.
- **Check what ran.** Before you trust a result, check the banner
  `cross-check: <pkg> @ origin/<branch> (<sha>) + N changed file(s) as <rev>`.
  `--no-tests=pass` means a green leg does not prove that a `cfg`-gated test
  ran, so look for its PASS line, or tee the output to a file.
- **Level 2 remotely.** Set `BISCUIT_TEST_REQUIRED_BACKENDS` in your shell
  (`cross-check` forwards it). Otherwise a backend the host lacks makes the
  test skip, and nextest still prints PASS. The Unix legs never read the
  remote `~/.config` (`GIT_CONFIG_GLOBAL=/dev/null`, empty
  `XDG_CONFIG_HOME`), so a down NAS cannot fail them.
- **Remote output.** A remote test failure does not show child-process
  stderr. If a probe must be read back, have it append to a file in the clone
  dir and `Get-Content` or `cat` that file over SSH.

### Publication

A leg publishes a `rig` observation for its own environment only
(`wsl2-ubuntu` or `windows-latest`), and only when all of these hold:

- the remote tested tree is exactly the outgoing `HEAD`'s tree;
- the remote worktree is clean;
- the run is unfiltered and in archive mode;
- the leg reported `cross-check-manifest-exit: 0`.

Any other run prints `cross-check: publishing no <environment> observation -
<reason>`. A completed failure is published as a failing observation. A
missing or malformed report, an empty expectation, or a run that never reached
its tests publishes nothing.

The leg leaves its staging dir (`manifest.jsonl`, `L1/<package>.xml`,
`expected-L1.json`) at `<clone dir>/cross-check-<run id>-reports`. The
orchestrating host fetches it and then deletes it remotely. `completion.py`
and the publisher (`scripts/ci/observation_notes.py publish-receipt --kind
rig`) run on the orchestrating host, because the Windows host has no usable
`python3`. Evidence lands under
`${BISCUIT_CI_EVIDENCE_DIR:-~/.rusty-biscuit/ci-evidence}/<head>/<environment>/`.

Exit statuses:

- **75** is `BUSY` (the lock is held) and **255** is `UNREACHABLE`; nothing
  ran in either case.
- An interrupt prints `cross-check: <os> interrupted; no observation
  recorded` and keeps the observations of earlier legs.

### Standing clones

Each host keeps one clone per originating checkout, at
`<coding dir>/<local host>--<worktree>/rusty-biscuit`:

- `<coding dir>` is the host's own `CODING_DIR`, or `~/coding` if that is
  unset. It is resolved over SSH and printed as `clone: <host>:<dir>`. **On
  Windows, keep `CODING_DIR` set to the large volume.** If it is unset, the
  clone falls back to the small `C:` system drive, which has filled and frozen
  the host before.
- `<local host>--<worktree>` is the machine running `cross-check` plus the
  worktree's directory name (`main` for the main checkout), lowercased. Runs
  of `-` in the host name collapse, so the first `--` is always the
  separator.

**Lock.** A run holds `<clone dir>/.cross-check.lock` for its whole
reset/checkout/test sequence. The lock is a directory with an `owner` file, so
two runs from the same checkout queue instead of clobbering each other. A
waiter gives up after 30 minutes with exit 75. A lock left by a dead run is
reported but never removed automatically; only its owner removes it. Never
work in a standing clone directly.

**Pruning.** Before a leg starts, each selected host retires every unlocked
clone that is either:

- **stale**: a `<this host>--*` clone whose worktree no longer exists here
  (absent from `git worktree list`, or marked `prunable`); or
- **expired**: any clone last used `CROSS_CHECK_MAX_AGE_DAYS` (default 3) or
  more days ago, judged by its `.git/FETCH_HEAD` time.

Retiring renames the clone to trash and deletes it in the background. On
Windows the delete starts through WMI, because OpenSSH kills session
children, and uses a `\\?\` path for deep trees. `just cross-check-refresh
[--os …] [--dry-run]` (`scripts/cross-check-hosts.sh`) applies the same rules
to every declared host, deletes synchronously, and reports sizes and the
space reclaimed. `just cross-check-test` runs the rules' tests.

**Never run `cross-check` from a scratch clone that is not a worktree of the
real repository.** The stale rule keeps only clones that this checkout's
`git worktree list` names. A scratch clone therefore deletes the warm clones
of every live worktree on that host. To rehearse, use a linked worktree, or
publish from fetched files with `observation_notes.py publish-receipt --kind
rig`.

**Working around a stale lock** (Linux). Make a private clone that only
*reads* the standing one: `git clone --shared --no-checkout <clone
dir>/rusty-biscuit ~/scratch/<name>`. Send any missing base commit as a
`git bundle` instead of fetching into the standing clone. Apply your changes
as a `git diff --cached --binary <base>` built with a temporary
`GIT_INDEX_FILE`, so untracked files come along. Run recipes through
`bash -lc`, and delete the scratch dir afterward, since nothing sweeps it.

## Host traps

- **A login shell returns its `~/.bash_logout` status, not the script's.**
  The WSL guest's `~/.bash_logout` runs `clear_console`, which fails without a
  TTY, so `ssh <host> 'bash -l <script>'` returns 1 after every test passed.
  `cross-check` sends `bash -lc 'bash "$0"' <script>` instead. Suspect this
  whenever a login-shell remote command reports a failure that the output
  contradicts.
- **A `wsl` leg times out on port 22 while `BUILD_WIN` answers.** The guest
  is stopped, not unreachable. `ssh "$BUILD_WIN" 'wsl.exe -l -v'` shows
  `Stopped`, and `ssh "$BUILD_WIN" 'wsl.exe -d <distro> -e true'` boots it.
  If the guest's sshd hangs while the distro is running, run commands through
  `wsl.exe -d <distro> -e <argv>` from the Windows side and strip NULs from
  the output.
- **`BUILD_WIN` and `BUILD_WSL` are usually one machine.** Do not run a
  `--os windows` leg while a `--os wsl` leg is compiling. Commit memory runs
  out (`The paging file is too small … (os error 1455)`), and the Windows
  clone's `target\` is left poisoned, so the retry fails with `E0463: can't
  find crate`. Run the legs one after another, and discard a clone whose build
  hit 1455.
- **Windows `PATH` tools are Cygwin's.**
  - `python3` is a broken shim; use `py` for anything that runs
    `scripts/ci/*.py`.
  - `tar` cannot open `C:\…` paths; use
    `& "$env:SystemRoot\System32\tar.exe"`.
  - `bash` is Cygwin's (`CYGWIN_NT`, `/cygdrive/b`). To run a script the way
    a Git Bash user would, call `& "C:\Program Files\Git\bin\bash.exe"`
    (`MINGW64_NT`).
  - `jq` (Cygwin's) and `just` both work, so `_expected_manifest` runs there.
- **The Windows host covers both kache qualification outcomes.** Its two
  volumes are ReFS and NTFS. A scratch dir on the ReFS volume plus a scratch
  `LOCALAPPDATA` keeps a `scripts/kache-host.sh qualify` probe away from the
  real store and user cache.
- **Git for Windows has its own 260-character path limit.** Setting
  `LongPathsEnabled` in the registry is not enough. The clone needs
  `core.longpaths true`, which `cross-check` sets.
- **Ad hoc PowerShell.** `scp` the script over and run it with `-File`.
  Piping it into `powershell -Command -` reads it line by line and silently
  drops multi-line blocks. The remote shell is PowerShell, so use `$env:TEMP`,
  not `%TEMP%`, and quote it for your local shell. When an inline argument
  needs a `$`, use `-EncodedCommand` (UTF-16LE base64).

  ```sh
  scp -q probe.ps1 "$BUILD_WIN":AppData/Local/Temp/probe.ps1
  ssh -o BatchMode=yes "$BUILD_WIN" 'powershell -NoProfile -ExecutionPolicy Bypass -File $env:TEMP\probe.ps1; Remove-Item $env:TEMP\probe.ps1'
  ```
- **Remote processes outlive local ones.** Stopping a local background task
  does not stop a backgrounded remote PowerShell pipeline. Kill the parent
  `powershell.exe`, or it starts the next cargo command.
- **Use the gates.** Run remote work through `just` recipes rather than raw
  `cargo`, so timing records and preflights apply. After you `scp` a single
  file into a clone, confirm that `git status --short` shows only that file.

## Storage

Disk-space crises are handled by the `storage-strategy` skill. What
`cross-check` users need to know:

- `just` recipes run `scripts/storage-preflight.sh`, which refuses to build
  below 50 GiB free (`BISCUIT_BUILD_MIN_FREE_GIB`). When it refuses, free
  space or use another host; do not work around it.
- **Never override `CARGO_TARGET_DIR` on the Windows host.** No Cargo config
  pins the target directory: each clone builds into its own `<clone>\target`,
  which sits on the `CODING_DIR` volume because the clone does. An override
  pointing anywhere else can fill the small system drive and freeze the host.
- No scheduled sweep covers standing clones or scratch checkouts.
  `cross-check-refresh` is how clones get retired. Delete your own scratch
  targets, on Windows and inside the WSL guest, before you report.
- **A full volume looks like a different bug.** On Windows it can show up as
  `scp … Failure`, `unable to write file`, `Could not reset index file`, or
  `os error 112`. The WSL guest's VHDX lives on the large volume, so SSH to
  `BUILD_WSL` failing with `Connection reset` at key exchange is the same
  problem. When both legs fail together, check free space first:
  `ssh "$BUILD_WIN" "Get-PSDrive"`, and `df -h ~` in the guest.

## Compiler cache on the hosts

Standing clones must never build through kache, matching CI. The Unix remote
prelude exports `RUSTC_WRAPPER=""` and `RUSTC_WORKSPACE_WRAPPER=""`. Exporting
them as empty is required, because unsetting is not enough: host config or a
`cargo` shim (for example one in `/usr/local/bin` that turns kache on) can
bring a wrapper back. The same applies to private scratch checkouts on hosts
that don't qualify for kache.

- **"Output not writeable" in a Linux archive build** means the target holds
  read-only, cache-linked artifacts: they share device/inode with kache store
  blobs, because reflink failed on that filesystem. To repair a clone:
  1. Take its `.cross-check.lock`.
  2. Confirm the links against the store by device/inode.
  3. Unlink those artifacts, or replace them with independent writable
     copies.

  **Never chmod a shared artifact**, because that changes the cache blob too.
  A feature flag is not a fix: it only switches to different coverage.
- **Timing a "kache off" build on the macOS dev host.** Its
  `~/.cargo/config.toml` sets `rustc-wrapper = "kache"`, and kache
  `cc`/`gcc`/`clang` shims (`~/.local/lib/kache/shims`) are on `PATH`. Set
  `RUSTC_WRAPPER=""` and remove the shims from `PATH`. Otherwise the second
  "clean" build restores the first one's dependencies.
