---
$schema: feature-review.yaml
ready: false
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/default
created: 2026-09-25T13:55:37-07:00
spec: 2026-09-24-ux-improvements/spec.md
implemented: true
implemented_by: claude/opus
log: worktree/fixes/2026-09-24-ux-improvements/implementation-log.md
description: "A **fix** review of `2026-09-24-ux-improvements/spec.md`"
fix: 2026-09-24-ux-improvements/review-6.md
previous: 2026-09-24-ux-improvements/review-5.md
next: 2026-09-24-ux-improvements/review-7.md
findings:
    - title: Changes inside an untracked nested repository escape the handoff check and are deleted
      priority: high
---

# Review 6: Worktree UX improvements

## Verdict

**Not ready for production.** Review 5's remote-address finding is resolved. A separate, reproduced defect lets removal delete new or changed files inside an untracked nested repository after the first invocation approved different contents. This violates the specification's requirement to refuse when working content changes during the shell handoff.

This review changed only review documents and specification metadata. The reproduction used a disposable local repository and the current debug binary; no external repository was modified.

## Previous findings

| Review 5 unblocked finding | Assessment |
| --- | --- |
| An approved relative endpoint can resolve as another remote and delete from its push repository | Resolved. The implementation checks configuration from all scopes for a matching remote name and checks legacy remote files. Direct removal, handoff, and the final deletion guard use that protection. The direct, handoff, and command-line-configuration regression tests passed, as did the successful unambiguous-relative-address case. |

In the `worktree` package, [endpoint_reinterpretation](../../lib/src/remove/remote.rs:100), which checks whether Git would reinterpret the approved repository address, now covers remote names as well as URL rewrites. Its library tests cover legacy files and the final deletion guard. In `worktree-cli`, the [remote-address regression tests](../../cli/tests/remove.rs:1143) assert exit 3 and preservation of the worktree, registration, local branch, remote branches, and the unrelated repository's complete ref listing. These satisfy the previous finding's requested regression coverage.

Review 5 had no blocked findings and required no human decisions, so there were no findings to unblock. Refusing ambiguous remote addresses follows the already accepted safety contract; it does not require renewed approval. The later ignored-file and performance specifications remain separate work and are not treated as implemented changes to this review's contract.

## Unblocked Findings

### High: Changes inside an untracked nested repository escape the handoff check and are deleted

In the `worktree` package, [content_digest](../../lib/src/remove/inventory.rs:160), which supplies working-content evidence for removal approval, returns the constant `"dir"` for a directory. Git can report an untracked nested repository as a single `?? nested/` entry even with `--untracked-files=all`. Consequently, [Inventory::fingerprint](../../lib/src/remove/inventory.rs:109), which detects changes between the two removal invocations, records neither that directory's child paths nor their contents. The outer repository's index does not contain those children either.

In `worktree-cli`, [run_handoff](../../cli/src/commands/remove/mod.rs:584), which resumes removal after the shell moves, accepts the unchanged fingerprint and removes the directory. A nested repository is an ordinary possible worktree occupant, such as a locally cloned dependency or experiment. Its uncommitted files may be the only copy of that work.

**Reproduction, observed on this checkout:**

1. Create a disposable repository on `main` with one commit, and a linked worktree named `feat-x` on branch `feat/x` at the same commit.
2. Inside `feat-x`, create `nested/`, run `git init` there, and write `nested/notes` with `approved content`. No submodule registration is involved.
3. From `feat-x`, run `WT_SHELL_WRAPPER=1 wt remove feat-x --force-worktree` non-interactively. It exits 0 and prints the landing directory and handoff token. The report lists one uncommitted entry, `nested`.
4. Before the second invocation, replace `nested/notes` with `new work after approval` and add `nested/new-file` containing new work.
5. From the printed landing directory, run `wt remove --handoff <token>`.

**Observed:** the outer Git status is `?? nested/` before and after those edits. The second invocation exits 0, prints `Removed worktree` and `Deleted branch`, and deletes the entire worktree, including both changed files. The local `feat/x` branch is also gone.

**Expected:** exit 3 with nothing removed because the working content changed after approval. `--force-worktree` authorizes discarding the contents approved by the first invocation; the specification still requires the second invocation to detect intervening changes. Existing ordinary-file and staged-content tests enforce that rule for other paths.

**Required change:** include the child-path set and working contents of dirty directory entries in the fingerprint, including nested repositories, or refuse the handoff before mutation when their contents cannot be verified. A directory marker alone must not establish unchanged contents. Keep traversal deterministic, handle symlinks without following them outside the directory, and report inspection failures instead of silently accepting incomplete evidence. This concerns an **untracked dirty directory**, not the separately accepted rule to fingerprint only the set of ignored entries.

Review the associated documentation at the same time. The `collect_inventory` comment currently claims `-uall` lists every path individually; the reproduced nested-repository case disproves that claim. The fingerprint documentation also promises detection of new untracked files without describing this omission. Do not resolve that drift by weakening the specified safety guarantee.

**Verification needed: Level 1.** Add a library test proving that an edit and a newly added child each change the fingerprint when Git reports only the parent directory. Add a CLI handoff regression using a disposable nested repository: approve removal, mutate its contents, resume, and assert exit 3 with the worktree, registration, local branch, and new contents intact. Retain a successful unchanged-content case if nested-directory handoffs remain supported. No physical keyboard or terminal-rendering test is needed for this state-validation defect.

## Blocked Findings

None. The required change implements the existing handoff safety requirement and needs no new design decision.

## Requirement verification

Level 1 verifies logic and subprocess behavior; Level 2 verifies output and shell flows through a real terminal. This spec introduces no physical-key encoding, modifier, mouse, or hotkey behavior requiring Level 3. Injected answers to the existing prompts verify the specified flow without claiming to exercise an OS keyboard encoder.

| User-observable requirement | Verification present | Assessment |
| --- | --- | --- |
| Completion names, `base`, detached checkouts, ambiguity, and refusal to remove the base checkout | Level 1 resolver and CLI tests | Passed. |
| Safety tiers, PR source/head identity, independent force flags, dirty/ignored-file consent, branch retention, exit codes | Level 1 policy matrix, repository tests, and CLI tests | Passed. |
| Approved remote address, changed heads, multiple URLs, URL rewrites, remote-name collisions, deletion lease | Level 1 local-remote and CLI tests | Passed, including review 5's new cases. |
| Working content, staged content, branch state, token expiry/replay, and refusal after a changed handoff state | Level 1 library/CLI tests; Level 2 failure scenes | Existing cases passed; the untracked nested-directory case above is missing and fails in reproduction. |
| Report before prompts, one blank line, default answers preserve work | Level 1 policy tests and Level 2 tmux captures | Passed. |
| Dirty-tree glyphs, connector/file colors, count replacing more than ten paths | Level 1 rendering assertions and Level 2 styled captures | Passed. |
| Bash, zsh, fish move-first flow, fork-parent/base landing, subdirectory preservation, failed movement | Level 1 wrapper tests and Level 2 tmux shell scenes | Passed. |
| PowerShell moves both directory states; a held Windows directory returns exit 4 without removal | Windows Level 1 wrapper tests and Level 2 console-screen tests | Appropriate tests exist; Windows execution not repeated in this review. |
| Local `--from`, existing/invalid/detached cases, fork records, explicit wrapper detection | Level 1 repository and CLI tests | Passed. |
| Caption/target selection, fork-parent rows, deleted parents, merge vocabulary, PR placement and cache age | Level 1 data tests/snapshots and Level 2 table captures | Passed. |
| Dot/badge colors, connectors, legend, current-row emphasis on light/dark backgrounds | Level 1 style tests and Level 2 styled-cell assertions | Passed. |
| Linked PR badge when supported and number-only fallback otherwise | Level 1 link/fallback assertions and Level 2 displayed badges | Existing accepted coverage retained. |
| Graph facts, lane/tag rules, origin/fork relationships, trimming and scale arithmetic | Level 1 worktree and component tests | Worktree tests passed; dependency component tests not rerun. |
| Actual graph pixels, narrow width, half-height cap, omitted-lane notice, table surviving image output | Level 2 private-Kitty screenshot and transmission assertions | Correct level exists. Not rerun: review 3 documented a blank screenshot capture limitation; earlier implementation records report passing captures. No fresh screenshot result is claimed. |
| Warm/cold gathering, full-command timing, unavailable/slow PR request, fresh-cache request suppression | Level 1 serial performance and request-count tests | Passed. |
| Four-provider PR normalization and renderer pie-color compatibility | Level 1 provider fixtures and renderer tests | Earlier coverage retained; dependency suites not rerun. |

No additional requirement was identified whose strongest existing test is at the wrong verification level. The finding is a missing Level 1 state case, not a demand for a higher tier. Cross-OS evidence and human sign-off are not readiness blockers in this review.

The new removal tests are compiled by the automatically discovered `remove` integration target and selected by the live Level 1 recipe. The Level 2 targets require `terminal-tests`; the live recipe and CI feature metadata enable it. The tier audit reports zero stranded tests.

## Validation and metadata

- `worktree/just test`: **312 passed**, 17 excluded by the area filter.
- `worktree/just test-perf`: **17 passed**, run serially.
- `BISCUIT_TEST_REQUIRED_BACKENDS=tmux just test-l2 level2_dirty level2_list level2_move level2_remove`: **9 passed**; backend proof records nine tmux executions, zero skips, and zero panics.
- `just check-tier-coverage worktree`: **zero stranded tests**.
- A disposable-repository reproduction confirmed successful deletion after edits and additions inside an untracked nested repository between handoff invocations.

Lint, Kitty screenshots, cross-OS runs, and dependency suites were not rerun. No source or test files were changed, no formatting command was run, and no commit was created.

The requested previous-review location under `prompts/_reviews` is absent. The existing review beside the spec was updated instead: `implemented: true` records the completed prior implementation, and `next` points to this review. The spec's `review_iterations` is now 6; it is not marked completed and remains in its current lifecycle directory.
