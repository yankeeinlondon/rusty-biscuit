# Windows WASAPI Ducking Backend Design

**Date:** 2026-04-09  
**Context:** Follow-up to review issues `C1. Windows ducking is completely unimplemented` and `C2. Windows WASAPI SFX: COM initialization is unchecked and leaked` in `playa/reviews/2026-04-09-cross-platform/review.md`

## Problem

Windows currently compiles the ducking abstraction but never provides a real backend:

- `playa/lib/src/ducking/factory.rs` always returns `NoopBackend` on Windows.
- `backend_name()` also reports `"noop"` on Windows.
- `SessionId::WasapiSession { pid, key }` already exists in `types.rs`.
- The `windows` crate is already in use for WASAPI sound-effect playback in `sfx_player.rs`.

The result is silent failure: users can enable `audio-ducking`, but Windows playback never attenuates other sessions and emits no warning.

Windows also has a second WASAPI issue in the native SFX path:

- `playa/lib/src/sfx_player.rs` calls `CoInitializeEx(None, COINIT_MULTITHREADED)` and discards the result.
- If the current thread is already initialized as STA, this returns `RPC_E_CHANGED_MODE`.
- The code never balances successful COM initialization with `CoUninitialize()`.

That means the Windows design work should not stop at a new ducking backend. It should also define a single correct COM lifecycle pattern for all Playa WASAPI code.

## Decision Summary

Implement a real `WindowsBackend` that uses WASAPI session enumeration plus `ISimpleAudioVolume` to duck active render sessions on the default multimedia output device.

The backend will:

1. Enumerate active sessions from `IAudioSessionManager2`.
2. Snapshot per-session master volume and mute state.
3. Exclude Playa-owned sessions by PID.
4. Fade matched sessions to `config.floor_scalar()`.
5. Re-enumerate and restore the exact snapshot by stable session instance key.

This backend should be gated behind a new `audio-ducking-windows` feature. As an immediate mitigation, the existing Windows `NoopBackend` path should emit a warning until the backend ships.

In the same design, the Windows SFX path should adopt the same COM guard so WASAPI playback:

1. handles `RPC_E_CHANGED_MODE` correctly,
2. never runs WASAPI with an effectively uninitialized COM state,
3. and always balances successful `CoInitializeEx` calls with `CoUninitialize()`.

## Goals

- Match the current `DuckingBackend` contract without changing the public ducking API.
- Duck other Windows audio sessions without ducking Playa's own playback.
- Restore exact prior per-session volume and mute state on normal completion or guard drop.
- Avoid storing live COM interfaces in the backend so the type remains `Send + Sync`.
- Keep the implementation aligned with existing crate patterns in `macos.rs` and `linux.rs`.
- Define one correct COM initialization pattern that both the ducking backend and the Windows SFX path use.

## Non-Goals

- No endpoint-wide fallback in v1.
- No Windows communications ducking integration (`AudioCategory_Communications`, `SetDuckingPreference`, or session event subscriptions).
- No attempt to duck sessions created after the snapshot is taken.
- No redesign of the existing `DuckGuard` lifecycle or global multi-guard coordination.

## Why Per-Session WASAPI

Per-session volume control is the correct fit for Playa's current ducking model:

- The trait already snapshots/restores individual session entries.
- Linux already ducks other applications while excluding Playa.
- A Windows endpoint-volume fallback would duck Playa's own audio and would require either a new `SessionId` variant or an awkward overload of `WasapiSession`.
- `ISimpleAudioVolume` is the same Windows mixer primitive users see per app in Volume Mixer.

For these reasons, v1 should be a pure per-session backend. Endpoint-wide fallback can remain deferred until there is a clear product requirement for "duck something even if sessions cannot be enumerated".

## Current Architecture Constraints

The design must fit these existing constraints:

- `DuckingBackend` async methods return `Send` futures.
- Existing backends do not retain platform handle types that are hard to share across threads.
- `VolumeSnapshot` already stores the required state:
    - `SessionId::WasapiSession { pid, key }`
    - `channels: Vec<f32>`
    - `mute: bool`
- `DuckGuard` handles lifecycle and restoration timing.

That implies the Windows backend should be mostly stateless and should reacquire COM/WASAPI objects inside each async operation.

It also implies Windows-specific COM setup should be centralized rather than copied into `ducking/windows.rs` and `sfx_player.rs` independently.

## Build and Feature Wiring

### Library feature

Add a Windows-specific ducking feature in `playa/lib/Cargo.toml`:

```toml
[features]
audio-ducking-windows = ["audio-ducking", "windows"]
```

### CLI feature

Mirror that in `playa/cli/Cargo.toml`:

```toml
[features]
audio-ducking-windows = ["audio-ducking", "playa/audio-ducking-windows"]
```

### Module wiring

Update `playa/lib/src/ducking/mod.rs`:

- Add `mod windows;` behind `#[cfg(all(target_os = "windows", feature = "audio-ducking-windows"))]`
- Re-export `WindowsBackend` under the same cfg

Update `factory.rs`:

- If Windows + `audio-ducking-windows`, instantiate `WindowsBackend`.
- If Windows without that feature, return `NoopBackend` and emit a warning once.
- `backend_name()` should return `"windows-wasapi"` when the real backend is selected.

## Proposed Backend Shape

File: `playa/lib/src/ducking/windows.rs`

```rust
#[derive(Debug)]
pub struct WindowsBackend {
    our_pid: u32,
    available: AtomicBool,
}
```

### Stored state

- `our_pid`: used for self-exclusion.
- `available`: cached result of a lightweight probe, similar to `LinuxBackend` and `MacOsBackend`.

No COM interface pointers should be stored in the struct.

## COM Model

Every backend method that touches WASAPI should initialize COM on the current thread and clean it up via RAII.

### Helper

Introduce a small internal helper:

```rust
struct ComGuard {
    should_uninit: bool,
}
```

Behavior:

- Call `CoInitializeEx(None, COINIT_MULTITHREADED)`.
- If the result is `S_OK` or `S_FALSE`, mark `should_uninit = true`.
- If the result is `RPC_E_CHANGED_MODE`, continue without failing. COM is already initialized on the thread with a different apartment model.
- On drop, call `CoUninitialize()` only when `should_uninit` is true.

This mirrors the fix already needed for the Windows SFX path and avoids introducing another unchecked COM initialization bug.

### Shared usage

This helper should be used in both places:

- `playa/lib/src/ducking/windows.rs`
- `playa/lib/src/sfx_player.rs` inside the `windows_sfx` module

That keeps the COM policy identical across all WASAPI entry points.

### SFX-specific requirement

The current SFX code:

```rust
let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
```

must be replaced with the shared guard or equivalent RAII logic. The behavioral requirements are:

- `S_OK` and `S_FALSE`: continue and call `CoUninitialize()` on drop
- `RPC_E_CHANGED_MODE`: continue without failing and without calling `CoUninitialize()`
- all other COM init failures: propagate as an error before invoking WASAPI APIs

This is important for GUI embeddings and any host that initializes COM as STA before calling Playa.

## Session Discovery

### Endpoint selection

Use:

- `IMMDeviceEnumerator`
- `GetDefaultAudioEndpoint(eRender, eMultimedia)`

Rationale:

- The existing WASAPI SFX path already targets the multimedia render endpoint.
- Ducking should focus on the user's default playback device for media output.

### Session enumeration flow

1. Activate `IAudioSessionManager2` from the default endpoint.
2. Call `GetSessionEnumerator()`.
3. Iterate `0..count`.
4. For each session:
   - cast to `IAudioSessionControl2`
   - cast to `ISimpleAudioVolume`
   - read:
     - `GetProcessId()`
     - `GetState()`
     - `GetSessionInstanceIdentifier()` preferred
     - `GetMasterVolume()`
     - `GetMute()`

### Session key choice

Use `GetSessionInstanceIdentifier()` for `SessionId::WasapiSession.key`.

Rationale:

- `GetSessionIdentifier()` can remain stable across multiple lifetimes of the same logical app session.
- `GetSessionInstanceIdentifier()` is a better restore key because it identifies the current live session instance and reduces accidental matches if a session dies and a new one appears during playback.

The existing field name `key` can remain unchanged, but the implementation should document that it stores the session instance identifier.

### Filtering rules

Only include sessions that satisfy all of the following:

- `AudioSessionStateActive`
- `pid != our_pid`
- successfully expose `ISimpleAudioVolume`
- have a non-empty session instance identifier

Skip:

- expired sessions
- inactive sessions
- sessions owned by the current process
- sessions whose COM calls fail partway through enumeration

Skipping inaccessible sessions is preferable to failing the whole ducking request.

## Snapshot Semantics

`snapshot()` should return:

```rust
VolumeSnapshot {
    entries: vec![
        SessionVolume {
            id: SessionId::WasapiSession { pid, key },
            channels: vec![master_volume],
            mute,
        }
    ]
}
```

Notes:

- WASAPI session master volume is scalar, so `channels` contains a single value.
- Snapshotting a muted session is valid; restore should preserve the muted state exactly.
- An empty snapshot is not an error. It means "no other active sessions to duck".

## Fade Algorithm

The backend should reuse the existing envelope helpers:

- `compute_fade_steps(start, target, config)`

For each session entry:

1. Resolve the live session again by `key`.
2. For each step:
   - call `ISimpleAudioVolume::SetMasterVolume(step.volume, null())`
   - sleep `step.delay_ms`

Important details:

- Use absolute `SetMasterVolume`, not deltas.
- Clamp every written value to `0.0..=1.0`.
- Leave mute unchanged during fade-down unless the snapshot was already muted.

Absolute writes avoid the rounding drift already identified in the Linux PulseAudio backend.

## Restore Algorithm

`fade_restore()` should:

1. Re-enumerate live sessions on the current default multimedia endpoint.
2. Build a lookup from session instance key to `ISimpleAudioVolume` and current session metadata.
3. For each snapshot entry:
   - if the session still exists, fade current volume back to the snapshot volume
   - after the last step, restore `SetMute(snapshot.mute, null())`
4. Ignore missing sessions.

### Why re-enumerate

- COM interface objects should not be kept across async boundaries in the backend struct.
- Sessions can disappear and reappear during playback.
- Re-enumeration keeps the backend `Send + Sync` and matches the short-lived snapshot/restore lifecycle.

### Missing session behavior

If a session no longer exists at restore time, treat it as already restored. The sound source is gone, so there is nothing left to fix.

## Availability Probe

`WindowsBackend::new()` should perform a lightweight probe:

1. initialize COM
2. resolve default render multimedia endpoint
3. activate `IAudioSessionManager2`

If that succeeds, mark `available = true`.

This should not require there to be active sessions. Backend availability means "WASAPI ducking APIs are usable on this machine", not "there is currently something to duck".

## Error Handling Policy

### Hard failures

These should surface as `DuckingError` from `snapshot()`:

- COM/WASAPI initialization failure other than `RPC_E_CHANGED_MODE`
- failure to get the default render endpoint
- failure to activate `IAudioSessionManager2`
- failure to enumerate sessions at all

### Soft failures

These should be skipped per session and optionally logged at debug level later:

- inability to cast one session to `ISimpleAudioVolume`
- missing session instance identifier
- a session disappearing during fade or restore

This keeps Playa's current graceful-degradation model intact.

## Logging and User Diagnostics

### Immediate mitigation

Before the real backend lands, change the current Windows `NoopBackend` path to warn clearly:

```rust
#[cfg(all(target_os = "windows", not(feature = "audio-ducking-windows")))]
{
    eprintln!("Warning: audio ducking is not yet implemented on Windows");
    return Box::new(NoopBackend::new());
}
```

### After backend lands

- `backend_name()` should report `"windows-wasapi"`.
- `playa duck-info` should gain a Windows branch:
    - Strategy: per-application volume control via WASAPI
    - Excludes current-process sessions
    - Ducks only active sessions present when playback begins

## File-Level Change List

### New file

- `playa/lib/src/ducking/windows.rs`

### Updated files

- `playa/lib/Cargo.toml`
- `playa/cli/Cargo.toml`
- `playa/lib/src/ducking/mod.rs`
- `playa/lib/src/ducking/factory.rs`
- `playa/lib/src/ducking/types.rs`
- `playa/lib/src/ducking/tests.rs`
- `playa/lib/src/sfx_player.rs`
- `playa/cli/src/main.rs`
- `playa/docs/audio-ducking.md`

### Optional cleanup in same series

- Factor COM initialization into a shared helper that both `ducking/windows.rs` and `sfx_player.rs` reuse.

## Windows SFX COM Remediation

The WASAPI ducking backend and the existing Windows SFX path overlap at the COM boundary, so they should be designed together.

### Current failure mode

Today `play_sfx_with_category()`:

1. calls `CoInitializeEx(None, COINIT_MULTITHREADED)`,
2. ignores the returned `HRESULT`,
3. continues into WASAPI activation,
4. and never calls `CoUninitialize()`.

This creates two concrete risks:

- on STA threads, `RPC_E_CHANGED_MODE` makes the intent of the call fail, and the code has no explicit policy for that state;
- on threads where COM initialization succeeds, the per-thread COM reference count leaks.

### Design requirement

The SFX path should use the same `ComGuard` abstraction as the ducking backend.

That yields this shape:

```rust
unsafe {
    let _com = ComGuard::new()?;

    let enumerator: IMMDeviceEnumerator =
        CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;
    let device = enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia)?;
    let client: IAudioClient2 = device.Activate(CLSCTX_ALL, None)?;
    // ...
}
```

### Why unify this with ducking

- both code paths are WASAPI-based;
- both need to run correctly on arbitrary caller threads;
- both currently sit in the same crate and use the same `windows` bindings;
- a shared helper reduces the chance that one path handles `RPC_E_CHANGED_MODE` correctly while the other regresses later.

### Placement options

Any of these are acceptable:

- a small private helper inside `sfx_player.rs` and duplicated in `ducking/windows.rs`
- a `playa/lib/src/windows/com.rs` internal module used by both
- a `ducking/windows.rs` helper re-exported privately for the SFX module

The preferred option is a tiny internal shared module so the COM contract has one implementation.

## Pseudocode

```rust
impl DuckingBackend for WindowsBackend {
    fn snapshot(&self) -> DuckResult<'_, VolumeSnapshot> {
        Box::pin(async move {
            let _com = ComGuard::new()?;
            let sessions = enumerate_sessions(self.our_pid)?;
            let entries = sessions
                .into_iter()
                .filter(|s| s.state == Active && s.pid != self.our_pid)
                .map(|s| SessionVolume::new(
                    SessionId::WasapiSession { pid: s.pid, key: s.instance_id },
                    vec![s.volume],
                    s.mute,
                ))
                .collect();
            Ok(VolumeSnapshot::with_entries(entries))
        })
    }

    fn fade_to_floor(&self, snapshot: &VolumeSnapshot, config: &DuckConfig) -> DuckResult<'_, ()> {
        let snapshot = snapshot.clone();
        let config = *config;
        Box::pin(async move {
            let _com = ComGuard::new()?;
            for entry in &snapshot.entries {
                let SessionId::WasapiSession { key, .. } = &entry.id else { continue };
                let Some(session) = find_session_by_key(key) else { continue };
                let start = session.get_master_volume()?;
                let target = entry.channels[0] * config.floor_scalar();
                for step in compute_fade_steps(start, target, &config) {
                    session.set_master_volume(step.volume)?;
                    sleep(step.delay_ms).await;
                }
            }
            Ok(())
        })
    }

    fn fade_restore(&self, snapshot: &VolumeSnapshot, config: &DuckConfig) -> DuckResult<'_, ()> {
        let snapshot = snapshot.clone();
        let config = *config;
        Box::pin(async move {
            let _com = ComGuard::new()?;
            for entry in &snapshot.entries {
                let SessionId::WasapiSession { key, .. } = &entry.id else { continue };
                let Some(session) = find_session_by_key(key) else { continue };
                let current = session.get_master_volume()?;
                let target = entry.channels[0];
                for step in compute_fade_steps(current, target, &config) {
                    session.set_master_volume(step.volume)?;
                    sleep(step.delay_ms).await;
                }
                session.set_mute(entry.mute)?;
            }
            Ok(())
        })
    }
}
```

## Testing Plan

### Unit tests

Add pure-Rust tests for helper logic that does not require a Windows device:

- session filtering excludes `our_pid`
- session filtering ignores inactive and expired sessions
- snapshot-to-lookup matching uses `key`
- restore skips missing sessions without failing

These tests should be built around small helper structs rather than direct COM calls.

### Windows integration tests

Add ignored tests under `#[cfg(all(target_os = "windows", feature = "audio-ducking-windows"))]`:

- can create `WindowsBackend` and report availability
- snapshot succeeds when the default render endpoint exists
- `backend_name()` returns `"windows-wasapi"`
- volume round-trip on a live session if a test session is available

These tests should be best-effort and marked `#[ignore = "requires Windows audio session"]`.

### Windows SFX regression tests

Add Windows-only tests or helper-level tests for COM guard behavior:

- successful `ComGuard::new()` balances `CoInitializeEx` with `CoUninitialize()`
- `RPC_E_CHANGED_MODE` is treated as usable rather than fatal
- the Windows SFX path can still create and tag an `IAudioClient2` stream when COM is already initialized on the thread

Where direct COM-state assertions are awkward, test the helper contract in isolation and keep the end-to-end SFX test ignored and device-dependent.

### Compile coverage

At minimum, CI or local verification should include:

```bash
cargo check -p playa --features audio-ducking-windows
cargo check -p playa-cli --features audio-ducking-windows
```

If cross-compiling is used in CI, add a Windows target check so the backend does not bit-rot on non-Windows development machines.

## Risks and Tradeoffs

### Sessions created after snapshot are not ducked

This is acceptable for v1 and matches the current guard lifecycle. The backend only controls the set of sessions that exist when ducking begins.

### User volume changes during ducking may be overwritten

If a user manually adjusts a ducked application's volume while Playa is active, restore will currently return it to the snapshot value. This is an existing cross-platform limitation.

### Default endpoint may change mid-playback

The backend intentionally reuses the current default multimedia endpoint on each call. If the user changes devices during playback, restore may not find the original session on the new endpoint. Missing sessions should be ignored, not treated as fatal.

## Rollout Plan

### Phase 0

Land the warning-only change so Windows users stop getting silent no-op behavior.

At the same time, patch `sfx_player.rs` to stop discarding `CoInitializeEx` and to balance successful initialization with `CoUninitialize()`.

### Phase 1

Add the real backend behind `audio-ducking-windows` and wire it into `factory.rs`.

If a shared COM helper was not added in Phase 0, add it here and migrate both ducking and SFX to it together.

### Phase 2

Update docs and `duck-info`, then add ignored Windows integration tests.

### Phase 3

Tighten Windows-only tests around COM and WASAPI lifecycle edge cases.

## Recommended Implementation Notes

- Prefer the `windows` crate already in use instead of adding a second WASAPI dependency.
- Keep helpers small and synchronous internally; expose async only at the `DuckingBackend` boundary.
- Centralize `PWSTR` to `String` conversion plus `CoTaskMemFree` in one helper to avoid leaks.
- Use session instance identifiers consistently for both snapshot and restore.
- Do not leave raw `CoInitializeEx` calls at WASAPI call sites once the shared helper exists.

## Acceptance Criteria

The design is complete when the implementation can satisfy all of the following:

- `create_backend()` returns `WindowsBackend` on Windows when `audio-ducking-windows` is enabled and WASAPI is available.
- `backend_name()` returns `"windows-wasapi"` in that configuration.
- `snapshot()` returns active non-self WASAPI sessions with scalar volume and mute state.
- `fade_to_floor()` attenuates those sessions to the configured floor.
- `fade_restore()` restores exact prior volume and mute state for surviving sessions.
- Windows without the feature no longer fail silently; they emit an explicit warning and fall back to `NoopBackend`.
- Windows WASAPI SFX no longer discard `CoInitializeEx` results and no longer leak COM initialization counts.
