---
name: cross-platform
description: |-
    A review process that looks for indications that source code is NOT appropriately
    cross-platform. Packages in this monorepo are expected to run on:

    1. macOS
    2. Linux
    3. Windows:
        - native
        - WSL
review: "@{{ctx.area}}/reviews/{{ctx.today}}-cross-platform/review-1.md"
favorites: true
start:
    message: "🎛️ starting a **cross-platform** review in the **{{ctx.area}}** package area (_{{ctx.now}}_)"
success:
    stack:
        - when: "frontmatter(review, 'ready') == true"
          action:
              - message: "✅ **cross-platform** review in **{{ctx.area}}** completed successfully (_all platforms are deemed to be in good shape_). Review took: {{timing.duration}}."
              - effect: small-group-cheer
        - when: "frontmatter(review, 'ready') != true"
          action:
              - message: "⚠️ **cross-platform** review completed successfully but was deemed to have code issues that need addressing: `{{review}}`"
              - effect: phase-jump-3
failure:
    message: "💥 failed to complete the **cross-platform** review in **{{ctx.area}}**. Error: {{err.msg}}."
    effect: sad-trombone
---
# Ensuring Cross Platform Support

## Context

Your task is to review the "{{ ctx.area }}" package area for signs that it is not upholding the requirement that this Rust code base work across:

1. macOS
2. Linux
3. Windows:
    - native
    - WSL

> **Note:** you are running this review on host which is running the {{ctx.os}} OS. That doesn't mean you should focus more on that OS. All OS's are given equal weight and importance. Ironically it is likely that the code base is actually in better shape for the {{ctx.os}} OS simply because this host may be being used as the primary development and testing platform too.

## Key areas to examine

### 1. Hard‑coded file paths and separators

- **Path separators differ by OS.** Windows uses `\\` as a directory separator, whereas Unix‑like systems (macOS, Linux, WSL) use `/`. On Windows, `PathBuf` treats both `\\` and `/` as separators, but on Linux a backslash is treated as a normal character, so a path like `foo\\bar` is considered a single component [oai_citation:0‡udoprog.github.io](https://udoprog.github.io/rust/2017-11-05/portability-concerns-with-path.html#:~:text=On%20Windows%2C%20it%20would%20give,this%20output). The example below passes an equality check on Windows but fails on Linux because backslashes are valid filename characters on Unix:

    ```rust
    let path_a = PathBuf::from("C:\\path/to\\example/thingy");
    let path_b = PathBuf::from("C:\\path\\to\\example\\thingy");
    assert_eq!(path_a, path_b); // passes on Windows, fails on Linux
    ```

- **String concatenation vs. `Path` API.** Avoid building paths with string concatenation, `format!`, or macros like `concat!` that hard‑code `/` or `\\`. A GitHub issue notes that `include_*` macro paths are not assembled by `Path::join`, and concatenating `OUT_DIR` with `"/myfile"` or `std::path::MAIN_SEPARATOR` can break on some platforms [oai_citation:1‡github.com](https://github.com/rust-lang/rust/issues/75075). Prefer `std::path::Path`, `PathBuf::push`, and `Path::join` for runtime paths [oai_citation:2‡udoprog.github.io](https://udoprog.github.io/rust/2017-11-05/portability-concerns-with-path.html#:~:text=Portable%20paths). For compile-time embedded files, check whether the code uses `include_str!`, `include_bytes!`, or generated paths; build scripts may need to emit a target-specific path with `cargo:rustc-env`.
- **Relative vs. absolute paths.** Windows has multiple drive roots (`C:\`, `D:\`, UNC roots), whereas Unix systems have a single `/` root. Code that assumes an absolute path starts with `/`, strips a leading slash, or parses drive letters by hand may not work on Windows. Use `Path`/`Component` APIs, relative paths, `env::current_dir()`, or explicit configuration instead of string-prefix checks [oai_citation:3‡udoprog.github.io](https://udoprog.github.io/rust/2017-11-05/portability-concerns-with-path.html#:~:text=Another%20major%20difference%20is%20how,Linux%20only%20has%20one%3A).
- **Detection clues.** Search for `"/"`, `"\\\\"`, `format!(.*"/`, `format!(.*\\\\`, `split('/')`, `replace("\\\\", "/")`, `strip_prefix("/")`, `starts_with("/")`, `MAIN_SEPARATOR`, `include_str!`, `include_bytes!`, and code that converts `Path`/`OsStr` to UTF-8 with `to_str().unwrap()` or `display().to_string()` before doing path logic.

### 2. File‑system case sensitivity and reserved names

- **Case sensitivity.** File and directory names are normally case-sensitive on Linux but commonly case-insensitive on Windows and the default macOS file system; both Windows and macOS can also be configured with case-sensitive file systems. Accessing `LE72020252003106EDC00_B1.tif` works on case-insensitive file systems but fails on Linux if the actual file name is `LE72020252003106EDC00_B1.TIF` [oai_citation:4‡blog.rtwilson.com](https://blog.rtwilson.com/reminder-about-cross-platform-case-sensitivity-differences/#:~:text=Basically%3A%20file%20names%2Fpaths%20are%20case,on%20OS%20X%20or%20Windows) [oai_citation:5‡blog.rtwilson.com](https://blog.rtwilson.com/reminder-about-cross-platform-case-sensitivity-differences/#:~:text=It%20was%20purely%20down%20to,as%20different%20files). Ensure that checked-in fixture references match actual file-name case exactly, and flag generated names that differ only by case.
- **Reserved names and characters.** Windows file names cannot contain `< > : " / \ | ? *`, ASCII control characters, or reserved device names such as `CON`, `PRN`, `AUX`, `NUL`, `COM1`-`COM9`, and `LPT1`-`LPT9`; these names are reserved even when an extension is appended [oai_citation:6‡learn.microsoft.com](https://learn.microsoft.com/en-us/windows/win32/fileio/naming-a-file). When code derives file names from user input, URLs, model names, branch names, timestamps, or test names, verify that it sanitizes for Windows.
- **Path length limits.** Many modern Windows applications can exceed `MAX_PATH` only when long paths are enabled in the system and the executable is marked `longPathAware`; otherwise classic Win32 path limits can still appear [oai_citation:7‡learn.microsoft.com](https://learn.microsoft.com/en-us/windows/win32/fileio/maximum-file-path-limitation). Flag deeply nested generated paths, repeated temp-directory nesting, and tests that create long fixture names.
- **Detection clues.** Search for file-name generation from arbitrary strings, timestamps containing `:`, slug functions, test fixture paths whose case differs from on-disk names, and code that assumes `rename`, `remove_file`, or temp-file behavior is identical while handles are still open.

### 3. Environment variables and OS‑specific API differences

- **Environment variable case on Windows.** Windows environment variable names are case-insensitive. Code that iterates `std::env::vars()` and checks for a specific spelling such as `"path"` can fail or behave inconsistently; use `env::var_os("PATH")`/`env::var("PATH")` for direct lookups, or compare names with ASCII-case-insensitive logic when iterating.
- **PATH separators.** The `PATH` variable is separated by `:` on Unix and `;` on Windows. Use `std::env::split_paths` and `join_paths` to parse or construct `PATH` rather than splitting on `:` or `;`.
- **Conditional compilation.** Identify any usage of `std::os::unix`, `std::os::linux`, `std::os::macos`, `std::os::windows`, `libc`, `nix`, `winapi`, `windows`, or raw FFI. Ensure calls are protected by appropriate `#[cfg(...)]` attributes and that each supported target has a working implementation or a clear unsupported error. Remember that `#[cfg(unix)]` and `#[cfg(windows)]` test the **target** OS, not the build host, which matters for build scripts and cross-compilation.
- **Detection clues.** Search for `env::vars`, `env::vars_os`, `PATH`, `PathExt`, `MetadataExt`, `PermissionsExt`, `CommandExt`, `std::os::`, `libc::`, `nix::`, `winapi::`, `windows::`, `cfg!(target_os`, `#[cfg(unix)]`, and `#[cfg(windows)]`.

### 4. Symlinks and file operations

- **Symlink creation.** The functions `std::os::unix::fs::symlink` and `std::os::windows::fs::symlink_file` / `symlink_dir` are platform-specific. On Windows, creating symlinks may require Developer Mode or elevated privileges depending on OS policy and API flags. Consider providing fallbacks, skipping symlink tests with an explicit reason, or documenting prerequisites.
- **Canonicalization and normalization.** Different OSes canonicalize paths differently. On Windows, two paths with different separators can canonicalize to the same path, while on Linux they remain distinct. `std::fs::canonicalize()` can also fail when the path does not exist and may return extended-length Windows paths. Use it intentionally, and avoid comparing unnormalized display strings.
- **Open-handle semantics.** Windows is stricter than Unix about deleting, renaming, or overwriting files that are still open. Tests that pass on Unix can fail on Windows if a `File`, temp directory, mmap, or spawned child still holds a handle. Verify that handles are dropped before cleanup assertions.
- **Detection clues.** Search for `symlink`, `canonicalize`, `remove_file`, `remove_dir_all`, `rename`, `NamedTempFile`, `TempDir`, `persist`, and tests that remove or rename files immediately after opening them.

### 5. Newlines and text files

- **CRLF vs. LF.** Text files may use `CRLF` (`\r\n`) on Windows and `LF` (`\n`) on Unix. Avoid assuming one newline style in parsers, snapshots, golden files, or generated text unless the format requires it. Prefer tolerant reading (`lines()`, parser-level normalization, or explicit normalization at file boundaries) and intentional writing (for example, always LF for repository files, CRLF only where required by a Windows-specific format).
- **Snapshot and fixture tests.** Tests that compare full strings often fail across OSes because line endings come from checked-out files, command output, or spawned tools. Normalize line endings in assertions when the behavior under test is not the newline convention itself.
- **Detection clues.** Search for `"\n"`, `"\r\n"`, `.lines()`, `read_to_string`, snapshot tests, `insta`, golden fixtures, and scripts generated for `.sh`, `.ps1`, `.cmd`, or `.bat`.

### 6. Dynamic libraries and plugins

- **Shared library extensions and prefixes.** Dynamic libraries use `.dll` on Windows, `.dylib` on macOS, and `.so` on Linux. Unix-like platforms commonly use a `lib` prefix, while Windows DLL names often do not. If the project loads plugins or dynamic libraries at runtime, ensure that names are selected with target-aware logic rather than hard-coded suffixes.
- **Linker behavior.** If build scripts produce or link to shared libraries, check for OS-specific flags such as `-ldl` on Linux, framework links on macOS, MSVC vs. GNU Windows differences, and conditional `cargo:rustc-link-*` output. Build scripts run on the host but emit instructions for the target, so target triples matter.
- **Detection clues.** Search for `.so`, `.dylib`, `.dll`, `libloading`, `dylib`, `cdylib`, `cargo:rustc-link`, `build.rs`, `target_os`, `target_env`, and `target_family`.

### 7. WSL‑specific considerations

- **File‑system performance.** Accessing code on an NTFS partition from WSL can cause extremely slow builds. Benchmarks show that compiling Rust on WSL from an NTFS location can be much slower than compiling within the WSL ext4 file system [oai_citation:8‡markentier.tech](https://markentier.tech/posts/2022/01/speedy-rust-builds-under-wsl2/#:~:text=Developing%20on%20Windows%20%26%20Linux%3F,build%20times%3F%20Then%20compile%20elsewhere). Suggest that users work with the project in a Linux file system, such as under the WSL home directory, to avoid sluggish builds [oai_citation:9‡markentier.tech](https://markentier.tech/posts/2022/01/speedy-rust-builds-under-wsl2/#:~:text=,run%20the%20program%28s).
- **Path translation.** WSL uses mount points like `/mnt/c/...` for Windows paths, while native Windows tools expect paths like `C:\...`. If code passes paths between WSL and Windows executables, ensure that translation is explicit and tested rather than relying on string replacement. Consider `wslpath` for command-line integration or a dedicated path-translation crate when library code needs this behavior.
- **WSL detection.** WSL is Linux from Rust's target perspective, so `cfg!(target_os = "linux")` is true. Runtime checks such as `/proc/version`, `/proc/sys/kernel/osrelease`, `WSL_DISTRO_NAME`, or `WSL_INTEROP` may be necessary only when behavior genuinely differs under WSL.
- **Detection clues.** Search for `/mnt/c`, `C:\\`, `wslpath`, `WSLENV`, `WSL_DISTRO_NAME`, `WSL_INTEROP`, Windows executable invocations from Linux code, and tests that assume native Linux filesystem performance or path syntax.


### 8. Sockets and local IPC

- **Unix domain sockets are not a single portability story.** Unix domain sockets are the normal local IPC primitive on macOS, Linux, and WSL. Native Windows supports AF_UNIX on Windows 10 build 17063 and later, but Windows support is not identical to Unix: older Windows releases do not support it, some Unix-specific capabilities such as `socketpair` and ancillary data are not supported by Windows AF_UNIX, and Rust's `std::os::unix::net` module is still Unix-only. Review Windows socket support through the actual crate or API in use, not just through the OS feature headline.
- **Named pipes are a different API, not a drop-in socket spelling.** Windows named pipes use names such as `\\.\pipe\name` and support message or byte modes, overlapped I/O, impersonation/security descriptors, and different connection semantics. Unix FIFOs are also called named pipes but are not equivalent to Windows named pipes. Supporting both Unix sockets and Windows named pipes usually requires an abstraction boundary, separate connection setup, separate test fixtures, and platform-specific error handling.
- **Modern strategy for Rust projects.** Prefer TCP loopback (`127.0.0.1` / `[::1]`) when the security model and port allocation are acceptable, because it is the most portable transport and easiest to exercise in CI. For local-only IPC where filesystem permissions, path-based discovery, or avoiding TCP ports matters, use Unix domain sockets on Unix targets and either Windows AF_UNIX through a crate that explicitly supports it or Windows named pipes through a Windows-specific crate/module. Do not assume `std::os::unix::net::{UnixListener, UnixStream}` compiles on Windows.
- **Socket-path constraints.** Unix socket addresses are often path-like and are subject to platform-specific length limits. Linux also has abstract namespace sockets; do not treat those as portable to macOS, and verify Windows behavior through the exact API or crate in use before relying on it. A socket file may need cleanup before bind on pathname-based implementations, while TCP ports require race-free port allocation. Tests should use temporary directories and unique names, and they should clean up stale socket files without deleting unrelated paths.
- **Detect socket-based code.** Search for `UnixListener`, `UnixStream`, `UnixDatagram`, `tokio::net::Unix*`, `async_std::os::unix::net`, `interprocess`, `socket2`, `AF_UNIX`, `SOCK_STREAM`, `TcpListener`, `TcpStream`, `UdpSocket`, `localhost`, `127.0.0.1`, `[::1]`, `.sock`, `.socket`, `bind`, `listen`, `connect`, and `accept`.
- **Detect named-pipe code.** Search for `named_pipe`, `NamedPipe`, `tokio::net::windows::named_pipe`, `\\.\pipe\`, `CreateNamedPipe`, `ConnectNamedPipe`, `PIPE_ACCESS`, `PIPE_TYPE`, `overlapped`, `miow`, `uds_windows`, and `interprocess::local_socket`.
- **Telltale test isolation problems.** Flag tests that bind fixed TCP ports, use fixed socket or pipe names, share global temp paths, assume `/tmp`, sleep instead of waiting for readiness, run socket tests concurrently without unique addresses, leave Unix socket files behind, or skip Windows with `#[cfg(unix)]` without a Windows equivalent test. Also flag tests that claim Windows coverage but only exercise loopback TCP while production uses Unix sockets or named pipes.
- **Telltale implementation problems.** Flag source that imports `std::os::unix::net` outside a Unix-only module, builds socket paths with string concatenation, hard-codes `/tmp/app.sock`, assumes Linux abstract sockets, treats named-pipe paths as filesystem paths, maps all connection errors to one generic retry path, or has `#[cfg(windows)]` stubs that return `unimplemented!`, `todo!`, or a permanent "unsupported" error despite Windows being a target platform.

## Task

Write a review to {{review}}, including:

- summary overview of what you found (add to a `## Summary` section)
- recommendations on how this should be fixed/improved (add to a `## Suggestions` section)
- assess whether this code base is acceptable based solely on its ability to support all of the target platforms

While performing the review review the Rust project for **cross‑platform compatibility issues**.  The project is intended to run on **macOS**, **Linux**, **native Windows**, and **Windows Subsystem for Linux (WSL)**.  When reviewing the code, please:

1. **Search for platform-specific implementation code.** Include `std::os::*`, conditional compilation, raw FFI, OS-specific crates, build scripts, process spawning, filesystem behavior, dynamic library loading, sockets, and local IPC. Verify that every target platform has either a working implementation or a documented, intentional unsupported path.

2. **Inspect tests as carefully as implementation code.** Cross-platform support is weak when tests only pass because they run serially on one OS, use fixed ports or file names, depend on `/tmp`, assume a shell, depend on line endings, require symlink privileges, or skip entire platforms without equivalent coverage. Identify tests that should be parameterized, isolated with temp directories and unique names, or split by target-specific behavior.

3. **Distinguish portability bugs from intentional target differences.** It is acceptable for code to use `#[cfg]` when the platform behavior genuinely differs. It is not acceptable for a supported platform to compile only through `todo!`, `unimplemented!`, a permanent "unsupported" error, or an untested fallback that cannot perform the advertised behavior.

4. **Report with code references and remediation.** For each issue, reference the relevant file and line, name the affected target(s), explain the likely failure mode, and suggest the smallest practical fix. Examples include using `Path::join`, `split_paths`, target-specific dynamic-library naming, portable temp-file handling, unique socket names, loopback TCP, Windows named pipes, or a tested abstraction over platform-specific IPC.

### **Deliverable:** 

Save a structured report to {{review}}. 

- Include `## Summary`, findings ordered by severity, `## Suggestions`, and a final assessment of whether the package area appears acceptable for macOS, Linux, native Windows, and WSL based only on cross-platform support.
- Save to the review file's Frontmatter:
    - `$schema` as `{ ready: boolean, macos_ready: boolean, windows_ready: boolean, linux_ready: boolean, agent: string, created: string }`
    - `about` as 'A cross-platform review of the {{ctx.area}} package area on {{ctx.today}}'
    - `author` as 'prompts/cross-platform.md'
    - `created` as '{{ctx.now}}'
    - `agent` as '{{ctx.agent}}/{{ctx.model}}'
    - `ready` as a boolean flag which indicates whether this code "acceptable" based solely on its ability to support all target platforms
    - `macos_ready` as a boolean flag which indicates whether the code base is "acceptable" based solely on its ability to support the **macOS** platform
    - `windows_ready` as a boolean flag which indicates whether the code base is "acceptable" based solely on its ability to support the **windows** platform
    - `linux_ready` as a boolean flag which indicates whether the code base is "acceptable" based solely on its ability to support the **linux** platform
