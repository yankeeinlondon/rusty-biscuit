# Detailed Design: Linux SFX and Ducking Fixes

## Goal

Fix the Linux-specific correctness and hang risks identified in the 2026-04-09 cross-platform review without changing Playa's public playback model.

This design covers:

- H1: unbounded PulseAudio SFX waits
- H2: PulseAudio ducking restore drift
- H3: ALSA ducking self-ducks Playa
- M1: clarify and bound the synchronous PulseAudio SFX path

## Why This Design Is High Confidence

- The existing code already has shared timeout conventions in `playa/lib/src/native_audio.rs` and `playa/lib/src/sfx_player.rs`:
  - `NATIVE_DEVICE_TIMEOUT = 5s`
  - `PLAYBACK_TIMEOUT = 30s`
- The current Linux PulseAudio path is isolated to `playa/lib/src/sfx_player.rs`, so the hang fix is localized.
- The Linux ducking backend already snapshots Pulse sink inputs and excludes Playa by PID/name in `playa/lib/src/ducking/linux.rs`; the drift issue is in the write strategy, not the backend shape.
- The ALSA fallback is already explicitly described as coarse. Changing factory selection is localized to `playa/lib/src/ducking/factory.rs`, docs, and CLI diagnostics.
- The sync sound-effect API is already blocking on all platforms. The real regression is unbounded blocking, not the existence of blocking itself.

## Decisions To Lock Before Coding

1. `SoundEffect::play()` and `SoundEffect::play_with_options()` remain synchronous. This fix makes Linux bounded and documented, not fire-and-forget.
2. The PulseAudio SFX path must not rely on `mainloop.iterate(true)` inside deadline loops. A pre-iterate deadline check is insufficient because `iterate(true)` itself can block forever.
3. PulseAudio context-ready and stream-ready waits use the shared `NATIVE_DEVICE_TIMEOUT` budget. Drain uses a bounded playback deadline derived from clip duration and capped by `PLAYBACK_TIMEOUT`.
4. PulseAudio ducking restore must snap the final step to the exact cached original volume, using raw Pulse volume units internally instead of only percentage floats.
5. `create_backend()` must stop auto-selecting `AlsaBackend`. When PulseAudio/PipeWire is unavailable, Linux ducking falls back to `NoopBackend` with a one-time warning explaining why.
6. `AlsaBackend` may stay in the codebase as an internal/manual fallback for future work, but it is not part of the default runtime backend selection after this change.

## Scope

Files that should change:

- `playa/lib/src/sfx_player.rs`
- `playa/lib/src/ducking/linux.rs`
- `playa/lib/src/ducking/factory.rs`
- `playa/lib/src/ducking/mod.rs`
- `playa/lib/src/effects.rs`
- `playa/lib/src/ducking/tests.rs`
- `playa/cli/src/main.rs`
- `playa/docs/audio-ducking.md`

Likely no Cargo feature changes are required.

## Design

### 1. PulseAudio SFX: replace indefinite waits with deadline-aware polling

The current implementation uses the standard PulseAudio mainloop with:

- `context.connect(...)`
- `stream.connect_playback(...)`
- `stream.drain(None)`
- repeated `mainloop.iterate(true)`

That pattern is unsafe for deadlines because `iterate(true)` can block inside a single iteration. Checking `Instant::now()` before the call does not help if the thread is already stuck in `iterate(true)`.

#### Proposed structure

Keep `play_sfx_as_event()` synchronous, but change the wait strategy:

1. Decode audio and compute the sample spec as today.
2. Compute explicit phase deadlines:
   - `context_deadline = Instant::now() + NATIVE_DEVICE_TIMEOUT`
   - `stream_deadline = Instant::now() + NATIVE_DEVICE_TIMEOUT`
   - `drain_deadline = Instant::now() + effective_drain_timeout(...)`
3. Replace every `loop { iterate_or_fail(&mut mainloop)?; ... }` with a helper that uses `iterate(false)` plus a short sleep.

Recommended helper shape inside the Linux module:

```rust
fn wait_for_pulse_condition<F>(
    mainloop: &mut Mainloop,
    deadline: Instant,
    phase: &'static str,
    mut check: F,
) -> Result<(), Box<dyn std::error::Error>>
where
    F: FnMut() -> Result<bool, Box<dyn std::error::Error>>,
{
    loop {
        if Instant::now() >= deadline {
            return Err(format!("PulseAudio {phase} timed out").into());
        }

        iterate_or_fail_nonblocking(mainloop)?;

        if check()? {
            return Ok(());
        }

        std::thread::sleep(Duration::from_millis(10));
    }
}
```

`iterate_or_fail_nonblocking()` should call `mainloop.iterate(false)` instead of `iterate(true)`.

This avoids indefinite blocking while preserving the existing single-threaded control flow.

#### Drain timeout policy

Drain is not a device-open phase. It is playback completion, so it should not reuse the fixed 5 second connect timeout blindly.

Recommended drain deadline:

```rust
let clip_duration = Duration::from_secs_f64(
    samples.len() as f64 / (channels_u8 as f64 * effective_rate as f64)
);
let drain_timeout = (clip_duration + Duration::from_secs(5)).min(PLAYBACK_TIMEOUT);
let drain_timeout = drain_timeout.max(NATIVE_DEVICE_TIMEOUT);
```

This keeps short effects responsive while still giving legitimate playback time to finish.

#### Error behavior

Do not change the outer `play_sfx()` contract. The Linux native path should still return an error and fall through to the default rodio path.

What should change is the error quality:

- `"PulseAudio context connection timed out"`
- `"PulseAudio stream connection timed out"`
- `"PulseAudio drain timed out"`

These phase-specific errors make fallback behavior diagnosable if logging is later added.

#### Why not `run_with_timeout` around the whole PulseAudio function

`run_with_timeout()` would prevent the caller from hanging, but it would leave a stuck worker thread behind if the PulseAudio daemon never responds. That is acceptable for device-open probes where the operation is isolated and rare, but it is the wrong primary design here because the PulseAudio mainloop can be made deadline-aware directly.

The better fix is to make the inner loop non-blocking and bounded.

### 2. PulseAudio SFX: keep the sync contract, but document it

The review calls out that the PulseAudio path blocks the calling thread. That is true, but it is also the current contract of `SoundEffect::play*` across backends:

- rodio/native SFX waits for playback completion
- host-player fallback waits for the child process
- `SoundEffect::play_with_options()` is a sync API

Changing the Linux Pulse path to background playback would change observable behavior and would diverge from the rest of the library.

#### Decision

Do not change the sync API in this fix.

Instead:

- make the Linux Pulse path bounded as described above
- add Rustdoc warnings that the sync API blocks until playback completes
- explicitly tell async callers to use `tokio::task::spawn_blocking`

Docs to update:

- `playa/lib/src/sfx_player.rs`
- `playa/lib/src/effects.rs`

Recommended wording direction:

> This function blocks the current thread until playback completes or times out. In async code, call it from `tokio::task::spawn_blocking`.

That resolves M1 without introducing a behavior change.

### 3. PulseAudio ducking: eliminate restore drift with exact final-step snapping

The current Linux ducking path computes each fade step as a relative percent delta from the live current volume:

- read current Pulse volume
- convert to percent
- compute `delta = desired_percent - current_percent`
- call `increase_app_volume_by_percent` or `decrease_app_volume_by_percent`

That strategy is acceptable for intermediate motion, but it is not safe for exact restoration because PulseAudio stores volume in quantized integer units and each relative adjustment rounds independently.

#### Core design change

Store exact original Pulse volume units in the backend cache, not only a float percentage.

Replace:

```rust
original_volumes: Mutex<HashMap<u32, f64>>
```

with something like:

```rust
struct CachedPulseVolume {
    avg_units: u32,
}

original_volumes: Mutex<HashMap<u32, CachedPulseVolume>>
```

The public `VolumeSnapshot` can remain unchanged. The exact-unit cache is an internal Linux backend detail.

#### Fade algorithm

Keep the current high-level envelope:

- use `compute_fade_steps(...)`
- re-read current volume before each write so external changes are tolerated

But change the final-write policy:

1. For intermediate steps:
   - continue using relative percent deltas
   - this preserves compatibility with the current `pulsectl-rs` control surface
2. For the final step of `fade_to_floor`:
   - compute the exact floor target from the original cached units
   - snap to that target using the delta from the current live value
3. For the final step of `fade_restore`:
   - compute the delta from the current live value to the exact cached original units
   - apply that exact delta even if prior steps drifted

This guarantees that every completed duck/restore cycle lands on the exact original Pulse volume, even if the intermediate steps rounded.

#### Why exact units matter

The current code converts through:

- Pulse raw units
- floating-point percent
- relative percent delta
- Pulse raw units again

If the cache stores only percent, the final snap still inherits earlier conversion loss. Caching raw units makes the last step authoritative.

#### Helper design

Add small conversion helpers in `playa/lib/src/ducking/linux.rs`:

- `pulse_units_to_percent(u32) -> f64`
- `pulse_units_to_scalar(u32) -> f32`
- `scalar_to_pulse_percent(f32) -> f64`

Then introduce one helper that applies a target volume:

```rust
fn apply_app_target_percent(
    controller: &mut SinkController,
    index: u32,
    target_percent: f64,
) -> Result<(), DuckingError>
```

That helper should:

- read the current app volume
- compute `delta = target_percent - current_percent`
- call `increase_app_volume_by_percent` or `decrease_app_volume_by_percent`
- treat tiny deltas as no-op to avoid oscillation near zero

The fade code then becomes simpler and centralizes the rounding policy.

#### If `pulsectl-rs` later gains absolute app volume writes

Do not redesign the backend API again. Keep the helper and swap its internals from relative-delta writes to absolute writes.

### 4. ALSA ducking: stop selecting it automatically

The ALSA backend is not merely coarse. It violates the intended behavior of ducking:

- it affects the entire playback device
- it cannot exclude Playa
- it therefore reduces Playa's own audio along with everything else

That defeats the feature's purpose on ALSA-only systems.

#### Decision

`create_backend()` must stop returning `AlsaBackend` automatically.

New Linux selection order:

1. `LinuxBackend` if PulseAudio/PipeWire is available
2. otherwise `NoopBackend`

At selection time, emit a warning once:

> playa: PulseAudio/PipeWire unavailable; skipping Linux ducking because ALSA fallback would also duck Playa's own output

Use `std::sync::Once` or `OnceLock` so the warning is not spammed.

#### What happens to `AlsaBackend`

Keep it for now, but make its status explicit:

- update doc comments to say it is system-wide and self-ducking
- remove it from the factory's default selection path
- do not advertise it as the normal Linux fallback in docs or CLI output

This keeps the implementation available for experimentation or a future explicit opt-in, without exposing a broken default.

#### CLI and doc impact

Update:

- `playa/cli/src/main.rs`
- `playa/docs/audio-ducking.md`
- `playa/lib/src/ducking/mod.rs`
- `playa/lib/src/ducking/factory.rs`

Specifically:

- `backend_name()` should no longer report `"linux-alsa"` from normal selection
- `duck-info` should describe Linux as PulseAudio/PipeWire per-app ducking, otherwise `noop`
- docs must stop claiming ALSA is the default Linux fallback

### 5. Testing strategy

#### `sfx_player.rs`

Add unit-level coverage for the new timeout machinery by factoring the wait logic into small helpers that can be tested without a real PulseAudio daemon.

Recommended tests:

- timeout occurs when a condition never becomes ready
- helper returns immediately when the condition is already ready
- helper surfaces explicit phase names in timeout errors

Keep the existing ignored Linux integration tests, but update them to use the same timeout-aware helpers rather than open-coded loops. This prevents the tests from reintroducing the original bug pattern.

Recommended ignored integration updates:

- `can_connect_pulseaudio_context`
- `can_create_event_stream`

Both should fail within a bounded timeout instead of hanging.

#### `ducking/linux.rs`

Add pure tests around conversion and final-step behavior. These should not require a live PulseAudio server.

Recommended test coverage:

- `pulse_units_to_percent` round-trips expected values
- final restore target uses the exact cached original units
- intermediate steps may round, but final restore snaps back
- repeated duck/restore cycles do not drift when the final snap is applied

If the code structure allows it, test the target-selection math separately from the controller I/O.

#### `ducking/factory.rs`

Update Linux factory tests to expect only:

- `"linux-pulse"`
- `"noop"`

Remove the current `"linux-alsa"` expectation from default backend selection tests.

#### CLI `duck-info`

No heavy testing is required, but the Linux messaging should be updated so user-facing diagnostics match the new selection policy.

### 6. Recommended delivery order

1. Fix PulseAudio SFX wait loops in `playa/lib/src/sfx_player.rs`.
2. Update docs in `sfx_player.rs` and `effects.rs` to clarify blocking semantics.
3. Fix PulseAudio ducking drift in `playa/lib/src/ducking/linux.rs`.
4. Remove automatic ALSA selection from `playa/lib/src/ducking/factory.rs`.
5. Update `duck-info` output and `audio-ducking.md`.
6. Add and update tests.

This order lands the hang fix first, then the restore-correctness fix, then the behavioral cleanup around ALSA fallback.

## Acceptance Criteria

### PulseAudio SFX

- A hung or non-responsive PulseAudio daemon no longer blocks the process forever.
- Context connection, stream connection, and drain each fail with bounded timeouts.
- `SoundEffect::play_with_options()` still behaves synchronously, but Linux now fails fast and falls back instead of hanging.

### PulseAudio ducking

- After one duck/restore cycle, restored app volumes exactly match the cached originals.
- Repeated duck/restore cycles do not drift user volumes.

### ALSA selection

- ALSA is no longer selected automatically for Linux ducking.
- Linux systems without PulseAudio/PipeWire fall back to `NoopBackend` with a clear warning.
- `duck-info` and docs no longer advertise ALSA as the default Linux ducking fallback.

## Out Of Scope

- introducing a new async sound-effect API
- deleting `AlsaBackend` entirely
- implementing per-device compensation to offset ALSA self-ducking
- extending Linux ducking to catch applications that start after the initial snapshot
