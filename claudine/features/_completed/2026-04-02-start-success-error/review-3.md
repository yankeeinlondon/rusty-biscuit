# Review: `start` / `success` / `blocked` / `failure` (Pass 3)

## Findings

### 1. Harness re-materialization / re-parse failures before first launch still skip `blocked`

`run_harness_loop()` now uses `LifecycleRunGuard`, but the retry-loop re-materialization and harness-plan parse still use bare `?` returns before `start` is emitted:

- `claudine/cli/src/commands/wrap/mod.rs:2509-2518`
- `claudine/cli/src/commands/wrap/mod.rs:2520-2531`

Because `LifecycleRunGuard` only auto-emits on `Drop` after `start_emitted == true`, a failure in either of those branches exits with no lifecycle terminal signal at all on the first pre-launch attempt.

Why this still matters:

- The tech design classifies terminal pre-launch failures as `blocked` whenever lifecycle config is available.
- This is exactly the retry / redirect / resume edge the earlier review called out.

Suggested fix:

- Wrap those two fallible branches with `map_err(|e| { guard.emit_blocked_or_failure(); ... })`, the same way the surrounding pre-launch failure paths already do.

### 2. Launch-state is still tied to helper success, so some post-launch errors will be mislabeled as `blocked`

The non-harness and harness paths only call `mark_provider_launched()` after their execution helpers return `Ok(...)`:

- Non-harness inline/direct: `claudine/cli/src/commands/wrap/composition.rs:650-686`, `claudine/cli/src/commands/wrap/composition.rs:690-717`
- Harness: `claudine/cli/src/commands/wrap/mod.rs:2721-2779`

That is still too late, because the helpers can fail after the child has already been spawned and even after it has produced output:

- `run_child_stream()` spawns first, then can still fail on stdin write: `claudine/cli/src/commands/wrap/exec.rs:730-832`
- `execute_harness_attempt()` can fail on stdout rendering / flushing after `run_child_stream()` or `run_child_capture()` has already completed: `claudine/cli/src/commands/wrap/mod.rs:1888-1918`, `claudine/cli/src/commands/wrap/mod.rs:1938-1968`
- `execute_direct_without_harness()` has the same pattern: `claudine/cli/src/commands/wrap/composition.rs:1247-1279`

Result:

- A broken pipe on stdin or stdout, or another helper-level I/O failure after spawn, will drop the guard while `provider_launched == false`.
- The lifecycle terminal signal becomes `blocked`, even though the provider already launched and may even have finished.

Suggested fix:

- Move the “provider launched” transition closer to actual child spawn rather than helper success.
- The cleanest option is to have `run_child*` / `execute_*` return a small result that explicitly reports whether spawn succeeded before any later I/O/render error occurred.

## Coverage Gaps

### 1. Wrapper-level lifecycle integration is still effectively untested

I still do not see lifecycle-focused integration tests under `claudine/cli/tests`. `wrap_commands.rs` ends without any lifecycle cases, and `rg -n "lifecycle" claudine/cli/tests` returns no matches.

Missing tests that would catch the two remaining bugs:

- Harness re-materialization failure before first launch should emit `blocked`
- Harness re-parse failure before first launch should emit `blocked`
- Post-spawn stdin write failure should emit `failure`, not `blocked`
- Post-spawn stdout/render/flush failure should emit `failure`, not `blocked`
- Non-harness direct and inline variants of the post-launch I/O failures above

### 2. The unit tests cover ordering now, but not failure isolation

The new `LifecycleRunGuard` tests in `claudine/lib/src/composition/lifecycle.rs:955-1343` cover ordering and drop behavior well, but I still do not see tests for:

- emitter failure isolation when speech/effect/message emission itself fails
- a configured message route, rather than only the no-route no-op in `claudine/lib/src/messaging/send.rs:449-465`

## Ergonomics / Maintainability

### 1. The old lifecycle API is still easy to bypass

The new guard is the right direction, but both of these are still publicly exported:

- `emit_lifecycle_signal`
- `LifecycleRuntimeState`

See `claudine/lib/src/composition/mod.rs:18-22`.

That leaves two lifecycle APIs in circulation: the safe one and the foot-gun. If the old API is still needed, it should at least be clearly documented as low-level / legacy. If it is not needed outside tests or compatibility shims, narrowing the public surface would make regressions less likely.

## Verification

I ran `cargo test -p claudine lifecycle --lib`, and the lifecycle-focused library tests passed.
