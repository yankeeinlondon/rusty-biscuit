---
name: worktree
description: Details on the `worktree` package area of the **rusty-biscuit** repo
---
# Worktree Package Area

Like many package areas in Rusty Biscuit, the `worktree` package area has two distinct packages:

1. Library: `worktree` is the library where the business logic for how we handle git worktrees is regulated
2. CLI: `worktree-cli` (binary name of `wt`) is the CLI which leverages the worktree library for most of it's functionality
