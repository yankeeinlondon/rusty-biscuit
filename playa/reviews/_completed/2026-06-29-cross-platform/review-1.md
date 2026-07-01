---
$schema:
  ready: boolean
  macos_ready: boolean
  windows_ready: boolean
  linux_ready: boolean
  agent: string
  created: string
about: "A cross-platform review of the playa package area on 2026-06-29"
author: "prompts/cross-platform.md"
created: "2026-06-29T21:19:52"
agent: "codex/default"
ready: false
macos_ready: true
windows_ready: false
linux_ready: true
---

## Summary

The `playa` package area is generally written with target-aware Rust APIs: paths are represented as `PathBuf`, host player arguments are passed as `OsStr`/`OsString`, OS-specific audio implementations are behind `#[cfg(target_os = ...)]`, and the macOS, Linux, and Windows native audio/ducking dependency slices are separated in `Cargo.toml`.

I did not find Unix socket, named pipe, dynamic-library loading, hard-coded Windows drive paths, shell-script spawning, manual `PATH` parsing, or direct `std::os::unix` imports in the reviewed Rust code. WSL is treated as Linux, which is appropriate for this package because no Windows executable/path translation is used from Linux code.

The main cross-platform risk is the shared temp-file cache used when playing in-memory audio bytes through host players. It has a deterministic temporary filename and then renames that file into the deterministic cache filename. That is fragile under concurrent processes and is especially risky on native Windows because rename/overwrite behavior is stricter when another process has the file open.

Local verification on macOS passed with `just test` for `playa` and `playa-cli` on June 29, 2026. Windows cross-checking was attempted with `cargo check -p playa --target x86_64-pc-windows-msvc --no-default-features` and `cargo check -p playa-cli --target x86_64-pc-windows-msvc --no-default-features`, but this host lacks the Windows C headers/SDK needed by transitive C dependencies (`windows.h`, `stdlib.h`, `ctype.h` failures), so that did not verify the package code.

## Findings

### High: Byte playback temp cache is not concurrency-safe across processes

References:

- `playa/lib/src/playback.rs:424` builds one shared cache directory with `std::env::temp_dir().join("playa")`.
- `playa/lib/src/playback.rs:454` writes byte playback to a content-hash filename.
- `playa/lib/src/playback.rs:468` writes a deterministic `{hash}.tmp`.
- `playa/lib/src/playback.rs:470` renames that deterministic temp path to `{hash}.audio`.
- `playa/lib/src/playback.rs:493` and `playa/lib/src/playback.rs:495` repeat the same pattern in the async path.

Affected targets: macOS, Linux, native Windows, WSL. Native Windows has the sharpest failure mode.

Likely failure mode:

Two processes playing the same byte content can race on the same `.tmp` path. One process may overwrite or remove the other process's temp file before `rename`, or a stale/open handle can make `rename` fail. On Windows, `rename`/replacement is more likely to fail when another process or player still holds a handle. On Unix/WSL, the race can still corrupt the cache creation path or cause spurious IO errors.

Smallest practical fix:

Use a unique temporary file name per writer, then publish to the deterministic cache path with a race-tolerant create/link/rename strategy. For example, write into the same directory using a name containing process id plus a monotonic/random suffix, flush and close the file, then attempt to publish. If the final cache file already exists, delete the unique temp and return the existing cache path. Consider `tempfile::NamedTempFile::new_in(dir)` plus explicit persistence handling, or an internal equivalent that does not rely on a deterministic `.tmp` name. Add a concurrent test that starts multiple threads/processes writing the same bytes and verifies every caller gets a valid cache file.

### Medium: Windows behavior is not directly exercised by path-oriented tests

References:

- `playa/lib/src/playback.rs:688` uses `PathBuf::from("/tmp/test.wav")` for command construction tests.
- `playa/lib/src/playback.rs:774`, `playa/lib/src/playback.rs:866`, `playa/lib/src/playback.rs:892`, `playa/lib/src/playback.rs:952`, and `playa/lib/src/playback.rs:995` assert against the same Unix-style mock path.
- `playa/lib/src/playback.rs:1103` repeats the Unix-style path check in async argument tests.

Affected targets: native Windows primarily; also weakens confidence for macOS/Linux path edge cases.

Likely failure mode:

The implementation passes `PathBuf` directly to `Command`, which is the right approach, so this is a coverage gap rather than a proven runtime bug. The current tests do not exercise spaces, backslashes, drive prefixes, UNC-like paths, or non-UTF-8-ish path handling. A future change could accidentally convert paths to strings or split on `/` without these tests catching it.

Smallest practical fix:

Parameterize command-construction tests with platform-shaped paths. On Windows builds, include paths such as `C:\Users\Example\audio file.wav` and a UNC-shaped path. On Unix builds, include paths with spaces and backslash characters as ordinary filename characters. Keep assertions at the `OsStr`/`OsString` level instead of stringifying paths.

### Medium: Native Windows support could not be verified in this review environment

References:

- `playa/lib/Cargo.toml:73` enables the `windows` crate only for `target_os = "windows"`.
- `playa/lib/src/lib.rs:12` gates `windows_com` behind Windows plus the Windows native feature set.
- `playa/lib/src/sfx_player.rs:613` contains the Windows SFX implementation.
- `playa/lib/src/ducking/windows.rs:1` contains the Windows WASAPI ducking backend.

Affected targets: native Windows.

Likely failure mode:

The code is properly target-gated, but the review could not prove that the Windows feature matrix compiles on an actual Windows toolchain. The local cross-check failed before reaching package diagnostics because this macOS host does not have the Windows C runtime/SDK headers required by transitive C dependencies. This leaves native Windows readiness dependent on CI or a Windows host.

Smallest practical fix:

Add CI jobs that run at least:

- `cargo check -p playa --target x86_64-pc-windows-msvc --features sfx-native-windows,native-playback`
- `cargo check -p playa --target x86_64-pc-windows-msvc --features audio-ducking-windows`
- `cargo check -p playa-cli --target x86_64-pc-windows-msvc`
- `cargo nextest run -p playa --target x86_64-pc-windows-msvc --no-default-features`

Run those on a real Windows runner rather than relying only on macOS cross-compilation.

### Low: macOS ducking diagnostics shell out to `which`

References:

- `playa/cli/src/main.rs:807` runs `std::process::Command::new("which").arg("nowplaying-cli")`.

Affected targets: macOS only, because this branch is reached for the `macos-media-keys` backend.

Likely failure mode:

This is not a native Windows issue because the branch is macOS-specific. It is still less portable and less idiomatic than using a Rust executable lookup because it assumes a Unix helper program is available and behaves as expected.

Smallest practical fix:

Use the existing detection layer, a small `PATH` lookup via `std::env::split_paths`, or the `which` crate instead of spawning `which`.

## Suggestions

Fix the temp-file cache first. It is the only reviewed item that looks like a real cross-platform runtime bug, and it affects every supported target. Use unique writer temp files in the shared cache directory and make final publication idempotent when another process wins the race.

Add Windows CI as a hard gate for the `playa` area. The package has substantial Windows-specific code, but this macOS review could not validate it. Native Windows readiness should be based on a Windows runner, not just source inspection or target-gated compilation from macOS.

Broaden path tests before future playback changes. Keep the current command-construction tests, but add platform-shaped `PathBuf` fixtures so regressions around stringified paths, separators, spaces, and Windows prefixes are caught early.

Keep WSL documented as Linux behavior. There is no current WSL-specific path translation or Windows process interop in `playa`, so no special WSL implementation is needed. The main WSL recommendation is operational: build from the WSL Linux filesystem rather than `/mnt/c/...` to avoid slow filesystem behavior.

Prefer portable executable lookup in CLI diagnostics. Replacing `Command::new("which")` removes a small shell-environment assumption and keeps process spawning limited to actual playback or install workflows.

## Assessment

Based only on cross-platform support, I would not mark the package area fully acceptable yet.

macOS: acceptable. The reviewed platform-specific code is gated and tested locally through the standard `just test` path.

Linux and WSL: acceptable with the same temp-cache caveat. The Linux-specific code is gated, WSL does not need a separate implementation for the current behavior, and no Linux-only IPC/path assumptions were found beyond ordinary Linux audio backends.

Native Windows: not acceptable yet. The source has intentional Windows implementations, but the byte-playback temp cache has Windows-sensitive race behavior and the Windows build/test matrix was not verified in this environment.
