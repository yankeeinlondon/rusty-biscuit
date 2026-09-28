---
$schema: feature-review.yaml
ready: false
findings:
    - title: Forced refresh mistakes a same-second pull request update for no update
      priority: high
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6-sol
created: 2026-09-27T12:58:19-07:00
spec: 2026-09-27-list-freshness-ux/spec.md
implemented: true
implemented_by: claude/opus
log: worktree/fixes/2026-09-27-list-freshness-ux/implementation-log.md
description: "A **fix** review of `2026-09-27-list-freshness-ux/spec.md`"
fix: 2026-09-27-list-freshness-ux/review-2.md
previous: 2026-09-27-list-freshness-ux/review-1.md
next: 2026-09-27-list-freshness-ux/review-3.md
---

# Review 2: list freshness UX

## Verdict

**Not production ready.** Three findings from the first review are resolved. The forced pull request refresh still has a reachable timing error. No finding from the first review was blocked by human review, and this review needs no human decision.

## Previous findings

| Finding in the first review | Result |
| --- | --- |
| Forced refresh can finish without a pull request result after lock contention | Partially resolved. A failed holder now leads to a bounded retry, but a successful holder can be mistaken for a failure when both cache writes occur in the same second. |
| Spinner phase changes lack real-terminal verification | Resolved. New Level 2 tmux tests capture the no-key and rate-limit fallback phases, the transition to fetching, and cleanup before the caption. |
| Ignored API preferences merge distinct SSH ports | Resolved. Nonstandard SSH ports retain their identity, with a persisted preference test. |
| A failed remote check hides an available fast-forward suggestion | Resolved. The suggestion now appears after a completed failed check or fetch, with Level 1 binary tests for both. |

## Unblocked Findings

### High: Forced refresh mistakes a same-second pull request update for no update

In `worktree-cli`, the [forced wait](../../cli/src/commands/list/wait.rs:197) decides whether another process published a pull request answer by requiring the new `fetched_at` value to be greater than the value read before launch. The `worktree` library [records this time in whole Unix seconds at request start](../../lib/src/pull_requests.rs:295). A forced request may replace a young cached answer in the same second, or two quick forced requests may start in the same second. The stored answer and its pull requests can change while `fetched_at` stays equal. The wait then launches a second network request even though the first holder succeeded. If that second request fails, its receipt reports failure despite the newly published answer. The current binary test [seeds an answer two hours old](../../cli/tests/list_flags.rs:149), so it cannot exercise this case.

Use a publication identity that changes for every successful write, such as a stored attempt token or generation, and compare that identity after the lock opens. Keep the visible age timestamp separate. Add a deterministic Level 1 wait test with an unchanged second but a changed publication identity, plus a Level 1 binary test with a young answer that verifies one request and the new pull request badge. These tests should also verify that a holder which publishes nothing still triggers the bounded retry.

## Blocked Findings

None.

## Requirement verification

| User-observable requirement | Strongest verification found | Assessment |
| --- | --- | --- |
| Remote check and fetch, caption, counts, and fast-forward suggestion | Level 1 real-Git binary tests and injected worker tests | Appropriate for Git state and command behavior. The failed-check and failed-fetch suggestion cases now pass. |
| Forced refresh after pull request lock contention | Level 1 binary and wait tests | Correct tier, but the tests use an old cached answer and miss the same-second case above. |
| Spinner text changes and removal before the caption | Level 2 tmux pane captures | Appropriate for real-terminal rendering; both new phase tests passed. |
| Physical keyboard input or terminal key encoding | No changed requirement | Level 3 is not needed for these command flags. |

The new Level 2 tests are compiled by the declared `level2_list_verbose` target with `terminal-tests` enabled by the live `test-l2` recipe. The new Level 1 tests are compiled by their declared targets and selected by `just test`. `just check-tier-coverage worktree` reported zero stranded tests.

## Validation

- `just check-tier-coverage worktree`: zero stranded tests.
- `worktree/just test a_forced_wait`: 12 passed across the CLI library and binary targets.
- `worktree/just test refresh_shows_the_answer_a_contending_holder_published`: passed.
- `worktree/just test refresh_asks_again_when_a_contending_holder_failed`: passed.
- `worktree/just test a_failed_check_still_suggests_fast_forwarding_to_the_local_tracking_ref`: passed.
- `worktree/just test-l2 level2_list_spinner`: two real-terminal tests passed.

Cross-OS results and human review are outside this readiness decision.
