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
- **In zsh, never use `status` as a shell variable name in commit wrappers
  (2026-07-10 darkmatter suggest_constraint batch).** `status` is a read-only
  zsh special parameter; assigning to it in a wrapper that wraps `git commit`
  raises an error *after* the commit has already succeeded, so the wrapper
  fails to print the captured `[branch hash] subject` line — leaving the
  subagent with a successful commit on disk but no way to capture its hash
  for verification. Use `commit_status` (or similar) instead. The
  underlying commit is unaffected; the fix is to rename the local variable.
- **Never run `git commit --amend` after a successful commit in a concurrent
  batch (2026-07-09 dmls + darkmatter batch, stray `7873a9a05`).** Once
  `git commit` returns zero, treat the commit as final for that invocation.
  In a multi-agent batch, by the time a subagent's commit succeeds, sibling
  commits may have advanced HEAD past the subagent's own commit — at which
  point `git commit --amend --only -F - -- <paths>` does not amend the
  subagent's commit in place; it creates a *new* commit on top of the
  current HEAD whose parent is HEAD\^ and whose tree matches HEAD's tree
  (the `--only` + pathspec is effectively a no-op for tree derivation
  under `--amend`). The original commit stays in the chain at HEAD\^, the
  intent (typo fix, content tweak) is *not* applied to it, and the new
  commit is an extraneous sibling on top of whatever siblings have already
  landed. Verified 2026-07-09 in the dmls + darkmatter 6-commit batch: a
  subagent ran `git commit --amend --only -F - -- <three-paths>` to fix a
  `D3/d4/D5` → `D3/D4/D5` typo in its own `feat(dmls)` commit subject,
  producing stray `7873a9a05` (subject OK, content the page.rs diff that
  belonged to a sibling `fix(darkmatter)` commit) and rendering its own
  corrected-content sibling `b7aa1973c` an orphan (no longer reachable
  from `HEAD`). The chain now carries both the typo'd
  `ed390d7aa feat(dmls): …(D3/d4/D5)` (correct content, bad subject) and
  a `feat(dmls): …(D3/D4/D5)` with the wrong file scope. Three reinforcing
  points: (1) if a subagent notices a typo or content drift *after* its
  `git commit` returns zero in a concurrent batch, the correct response is
  to **report it back to the orchestrator** — never to amend or follow up
  with another commit on the same branch from within the subagent; (2)
  the orchestrator can then decide between a downstream `git revert` of
  the stray, a coordinated `git reset --hard <known-good> + cherry-pick`
   pass (when the user authorizes the rewrite), or simply accepting the
   typo in the original commit subject; (3) this rule covers all
   *post-success* follow-ups — `git commit --amend`, `git commit
   --amend --only`, `git commit --amend --no-edit`, even `git commit
   --fixup=…` — none of them are safe inside a concurrent batch, because
   every sibling commit narrows the window in which the amendment would be
   unambiguous.

## Verifying a Concurrent Commit

- After `git commit` returns 0, do not use `git log -1 --stat` to verify
  the assigned paths. By the time the verifying command runs, a sibling
  commit may have advanced HEAD past the subagent's own commit, so
  `git log -1` shows the sibling's commit instead. Verified 2026-07-09
  in the dmls + darkmatter 6-commit batch: two of six subagents reported
  exactly this and had to fall back to `git show --stat <their-hash>` (or
  `git show --name-status <their-hash>` for renames) to confirm the
  assigned paths. The same race affects `git rev-parse HEAD` if it is
  not the very next command after the commit.
- Capture the new commit's hash from `git commit`'s own stdout — git
  prints `[<branch> <hash>] <subject>` on success — and verify with that
  hash. Reading git's stdout avoids the race entirely because it does
  not depend on the index/refs state at verification time.
- If a wrapper (e.g. a zsh function that pipes `git commit` through
  `tee`/`cat`/`read`) hides that stdout line, recover the hash via
  `git reflog -1` (or `git log --format='%H %s' -1 HEAD` immediately
  after the commit, before sibling commits land) and verify with
  `git show --stat <hash>`. Verified 2026-07-10 in the
  claudine + darkmatter 4-commit batch: one subagent's zsh wrapper
  swallowed `git commit`'s stdout; the subagent recovered the hash from
  reflog and verified paths with `git show --stat`, no content drift.
  This is a fallback for wrappers, not a recommendation to wrap —
  prefer the un-wrapped `git commit` form so stdout reaches the
  orchestrator directly.
- This is a sibling of the no-`--amend`-after-success rule: by the time
  a post-success action runs, HEAD may no longer be the subagent's
  commit. Treat the post-success window as a high-contention region;
  minimize the number of git commands that touch HEAD, refs, or the
  index between `git commit` returning 0 and the subagent's "I'm done"
  report.

## Verifying a Whitespace-Only Diff

- For a `style` commit (or any group expected to be whitespace-only),
  sanity-check the staged diff before committing with
  `git diff --staged --stat --ignore-all-space --ignore-blank-lines`
  (or `--numstat --ignore-all-space --ignore-blank-lines` for the
  numeric form). Zero output from the `--ignore-all-space
  --ignore-blank-lines` form means the diff contains no non-whitespace
  changes; a non-zero result means semantic content is in the diff and
  the group is mis-classified. Verified 2026-07-10 in the
  claudine + darkmatter 4-commit batch: the `style(darkmatter)`
  subagent ran this on `functions/collections.rs` (zero output)
  before committing the 9-file whitespace-only group.
