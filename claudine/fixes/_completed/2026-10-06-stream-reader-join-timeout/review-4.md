---
$schema: feature-review.yaml
ready: true
findings: []
observations:
    - "Kimi wire joins retain their earlier unbounded behavior"
human_review: true
human_review_items:
    - |-
        The spec's existing author decision remains unresolved: confirm whether later loop iterations may continue silently after a permanent terminal stall, with one write already in progress allowed to finish late. This review introduces no new design decision.

        - Accept the single output worker and the loss of terminal visibility for the rest of the invocation.
        - Require later iterations to regain visible output, which needs cancellable delivery or a separate writer process and a larger design change.
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-10-06T20:07:00-07:00"
spec: 2026-10-06-stream-reader-join-timeout/spec.md
implemented: false
description: "A **fix** review of `2026-10-06-stream-reader-join-timeout/spec.md`"
fix: 2026-10-06-stream-reader-join-timeout/review-4.md
previous: 2026-10-06-stream-reader-join-timeout/review-3.md
---

# Review: stream reader join timeout, iteration 4

The fix is **production ready** under the review's readiness rules. Review 3's blocking finding is implemented, and this review found no new high, medium, or low findings. The existing author decision about terminal visibility after permanent abandonment remains a separate human-review item.

## Prior findings and repair log

Review 3 contains one unblocked finding, **“Terminal write errors are reported as complete delivery.”** It contains no blocked findings; none was subsequently unblocked. The repair log marks the finding fixed, with none deferred or disputed.

The claudine-cli [output worker](../../cli/src/commands/wrap/output_worker.rs), which delivers queued terminal frames, now counts a returned write error before clearing its active-write state. A drain that observes an idle queue therefore also observes the failure. The loss counters distinguish successful draining from successful delivery, without changing the provider's exit code or result. Later frames can still reach the healthy stream when the other stream has closed.

The following sweep checks the entire delivery-loss class reported in review 3. Each failing-stream case has a healthy-sink control.

| Site in claudine-cli | Shape tested | Result |
|---|---|---|
| Shared output worker | Small stdout and stderr frames; each stream separately returns `BrokenPipe` | Failed frames are counted, the drain finishes, and the healthy stream keeps its content and order. |
| Per-run loss accounting | Failed writes before and after a saved counter reading | Only subsequent failures enter the run's counter difference. |
| [Inherited forwarding](../../cli/src/commands/wrap/exec/spawn/inherited.rs) | Platform-shell fixture writes both streams; separately fail stdout and stderr | Child exit 0 survives with a distinct forwarding-loss warning. |
| [Captured delivery and attempt status](../../cli/src/commands/wrap/harness_orch/attempt.rs) | Short captured answer and stderr; separately fail each stream | Captured text and provider success survive; attempt status reports incomplete delivery. |
| Structured attempt and [session-end publication](../../cli/src/commands/wrap/policy.rs) | Successful summary with answer/trailer; separately fail each stream | Attempt status and the single stored record include delivery loss and `failed_writes`. |
| [Diagnostic route](../../cli/src/terminal_gate.rs), shared by status, tracing, performance, panic, interrupt, and library-console writers | Small diagnostic on either failing stream | Submission remains nonblocking; its failed delivery reaches the session record. |
| Shipped `claudine compose --no-interactive --claude` | Existing scripted initialization, assistant answer, and successful result; close Claudine's stdout or stderr reader after spawning | Both runs exit successfully and store exactly one session-end record with failed-write loss. The healthy control prints the answer and stores no incomplete-output marker. |
| Existing blocked-sink, queue-capacity, and following-iteration controls | Permanently blocked delivery, rejected frames, and late reader output | Bounded cleanup and loss reporting remain intact; no replacement writer or subsequent scoped output is admitted after abandonment. |

The repair stays within the existing worker and status types. Its documented choices—process-wide counters and continuing with later frames after a returned error—do not conflict with the spec. A cancellation layer or per-stream recovery mechanism is unnecessary for this repair.

Reviews 1 and 2 were also checked. Full-summary publication still precedes completion callbacks for all nine structured provider identities. Their answer, session, usage, and verdict tests remain present, including Claude's stopped-subagent failure. Diagnostics and panic reporting remain queued while a wrapped invocation owns output; captured and inherited warnings reach their attempt outcomes; final delivery is drained before the session-end record is stored. The Markdown gutter tests added after review 1 remain active.

## Unblocked Findings

None.

## Blocked Findings

None. The existing author decision does not create a blocked finding or change `ready`.

## Recurrence

No finding is re-raised. The delivery-loss class that recurred in review 3 is now repaired across the worker, all three run paths, diagnostics, both streams, and stored session records. `recurrence` is therefore `false` for this review.

## Observations

### Kimi wire joins retain their earlier unbounded behavior

The claudine-cli [Kimi wire session](../../cli/src/commands/wrap/exec/wiring/session.rs), which coordinates that provider's separate wire protocol, still joins its reader threads without a deadline and replaces a stderr-reader panic with empty capture. This predates the fix and was an observation in every earlier review. Its terminal writes now use the worker; the earlier join behavior remains information for a separate follow-up, not a repair requested by this cycle.

## Verification and requirement coverage

No file-format or configuration input contract changed. The parser changes expose existing accumulated state and defer semantic-event delivery; the input robustness matrix does not apply.

The new worker and status tests compile in the existing CLI unit target. The closed-pipe tests are declared in the consolidated L1 target and were selected by the canonical recipe. They retain an explicit Unix scope; the synthetic failing-sink tests are portable. The Markdown pane tests are declared in the `terminal-tests` Level-2 target and selected by the live `test-l2` recipe. No requirement exercises a keyboard encoder or needs Level 3.

| Spec requirement | Verification level and assessment |
|---|---|
| Report reader panic and timeout distinctly, retaining panic payload and visible diagnostics | Level 1 reader-outcome, payload, warning, tracing, and actual panic-hook checks. Pass. |
| Apply the short cap only to a settled pipe wait; retain the long cap and shared reader clock | Level 1 reader-state, settle-period, completed-at-boundary, and shared-budget checks. Pass. |
| Preserve complete results and primary failures; flag partial capture and failed forwarding; isolate completed readers | Level 1 real-provider callback-held and held-open-source fixtures, outcome matrix, captured/inherited status checks, late-output controls, and stored-record checks. Pass. |
| Bound terminal cleanup, queue memory, and writer count | Level 1 controlled blocked sinks, ticker/trailer and diagnostic checks, queue limits, and subsequent-iteration rejection. Returned write errors now report loss as well. Pass. |
| Investigate the WezTerm stall without claiming an unproved terminal defect | Earlier real-WezTerm/XOFF boundary measurements remain recorded in the timeout topic page, together with confirmed synchronous-write and repeated-option-detection contributions and the unresolved original terminal cause. Appropriate integration evidence; no new terminal attribution is made. |
| Include units in step-timeout durations | Level 1 boundary cases at 59, 60, 120, 3599, and 3600 seconds. Pass. |
| Keep rendered Markdown inside the sequence gutter | Level 1 supplied-width, cached-option, narrow-width, and plain-output controls; Level 2 narrow tmux captures for complete Claude text, streamed Claude text, and Codex final Markdown, each including prose and code blocks. Pass. |
| Reliably reap the shell-task descendants under full-suite load | Level 1 strengthened process-group and descendant regression, exercised in the package-area suite. Pass. |

Results from this review:

- Focused `just test-cli commands::wrap:: terminal_gate log::tests perf_report_is_queued compose_closed_pipe_delivery`: **1,233 passed**.
- `just test-l2 level2_sequence_task_stream_capture`: **13 CLI tests passed**, including all three Markdown gutter captures. The generator selection contained zero tests and exited successfully. The repository harness preserved terminal focus.
- Sequential full package-area `just test`: **8,482 passed, 9 skipped**, at the default worker count. The descendant-reaping and notification-stall regressions both passed.

The first full-suite attempt was invalidated by reviewer orchestration: while its tests were running, the Level-2 recipe rebuilt the shared `claudine` executable without `test-fixtures`. The notification-stall test then reached the host backend instead of its compiled test seam and failed. Both recipes finished before the full suite was started again, sequentially, with its required features. No source, assertion, retry policy, or worker limit was changed to address that failure.

Logs are retained on this host at `/tmp/claudine-review-4-focused.log`, `/tmp/claudine-review-4-l2.log`, `/tmp/claudine-review-4-l1.log` (invalid overlapping run), and `/tmp/claudine-review-4-l1-sequential.log`. Cross-OS evidence remains CI's responsibility and does not affect this review's readiness decision.
