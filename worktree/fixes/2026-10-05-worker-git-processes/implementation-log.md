# Implementation log

## 2026-10-05

- Added `worktree::git_metadata` (private module) with in-process `gix` reads
  for the five values in the spec; each falls back to its git command when
  `gix` cannot open the repository (`bail_if_untrusted`, `GIT_DIR` honored).
- **Departure:** `live_remote::is_valid_branch_name` lost its `base`
  parameter. Validation needs no repository, and keeping an unused parameter
  only to preserve the signature was rejected. Its four callers were updated.
- The parity test found that git accepts `@` as a branch name; an early
  special case refusing it was removed.
- Tests that pinned the old git processes now pin their absence:
  `git::tests::default_branch_starts_no_git_process`,
  `list::tests::list_worktrees_resolves_default_branch_without_git`, and the
  foreground git-call list in
  `every_listing_with_an_origin_launches_once_and_waits_for_both_halves`
  (now empty). New: `remote_update::tests::a_fetching_attempt_starts_only_ls_remote_and_fetch`.
- Same change, separate Windows-only test fixes found by the same runs: the
  `mklink /J` path spelling in `remove::test_support::replace_with_link`, the
  `list_table` fixture path spelling, and `list_output`'s markup-named entry,
  which Windows reports as uninspectable (os error 123) rather than gone.

### Evidence

`just cross-check <pkg> --os <os>` at the final tree, ordinary 3 s wait:
`worktree` 521/521 (Windows), 542/542 (Linux, WSL2); `worktree-cli` 525/525
(Windows), 553/553 (Linux, WSL2); macOS `just test` 1094/1094.

Windows worker, same probe and host: 1021 ms → 425 ms; `wt list` 1315 ms →
644 ms. The fake-Gitea HTTP fixture (each request starts `git http-backend`
on the test host) leaves the heaviest listings ~1.2 s of the 3 s wait on a
quiet Windows host; two of them failed once when a WSL2 run shared the same
physical machine, and passed 3/3 alone.
