# Commit Lessons Learned

## Never Reset or Rebase Commits

**Do NOT use `git reset` or `git rebase` in this monorepo worktree.** Staging and unstaging files can have unexpected consequences when developers are actively working on the codebase.

## Commit File Grouping

**Do NOT group commits by unstaging and restaging files one group at a time.** This risks worktree corruption and unexpected state changes from concurrent development.

Instead: commit file groups **explicitly** using `git commit --only -m "message" -- <file1> <file2> ...`.

The `--only` flag ensures git commits exactly the specified files, regardless of what else might be staged.

## Lock Contention

When multiple subagents commit in parallel against the same worktree, `git commit` can fail with:

```
fatal: Unable to create '.git/index.lock': File exists.
```

or the equivalent `refs/heads/<branch>.lock` variant. This is **not** corruption — git's locks are fail-fast, not queuing. Retry the same `git commit --only …` command after a 1–3 second backoff. Retry up to 5 times before giving up.
