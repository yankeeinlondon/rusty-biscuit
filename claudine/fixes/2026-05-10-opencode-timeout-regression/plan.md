---
source_files_during_phase_1:
  - claudine/cli/tests/wrap_commands.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - claudine/lib/src/stream/progress.rs
  - claudine/cli/src/commands/wrap/exec/spawn.rs
  - claudine/cli/src/commands/wrap/exec/watchdog.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - claudine/lib/src/stream/progress.rs
  - claudine/cli/src/commands/wrap/exec/timeouts.rs
  - claudine/cli/src/commands/wrap/exec/watchdog.rs
  - claudine/cli/src/commands/wrap/composition/mod.rs
  - claudine/cli/src/commands/wrap/harness_orch.rs
  - claudine/cli/src/commands/wrap/mod.rs
  - claudine/cli/tests/sequence_cli.rs
  - claudine/cli/tests/wrap_commands.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - claudine/cli/src/commands/wrap/exec/watchdog.rs
  - claudine/cli/tests/wrap_commands.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5: []
docs_updated_during_phase_5:
  - claudine/docs/topics/timeouts.md
docs_created_during_phase_5: []
skills_files_updated_during_phase_5:
  - .claude/skills/claudine/SKILL.md
source_files_during_phase_6: []
docs_updated_during_phase_6:
  - claudine/docs/topics/timeouts.md
docs_created_during_phase_6: []
skills_files_updated_during_phase_6: []
packages:
  - claudine
  - claudine-cli
---

# OpenCode Timeout Regression Fix

## Problem statement

A claudine `compose` workflow that previously succeeded ~99% of the time
on OpenCode (specifically the `prompts/commit.md` flow with Minimax 2.7)
now succeeds only ~15% of the time. The remaining runs are killed by
the `step_timeout` rule mid-flight, even though the actual work has
already completed (commits land, subagents return, etc.). The wrapped
OpenCode process is **not** hung — claudine misclassifies legitimate
silence as a hang.

The user has confirmed:

- The transcript shows all subagents (`Task(successful, …)`) returning.
- The closing parent text ("Now running sniff repo to show the final
  state:") was rendered to the console before the kill fired.
- The only variable that changed between the 99%-success regime and
  the 15%-success regime is **claudine itself**. The prompt, the
  agentic CLI (OpenCode), and the model (Minimax 2.7) are identical.

## Root cause

Two structural facts combine to produce the regression.

### 1. OpenCode's stream protocol is sparser than other providers

Per [`claudine/lib/src/stream/providers/opencode.rs`](../../lib/src/stream/providers/opencode.rs)
and observed transcripts:

- OpenCode does **not** emit `tool_start` events. Tools surface only as
  `tool_use` (or `tool_result` / `tool_end`) **after** they reach
  `completed` / `error`.
- OpenCode does **not** emit `task_started` events for the `task`
  subagent tool. The parser variant exists but never fires from current
  OpenCode releases.

Consequence: `LiveMetricsState.in_flight` and `in_flight_subagents` are
never populated during an OpenCode run. The "stuck-aware suppression"
in [`detect_step_timeout`](../../cli/src/commands/wrap/exec/timeouts.rs)
and [`evaluate_timeout_tick`](../../cli/src/commands/wrap/exec/watchdog.rs)
has nothing to suppress against; the silence clock ticks straight
through legitimate tool execution and reasoning.

### 2. The previous OpenCode-specific guard was removed

Before the unified watchdog landed (commits `b492e82d`, `8dde01b4`,
`0b603979`, finalized with `ce026a18`), claudine had a function called
`detect_opencode_hang_termination` with two OpenCode-specific guards:

1. A `provider_status.is_some()` check — silence kills could not fire
   until at least one `step_finish` boundary had been observed. Slow
   provider startup and long first turns were protected.
2. A separate threshold parameter (`stop_threshold`) distinct from the
   user-facing `step_timeout`. The user's frontmatter
   `step_timeout: 7m` did not directly govern OpenCode silence kills —
   an internal, more generous threshold did.

When the unified `detect_step_timeout` replaced the OpenCode-specific
detector, both guards disappeared. The user-set `step_timeout` now
applies to OpenCode silence kills with no compensation for OpenCode's
stream sparsity. Workloads that happened to consume 5–8 minutes of
post-fan-out silence (a normal Minimax 2.7 closing-synthesis pattern)
now collide with reasonable `step_timeout` settings.

## Goal

Restore the historical behaviour where the same prompt + model + CLI
combination succeeds reliably, **without**:

- Raising the `step_timeout` default above `30m` (the user explicitly
  approved keeping `30m`).
- Introducing a third user-facing timeout knob (only `CLAUDINE_TIMEOUT`
  and `CLAUDINE_STEP_TIMEOUT` are permitted, mapping 1-for-1 to the
  `timeout` and `step_timeout` frontmatter properties).
- Promoting "watchdog" to user-facing nomenclature anywhere (env vars,
  error strings, CLI flags, rendered prose). Internal Rust symbols may
  retain `watchdog`.

## Fix shape

A two-layer repair: a provider-agnostic activity heartbeat sourced from
raw stdout bytes, plus restoration of the OpenCode `provider_status`
grace as a defensive backstop.

### Layer A — Stdout-byte activity heartbeat (provider-agnostic)

Any bytes received from the wrapped child's stdout (or stderr where
the provider streams structured events on stderr) reset
`last_event_at`, **before** the bytes are passed to the semantic
parser. This is the most robust signal because it does not depend on
any one provider's event richness.

Specifically:

- A `Reasoning` chunk that fails to deserialize and falls through to
  `ProviderExtension` already counts as activity, but only because
  the parser eventually produces a `SemanticEvent`. The new heartbeat
  fires earlier — at the byte-stream layer — so even partially
  buffered output (provider mid-flush) refreshes the clock.
- The heartbeat is gated to ignore empty / whitespace-only writes so
  a child that flushes blank lines does not look infinitely active.

### Layer B — Restore OpenCode `provider_status` grace

When the wrapped provider is OpenCode and `LiveMetricsState.provider_status`
is `None` (no `step_finish` observed yet), suppress the `step_timeout`
breach entirely. The wall-clock `timeout` rule still applies as the
backstop. This mirrors the historical
`detect_opencode_hang_termination` guard and gives slow first turns /
slow startup unconditional protection.

This guard fires only on the OpenCode provider. Other providers do not
need it because their richer event surface populates `in_flight` and
`in_flight_subagents` correctly.

### Out of scope (deliberately)

- **Synthesizing in-flight subagents from parent-text patterns** —
  considered and rejected as too fragile (the "Launching N subagents"
  text varies by model and version).
- **Provider-aware `step_timeout` floor multipliers** — considered and
  rejected as crude; documented in `docs/topics/timeouts.md` as a user
  recommendation instead.
- **Removing or renaming `CLAUDINE_WATCHDOG_INTERVAL`** — handled in a
  sibling fix (`fixes/watchdog-nomenclature/plan.md`, to be authored).
  Out of scope here so this plan stays focused on the regression.

## Phases

### Phase 1 — Failing reproduction test

Author an integration test that drives an OpenCode-shaped JSONL stream
through the wrapper:

1. Emit a `step_start` and a sequence of `text` events.
2. Emit several `tool_use` (post-completion) events with no
   corresponding `tool_start`.
3. Emit a final parent-text event ("Now running sniff repo:").
4. Go silent for slightly longer than the test's `step_timeout`.
5. Assert that the wrapper does **not** kill the session prematurely
   under the fixed behaviour. (The test starts as failing; the fix
   makes it pass.)

The test must be deterministic — drive `Instant::now()` through a
mockable clock or use an existing test seam.

Files:

- `claudine/cli/tests/wrap_commands.rs` (new test cases near the
  existing `watchdog_*` tests).
- Possibly `claudine/cli/src/commands/wrap/exec/watchdog.rs` if a new
  test seam is required.

### Phase 2 — Stdout-byte activity heartbeat

1. Add an activity-heartbeat hook in the wrapper's stdout/stderr read
   loops (`claudine/cli/src/commands/wrap/exec/spawn.rs` and the live
   semantic sink).
2. On every non-empty byte chunk read from the child, refresh
   `LiveMetricsState.last_event_at` (or a parallel `last_byte_at`
   field with the same semantics).
3. Update `evaluate_timeout_tick` to use the byte-clock as the
   silence reference when it is more recent than `last_event_at`.

Files:

- `claudine/lib/src/stream/progress.rs` — extend `LiveMetricsState`
  with the new field and a record method.
- `claudine/cli/src/commands/wrap/exec/spawn.rs` — call the record
  method on every byte read.
- `claudine/cli/src/commands/wrap/exec/watchdog.rs` — change the
  silence reference to `max(last_event_at, last_byte_at)`.

### Phase 3 — Restore OpenCode `provider_status` grace

1. Add an OpenCode-specific guard in `evaluate_timeout_tick` (or a
   helper it calls): when the active provider is OpenCode and
   `provider_status` is `None`, return `WatchdogTickResult::Ok`
   without firing `step_timeout`.
2. The wall-clock `timeout` rule continues to apply unconditionally.

Files:

- `claudine/cli/src/commands/wrap/exec/watchdog.rs`.
- `claudine/cli/src/commands/wrap/exec/timeouts.rs` if the helper
  shape moves.
- The `Provider` discriminant must thread through to the tick
  evaluator; check whether it already does via `TimeoutConfig` or the
  spawn context.

### Phase 4 — Tests

1. Reproduction test from Phase 1 passes.
2. New unit tests in `timeouts.rs` for the OpenCode `provider_status`
   guard (covers: no step_finish + silence → suppressed; step_finish
   seen + silence → fires).
3. New unit tests for the byte-heartbeat: silence with stdout bytes
   continuing → suppressed; silence with no bytes → fires.
4. Ensure existing watchdog tests (`watchdog_subagent_hang_*`,
   `watchdog_stream_idle_*`, `watchdog_wall_clock_*`) still pass —
   the byte heartbeat must not mask genuine hangs (e.g. a child that
   is truly stuck producing zero bytes).
5. Add a Claude-shaped reproduction test that confirms the in-flight
   gate continues to suppress on Claude (regression guard for
   layer interactions).

### Phase 5 — Documentation

1. Update [`claudine/docs/topics/timeouts.md`](../../docs/topics/timeouts.md)
   to remove the "open fix plan" caveat once the fix lands, and to
   document the byte-heartbeat in the
   [Activity vocabulary](../../docs/topics/timeouts.md#activity-vocabulary)
   section.
2. Update the OpenCode provider research at
   [`claudine/docs/research/hooks/opencode.md`](../../docs/research/hooks/opencode.md)
   if newer OpenCode releases now emit `tool_start` /
   `task_started` (the doc may be stale; verify against current
   OpenCode behaviour as part of this phase).
3. Update the local skill at
   [`claudine/.claude/skills/claudine/SKILL.md`](../../.claude/skills/claudine/SKILL.md)
   to mention the byte-heartbeat behaviour and provider-specific
   silence semantics.

### Phase 6 — Validation

1. Re-run the user's `prompts/commit.md` flow on OpenCode + Minimax
   2.7 ten times. Target ≥ 9/10 success.
2. Re-run an equivalent commit-style prompt on Claude and Codex to
   confirm no behavioral regression on rich-stream providers.
3. Confirm the `claudine logs errors` query still surfaces real
   `step_timeout` kills (e.g. a deliberately hung bash test) so the
   fix has not over-suppressed.

## Verification checklist

Before merging:

- [ ] Phase 1 reproduction test passes.
- [ ] All existing `wrap_commands.rs` watchdog tests pass.
- [ ] Clippy clean: `just lint claudine` and `just lint claudine-cli`.
- [ ] Doc test for `claudine/docs/topics/timeouts.md` (if any) renders
      correctly.
- [ ] User-facing env vars are still exactly `CLAUDINE_TIMEOUT` and
      `CLAUDINE_STEP_TIMEOUT`. No new env var introduced.
- [ ] No "watchdog" string appears in any user-facing surface added or
      changed by this fix.
- [ ] Manual reproduction of the user's flow succeeds ≥ 9/10 runs.

## Risks

- **Byte heartbeat masks a real hang.** A child process that is truly
  stuck (e.g. blocked on syscall) typically produces zero stdout
  bytes, so the heartbeat will still allow `step_timeout` to fire.
  Validate this in Phase 6 by deliberately hanging a bash tool and
  confirming the kill still happens within `step_timeout + grace`.
- **Provider plumbing.** The OpenCode-specific guard in Phase 3
  requires the `Provider` discriminant inside the tick evaluator. If
  the plumbing is awkward, fall back to a `provider: Option<Provider>`
  field on `TimeoutConfig` that is set at spawn time.
- **Test flakiness.** Time-based watchdog tests can be flaky on slow
  CI. Use the existing 1s `CLAUDINE_WATCHDOG_INTERVAL` test override
  pattern (this env var is internal-only and will be renamed in a
  separate fix; do not extend its surface here).

## Open questions

- Should the byte heartbeat refresh `last_event_at` directly, or live
  on a separate `last_byte_at` field? Direct refresh is simpler but
  conflates "structured event observed" with "bytes flowed." A
  separate field is cleaner but requires more touches in the
  evaluator. Recommendation: separate field, evaluator picks the more
  recent of the two.
- Should the OpenCode `provider_status` grace also apply to other
  providers known to have sparse streams (Goose, Kimi, Qwen)? Defer
  until incidents on those providers actually appear; do not preemptively
  expand scope.
