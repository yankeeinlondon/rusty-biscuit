# Review: `start` / `success` / `blocked` / `failure`

## Findings

### 1. Pre-launch harness re-materialization / re-parse failures can exit without `blocked`

`execute_composition_request()` emits `blocked` for the initial harness parse/preflight failures in `claudine/cli/src/commands/wrap/composition.rs:456-518`, but `run_harness_loop()` re-materializes and re-parses the harness plan on every retry / redirect / resume iteration in `claudine/cli/src/commands/wrap/mod.rs:2265-2277`.

Those later `materialize_harness_prompt(...)?` / `parse_harness_plan(...)?` calls are bare `?` returns. If a handler changes the source path or frontmatter and the next pre-launch re-parse fails before the first provider launch, the run exits without emitting `blocked`.

Why this matters:

- The design explicitly classifies any terminal pre-launch stop as `blocked`.
- This is most likely to surface in the exact paths the design called out: retry/redirect recovery before first launch.

Suggested fix:

- Wrap the pre-launch `materialize_harness_prompt()` and `parse_harness_plan()` failures inside `run_harness_loop()` the same way the initial parse is wrapped in `execute_composition_request()`.
- Gate the emission on `!lifecycle_state.provider_launch_started` so recovered retries still suppress `blocked`.

### 2. `start` is emitted before the last fallible pre-launch steps, so some runs can produce `start` with no terminal lifecycle signal

The non-harness paths emit `start` immediately in `claudine/cli/src/commands/wrap/composition.rs:630-668`, before `execute_inline_without_harness()` / `execute_direct_without_harness()` do any spawn/stream/capture work. Inside those functions, the actual launch calls still fail with `?` (`claudine/cli/src/commands/wrap/composition.rs:725-756`, `claudine/cli/src/commands/wrap/composition.rs:1217-1227`, `claudine/cli/src/commands/wrap/composition.rs:1264-1276`).

The harness path has the same issue, and it is slightly worse: `run_harness_loop()` sets `start_emitted = true` and `provider_launch_started = true` in `claudine/cli/src/commands/wrap/mod.rs:2464-2469` before `capture_pre_run_snapshot()`, `build_harness_launch()`, and `execute_harness_attempt()` (`claudine/cli/src/commands/wrap/mod.rs:2471-2506`). Those steps still contain fallible pre-launch work, including argument validation and the actual child start (`claudine/cli/src/commands/wrap/mod.rs:1616-1655`, `claudine/cli/src/commands/wrap/mod.rs:1696-1706`, `claudine/cli/src/commands/wrap/mod.rs:1746-1758`).

Result:

- A launch-construction or spawn failure can emit `start` even though the provider never actually launched.
- Because the error bubbles out via `?`, neither `blocked` nor `failure` is emitted afterward.
- In harness mode, the state is also misclassified as "launch started" before that is true.

Suggested fix:

- Move the transition to `provider_launch_started = true` until after the child process has actually been started.
- Prefer a small lifecycle state machine / guard around "about to launch", "launch succeeded", and "terminal outcome" so these edges are not hand-wired in multiple places.

## Coverage Gaps

### 1. Wrapper-level lifecycle behavior is effectively untested

The library tests in `claudine/lib/src/composition/lifecycle.rs:463-730` and `claudine/lib/src/composition/prepare.rs:267-320` cover parsing and `audio_phases()`, but they do not exercise the actual wrapper integration points.

`claudine/cli/tests/wrap_commands.rs` does have nearby harness/compose coverage, for example `claudine/cli/tests/wrap_commands.rs:1737-1771`, but there are no assertions for:

- `start.stderr` before launch
- `success.stderr` after direct compose
- `success.stderr` after inline closure
- `blocked.stderr` on pre-launch terminal failures
- `failure.stderr` on agent exit, closure failure, or post-check failure
- retry-before-launch suppressing `blocked`
- retry-after-launch producing `failure`
- `start` being emitted only once across retries / redirects / resumes

The design asked for this exact integration matrix. It is not here yet.

### 2. The emitter itself is only tested through helpers, not through observable ordering/failure isolation

`emit_lifecycle_signal()` in `claudine/lib/src/composition/lifecycle.rs:414-455` is the critical behavior, but the tests only cover `audio_phases()` and parser validation. The messaging tests in `claudine/lib/src/messaging/send.rs:449-465` only prove no-op cases.

Missing tests:

- non-audio fan-out happens before phase-two audio
- `speak_first + effect` ordering at the emitter boundary, not just in the helper
- resolved message send on an active route
- notification failure isolation (missing secrets, bad audio backend, etc.) without changing the main composition result

## Ergonomics / Performance

### 1. Centralize lifecycle state transitions

Right now the lifecycle contract is scattered across `claudine/cli/src/commands/wrap/composition.rs` and `claudine/cli/src/commands/wrap/mod.rs`. That duplication is exactly why the pre-launch holes above slipped in. A small helper like `LifecycleRunGuard` or `LifecycleOutcomeClassifier` would be more ergonomic and would make the blocked-vs-failure contract mechanically enforceable.

This is both an ergonomics and correctness improvement.

### 2. Make the emitter injectable for tests

`emit_lifecycle_signal()` directly hits stderr, messaging, sound effects, and TTS. That keeps the code simple, but it blocks meaningful unit tests of ordering and failure isolation. An injected executor trait / callback bundle would make the lifecycle behavior testable without real side effects and would eliminate the current reliance on helper-only tests.

This is mostly an ergonomics improvement, but it will also reduce regressions.

### 3. Avoid building TTS config when there is no speech phase

`emit_lifecycle_signal()` computes `tts_config_from_settings(...)` unconditionally in `claudine/lib/src/composition/lifecycle.rs:447-448`, even for pure `stderr` / `message` / `effect` notifications.

This is a small optimization, but it is trivial to avoid by building the config lazily only when a `Speak` phase is encountered.

## Documentation Drift

The tech design called for same-change doc updates, but the user-facing composition docs still do not mention these frontmatter properties. `claudine/docs/topics/composition.md:31-179` documents compose/inline-compose, harness, handlers, and shell policy, but not `start`, `success`, `blocked`, `failure`, `speak_first`, or their timing semantics.

That leaves the feature discoverability/documentation side incomplete even though the code landed.
