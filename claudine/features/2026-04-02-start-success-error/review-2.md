# Review: `start` / `success` / `blocked` / `failure`

## Findings

### 1. Harness pre-launch failures can still exit without a terminal lifecycle signal

`run_harness_loop()` still hand-manages `LifecycleRuntimeState` instead of enforcing transitions through a single guard. That leaves several pre-launch error paths able to return with only `start`, or with no terminal lifecycle signal at all.

Problematic edges:

- Re-materialization and harness re-parse use bare `?` returns before launch state is classified: `claudine/cli/src/commands/wrap/mod.rs:2507-2529`
- `provider_launch_started` is flipped to `true` before snapshot capture, launch-plan construction, or child spawn: `claudine/cli/src/commands/wrap/mod.rs:2727-2739`
- Snapshot and launch-plan construction then use bare `?` returns, so a failure there exits after `start` with no `blocked`: `claudine/cli/src/commands/wrap/mod.rs:2734-2758`
- `execute_harness_attempt(...)` is also a bare `?`, so a spawn/startup failure inside `run_child_stream` / `run_child_capture` likewise exits without a terminal lifecycle signal: `claudine/cli/src/commands/wrap/mod.rs:2760-2781`, `claudine/cli/src/commands/wrap/exec.rs:260-303`, `claudine/cli/src/commands/wrap/exec.rs:575-640`, `claudine/cli/src/commands/wrap/exec.rs:693-760`

Why this matters:

- The tech design explicitly defines `blocked` as the terminal state when the provider never launched.
- The current harness path can misclassify pre-launch failures as post-launch, and in several branches it emits neither `blocked` nor `failure`.

Suggested fix:

- Do not set `provider_launch_started` until the child process has actually been spawned successfully.
- Wrap every fallible pre-launch step after `start` in a helper that emits `blocked` exactly once when launch has not yet begun.
- Replace the manual state handling in `run_harness_loop()` with a dedicated lifecycle guard/state machine so these transitions are enforced mechanically instead of branch-by-branch.

### 2. Non-harness `compose` / `inline-compose` can emit `start` and then return without `blocked` or `failure`

The non-harness paths emit `start` and then immediately `?` the direct execution helpers:

- Inline path: `claudine/cli/src/commands/wrap/composition.rs:635-671`
- Direct path: `claudine/cli/src/commands/wrap/composition.rs:672-699`

If `execute_inline_without_harness(...)` or `execute_direct_without_harness(...)` fails before the provider actually starts, the command returns early and never emits `blocked` or `failure`:

- Inline execution: `claudine/cli/src/commands/wrap/composition.rs:709-945`
- Direct execution: `claudine/cli/src/commands/wrap/composition.rs:1200-1288`

Why this matters:

- The design’s blocked-vs-failure contract depends on tracking whether launch actually began.
- The current non-harness path has no launch-state tracking at all, so pre-launch execution errors can produce `start` as the only lifecycle output.

Suggested fix:

- Give the non-harness path the same launch-state tracking as harness mode.
- Treat execution-helper errors as `blocked` until child spawn is confirmed, then as `failure` afterward.
- Use the same lifecycle abstraction in both harness and non-harness flows so the terminal-state rules stay identical.

## Coverage Gaps

### 1. The wrapper integration points are still lightly tested

I could not find lifecycle-focused assertions in the current `claudine/cli/tests/wrap_commands.rs`, and that is where the broken behavior above would need to be caught.

Missing integration coverage that would matter most:

- Non-harness spawn/setup failures should emit `blocked`, not just `start`
- Harness re-materialization / re-parse failure before first launch should emit `blocked`
- Harness snapshot failure before first launch should emit `blocked`
- Harness launch-plan / child-spawn failure before first launch should emit `blocked`
- Inline success/failure coverage, including closure failure and post-check failure
- Retry / redirect / resume paths should prove `start` is emitted once and `blocked` is suppressed after a real launch

### 2. `emit_lifecycle_signal()` itself has almost no direct behavioral tests

The unit tests in `claudine/lib/src/composition/lifecycle.rs:458-731` cover parsing and `audio_phases()`, but they stop before the actual emitter behavior.

Missing unit coverage:

- Non-audio fan-out happens before audio
- `speak_first + effect` ordering at the emitter boundary
- `message` dispatch on a configured route
- Failure isolation when TTS, sound playback, or messaging send fails

The only direct `execute_resolved_message(...)` tests I found are no-op cases in `claudine/lib/src/messaging/send.rs:449-465`.

## Ergonomics / Performance

### 1. The central lifecycle guard from the design still has not landed

The tech design called for one explicit lifecycle state tracker and one emission contract. The current implementation still duplicates that logic across:

- `claudine/cli/src/commands/wrap/composition.rs`
- `claudine/cli/src/commands/wrap/mod.rs`

That duplication is the main reason the pre-launch holes above still exist. A dedicated `LifecycleGuard` / `LifecycleOutcomeClassifier` would improve ergonomics and correctness at the same time.

### 2. Runtime config is loaded even when no lifecycle notifications are configured

Lifecycle runtime config is loaded unconditionally in `claudine/cli/src/commands/wrap/composition.rs:461-483`, even when `request.prepared.lifecycle.is_empty()`.

That work can be skipped entirely for the common case where the composition does not define `start` / `success` / `blocked` / `failure`.

### 3. `emit_lifecycle_signal()` eagerly builds TTS config even when nothing will be spoken

`emit_lifecycle_signal()` computes `tts_config_from_settings(...)` before iterating audio phases: `claudine/lib/src/composition/lifecycle.rs:445-454`.

That is small, but unnecessary for pure `stderr` / `message` / `effect` notifications. Building the TTS config lazily only when a `Speak` phase is encountered would be a simple cleanup.
