# Windows: Path Spelling

Load before touching `#[cfg(windows)]` path code, path comparison or
containment, `file://` URIs, or a test that is red only on `windows-latest`
with a path in the message. Processes and the environment are in
[windows.md](windows.md); consoles and terminals in
[windows-console.md](windows-console.md).

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

   Git for Windows also rejects a verbatim absolute path passed to
   `ls-files --exclude-from=`. The include resolver uses
   `biscuit_file::canonicalize_simplified` for that argument. Raw canonical
   paths remain inside the copy record's identity comparison.
   `reflink-copy` reports an unsupported block clone on NTFS as an HRESULT
   shaped `io::Error` (`0x80070001`, "Incorrect function"), not raw error 1.
   The worktree copy fallback recognizes that form and byte-copies instead.
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
   `dirs::cache_dir()` is the same (`%LOCALAPPDATA%` from the known folder),
   so a Windows test that seeds a cache file writes to the real per-user
   path, keyed by its temporary repository, and deletes what it seeded
   (`worktree/cli/tests/perf_support`'s `pr_store`, 2026-09-25).
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
10. **A fixture joined as `root.join("a/b/x.md")` keeps the `/` in the native
    text.** Ordinary Win32 calls accept it, so most tests pass, but it breaks
    the two places where `/` is not a separator: a `\\?\` path built from it
    fails every probe with os error 123 ("filename, directory name, or volume
    label syntax is incorrect"), and `cmd /C mklink /J` refuses the link.
    Join name by name (`rel.split('/').fold(root, |p, n| p.join(n))`) in any
    fixture that feeds a verbatim spelling or `mklink`. Found 2026-10-01 by the
    `biscuit-file` `portable_path::platform` tests on `build-win-native`.
11. **A `{{VAR}}` reference whose value is a verbatim path does not resolve.**
    `FileReference` interpolation concatenates text, so `{{ROOT}}/x.md` with
    `ROOT=\\?\C:\r` becomes `\\?\C:\r/x.md`, one component under the
    verbatim prefix. `PortablePath` therefore never writes `{{ROOT}}/…` for
    such a value (verification rejects it) and falls through to `~` or the
    absolute path. A test that sets a variable from `fs::canonicalize` must
    store `to_portable_string(&path)`, the spelling a user would export.
    Found 2026-10-01 by the Darkmatter `link_normalization` tests.
12. **A suffix splitter that cuts at `?` cuts a verbatim path at its prefix.**
    `\\?\C:\…` contains `?`, so treating the first `?` as a URL query turns
    every verbatim destination into `\\` (not absolute) and silently skips
    it. Skip the `\\?\` / `//?/` prefix before looking for `#`, `?`, or `:`.
    Darkmatter's `link_normalization::split_suffix` does; the macOS and Linux
    legs cannot see it. Found 2026-10-01 on `build-win-native`.
13. **`PathBuf::push` drops `.` and `..` pushed onto a verbatim buffer**, and
    `collect::<PathBuf>()` pushes, so rebuilding `\\?\C:\a\..\b` from its
    components yields `\\?\C:\b`, a different directory (under `\\?\` the
    dots are literal names). A lexical normalizer that must keep them, such
    as `biscuit-file`'s `normalize_native`, assembles the result as text.
    Likewise, never write a `..` loop that calls `Vec::pop` on components:
    it pops the `RootDir` or drive prefix and makes `/../a` relative.
    Confirmed 2026-10-01 on `build-win-native`.


## Worktree include links and junctions

For `.worktreeinclude` traversal, treat any directory with
`FILE_ATTRIBUTE_REPARSE_POINT` (`0x400`) as a boundary. A native Windows probe
created a junction whose attributes were `Directory, ReparsePoint`; Git for
Windows **traversed it** during `ls-files` and returned a file underneath.
Check every candidate ancestor, not only the final file. A file
symlink reported `Archive, ReparsePoint`. `std::os::windows::fs::FileTypeExt`
distinguishes file and directory symlinks, while
`std::os::windows::fs::MetadataExt::file_attributes()` exposes the reparse bit
needed to catch junctions too. Check `symlink_metadata` before entering an
ancestor. The stable Windows metadata API does not expose file index; do not
use it as a copy-record registration ID.

The native build host allowed `mklink` to create a file symlink, so it does
not exercise the denial path. A machine without symlink privilege can return
Win32 `ERROR_PRIVILEGE_NOT_HELD` (1314); match `raw_os_error() == Some(1314)`
for the warning-and-skip path instead of assuming a particular Rust
`ErrorKind`. Developer Mode can permit unprivileged creation. Git reused the
same worktree admin-directory name after remove, prune, and re-add, but removed
a marker stored inside the old admin directory. See the runnable
`worktree/fixes/2026-09-25-worktree-file/spike_windows.ps1` probe.
