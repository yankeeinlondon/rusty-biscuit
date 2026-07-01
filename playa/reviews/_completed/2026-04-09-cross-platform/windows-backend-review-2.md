# Windows Backend Implementation Review 2

## Validation Performed

- Ran `cargo test -p playa --lib`
- Ran `cargo test -p playa --lib --features audio-ducking`
- Installed the Rust target with `rustup target add x86_64-pc-windows-msvc`
- Attempted `cargo check -p playa --target x86_64-pc-windows-msvc --features audio-ducking-windows,sfx-native-windows`

The native Playa test suites passed. The Windows-target `cargo check` still could not complete in this environment because transitive C dependencies require a real Windows SDK / MSVC-capable cross toolchain (`windows.h`, `stdio.h`, `sys/types.h` were unavailable), so the Windows-only findings below combine local verification with static review of the target-gated code.

## Findings

1. High: the fade and restore loops are serialized per session, so playback startup and restoration time scale linearly with the number of active apps instead of respecting `ramp_ms`.

`playa/lib/src/ducking/windows.rs:321`, `playa/lib/src/ducking/windows.rs:334`, `playa/lib/src/ducking/windows.rs:360`, `playa/lib/src/ducking/windows.rs:373`, `playa/lib/src/ducking/guard.rs:75`, `playa/lib/src/ducking/guard.rs:78`

`DuckGuard::new()` waits for `fade_to_floor()` to finish before playback begins. In the Windows backend, `fade_to_floor_blocking()` fully fades session A, then fully fades session B, and so on. With the default 1000 ms ramp, 5 active sessions means roughly 5 seconds of pre-playback delay; restore has the same problem on the way back up.

That is a real functional gap from the design. The intended behavior is one global ramp window where all target sessions move together. The current implementation turns `ramp_ms` into “per session ramp” instead of “overall ducking ramp”.

Recommended fix:

- Precompute the fade steps for each snapshotted session.
- Iterate by step index first, applying that step to every live session in the map.
- Sleep once per step, not once per session.

1. Medium: write failures during ducking and restore are silently discarded, so broken Windows ducking can report success even when no volume changes were applied.

`playa/lib/src/ducking/windows.rs:336`, `playa/lib/src/ducking/windows.rs:375`, `playa/lib/src/ducking/windows.rs:379`

All `SetMasterVolume` and `SetMute` results are ignored with `let _ = ...`. If the COM object goes stale, the session rejects writes, or restore fails for just one session, the backend still returns `Ok(())`. That defeats the crate’s graceful-degradation model because callers only log and skip ducking when an actual error is returned.

Skipping vanished sessions is correct. Silently swallowing setter failures on still-resolved sessions is not. It makes the backend look healthy while doing nothing.

Recommended fix:

- Continue best-effort across sessions, but count write failures.
- Return `FadeFailed` / `RestoreFailed` when any resolved session could not be updated, ideally with enough detail to identify whether the problem was volume write failure, mute restore failure, or both.

1. Medium: the added test coverage is much broader than in the first review, but it still does not exercise the real Windows backend logic that is most likely to regress.

`playa/lib/src/ducking/tests.rs:147`, `playa/lib/src/ducking/windows.rs:385`, `playa/lib/src/windows_com.rs:163`

Most of the new “Windows policy” tests are value-level assertions over hand-built `VolumeSnapshot` data. They do not execute:

- `enumerate_sessions()`
- `build_volume_map()`
- `fade_to_floor_blocking()`
- `fade_restore_blocking()`
- failure propagation from `SetMasterVolume` / `SetMute`
- synchronized multi-session ramp timing

The Windows-only tests in `windows.rs` are all ignored smoke tests against live hardware, which is useful, but they are too coarse to catch the two issues above. The `windows_com.rs` tests also validate `ComInitKind` classification directly, not the behavior of `ComGuard::new()` against the actual `CoInitializeEx` result mapping.

Recommended fix:

- Extract pure helpers for multi-session fade scheduling and session filtering so they can be unit-tested without COM.
- Add unit tests that assert one `ramp_ms` window is shared across multiple sessions.
- Add a small injectable write interface around volume/mute setters so failure propagation can be tested without a live WASAPI device.

## Resolved Since The First Review

- The `PWSTR` conversion issue appears fixed via the shared helper in `playa/lib/src/windows_com.rs`.
- COM state is no longer held across `.await` points; the fade paths now run inside `spawn_blocking`.
- The shared COM helper is wired into both the Windows ducking path and the Windows SFX path.

## Overall

The core Windows backend is now present and the major correctness issues from the first review were addressed. The main remaining problem is that the fade algorithm is still shaped incorrectly for multi-session ducking, and the tests are not yet exercising the concrete backend paths strongly enough to prevent that kind of regression.
