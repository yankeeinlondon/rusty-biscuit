# Lifecycle Guard + Review Fixes

Implements all suggestions from `claudine/features/2026-04-02-start-success-error/review.md`.

## Problem

The lifecycle signal implementation (`start`/`success`/`blocked`/`failure`) has two correctness bugs, missing test coverage, and a scattered state machine that makes the bugs easy to reintroduce:

1. **Pre-launch re-parse gap**: `run_harness_loop()` re-materializes and re-parses the harness plan on every retry/redirect/resume. Those calls use bare `?` returns — if they fail before the first provider launch, the run exits without emitting `blocked`.
2. **Premature `provider_launch_started`**: The flag is set before `capture_pre_run_snapshot`, `build_harness_launch`, and `execute_harness_attempt` — all of which can fail. A spawn failure after `start` emits no terminal signal.
3. **No integration tests** for lifecycle signals in the wrapper.
4. **Emitter untestable** — `emit_lifecycle_signal()` directly hits stderr, messaging, sound effects, and TTS.
5. **TTS config built unconditionally** even when no speech phase exists.
6. **Documentation** doesn't mention lifecycle properties.

## Approach: LifecycleRunGuard with Injectable Emitter

Centralize lifecycle state transitions into a guard struct that mechanically enforces the contract. Make the emitter injectable via a trait so tests can observe ordering and failure isolation without real side effects.

## Design

### 1. LifecycleEmitter Trait

New trait in `claudine/lib/src/composition/lifecycle.rs`:

```rust
pub trait LifecycleEmitter {
    fn emit_stderr(&self, signal: LifecycleSignal, text: &str, term: &Terminal);
    fn emit_message(
        &self,
        text: &str,
        source_path: &Path,
        repo_root: Option<&Path>,
        messaging: &RuntimeMessagingSettings,
    );
    fn emit_speech(&self, text: &str, tts_config: TtsConfig);
    fn emit_effect(&self, name: &str);
}
```

`DefaultLifecycleEmitter` implements this with the current real side effects (stderr via `Status`, messaging via `execute_resolved_message`, TTS via `speak_blocking`, effects via `play_effect_blocking`).

`RecordingEmitter` (test-only, `#[cfg(test)]`) captures a `Vec<EmittedAction>` for assertions:

```rust
#[cfg(test)]
#[derive(Debug, Clone, PartialEq)]
enum EmittedAction {
    Stderr { signal: LifecycleSignal, text: String },
    Message { text: String },
    Speech { text: String },
    Effect { name: String },
}
```

### 2. LifecycleRunGuard

```rust
pub struct LifecycleRunGuard<'a> {
    config: &'a LifecycleConfig,
    ctx: &'a LifecycleRuntimeContext<'a>,
    emitter: &'a dyn LifecycleEmitter,
    state: LifecycleRuntimeState,
}
```

Methods:

- **`emit_start_once()`** — emits `Start` if `!state.start_emitted`, sets `start_emitted = true`.
- **`mark_provider_launched()`** — sets `provider_launch_started = true`. Called only after the child process has actually been spawned.
- **`emit_blocked_or_err(&self, err: eyre::Report) -> eyre::Report`** — if `!provider_launch_started`, emits `Blocked` then returns the error. Otherwise returns the error unchanged.
- **`emit_terminal_outcome(&self, exit_code: i32)`** — emits `Success` if exit_code == 0, `Failure` otherwise.
- **`emit_failure()`** — emits `Failure` unconditionally (for interrupts, handler exhaustion).
- **`provider_launched(&self) -> bool`** — read access to `provider_launch_started`.
- **`start_emitted(&self) -> bool`** — read access to `start_emitted`.

Internal method `emit_signal(&self, signal: LifecycleSignal)` replaces the current free function, delegating to the emitter trait methods.

The existing `emit_lifecycle_signal` free function is removed. All callers migrate to guard methods.

### 3. Bug Fix: Pre-launch Re-parse Failures

In `run_harness_loop()` (`mod.rs:2265-2277`), wrap `materialize_harness_prompt` and `parse_harness_plan` with `guard.emit_blocked_or_err()`:

```rust
let materialized = match materialize_harness_prompt(prompt_state, repo_root) {
    Ok(m) => m,
    Err(e) => return Err(guard.emit_blocked_or_err(e.into())),
};
let mut plan = match claudine::harness::parse_harness_plan(...) {
    Ok(p) => p,
    Err(e) => return Err(guard.emit_blocked_or_err(e.into())),
};
```

`emit_blocked_or_err` checks `!provider_launch_started` internally — first iteration emits `blocked`; recovered retries after launch propagate the error only.

### 4. Bug Fix: Premature `provider_launch_started`

In `run_harness_loop()` (`mod.rs:2464-2506`), move `mark_provider_launched()` to after `execute_harness_attempt` returns `Ok`:

```rust
guard.emit_start_once();
let snapshot = claudine::harness::capture_pre_run_snapshot(&plan)?;
let launch = build_harness_launch(...)?;
let outcome = execute_harness_attempt(...)?;
guard.mark_provider_launched();  // NOW the child actually ran
```

For non-harness paths (`composition.rs:631, 668`), `emit_start_once()` is called before execution and `mark_provider_launched()` immediately after, since there is no retry loop.

### 5. Lazy TTS Config

In the internal `emit_signal` method, build `tts_config_from_settings()` only when a `Speak` audio phase is encountered:

```rust
for phase in phases {
    match phase {
        AudioPhase::Speak(text) => {
            let tts_config = tts_config_from_settings(ctx.settings.tts.as_ref());
            self.emitter.emit_speech(&text, tts_config);
        }
        AudioPhase::Effect(name) => self.emitter.emit_effect(&name),
    }
}
```

### 6. Caller Migration

**`composition.rs` changes:**

- Remove `LifecycleRuntimeState` creation; create `LifecycleRunGuard` instead
- Initial harness parse/preflight failures: replace `emit_lifecycle_signal(... Blocked ...)` with `guard.emit_blocked_or_err(e)`
- Non-harness inline path: `guard.emit_start_once()` + `guard.mark_provider_launched()` before/after execution, `guard.emit_terminal_outcome(exit_code)` after
- Non-harness direct path: same pattern
- Pass `&mut guard` to `run_harness_loop` instead of separate `lifecycle`, `lifecycle_state`, `lifecycle_ctx` params

**`mod.rs` changes:**

- `run_harness_loop` signature: replace `lifecycle: &LifecycleConfig, lifecycle_state: &mut LifecycleRuntimeState, lifecycle_ctx: &LifecycleRuntimeContext` with `guard: &mut LifecycleRunGuard`
- All `emit_lifecycle_signal(lifecycle, signal, lifecycle_ctx)` calls become `guard.emit_*()` calls
- All `lifecycle_state.provider_launch_started` checks become `guard.provider_launched()`
- Pre-launch re-parse: wrap with `guard.emit_blocked_or_err()`
- Launch section: `guard.emit_start_once()` before, `guard.mark_provider_launched()` after `execute_harness_attempt`

### 7. Tests

**7a. Emitter unit tests** (in `lifecycle.rs`, using `RecordingEmitter`):

- `guard_emits_start_once` — multiple `emit_start_once()` calls produce one `Start`
- `guard_blocked_before_launch` — `emit_blocked_or_err()` emits `Blocked` when `!provider_launched()`
- `guard_no_blocked_after_launch` — `emit_blocked_or_err()` does NOT emit after `mark_provider_launched()`
- `guard_terminal_outcome_success` — `emit_terminal_outcome(0)` emits `Success`
- `guard_terminal_outcome_failure` — `emit_terminal_outcome(1)` emits `Failure`
- `guard_emit_failure` — `emit_failure()` emits `Failure`
- `non_audio_before_audio` — stderr and message fire before speech/effect
- `speak_first_ordering` — `speak_first + effect` → speech then effect
- `speak_ordering` — `speak + effect` → effect then speech

**7b. Integration tests** (in `wrap_commands.rs`):

- `lifecycle_start_stderr_before_compose` — `start.stderr` text appears in stderr
- `lifecycle_success_stderr_after_compose` — `success.stderr` text appears after successful run
- `lifecycle_success_stderr_after_inline` — `success.stderr` after inline closure
- `lifecycle_blocked_stderr_on_parse_failure` — `blocked.stderr` on pre-launch failure
- `lifecycle_failure_stderr_on_agent_error` — `failure.stderr` on exit code != 0
- `lifecycle_start_emitted_once_across_retries` — `start` text appears exactly once
- `lifecycle_retry_before_launch_no_blocked` — handler recovery suppresses `blocked`
- `lifecycle_retry_after_launch_emits_failure` — post-launch failure → `failure` not `blocked`

### 8. Documentation

Add "Lifecycle Notifications" section to `claudine/docs/topics/composition.md` between "Shell Policy" and "Retired Interfaces":

- Signal table (start/success/blocked/failure with timing)
- Channel descriptions (stderr, message, speak, speak_first, effect)
- Example frontmatter
- Timing semantics subsection

## Files Modified

| File | Change |
|------|--------|
| `claudine/lib/src/composition/lifecycle.rs` | Add `LifecycleEmitter` trait, `DefaultLifecycleEmitter`, `LifecycleRunGuard`, `RecordingEmitter` (test), unit tests; lazy TTS; remove free `emit_lifecycle_signal` |
| `claudine/cli/src/commands/wrap/composition.rs` | Create guard, migrate all signal emission to guard methods, pass guard to `run_harness_loop` |
| `claudine/cli/src/commands/wrap/mod.rs` | Accept guard param, wrap re-parse failures, move `mark_provider_launched` after spawn, migrate all emission calls |
| `claudine/cli/tests/wrap_commands.rs` | Add integration tests for lifecycle stderr signals |
| `claudine/docs/topics/composition.md` | Add Lifecycle Notifications section |

## Non-Goals

- Event sourcing or persistent lifecycle logs
- Making audio channels fallible (they already warn-and-continue)
- Changing the existing `LifecycleConfig` / `LifecycleNotification` types
