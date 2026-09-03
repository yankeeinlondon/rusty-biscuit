---
description: Core guidance for committing staged changes in this monorepo.
---
# Committing Staged Changes

Keep this file limited to durable, non-obvious commit guidance. The commit
prompt is the workflow authority; incident reports and package details do not
belong here.

## Scope

- Commit only what the caller staged. Never stage, unstage, restore, stash, or
  otherwise mutate the index or working tree to manufacture a commit. Extra
  staged paths are sibling work in a concurrent batch — scope with
  `git commit --only -F - -- <assigned-paths>` and leave the rest alone.
- Mixed-state (`MM`/`AM`) paths do not block unrelated groups. `--only` commits
  the *working-tree* content of named paths, so use it on an `MM` path only
  when the working tree is a clean superset of the staged snapshot.
  Otherwise use the temp-index plumbing fallback (`update-index --index-info`
  → `write-tree` → `commit-tree -S -F -` → `git update-ref HEAD <new> <old>`).
- Use `git log` for history examples; `sniff git commits` does not exist.

## Inspect First

- Never pre-flight signing with `git commit --allow-empty`. It only bypasses
  the "nothing to commit" check, not the index: any staged paths land in that
  commit. Read-only checks: `git log -5 --pretty='%G? %s'` (recent `G` means
  the agent has the subkey cached) or `gpg-connect-agent 'getinfo passphrase' /bye`.
- Scan full staged blobs (`git show :<path>`) for conflict markers, including
  the diff3 base marker `|||||||` that partial resolutions leave behind.
  `git diff --check` and hunk-only scans miss markers outside the changed hunk.
  On a hit, refuse, leave the path staged, and report `file:line`.
- `Cargo.lock` is coupled to the `Cargo.toml` that declares the dep. Check
  `git show :Cargo.lock | grep '"<dep>"'` against the staged manifest; a lock
  entry with no declaring manifest in the same commit is an orphan. In a
  parallel batch the manifest's group commits first, or absorbs the lock.

## Path-Limited Commits

- `--` before the pathspec list is mandatory, even with `--only`.
- Renames need both old and new paths in the pathspec; the destination alone
  commits only the `A` half.
- Feed messages via `-F -` with a single-quoted heredoc, or `-F <file>` when
  the commit may be retried (lock contention). A bare `-- <paths> <<EOF`
  without `-F` opens the editor and blocks.
- When writing a message file with the `Write` tool, do not put `$$` in the
  filename: the tool stores it literally while the shell later expands it.
- Under zsh, prefer `git cat-file -p "${rev}":path` over `git show "$rev:$path"`
  (`:` after a parameter is parsed as a modifier), and avoid the variable
  names `status` and `path`.

- A staged `R` is display-time similarity, not an index fact: the index holds
  an independent `D` + `A`, `git ls-files -s <new>` hides the old path's `D`,
  and the two endpoints can even hold different contents. Put BOTH endpoints
  in the brief, confirm with `git diff --cached -- <old> <new>` that the old
  side is a deletion of the expected blob, and check `git status --short`
  afterwards for leftover `D` entries.

## Signing

- Never disable or override signing (`commit.gpgsign`, `gpg.program`,
  `-c commit.gpgsign=false`). If signing hangs or fails, stop and report.
- Plumbing commits (`commit-tree` + `update-ref`) do NOT honor
  `commit.gpgsign=true`; pass `-S` to `commit-tree` explicitly.
- Commit exit status covers the index update, not the signature. Always follow
  up with `git verify-commit <hash>`; review-cycle tooling under
  `darkmatter/features/*` depends on `%G?` showing `G`.

## Merge State

- `--only`/pathspec-restricted commits are refused while `.git/MERGE_HEAD`
  exists. Check before dispatch; a wrapper that ran `git pull --no-edit` may
  have already landed all staged files as one merge commit, making the
  sub-agent brief stale. Check `git log --oneline -5` for that subject first.
- A merge commit that absorbs an out-of-scope group must name the absorbed
  scope in its body so reviewers do not expect a follow-up.

## Commit Messages

- Conventional Commits, lowercase after the colon, subject < 72 chars.
- `planning` covers moves into `_completed` / out of `_unscheduled` AND
  review-cycle doc edits inside a fix/feature directory (`log.md` entry,
  `review-N.md` flipping `implemented: true`, new `review-(N+1).md`, `spec.md`
  bumping `review_iterations`): `planning(<area>): close <fix> cycle N, open
  cycle N+1` (see `4c903c586`, `152ea6b84`, `690b2ecc3`). Such commits may
  have zero source diff; they are valid cycle iterations, not no-ops.
- In cycle-close bodies quote what the diff says; do not paraphrase into
  claims the staged text did not make ("smoke test failed" vs. "smoke attempt
  interrupted by host load").

- A brief that says "write the message body to a temp file" yields a file
  with no subject line, and `git commit -F` then collapses every bullet into
  one multi-hundred-character subject. Say "write the FULL message: subject
  on line 1, blank line 2, bullets after" and have the agent verify with
  `git log -1 --format=%B <hash>`.

## Concurrency

- Parallel groups need disjoint paths; producer commits before consumer. New
  variants on a non-`#[non_exhaustive]` enum couple producer and every
  matching consumer — merge them into one group even across package areas and
  drop the scope (`perf:` not `perf(sniff):`).
- GitNexus `detect_changes` on a shared staged worktree reports the aggregate
  of all sibling groups; a HIGH rating on a docs-only group usually describes
  sibling runtime work.
- Intermediate commits may reference files from sibling commits and fail to
  compile alone; expected for parallel structural refactors.
- Lock failures are transient: wait 1–3 s, retry the identical commit up to
  five times. Never remove a lock file. In a linked worktree the index lock is
  under `git rev-parse --git-dir` and ref locks under `--git-common-dir`.
  With five or more concurrent agents the budget can still run out — expect
  a second dispatch round after re-checking `git status --short`.
- Never `--no-verify`, override `core.hooksPath`, amend, or add fixup commits
  mid-batch. Report and let the orchestrator decide.
- Run from the inherited worktree root; never push.

- A shared `/tmp/commit_msg.txt` is a single hot slot: concurrent agents
  overwrite each other and the last writer's body ships under the wrong
  subject. Give every sub-agent a scope-unique message filename.

## Orchestration

- Before dispatch, diff the sorted union of all group pathspecs against the
  sorted staged set. A missed path (`area.rs` slipped an 8-group split) forces
  an unplanned catch-up commit.
- Long heredoc bodies delivered through sub-agents sometimes truncate
  mid-line even with `<<'MSG'`; keep bodies to 3–5 bullets and verify with
  `git log -1 --format=%B <hash>`.
- An empty sub-agent report can mean success *or* that the commit never ran
  (seen in a 13-group batch with three paths still staged). Verify each agent
  via `git status --short` plus the expected hash while the body and pathspec
  are still in context.
- "Path no longer staged" from a sub-agent usually means a sibling or the
  developer already committed it; check `git log -3` before restaging.
- A brief that pairs a pathspec file with `xargs -I {} git commit …` yields N
  stacked commits (a 35-path refactor landed as 35 identical commits). Say
  explicitly: one invocation with all paths positional, or
  `--pathspec-from-file`, never a per-path loop.
- Recovery from N agent-authored stacked commits: `git update-ref HEAD <new>
  <old>` (ref, new, old) is a CAS soft-reset; index and working tree are kept
  and the paths reappear staged for a single recommit.
- Active-file race: a path the developer is editing drifts between `add`,
  `status`, and `commit`; re-dispatching never catches a stable snapshot.
  Detect via mtime / repeated `MM`, then commit it directly from the
  orchestrator in one shell when the working tree is a clean superset.

## Verification

- Capture the hash from the bracketed banner in `git commit` stdout and verify
  with `git show --name-status <hash>` and `git verify-commit <hash>`. In a
  batch, neither `git log -1`, `rev-parse HEAD`, nor `git reflog -1` is
  authoritative — a sibling commit (e.g. `chore: refresh GitNexus index
  counts`) can land in between. If stdout was hidden, recover with
  `git reflog --grep '<subject-substring>' -1`.
- Use `git show --pretty=format: --name-only <hash>` when diffing the committed
  path list against a pathspec file.
- After all groups finish, reconcile `git status --short` against the
  original staged set; anything left belongs to a failed or unassigned group.
