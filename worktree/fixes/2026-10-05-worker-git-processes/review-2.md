---
$schema: feature-review.yaml
ready: true
findings: []
observations:
  - An unchanged remote-head test timed out in the full suite
human_review: false
human_review_items: []
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-10-11T17:29:39-07:00"
spec: 2026-10-05-worker-git-processes/spec.md
implemented: false
description: "A **fix** review of `2026-10-05-worker-git-processes/spec.md`"
fix: 2026-10-05-worker-git-processes/review-2.md
previous: 2026-10-05-worker-git-processes/review-1.md
---

# Review 2

The fix is **production ready** under this review's criteria. The previous blocking finding is implemented, and no new high, medium, or low finding was identified. One unchanged test timed out during the full local suite; that result is recorded below rather than reported as a passing run.

## Previous review and repair log

Review 1 had one unblocked finding, “The process-count regression test does not cover the complete worker.” It had no blocked findings to reconsider. The repair log records that finding as fixed, with none deferred or disputed.

The `worktree-cli` package now has a maintained [whole-worker test](../../cli/src/commands/refresh_worker.rs:1143). It creates a main checkout and local bare origin, pushes a second commit, moves the tracking ref back to the first commit, and calls the public worker entry point inside a Git-count scope. Its two assertions establish that the worker fetched the new commit and started exactly three Git processes. Setup and verification commands are outside the counting scope.

The worker's [run_halves](../../cli/src/commands/refresh_worker.rs:247), which runs the PR and remote-head work concurrently in `worktree-cli`, carries the caller's counting scope into both threads and merges each successful join's count. The existing two-process component test remains in the `worktree` library. The new test passed in both the CLI library and binary targets during this review.

I checked every stage omitted by the previous component test:

| Worker stage | Verification | Result |
| --- | --- | --- |
| Checkout lookup and setup | Whole-worker test includes checkout, origin, preferences, and receipt-target setup inside the scope | Covered by the three-process assertion |
| PR half | Real PR half against a local origin without a supported provider; its thread carries the scope | Covered; no provider request required for this fixture |
| Remote-head half | Real remote check and fetch; tracking ref starts behind origin | Tracking ref moves to the pushed commit |
| Receipt completion | Public worker call returns after both halves join and receipt processing completes | Included in the same scope |
| Transport pipe-reader threads | Inspection of `live_remote::drain` in `worktree` | Read pipes; start no Git processes |

No sibling within this worker path still loses a successful half's process count. Counts from a panicked half remain unavailable, as the updated comment explicitly states; the spec's successful fetch-attempt assertion does not require those counts.

## Unblocked Findings

None.

## Blocked Findings

None.

## Requirement coverage

Reviewed the original metadata replacement and its call sites, the repair's thread propagation and fixture cleanup, the parity tests, and the updated performance documentation.

| Requirement | Strongest verification relevant to this fix | Assessment |
| --- | --- | --- |
| Origin URL matches Git, including local, scp-like, HTTPS, overlapping rewrites, multiple URLs, and missing origin | Level 1 Git parity tests in `worktree::git_metadata` | Required cases pass |
| Default branch uses symbolic origin HEAD, then main/master | Level 1 symbolic-ref comparison and temporary repositories with main, master, or neither; public default-branch tests | Pass |
| Branch validation matches Git | Level 1 comparison across valid names and invalid syntax | Pass |
| SSH command and tracking tip retain their meanings | Level 1 comparisons for unset/multiple SSH settings and missing, symbolic, annotated-tag, and packed reference targets | Pass |
| Configuration changes remain visible | Fresh repository discovery per read; Level 1 origin-change tests | Pass |
| Complete fetching worker starts only checkout lookup, remote check, and fetch | Level 1 whole-worker count plus tracking-ref assertion; two-process component test retained | Previous gap closed |
| Ordinary wait remains three seconds | Production budget unchanged; ordinary-listing Level 1 tests | No wait-policy change |
| Metadata-reading documentation is current | Inspection of `docs/performance-testing.md` and the worktree skill | Updated, including complete-worker counting |

Level 1 is appropriate for the repository reads, subprocess counts, and local-origin fetch behavior changed here. This fix introduces no terminal-rendering or keyboard-input requirement needing Level 2 or Level 3. Both CLI targets compile the new test, the development dependency enables `count-git`, and `just check-tier-coverage worktree` reports **zero stranded tests**.

The implementation logs record prior macOS, Linux, native Windows, and WSL2 runs, plus Linux and native Windows runs of the repaired whole-worker test. These are recorded implementation evidence, not new cross-OS results from this review; cross-OS reruns are not a readiness gate for this review.

## Verification

Ran `just test` from `worktree/`: **1,103 passed, one timed out, 14 excluded by target/tier selection**. The new whole-worker test passed in both compiled targets, and the metadata parity and component-count tests passed. The timeout was in an unchanged remote-head test described below.

A separate diagnostic run through `just test a_fetch_still_running_at_the_deadline_is_still_pulling_and_publishes_after_the_listing` passed in **1.703 seconds**. This does not erase the full-suite timeout or establish its cause.

This review changes only this review file and the requested metadata in the previous review and spec. It marks the spec completed without moving its lifecycle directory. No production code or tests were edited.

## Observations

### An unchanged remote-head test timed out in the full suite

The `worktree-cli` test [a_fetch_still_running_at_the_deadline_is_still_pulling_and_publishes_after_the_listing](../../cli/tests/list_remote_head.rs:231), which checks a listing while a remote fetch is deliberately held, reached the 30-second timeout in the full run and passed in 1.703 seconds separately. Its test and holding fixture were not changed by this fix, and this review did not establish that the metadata replacement caused the timeout. The suite result warrants a separate investigation of the held-fetch fixture under concurrent execution; it is an observation, not a request to expand this repair cycle.
