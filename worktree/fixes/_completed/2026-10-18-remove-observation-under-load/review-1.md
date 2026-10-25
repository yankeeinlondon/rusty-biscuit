---
$schema: feature-review.yaml
reviewed_by: codex/gpt-6.1-sol
created: "2026-10-19T09:04:54-07:00"
spec: 2026-10-18-remove-observation-under-load/spec.md
fix: 2026-10-18-remove-observation-under-load/review-1.md
implemented: true
implemented_by: "claude/opus"
description: "A **fix** review of `2026-10-18-remove-observation-under-load/spec.md`"
review_scope: decision-screen-only
ready: true
findings: []
observations:
    - Watch-release loss diagnostics remain deferred
    - Repeated marker timing remains unexplained
recurrence: false
human_in_the_loop:
    - |-
        **Decide whether the macOS deletion interval is an acceptable mitigation.**

        The fix reduces the number of entries deleted between observation checks from 2,048 to 512 on macOS. The specification requires revisiting the interval or pacing design if the 100,000-entry regression sample still loses notifications, before claiming success.

        The implementation log records a 101,002-entry sample that stopped after 18,600 deletions. macOS reported UserDropped and MustScanSubDirs, meaning complete change coverage was unavailable. Retrying without a force flag completed the remaining 82,402 entries. A second sample, started after waiting 90 seconds after file creation, completed in 24.7 seconds without loss. These results do not establish why the first attempt lost notifications or whether 512 improves reliability over 2,048 under comparable conditions.

        The author should choose one course:

        - Accept 512 as an unproven mitigation, retain stopping on loss and explicit retries, and narrow the success criterion accordingly. This avoids more implementation work but accepts that this regression can still require a retry.
        - Authorize a small controlled comparison of freshly created and settled trees, including a smaller interval. This adds measurement work but can inform whether changing the interval helps.
        - Require a separate pacing design before closing the fix. This addresses deletion pressure more directly, but needs evidence for the pacing rule; macOS exposes no reliable queue-depth signal here.
        - Separately specify automatic recovery after a drop. This could reduce manual retries but changes the current stop-on-loss contract and requires its own safety design.

        Recommendation: authorize the small controlled comparison before accepting a changed success criterion. Keep the current stop-on-loss behavior while deciding. The implementation log recommends accepting the mitigation with a follow-up; the available samples do not yet demonstrate a comparative reliability improvement.
human_resolved: true
human_review: true
human_review_items:
    - |-
        Choose whether to accept the 512-entry macOS interval with a narrower success criterion, authorize controlled measurements, or require a new pacing design. The evidence, trade-offs, and recommendation are recorded in human_in_the_loop and the body below. Automatic recovery after lost notifications would need a separate design.
has_blocked_findings: false
blocked: false
---

# Review 1: author decision required

This review stops at the initial decision screen. Two independent agents reviewed the specification/plan and the implementation log for decisions and deferred work. No implementation-to-spec comparison or implementation tests were performed. Production readiness has **not been assessed**.

The `ready: true` value records that this decision-only review has no high findings, as required by the supplied frontmatter rule; it is not a production-readiness endorsement. The unresolved `human_in_the_loop` entry halts the review loop. The specification's `completed` flag follows the requested propagation from `ready`; implementation review remains outstanding, and the fix must stay in its current lifecycle directory.

## Decision needed

The specification says: “If 512 still loses coverage in that sample, the mitigation is unproven: record the result and revisit the interval or pacing design before claiming success.” The implementation log's Phase 6 records exactly that condition.

| Recorded run | Result | What it establishes |
| --- | --- | --- |
| Freshly created 101,002-entry tree | Lost notifications after 18,600 deletions, about 2,900 entries/second | Checking every 512 entries did not avoid this failure |
| Retry of that tree | Remaining 82,402 entries deleted without a force flag, about 8,200 entries/second | This recorded retry completed despite the earlier deletions |
| Second tree, after a 90-second settling period | All 101,002 entries deleted in 24.7 seconds, without loss | Completion was possible under different conditions |

The recorded flags were `UserDropped` and `MustScanSubDirs`. The proposed explanation that file-creation events were still being processed is plausible but unconfirmed. Different host load and settling times prevent these samples from isolating the effect of the interval.

The author needs to decide whether occasional stops and explicit retries satisfy the intended outcome, or whether more measurement or a different pacing rule is required first. Accepting the current interval changes the specification's success criterion. It should be an explicit author decision, not an implementation assumption. The available options and recommendation are written in full in the frontmatter handoff.

## Implementation decisions reported for information

The log reports routine choices that do not independently require human review: explicit monotonic-time arguments instead of a clock trait; asynchronous macOS flush requests repeated while waiting; retaining the existing post-move interruption reporting; reusing the existing loss representation in persisted diagnostics; and placing the slow-startup threshold in the library because removal proceeds there before returning to the CLI. It also records two regression samples instead of the requested single sample, to clarify the first failure. These are reported choices, not conclusions from a code audit.

The specification already selects stopping after notification loss. Continuing through a rescan is deliberately excluded; that open discussion does not leave permission to implement it implicitly. Earlier deferrals of documentation and cross-platform verification are reported completed in the log.

## Findings

None recorded. Implementation review and its requirement-to-test assessment remain outstanding; this is not a clean code-review result.

## Observations

### Watch-release loss diagnostics remain deferred

The implementation log reports that a loss during release of the Git administrative-directory watch remains queued and absent from diagnostics. It describes this as pre-existing and says a direct index/HEAD recheck remains in place. Preserve this as a follow-up for the author; this decision screen has not inspected the code or established a new defect.

### Repeated marker timing remains unexplained

The implementation log reports a longest marker round-trip value of 1,012 milliseconds in every native sample and leaves open whether that reflects actual delivery time or a bound inside the marker round. This diagnostic question remains uninvestigated and does not independently require a design decision now.

## Author decisions (2026-10-19)

Ken reviewed the open items one by one.

| Item | Decision | Result |
| --- | --- | --- |
| `human_in_the_loop`: the 512-entry macOS interval | **Accept 512 as a mitigation that lowers pressure but does not prevent loss.** Keep stop-on-loss and the explicit retry. The success criterion narrows from "512 avoids the loss in the sample" to "loss is detected, named, and recoverable by a plain retry", which run 1b/1c showed. The controlled comparison is scheduled as its own fix. | `2026-10-19-macos-drain-interval-comparison` |
| Observation: watch-release loss diagnostics | **Fix now, diagnostics only.** The spec lists `unregistration` as a reportable loss stage, so this was a gap in this fix's requirement, not a separate issue. | `Observation::release_admin` records `Backend::queued_loss()` at `unregistration`; unregistration still proceeds on the direct index/HEAD reread. Test: `observation::tests::a_loss_during_the_admin_release_is_recorded_in_the_diagnostics` |
| Observation: repeated 1,012 ms marker timing | **Investigate now.** | Real delay, quantized by fseventsd: across 912 probed rounds every round took about 11 ms or 1,002–1,016 ms, nothing between, independent of settle time and stream latency (0.1 s vs 0.5 s). No 1 s bound exists in `wt`'s wait path. Recorded in `docs/cli/remove.md` and the `worktree` skill. Details in the implementation log |

