---
fixed: 2026-05-07
agent: claude
---

# Fix: OpenCode silently hangs after the last parallel subagent returns

## Symptom

When the user runs an OpenCode-wrapped composition that dispatches multiple
`task` (subagent) tools in parallel, the last visible activity on stderr is the
final `← Task(successful, ...)` tool result. After that the wrapper sits
indefinitely with no further output. Eventually the user presses Ctrl+C; the
work the subagents performed (e.g. four `git commit` operations) had already
completed long before.

Reproduction trace from the report:

```
 ← Task(successful, Commit context-variables docs)
 ← Task(successful, Commit shell expansion lib)
 ← Task(successful, Commit feature review doc)
 ← Task(successful, Commit prompt template)
^C^C
✓ 370s · 60K input tokens · 2K output tokens · 114K cached tokens · $0.06 cost basis · 16 tool calls
 User terminated non-interactive session with CTRL+C
```

## Root cause

OpenCode dispatches subagents via the `task` tool, which is emitted on the
parent stream as ordinary `tool_use` / `tool_result` events — **not** as
`task_started` / `task_completed`. As a result `LiveMetricsState.in_flight_subagents`
stays empty for OpenCode subagent runs. (Only Claude's hierarchical streams emit
`SubagentStart` / `SubagentStop` semantic events.)

Before the parallel `task` tools fire, OpenCode emits `step_finish` with
`reason: "tool-calls"`. Claudine routes that into `LiveMetricsState.provider_status`
via the `Info { step_phase: "finish", reason: "tool-calls" }` event observer.

When all parallel `task` tools complete (`tool_result` events arrive),
`in_flight` empties out. Normally OpenCode would now start a new step that
synthesises results and emits a final `step_finish` with `reason: "stop"`. In
practice it occasionally never emits that final step — the LLM "decides" it is
done but OpenCode never fires the terminal event and never closes stdout. The
parent process stays alive forever.

`detect_opencode_hang_termination` was the recovery path for this class of
silent-stall, but it was gated on `provider_status == Some("stop")`. Since the
last observed `step_finish.reason` was `"tool-calls"`, the gate never tripped
and the wrapper waited indefinitely.

## Fix

`claudine/cli/src/commands/wrap/exec/timeouts.rs::detect_opencode_hang_termination`
now fires when **all** of:

- silence ≥ `stop_threshold` (default 120 s),
- `in_flight` and `in_flight_subagents` are both empty, AND
- `provider_status.is_some()` (i.e. at least one `step_finish` boundary has
  been observed since session start; this preserves the original first-step
  grace and rules out slow-startup false positives).

The synthesised `CompletedButHung` message names the last observed
`step_finish.reason` so operators can distinguish a clean-finish hang
(`"stop"`) from a parallel-tool-dispatch hang (`"tool-calls"`).

## Tests added

In `timeouts.rs`:

- `detect_opencode_hang_termination_recovers_after_tool_calls_reason` —
  fires when the last `provider_status` was `"tool-calls"`.
- `detect_opencode_hang_termination_skips_when_no_step_finish_seen` —
  preserves first-step grace.
- `detect_opencode_hang_termination_skips_when_silence_below_threshold` —
  preserves the silence floor.
- `detect_opencode_hang_termination_skips_when_in_flight_tool` —
  preserves the in-flight tool gate.

Existing test `detect_opencode_hang_termination_recovers_after_stop_reason`
was tightened to assert the synthesised message still names the `"stop"`
reason.

## Docs updated

- `claudine/docs/topics/non-interactive-sessions.md` — replaced the stale
  "two-tier" recovery description (which referenced a removed
  `CLAUDINE_OPENCODE_HANG_TIMEOUT_SECONDS` env var and the
  `provider_stalled` synthesised error kind) with the actual single-rule
  recovery, and explained why OpenCode subagent dispatch does not populate
  `in_flight_subagents`.
