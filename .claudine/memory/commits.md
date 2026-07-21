---
description: Core guidance for committing staged changes in this monorepo.
---
# Committing Staged Changes

Keep this file limited to durable commit guidance. The commit prompt is the
workflow authority; one-off incident reports and package implementation details
do not belong here.

## Scope

- Commit only changes the caller staged. Never stage, unstage, discard, stash,
  or otherwise rewrite the caller's index or working tree to manufacture a
  commit.
- If no changes are staged, report that there is nothing to commit and exit.
- Derive semantic groups from the staged diff. Every staged path must belong to
  exactly one group unless the caller explicitly changes the index.
- Unstaged and untracked paths are not part of the task. Mixed-state paths
  (`MM` or `AM`) do not block unrelated groups; commit unaffected groups and
  report any mixed path that cannot be committed safely.

## Inspect First

- Use `git status --short`, `git diff --staged --name-status`,
  `git diff --staged --stat`, and `git diff --staged` to review the exact
  staged set.
- Before committing an assigned path, check `git diff -- <path>`. A nonempty
  result means the working tree differs from the staged snapshot.
- Do not commit unresolved conflict markers. If staged content contains them,
  leave that group staged and report it.
- Use `git log` for commit-history examples. `sniff git commits` is not valid.

## Path-Limited Commits

- Use explicit file pathspecs for each semantic group. Avoid directory
  pathspecs when unrelated staged, unstaged, or untracked files exist beneath
  that directory.
- For renames, include both old and new paths in the same pathspec list. Passing
  only the destination path commits only the `A` half and leaves the staged
  deletion behind.
- Put Git options before `--`. For very large explicit path lists, use
  `--pathspec-from-file` and inspect the generated list.
- Feed commit messages through `-F -` with a single-quoted heredoc, or through a
  checked temp file for long bodies. A bare `git commit -- <paths> <<EOF` can
  open the configured editor and block.
- Do not place messages containing backticks, dollar signs, or other shell
  metacharacters in a double-quoted `-m` argument.
- **`--only` + `-F -` argument order.** In a path-restricted commit that reads
  the message from stdin, place `-F -` BEFORE `--only`, not after:
  `git commit -F - --only -- <paths>` works; `git commit --only -F - -- <paths>`
  makes `--only` absorb `-F` as a pathspec and git returns
  `error: pathspec '-F' did not match any file(s) known to git`. The heredoc
  payload is then never read. Pair this with the single-quoted `<<'COMMIT_MSG'`
  delimiter above.

## Mixed-State Paths

- `git commit --only -- <paths>` commits the working-tree content for the named
  paths. Do not use it when an `MM`/`AM` path has unstaged edits that regress or
  diverge from the staged snapshot.
- `--only --` is acceptable for an `MM` path only when the working tree is a
  clean superset of the staged snapshot. Verify by comparing `git show :<path>`
  and `git diff -- <path>` / `git diff --staged -- <path>`.
- If the staged snapshot must be committed while preserving divergent unstaged
  edits, use a temporary-index plumbing fallback:
  `ls-files -s` the staged entry, build a temp index from current `HEAD`,
  overlay the entry with `update-index --index-info`, `write-tree`,
  `commit-tree -F -`, and CAS-advance `HEAD` with
  `git update-ref HEAD <new> <old>`. Retry if the CAS fails.
- Keep all temp-index plumbing in one shell invocation; `mktemp -d` plus a trap
  will remove the directory when that shell exits.
- Under zsh, prefer `git cat-file -p "${rev}":path` over `git show "$rev:$path"`
  for blob verification. `"$rev:$path"` can be parsed as a zsh parameter
  modifier.

## Commit Messages

- Follow recent repository history and the prompt's Conventional Commit format.
  Subjects use lowercase after the colon and stay under 72 characters.
- Use `planning` for physical moves into `_completed` or out of `_unscheduled`.
  An in-place planning-document edit is normally `docs`, not `planning`.
- Describe the semantic change, not Git's similarity score or mechanics.

## Concurrency

- Parallel groups must have disjoint paths. If one group introduces a module,
  dependency, or symbol consumed by another group, commit the producer first.
- Extra staged paths are normal in concurrent batches. Treat them as sibling
  work and scope with `git commit --only -- <assigned-paths>`. `--only` leaves
  every other staged entry untouched in the index, so do NOT reach for
  `git restore --staged` (or any other index mutator) to "clean up" siblings
  before committing. Doing so corrupts sibling work and forces the orchestrator
  to re-stage from the working tree.
- The staged set can shrink between inspection and commit when a sibling commit
  lands. `--only -- <paths>` handles this; verify the resulting commit shape.
- Git accepts intermediate commits that reference files introduced by sibling
  commits. That may make individual commits non-compiling; this is expected for
  parallel structural refactors when the full final history is coherent.
- Git lock failures are transient contention. Retry the identical commit up to
  five times with a short backoff.
- In a linked worktree, `.git` is a file and the index lock is under the
  worktree-specific gitdir, while branch-reference locks are under the shared
  gitdir. Resolve both with `git rev-parse --git-dir` and `git rev-parse
  --git-common-dir`; do not infer lock locations from `.git/index.lock` or
  remove locks manually.
- Under heavy parallel contention (five or more sub-agents committing
  concurrently against the same worktree), the per-agent retry budget is
  sometimes insufficient: `index.lock` can persist longer than five
  attempts. Expect some groups to fail in the first round and re-dispatch
  them in a second round once the bulk of contention has cleared. Before
  re-dispatching, re-verify each failed group's assigned paths are still
  staged (`git status --short`). Never `rm` the lock file — that races
  against sibling agents and can corrupt an in-flight index write.
- Never disable repository signing or override signing configuration (including
  `gpg.program`) to avoid or preempt signing failures. Let the repository and
  host defaults apply. If signing hangs or fails, stop and report it.
- Never bypass repository hooks with `--no-verify`, `-c core.hooksPath=...`, or
  equivalent overrides. An instruction not to run validations means do not
  launch them explicitly; configured commit hooks still run normally. If a hook
  blocks the commit, report the failure rather than suppressing it.
- Never amend or create follow-up fixup commits after a successful commit in a
  concurrent batch. Report the issue so the orchestrator can decide whether to
  accept, revert, or coordinate a rewrite.
- Run commands from the inherited worktree root. Do not change to a guessed
  repository path, and do not push commits.
- In zsh wrappers, avoid special variable names such as `status` and `path`.
- **Active-file iteration race.** A staged file that the caller is actively
  editing will drift between `git add`, `git status`, and `git commit`. A
  dispatch loop (stage → sub-agent → see `MM` → re-stage → re-dispatch) can
  never catch a stable snapshot while edits arrive faster than the agent
  overhead. Detect by `stat -f '%Sm' <path>` (mtime in the last few seconds)
  or by repeated `MM` reports across two dispatches. When the file is being
  actively iterated and the working tree is a clean superset of staged
  intent, stage it once and commit directly as the orchestrator in a single
  shell — do not keep re-dispatching. Reserve sub-agents for stable snapshots.

## Verification

- A successful `git commit` exit status is authoritative for that invocation.
- Capture the new commit hash from `git commit` stdout and verify that hash with
  `git show --stat <hash>` or `git show --name-status <hash>`. Do not rely on
  `git log -1` after concurrent commits; HEAD may already have advanced.
- If a wrapper hides commit stdout, recover immediately with `git reflog -1`
  and verify that hash. Prefer unwrapped `git commit` so stdout is visible.
- Agent or task completion alone does not prove that its commit landed. After
  all groups finish, inspect `git status --short` and recent history for staged
  paths or missing commits. Treat empty or ambiguous agent reports as unknown.
- For whitespace-only groups, sanity-check with
  `git diff --staged --stat --ignore-all-space --ignore-blank-lines` or
  `--numstat --ignore-all-space --ignore-blank-lines`. Zero output means no
  non-whitespace changes remain.
