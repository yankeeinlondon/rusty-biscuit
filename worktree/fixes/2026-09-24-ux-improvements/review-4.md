---
$schema: feature-review.yaml
ready: false
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/default
created: 2026-09-25T11:11:35-07:00
spec: 2026-09-24-ux-improvements/spec.md
implemented: true
implemented_by: claude/opus
log: worktree/fixes/2026-09-24-ux-improvements/implementation-log.md
description: "A **fix** review of `2026-09-24-ux-improvements/spec.md`"
fix: 2026-09-24-ux-improvements/review-4.md
previous: 2026-09-24-ux-improvements/review-3.md
next: 2026-09-24-ux-improvements/review-5.md
findings:
    - title: Git can rewrite the approved deletion endpoint a second time
      priority: high
    - title: Remote deletion can use the branch being deleted as its safety evidence
      priority: high
---

# Review 4: Worktree UX improvements

## Verdict

**Not ready for production.** The staged-content finding is resolved. The remote-repository finding has the requested implementation and passing regressions, but Git's URL rewriting can still send deletion to an unapproved repository. A separate safety-classification defect can delete both local and remote copies of unique work without `--force-branch`.

Both defects were reproduced through the current `wt` binary using disposable repositories and local bare remotes. No external remote was changed. This review changes only review metadata and this document.

## Previous findings and author decisions

| Previous unblocked finding | Assessment |
| --- | --- |
| Remote deletion approval does not identify the remote repository | Partially resolved. The approval now stores the resolved push URL, the second invocation compares it, separate push URLs are observed, and multiple push URLs are refused before removal. All four new CLI regressions pass. However, passing the resolved URL back to Git permits another rewrite; the first finding below reproduces deletion in a different repository despite an unchanged approved endpoint. |
| Handoff fingerprint misses changes to staged content | Resolved. In the `worktree` package, [Inventory::fingerprint](../../lib/src/remove/inventory.rs:112), which checks for changed files before completing removal, includes the logical index listing with object identities, modes, paths, and conflict stages. The library regression and CLI regression both pass for restaging while status remains `MM` and working-file bytes remain unchanged. Failure to read the index propagates as an error. |

Review 3 had no blocked findings. Its two human-review items were answered in its “Author decisions” section before the latest implementation: the author confirmed the proposed implementation choices, with remote approval amended to identify the deletion endpoint, and retained graphs below 80 columns. The specification and implementation log record those decisions. They do not need another confirmation.

The note about future ignored-file behavior belongs to `2026-09-25-worktree-file`; this review evaluates the implemented consent behavior described by the current specification's removal flow.

## Unblocked Findings

### High: Git can rewrite the approved deletion endpoint a second time

In the `worktree` package, [push_endpoints](../../lib/src/remove/remote.rs:41) obtains the resolved destination using `git remote get-url --push --all origin`. The same package's [delete_remote_branch](../../lib/src/remove/remote.rs:164), which performs the approved deletion, then passes that resolved string to `git push`. Git applies URL rewriting to this new argument too. In particular, `pushInsteadOf` applies to the push but not to the preceding live-head query, so the query and deletion can address different repositories.

The endpoint comparison in `worktree-cli`'s [run_handoff](../../cli/src/commands/remove/mod.rs:555), which resumes removal after the shell moves, cannot detect this when the first resolution still returns the same string. Matching commit identities also cannot distinguish two repositories containing the same branch tip.

**Reproduction:** use three local bare repositories, referred to below as `fetch.git`, `approved.git`, and `other.git`, each containing `main` and `feat/x` at the same commit. The base checkout is on `main`; a linked checkout named `feat-x` is on `feat/x`.

1. Set `origin` to the absolute path of `fetch.git`.
2. Configure `url.<absolute-approved.git-path>.pushInsteadOf` to the absolute path of `fetch.git`. The resolved push URL is now `approved.git`.
3. Inside the linked checkout, run `WT_SHELL_WRAPPER=1 wt remove feat-x --force-remote`. Retain the printed landing directory and handoff token. The report identifies `approved.git`.
4. Before the second invocation, configure `url.<absolute-other.git-path>.pushInsteadOf` to the absolute path of `approved.git`.
5. Confirm that `git remote get-url --push origin` still returns `approved.git`. From the landing directory, run `wt remove --handoff <token>`.

**Observed:** exit 0; the worktree and local branch are deleted; `other.git` loses `feat/x`; `approved.git` keeps it. The output claims successful deletion. I also reproduced the same wrong-destination deletion in a direct invocation from the base checkout with both rewrite rules already configured.

**Required change:** ensure the destination recorded in the approval is the effective destination used by both observation and deletion, without a second URL interpretation changing it. Either execute against a pinned destination with rewriting prevented, or refuse configurations whose observation and push destinations cannot be proven identical before removing anything. Simply resolving the string once more is insufficient for arbitrary chains of rewrite rules. Update the endpoint documentation to match the actual guarantee.

**Verification needed:** Level 1 CLI regressions using local bare repositories for both the direct and handoff sequences above. Assert that `other.git` is never modified. If the command refuses, assert that the worktree, local branch, and both remote branches remain intact. Keep the existing separate-push-URL and multiple-push-URL tests. The present URL-rewrite library test checks the returned string but does not exercise deletion through that string.

### High: Remote deletion can use the branch being deleted as its safety evidence

In the `worktree` package, [classify](../../lib/src/remove/safety.rs:265), which decides whether the local branch can be deleted without consent, checks default-branch evidence before excluding the remote branch scheduled for deletion. The exclusion is applied only later, when considering other remote branches.

The remote destination is allowed to come from the configured upstream. When `feat/x` tracks `origin/main`, `--force-remote` therefore targets `main`. If the feature's unique commit is on `origin/main` but not local `main`, the early default-branch check declares it Safe using the very copy that this command will remove. This contradicts the requirement to assess safety without the destination being deleted.

**Reproduction:**

1. Create a base checkout on `main` and a linked checkout on `feat/x`.
2. Make one new commit on `feat/x`, leaving local `main` behind.
3. Add a disposable bare `origin`. Push `feat/x:main`, then configure `feat/x` to track `origin/main`.
4. From the base checkout, run `wt remove feat-x --force-remote` non-interactively, without `--force-branch`.

**Observed:** exit 0. The report says the branch is Safe because its commits are on `origin/main`, and then says that `origin/main` will be deleted. The command deletes the worktree, local `feat/x`, and remote `main`. Only the older local `main` remains; no branch retains the unique commit. Possible recovery through Git's retained objects is not the promised preservation of work.

**Required change:** remove the destination scheduled for deletion from safety evidence before every tier check, including the default-branch check. If no other qualifying copy remains, retain the local branch unless `--force-branch` or an explicit interactive answer authorizes deleting it. This does not require banning remote-default-branch deletion; it requires honoring the existing independent consent rules for local deletion.

**Verification needed:** a Level 1 classification regression and CLI regression for a non-default local branch whose configured upstream is `origin/main`. With only `--force-remote`, expect the worktree and remote destination to be removed, exit 0, and the local branch and unique commit to remain. Cover the move-first path as well, and verify that genuinely independent surviving evidence still permits local deletion.

## Blocked Findings

None. Both fixes follow the existing specification and require no new author decision.

## Requirement verification

The table distinguishes retained coverage from tests executed during this review. Level 1 verifies logic and subprocess behavior. Level 2 verifies the terminal's rendered output. These requirements do not introduce physical-key encoding, bare modifiers, mouse input, or hotkeys requiring Level 3.

| User-observable requirement | Strongest relevant verification | Assessment |
| --- | --- | --- |
| Branch and directory completion names, `base`, detached checkouts, ambiguous names | Level 1 resolver and repository tests | Appropriate; area suite passed. |
| Safety tiers, PR source/head identity, flags, branch retention, ignored-file consent, exit codes | Level 1 library and CLI repository tests | Appropriate level, but the two missing state cases above block readiness. |
| Changed working files, staged files, branch state, expiry, token replay, and failed shell movement prevent removal | Level 1 handoff tests plus Level 2 failure scenes | Passing; the staged-content regression closes the previous finding. Remote endpoint interpretation still has the reported gap. |
| Reports precede questions with one blank line; default answer retains work | Level 2 tmux captures and Level 1 policy tests | Passed in this review. |
| Dirty-tree glyphs, file colors, connectors, and replacement by a count above ten files | Level 1 rendering tests plus Level 2 styled capture | Passed in this review. |
| Bash, zsh, and fish move-first removal; base/fork-parent landing; subdirectory preservation | Level 1 wrapper tests plus Level 2 shell scenes | Retained shell scenes passed in this review. |
| PowerShell moves both directory states and completes removal after launching inside the target | Windows Level 2 console-screen tests and Level 1 wrapper tests | Correct level present; inspected, not rerun here. |
| A held Windows directory produces exit 4 without deleting files, branch, or registration | Windows Level 2 console-screen regression | Correct level present; inspected, not rerun here. |
| Explicit local `--from`, existing/detached/error cases, fork-parent recording, wrapper detection | Level 1 CLI and repository tests | Appropriate; area suite passed. |
| Table captions, selected default target, parent rows, deleted parents, merge vocabulary, PR placement and cache age | Level 1 snapshots/data tests plus Level 2 table captures | Passed in this review. |
| Table dots, badge colors, connectors, legend, and current-row emphasis on dark/light backgrounds | Level 2 styled-cell assertions, supported by Level 1 tests | Passed in this review. |
| Linked PR badges where supported and number-only fallback elsewhere | Level 1 OSC 8/fallback assertions plus Level 2 displayed badges | Appropriate for the revised contract; those suites passed. |
| Graph lanes/tags, fork and origin relationships, PR tags, trimming, scale arithmetic | Level 1 graph facts and component tests | Worktree-side suite passed; component coverage retained from earlier reviews, not rerun here. |
| Actual graph pixels, narrow width, half-height cap, omitted-lane notice, and table surviving image output | Level 2 private Kitty screenshots, text capture, and image-transmission checks | Correct level present; not rerun here. Review 3's blank screenshot limitation remains unresolved by this review. |
| Warm/cold list timing, full command timing, offline/slow PR lookup, fresh-cache request suppression | Level 1 timed CLI and request-count tests | All 17 serial performance tests passed. |
| Four-provider PR normalization and renderer pie-color compatibility | Level 1 provider fixtures and renderer assertions | Earlier review coverage retained; cross-package suites not rerun here. |

No additional requirement was found whose strongest retained test is at the wrong level. The new findings require missing cases within Level 1, rather than physical-key or display testing.

The CLI's Level 2 targets declare `terminal-tests` as required; the live `test-l2` recipe and CI feature metadata enable it. The new removal regressions are in the automatically discovered `remove` test target and ran in Level 1. The tier audit found zero stranded tests. The previously recorded Windows console provisioning gap remains a cross-OS scheduling limitation, not a separate readiness finding under these review instructions.

## Validation and document updates

- `worktree/just test`: **297 passed**, 17 excluded by the area filter.
- `worktree/just test-perf`: **17 passed**.
- `BISCUIT_TEST_REQUIRED_BACKENDS=tmux just test-l2 level2_dirty level2_list level2_move level2_remove`: **9 passed**; backend proof records nine tmux executions and no skips or panics for that backend.
- `just check-tier-coverage worktree`: **zero stranded tests**.
- Three disposable-repository probes reproduced the findings: URL rewriting during direct removal, URL rewriting added between handoff invocations, and deletion of a configured default-branch upstream used as safety evidence.

I did not rerun lint, Kitty screenshots, cross-OS suites, or unrelated dependency suites. This review does not claim new evidence for them. No implementation or test source was edited, and no repository commit was created.

The requested previous-review location under `prompts/_reviews/worktree/…` does not exist. The existing review beside the specification was updated instead: `implemented` remains `true`, and `next` points to this review. That flag records the completed implementation attempt, not acceptance of its correctness. The specification's `review_iterations` is now 4; it is not marked completed.
