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
- Conflict-marker checks must inspect the full staged blob (`git show :<path>`),
  not only staged diff hunks or `git diff --check`; malformed or pre-existing
  marker fragments outside the changed hunk can otherwise pass unnoticed.
- When a full-blob scan finds unresolved markers in an assigned path, the
  subagent MUST refuse the commit, leave the path staged, and report the
  markers' file coordinates (grep output or `file:line` from `git show`).
  The orchestrator resolves them in the working tree (`edit` or equivalent),
  restages the same single path (`git add <path>`), and re-issues the
  `--only -- <path>` commit — never amend, never reset, never restage peers.
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
  checked temp file for long bodies. Prefer the temp-file pattern when the
  commit may need to be retried (lock contention): an in-place heredoc does not
  compose cleanly with a retry loop inside a single shell invocation, while a
  prewritten file lets the loop reuse the same body verbatim. A bare
  `git commit -- <paths> <<EOF` can open the configured editor and block.
- Do not place messages containing backticks, dollar signs, or other shell
  metacharacters in a double-quoted `-m` argument.

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
- `git commit --only -- <pathspec>` commits only the named paths while preserving
  unrelated staged entries. Parallel agents still need disjoint pathspecs and
  lock retries, but successful sibling commits do not naturally narrow the
  remaining index.
- After all groups finish, reconcile `git status --short` against the original
  staged set. Any staged path left behind belongs to a failed or unassigned
  group and must be reported before taking further action.
- Git lock failures are transient contention. Retry the identical commit up to

- Run commands from the inherited worktree root. Do not change to a guessed
  repository path, and do not push commits.
- In zsh wrappers, avoid special variable names such as `status` and `path`.

## Orchestration

- When the orchestrator delegates a commit to a sub-agent via heredoc, the
  body sometimes truncates mid-line: the file set, tree, and subject land
  correctly, but the trailing bullets of a long message are lost. The cause
  appears to be sub-agent shell-tool delivery, not shell expansion (single-
  quoted `<<'COMMIT_MSG'` still drops content). Observed twice in a single
  concurrent batch, so it is a reproducible pattern, not a one-off.
- Mitigation: keep sub-agent message bodies short (3–5 bullets, ≤20 lines).
  Verify the committed body with `git log -1 --format=%B <hash>` after each
  sub-agent commit; do not trust the sub-agent's textual report alone, since
  the truncation often shows up only on the actual commit.
- A truncated body is not corruption. Per the Concurrency rule, do not amend
  mid-batch — accept the loss, note it in the orchestrator's summary, and let
  the developer decide whether to rewrite the message after the batch settles.
- A sub-agent's *return report* can come back empty even when the commit
  itself landed cleanly with a fully intact body. This is distinct from the
  body-truncation pattern above (which corrupts the commit) — here the
  committed `git log -1 --format=%B <hash>` shows all bullets, but the
  agent's final message to the orchestrator is blank. Treat an empty
  report as "unknown, verify independently" rather than "failed". The
  existing Verification section already mandates this; the lesson is that
  it can happen even when the sub-agent had nothing to report as a problem.
- The corollary is sharper than the existing wording suggests: an empty
  return report can also mean the commit *never ran*. Observed in a 13-group
  batch: one sub-agent returned a blank final message and its three assigned
  paths were still staged. Treat empty/ambiguous reports as **silent failure
  until proven otherwise** by `git status --short` showing the assigned paths
  gone (and the expected hash in the log). Verify per-agent rather than
  only at the end of the batch — recovering from a missed commit is much
  cheaper while the orchestrator still has the intended commit body and
  pathspec in context.

## Verification

- A successful `git commit` exit status is authoritative for that invocation.
- Capture the new commit hash from `git commit`'s bracketed success banner and
  verify it with `git show --stat <hash>` or `git show --name-status <hash>`.
  Parse the hash token inside the brackets, not the first whitespace-delimited
  field, because branch refs and wrappers can make that field ambiguous.
- In a concurrent batch, `git rev-parse HEAD` immediately after success is not
  authoritative for that invocation: a sibling can advance `HEAD` between the
  commit and the read. Use the captured banner hash; if stdout was hidden,
  recover with `git reflog --grep '<subject-substring>' -1` and verify the
  subject and exact path set.
- Do not rely on `git log -1` after concurrent commits; HEAD may already have
  advanced.
- After all groups finish, inspect `git status --short` for staged paths left
  behind and report or commit them as appropriate.
- Agent or task completion alone does not prove that its commit landed. After
  all groups finish, inspect `git status --short` and recent history for staged
  paths or missing commits. Treat empty or ambiguous agent reports as unknown.
- For whitespace-only groups, sanity-check with
  `git diff --staged --stat --ignore-all-space --ignore-blank-lines` or
  `--numstat --ignore-all-space --ignore-blank-lines`. Zero output means no
  non-whitespace changes remain.
- `git reflog -1` is unreliable as a post-commit lookup when other agents are
  committing against the same worktree in parallel: any operation that lands
  between your commit and your read — e.g. an unrelated `chore: refresh
  GitNexus index counts` commit from a sibling agent — displaces the top of
  reflog and your hash becomes `HEAD@{1}` (or deeper), not `HEAD@{0}`. Recover
  with `git reflog --grep '<subject-substring>' -1` to filter by commit
  message, then verify that hash with `git show --name-status <hash>` and
  `git log -1 --format=%B <hash>`.
