---
ready: false
agent: codex
model: ""
---

# Review: OpenCode Stderr Returns

## Findings

### High: `service=bus` records are not actually filtered from the wrapper stderr surface

The spec requires bus chatter to be "aggressively filtered" because it dominates INFO output and carries no useful signal. The bridge recognizes `service=bus`, but returns `StderrIngestOutcome::NotConsumed` from `OpenCodeLogBridge::handle_structured` ([reasoning.rs:210](../../lib/src/stream/logs/opencode/reasoning.rs#L210)). The stderr reader treats only `Consumed` as suppressed; anything else flows into formatting/capture and may be written or retained as raw stderr ([spawn.rs:808](../../cli/src/commands/wrap/exec/spawn.rs#L808), [spawn.rs:822](../../cli/src/commands/wrap/exec/spawn.rs#L822)).

That means bus records are absent from the semantic stream, but they are not filtered from the raw wrapper path. On failure, `capture_always` is enabled when a bridge exists, so this can still stuff the captured stderr with the exact high-volume INFO noise the spec says to drop.

Verification level present: Level 1 only, and only for "not in semantic events" ([opencode_stderr_lifecycle.rs:135](../../lib/tests/opencode_stderr_lifecycle.rs#L135)). There is no CLI-level assertion that bus records are suppressed from captured/rendered stderr. This is a requirement mismatch.

Recommendation: return `Consumed` for bus records, or introduce a distinct "filtered/suppressed" outcome that still prevents raw passthrough while avoiding semantic event emission.

### High: Subagent completion is emitted on the child's first `exiting loop`, not the parent-confirmed completion described by the spec

The spec maps `service=session.prompt session.id=<CHILD> ... exiting loop` followed by the parent `loop` line to `SubagentStop`. The implementation emits `SubagentStop` immediately for any tracked child session's first `StepExit` ([reasoning.rs:686](../../lib/src/stream/logs/opencode/reasoning.rs#L686), [reasoning.rs:696](../../lib/src/stream/logs/opencode/reasoning.rs#L696)). The comment even notes OpenCode emits `exiting loop` at the end of every step within a session ([reasoning.rs:691](../../lib/src/stream/logs/opencode/reasoning.rs#L691)).

For multi-step child sessions, this closes `LiveMetricsState.in_flight_subagents` too early. The user-facing stall diagnostics can then omit an actually active subagent, and the watchdog no longer has the real in-flight roster required by the acceptance criteria.

Verification level present: Level 1 fixture replay only, with one loop per child ([opencode-subagent-lifecycle.txt](../../lib/tests/fixtures/logs/opencode-subagent-lifecycle.txt#L8)). There is no fixture with a child `loop -> exiting loop -> loop -> exiting loop` sequence, nor a metrics assertion that the in-flight window remains open until the parent resumes. This is a requirement mismatch.

Recommendation: track child exits as pending and emit `SubagentStop` only when the parent session resumes/loops after that child exit, or when stdout task completion with matching `metadata.sessionId` provides the cross-stream completion attribution.

### High: Boot banner is required as a session anchor but is explicitly not promoted

The signal table requires `service=default ... version=... opencode` to emit `SessionStart` if no NDJSON `sessionID` has been seen. The implementation classifies the boot banner but then deliberately returns `NotConsumed` without emitting any semantic event ([reasoning.rs:257](../../lib/src/stream/logs/opencode/reasoning.rs#L257)). If the later `service=session ... created` line is missing, malformed, or delayed, the deterministic stream anchor required by the spec is absent.

Verification level present: none for the required fallback. The fixture includes a boot banner but only asserts one `SessionStart` overall; that start comes from `SessionCreated`, not from boot-banner fallback ([opencode_stderr_lifecycle.rs:64](../../lib/tests/opencode_stderr_lifecycle.rs#L64)).

Recommendation: implement the boot-banner fallback exactly as specified, gated by the same first-arrival-wins deduplication used for `SessionCreated`.

### High: End-of-run summary captures model but drops the primary provider id

The bridge stores both `primary_provider_id` and `primary_model_id` from the first primary LLM call ([reasoning.rs:629](../../lib/src/stream/logs/opencode/reasoning.rs#L629)), but `merge_stderr_state_into_summary` only backfills `summary.model` ([reasoning.rs:821](../../lib/src/stream/logs/opencode/reasoning.rs#L821)). `primary_provider_id` is never surfaced in `StreamExecutionSummary`; the summary schema currently only has the wrapper provider enum and model field ([summary.rs:57](../../lib/src/stream/summary.rs#L57)).

The acceptance criteria require the primary provider/model in the end-of-run summary. Today the model can appear, but the underlying OpenCode provider id (`providerID=...`) is lost except in transient `Info.extra`.

Verification level present: Level 1 state/unit tests confirm the value is stored, and a merge test confirms only `model` backfill. No test asserts summary provider-id output because there is no output field.

Recommendation: add an explicit summary field for the OpenCode model provider id, or put it in a structured provider-summary object that is serialized into JSONL/reporting and rendered where summaries expose model identity.

### High: Live renderer and watchdog acceptance criteria are not verified at the appropriate level

The implementation has useful Level 1 bridge tests, but the user-observable requirements are broader:

- continuous activity reporting in the live renderer during subagent work
- active subagents listed by real start time during a stall
- `step_timeout` firing only when both NDJSON and stderr semantic activity are silent
- primary provider/model visible in the final summary

Current tests mostly replay stderr into `OpenCodeLogBridge` and inspect emitted events ([opencode_stderr_lifecycle.rs:35](../../lib/tests/opencode_stderr_lifecycle.rs#L35)). The plan references one CLI watchdog test, but I did not find coverage that runs the fake OpenCode wrapper with structured stderr records and asserts renderer output, final summary contents, and watchdog in-flight subagent state over time.

Verification level present: Level 1 for parser/bridge behavior; partial Level 1 CLI timeout coverage for "no stderr and stdout idle". Missing Level 1/Level 2 coverage for the actual live-rendered surface and summary output. Given the review instructions, these user-observable requirements cannot be called production-ready yet.

Recommendation: add a fake OpenCode CLI integration test that emits stdout NDJSON plus structured stderr over a controlled timeline, captures wrapper stderr/stdout, and asserts rendered progress lines, summary model/provider attribution, and timeout behavior. Add a Level 2 terminal-capture test if exact terminal rendering, colors, glyph widths, or section spacing are considered part of the requirement.

## Notes

I attempted a focused test run:

```text
cargo test --color=never -p claudine opencode_stderr_lifecycle --test opencode_stderr_lifecycle
```

It was still compiling dependencies after roughly 60 seconds, so I stopped it to honor the non-interactive session constraint. No test result is claimed here.

## Production Readiness

Not ready for production. The core direction is sound, but the implementation still misses required behavior in stderr filtering, boot-banner promotion, subagent completion timing, summary provider attribution, and verification rigor for user-observable behavior.
