---
agent: open_code/zai-coding-plan/glm-5.2
phases: 7
created: 2026-06-19
start_phase: 1
yolo: "true"
source_files_during_phase_1:
  - claudine/lib/src/harness/model.rs
  - claudine/lib/src/stream/logs/opencode/reasoning.rs
  - claudine/lib/src/harness/handlers.rs
  - claudine/lib/src/harness/runtime.rs
  - claudine/lib/src/harness/validate/mod.rs
  - claudine/cli/src/commands/wrap/exec/termination.rs
  - claudine/cli/src/commands/wrap/harness_orch/attempt.rs
  - claudine/cli/src/commands/wrap/harness_orch/loop_control.rs
docs_updated_during_phase_1: []
docs_created_during_phase_1: []
skills_files_updated_during_phase_1: []
source_files_during_phase_2:
  - claudine/lib/src/runaway/mod.rs
  - claudine/lib/src/runaway/detector.rs
  - claudine/lib/src/runaway/patterns.rs
  - claudine/lib/src/lib.rs
docs_updated_during_phase_2: []
docs_created_during_phase_2: []
skills_files_updated_during_phase_2: []
source_files_during_phase_3:
  - claudine/lib/src/runaway/config.rs
  - claudine/lib/src/runaway/mod.rs
  - claudine/lib/src/runaway/patterns.rs
  - claudine/lib/src/config/claudine_config.rs
  - claudine/lib/src/config/merge.rs
  - claudine/lib/src/dispatch/runner/mod.rs
  - claudine/lib/src/dispatch/runner/speak.rs
  - claudine/cli/src/commands/init/mod.rs
  - claudine/cli/src/commands/init_wizard.rs
docs_updated_during_phase_3: []
docs_created_during_phase_3: []
skills_files_updated_during_phase_3: []
source_files_during_phase_4:
  - claudine/cli/src/commands/wrap/exec/termination.rs
  - claudine/cli/src/commands/wrap/exec/spawn.rs
docs_updated_during_phase_4: []
docs_created_during_phase_4: []
skills_files_updated_during_phase_4: []
source_files_during_phase_5:
  - claudine/cli/src/commands/wrap/exec/spawn.rs
  - claudine/cli/src/commands/wrap/exec/termination.rs
  - claudine/cli/src/commands/wrap/exec/timeouts.rs
  - claudine/cli/src/commands/wrap/exec/watchdog/mod.rs
  - claudine/cli/src/commands/wrap/exec/watchdog/spawn.rs
  - claudine/cli/src/commands/wrap/harness_orch/attempt.rs
  - claudine/cli/src/commands/wrap/wrapper_stages.rs
  - claudine/cli/Cargo.toml
  - biscuit-tui/lib/src/core/standalone/mod.rs
docs_updated_during_phase_5: []
docs_created_during_phase_5: []
skills_files_updated_during_phase_5: []
packages_during_phase_5:
  - claudine-cli
  - biscuit-tui
source_files_during_phase_6:
  - claudine/lib/src/runaway/mod.rs
  - claudine/cli/src/commands/wrap/mod.rs
  - claudine/cli/src/commands/wrap/runaway_guard.rs
  - claudine/cli/src/commands/wrap/policy.rs
  - claudine/cli/src/commands/wrap/wrapper_exec.rs
  - claudine/cli/src/commands/wrap/exec/mod.rs
  - claudine/cli/src/commands/wrap/exec/spawn.rs
  - claudine/cli/src/commands/wrap/exec/termination.rs
  - claudine/cli/src/commands/wrap/exec/wiring/session.rs
  - claudine/cli/src/commands/wrap/live_semantic_sink/mod.rs
  - claudine/cli/src/commands/wrap/live_semantic_sink/event_sink.rs
  - claudine/cli/src/commands/wrap/live_semantic_sink/tests/content_guard.rs
  - claudine/cli/src/commands/wrap/harness_orch/attempt.rs
docs_updated_during_phase_6: []
docs_created_during_phase_6: []
skills_files_updated_during_phase_6: []
packages_during_phase_6:
  - claudine
  - claudine-cli
source_files_during_phase_7:
  - claudine/lib/src/harness/handlers.rs
  - claudine/lib/tests/runaway_handler_payload.rs
docs_updated_during_phase_7:
  - claudine/docs/topics/timeouts.md
  - claudine/docs/topics/signal-handling.md
docs_created_during_phase_7: []
skills_files_updated_during_phase_7:
  - .claude/skills/claudine/SKILL.md
packages_during_phase_7:
  - claudine
source_code:
  - claudine/lib/src/harness/model.rs
  - claudine/lib/src/stream/logs/opencode/reasoning.rs
  - claudine/lib/src/harness/handlers.rs
  - claudine/lib/src/runaway/mod.rs
  - claudine/lib/src/runaway/detector.rs
  - claudine/lib/src/runaway/patterns.rs
  - claudine/lib/src/runaway/config.rs
  - claudine/lib/src/lib.rs
  - claudine/lib/src/config/claudine_config.rs
  - claudine/cli/src/commands/wrap/exec/termination.rs
  - claudine/cli/src/commands/wrap/exec/spawn.rs
  - claudine/cli/src/commands/wrap/exec/timeouts.rs
  - claudine/cli/src/commands/wrap/live_semantic_sink/mod.rs
  - claudine/cli/src/commands/wrap/live_semantic_sink/event_sink.rs
  - claudine/cli/src/commands/wrap/harness_orch/attempt.rs
  - claudine/cli/src/commands/wrap/harness_orch/launch.rs
  - claudine/cli/src/commands/wrap/harness_orch/types.rs
  - claudine/cli/Cargo.toml
documentation:
  - claudine/docs/topics/timeouts.md
  - claudine/docs/topics/signal-handling.md
  - .claude/skills/claudine/SKILL.md
packages:
  - claudine
  - claudine-cli
---

# Runaway-Output Guards + Ctrl+C Hardening — Execution Plan

Converts [`spec.md`](spec.md) into a high-confidence, dependency-ordered
implementation plan. Every task is observable through a test, a compile, or a
behavioral checkpoint. The [Decisions Log](spec.md#decisions-log) in the spec
is the authoritative design source — every open question (Q1–Q17, Q14b) is
resolved there, so this plan does **not** re-litigate any decision; it orders
the work and makes each decision concrete at the file/function level.

## Context (one-paragraph recap)

A non-interactive `claudine compose` run (OpenCode, Kimi K2.7) entered a tight
token-level repetition loop that streamed degenerate output indefinitely and
could not be stopped with Ctrl+C. Claudine had no content-driven defense:
the wall-clock `timeout` is opt-in, the silence-based `step_timeout` cannot
fire against a flooding child, and no output-volume / repetition / exit-pattern
guard exists. Separately, the Ctrl+C path has a structural gap —
`run_child_capture` and the direct `run_child` wait via
`exec/timeouts.rs::wait_with_timeout`, which installs **no SIGINT handler**
and kills the bare child PID (not the process group), so a configured
`timeout` makes Ctrl+C silently ineffective on the capture path.

This plan adds **three** content guards (exit expressions, a repetition
heuristic, a volume-cap backstop) and **hardens Ctrl+C** by retiring
`wait_with_timeout` and routing every spawn path through the one signal-aware
wait loop that already owns SIGTERM→SIGKILL escalation against the child
process group — with real Windows parity.

## Resolved design decisions that shape this plan

All decisions are in the spec's Decisions Log. The handful that most
constrain the task order are restated here so implementers do not have to
context-switch:

1. **Hybrid detector placement (A1).** A pure, stateful, line-assembling
   detector in `claudine/lib` with a `feed(&str) -> Option<Trip>` surface +
   `flush()`, driven from the CLI wiring point that observes the text. The
   caller owns provider/model scope selection and the termination channel.
   → This makes the detector **independent** of the termination types, so the
   detector phase parallelizes with the termination-types phase.

2. **Scan `OutputText` + `Reasoning`, never tool payloads (A2).** The detector
   hooks the typed `SemanticEvent` text path, not the raw byte stream and not
   tool input/result JSON.

3. **One termination path for all three guards + Ctrl+C (Cluster C + D).** All
   trips flow through the existing `EarlyTermination` channel →
   `wait_with_signal_and_early_termination` → SIGTERM→SIGKILL against `-pid`.
   Decision D retires `wait_with_timeout` and routes `run_child` +
   `run_child_capture` through the same loop, making "a path that forgot
   Ctrl+C" structurally impossible.

4. **`ProcessTermination::Aborted` is the shared label (C3).** All three guards
   map to one new variant → `FailureEvent::AgentFailure` (fires `failure:`,
   fail-fast, **never** the `handle_timeout:` retry path, **never**
   `Interrupted`). `error_kind` stays honest per-guard in logs/metrics.

5. **Repetition on by default; exit-expressions empty by default; volume cap
   on by default (F1/F2).** Each has a config kill-switch. Defaults are
   conservative (high thresholds) because a wrongful kill is the worst
   outcome.

### Verified facts driving the plan

- `EarlyTermination` (lib, `stream/logs/opencode/reasoning.rs:73`) has exactly
  three variants (`RateLimit`, `Timeout`, `StepTimeout`) and is matched
  exhaustively in three CLI sites: `apply_early_termination_to_summary`,
  `early_termination_process_outcome` (`exec/termination.rs`), and the
  post-wait match at `exec/spawn.rs:902`. **Adding variants is a forced
  compile-break** that gates correct handling — desirable.
- `run_child_stream_semantic` (`exec/spawn.rs:554`) **already** uses the
  unified `wait_with_signal_and_early_termination` loop and already owns an
  `early_terminate_rx` channel (from the stderr bridge, `:769`). So the
  content guards can be wired into the streaming path **without** waiting for
  the wait-path unification.
- `run_child` (`exec/spawn.rs:67`) and `run_child_capture` (`:375`) do **not**
  use the unified loop — they branch on `timeout` between
  `wait_with_timeout` and `wait_with_signal_handling` (`:246` and `:506`).
  These are the two sites Decision D unifies.
- `run_child_capture`'s reader threads accumulate stdout/stderr into
  unbounded `String`s (`:438–458`, `:471–491`) — the real memory exposure the
  volume cap bounds.
- `classify_failure` (`harness/handlers.rs:94`) and `AttemptOutcome`
  (`harness/model.rs:296`) live in the lib. `AttemptOutcome` carries
  `termination` but **not** `error_kind` today (C3a closes this gap).
- The provider/model identity is known at `harness_orch/launch.rs:50`
  (`resolve_timeouts(...).with_provider(provider)`) and from
  `SemanticEvent::SessionStart { model }` — the inputs the detector needs for
  scope selection.
- `windows = "0.62"` is the workspace convention (`biscuit-location`, `sniff`,
  `playa` all use it). `signal-hook` + `libc` are already in `claudine-cli`.
  Windows is **not** currently a claudine-cli dependency — Phase 5 adds it as
  a target-specific dep.
- The `protect` service (`protect/config.rs`) is the house precedent for
  array-or-object untagged deserialization and regex validation at
  config-load (`ProtectConfig::validate`).

### The surgical insight that shapes this plan

The termination infrastructure already exists and already does the right
thing on the streaming path. The work is: (a) add types, (b) add a pure
detector, (c) add config, (d) connect them, and (e) extend the *good* wait
loop to the two paths that don't use it yet. No new kill mechanism is
invented — every guard converges on the existing SIGTERM→SIGKILL plumbing.

---

## Phase 1 — Termination & failure types (lib foundation)

**Goal:** the library knows about the three new trip reasons and the new
`Aborted` termination, with honest `error_kind`s and correct
`classify_failure` routing. After this phase the types exist and round-trip;
nothing reads a stream yet.

**Dependency:** none (foundation). **Parallelizable with:** Phase 2, Phase 3.

**Files touched (expected):**

- `claudine/lib/src/stream/logs/opencode/reasoning.rs` — three new
  `EarlyTermination` variants.
- `claudine/lib/src/harness/model.rs` — `ProcessTermination::Aborted`;
  `AttemptOutcome` gains `error_kind` + a guard-context blob.
- `claudine/lib/src/harness/handlers.rs` — `classify_failure` maps `Aborted`.

### Tasks

- [x] **1.1 Add three `EarlyTermination` variants.**
  In `stream/logs/opencode/reasoning.rs` extend the enum (currently
  `RateLimit` / `Timeout` / `StepTimeout`) with:
  - `ExitExpression { pattern: String, scope: Option<String> }` —
    `error_kind = "exit_expression"`;
  - `RunawayRepetition { cycle_len: usize, repeats: usize }` —
    `error_kind = "runaway_repetition"`;
  - `RunawayVolume { lines: u64, bytes: u64 }` —
    `error_kind = "runaway_volume"`.
  Document each variant with a rustdoc `## Notes` paragraph naming its
  `error_kind` and that it maps to `ProcessTermination::Aborted` (no `# H1`
  per repo rustdoc policy). Keep `#[derive(Debug, Clone, PartialEq, Eq)]`.

- [x] **1.2 Add `ProcessTermination::Aborted`.**
  In `harness/model.rs` add variant `Aborted` to the `ProcessTermination`
  enum (serde `snake_case`, already persisted). Update `Display` to render
  `"aborted"`. Add a module-level `pub const CLAUDINE_TERMINATION: &str =
  "aborted";`-style constant only if one already exists for the other
  variants (check the surrounding code — do not introduce a new constant
  family if none exists). `Aborted` must remain forward-compatible (no
  `#[non_exhaustive]` churn beyond what the enum already has).

- [x] **1.3 Extend `AttemptOutcome` with `error_kind` + guard context.**
  In `harness/model.rs` add two fields to `AttemptOutcome`:
  - `pub error_kind: Option<String>` — the honest per-guard label
    (`"exit_expression"` / `"runaway_repetition"` / `"runaway_volume"` /
    `"timeout"` / ...), carried from the stream summary so the failure
    handler payload can read it (C3a). `None` for non-error outcomes.
  - `pub guard_context: Option<GuardContext>` — a small new serializable
    struct (`pattern`, `scope`, `cycle_len`, `repeats`, `lines`, `bytes`)
    where every field is `Option` and at most the cluster relevant to the
    trip is populated. Default `None`.
  Update **every** existing `AttemptOutcome { ... }` literal to set
  `error_kind: None, guard_context: None` (grep for `AttemptOutcome {` —
  the production site is `harness_orch/attempt.rs:327`; test sites in
  `harness/handlers.rs` tests and elsewhere). Define `GuardContext` in
  `model.rs` with `#[derive(Debug, Clone, Default, Serialize, Deserialize)]`.

- [x] **1.4 Route `Aborted` through `classify_failure`.**
  In `harness/handlers.rs::classify_failure` add the arm
  `ProcessTermination::Aborted => Some(FailureEvent::AgentFailure)`. This
  intentionally does **not** reuse `TimedOut` (would trigger
  `handle_timeout:` retry and reproduce the runaway) and does **not** map to
  `None` (would suppress failure handling like `Interrupted`). Add a
  rustdoc `## Notes` line stating both rejections and why.

- [x] **1.5 Carry `error_kind` into `build_agent_failure_context`.**
  Add an `error_kind: Option<String>` parameter (and optional
  `&GuardContext`) to `build_agent_failure_context` so the failure context
  can forward it to the handler payload in Phase 7. Update call sites
  (grep `build_agent_failure_context(`) to pass `None`/`None` for now — the
  real values land in Phase 6 when the streaming path populates
  `AttemptOutcome.error_kind`.

### Phase 1 validation checkpoints

- [x] **VC-1.1 `ProcessTermination::Aborted` unit tests.** `Display` renders
  `"aborted"`; serde round-trips `Aborted` through the snake_case form;
  `classify_failure` on an `Aborted` outcome returns
  `Some(FailureEvent::AgentFailure)` (new test alongside the existing
  `classify_*` tests in `harness/handlers.rs`).
- [x] **VC-1.2 `EarlyTermination` variant parity tests.** Each new variant
  clones/equals cleanly and its rustdoc names the right `error_kind` (light
  smoke test; the summary-mapping behavior is proven in Phase 4).
- [x] **VC-1.3 `AttemptOutcome` compiles across the workspace.**
  `cargo build -p claudine && cargo build -p claudine-cli` both succeed after
  updating every literal. This is the proof the field addition is exhaustive.
- [x] **VC-1.4 Existing termination/handler tests pass unchanged.**
  `cargo nextest run -p claudine` for `harness::` and `stream::logs::` is
  green (the new fields default to `None`/`None`, so semantics are unchanged).

---

## Phase 2 — Pure content detector (lib)

**Goal:** a pure, stateful, line-assembling detector in `claudine/lib` that
takes arbitrary streamed text in and emits a `Trip` out — covering exit
expressions, group-cycle repetition, and volume. Fully unit-testable with no
I/O, including a captured copy of the real runaway.

**Dependency:** none (the detector owns its own `Trip` type). **Parallelizable
with:** Phase 1, Phase 3.

**Files touched (expected):**

- `claudine/lib/src/runaway/mod.rs` — module root, constants, `Trip`, re-exports.
- `claudine/lib/src/runaway/detector.rs` — line assembler + cycle detection +
  volume counter.
- `claudine/lib/src/runaway/patterns.rs` — compiled exit-expression set +
  per-line matching.
- `claudine/lib/src/lib.rs` — `pub mod runaway;`.

### Tasks

- [x] **2.1 Define the `Trip` enum and constants.**
  In `runaway/mod.rs` define:
  ```rust
  pub enum Trip {
      ExitExpression { pattern: String, scope: Option<String> },
      RunawayRepetition { cycle_len: usize, repeats: usize },
      RunawayVolume { lines: u64, bytes: u64 },
  }
  ```
  Constants (Cluster B3): `pub const MAX_REPETITION_ALLOWED: usize = 30;`,
  `pub const MAX_CYCLE_LENGTH: usize = 16;` (so the ring buffer holds
  `2 * MAX_CYCLE_LENGTH` lines), `pub const VOLUME_LINES: u64 = 50_000;`,
  `pub const VOLUME_BYTES: u64 = 32 * 1024 * 1024;`. Mark each with a brief
  `// Cluster B3 / F2` provenance comment and the false-positive rationale.

- [x] **2.2 Implement the compiled exit-expression set.**
  In `runaway/patterns.rs` define `CompiledExitExpressions` (a `Vec` of
  entries, each holding the original `pattern` + `scope`, plus either a
  precompiled `regex::Regex` or a literal substring matcher with an
  `ignore_case` flag). Provide:
  - `compile(entries: &[ExitExpressionEntry]) -> Result<Self, ClaudineError>`
    (compile regexes once — Cluster E wiring note); the `ExitExpressionEntry`
    config type is defined in Phase 3, so accept a small input struct here or
    gate the compile signature on Phase 3 landing (see task 2.6).
  - `matches_line(&self, line: &str) -> Option<(pattern, scope)>` — returns
    the first matching entry's `(pattern, scope)`, `None` otherwise. Literal
    uses `str::contains` (honor `ignore_case` via `to_lowercase` on both
    sides); regex uses `is_match`. Empty pattern set ⇒ always `None`, zero
    per-line cost.

- [x] **2.3 Implement the line-assembling detector core.**
  In `runaway/detector.rs` define `ContentDetector` holding: a partial-line
  buffer (`String`), a `VecDeque<String>` ring of the last
  `2 * MAX_CYCLE_LENGTH` normalized lines, a running per-turn volume
  (`lines: u64`, `bytes: u64`), the compiled exit-expression set, and the
  repetition config (enabled + threshold). Surface:
  ```rust
  pub fn feed(&mut self, chunk: &str) -> Option<Trip>;
  pub fn flush(&mut self) -> Option<Trip>;
  pub fn reset_turn(&mut self);   // called on TurnComplete (F2 per-turn reset)
  ```
  `feed` appends the chunk to the partial buffer, splits on `\n`, and for
  each completed line: (a) normalize (trim trailing whitespace; keep blanks
  as `""` per B3), (b) increment volume, (c) test exit-expressions, (d) push
  to the ring + run cycle detection. Return the **first** `Trip` observed
  and stop further work (a trip is terminal). `flush` processes any trailing
  partial line without a newline.

- [x] **2.4 Implement group-cycle detection (B1/B2).**
  A private `detect_cycle(&self) -> Option<(cycle_len, repeats)>` called
  after each line push. Find the smallest `L` in `1..=MAX_CYCLE_LENGTH` such
  that the last `2L` ring entries are two identical halves (exact equality on
  normalized lines — B2). Count full cycles as
  `consecutive_matching_lines / L`. Return `Some` when the count reaches
  `MAX_REPETITION_ALLOWED`. Single-line spam is the `L = 1` case. Keep the
  scan `O(K * K)` worst case — the ring is bounded at `2K = 32` entries, so
  this is trivially cheap.

- [x] **2.5 Implement volume accounting.**
  In `feed`/`flush`, add the byte length of each completed line (including
  the implicit newline) to `bytes` and increment `lines`. Return
  `Trip::RunawayVolume { lines, bytes }` when either exceeds its threshold.
  `reset_turn` zeroes both counters (the streaming path resets on
  `TurnComplete`). Provide a `check_capture_run(&self, total_bytes: u64)`
  helper (or a separate `CaptureVolumeCap` type) for the capture path's
  per-run cap — see task 6.4.

- [x] **2.6 Decouple from Phase 3 with a local input struct.**
  To keep Phase 2 independently testable, define a minimal
  `pub struct ExitExpressionInput { pub patterns: Vec<String>, pub kind:
  PatternKind, pub ignore_case: bool, pub scope: Option<String> }` in
  `runaway/patterns.rs` and have `compile` accept `&[ExitExpressionInput]`.
  Phase 3's resolved config will map `ExitExpressionEntry →
  ExitExpressionInput` (one-line conversion), so the detector never depends
  on the config crate. This is what lets Phase 2 run before Phase 3.

### Phase 2 validation checkpoints

- [x] **VC-2.1 Captured-runaway trip.** Feed the **exact** 6-line cycle from
  the spec (after the one-time `This is the final listening.` preamble) and
  assert the detector emits `Trip::RunawayRepetition { cycle_len: 6, repeats
  >= 30 }` — not earlier than 30 full cycles (false-positive posture).
- [x] **VC-2.2 Single-line spam trips at L = 1.** Feed `STOP.\n` × 40 and
  assert `RunawayRepetition { cycle_len: 1, repeats: 30 }` exactly at the
  30th cycle.
- [x] **VC-2.3 Blank-line flood trips.** Feed `"\n"` × 40 and assert the
  `L = 1` cycle of `""` trips (B3: blanks are kept, not skipped).
- [x] **VC-2.4 Realistic repetitive-but-legitimate output does NOT trip.**
  Feed a 6-line cycle repeated 10× (well under threshold) + a numbered list
  1..100 + a markdown table — assert `None`. This is the false-positive
  guardrail.
- [x] **VC-2.5 Exit-expression match across chunk boundaries.** Feed
  `"STO"` then `"P.\n"` with a literal `pattern: "STOP."` and assert a trip
  (the line assembler must reassemble before matching — E3d).
- [x] **VC-2.6 Literal `ignore_case` + regex inline flags.** Literal
  `ignore_case: true` matches `"stop."`; regex `(?i)stop\.` matches without
  `ignore_case`. Literal default (case-sensitive) does **not** match
  `"StopS"` against `STOP.` (the metacharacter-surprise test from E3a).
- [x] **VC-2.7 Volume cap trips on acyclic flood.** Feed 50_001 distinct
  lines (no cycle) and assert `Trip::RunawayVolume`; verify `reset_turn`
  zeroes the counter so a long multi-turn run does not accumulate.
- [x] **VC-2.8 Ring buffer stays bounded.** After feeding 100k lines, assert
  the internal ring never exceeds `2 * MAX_CYCLE_LENGTH` (memory invariant).

---

## Phase 3 — Exit-expression + guard config and validation (lib)

**Goal:** declare `exit_expressions` across user/repo/frontmatter with
per-layer combine mode and `scope`, plus repetition/volume kill-switches and
threshold overrides; validate at config-load (unknown agent, invalid regex).

**Dependency:** none strictly (Phase 2's `ExitExpressionInput` is local).
**Parallelizable with:** Phase 1, Phase 2. The bridge that compiles config →
detector lives in Phase 6.

**Files touched (expected):**

- `claudine/lib/src/runaway/config.rs` — `ExitExpressionEntry`, kind/scope,
  layer combine mode, 3-layer resolution pipeline.
- `claudine/lib/src/config/claudine_config.rs` — `ClaudineConfig` +
  `RepoOverrideConfig` fields; `validate()` calls into the new validator.

### Tasks

- [x] **3.1 Define `ExitExpressionEntry` and `PatternKind`.**
  In `runaway/config.rs`:
  ```rust
  pub enum PatternKind { Literal, Regex }   // default Literal (E3a)
  pub struct ExitExpressionEntry {
      pub patterns: Vec<String>,            // 1+ (pattern | patterns — E3b)
      pub kind: PatternKind,
      #[serde(default)] pub ignore_case: bool,
      pub scope: Option<String>,            // E2-scope
  }
  ```
  Implement untagged deserialization so `pattern: "…"` deserializes as a
  one-element `patterns`, and so `exit_expressions` accepts **either** an
  array (default combine mode) or an object `{ mode, rules }` (explicit mode)
  — matching the `ProtectConfig`/`TtsValue` house style (Cluster E1).

- [x] **3.2 Parse `scope` (E2-scope).**
  `pub fn parse_scope(scope: &str) -> (Provider, String)` — split on the
  **first** `/`: first segment is the agent (`Provider`), remainder is the
  model verbatim (models may contain `/`). Absent scope = global (wildcard).
  Add `ScopeSelector` with `matches(provider, model) -> bool` implementing
  the additive rule: a run is checked against the union of global ∪ agent ∪
  agent/model entries whose scope matches.

- [x] **3.3 Implement the 3-layer resolution pipeline (E1).**
  `pub fn resolve_exit_expressions(user, repo, frontmatter) ->
  Vec<ExitExpressionEntry>` following the resolution pipeline in the spec:
  start `effective = user`; repo present → `override` (default) replaces or
  `merge` adds; frontmatter present → `merge` (default) adds or `override`
  replaces. Encode each layer's default mode and the explicit `{ mode, rules }`
  override. The result is the compiled-in set the streaming path receives.

- [x] **3.4 Define scalar guard settings (repetition + volume).**
  A `GuardSettings` struct (last-writer precedence: frontmatter > repo > user
  > built-in, like `timeout`/`step_timeout` — Cluster E1 note):
  - repetition: `enabled: bool` (default `true`), `max_repeats: usize`
    (default `MAX_REPETITION_ALLOWED`), `max_cycle_length: usize` (default
    `MAX_CYCLE_LENGTH`);
  - volume: `enabled: bool` (default `true`), `max_lines: u64` (default
    `VOLUME_LINES`), `max_bytes: u64` (default `VOLUME_BYTES`).
  These do **not** use merge/override — only the list-typed `exit_expressions`
  carries a combine mode (Cluster E1 note).

- [x] **3.5 Wire config fields onto `ClaudineConfig` / `RepoOverrideConfig`.**
  Add `#[serde(default, skip_serializing_if = "...")] pub exit_expressions:
  Vec<ExitExpressionEntry>` and `pub guard_settings: GuardSettings` to
  `ClaudineConfig`. Add the same to `RepoOverrideConfig` (repo may override
  both). Keep `#[serde(deny_unknown_fields)]` on both. The frontmatter layer
  is parsed from the composition document (the composition layer already
  reads frontmatter into JSON); add a helper that extracts
  frontmatter-scoped `exit_expressions` for the resolution pipeline.

- [x] **3.6 Validate at config-load.**
  Extend `ClaudineConfig::validate()` (`config/claudine_config.rs:302`) to:
  - compile every `regex`-kind entry and reject invalid regex with
    `ClaudineError::ConfigValidation` (never mid-stream — E3a);
  - reject any `scope` whose agent segment is not a known `Provider` variant
    (E2 validation rule) with a message naming the unknown agent;
  - reject an empty `patterns` vec per entry.
  This mirrors `ProtectConfig::validate`'s regex pre-compile.

### Phase 3 validation checkpoints

- [x] **VC-3.1 Array + object `exit_expressions` both deserialize.** Array
  form uses the layer default mode; `{ mode: "merge", rules: [...] }` parses
  explicitly. Unknown field on an entry is rejected (`deny_unknown_fields`).
- [x] **VC-3.2 Scope parsing.** `"opencode"` → `(OpenCode, "")`;
  `"opencode/kimi-for-coding/k2p7"` → `(OpenCode, "kimi-for-coding/k2p7")`
  (model keeps its inner `/`); absent → global wildcard.
- [x] **VC-3.3 Resolution pipeline matrix.** Assert each of: repo default
  (`override`) replaces user; repo `merge` adds; frontmatter default
  (`merge`) adds on top of repo; frontmatter `override` replaces all. A
  captured table of input → effective set is the proof.
- [x] **VC-3.4 Config validation rejects.** Invalid regex (`"["`) errors at
  load; unknown agent (`"nonsense"`) errors at load; empty `patterns` errors
  at load. Valid config (all three scopes, both kinds) passes.
- [x] **VC-3.5 Repo + user configs round-trip.** `ClaudineConfig` and
  `RepoOverrideConfig` serialize/deserialize with the new fields; defaults
  preserve today's behavior (empty exit-expressions, guards enabled with
  built-in thresholds).

---

## Phase 4 — Termination-plumbing wiring (cli)

**Goal:** the CLI's wait loop knows how to label and summarize the three new
trips, and a pure `Trip` converts to an `EarlyTermination`. After this phase a
trip sent on the channel flows end-to-end through summary → outcome →
classify, even though nothing sends one yet.

**Dependency:** Phase 1 (variants) + Phase 2 (`Trip`). **Parallelizable
with:** Phase 3, Phase 5 (different files in `exec/`).

**Files touched (expected):**

- `claudine/cli/src/commands/wrap/exec/termination.rs` — extend
  `apply_early_termination_to_summary` + `early_termination_process_outcome`;
  add `Trip → EarlyTermination` conversion.

### Tasks

- [x] **4.1 Summarize the three new variants.**
  Extend `apply_early_termination_to_summary` (`exec/termination.rs:348`)
  with one arm per new variant (the existing exhaustive match will fail to
  compile until you do — that's the gate). Each arm sets `exit_code = 1`,
  `is_error = true`, the honest `error_kind`, and an `error_message` naming
  the detail:
  - `ExitExpression { pattern, scope }` → message naming pattern + scope;
  - `RunawayRepetition { cycle_len, repeats }` → message naming both;
  - `RunawayVolume { lines, bytes }` → message naming both.

- [x] **4.2 Map the three new variants to `Aborted`.**
  Extend `early_termination_process_outcome` (`:409`) so each new variant
  returns `ProcessTermination::Aborted` (not `TimedOut`, not `Completed`).
  This is the routing decision C3 — `classify_failure` then yields
  `AgentFailure`, never the timeout-retry path.

- [x] **4.3 Add `trip_to_early_termination(Trip) -> EarlyTermination`.**
  A pure conversion in `exec/termination.rs` mapping the lib `Trip` to the
  lib `EarlyTermination` (`ExitExpression`/`RunawayRepetition`/`RunawayVolume`
  → the matching variant, fields copied verbatim). This is the single bridge
  between the detector and the termination channel — keeping it here means
  the detector (Phase 2) never imports `EarlyTermination`.

- [x] **4.4 Update the post-wait match in `spawn.rs`.**
  The match at `exec/spawn.rs:902` currently handles only `Timeout |
  StepTimeout`. Add arms (or a wildcard that logs) for the three new variants
  so they do not fall through silently — they need no special spawn-side
  action (the summary was already synthesized in 4.1), but the match must be
  exhaustive or explicitly acknowledge the new variants.

### Phase 4 validation checkpoints

- [x] **VC-4.1 Summary mapping unit tests.** For each new variant, drive
  `apply_early_termination_to_summary` and assert the exact `error_kind`,
  `is_error = true`, and that `error_message` contains the diagnostic token
  (pattern name / `cycle_len` / byte count). Add these alongside the existing
  `apply_early_termination_*` tests in `exec/termination.rs`.
- [x] **VC-4.2 Outcome mapping unit tests.**
  `early_termination_process_outcome` returns `Aborted` for all three new
  variants (extend the existing `early_termination_process_outcome_maps_*`
  tests).
- [x] **VC-4.3 `trip_to_early_termination` round-trips fields.** Each `Trip`
  variant converts to the matching `EarlyTermination` with fields preserved.
- [x] **VC-4.4 `cargo build -p claudine-cli` compiles.** The exhaustive
  matches are the proof all variants are handled.

---

## Phase 5 — Ctrl+C hardening, wait-path unification, and Windows (cli)

**Goal:** retire `wait_with_timeout`; route `run_child` + `run_child_capture`
through the unified `wait_with_signal_and_early_termination` loop; add visible
interrupt feedback and the shortened non-interactive ladder; ship a real
Windows implementation with parity to the Unix group-signal/escalation
behavior.

**Dependency:** Phase 4 (the unified loop carries the content-trip channel).
**Parallelizable with:** Phase 6's *streaming* detector work (different code
path — `run_child_stream_semantic` already uses the unified loop), but
Phase 6's *capture* volume-cap task depends on this phase's capture
unification.

**Files touched (expected):**

- `claudine/cli/src/commands/wrap/exec/timeouts.rs` — retire/gut
  `wait_with_timeout` (keep `TimeoutConfig`, `detect_step_timeout`).
- `claudine/cli/src/commands/wrap/exec/termination.rs` — visible feedback,
  non-interactive ladder, Windows parity in the unified loop.
- `claudine/cli/src/commands/wrap/exec/spawn.rs` — `run_child` +
  `run_child_capture` route through the unified loop.
- `claudine/cli/Cargo.toml` — `windows = "0.62"` target dep.

### Tasks

- [x] **5.1 Fold wall-clock `timeout` into the unified watchdog.**
  The watchdog ticker (`exec/watchdog/`) currently emits `WatchdogTermination`
  for `Timeout` + `StepTimeout`. Extend it (or the wiring in
  `run_child`/`run_child_capture`) so a configured wall-clock `timeout` is
  enforced by the **same** watchdog the streaming path already uses, rather
  than by `wait_with_timeout`. The deadline comes from `TimeoutConfig.timeout`
  already threaded into every call site (`harness_orch/launch.rs:50`). This
  makes the timeout kill group-targeted (`-pid`) and signal-aware everywhere.

- [x] **5.2 Route `run_child` + `run_child_capture` through the unified loop.**
  Replace the `if let Some(seconds) = timeout { wait_with_timeout(...) } else
  { wait_with_signal_handling(...) }` branch at `spawn.rs:246` and `:506` with
  a single call to `wait_with_signal_and_early_termination` (constructing a
  disconnected `early_rx` when no content-trip channel is supplied, as the
  streaming path already does at `spawn.rs:880`, and passing the watchdog
  receiver). Preserve the interactive-TUI passthrough: when
  `child_in_own_pgroup == false` (shared pgroup + TTY inheritance for
  Claude/Codex) the loop must keep letting the terminal deliver SIGINT
  naturally — do **not** isolate that path into its own pgroup (the
  `SIGTTIN`-hang risk the existing code comments call out).

- [x] **5.3 Retire `wait_with_timeout`.**
  Delete the `wait_with_timeout` fn from `exec/timeouts.rs` (both the
  `#[cfg(unix)]` and `#[cfg(not(unix))]` bodies) and its two call sites.
  Keep `TimeoutConfig`, `detect_step_timeout`, `format_internal_duration`,
  and the `parse_env_duration` helpers — only the wait loop goes. Remove its
  tests (the absurd-timeout regression test is now the watchdog's
  responsibility; re-anchor it if the watchdog takes a deadline). Grep for
  any remaining `wait_with_timeout` references and confirm zero.

- [x] **5.4 Add visible interrupt feedback (Q14).**
  In the unix SIGINT handler inside
  `wait_with_signal_and_early_termination` (`exec/termination.rs:87`), on
  each counted press emit a stderr line via `eprintln!` (or the existing
  `Status::Warning` renderer used elsewhere in the wrap command) such as
  `⚠ interrupt received — press again to force-kill`. Keep the handler
  async-signal-safe: the message write must be the only side effect beyond
  the existing atomic + `libc::kill`. Verify the message does not interleave
  with the runaway flood (stderr is unbuffered).

- [x] **5.5 Implement the shortened non-interactive ladder (F5).**
  Thread an `interactive: bool` (or `non_interactive: bool`) flag into the
  unified wait loop. When non-interactive, press 1 → `SIGTERM` directly
  (escalating to `SIGKILL` on a repeat); when interactive, keep the full
  `SIGINT → SIGTERM → SIGKILL` ladder. The flag derives from the existing
  `effective_non_interactive` value already computed in
  `harness_orch/attempt.rs` and from the direct-wrapper path. Plumb it
  through `TimeoutConfig` (add a field) or as an explicit loop argument —
  match whatever the surrounding call convention is.

- [x] **5.6 Implement Windows parity (Q15).**
  Replace the current `#[cfg(not(unix))]` stub of
  `wait_with_signal_and_early_termination` (`exec/termination.rs:246`, which
  today only calls `child.kill()` and has no group targeting and no real
  console-event handling) with a real implementation:
  - spawn children with `CREATE_NEW_PROCESS_GROUP` (this is set at the
    `Command` build site in `spawn.rs` — add a Windows guard mirroring the
    `#[cfg(unix)] command.process_group(0)` block) and/or assign the child
    to a **Job Object** so the whole tree terminates as a unit;
  - register `SetConsoleCtrlHandler` to observe Ctrl+C and deliver
    interrupts to the child group via
    `GenerateConsoleCtrlEvent(CTRL_BREAK_EVENT, process_group_id)` (Ctrl+C
    events cannot target a specific group; Ctrl+Break can);
  - map the escalation ladder: Ctrl+Break (graceful) →
    `TerminateJobObject` / `TerminateProcess` (forceful) — Windows has no
    SIGTERM/SIGKILL split;
  - preserve the grace/reap deadlines so a wedged child does not hang the
    wrapper (mirror the unix `POST_SIGKILL_REAP_TIMEOUT`).
  Add `windows = { version = "0.62", features = [...] }` under
  `[target.'cfg(windows)'.dependencies]` in `claudine/cli/Cargo.toml`,
  selecting only the feature subsets needed
  (`Win32_System_JobObjects`, `Win32_System_Console`,
  `Win32_Foundation`). Match the workspace convention exactly.

### Phase 5 validation checkpoints

- [x] **VC-5.1 `wait_with_timeout` is gone.** `rg wait_with_timeout
  claudine/cli/src` returns zero hits; `cargo build -p claudine-cli`
  succeeds on Unix without it.
- [x] **VC-5.2 Ctrl+C terminates the child on every spawn path (Unix).**
  Using the existing real-process harness patterns, spawn a long-running
  child via each of `run_child`, `run_child_capture`, and
  `run_child_stream_semantic`; send SIGINT to the wrapper; assert the child
  (and, for owned-pgroup paths, its descendants) is reaped and the outcome
  is `Interrupted`. **Crucially: repeat with a wall-clock `timeout`
  configured** — this is the exact scenario the spec calls out as broken
  (opting into the safety timeout disabled Ctrl+C).
- [x] **VC-5.3 Visible feedback appears on each press.** Capture stderr
  during a SIGINT and assert the `interrupt received` line is present (one
  per counted press).
- [x] **VC-5.4 Non-interactive ladder is SIGTERM-first.** A single SIGINT on
  a non-interactive run sends SIGTERM (observable via a child that traps
  SIGTERM); interactive runs still send SIGINT first.
- [x] **VC-5.5 Windows compiles with parity APIs.** `cargo check
  --target x86_64-pc-windows-gnu -p claudine-cli` succeeds (cross-compile
  from macOS). The Windows implementation registers a console handler,
  targets a process group / Job Object, and escalates to
  `TerminateJobObject`. **Document the macOS-host verification gap:**
  runtime Windows behavior must be validated in CI or on a Windows host
  (flagged risk per the all-OS rule).
- [x] **VC-5.6 Regression: existing termination tests pass.**
  `just test` (or `cargo nextest run -p claudine-cli`) for the `exec::`
  modules is green, including the watchdog-disconnection and early-kill
  regression tests already in `exec/termination.rs`.

---

## Phase 6 — Detector + volume wiring into the spawn paths (cli)

**Goal:** the pure detector actually drives termination. On the streaming
path it scans `OutputText` + `Reasoning` with the resolved in-scope pattern
set and trips on exit-expression / repetition / per-turn volume; on the
capture path a per-run volume cap bounds the unbounded buffer. Trips flow
through the channel plumbed in Phases 4–5.

**Dependency:** Phase 2 (detector), Phase 3 (config), Phase 4
(`trip_to_early_termination` + summary mapping). The capture-path volume cap
additionally depends on Phase 5's capture-path unification (so the cap can
send on the termination channel the unified loop polls).

**Files touched (expected):**

- `claudine/cli/src/commands/wrap/harness_orch/attempt.rs` — build the
  compiled in-scope pattern set + `ContentDetector`; populate
  `AttemptOutcome.error_kind`/`guard_context` from the summary.
- `claudine/cli/src/commands/wrap/harness_orch/launch.rs` /
  `types.rs` — carry the resolved guard config + provider/model.
- `claudine/cli/src/commands/wrap/live_semantic_sink/mod.rs` +
  `event_sink.rs` — feed the detector from `OutputText`/`Reasoning`;
  `reset_turn` on `TurnComplete`.
- `claudine/cli/src/commands/wrap/exec/spawn.rs` — wire the detector's
  trip-sender into the termination channel; per-run volume cap on the
  capture reader threads.

### Tasks

- [x] **6.1 Resolve the in-scope pattern set once before streaming.**
  At the wiring point (`harness_orch/attempt.rs` around the sink
  construction, `:85`), call `runaway::resolve_exit_expressions(user, repo,
  frontmatter)` (Phase 3) and filter to the entries whose `scope` matches the
  run's `(provider, model)` (model from `--model` /
  `SemanticEvent::SessionStart`). Compile the result to
  `CompiledExitExpressions` (Phase 2) **once** — never per line (Cluster E
  wiring note). Merge in the resolved `GuardSettings` (repetition/volume
  thresholds + kill-switches).

- [x] **6.2 Construct + drive the `ContentDetector` from the sink.**
  Own the `ContentDetector` (and a trip `Sender`) in the `LiveSemanticSink`
  (or a small companion struct the sink holds). In
  `on_semantic_event(SemanticEvent::OutputText { text, .. })` and
  `Reasoning { text, .. }` call `detector.feed(&text)`; on
  `TurnComplete` call `detector.reset_turn()`. When `feed`/`flush` returns a
  `Trip`, convert via `trip_to_early_termination` (Phase 4) and send it on
  the termination channel exactly once (a trip is terminal — guard against
  double-send). **Never** feed `ToolCall`/`ToolResult` payloads (A2). Keep
  the detector behind the kill-switches: skip construction entirely when
  both repetition and volume are disabled and the exit-expression set is
  empty (zero overhead for runs that opt out).

- [x] **6.3 Wire the trip-sender into the streaming wait loop.**
  The streaming path already builds `early_terminate_rx` from the stderr
  bridge (`spawn.rs:769`). Multiplex the detector's trips onto the same
  receiver the loop polls: either share one `mpsc::Sender<EarlyTermination>`
  (mpsc supports multiple senders) between the stderr bridge and the
  detector, or add a tiny merger. The loop already handles `early_rx`
  (`termination.rs:152`) so no loop change is required — only the sender
  wiring. On trip, the existing SIGTERM→SIGKILL escalation runs unchanged.

- [x] **6.4 Volume cap on the capture path (F3).**
  In `run_child_capture`'s stdout/stderr reader threads (`spawn.rs:438` and
  `:471`), track `captured.len()` (bytes) and a line count; when either
  exceeds the per-run volume threshold, stop appending to the buffer
  (bounding memory) and send an `EarlyTermination::RunawayVolume` on the
  termination channel. This requires the capture path to be on the unified
  loop (Phase 5) so the channel is polled — that is the dependency on Phase
  5. Do **not** add exit-expression or repetition detection to the capture
  path (F3: capture gets Ctrl+C + volume cap only).

- [x] **6.5 Stop feeding the renderer on trip.**
  On a content trip, stop pushing further output to the terminal renderer
  (spec Part 1 step 1) so the tail of the runaway is not echoed. The sink's
  trip-send is the trigger; gate subsequent `OutputText` rendering on a
  "tripped" flag the sink holds.

- [x] **6.6 Populate `AttemptOutcome.error_kind` + `guard_context`.**
  In `harness_orch/attempt.rs` where `AttemptOutcome` is built (`:327`),
  read `summary.error_kind` (set by Phase 4's
  `apply_early_termination_to_summary`) and the guard detail, and populate
  the two new fields added in Phase 1. For the capture path, populate from
  the volume trip if one occurred. This is what makes C3a (threading
  `error_kind` into the handler payload) possible in Phase 7.

### Phase 6 validation checkpoints

- [x] **VC-6.1 Exit-expression trip end-to-end (streaming).** A stub
  provider streaming a line matching an in-scope literal at the correct
  `scope` (global / agent / agent+model) trips, terminates the child, and
  the resulting summary carries `error_kind = "exit_expression"` with the
  pattern + scope in `error_message`. An out-of-scope entry does **not**
  trip.
- [x] **VC-6.2 Repetition trip end-to-end (streaming).** A stub provider
  streaming the captured 6-line cycle trips at the threshold and the summary
  carries `error_kind = "runaway_repetition"`. Realistic output below the
  threshold does not trip (reuse the VC-2.4 fixture).
- [x] **VC-6.3 Volume trip end-to-end (streaming).** A stub provider
  streaming > 50k lines in one turn trips `runaway_volume`; a multi-turn run
  where each turn stays under the cap does **not** trip (per-turn reset).
- [x] **VC-6.4 Capture-path volume cap.** A capture-mode run emitting > 32
  MB trips `runaway_volume`, the captured `String` stays bounded (assert its
  length is near the cap, not unbounded), and the child is reaped. Ctrl+C
  still works on the capture path (VC-5.2 already covers this; re-confirm
  with the cap armed).
- [x] **VC-6.5 `AttemptOutcome.error_kind` is populated.** For each trip
  kind, the outcome built at `attempt.rs:327` carries the matching
  `error_kind` and a populated `guard_context`.
- [x] **VC-6.6 Kill-switch honored.** With repetition disabled in config, the
  6-line cycle does **not** trip; with the exit-expression set empty + both
  guards disabled, the detector is not constructed and behavior is identical
  to today (no regression).
- [x] **VC-6.7 No tool-payload scanning.** A stub provider emitting highly
  repetitive `ToolResult` payloads does **not** trip (A2 — only
  `OutputText`/`Reasoning` are scanned).

---

## Phase 7 — Handler payload, integration tests, and docs

**Goal:** the programmatic failure handler can distinguish a runaway from a
genuine crash and act accordingly (C3a); the full success-criteria matrix is
proven; docs and the claudine skill reflect the new guards and the unified
Ctrl+C story.

**Dependency:** all prior phases.

**Files touched (expected):**

- `claudine/lib/src/harness/handlers.rs` — `CLAUDINE_ERROR_KIND` env +
  guard context in the JSON payload.
- `claudine/lib/src/harness/model.rs` — any `GuardContext` rendering helper
  not already added in Phase 1.
- `claudine/docs/topics/timeouts.md` + `signal-handling.md` — new guards,
  unified wait loop, Windows behavior.
- `.claude/skills/claudine/SKILL.md` — skill catalog update.

### Tasks

- [x] **7.1 Thread `error_kind` + guard context into the handler payload.**
  In `execute_programmatic_handler` (`harness/handlers.rs:153`):
  - add env var `CLAUDINE_ERROR_KIND` (from `failure.outcome.error_kind`,
    falling back to the validation/timeout mapping);
  - add `error_kind` and `guard_context` (pattern/scope, cycle_len/repeats,
    lines/bytes — whichever cluster is populated) to the JSON payload built
    at `:177`, so a handler can branch on `runaway_repetition` vs
    `agent_failure`. This is the C3a requirement that "error handling cannot
    make good decisions without context."

- [x] **7.2 End-to-end integration test: trip → terminate → classify →
  handler payload.** A library-level test (or a `claudine/lib/tests/`
  integration test) that drives a stub provider through the streaming path,
  forces a repetition trip, and asserts: (a) `ProcessTermination::Aborted`,
  (b) `FailureEvent::AgentFailure` from `classify_failure`, (c) the handler
  payload env + JSON carry `error_kind = "runaway_repetition"` and the
  cycle detail. This ties Phases 1 + 4 + 6 + 7.1 together.

- [x] **7.3 Spawn × wait matrix proof.** A test (or a documented matrix in
  `signal-handling.md` if a real-process test is impractical for every cell)
  showing Ctrl+C terminates the child for every spawn-path × wait-loop ×
  timeout-configured combination, on **both Unix and Windows**: {
  `run_child`, `run_child_capture`, `run_child_stream_semantic` } × {
  unified-loop (all, post-Phase-5) } × { no-timeout, with-timeout }. The
  "with-timeout" column is the regression that motivated Part 4.

- [x] **7.4 Update `timeouts.md`.** Document the three new guards
  (exit-expressions, repetition, volume cap), their `error_kind`s, their
  config surfaces + defaults, the conservative false-positive posture, and
  that they all map to `ProcessTermination::Aborted` (not the timeout-retry
  path). Note that wall-clock `timeout` remains opt-in (F4) and that the
  volume cap is the content backstop.

- [x] **7.5 Update `signal-handling.md`.** Document the unified wait loop
  (all spawn paths now route through it), the visible interrupt feedback,
  the shortened non-interactive ladder (F5), and the Windows
  Job-Object/console-event model. Add the new `Aborted` termination to the
  exit-code/label table (distinct from `Interrupted` and `TimedOut`).

- [x] **7.6 Update the claudine skill.** Add a concise note to
  `.claude/skills/claudine/SKILL.md` (and `architecture.md` if it lists the
  termination model) covering the runaway guards and the unified Ctrl+C
  path, linking to the updated topic docs.

### Phase 7 validation checkpoints

- [x] **VC-7.1 Handler payload carries `error_kind`.** A programmatic
  `handle` stub receives `CLAUDINE_ERROR_KIND=runaway_repetition` in its env
  and `error_kind` + `guard_context` in the stdin JSON for a repetition trip;
  it receives `exit_expression` + the pattern for an exit-expression trip.
- [x] **VC-7.2 Full success-criteria matrix green.** Every bullet in the
  spec's [Success Criteria](spec.md#success-criteria) is covered by at least
  one checkpoint in the validation matrix below.
- [x] **VC-7.3 `just test` and `just lint` pass in the claudine package
  area** (the spec's explicit gate). `cargo fmt --check` is read-only-clean
  for touched files (do **not** run `cargo fmt` write mode — repo policy).
- [x] **VC-7.4 Docs + skill updated and internally consistent** with the
  implemented `error_kind` strings, config keys, and default thresholds.

---

## Validation matrix (maps spec success criteria → checkpoints)

| Spec success criterion | Proven by |
|---|---|
| Exit expression (literal/regex, correct `scope`, all 3 layers) terminates + reports `exit_expression` with pattern + scope | VC-2.5, VC-2.6, VC-3.3, VC-6.1, VC-7.1 |
| Synthetic 6-line runaway trips repetition at threshold; realistic repetitive output does not | VC-2.1, VC-2.4, VC-6.2 |
| Acyclic / over-`K` flood + unbounded capture buffer trip the volume cap | VC-2.7, VC-6.3, VC-6.4 |
| All three trips → `Aborted` → `AgentFailure` (fail-fast, never timeout-retry, never `Interrupted`) + thread `error_kind` + context into payload | VC-1.1, VC-1.4, VC-4.2, VC-6.5, VC-7.1, VC-7.2 |
| Matrix proof: Ctrl+C terminates child on every spawn/wait path on Unix + Windows, incl. with `timeout` configured | VC-5.2, VC-5.5, VC-7.3 |
| All new behavior covered by unit tests; signal/Ctrl+C uses real-process harness patterns | VC-1.1–VC-1.4, VC-2.1–VC-2.8, VC-5.2, VC-5.6 |
| `just test` + `just lint` pass in claudine | VC-7.3 |

## Dependency graph & parallelization

```
Phase 1 (types) ──┐
                  │
Phase 2 (detector)┼──┐  (1, 2, 3 are mutually independent)
                  │  │
Phase 3 (config) ─┘  │
                     │
                     ├─→ Phase 4 (plumbing: depends on 1 + 2)
                     │        │
                     │        ├─→ Phase 5 (Ctrl+C/wait unify/Windows: depends on 4)
                     │        │        │
                     │        │        └─→ Phase 6 capture-volume task (needs unified capture loop)
                     │        │
                     │        └─→ Phase 6 streaming wiring (depends on 2, 3, 4; can overlap Phase 5)
                     │                  │
                     └──────────────────┴─→ Phase 7 (handler payload + tests + docs)
```

- **Fully parallel at the start:** Phases 1, 2, 3 touch disjoint lib modules
  (`harness`/`stream-logs`, `runaway/detector`, `runaway/config`+`config`)
  and can proceed concurrently.
- **Critical path:** Phase 1 → Phase 4 → Phase 5 → Phase 6 (capture) →
  Phase 7.
- **Parallel lane:** Phase 3 can land any time before Phase 6's wiring
  compiles the pattern set. Phase 6's *streaming* detector wiring depends on
  Phases 2 + 3 + 4 but **not** on Phase 5 (the streaming path already uses
  the unified loop), so it can overlap Phase 5.
- **Phase 5 internal order is fixed:** fold timeout into the watchdog (5.1)
  before retiring `wait_with_timeout` (5.3); the Windows body (5.6) can be
  developed in parallel with the unix ladder/feedback work (5.4–5.5) since
  they are in separate `#[cfg]` arms.

## Risks & notes

- **Windows runtime verification gap (highest risk).** The dev host is macOS;
  the Windows Job-Object/console-event path (5.6) can be cross-compile-checked
  (`cargo check --target x86_64-pc-windows-gnu`) but not runtime-validated
  locally. It must be exercised in CI or on a Windows host before claiming
  parity. Flagged explicitly in VC-5.5. This is the spec's Q15 known
  verification risk.
- **Channel multiplexing (6.3).** The detector and the stderr bridge both
  want to send `EarlyTermination` to the same loop receiver. mpsc allows
  multiple `Sender` clones, so the cleanest path is a shared sender. If the
  stderr bridge's channel ownership is awkward to share, add a 1-thread
  merger; do **not** add a second receiver to the loop (it only polls one
  `early_rx`).
- **`AttemptOutcome` field addition (1.3) is a broad change.** Every literal
  must set the two new fields. The grep over `AttemptOutcome {` is the
  completeness gate; VC-1.3 (workspace builds) is the proof.
- **`wait_with_timeout` retirement (5.3) is load-bearing.** It is currently
  the only kill path for `run_child`/`run_child_capture` when a timeout is
  configured. Task 5.1 (fold timeout into the watchdog) **must** land before
  5.3 deletes the function, or those paths lose their timeout kill. The
  ordering inside Phase 5 enforces this.
- **False-positive posture is load-bearing for trust.** The conservative
  thresholds (30 cycles, 50k lines, 32 MB) and the exact-equality repetition
  match (B2) are deliberate. Do not "tune" them down without a real
  false-positive incident; VC-2.4 guards the legitimate-output case.
- **No `cargo fmt` write mode** (repo policy, `AGENTS.md`). All edits
  hand-match surrounding style; `cargo fmt --check` is read-only only.
- **Known limitations (documented, out of scope):** multi-line exit-expression
  patterns; fuzzy/near-match repetition; `*/model` (any-agent) scope;
  prefix/glob model matching; full content detection on the capture path
  (it gets Ctrl+C + volume cap only — F3). Each is noted in the spec as a
  deferred enhancement; do not expand scope to cover them.
