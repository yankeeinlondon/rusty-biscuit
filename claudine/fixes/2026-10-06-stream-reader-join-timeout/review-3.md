---
$schema: feature-review.yaml
ready: false
findings:
    - "high: Terminal write errors are reported as complete delivery"
observations:
    - "Kimi wire joins retain their earlier unbounded behavior"
human_review: true
human_review_items:
    - |-
        The spec's existing author decision remains unresolved: accept that later loop iterations continue silently after a terminal stall, and that one outstanding write may finish late. This review does not change that decision or require it before repairing the finding.

        - Accept the single output worker and the loss of terminal visibility for the rest of the invocation.
        - Require later iterations to regain visible output, which needs cancellable delivery or a separate writer process and a larger design change.
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: "2026-10-06T18:52:07-07:00"
spec: 2026-10-06-stream-reader-join-timeout/spec.md
implemented: true
implemented_by: claude/opus
log: claudine/fixes/2026-10-06-stream-reader-join-timeout/log.md
description: "A **fix** review of `2026-10-06-stream-reader-join-timeout/spec.md`"
fix: 2026-10-06-stream-reader-join-timeout/review-3.md
previous: 2026-10-06-stream-reader-join-timeout/review-2.md
next: 2026-10-06-stream-reader-join-timeout/review-4.md
---

# Review: stream reader join timeout, iteration 3

The fix is **not production ready**. Both blockers from review 2 are repaired. One remaining output-accounting defect reports failed terminal writes as complete delivery, including through the shipped CLI. It can be repaired without changing the chosen delivery strategy.

## Prior findings and repair log

Review 2 lists its two findings under `Findings` and explicitly marks both unblocked. It contains no blocked findings, and no finding was subsequently unblocked. The outstanding author decision concerns terminal visibility after abandonment. The repair log marks neither finding disputed or deferred.

| Review 2 finding | Assessment |
|---|---|
| Direct diagnostics and panic reporting still escape bounded terminal delivery | Implemented. The claudine-cli [terminal diagnostic route](../../cli/src/terminal_gate.rs) queues log, tracing, performance, panic, interrupt, and library-console output once a wrapped run creates the worker. The blocked-standard-stream tests pass before any drain declares a stall. Unix handlers notify ordinary code rather than writing to the terminal. |
| A timeout inside completion handling still loses the answer and provider verdict | Implemented. The claudine-cli [reader's event-delivery boundary](../../cli/src/commands/wrap/run_scope.rs) publishes the full parser summary before completion callbacks run. The completion-marker fallback is removed. Tests hold both logging and hook callbacks for all nine structured provider identities and retain answer, session, usage, and verdict; Claude's stopped-subagent failure is also preserved. |

The sweep checked the generic, Codex, and OpenCode parser-builder branches, semantic stdout and stderr readers, captured and inherited settlement, warning sites, shared diagnostic writers, interrupt feedback, library console writers, panic-hook installation, and ordinary shutdown. The repaired classes are not re-raised. Review 1's Markdown gutter verification and reader-warning persistence remain present and pass their relevant checks. The earlier prompt-guide load sensitivity did not recur in this full-suite run.

## Unblocked Findings

### High: Terminal write errors are reported as complete delivery

**Authority — spec:** The result-preservation requirement says “inherited runs must distinguish failed forwarding from a provider failure,” and the output-delivery decision requires reporting discarded presentation: “Discarding terminal presentation may be necessary after the output budget expires; report that loss.” A sink returning `BrokenPipe` is failed forwarding, even though it returns promptly rather than exhausting the drain deadline. A downstream consumer closing a pipe early is realistic; the provider's successful result should survive with a distinct delivery-loss indication.

**Defect class:** Discarding a terminal write's I/O error makes an empty output queue look like successful delivery to every consumer of the shared worker.

In claudine-cli, [write_frames](../../cli/src/commands/wrap/output_worker.rs#L312), the worker that delivers queued output, discards `sink.write(...)` errors and marks the write finished. It updates no loss counter. Consequently, the drain returns `Complete`, inherited forwarding returns no warning, captured attempts report complete output, and structured attempts and session records have no delivery-loss indication. The stdout/stderr adapters correctly avoid failing provider parsing when they submit a frame; the missing information is at the worker's actual write boundary.

A temporary Level-1 test ran the **shipped `claudine compose --no-interactive --claude` command** through `CliProcessFixture`, using the existing scripted Claude stream shape: initialization, assistant answer, and successful result. The control delivered `done` and stored one successful session-end record. Two otherwise identical runs closed the reader of Claudine's stdout pipe or stderr pipe immediately after spawning. Both exited successfully and stored one session-end record **without `output_incomplete`**, despite losing terminal delivery. No artificial timeout or oversized message was needed.

Three additional Level-1 probes substituted a sink returning `BrokenPipe`, confirmed that its write was actually attempted, and checked the existing forwarding entry point, capture delivery, diagnostic route, public attempt status, and stored session record. The same small answer and warning were used on both streams, with healthy-sink controls.

| Site in claudine-cli | Shape tested | Observed result | Expected result |
|---|---|---|---|
| Shared output worker | Small stdout or stderr frame; sink returns `BrokenPipe` | Drain completes and loss counters remain empty | Record the failed delivery without changing the provider verdict |
| Inherited stdout forwarding | Existing platform-shell fixture emits an answer; stdout sink fails | Child exit 0 retained, no reader/forwarding warning | Retain exit 0 with a forwarding-loss warning |
| Inherited stderr forwarding | Same fixture emits a warning; stderr sink fails | Same omission | Same distinct forwarding-loss report |
| Captured response and stderr, through [deliver_capture and captured_output_status](../../cli/src/commands/wrap/harness_orch/attempt.rs#L567) | Short captured answer/warning; separately fail stdout and stderr | `OutputStatus::is_complete()` is true | Attempt reports incomplete delivery while preserving captured text and child exit |
| Structured wrapper/composition, through the shared [session-end writer](../../cli/src/commands/wrap/policy.rs#L477) and structured attempt status | Queued answer/trailer; separately fail stdout and stderr; shipped-compose closed-pipe reproduction | Attempt reports complete delivery; record omits `output_incomplete` | Attempt and the single session-end record carry the delivery loss |
| [Diagnostic route](../../cli/src/terminal_gate.rs), shared by log, tracing, performance, panic reports, interrupts, and library console lines | Routed diagnostic on each stream; sink fails, then publish the session record | Diagnostic submission succeeds; delivery failure is absent from status and record | Submission remains nonblocking; failed delivery is accounted for |
| All corresponding healthy-sink controls | Same short frames with successful writes | Complete delivery, expected content, no loss report | Clean |
| Existing blocked-sink and full-queue controls | Deadline expiry or queue-capacity rejection | Worker counters report loss; inherited/structured consumers inspect that loss | Clean for those tested cases; they do not exercise a returned write error |

Account for failed writes at the shared worker, then carry that information through inherited warnings, captured delivery status, and structured attempt/session reporting. Captured status currently consumes only `Drained::Disabled`, so it needs a delivery result that can also represent a completed drain containing failed writes. Keep the provider's exit code and parsed/captured result intact. Verify the small failing-sink cases and the closed-pipe CLI case alongside their healthy controls. No cancellation layer, new output syntax, or additional worker is needed.

## Blocked Findings

None. The author decision does not block the repair above.

## Recurrence

This finding repeats the delivery-loss reporting class from review 1's **“Session-end records omit reader warnings and final-delivery loss.”** That repair correctly moved recording after the final drain and carried reader warnings into attempt outcomes. Its sweep should also have checked a sink that **returns a write error**, alongside a sink that blocks until a deadline, at the worker and all three result consumers: inherited forwarding, captured delivery, and structured attempt/session publication. This review checks both streams, diagnostics, and healthy controls across those sites.

The remaining case is explicitly covered by the spec's failed-forwarding requirement. The repair needs loss accounting in the existing mechanism, rather than another layer of delivery machinery.

## Observations

### Kimi wire joins retain their earlier unbounded behavior

The claudine-cli [Kimi wire session](../../cli/src/commands/wrap/exec/wiring/session.rs) still joins its readers without the new deadline. That behavior predates this fix and was an observation in both earlier reviews. Its stderr delivery now uses the worker; its join behavior remains a separate follow-up rather than a blocker for this cycle.

## Verification and requirement coverage

No file-format or configuration parsing behavior was added or changed: the parser API snapshots existing state and the new sink changes event delivery. The input robustness matrix does not apply. The added reader/routing tests compile in the existing unit target; `parser_snapshot` is declared in the library's L1 binary. Markdown pane tests compile in the declared `level2` target with `terminal-tests` and are selected by the live `test-l2` recipe. No keyboard-encoder behavior requires Level 3.

| Requirement | Verification level and assessment |
|---|---|
| Distinguish panic from timeout and report reader diagnostics | Level 1, including the production tracing writer and real panic-report hook with standard-stream locks held. Pass. |
| Short pipe cap, settle period, long cap, shared reader clock | Level 1 reader-state and shared-budget tests. Pass. |
| Preserve answers/failures and isolate completed readers | Level 1 real-provider callback-held and held-open-pipe tests, outcome matrix, and late-output controls. Pass for result preservation. Failed-forwarding reporting remains the finding above. |
| Bound terminal cleanup, queue memory, and writer count | Level 1 controlled blocked sinks, ticker/trailer delivery, diagnostic routing, and next-iteration rejection. Pass for bounded completion. Returned I/O errors remain unreported. |
| Explain the WezTerm stall | The earlier real-WezTerm/XOFF sample, confirmed synchronous-write and repeated-option-detection contributions, and unresolved original terminal cause remain documented in the timeout topic page. No new attribution to WezTerm is made. |
| Duration messages carry units | Level 1 boundary cases at 59, 60, 120, 3599, and 3600 seconds. Pass. |
| Sequence Markdown honors the gutter inset | Level 1 width, cached-option, and plain-output controls; Level 2 narrow tmux captures for complete Claude text, streamed Claude text, and the Codex final-message path, including prose and code blocks. Pass at the required level. |
| Reliable descendant reaping under suite load | Level 1 strengthened shell cleanup regression passed in the full package-area run. |

Results from this review:

- Focused `just test-cli commands::wrap:: terminal_gate log::tests perf_report_is_queued`: **1,224 passed**.
- Full package-area `just test`: **8,473 passed, 9 skipped**, at the default worker count. The descendant-reaping and previously slow prompt-guide tests passed.
- `just test-l2 level2_sequence_task_stream_capture`: **13 CLI tests passed**; the subsequent generator selection contained zero tests and the recipe exited successfully. Pane tests ran through the repository harness without taking focus.
- Three temporary failing-sink probes: **3 passed**, asserting the defective behavior in the finding. The corrected shipped-CLI probe: **1 passed**, covering healthy, closed-stdout, and closed-stderr runs. Its first attempt used a result-only stub without a streamed assistant message and failed the positive-output control; that fixture was corrected before using its results.
- Every temporary source edit was restored byte-for-byte. Probe sources and logs are retained under `/tmp/claudine-review-3-probes`, `/tmp/claudine-review-3-probes.log`, and `/tmp/claudine-review-3-cli-probe.log`; suite logs are `/tmp/claudine-review-3-l1.log` and `/tmp/claudine-review-3-l2.log` on this host.

Cross-OS evidence remains CI's responsibility and does not affect this readiness decision.
