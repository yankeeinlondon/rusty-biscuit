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
2. **`dirs::home_dir()` on Windows uses the known-folder API and ignores
   `USERPROFILE` and `HOME`.** Hermetic test homes silently do not apply, so
   a Windows test reads the machine's real `~/.claudine`. Use
   `std::env::home_dir()` (un-deprecated, environment-first on Rust ≥ 1.97).
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

Contract to test against: `ctx.repo_root`, `package_root`,
`package_area_root`, and `area_root` are portable `/`-separated strings
without verbatim prefixes on every OS (`biscuit_file::to_portable_string`).
Compare against that, never against `to_string_lossy()`.

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
- **A `.cmd`/`.bat` cannot receive an argument containing a newline** ("batch
  file arguments are invalid"). A fake provider that receives a multi-line
  prompt must be a compiled `.exe`; see the rustc-built fixture in claudine's
  `inline_compose_hash.rs`.
- **Open handles block delete and rename.** A `File`, temp dir, mmap, or
  child that still holds a handle makes cleanup assertions fail on Windows
  only. Drop before asserting.
- **Ctrl+C and the exit-130 contract are Unix-only in Claudine today.** The
  Windows termination path is a bare `child.wait()` with no console control
  handler, and the child sits in `CREATE_NEW_PROCESS_GROUP`. Do not accept a
  cross-platform Ctrl+C acceptance criterion as met until a
  `SetConsoleCtrlHandler` path exists; see the claudine skill's
  `signal-handling.md`, "Windows parity".

## The `windows-latest` leg

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
