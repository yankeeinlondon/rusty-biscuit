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
- `reqwest`, `bytes`, and `tokio` are gated behind the off-by-default `fetch`
  feature. They provide the shared policy-enforcing HTTP primitive used by
  Darkmatter compose and side-effect network paths.

The default feature set includes URL classification through `file-reference`,
but it does not compile the HTTP client stack unless `fetch` is enabled.
