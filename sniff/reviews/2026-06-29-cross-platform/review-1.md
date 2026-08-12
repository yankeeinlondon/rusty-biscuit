---
$schema:
  ready: false
  macos_ready: true
  windows_ready: false
  linux_ready: false
  agent: string
  created: string
about: A cross-platform review of the sniff package area on 2026-06-29
author: prompts/cross-platform.md
created: 2026-06-29T15:50:35
agent: codex/default
ready: false
macos_ready: true
windows_ready: false
linux_ready: false
---

## Summary

I reviewed `sniff/lib` and `sniff/cli` for cross-platform compatibility across macOS, Linux, native Windows, and WSL. The codebase has a clear intent to support platform-specific behavior: Windows SCM is isolated behind `#[cfg(target_os = "windows")]`, macOS bundle discovery is separated, Linux/macOS/Windows time detection have target branches, and PATH lookup commonly uses `which` or `std::env::split_paths`.

The package area is not acceptable yet based solely on cross-platform support. macOS is in the best shape and `cargo check --color=never -p sniff --all-targets` passed on the macOS host. Native Windows is not acceptable because multiple tests contain unconditional Unix-only imports/helpers or Windows-malformed PATH setup. Linux is also not acceptable because at least one test module references macOS-only storage helpers without a Linux/test cfg path. WSL mostly follows the Linux path; I did not find WSL-specific Windows path translation or Windows executable interop, but WSL inherits the Linux test/build concerns.

The Windows cross-check could not reach crate code on this macOS host because transitive native crates failed before `sniff` compiled for `x86_64-pc-windows-msvc` due missing Windows/MSVC C headers (`windows.h`, `stdlib.h`, `ctype.h`). That is an environment limitation for this review, not proof that the crate compiles on Windows.

## Findings

### High: Non-macOS tests reference macOS-only storage helpers

Affected targets: Linux, native Windows, WSL test builds.

`sniff/lib/src/hardware/storage.rs:193` gates `parse_diskutil_info_output` with `#[cfg(target_os = "macos")]`, but the test module at `sniff/lib/src/hardware/storage.rs:210` is only `#[cfg(test)]` and calls that helper at `sniff/lib/src/hardware/storage.rs:217`, `sniff/lib/src/hardware/storage.rs:223`, and `sniff/lib/src/hardware/storage.rs:229`. On Linux or Windows test builds, the helper is not compiled but the tests still are.

Smallest fix: gate those tests with `#[cfg(target_os = "macos")]`, or change the helper cfg to `#[cfg(any(target_os = "macos", test))]` if the parser is intended to be tested as pure logic on every host.

### High: Unit tests unconditionally import Unix-only APIs

Affected targets: native Windows test builds.

`sniff/lib/src/programs/local_bin.rs:283` defines an all-platform test module, but `sniff/lib/src/programs/local_bin.rs:288` imports `std::os::unix::fs::PermissionsExt` unconditionally and `make_executable` uses Unix mode bits at `sniff/lib/src/programs/local_bin.rs:293`.

`sniff/lib/src/programs/test_runner.rs:331` has the same problem: the test module imports `std::os::unix::fs::PermissionsExt` at `sniff/lib/src/programs/test_runner.rs:336` and uses Unix permissions in `make_executable` at `sniff/lib/src/programs/test_runner.rs:341`.

Smallest fix: make the test helper platform-aware. On Unix, set `0o755`; on Windows, create a runnable file with a Windows extension such as `.cmd` or `.exe` as appropriate for the resolver being tested. Keep assertions conditional when the expected suffix differs.

### High: Several PATH tests build PATH with a Unix separator

Affected targets: native Windows test behavior.

`sniff/lib/src/executable_index.rs` mutates `PATH` in tests by pushing a literal `":"` into an `OsString` at lines `508`, `538`, `575`, `643`, and `686`. Those tests are not all Unix-only. On Windows, `PATH` entries are separated with `;`, so the temporary directory and the existing PATH become one malformed entry.

Smallest fix: use `std::env::join_paths` for test PATH construction, or use a small test helper that accepts `PathBuf` entries and preserves the original `PATH` via `split_paths`/`join_paths`.

### Medium: Production PATH parsing reimplements platform rules and rejects non-Unicode PATH

Affected targets: all targets, with highest risk on native Windows.

`sniff/lib/src/os/package_manager.rs:364` reads `PATH` through `std::env::var("PATH")`, then manually chooses `;` or `:` at `sniff/lib/src/os/package_manager.rs:378` and splits the string at `sniff/lib/src/os/package_manager.rs:383`. This duplicates standard library behavior and loses non-Unicode environment values that `var_os` would preserve.

Smallest fix: replace the manual parsing with:

```rust
let Some(path_var) = std::env::var_os("PATH") else { return Vec::new(); };
let dirs = std::env::split_paths(&path_var)
    .filter(|p| p.is_dir())
    .collect();
```

The tests in `sniff/lib/src/os/package_manager.rs:1675` should also build expected PATH values with `join_paths` instead of `format!("{}:{}", ...)` / `format!("{};{}", ...)`.

### Medium: Windows audio detection depends on `wmic`

Affected targets: native Windows.

`sniff/lib/src/hardware/audio.rs:711` documents Windows audio detection through `wmic`, and `sniff/lib/src/hardware/audio.rs:718` runs `wmic path Win32_SoundDevice get Name,Status`. `wmic` is deprecated and absent on some modern Windows installations, so supported Windows hosts can report no audio devices even when devices exist.

Smallest fix: use a supported Windows API path. A pragmatic step is `powershell -NoProfile -Command Get-CimInstance Win32_SoundDevice ...` with a timeout and parser tests; the stronger long-term fix is a Windows MMDevice/CoreAudio API implementation behind `#[cfg(target_os = "windows")]`.

### Medium: Windows path display aliasing assumes Unix-like home/env semantics

Affected targets: native Windows CLI output.

`sniff/cli/src/output/filesystem/mod.rs:587` aliases paths using `std::env::vars_os()` and `HOME` from `sniff/cli/src/output/filesystem/mod.rs:589`. On Windows, `USERPROFILE` is the normal home variable, environment variable names are case-insensitive, and path prefix comparisons may differ by drive and case. The skip list at `sniff/cli/src/output/filesystem/mod.rs:599` only filters exact `PWD` / `OLDPWD`.

Smallest fix: use `dirs::home_dir()` for the home path, compare environment variable names with ASCII-case-insensitive logic on Windows, and add Windows-shaped unit tests with drive-letter paths and `USERPROFILE`.

### Low: Windows dynamic/IPC concerns were not present

Affected targets: none found.

I did not find Unix domain socket, named pipe, fixed TCP port, dynamic library loading, or build-script linking code in the `sniff` package area. The socket and dynamic-library portability risks from the prompt do not appear to be active concerns for this area today.

## Suggestions

Prioritize making all unit tests compile under Linux and Windows targets. Fix the `storage.rs`, `local_bin.rs`, and `test_runner.rs` cfg issues first, then add a CI job that runs `cargo check -p sniff --all-targets` and `cargo nextest run -p sniff` on Linux and Windows.

Centralize test PATH construction in one helper that uses `std::env::split_paths` and `std::env::join_paths`. Then replace the literal `":"` mutations in `executable_index.rs` and the manual `format!` PATH strings in `package_manager.rs`.

Replace production `PATH` parsing in `os/package_manager.rs` with `var_os` plus `split_paths`. This is a small code change and aligns that module with the more portable approach already used elsewhere in the package area.

Modernize Windows hardware detection by replacing `wmic` with a supported API or PowerShell CIM fallback. Treat missing `wmic` as expected on Windows, not as "no devices".

Add explicit Windows-shaped pure tests for path rendering and executable lookup: drive-letter roots, semicolon PATH, `.exe` / `.cmd` / `.ps1`, and case-insensitive env var names. Add Linux/WSL-shaped tests for `/proc`-backed detectors that make fallback behavior explicit when `/proc` files are missing.

Document the supported CI matrix for `sniff`: macOS, Linux, and Windows. WSL can be documented as the Linux target at compile time, with any runtime WSL-specific behavior added only when the detector needs to cross the WSL/native-Windows boundary.

## Assessment

Overall readiness: not acceptable.

macOS readiness: acceptable based on this review. The local macOS `cargo check --all-targets` passed, and the macOS-specific code is generally cfg-isolated.

Linux readiness: not acceptable. Production Linux paths look intentional, but at least one test module is structured in a way that should fail non-macOS test compilation.

Native Windows readiness: not acceptable. Several tests are Unix-specific without cfg guards, PATH tests build malformed Windows PATH values, and Windows audio detection relies on a deprecated external tool.

WSL readiness: not acceptable by inheritance from Linux test readiness. I did not find WSL-specific path translation issues in production code, but WSL support should not be considered proven until Linux test compilation and execution are covered in CI.
