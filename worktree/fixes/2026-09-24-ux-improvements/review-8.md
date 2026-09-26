---
$schema: feature-review.yaml
ready: false
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/default
created: 2026-09-25T15:05:14-07:00
spec: 2026-09-24-ux-improvements/spec.md
implemented: true
implemented_by: claude/opus
log: worktree/fixes/2026-09-24-ux-improvements/implementation-log.md
description: "A **fix** review of `2026-09-24-ux-improvements/spec.md`"
fix: 2026-09-24-ux-improvements/review-8.md
previous: 2026-09-24-ux-improvements/review-7.md
next: 2026-09-24-ux-improvements/review-9.md
findings:
    - title: Executable-bit changes after approval escape the removal handoff check
      priority: high
---

# Review 8: Worktree UX improvements

## Verdict

**Not ready for production.** Review 7's path-encoding finding is implemented. A separate omission in the same removal check allows a working file's executable bit to change after approval without invalidating that approval. The current binary then deletes the worktree and branch. This was reproduced in a disposable local repository on macOS, without a remote.

## Previous findings

| Review 7 unblocked finding | Assessment |
| --- | --- |
| Lossy path encoding lets changed symlinks pass the removal handoff | Implemented. Git status and index output remain bytes, Unix paths retain their exact bytes, and symlink targets and directory-child names enter length-prefixed fingerprint records without lossy conversion. Windows rejects status paths that cannot be decoded faithfully. Display-only decoding remains separate. |

In the `worktree` package, [git_from_bytes](../../lib/src/git.rs:143) supplies unchanged Git output to inventory collection, and [Inventory::fingerprint](../../lib/src/remove/inventory.rs:110) records the state approved for deletion. The updated comments accurately describe exact-byte identity and the remaining deliberate exclusion of ignored-file contents.

The new library tests check exact status paths and changed symlink targets at the root and inside a nested repository. In `worktree-cli`, the [non-UTF-8 path regressions](../../cli/tests/remove.rs:879) exercise both invocations: changed symlinks refuse with exit 3 and preserve the target, registration, and branch; an unchanged symlink succeeds. These ran successfully here.

Three filename-dependent test bodies return early when the filesystem refuses non-UTF-8 names. Their macOS pass results are not evidence that those filesystem cases executed. The implementation log records Linux passing runs and deliberate fail-before runs for those cases; those are prior implementation evidence, not new executions by this reviewer. The Windows rejection test likewise was not executed locally.

Review 7 had no blocked findings or pending human decisions. Nothing needed to be unblocked. Keeping exact bytes and using length-prefixed fields follow the existing safety requirement and require no new design approval. The separately scheduled ignored-file behavior and performance work are not treated as requirements added by this review.

## Unblocked Findings

### High: Executable-bit changes after approval escape the removal handoff check

In the `worktree` package, [entry_digest](../../lib/src/remove/inventory.rs:251), which records each working file for removal approval, uses metadata to distinguish a regular file from a symlink or other object, but hashes only a regular file's bytes. It does not record whether the working file is executable. The fingerprint includes the **index's** modes, but changing permissions without staging does not change the index.

This matters when a tracked file is already modified: changing its executable bit leaves its porcelain status at ` M`, leaves its bytes unchanged, and leaves its index entry unchanged. All inputs currently checked by the handoff remain equal even though Git recognizes an additional change. Making a script executable is meaningful work, and the spec promises that a change between approval and removal causes refusal.

**Reproduced against `target/debug/wt` from this checkout:**

1. Initialize a disposable repository on `main`, with a committed `run.sh` at mode `100644`, and set `core.filemode=true`.
2. Add a linked worktree `feat-x` on branch `feat/x` at that commit.
3. Edit `run.sh` in the linked worktree without staging it. Leave its permissions at `0644`.
4. From that worktree, run `WT_SHELL_WRAPPER=1 wt remove feat-x --force-worktree` with redirected stdin. Save its handoff token. The command exits 0 and removes nothing.
5. Change only the script's mode to `0755`. `git diff --summary` now reports `mode change 100644 => 100755 run.sh`. Porcelain status is still ` M run.sh`.
6. From the base repository, run `wt remove --handoff <token>`.

**Observed:** exit 0, `Removed worktree feat-x`, and `Deleted branch feat/x`. The directory and branch no longer exist.

**Expected:** exit 3, with the worktree, registration, local branch, script bytes, and new executable mode preserved. The first invocation's force flag authorized the state at that time; it did not authorize subsequent changes.

**Required change:** include the working file's Git-relevant executable state in the removal fingerprint on platforms that support it, including files visited inside dirty directories. Keep the representation explicit and preserve the existing byte-faithful path handling and no-symlink-traversal behavior. This can use metadata already read by the fingerprint; it does not require another Git subprocess. Avoid introducing timestamps into the comparison, since those can change without new work. Update the relevant fingerprint comments alongside the behavior.

**Verification needed: Level 1.** Add a library regression and a CLI regression for the tracked, already-modified script above. The CLI test must change only the mode between invocations and assert exit 3 plus preservation of the file's bytes and executable state, worktree registration, and branch. Cover a mode change inside an untracked nested repository as well, since that path shares the same digest helper. Keep an unchanged-mode success case. Gate Unix permission manipulation by platform, set `core.filemode=true` in the fixture, and ensure every test is compiled and selected by the live Level 1 recipe. No physical-key or real-terminal test is needed for this state-comparison defect.

## Blocked Findings

None. Rejecting a newly changed executable bit follows the existing approval contract; no human design decision is needed.

## Requirement verification

The table distinguishes tests present from tests executed in this review. Level 1 verifies state, commands, and policy. Level 2 verifies rendering and shell flows inside a real terminal. This specification introduces no modifier, hotkey, paste, IME, mouse, or physical-key encoding requirement needing Level 3; injected prompt answers verify the removal flow without claiming to exercise a terminal's keyboard encoder.

| User-observable requirement | Strongest relevant verification present | Review assessment |
| --- | --- | --- |
| Completion names, `base`, detached checkouts, ambiguous names, and base-removal refusal | Level 1 resolver and CLI tests | Passed locally. |
| Safety tiers, independent force flags, dirty/ignored consent, branch retention, exit codes, and merged-branch removal | Level 1 policy, repository, and CLI tests | Passed locally. |
| PR source/head identity; live remote heads, approved push destination, rewrite/name collisions, and deletion lease | Level 1 stubs and local-remote tests | Worktree tests passed; provider suites not rerun. |
| Handoff state/index checks, expiry/replay, changed risks, nested contents, and exact-byte symlink identity | Level 1 library and CLI tests; Level 2 failure scenes | Existing tests passed. Executable-bit changes are an uncovered Level 1 case and fail the reproduction above. |
| Report precedes questions, one blank line, default answers preserve work | Level 2 tmux captures and Level 1 policy tests | Passed locally. |
| Dirty-tree glyphs/colors and count replacing more than ten paths | Level 2 styled capture and Level 1 rendering tests | Passed locally. |
| Bash, zsh, fish move-first flow; fork-parent/base landing, subdirectories, and failed movement | Level 2 tmux shell scenes and Level 1 wrapper tests | Selected terminal suite passed; shell scenes iterate installed shells. |
| PowerShell moves both directory states; held Windows directory refuses before deletion | Windows Level 2 console-screen tests and Level 1 wrapper tests | Tests exist at the appropriate level; Windows execution not repeated here. |
| Local `--from`, invalid/existing destinations, detached source, fork records, and explicit wrapper detection | Level 1 repository and CLI tests | Passed locally. |
| Caption/target choice, parent/deleted-parent rows, merge vocabulary, PR placement, and cached age | Level 1 data/snapshot tests and Level 2 displayed table | Passed locally. |
| Dot/badge/connector colors, legend, current-row emphasis on light/dark backgrounds | Level 2 assertions on rendered cell styles and Level 1 styles | Passed locally. |
| Linked PR badges where supported and number-only fallback | Level 1 link/fallback assertions and Level 2 displayed badges | Existing coverage retained; local worktree suites passed. |
| Graph facts, lanes/tags, origin/fork relationships, trimming, and scale arithmetic | Level 1 worktree/component tests | Worktree tests passed; component suites not rerun. |
| Actual graph pixels, narrow width, half-height cap, omitted-lane notice, and intact table | Level 2 private-Kitty screenshot/transmission tests | Appropriate tests exist; not rerun. A tmux graph-path test is not treated as pixel-rendering proof. |
| Warm/cold gathering, full-command time, unavailable/slow PR service, and fresh-cache request suppression | Level 1 serial performance/request-count tests | All 17 performance tests passed. |
| Four-provider PR normalization and renderer pie-color compatibility | Level 1 provider fixtures and renderer tests | Prior coverage retained; dependency suites not rerun. |

No further requirement was identified whose strongest existing test is at the wrong level. The readiness blocker is missing state coverage within Level 1, not lack of cross-OS execution evidence or human sign-off.

The new library tests compile in the library unit-test target. The CLI regressions compile in the automatically discovered `remove` integration target; this package does not disable automatic test discovery. Their names select Level 1. Terminal targets require `terminal-tests`, enabled by the live Level 2 recipe and CI metadata. The tier audit reports zero stranded tests.

## Validation and metadata

- `worktree/just test`: **326 passed**, 17 excluded by the area filter. The filename-dependent early returns described above are included in the pass count.
- `worktree/just test-perf`: **17 passed** with the serial recipe.
- `BISCUIT_TEST_REQUIRED_BACKENDS=tmux just test-l2 level2_dirty level2_list level2_move level2_remove`: **9 passed**; backend proof reports nine tmux executions, zero skips, zero panics.
- `worktree/just lint`: **passed** for both packages.
- `just check-tier-coverage worktree`: **zero stranded tests**.
- Disposable-repository executable-bit reproduction: **confirmed removal after a post-approval mode change**.

Kitty screenshot tests, cross-OS tests, and dependency suites were not rerun. This review changed no implementation or test files, ran no formatting command, and created no commit in this repository.

The requested previous-review location under `prompts/_reviews` does not exist. The existing review beside the spec was updated instead: `implemented: true` and `next: 2026-09-24-ux-improvements/review-8.md`. The specification now has `review_iterations: 8`; it is not marked completed and remains in its current lifecycle directory.
