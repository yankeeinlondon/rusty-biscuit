---
$schema: feature-review.yaml
ready: false
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/default
created: 2026-09-25T11:45:31-07:00
spec: 2026-09-24-ux-improvements/spec.md
log: worktree/fixes/2026-09-24-ux-improvements/implementation-log.md
implemented: true
implemented_by: claude/opus
description: "A **fix** review of `2026-09-24-ux-improvements/spec.md`"
fix: 2026-09-24-ux-improvements/review-5.md
previous: 2026-09-24-ux-improvements/review-4.md
next: 2026-09-24-ux-improvements/review-6.md
findings:
    - title: An approved relative endpoint can resolve as another remote and delete from its push repository
      priority: high
---

# Review 5: Worktree UX improvements

## Verdict

**Not ready for production.** Both specific defects from review 4 have fixes and passing regression tests. However, the broader guarantee that remote deletion uses the approved repository still fails when the approved relative path also names a configured Git remote. Direct removal and move-first removal both reproduced deletion in the wrong repository with exit 0.

This review changed only review documents and specification metadata. Reproductions used disposable local repositories; no external repository was modified.

## Previous findings

| Review 4 unblocked finding | Assessment |
| --- | --- |
| Git can rewrite the approved deletion endpoint a second time | The reported URL-rewrite sequences are fixed. The implementation refuses matching `insteadOf` and `pushInsteadOf` rules before removal, checks again during handoff, and checks before pushing. The direct and handoff CLI regressions passed. Another form of Git endpoint interpretation remains unsafe, as detailed below. |
| Remote deletion can use the branch being deleted as its safety evidence | Resolved. In the `worktree` package, [classify](../../lib/src/remove/safety.rs:236), which decides whether branch deletion preserves work, now excludes the destination before every tier check. The lost-commit calculation also excludes the `origin/HEAD` alias. Library and CLI regressions passed for an upstream of `origin/main`, including handoff and independent surviving evidence. |

Review 4 had no blocked findings and required no human decisions. There were consequently no blocked findings to unblock before this implementation. Previously accepted design choices remain unchanged. The later ignored-file proposal in `2026-09-25-worktree-file` remains separate from this review's evaluation of the implemented consent flow.

## Unblocked Findings

### High: An approved relative endpoint can resolve as another remote and delete from its push repository

In the `worktree` package, [push_endpoints](../../lib/src/remove/remote.rs:53) returns origin's resolved push URL, which may be a relative path such as `approved`. [endpoint_rewrite_rule](../../lib/src/remove/remote.rs:68), which checks whether that string will change meaning when passed to Git, checks only URL-rewrite configuration. Git also interprets its repository argument as a configured remote name when one matches.

Consequently, [preflight_remote_deletion](../../lib/src/remove/remote.rs:135), which observes the branch before deletion, can query the matching remote's fetch URL, while [delete_remote_branch](../../lib/src/remove/remote.rs:227), which performs the deletion, uses that remote's different push URL. No `url.*` rule is necessary. The lease checks a commit identity; it does not distinguish repositories that hold the same commit.

**Reproduction, direct removal:**

1. Create a base checkout on `main` and a clean linked checkout named `feat-x` on `feat/x`, both at the same commit.
2. Create bare repositories at `<base>/approved` and `<base>/other`, each holding `main` and `feat/x` at that commit.
3. In the base checkout, configure:

   ```sh
   git remote add origin approved
   git remote add approved /absolute/path/to/base/approved
   git remote set-url --push approved /absolute/path/to/base/other
   git remote get-url --push origin
   ```

   The final command prints `approved`.

4. Run `wt remove feat-x --force-remote` from the base checkout, non-interactively.

**Observed:** exit 0. The report identifies `Origin (approved)` and claims successful deletion. The worktree and local branch are removed. `<base>/approved` retains `feat/x`; `<base>/other` loses it. This is a deletion from a repository the report did not identify.

**Reproduction, move-first removal:** start with only `origin` configured to the relative path `approved`. From inside `feat-x`, run `WT_SHELL_WRAPPER=1 wt remove feat-x --force-remote` and retain the landing directory and token. Before running the second invocation, add the remote named `approved` and its different push URL as above. Origin's resolved push URL still prints `approved`. Run `wt remove --handoff <token>` from the landing directory.

**Observed:** the same wrong-repository deletion, exit 0, and success messages. In `worktree-cli`, [run_handoff](../../cli/src/commands/remove/mod.rs:563), which validates the stored approval after the shell moves, sees an unchanged endpoint string and an unchanged head. The new rewrite-rule check finds no matching rule, so removal proceeds.

**Required change:** ensure that the recorded endpoint is used as a literal repository address by both observation and deletion, or refuse an endpoint that Git would interpret as another remote before removing anything. Apply the protection to direct removal, handoff, and the final deletion guard. Account for Git's own remote resolution rather than only the ordinary `remote.<name>.url` case. Preserve the existing URL-rewrite and multiple-push-URL protections. Update the documentation that currently claims absence of a rewrite rule guarantees literal interpretation.

**Verification needed:** Level 1 CLI regressions with local bare repositories for both sequences above. Assert the unrelated repository's complete ref listing is unchanged. If the implementation refuses, assert exit 3 and that the worktree, registration, local branch, and both remote branches remain intact. Retain a successful case for a relative endpoint that is unambiguous. The existing tests cover URL rewriting and explicit push URLs, but not a resolved endpoint that also names a remote.

## Blocked Findings

None. The required change follows the already accepted requirement that deletion address the approved repository; no new human decision is needed.

## Requirement verification

Level 1 verifies logic and subprocess behavior. Level 2 checks output through an actual terminal. This specification introduces no physical-key encoding, modifier, mouse, or hotkey requirement needing Level 3. Tests that answer the existing prompts through tmux exercise the required prompt flow without claiming to test a physical keyboard encoder.

| User-observable requirement | Verification present | Review assessment |
| --- | --- | --- |
| Branch and directory completion names, `base`, detached checkouts, ambiguity, and refusing base removal | Level 1 resolver and CLI tests | Area suite passed. |
| Safety tiers, PR source/head identity, independent force flags, dirty and ignored-file consent, branch retention, exit codes | Level 1 policy, repository, and CLI tests | Area suite passed, including the repaired default-upstream case. |
| Remote destination approval, changed remote heads, multiple URLs, URL rewrites, lease protection | Level 1 local-remote tests | Existing cases passed; the missing remote-name case above blocks readiness. |
| Changed working content, staged content, branch state, token expiry/replay, failed shell movement | Level 1 handoff tests and Level 2 failure scenes | Passed. |
| Report before prompts, one blank line, default answers preserve work | Level 1 policy tests and Level 2 tmux captures | Passed. |
| Dirty-tree glyphs, file colors, connectors, replacement by a count above ten paths | Level 1 rendering tests and Level 2 styled captures | Passed. |
| Bash, zsh, and fish move-first flow, base/fork-parent landing, subdirectory preservation | Level 1 wrapper tests and Level 2 shell scenes | Passed. |
| PowerShell moves both directory states; held Windows directory returns exit 4 without removal | Windows Level 1 wrapper tests and Level 2 console-screen tests | Appropriate coverage retained; not rerun on Windows in this review. |
| Explicit local `--from`, invalid/existing/detached cases, fork records, wrapper detection | Level 1 repository and CLI tests | Area suite passed. |
| Table captions, selected comparison target, fork-parent rows, deleted parents, merge vocabulary, PR placement and cache age | Level 1 data tests and snapshots, Level 2 displayed table | Passed. |
| Table dot/badge colors, connectors, legend, current-row emphasis on light/dark backgrounds | Level 1 style assertions and Level 2 styled-cell assertions | Passed. |
| Linked PR badge when supported, number-only fallback otherwise | Level 1 OSC 8/fallback assertions and Level 2 displayed badges | Passed for the accepted contract. |
| Graph facts, lane/tag rules, origin/fork relationships, trimming, scale arithmetic | Level 1 worktree and component tests | Worktree tests passed; dependency component results retained from earlier reviews. |
| Actual graph pixels, narrow width, half-height cap, omitted-lane notice, table surviving image output | Level 2 private-Kitty screenshots and transmission checks | Correct level exists. Not rerun; review 3's blank screenshot capture limitation remains unresolved by this review. Earlier implementation records report passing captures. |
| Warm/cold gathering, full-command timing, unavailable/slow PR lookup, fresh-cache request suppression | Level 1 serial performance and request-count tests | See validation below. |
| Four-provider PR normalization and renderer pie-color compatibility | Level 1 provider fixtures and renderer tests | Coverage retained from earlier reviews; dependency suites not rerun. |

No additional requirement was found with coverage at the wrong verification level. The finding above requires a missing Level 1 state case. Cross-OS evidence and human sign-off are not used as readiness blockers.

The new removal regressions are compiled by the automatically discovered `remove` integration target and were selected in Level 1. Level 2 targets require `terminal-tests`; both the live recipe and CI feature metadata enable it. The tier audit found zero stranded tests.

## Validation and metadata

- `worktree/just test`: **307 passed**, 17 excluded by the area filter.
- `worktree/just test-perf`: **17 passed**, run serially.
- `BISCUIT_TEST_REQUIRED_BACKENDS=tmux just test-l2 level2_dirty level2_list level2_move level2_remove`: **9 passed**; backend proof records nine tmux executions, zero skips, and zero panics.
- `just check-tier-coverage worktree`: **zero stranded tests**.
- Two disposable-repository probes reproduced wrong-repository deletion, through direct removal and through handoff after adding a colliding remote name.

Lint, Kitty screenshots, cross-OS runs, and dependency suites were not rerun. No source or test files were changed, no formatting command was run, and no commit was created.

The requested previous-review path under `prompts/_reviews` does not exist. The existing review beside the spec was updated instead: `implemented: true` records the implementation attempt, and `next` points to this review. The spec's `review_iterations` is 5; it is not marked completed.
