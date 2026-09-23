# Windows

Native Windows is the environment most often broken by code that passed on
macOS and Linux. Each trap below has bitten a real PR; the fix or the shared
helper that resolves it is named so it is not re-derived.

## Path spelling

1. **`std::fs::canonicalize` returns a verbatim `\\?\C:\...` path with long
   names.** Any such value that crosses a comparison, containment, or
   reference-grammar boundary breaks: the `?` becomes an extra lexical
   segment and the finalized reference grammar rejects device-prefix
   spellings outright. Route through `biscuit_file::canonicalize_simplified`
   (dunce-backed). Raw `canonicalize` is only safe inside a closed key space
   that never leaves the process.

   Two concrete instances live in `cargo nextest`'s own argument grammar
   (found 2026-09-14, `scripts/ci-build-archive.rs`): `--tool-config-file` is
   split on the colon after the tool name, and `--workspace-remap` is compared
   against ordinary paths. A verbatim value breaks both. `repo-deps` is the one
   place that may not reach for `biscuit_file` — the planner calls `ci-build`
   on every scope calculation, so that binary is deliberately kept to
   `biscuit-hash` plus `biscuit-terminal` — so it carries a six-line
   `canonical_path` that strips the prefix for a drive-qualified path under 260
   characters and leaves a long or UNC one alone. Anywhere else, use
   `biscuit_file`.
2. **`dirs::home_dir()` on Windows uses the known-folder API and ignores
   `USERPROFILE` and `HOME`.** Hermetic test homes silently do not apply, so
   a Windows test reads the machine's real `~/.claudine`. Use
   `std::env::home_dir()` (un-deprecated, environment-first on Rust ≥ 1.97).
   Python's `Path.home()` is environment-first but reads `USERPROFILE` on
   Windows and ignores `HOME` (which native Windows does not set outside Git
   Bash), so a fixture that relocates the home for a Python tool such as
   `scripts/ci/constraints.py` must set both `HOME` and `USERPROFILE`; a
   shell `$HOME` literal is a Unix-only spelling.
   Claudine's provider overlay still resolves through the known folder, so a
   Windows launch test names its roots instead: the provider selector (e.g.
   `CODEX_HOME`) for the source and `CLAUDINE_OVERLAY_DIR` for overlay
   storage (`level2_provider_overlay_capture.rs`, 2026-09-16).
3. **GitHub's Windows runner has an 8.3 short-name TEMP (`RUNNER~1`); no
   developer machine does.** Short-versus-long spelling bugs reproduce only
   on CI. `current_dir()` reports the spelling it was given; `canonicalize`
   re-spells to long names. Model "what the child reports": the
   `launched_spelling` test helper canonicalizes on Unix (for the macOS
   `/var` symlink) and keeps the raw path on Windows.
4. **`Path::join("a/b")` keeps the literal `/`.** A native-spelling needle
   built from it has mixed separators and matches nothing. Re-join through
   `.components().collect::<PathBuf>()` to normalize.
5. **Windows temp dirs contain a dot-initial segment (`\.tmpXXXX`).** Any
   Markdown round-trip that resolves CommonMark backslash escapes will eat
   the `\` before `.`, `-`, or `_`. Darkmatter's compose Cleanup phase now
   preserves them; when a Windows-only failure shows a path missing one
   backslash, suspect Markdown escape handling, not path resolution.

6. **`$PWD` in a `shell: bash` step is an MSYS path; `$RUNNER_TEMP` in the
   same shell is a Windows one.** Git Bash answers `/d/a/repo/repo` for the
   checkout and `D:\a\_temp` for the temp directory, and a CI step routinely
   hands both to native programs. `cargo-nextest`'s `--workspace-remap`,
   `INSTA_WORKSPACE_ROOT`, and `BISCUIT_JUNIT_*` cannot open the first; MSYS
   `test`/`mkdir` cope with the second only by conversion. `just _native_path`
   (`just/devops.just`) answers the one spelling both layers accept —
   `cygpath -m`, drive-qualified with forward slashes, no verbatim prefix.
   Measured 2026-09-14 on `build-win-native`: `/w/…/rusty-biscuit` →
   `W:/…/rusty-biscuit`, `D:\a\_temp/build` → `D:/a/_temp/build`. In CI,
   `_ci_build_verify` computes it once and publishes it as its `workspace`
   step output.
7. **Backslashes do not survive a `just` recipe's `*args` list.** A recipe
   pastes `{{ args }}` raw into `forwarded=({{ args }})`, so bash word-splits
   it *and* processes backslash escapes:
   `--archive-file=C:\Users\ken\…\x.tar.zst` reaches the command as
   `C:Usersken…x.tar.zst`, and the tool reports a missing file for a path
   nobody typed. Pass what `just _native_path` answers. A recipe parameter
   interpolated inside **single quotes** (`'{{ path }}'`) is safe, which is why
   `_native_path` itself can be handed a native spelling; an array literal is
   not. `_archive_file_check` refuses an unreadable `--archive-file` and names
   this hazard rather than letting the mangled value reach nextest.
8. **A `file://` URI must carry neither the verbatim prefix nor a `\`.**
   Percent-encoding a canonicalized Windows path yields
   `file://%5C%5C%3F%5CC:/…`, which no terminal opens, and a drive-absolute
   path still needs the extra leading `/` that makes `file:///C:/…`. Normalize
   separators to `/`, strip `\\?\` (mapping `\\?\UNC\server\share` to the URI
   authority `server/share`), then prefix. Test the spellings as string
   literals so the macOS and Linux cells cover them too — `fs::canonicalize`
   only produces the verbatim form on Windows, so a fixture built from it is
   dead code everywhere else. Found 2026-09-14 in `scripts/drift.rs::file_uri`.

Contract to test against: `ctx.repo_root`, `package_root`,
`package_area_root`, and `area_root` are portable `/`-separated strings
without verbatim prefixes on every OS (`biscuit_file::to_portable_string`).
Compare against that, never against `to_string_lossy()`.

9. **A user-typed path fragment never matches walker output by raw text.**
   `ignore::Walk` yields native `\` paths; the fragment is whatever was typed,
   `/` on every platform. Claudine's partial-file and operation-file
   autocomplete compared them raw and found zero candidates on Windows for
   every `/`-spelled partial, so the typed "no existing file matched" failure
   fired where macOS offered the confirmation (found 2026-09-10 by the first
   native-Windows Level 2 run; `claudine/cli/src/completion/scopes.rs`
   `path_matches_query`). Compare both sides through `to_portable_string`
   and normalize `\` in the fragment. The non-interactive Windows tests had
   passed the whole time — the zero-candidate path and the interaction-denied
   path produce the same diagnostic — which is why only a real-terminal run
   on Windows could see it.

## WezTerm on `build-win`

- `~/.wezterm.lua` there `dofile`s the shared config from `X:` and then a UNC
  path. `X:` is a per-logon mapped drive that no SSH, nextest, or mux-server
  process has, so every config *evaluation* from those contexts blocks for
  the SMB connect timeout — measured at 21.2–21.3 s, deterministic, on each
  `wezterm cli spawn`; Windows negative-caches the failure for well under a
  minute, so a second spawn seconds later is 0.1 s. `wezterm cli list` does
  not evaluate the config at all, which is why the harness's availability
  gate passes and only the spawn times out (15 s `SPAWN_TIMEOUT`).
  `biscuit-test-harness` now runs every `wezterm` client under an empty
  `WEZTERM_CONFIG_FILE` unless the caller set one; the mux server keeps its
  own config. The empty config is a private, uniquely created temporary file
  retained until that client exits and then removed; creation/write errors
  propagate rather than selecting an existing shared pathname. Do not "fix"
  this by raising the timeout.
- A headless `wezterm-mux-server` reachable through `WEZTERM_UNIX_SOCKET`
  (`C:\Users\ken\.local\share\wezterm\sock`) is all the Level 2 tier needs
  there; the twin in `level2_windows_provided_partial_file_capture.rs` passes
  against it in ~4 s. Never blanket-stop mux servers on that host — one of
  them may be carrying the session the developer is working in.
- `cross-check --os windows` runs in an SSH session with no
  `WEZTERM_UNIX_SOCKET`, so a WezTerm Level 2 test **skips there and nextest
  prints PASS in ~0.02 s**. Read the duration, or set
  `BISCUIT_TEST_REQUIRED_BACKENDS=wezterm` so a missing backend fails.

## Environment and processes

- **Environment variable names are case-insensitive.** Iterating
  `env::vars()` and matching an exact spelling misses `Path`/`PATH`. Compare
  ASCII-case-insensitively when iterating; `env::var_os("PATH")` is fine.
- **Handle inheritance blocks `.output()`.** Rust std spawns with inheritable
  handles. A child that spawns a detached grandchild with `Stdio::null()`
  still passes the *parent's* pipe write ends along, and the parent's
  `.output()` waits for every holder to close. Symptom: Windows-only failure
  whose elapsed time equals some child's timeout while the exit status is
  success. Fix: `playa::detached::configure_detached_child` clears
  `HANDLE_FLAG_INHERIT` on the current process's stdio before setting the
  detached creation flags (needs `windows-sys` features `Win32_Foundation`
  and `Win32_System_Console`). Unix never sees this; fds are close-on-exec.
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
- **A `.cmd`/`.bat` cannot receive an argument containing a newline** ("batch
  file arguments are invalid"). A fake provider that receives a multi-line
  prompt must be a compiled `.exe`; see the rustc-built fixture in claudine's
  `inline_compose_hash.rs`. This is not only a test concern: Claudine delivers
  the composed prompt as one `-t` argument, so a real npm-installed
  `goose.cmd`/`claude.cmd` on a user's `PATH` fails the same way. The
  `wrap_compose_validation` stub was still a `.cmd` until 2026-09-22, and the
  first native-Windows run of `claudine-cli` is what found it.
- **An SSH session's processes are already inside a Job Object**, and that Job
  forbids nesting, so `AssignProcessToJobObject` returns
  `ERROR_ACCESS_DENIED` (`Access is denied. (0x80070005)`, the `windows`
  crate's HRESULT spelling, not `os error 5`). Anything that treats that call
  as fatal cannot run under SSH at all: it failed every `sequence_budget`
  launch until `windows_wait_loop` was made to degrade to terminating the
  child alone (2026-09-22). Verify a host with `IsProcessInJob` before
  blaming the code. Tests that spawn through a Job therefore behave
  differently over `just cross-check` than in an interactive session.
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
- **A PowerShell function's output stream is not its return value.** Every
  native command inside a function writes its stdout into that function's
  output, so `$code = Invoke-Thing` binds an *array* whose first element is
  some program's chatter, and `exit $code` reports that. Symptom: a remote run
  whose tier exited 1 is summarized as a pass (`cross-check --os windows`,
  build-win-native, 2026-09-14). Assign a `$script:`-scoped variable at every
  failure point and call the function as `Invoke-Thing | Out-Host`, which keeps
  the log on the console without binding it. `$LASTEXITCODE` after each native
  command is still the right check — the bug is in how the result leaves the
  function, not in how it is read.
- **Ctrl+C and the exit-130 contract are Unix-only in Claudine today.** The
  Windows termination path is a bare `child.wait()` with no console control
  handler, and the child sits in `CREATE_NEW_PROCESS_GROUP`. Do not accept a
  cross-platform Ctrl+C acceptance criterion as met until a
  `SetConsoleCtrlHandler` path exists; see the claudine skill's
  `signal-handling.md`, "Windows parity".

## Attaching a console inside a nextest process

`biscuit-tui/cli/tests/windows_captured_stdout.rs` is ordinary `windows-latest`
**L1** evidence inside `biscuit-tui-cli`'s own cell — not an `#[ignore]`d test
behind a hand-invoked recipe or workflow. Everything below was measured on
`build-win-native` at the CI thread count (`--test-threads 4`), 2026-09-14.

- **Process-wide handle rewiring is safe only because nextest gives each test
  its own process.** `AllocConsole` + `SetStdHandle` mutate process state; under
  `cargo test`'s shared harness they would corrupt every sibling test in the
  binary. Say so in the test's `//!` docs — it is the reason the tier is L1
  rather than a serialized L3.
- **`AllocConsole` returning `ERROR_ACCESS_DENIED` (0x80070005) is the normal
  path, not a failure.** A console is usually already present, and the API
  reports that as access denied. Treat "already present or failed" as one state
  and assert the *precondition you actually need* — `stderr.is_terminal()` and
  `CONOUT$` openable — instead of the call's return value.
- **Redirecting a std handle to `CONOUT$` makes everything printed afterwards
  invisible to nextest.** The line goes to the attached console, not to the
  harness pipe. Two consequences, both found the hard way: a success diagnostic
  printed after the redirect never reaches the log (`grep -c` returns 0), and —
  worse — an assertion that panics *after* the redirect leaves nextest reporting
  `FAIL` with an empty message. Redirect only the handle the contract requires
  (stderr here; the stdout redirect was deleted as unnecessary), capture the
  original handle before redirecting, and restore it the moment the child exits
  so later failures are reported through the pipe.
- **The console input buffer queues injected records**, so a written input
  record survives the child not having started its event loop yet. The 750 ms /
  250 ms fixed sleeps this test shipped with were covering a measured
  requirement of **0 ms**: the test's real work is ~45 ms and the sleep *was*
  its 0.78 s runtime. A bounded readiness loop — 2 s deadline, 25 ms poll,
  re-inject at 500 ms — replaced them; no passing run has needed the second
  injection. Keep the loop anyway: it converts a timing assumption into an
  assertion that fails loudly at its own deadline rather than at nextest's 90 s
  `ci` termination ceiling, and it kills and reaps the child so the cell reports
  `FAIL` rather than `LEAK`.
- Six consecutive clean runs, zero flakes (392 run / 392 passed / 7 skipped).
  Runs that died in `git fetch` with `ssh: connect to host github.com port 22`
  are a build-host network fault, not a test result — exclude them rather than
  counting them as failures.

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

A cross-compile is compile evidence. Behavioral evidence comes from
`just cross-check <pkg> --os windows` or the `windows-latest` CI leg.
