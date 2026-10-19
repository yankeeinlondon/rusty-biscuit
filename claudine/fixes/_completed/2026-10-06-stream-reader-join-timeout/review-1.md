---
$schema: feature-review.yaml
ready: false
findings:
    - "high: Direct terminal writes still defeat bounded cleanup and reader isolation"
    - "high: Timeout snapshots discard the answer and can erase a known provider failure"
    - "high: Session-end records omit reader warnings and final-delivery loss"
    - "high: Sequence Markdown wrapping lacks the required real-terminal verification"
observations:
    - "Kimi wire joins retain their earlier unbounded behavior"
human_review: true
human_review_items:
    - |-
        The implementation chose the spec's recommended output strategy: after a terminal stalls, later loop iterations continue without terminal output, and an outstanding write may finish late. Confirm whether that loss of visibility is acceptable before closing the spec.

        - Keep this strategy, with the remaining delivery paths and diagnostics fixed as described below.
        - If later iterations must regain visible output, use cancellable terminal delivery or a separate writer process. Either choice needs a larger design and testing change.

        This is the author decision explicitly requested by the spec. It does not block repairing the findings below.
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: false
created: "2026-10-06T15:46:01-07:00"
spec: 2026-10-06-stream-reader-join-timeout/spec.md
implemented: true
implemented_by: claude/opus
log: claudine/fixes/2026-10-06-stream-reader-join-timeout/log.md
description: "A **fix** review of `2026-10-06-stream-reader-join-timeout/spec.md`"
fix: 2026-10-06-stream-reader-join-timeout/review-1.md
next: 2026-10-06-stream-reader-join-timeout/review-2.md
---

# Review: stream reader join timeout

The fix is **not production ready**. Typed reader outcomes and the bounded output queue improve the incident path, but four requirements remain unmet. None of the findings requires a human decision to begin repairing it.

## Findings

### High: Direct terminal writes still defeat bounded cleanup and reader isolation

**Authority — spec:** The terminal-cleanup requirement says the wrapper must finish within a bounded time when the terminal never accepts output, including “warnings” and “output-lock acquisition.” It also says not to “allow an unbounded new writer per iteration.” The result-preservation requirement prohibits readers from writing into the next iteration after cutoff. The implementation log's proposed deferral does not change these requirements.

**Defect class:** A bounded reader join cannot bound a subsequent synchronous terminal write or isolate a detached reader that continues using the terminal directly.

In claudine-cli, [eprint_reader_warning](../../cli/src/commands/wrap/exec/reader_join.rs#L300), the common warning printer used by captured and inherited runs, calls `eprintln!` directly. If an abandoned stderr forwarder holds stderr's lock, the main thread stops here indefinitely. The new worker's disabled flag cannot protect this write. Captured response delivery also takes the direct branch of [emit_final_message](../../cli/src/output/assistant.rs#L17), the CLI helper that writes the completed answer.

A temporary Level-1 probe invoked the actual inherited `run_child` entry point with `/bin/sh -c 'echo forwarded >&2'`, a noise filter enabling the stderr reader, and stderr's lock held behind a release signal. The child exited successfully; after the 5-second reader cutoff, the command still could not return at 6 seconds. Releasing the lock let it return with the timeout warning. Inspection confirms the warning printer has no deadline. A second probe gated the actual forwarding helper on `one\ntwo\n`, let its short join budget expire, then released it: **both lines were written after cutoff**.

The sweep covered the delivery sites changed by this fix and the callers reaching them:

| Site in claudine-cli | Shape checked | Observed result | Required result |
|---|---|---|---|
| Shared warning printer, reached by captured and inherited joins | Reader timeout with stderr unavailable; actual inherited entry-point probe, both call sites inspected | Main thread performs an unbounded direct write | Queue the warning and bound delivery |
| Inherited stdout and stderr readers in [inherited.rs](../../cli/src/commands/wrap/exec/spawn/inherited.rs) | Filtered lines through the shared `forward_lines` helper; gated forwarding probe and healthy/failing-sink tests | Cutoff detaches the writer; release allows subsequent lines to reach the sink | Stop subsequent presentation after cutoff and share the bounded worker |
| Semantic stderr passthrough in [semantic.rs](../../cli/src/commands/wrap/exec/spawn/semantic.rs#L552) | Unsuppressed provider stderr; source inspection of the same direct lock/write pattern | Reader uses stderr directly and never enters the run scope | Queue passthrough and reject late frames |
| Captured response and stderr delivery in [attempt.rs](../../cli/src/commands/wrap/harness_orch/attempt.rs#L417) | Nonempty captured output; caller and `emit_final_message(None)` inspected | Main-thread stdout/stderr writes bypass the worker | Bounded delivery before lifecycle completion |
| Semantic stdout, queued warnings, and trailer emissions through `StreamOutput` | Controlled blocked sink, unread result, closed run, healthy output | Existing Level-1 tests pass: callers continue and scoped late writes are rejected | Clean for these worker-backed emissions |

Route the remaining wrapper-owned delivery through the existing worker, including capture delivery and warning printing. Apply cutoff isolation to both readers. This uses the mechanism already added; a new platform-specific cancellation layer is unnecessary for the chosen strategy. Add entry-point tests that exercise cleanup, warning delivery, and a following iteration together: testing `drain` alone did not expose these bypasses.

### High: Timeout snapshots discard the answer and can erase a known provider failure

**Authority — spec:** The result-preservation requirement says a successful run “keeps the agent's exit code and result,” does not authorize “silently truncating the result,” and must “preserve an already established provider failure.”

**Defect class:** Reconstructing an outcome from completion status and duration loses result data and failures that the provider parser derives when finalizing its accumulated state.

In claudine-cli, [ResultSnapshot](../../cli/src/commands/wrap/run_scope.rs#L27), the reader's shared outcome record, contains only a turn status, duration, and terminal error. [settle_parser](../../cli/src/commands/wrap/exec/reader_join.rs#L399), which chooses the summary after the reader join, constructs a default summary when it sees a completed turn and exit 0. It loses the answer, session identity, usage, and provider-specific finalization. This branch is reachable when a descendant keeps stdout open **after the successful result has already been parsed**; it does not require a slow renderer.

Two temporary Level-1 probes used the claudine library's real Claude parser and the same snapshot publication rules as the CLI sink. The first copied the existing scripted Claude stream shape: initialization, an assistant message containing `review-answer`, and a successful result. A channel held the reader open after those lines. Normal finalization returned `review-answer`; timeout settlement returned success with an empty answer. The second copied the existing stopped-task regression fixture: a started task, its `stopped` notification, and a successful parent result with native exit 0. Normal finalization reported `incomplete_subagents`; snapshot settlement reported success.

| Settlement site/branch | Shape tested | Observed result | Required result |
|---|---|---|---|
| Completed-turn snapshot, reader still owns parser | Real Claude answer followed by held-open pipe | Exit 0, success, empty `assistant_text` | Preserve the parsed answer |
| Same snapshot branch | Existing Claude stopped-task fixture followed by reader cutoff | Known `incomplete_subagents` failure becomes success | Preserve the parser's failure and task outcomes |
| Parser already handed back | Same answer finalized normally; existing retained-parser timeout tests | Answer and parser verdict survive | Clean |
| Published terminal-error snapshot | Existing provider-error tests and outcome matrix | Error survives | Clean for the recorded error |
| No result, nonzero exit, interruption, provider timeout, panic | Existing outcome matrix | No invented success; termination overrides are applied | Clean for these tested shapes |
| Captured buffer settlement and inherited exit settlement | Existing partial-capture and failed-forward tests | Collected text/native exit retained | These do not use the defective semantic snapshot branch |

The semantic spawn has one common settlement call for all its providers; this is one shared defect, not a separate repair per provider. Snapshot the complete result and authoritative failure state before presentation, including the verdict normally produced by parser finalization. The matrix currently substitutes a small fake parser, so it cannot detect lost answer text or the real Claude task ledger. Keep at least these real-provider fixtures in the repaired tests.

### High: Session-end records omit reader warnings and final-delivery loss

**Authority — spec:** The reader-diagnostics requirement says that when stderr is blocked, the diagnostic must remain “in the session record where that record exists.” The result-preservation requirement says captured runs must “flag incomplete capture,” and the output design requires reporting discarded presentation while publishing exactly one `session_end` before lifecycle completion.

**Defect class:** Failure information is either discarded before reaching the public outcome or recorded before the final output operation that can discover the failure.

The claudine-cli spawn functions populate `ProcessResult.reader_warnings`, and captured output also populates `CapturedChildOutput.incomplete`. Their wrapper/composition callers do not read either field. The new [session-end writer](../../cli/src/commands/wrap/policy.rs#L445) only reads worker loss counters. Consequently, a successful semantic run whose reader timed out can have no persisted reader diagnostic, and captured composition treats a partial response as complete. Generic `output_incomplete` counters also do not preserve the reader's panic payload or timeout explanation.

Both the direct wrapper and composition write `session_end` **before** calling `section_stream.drain_final()`. A stall starting during final-answer or trailer delivery is therefore discovered too late to be included. A temporary Level-1 probe queued a trailer into the existing controlled sink: the exact `output_loss()` input used at record time was `None`; after the final drain expired, it reported `stalled: true`.

| Outcome publication site | Shape checked | Observed result | Required result |
|---|---|---|---|
| [Direct structured wrapper](../../cli/src/commands/wrap/wrapper_exec.rs#L199) | Reader warning; stall beginning during final delivery | Warning field unused; record precedes final drain | One record with the diagnostic and final delivery status |
| [Structured composition attempt](../../cli/src/commands/wrap/harness_orch/attempt.rs#L308) | Same reader/final-delivery cases | Same omissions and ordering | Same requirement |
| Captured composition branch in `attempt.rs` | Existing held-open capture fixture producing partial text and a warning | Low-level buffer survives, but caller ignores `incomplete` and warnings | Public attempt distinguishes partial capture |
| Inherited caller in `attempt.rs` | Existing failed-forward fixture | Native exit survives, but forwarding warnings are not carried into the attempt result | Keep the provider outcome and forwarding diagnostic distinct |
| Shared worker, loss discovered before summary rendering | Existing controlled stalled-sink tests; session writer inspected | `output_incomplete` is included | Clean for already-known worker loss |

Render/queue the final output, complete its bounded drain, then publish the single session-end record with both reader diagnostics and final output-loss status. Carry incomplete capture into the composition result instead of leaving it as an unused internal flag. Verify the stored record and public attempt outcome, rather than only testing that the low-level helper returns a warning.

### High: Sequence Markdown wrapping lacks the required real-terminal verification

**Authority — spec:** The gutter requirement says “Rendered Markdown in a sequence task must honor the same inset.” Verification explicitly includes “Markdown prose and code blocks inside a sequence gutter,” narrow terminals, cached options, and plain output. The review's test-rigor rule requires Level-2 capture for terminal width and rendering behavior.

**Defect class:** Tests of generated strings do not exercise the terminal boundary where the specified gutter and wrapping must remain intact.

The claudine library's [FinalMessage width tests](../../lib/src/render/final_message.rs#L145) and [assistant stream width tests](../../lib/tests/l1/assistant_stream_width.rs) prove the width option reaches rendering at Level 1. They do not run a sequence prompt in a terminal. Existing [Level-2 sequence capture tests](../../cli/tests/level2/level2_sequence_task_stream_capture.rs) verify narrow wrapping using **shell** tasks; their provider fixtures use short assistant text or a short held block. None tests a Markdown code panel or long rendered prose through the changed provider path in a narrow pane.

| Rendering site/shape | Strongest relevant verification | Result | Required verification |
|---|---|---|---|
| Final Markdown prose and code block | Level 1, generated line widths | Pass | Level-2 sequence pane capture |
| Streaming prose with cached options | Level 1, generated line widths | Pass | Level-2 sequence pane capture |
| One-column/narrowed terminal and explicit width options | Level 1 | Pass | Keep these layout edge checks; include a practical narrow pane in Level 2 |
| Plain piped output | Level 1, exact bytes | Pass | Appropriate level; retain it |
| Actual task gutters and narrow shell output | Existing Level 2 | Different rendering path | Does not establish the Markdown requirement |

Extend the existing Level-2 provider fixture with one long prose paragraph and one fenced code block in a narrow pane. Assert that wrapped lines retain their task gutter and content. No keyboard injection, new terminal backend, or extra CI cell is needed.

## Observations

### Kimi wire joins retain their earlier unbounded behavior

The claudine-cli [Kimi wire session](../../cli/src/commands/wrap/exec/wiring/session.rs#L297) still joins stdout and stderr without a deadline, discards the stdout panic payload, and replaces a stderr panic with empty capture. This fix changed its result construction but did not repair those joins; the implementation log also identifies them as open. Treat this earlier behavior as a separate follow-up, rather than an additional blocker in this review.

## Verification and requirement coverage

Reviewed the spec, implementation log and plan, implementation commits, current uncommitted documentation, affected readers, output worker, callers, and test declarations. No file-format/configuration reader was added or changed, so the input robustness matrix does not apply.

- `just test-cli commands::wrap::`: **1,177 passed**.
- `just test` in the worktree's claudine area: **8,424 passed, 9 skipped**. This includes the descendant-reap regression under full-suite load.
- Five temporary Level-1 review probes passed while asserting the defective behavior described above. Their controlled sinks/readers were released during teardown. All probe edits were removed and the three source files were checked against Git for exact restoration. Probe logs and source copies were retained under `/tmp/claudine-stream-reader-review-probes.log` and `/tmp/claudine-review-probe-*` on this host.
- `just check-tier-coverage claudine` was started from the repository root, then stopped after it pulled the nested rendezvous/DuckDB build into the audit. Its listing failure is an interrupted audit, not evidence of a stranded test. The new tests inspected here are compiled into the existing unit-test target or explicitly declared L1 binary, and the successful L1 run selected them.
- No new Level-2 verification was performed. The missing test is reported above; the earlier real-WezTerm boundary sample recorded in the docs addresses the stall-investigation requirement, not the sequence Markdown requirement.

The reader distinction/shared deadline and duration-unit requirements have appropriate Level-1 checks. The incident investigation records a real terminal sample, confirmed Claudine contributions, and uncertainty about the original WezTerm cause. The descendant-ready handshake retains the reap assertion and passed under suite load. Result preservation, complete bounded cleanup, and durable diagnostics remain incomplete as described in the findings. Cross-OS evidence is left to CI and does not affect this review's readiness decision.
