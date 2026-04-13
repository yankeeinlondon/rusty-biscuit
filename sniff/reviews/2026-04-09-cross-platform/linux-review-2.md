# Linux Implementation Review, Pass 2

## Scope

Second-pass review of the Linux work that was updated after `playa/reviews/2026-04-09-cross-platform/linux-review.md`.

Focus areas:

- functionality that was designed but still not implemented
- broken or incomplete implementations
- test coverage gaps
- ergonomic and performance follow-ups

## Local Verification

- `cargo test -p playa --lib --no-default-features --features audio-ducking-linux,sfx-native-linux` on this macOS worktree: passes (`115` tests), but this does **not** compile `#[cfg(target_os = "linux")]` code
- `cargo check -p playa --lib --no-default-features --features audio-ducking-linux --target x86_64-unknown-linux-gnu`: still blocked in this environment by missing Linux cross toolchain / OpenSSL / zlib setup, so Linux-only code was verified primarily by inspection

## Findings

### 1. Linux ducking still does not compile on an actual Linux target

`playa/lib/src/ducking/linux.rs` now contains two identical `percent_to_scalar()` methods in the same `impl LinuxBackend` block:

- `playa/lib/src/ducking/linux.rs:129`
- `playa/lib/src/ducking/linux.rs:138`

That is a hard compile error once the Linux-only module is actually built. The reason it slipped through is that the local feature-enabled test run was performed on macOS, so `#[cfg(target_os = "linux")]` excluded the file entirely.

Recommendation:

- Remove the duplicate method immediately.
- Add a real Linux CI build/test job for `playa` with `audio-ducking-linux` and `sfx-native-linux` enabled.
- Treat macOS-hosted `--features audio-ducking-linux,...` runs as insufficient for Linux verification.

### 2. A Pulse drain timeout can replay the same sound effect twice

`play_sfx()` falls through to another playback path whenever the Linux Pulse path returns `Err`:

- fallback trigger: `playa/lib/src/sfx_player.rs:188-195`
- host-player fallback from `SoundEffect::play_with_options()`: `playa/lib/src/effects.rs:1377-1389`

But `play_sfx_as_event()` writes audio to the Pulse stream **before** waiting for drain completion:

- `stream.write(...)`: `playa/lib/src/sfx_player.rs:943-945`
- drain wait: `playa/lib/src/sfx_player.rs:955-965`

If the server accepted the bytes and only the drain phase times out or errors, the function returns `Err` after playback has already started. The caller then falls back to a second playback path and can replay the same effect.

Recommendation:

- Split Linux Pulse failures into:
  - pre-playback setup failures: safe to fall back
  - post-write completion failures: do **not** fall back to another player
- After a successful `stream.write(...)`, return a terminal playback error rather than triggering a second playback attempt.
- Add a regression test around this state transition by isolating the post-write path behind a small injectable abstraction.

### 3. The Linux backend still ducks inactive/corked sink inputs

The design/docs now say Linux ducking should operate on active sink inputs only, with “active” defined as running or uncorked:

- design doc: `playa/docs/audio-ducking.md:79-83`

The implementation still snapshots every writable sink input:

- `playa/lib/src/ducking/linux.rs:170-176`

There is no filter for `app.corked`, `app.has_volume`, or any equivalent “currently active” condition. That means paused or otherwise inactive applications can still be ducked and later restored even though they were not actually participating in playback.

Recommendation:

- Add a single `is_active_sink_input(&ApplicationInfo)` helper and use it from `snapshot()`.
- At minimum, skip corked inputs.
- Add unit tests for active/inactive filtering and for self-exclusion by PID / application name.

### 4. The Linux test coverage is still materially weaker than the implementation needs

The new tests improved coverage volume, but not enough of the real Linux behavior is pinned down yet.

Examples:

- `playa/lib/src/ducking/tests.rs:584-640` mostly re-derives arithmetic locally instead of exercising `LinuxBackend` helpers or `snapshot()` policy
- `playa/lib/src/ducking/tests.rs:635` produces an `unused_assignments` warning, which is a sign that `linux_multiple_duck_restore_cycles_no_drift` is effectively vacuous
- `playa/lib/src/sfx_player.rs:1014-1066` tests `wait_for_pulse_condition_with_mock`, not `wait_for_pulse_condition()` itself
- the successful macOS-hosted feature test run still does not compile the Linux-only modules

Recommendation:

- Add a Linux CI job that actually builds and tests the Linux cfg slices.
- Replace the mock-only wait-helper tests with tests against the real helper via an injectable iterate/check shim.
- Add focused tests for:
  - duplicate-playback prevention after `stream.write(...)`
  - active/corked sink filtering
  - `should_exclude()` PID/name matching
  - exact-target helpers in `linux.rs`, not just copy-pasted arithmetic in `ducking/tests.rs`

## Ergonomics / Performance Suggestions

- Centralize Linux sink selection into a helper such as `is_target_duck_candidate(&ApplicationInfo)`. Right now the ducking policy is spread across `should_exclude()` plus ad hoc checks in `snapshot()`, which made it easy to leave the active/corked rule unimplemented.
- Split Linux Pulse playback results into an explicit state enum such as `SetupFailed`, `PlaybackStarted`, and `PlaybackCompleted`. That would make the fallback policy much easier to reason about and would prevent the current “drain timeout replays audio” bug.
- Tighten the test suite by moving Linux-specific math tests next to `playa/lib/src/ducking/linux.rs` and making `ducking/tests.rs` focus on cross-platform policy only. That will reduce duplication and make Linux regressions easier to spot.
