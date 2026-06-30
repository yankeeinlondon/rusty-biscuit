# Linux Implementation Review

## Scope

Reviewed the implementation against `playa/reviews/2026-04-09-cross-platform/linux-plan.md`, with extra attention to Linux-only correctness, fallback behavior, and test coverage.

Local verification performed:

- `cargo test -p playa --lib` on the current macOS worktree: passes (`41` tests)
- `cargo test -p playa --lib ducking::tests`: matches `0` tests on this host
- `cargo test -p playa --lib sfx_player`: matches `0` tests on this host
- `cargo check -p playa --lib --target x86_64-unknown-linux-gnu --features audio-ducking-linux,sfx-native-linux`: blocked by missing cross C toolchain (`x86_64-linux-gnu-gcc`)

That means the Linux-gated code was reviewed primarily by inspection rather than by executing it on Linux.

## Findings

### 1. `LinuxBackend` does not compile as written on Linux

`CachedPulseVolume` is not `Clone`, but both fade paths clone `self.cached_volumes`:

- `playa/lib/src/ducking/linux.rs:31`
- `playa/lib/src/ducking/linux.rs:196`
- `playa/lib/src/ducking/linux.rs:249`

`MutexGuard<HashMap<u32, CachedPulseVolume>>::clone()` requires `CachedPulseVolume: Clone`. As written, the Linux ducking backend will fail to compile once the `audio-ducking-linux` cfg is actually built.

Recommendation:

- Derive or implement `Clone` for `CachedPulseVolume`.
- Add at least one Linux-target compile check in CI so Linux-only cfg breakage is caught immediately.

### 2. The PulseAudio stream-ready phase does not get its own full timeout budget

The plan explicitly called for separate `NATIVE_DEVICE_TIMEOUT` budgets for context readiness and stream readiness. The implementation computes one `ready_deadline` and reuses it for both waits:

- planned behavior: `playa/reviews/2026-04-09-cross-platform/linux-plan.md:27-30`
- current implementation: `playa/lib/src/sfx_player.rs:901-950`

If the context connection consumes most of the first 5 seconds, the stream connection gets only the remainder. On a slow or contended Pulse server, that can produce false `"PulseAudio stream connection timed out"` failures even though the stream itself was still within the intended budget.

Recommendation:

- Split this into `context_deadline` and `stream_deadline`, each created immediately before its own wait.

### 3. The Linux ducking "exact snap" work is only partially implemented

The plan/design called for the final duck/restore step to be driven from cached raw Pulse units, plus a helper that reports write failures. The current code still falls short in two ways:

- Final ducking still derives the target from the float snapshot value instead of from cached raw units: `playa/lib/src/ducking/linux.rs:212-224`
- `apply_volume_delta()` swallows all controller lookup/write errors and returns `()`, so duck/restore can report success even when no Pulse write actually succeeded: `playa/lib/src/ducking/linux.rs:316-326`

The restore side is closer to the intended design than before, but the duck side is still float-derived, and silent write failures make the backend hard to trust or diagnose.

Recommendation:

- Compute the final duck target from `cached.avg_units` directly.
- Change `apply_volume_delta()` to return `Result<(), DuckingError>`.
- Bubble write failures up to the fade methods, or at minimum log them once.

### 4. Test coverage for the Linux work is still too light

The new code is not strongly pinned down yet:

- The package has `default = []`, so ordinary `cargo test -p playa --lib` does not exercise the Linux ducking or Linux SFX code at all: `playa/lib/Cargo.toml:6-27`
- The new wait-helper unit tests do not call `wait_for_pulse_condition()`; they test a mock helper with similar logic instead: `playa/lib/src/sfx_player.rs:1028-1084`
- The Linux policy tests in `playa/lib/src/ducking/tests.rs` mostly recompute math locally rather than exercising `LinuxBackend` helpers or the write-targeting path directly: `playa/lib/src/ducking/tests.rs:557-639`

Given that this change set is specifically about Linux-only hangs and restore correctness, stronger Linux-targeted testing is still missing.

Recommendation:

- Add at least one CI job that builds/tests the Linux feature set on Linux:
  `cargo test -p playa --features audio-ducking-linux,sfx-native-linux`
- Replace the mock wait-helper tests with tests against the real helper via a minimal injectable iterate strategy, or add a small abstraction that makes the real helper unit-testable.
- Add pure tests around "target percent from cached raw units" and "write failure propagates" behavior.

## Incomplete Cleanup

The CLI still carries a `linux-alsa` diagnostic branch even though the plan explicitly removed ALSA from the normal backend-selection story:

- planned cleanup: `playa/reviews/2026-04-09-cross-platform/linux-plan.md:174-180`
- current code: `playa/cli/src/main.rs:877-882`

This is low severity because `backend_name()` no longer returns `linux-alsa` by default, but it is still incomplete cleanup and leaves dead diagnostic code behind.

Recommendation:

- Remove the `linux-alsa` branch from `print_duck_info()` unless there is a concrete manual selection path that can still surface it.

## Ergonomics / Performance Suggestions

- Raise the `wait_for_pulse_condition()` sleep interval from `1ms` to something closer to the design sketch (`10ms`) unless profiling shows that the tighter loop is materially needed. The current loop is bounded, but it is still a fairly aggressive busy poll for waits that can last up to 30 seconds.
- Centralize the Linux Pulse target-volume math in one helper that accepts cached units plus a desired scalar. Right now the float-to-percent math is duplicated across snapshot, duck, and restore paths, which makes regressions like the reused deadline and float-derived final duck step easier to introduce.
