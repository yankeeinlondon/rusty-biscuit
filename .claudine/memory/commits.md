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
- Scan staged blobs for all diff3 conflict-marker forms, including the base
  marker `|||||||`; partial resolutions can leave it behind after deleting the
  `=======` separator and `>>>>>>>` terminator.
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
- Before dispatching subagents, cross-check that the union of every group's
  pathspec list equals the exact staged file set (sort both and diff). If a
  staged path is not in any group, the orchestrator has missed it and must
  recover before dispatch, not after the batch lands. Catching it after the
  fact still works (`git status --short` exposes leftovers), but it produces
  an unplanned catch-up commit mid-summary that complicates the report and
  shifts the convention from "every commit was planned" to "we patched one
  in late". Observed in this repo with `area.rs` slipping past an 8-group
  split — small file, big directory list, easy to skip during fast path
  partitioning.

## Verification

- A successful `git commit` exit status is authoritative for that invocation.
- Capture the new commit hash from `git commit` stdout and verify that hash with
  `git show --stat <hash>` or `git show --name-status <hash>`. Do not rely on
  `git log -1` after concurrent commits; HEAD may already have advanced.
- If a wrapper hides commit stdout, recover immediately with `git reflog -1`
  and verify that hash. Prefer unwrapped `git commit` so stdout is visible.
- After all groups finish, inspect `git status --short` for staged paths left
  behind and report or commit them as appropriate.
- For whitespace-only groups, sanity-check with
  `git diff --staged --stat --ignore-all-space --ignore-blank-lines` or
  `--numstat --ignore-all-space --ignore-blank-lines`. Zero output means no
  non-whitespace changes remain.
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
