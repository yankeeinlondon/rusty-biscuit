---
description: Verified, non-obvious guidance for committing staged changes in this monorepo.
---
# Knowledge Learned about Committing

Keep this file limited to durable commit guidance. The commit prompt is the
authority for the workflow; one-off incident reports and package implementation
details do not belong here.

## Commit Scope

- Commit only changes the caller staged. Never stage, unstage, discard, stash,
  or otherwise rewrite the caller's index or working tree to manufacture a
  commit.
- Derive semantic groups from the staged diff. Every staged path must belong to
  exactly one group; a path cannot be split across commits without changing the
  index.
- An unstaged or untracked path is not part of the task. A path with both staged
  and unstaged changes (`MM` or `AM`) does not invalidate unrelated groups:
  commit the unaffected groups and report the mixed-state path if it cannot be
  committed without including unstaged work.
- If no changes are staged, report that there is nothing to commit and exit.

## Inspect Before Committing

- Use `git status --short`, `git diff --staged --name-status`,
  `git diff --staged --stat`, and `git diff --staged` to identify and review the
  exact staged set.
- Before committing an assigned path, check `git diff -- <path>`. A nonempty
  result means the working-tree content differs from the staged snapshot.
- Do not commit unresolved conflict markers. If staged content contains them,
  leave that group staged and report it.
- Use `git log` for commit-history examples. `sniff git commits` is not a valid
  command.

## Path-Limited Commits

- Use explicit file pathspecs for each semantic group. Avoid directory
  pathspecs when unrelated staged, unstaged, or untracked files exist beneath
  that directory.
- `git commit --only -- <paths>` commits the working-tree content of the named
  paths rather than preserving a different staged snapshot. Therefore, do not
  use it on an `MM` or `AM` path unless the working tree and staged content have
  first become identical through changes made by the caller.
- For a rename, include both the old and new paths so the deletion and addition
  remain in the same commit.
- Put all Git options before `--`. For very large explicit path lists, use
  `--pathspec-from-file`; inspect the generated list and ensure rename lists
  contain both source and destination paths.
- Feed commit messages through `-F -` and a single-quoted heredoc. Do not place
  messages containing backticks, dollar signs, or other shell metacharacters in
  a double-quoted `-m` argument.

## Commit Messages

- Follow recent repository history and the prompt's Conventional Commit format.
  Subjects use lowercase after the colon and stay under 72 characters.
- Use `planning` for physical moves into a `_completed` directory or out of an
  `_unscheduled` directory. An in-place planning-document edit is normally
  `docs`, not `planning`.
- Keep a rename or move atomic and describe the semantic change, not Git's
  similarity score or mechanical file operation.

## Concurrent Commits

- Parallel groups must have disjoint paths. If one group introduces a module,
  dependency, or symbol consumed by another group, commit the producer first.
- Git lock failures are transient contention. Retry the identical commit up to
  five times with a short backoff; never disable repository signing to bypass a
  failure.
- A successful `git commit` exit status is authoritative for that invocation.
  After all groups finish, inspect `git status --short` to find staged paths
  left behind and report or commit them as appropriate.
- Run commands from the inherited worktree root. Do not change to a guessed
  repository path, and do not push commits.
