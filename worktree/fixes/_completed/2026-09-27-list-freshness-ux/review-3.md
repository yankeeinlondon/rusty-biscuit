---
$schema: feature-review.yaml
ready: false
findings:
    - title: An ordinary listing can replace a newer pull request answer
      priority: high
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6-sol
created: 2026-09-27T16:37:21-07:00
spec: 2026-09-27-list-freshness-ux/spec.md
implemented: true
next: 2026-09-27-list-freshness-ux/review-4.md
description: "A **fix** review of `2026-09-27-list-freshness-ux/spec.md`"
fix: 2026-09-27-list-freshness-ux/review-3.md
previous: 2026-09-27-list-freshness-ux/review-2.md
---

# Review 3: list freshness UX

## Verdict

**Not production ready.** The sole unblocked finding in review 2 is resolved for a single pull request refresh holder: every successful write now has a distinct publication ID, and a forced listing can recognize a same-second replacement. Review 2 had no blocked findings to reconsider. A separate concurrent write path can still replace a newer answer, so the freshness promise is not yet reliable when two listings run together. No human decision is needed.

## Previous findings

| Finding in review 2 | Result |
| --- | --- |
| Forced refresh mistakes a same-second pull request update for no update | Resolved for a refresh holder writing on its own. The store now assigns a new publication ID on each successful write, the forced wait compares it after the lock opens, and Level 1 wait and binary tests pass for a young answer and an unchanged second. The new finding below concerns a different writer that does not use that lock. |

## Unblocked Findings

### High: An ordinary listing can replace a newer pull request answer

In the `worktree` library, [fetch_and_publish](../../lib/src/pull_requests.rs) handles an ordinary listing's missing pull request answer and writes the shared store without taking its refresh lock. The background [refresh](../../lib/src/pull_requests.rs) holds that lock while requesting and writing an answer. In `worktree-cli`, [gather_remote](../../cli/src/commands/list.rs) reads the store again after waiting and uses whichever answer was written last. Atomic file replacement protects the file from corruption, but does not protect the order of the answers.

For example, an ordinary listing starts a foreground request when the store is empty. A second listing starts `wt -r`; its worker fetches and stores a newer pull request answer under the lock. If the first request finishes afterward, it writes its older answer over the worker's result. The forced listing can then show the older badges even though its completion receipt says the refresh succeeded. The first request is bounded to 300 ms, which narrows this window but does not close it. The publication ID changes for both writers, so it cannot establish that the lock holder produced the answer in the contended case either.

Make both write paths obey one ordering rule, such as using the same persistent lock and refusing a foreground publication when a newer answer has already appeared. Preserve the bounded foreground request. Add a deterministic Level 1 test that holds the foreground answer until after a forced worker publishes, then verifies that the newer badges and stored answer survive. Also test a contended forced wait where a separate foreground writer publishes while the holder fails; the wait must not treat the unrelated publication as proof that the holder succeeded.

## Blocked Findings

None.

## Requirement verification

| User-observable requirement | Strongest verification found | Assessment |
| --- | --- | --- |
| Remote check, fetch, caption, metrics, and fast-forward suggestion | Level 1 real-Git binary tests and injected worker tests | Appropriate for Git state and command behavior; the failed-check and failed-fetch suggestion cases pass. |
| Forced refresh waits for both halves and shows a same-second pull request update | Level 1 wait and binary tests | Appropriate tier and the review 2 timing case passes. The tests do not exercise a foreground writer racing the locked worker, so the new finding remains. |
| Spinner phase text and clearing before the caption | Level 2 tmux pane captures | Appropriate for terminal rendering; the fallback and fetch transition tests pass. |
| Flag parsing, help, ignored API choice, warnings, and output order | Level 1 command and renderer tests, plus Level 2 captures for styled warning and hint lines | Appropriate for these command and terminal behaviors. |
| Physical key presses or terminal input encoding | No requirement introduced by this fix | Level 3 is not applicable to these command flags. |

The changed Level 1 tests are compiled by their declared targets and selected by `just test`. The changed spinner tests are compiled by the declared `level2_list_verbose` target with `terminal-tests` enabled by the live `test-l2` recipe. `just check-tier-coverage worktree` reports no stranded tests.

## Validation

- `just check-tier-coverage worktree`: zero stranded tests.
- `worktree/just test`: 695 passed.
- `worktree/just lint`: passed.
- `worktree/just test-l2 level2_list_spinner`: two real-terminal tmux tests passed.
- Focused Level 1 checks: 16 forced-wait tests, the same-second binary test, and the publication-ID library test passed.

The concurrent overwrite finding follows from the two write paths above; the current tests do not cover that interleaving. Cross-OS evidence and human review are outside this readiness decision.
