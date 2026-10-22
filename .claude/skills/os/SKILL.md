---
name: os
description: |-
  Non-obvious, repo-specific operating-system knowledge for rusty-biscuit:
  which hosts can produce macOS, Linux, native-Windows, and WSL2 evidence and
  how to reach them; how the WSL2 CI leg runs and how to reproduce it; Windows
  path, process, and console traps; Linux and macOS host traps; hosted runner
  differences and per-OS evidence reuse. Load before claiming an OS cannot be
  tested from here, before touching `#[cfg(windows)]` or path-comparison code,
  before tuning or comparing CI legs, and whenever a test is red on exactly
  one environment.
---

# OS Matters

Every package must compile and behave on macOS, Linux, native Windows, and
WSL2. This skill records what is *not* obvious about doing that from this
repository: where each OS can be exercised, the traps each OS sets, and how to
work with them. Generic Rust portability
guidance lives in `prompts/cross-platform.md`; the `wsl` skill covers
configuring WSL itself; the `rust-testing` skill owns test design.

## Where each OS can be exercised

Build hosts are declared by environment variables (`BUILD_LINUX`, `BUILD_WIN`,
`BUILD_WSL`, `BUILD_MACOS`), each an SSH destination. A set variable means the
host is available from this machine; unset means no destination is configured
for the runner. If the user says a rig is available, inspect the SSH config
for its documented OS role, verify connectivity, and supply the matching
`BUILD_*` value for that invocation. Never hardcode an
alias; check `env | grep '^BUILD_'` first and report which hosts you had.

| Need | Use | Detail |
|---|---|---|
| Real Linux run | `$BUILD_LINUX` via `just cross-check <pkg> --os linux`, or Docker Desktop on a macOS host | [build-hosts.md](build-hosts.md), [macos.md](macos.md) |
| Native Windows run | `$BUILD_WIN` via `just cross-check <pkg> --os windows` | [build-hosts.md](build-hosts.md) |
| WSL2 run, exactly as CI does it | `$BUILD_WSL` via `just cross-check <pkg> --os wsl` (nextest archive, builder target dir hidden) | [wsl.md](wsl.md) |
| Another macOS | `$BUILD_MACOS` (reserved; no recipe consumes it yet) | [build-hosts.md](build-hosts.md) |
| Windows compile evidence only | `cargo check --target x86_64-pc-windows-gnu` (never msvc from macOS); an isolated probe crate for `#[cfg(windows)]` code | [windows.md](windows.md#compile-evidence-from-macos) |
| Authoritative proof | Hosted CI (`ubuntu-latest`, `macos-latest`, `windows-latest`, `wsl2-ubuntu`) | [ci-runners.md](ci-runners.md), `.github/ci/README.md`, `.github/ci/environments.json` |

CI is the final proof, not the discovery loop. Surface an OS's exact failure
on the matching host first, then push once. Before pushing, read
[ci-evidence.md](ci-evidence.md) for which receipt satisfies which
environment's cell and how to record an execution ban.

## Red on one environment only: start here

| Symptom | Likely cause | Read |
|---|---|---|
| Hook fixture red only on Ubuntu | missing local git identity, inherited default branch, or `gh` already in `/usr/bin` | [linux.md](linux.md#hook-fixtures-red-only-on-ubuntu) |
| L2 timeout only on Ubuntu, green on the macOS host | depth-1 CI checkout meets git-history code | [linux.md](linux.md#shallow-checkouts-l2-red-only-on-ubuntu-a-timeout) |
| Linux and WSL red, macOS and Windows green, only an order differs | ext4 `read_dir` hash order | [linux.md](linux.md#read_dir-order-red-on-linux-and-wsl-green-on-macos-and-windows) |
| ASCII `>` where `▶` was expected, only on `BUILD_LINUX` | `LANG=C` in that host's shells | [linux.md](linux.md#langc-on-the-build_linux-host-ascii-where-a-glyph-was-expected) |
| A substring count in rendered output red on Linux | temp-path length changes where text wraps | [linux.md](linux.md#temp-path-length-changes-word-wrapping) |
| A test hangs to its timeout on Debian/Ubuntu/WSL2 only when run from a terminal, a `set -m` shell fixture | `/bin/sh` is dash, whose job control stops itself in a background process group | [linux.md](linux.md#binsh-is-dash-set--m-hangs-or-silently-does-nothing) |
| A `SIGINT`-then-grace stop of an `sh -c` command takes the whole grace period on Linux/WSL2 only | dash defers a PID-targeted `SIGINT` until its foreground child exits | [linux.md](linux.md#binsh-is-dash-set--m-hangs-or-silently-does-nothing) |
| A process count too high on Linux or WSL | `sysinfo` lists threads as processes | [linux.md](linux.md#sysinfo-lists-threads-as-processes) |
| Red only on `wsl2-ubuntu` | the guest runs an archive built on `ubuntu-latest`; builder paths baked in at compile time (`env!("CARGO_BIN_EXE_*")`, `CARGO_MANIFEST_DIR` fixtures, toolchain lookups; fix binaries with `biscuit_test_harness::bin_exe!`) | [wsl.md](wsl.md) |
| Red only in an archive run (WSL2, or `just cross-check --os windows`), "could not resolve `../../…`" | an extracted archive has no `.git`, so `build_resolution_context` makes the request directory the tree root and a snapshot built at a package refuses a fixture's `..` climb; anchor it at the run-time repository root (`manifest_dir!()`'s ancestor), never `env!("CARGO_MANIFEST_DIR")` | [wsl.md](wsl.md) |
| WSL guest red at provisioning with a 403 | anonymous GitHub API rate limit | [wsl.md](wsl.md) |
| WSL guest "lost communication with the server", killed with no log | the runner agent died during provisioning | [wsl.md](wsl.md) |
| Red only on `windows-latest`, a path in the message | 8.3 short-name TEMP (`RUNNER~1`) or verbatim `\\?\` spelling | [windows-paths.md](windows-paths.md) |
| `wt remove` exits 3 on Windows naming `SearchProtocolHost.exe` | Windows Search held a file through the usage check's one 500 ms recheck (holds measured at ≤ 17 ms), or the first check also found another holder | [windows.md](windows.md#transient-system-holders-and-wt-remove) |
| Red only on `windows-latest`, elapsed time equals some child's timeout | handle inheritance keeps pipes open | [windows.md](windows.md#environment-and-processes) |
| Windows cell red although every test passed, `UnicodeEncodeError` in the last step | Python's piped stdout is cp1252; workflows must set `PYTHONUTF8=1` | [windows.md](windows.md#environment-and-processes) |
| Red on Windows only when the agent ran it, a git child exiting `0xC0000005` with empty stderr | the run descended from Git Bash (the agent's Bash tool); rerun from PowerShell | [windows.md](windows.md#environment-and-processes) |
| Python-gated test passes almost instantly on BUILD_WIN over SSH, nothing ran | `python3`/`python` resolve to WindowsApps aliases; prepend the real interpreter on a process-scoped `PATH` | [windows.md](windows.md#environment-and-processes) |
| Red only on Windows, "still checking" or another wait expiring, many small git commands in `GIT_TRACE` | every git process is expensive on Windows; read metadata in-process instead of raising the wait | [windows.md](windows.md#environment-and-processes) |
| A Windows process trace shows `git --exec-path` and a `conhost.exe` once per process that uses `gix`; a test 2–3× slower in the full `build-win-native` run than alone | gix-path asks Git for its install location unless `EXEPATH` is set; Defender takes about a quarter of the CPU in a full run there (CI disables it) | [windows.md](windows.md#environment-and-processes) |
| Red only on `windows-latest` with an empty failure message | a std handle redirected to `CONOUT$` | [windows-console.md](windows-console.md#attaching-a-console-inside-a-nextest-process) |
| Kitty or Sixel output vanishes only when the Windows program runs over SSH; iTerm2 images and OSC 8 links arrive | the OpenSSH session's system conhost removes APC and DCS and passes OSC | [windows-console.md](windows-console.md#escape-sequences-through-the-windows-openssh-server) |
| macOS watcher receives events but its target is absent from `lsof` | FSEvents directory watches do not require an open descriptor naming the target | [macos.md](macos.md#finding-filesystem-watchers) |
| `ErrorKind::NotFound` check misses on Windows only; message shows `os error -2147024894` | a `windows` crate error converted with `io::Error::from` keeps the HRESULT | [windows.md](windows.md#environment-and-processes) |
| A per-handle file query hangs only on Windows; `CancelSynchronousIo` returns 1168 | the handle's owner has synchronous I/O pending on that file object | [windows.md](windows.md#inspecting-another-processs-handles) |
| A test aborts with `0xc0000409`, empty stderr, only on native Windows and only under parallel load | aws-lc's CPU-jitter RNG seeding calls `abort()` on its first random bytes (any aws-lc user, so every reqwest/rustls TLS test); not reqwest's TLS path | [windows.md](windows.md#aws-lc-aborting-the-process-on-its-first-random-bytes-under-load) |
| A Windows process-module scan takes most of a second, or a hard-linked DLL is missed | `GetModuleFileNameExW` walks the loader list per module; the kernel's mapped name names the first-mapped hard link | [windows.md](windows.md#reading-another-processs-loaded-modules) |
| Need "which processes hold this path" on Windows without the per-handle hang | ask from the file side with `FileProcessIdsUsingFileInformation` | [windows.md](windows.md#asking-which-processes-have-a-path-open) |
| A directory rename fails with `ERROR_ACCESS_DENIED` on Windows only, and permissions look fine | something holds a file, subdirectory, or cwd inside it; the code is the same as an ACL denial | [windows.md](windows.md#current-directory-locks) |
| A directory rename fails with 5 now and then on Windows while a watcher runs, and the file-side holder query finds nobody | the watcher itself opened (even only stat'ed) an entry inside the tree; take entry types from the notification | [windows.md](windows.md#directory-change-notifications) |
| A Windows watcher test passes on `C:` but misses a directory removal or sees a rename as remove-plus-modify on `B:` | ReFS reports neither a POSIX-semantics directory delete nor a replacing rename the way NTFS does; temp is NTFS on `build-win-native` | [windows.md](windows.md#directory-change-notifications) |
| A file watcher misses events, or loses them silently, after a rename or a burst | a path-tracking wrapper drops a renamed inotify watch that the kernel keeps; Windows overflow is silent; FSEvents drops bursts, reorders, needs a marker plus flush to drain, and hides child events behind a directory's rename | [linux.md](linux.md#inotify-watches-follow-inodes-new-directories-are-not-watched), [windows.md](windows.md#directory-change-notifications), [macos.md](macos.md#receiving-fsevents-reliably) |
| macOS FSEvents delivery takes seconds (`wt remove` stalls with `delivery_stalled`) although the host looks idle | a long-running fseventsd pinned on one efficiency core; restarting it is manual and system-wide | [macos.md](macos.md#a-stuck-fseventsd-delays-every-stream) |
| `renameat2(RENAME_NOREPLACE)` gives `EINVAL` under WSL | drvfs (`/mnt/<drive>`) does not support the flag | [wsl.md](wsl.md#drvfs-volumes-reject-rename_noreplace-inotify-instances-are-scarce) |
| `/proc` fd scan finds nothing for a process you know holds files | `procfs` skips per-fd `EACCES`, or `sdev` was compared without decoding | [linux.md](linux.md#reading-proc-for-another-processs-descriptors) |
| A test or loop that flushes files is 10–100× slower on macOS than on Linux | std's `sync_all`/`sync_data` issue `F_FULLFSYNC` on Apple platforms (~25 ms each) | [macos.md](macos.md#file-flushes-cost-25-ms-each) |
| Green gates that did not test your worktree | Bash `cd <area>` followed `CDPATH` into the main checkout | [macos.md](macos.md) |
| WezTerm L2 tests time out at spawn or capture, or report `WezTerm not available`, a different set each run | too many concurrent `wezterm cli` clients for the GUI's main thread; the harness lets two run at a time | [macos.md](macos.md) |
| macOS host L2 red with a shell prompt in the captured frame | a host shell-startup prompt swallowed the input; not a repo defect | [macos.md](macos.md) |
| Slow on one leg only, or a timing delta under 15% | runner size and per-leg profile; noise is 5–15% per leg | [ci-runners.md](ci-runners.md) |

## Standing rules

- **CI cadence:** ordinary PRs and main pushes cover Linux/macOS; the
  scoped nightly covers native Windows and WSL2. A Windows failure surfaces
  in the nightly, after merge, and is fixed forward. `just ci-seed` proves
  the nightly's cells on the rigs at a pinned `main` checkout, resumably; the
  nightly reuses that proof per cell. Read `docs/topics/ci-cd.md`; WSL2 is
  never native Windows evidence.
- The Windows and WSL2 Level 2 CI cells are a **temporary policy gap**
  (`.github/ci/environments.json`, owner and expiry recorded there). Do not
  amend an acceptance criterion to exclude them; record the criterion as unmet
  with provisioning as the required change. When a gap is lifted, `just
  ci-seed` still proves Level 1 only: it reports those cells `unsupported`
  and the nightly runs them on hosted runners.
- A cross-compile is compile evidence, never behavioral evidence. Say which
  one you have.
- CI legs are compared within one environment only, on matched test
  identities, with three consecutive green runs as the evidence standard.
  WSL2 follows Linux code paths and is never evidence for native Windows
  ([ci-runners.md](ci-runners.md)).
- `ctx.*` path values and Markdown presentation are portable (`/`); an eager
  `file()`'s effective frontmatter value keeps native spelling. Tests compare
  against the spelling the surface actually emits (`biscuit_file`'s
  `to_portable_string` for portable surfaces).
- `#[cfg(unix)]` and `#[cfg(windows)]` test the **target**, not the build
  host. A `#![cfg(unix)]` inside a test file does not stop Cargo building that
  test's dev-dependencies for a Windows target; gate the dependency graph.
- Never override `CARGO_TARGET_DIR` on the `BUILD_WIN` host: each clone's
  `target` already sits on the large `CODING_DIR` volume, and an override can
  fill the system drive ([build-hosts.md](build-hosts.md)).

## Files

| File | Covers |
|---|---|
| [build-hosts.md](build-hosts.md) | The `BUILD_*` contract, standing clones, storage rules, `just cross-check` usage and gotchas, compiler cache, remote-process hygiene |
| [ci-evidence.md](ci-evidence.md) | Per-cell evidence reuse across environments and commits, execution bans, `scope-only`, what the pre-push hook reviews |
| [ci-runners.md](ci-runners.md) | Hosted runner sizes (macOS is the tightest), per-leg build and execution profiles, the anonymous API limit, cache quota, `main` cancellation, merge-gate bypass, cross-run noise and comparison rules |
| [linux.md](linux.md) | Ubuntu hook fixtures, shallow checkouts, `read_dir` order, `LANG=C`, wrap-dependent counts, `sysinfo` threads |
| [wsl.md](wsl.md) | The archive-mode contract, faithful reproduction on `BUILD_WSL`, provisioning failures (the 403 rate limit, a lost runner), the guest's packages, Level 2 on WSL, the guest's network-share `~/.config`, which red step means what |
| [windows.md](windows.md) | Environment and processes (handle inheritance, Job Objects, stack size, killing `git`, `python3` alias, batch arguments, Ctrl+C), current-directory locks, the `windows-latest` leg, compile evidence from macOS |
| [windows-paths.md](windows-paths.md) | Verbatim `\\?\` paths, home and cache known folders, 8.3 short names, separators, `file://` URIs, Markdown escapes, junctions and reparse points |
| [windows-console.md](windows-console.md) | WezTerm on `build-win`, PowerShell output streams and encoding, a windowless ConPTY console for L2, attaching a console under nextest, which escape sequences survive the Windows OpenSSH server's conhost |
| [macos.md](macos.md) | `/var` symlink, Docker for Linux evidence, L2 capture wedges, terminal-detection forks, perf triage, lldb work counters, shell-init and `cd` traps, the kache `DYLD_*` link failure |

When you learn a new OS-specific fact the hard way, add it to the matching
file in the same change that fixes it, and add a row to the symptom table
when the symptom points somewhere non-obvious. Keep this file a router: facts
belong on topic pages.
