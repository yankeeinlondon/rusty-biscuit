# OpenCode Stability — Specification

This specification describes a class of hangs observed when wrapping OpenCode
under `claudine compose` / `inline-compose` / `sequence`, and the watchdog
contract that a fix must satisfy. The upstream cause sits inside OpenCode, but
claudine has no defenses today and silently deadlocks the user. This document
is the input to a follow-up implementation plan; it does not itself change
code.

## Problem

When OpenCode spawns parallel `task` subagents during a non-interactive run,
some subagents go silent without ever emitting a `task_completed` (or
`task_error`) event. The parent OpenCode agent waits indefinitely for those
tool callbacks, never closes stdout, and never exits. Claudine's wrapper
blocks on the stdout reader thread waiting for EOF and on `child.wait()`
waiting for process exit. The user has no recourse but Ctrl+C.

The deadlock is not a parser bug. Every event that does reach claudine is
correctly parsed and rendered. The bug is the **absence** of events for the
stuck subagents combined with the wrapper's reliance on
process-exit-and-stdout-EOF as the sole termination contract.

## Reference Incident

Session `ses_2191c14eeffe…` (run `claudine compose --opencode prompts/commit.md`,
2026-05-02 04:13:13 UTC, OpenCode v1.14.30, `--yolo`, 9 parallel `task`
subagents committing semantic groups of staged files):

| Phase | Time (UTC) | Observed |
|---|---|---|
| Session start | 04:13:13 | parent `ses_2191c14e…` begins |
| Subagent spawn | 04:13:50 → 04:14:12 | 9 × `before_tool task` log entries; 9 × `session_start` for child sessions |
| Activity | 04:14:59 → 04:15:23 | **7** × `after_tool task` entries; 7 commits land on the branch (matched by both timestamp and the subagent titles in the user-visible transcript) |
| Silent gap | 04:15:23 → 04:21:19 | **5 min 56 s of total silence** — no events of any kind in `~/.claudine/logs/2026-05-01.jsonl`, no protect denials, no errors, no progress |
| SIGINT | 04:21:19 | user presses Ctrl+C; claudine synthesises `session_end` with `exit_code: 130`, `provider_status: "tool-calls"`, `duration_ms: 489736` |

The 2 subagents that never emitted `after_tool task` (and never produced
git commits) were the two largest groups:

- `ses_2191b6c4cffe…` — *"Commit tui-chrome-cli src files"*
- `ses_2191b4a04ffe…` — *"Commit feature work files"*

OpenCode's session storage at
`~/.local/share/opencode/storage/session_diff/<sid>.json` was empty (`[]`,
2 bytes) for the parent session — opencode persists session state on clean
shutdown only, and the SIGINT short-circuited that.

## Recurrence

Across recent days of compose+opencode activity, every SIGINT'd session in
the JSONL log carries the same fingerprint (`exit_code: 130`,
`provider_status: "tool-calls"` — meaning the last semantic event before
silence was a tool call still in flight):

| Date | Prompt | Hang duration |
|---|---|---|
| 2026-04-29 18:55 | `prompts/implement-feature-review-suggestions.md` | 7 min 30 s |
| 2026-04-29 19:34 | `prompts/implement-feature-review-suggestions.md` | 75 min |
| 2026-04-29 20:03 | `prompts/implement-phase.md` | 4 s (early kill) |
| 2026-04-30 05:36 | `docs/research/agent-logging/_build.md` | 4 h 13 min |
| 2026-05-02 04:13 | `prompts/commit.md` (this incident) | 8 min 9 s |

By contrast, every cleanly-terminated compose+opencode session in the same
window has `exit_code: 0` and `provider_status: "stop"`. The two outcomes
are mutually exclusive in the logs — there is no "long but successful
tool-calls" status — so `provider_status: "tool-calls"` in a `session_end`
record is a **strong indicator of a hang followed by user interrupt**.

## Architectural Cause

### Termination contract today

In [`exec.rs`][exec], `run_child_stream_semantic` (≈ line 1790) drives
the wrapper run. It:

1. Spawns the child opencode process.
2. Spawns one stdout reader thread that loops over
   `BufRead::lines()` (≈ line 1928), feeding each line through the
   semantic parser.
3. Spawns one stderr reader thread (≈ line 1993) for log-bridge
   intake.
4. Spawns a `flush_if_idle` heartbeat ticker (≈ line 1859, 30 s
   `SILENCE_WINDOW` defined at ≈ line 1358) that flushes pending
   stream-text fragments so a dangling final paragraph reaches the
   user even if opencode never closes stdout cleanly.
5. Waits for the child to exit via `wait_with_signal_handling`
   (≈ line 2077) or `wait_with_signal_and_early_termination`
   (≈ line 2067).
6. **Only after the child exits** does it join the reader threads
   with a 5-second `thread_join_timeout` (≈ line 2095) and call
   `parser.finish(exit_code)` to synthesise the summary.

There is **no semantic terminator event** in the OpenCode protocol —
`opencode.rs` emits no `session.end` / `done` / `result` event. The
existing `step_complete` / `turn_complete` events feed
`SemanticEvent::TurnComplete` (per-turn metadata only); they are not
treated as a stream terminator and would not be reliable for that role
(turns can repeat).

`flush_if_idle` (defined at [`exec.rs:231`][exec-flush] and ticked from
[`exec.rs:1354-1370`][exec-ticker]) only **flushes pending stream-text
buffers** so trailing assistant prose reaches the user. It does **not**
kill the run, signal the child, or otherwise unblock the wait.

### Where the deadlock happens

When opencode hangs holding stdout open:

- `child.wait()` blocks indefinitely.
- The stdout reader thread blocks on `reader.lines()` waiting for
  data or EOF, neither of which arrives.
- The flush ticker continues to tick but, finding no new buffered
  text, has nothing to flush.
- The user sees nothing happen on stderr after the last rendered
  `← Task(successful, ...)` line. They eventually Ctrl+C.

### Subagent state is observable but unused

`SemanticEvent::SubagentStart` and `SemanticEvent::SubagentStop` are
emitted by the OpenCode parser at
[`opencode_semantic.rs:229`][osem-task-start] and
[`:241`][osem-task-stop] for every `task_started` / `task_completed`
wire event. They flow through
[`live_semantic_sink.rs:829`][lss-saw-start] and
[`:836`][lss-saw-stop] for rendering, but **no component tracks the
delta** between starts and stops. There is no `active_subagents` set,
no per-subagent timer, and no metric exposed from `LiveMetrics` for
"how long has subagent X been outstanding". When 9 subagents start
and 7 finish, the wrapper has no way of knowing 2 are still pending
even though the data needed to compute it has already passed through.

## Fixes

Three layered fixes. Each is independently shippable and stops a
distinct failure mode. Implementation may bundle them into one or
multiple plans; this section describes them as separate contracts so
review can size each on its own.

### Fix 1 — Subagent watchdog (highest impact)

The wrapper MUST track active subagents and fail loudly when one
goes silent for too long. This is the single change that prevents
the "indefinite hang on parallel `task` subagents" failure mode
end to end.

#### Required behaviour

- The live sink (or a sibling component fed by the same
  `SemanticEvent` stream) MUST maintain an
  `active_subagents: HashMap<SubagentId, ActiveSubagentInfo>`
  populated by every `SubagentStart` and drained by every
  matching `SubagentStop`. `ActiveSubagentInfo` MUST include at
  minimum: subagent id, optional human-readable name/title,
  `started_at: Instant`, and `last_progress_at: Instant`
  (initialised from `started_at` and updated when any wire event
  references that subagent id).
- A new background ticker — **subagent watchdog**, separate
  from the existing `flush_if_idle` ticker — MUST fire on a
  short interval (default 5 s) and check, for each entry in
  `active_subagents`, whether
  `Instant::now() - last_progress_at` exceeds a configurable
  silence threshold (`CLAUDINE_SUBAGENT_IDLE_KILL_SECONDS`,
  default 180).
- On the first detected silence breach for a session, the
  watchdog MUST:
  1. Render a stderr report enumerating which subagents are
     stuck (id, name/title if known, elapsed silence). The
     report MUST follow the existing
     `SemanticEvent::Error { kind: SemanticErrorKind::AgentNative }`
     rendering contract introduced in the 2026-04-16 *more-is-more*
     fix so the message lands as a coloured `BlockQuote` with the
     `▌ ` border.
  2. Send `SIGTERM` to the child opencode process group (the
     wrapper already isolates the process group when
     `isolate_process_group = true` at
     [`exec.rs:2077`][exec-wait]; the watchdog reuses that group
     id).
  3. After a short grace period (default 10 s, configurable via
     `CLAUDINE_SUBAGENT_KILL_GRACE_SECONDS`), escalate to
     `SIGKILL` if the child has not exited.
  4. Mark the run's exit reason in the synthesised summary as
     `"subagents_unresponsive"` so JSONL reporting and the
     trailer badges can distinguish a watchdog-killed hang from a
     user-initiated SIGINT (`exit_code 130`) or a clean stop.
- The watchdog MUST be disabled when the kill threshold is set
  to `0` (escape hatch for users who genuinely run subagents
  longer than 3 minutes — e.g. long-running tests).
- The watchdog MUST **not** fire while any non-subagent stream
  activity is happening. The `last_progress_at` clock applies
  per subagent; a subagent that emits `task_progress` events
  resets its own clock and remains alive even past the global
  threshold. Only subagents that produce **zero** events for
  `CLAUDINE_SUBAGENT_IDLE_KILL_SECONDS` are eligible.
- The watchdog MUST cooperate with `wait_with_signal_handling`
  / `wait_with_signal_and_early_termination` rather than
  competing with them: when the watchdog fires, the
  termination signal MUST flow through the same pathway the
  signal handler already uses so the subsequent
  `parser.finish(exit_code)` call observes the correct
  exit-code semantics.

#### Required configuration surface

| Knob | Default | Notes |
|---|---|---|
| `CLAUDINE_SUBAGENT_IDLE_KILL_SECONDS` | `180` | per-subagent silence ceiling; `0` disables watchdog |
| `CLAUDINE_SUBAGENT_KILL_GRACE_SECONDS` | `10` | between SIGTERM and SIGKILL |
| `CLAUDINE_SUBAGENT_WATCHDOG_INTERVAL_SECONDS` | `5` | ticker frequency (kept separate so tests can drive it deterministically) |

#### Fix locations

- [`claudine/cli/src/commands/wrap/live_semantic_sink.rs:829`][lss-saw-start]
  / [`:836`][lss-saw-stop] — extend the `SubagentStart` /
  `SubagentStop` arms to mutate a shared `active_subagents`
  state. The state should live on the sink itself or on a
  small `WatchdogState` adjacent struct shared via
  `Arc<Mutex<…>>`.
- [`claudine/cli/src/commands/wrap/exec.rs:1354`][exec-ticker] —
  add `spawn_subagent_watchdog_ticker` next to
  `spawn_flush_if_idle_ticker` (do **not** overload the
  existing ticker; their cadences and side effects are
  different).
- [`claudine/cli/src/commands/wrap/exec.rs:1859`][exec-spawn-flush]
  — wire the new ticker into `run_child_stream_semantic` and
  ensure it is cleanly stopped after `wait_with_signal_handling`
  returns, mirroring the
  `stop_timing_ticker(flush_ticker)` call at
  [`exec.rs:2092`][exec-stop-flush].

#### Required tests

- Unit test on the sink-level `active_subagents` map: feed
  N `SubagentStart` events and M `SubagentStop` events with a
  controlled clock; assert membership and `last_progress_at`
  drift.
- Watchdog timer test driven by an injected fake clock /
  channel: simulate a stream that emits `SubagentStart` and
  then no further events for the threshold window; assert the
  watchdog produces an `AgentNative` error event naming the
  stuck subagent and signals the child.
- End-to-end fixture test backed by a recorded OpenCode
  stream: replay the
  `prompts/commit.md` reference incident's first 7 successful
  `task_completed` events, then feed nothing further; assert
  that after the threshold the run terminates with exit reason
  `subagents_unresponsive` and the rendered stderr names the 2
  stuck subagent ids.

### Fix 2 — Stream silence kill switch (defence in depth)

A coarser, provider-agnostic safety net for the case where the
hang is **not** signalled by an outstanding subagent — for
example, opencode (or any provider) emitting a tool call but
never the matching tool result, or an upstream API stall after
the assistant turn started.

#### Required behaviour

- A second watchdog (or the same ticker, fed by an additional
  rule) MUST monitor stream-level silence using the existing
  `LiveMetrics.last_event_at` field already maintained at
  [`live_semantic_sink.rs:970`][lss-last-event].
- When `Instant::now() - last_event_at` exceeds
  `CLAUDINE_STREAM_IDLE_KILL_SECONDS` (default `300`,
  i.e. 5 minutes) **and** the run has reached at least one
  semantic event past the initial session start (so we do not
  kill cold-start sessions during model warm-up), the watchdog
  MUST follow the same SIGTERM → grace → SIGKILL escalation
  as Fix 1, with a distinct exit reason
  `"stream_idle_timeout"`.
- The kill threshold MUST be set to `0` to disable.
- This rule MUST NOT fire while Fix 1 is already firing — i.e.
  if `active_subagents.is_empty() == false`, Fix 1 owns the
  watchdog responsibility and Fix 2 is suppressed for that
  session.

#### Required configuration surface

| Knob | Default | Notes |
|---|---|---|
| `CLAUDINE_STREAM_IDLE_KILL_SECONDS` | `300` | stream-level silence ceiling; `0` disables |

#### Fix locations

Same files as Fix 1; extend the watchdog ticker to evaluate
both rules in priority order.

#### Required tests

- A fixture-replay test that emits one tool call without a
  matching result and no further events; assert termination
  with exit reason `stream_idle_timeout`.
- A negative test that confirms Fix 2 stays quiet when Fix 1
  is in scope (subagents outstanding) so the two rules do not
  double-fire.

### Fix 3 — Diagnostic on idle (zero behaviour-risk fallback)

Even if Fix 1 and Fix 2 ship later, the stderr surface MUST
make it possible for a user to diagnose **why** a session is
stuck before they Ctrl+C. Today the trailing rendered line is
a successful tool result, with no indication that anything is
still in flight.

#### Required behaviour

- When the existing `flush_if_idle` heartbeat at
  [`exec.rs:1354`][exec-ticker] fires and observes that
  `active_subagents` is non-empty (Fix 1 must land first or
  the same data must be plumbed for this fix), the heartbeat
  MUST render a single status line per active subagent in the
  Tool Use & Events section, of the form:

  ```text
   ⏳ Awaiting subagent: <name-or-id> (<elapsed-since-start>)
  ```

  rendered at most once per subagent per silence window so the
  same line does not repeat every 30 s.
- The line MUST flow through `SectionTracker` so it follows
  the existing "at most one blank line between adjacent
  rendered lines" invariant.
- Suppression MUST gate on the same disable knob as Fix 1
  (`CLAUDINE_SUBAGENT_IDLE_KILL_SECONDS=0` disables the
  diagnostic too) so users who explicitly opt out of the
  watchdog do not get unsolicited noise.

#### Fix locations

- [`claudine/cli/src/commands/wrap/exec.rs:1370`][exec-flush-call]
  — extend the `flush_if_idle` callback to consult the shared
  `active_subagents` snapshot and emit diagnostic lines. The
  `LiveSemanticSink` should expose a read-only accessor that
  returns a `Vec<DiagnosticLine>` so the ticker thread does
  not need to hold the live sink mutex while writing.

#### Required tests

- A fixture test that runs the watchdog in
  `diagnostic-only` mode (kill disabled, idle threshold low)
  and asserts the rendered stderr contains an
  `⏳ Awaiting subagent:` line for each outstanding subagent.

## Non-Goals

- This document does **not** propose patching OpenCode itself.
  The upstream cause — that opencode v1.14.30 fails to emit
  `task_completed`/`task_error` for some subagents under
  parallel `task` execution with `--yolo` — should be filed
  upstream against `sst/opencode` separately. This spec is
  scoped to making claudine resilient regardless of upstream
  behaviour.
- This document does **not** propose changing the existing
  `flush_if_idle` semantics, the section-tracker contract,
  the `ToolCallDisplay` format, or any other rendering
  surface beyond adding the new diagnostic line in Fix 3.
- This document does **not** propose adding a generic
  per-tool-call timeout. Tool calls can legitimately run for
  hours (long shells, large file writes, slow network). Only
  subagents (Fix 1) and full stream silence (Fix 2) get a
  bounded ceiling, and both default to thresholds well above
  any reasonable single-tool runtime.
- The same hang class may exist for Goose, Kimi, and Qwen.
  Cross-provider audit is **not** covered here; the
  watchdog wires up at the provider-agnostic
  `SemanticEvent::SubagentStart` / `SubagentStop` boundary
  and will work for any provider whose parser emits those
  events. Providers that do not emit subagent events get
  Fix 2 coverage by default.

## Acceptance Output

After Fix 1 lands, replaying the reference incident
(`prompts/commit.md` with 9 parallel `task` subagents, 2 of
which go silent inside opencode) MUST produce the following
trailing stderr surface within ~3 minutes of the last
successful `task_completed`:

```text
 ← Task(successful, Commit tui-chrome lib components)

▌ Agent Error
▌ 2 subagents went silent and were terminated by the
▌ watchdog after 180 s of inactivity:
▌
▌   • ses_2191b6c4… "Commit tui-chrome-cli src files"
▌     idle since 04:14:??Z (3m 0s)
▌   • ses_2191b4a04… "Commit feature work files"
▌     idle since 04:14:??Z (3m 0s)
▌
▌ The wrapped opencode process was terminated. The 7
▌ subagents that completed before the silence have
▌ already committed their groups. Re-run the prompt to
▌ retry the missing groups, or commit those files
▌ manually.

✗ 3m 12s · ... · exit reason: subagents_unresponsive
```

Specifically:

- Termination happens within ~`CLAUDINE_SUBAGENT_IDLE_KILL_SECONDS`
  of the last subagent activity (default 180 s) plus the
  watchdog interval (default 5 s) plus the SIGTERM grace
  (default 10 s).
- The exit reason in the synthesised `session_end` log entry
  is `subagents_unresponsive`, distinct from `130` /
  user-initiated SIGINT.
- The 7 successfully-completed subagents' commits are already
  on the branch — the watchdog terminates the **wrapper child**,
  not the subagents that already committed.
- No spurious watchdog firings on the legitimate
  long-running compose+opencode sessions in the reference
  table above (those have continuous stream activity and
  would never accumulate 180 s of silence with subagents
  outstanding).

[exec]: ../cli/src/commands/wrap/exec.rs
[exec-flush]: ../cli/src/commands/wrap/exec.rs
[exec-ticker]: ../cli/src/commands/wrap/exec.rs
[exec-spawn-flush]: ../cli/src/commands/wrap/exec.rs
[exec-stop-flush]: ../cli/src/commands/wrap/exec.rs
[exec-wait]: ../cli/src/commands/wrap/exec.rs
[exec-flush-call]: ../cli/src/commands/wrap/exec.rs
[osem-task-start]: ../lib/src/stream/opencode_semantic.rs
[osem-task-stop]: ../lib/src/stream/opencode_semantic.rs
[lss-saw-start]: ../cli/src/commands/wrap/live_semantic_sink.rs
[lss-saw-stop]: ../cli/src/commands/wrap/live_semantic_sink.rs
[lss-last-event]: ../cli/src/commands/wrap/live_semantic_sink.rs
