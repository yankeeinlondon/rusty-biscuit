# Playa stuck-device resilience

Date: 2026-04-29
Owner: Ken Snyder
Status: Approved (brainstorm) — pending spec review

## Background

The claudine integration test `dispatch_sound_effect_action` (in
`claudine/lib/tests/canonical_dispatch.rs`) was observed taking several
minutes on a developer macOS machine where the audio system had wedged.
Investigation traced the hang to two compounding factors:

1. The dispatch runner's `execute_sound_effect`
   (`claudine/lib/src/dispatch/runner.rs`) drives real CoreAudio playback
   through `playa::Playa::from_bytes(...).play()` even when the test's
   intent is to verify dispatch behavior, not playback. The work runs on
   a tokio blocking-pool thread; the `#[tokio::test]` runtime drop waits
   on outstanding blocking tasks, so a stuck device blocks the test.
2. `playa::native_player::wait_with_timeout` polls `Player::empty()`
   with a 300 second wall-clock ceiling and no progress detector. A
   wedged CoreAudio session never drains, so playback waits the full
   300 seconds before giving up.

A separately reported pattern — the host audio system gradually
becoming unresponsive every few days, recoverable via reboot — is
hypothesized to be caused by rapid open/close churn of
`rodio::MixerDeviceSink` objects against `coreaudiod`. Each sound
effect today opens a fresh device sink, plays a sub-second clip, and
drops the sink. Frequent re-open against `coreaudiod` is a known
sensitivity on macOS.

## Goals

- The dispatch test must complete in well under one second regardless
  of the host audio system state.
- Native playback must abandon a wedged device in single-digit seconds,
  not 300 seconds.
- Once a device-stall has been detected, the rest of the process must
  not retry native playback.
- Reduce CoreAudio device-open churn so the host audio system does not
  drift toward an unrecoverable state during normal claudine use.

## Non-goals

- Replacing rodio. The fixes work within the current rodio 0.22 API.
- Changing the host-player fallback path beyond what is required for
  the breaker to route to it.
- Auditing or rewriting audio ducking.
- Multi-device sink caching (kept explicitly out of scope; see Fix 3).

## Fix 1 — Test no longer triggers real playback

### Mechanism

Add a process-wide dry-run mode to `playa::Playa`:

- Builder method `Playa::dry_run(self) -> Self` sets an in-struct flag.
- Environment variable `PLAYA_DRY_RUN=1` is read once per `play()` /
  `play_async()` call. When set (or when the builder flag is true),
  both methods log at `debug!` level and return `Ok(())` without
  opening any device, decoding any bytes, or spawning any subprocess.
- The dry-run check runs before native and host paths, before ducking
  setup, and before any device lookup.

The env var is the recommended trigger for tests and CI; the builder
method exists so library consumers can enable dry-run programmatically
without touching the process environment.

### Test changes

`claudine/lib/tests/canonical_dispatch.rs`:

- Add a setup helper that sets `PLAYA_DRY_RUN=1` for the duration of
  the test using `serial_test::serial` to guard against parallel-test
  interleaving with other audio-touching tests in the same binary.
- Apply the helper to `dispatch_sound_effect_action` and any other
  test in the file that may dispatch a `SoundEffect` action.
- The assertion surface does not change. The test continues to verify
  `outcome.exit_code`, `protect_pre`, and `protect_post`.

### Why not dependency-injection refactor?

Making `execute_sound_effect` injectable through the runtime config
would touch every dispatch test and the runtime construction surface.
The env var is one line in playa, useful beyond this single test
(headless CI, sandboxed builds), and self-documenting.

## Fix 2 — Bound playback wait by lack of progress

### Mechanism

Replace `wait_with_timeout` in `playa/lib/src/native_player.rs` with a
two-tier deadline:

- **Absolute ceiling**: keep the existing 300 second `PLAYBACK_TIMEOUT`
  as a safety backstop.
- **Stall window**: default 5 seconds. If `Player::get_pos()` does not
  advance for the stall window while `!player.empty()`, treat the
  device as wedged.

Configuration:

- `PLAYA_PLAYBACK_STALL_SECONDS` env var overrides the default. Parsed
  once at the start of each `wait_with_progress` call. Invalid values
  fall back to the default with a single `warn!`.

Behavior on stall:

1. Call `player.stop()`.
2. Call `trip_native_audio_breaker(NativeAudioFailureKind::DeviceOpenTimeout)`
   so subsequent native attempts in this process route directly to the
   host fallback.
3. Return `NativePlaybackError::Timeout(stall_seconds)` with an
   `eprintln!` matching the existing message style.

### Naming and surface

- Rename `wait_with_timeout` to `wait_with_progress` to reflect that it
  is now progress-aware rather than purely time-bounded.
- Polling cadence remains 50 ms.
- The breaker reason variant is reused — no new
  `NativeAudioFailureKind` variant is added in this spec.

### Tests

- Unit test in `native_player.rs` using a mock player abstraction or a
  fake whose `get_pos()` is controllable. Acceptable alternative: a
  small trait extracted for testability, with the production impl
  delegating to `rodio::Player`.
- Tests must cover: progress advancing (succeeds), progress static
  (stall trip), absolute deadline reached (existing path).

## Fix 3 — Cached output sink to reduce CoreAudio churn

### Mechanism

Introduce a process-local lazy default-device sink:

- A `static SHARED_DEFAULT_SINK: OnceLock<Mutex<Option<Arc<MixerDeviceSink>>>>`
  (or equivalent) in `playa/lib/src/native_player.rs`.
- First native playback through the default-device path opens the sink
  and stores it.
- Subsequent default-device calls call
  `Player::connect_new(sink.mixer())` against the cached sink.
- `Player` instances themselves are still short-lived per call; only
  the underlying device sink is reused.

### Scope limits

- **Default device only.** Calls with `options.channel = Some(...)` go
  through the existing one-shot open path. Multi-device caching is out
  of scope for this spec; revisit if a measurable need appears.
- **No eviction.** The cache lives for the process lifetime. Closing
  and re-opening a long-lived `MixerDeviceSink` defeats the purpose.
- **First-open failure.** If the initial sink open fails, the cache is
  not poisoned — the slot remains `None`, the call falls through to
  host playback as today, and the next call retries the open.

### Interaction with existing breaker

The native breaker is unchanged. If a stall trips the breaker (Fix 2),
all subsequent native paths short-circuit before touching the sink
cache, which is the desired behavior.

### Volume, speed, ducking

- Volume and speed are `Player`-level settings; per-call configuration
  continues to work because each call creates a fresh `Player`.
- Ducking wraps the native/host decision in `play_async` and is not
  affected by sink reuse.

### Tests

- Unit test that two consecutive native playbacks reuse the cached
  sink. Acceptable surface: an internal `cached_default_sink_ptr()`
  helper (test-only) that exposes the underlying `Arc` pointer for
  identity comparison.
- Existing `play_native_*` error-path tests continue to pass.

## Out of scope

- Multi-device sink caching.
- Replacing the 300 second absolute ceiling.
- Persisting native breaker state across processes.
- Adding a new public API for explicit "warm up" or "release" of the
  sink — the cache is implicit.
- Migrating away from `tokio::task::spawn_blocking` in claudine. The
  blocking-task interaction with tokio runtime drop becomes irrelevant
  once Fix 1 lands.

## Order of work

1. Fix 1 (`Playa::dry_run` + env var, test wired up).
2. Fix 2 (progress-aware wait + env-configurable stall window).
3. Fix 3 (cached default sink).

Each fix is independently shippable and can be reviewed/landed
separately.

## Open questions

None at the time of writing. Re-evaluate after Fix 3 lands; if audio
corruption persists, add tracing spans around native device open and
close to capture cumulative open count and lifetime distribution
before designing further changes.
