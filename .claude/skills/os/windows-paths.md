# Windows: Path Spelling

Part of the `os` skill; [SKILL.md](SKILL.md) is the index.

Load before touching `#[cfg(windows)]` path code, path comparison or
containment, `file://` URIs, or a test that is red only on `windows-latest`
with a path in the message. Processes and the environment are in
[windows.md](windows.md); consoles in [windows-console.md](windows-console.md);
the path APIs themselves in the `biscuit-file` skill.

Contract to test against: `ctx.repo_root`, `package_root`,
`package_area_root`, and `area_root` are portable `/`-separated strings with no
verbatim prefix on every OS (`biscuit_file::to_portable_string`). Compare
against that, never against `to_string_lossy()`.

## Path spelling

### Verbatim `\\?\` paths

1. **`std::fs::canonicalize` returns `\\?\C:\...`.** Any such value that
   crosses a comparison, containment, printed output, or reference-grammar
   boundary breaks (`?` becomes a lexical segment; the reference grammar
   rejects device prefixes). Use `biscuit_file::canonicalize_simplified`. Raw
   `canonicalize` is safe only in a key space that never leaves the process
   *and is never printed* (graph node ids and printed paths have leaked
   `\\?\C:` on Windows only). In biscuit-file, Claudine, and Darkmatter,
   `darkmatter/cli/tests/common/path_lookup_guard.rs` fails on a new direct
   call (and on direct home lookups, trap 5).
   - External tools reject it too: `cargo nextest`'s `--tool-config-file`
     (split on the colon) and `--workspace-remap`, and Git for Windows'
     `ls-files --exclude-from=`. `scripts/ci-build-archive.rs` cannot depend
     on `biscuit_file` (it runs on every scope calculation), so it carries its
     own `fn canonical_path`; everywhere else use `biscuit_file`.
2. **Test expectations built from raw `canonicalize` fail against simplified
   production output** (left `C:\…`, right `\\?\C:\…`). Build the expected
   value with `canonicalize_simplified`.
3. **`PathBuf::push` drops `.` and `..` pushed onto a verbatim buffer**, and
   `collect::<PathBuf>()` pushes, so rebuilding `\\?\C:\a\..\b` yields
   `\\?\C:\b` (under `\\?\` the dots are literal names). A lexical normalizer
   that must keep them assembles text (`normalize_native` in biscuit-file).
   Never write a `..` loop that `Vec::pop`s components: it pops the root or
   drive prefix.
4. **Anything that scans verbatim text breaks on its `?` or its literal
   separators.**
   - A suffix splitter that cuts at the first `?` turns every verbatim path
     into `\\`. Skip the `\\?\` / `//?/` prefix before looking for `#`, `?`,
     or `:` (`link_normalization::split_suffix`).
   - `{{VAR}}` interpolation concatenates text, so `{{ROOT}}/x.md` with a
     verbatim `ROOT` is one bogus component. A test that sets a variable from
     `canonicalize` stores `to_portable_string(&path)`.
   - A `\\?\` path built from `root.join("a/b")` fails every probe with OS
     error 123, and `cmd /C mklink /J` refuses it. Join name by name
     (`rel.split('/').fold(root, |p, n| p.join(n))`).

### Home and cache directories

5. **`dirs::home_dir()` and `dirs::cache_dir()` use the Windows known-folder
   API and ignore `USERPROFILE` and `HOME`**, so a hermetic test home silently
   does not apply and the test touches the real profile. Use
   `biscuit_file::home_dir()` (`std::env::home_dir()`, environment-first:
   `USERPROFILE`, never `HOME`). A Windows fixture relocates home with
   `USERPROFILE`; a child spawned with only `HOME` still reaches the real
   profile, so set both. Python's `Path.home()` likewise reads `USERPROFILE`
   on Windows. Do not gate a home-relocating test `cfg(not(windows))`; the
   fixture works there now. A test that must seed `dirs::cache_dir()` writes
   the real per-user path, so key it uniquely and delete what it seeded.

### Separators and spellings

6. **GitHub's Windows runner has an 8.3 short-name TEMP (`RUNNER~1`); a dev
   machine usually does not.** `current_dir()` reports the spelling it was
   given; `canonicalize` re-spells to long names. A test that mixes a
   canonical path with a fixture's raw one passes locally and fails in CI.
   Model what the child reports (`launched_spelling` in
   `claudine/cli/tests/l1/ctx_launch_anchor.rs`). To reproduce, read a TEMP
   directory's short name (`for %I in ("<dir>") do @echo %~sI` in `cmd`) and
   run from PowerShell with `$env:TEMP`/`$env:TMP` set to it.
7. **`windows-latest` fails at 260 characters where a dev host may not**
   (`LNK1104: cannot open file`); a host with `LongPathsEnabled=1` cannot
   reproduce it. Nested builds inside an extracted nextest archive
   (`%TEMP%\nextest-archive-…\target\…`) hit it fast. Keep nested builds under
   a short `%TEMP%` directory and count the deepest path against 260.
8. **`Path::join("a/b")` keeps the literal `/`**, so a native-spelling needle
   has mixed separators and matches nothing. Normalize with
   `.components().collect::<PathBuf>()`. Values such as a scope catalog's
   `package_root()` arrive this way (`C:\…\cwd\area/pkg`).
9. **A user-typed fragment never matches walker output by raw text.**
   `ignore::Walk` yields `\`; the user typed `/`. Compare both sides through
   `to_portable_string` (`path_matches_query` in
   `claudine/cli/src/completion/scopes.rs`). Non-interactive tests can miss
   it when "zero candidates" and "interaction denied" print the same
   diagnostic.
10. **`$PWD` in a `shell: bash` step is an MSYS path (`/d/a/repo/repo`);
    `$RUNNER_TEMP` is a Windows one (`D:\a\_temp`).** Native programs
    (`--workspace-remap`, `INSTA_WORKSPACE_ROOT`, `BISCUIT_JUNIT_*`) cannot
    open the first. `just _native_path` (`just/devops.just`) answers the one
    spelling both layers accept: `cygpath -m`, drive-qualified, forward
    slashes. In CI `_ci_build_verify` publishes it as its `workspace` output.
11. **Backslashes do not survive a `just` recipe's `*args` list.**
    `forwarded=({{ args }})` word-splits and processes escapes, so
    `C:\Users\ken\x.tar.zst` arrives as `C:Usersken…`. Pass what
    `just _native_path` answers. A parameter inside single quotes
    (`'{{ path }}'`) is safe; an array literal is not. `_archive_file_check`
    refuses an unreadable `--archive-file` and names this hazard.
12. **A `file://` URI must carry neither `\\?\` nor `\`.** Normalize
    separators to `/`, strip `\\?\` (mapping `\\?\UNC\server\share` to the
    authority `server/share`), and prefix `file:///` for a drive path. Test
    the spellings as string literals so Unix cells cover them; a fixture built
    from `canonicalize` is dead code off Windows.
13. **A rootless `/x` path is relative (or current-drive) on Windows.** A
    `FileResolutionContext` over `/w/...` fails `validate()` with
    `RelativeContextDirectory`; a reference `/review-2.md` is
    `ForeignAbsolutePath`; a fixture `/repo/x.md` renders as
    `file:///B:/repo/x.md`. Give unit-test paths a drive
    (`context::test_support::abs` in `darkmatter/dmls/src/context.rs` maps
    `/w/x` to `C:/w/x`) or use a real temp dir. When a test passes only on
    Unix, assert the exact path, not a loose `ends_with`.
14. **On Unix `\` is a filename character, and `to_portable_string` turns it
    into `/`** (`back\slash.rs` becomes a different file). Expected paths for
    such files keep native spelling on Unix; a `PathIdentity` route
    (`portable_relative` in `tree-hugger/lib/src/file/naming.rs`) keeps the
    literal `\`.
15. **Markdown escape handling eats a backslash.** Windows temp dirs contain
    `\.tmpXXXX`, and CommonMark resolves `\.`, `\-`, `\_`. A Windows-only path
    missing one backslash is a Markdown round-trip, not path resolution.

### Invalid names and error kinds

16. **`metadata` on a name holding `*` or `?` fails with
    `ErrorKind::InvalidFilename` (OS error 123), not `NotFound`.** Code that
    treats only `NotFound` as absent turns a miss into an I/O failure on
    Windows alone; biscuit-file's probes (`deepest_existing_ancestor`) treat
    both as absent.
17. **Windows rejects C0 controls (TAB, ESC) in filenames** before a writer
    sees them (error 123). For a cross-platform fixture exercising control
    neutralization, use a C1 control such as U+009B; test TAB/ESC at the
    writer boundary without creating files.
18. **Windows filenames need not be valid Unicode.** `OsStringExt::from_wide`
    can create an unpaired surrogate (U+D800) name or directory, including a
    child's `current_dir`. `to_string_lossy()` substitutes U+FFFD and names a
    different file: treat it as display only, and validate a native relative
    route before a UTF-8 export (invalid units confined to the scan root do
    not invalidate the relative route). Test with native Windows `cfg` tests,
    not only on Linux.
19. **`reflink-copy` reports an unsupported NTFS block clone as an
    HRESULT-shaped `io::Error` (`0x80070001`, "Incorrect function"), not raw
    error 1.** Match that form to fall back to a byte copy
    (`unsupported_clone` in `worktree/lib/src/include/copy.rs`).

## Worktree include links and junctions

- Treat any directory with `FILE_ATTRIBUTE_REPARSE_POINT` (`0x400`) as a
  traversal boundary. Git for Windows' `ls-files` **traverses junctions**, so
  check every candidate ancestor with `symlink_metadata` and
  `MetadataExt::file_attributes()`, not only the final file. `FileTypeExt`
  distinguishes file and directory symlinks but not junctions.
- The stable metadata API does not expose a file index; do not use it as a
  copy-record ID.
- Symlink creation without privilege fails with `ERROR_PRIVILEGE_NOT_HELD`;
  match `raw_os_error() == Some(1314)`, not an `ErrorKind`
  (`worktree/lib/src/include/copy.rs`). The native build host can create
  symlinks, so it does not exercise the denial path; Developer Mode also
  permits it.
- Git reuses a worktree admin-directory name after remove, prune, and re-add,
  but deletes anything stored inside the old one.
