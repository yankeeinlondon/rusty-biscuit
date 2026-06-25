---
agent: open_code/zai-coding-plan/glm-5.2
phases: 6
created: 2026-06-23
start_phase: 1
yolo: "true"
---

# Execution Plan: Live-but-Dead Guard (Stalled-Generation Detection)

Converts [`spec.md`](spec.md) into a dependency-ordered, OpenCode-scoped
implementation. The guard keys on a **retry-churn fingerprint** (repeated
`llm_call_start` with zero intervening progress) that is provably distinct
from a legitimately long tool call, and aborts fail-fast as
`ProcessTermination::Aborted` (`error_kind = "stalled_generation"`).

## Design decisions locked from the spec

These are settled by the spec and must not be relitigated during implementation:

1. **Mechanism = Option B (stalled-generation backstop).** Two-condition trip:
   retry-churn (`llm_call_start` count since last progress `>= N=4`) **AND**
   progress-silence (`now - last_progress_at >= stall_timeout`, default `10m`).
   Both must hold. Either condition alone does not fire.
2. **Scope = OpenCode bridge only.** State lives in
   `OpenCodeLogBridge` (`claudine/lib/src/stream/logs/opencode/reasoning.rs`).
   Do **not** change `SemanticEvent::is_activity()` or reclassify `Info`
   globally — that would regress `step_timeout`.
3. **Progress vs. liveness is bridge-local.** The reset helper is called at
   the bridge's progress-class handler sites. The bridge is a *producer* of
   the shared semantic sink, not a *consumer*, so it cannot directly observe
   stdout-origin events (`OutputText`/`Reasoning`/`ToolCall`/`ToolResult`/
   `FileChange`/`PlanUpdate`). This is safe and intentional: those events are
   **anti-correlated** with the retry-churn fingerprint — a run producing
   assistant output is not in a silent generation-drop loop, so the
   count condition (≥4 dropped generations) cannot co-occur with flowing
   output. The guard therefore resets on the progress events the bridge
   *can* see: genuine `StepLoop` advance, `StepExit`, `SubagentStart`,
   `SubagentStop`. See Phase 2 for the verification test that locks this
   anti-correlation.
4. **Routing = fail-fast `Aborted`, never `handle_timeout:`.** Mirrors the
   `RepeatedStreamError` shape exactly. `SemanticErrorKind::AgentNative`.
5. **Distinct label "Stalled Generation"** (spec Open Question 2 → Option A),
   while keeping `AgentNative` color/style for consistency.
6. **Time uses `Instant`** (monotonic, immune to wall-clock skew). Detector
   logic accepts `now: Instant` so tests inject time without sleeping.
7. **No single-generation threshold** (spec Open Question 1 → Option A).
   Only retry-churn is in scope; single-generation dead air stays out.
8. **Generation-count threshold `N` ships as a constant**
   (`MAX_GENERATIONS_WITHOUT_PROGRESS = 4`), paralleling
   `MAX_CONSECUTIVE_STREAM_ERRORS`. Not a knob in this cut.
9. **`stall_timeout_warn` is out of scope.**
10. **Count only `LlmCall` where `is_stream == true`.** Store safe metadata
    only (`session_id`, `step`, `agent`, `provider_id`, `model_id`, `mode`).
    Never store prompt text, tool inputs, HTTP URLs, auth headers, or raw
    stderr lines.

## Parallelism map

- **Phase 1** is the blocking foundation (pure type/constant additions).
  Everything depends on it.
- **Phase 2** (detector/bridge) and **Phase 3** (termination helpers) are
  **parallelizable** after Phase 1 — they touch different files
  (`reasoning.rs` vs. `termination.rs`) and both depend only on the Phase 1
  type definitions.
- **Phase 4** (configuration + threading) depends on Phase 2 (the bridge
  constructor signature change).
- **Phase 5** (test suite) depends on Phases 2–4. Detector unit tests may be
  authored alongside Phase 2; the dedicated phase ensures the full spec
  matrix is covered and the `RepeatedStreamError` regression is locked.
- **Phase 6** (documentation) can be **drafted in parallel** once Phases 1–3
  land (it needs the final `error_kind` name and termination-table rows), but
  must be finalized after Phase 4 confirms the exact CLI/env surface.

## Shared constants & names (use these verbatim)

| Symbol | Value | Defined in |
|---|---|---|
| `MAX_GENERATIONS_WITHOUT_PROGRESS` | `4` | `reasoning.rs` (near `MAX_CONSECUTIVE_STREAM_ERRORS`) |
| `error_kind` | `"stalled_generation"` | `termination.rs` |
| env var | `CLAUDINE_OPENCODE_STALL_TIMEOUT` | timeout resolution |
| built-in default | `10m` (`Duration::from_secs(10 * 60)`) | timeout resolution |
| CLI flag | `--stall-timeout <DURATION>` | wrapper + compose flags |
| frontmatter key | `stall_timeout` | `HarnessPlan` + composition request |

---

## Phase 1 — Foundation types and constants

**Goal.** Add the new `EarlyTermination` variant, the context struct, the
`MAX_GENERATIONS_WITHOUT_PROGRESS` constant, and the `GuardContext`
extension. Pure type additions — no behavior, no trip logic. Everything
downstream depends on these symbols existing.

**Files.**
- `claudine/lib/src/stream/logs/opencode/reasoning.rs` — enum + constant + context struct
- `claudine/lib/src/harness/model.rs` — `GuardContext` fields

**Tasks.**

- [ ] Add `StalledGenerationContext` struct to `reasoning.rs` (derives
      `Debug, Clone, PartialEq, Eq`, all fields `Option<String>`):
      `session_id`, `step` (store as `Option<u32>`), `agent`, `provider_id`,
      `model_id`, `mode`. Document that it carries **only** safe metadata —
      no prompt text, tool payloads, HTTP URLs, or auth headers.
- [ ] Add `EarlyTermination::StalledGeneration` variant to the enum in
      `reasoning.rs` (alongside `RepeatedStreamError`):
      `{ generation_count: u32, stall_duration: Duration,
       context: StalledGenerationContext }`. Rustdoc must state: maps to
      `ProcessTermination::Aborted`, `error_kind = "stalled_generation"`,
      fail-fast (never `handle_timeout:`), and that it fires only when both
      the retry-churn and progress-silence conditions hold.
- [ ] Add `const MAX_GENERATIONS_WITHOUT_PROGRESS: u32 = 4;` to
      `reasoning.rs`, immediately above/beside
      `MAX_CONSECUTIVE_STREAM_ERRORS`, with a rustdoc comment explaining the
      two-condition defense against false positives (single slow generation
      is exempt because the count condition remains required).
- [ ] Extend `GuardContext` in `claudine/lib/src/harness/model.rs` with two
      `Option` fields: `generation_count: Option<u32>` and
      `stall_duration_ms: Option<u64>`. Update the struct doc comment's
      per-trip cluster list to include the stalled-generation cluster.
- [ ] Satisfy every exhaustive `match` on `EarlyTermination` the compiler
      flags (notably in `termination.rs` — those are implemented in Phase 3,
      but add temporary `todo!()`-free arms or leave for Phase 3; preferred:
      let Phase 3 own them, but ensure `cargo check -p claudine` still
      compiles by adding the arms here if the enum is matched exhaustively
      outside `termination.rs`). Record any non-`termination.rs` exhaustive
      match sites discovered.

**Validation checkpoint.**

- [ ] `cargo check -p claudine` compiles (any new exhaustive-match arms are
      present; Phase 3 fills their bodies).
- [ ] `cargo check -p claudine-cli` compiles only after Phase 3 lands its
      `termination.rs` arms — if Phase 1 and Phase 3 run in parallel, the
      integration build is gated on Phase 3 completing.

---

## Phase 2 — Stalled-generation detector (bridge-local)

**Goal.** Implement the detector: the bridge-local `StalledGenerationState`,
the progress/liveness taxonomy at the bridge's handler sites, `LlmCall`
counting with safe context capture, and the two-condition trip that fires
`EarlyTermination::StalledGeneration`. Includes the bridge constructor change
to accept `stall_timeout`.

**Depends on.** Phase 1.
**Parallelizable with.** Phase 3 (different file).

**Files.**
- `claudine/lib/src/stream/logs/opencode/reasoning.rs` — state, helper, handler wiring, constructor

**Tasks.**

- [ ] Add bridge-local state. Add three fields to `OpenCodeLogBridge`:
      `last_progress_at: Instant`, `generation_count_since_progress: u32`,
      `last_generation_context: Option<StalledGenerationContext>`. Initialize
      `last_progress_at = Instant::now()` in the constructor (the spec
      mandates initialization at bridge creation, not waiting for stdout
      NDJSON, because the incident happened entirely while stderr was active).
- [ ] Extend `OpenCodeLogBridge::new` signature to accept
      `stall_timeout: Option<std::time::Duration>`. Store it on the struct.
      `None` disables the guard (the `0s`-disables contract resolves to
      `None` upstream in Phase 4). Update the two call sites in `policy.rs`
      to pass a placeholder for now (`None`) — Phase 4 replaces the
      placeholder with the resolved value. (These two `policy.rs` edits are
      mechanical stubs so this phase compiles standalone.)
- [ ] Add a private `reset_stalled_generation_progress(&mut self, now: Instant)`
      helper that sets `last_progress_at = now` and
      `generation_count_since_progress = 0` (context is retained until the
      next `LlmCall` overwrites it, so the trip message still names the last
      attempted generation).
- [ ] Add a private `record_llm_call_and_check_trip(&mut self, now: Instant,
      ctx: StalledGenerationContext) -> Option<EarlyTermination>` helper.
      Logic: increment `generation_count_since_progress`; store
      `last_generation_context = Some(ctx)`; if `stall_timeout` is `Some(d)`
      AND `generation_count_since_progress >= MAX_GENERATIONS_WITHOUT_PROGRESS`
      AND `now.duration_since(last_progress_at) >= d`, return
      `Some(EarlyTermination::StalledGeneration { generation_count, stall_duration,
      context: ctx.clone() })`; else `None`. Note: the *first* `LlmCall` after
      progress alone never trips — the count condition is required (protects
      legitimately slow first generations).
- [ ] Wire the trip into `on_llm_call`: guard on `is_stream == true`; build
      `StalledGenerationContext` from the record's safe tags
      (`session.id` → `session_id`, current dedup step for `step`, `agent`,
      `provider_id`, `model_id`, `mode`); call the trip helper; if it returns
      `Some`, emit a terminal `SemanticEvent::Error { kind:
      SemanticErrorKind::AgentNative, terminal: true, message, extra }` with
      the "Stalled Generation" classification and `fire_early_termination()`
      exactly once (reuse the existing `early_terminate_fired` idempotency).
      The existing primary provider/model capture logic stays unchanged.
- [ ] Wire resets into the progress-class handler sites:
      - `on_step_loop`: call `reset_stalled_generation_progress(now)` **only
        after** the dedup check passes (genuine step advance). This sits next
        to the existing `consecutive_stream_errors = 0` reset. Deduped
        repeats (same `(session_id, step)`) must NOT reset — they return
        early as today.
      - `on_step_exit`: call the reset (it already clears
        `last_step_per_session`; the reset makes a follow-up prompt fresh).
      - `on_session_created` (the `SubagentStart` branch only — the child
        session path): call the reset.
- [ ] Add the terminal-error renderer for the live stderr badge. Emit a
      `SemanticEvent::Error` whose `message` includes the generation-attempt
      count and elapsed progress silence, plus available context (session id,
      step, agent, provider id, model id, mode) without leaking prompt/tool
      payloads. Use the "Stalled Generation" label (spec OQ2 → A) with
      `SemanticErrorKind::AgentNative` so color/style stays consistent with
      other agent-native failures.
- [ ] **Do not** reset on: `LlmCall`, deduped/repeated `StepLoop` for the
      same `(session_id, step)`, `HttpResponse`, `PermissionEvaluated`,
      `BootBanner`, `Snapshot`, `Unclassified`, filtered `service=bus`, or
      raw bytes. Verify each of these handler paths leaves
      `generation_count_since_progress` untouched.

**Validation checkpoint.**

- [ ] `cargo check -p claudine` compiles; the `policy.rs` stub call sites
      compile.
- [ ] A quick inline sanity test (will be formalized in Phase 5): 4 streamed
      `LlmCall` (`is_stream=true`) records with `now` advanced past
      `stall_timeout` and no progress in between fires
      `EarlyTermination::StalledGeneration`; 3 do not; 4 under the duration do
      not; a genuine `StepLoop` advance between them resets.

---

## Phase 3 — Termination plumbing (summary, message, mapping, guard context)

**Goal.** Extend the four existing termination helpers so
`EarlyTermination::StalledGeneration` flows end-to-end to the synthesized
summary, the failure-handler payload, and `ProcessTermination::Aborted`.
This is the CLI-side counterpart of the `RepeatedStreamError` plumbing.

**Depends on.** Phase 1.
**Parallelizable with.** Phase 2 (different file: `termination.rs`).

**Files.**
- `claudine/cli/src/commands/wrap/exec/termination.rs` — the four helpers + renderer + tests

**Tasks.**

- [ ] Add a `render_stalled_generation_message(generation_count, stall_duration,
      context)` renderer in `termination.rs` (beside
      `render_repeated_stream_error_message`). The message must include the
      generation-attempt count, the elapsed progress silence, and — when
      available — session id, step, agent, provider id, model id, and mode.
      Never include prompt text or tool payloads.
- [ ] Extend `apply_early_termination_to_summary` with a
      `StalledGeneration { generation_count, stall_duration, context }` arm:
      set `exit_code = 1`, `is_error = true`,
      `error_kind = Some("stalled_generation".into())`, and
      `error_message = Some(render_stalled_generation_message(...))`.
- [ ] Extend `early_termination_message` with the matching arm returning the
      same rendered string (so the live stderr badge and the summary never
      drift — this is exactly the contract locked by the existing
      `early_termination_message_matches_summary_error_message_for_all_variants`
      test).
- [ ] Extend `early_termination_process_outcome` (the spec calls this
      `process_termination_from_early`): add `StalledGeneration` to the
      `ProcessTermination::Aborted` arm alongside `ExitExpression`,
      `RunawayRepetition`, `RunawayVolume`, and `RepeatedStreamError`. It must
      **never** route to `TimedOut` (which would trigger `handle_timeout:`
      and reproduce the stall).
- [ ] Extend `early_termination_guard_context` with a `StalledGeneration` arm
      returning a `GuardContext` populated with `generation_count` and
      `stall_duration_ms` (computed from `stall_duration.as_millis()`), plus
      optional context-derived fields where `GuardContext` exposes them
      (leave pattern/scope/cycle_len/repeats/lines/bytes as `None`).

**Validation checkpoint (tests added in this phase).**

- [ ] Extend `early_termination_guard_context_populates_relevant_cluster`
      (or add a sibling test) to assert the `StalledGeneration` cluster
      populates `generation_count` + `stall_duration_ms` and leaves the
      runaway/exit-expression clusters `None`.
- [ ] The existing exhaustive
      `early_termination_message_matches_summary_error_message_for_all_variants`
      test now covers `StalledGeneration` (add the variant to its case list)
      and proves the helper and summary message strings are byte-identical.
- [ ] Add a test asserting `early_termination_process_outcome(StalledGeneration)`
      yields `ProcessTermination::Aborted`.

---

## Phase 4 — Configuration surface and threading

**Goal.** Add the `stall_timeout` knob across the four precedence layers
(CLI flag > frontmatter > env > built-in `10m`), resolve it with the same
duration grammar as `step_timeout`, and thread the resolved value from the
wrapper/compose entry points through `build_structured_plumbing` into the
bridge constructor (replacing the Phase 2 stub). `0s` disables. Non-OpenCode
runs accept the key silently as inert config (debug trace only, never warn).

**Depends on.** Phase 2 (bridge constructor accepts `stall_timeout`).

**Files.**
- `claudine/cli/src/commands/wrap/flags.rs` — `--stall-timeout` wrapper flag
- `claudine/cli/src/commands/compose/mod.rs` — `--stall-timeout` compose flag + `step_timeout_secs`-style parser
- `claudine/lib/src/harness/model.rs` — `HarnessPlan.stall_timeout`
- `claudine/lib/src/harness/parse/mod.rs` — parse frontmatter `stall_timeout`
- `claudine/lib/src/composition/types.rs` — `SessionRequest.stall_timeout`
- `claudine/cli/src/commands/wrap/composition/timeouts.rs` — resolution helper
- `claudine/cli/src/commands/wrap/composition/mod.rs` — resolve into the request
- `claudine/cli/src/commands/wrap/wrapper_stages.rs` — resolve on the direct wrapper path
- `claudine/cli/src/commands/wrap/policy.rs` — thread into `build_structured_plumbing`
- `claudine/cli/src/commands/wrap/wrapper_exec.rs`, `harness_orch/attempt.rs` — pass resolved value
- `claudine/cli/src/argv/flag_surface.rs` — drift-detection list for value-bearing flags

**Tasks.**

- [ ] Add `--stall-timeout <DURATION>` to `WrapperArgs` in
      `cli/src/commands/wrap/flags.rs`, modeled on the `--step-timeout` field
      (same `value_name = "DURATION"`, doc noting it is OpenCode-scoped and
      `0s` disables). Add the matching line to `print_wrapper_help`.
- [ ] Add `--stall-timeout <DURATION>` to the compose args in
      `cli/src/commands/compose/mod.rs` with a `stall_timeout_secs(&self)`
      parser mirroring `step_timeout_secs` (uses
      `claudine::harness::parse_timeout`). Apply the same
      interactive-conflict posture `--step-timeout` uses (structured-stream
      only; emit the "only enforced in structured-stream mode" warning when
      the run does not qualify, like `wrapper_stages.rs` does for
      `--step-timeout`).
- [ ] Add `stall_timeout: Option<String>` to the composition `SessionRequest`
      in `lib/src/composition/types.rs` (beside `step_timeout`), threaded
      through `prep.rs` / `sequence/iterate.rs` exactly as `step_timeout` is.
- [ ] Add `stall_timeout: Option<std::time::Duration>` to `HarnessPlan` in
      `lib/src/harness/model.rs` and parse it from frontmatter in
      `lib/src/harness/parse/mod.rs` (`parse_harness_plan`), using the same
      duration grammar and source-path diagnostics as `step_timeout`.
- [ ] Add a `resolve_stall_timeout` resolution path in
      `cli/src/commands/wrap/composition/timeouts.rs` that reuses
      `resolve_single_timeout` with `env_var =
      "CLAUDINE_OPENCODE_STALL_TIMEOUT"` and `built_in =
      Some(Duration::from_secs(10 * 60))`. Reuse `is_zero_duration_literal`
      so `0s` disables (resolves to `None`). Wire it into both
      `resolve_timeouts`-style assembly sites (direct wrapper in
      `wrapper_stages.rs`, composition in `composition/mod.rs`) so CLI >
      frontmatter > env > built-in precedence matches `step_timeout`.
- [ ] Thread the resolved `Option<Duration>` into `build_structured_plumbing`
      in `policy.rs` (extend its signature) and from there into
      `OpenCodeLogBridge::new(... stall_timeout)`, **replacing the Phase 2
      stub**. Update the two call sites (`wrapper_exec.rs:82` direct path,
      `harness_orch/attempt.rs:147` harness path) to pass the resolved value.
      For non-OpenCode providers the value is ignored silently (the bridge is
      only constructed for OpenCode/Codex; stall logic is OpenCode-only).
- [ ] Add `--stall-timeout` to the `COMPOSITION_FLAGS_WITH_VALUE` drift
      surface in `cli/src/argv/flag_surface.rs` and extend its parametric test
      (`rule_3_skips_value_for_every_composition_flag_with_value`) case list
      with `("--stall-timeout", "10m")` so a future flag-list/derive drift is
      a test failure, not a latent bug.
- [ ] Non-OpenCode behavior: when `stall_timeout` is supplied to a
      non-OpenCode run, accept it silently (the key is provider-neutral for
      portable prompt files). Emit at most a `debug!` trace; never warn the
      user. Confirm no `--stall-timeout cannot be used with...` style errors
      fire for non-OpenCode providers.

**Validation checkpoint.**

- [ ] `cargo check -p claudine -p claudine-cli` compiles.
- [ ] `CLAUDINE_OPENCODE_STALL_TIMEOUT=0s` resolves to `None` (guard
      disabled); an unset env with no CLI/frontmatter resolves to `10m`; a
      CLI `--stall-timeout 2m` wins over a `5m` env; frontmatter wins over
      env (mirror the existing `step_timeout` resolution tests in
      `composition/tests.rs`).

---

## Phase 5 — Test suite and regression lock

**Goal.** Land the full spec test matrix (acceptance criteria 7 + the
"Tests" section) and prove no regression in `RepeatedStreamError` or the
existing timeout/content-guard paths.

**Depends on.** Phases 2, 3, 4.

**Files.**
- `claudine/lib/src/stream/logs/opencode/reasoning.rs` (`#[cfg(test)] mod tests`)
- `claudine/cli/src/commands/wrap/exec/termination.rs` (extend existing tests)
- `claudine/lib/tests/` (handler-payload level, if the existing `runaway_handler_payload.rs` pattern applies to the new `error_kind`)

**Tasks — detector (reasoning.rs, inject `now: Instant`, no real sleeps).**

- [ ] Four streamed `LlmCall` (`is_stream=true`) records over `>= stall_timeout`
      with no progress fire `EarlyTermination::StalledGeneration`.
- [ ] Three records over the same duration do **not** fire.
- [ ] Four records **under** the duration do **not** fire (proves both
      conditions are required).
- [ ] A progress-class event between `LlmCall` records resets count and time
      (use a genuine `StepLoop` advance as the reset trigger).
- [ ] Repeated/deduped `StepLoop` for the same `(session_id, step)` does
      **not** reset; a genuine step advance does reset.
- [ ] `HttpResponse`, `PermissionEvaluated`, filtered `service=bus`, and raw
      bytes do **not** reset the stalled-generation state.
- [ ] Long-tool shape (no `LlmCall` records at all) never trips this guard,
      even when elapsed time exceeds `stall_timeout`.
- [ ] **Anti-correlation lock** (Design Decision 3): a run that would be
      producing stdout-origin progress cannot false-trip because the
      retry-churn count cannot accumulate; encode this as a test comment + a
      case showing that with `stall_timeout` disabled (`None`) the count
      never triggers termination even with 4+ `LlmCall` records.
- [ ] The terminal `SemanticEvent::Error` carries `kind = AgentNative`,
      `terminal = true`, a "Stalled Generation" classification, and a
      `guard_context`-equivalent `extra` map containing `generation_count`
      and a stall duration, **without** prompt text or tool payloads.

**Tasks — bridge independence from `RepeatedStreamError`.**

- [ ] `RepeatedStreamError` still fires independently for repeated
      `message="stream error"` records and is **not** reset by `LlmCall`
      (regression for `fixes/2026-06-21-opencode-log-fix`). Confirm the two
      counters are independent: `LlmCall` churn does not clear
      `consecutive_stream_errors`, and a `stream error` does not clear the
      stalled-generation count.
- [ ] `fire_early_termination` still fires at most once per bridge
      (`early_terminate_fired` idempotency holds when both guards could trip
      on adjacent records).

**Tasks — termination mapping (termination.rs).**

- [ ] Summary/error-message/termination mapping tests include
      `StalledGeneration`: `error_kind = "stalled_generation"`,
      `ProcessTermination::Aborted`, `GuardContext` cluster populated, and
      the helper message equals the summary message (covered by the extended
      exhaustive test from Phase 3 — confirm it is green here).

**Validation checkpoint.**

- [ ] `just test` passes in the `claudine` package area (nextest, L1).
- [ ] `just lint` passes (clippy clean, no new warnings).

---

## Phase 6 — Documentation

**Goal.** Update every user-facing and skill surface named in the spec's
"Documentation updates" section so the "exactly two timeout rules" wording
does not become stale and the new `error_kind` is discoverable. The
backstop is documented as an **OpenCode stalled-generation guard**, not a
third general timeout.

**Can be drafted in parallel** once Phases 1–3 land (needs final
`error_kind` + termination-table rows); finalize after Phase 4 confirms the
CLI/env names.

**Files.**
- `claudine/docs/topics/timeouts.md`
- `.claude/skills/claudine/opencode-event-sources.md`
- `.claude/skills/claudine/SKILL.md`
- `.claude/skills/claudine/timeline.md`
- CLI/frontmatter reference (skill `cli-reference.md`) + shell-completion metadata

**Tasks.**

- [ ] `timeouts.md`: preserve the "exactly two general timeout-rule" contract
      and the two-env-var (`CLAUDINE_TIMEOUT` / `CLAUDINE_STEP_TIMEOUT`)
      wording. Add a short **OpenCode stalled-generation backstop** subsection
      near the content-guards / OpenCode-variant discussion. Add
      `stalled_generation` to the `Aborted` failure-event table (the row that
      lists `exit_expression` / `runaway_repetition` / `runaway_volume` /
      `repeated_stream_error`). State the two-condition trip, the `10m`
      default, `CLAUDINE_OPENCODE_STALL_TIMEOUT`, `--stall-timeout`,
      `0s`-disables, and the `MAX_GENERATIONS_WITHOUT_PROGRESS = 4` constant.
      Note explicitly that it is OpenCode-scoped and that non-OpenCode runs
      accept `stall_timeout` as inert config.
- [ ] `opencode-event-sources.md`: document the new `LlmCall` retry-churn
      counter next to the `RepeatedStreamError` backstop (same "Repeated-
      stream-error backstop" section neighborhood). Cover: what is counted
      (`LlmCall` with `is_stream == true`), the two conditions, the reset
      taxonomy (genuine step advance / step exit / subagent lifecycle reset;
      deduped step loops, `http_response`, `permission_evaluated`, `service=bus`
      do not), and the `error_kind = "stalled_generation"` /
      `ProcessTermination::Aborted` routing.
- [ ] `.claude/skills/claudine/SKILL.md`: mention the new OpenCode
      live-but-dead guard and its `error_kind` in the timeout/runaway
      discussion (near the existing runaway content-guards bullet).
- [ ] `.claude/skills/claudine/timeline.md`: add a dated entry for the
      stalled-generation backstop.
- [ ] CLI/frontmatter reference (`cli-reference.md`) and shell-completion
      metadata: add `stall_timeout` / `--stall-timeout` with the OpenCode-only
      note and the `CLAUDINE_OPENCODE_STALL_TIMEOUT` env default, alongside
      the existing `--step-timeout` / `step_timeout` entries.

**Validation checkpoint.**

- [ ] `just test` still passes (doc changes are inert; if any doc-tests
      assert flag surfaces, they are satisfied by the Phase 4
      `flag_surface.rs` addition).
- [ ] `cargo check -p claudine -p claudine-cli` green.

---

## Final acceptance gate

Re-check against `spec.md` acceptance criteria 1–9 before declaring done:

- [ ] **AC1** — repeated `LlmCall` (≥ `MAX_GENERATIONS_WITHOUT_PROGRESS`) with
      no progress for ≥ `stall_timeout` terminates with
      `EarlyTermination::StalledGeneration`.
- [ ] **AC2** — maps to `ProcessTermination::Aborted`,
      `summary.error_kind = "stalled_generation"`, an `AgentFailure` event,
      and never routes through `handle_timeout:`.
- [ ] **AC3** — the error message includes the generation-attempt count,
      elapsed progress silence, and (when available) session id, step, agent,
      provider id, model id, mode.
- [ ] **AC4** — `guard_context` includes `generation_count` and
      `stall_duration_ms`, plus OpenCode metadata when present, with no prompt
      text or tool payloads leaked.
- [ ] **AC5** — progress-class events reset the count and advance
      `last_progress_at`; liveness-only events and raw bytes do not.
- [ ] **AC6** — long tools producing no `llm_call_start` do not trip the
      guard, even past `stall_timeout`.
- [ ] **AC7** — `RepeatedStreamError` tests still pass; new tests cover the
      no-error/no-progress retry loop.
- [ ] **AC8** — timeouts docs, OpenCode event-source docs, CLI help/reference,
      and frontmatter/completion metadata are updated.
- [ ] **AC9** — `just test` passes in the `claudine` package area; `just lint`
      is clean.

## Out of scope (do not implement)

Per `spec.md`: resuming the dropped generation; changing `step_timeout`'s
clocks or the byte heartbeat; changing `SemanticEvent::is_activity()` or
making all `Info` liveness-only; provider-general generalization; the
usage-cap variant (already covered by `RepeatedStreamError`); a
`stall_timeout_warn` companion; any single-generation dead-air threshold
(Open Question 1 → Option A).
