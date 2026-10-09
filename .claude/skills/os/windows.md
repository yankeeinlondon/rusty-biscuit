# Windows

Native Windows is the environment most often broken by code that passed on
macOS and Linux. Each trap below has bitten a real PR; the fix or the shared
helper that resolves it is named so it is not re-derived.

This page covers processes, the environment, current-directory locks, the
`windows-latest` leg, and compile evidence from macOS. Two companion pages
hold the rest:

- [windows-paths.md](windows-paths.md): verbatim `\\?\` paths, 8.3 short
  names, the home and cache known folders, separators, `file://` URIs,
  Markdown escapes, junctions and reparse points.
- [windows-console.md](windows-console.md): WezTerm on `build-win`,
  PowerShell output and encoding, a windowless ConPTY console for Level 2,
  and attaching a console inside nextest.

## Environment and processes

- **Environment variable names are case-insensitive.** Iterating
  `env::vars()` and matching an exact spelling misses `Path`/`PATH`. Compare
  ASCII-case-insensitively when iterating; `env::var_os("PATH")` is fine.
- **Handle inheritance blocks `.output()`.** Rust std spawns with inheritable
  handles. A child that spawns a detached grandchild with `Stdio::null()`
  still passes the *parent's* pipe write ends along, and the parent's
  `.output()` waits for every holder to close. Symptom: Windows-only failure
  whose elapsed time equals some child's timeout while the exit status is
  success. Fix: `sniff::process::configure_detached_child` clears
  `HANDLE_FLAG_INHERIT` on the current process's stdio before setting the
  detached creation flags (`windows` crate features `Win32_Foundation` and
  `Win32_System_Console`). Unix never sees this; fds are close-on-exec.
- **Python's piped stdout is cp1252 on a Windows runner.** A script that
  prints non-ASCII (a check mark, a box-drawing character) raises
  `UnicodeEncodeError` the moment its output is piped, which every CI step
  is. `PYTHONUTF8=1` fixes it; `_package-ci.yml` and `_wsl-ci.yml` set it at
  workflow level, and a contract test keeps it there. Found 2026-10-06:
  `completion.py`'s check mark failed every Windows L1 cell whose tests had
  all passed, so Windows could not go green on `main`. Read the job's last
  step before blaming a runner: these failures were first mistaken for the
  unrelated "bash startup failure" text that `install-action` prints.
- **A git process costs ~47 ms on Windows, ~5 ms on macOS.** Process start
  dominates any code that runs many small git commands, and git for Windows
  also routes a local-path transport through MSYS `sh.exe`. Symptom: a
  Windows-only timeout where `GIT_TRACE` shows the same metadata command
  (`remote get-url`, `symbolic-ref`, `config --get`) run repeatedly. Do not
  raise the budget; read the metadata in-process (`worktree`'s
  `git_metadata`, `sniff`'s `gix` use) and keep git for transport. Measure
  with `GIT_TRACE=<file>` on the child, which timestamps every process. Found
  2026-10-05: `wt list`'s refresh worker started 14 git processes per fetch
  and missed its 3 s wait on `windows-latest`.
- **A refused loopback connection is slow.** Connecting to a closed
  `127.0.0.1` port fails in milliseconds on macOS and Linux, but native
  Windows retries the SYN and reports the refusal after about 2 s. A test
  that expects a refused request to fail *inside* some deadline (for example
  `wt list`'s 3 s wait over `ProxyStub::refusing`) passes alone and fails
  under parallel load there. Assert only what holds either way, or accept and
  close connections instead (`ProxyStub::closing_after`). Found 2026-09-27 by
  `list_prs::a_changed_origin_hides_the_stored_badges_and_its_worker_stores_nothing`.
- **`python3` is an App Execution Alias, not an interpreter.** Windows ships a
  stub at `python3.exe` that *spawns successfully* and then exits non-zero with
  "Python was not found; run without arguments to install from the Microsoft
  Store". `Command::new("python3").output()` returning `Ok` is therefore not
  evidence an interpreter exists, and every "skip where python3 is missing"
  guard written against `Err` fails on Windows instead of skipping. Probe
  `--version` and require `status.success()`, and try `python` as well — the
  hosted `windows-latest` image installs the real interpreter under that name.
  Found 2026-09-14 by the first native-Windows run of `repo-deps`
  (`scripts/ci-rollup-tests.rs::python_interpreter`).
- **BUILD_WIN SSH sessions resolve `python3`/`python` to the WindowsApps
  aliases ahead of the real interpreter.** `--version` fails, so tests gated on
  `python_interpreter()` skip and nextest reports PASS in ~0.3 s having run
  nothing. Before a `just cross-check` run that needs Python, prepend the real
  interpreter's directory to a process-scoped `PATH` (confirm a successful
  `--version`; do not assume the install directory, e.g. Python313). A PASS
  alone is not evidence: read the captured output for the planner's own lines.
- **Run Windows tests from PowerShell, not from Git Bash.** An agent's Bash
  tool on Windows is Git Bash (MSYS). Under it, every `pr_flow_rehearsal` route
  whose stage runs a local-transport `git push` failed: git exited
  `0xC0000005` (`STATUS_ACCESS_VIOLATION`) with empty stderr, and the unmodified
  code failed the same way. The same tests pass from PowerShell. They still
  fail from a `powershell.exe` launched *by* Git Bash, and they still pass from
  PowerShell given Git Bash's extra variables or its `PATH` order. So the
  trigger is an MSYS ancestor, not an environment value. Reproduce a Windows
  failure from the PowerShell tool before diagnosing it. Found 2026-10-08.
- **A Cygwin install on the machine `PATH` takes over `just` recipes.** On the
  `B:` dev host `C:\cygwin64\bin` is a machine entry, which Windows places ahead
  of every user entry, so `bash`, `env`, `mktemp`, and `python3` resolve to
  Cygwin and Git's `usr\bin` is absent. A `#!/usr/bin/env bash` recipe then
  runs under Cygwin bash. Its `mktemp` yields `/tmp/…` or `/cygdrive/c/…`,
  which native Python opens as `B:\tmp\…`, and its `python3` is a wrapper that
  fails under any other shell. `just ci-local --plan` and the pre-push hook fail
  with `FileNotFoundError` on their own temp files. With Git's `usr\bin` first
  but Cygwin still present, the same recipe's `git` children segfaulted
  (exit 139). With Cygwin removed, everything resolved to native or MSYS tools
  and `git` ran cleanly. Before running a hook or `ci-local` there, use a
  process-scoped `PATH` that drops every `cygwin` entry and prepends
  `C:\Program Files\Git\usr\bin` and the real Python directory. Found
  2026-10-08 while pushing from that host.
- **A `cmd /C` line cannot start a command with `@1>&2`.** `for /L %i in
  (…) do @1>&2 echo x` runs a command named `1`, which fails. Write
  `@(1>&2 echo x)`. The group also avoids the trailing space that
  `echo x 1>&2` puts on each line. Found 2026-10-08 in claudine's
  `semantic_stderr_buffer_is_drained_before_run_closes`.
- **A `.cmd`/`.bat` cannot receive an argument containing a newline** ("batch
  file arguments are invalid"). A fake provider that receives a multi-line
  prompt must be a compiled `.exe`; see the rustc-built fixture in claudine's
  `inline_compose_hash.rs`. This is not only a test concern: Claudine delivers
  the composed prompt as one `-t` argument, so a real npm-installed
  `opencode.cmd` on a user's `PATH` failed `just commit` the same way.
  `claudine::child_environment::command` now starts an npm `cmd-shim`'s
  target directly (`claudine/lib/src/npm_shim.rs`); a batch file in any other
  shape still fails. The `wrap_compose_validation` stub was still a `.cmd`
  until 2026-09-22, and the first native-Windows run of `claudine-cli` is
  what found it.
- **An SSH session's processes are already inside a Job Object**, and that Job
  forbids nesting, so `AssignProcessToJobObject` returns
  `ERROR_ACCESS_DENIED` (`Access is denied. (0x80070005)`, the `windows`
  crate's HRESULT spelling, not `os error 5`). Anything that treats that call
  as fatal cannot run under SSH at all: it failed every `sequence_budget`
  launch until `windows_wait_loop` was made to degrade to terminating the
  child alone (2026-09-22). Verify a host with `IsProcessInJob` before
  blaming the code. Tests that spawn through a Job therefore behave
  differently over `just cross-check` than in an interactive session.
- **`io::Error::from(windows::core::Error)` keeps the HRESULT as the raw OS
  error**, so `e.kind()` is never `NotFound` for a missing file: it reports
  `os error -2147024894` (`0x80070002`) with an uncategorized kind. Code that
  branches on `ErrorKind` must unwrap a `FACILITY_WIN32` HRESULT
  (`code & 0xFFFF0000 == 0x80070000`) to `from_raw_os_error(code & 0xFFFF)`
  first; see `io_error` in `sniff/lib/src/filesystem/query/identity.rs`. It
  passed every macOS and Linux run and was found by the first native-Windows
  run of the query's root recheck (2026-10-05), which reported a deleted
  root as "unverified" instead of "changed".
- **Overriding `USERPROFILE` alone breaks the per-user known folders.**
  `dirs::data_local_dir()` / `data_dir()` go through `SHGetKnownFolderPath`,
  which resolves `LocalAppData`/`RoamingAppData` *beneath `USERPROFILE`* and
  verifies the directory exists; `LOCALAPPDATA`/`APPDATA` are not consulted.
  A fixture that points `USERPROFILE` at a bare temp home therefore gets
  `None` from `dirs` (zed-dmls: "unable to determine the required per-user
  directory") while the same binary works under the host profile. Fix: create
  `home\AppData\Local` and `home\AppData\Roaming` under the fixture home.
  Measured on build-win-native, 2026-09-10; `dirs::home_dir()` itself
  (`FOLDERID_Profile`) is fine with a bare directory.
- **Open handles block delete and rename.** A `File`, temp dir, mmap, or
  child that still holds a handle makes cleanup assertions fail on Windows
  only. Drop before asserting. Even after owned handles are closed, Playa's
  detached-journal atomic publication can transiently receive
  `ERROR_ACCESS_DENIED` or `ERROR_SHARING_VIOLATION` from a concurrent
  reader or host scanner during `MoveFileExW(REPLACE_EXISTING)`. Retry those
  two errors for a short bounded interval while preserving atomic replacement;
  never delete the destination first.
- **`std::fs::rename` and `tempfile::persist` differ against open readers.**
  Rust std opens files with `FILE_SHARE_DELETE`, and `std::fs::rename` over a
  destination held by such a handle succeeds (the holder keeps the old bytes).
  `tempfile::NamedTempFile::persist` over the same holder fails with error 5.
  A holder opened *without* delete sharing (CRT `_wopen`, editors, scanners)
  blocks both with error 5; error 32 was not observed. Renaming a directory
  that contains an open file also fails with error 5. Prefer `std::fs::rename`
  plus the bounded retry above for replace-in-place. Measured on
  build-win-native (NTFS), 2026-09-17; see
  `messenger/features/2026-09-17-research-metadata-pipeline/spikes/publication/findings.md`.
- **Claudine's Ctrl+C on Windows goes through one process-wide
  `SetConsoleCtrlHandler` handler** that accepts `CTRL_C_EVENT` and
  `CTRL_BREAK_EVENT`, and its force-exit rung exits `130` through
  `ExitProcess`. A test can deliver a press either way: `CTRL_BREAK_EVENT`
  to a `CREATE_NEW_PROCESS_GROUP` child, or ETX typed into a pseudoconsole
  (see the ConPTY section of [windows-console.md](windows-console.md) for the
  inherited-ignore trap). See the
  claudine skill's `signal-handling.md`, "Windows parity".
- **The main thread gets a 1 MiB stack, not 8 MiB.** Symptom: a test's
  spawned binary dies with `code=-1073741571` (`0xC00000FD`,
  `STATUS_STACK_OVERFLOW`) and empty stderr on Windows only. Debug-build frames
  are large (claudine's harness loop frames run to 50–100 KiB each), so a
  finite call chain that fits Unix's 8 MiB can overflow here. Reproduce on
  macOS with `(ulimit -s 768; <binary> …)` and size the frames with an lldb
  script over `frame.GetSP()` deltas. The fix is the binary's `build.rs`
  reserving 8 MiB (`/STACK:8388608` for msvc, `-Wl,--stack,8388608` for gnu,
  via `cargo:rustc-link-arg-bin=<bin>=…`), as `darkmatter/cli`,
  `claudine/cli`, and `sniff/cli` do. It covers the main thread only; spawned
  threads keep Rust's 2 MiB default.
- **Killing `git` at a deadline leaves the real git running.** `git.exe` on
  `PATH` is the `Git\cmd\git.exe` launcher, which starts
  `mingw64\bin\git.exe`; that in turn runs the transport (`git-remote-https`,
  `ssh`). `Child::kill` ends only the launcher. The orphans keep the stderr
  pipe open (a reader waiting for EOF blocks about 20 s) and hold their working
  directory, so a git started inside a directory the tool then deletes locks
  it. `taskkill /T /F /PID <pid>` released everything at the deadline, under
  SSH too. Unix has the pipe half of this (the transport grandchild holds
  stderr) and is fixed by `process_group(0)` plus a negative-PID `SIGKILL`.
  Measured on build-win-native, 2026-09-24
  (`worktree/fixes/2026-09-24-ux-improvements/spike-s2.md`).

## Current-directory locks

Measured on `build-win-native` on 2026-09-24 (Windows PowerShell 5.1,
git 2.55.0.windows.3) while designing `wt remove` for the worktree the shell
is standing in (`worktree/fixes/2026-09-24-ux-improvements`).

- **A process's current directory cannot be deleted, moved, or renamed**
  ("The process cannot access the file because it is being used by another
  process"). Unix has no such lock, so code that removes a directory the user
  may be standing in is untested until it runs here.
- **Which shells actually hold the lock:**

  | Shell state | Locks the directory? |
  |---|---|
  | Any process *launched* with the directory as its working directory (a terminal tab opened there, `Start-Process -WorkingDirectory`) | Yes |
  | cmd.exe after `cd /d` | Yes |
  | Windows PowerShell after `Set-Location`/`cd` | No: `Set-Location` does not change the process's Win32 current directory |
  | Git Bash (MSYS) after `cd` | No |
  | A `FileSystemWatcher` on it (Explorer, editors) | No |

- **Child processes start in PowerShell's *location*, not its Win32 current
  directory.** A `wt.exe` or `git` launched from a PowerShell that has
  `cd`'d into a directory holds that directory itself, so a tool that deletes
  it must first move its own current directory out
  (`std::env::set_current_dir`) and address the repository with `git -C`.
- **Releasing a PowerShell lock takes both moves:** `Set-Location` *and*
  `[Environment]::CurrentDirectory = …`. Moving only the location left a
  window that was launched inside the directory still holding it.
- **`git worktree remove` fails late, not cleanly.** With the directory held,
  it exits 255 with "failed to delete '…': Permission denied" *after* deleting
  every file, removing `.git/worktrees/<name>`, and dropping the worktree from
  `git worktree list`. Only the empty directory remains, and a following
  `git branch -D` succeeds. Check for the lock before calling it.
- **Detecting the lock without side effects:** rename the directory to a
  sibling name and straight back (`std::fs::rename` twice). It fails exactly
  for current-directory holders and open files, and leaves an unlocked
  directory untouched. Cost on ReFS with 3,000 files inside: about 1.5 ms per
  rename pair; a held directory fails on the *first* rename in about 2 ms, so
  the rename back only fails if something takes the lock in between
  (2026-09-24, `spike-s4.md` in the same fix directory).
- **A test that needs a lock holder must spawn the holder directly.**
  `Command::new("cmd").args(["/C", "ping -n 30 …"])` followed by `kill()`
  kills `cmd` only. `ping` keeps running, holds the directory, and keeps
  the inherited output pipe open, so nextest waits the full 30 s. Spawn
  `ping` itself with `current_dir` set and null stdio
  (`worktree/lib/src/remove/mod.rs`, `worktree/cli/tests/powershell_wrapper_exec.rs`,
  2026-09-24).

## The `windows-latest` leg

- **Malformed bytes with an audio extension still reach host playback.**
  Playa's format detector accepts a recognized extension when header detection
  fails; native decode failure then falls back to installed host players. The
  detached CLI journal test's invalid `.wav` therefore depended on the Windows
  runner's players; a CI journal timeout exposed this mismatch. Worker-failure fixtures
  use an unrecognized extension and assert source rejection before any backend
  is reached; deterministic library tests cover playback-error classification.

Slowest build phase of the four CI legs, fewer test identities because
`#![cfg(unix)]` binaries are excluded (declare them as platform exclusions,
not lost coverage), and Sniff runs its L1 group one process at a time there
because overlapping host-network detections fail-fast the test process
(`.config/nextest.toml`). Sizes, timings, and the Level 2 gap are in
[ci-runners.md](ci-runners.md).

## Compile evidence from macOS

- Use `x86_64-pc-windows-gnu`. `x86_64-pc-windows-msvc` dies inside
  `aws-lc-sys` (`'windows.h' file not found`) because it needs the Windows
  SDK, and `aws-lc-sys` is non-optional through reqwest → rustls for most of
  the workspace. Re-add the target with `rustup target add
  x86_64-pc-windows-gnu` after a toolchain change; it does not always survive.
  The same target also dies in `blake3` with `cc-rs: failed to find tool
  "ml64.exe"`: its SIMD assembly needs the MSVC assembler, which a macOS host
  has no more of than it has `windows.h`. Anything that enables
  `biscuit-hash`'s `blake3` feature inherits that — `repo-deps` does, for
  `ci-build`'s archive checksums. Check such a `#[cfg(windows)]` block with the
  throwaway-probe technique below.
- Crates that reach `duckdb-sys` (rendezvous-daemon and anything with it as a
  dev-dependency) overflow mingw's COFF section limit. The claudine area's
  `just check-windows` recipe sets `-Wa,-mbig-obj` and a separate target dir;
  claudine-cli gates the daemon dev-dependency on the ABI
  (`cfg(not(all(windows, target_env = "gnu")))`), which is why its `--tests`
  check passes.
- Two host traps: a `rustc-wrapper = "kache"` setting leaks into `cc-rs` and
  breaks every C build for the cross target (`RUSTC_WRAPPER=""`), and kache's
  read-only cached artifacts need their own `CARGO_TARGET_DIR`. Repository
  ruling (2026-09-09, `docs/kache-strategy.md`): kache is off on Windows and
  WSL.
- For a `#[cfg(windows)]` file that cannot be reached through the workspace
  graph, copy it into a throwaway probe crate with `windows` and `tokio`,
  stub the seams, and `cargo check --target x86_64-pc-windows-gnu` there. It
  validates Win32 signatures in seconds and has caught real defects (a
  `drop()` of a `Copy` return value, a deleted function). It proves
  signatures, not runtime behavior.

- **Clippy can be red only on Windows.** `PathBuf`/`OsString` is 32 bytes on
  Windows and 24 on Unix, so an error enum holding several paths (or an error
  that does, like `biscuit_file::GlobReferenceError`) can cross
  `result_large_err`'s 128-byte limit on Windows alone. One such variant failed
  every `Result` in Darkmatter that carried the enum (190 errors at 136 bytes).
  Fix it by sharing the large cause (`Arc`/`Box`) in that variant, not by
  raising the limit. If the error is classified by downcasting its `source()`
  chain, the downcast has to name the wrapper type. The workspace
  `clippy.toml` threshold (136) was measured on Unix. `claudine::CompositionError`
  is 144 bytes on Windows, with ten variants over the limit, so the claudine
  library allows the lint under `cfg(windows)` instead of changing a public
  error shape; Linux lint runs still enforce the threshold. Lints that depend
  on `cfg` code also show up only here, and only once the crates beneath them
  compile: `dead_code` and `unused_mut` on items only `#[cfg(unix)]` code uses,
  and `needless_return` on a `return` inside a `#[cfg(windows)]` block that
  becomes the function's tail once its `#[cfg(not(windows))]` sibling is
  compiled out.

A cross-compile is compile evidence. Behavioral evidence comes from
`just cross-check <pkg> --os windows` or the `windows-latest` CI leg.

## Inspecting another process's handles

Measured 2026-10-05 on `BUILD_WIN` for Sniff's path-usage query. The SSH
session there is **elevated** (High integrity, SeDebugPrivilege), so
handle/permission counts taken over SSH overstate what a normal user sees.
Simulate Medium integrity with a filtered token, or use a desktop session.

- **Identity queries on a duplicated *disk* handle can block indefinitely.**
  If the owning process has synchronous I/O pending on that file object (for
  example a synchronous handle waiting in `LockFileEx`), the following all
  block until the owner's I/O ends: `GetFileInformationByHandleEx`
  (`FileIdInfo`, `FileBasicInfo`), `NtQueryInformationFile`
  (`FileModeInformation`), `GetFinalPathNameByHandleW`, and
  `NtQueryObject(ObjectNameInformation)`. `CancelSynchronousIo` on the stuck
  thread fails with 1168, because the thread is waiting on the file-object
  lock, not on its own I/O. A synchronous named-pipe server blocks the name
  queries the same way. `GetFileType` never blocked, so it is the only safe
  in-process pre-filter.
- **Terminating a helper process does release it.** A helper process stuck
  in those calls exited 1.0-1.5 ms after `TerminateProcess`, in 7 of 7
  trials, and the owner's lock state was unchanged. A thread inside your own
  process cannot be rescued this way; never terminate it.
- A process's current directory appears in its handle table as an ordinary
  directory handle (access `0x100020`), so a FileId match finds it. It cannot
  be told apart from any other directory handle without reading the PEB.
- On ReFS (`B:` on that host), `FileIdInfo` is a full 128-bit id, and files in
  one directory share the upper 64 bits. Compare the pair
  `(VolumeSerialNumber, 128-bit id)`; the 64-bit
  `GetFileInformationByHandle` index is not enough.
- `NtQuerySystemInformation(64)` (`SystemExtendedHandleInformation`, not a
  named constant in `windows` 0.62) needs a `STATUS_INFO_LENGTH_MISMATCH`
  retry loop; the snapshot was about 100k handles, 4 MB.
  `EnumProcessModulesEx` also needs a size retry.
- A live loaded-module test needs no fixture DLL: hard-link (or copy) the
  running test executable into the temp tree and start it there. Its main
  module is then reported at the link's path. `GetModuleFileNameExW`
  returns a plain `C:\...` spelling, while a `filesystem::query` report's
  `matched_paths` sit under the verbatim `\\?\` canonical root, so compare
  against `canonicalize()`, not the plain spelling (found 2026-10-06; eight
  tests failed only on Windows for this reason).
- `fsutil.exe file setCaseSensitiveInfo <dir> enable` works on
  build-win-native (2026-10-06) and needs the directory to exist. Whether the
  hosted `windows-latest` runner allows it is unverified; the
  `filesystem::query` case-sensitivity test fails, rather than skips, where it
  cannot.

## Asking which processes have a path open

Measured 2026-10-06 on `BUILD_WIN`. This is the safe alternative to resolving
another process's handles (above): open the path yourself and ask the kernel
which processes hold it.

- **The call:** `CreateFileW(path, FILE_READ_ATTRIBUTES, FILE_SHARE_READ |
  FILE_SHARE_WRITE | FILE_SHARE_DELETE, OPEN_EXISTING,
  FILE_FLAG_BACKUP_SEMANTICS for directories)`, then
  `NtQueryInformationFile(h, …, 47 /* FileProcessIdsUsingFileInformation */)`.
  The result is a `u32` count, then `usize` PIDs from byte offset 8. Start with
  a 4 KiB buffer, because every `STATUS_INFO_LENGTH_MISMATCH` retry repeats the
  whole costly call.
- **It does not hang** where the handle-side queries do. It returned
  promptly while the holder was blocked in `LockFileEx` or in a synchronous
  `ReadDirectoryChangesW` call. Our open and close did not wake the
  watcher.
- **Permissions.** No elevation is needed, and no other process is opened. A
  non-elevated desktop session saw System (PID 4) and other users' processes,
  the same holders as the elevated SSH session.
- **What it reports and what it misses:**
  - It reports open files and directories, cwd, loaded DLLs, a running exe,
    share-mode-0 holders, and holders with attribute-only or zero access.
  - Results never roll up to parent directories.
  - The caller's own PID is never listed.
  - A delete-pending file cannot be opened (`0xC0000056`), so its holders
    are invisible.
  - Long paths need the `\\?\` prefix.
- **Cost is per call and scales with the system's handle count, not the
  tree.** About 4 ms plus about 25 µs per 1,000 system handles, whether or
  not anything holds the path (6 ms at 100k handles). Calls scale nearly
  linearly across threads up to the core count, so walk large trees in
  parallel and in priority order.
- **Never open a pipe path to probe it.** Opening `\\.\pipe\…`, even
  `GetFileAttributesW` on it, connects to a waiting server as a client.
  Take the entry type from the directory walk.
- **Restart Manager (`RmGetList`) is a worse wrapper for the same question.**
  It has no directory or long-path support, it is slower per path, and a
  missing path returns success with an empty list.

## COM lifetime and cpal

Found 2026-10-08 on the `B:` dev host: `playa effect drop-4` exited with an
access violation (`0xC0000005`, reported by Git Bash as `Segmentation fault`)
at the end of every `just install`.

- **cpal's WASAPI host caches one process-global `IMMDeviceEnumerator` but
  initializes COM per thread.** Its thread-local guard calls `CoUninitialize`
  when its thread exits. If that was the last COM apartment in the process,
  COM unloads `MMDevApi.dll`, and the next cpal call on any other thread calls
  through a vtable in unmapped memory. Any code that gives each cpal call its
  own short-lived thread (a timeout worker, for example) hits this on the
  *second* call. playa pins COM with `CoIncrementMTAUsage` before spawning
  those workers (`pin_com_for_process` in `playa/lib/src/native_audio.rs`).
- **Diagnose with `cdbX64.exe`.** It ships with the WinDbg Store app at
  `%LOCALAPPDATA%\Microsoft\WindowsApps\cdbX64.exe`. Put commands in a file
  passed with `-cf`, because quoting `-c` through PowerShell silently produced
  no output. Use `bu combase!CoInitializeEx` / `CoCreateInstance` /
  `CoUninitialize` with `~.; kc 8; gc` actions, and finish with `lm u` to see
  which DLLs were unloaded.
- **A host with no sound hardware still lists one `SWD\MMDEVAPI` audio
  endpoint**, while `Get-CimInstance Win32_SoundDevice` is empty. cpal then
  reports no output device, and playa reports that error directly instead
  of falling back to host players.
