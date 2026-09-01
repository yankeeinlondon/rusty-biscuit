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

- Do NOT use `git commit --allow-empty -m "..."` (or any variant of
  `--allow-empty`) to pre-flight that signing works in a non-interactive
  session. `--allow-empty` overrides git's "no changes" safety check; it
  does NOT bypass the index. If the index has staged paths, the empty
  commit will land ALL of them, signed, with the supplied message — exactly
  the opposite of what an `allow-empty` pre-flight intends. Cheap, safe
  signing pre-flights are read-only: `git log -5 --pretty='%G? %s'` (recent
  commits with good `G` signatures imply the gpg-agent has the signing
  subkey cached for this session), or `gpg-connect-agent 'getinfo
  passphrase' /bye` to ask the agent directly. If those answers are
  inconclusive and a real test is needed, checkout an orphan branch,
  commit there, verify, delete the branch, and return to main. Recovery
  from an accidental `git commit --allow-empty` that swept the index is
  the documented `git update-ref HEAD <old> <new>` pattern (CAS-advance);
  the orphan stays reachable from `git log --reflog` until GC.
- Use `git status --short`, `git diff --staged --name-status`,
  `git diff --staged --stat`, and `git diff --staged` to review the exact
  staged set.
- Before committing an assigned path, check `git diff -- <path>`. A nonempty
  result means the working tree differs from the staged snapshot.
- **`Cargo.lock` is coupled to its declaring `Cargo.toml`.** A staged
  `Cargo.lock` adding a dep that is not yet declared by any `Cargo.toml` in
  the same commit is an orphan lock entry. Detect before committing:
  `git show :Cargo.lock | grep '"<dep-name>"'` then `git show :<cargo-toml>`
  to confirm the dep is declared in the manifest. If your sibling group
  owns the matching `Cargo.toml` change, expand your pathspec to include
  the manifest (and any other manifest in the dep closure) rather than
  committing `Cargo.lock` alone. Lockfile-without-manifest is orphan;
  manifest-without-lockfile produces a tree that does not match the new
  manifests until a `cargo metadata` regenerates it. In a parallel batch,
  commit the producer's `Cargo.toml` before any sibling commits the
  matching `Cargo.lock` line.
- Do not commit unresolved conflict markers. If staged content contains them,
  leave that group staged and report it.
- Scan staged blobs for all diff3 conflict-marker forms, including the base
  marker `|||||||`; partial resolutions can leave it behind after deleting the
  `=======` separator and `>>>>>>>` terminator.
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
- **`git status --short`'s `R` only proves the index has two similar-content
  endpoints — it does not prove either endpoint is the "right" one.** Git's
  rename detector infers `R` at display time from blob similarity; the index
  itself stores the pair as independent M (or D) + A entries. When the caller
  stages a rename via `git add` on each endpoint rather than an explicit
  `git mv`, `git commit --only -- <old> <new>` commits each endpoint's blob
  independently, and a similarity-detected "rename" can land in the tree as a
  modify+add pair with two completely different contents at the two paths.
  Confirmed in a 5-group batch where `git status --short` showed
  `R  prompts/_reviews/.../review-1.md -> claudine/fixes/.../review-1.md`
  but the actual staging held a fresh review at the old path (different
  findings, different verdict) and the implementation-flipped review at the
  new path; the commit's `git show --name-status` came back `M` + `A` with
  different blobs, not `R`. **Before relying on a staged `R`:** inspect
  `git ls-files -s <old> <new>` (compare the stage-0 blob hashes) or
  `git diff --cached -- <old> <new>` to confirm the OLD endpoint is a
  deletion (or a zero-content modification) of the expected blob, not an
  unrelated substitution. If the blobs diverge, surface the discrepancy to
  the orchestrator and do not commit both endpoints as a "rename" — the
  resulting tree preserves both contents and the developer's intent is
  ambiguous. The hard rule "do not amend, do not reset, do not restage peers"
  still applies; report and let the developer decide.
- Put Git options before `--`. For very large explicit path lists, use
  `--pathspec-from-file` and inspect the generated list.
- Feed commit messages through `-F -` with a single-quoted heredoc, or through a
  checked temp file for long bodies. Prefer the temp-file pattern when the
  commit may need to be retried (lock contention): an in-place heredoc does not
  compose cleanly with a retry loop inside a single shell invocation, while a
  prewritten file lets the loop reuse the same body verbatim. A bare
  `git commit -- <paths> <<EOF` can open the configured editor and block.
  checked temp file for long bodies. A bare `git commit -- <paths> <<EOF` can
  open the configured editor and block.
- When staging a message in a temp file via the `Write` tool, avoid `$$` (PID
  placeholder) in the filename. The `Write` tool stores shell metacharacters
  literally rather than expanding them, so `/tmp/commit_msg_$$.txt` lands with
  the literal `$$` in the path while the subsequent `git commit -F
  /tmp/commit_msg_$$.txt` shell-expands to a different PID and the file is not
  where the commit looks. Use a static name (`/tmp/commit_msg.txt`) or echo
  the agent's actual `$$` value before writing.
- **Parallel-batch temp-file race on `/tmp/commit_msg.txt`.** A static shared
  name like `/tmp/commit_msg.txt` is a single hot slot when N sub-agents
  commit concurrently. Each agent's `Write` of its own body overwrites the
  previous one; whichever `git commit -F /tmp/commit_msg.txt` reads last
  wins, and the others' trees ship with the wrong subject/body. Observed in a
  4-group batch: the codebook `chore(codebook): …` commit landed with the
  homelab `docs(homelab): add unifi product documentation` body because the
  codebook agent's write lost the race. Signatures are unaffected, trees are
  correct, but two commits in the log share a subject. **Mitigation in the
  brief:** when the orchestrator dispatches a parallel commit batch, give
  each sub-agent a unique message filename derived from its scope (e.g.
  `/tmp/commit_msg_codebook.txt`, `/tmp/commit_msg_homelab_unifi.txt`,
  `/tmp/commit_msg_homelab_justfile.txt`, `/tmp/commit_msg_ha_skill.txt`).
  The `Write`/`-F` round trip still composes with the lock-retry loop and
  costs nothing; the static name only made sense in single-agent flows.
  **Recovery from a wrong-body commit:** per the existing rule, do not amend
  in batch; report the wrong body and let the developer rewrite or
  `update-ref`-rewind after the batch settles.
- `--` between `-F -` and the pathspec list is mandatory, even with `--only`.
  Without it, git parses the first path as a subcommand (e.g. `git commit
  --only -F - claudine/lib/...` tries to invoke `git-claudine`) and may also
  re-enter the working directory as a relative path, triggering a permission
  denied error. Always: `git commit --only -F - -- path1 path2 … <<'MSG' …`.
- Do not place messages containing backticks, dollar signs, or other shell
  metacharacters in a double-quoted `-m` argument.
- **`--only` + `-F -` argument order.** In a path-restricted commit that reads
  the message from stdin, place `-F -` BEFORE `--only`, not after:
  `git commit -F - --only -- <paths>` works; `git commit --only -F - -- <paths>`
  makes `--only` absorb `-F` as a pathspec and git returns
  `error: pathspec '-F' did not match any file(s) known to git`. The heredoc
  payload is then never read. Pair this with the single-quoted `<<'COMMIT_MSG'`
  delimiter above.
- **`-F -` heredoc subject/body separator.** A commit message fed via `-F -`
  MUST contain a blank line between the subject and the body. Without it,
  `git commit` treats the entire input as one subject line — the
  conventional-commit subject is lost, the first body bullet is absorbed into
  a multi-hundred-char "subject", and the rest of the bullets are silently
  dropped (the resulting `git log -1 --format=%B <hash>` shows a one-line
  blob). The `-F -` argument-ordering and heredoc-quoting bullets above do
  not cover this; it is a message *structure* rule, not a shell-expansion or
  arg-parsing rule. Recovery is `git reset --soft HEAD~1` (preserves the
  index exactly so a re-issued commit picks up the same staged snapshot),
  followed by a re-issued commit whose stdin begins with the conventional
  subject line, then a blank line, then the body bullets. Verify recovery
  with `git log -1 --format=%B <hash>`.

## Credential and Signing Blockers

- The system prompt warns against running `gpg` / `ssh-add` / credential
  helpers directly. The same hang hazard applies when parent config already
  enables signing — `commit.gpgsign=true` (set globally or per-repo) makes
  every `git commit` silently invoke `gpg-agent` and block on `/dev/tty`
  for the passphrase, even though the sub-agent did not opt in. Pre-flight
  with `git config --get commit.gpgsign` and `git config --show-origin
  --get commit.gpgsign`; if truthy, override per invocation:
  `git -c commit.gpgsign=false commit --only -F - -- <paths>`. Do not
  `git config --unset` or otherwise rewrite repo config from a sub-agent.
- Always export `GIT_TERMINAL_PROMPT=0` before any git invocation in a
  non-interactive session, even when no prompt is expected. Cheap insurance
  against credential helpers opening `/dev/tty` for HTTP proxies, push
  remotes, etc.

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

## Merge State

- `git commit` refuses `--only` (and any `-- <pathspec>` restriction) while a
  merge is in progress: `fatal: cannot do a partial commit during a merge.`
  Even the sub-agent's narrow pathspec cannot restrict scope when
  `.git/MERGE_HEAD` exists. Detect with `git status | grep -i merge` or
  `test -f .git/MERGE_HEAD` BEFORE running any `git commit --only`; if a merge
  is open, the only path is either to commit the whole merge as a single
  commit or to drop into plumbing.
- Plumbing bypass (`commit-tree` + `update-ref`) does NOT honor
  `commit.gpgsign=true` even when the global config says to sign — the
  resulting commit has `N` (no signature) and the user's required-signing
  contract is silently violated. When plumbing around a merge-state block,
  pass `-S <key>` (or set `GIT_COMMIT_SIGN=1`) explicitly so the produced
  commit lands signed.
- Before dispatching parallel commit work, check `git log --oneline -5` (or
  recent commits for the expected subject) for a wrapper-pre-staged merge
  that already resolved the change. A wrapper that ran `git pull --no-edit`
  and pre-resolved conflicts will leave `MERGE_HEAD` in place AND land all
  staged files as a single merge commit BEFORE any sub-agent runs. The
  sub-agent prompt is then stale with respect to actual repo state. If
  evidence of a recent merge of the same change exists, abort the parallel
  dispatch — the parallel group has nothing left to do.
- When a merge-state commit absorbs an out-of-scope group, the commit message
  must explicitly name the absorbed scope and flag the parallel group as
  redundant, so reviewers see the wider scope and do not expect a follow-up.

## Commit Messages

- Follow recent repository history and the prompt's Conventional Commit format.
  Subjects use lowercase after the colon and stay under 72 characters.
- Use `planning` for physical moves into `_completed` or out of `_unscheduled`,
  AND for review-cycle doc commits inside a fix/review directory: a cycle-N→N+1
  in-place edit (`log.md` verification entry, `review-N.md` flipping
  `implemented: true` and pointing at `next`, the new `review-(N+1).md`,
  `spec.md` bumping `review_iterations`) ships as
  `planning(<area>): close <fix> cycle N, open cycle N+1`. `main` is the
  authority — this is the established convention for the `redundant-walk`
  and `invalid-frontmatter` fix cycles (see `4c903c586`, `152ea6b84`,
  `690b2ecc3`); follow the most recent sibling commits, not a generic
  "in-place edits are docs" rule.
- When drafting a cycle-close commit body, the diff's literal wording matters.
  Paraphrasing too aggressively risks inventing commitments the staged content
  did not make (e.g. "smoke test failed" vs. "smoke attempt was interrupted by
  host load before any benchmark case ran" are materially different). Quote or
  paraphrase only what the diff actually says; if a section is too thin to
  characterize, summarize the size and the file/line range rather than
  speculating on its content.
- `planning` commits may have zero source diff. The fix-review pattern uses
  verification iterations to re-audit a prior cycle's implementation rather than
  redo it, so a cycle-N log/spec/review update can ship with no Rust changes.
  Treat such commits as valid cycle iterations, not no-ops.
- Describe the semantic change, not Git's similarity score or mechanics.

## Concurrency

- GitNexus `detect_changes` against a shared staged worktree reports aggregate
  symbols, processes, and risk for sibling groups as well as the group under review.
  A HIGH/CRITICAL result for a docs-only group may therefore describe unrelated
  staged runtime work; review the assigned diff and path-limited commit shape before
  treating the aggregate rating as group-local risk.
- Parallel groups must have disjoint paths. If one group introduces a module,
  dependency, or symbol consumed by another group, commit the producer first.
- Adding new variants to a non-`non_exhaustive` enum couples the producer and
  every consumer whose match must update. Producer-first leaves consumers with
  non-exhaustive matches; consumer-first references variants that do not yet
  exist. Combine producer and all matching consumers into a single semantic
  group, even if the commit crosses package boundaries; drop the scope in the
  message (`perf:` instead of `perf(sniff):`) to acknowledge the cross-area
  span. The producer-first rule above assumes the additions are
  backward-compatible (e.g., new items behind `#[non_exhaustive]`, trait
  methods, struct fields with defaults).
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
- In a non-interactive session, gpg-agent pinentry is the most likely commit
  failure mode. Before committing, sanity-check `git config --get commit.gpgsign`
  (or repo-local `commit.gpgsign` truth) and `pgrep gpg-agent`; if the agent is
  alive but the signing-subkey passphrase is not cached, `git commit` will
  block waiting for a TTY. Verify the agent has the key cached (or commit is
  deliberately unsigned) before dispatching, and report any signing hang
  immediately rather than letting the agent time out.
- Never bypass repository hooks with `--no-verify`, `-c core.hooksPath=...`, or
  equivalent overrides. An instruction not to run validations means do not
  launch them explicitly; configured commit hooks still run normally. If a hook
  blocks the commit, report the failure rather than suppressing it.
- Never amend or create follow-up fixup commits after a successful commit in a
  concurrent batch. Report the issue so the orchestrator can decide whether to
  accept, revert, or coordinate a rewrite.
- `git commit --only -- <pathspec>` commits only the named paths while preserving
  unrelated staged entries. Parallel agents still need disjoint pathspecs and
  lock retries, but successful sibling commits do not naturally narrow the
  remaining index.
- After all groups finish, reconcile `git status --short` against the original
  staged set. Any staged path left behind belongs to a failed or unassigned
  group and must be reported before taking further action.
- Git lock failures are transient contention. On index/ref lock contention, do
  not remove the lockfile; wait 1–3 seconds and retry the identical commit up
  to five times. A concurrent worker may own the lock.

- Run commands from the inherited worktree root. Do not change to a guessed
  repository path, and do not push commits.
- In zsh wrappers, avoid special variable names such as `status` and `path`.
- `git commit --only -F - -- <pathspec...>` is safe under concurrent index
  churn: when another sub-agent's commit lands between this agent's read and
  its commit, the `--only -- <paths>` form commits only the named paths
  regardless of intermediate index changes, so the agent never accidentally
  sweeps in files that left its assigned set during the gap. Confirmed in a
  three-group batch where the planning-archive group landed last while the
  docs-only and rustdoc-only groups had already moved their files out of the
  index.

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
- **Active-file iteration race.** A staged file that the caller is actively
  editing will drift between `git add`, `git status`, and `git commit`. A
  dispatch loop (stage → sub-agent → see `MM` → re-stage → re-dispatch) can
  never catch a stable snapshot while edits arrive faster than the agent
  overhead. Detect by `stat -f '%Sm' <path>` (mtime in the last few seconds)
  or by repeated `MM` reports across two dispatches. When the file is being
  actively iterated and the working tree is a clean superset of staged
  intent, stage it once and commit directly as the orchestrator in a single
  shell — do not keep re-dispatching. Reserve sub-agents for stable snapshots.
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
- A sub-agent given a pathspecs **file** plus an `xargs -I {} git commit ...
  < paths.txt` command shape will produce **N stacked commits** (one per
  line) instead of one commit. The shape that says "commit a list of
  paths" is not `xargs` over a per-line list — it is one
  `git commit --only -F body -- p1 p2 ... pN` invocation, or
  `git commit --only -F body --pathspec-from-file=<file>`. Observed in a
  3-group batch: a 35-path refactor landed as 35 stacked `refactor(claudine):
  plumb request-scoped FileResolutionContext through composition` commits.
  The path set, tree, and subject were correct in every one; the batch
  shape was wrong.
- **Mitigation in the brief.** When the orchestrator wants one commit, the
  brief must say, explicitly, "one `git commit --only -F body -- p1 p2 ...
  pN` invocation with all paths as positional arguments; never `xargs` per
  path; never per-path loops". For very long path lists use
  `--pathspec-from-file=<file>` and a single invocation, not a shell loop.
- **Recovery from N stacked commits when the orphans are yours.** If the
  wrong-shape commits are entirely the sub-agent's work (not the
  developer's), `git update-ref HEAD <old> <new>` (note: ref, newvalue,
  oldvalue — the order differs from `git reset`) is the documented
  CAS-advance pattern. It is the non-destructive equivalent of
  `git reset --soft`: HEAD moves, the index is preserved, the working
  tree is preserved, and the orphaned commits stay reachable from the
  reflog / object database until GC. After the move, all of the formerly
  committed paths reappear in `git diff --staged --name-only` (because
  the index holds their post-commit state, which now differs from the
  new HEAD) and the orchestrator can recommit them as one commit. Verify
  the new HEAD before recommitting: `git log --oneline -3` and
  `git status --short`. The developer's *unstaged* changes are
  untouched by the move; only the staged snapshot, which is exactly what
  a soft reset would touch, is at issue, and the soft-reset is what
  `update-ref` performs.

## Verification

- A successful `git commit` exit status is authoritative for that invocation.
- Capture the new commit hash from `git commit` stdout and verify that hash with
  `git show --stat <hash>` or `git show --name-status <hash>`. Do not rely on
  `git log -1` after concurrent commits; HEAD may already have advanced.
- If a wrapper hides commit stdout, recover immediately with `git reflog -1`
  and verify that hash. Prefer unwrapped `git commit` so stdout is visible.
- When `commit.gpgsign` is true, follow `git show --stat <hash>` with
  `git verify-commit <hash>`. The commit exit status covers the index update
  but not the signature, so a misconfigured `gpg.program` or an expired
  signing subkey can ship an unsigned commit silently. `verify-commit`
  confirms the signature is `G` (good) rather than `B` (bad) or absent,
  which protects downstream tooling that depends on `git log --pretty=%G?`
  showing a good signature (e.g. `darkmatter/features/*` review-cycle
  tooling).
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
- A subagent report of "path no longer staged" or similar is often a positive
  signal that a parallel committer (developer or sibling subagent) already
  produced the commit. Before re-staging or re-committing, run `git log -3` (or
  search recent commits for the expected subject) to confirm the intended
  commit landed. Restaging after a parallel committer has already produced the
  commit produces a redundant second commit.
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
