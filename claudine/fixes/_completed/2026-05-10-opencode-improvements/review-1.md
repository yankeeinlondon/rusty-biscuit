---
review_of: plan.md
date: 2026-05-10
reviewer: claude-opus-4-7
verdict: APPROVE with minor follow-ups
---

# Review 1 — OpenCode Subagent Visibility & Per-Step Grace

## Summary

The implementation matches the plan closely. All 6 phases landed, all 55
watchdog-related unit tests and 152 OpenCode tests pass, the workspace
builds clean, and behavior at the critical boundary points (per-step grace
on / off, byte-heartbeat backstop, synthesized lifecycle, ring buffer cap)
is covered by tests. There are no functional gaps that I would block on,
but there are several small correctness and quality issues worth fixing
before this gets a follow-up plan.

## Phase-by-phase findings

### Phase 1 — Synthesize SubagentStart/SubagentStop for `task` tool

Location: `claudine/lib/src/stream/providers/opencode.rs:321-456`.

**Correct:**

- `is_task` detection is case-insensitive (line 350) ✓
- Display name preference order matches the plan
  (`description → subagent_type → prompt → name`) at lines 358-374 ✓
- Subagent id falls back from `metadata.sessionId` to the tool's own id
  at lines 376-384 ✓
- `extra.synthesized = true` marker is set on both start and stop
  (lines 411, 444) ✓
- `extra.description` is propagated through both events so the live
  sink can pick it up for the ring buffer ✓
- `ToolResult` is preserved in addition to the synthesized events —
  the rendered `→ Task(...)` / `← Task(...)` lines are unchanged ✓
- A `tracing::debug!` log fires when synthesis happens (lines 419-424),
  matching the plan's "Risks" mitigation ✓
- Error completions still trigger synthesis (status propagates through
  `resolved.status`), answering the plan's open question affirmatively ✓

**Issue 1 (minor, code quality): `_started_at` is dead code.**

Lines 387-408 compute a complex `Instant` from
`state.time.start` epoch ms with a delta-from-now offset, bind it to
`_started_at`, and then never use it.
`SemanticEvent::SubagentStart` does not accept a `started_at`; the
downstream `LiveMetricsState::observe_event` uses the `now` passed
to it. The only piece of `time.start` that *is* used is the raw
`epoch_ms`, which is inserted into `start_extra` at line 416. The
22-line `Instant` reconstruction block should be deleted entirely —
it adds nothing today, and if a future caller wants the wall-clock
start they can read `extra.started_at_epoch_ms` and reconstruct
once.

**Issue 2 (minor, but real): the `subagent_type` value never lands in
`extra` from the synthesizer.**

The synthesizer picks `description → subagent_type → prompt → name`
into a single `description` field on `extra` (lines 412-414, 445-447).
Meanwhile the live sink at
`claudine/cli/src/commands/wrap/live_semantic_sink/mod.rs:686-695`
also looks for `extra.subagent_type` as a fallback. That fallback is
dead today on the synthesized path because the synthesizer collapses
both into `extra.description`. Not a bug, but the comment/structure
implies two distinct sources. Either:

- have the synthesizer emit `description` and `subagent_type` as
  separate fields when both are present, OR
- drop the `subagent_type` lookup from the live sink (it's only
  reachable via native `task_completed` events that are not currently
  emitted by OpenCode).

### Phase 2 — Surface `task` tool metadata in the protocol layer

Location: `claudine/lib/src/stream/protocol/opencode.rs:305-444`.

**Correct:**

- `OpenCodeToolMetadata` with optional `sessionId` (camelCase rename) ✓
- `OpenCodeToolTime` with optional `start` (epoch ms) ✓
- Both fields plumbed through `OpenCodeToolFields`, `OpenCodeToolPart`,
  `ResolvedOpenCodeTool`, and `OpenCodeTool::resolve` ✓
- Top-level > `part.fields` > `part.state` priority preserved
  (tests `opencode_tool_task_accessors_from_part_fields` at 831 and
  the fallback test at 847) ✓
- `task_subagent_id()` and `task_started_at_epoch_ms()` accessor
  methods on `ResolvedOpenCodeTool` ✓
- All fields remain `Option<>` with `#[serde(default)]`; no
  `deny_unknown_fields` introduced — format drift remains harmless ✓
- New unit tests cover both presence and absence of each field ✓

No issues in Phase 2.

### Phase 3 — Per-step `provider_status` grace

Location: `claudine/lib/src/stream/progress.rs:92, 278-293` plus
`claudine/cli/src/commands/wrap/exec/watchdog.rs:469-538`.

**Correct:**

- `step_in_flight: bool` added to `LiveMetricsState` (line 92) ✓
- `observe_event` toggles it on `step_phase=start` (line 281) and clears
  it on `step_phase=finish` (line 286) ✓
- Unit tests cover both transitions and the no-reason fallback
  (`info_step_start_sets_step_in_flight_and_does_not_set_provider_status`,
  `info_step_finish_records_provider_status_from_reason_and_clears_step_in_flight`,
  `info_step_finish_without_reason_falls_back_to_finish`) ✓
- The watchdog guard is a composite predicate at watchdog.rs:534-538:
  `OpenCode AND ((step_in_flight && !both_clocks_stale) || !provider_status_seen)`
  — which preserves the original cold-start grace while adding the
  per-step grace ✓
- Wall-clock rule remains unaffected (verified by
  `evaluate_timeout_tick_opencode_grace_does_not_block_wall_clock`) ✓
- Other providers (Claude, `None`) get no grace
  (`evaluate_timeout_tick_grace_does_not_apply_to_other_providers`,
  `evaluate_timeout_tick_grace_does_not_apply_when_provider_unset`) ✓

**Observation (not an issue, but worth documenting): the
both-clocks-stale safety only protects the *per-step* path, not the
cold-start path.** The composite expression groups the byte-clock
check only with `step_in_flight`. If a session never sees any
`step_finish` and the byte clock goes stale beyond budget, the rule
remains suppressed forever (until the wall-clock backstop fires).
This is intentional per the inline comment at watchdog.rs:519-526
("provider_status is None — no `step_finish` has ever been observed,
protecting slow startup / slow first turns") and matches the prior
regression-fix semantics, but the `both_clocks_stale` test in
`opencode_byte_heartbeat_still_catches_zero_byte_hang` only exercises
the `step_in_flight=true, provider_status=Some` arm. **Recommendation:
add a sibling test** that asserts the cold-start path correctly
suppresses even when both clocks are stale (counter-test for the
documented behavior), so a future refactor can't silently invert it.

### Phase 4 — Enrich the breach diagnostic

Locations: `subagent_watchdog.rs:60-93, 133-153`, `watchdog.rs:583-686`,
`live_semantic_sink/mod.rs:684-704`.

**Correct:**

- `RecentSubagentInfo` struct with `id, name, description, completed_at,
  status` ✓
- `recent_subagents: VecDeque<RecentSubagentInfo>` field on
  `WatchdogState` with `RECENT_SUBAGENTS_CAP = 5` cap (lines 70, 82) ✓
- `subagent_stopped` pushes newest-first and pops_back when full
  (lines 142-152) ✓
- Live sink pulls `description` from `extra.description` and
  `extra.subagent_type` (lines 686-695) ✓
- `OpenCodeBreachContext` carries `subagent_done_count`,
  `step_in_flight`, `recent_subagents`, `now` ✓
- `format_step_timeout_breach_message` enriches the empty-outstanding
  case with subagent count, last-completion elapsed, step-in-flight hint,
  and the "Recent subagents:" listing in newest-first order
  (lines 615-641) ✓
- Tests cover count rendering, recent-subagent listing in newest-first
  order, the no-recent case, and the ring-buffer cap ✓

**Issue 3 (minor, presentation): `step_boundary` hint can disagree with
reality after the fact.**

At watchdog.rs:625-630 the breach prose says

> "A step boundary was still open at the time of timeout — the parent
> agent may have been waiting on a parallel subagent that has not yet
> returned."

This is true at the moment `step_in_flight` is observed. But because
the per-step grace *suppresses* the breach while `step_in_flight=true`
(except when both clocks are stale), the only way for the breach to
fire with `step_in_flight=true` is via the byte-heartbeat backstop —
which fires when the child is *genuinely silent*. So this hint is
specifically the "byte heartbeat fired during an open step" diagnostic;
it should not appear in the much more common "step closed and now we
fell silent" path. Today the test
`format_step_timeout_breach_message_opencode_no_recent_subagents`
explicitly passes `step_in_flight: true` and asserts the hint shows.
Verify in incident triage whether the wording is clear; if not,
consider tightening to e.g. "A step boundary was still open *and the
child produced no bytes* for {silence}". Not blocking.

### Phase 5 — Tests

All eight named tests in the plan landed, with three minor caveats:

| Plan name | Implementation | Location |
|---|---|---|
| `opencode_task_completion_synthesizes_subagent_lifecycle` | ✓ identical name | `lib/src/stream/providers/opencode.rs:1185` |
| `opencode_non_task_tool_does_not_synthesize_subagent_lifecycle` | ✓ identical name | `providers/opencode.rs:1247` |
| `opencode_step_in_flight_suppresses_silence` | ✓ identical name | `cli/.../watchdog.rs:1760` |
| `opencode_step_in_flight_resets_per_step` | ✓ identical name | `watchdog.rs:1809` (see Issue 4) |
| `opencode_byte_heartbeat_still_catches_zero_byte_hang` | ✓ identical name | `watchdog.rs:1889` |
| `opencode_breach_diagnostic_names_subagent_count` | ✓ identical name | `watchdog.rs:1926` |
| `watchdog_state_recent_subagents_ring_buffer_caps_at_5` | renamed to `recent_subagents_ring_buffer_caps_at_5` | `subagent_watchdog.rs:501` |
| `opencode_breach_diagnostic_lists_recent_subagent_descriptions` | ✓ identical name | `watchdog.rs:1979` |

**Issue 4 (test correctness): `opencode_step_in_flight_resets_per_step`
does not actually exercise *reset*.**

Lines 1809-1886 of `watchdog.rs`. The plan says: "drive two
`step_start` / `step_finish` cycles with silence in the middle of the
second; assert the silence rule is suppressed for both in-flight
windows." The implementation creates **three separate `WatchdogState`
+ `LiveMetricsState` instances**, one per cycle, and toggles the
`step_in_flight` field directly. It demonstrates that the *predicate*
behaves correctly given a particular value of `step_in_flight`, but it
does **not** demonstrate that `observe_event` correctly toggles the
flag back to `true` after a prior cycle has cleared it to `false`. The
toggling logic is covered separately in
`progress.rs::observe_event_tests`, so test coverage exists — but the
named test in this file does not match its plan-level intent. Either:

- rewrite this test to use a single state instance and drive
  `Info{step_phase=start}` → silence → `Info{step_phase=finish}` →
  silence → `Info{step_phase=start}` events through `observe_event`,
  OR
- accept that the predicate-behavior coverage is sufficient and rename
  the test to make that explicit (e.g.
  `opencode_step_in_flight_predicate_behavior_across_values`).

**Issue 5 (test location, minor): plan said `cli/tests/wrap_commands.rs`,
all tests are in `mod tests` blocks of source files.** The plan said
Phase 5 should live in `claudine/cli/tests/wrap_commands.rs` and the
protocol parser's `mod tests`. In practice the new tests are split
across unit-test modules in `lib/src/stream/providers/opencode.rs`,
`lib/src/stream/protocol/opencode.rs`, `cli/src/commands/wrap/exec/watchdog.rs`,
and `cli/src/commands/wrap/exec/subagent_watchdog.rs`. No
end-to-end integration tests were added to `wrap_commands.rs`. This is
arguably **better** for fast iteration and locality, but it does mean
there is **zero coverage of the synthesized-lifecycle path actually
reaching the live sink and populating `WatchdogState.recent_subagents`
in a real wrap run** — the unit tests stub that linkage. The existing
`watchdog_*` integration tests in `wrap_commands.rs` already cover
related paths; **recommendation**: add at least one integration test
that feeds an OpenCode `tool_use` NDJSON line for a `task` tool
through the full wrap stack and asserts the
`WatchdogState.recent_subagents` deque contains the entry. This guards
against any future plumbing regression between the synthesizer, the
live sink, and the watchdog state.

### Phase 6 — Documentation

Locations: `claudine/docs/topics/timeouts.md:411-466` and
`claudine/.claude/skills/claudine/SKILL.md:35`.

**Correct:**

- New OpenCode sub-section in `timeouts.md` lists all three layers
  (byte heartbeat, per-step grace, synthesized lifecycle) ✓
- Worked example at lines 480-514 walks through the step/silence
  interaction explicitly ✓
- Wall-clock rule's primacy is restated several times ✓
- SKILL.md paragraph on the 2026-05-10 fix now describes per-step
  grace and synthesized subagent lifecycle (line 35) ✓

No issues in Phase 6.

## Cross-cutting findings

### Issue 6 (minor, semantics): synthesized lifecycle has zero in-flight duration.

When the synthesizer fires from `handle_tool_use_completed`, the three
events (`SubagentStart`, `ToolResult`, `SubagentStop`) are emitted
back-to-back within the same `feed_line` call. The synthesized
SubagentStart's `started_at_epoch_ms` value in `extra` reflects the
*real* start time from the wire, but downstream `LiveMetricsState`
binds `started_at = now` (the parse-time `Instant`). The result is
that `in_flight_subagents` momentarily contains an entry, then
immediately loses it on the same tick.

This is correct for the breach-diagnostic path (recent_subagents
captures the data we care about). It would matter if anything between
`SubagentStart` and `SubagentStop` ever read `in_flight_subagents` —
but since the synthesizer is single-threaded with the parser, it
cannot. **Recommendation**: add a comment to
`handle_tool_use_completed` flagging that the start/stop are
synthesized atomically and that `in_flight_subagents` is not a useful
observation window for synthesized subagents — future readers will not
infer this from the code.

### Issue 7 (minor, plan-level): "Open questions" answered implicitly, not explicitly.

The plan's open question — "Should the synthesized events also fire
for the `task` tool's `error` completion path?" — was answered
correctly in the implementation (yes, status propagates), but no
explicit test guards the error path. **Recommendation**: add a one-line
test
`opencode_task_error_completion_still_synthesizes_subagent_lifecycle_with_error_status`
to lock the contract.

## Verification checklist status

Reading the plan's checklist (lines 401-416):

| Item | Status |
|---|---|
| All new Phase 5 tests pass | ✓ verified (55 watchdog, 152 opencode, all pass) |
| Existing `watchdog_*` tests still pass | ✓ verified |
| Existing OpenCode parser tests still pass | ✓ verified |
| `just lint` clean | not re-verified in this review — plan marks `[x]`, trust unless follow-up shows otherwise |
| No new user-facing env vars | ✓ verified by inspection |
| No "watchdog" in user-facing surface | ✓ verified (error block says "Agent Error" / "Step Timeout") |
| ≥ 9/10 success on real `compose prompts/commit.md` | NOT verified in this review — only the user can do this. Plan leaves this as `[ ]`; this remains the most important open empirical validation. |
| Deliberately stuck bash still hits `step_timeout` | ✓ logically follows from `opencode_byte_heartbeat_still_catches_zero_byte_hang` and the wall-clock backstop tests |

## Recommended follow-up actions

In rough priority order:

1. **Add a synthesized-error-completion test.** One small test guarding
   the "open question" answer (Issue 7). Low risk, high value as a
   regression guard.
2. **Rewrite or rename `opencode_step_in_flight_resets_per_step`.** Either
   make it drive `observe_event` for real or rename it so the test
   coverage gap is clear (Issue 4).
3. **Add an integration test in `wrap_commands.rs`** that drives the
   synthesized lifecycle through the full wrap stack and asserts the
   `WatchdogState.recent_subagents` deque populates (Issue 5).
4. **Delete the dead `_started_at` computation** in
   `handle_tool_use_completed` (Issue 1).
5. **Decide on the `subagent_type` ambiguity** in the live sink —
   either propagate it separately or drop the dead fallback (Issue 2).
6. **Add a cold-start byte-heartbeat counter-test** asserting the
   documented "no breach until step_finish observed" semantics
   (Phase 3 observation).
7. **Optional**: tighten the "step boundary still open" wording in the
   breach prose so it does not surface during the byte-heartbeat path
   (Issue 3) — only if the wording proves confusing in real triage.
8. **Optional**: comment the synthesized-lifecycle atomicity in
   `handle_tool_use_completed` so future readers understand the
   in-flight observation gap (Issue 6).

## Verdict

**Approve.** The plan was thorough, the implementation is faithful, all
critical correctness invariants are covered by tests, the docs are
updated, and there are no behavioral regressions. The issues above are
either follow-up polish or test-coverage hardening, none of which need
to block landing this fix. The single empirical validation that remains
(`compose prompts/commit.md` ≥ 9/10) is the user's call and cannot be
adjudicated from code review alone.
