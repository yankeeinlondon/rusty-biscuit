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

## Path Resolution in This Worktree

The git repo root is `/Users/ken/.claudine/worktrees/rusty-biscuit/darkmatter`. Staged files are specified relative to this root:

- Files in `darkmatter/` package area use `darkmatter/<path>` prefix (e.g., `darkmatter/lib/src/layout/page.rs`)
- Files outside the darkmatter package area (e.g., `prompts/`) are at the repo root and may require `../prompts/` prefix when the subagent's working directory differs from the git repo root

When in doubt, run `git status` to see the actual staged file paths and use those exact paths in `git commit --only`.

## Zsh Backtick Expansion in Commit Messages

Literal backticks inside a double-quoted `-m` argument trigger command substitution in `zsh`. When a commit message contains Markdown code spans (backticks), escape them or split the message across multiple `-m` flags:

```bash
# Wrong — backticks are interpreted as command substitution
git commit -m "feat(foo): add `bar` helper"

# Works — separate -m flags are concatenated by git
git commit -m "feat(foo): add" -m "`bar` helper"
```

The multi-flag approach avoids shell escaping entirely and keeps the message intact.
