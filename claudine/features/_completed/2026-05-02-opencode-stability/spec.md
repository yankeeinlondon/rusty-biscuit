# OpenCode Stability — Specification

This specification describes a class of hangs observed when wrapping OpenCode
under `claudine compose` / `inline-compose` / `sequence`, and the unified
timeout contract that a fix must satisfy. The upstream cause sits inside
OpenCode, but claudine has no defenses today and silently deadlocks the user.
This document is the input to a follow-up implementation plan; it does not
itself change code.

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

In [`exec.rs`][exec], `run_child_stream_semantic` (≈ line 1790) drives the
wrapper run. It:

1. Spawns the child opencode process.
2. Spawns one stdout reader thread that loops over `BufRead::lines()`
   (≈ line 1928), feeding each line through the semantic parser.
3. Spawns one stderr reader thread (≈ line 1993) for log-bridge intake.
4. Spawns a `flush_if_idle` heartbeat ticker (≈ line 1859, 30 s
   `SILENCE_WINDOW` defined at ≈ line 1358) that flushes pending
   stream-text fragments so a dangling final paragraph reaches the user
   even if opencode never closes stdout cleanly.
5. Waits for the child to exit via `wait_with_signal_handling`
   (≈ line 2077) or `wait_with_signal_and_early_termination`
   (≈ line 2067).
6. **Only after the child exits** does it join the reader threads with a
   5-second `thread_join_timeout` (≈ line 2095) and call
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
- The stdout reader thread blocks on `reader.lines()` waiting for data or
  EOF, neither of which arrives.
- The flush ticker continues to tick but, finding no new buffered text,
  has nothing to flush.
- The user sees nothing happen on stderr after the last rendered
  `← Task(successful, ...)` line. They eventually Ctrl+C.

### Subagent state is observable but unused

`SemanticEvent::SubagentStart` and `SemanticEvent::SubagentStop` are
emitted by the OpenCode parser at
[`opencode_semantic.rs:229`][osem-task-start] and
[`:241`][osem-task-stop] for every `task_started` / `task_completed` wire
event. They flow through [`live_semantic_sink.rs:829`][lss-saw-start] and
[`:836`][lss-saw-stop] for rendering, but **no component tracks the
delta** between starts and stops. There is no `active_subagents` set, no
metric exposed from `LiveMetrics` for "which subagents are still
outstanding," and no error report when the wrapper terminates with
subagents still in flight.

## Fix — Unified Timeout Enforcement

Two timeouts, one termination path. The same vocabulary is used in
markdown frontmatter, the CLI, env-var defaults, and the watchdog: there
is no separate "watchdog idle threshold" parallel to `step_timeout`.

### The two timeouts

| Name | What it measures | What resets it |
|---|---|---|
| **`timeout`** | Wall-clock budget for the prompt — total elapsed time from child-process spawn | nothing (monotonic) |
| **`step_timeout`** | Stream-silence budget — time since the last event arrived on the parent stream | **any** stream event: tool call, tool result, message delta, subagent start/stop, subagent progress, status change |

These are the **only** two timeout types in the system. There is no
separate per-subagent kill threshold and no separate stream-idle kill
threshold. Subagent progress is a stream event; if subagents are
reporting progress, `step_timeout` does not fire. If the parent stream
is silent for `step_timeout` regardless of which agent (parent or
subagent) was supposed to be working, the run is killed.

#### `timeout` — wall-clock budget

```text
Starts when the child process is spawned. Never resets.
When elapsed >= timeout, send SIGTERM; after kill_grace, send SIGKILL.

Example:
  timeout: 2h
  → child spawned at 14:00:00
  → SIGTERM sent at 16:00:00 regardless of activity
  → SIGKILL sent at 16:00:10 if the child has not exited
```

#### `step_timeout` — stream-silence budget

```text
Tracks `last_event_at` — the most recent event on the parent stream.
Tool calls, tool results, message deltas, subagent start/stop, subagent
progress, and status changes all count and reset the clock.

When (now - last_event_at) >= step_timeout, send SIGTERM; after
kill_grace, send SIGKILL.

Example:
  step_timeout: 30m
  → last event observed at 14:00:00
  → SIGTERM sent at 14:30:00 unless something hits the stream first
  → SIGKILL sent at 14:30:10 if the child has not exited
```

### Configuration sources (priority order)

For each of the two timeouts, the resolved value is the first present
of:

1. **CLI flag** — `--timeout DURATION`, `--step-timeout DURATION`
2. **Markdown frontmatter** — `timeout: 30m`, `step_timeout: 5m` parsed
   into `HarnessPlan.timeout` and `HarnessPlan.step_timeout`
   ([`harness/model.rs:16,23`][harness-model])
3. **Env-var default** — `CLAUDINE_TIMEOUT`, `CLAUDINE_STEP_TIMEOUT`
4. **Built-in default** — `timeout` has no default (no wall-clock kill
   unless opted in); `step_timeout` defaults to `30m`

All values use the same grammar as
[`parse_timeout`][harness-parse_timeout] (`30s`, `5m`, `2h`,
`30 seconds`, `5 minutes`, `2 hours`). Bare seconds are not accepted.

The watchdog enforces whatever resolves at this stage; it has no
private threshold.

### Configuration knobs

| Env var | Default | What |
|---|---|---|
| `CLAUDINE_TIMEOUT` | none | wall-clock default when no CLI flag and no frontmatter; duration string |
| `CLAUDINE_STEP_TIMEOUT` | `30m` | stream-silence default when no CLI flag and no frontmatter; duration string |
| `CLAUDINE_KILL_GRACE` | `10s` | between SIGTERM and SIGKILL |
| `CLAUDINE_WATCHDOG_INTERVAL` | `5s` | internal ticker cadence |

Setting `CLAUDINE_TIMEOUT` or `CLAUDINE_STEP_TIMEOUT` to `0s` disables
the corresponding rule. Frontmatter and CLI flags can also disable a
rule by omitting it, but cannot set `0s` (parser rejects zero per
[`parse_timeout`][harness-parse_timeout]) — disable from frontmatter by
omission, or from env by exporting `=0s`.

### Required behaviour

- A single watchdog ticker MUST evaluate both rules on
  `CLAUDINE_WATCHDOG_INTERVAL` cadence. The wall-clock rule fires when
  `now - started_at >= timeout`; the stream-silence rule fires when
  `now - last_event_at >= step_timeout`.
- The watchdog MUST cooperate with `wait_with_signal_handling` /
  `wait_with_signal_and_early_termination` rather than competing with
  them: when the watchdog fires, the termination signal MUST flow
  through the same pathway the signal handler already uses so the
  subsequent `parser.finish(exit_code)` call observes the correct
  exit-code semantics.
- Both rules MUST send SIGTERM to the child opencode process group
  (the wrapper already isolates the process group when
  `isolate_process_group = true` at [`exec.rs:2077`][exec-wait]).
  After `CLAUDINE_KILL_GRACE`, MUST escalate to SIGKILL if the child
  has not exited.
- The synthesised `session_end` summary MUST mark the exit reason as
  `"timeout"` (wall-clock breach) or `"step_timeout"` (silence
  breach), distinct from `130` / user-initiated SIGINT and from
  cleanly-completed runs.
- A one-shot `fired` guard MUST prevent double-fire across the two
  rules.
- The stream-silence rule MUST require at least one observed activity
  event past the initial session start so cold-start sessions during
  model warm-up do not trip the kill (matches today's
  `last_event_at: Option<Instant>` semantics).

### Subagent diagnostics (in error reports only)

When `step_timeout` fires and one or more subagents were outstanding
at the moment of breach, the rendered error MUST enumerate them so the
user knows which workers stalled. The watchdog continues to maintain
an `active_subagents` map populated by `SubagentStart` and drained by
`SubagentStop`, but **only for diagnostic enrichment** — there is no
per-subagent kill threshold and no separate exit reason for "subagents
unresponsive." All silence kills are `step_timeout`.

This preserves the operator-friendly stderr the previous design aimed
for ("which 2 subagents stalled?") without splitting the timeout
vocabulary.

### Fix locations

- [`claudine/cli/src/commands/wrap/subagent_watchdog.rs`][watchdog-mod]
  — rename file or contents to reflect unified timeout enforcement
  (working name: `timeout_watchdog`); replace the two-threshold
  `WatchdogConfig` with the new `TimeoutConfig` (`timeout: Option<Duration>`,
  `step_timeout: Option<Duration>`, `kill_grace: Duration`, `interval: Duration`).
- [`claudine/cli/src/commands/wrap/live_semantic_sink.rs:829`][lss-saw-start]
  / [`:836`][lss-saw-stop] — extend the `SubagentStart` /
  `SubagentStop` arms to mutate the shared `active_subagents` state
  used for diagnostics.
- [`claudine/cli/src/commands/wrap/exec.rs:1354`][exec-ticker] — add
  `spawn_timeout_watchdog_ticker` (renamed from
  `spawn_subagent_watchdog_ticker`) next to `spawn_flush_if_idle_ticker`;
  keep cadences and side effects separate.
- [`claudine/cli/src/commands/wrap/composition.rs`][composition-mod] —
  resolve `TimeoutConfig` from CLI flag → `HarnessPlan.timeout` /
  `HarnessPlan.step_timeout` → env-var defaults → built-in defaults
  before launching the wrapper.

### Required tests

- Unit test on `TimeoutConfig::resolve` covering each precedence rung
  (CLI > frontmatter > env > built-in default), valid duration
  parsing, and the `0s` env-var disable case.
- Unit test on the sink-level `active_subagents` map: feed N
  `SubagentStart` events and M `SubagentStop` events with a
  controlled clock; assert membership is correct at each step.
- Watchdog timer test driven by an injected fake clock / channel:
  - simulate a stream that emits some events and then no further
    events for `step_timeout`; assert the watchdog produces an error
    naming any outstanding subagents and signals the child;
  - simulate a stream that runs past `timeout` while still emitting
    events; assert the watchdog fires `timeout` and not
    `step_timeout`;
  - simulate `0s` disable for each rule and assert no firing.
- End-to-end fixture test backed by a recorded OpenCode stream:
  replay the `prompts/commit.md` reference incident's first 7
  successful `task_completed` events, then feed nothing further;
  assert that after `step_timeout` the run terminates with exit
  reason `step_timeout` and the rendered stderr names the 2 stuck
  subagent ids.
- Diagnostic on idle: a fixture that runs with a low `step_timeout`
  threshold and asserts the `flush_if_idle` heartbeat emits an
  `⏳ Awaiting subagent: <name-or-id> (<elapsed>)` line per
  outstanding subagent before the kill fires.

## Non-Goals

- This document does **not** propose patching OpenCode itself. The
  upstream cause — that opencode v1.14.30 fails to emit
  `task_completed`/`task_error` for some subagents under parallel
  `task` execution with `--yolo` — should be filed upstream against
  `sst/opencode` separately. This spec is scoped to making claudine
  resilient regardless of upstream behaviour.
- This document does **not** propose changing the existing
  `flush_if_idle` semantics, the section-tracker contract, the
  `ToolCallDisplay` format, or any other rendering surface beyond
  adding the diagnostic line for outstanding subagents and the
  watchdog termination error block.
- This document does **not** propose adding a generic per-tool-call
  timeout. Tool calls can legitimately run for hours (long shells,
  large file writes, slow network). Only the wall-clock and
  stream-silence ceilings apply, and the default `step_timeout`
  (`30m`) sits well above any reasonable single-tool runtime.
- The same hang class may exist for Goose, Kimi, and Qwen.
  Cross-provider audit is **not** covered here; the watchdog wires up
  at the provider-agnostic `SemanticEvent` boundary and uses the
  parent stream's `last_event_at`, so it works for any provider.

## Acceptance Output

After this fix lands, replaying the reference incident
(`prompts/commit.md` with 9 parallel `task` subagents, 2 of which go
silent inside opencode) MUST produce the following trailing stderr
surface within ~`step_timeout` + `CLAUDINE_WATCHDOG_INTERVAL` +
`CLAUDINE_KILL_GRACE` of the last successful `task_completed`:

```text
 ← Task(successful, Commit tui-chrome lib components)

 ⏳ Awaiting subagent: Commit tui-chrome-cli src files (15m 12s)
 ⏳ Awaiting subagent: Commit feature work files (15m 12s)

▌ Step Timeout
▌ No stream activity for 30m. The wrapped opencode process was
▌ terminated. 2 subagents were still outstanding when the timeout
▌ fired:
▌
▌   • ses_2191b6c4… "Commit tui-chrome-cli src files"
▌     idle since 04:14:??Z (30m 0s)
▌   • ses_2191b4a04… "Commit feature work files"
▌     idle since 04:14:??Z (30m 0s)
▌
▌ The 7 subagents that completed before the silence have already
▌ committed their groups. Re-run the prompt to retry the missing
▌ groups, or commit those files manually.

✗ 30m 15s · ... · exit reason: step_timeout
```

Specifically:

- Termination happens within ~`step_timeout` of the last activity
  event, plus the watchdog interval, plus the SIGTERM grace.
- The exit reason in the synthesised `session_end` log entry is
  `step_timeout`, distinct from `130` / user-initiated SIGINT and
  from `timeout` (wall-clock breach).
- The 7 successfully-completed subagents' commits are already on the
  branch — the watchdog terminates the **wrapper child**, not the
  subagents that already committed.
- No spurious watchdog firings on the legitimate long-running
  compose+opencode sessions in the reference table above (those have
  continuous stream activity and would never accumulate `step_timeout`
  of silence).

[exec]: ../cli/src/commands/wrap/exec.rs
[exec-flush]: ../cli/src/commands/wrap/exec.rs
[exec-ticker]: ../cli/src/commands/wrap/exec.rs
[exec-wait]: ../cli/src/commands/wrap/exec.rs
[osem-task-start]: ../lib/src/stream/opencode_semantic.rs
[osem-task-stop]: ../lib/src/stream/opencode_semantic.rs
[lss-saw-start]: ../cli/src/commands/wrap/live_semantic_sink.rs
[lss-saw-stop]: ../cli/src/commands/wrap/live_semantic_sink.rs
[harness-model]: ../lib/src/harness/model.rs
[harness-parse_timeout]: ../lib/src/harness/timeout.rs
[watchdog-mod]: ../cli/src/commands/wrap/subagent_watchdog.rs
[composition-mod]: ../cli/src/commands/wrap/composition.rs
