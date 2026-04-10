# More Complete Fix Plan: Native Audio Device Open Timeouts

## Goal

Keep the current library-safe behavior of returning an error instead of terminating the process, and harden the implementation so repeated native device-open hangs do not keep spawning orphaned blocked threads or repeatedly delay playback attempts.

## Summary Of Recommended Direction

Implement a process-local native playback circuit breaker and unify timeout behavior around a single native-device deadline. The first native device-open timeout should disable further native playback attempts for the rest of the process, causing future calls to fall back directly to host playback. This avoids repeated thread leaks and repeated 5s stalls while preserving the safer error-returning design.

This is a pragmatic fix that can be implemented in-process without introducing a helper subprocess. A helper subprocess remains a possible later enhancement if cancellation of hung backend calls becomes necessary.

## Scope

- `playa/lib/src/native_player.rs`
- `playa/lib/src/sfx_player.rs`
- `playa/lib/src/playa.rs`
- `playa/lib/src/effects.rs`
- `playa/lib/src/channels.rs`
- tests in the affected modules

## Implementation Plan

1. Introduce a shared native audio circuit breaker.

Create a small internal module or helper that tracks whether native playback is still considered healthy for the current process.

Requirements:
- Use a process-global atomic state.
- Initial state: native playback enabled.
- Trip the breaker when a device-open timeout occurs.
- Expose helpers such as:
  - `native_audio_available() -> bool`
  - `trip_native_audio_breaker(reason: NativeAudioFailureKind)`
  - optional `native_audio_failure_reason() -> Option<...>` for diagnostics/tests

Recommended shape:
- Keep the first implementation simple with `AtomicBool`.
- If diagnostics are useful, store a separate small enum or static string behind a lock or atomic integer.

2. Gate native playback and native SFX playback on the breaker before any thread spawn.

Update both native entry points so they fail fast before attempting channel lookup or stream open when the breaker is already tripped.

Targets:
- `play_native(...)`
- `play_sfx(...)`

Behavior:
- If breaker is tripped, return a normal native-path error immediately.
- Callers that already do `is_ok()`-based fallback will naturally route to host playback without waiting.

Suggested error additions:
- `NativePlaybackError::NativePlaybackDisabled`
- `SfxPlaybackError::NativePlaybackDisabled`

3. Trip the breaker specifically on device-open timeout.

Keep non-timeout native failures non-fatal to the breaker.

Why:
- Decode errors, unsupported formats, missing devices, and ordinary stream errors do not mean the backend is hung.
- The breaker should respond only to failures that indicate native device operations are unsafe to retry in this process.

Targets:
- `open_stream_with_timeout(...)`
- `open_sfx_stream_with_timeout(...)`

Behavior:
- On `recv_timeout`, set the breaker before returning `DeviceOpenTimeout(...)`.
- Do not trip the breaker on ordinary `Stream(...)` errors unless later evidence shows those also indicate backend wedging.

4. Unify native time budget across lookup and open.

The current implementation can spend up to 10s in device lookup but only 5s in outer stream-open wait, which makes fallback behavior inconsistent.

Refactor the native open path to operate from a single deadline:
- Introduce one shared constant for native device operations, for example `NATIVE_DEVICE_TIMEOUT`.
- Pass the same budget into:
  - device lookup
  - stream open wait
- Prefer deadline-based APIs over nested fixed waits where practical.

Recommended minimum change:
- Add timeout-taking variants in `channels.rs`, e.g.
  - `find_device_by_id_or_name_with_timeout(...)`
- Use the same timeout constant from native playback and SFX playback paths.

Result:
- Channel lookup fallback will be deterministic.
- The native attempt will consume one bounded time budget instead of two independent ones.

5. Make the channel lookup fallback explicit.

Preserve the existing intended behavior:
- If a requested channel lookup times out or fails, native playback should try the default output device before giving up, unless the circuit breaker is already tripped.

Implement this as an explicit sequence rather than relying on nested helpers with different timeout semantics.

Recommended flow:
1. If a channel is requested, try to resolve it within the shared timeout budget.
2. If resolution fails or times out, log once and continue with default device.
3. Attempt default-device stream open within the remaining budget.
4. If stream open times out, trip breaker and return timeout error.

6. Add low-noise diagnostics.

Add concise stderr logging only when:
- the breaker trips for the first time
- native playback is skipped because the breaker is already tripped

Avoid logging on every fallback attempt once the breaker is tripped unless the message is explicitly deduplicated.

7. Add targeted tests around the new control flow.

Focus on deterministic tests for policy, not real hung-device integration tests.

Suggested tests:
- breaker starts enabled
- breaker trips on simulated device-open timeout
- once tripped, `play_native` short-circuits before attempting open
- once tripped, `play_sfx` short-circuits before attempting open
- ordinary native failures do not trip the breaker
- requested-channel lookup failure still falls back to default device path

Implementation note:
- To make this testable, factor the timeout/open operations behind small private helpers so unit tests can inject simulated outcomes under `#[cfg(test)]`.

8. Document the new runtime policy.

Update native playback docs/comments to state:
- native playback no longer terminates the process
- a native device-open timeout disables further native playback attempts for the current process
- host-player fallback remains the recovery path

Targets:
- module docs in `native_player.rs`
- module docs in `sfx_player.rs`
- any README/docs that describe native playback reliability behavior

## Optional Follow-Up

If in-process circuit breaking is still too weak for the observed failures, the next step is isolating native audio opens in a helper subprocess:

- Parent process asks helper to open/play native audio.
- If helper hangs during backend calls, parent times out and kills helper.
- This is the only robust way to cancel truly stuck backend operations.

This should be treated as a second-phase design, not part of the first implementation unless real-world hangs continue after the breaker approach.

## Validation

After implementation:

1. Run focused tests:
   - `cargo test -p playa --lib`
2. Add or run tests specific to native timeout policy if feature-gated.
3. Verify ordinary playback still falls back to host players when native playback is disabled.
4. On macOS, do a manual smoke check for:
   - default playback
   - requested-channel playback
   - sound effect playback

## Acceptance Criteria

- No library code calls `std::process::exit`.
- A native device-open timeout does not terminate the process.
- After the first native device-open timeout, later native playback attempts do not spawn more device-open threads.
- Host fallback still works for both general playback and sound effects.
- Timeout behavior is consistent across channel lookup and stream opening.
