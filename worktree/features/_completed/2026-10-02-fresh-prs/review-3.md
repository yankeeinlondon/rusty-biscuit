---
$schema: feature-review.yaml
ready: false
findings:
    - title: Receipt-cleanup assertions race successful publication
      priority: medium
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: 2026-10-03T10:42:33-07:00
spec: 2026-10-02-fresh-prs/spec.md
log: worktree/features/2026-10-02-fresh-prs/log.md
implemented: true
implemented_by: claude/opus
description: "A **feature** review of `2026-10-02-fresh-prs/spec.md`"
feature: 2026-10-02-fresh-prs/review-3.md
previous: 2026-10-02-fresh-prs/review-2.md
next: 2026-10-02-fresh-prs/review-4.md
---

# Review 3: Fresh pull requests

**Not production ready.** Both findings from review 2 are implemented. The remaining finding is a reproduced timing-dependent test failure: a successful listing can finish before its worker writes the completion receipt, but two assertions require that receipt to have been deleted already. No human decision is needed.

Reviewed the specification, plan, both implementation logs, both earlier reviews, current source and uncommitted repairs, worker execution, wait transitions, stored-input readers, presentation, test declarations, and current documentation. This review changes only review metadata and the specification's review counter. A temporary ordering probe reused the scripted wait fixture and production public `wait` function; it was removed afterward, restoring the source file byte for byte.

## Previous findings

Review 2 had two unblocked findings and explicitly no blocked findings. Its repair log records no deferrals or intervening decisions to unblock. Review 1 likewise had no blocked findings.

| Earlier finding | Verification in this review | Result |
| --- | --- | --- |
| Review 2: PR retries leave completed head progress on the spinner | `Follow::observe` resets the PR-only display flag whenever a head phase is shown. The L1 regression crosses four routes with fetching, no-key fallback, and rate-limit fallback. The new tmux test follows a real PR retry, captures the replacement fallback, then exactly one generic spinner line while PRs remain held, and finally the cleared line. | Addressed at L1 and L2. |
| Review 2: Stored-input matrices omit decision fields and receipt behavior checks | The PR matrix includes all envelope and element fields. The receipt matrix includes all three failures with fields, including the rate-limit authentication flag, and runs through real files and the public wait result. The additional remote-head matrix checks independent validation of both stored halves. All ran in the passing L1 suite. | Addressed. |
| Review 1: Retries discard results already established by the other half | Reviewed retained head and PR results, launch/id failures, missing replacement receipts, unfinished replacements, and superseding results; their regression tests ran. | Repair retained. |
| Review 1: Retry paths bypass the original deadline and hide timeouts | Both retry routes enforce the shared deadline; before/at/after boundary tests and holder-timeout tests ran. | Repair retained. |
| Review 1: Several worker fixtures do not guarantee cleanup after an assertion fails | Fixture-owned cleanup, request-release ordering, repository-scoped worker discovery, and unwind tests at L1 and in tmux were checked. | Repair retained; the new finding concerns receipt publication ordering, not a surviving worker. |

## Unblocked Findings

### Medium: Receipt-cleanup assertions race successful publication

**Defect class:** an asynchronous producer's final file publication is assumed to precede a consumer's successful return, although the consumer can resolve success from earlier publications and has already attempted cleanup before the final file exists.

In the `worktree-cli` package, [refresh_waits_for_both_halves_and_asks_again_like_every_listing](../../cli/tests/list_flags.rs:123) checks that no receipts remain after both an ordinary listing and a forced listing (lines 140 and 153). The first `just test` run failed this test with a remaining receipt containing `prs.kind: ok` and `head: failed`. The provider request and stored answer had succeeded; a failed head is still a completed head result. The test had waited for the worker to disappear, so this is not a worker-cleanup failure. A second unchanged full run passed all 874 tests, demonstrating why a single green run does not settle the assertion.

In that package, [Follow::run](../../cli/src/commands/list/wait.rs:249) can return as soon as a head outcome and a new PR publication are observed. It does not require the combined receipt when publication already proves PR success. [wait](../../cli/src/commands/list/wait.rs:158) then tries to delete each launched attempt's receipt once. The background worker's [run_and_record](../../cli/src/commands/refresh_worker.rs:121) writes the receipt only after joining both halves and sweeping old receipts. Therefore the foreground deletion can precede the write, leaving a late receipt even when the listing did not time out. Waiting for the worker to exit afterward cannot make the earlier deletion happen again.

The controlled reproduction kept the existing scripted fixture's stores, launch handles, and clock, and changed receipt availability relative to completed results. It called the production public wait API. Both CLI targets produced the same ordering:

| Site / sibling | Shape tested | Observed result | Expected contract / repair |
| --- | --- | --- | --- |
| Ordinary successful wait; first empty-receipt assertion at line 140 | Head and new PR publication at 100 ms; receipt and worker exit at 200 ms | Successful return and cleanup at 100 ms, before receipt exists | Test must allow this ordering or implementation must explicitly coordinate final publication and cleanup. |
| Forced successful wait; second empty-receipt assertion at line 153 | Same timings, forced mode | Same early success and cleanup | Same repair; the larger budget does not prevent the race. |
| Forced PR retry | First receipt reports contention; holder releases without publication; replacement publishes at 1.1 s and writes its receipt at 1.2 s | Success at 1.1 s; cleanup requested for both ids before replacement receipt | Include replacement receipt ordering in the regression. |
| Forced head retry | First receipt reports head contention and PR success; replacement head finishes at 1 s, replacement receipt at 1.2 s | Retained PR success permits return and cleanup at 1 s | Include retained-success ordering; no new PR publication is needed for this case. |
| Failure without a new publication | Head finishes at 100 ms; failed PR receipt arrives at 200 ms | Wait ends at 200 ms using that receipt — clean control | Existing-file deletion can be asserted after the receipt is observed. |
| Adopted head, own receipt already written | Existing adoption and overlapping-run fixtures: own worker exits after writing its receipt; another head is followed | Own receipt is available before return; adopted receipt is not deleted — clean | Preserve ownership; do not delete the followed worker's receipt. |
| Local-origin forced listing; empty-receipt assertion at line 264 | PR outcome is unsupported, with no new publication | Its PR result requires the receipt — clean for this race | Retain the assertion for this controlled receipt-dependent route. |
| Real-file wait and malformed-receipt matrices | Fixture writes the receipt synchronously before returning an exited worker handle | Receipt exists before cleanup; valid and malformed files are removed — clean | Keep these checks; they cannot prove cleanup of a file written later. |
| Timeout with late publication | Existing timeout fixtures | Cleanup is attempted before late completion; subsequent worker sweep removes old files | Already permitted by the specification; preserve the bounded wait. |

All empty-directory receipt assertions in `list_flags.rs` were enumerated: the two in the successful provider test above and the local-origin assertion. The scripted cleanup assertions record that deletion was requested; they do not prove a future file cannot appear. The real-file tests deliberately publish before exit. The worker's stale-receipt tests cover removal of old files while retaining active and other-repository receipts.

**Recommended repair:** keep the existing early-success and bounded-wait behavior. Make cleanup tests distinguish an observed receipt, which must be deleted, from a receipt written after the wait's cleanup, which is left for the existing stale sweep. Add a deterministic regression covering the four affected success routes above and the receipt-dependent controls. Do not use sleeps or automatic retries to make the assertions pass. The simplest repair changes the two unconditional assertions and proves late-file sweeping, rather than introducing another foreground/worker handshake.

Also correct the unconditional deletion claims in the `worktree` package's [receipt lifecycle documentation](../../docs/cli/list.md:54), [performance documentation](../../docs/performance-testing.md:57), and [worktree skill](../../../.claude/skills/worktree/SKILL.md). Explain that the wait attempts deletion of its own receipts and a later write can survive until the age sweep, including after early success. The specification describes cleanup as best effort but mentions late receipts only after timeout; record this clarification in the implementation log without rewriting that snapshot.

The listing's PR data and completed head result are correct in the reproduced ordering. The readiness issue is the flaky acceptance test and the cleanup guarantee its assertion and documentation imply.

## Blocked Findings

None.

## Recurrence

No finding repeats an earlier defect class. Review 1's cleanup finding concerned workers and held requests outliving fixtures during assertion failure. This finding concerns a receipt written by a worker that finishes normally after the foreground's deletion attempt. Review 2's findings concerned spinner state and incomplete malformed-input matrices; both are addressed. `recurrence` is therefore `false`.

## Input robustness audit

The permanent tests start from files written by the production writers, make one edit per cell, and include positive controls. They assert public cached results or the public wait result, not just parse success.

| Reader / load-bearing fields | Shapes and public result checked | Assessment |
| --- | --- | --- |
| PR store envelope: `format_version`, `publication`, `origin_digest`, `fetched_at`, `source_repo`, `pull_requests` | Missing, null, wrong type, empty containers/string, duplicate, invalid/trailing content; invalid and mixed list elements; valid empty answer; wrong origin/version, future and stale dates | Invalid answers miss without publication ids; valid empty answers remain answers. |
| Every PR element: `number`, `url`, `source_repo`, `source_branch`, `target_branch` | Missing, null, wrong type, empty containers/string, duplicates | No invalid element is silently dropped. Explicit null remains valid only for optional metadata; permitted empty strings stay strings. |
| Receipt envelope: `format_version`, `attempt_id`, `origin_digest`, `branch`, `finished_at`, `head`, `prs` | Missing, null, wrong type, empty, duplicate, invalid/trailing content, wrong bindings and pre-attempt timestamps | Missing receipt becomes generic PR failure while keeping the finished head, within the ordinary budget. |
| Receipt nested fields: `prs.kind`, `prs.failure`, `prs.failure.kind`, `key`, and rate-limit `authenticated` | Same applicable shapes for rejected credentials, insufficient permissions, and rate limits, with three positive controls and valid null-key/false-authentication controls | Invalid input invents no credentials diagnosis; valid null keeps the specific failure without a variable name. |
| Remote-head store: envelope plus answer/attempt bindings, timestamps, SHA/source, phase/reason, outcome/reason, API condition/authentication/key/fallback flag | Per-field mutations, null controls, duplicate keys, invalid document, stale/future/misbound selections | A malformed half is dropped independently; repeated top-level keys or invalid documents lose both. |

The receipt repair avoids serde's buffered integer-to-variant coercion in both the flattened envelope and nested failure. The remote-head repair rejects duplicate keys before converting independent members to typed values. Tests cover those defects. Smell inspection found no newly defaulted load-bearing field in these readers; their parse failures become misses rather than partial answers.

## Requirement verification

| User-facing requirement | Strongest relevant verification | Assessment |
| --- | --- | --- |
| Sequential eligible listings ask again, including young stored answers; overlapping listings share an in-flight request | L1 shipped binary, loopback provider, request counts and locks | Verified; receipt-cleanup assertion needs repair. |
| In-budget answers appear immediately; empty success clears badges; only the worker publishes PR answers | L1 binary/store tests and writer-call inspection | Verified. |
| Ordinary 3 s and forced 75 s budgets, independent outcomes, adoption, retries, and superseding evidence | L1 public wait boundary tests, binary tests; serial performance checks | Verified behavior. |
| Failed, pending, empty, stale, ignored, unsupported, changed-origin, and no-origin presentation | L1 snapshots and binary tests; L2 tmux captures for visible status styles/order | Verified. |
| Spinner returns to generic progress after a head completes, including a PR retry, and clears before final output | L1 four-route/three-phase regression; L2 ordinary and retry transitions in tmux | Verified at the required levels. |
| Credentials warning is dim beneath the caption and preserves head-warning precedence | L1 renderer/binary tests and malformed-receipt wait matrix; L2 tmux style capture | Verified. |
| Fork filtering, advisory failures, and permitted fast-forward despite PR failure | L1 matching/gathering and shipped binary tests | Verified. |
| Every worker attempt writes its own receipt; panics in one half preserve the other | L1 worker and real-file tests | Verified; cleanup ordering finding above. |
| Assertion failures release held requests and reap workers without replacing the original panic | L1 unwind tests and L2 tmux unwind scene | Verified. |

No feature requirement handles keyboard, mouse, paste, or IME events, so Level 3 is not required. Cross-OS evidence and a separate human review are not readiness conditions here. The unavailable Kitty pixel checks below concern existing graph behavior outside this feature's changes.

## Validation performed

- First `just test`: **520 passed, 1 failed**, 353 not run after fail-fast, 30 excluded. The failure was `list_flags::refresh_waits_for_both_halves_and_asks_again_like_every_listing`, with a late successful PR receipt still present.
- Second unchanged `just test`: **874 passed**, 30 excluded. This is additional verification, not a reason to dismiss the first failure.
- Temporary public-wait ordering sweep: **2 target executions passed**, each proving four early-success orderings and one receipt-dependent failure control. Source restored byte for byte afterward.
- `BISCUIT_TEST_REQUIRED_BACKENDS=tmux just test-l2`: **31 runner passes**; backend proof records **25 tmux executions**, zero tmux skips or panics. The new retry-spinner capture executed. Several existing Kitty tests explicitly skipped their pixel checks because the captured window had no visible contents; runner passes are not pixel evidence for those tests.
- `just lint`: **passed** for both packages.
- `just check-tier-coverage worktree`: **zero stranded tests**. L2 targets require `terminal-tests`, that feature is declared for CI, and the area has a live L2 recipe.
- `just test-perf`: **24 passed, 1 failed**, 5 not run after fail-fast. The stale-answer/failing-refresh fixture's best full-command time was **1.03 s**, exceeding its 1 s ceiling; its store-read stages were within their limits. Held PR/head/fetch ordinary waits remained **3.00 s**; the held forced fetch ended at **60.20 s** of remote wait.
- Focused `just test-perf perf_list_meets`: **2 passed** unchanged; stale-answer full-command time was **477 ms**, and network-down full-command time **354 ms**. `just test-perf perf_refresh`: **1 passed**. These follow-ups do not turn the failed full invocation into a passing gate. No specific code defect was established for the performance miss, so it is recorded as a validation limitation rather than a second finding.

Repair the timing-dependent receipt-cleanup assertions and corresponding lifecycle claims before the next review.
