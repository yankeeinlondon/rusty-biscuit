---
name: worktree
description: Details on the `worktree` package area of the **rusty-biscuit** repo
---
# Worktree Package Area

Like many package areas in Rusty Biscuit, the `worktree` package area has two distinct packages:

1. Library: `worktree` is the library where the business logic for how we handle git worktrees is regulated
2. CLI: `worktree-cli` (binary name of `wt`) is the CLI which leverages the worktree library for most of it's functionality

The `worktree-cli` crate contains both a binary target (`src/main.rs`, binary `wt`) and a library target (`src/lib.rs`). Shared modules such as `commands` and `perf` are declared in both `lib.rs` and `main.rs` so they are available to integration tests (which use the library target) and to the binary.

`worktree::worktree::list_worktrees` uses `worktree::cache` to persist SHA-pair branch comparison results under the user cache directory. The cache key is `(default_tip_sha, branch_tip_sha, CACHE_FORMAT_VERSION)`, so branch/default tip movement self-invalidates ahead/behind and clean-merge results. Dirty working-tree status is never cached and remains a live `git status` check.

`list_worktrees` is a thin wrapper over `parse_worktree_state` and `fill_worktree_statuses`. The CLI uses that seam to start graph/verbose data gathering from the parsed branch names while the expensive per-worktree status pass runs.
