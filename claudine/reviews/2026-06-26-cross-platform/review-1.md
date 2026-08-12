# Cross-Platform Review: Claudine

Date: 2026-06-26

Scope reviewed:

- `claudine/lib`
- `claudine/cli`
- `claudine/contract`
- `claudine/rendezvous/{core,client,daemon}`

Targets considered:

- macOS
- Linux
- Windows native
- WSL

## Summary

Claudine already has several good cross-platform patterns: most paths are built with `Path`/`PathBuf`, provider wrapper process spawning has explicit Unix and Windows branches, PATH probing uses `which` or `std::env::split_paths`, and rendezvous client IPC has separate Unix Domain Socket and Windows named-pipe code.

The main gaps are concentrated in:

1. the rendezvous daemon server, which is still Unix-socket-only despite being a workspace package;
2. resource linking, where production symlink creation is Unix-only and reports an error on Windows;
3. the contract adapter, which treats `HOME` as the only home variable and does not establish Windows home/profile variables for child CLIs;
4. path rendering and server-name derivation helpers that assume Unix-style path text in places where Windows paths are likely;
5. test coverage skew: many CLI integration tests are `#[cfg(unix)]`, so Windows regressions can compile unnoticed unless CI has explicit Windows jobs.

I attempted `cargo check -p rendezvous-daemon --target x86_64-pc-windows-gnu --color=never`. The command spent over 60 seconds compiling dependencies and was stopped before reaching the daemon source, so this review relies on static source evidence for the daemon Windows issue.

## Findings

### 1. High: `rendezvous-daemon` appears not to compile or run on native Windows

References:

- `claudine/rendezvous/daemon/src/server.rs:19`
- `claudine/rendezvous/daemon/src/server.rs:147`
- `claudine/rendezvous/daemon/src/lib.rs:24`
- `claudine/rendezvous/daemon/Cargo.toml:12`

The daemon library imports `tokio::net::UnixListener` and `tokio_stream::wrappers::UnixListenerStream` at module scope, and the public API exports `spawn_uds_server`. The package is a workspace member, with no manifest-level Unix target gate.

Why this matters:

- `tokio::net::UnixListener` is Unix-only. Native Windows builds of this package are likely to fail once compilation reaches this module.
- The client and core already define a Windows named-pipe contract, so the daemon side is the missing half of that cross-platform story.
- WSL is fine because it is Linux and supports Unix sockets, but that does not satisfy native Windows support.

Recommendation:

- Split the daemon local IPC server into platform modules, for example `server/unix.rs` and `server/windows.rs`.
- Keep the current Unix implementation behind `#[cfg(unix)]`.
- Add a Windows named-pipe tonic server path matching `rendezvous-client::connect_named_pipe`.
- Rename the public API from `spawn_uds_server` to a neutral `spawn_local_server`, or export a platform-neutral wrapper that dispatches to `spawn_uds_server` / `spawn_named_pipe_server`.
- Add `cargo check -p rendezvous-daemon --target x86_64-pc-windows-msvc` to CI, even if full runtime tests remain Windows-only or manual.

### 2. High: shared-resource linking cannot create links on native Windows

References:

- `claudine/lib/src/linking/symlink.rs:84`
- `claudine/lib/src/linking/symlink.rs:117`
- `claudine/lib/src/linking/symlink.rs:166`
- `claudine/cli/src/commands/wrap/repo_home.rs:397`

`create_resource_link` uses `std::os::unix::fs::symlink` and returns `"symlink creation is only supported on Unix"` for every non-Unix target. This is production linking behavior, not just test code. In contrast, `repo_home.rs` already has a more portable pattern: try `std::os::windows::fs::symlink_file` on Windows and fall back to copying.

Why this matters:

- `claudine skills`, `claudine commands`, `claudine agents`, or `claudine sync --fix` workflows that rely on linking will fail on native Windows.
- Windows symlink creation may require Developer Mode or privileges, so even `std::os::windows::fs::symlink_dir` is not enough by itself.
- WSL can create Unix symlinks, but links created inside WSL can behave poorly when accessed by native Windows tools.

Recommendation:

- Add a Windows branch using `std::os::windows::fs::symlink_dir` for resource directories.
- If symlink creation fails with permission-related errors, fall back to copy/sync semantics and report degraded portability clearly.
- Reuse or extract the `link_or_copy_file` pattern from `repo_home.rs` into a shared helper, extended for directories.
- Update link reports to distinguish `Linked`, `Copied`, `AlreadyLinked`, and `Skipped`.
- Add Windows tests for the fallback path that do not require symlink privileges.

### 3. Medium: contract sessions set only `HOME`, not Windows home/profile variables

References:

- `claudine/contract/src/adapter.rs:158`
- `claudine/contract/src/adapter.rs:168`
- `claudine/contract/src/session.rs:75`
- `claudine/contract/src/home.rs:26`

The contract adapter reads the real home from `EnvSource::get("HOME")` and injects only `HOME` into the child environment. The baseline env allowlist also omits `USERPROFILE`, `APPDATA`, `LOCALAPPDATA`, `HOMEDRIVE`, and `HOMEPATH`.

Why this matters:

- Native Windows tools commonly resolve home/config locations from `USERPROFILE`, `APPDATA`, or `LOCALAPPDATA`, not just `HOME`.
- A Windows provider CLI may ignore the shadow `HOME` and read the user's real profile if inherited variables are present elsewhere, or may fail to find credentials if they are not present.
- Environment variable names are case-insensitive on Windows, but `EnvSource::get("HOME")` and the allowlist are exact-name oriented.

Recommendation:

- Resolve the real home with a cross-platform helper, preferably `dirs::home_dir()` or a helper in `biscuit-file`/`sniff` if the project has one.
- When spawning on Windows, set `HOME`, `USERPROFILE`, `APPDATA`, and `LOCALAPPDATA` to shadow locations. If a provider expects different subtrees, create them inside the shadow tree.
- Normalize environment lookup on Windows so `home`, `Home`, and `HOME` cannot diverge.
- Add contract tests with Windows-shaped environment maps to assert that the child environment never points at the caller's real profile.

### 4. Medium: some path helpers parse Windows-looking text with host-native semantics

References:

- `claudine/lib/src/mcp/types.rs:397`
- `claudine/lib/src/mcp/types.rs:420`
- `claudine/lib/src/stream/path_link.rs:34`
- `claudine/lib/src/prompt_reporting/system_prompt.rs:85`

`derive_launcher_aware_name` uses `Path::new(command).components().next_back()` and later `arg.rsplit('/')`. On Unix and WSL, a Windows path such as `C:\Tools\node.exe` is one normal component, because backslash is a valid filename character. `format_file_link` similarly treats `raw` via host-native `Path::new(raw)` and only recognizes absolute paths according to the current target. `render_system_prompt_summary` builds links with `format!("file://{}", absolute.display())`, which is not a robust Windows file URL form.

Why this matters:

- Importing MCP config that contains Windows command paths from WSL or from synced config may produce poor names or IDs.
- Windows absolute paths can be rendered as plain escaped text instead of OSC8 links when processed under WSL/Linux.
- `file://C:\...`-style links are not portable URL syntax and may not open correctly from terminals.

Recommendation:

- For command/script name extraction from config text, handle both `/` and `\` separators deliberately instead of relying only on `Path` components.
- For URLs, use `url::Url::from_file_path` where available rather than formatting `file://` manually.
- Consider a small `PathText` helper for provider config fields that are path-like strings but may come from another OS.
- Add tests for Windows-looking strings on Unix/WSL and native Windows paths on Windows.

### 5. Medium: `relative_path` has Unix-only invariants and no Windows drive-prefix handling

References:

- `claudine/lib/src/linking/symlink.rs:166`
- `claudine/lib/src/linking/symlink.rs:167`
- `claudine/lib/src/linking/symlink.rs:181`

`relative_path` compares native path components and constructs `..` segments, but its absolute-path debug assertions are Unix-only. On Windows, paths on different drives or with different prefixes cannot be represented as a relative path safely. Today this is mostly hidden because symlink creation is Unix-only, but it becomes important if Windows linking is added.

Why this matters:

- A repo-scope relative link from `C:\repo\...` to `D:\shared\...` cannot be represented with `..`.
- Windows prefix components (`C:`, UNC roots, verbatim paths) need explicit treatment.

Recommendation:

- Before enabling Windows symlinks, make `relative_path` return `Result<PathBuf>` and detect incompatible prefixes.
- Use absolute symlink targets when relative targets cannot be represented.
- Add Windows-specific tests for same-drive, different-drive, UNC, and verbatim paths.

### 6. Medium: test coverage is heavily Unix-gated

References:

- Many CLI integration tests use file-level or function-level `#[cfg(unix)]`; examples include `claudine/cli/tests/wrap_basics.rs`, `claudine/cli/tests/wrap_structured_stream.rs`, `claudine/cli/tests/sequence_cli.rs`, `claudine/cli/tests/compose_schema_cli.rs`, and `claudine/cli/tests/loop_cli.rs`.
- `claudine/cli/tests/level3_wrap_ctrl_c.rs` has an explicit Windows arm, which is a good pattern to copy.

Why this matters:

- Unix gates are appropriate for PTY/tmux/libc signal tests, but many wrapper/composition tests appear to be gated because of harness convenience rather than product semantics.
- Native Windows behavior can compile but remain behaviorally untested.

Recommendation:

- Split tests into platform-neutral core assertions and Unix-only harness assertions.
- Add Windows-native L1/L2 tests for argv normalization, composition, MCP import/export, stream parsing, and non-interactive wrapper spawn behavior.
- Add a CI matrix with at least `cargo check` for all claudine packages on Windows and Linux, and a smaller set of nextest tests on Windows where provider binaries are mocked.

### 7. Low: generated names should guard against Windows reserved filenames

References:

- `claudine/lib/src/mcp/types.rs:329`
- `claudine/lib/src/mcp/types.rs:386`
- skill/resource names are taken from directory names in `claudine/lib/src/linking/skills/portable.rs`.

`slugify` removes characters that are illegal on Windows, which is good. It does not appear to guard against reserved Windows basenames such as `CON`, `PRN`, `AUX`, `NUL`, `COM1`, or `LPT1`.

Why this matters:

- If a slug or generated file/directory name is ever used directly on disk, a name that is valid on Linux/macOS can fail on Windows.

Recommendation:

- Add a central `sanitize_windows_filename_component` helper for generated disk names.
- Reserve or suffix Windows device names case-insensitively.
- Add tests for `CON`, `con`, `NUL.txt`, `COM1`, and names with trailing spaces/dots.

### 8. Low: WSL guidance should be documented for builds and path translation

References:

- `claudine/rendezvous/core/src/socket.rs:74`
- `claudine/rendezvous/core/src/socket.rs:83`

The Unix socket path logic works under WSL because WSL is Linux, but the project docs should call out that building under `/mnt/c/...` can be much slower than building inside the WSL ext4 filesystem. Cross-boundary path exchange is also relevant because Claudine manages provider config paths that may originate from Windows or WSL.

Recommendation:

- Add WSL guidance to Claudine docs: prefer cloning/building inside the WSL filesystem, not an NTFS mount.
- If Claudine later exchanges paths between Windows providers and WSL processes, add explicit path translation rather than string replacement.

## Positive Notes

- `claudine/cli/src/commands/wrap/exec/spawn.rs` has separate Unix process-group and Windows `CREATE_NEW_PROCESS_GROUP` setup.
- `claudine/cli/src/commands/wrap/exec/termination.rs` has an explicit Windows wait loop with Job Object and console-control handling.
- `claudine/rendezvous/client/src/lib.rs` correctly dispatches client IPC between Unix sockets and Windows named pipes.
- `claudine/contract/tests/real_provider.rs` uses `std::env::split_paths` for PATH inspection.
- Most production filesystem paths are assembled with `Path::join` rather than manual string concatenation.

## Suggested Fix Order

1. Make `rendezvous-daemon` compile on Windows or explicitly remove it from the Windows support surface with a manifest/CI gate and documentation. Given the existing Windows client code, implementing named-pipe server support is preferable.
2. Add Windows link/copy fallback behavior for shared resource linking.
3. Fix contract shadow-home environment handling for native Windows.
4. Add Windows/WSL tests for path-text parsing and file URL rendering.
5. Add CI checks for `claudine`, `claudine-cli`, `claudine-contract`, and all rendezvous crates across Linux, macOS, and Windows.
