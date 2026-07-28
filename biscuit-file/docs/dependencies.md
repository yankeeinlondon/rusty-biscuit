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
- `dunce` is gated behind `file-reference` and reduces a Windows `\\?\`
  verbatim path to its legacy spelling at the resolver's root boundary. Anchors
  reach the resolver in both spellings (`std::fs::canonicalize` yields verbatim;
  `gix` and `dirs` yield legacy), and Win32 applies no path normalization under
  the verbatim prefix, so a reference's own `/` separators would never resolve.
  It is a no-op on every other target, and is also a dev-dependency because the
  integration tests must build expectations in the same spelling.
- `reqwest`, `bytes`, and `tokio` are gated behind the off-by-default `fetch`
  feature. They provide the shared policy-enforcing HTTP primitive used by
  Darkmatter compose and side-effect network paths.

The default feature set includes URL classification through `file-reference`,
but it does not compile the HTTP client stack unless `fetch` is enabled.
