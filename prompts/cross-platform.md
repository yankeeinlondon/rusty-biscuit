----
name: cross-platform
description: |-
    A review process that looks for indications that source code is NOT appropriately
    cross-platform. Packages in this monorepo are expected to run on:

    1. macOS
    2. Linux
    3. Windows:
        - native
        - WSL
----
# Ensuring Cross Platform Support

## Key areas to examine

### 1. Hard‑coded file paths and separators

- **Path separators differ by OS.**  Windows uses `\\` as a directory separator, whereas Unix‑like systems (macOS, Linux, WSL) use `/`.  On Windows, `PathBuf` treats both `\\` and `/` as separators, but on Linux a backslash is treated as a normal character, so a path like `foo\\bar` is considered a single component [oai_citation:0‡udoprog.github.io](https://udoprog.github.io/rust/2017-11-05/portability-concerns-with-path.html#:~:text=On%20Windows%2C%20it%20would%20give,this%20output).  The example below passes an equality check on Windows but fails on Linux because backslashes are valid filename characters on Unix:
  ```rust
  let path_a = PathBuf::from("C:\\path/to\\example/thingy");
  let path_b = PathBuf::from("C:\\path\\to\\example\\thingy");
  assert_eq!(path_a, path_b); // passes on Windows, fails on Linux
  ```
- **String concatenation vs. `Path` API.**  Avoid building paths with string concatenation or macros like `concat!` that hard‑code `/` or `\\`.  A GitHub issue notes that there is no way to call `include_bytes!` with a platform‑agnostic separator, and concatenating `OUT_DIR` with `"/myfile"` or `std::path::MAIN_SEPARATOR` can break on some platforms [oai_citation:1‡github.com](https://github.com/rust-lang/rust/issues/75075).  Always use `std::path::Path`, `PathBuf::push` and `Path::join` to assemble paths component by component [oai_citation:2‡udoprog.github.io](https://udoprog.github.io/rust/2017-11-05/portability-concerns-with-path.html#:~:text=Portable%20paths).
- **Relative vs absolute paths.**  Windows has multiple drive roots (C:\\, D:\\…), whereas Unix systems have a single `/` root.  Code that assumes an absolute path starting with `/` may not work on Windows.  Use relative paths and combine them with `env::current_dir()` or configuration to determine an absolute path [oai_citation:3‡udoprog.github.io](https://udoprog.github.io/rust/2017-11-05/portability-concerns-with-path.html#:~:text=Another%20major%20difference%20is%20how,Linux%20only%20has%20one%3A).

### 2. File‑system case sensitivity and reserved names

- **Case sensitivity.**  File and directory names are case‑sensitive on Linux but case‑insensitive on Windows and on the default macOS file system [oai_citation:4‡blog.rtwilson.com](https://blog.rtwilson.com/reminder-about-cross-platform-case-sensitivity-differences/#:~:text=Basically%3A%20file%20names%2Fpaths%20are%20case,on%20OS%20X%20or%20Windows).  Accessing `LE72020252003106EDC00_B1.tif` works on Windows/macOS but fails on Linux if the actual file name is `LE72020252003106EDC00_B1.TIF` [oai_citation:5‡blog.rtwilson.com](https://blog.rtwilson.com/reminder-about-cross-platform-case-sensitivity-differences/#:~:text=It%20was%20purely%20down%20to,as%20different%20files).  Ensure that references to file names match the actual case, and avoid relying on different case variants.
- **Reserved names on Windows.**  Windows prohibits certain file names (e.g., `CON`, `NUL`, `PRN`) and disallows characters such as `<`, `>`, `?`, `*`, `:` in filenames.  When generating files or using `tempfile`, ensure names are valid on Windows and avoid these characters.
- **Path length limits.**  Classic Windows APIs limit paths to `MAX_PATH` (260 characters) unless the executable’s manifest sets `longPathAware` to true.  Consider adding a manifest or avoiding deep directory nesting if your code constructs long paths.

### 3. Environment variables and OS‑specific API differences

- **Environment variable case on Windows.**  A Rust issue highlights that `std::env::vars()` on Windows returns all environment variable names in uppercase [oai_citation:6‡github.com](https://github.com/rust-lang/rust/issues/85242#:~:text=%3Cscript%20type%3D%22application%2Fjson%22%20data,nightly%5Cr%5CnLLVM%20version) because Windows treats environment variable names as case‑insensitive.  Code that checks for lower‑case names (e.g., `"path"`) may fail on Windows.  Use `env::var_os` or normalize the key to one case when comparing.
- **PATH separators.**  The `PATH` variable is separated by `:` on Unix and `;` on Windows.  Use `std::env::split_paths` and `join_paths` to parse or construct `PATH` rather than splitting on `:`.
- **Conditional compilation.**  Identify any usage of `std::os::unix`, `std::os::linux`, `std::os::macos`, or `std::os::windows` APIs.  These modules expose OS‑specific functions such as `MetadataExt` or `symlink` that are not available on other platforms.  Ensure that calls are protected by appropriate `#[cfg(target_os = "...")]` attributes and that a suitable alternative exists for each supported platform.  The Rust language reference notes that `#[cfg(unix)]` and `#[cfg(windows)]` test the **target** OS, not the host [oai_citation:7‡github.com](https://github.com/rust-lang/rust/issues/75075), which matters for build scripts and cross‑compilation.

### 4. Symlinks and file operations

- **Symlink creation.**  The functions `std::os::unix::fs::symlink` and `std::os::windows::fs::symlink_file` / `symlink_dir` are platform‑specific.  On Windows, creating symlinks may require elevated privileges or developer mode.  Consider providing fallbacks (copying files) or instructing users about prerequisites.
- **Canonicalization and normalization.**  Different OSes canonicalize paths differently.  On Windows, two paths with different separators can canonicalize to the same path, while on Linux they remain distinct.  When comparing paths, use `std::fs::canonicalize()` or normalize separators across platforms.

### 5. Newlines and text files

- **CRLF vs LF.**  Text files may use `CRLF` (`\r\n`) on Windows and `LF` (`\n`) on Unix.  Avoid hard‑coding newline characters; instead use `std::env::consts::OS` to determine the OS, or use libraries like `lines()` which handle both.  When reading/writing text across platforms (e.g., generating scripts), normalize line endings or specify the desired mode.

### 6. Dynamic libraries and plugins

- **Shared library extensions.**  Libraries use `.dll` on Windows, `.dylib` on macOS, and `.so` on Linux.  If the project loads plugins or dynamic libraries at runtime, ensure that filenames are determined based on `cfg!(target_os)`.
- **Linker behaviour.**  If build scripts produce or link to shared libraries, check for OS‑specific flags (e.g., `-ldl` on Linux) and ensure that `build.rs` uses conditional logic to apply them only on the relevant platforms.

### 7. WSL‑specific considerations

- **File‑system performance.**  Accessing code on an NTFS partition from WSL can cause extremely slow builds.  Benchmarks show that compiling Rust on WSL from an NTFS location can be four times slower than compiling within the WSL ext4 file system [oai_citation:8‡markentier.tech](https://markentier.tech/posts/2022/01/speedy-rust-builds-under-wsl2/#:~:text=Developing%20on%20Windows%20%26%20Linux%3F,build%20times%3F%20Then%20compile%20elsewhere).  Suggest that users work with the project in a Linux file system (e.g., copy the project into `~/tmp` in WSL before building) to avoid sluggish performance [oai_citation:9‡markentier.tech](https://markentier.tech/posts/2022/01/speedy-rust-builds-under-wsl2/#:~:text=,run%20the%20program%28s).
- **Path translation.**  WSL uses mount points like `/mnt/c/...` for Windows paths.  If the code manipulates Windows paths, ensure that translation to WSL paths is handled correctly, perhaps using a crate such as `cross-path` or by detecting `WSLENV`.

## Task

Review the Rust project for **cross‑platform compatibility issues**.  The project is intended to run on **macOS**, **Linux**, **native Windows**, and **Windows Subsystem for Linux (WSL)**.  When reviewing the code, please:

1. **Search for any OS‑specific code** (e.g., use of `std::os::unix`, `std::os::windows`, `libc`, or external C APIs) and verify that it is guarded by appropriate `#[cfg(...)]` attributes.  Ensure that an equivalent implementation exists for each of the four targets.

2. **Inspect all file‑system interactions**, including file creation, reading, symlink handling and temporary directories:
   - Avoid hard‑coded separators (`/` or `\\`).  Use `Path`/`PathBuf` and methods like `join` or `push` to build paths [oai_citation:10‡udoprog.github.io](https://udoprog.github.io/rust/2017-11-05/portability-concerns-with-path.html#:~:text=On%20Windows%2C%20it%20would%20give,this%20output) [oai_citation:11‡udoprog.github.io](https://udoprog.github.io/rust/2017-11-05/portability-concerns-with-path.html#:~:text=Portable%20paths).
   - Confirm that the code does not rely on backslash and forward‑slash being interchangeable; `\\` is a valid filename character on Unix and may change path semantics.
   - Use relative paths where possible and avoid assuming a single root (`/`), because Windows has multiple drive roots [oai_citation:12‡udoprog.github.io](https://udoprog.github.io/rust/2017-11-05/portability-concerns-with-path.html#:~:text=Another%20major%20difference%20is%20how,Linux%20only%20has%20one%3A).
   - Check whether file names differ only by case; Linux treats `foo.txt` and `Foo.txt` as distinct files while Windows/macOS do not [oai_citation:13‡blog.rtwilson.com](https://blog.rtwilson.com/reminder-about-cross-platform-case-sensitivity-differences/#:~:text=Basically%3A%20file%20names%2Fpaths%20are%20case,on%20OS%20X%20or%20Windows).
   - Ensure generated filenames avoid reserved characters (`< > : " / \\ | ? *`) and reserved names (e.g., `CON`, `NUL`) on Windows.
   - Be aware of the `MAX_PATH` limit on Windows.  If code constructs deep directory structures, consider using a manifest with `longPathAware` enabled or flattening the structure.

3. **Evaluate environment variable usage**:
   - Ensure that environment variable names are compared in a case‑insensitive way when appropriate, because Rust’s `std::env::vars()` on Windows returns variable names in uppercase [oai_citation:14‡github.com](https://github.com/rust-lang/rust/issues/85242#:~:text=%3Cscript%20type%3D%22application%2Fjson%22%20data,nightly%5Cr%5CnLLVM%20version).
   - Use `split_paths` and `join_paths` for parsing or constructing `PATH` rather than splitting on `:` or `;`.
   - Confirm that build scripts using `include_*` macros do not assume a single separator; if necessary, use build scripts to compute the path and embed it via `cargo:rustc-env` [oai_citation:15‡github.com](https://github.com/rust-lang/rust/issues/75075).

4. **Check dynamic library loading or plugin handling**.  Ensure that file extensions (`.so`, `.dylib`, `.dll`) and linker flags are selected based on the target OS.
5. **Review concurrency and process spawning code** to avoid Unix‑specific functions such as `fork()`.  Use `std::process::Command` and other cross‑platform abstractions.
6. **Consider WSL performance.**  Recommend that developers build the project within the WSL file system rather than on an NTFS mount to avoid slow compilation [oai_citation:16‡markentier.tech](https://markentier.tech/posts/2022/01/speedy-rust-builds-under-wsl2/#:~:text=Developing%20on%20Windows%20%26%20Linux%3F,build%20times%3F%20Then%20compile%20elsewhere), and ensure any path conversions from Windows to WSL use appropriate APIs or crates.
**Deliverable:** Provide a structured report summarizing any potential cross‑platform issues found, referencing the code and explaining why the issue could cause problems on one of the target platforms.  Offer suggestions for remediation (e.g., use `Path::join` instead of string concatenation, add `#[cfg]` attributes, normalize case, or use cross‑platform crates like `relative-path`).
