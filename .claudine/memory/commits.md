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
- For a 50+ path commit (e.g. one feature's entire source tree plus
  fixtures), pass the paths via `--pathspec-from-file=<list>` rather than
  expanding them inline. Inlining ~50 absolute or long paths approaches
  `ARG_MAX` on macOS/Linux shells and either silently truncates the
  pathspec or errors with `argument list too long`. The list file is read
  by git, so each path is on its own line (LF or CRLF) and `#`-comments
  are allowed. Combine with `-F /tmp/commit_msg_<scope>.md` for the
  message and place `-F <msg>` BEFORE `--pathspec-from-file=`:
  `git commit -F /tmp/msg.md --pathspec-from-file=/tmp/paths.txt`.
  The sub-agent brief must still list the paths verbatim in its body so
  `git show --name-status <hash>` against the brief's path list is the
  post-commit verification.
- Mixed-state (`MM`/`AM`) paths do not block unrelated groups. `--only` commits
  the *working-tree* content of named paths, so use it on an `MM` path only
  when the working tree is a clean superset of the staged snapshot.
  Otherwise use the temp-index plumbing fallback (`update-index --index-info`
  → `write-tree` → `commit-tree -S -F -` → `git update-ref HEAD <new> <old>`).
- **Temp-index capture order.** Setting `GIT_INDEX_FILE=$TMPIDX` *before*
  `git ls-files -s` reads from the temp index (which is empty) and yields an
  empty commit. Capture the staged blob lines first (they are in the real
  index), then `export GIT_INDEX_FILE=$TMPIDX`, then pipe the saved lines
  into `git update-index --index-info`. An empty commit is recoverable via
  `git update-ref HEAD <previous-tip> <empty-hash>` (no `git reset`,
  no index/worktree churn).
- **Temp-index fallback is a *replacement*, not an additive, tree.** Populating
  a fresh temp index with only the captured `ls-files -s` lines and then
  `git write-tree` produces a tree containing **only** those paths — not
  `HEAD + those paths`. The resulting commit's diff against the parent shows
  every other file in the tree as deleted (10k+ deletions in this monorepo),
  which is not a recovery case the orchestrator can detect after the fact. The
  fallback is only correct when the captured set is intended to fully
  replace HEAD's tree at those paths (e.g. one sibling's slice of a multi-agent
  batch where another agent owns the rest). When the assigned set is purely
  additive (no `D ` entries in `git diff --cached --name-only`, and the real
  index already has exactly that additive set on top of HEAD — i.e. no
  *other* sibling staged paths need excluding), skip the temp index entirely
  and run `git write-tree` against the real index, then `commit-tree -S -F - -p
  HEAD`. The staged snapshot of any `AM`/`MM` paths you want to commit at their
  pre-supersede content is already in the real index; you only need the temp
  index when sibling staged paths must be excluded from the tree.
- **Plumbing is for merged multi-agent batches, not sequential commits.** The
  temp-index fallback only restores the full tree at the merge of all sibling
  branches; for *sequential* commits on a single branch, the fallback's
  missing HEAD paths propagate into every subsequent `--only` commit (whose
  tree is `previous-HEAD + named-change`, with `previous-HEAD` itself sparse)
  and the cumulative state loses the other files. Prefer `--only` for
  non-clean-superset `AM` paths in any sequential flow even though it commits
  working-tree content beyond the staged snapshot — describe those additions
  in the body so the bullet list matches the diff. Reserve the plumbing
  fallback for parallel branches that converge in a merge.
- `--only` on a clean-superset `MM` path still captures working-tree-only
  content (e.g. a manifest `[[test]]` block whose source file is currently
  untracked). The pre-flight `git show :<path>` only sees the staged blob;
  the only way to detect that is to diff the staged snapshot against the
  working tree (`git diff -- <path>`) before commit. If the working-tree
  addition is meant to ship in a separate commit (typical for an actively
  in-progress sibling change), prefer the temp-index plumbing fallback so
  the commit body reflects only what was staged; otherwise the bullet list
  drifts from the diff and reviewers see a `[[test]]` registration with no
  matching source file in the same change.
- Splitting a single file's hunks across two commits: when the staged `M `
  has two semantic groups of hunks (e.g. finding A and finding B both
  touching the same file), the simplest split is to construct each
  desired version offline (HEAD + selected-hunks applied) and write
  version N to the working tree just before commit N. `--only` reads
  working-tree content, so each commit captures the version you wrote.
  Constructing versions needs two cumulative offsets: an in-set offset
  per hunk (cumulative `new_count - old_count` of earlier hunks in the
  SAME set, so sequentially-applied hunks line up) and a prior offset
  per hunk for cross-set application (cumulative change from hunks in
  earlier sets whose `old_end < current old_start`, so applying Group 2
  on top of Group 1's output lands at the right line). Verify by
  concatenating the two versions back to STAGED — a mismatch means a
  hunk was classified wrong. Use `git apply` for one-shot splits when
  all hunks in one set appear before all hunks in the other in HEAD
  order and you can pass the patch directly; the offset machinery is
  for interleaved hunks (`feat-a` at line 30, `feat-b` at line 40,
  `feat-a` at line 50, etc.), where `git apply` cannot apply a subset.
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
- `git grep --cached` (and the other index-mode flags `--no-index`,
  `--untracked`, `--exclude-standard`) MUST appear before the pattern, not
  after. After the pattern, git parses them as revisions and dies with
  `fatal: unable to resolve revision: --cached`. The `--` separator only
  splits paths from patterns; it does not re-enable option parsing. Use
  `git grep --cached <pattern> -- <path>` (option-before-pattern) or pipe
  `git diff --cached | grep`.
- `Cargo.lock` is coupled to the `Cargo.toml` that declares the dep. Check
  `git show :Cargo.lock | grep '"<dep>"'` against the staged manifest; a lock
  entry with no declaring manifest in the same commit is an orphan. In a
  parallel batch the manifest's group commits first, or absorbs the lock.
- A workflow `BISCUIT_REQUIRE_<TOOL>: "1"` declaration on a step is coupled
  to a `require_tools("<tool>", ...)` call in the Python suite that step
  runs: the env var only does work when the guard reads it, and the guard
  only fails (vs. skips) when the var is set. Either half alone is dead —
  declaration without consumer is a marker no test reads, consumer without
  declaration is a guard that can only ever skip. When splitting the work
  into multiple commits, ship the declaration alongside its consumer in
  the commit that introduces the guard, or accept the intermediate state
  where one half is dead until the matching half catches up. The
  `ci_workflow_contracts::every_tool_guard_declaration_is_set_by_the_job_
  that_enforces_it` test pins both directions: a guard whose variable no
  job sets, and a variable no guard reads.

## Path-Limited Commits

- `--` before the pathspec list is mandatory, even with `--only`.
- Renames need both old and new paths in the pathspec; the destination alone
  commits only the `A` half.
- Feed messages via `-F -` with a single-quoted heredoc, or `-F <file>` when
  the commit may be retried (lock contention). A bare `-- <paths> <<EOF`
  without `-F` opens the editor and blocks.
- `-F <file>` MUST come BEFORE the `--` pathspec boundary (`git commit
  --only -F /tmp/msg.md -- <paths>`). Placing it after the `--` makes git
  resolve the file path relative to the worktree root, not the caller's
  cwd, so any absolute path outside the repo — e.g. the `/tmp/commit_msg_*.md`
  files used to keep per-sub-agent messages from colliding — dies with
  `fatal: '<path>' is outside repository`.
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
- A staged rename is read at the NEW path. Once `git add` has registered the
  rename, only the new path is in the index; `git show :<old-path>` fails with
  "path does not exist (neither on disk nor in the index)". Read the staged
  blob at `git show :<new-path>` (or `git cat-file -p :<new-path>` under zsh).
  For a rename-only commit the OLD path's content is whatever `git show
  HEAD:<old-path>` prints; if the rename is R100 the two blobs match.
- Splitting a single file's content across two commits (e.g. two
  `planning(repo)` commits whose spec.md needs `review_iterations: 5→6` in
  commit 1 and `6→7` in commit 2): `git commit --only -- <path>` UPDATES
  the index entry for `<path>` to the working-tree blob it just committed,
  so the obvious "restore the second state from staging with
  `git show :<path> > <path>`" returns the FIRST commit's blob, not the
  pre-commit staging. Save the pre-commit staged version to a temp file
  (e.g. `git show :<path> > /tmp/<path>-staged.md`) BEFORE the first
  `--only`, then `cp` it back to the working tree after the first commit
  and BEFORE the second. If the original is lost, `git fsck --dangling`
  can recover the blob (`git cat-file -p <hash>` shows the file;
  `git cat-file -t <hash>` confirms `blob`); the reflog only retains
  the post-`--only` blob because the index-update is not a ref update.
- **Staged-vs-working-tree swap for an `AM`/`MM` path with draft
  follow-up work.** When the staged set contains an `AM`/`MM` path whose
  working-tree delta is *draft* work for a future cycle (e.g. an
  `implementation-log.md` records cycle N as complete and cycle N+1 as
  "starting work", and the working tree carries cycle N+1 in-progress
  edits that must NOT ship yet), prefer a staged-to-working-tree swap
  over temp-index plumbing. Capture both: `git show :<p> > /tmp/<p>-staged`
  for the staged blob, `cp <p> /tmp/<p>-wt` for the working tree, then
  `cp /tmp/<p>-staged <p>` so the working tree matches the index, then
  `git commit --only -F <msg> -- <paths>` (commits staged), then
  `cp /tmp/<p>-wt <p>` to restore the draft. This sidesteps the
  temp-index lost-files side effect in a sequential flow (every
  subsequent `--only` commit would otherwise build on a sparse tree and
  drop files) while still keeping the working tree's draft intact. The
  swap is identical to `--only` with a transient working-tree
  replacement; verify with `git status --short <p>` showing ` M` after
  the restore.

## Signing

- Never disable or override signing (`commit.gpgsign`, `gpg.program`,
  `-c commit.gpgsign=false`). If signing hangs or fails, stop and report.
- Plumbing commits (`commit-tree` + `update-ref`) do NOT honor
  `commit.gpgsign=true`; pass `-S` to `commit-tree` explicitly.
- `git filter-branch --msg-filter` (or any filter-branch filter) also strips
  signatures: even with `commit.gpgsign=true`, the rewritten commits come out
  with `%G? = N` rather than `G`. Verify with `git verify-commit <hash>` after
  filter-branch; if unsigned, roll back via `git update-ref HEAD <pre-batch-sha>`
  (keeps index/working tree staged) and replay with normal `git commit`. Using
  filter-branch to "fix message only" forces an unsigned chain unless you
  resign each commit afterwards, which is more invasive than a clean replay.
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
- `planning` covers moves into `_completed` / out of `_unscheduled`, **new
  spec files added to `features/_unscheduled/`** (a pure `A` for the spec —
  `planning(<area>): schedule <name>` for implementation), **new spec files
  added to an active `fixes/YYYY-MM-DD-<name>/` directory** (a new dated fix
  being scheduled for implementation, distinct from `_unscheduled/`; same
  `planning(<area>): schedule <name>` shape — see `97f12132c` adding
  `fixes/2026-09-10-local-affected-scope/spec.md`, `aedeeb46d` adding
  `fixes/2026-09-11-cicd-cleanup/spec.md`), **new plan.md /
  implementation-log.md added to an active `fixes/YYYY-MM-DD-<name>/`
  directory whose `spec.md` is already tracked at HEAD** is
  `planning(<area>): record execution plan for <name> fix`, NOT another
  `schedule` event — the spec was committed in a prior `planning: schedule`
  commit and a naive `schedule` heading overstates the work. Pre-flight
  `git ls-tree HEAD <dir>` distinguishes the two: if the spec is already
  there, treat the new file as an execution-time artifact. See `5b772e59a`
  adding `claudine/fixes/2026-09-16-better-spec-syntax/plan.md` and
  `ef4824fdd` adding `darkmatter/fixes/2026-09-15-nullable-directive-targets/plan.md`.
  **new spec files added to an
  active `features/YYYY-MM-DD-<name>/` directory** (the `features/` analog
  of the dated-fix case above — same `planning(<area>): schedule <name>`
  shape; see `c36f72fd0c` adding `features/2026-09-09-more-context/spec.md`,
  `0b80ca7c9` adding `features/2026-09-15-dasherized-identifiers/spec.md`),
  **new plan.md / implementation-log.md / spike-*.md added to an
  active `features/YYYY-MM-DD-<name>/` directory whose `spec.md` is
  already tracked at HEAD** is `planning(<area>): record execution plan
  for <name> feature` (the features/ analog of the dated-fix
  record-execution-plan case above); when a single commit lands the
  expanded spec, the plan, AND a spike report together (e.g.
  `d97996487` `planning(sniff): record decisions and execution plan for
  recent-commits feature` for sniff/features/2026-09-15-recent-commits
  with modified `spec.md`, new `plan.md`, and new `spike-linking-cost.md`
  referenced from spec.md Decision 9), the spec/plan/spike
  cross-references must resolve within that one commit — splitting
  them lands a spec that cites a non-existent plan or spike file. The
  spec's modification belongs with the new artifacts because the
  decisions that distill the spec's expanded content are what the plan
  derives from and the spike is what one of those decisions cites; a
  separate `docs(sniff): expand spec` commit splits a single planning
  cycle into two log entries with no boundary between them.
  Pre-flight `git ls-tree HEAD <dir>` confirms spec is already tracked.
  See `95216cfd8` for the larger multi-artifact example
  (`darkmatter/features/2026-09-09-more-context` landed spec.md +
  decisions.md + plan.md + implementation-log.md in one commit,
  `planning(darkmatter): record Phase 1-3 execution of more-context
  feature`).
  AND review-cycle doc edits
  inside a fix/feature directory (`log.md` entry, `review-N.md` flipping
  `implemented: true`, new `review-(N+1).md`, `spec.md` bumping
  `review_iterations`): `planning(<area>): close <fix> cycle N, open
  cycle N+1` (see `4c903c586`, `152ea6b84`, `690b2ecc3`). Such commits may
  have zero source diff; they are valid cycle iterations, not no-ops. The
  unscheduled-add case is not a `feat` because no code ships, and not `docs`
  because `_unscheduled/` is a planning surface (the frontmatter `area`
  is the scope — `area: repo` → `planning(repo)` even for CI-leg specs).
  The active-fix-spec case uses the same rationale: no code ships, the
  dated directory is a planning surface, the frontmatter `area` is the
  scope (e.g. `area: repository-ci` for a repo-wide CI fix still becomes
  `planning(repo):` per the analogous `97f12132c` precedent). The
  active-feature-spec case (`features/YYYY-MM-DD-<name>/spec.md`) is the
  same reasoning: the `'features/'` segment is a project-naming convention
  for active work, not a claim that code ships in this commit — `feat` is
  reserved for code that actually ships — and the frontmatter `area` is
  the scope, so `area: darkmatter` becomes `planning(darkmatter):` even
  though the path lives under `features/`.
- When `review-1.md` is a feature's FIRST independent review, no review
  cycle exists to close — the `close cycle N, open cycle N+1` shape does
  not apply. Use `planning(<area>): add review 1 for <feature>` for the
  `A review-1.md` plus the `review_iterations: 0→1` spec bump, even when
  the feature's phase-based execution close already landed at HEAD. See
  `0cee5151f` (research-metadata-pipeline review 1 after Phase 8 close).
  Design-phase `M`-only edits recording human-confirmed rulings together
  with follow-up investigation sections are likewise one `planning(<area>):`
  commit, subject shaped `confirm <decisions> and record <investigations>`
  (see `44b5fcfe0`, following the `538734269` precedent).
- A missed prior-cycle close can be combined with the current cycle's
  close in one `planning(<area>):` commit. When a fix's review cycle work
  spans multiple orchestrators (or a prior close simply never landed) and
  the prior `review-N.md` is still `implemented: false` while finding
  code already shipped, the catch-up commit flips the prior review's
  `implemented: true`, adds `next:`/`log:`/`implemented_by:` to chain it
  forward, then adds the current `review-M.md` (implemented) and
  `review-(M+1).md` (new), bumps `review_iterations` by the cumulative
  delta, and backfills the missing `## Implementation of Review Findings
  #N` log sections for both cycles in one hunk (the prior cycle's
  "Successful Completion" summary and the current cycle's iteration record
  land together because the log body is a single continuous block). Body
  must explicitly say "cycle N was not formally closed when its findings
  landed in <list-of-fix-commits>" so reviewers understand the retroactive
  flip. See `3ccedd9d9` (research-metadata-pipeline close cycle 2 and
  open cycle 3, retroactively closing cycle 1 alongside it).
- A review-to-implement iteration — an `implementation-log.md` section
  "Implementation of Review Findings #N" recording the fixes applied to a
  review's findings — is `planning(<area>): record review-to-implement
  iteration N`, not `close cycle N, open cycle N+1` (no new review exists
  yet, and no `review-N.md` flips in it). It lands before the per-finding
  `fix:`/`test:` commits it documents; each finding's code, tests, and its
  "docs updated" files (user guide, README, skill surface) ship in that
  finding's own commit keyed on the finding's primary subject. When the
  findings' test hunks interleave in one shared test file, the earlier
  finding's commit carries the file minus the later finding's contiguous
  test block and the later finding's commit carries the full staged file;
  verify by reinserting the block back into the earlier version.
- In cycle-close bodies quote what the diff says; do not paraphrase into
  claims the staged text did not make ("smoke test failed" vs. "smoke attempt
  interrupted by host load").
- A terminal review (`review-N.md` with `ready: true`, `implemented: false`,
  and no `next:` field) marks the end of a feature/fix, and the close can
  bundle the FINAL cycle with the directory move into
  `<area>/<features|fixes>/_completed/` in one atomic commit rather than
  splitting them. Subject shape: `planning(<area>): close <name> cycle N and
  move to completed` for the single-cycle terminal variant (e.g.
  `8ce2121af`), or `close <name> cycles N-M and move to completed` when
  the implementer's batched work closed several cycles before the final
  review landed (e.g. `9bb5bd8ac` closing cycles 1-3 with three review
  files added at once, the terminal review-3 carrying no `next:`, and
  the spec's `review_iterations` bumped once to the final count). The
  move and the cycle closure share the same rename-and-bump commit
  because splitting the renames from the review files leaves a populated
  `_completed/<name>/` directory without the cycle history until each
  cycle-close catches up.
- Multi-spec consolidation is one atomic `planning(<area>):` commit, not
  N+M separate commits: marking N existing specs `status: superseded`
  (with `superseded_by: ../<new>/spec.md` frontmatter pointer), adding
  M new spec/annex files that absorb their content, and recording the
  ratification in a charter spec's decision block all belong together.
  Splitting the A's from the M's ships the new spec without the supersede
  banner, so the successor exists without historical evidence anything
  was retired; splitting the M's from the A's retires the old specs but
  leaves readers with no path to the successor. The supersede
  relationship between old and new IS the consolidation — commit both
  sides together. See `4616e9aec` for a 5-file example (3 M supersede +
  ratification, 2 A new spec + design annex).
- A supersede banner may land as a NEW spec file added directly to
  `fixes/_completed/` (or `features/_completed/`) on its first commit,
  paired atomically with the successor spec. The variant covers the
  case where the superseded spec was a draft that never advanced to
  implementation, so it has no active-directory history — it ships
  already in `_completed/` with `status: superseded` and `superseded-by:
  ../<new>/spec.md` frontmatter. The atomic commit is still mandatory:
  the new spec's `supersedes:` frontmatter and the supersede banner's
  `superseded-by:` frontmatter must resolve against each other inside
  one tree, so splitting them ships either a successor that points at a
  non-existent banner or a banner that points at a non-existent
  successor. See `817de5bb9` (planning(repo): schedule direct-cell-
  execution feature and supersede 2026-09-12-better-cicd-flow fix) for
  the canonical example: one commit added the successor
  `features/2026-09-19-direct-cell-execution/spec.md` and the banner
  `fixes/_completed/2026-09-12-better-cicd-flow/spec.md` together.
  Contrast with the existing 5-file consolidation at `4616e9aec`, where
  the superseded specs were already tracked at HEAD and only their
  `status:` flipped.
- `planning(<area>): close <fix> as invalidated` is distinct from
  `close <fix>` (completed/implemented) or `close <fix> with <deferral>`
  (`773bbac93`). The diff adds `status: invalidated` + `reviewed_on:
  <date>` to the frontmatter and prepends a viability-review section
  that names the upstream work that pre-empted the fix and the
  contracts the proposed boundary would have violated. The original
  investigation is retained as historical evidence; requirements and
  success criteria are explicitly marked superseded by the review. Do
  not confuse with supersession (consolidation entry above): invalidation
  has no successor spec and no `superseded_by:` pointer — the proposed
  work is simply no longer needed. Example: `planning(sniff): close
  2026-07-22-inefficient-calling fix as invalidated` (`1881b7919`).

- An in-design.md supersede (a decision `D{N}` confirmed and then explicitly
  replaced by `D{N+1}` within the same human review checkpoint, e.g. when
  the user reverses direction immediately after the option-A confirmation)
  is one atomic `planning(<area>):` commit, not two. `D{N}` stays in
  `design.md` under a leading "**Superseded by D{N+1}.**" banner so the
  audit trail records what was first agreed and why it was replaced;
  `D{N+1}` carries the effective rule and is the only decision reflected in
  `spec.md`. The commit subject explicitly calls out the supersede
  ("confirm D{N+1}-D{M} and record D{N} supersede by D{N+1}") so reviewers
  know both sections in `design.md` were intentional. Splitting the
  confirmation commit from the supersede ships a checkpoint where `D{N}` is
  authoritative text in `spec.md` between the two commits, which is wrong
  for the period before the supersede lands. See `2d783c6ad` for the
  2026-09-17-remove-strict-mode D19→D20 example (D19 retained for
  traceability, D20 is the effective tier policy in `spec.md`).

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
- Pre-flight status staleness: an agent's `git status --short` snapshot can
  be invalidated by a sibling agent's commit landing between the snapshot
  and the agent's own `git commit --only`. The agent sees the path staged,
  but by the time the agent's commit lands, the sibling has already
  consumed the path. After-commit `git status` is clean (no missing path),
  which masks the staleness; verify `git show --name-status <hash>` and the
  reflog (`git reflog --grep '<subject-substring>' -1`) to confirm the path
  landed in the sibling's commit. `--only` itself does NOT unstage other
  paths — empirically verified with `AM file1.txt / A  file2.txt` plus
  `git commit --only -- file1.txt`, which leaves `A  file2.txt` staged.
- A brief that pairs a pathspec file with `xargs -I {} git commit …` yields N
    stacked commits (a 35-path refactor landed as 35 identical commits). Say
    explicitly: one invocation with all paths positional, or
    `--pathspec-from-file`, never a per-path loop.
- For 100+ path commits (e.g. the `*/tests/*` mass sweep that landed
    archive-compatible `manifest_dir!()` across the monorepo), construct the
    pathspec with `git diff --cached --name-only <scope-glob> > /tmp/paths.txt`
    and then `git commit --only -F msg --pathspec-from-file=/tmp/paths.txt`.
    Two hazards the per-line file avoids that an inline arg list does not:
    (a) shell expansion of a glob inside the args (`git commit … -- '*/tests/*'
    '*/benches/*' …` lets zsh expand `*/tests/*` against the current
    worktree, producing a `pathspec 'file1 file2 …' did not match` failure
    that surfaces as a noise wall); (b) shell ARG_MAX limits when the path
    list is in the thousands. Verify by counting `wc -l < /tmp/paths.txt`
    against `git diff --cached --name-only <scope-glob> | wc -l`.
- When a glob pathspec misses a few paths (e.g. `*/tests/*` did not match
    `claudine/gen/src/agent_errors_check/review6_tests.rs` and
    `playa/lib/src/detached/tests.rs` — both `src/*tests.rs` unit-test
    files whose path the glob did not reach), ship the missed paths as a
    *follow-up* `test(<scope>): follow-up …` commit rather than amending the
    mass commit. The corpus guard in `test-toolkit::archive_path_guard`
    catches the same miss next run, so the follow-up is documentation, not
    drift.
- Recovery from N agent-authored stacked commits: `git update-ref HEAD <new>
  <old>` (ref, new, old) is a CAS soft-reset; index and working tree are kept
  and the paths reappear staged for a single recommit.
- `git commit --amend` (no explicit ref) targets HEAD silently. If the goal is
  to fix a non-HEAD commit's message (e.g. commit N in a chain of N+1 — "I
  mistyped a file count in the body and want to correct commit 3, not the tip"),
  `git commit -F <corrected-msg> --amend` rewrites HEAD with the new message
  *and HEAD's tree content*, so the chain becomes inconsistent: HEAD now
  carries commit N's intended message but commit N+1's tree (or vice versa).
  `--only -- <paths>` does NOT pin amend to those paths — it operates on HEAD
  regardless. The clean fix is `git update-ref HEAD <pre-batch-sha>` (rolls
  the chain back while leaving the index staged), then replay the commits in
  order with `git commit -F <msg> -- <paths>`. Pre-flight `git log -1
  --pretty=%P HEAD` to capture the pre-batch parent before any amend attempt.
- A 7-commit rename-to-_completed batch where one commit's body said "the 14
  files" but the diff had 13 was recovered this way: `git update-ref HEAD
  9b797b61a` rolled the chain back to the pre-batch commit while keeping all
  19 staged renames in the index, then the 7 `git commit --only -F <msg> --
  <old-path> <new-path>` invocations replayed with corrected messages and
  fresh GPG signatures. The new SHAs all differ from the originals (each
  commit re-signs with the current author/key) but the tree content is
  identical — verify with `git diff <old-sha> <new-sha>` for each corrected
  commit to confirm only the message changed.
- Multi-agent batch + `update-ref` chain loss. When agents A and B commit in
    parallel (B on top of A) and you `update-ref` from B back to A's parent to
    recover from a bad B, A is severed from HEAD too — A is still reachable
    from the reflog and from B, but `git log` no longer does. Capture A's SHA
    before the `update-ref`, fix B's commit (now first), then `git
    cherry-pick <A-sha>` to restore A on top of the corrected B. Verify the
    final chain with `git verify-commit` on every recovered commit; the
    cherry-picked A re-signs with the current author/key, so its hash differs
    from the original.
- Active-file race: a path the developer is editing drifts between `add`,
  `status`, and `commit`; re-dispatching never catches a stable snapshot.
  Detect via mtime / repeated `MM`, then commit it directly from the
  orchestrator in one shell when the working tree is a clean superset.
- A brief's claim that a /tmp snapshot was "pre-saved by the orchestrator"
  is unverified input: the agent should check the file exists before relying
  on it. When it is missing, re-capturing via `git show :<path>` is faithful
  only while no `--only` commit has touched that path since the brief was
  written — `--only` rewrites the index entry to the blob it just committed,
  so a later re-capture returns the previous commit's content, not the
  original staged snapshot.

## Verification

- Capture the hash from the bracketed banner in `git commit` stdout and verify
  with `git show --name-status <hash>` and `git verify-commit <hash>`. In a
  batch, neither `git log -1`, `rev-parse HEAD`, nor `git reflog -1` is
  authoritative — a sibling commit (e.g. `chore: refresh GitNexus index
  counts`) can land in between. If stdout was hidden, recover with
  `git reflog --grep '<subject-substring>' -1`.
- "HEAD raced ahead" surfaces as: agent reports `git show --name-status HEAD`
  and `git verify-commit HEAD` succeed, but the path list / signature belong
  to the sibling commit that landed after its own. Always verify against the
  hash captured in the `[branch abbrev] subject` banner, not against `HEAD`.

## Content Patterns

- A new docs subtree frequently lands with several 0-byte placeholder files
  (e.g. `shared-resources/agent-definitions/agent.md`, `mcp/mcp-services.md`,
  `prompts/prompts.md`, `agent-skills/upgrading-skill-props.md`) alongside
  prose siblings. The placeholders are intentional scaffolding, not missing
  content — commit them together with the prose; do not omit them as "empty
  files" or split them into a follow-up.
- Docs consolidation across multiple deleted sources plus a single new file is
  not a rename. When two `docs/topics/*.md` files are deleted and replaced by
  one heavily synthesized `docs/<topic>.md` (similarity below `git diff -M50%`
  threshold), the index holds an independent `D + D + A + M(sibling link-fix)`
  set. Splitting the A from the D pair ships a 1200-line file with no
  antecedent; splitting a D from the A loses the "what was consolidated"
  evidence. Commit all of them in one `--only` invocation so reviewers see
  the replacement as one change.
- A sibling skill that introduces a contract (e.g. rust-devops rewrites its
  CI/CD section) and a referencing skill (e.g. `os` adds a cross-reference to
  the new contract) are disjoint paths and commit safely in parallel, but the
  cross-reference is stale between the two commits and the reflog shows it.
  Either ship them in the same commit when paths allow, or document the
  ordering in the second commit's body so reviewers know the cross-reference
  resolves against an earlier sibling.
- Use `git show --pretty=format: --name-only <hash>` when diffing the committed
  path list against a pathspec file.
- A workspace-wide version-pin refresh that swaps the version number in a
  `Cargo.toml` table row often leaves a stale numeric reference in the
  surrounding prose of a sibling `docs/dependencies.md` (e.g. "Pinned to the
  workspace-wide `0.42` used by …" three lines below the now-`0.55` row). A
  regex pass over only the `.toml` files misses this; before staging a
  bump, `git grep -nF '<old-version>' -- '*.md'` over the in-scope area or
  scan each staged `docs/dependencies.md` with `git show :<path>` for the old
  pin string. Flag the prose in the commit body as a follow-up rather than
  silently shipping a self-contradicting paragraph.
- After all groups finish, reconcile `git status --short` against the
  original staged set; anything left belongs to a failed or unassigned group.
- When a commit subject describes a structural move ("restructure skill tree",
  "extract to new module", "consolidate under `foo/`") but the staging only
  adds the new path without staging the old as `D` or `R`, the tracking-tree
  ends up with both old and new files. The pre-commit diff against HEAD will
  not surface the leftover because nothing is staged for it; detect by
  `git ls-files <old-glob>` after staging and either re-stage the deletes or
  flag the leftover tracked paths in the commit body as a follow-up. A batch
  of `A`-only entries alongside a single `R` is the giveaway: the rename
  collapses a `D + A` into one index fact but every other plain `A` is a
  tracked-path addition, not a move.
- Extracting a shared fixture into a CI-skipped crate (e.g. one whose
  `[package.metadata.ci] gates = false` lists the package out of every
  CI leg's test run) requires a parallel regression test in a CI-gated
  package — the canonical fixture test in the skipped crate is reference
  evidence only and is never executed in CI. Ship the parallel test in
  the same atomic commit as the extraction; otherwise the gate that proves
  the fix lands in a follow-up that drifts from the fixture's actual
  behavior, and the only signal that the two have diverged is a developer
  running both by hand.
- A closure document (`fixes/<name>/closure.md` or an analogous
  implementation-time artifact under a fix / feature directory) may
  name an "unrelated user edits at entry" list — paths the implementer
  touched but does not consider part of this fix. The list is the
  orchestrator's signal to commit those paths in a separate
  `docs(<area>):` or `chore:` commit, alongside the orchestrator's own
  `planning(<area>):` close commit. Treat the list as authoritative;
  do not fold the named paths into the implementation, tests, or docs
  commits of the same fix even when they share a package area with the
  fix's scope — the implementer's "unrelated" is a stronger signal than
  the orchestrator's "lives in the same directory tree".
- A cycle close that moves a fix/feature directory into `_completed/`
  often unblocks one or more downstream plans whose `depends_on` lists
  the closing cycle's `spec.md` (those plans typically carry
  `status: blocked` in their frontmatter and a matching `blocked_on`
  entry). The unblock is its own `planning(<area>):` sibling commit:
  drop `status: blocked` from the downstream plan's frontmatter in a
  separate `--only -- <downstream-plan-path>` invocation. Do not bundle
  the unblock into the cycle-close commit — the downstream plan lives
  in a different fix directory with its own lifecycle, and reviewers
  need the unblock visible as a deliberate scheduling decision. The
  unblock body should name the satisfied `depends_on` prerequisite and
  reference the cycle-close commit's hash (e.g. "the cicd-cleanup
  cycle is moved to _completed/ in <hash>") so the relationship is
  auditable without `git log --graph`. A wholesale same-file
  whitespace reformat that rides along with the `status: blocked`
  removal (typical of an editor that auto-indents nested list items)
  is part of the same cohesive edit, not a second semantic group —
  do not split it into a separate commit just because the diff is
  ~290+/290- of pure indentation.
- A cycle close into `_completed/` is valid even when the moved spec
  still shows `implemented: false`, e.g. when the only remaining
  work is a branch-protection migration that requires separate human
  approval (the `ci-verdict` → `ci-gate` switch in
  `fixes/2026-09-11-cicd-cleanup/closure.md` C10 is the canonical
  example). The `_completed/` move closes the planning surface; the
  `implemented: false` flag is the implementation sign-off gate and
  is independent. Call this out in the cycle-close body so reviewers
  don't mistake the move for a full implementation sign-off, and so
  the closure-checklist items still "Waiting for proof / approval"
  are visible rather than buried.
- A `RESOLVED_PLAN_SCHEMA_VERSION` bump is one inseparable change with the
  new required fields in `scripts/ci/schema.py`, the regenerated
  `.github/ci/schemas/contract.json`, the version constant in any Rust
  reader (e.g. `scripts/ci-rollup.rs`'s `PLAN_SCHEMA_VERSION`), and every
  hand-built plan fixture scattered across the test suites
  (`test_schema.py::plan()`, `test_resolved_plan.py::PlannerFixture`,
  `test_local_evidence.py::ScopeReceiptTests`,
  `test_evidence_reuse.py::EvidenceFixture`, the single-line
  `.githooks/tests/fixtures/plan-*.json`). Pre-flight
  `git diff --cached --stat` counts the fixture files but does not show
  which builders still carry the old shape; check each builder explicitly
  (`git show :<path> | grep -F '"<new-field>"'`) or accept the test
  failure as "missing required field" rather than a missing path.
- A fix's `implementation-log.md` frontmatter carries the
  authoritative per-phase file lists
  (`source_files_during_phase_N`, `docs_updated_during_phase_N`,
  `skills_files_updated_during_phase_N`) plus a top-level `source_code:`
  catalog of every file touched across all phases. When a multi-package
  fix spans a `commit-messages` batch (the canonical pattern is one commit
  per package area per phase group, plus a separate `planning(<area>):`
  close), diffing the sorted staged set against the sorted
  `source_code:` list gives an exact "what belongs to this fix" scope
  without scanning every file's diff. Files in the staged set that are
  NOT in the catalog belong to a sibling fix (different planning dir or
  unrelated entry) and route to their own commit. Lines that begin with
  embedded Unicode glyphs in `source_code:` (e.g. raw emoji or a marker
  prefix) and adjacent truncation (e.g. `lint.tx` instead of `lint.txt`)
  are normal extraction artifacts, not missing files; match by string
  after stripping non-ASCII prefix bytes and treating the truncated name
  as the full one.
- A `planning(<area>):` cycle close whose `implementation-log.md` records a
  follow-up code change as "present only in the working tree" (e.g. a
  non-vacuity mutant accidentally captured by a sibling `fix:` commit,
  restored in the working tree, awaiting a follow-up) must land BEFORE
  the follow-up `fix:` commit. The log's present-tense claim is accurate
  at the moment of its own commit (the working tree has the fix, history
  does not); after the follow-up fix lands, the line is also in history,
  so the claim is no longer accurate at HEAD. Committing the follow-up
  fix first makes the close commit land on top of an already-fixed tree,
  inverting the log's narrative. Sequential ordering matters even when
  the paths are disjoint and the commits could otherwise run in parallel.
  See `7600faaee` (planning close) followed by `8a1fdc2f2` (one-line
  `Err(error) => return Err(error)` follow-up to the M1 mutant captured
  in `57e23751f`).
- The `source_files_during_phase_N` lists can be INCOMPLETE — the last
  phase (typically Phase 7 acceptance regression) is usually written
  *after* the plan's frontmatter is committed, so its source list is
  empty or partial even when the corresponding test files ARE in the
  staged set. A staged file that matches no phase source list is NOT
  automatically a sibling-fix path: cross-reference the file's content
  (or its sibling Phase 4 file's `!` comment about it, e.g.
  `compose_initialize_staged_boot.rs` naming its
  `compose_initialize_acceptance.rs` companion). When the cross-ref
  confirms same-fix, route the file to its own `test(<area>):` or
  `feat(<area>):` group keyed on the file's primary subject (AC4-AC12
  coverage in this example) rather than treating it as a missed group
  or splitting it into the prior phase's commit.
- A file listed in MULTIPLE `source_files_during_phase_N` lists of the
  same fix accumulates changes from each phase (e.g.
  `claudine/lib/src/composition/mod.rs` re-exports new
  `prepare::bootstrap` symbols in Phase 3 and a new
  `looping::build_loop_seed_from_bootstrap` in Phase 4; the same
  applies to `composition_seams.rs` allowlist entries). Place the file
  in the LATEST phase's commit so every symbol is defined before any
  re-export references it; splitting it would require the
  `composition::*` re-exports to point at a function that does not
  exist in the earlier commit's tree, breaking compilation between
  the two commits.
- A phase-based fix's final close (e.g. `2026-09-15-initialize-after-proxy`
  Phase 8 documentation phase) has a different shape than a cycle-based
  fix's review close. There is no `review-N.md` to flip; the close
  artifacts are the `plan.md` and `implementation-log.md` frontmatter
  additions plus an optional new `evidence.md`. The single
  `planning(<area>):` commit appends `## Phase N-1 close` (the prior
  acceptance phase) and `## Phase N` (the current docs / comments
  phase) blocks to `implementation-log.md`, adds the
  `source_files_during_phase_N`, `docs_updated_during_phase_N`, and
  `skills_files_updated_during_phase_N` blocks to `plan.md`, finalizes
  the plan frontmatter (`phase: <N>`, `completed_phase: "<N>"`,
  `implemented: true`), and may attach a new `evidence.md` carrying
  the requirement-to-test mapping and gate results. The fix remains
  in its active directory until author review moves it to
  `_completed` — `planning(<area>):` is correct, NOT `chore:` or
  `docs(<area>):`. See `5aff59c38` for the
  2026-09-15-initialize-after-proxy Phase 8 example.
- A `planning(<area>):` phase close can land as "Outcome: blocked on
  required human input" rather than "shipped deliverables" when the spec
  gates the next phase on operator-supplied values that have not been
  supplied (per-platform time/invocation limits, approver name, agent
  choice, etc.). The shape differs from the deliverable close in three
  ways: do NOT flip any wave checkboxes (no forward progress was made);
  carry forward prior unanswered `human_review_items` and ADD new ones
  for gating that emerged during this phase rather than merging them; and
  push procedural findings (PATH ordering, dry-run side effects, agent
  hints) into the skill surface as a separate `docs(<area>):` commit
  keyed on the relevant `research-contract` / SKILL section so the
  findings survive the wait for operator input. See `58b946717` for the
  2026-09-17-research-metadata-pipeline Phase 7 example.
- Pre-flight a `docs(repo):` rename by listing BOTH endpoints in
  `git ls-files -s <old> <new>` — the rename is a single index fact
  but the index holds independent `D` + `A` entries, and the
  `--only` pathspec must name both. A `git show --name-status <hash>`
  after commit will surface a single `R0NN` row when both endpoints
  were included; if only the new path appears as `A`, the old endpoint
  was silently dropped and the prior `claudine/docs/topics/...` path
  remains tracked at HEAD. Verify with `git ls-files <old-glob>`
  before reporting success.
- Hand-rolled CLI help registries are a hidden integration point.
  A `feat:` that adds a new subcommand for a CLI whose `--help` is
  built from a hand-rolled registry (e.g. `commands::help::groups()`
  in claudine) MUST ship the registry row in the same atomic commit
  as the subcommand code. The orchestrator's path-by-path semantic
  grouping catches the obvious source/doc/test/snapshot rows but a
  small `cmd("new-name", "…")` row in the registry can slip through
  when the dominant paths are clearly the new subcommand's source
  file, doc, and tests. The reconciliation pass then surfaces the
  missed path as a still-staged `M `, and a single follow-up
  `fix(<area>):` commit is the cheapest fix. Verify the registry
  file's diff against the new subcommand name *before* dispatching
  the feat's group agent — the registry edit is a 4-line addition
  in the same module as other subcommand entries and is easy to
  miss when the agent is briefed by file path rather than by
  integration-point checklist.
- A developer WIP can leave an ` M` (working-tree-only change) by
  the time the orchestrator reconciles. The original staged-set
  snapshot at the prompt's start is the working list; an ` M` path
  that was NOT in the original list is the developer's active edit
  and belongs to a future batch. Do not stage it, do not commit it,
  do not flag it as a sibling-fix path; leave it alone and report
  the presence of unrelated working-tree changes in the summary so
  the operator knows it pre-dated the operation.
- A single-file-to-module-directory split (D old file + N A new sub-module
  files, where the new directory's `mod.rs` re-exports the sub-modules)
  must ship atomically in one commit. Splitting it lands broken code at
  every intermediate state: a D without any A's removes the module outright,
  a partial set of A's leaves callers importing old paths the D removed,
  and the surviving sub-modules cannot be reached because the `mod.rs`
  re-export hasn't landed. This is a stronger coupling than the
  staged-`R` rename case (lines 126-131) — the rename is a single D+A pair
  with both endpoints in one index fact, while the module split is a D+N
  where the N re-exports cohere only when all arrive together. The git
  index makes this look safe to split (each `A` is independently staged),
  but every intermediate commit fails to compile. Group the D, the
  `mod.rs`, and every sub-module `A` into a single `--only` pathspec
  alongside the call-site `M` updates that consume the new module path,
  even when the sub-modules individually look independent.
- CLI test files (`cli/tests/cli.rs`, `cli/tests/snapshots.rs`, etc.) that
  cover both focused subcommands AND aggregate output force the aggregate's
  library driver (`filesystem/repo/aggregate_view.rs` and friends) to ship
  in the same commit as the CLI tests, even when the aggregate driver is
  technically library code. The coupling is through the test's imports:
  the test deserializes or asserts on a `RepoAggregate` / aggregate JSON
  shape whose struct is owned by the library file, and the test will fail
  to compile (or test a stale shape) if the library file is committed
  separately. Splitting "library feat" and "CLI refactor" along the
  conventional `sniff/lib/**` vs `sniff/cli/**` boundary can lose this
  coupling; pre-flight `git grep -nE 'fn test_.*(aggregate|json)'` over
  the CLI test file reveals which library symbols the tests reference,
  and any of those symbols' defining file belongs with the CLI commit
  rather than the library one.
