# Biscuit File Dependencies

## File References And Remote Fetch

- `url` is enabled by `file-reference` so `FileReference` can classify HTTP(S)
  descriptors without applying local path normalization to URL syntax.
- `gix` (pure-Rust, `default-features = false` + `sha1`) is gated behind
  `file-reference` and used only for repository-root discovery in
  `find_git_root`. It replaces the former `git2`/libgit2 dependency so the
  crate (and its consumers, e.g. `sniff`) carries no C-linked git backend.
- `dirs` is gated behind `file-reference` and supplies the cross-platform
  home directory for `home_dir` / `~` (home-pinned) references. It replaces a
  bare `$HOME` read, which is not a complete contract on native Windows.
- `globset` is gated behind `file-reference` and will compile the glob half
  of a glob reference (a file-reference prefix followed by a glob)
  (**planned**: no code uses it yet). Darkmatter already depends on the same
  `0.4` line, so the workspace gains no new crate. Patterns must be built with
  explicit options, not the crate defaults: `backslash_escape` is off by
  default on Windows, so a literal `[` or `*` is escaped with character
  classes (`globset::escape`), which read the same on every OS.
- `dunce` is unconditional and reduces a Windows `\\?\` verbatim path to its
  legacy spelling — but only when the legacy spelling is equivalent — at the
  crate's two boundaries:
  - `simplify_root` (behind `file-reference`), the resolver's root boundary.
    Anchors reach the resolver in both spellings (`std::fs::canonicalize` yields
    verbatim; `gix` and `dirs` yield legacy), and Win32 applies no path
    normalization under the verbatim prefix, so a reference's own `/` separators
    would never resolve.
  - `to_portable_string` / `try_portable_string`, the path→text boundary, which
    is unfeatured and so cannot depend on an optional crate. `dunce`'s refusal
    to reduce is authoritative there: a declined path keeps its native spelling
    rather than becoming URL-shaped text.

  It is a no-op on every other target and has no transitive dependencies.
- `reqwest`, `bytes`, and `tokio` are gated behind the off-by-default `fetch`
  feature. They provide the shared policy-enforcing HTTP primitive used by
  Darkmatter compose and side-effect network paths.

The default feature set includes URL classification through `file-reference`,
but it does not compile the HTTP client stack unless `fetch` is enabled.

## Development Only

- `test-toolkit` is a development dependency for the `test_layout` gate in the
  consolidated `l1` test binary. It adds no crate to the workspace: every crate
  it brings is already in `Cargo.lock`.
