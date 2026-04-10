# High-Confidence Plan: Linux SFX and Ducking Fixes

## Goal

Fix the Linux-specific hang and correctness issues called out in [playa/reviews/2026-04-09-cross-platform/linux-design.md](/Users/ken/.claudine/worktrees/rusty-biscuit/playa/playa/reviews/2026-04-09-cross-platform/linux-design.md) without changing Playa's public playback model or the existing ducking lifecycle.

## Why This Plan Is High Confidence

- The failing behavior is localized and already visible in the current implementation:
    - unbounded PulseAudio wait loops are isolated to [playa/lib/src/sfx_player.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/playa/playa/lib/src/sfx_player.rs)
    - Pulse ducking restore logic is isolated to [playa/lib/src/ducking/linux.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/playa/playa/lib/src/ducking/linux.rs)
    - backend selection and user-facing diagnostics are isolated to [playa/lib/src/ducking/factory.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/playa/playa/lib/src/ducking/factory.rs), [playa/lib/src/ducking/mod.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/playa/playa/lib/src/ducking/mod.rs), [playa/cli/src/main.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/playa/playa/cli/src/main.rs), and [playa/docs/audio-ducking.md](/Users/ken/.claudine/worktrees/rusty-biscuit/playa/playa/docs/audio-ducking.md)
- The timeout conventions already exist in [playa/lib/src/sfx_player.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/playa/playa/lib/src/sfx_player.rs):
    - `NATIVE_DEVICE_TIMEOUT = 5s`
    - `PLAYBACK_TIMEOUT = 30s`
- The Linux ducking backend already has the right overall shape:
    - `snapshot()`
    - `fade_to_floor()`
    - `fade_restore()`
- The sync contract is already the current behavior:
    - [playa/lib/src/effects.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/playa/playa/lib/src/effects.rs) already documents that `SoundEffect::play()` blocks until playback completes
    - the fix is to bound Linux blocking behavior, not redesign the API
- No feature-flag or Cargo surgery is required. This is mostly implementation, test, and documentation cleanup.

## Decisions To Lock Before Coding

1. `SoundEffect::play()` and `SoundEffect::play_with_options()` remain synchronous.
2. The PulseAudio SFX path must stop using `mainloop.iterate(true)` inside readiness and drain loops because a single iteration can block indefinitely.
3. Context-ready and stream-ready waits should use `NATIVE_DEVICE_TIMEOUT`; drain should use a clip-derived timeout capped by `PLAYBACK_TIMEOUT`.
4. Pulse ducking restore must cache exact Pulse volume units and snap the final restore step to those cached units.
5. `create_backend()` and `backend_name()` must stop auto-selecting ALSA on Linux. The default Linux selection becomes PulseAudio/PipeWire or noop.
6. Keep `AlsaBackend` in the codebase for now, but do not present it as the normal runtime fallback. If API compatibility matters, leave it exported and only remove it from factory selection.

## Scope

Files that should change:

- [playa/lib/src/sfx_player.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/playa/playa/lib/src/sfx_player.rs)
- [playa/lib/src/ducking/linux.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/playa/playa/lib/src/ducking/linux.rs)
- [playa/lib/src/ducking/factory.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/playa/playa/lib/src/ducking/factory.rs)
- [playa/lib/src/ducking/mod.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/playa/playa/lib/src/ducking/mod.rs)
- [playa/lib/src/effects.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/playa/playa/lib/src/effects.rs)
- [playa/lib/src/ducking/tests.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/playa/playa/lib/src/ducking/tests.rs)
- [playa/cli/src/main.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/playa/playa/cli/src/main.rs)
- [playa/docs/audio-ducking.md](/Users/ken/.claudine/worktrees/rusty-biscuit/playa/playa/docs/audio-ducking.md)

Out of scope:

- adding a new async SFX API
- deleting `AlsaBackend`
- compensating for ALSA self-ducking
- expanding Linux ducking to catch apps that appear after snapshot time

## Implementation Plan

1. Replace the Linux PulseAudio wait loops with deadline-aware nonblocking polling.

In [playa/lib/src/sfx_player.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/playa/playa/lib/src/sfx_player.rs):

- keep `play_sfx_as_event()` synchronous
- add a nonblocking iterator helper that wraps `mainloop.iterate(false)`
- add a small wait helper along the lines of:

```rust
fn wait_for_pulse_condition<F>(
    mainloop: &mut Mainloop,
    deadline: Instant,
    phase: &'static str,
    mut check: F,
) -> Result<(), Box<dyn std::error::Error>>
where
    F: FnMut() -> Result<bool, Box<dyn std::error::Error>>,
```

- use that helper for:
    - context ready
    - stream ready
    - drain completion
- replace the current open-coded `loop { iterate_or_fail(...); ... }` blocks with phase-specific waits
- return phase-specific timeout errors:
    - `PulseAudio context connection timed out`
    - `PulseAudio stream connection timed out`
    - `PulseAudio drain timed out`

This is the most important fix because the current Linux path in [playa/lib/src/sfx_player.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/playa/playa/lib/src/sfx_player.rs) can block forever inside `iterate(true)`.

1. Make drain timeout clip-aware instead of reusing a fixed device-open budget.

Still in [playa/lib/src/sfx_player.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/playa/playa/lib/src/sfx_player.rs):

- compute clip duration from:
    - decoded sample count
    - channel count
    - effective sample rate
- derive:

```rust
let drain_timeout = (clip_duration + Duration::from_secs(5)).min(PLAYBACK_TIMEOUT);
let drain_timeout = drain_timeout.max(NATIVE_DEVICE_TIMEOUT);
```

- use `NATIVE_DEVICE_TIMEOUT` only for context and stream readiness

This matches the existing timeout policy in the same file and avoids incorrectly treating playback completion like device-open probing.

1. Clarify the sync blocking contract in Rustdoc instead of changing behavior.

In [playa/lib/src/effects.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/playa/playa/lib/src/effects.rs):

- keep the existing blocking note on `play()`
- add equivalent wording to `play_with_options()`
- explicitly tell async callers to use `tokio::task::spawn_blocking`

In [playa/lib/src/sfx_player.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/playa/playa/lib/src/sfx_player.rs):

- update the Linux native-path docs to say playback blocks until completion or timeout
- make it explicit that native Linux errors fall back to host playback

This resolves the documentation gap without changing the observable API.

1. Fix Pulse ducking restore drift by caching exact Pulse units and snapping the final step.

In [playa/lib/src/ducking/linux.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/playa/playa/lib/src/ducking/linux.rs):

- replace the percent-only cache:

```rust
original_volumes: Mutex<HashMap<u32, f64>>
```

with a small internal cached type such as:

```rust
struct CachedPulseVolume {
    avg_units: u32,
}
```

- capture exact `app.volume.avg().0` values in `snapshot()`
- keep `VolumeSnapshot` unchanged so the public ducking model stays the same
- add conversion helpers for raw units and percentages
- introduce one helper that applies a target percentage by reading current app volume and issuing the required relative delta write
- keep relative writes for intermediate steps
- on the final duck or restore step, compute the target from the cached raw units and force the last delta to land on that exact value

The current implementation in [playa/lib/src/ducking/linux.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/playa/playa/lib/src/ducking/linux.rs) re-reads volume and applies percent deltas on every step, which is where the drift comes from.

1. Remove automatic ALSA fallback from factory selection and diagnostics.

In [playa/lib/src/ducking/factory.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/playa/playa/lib/src/ducking/factory.rs):

- keep trying `LinuxBackend` first
- remove `AlsaBackend` from the default `create_backend()` selection path
- return `NoopBackend` when PulseAudio/PipeWire is unavailable
- emit a once-only warning:

```text
playa: PulseAudio/PipeWire unavailable; skipping Linux ducking because ALSA fallback would also duck Playa's own output
```

- update `backend_name()` so the default Linux result is only:
    - `linux-pulse`
    - `noop`

In [playa/lib/src/ducking/mod.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/playa/playa/lib/src/ducking/mod.rs):

- update module docs so Linux is described as PulseAudio/PipeWire per-app ducking with noop fallback
- do not claim ALSA is the default Linux path anymore

In [playa/lib/src/ducking/linux.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/playa/playa/lib/src/ducking/linux.rs):

- update `AlsaBackend` docs to make the self-ducking limitation explicit

1. Align CLI `duck-info` and package docs with the new Linux policy.

In [playa/cli/src/main.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/playa/playa/cli/src/main.rs):

- keep the `linux-pulse` branch
- remove the `linux-alsa` branch from the normal backend-selection story
- update `noop` messaging so Linux users understand noop can mean PulseAudio/PipeWire is unavailable

In [playa/docs/audio-ducking.md](/Users/ken/.claudine/worktrees/rusty-biscuit/playa/playa/docs/audio-ducking.md):

- remove statements that advertise ALSA as the default Linux fallback
- describe Linux as:
    - PulseAudio/PipeWire per-sink-input ducking when available
    - noop otherwise
- update the scope and limitations sections to reflect that ALSA is no longer selected automatically

1. Add tests around policy and helper behavior instead of relying on live audio servers.

In [playa/lib/src/sfx_player.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/playa/playa/lib/src/sfx_player.rs):

- add unit tests for the new wait helper:
    - returns immediately when ready
    - times out when never ready
    - includes the phase name in timeout errors
- update the ignored Linux integration tests to use the new helper so they cannot reintroduce indefinite waits

In [playa/lib/src/ducking/linux.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/playa/playa/lib/src/ducking/linux.rs):

- add pure tests for:
    - raw-unit to percent conversion
    - final-step exact restore targeting
    - repeated duck/restore cycles not drifting when the final snap is applied

In [playa/lib/src/ducking/factory.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/playa/playa/lib/src/ducking/factory.rs):

- update Linux expectations to only allow:
    - `linux-pulse`
    - `noop`

In [playa/lib/src/ducking/tests.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/playa/playa/lib/src/ducking/tests.rs):

- add policy tests that exercise Linux selection and snapshot/restore math without requiring a live Pulse daemon

## Recommended Delivery Order

1. Land the PulseAudio SFX wait-loop fix in [playa/lib/src/sfx_player.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/playa/playa/lib/src/sfx_player.rs).
2. Update blocking-behavior docs in [playa/lib/src/sfx_player.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/playa/playa/lib/src/sfx_player.rs) and [playa/lib/src/effects.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/playa/playa/lib/src/effects.rs).
3. Fix Pulse restore drift in [playa/lib/src/ducking/linux.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/playa/playa/lib/src/ducking/linux.rs).
4. Remove automatic ALSA selection in [playa/lib/src/ducking/factory.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/playa/playa/lib/src/ducking/factory.rs).
5. Update CLI and docs in [playa/cli/src/main.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/playa/playa/cli/src/main.rs), [playa/lib/src/ducking/mod.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/playa/playa/lib/src/ducking/mod.rs), and [playa/docs/audio-ducking.md](/Users/ken/.claudine/worktrees/rusty-biscuit/playa/playa/docs/audio-ducking.md).
6. Finish with tests.

This order lands the hang fix first, then the restore-correctness fix, then the backend-selection cleanup.

## Verification

Run focused checks from the repo root:

```bash
cargo test -p playa --lib ducking::tests
cargo test -p playa --lib sfx_player
```

If Linux audio ducking features are not enabled by default for the package test target, run the equivalent targeted command with the Linux feature set enabled for local verification.

Ignored integration coverage to run only on a Linux machine with PulseAudio or PipeWire available:

```bash
cargo test -p playa --features audio-ducking-linux,sfx-native-linux -- --ignored
```

## Acceptance Criteria

- A hung or unresponsive PulseAudio daemon no longer blocks Linux SFX playback forever.
- Context connection, stream connection, and drain each fail within bounded deadlines.
- `SoundEffect::play_with_options()` remains synchronous but no longer risks an unbounded Linux hang.
- Pulse ducking restore lands back on the exact cached original volume after a full duck/restore cycle.
- Repeated duck/restore cycles do not drift user-set volumes.
- Linux backend selection no longer returns `linux-alsa` by default.
- Linux systems without PulseAudio/PipeWire fall back to `noop` with a clear one-time warning.
- CLI output and docs no longer advertise ALSA as the normal Linux ducking fallback.

## Notes

- The one potentially breaking choice here is whether to stop re-exporting `AlsaBackend` from [playa/lib/src/ducking/mod.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/playa/playa/lib/src/ducking/mod.rs). The high-confidence path is to leave the type available for now and only remove it from default runtime selection.
- If `pulsectl` later exposes absolute sink-input volume writes, the target-volume helper in [playa/lib/src/ducking/linux.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/playa/playa/lib/src/ducking/linux.rs) can switch to absolute writes without redesigning the backend.
