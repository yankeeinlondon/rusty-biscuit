---
$schema: feature-review.yaml
ready: true
findings: []
human_review: false
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: 2026-10-03T11:04:06-07:00
spec: 2026-10-02-fresh-prs/spec.md
implemented: false
description: "A **feature** review of `2026-10-02-fresh-prs/spec.md`"
feature: 2026-10-02-fresh-prs/review-4.md
previous: 2026-10-02-fresh-prs/review-3.md
---

# Review 4: Fresh pull requests

**Production ready.** Review 3's receipt-cleanup finding is implemented, the earlier repairs remain covered, and this review found no additional functionality, performance, or required-verification gap. No human decision is needed.

Reviewed the specification, plan, implementation logs, all three earlier reviews, current source and uncommitted repairs, stored-input readers, worker and foreground wait, presentation, fixture cleanup, test declarations, and current documentation. This review changes only review documents and specification metadata; it does not change the implementation or move the feature into a lifecycle directory.

## Previous findings

Review 3 had one unblocked finding and no blocked findings. Neither earlier review nor the repair log records a blocked finding that became actionable before the latest repair. The latest implementation log reports no deferrals.

| Earlier finding | Verification | Result |
| --- | --- | --- |
| Review 3: Receipt-cleanup assertions race successful publication | The two provider-backed success assertions permit at most one valid receipt belonging to that listing's attempt, with a successful PR result. Deterministic public-wait tests cover ordinary success, forced success, PR retry, and head retry with retained PR success. A real-file test proves late publication survives the foreground deletion attempt and is removed by the later age sweep. | Addressed. |
| Review 2: PR retries leave completed head progress on the spinner | The display state resets whenever a followed head phase is shown. The regression crosses four routes with fetch, no-key fallback, and rate-limit fallback. The tmux retry scene observes the replacement fallback, its transition to one generic progress line while PRs remain held, and final clearing. | Addressed at Level 1 and Level 2. |
| Review 2: Stored-input matrices omit decision fields and receipt behavior checks | PR envelope and element fields, all three field-bearing receipt failures, and independent remote-head members have writer-generated mutation tests. The receipt matrix also runs through real files and the public wait result. | Addressed. |
| Review 1: Retries discard results already established by the other half | Reviewed retained results across replacement launch/id failure, missing receipts, stopped and pending replacements, and newer results. Their regressions passed under both CLI targets. | Repair retained. |
| Review 1: Retry paths bypass the original deadline and hide timeouts | Both replacement routes enforce the original deadline. Before, at, and after deadline cases retain independent results and distinguish timeout from immediate failure. | Repair retained. |
| Review 1: Several worker fixtures do not guarantee cleanup after an assertion fails | Shared fixture destructors and release/reap guards cover held provider, proxy, and Git requests, plus directly owned children. Level 1 unwind tests passed. | Repair retained. |

## Unblocked Findings

None.

## Blocked Findings

None.

## Recurrence

No new finding repeats an earlier defect class. All earlier findings were checked against their sibling paths; `recurrence` is `false`.

## Receipt-ordering sweep

The repaired class is an assumption that a background producer's final file write precedes the consumer's successful return. In the `worktree-cli` package, [the public wait](../../cli/src/commands/list/wait.rs) can finish from a new PR publication and a completed head before the worker writes its combined receipt. Its deletion attempt remains best effort.

| Site / shape | Observed result | Expected result |
| --- | --- | --- |
| Ordinary and forced success: head and PR publication at 100 ms, receipt at 200 ms | Return and one deletion attempt at 100 ms; later receipt remains possible | Preserve early success; do not require an empty receipt directory. |
| Forced PR retry: replacement publication before its receipt | Both launched ids are discarded once at return; the earlier receipt exists, the replacement receipt does not yet exist | Delete observed own receipts and allow the late replacement receipt. |
| Forced head retry: retained PR success, replacement head before replacement receipt | Retained success permits early return; the late replacement receipt is possible | Preserve the independently resolved PR result. |
| Failed or unsupported PR result, ordinary and forced | Wait ends when the receipt arrives and cleanup observes that receipt | Existing own receipt is deleted. |
| Adopted head | Only the listing's own receipt id is discarded | Preserve the adopted worker's receipt ownership. |
| Local-origin binary control and real-file receipt matrices | Result requires the already-written receipt; its deletion assertions pass | Keep receipt-dependent cleanup assertions. |
| Real late receipt and stale sweep in the `worktree` package | Valid young receipt remains; receipt older than the maximum attempt age is removed | Leave late publication for the repository-scoped sweep. |
| PR store absence and lock/worker cleanup assertions | Failed PR requests never publish; teardown waits for workers and both locks | These assertions do not assume receipt publication ordering. |

The relevant comments, [list documentation](../../docs/cli/list.md), [performance documentation](../../docs/performance-testing.md), and worktree skill now describe one deletion attempt and late writes after either timeout or early success. The implementation log records this clarification without rewriting the specification snapshot.

## Input robustness audit

The field tables in the plan and permanent tests distinguish absent, null, wrong whole type, empty values, duplicates, and invalid/trailing content. PR lists additionally cover mixed and entirely invalid elements, with valid empty answers as controls. These tests passed in this review.

| Reader / field inventory | Public behavior verified |
| --- | --- |
| PR store: `format_version`, `publication`, `origin_digest`, `fetched_at`, `source_repo`, `pull_requests`; each element's `number`, `url`, `source_repo`, `source_branch`, `target_branch` | Invalid stores return a cache miss and no publication id; no invalid element is silently discarded. Explicit null remains valid for optional repository/link metadata. Valid empty lists remain answers; stale, future, wrong-origin, and old-format controls are covered. |
| Receipt: `format_version`, `attempt_id`, `origin_digest`, `branch`, `finished_at`, `head`, `prs`; nested `kind`, `failure`, failure `kind`, `key`, and rate-limit `authenticated` | Invalid input produces generic PR failure through the public wait, preserves the completed head, and stays within budget. Valid null key and false authentication retain their specific meanings. Integer tags cannot invent another outcome or credentials diagnosis. |
| Remote-head store: envelope, answer binding/SHA/time/source, attempt binding/time/phase/outcome/API note and their nested fields | Invalid members are rejected independently; duplicate top-level keys and invalid documents reject the whole store. Null SHA means verified absence, null outcome means running, and null API note means no note. |

Inspection found no newly defaulted load-bearing field or silently filtered stored element. Parse failures in these readers become cache misses or missing receipts rather than empty successful answers. Provider parsing is unchanged by this feature.

## Requirement verification

| User-facing requirement | Strongest relevant verification | Assessment |
| --- | --- | --- |
| Sequential eligible listings ask again despite a young answer; overlapping queries share the lock | Level 1 shipped binary, loopback provider, request counts, and contention tests | Verified. |
| In-budget answers appear immediately; empty success clears badges; fork PRs cannot match local branches | Level 1 binary, gathering, and badge-matching tests | Verified. |
| Ordinary 3 s and forced 75 s budgets; independent results, adoption, retries, and deadline boundaries | Level 1 public-wait scripts and binary tests; serial performance checks | Verified. |
| Failed, pending, empty, stale, ignored, unsupported, changed-origin, and no-origin presentation | Level 1 presentation snapshot and binary tests; Level 2 tmux captures for status styling and order | Verified. |
| Spinner phase changes, generic progress when only PRs remain, and clearing before final output | Level 1 route/phase matrix; Level 2 tmux ordinary and retry transitions | Verified. |
| Dim credentials warning beneath caption, head-warning precedence, and generic ambiguous failures | Level 1 binary/renderer tests and receipt mutation waits; Level 2 tmux styling capture | Verified. |
| PR failure remains advisory and cannot prevent a permitted fast-forward | Level 1 binary test with loopback Git/provider transport | Verified. |
| Only the worker writes PR answers; every attempt writes its receipt; panic in one half preserves the other | Level 1 worker tests, real-file tests, and writer-call inspection | Verified. |
| Receipt ownership, deletion attempts, late publication, and stale sweep | Level 1 scripted public-wait and real-file tests, plus binary controls | Verified. |
| Assertion failures release held requests and reap workers before fixture removal | Level 1 unwind tests and fixture ownership review; existing Level 2 unwind scene inspected | Verified; the unwind terminal scene was not rerun by this review's filtered Level 2 command. |

No requirement introduces keyboard, mouse, paste, or IME handling, so Level 3 is not required. Test count alone is not used as evidence: the table identifies the level appropriate to each behavior. Terminal tests have declared targets, the required feature is enabled in CI, and the area has a live Level 2 recipe.

The current README, topic documentation, and skill agree with the implementation. The performance page records the unauthenticated development sample and explicitly explains why no authenticated sample was taken, as the plan permits. Cross-OS evidence and separate human review are not readiness conditions for this review.

## Validation performed

- `just test`: **880 passed**, 30 excluded tests. This includes all three new receipt-ordering tests under both CLI targets, the public-result input matrices, and earlier retry regressions.
- `BISCUIT_TEST_REQUIRED_BACKENDS=tmux just test-l2 level2_list_`: **13 passed**; backend proof records **13 actual tmux executions**, zero skips or panics. The filter covers the listing's changed status, credentials, and spinner scenes; it does not claim evidence for unrelated Kitty graph pixels or the separately named unwind scene.
- `just lint`: **passed** for both packages.
- `just check-tier-coverage worktree`: **zero stranded tests**.
- `just test-perf`: **30 passed**, run serially after the Level 1 and Level 2 suites and lint finished. The previous review's performance miss did not recur in this full invocation.
- Scoped metadata whitespace check: **passed**. A whole-worktree `git diff --check` also reported pre-existing extra final blank lines in `list_prs.rs` and `list_remote_head.rs`; these are formatting-only and were left untouched.
