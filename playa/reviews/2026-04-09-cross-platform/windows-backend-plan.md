# High-Confidence Plan: Windows WASAPI Ducking Backend

## Goal

Ship a real Windows ducking backend for Playa and fix the existing Windows WASAPI COM lifecycle bug, without changing the public ducking API or the current `DuckGuard` lifecycle.

## Why This Plan Is High Confidence

- The Windows path is already partially in place:
  - `SessionId::WasapiSession { pid, key }` already exists in [playa/lib/src/ducking/types.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/playa/playa/lib/src/ducking/types.rs).
  - The `windows` crate is already configured in [playa/lib/Cargo.toml](/Users/ken/.claudine/worktrees/rusty-biscuit/playa/playa/lib/Cargo.toml).
  - WASAPI endpoint activation already works in [playa/lib/src/sfx_player.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/playa/playa/lib/src/sfx_player.rs).
- The ducking abstraction already matches the proposed backend shape:
  - `snapshot()`
  - `fade_to_floor()`
  - `fade_restore()`
- Existing backends in [playa/lib/src/ducking/macos.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/playa/playa/lib/src/ducking/macos.rs) and [playa/lib/src/ducking/linux.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/playa/playa/lib/src/ducking/linux.rs) provide the implementation pattern to follow.
- The work is additive and localized. No public API redesign is required.

## Decisions To Lock Before Coding

1. Windows ducking ships behind a new `audio-ducking-windows` feature in both the library and CLI.
2. v1 is per-session only. Do not implement endpoint-volume fallback in this change.
3. `SessionId::WasapiSession.key` stores `GetSessionInstanceIdentifier()`, not `GetSessionIdentifier()`.
4. Windows COM initialization must be handled by one shared RAII helper used by both the ducking backend and the Windows SFX path.
5. When `audio-ducking-windows` is not enabled, Windows still returns `NoopBackend`, but `create_backend()` must warn once instead of failing silently.
6. The backend remains effectively stateless. Do not store live COM interfaces in `WindowsBackend`.

## Scope

Files that should change:

- [playa/lib/Cargo.toml](/Users/ken/.claudine/worktrees/rusty-biscuit/playa/playa/lib/Cargo.toml)
- [playa/cli/Cargo.toml](/Users/ken/.claudine/worktrees/rusty-biscuit/playa/playa/cli/Cargo.toml)
- [playa/lib/src/lib.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/playa/playa/lib/src/lib.rs)
- [playa/lib/src/windows_com.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/playa/playa/lib/src/windows_com.rs)
- [playa/lib/src/ducking/mod.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/playa/playa/lib/src/ducking/mod.rs)
- [playa/lib/src/ducking/factory.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/playa/playa/lib/src/ducking/factory.rs)
- [playa/lib/src/ducking/windows.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/playa/playa/lib/src/ducking/windows.rs)
- [playa/lib/src/ducking/types.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/playa/playa/lib/src/ducking/types.rs)
- [playa/lib/src/ducking/tests.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/playa/playa/lib/src/ducking/tests.rs)
- [playa/lib/src/sfx_player.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/playa/playa/lib/src/sfx_player.rs)
- [playa/cli/src/main.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/playa/playa/cli/src/main.rs)
- [playa/docs/audio-ducking.md](/Users/ken/.claudine/worktrees/rusty-biscuit/playa/playa/docs/audio-ducking.md)

Out of scope for this change:

- endpoint-wide Windows ducking fallback
- ducking sessions created after the initial snapshot
- communications ducking APIs
- redesign of `DuckGuard`

## Implementation Plan

1. Add feature wiring and correct the current Windows/noop behavior.

In [playa/lib/Cargo.toml](/Users/ken/.claudine/worktrees/rusty-biscuit/playa/playa/lib/Cargo.toml):
- add `audio-ducking-windows = ["audio-ducking", "windows"]`

In [playa/cli/Cargo.toml](/Users/ken/.claudine/worktrees/rusty-biscuit/playa/playa/cli/Cargo.toml):
- add `audio-ducking-windows = ["audio-ducking", "playa/audio-ducking-windows"]`

In [playa/lib/src/ducking/mod.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/playa/playa/lib/src/ducking/mod.rs):
- add `mod windows;` behind `#[cfg(all(target_os = "windows", feature = "audio-ducking-windows"))]`
- re-export `WindowsBackend` behind the same cfg
- fix the module docs so they no longer claim an endpoint fallback on Windows

In [playa/lib/src/ducking/factory.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/playa/playa/lib/src/ducking/factory.rs):
- instantiate `WindowsBackend` when the new feature is enabled
- return `"windows-wasapi"` from `backend_name()` in that configuration
- keep the existing Windows noop path when the feature is off, but emit a warning once

Use a `static Once` for the warning so repeated playback calls do not spam stderr.

2. Introduce one shared Windows COM helper and migrate all raw `CoInitializeEx` usage to it.

Add [playa/lib/src/windows_com.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/playa/playa/lib/src/windows_com.rs) and wire it from [playa/lib/src/lib.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/playa/playa/lib/src/lib.rs) behind:

```rust
#[cfg(all(
    target_os = "windows",
    any(feature = "sfx-native-windows", feature = "audio-ducking-windows")
))]
mod windows_com;
```

This module should provide:

- `ComGuard::new() -> Result<ComGuard, DuckingError or boxed error>`
- `Drop` that calls `CoUninitialize()` only when initialization returned `S_OK` or `S_FALSE`
- explicit handling for `RPC_E_CHANGED_MODE` as a usable state
- a small helper for converting the returned session identifier string into `String` and freeing the source allocation once, if needed

Behavior to lock in:

- `S_OK`: continue, uninitialize on drop
- `S_FALSE`: continue, uninitialize on drop
- `RPC_E_CHANGED_MODE`: continue, do not uninitialize on drop
- any other HRESULT: fail before using WASAPI

Then update [playa/lib/src/sfx_player.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/playa/playa/lib/src/sfx_player.rs):

- replace the production `CoInitializeEx` call in `windows_sfx::play_sfx_with_category()`
- replace the raw `CoInitializeEx` calls in the ignored Windows tests

This removes the current COM leak and ensures the tests stop encoding the broken pattern.

3. Implement the Windows ducking backend as a stateless WASAPI session backend.

Create [playa/lib/src/ducking/windows.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/playa/playa/lib/src/ducking/windows.rs) with:

```rust
#[derive(Debug)]
pub struct WindowsBackend {
    our_pid: u32,
    available: AtomicBool,
}
```

Backend construction:

- `our_pid = std::process::id()`
- availability probe:
  - create `ComGuard`
  - resolve default multimedia render endpoint
  - activate `IAudioSessionManager2`
  - set `available = true` if that succeeds

Internal helpers should stay synchronous and return plain data structures:

- `enumerate_sessions(our_pid) -> Result<Vec<LiveSession>, DuckingError>`
- `find_session_by_key(key) -> Result<Option<LiveSessionHandle>, DuckingError>` or equivalent
- `session_key_from_control2(...) -> Result<Option<String>, ...>`

Recommended plain-data shape for enumeration:

```rust
struct LiveSession {
    pid: u32,
    key: String,
    volume: f32,
    mute: bool,
}
```

Enumeration rules:

- use `IMMDeviceEnumerator -> GetDefaultAudioEndpoint(eRender, eMultimedia)`
- activate `IAudioSessionManager2`
- enumerate sessions from `GetSessionEnumerator()`
- only keep sessions that are:
  - `AudioSessionStateActive`
  - not owned by `our_pid`
  - expose `ISimpleAudioVolume`
  - have a non-empty session instance identifier
- skip per-session failures instead of failing the whole request

4. Implement `snapshot()`, `fade_to_floor()`, and `fade_restore()` using the existing ducking contract.

`snapshot()`:

- enumerate current sessions
- convert each one into:

```rust
SessionVolume {
    id: SessionId::WasapiSession { pid, key },
    channels: vec![volume],
    mute,
}
```

- return an empty snapshot if nothing else is currently active

`fade_to_floor()`:

- for each snapshot entry, compute target volume as `entry.channels[0] * config.floor_scalar()`
- use `compute_fade_steps(start, target, config)`
- use absolute `SetMasterVolume`, not deltas
- clamp every write to `0.0..=1.0`
- do not change mute during fade-down

`fade_restore()`:

- re-enumerate live sessions by key
- fade current volume back to the snapshot value
- restore mute only after the final volume step
- ignore sessions that disappeared

Implementation note:

- Keep `WindowsBackend` free of stored COM pointers.
- Keep the snapshot keyed by `GetSessionInstanceIdentifier()`.
- If local COM interface values turn out not to be `Send`, do not hold them across `.await` points. Re-resolve by key before each delayed step rather than fighting the trait bound.

5. Update type comments, CLI diagnostics, and docs to match the real design.

In [playa/lib/src/ducking/types.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/playa/playa/lib/src/ducking/types.rs):
- clarify that `WasapiSession.key` stores the session instance identifier
- keep the type itself unchanged

In [playa/cli/src/main.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/playa/playa/cli/src/main.rs):
- add a `windows-wasapi` branch to `print_duck_info()`
- describe the actual behavior:
  - per-application WASAPI ducking
  - excludes current-process sessions
  - only ducks sessions present when playback starts

In [playa/docs/audio-ducking.md](/Users/ken/.claudine/worktrees/rusty-biscuit/playa/playa/docs/audio-ducking.md):
- remove the old Windows endpoint fallback description
- align the feature name with the codebase: `audio-ducking-linux`, not `audio-ducking-pulse`
- document that Windows v1 is per-session only and keyed by session instance identifier

6. Add tests that cover policy and regression points instead of depending entirely on real devices.

In [playa/lib/src/ducking/tests.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/playa/playa/lib/src/ducking/tests.rs):
- add pure-Rust tests for:
  - excluding `our_pid`
  - ignoring inactive sessions
  - key-based snapshot/restore matching
  - skipping missing sessions on restore

In [playa/lib/src/sfx_player.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/playa/playa/lib/src/sfx_player.rs):
- keep the Windows device-dependent tests ignored
- make them use the shared COM helper

In [playa/lib/src/ducking/windows.rs](/Users/ken/.claudine/worktrees/rusty-biscuit/playa/playa/lib/src/ducking/windows.rs):
- add ignored Windows-only smoke tests for:
  - backend availability probe
  - snapshot on a machine with a default render device
  - `backend_name() == "windows-wasapi"` when the feature is enabled

## Recommended Delivery Order

1. Land the shared `ComGuard` and update `sfx_player.rs` first.
2. Add the new feature wiring and Windows noop warning.
3. Add `ducking/windows.rs` and wire it into the factory.
4. Update CLI diagnostics and docs.
5. Add Windows-only ignored tests and any remaining helper tests.

This order reduces risk because the COM fix is independently valuable and gives the new backend a known-good WASAPI entry pattern.

## Validation

Minimum compile coverage:

```bash
cargo check -p playa --features audio-ducking-windows,sfx-native-windows
cargo check -p playa-cli --features audio-ducking-windows,sfx-native-windows
```

Preferred Windows-target compile coverage when available:

```bash
cargo check -p playa --target x86_64-pc-windows-msvc --features audio-ducking-windows,sfx-native-windows
cargo check -p playa-cli --target x86_64-pc-windows-msvc --features audio-ducking-windows,sfx-native-windows
```

Focused test coverage:

```bash
cargo test -p playa --lib
```

Manual Windows smoke checks:

1. `playa duck-info` reports `windows-wasapi` when the feature is enabled.
2. Windows without `audio-ducking-windows` emits a warning once and continues with noop behavior.
3. Windows SFX playback still opens a Sound Effects category stream.
4. With another application actively playing audio, starting Playa ducks that app and restores it afterward.
5. Running from a process or host that initializes COM as STA no longer breaks the Windows SFX path.

## Acceptance Criteria

- `create_backend()` returns `WindowsBackend` on Windows when `audio-ducking-windows` is enabled and the WASAPI probe succeeds.
- `backend_name()` returns `"windows-wasapi"` in that configuration.
- Windows without the feature no longer fail silently; they warn once and fall back to noop.
- `snapshot()` returns active non-self sessions keyed by session instance identifier.
- `fade_to_floor()` ducks those sessions to the configured floor using absolute writes.
- `fade_restore()` restores exact prior volume and mute state for sessions that still exist.
- The Windows SFX path no longer ignores `CoInitializeEx` results and no longer leaks COM initialization counts.
- The docs and CLI output describe the actual shipped Windows behavior, not the older endpoint-fallback design.
