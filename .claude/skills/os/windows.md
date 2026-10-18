# Windows

Part of the `os` skill; [SKILL.md](SKILL.md) is the index.

Native Windows is the environment most often broken by code that passed on
macOS and Linux. This page covers processes, the environment, shells,
current-directory locks, the `windows-latest` leg, compile evidence from macOS,
and handle queries. Path spelling is in [windows-paths.md](windows-paths.md);
consoles, WezTerm, and PowerShell are in [windows-console.md](windows-console.md).

## Environment and processes

- **Environment variable names are case-insensitive.** Matching an exact
  spelling while iterating `env::vars()` misses `Path`/`PATH`; compare
  ASCII-case-insensitively. `env::var_os("PATH")` is fine.
- **Handle inheritance blocks `.output()`.** Rust std spawns with inheritable
  handles, so a detached grandchild started with `Stdio::null()` still holds
  the parent's pipe write ends, and `.output()` waits for it. Symptom: a
  Windows-only failure whose elapsed time equals some child's timeout while
  the exit status is success. Use `sniff::process::configure_detached_child`,
  which clears `HANDLE_FLAG_INHERIT` on the current stdio before setting the
  detached creation flags. Unix fds are close-on-exec and never show this.
- **`std::env::temp_dir()` ignores `TMPDIR`.** It reads `TMP`, then `TEMP`,
  then `USERPROFILE`. A fixture that redirects only `TMPDIR` writes into the
  host's real temp directory, and a later run trips over the leftovers. Set
  all three, as biscuit-speaks' `TtsFixture` does.
- **Overriding `USERPROFILE` alone breaks the per-user known folders.**
  `dirs::data_local_dir()`/`data_dir()` resolve `AppData\Local`/`Roaming`
  beneath `USERPROFILE` through `SHGetKnownFolderPath` and require them to
  exist (`LOCALAPPDATA`/`APPDATA` are not consulted), so a bare temp home
  yields `None`. Create both directories under the fixture home.
  `dirs::home_dir()` accepts a bare directory.
- **Python's piped stdout is cp1252 on a Windows runner.** Printing any
  non-ASCII character raises `UnicodeEncodeError` once output is piped, which
  turns a cell red after every test passed. `_package-ci.yml` and
  `_wsl-ci.yml` set `PYTHONUTF8=1` at workflow level, and a contract test keeps
  it there. Read the job's last step before blaming the runner.
- **`python3` may spawn successfully and still not be Python.** Windows ships
  App Execution Alias stubs that exit non-zero with "Python was not found", so
  `Command::new("python3").output()` returning `Ok` proves nothing, and a skip
  guard written against `Err` fails instead of skipping. Probe `--version`,
  require `status.success()`, and also try `python` (the hosted runner's real
  name); see `fn python_interpreter` in `scripts/ci-rollup-tests.rs`.
- **A skipped Python-gated test still prints PASS.** When an SSH session on
  `BUILD_WIN` resolves `python3`/`python` to a stub, tests gated on
  `python_interpreter()` skip and nextest reports a near-instant PASS. Before
  a `just cross-check` that needs Python, prepend a verified interpreter
  directory to a process-scoped `PATH`, and read the captured output for the
  test's own lines rather than trusting PASS. Host-specific spellings are in
  [build-hosts.md](build-hosts.md).
- **Run Windows tests from PowerShell, not Git Bash.** An agent's Bash tool
  on Windows is Git Bash (MSYS). Any process descended from it, including a
  `powershell.exe` it launches, can see a local-transport `git push` crash
  with `0xC0000005` (`STATUS_ACCESS_VIOLATION`) and empty stderr; the same
  tests pass from PowerShell. The trigger is the MSYS ancestor, not an
  environment value, so reproduce a Windows failure from the PowerShell tool
  before diagnosing it.
- **A Cygwin entry on the machine `PATH` takes over `just` recipes.** Machine
  entries precede user entries, so `bash`, `env`, `mktemp`, and `python3`
  resolve to Cygwin. A `#!/usr/bin/env bash` recipe then makes `/tmp/…` or
  `/cygdrive/…` temp paths that native Python cannot open (`FileNotFoundError`
  from `just ci-local --plan` or the pre-push hook), and with Cygwin merely
  present its `git` children can segfault (exit 139). Use a process-scoped
  `PATH` that drops every `cygwin` entry and prepends
  `C:\Program Files\Git\usr\bin` and the real Python directory.
- **Bash scripts lose data on Windows three ways:**
  - Native `jq.exe -r` ends lines with CRLF, so a value read back keeps a
    `\r` and matches nothing (for example a PR base of `main\r`). Use
    `jq --binary`; `just ci-local` and the pre-push hook do when
    `OS=Windows_NT`. Cygwin's `jq` writes LF and hides this.
  - `subprocess.run([bash, "-c", script])` lets MSYS/Cygwin bash re-split the
    script at nested quotes and run only the first fragment, exiting 0. Write
    the script to a file and run `bash <file>`.
  - Interpolating `{{ args }}` into a bash recipe eats a path's backslashes.
    Pass paths with forward slashes (`Path.as_posix()`).
- **`cmd /C` cannot start a command with `@1>&2`**; it runs a command named
  `1`. Write `@(1>&2 echo x)`, which also avoids the trailing space that
  `echo x 1>&2` leaves.
- **A `.cmd`/`.bat` cannot receive an argument containing a newline** ("batch
  file arguments are invalid"). A fake provider that gets a multi-line prompt
  must be a compiled `.exe` (see claudine's `inline_compose_hash` fixture).
  Real npm shims hit this too, so `claudine::child_environment::command`
  starts an npm `cmd-shim`'s target directly (`claudine/lib/src/npm_shim.rs`);
  a batch file of any other shape still fails.
- **Spawn cost dominates many small processes.** Each git process costs tens
  of milliseconds on Windows versus a few on macOS, and git for Windows routes
  a local-path transport through MSYS `sh.exe`. Symptom: a Windows-only
  timeout where `GIT_TRACE=<file>` shows the same metadata command repeated.
  Do not raise the budget: read metadata in-process (`worktree`'s
  `git_metadata`, `sniff`'s `gix` use) and keep git for transport. A `.cmd`
  shim adds a `cmd.exe` launch to every call, so a fake git in a fixture should
  be an `.exe` (`claudine-fake-relay` with `"forward": true`), and a fixture
  repository's config is cheaper written straight into `.git/config`.
- **A refused loopback connection takes about 2 s.** Windows retries the SYN
  to a closed `127.0.0.1` port before reporting refusal, so a test that
  expects a refused request to fail inside a deadline passes alone and fails
  under load. Assert only what holds either way, or accept and close instead
  (`ProxyStub::closing_after` rather than `ProxyStub::refusing`).
- **An SSH session's processes are already in a Job Object that forbids
  nesting**, so `AssignProcessToJobObject` fails with `ERROR_ACCESS_DENIED`
  (spelled `0x80070005` by the `windows` crate, not `os error 5`). Treat it as
  non-fatal and degrade to terminating the child alone, as `windows_wait_loop`
  does. Code that spawns through a Job behaves differently under
  `just cross-check` than in an interactive session; check `IsProcessInJob`
  before blaming the code.
- **`io::Error::from(windows::core::Error)` keeps the HRESULT as the raw OS
  error**, so a missing file reports `0x80070002` with an uncategorized kind,
  never `NotFound`. Before branching on `ErrorKind`, unwrap a `FACILITY_WIN32`
  HRESULT (`code & 0xFFFF0000 == 0x80070000`) to
  `from_raw_os_error(code & 0xFFFF)`; see `io_error` in
  `sniff/lib/src/filesystem/query/identity.rs`.
- **Open handles block delete and rename.** A live `File`, temp dir, mmap, or
  child makes cleanup assertions fail on Windows only; drop before asserting.
  Even with owned handles closed, a concurrent reader or host scanner can make
  `MoveFileExW(REPLACE_EXISTING)` fail transiently with `ERROR_ACCESS_DENIED`
  or `ERROR_SHARING_VIOLATION`. Retry those two for a short bounded interval,
  keep the replacement atomic, and never delete the destination first.
- **A delete refused because the file is open is os error 32 with an
  uncategorized `ErrorKind`.** Match `raw_os_error() == Some(32)` under
  `cfg(windows)`; no `io::ErrorKind` matches it. SQLite opens its database,
  `-wal`, and `-shm` without delete sharing, so this is how deleting an open
  DMLS index store fails (`dmls/src/index_store/location.rs`). Unix unlinks
  the same open files and the holder keeps writing to an orphan, so a delete
  failing is a liveness test on Windows only; the store uses a lock file.
- **`File::set_modified` needs a write handle.** Through a handle opened
  read-only it fails with os error 5; open with `.write(true)` first
  (`index_store::touch_mtime`).
- **`std::fs::rename` and `tempfile::persist` differ against open readers.**
  Std opens files with `FILE_SHARE_DELETE`, so `std::fs::rename` over a
  destination held by such a handle succeeds, while
  `NamedTempFile::persist` fails with error 5. A holder without delete sharing
  (CRT `_wopen`, editors, scanners) blocks both with error 5, as does an open
  file inside a directory being renamed. Prefer `std::fs::rename` plus the
  bounded retry above.
- **A named pipe's name briefly outlives its dropped server.** The kernel
  cancels the pending `ConnectNamedPipe` asynchronously, so a client opening
  the name just after shutdown can connect and then fail mid-RPC with
  `Unknown`, `Cancelled`, or `Unavailable`. Unix unlinks a socket
  synchronously. The rendezvous daemon's `wait_released` polls until the name
  resolves `NotFound`. To reproduce a one-OS flake, run its test binary many
  times in parallel (`--exact`), not nextest once.
- **Killing `git` at a deadline leaves the real git running.** `git.exe` on
  `PATH` is a launcher for `mingw64\bin\git.exe`, which runs the transport;
  `Child::kill` ends only the launcher. The orphans hold the stderr pipe (a
  reader waiting for EOF blocks) and their working directory (which then
  cannot be deleted). `taskkill /T /F /PID <pid>` releases everything, under
  SSH too. On Unix the pipe half is fixed with `process_group(0)` and a
  negative-PID `SIGKILL`.
- **The main thread gets a 1 MiB stack.** Symptom: a spawned binary dies with
  `0xC00000FD` (`STATUS_STACK_OVERFLOW`, `code=-1073741571`) and empty stderr,
  Windows only, because large debug-build frames overflow a call chain that
  fits Unix's 8 MiB. Reproduce on macOS with `(ulimit -s 768; <binary> …)`.
  Fix it in the binary's `build.rs` with `cargo:rustc-link-arg-bin(s)`
  (`/STACK:<bytes>` for msvc, `-Wl,--stack,<bytes>` for gnu), as
  `claudine/cli`, `darkmatter/cli`, and `sniff/cli` do. It covers the main
  thread only; spawned threads keep Rust's 2 MiB default.
- **Ctrl+C in Claudine goes through one process-wide
  `SetConsoleCtrlHandler` handler** accepting `CTRL_C_EVENT` and
  `CTRL_BREAK_EVENT`; its force-exit rung exits `130` via `ExitProcess`. A
  test delivers a press as `CTRL_BREAK_EVENT` to a `CREATE_NEW_PROCESS_GROUP`
  child or as ETX typed into a pseudoconsole (see the inherited-ignore trap in
  [windows-console.md](windows-console.md)). Details: the claudine skill's
  `topics/signal-handling.md`, "Windows parity".
- **Template commands cannot use `cmd.exe` builtins.** Claudine's
  template-mode commands resolve a native executable, and `echo` has none on
  a hermetic fixture `PATH`, so `shell: 'echo {{ value }}'` fails on Windows
  while macOS finds `/bin/echo`. Use the `claudine-argv-recorder` binary
  (copied as `argv-recorder.exe`) and assert its recorded argv; literal shell
  commands may still use builtins. Do not add an interpreter fallback.

## Current-directory locks

- **A process's current directory cannot be deleted, moved, or renamed**
  ("being used by another process"). Unix has no such lock, so code that
  removes a directory the user may stand in is untested until it runs here.
- **Which holders lock the directory:**

  | Holder | Locks? |
  |---|---|
  | Any process *launched* with it as working directory (a terminal tab, `Start-Process -WorkingDirectory`) | Yes |
  | cmd.exe after `cd /d` | Yes |
  | Windows PowerShell after `Set-Location` | No: the location is not the Win32 current directory |
  | Git Bash (MSYS) after `cd` | No |
  | A `FileSystemWatcher` (Explorer, editors) | No |

- **Children start in PowerShell's *location*,** so a `wt.exe` or `git`
  launched from a PowerShell standing in the directory holds it. A tool that
  deletes it must first `std::env::set_current_dir` out and address the
  repository with `git -C`.
- **Releasing a PowerShell lock takes both** `Set-Location` and
  `[Environment]::CurrentDirectory = …`.
- **`git worktree remove` fails late.** With the directory held it exits 255
  ("Permission denied") *after* deleting every file and the worktree's
  metadata; only the empty directory remains. Check for the lock first.
- **Detect the lock without side effects** by renaming the directory to a
  sibling name and straight back. It fails on the first rename exactly for
  current-directory holders and open files, and is cheap even for large trees.
- **A test's lock holder must be spawned directly.** `cmd /C "ping …"` then
  `kill()` kills only `cmd`; `ping` keeps the directory and the inherited
  output pipe, so nextest waits out its full run. Spawn the holder itself with
  `current_dir` and null stdio (`worktree/lib/src/remove/mod.rs`,
  `worktree/cli/tests/powershell_wrapper_exec.rs`). A windowless way to get
  a "window launched inside the directory" is a ConPTY child
  ([windows-console.md](windows-console.md)).

## The `windows-latest` leg

Leg size, timing, and excluded `#![cfg(unix)]` binaries are in
[ci-runners.md](ci-runners.md). Sniff runs its L1 group one process at a time
there (`sniff-windows-l1` in `.config/nextest.toml`) because overlapping
host-network detections fail the test process.

- **Malformed bytes with an audio extension still reach host playback.**
  Playa accepts a recognized extension when header detection fails, then
  falls back to installed host players, so a "bad `.wav`" fixture depends on
  the runner's players. Failure fixtures use an unrecognized extension and
  assert rejection before any backend is reached.

## Compile evidence from macOS

The `just check-windows` recipe and the GNU-not-MSVC rule are in the
`rust-testing` skill's `tiers-and-gating.md`. Traps beyond it:

- `x86_64-pc-windows-gnu` can vanish after a toolchain change; re-run
  `rustup target add x86_64-pc-windows-gnu`.
- `blake3` fails on that target with `cc-rs: failed to find tool "ml64.exe"`
  (its SIMD assembly needs the MSVC assembler). Anything enabling
  `biscuit-hash`'s `blake3` feature inherits this (`repo-deps` does); check
  such `#[cfg(windows)]` code with a throwaway probe (below).
- Crates reaching `duckdb-sys` (rendezvous-daemon and its dependents)
  overflow mingw's COFF section limit; `check-windows` sets `-Wa,-mbig-obj`
  and a separate target dir, and claudine-cli gates the daemon dev-dependency
  on `cfg(not(all(windows, target_env = "gnu")))`.
- A `rustc-wrapper = "kache"` setting leaks into `cc-rs` and breaks every C
  build for the cross target (set `RUSTC_WRAPPER=""`), and kache's read-only
  artifacts need their own `CARGO_TARGET_DIR`. Per-host kache policy is in
  `docs/kache-strategy.md`.
- **Throwaway probe:** for a `#[cfg(windows)]` file the workspace graph cannot
  reach, copy it into a scratch crate with `windows` and `tokio`, stub its
  seams, and `cargo check --target x86_64-pc-windows-gnu`. It validates Win32
  signatures in seconds, not runtime behavior.
- **Clippy can be red only on Windows.** `PathBuf`/`OsString` is 32 bytes
  there versus 24 on Unix, so an error enum holding paths can cross the
  workspace `large-error-threshold` (`clippy.toml`, measured on Unix) for
  `result_large_err` on Windows alone. Box or `Arc` the large cause in that
  variant rather than raising the limit; if errors are classified by
  downcasting `source()`, the downcast must name the wrapper. Where a public
  error shape cannot change, allow the lint under `cfg(windows)` as
  `claudine/lib/src/lib.rs` does. `cfg`-dependent lints also appear only here:
  `dead_code`/`unused_mut` on items only `#[cfg(unix)]` code uses, and
  `needless_return` on a `#[cfg(windows)]` block that becomes the tail once its
  sibling is compiled out.

A cross-compile is compile evidence only. Behavioral evidence comes from
`just cross-check <pkg> --os windows` or the `windows-latest` leg.

## Inspecting another process's handles

An SSH session on `BUILD_WIN` is elevated (High integrity, SeDebugPrivilege),
so handle and permission results taken over SSH overstate what a normal user
sees; use a filtered Medium-integrity token or a desktop session.

- **Identity queries on a duplicated disk handle can block indefinitely**
  when the owner has synchronous I/O pending on that file object (for example
  in `LockFileEx`): `GetFileInformationByHandleEx`, `NtQueryInformationFile`,
  `GetFinalPathNameByHandleW`, and `NtQueryObject(ObjectNameInformation)` all
  wait. `CancelSynchronousIo` fails with 1168 because the thread waits on the
  file-object lock, not its own I/O. A synchronous named-pipe server blocks
  name queries the same way. `GetFileType` never blocks and is the only safe
  in-process pre-filter.
- **Run those queries in a helper process.** `TerminateProcess` releases a
  stuck helper promptly without disturbing the owner's lock. A stuck thread in
  your own process cannot be rescued; never terminate it.
- A process's current directory is an ordinary directory handle (access
  `0x100020`) in its handle table; it cannot be told apart from other
  directory handles without reading the PEB.
- On ReFS, `FileIdInfo` is a full 128-bit id and files in one directory share
  the upper 64 bits. Compare `(VolumeSerialNumber, 128-bit id)`; the 64-bit
  `GetFileInformationByHandle` index is not enough.
- `NtQuerySystemInformation(64)` (`SystemExtendedHandleInformation`, not a
  named constant in `windows` 0.62) and `EnumProcessModulesEx` both need a
  size-retry loop (`STATUS_INFO_LENGTH_MISMATCH`); the system snapshot runs to
  megabytes.
- A loaded-module test needs no fixture DLL: hard-link the running test
  executable into the temp tree and start it there. `GetModuleFileNameExW`
  returns a plain `C:\…` spelling while `filesystem::query`'s `matched_paths`
  sit under the verbatim `\\?\` root, so compare against `canonicalize()`.
- `fsutil.exe file setCaseSensitiveInfo <dir> enable` works on
  `build-win-native` once the directory exists; whether `windows-latest`
  allows it is unverified, and the `filesystem::query` case-sensitivity test
  fails rather than skips where it cannot.

## Asking which processes have a path open

The safe alternative to resolving other processes' handles: open the path
yourself and ask the kernel who holds it.

- **The call:** `CreateFileW(path, FILE_READ_ATTRIBUTES, FILE_SHARE_READ |
  FILE_SHARE_WRITE | FILE_SHARE_DELETE, OPEN_EXISTING,
  FILE_FLAG_BACKUP_SEMANTICS for directories)`, then
  `NtQueryInformationFile(h, …, 47 /* FileProcessIdsUsingFileInformation */)`.
  The result is a `u32` count, then `usize` PIDs from byte offset 8. Start
  with a 4 KiB buffer; every `STATUS_INFO_LENGTH_MISMATCH` retry repeats the
  costly call.
- **It does not hang** where the handle-side queries do (holder in
  `LockFileEx` or synchronous `ReadDirectoryChangesW`), and the open does not
  wake a watcher.
- **No elevation needed** and no other process opened; a non-elevated session
  sees System and other users' holders too.
- **Reports** open files and directories, cwd, loaded DLLs, a running exe,
  share-mode-0 holders, and attribute-only or zero-access holders. **Misses:**
  roll-up to parent directories, the caller's own PID, and holders of a
  delete-pending file (cannot be opened, `0xC0000056`). Long paths need `\\?\`.
- **Cost scales with the system's handle count, not the tree,** and is a few
  milliseconds per call whether or not anything holds the path. Calls scale
  nearly linearly across threads, so walk large trees in parallel and in
  priority order.
- **Never open a pipe path to probe it.** Even `GetFileAttributesW` on
  `\\.\pipe\…` connects to a waiting server as a client; take the entry type
  from the directory walk.
- **Restart Manager (`RmGetList`) is worse for the same question:** no
  directory or long-path support, slower per path, and a missing path returns
  success with an empty list.

## COM lifetime and cpal

- **cpal's WASAPI host caches one process-global `IMMDeviceEnumerator` but
  initializes COM per thread.** When the last COM thread exits, COM unloads
  `MMDevApi.dll` and the next cpal call jumps through unmapped memory:
  `0xC0000005` (Git Bash prints `Segmentation fault`) on the *second* call
  from short-lived worker threads. Pin COM with `CoIncrementMTAUsage` before
  spawning them (`pin_com_for_process` in `playa/lib/src/native_audio.rs`).
- **Diagnose with `cdbX64.exe`** (ships with the WinDbg Store app under
  `%LOCALAPPDATA%\Microsoft\WindowsApps`). Pass commands in a `-cf` file;
  `-c` quoted through PowerShell silently produces nothing. Break on
  `combase!CoInitializeEx`/`CoCreateInstance`/`CoUninitialize` with
  `~.; kc 8; gc`, then `lm u` lists unloaded DLLs.
- **A host with no sound hardware still lists one `SWD\MMDEVAPI` endpoint**
  while `Win32_SoundDevice` is empty; cpal reports no output device and playa
  surfaces that error instead of falling back to host players.
