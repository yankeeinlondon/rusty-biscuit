---
$schema: feature-review.yaml
ready: false
findings:
    - "high: Direct diagnostics and panic reporting still escape bounded terminal delivery"
    - "high: A timeout inside completion handling still loses the answer and provider verdict"
observations:
    - "Kimi wire joins retain their earlier unbounded behavior"
    - "An unchanged prompt-guide test exceeds its deadline under suite load"
human_review: true
human_review_items:
    - |-
        The author decision identified in the spec and review 1 remains unresolved: accept that, after a terminal stall, later loop iterations continue silently and an outstanding terminal write may finish late.

        - Keep the single output worker and accept the loss of terminal visibility for the rest of the invocation.
        - Require later iterations to regain visible output, which needs cancellable delivery or a separate writer process and a larger design change.

        This existing decision can be resolved after the technical findings are repaired; it does not block either repair.
has_blocked_findings: false
blocked: false
reviewed_by: codex/gpt-6.1-sol
recurrence: true
created: "2026-10-06T17:57:35-07:00"
spec: 2026-10-06-stream-reader-join-timeout/spec.md
implemented: true
implemented_by: claude/opus
log: claudine/fixes/2026-10-06-stream-reader-join-timeout/log.md
description: "A **fix** review of `2026-10-06-stream-reader-join-timeout/spec.md`"
fix: 2026-10-06-stream-reader-join-timeout/review-2.md
previous: 2026-10-06-stream-reader-join-timeout/review-1.md
next: 2026-10-06-stream-reader-join-timeout/review-3.md
---

# Review: stream reader join timeout, iteration 2

The fix is **not production ready**. The repair closes the durable-diagnostics and terminal-verification findings, but its two documented implementation departures still violate the bounded-delivery and result-preservation requirements. Neither finding is blocked on a human decision.

## Prior findings and repair log

Review 1 puts all four findings under `Findings` and explicitly marks them unblocked; it has no separate `Unblocked Findings` or `Blocked Findings` sections. There were no blocked findings to unblock. Its pending author decision concerns terminal visibility after abandonment, rather than permission to repair the findings. The repair log disputes no finding. Its residual deferrals were checked against the spec, rather than treated as approved changes to the contract.

| Review 1 finding | Review 2 assessment |
|---|---|
| Direct terminal writes defeat bounded cleanup and isolation | Partially repaired. Provider forwarding, capture delivery, and warning presentation now use the worker. Direct diagnostics and panic reporting still escape it; finding 1 below carries the remaining sites. |
| Timeout snapshots discard the answer and provider failure | Partially repaired. A held-open pipe **after** the completion line retains the complete summary. A stall **inside** that line still loses it; finding 2 below sweeps every structured parser. |
| Session-end records omit reader warnings and final-delivery loss | Implemented. The common session-end writer drains final delivery before recording it, stores complete reader warnings, and the structured, captured, and inherited attempt branches carry output status. The relevant record and outcome tests pass. |
| Sequence Markdown lacks real-terminal verification | Implemented. New 50-column tmux captures verify a complete Claude message, streamed Claude text, and the Codex final-message path, each with long prose and a fenced code block. All three pass. |

## Findings

### High: Direct diagnostics and panic reporting still escape bounded terminal delivery

**Authority — spec:** The terminal-cleanup requirement says the wrapper must finish within a bounded time even when the terminal never accepts output, including “warnings” and “panic reporting.” The reader-failure requirement also says a reader that panics is reported as `parse_failure`, while a timeout must never be reported as a panic. Checking a gate only after a drain has already discovered the stall does not satisfy those requirements.

**Defect class:** Synchronous diagnostic writes can block before abandonment is declared, and closing the terminal gate cannot release a write that has already entered the operating system or waited on a standard-stream lock.

In claudine-cli, [queue_reader_warnings](../../cli/src/commands/wrap/exec/reader_join.rs), which submits reader diagnostics, calls `tracing::warn!` **before** queuing the warning. The production [tracing writer](../../cli/src/telemetry.rs) returns synchronous stderr while the gate is open. With `--debug warn` or `RUST_LOG=warn`, a blocked worker holding stderr therefore prevents the settling thread from reaching its bounded drain. The semantic spawn has the same tracing call before its drain. The repair log explicitly retains this pre-drain blocking window; that is an implementation departure, not a spec exception.

A controlled Level-1 probe used the real process-terminal worker and production tracing initialization. The sink held stderr's actual lock behind a release channel. Reader-warning tracing, the CLI status writer, and the performance report all remained blocked. An independent drain then disabled delivery, but those already-admitted writes still did not return until the sink was released. A second probe filled an operating-system pipe and invoked the Unix interrupt-feedback function: closing the gate did not release its already-blocked `write` either.

Panic reporting has a further consequence. The CLI installs color-eyre's panic hook without connecting it to the worker or gate. A third probe installed that actual hook, held stderr, closed the gate, and panicked in a tracked reader. The reader remained inside the hook until release, so the claudine-cli [reader join helper](../../cli/src/commands/wrap/exec/reader_join.rs) returned `TimedOut(Processing)` rather than `Panicked`. A genuine parser defect can consequently be reported as incomplete output or even preserve a previously successful result.

The sweep includes every diagnostic writer introduced or guarded by this repair, and the worker-backed delivery siblings:

| Site in claudine-cli | Shape checked | Observed result | Expected result |
|---|---|---|---|
| Captured and inherited warnings through `queue_reader_warnings` | Production tracing enabled; worker holds stderr before the first drain | Tracing blocks the settling caller before warning submission and drain | Submit diagnostics without synchronous terminal I/O |
| Semantic reader-warning loop in [semantic.rs](../../cli/src/commands/wrap/exec/spawn/semantic.rs) | Same production tracing writer and warning emission; call ordering inspected | Same blocking expression precedes the output deadline/drain | Same bounded diagnostic delivery |
| Shared [log writer](../../cli/src/log.rs), used by wrapper/composition status and budget notices | `log::message` while the worker holds stderr; all six public log functions share the inspected `emit` helper | Blocks while the gate is open; closing it does not release an admitted write | Wrapper status must use bounded delivery throughout the run |
| Shared [performance-report emitter](../../cli/src/perf/render.rs) | Actual CLI performance report with stderr held before abandonment | Direct `eprint!` blocks despite the gate | Queue the wrapper's performance presentation |
| [Unix interrupt feedback](../../cli/src/commands/wrap/exec/termination/unix.rs) | Actual feedback function with fd 2 pointing to a full pipe | Signal-feedback write blocks before the gate closes and remains blocked afterward | Signal handling must not depend on terminal delivery |
| [Windows interrupt feedback](../../cli/src/commands/wrap/exec/termination/windows.rs) | Matching branch inspected: gate check followed by synchronous stderr `write_all`; no native execution claimed | Same admission-then-block window; the std stderr locking mechanism is exercised by the status probe | Same bounded feedback contract |
| Process panic hook installed in [main.rs](../../cli/src/main.rs), shared by stdout/stderr readers in all three spawn modes | Actual color-eyre hook, real reader panic, stderr held, gate already closed | Hook bypasses the gate; panic is observed as a processing timeout | Reader unwinds with its payload independently of terminal delivery |
| Semantic stdout, semantic/Kimi stderr passthrough, inherited forwarding, capture delivery, warning presentation, and trailer through `StreamOutput` | Existing controlled blocked-sink, closed-run, following-iteration, and healthy-output tests | Clean: submission does not wait on the terminal, abandonment rejects later frames | Retain these repaired paths |

Route wrapper diagnostics through the existing worker while a wrapped invocation owns output. Keep signal handlers limited to recording/notifying the signal and let ordinary code submit feedback. Make the reader's panic reporting independent of synchronous stderr so the panic outcome remains available. This does not require cancellable terminal I/O or a new writer per iteration. Extend verification to the period **before** the first drain, with tracing enabled and a real panic hook; the new gate tests begin after `mark_stalled` and cannot detect that window.

### High: A timeout inside completion handling still loses the answer and provider verdict

**Authority — spec:** The result-preservation requirement says a successful run “keeps the agent's exit code and result,” “including inside the handling of a line,” and does not authorize “silently truncating the result.” It also requires preserving “an already established provider failure.” The repair's admitted success-without-answer fallback violates these explicit cases.

**Defect class:** Publishing the full summary only after synchronous completion callbacks return leaves settlement with a completion marker that cannot reproduce the parser's answer or authoritative failure state.

In claudine-cli, [run_scope::feed_line](../../cli/src/commands/wrap/run_scope.rs), which publishes the reader's recoverable state, calls the parser before taking its full snapshot. The [live semantic sink](../../cli/src/commands/wrap/live_semantic_sink/event_sink.rs) publishes a coarse `TurnComplete` marker, then renders, logs, and dispatches hooks. If any of that work stalls, the full summary is still absent. [settle_parser](../../cli/src/commands/wrap/exec/reader_join.rs), which chooses the final outcome at cutoff, then constructs success from the marker and child exit 0, with empty assistant text and no provider finalization verdict.

Level-1 probes used the real provider parsers and `LiveSemanticSink`, copied existing provider fixtures, and held the completion event's logging callback behind a channel. The reader was demonstrably processing the completion line when its short injected budget expired. For **all nine structured provider identities, using eight parser implementations**, settlement returned success with an empty answer. Releasing the callback and finalizing the original parser supplied a nonempty answer, proving the bytes had already been parsed. The existing Claude stopped-task fixture additionally finalized as `incomplete_subagents`, but cutoff settlement returned success. Rendering no longer blocking the terminal does not eliminate synchronous logging or hook dispatch from this path.

| Parser/delivery site | Shape tested | Observed cutoff result | Expected result |
|---|---|---|---|
| Claude structured parser | Existing initialization, assistant answer, and successful `result`; completion callback held | Exit 0, empty answer | Parsed answer, session/usage, and successful verdict retained |
| Claude task-ledger finalization | Existing started/stopped subagent plus successful parent `result`; same callback hold | Failure becomes success, empty answer | Retain `incomplete_subagents` and the answer |
| Codex structured parser | Existing completed agent message and `turn.completed`; same hold | Exit 0, empty answer | Retain the parsed answer and verdict |
| Gemini structured parser | Existing assistant-message and successful `result` shapes; same hold | Exit 0, empty answer | Same preservation |
| OpenCode structured parser | Existing text and `step_complete` shapes; same hold | Exit 0, empty answer | Same preservation |
| Kilo through the OpenCode parser | Same OpenCode fixture with the Kilo identity; same hold | Exit 0, empty answer | Same preservation |
| Qwen structured parser | Existing assistant message and `summary` shapes; same hold | Exit 0, empty answer | Same preservation |
| Pi structured parser | Existing text delta, completed message, and `agent_end`; same hold | Exit 0, empty answer | Same preservation |
| Kimi structured parser | Existing wire text and `TurnEnd` shapes; same hold | Exit 0, empty answer | Same preservation; its separate wire-session joins are outside this settlement path |
| Antigravity structured parser | Existing successful envelope with response, duration, and usage; same hold | Exit 0, empty answer | Same preservation |
| Full summary published after the completion line | Existing held-open Claude/Codex streams, Claude stopped-task and provider-error fixtures | Clean: answer, failure, session, and usage retained | Keep these controls |
| Parser already handed back; missing result, nonzero exit, interruption, provider timeout, and ordinary panic | Existing reader outcome matrix | Clean for the tested outcomes; no invented successful result | Keep these controls; finding 1 covers the blocked panic hook |
| Captured and inherited settlement, including Goose’s captured path | Partial-buffer and forwarding-outcome tests; Goose’s lack of a structured stream inspected | Clean for this class: neither reconstructs a semantic result from `TurnComplete` | Preserve existing behavior |

Separate parsing from potentially blocking sink work enough to publish the complete authoritative outcome before completion callbacks run. The claudine library's [SemanticStreamParser::snapshot](../../lib/src/stream/parser.rs), which computes a complete provider summary without consuming the parser, already returns the correct summary for every parser; the defect is the publication boundary. Do not retain the coarse-marker branch as permission to claim success without the result. Add callback-held controls alongside the held-open-pipe tests, checking answer text and the real Claude task-ledger failure.

## Recurrence

Both high findings repeat classes from [review 1](review-1.md).

- **“Direct terminal writes still defeat bounded cleanup and reader isolation.”** That repair should have swept direct diagnostics as well as provider presentation: the shared log helper, tracing writer and pre-drain warning sites, performance emitter, both interrupt-feedback implementations, and panic hook. The gate guards only writes made after abandonment; the complete remaining list is in finding 1.
- **“Timeout snapshots discard the answer and can erase a known provider failure.”** That repair should have swept both sides of the completion boundary for every structured parser: a held-open source after the line **and** a stalled callback inside it. It fixed the former. Finding 2 covers the latter for Claude, Codex, Gemini, OpenCode, Kilo, Qwen, Pi, Kimi, and Antigravity, plus Claude's task ledger.

These are bounded requirements already named by the spec. The review does not request recovery of terminal visibility or stronger guarantees for arbitrary new inputs.

## Observations

### Kimi wire joins retain their earlier unbounded behavior

The claudine-cli [Kimi wire session](../../cli/src/commands/wrap/exec/wiring/session.rs) still joins its reader threads without the new deadline. Its stderr presentation now uses the worker, but the join behavior predates this repair and was already an observation in review 1. It remains a separate follow-up, not a blocker added to this cycle.

### An unchanged prompt-guide test exceeds its deadline under suite load

The claudine-cli [prompt-guide robustness test](../../cli/tests/l1/prompt_guide_defects.rs), which composes 14 temporary documents, hit nextest's 30-second limit during the full-area run after reaching its twelfth case. The unchanged test passed alone in 7.769 seconds. Its assertions did not fail, but the full run stopped before all selected tests executed. This is outside the stream-reader repair's test changes; the evidence shows sensitivity to suite load, without establishing a production regression. No retry policy or assertion was changed.

## Verification and requirement coverage

No file-format/configuration reader was added or changed by this repair: the new parser API snapshots existing state. The input robustness matrix does not apply. New unit tests belong to declared unit targets; `parser_snapshot` is declared in the library's L1 binary. The new terminal cases belong to the declared `level2` target, require `terminal-tests`, and were selected by the live `test-l2` recipe.

| Spec requirement | Appropriate verification and assessment |
|---|---|
| Distinguish reader panics/timeouts and show diagnostics | Level 1. Existing tests pass; the real blocked-hook and pre-drain tracing probes expose finding 1. |
| Pipe settle period, short cap, long cap, shared reader clock | Level 1. Focused reader tests pass. |
| Preserve result/failure and isolate completed runs | Level 1. Held-open-source and outcome controls pass; callback-held real-provider probes expose finding 2. |
| Bound all terminal cleanup | Level 1 with controlled sinks and real stream locks/pipes. Worker-backed tests pass; direct diagnostics remain incomplete. No terminal input-encoder behavior is asserted. |
| Investigate the WezTerm stall | The earlier real-WezTerm boundary sample and its confirmed Claudine contributions remain recorded in the timeout topic page. This review does not remeasure or attribute the original stall to WezTerm. |
| Duration messages carry units | Level 1 boundary table covers 59, 60, 120, 3599, and 3600 seconds. Pass. |
| Sequence Markdown honors its gutter inset | Level 1 width/cached-option/plain-output controls plus Level 2 narrow-pane capture of complete, streamed, and final Markdown. Pass at the required level. No keyboard-injection requirement exists. |
| Reliable descendant reaping under suite load | Level 1 full-area run selects the strengthened shell-task regression. Results below. Cross-OS evidence remains CI's responsibility. |

Verification results:

- `just test-cli commands::wrap::`: **1,201 passed**.
- Six temporary Level-1 probes: **6 passed**, including five that assert the two findings above. The provider probe swept all eight parser implementations; a final repeat added Kilo’s separate provider identity and passed for all nine structured providers (`/tmp/claudine-review-2-kilo.log`). The supplementary probe inspected existing worker error accounting; it is not contract coverage for either finding. All blocked readers/sinks were released during teardown. All three edited source files were restored byte-for-byte to their saved worktree contents; sources and logs are under `/tmp/claudine-review-2-probes` and `/tmp/claudine-review-2-probes-complete.log` on this host.
- `BISCUIT_TEST_REQUIRED_BACKENDS=tmux just test-l2 level2_sequence_task_stream_capture`: the CLI ran **13 tests, all passed**, with `tmux run=13 skip=0 panic=0`. The area recipe subsequently selected zero generator tests for this CLI-only filter and failed its required-backend proof. That recipe exit is not a Markdown test failure; it is not reported as a passing area-wide L2 run. No test window took focus.
- The first full L1 attempt ran alongside the L2 build and stopped after **5,000 passes and one missing diagnostic-snapshot fixture**. Both recipes build the same CLI binary with different features. The fixture test passed after the fixture-enabled binary was rebuilt. The final full-area run was therefore performed alone, after restoring the probes; that run stopped at the separate prompt-guide timeout described above. A subsequent full run uses four nextest workers, with the same tests, assertions, deadlines, and zero-retry policy, to complete the listing with less contention.

- Full L1 at the default worker count, after restoration: **6,019 passed, one timed out, 9 skipped; 2,439 tests were not executed**. The shell-task descendant-reaping regression passed in this run. The prompt-guide timeout is recorded as an observation, not hidden by its isolated pass.
- Full L1 with four workers: **8,459 passed, 9 skipped** in 240.276 seconds. The same full listing ran with no retry policy or test changes. The earlier default-load timeout remains recorded above.
