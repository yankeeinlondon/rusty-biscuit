---
status: draft
created: 2026-06-22
area: claudine
packages:
    - claudine
---

# Live-but-Dead Guard: Stalled-Generation Detection

> **DRAFT.** This spec captures a real failure investigation and proposes a
> guard. The design section lists options and a recommendation; it is not yet
> a committed implementation plan. Open questions are tracked at the end.

## Problem

A wrapped OpenCode run with `zai-coding-plan/glm-5.2` hung indefinitely. The
parent session sat on `⏳ Awaiting subagent` for 40+ minutes while the wrapper
output streamed a steady beat of `llm_call_start` badges and never terminated:

```text
 llm_call_start zai-coding-plan/glm-5.2 (mode=all, agent=rust-developer)
 step_loop step=70 session=ses_10ea40010ffeUlahGfHA4R7Mmv
 llm_call_start zai-coding-plan/glm-5.2 (mode=all, agent=rust-developer)
 llm_call_start zai-coding-plan/glm-5.2 (mode=all, agent=rust-developer)
 llm_call_start zai-coding-plan/glm-5.2 (mode=all, agent=rust-developer)
 ⏳ Awaiting subagent: "Implement Finding 2 ... (@rust-developer subagent)" (22m 7s)
```

### What actually happened (from the OpenCode store)

- Parent session `ses_10eb6a3c…` (agent `build`) spawned subagent
  `ses_10ea4001…` (agent `rust-developer`) via the `task` tool at
  22:03:18 UTC.
- The subagent ran **70 healthy steps** (41 reads, 14 greps, 10 bash, 8 edits)
  iterating on an `E0505` borrow-checker error in `loop_engine.rs`. Step 70
  finished at **22:09:43 UTC**.
- At 22:09:43.959 the subagent's next assistant message
  (`msg_ef161e017…`) was created — but it has **no `time.completed`, zero
  tokens, no finish reason, and no error**. The generation was launched and
  **never returned a response**.
- **Zero database parts were written in either session after 22:09:43.** Total
  forward-progress stall.
- The parent's `task` tool part is frozen at `state.status = "running"`. Because
  the subagent never reports back, the parent waits forever.

The repeated `llm_call_start` lines are OpenCode **retrying** the dropped
generation. The provider returned neither a response nor an error — the worst
failure mode: a silent drop.

### How this differs from the usage-cap hang

This is **not** the case fixed in
[`2026-06-21-opencode-log-fix`](../../fixes/2026-06-21-opencode-log-fix/spec.md).
That fix added the `RepeatedStreamError` backstop, which counts consecutive
`message="stream error"` records and aborts after 5. Here there are **zero
`stream error` records** — the generation hangs/drops without ever emitting an
error envelope. The only on-wire signal is repeated `llm_call_start` with no
completion. The existing backstop's counter never increments, so it cannot fire.

## Why every existing guard misses it

| Guard | Why it does not fire |
|---|---|
| `timeout` (wall-clock) | Opt-in; not set for this run. Would have worked, but it is a blunt instrument — it also kills legitimately long sessions. |
| `step_timeout` (stream-silence) | Defeated on **two** clocks. (1) The `llm_call_start` / `step_loop` stderr lines classify as `SemanticEvent::Info`, which `is_activity()` counts → `last_event_at` keeps advancing. (2) Those same lines are real bytes on stderr → `last_byte_at` keeps advancing. The ticker uses `max(last_event_at, last_byte_at)`, so silence never accrues. |
| `step_timeout` stuck-aware suppression | The in-flight subagent's `last_progress_at` is refreshed by the same heartbeat `Info` events, so it is classified **active**, never **stuck**. |
| `RepeatedStreamError` backstop | Counts `message="stream error"` records. This stall emits **none** — the generation drops silently. |
| `runaway` content guards | Trip on *volume* (flood) or *repetition* of `OutputText`/`Reasoning`. A stalled generation produces **no** output text at all — the opposite of a flood. |

**The crux:** the raw-byte heartbeat (introduced specifically to protect
OpenCode's long silent synthesis windows from false kills) is also what masks a
genuinely-dead-but-noisy loop. Any guard based on *liveness* (events arriving,
bytes flowing) is defeated, because the process is provably alive — it is just
making no forward progress. The guard must measure **progress**, not liveness.

## The discriminating signal

The fingerprint of this stall is precise and — importantly — distinguishable
from a legitimately long operation:

- A **stalled generation** emits repeated `llm_call_start` (each a fresh LLM
  generation attempt) with **no resulting progress** between them.
- A **legitimately long tool call** (e.g. a 20-minute test suite via `bash`)
  emits **no** `llm_call_start` at all while the tool runs — there is no model
  generation in flight. It is silent on the generation channel.

So "many `llm_call_start` with zero intervening progress" is unique to the
stall and will not false-trip on long tools. This is the signal the guard keys
on.

### Progress vs. liveness taxonomy

Split the current `is_activity()` set into two classes:

**Progress-class** (genuine forward motion — advances a new `last_progress_at`
clock):

- `OutputText`, `Reasoning` — new model output
- `ToolCall`, `ToolResult` — tool lifecycle
- `SubagentStart`, `SubagentStop`
- `FileChange`, `PlanUpdate`
- `Info` `exiting_loop` (`StepExit`)
- `Info` `step_loop` **only when the step number advances** (the bridge
  already dedupes on `(session_id, step)`, so a true transition is detectable)

**Liveness-only** (process is alive but not progressing — does **not** advance
`last_progress_at`):

- `Info` `llm_call_start` (repeated for the same `(session, step)`)
- `Info` `http_response`
- `Info` `permission_evaluated`
- raw stderr/stdout bytes (`last_byte_at`)
- envelopes already excluded today: `SessionStart`, `TurnStart`,
  `TurnComplete`, `PermissionRequest`

`step_timeout` keeps using the existing liveness clocks unchanged (no
regression in its false-kill protection). The new guard reads only
`last_progress_at`.

## Proposed guard

A new **stalled-generation backstop** that trips when the run keeps attempting
generations without ever progressing.

Trip condition (both must hold, to defend against false positives):

1. **Retry churn** — `llm_call_start` count since the last progress-class event
   `>= N` (proposed `N = 4`). A healthy step emits one `llm_call_start` then
   makes progress, resetting the count to 0; only retries accumulate it.
2. **Progress silence** — `now - last_progress_at >= stall_timeout` (proposed
   default `10m`). Shorter than `step_timeout`'s 30m because the progress
   signal is stricter and the retry-churn condition already filters the
   ambiguous cases.

Both conditions reset the moment any progress-class event arrives.

On trip: emit a terminal `SemanticEvent::Error` ("provider attempted N
generations over M without progress; aborting") and fire a new
`EarlyTermination::StalledGeneration` → `ProcessTermination::Aborted` →
fail-fast `AgentFailure`. **Never** the `handle_timeout:` retry path — retrying
would reproduce the stall. This mirrors the `RepeatedStreamError` backstop's
termination shape exactly.

`error_kind = "stalled_generation"`, threaded into the `handle` payload
(`CLAUDINE_ERROR_KIND` env + JSON) like the other guards, with a
`guard_context` carrying the generation count and stall duration.

### Design options considered

**Option A — General progress-silence clock (a third timeout rule).**
Add `last_progress_at` at the stream layer and a `progress_timeout` rule
parallel to `step_timeout`, fired purely on progress silence.
*Rejected as the primary mechanism:* for OpenCode's DONE-only stream, a single
legitimately long tool call produces no progress events for its whole duration
and would false-trip. The retry-churn condition (Option B) is what makes the
signal safe; a pure progress clock lacks it.

**Option B — Stalled-generation backstop (recommended).**
The two-condition trip above. Precise (keys on the unique retry-churn
fingerprint), proven pattern (sits beside `RepeatedStreamError` in the OpenCode
log bridge), and immune to the long-tool false positive. Lives in
[`stream/logs/opencode/reasoning.rs`](../../lib/src/stream/logs/opencode/reasoning.rs)
initially because `llm_call_start` is an OpenCode-stderr-specific signal.

**Option C — Reclassify heartbeat `Info` as non-activity for `step_timeout`.**
Stop `llm_call_start` / `http_response` from advancing `last_event_at`.
*Insufficient alone:* the raw-byte clock still advances from the same stderr
lines, so `step_timeout` still never fires. Would require also gating the byte
heartbeat, which reintroduces the false-kill risk the byte clock was added to
prevent.

**Recommendation:** Option B, scoped to the OpenCode bridge first. If a second
provider later exhibits generation-retry churn, lift the progress/liveness
taxonomy and the backstop to the provider-general stream layer as a follow-up.

### Configuration

| Source | Knob | Default |
|---|---|---|
| CLI flag | `--stall-timeout <dur>` | — |
| Frontmatter | `stall_timeout: <dur>` | — |
| Env default | `CLAUDINE_STALL_TIMEOUT` | `10m` |
| Built-in | — | `10m` |

- Same duration grammar and strict top-down precedence as `timeout` /
  `step_timeout`. `0s` disables the guard.
- Generation-count threshold `N` ships as a constant
  (`MAX_GENERATIONS_WITHOUT_PROGRESS`, proposed `4`), mirroring
  `MAX_CONSECUTIVE_STREAM_ERRORS`. Promote to a knob only if real runs need it.

## Out of scope

- Recovering or resuming the dropped generation. The guard aborts cleanly;
  resume is the provider's job.
- Changing `step_timeout`'s clocks or the byte heartbeat. Their false-kill
  protection is preserved unchanged.
- Provider-general generalization (Option B lifted to the stream layer) — a
  follow-up once a second provider needs it.
- The usage-cap variant already covered by `RepeatedStreamError`.

## Open questions

1. **Default `stall_timeout` value.** `10m` balances catching real stalls
   against tolerating slow-but-genuine retry recovery (a provider that 429s a
   few times then succeeds). Is 10m too aggressive for known-flaky endpoints?
2. **Threshold `N` vs. time-only.** Is the count condition needed at all, or
   does "progress silence ≥ stall_timeout while `llm_call_start` fired at least
   once in the window" suffice? The count guards against a single legitimately
   slow first generation; confirm whether that case can exceed 10m.
3. **`permission_evaluated` classification.** Treated as liveness-only here. If
   a run legitimately blocks on a long human permission decision, does it need
   to keep the guard asleep? (It would, because no `llm_call_start` churn occurs
   while truly blocked — but confirm against the permission-request envelope.)
4. **Reset semantics across steps.** Confirm the bridge's `(session_id, step)`
   dedup state is the right hook for "a genuine step advance" and that it is
   cleared correctly on `StepExit` so a follow-up prompt starts the counter
   fresh.
5. **Interaction with `--silent` and the renderer.** Should the abort badge
   distinguish "stalled generation" from a usage-cap abort in the rendered
   error block, or share the `RepeatedStreamError` presentation?

## References

- Investigation session: `ses_10eb6a3cdffeRTxoa6t7bN8zZc` (parent),
  `ses_10ea40010ffeUlahGfHA4R7Mmv` (subagent), OpenCode store
  `~/.local/share/opencode/opencode.db`.
- [Timeouts](../../docs/topics/timeouts.md) — the two timeout rules, activity
  vocabulary, stuck-aware suppression.
- [OpenCode Event Sources](../../../.claude/skills/claudine/opencode-event-sources.md)
  — Dual-Source Contract, stderr promotion table, `RepeatedStreamError`
  backstop.
- [`2026-06-21-opencode-log-fix`](../../fixes/2026-06-21-opencode-log-fix/spec.md)
  — the usage-cap variant of this hang.
