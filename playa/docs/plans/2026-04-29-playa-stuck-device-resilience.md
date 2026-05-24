# Playa stuck-device resilience — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make playa survive a wedged macOS audio device in seconds, not minutes; eliminate device-open churn; and unblock the dispatch test from real playback.

**Architecture:** Add a `PLAYA_DRY_RUN` short-circuit so tests never touch the audio device. Replace the time-only playback wait with a progress-aware wait that uses `rodio::Player::get_pos()` and trips the existing native breaker on stall. Cache the default `MixerDeviceSink` for the process so per-effect playback reuses one CoreAudio stream.

**Tech Stack:** Rust 2024, rodio 0.22, tokio 1.x, serial_test 3, claudine integration tests via `cargo test -p claudine`.

**Spec:** [`playa/docs/specs/2026-04-29-playa-stuck-device-resilience.md`](../specs/2026-04-29-playa-stuck-device-resilience.md)

---

## File Map

- Modify: `playa/lib/src/playa.rs` — add `dry_run()` builder method, dry-run check at top of `play()` and `play_async()`.
- Modify: `playa/lib/src/native_player.rs` — rename `wait_with_timeout` → `wait_with_progress` with progress detection, add `SHARED_DEFAULT_SINK` cache and route default-device playback through it.
- Modify: `claudine/lib/tests/canonical_dispatch.rs` — set `PLAYA_DRY_RUN=1` for `dispatch_sound_effect_action` and any other test that dispatches a `SoundEffect`, guarded by `serial_test::serial`.

No new files. No public API breakage outside the new `Playa::dry_run()` method.

---

## Task 1: `Playa::dry_run()` builder + env var

**Files:**
- Modify: `playa/lib/src/playa.rs`

- [ ] **Step 1.1: Read the current Playa struct and `play()` body**

Run: `sed -n '36,170p' playa/lib/src/playa.rs`

Expected: see the `Playa` struct fields, builder methods, and the start of `play()`.

- [ ] **Step 1.2: Add the `dry_run` field and helper**

Edit the `Playa` struct in `playa/lib/src/playa.rs` to add a new field:

```rust
#[derive(Debug, Clone)]
pub struct Playa {
    audio: Audio,
    options: PlaybackOptions,
    show_meta: bool,
    force_host: bool,
    dry_run: bool,
    #[cfg(feature = "audio-ducking")]
    duck_config: Option<DuckConfig>,
}
```

Update `Playa::new` to default `dry_run: false`:

```rust
pub fn new(audio: Audio) -> Self {
    Self {
        audio,
        options: PlaybackOptions::default(),
        show_meta: false,
        force_host: false,
        dry_run: false,
        #[cfg(feature = "audio-ducking")]
        duck_config: None,
    }
}
```

Add a free helper at the top of the file (under the existing `use` block):

```rust
/// Returns `true` if the process-wide dry-run env var is enabled.
fn dry_run_env_enabled() -> bool {
    matches!(
        std::env::var("PLAYA_DRY_RUN").as_deref(),
        Ok("1") | Ok("true") | Ok("TRUE") | Ok("yes")
    )
}
```

Add a builder method on `Playa` (place it near `force_host`):

```rust
/// Skip all audio output. `play()` and `play_async()` log at debug
/// and return `Ok(())` without opening any device, decoding any
/// bytes, or spawning any subprocess.
///
/// Equivalent to setting the `PLAYA_DRY_RUN=1` environment variable.
/// Useful in tests, headless CI, and sandboxed builds.
pub fn dry_run(mut self) -> Self {
    self.dry_run = true;
    self
}
```

- [ ] **Step 1.3: Wire the dry-run check into `play()`**

Edit `play()` in `playa/lib/src/playa.rs` to short-circuit before native and host paths:

```rust
pub fn play(self) -> Result<(), PlaybackError> {
    if self.dry_run || dry_run_env_enabled() {
        tracing::debug!("playa: dry-run enabled, skipping playback");
        return Ok(());
    }

    let format = self.audio.format();
    // ... existing body unchanged ...
}
```

If `tracing` is not already imported in `playa.rs`, add `use tracing;` to the top of the file. Verify with: `grep -n '^use ' playa/lib/src/playa.rs | head`.

- [ ] **Step 1.4: Wire the dry-run check into `play_async()`**

Edit `play_async()` in `playa/lib/src/playa.rs` to add the same short-circuit before ducking setup:

```rust
#[cfg(feature = "audio-ducking")]
pub async fn play_async(self) -> Result<(), PlaybackError> {
    if self.dry_run || dry_run_env_enabled() {
        tracing::debug!("playa: dry-run enabled, skipping async playback");
        return Ok(());
    }

    let format = self.audio.format();
    // ... existing body unchanged ...
}
```

- [ ] **Step 1.5: Add a unit test for dry-run**

Append to the bottom of `playa/lib/src/playa.rs` (inside the existing `#[cfg(test)] mod tests` if one exists, otherwise create one):

```rust
#[cfg(test)]
mod dry_run_tests {
    use super::*;

    #[test]
    fn builder_dry_run_skips_playback() {
        // Use bytes that would fail to decode if we actually tried.
        let bogus = vec![0u8; 4];
        let result = Playa::from_bytes(bogus)
            .expect("Playa::from_bytes should accept any bytes")
            .dry_run()
            .play();
        assert!(result.is_ok(), "dry-run play should succeed: {result:?}");
    }

    #[test]
    fn env_var_dry_run_skips_playback() {
        // Safety: tests that read this env var must run serially.
        // This test sets it and unsets it within the same test.
        // SAFETY: std::env::set_var is safe within a single-threaded test scope.
        unsafe {
            std::env::set_var("PLAYA_DRY_RUN", "1");
        }
        let bogus = vec![0u8; 4];
        let result = Playa::from_bytes(bogus)
            .expect("Playa::from_bytes should accept any bytes")
            .play();
        unsafe {
            std::env::remove_var("PLAYA_DRY_RUN");
        }
        assert!(result.is_ok(), "env var dry-run play should succeed: {result:?}");
    }
}
```

Note: if `Playa::from_bytes` rejects 4 zero bytes due to format detection, replace with a tiny valid WAV header. Check by running the test first.

- [ ] **Step 1.6: Run the dry-run unit tests**

Run: `cargo test -p playa --lib dry_run_tests`

Expected: 2 passed. If `from_bytes` rejects 4 bytes, look at the `Audio::from_bytes` error and substitute a 44-byte minimal WAV header:

```rust
const MIN_WAV: [u8; 44] = [
    b'R', b'I', b'F', b'F', 36, 0, 0, 0, b'W', b'A', b'V', b'E',
    b'f', b'm', b't', b' ', 16, 0, 0, 0, 1, 0, 1, 0,
    0x44, 0xAC, 0, 0, 0x88, 0x58, 0x01, 0, 2, 0, 16, 0,
    b'd', b'a', b't', b'a', 0, 0, 0, 0,
];
```

- [ ] **Step 1.7: Commit**

```bash
git add playa/lib/src/playa.rs
git commit -m "feat(playa): add dry_run() builder and PLAYA_DRY_RUN env var"
```

---

## Task 2: Wire `PLAYA_DRY_RUN` into the dispatch test

**Files:**
- Modify: `claudine/lib/tests/canonical_dispatch.rs`

- [ ] **Step 2.1: Inspect the existing imports and tests**

Run: `sed -n '1,55p' claudine/lib/tests/canonical_dispatch.rs`

Expected: see the test file's imports and the `dispatch_sound_effect_action` test currently at lines 25-52.

- [ ] **Step 2.2: Add `serial_test` import and env-guard helper**

Edit `claudine/lib/tests/canonical_dispatch.rs`. Add to the top of the file after the existing `use` block:

```rust
use serial_test::serial;

/// RAII guard that sets `PLAYA_DRY_RUN=1` for its lifetime.
///
/// Pair with `#[serial]` so concurrent tests cannot observe inconsistent
/// env-var state.
struct PlayaDryRunGuard;

impl PlayaDryRunGuard {
    fn enable() -> Self {
        // SAFETY: tests using this guard are marked #[serial] so no
        // parallel test reads or writes the env var concurrently.
        unsafe {
            std::env::set_var("PLAYA_DRY_RUN", "1");
        }
        Self
    }
}

impl Drop for PlayaDryRunGuard {
    fn drop(&mut self) {
        // SAFETY: same as `enable` above.
        unsafe {
            std::env::remove_var("PLAYA_DRY_RUN");
        }
    }
}
```

- [ ] **Step 2.3: Apply guard + `#[serial]` to `dispatch_sound_effect_action`**

Replace the current `dispatch_sound_effect_action` test (lines 25-52) with:

```rust
/// Dispatching a `SoundEffect` action completes without error and returns
/// no blocking response.  Real audio playback is suppressed via
/// `PLAYA_DRY_RUN=1` so a wedged audio device cannot stall the test.
#[tokio::test]
#[serial]
async fn dispatch_sound_effect_action() {
    let _guard = PlayaDryRunGuard::enable();

    let runtime = make_config_with_action(
        AgenticEvent::HumanInTheLoop,
        HookAction::SoundEffect {
            effect: "confirmation".to_string(),
            volume: 0.0,
            speed: 1.0,
        },
    );

    let meta = EventMeta::new(Provider::Claude, AgenticEvent::HumanInTheLoop);

    let outcome = dispatch_canonical_with_runtime(
        Provider::Claude,
        AgenticEvent::HumanInTheLoop,
        meta,
        &runtime,
    )
    .await
    .unwrap();

    // HumanInTheLoop is non-blocking for the Claude adapter, so no exit code.
    assert_eq!(outcome.exit_code, None);
    assert!(outcome.protect_pre.is_none());
    assert!(outcome.protect_post.is_none());
}
```

- [ ] **Step 2.4: Audit other tests in the file for `SoundEffect` dispatch**

Run: `grep -n 'SoundEffect\|sound_effect' claudine/lib/tests/canonical_dispatch.rs`

For each test that constructs a `HookAction::SoundEffect` AND actually dispatches it (not just constructs the runtime), apply the same `#[serial]` + `PlayaDryRunGuard::enable()` pattern. Tests that only construct a runtime without dispatching the event do not need the guard.

Specifically check:
- `dispatch_no_binding_returns_default` (lines 56-93): the binding exists for `HumanInTheLoop` but the dispatch is for `SessionStart`, which does not run any action — no guard needed.
- `dispatch_empty_actions_returns_non_blocking_ack` (line 96+): empty actions, no guard needed.

If you find additional tests that DO dispatch a sound effect, add the guard and `#[serial]` to each.

- [ ] **Step 2.5: Run the dispatch test and confirm it completes quickly**

Run: `cargo test -p claudine --test canonical_dispatch dispatch_sound_effect_action -- --nocapture`

Expected: passes in well under 1 second. If it still hangs, run with `RUST_LOG=playa=debug` and confirm the dry-run debug line is emitted.

- [ ] **Step 2.6: Commit**

```bash
git add claudine/lib/tests/canonical_dispatch.rs
git commit -m "test(claudine): use PLAYA_DRY_RUN to skip real audio in dispatch test"
```

---

## Task 3: Progress-aware native playback wait — extract trait

**Files:**
- Modify: `playa/lib/src/native_player.rs`

This task introduces a small abstraction so the wait loop can be unit-tested without opening a real audio device. It does not yet change behavior.

- [ ] **Step 3.1: Inspect the current wait function**

Run: `sed -n '225,245p' playa/lib/src/native_player.rs`

Expected: see `wait_with_timeout(player: &Player, timeout: Duration)`.

- [ ] **Step 3.2: Define a `PlayerProgress` trait**

In `playa/lib/src/native_player.rs`, add after the imports:

```rust
/// Minimal player surface used by the progress-aware wait loop.
///
/// Extracted as a trait so the wait logic can be tested with a fake
/// player whose progress and emptiness are scriptable.
trait PlayerProgress {
    /// Returns true once all queued audio has finished playing.
    fn empty(&self) -> bool;
    /// Returns the current playback position.
    fn get_pos(&self) -> Duration;
    /// Stops playback and clears the queue.
    fn stop(&self);
}

impl PlayerProgress for rodio::Player {
    fn empty(&self) -> bool {
        rodio::Player::empty(self)
    }
    fn get_pos(&self) -> Duration {
        rodio::Player::get_pos(self)
    }
    fn stop(&self) {
        rodio::Player::stop(self);
    }
}
```

- [ ] **Step 3.3: Add a stall-window helper**

Add to `playa/lib/src/native_player.rs` near the existing `PLAYBACK_TIMEOUT` constant:

```rust
/// Default time without forward playback progress before the device is
/// considered wedged.
const DEFAULT_STALL_WINDOW: Duration = Duration::from_secs(5);

/// Reads `PLAYA_PLAYBACK_STALL_SECONDS` from the environment.
///
/// Returns the parsed duration, or `DEFAULT_STALL_WINDOW` if the var is
/// unset, empty, or invalid.  Invalid values emit a single `warn!`.
fn resolved_stall_window() -> Duration {
    match std::env::var("PLAYA_PLAYBACK_STALL_SECONDS") {
        Ok(raw) if !raw.is_empty() => match raw.parse::<u64>() {
            Ok(secs) if secs > 0 => Duration::from_secs(secs),
            _ => {
                tracing::warn!(
                    raw = %raw,
                    "PLAYA_PLAYBACK_STALL_SECONDS is not a positive integer; using default"
                );
                DEFAULT_STALL_WINDOW
            }
        },
        _ => DEFAULT_STALL_WINDOW,
    }
}
```

If `tracing` is not already imported in this file, add `use tracing;` to the top.

- [ ] **Step 3.4: Commit the scaffolding**

```bash
git add playa/lib/src/native_player.rs
git commit -m "refactor(playa): extract PlayerProgress trait for testable wait loop"
```

---

## Task 4: Progress-aware wait — replace `wait_with_timeout`

**Files:**
- Modify: `playa/lib/src/native_player.rs`

- [ ] **Step 4.1: Write failing tests for the new wait logic**

Append to the `#[cfg(test)] mod tests` block in `playa/lib/src/native_player.rs`:

```rust
mod progress_wait {
    use super::*;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

    struct FakePlayer {
        empty: AtomicBool,
        pos_micros: AtomicU64,
        stop_count: Mutex<u32>,
    }

    impl FakePlayer {
        fn new() -> Self {
            Self {
                empty: AtomicBool::new(false),
                pos_micros: AtomicU64::new(0),
                stop_count: Mutex::new(0),
            }
        }
        fn set_empty(&self) {
            self.empty.store(true, Ordering::SeqCst);
        }
        fn advance_pos(&self, by: Duration) {
            self.pos_micros
                .fetch_add(by.as_micros() as u64, Ordering::SeqCst);
        }
        fn stops(&self) -> u32 {
            *self.stop_count.lock().unwrap()
        }
    }

    impl PlayerProgress for FakePlayer {
        fn empty(&self) -> bool {
            self.empty.load(Ordering::SeqCst)
        }
        fn get_pos(&self) -> Duration {
            Duration::from_micros(self.pos_micros.load(Ordering::SeqCst))
        }
        fn stop(&self) {
            *self.stop_count.lock().unwrap() += 1;
        }
    }

    #[test]
    fn returns_ok_when_player_empties() {
        let _guard = crate::native_audio::lock_native_audio_test_state();
        let fake = FakePlayer::new();
        fake.set_empty();
        let result = wait_with_progress(
            &fake,
            Duration::from_secs(60),
            Duration::from_millis(100),
            Duration::from_millis(1),
        );
        assert!(result.is_ok());
        assert_eq!(fake.stops(), 0);
        assert!(crate::native_audio::native_audio_available());
    }

    #[test]
    fn trips_breaker_on_stall() {
        let _guard = crate::native_audio::lock_native_audio_test_state();
        let fake = FakePlayer::new();
        // Position never advances and player never empties.
        let result = wait_with_progress(
            &fake,
            Duration::from_secs(60),
            Duration::from_millis(50),
            Duration::from_millis(1),
        );
        assert!(matches!(result, Err(NativePlaybackError::Timeout(_))));
        assert_eq!(fake.stops(), 1);
        assert!(!crate::native_audio::native_audio_available());
    }

    #[test]
    fn absolute_deadline_takes_precedence() {
        let _guard = crate::native_audio::lock_native_audio_test_state();
        let fake = FakePlayer::new();
        // Stall window > absolute timeout, so the absolute deadline fires first.
        let result = wait_with_progress(
            &fake,
            Duration::from_millis(30),
            Duration::from_secs(60),
            Duration::from_millis(1),
        );
        assert!(matches!(result, Err(NativePlaybackError::Timeout(_))));
        assert_eq!(fake.stops(), 1);
        assert!(!crate::native_audio::native_audio_available());
    }

    #[test]
    fn progress_resets_stall_clock() {
        let _guard = crate::native_audio::lock_native_audio_test_state();
        let fake = FakePlayer::new();
        // Spawn a thread that advances pos, then sets empty before stall fires.
        let handle = {
            let fake_ptr = &fake as *const FakePlayer as usize;
            std::thread::spawn(move || {
                // SAFETY: main thread holds the FakePlayer alive for the
                // duration of this thread via join() below.
                let fake = unsafe { &*(fake_ptr as *const FakePlayer) };
                std::thread::sleep(Duration::from_millis(20));
                fake.advance_pos(Duration::from_millis(10));
                std::thread::sleep(Duration::from_millis(20));
                fake.advance_pos(Duration::from_millis(10));
                std::thread::sleep(Duration::from_millis(20));
                fake.set_empty();
            })
        };
        let result = wait_with_progress(
            &fake,
            Duration::from_secs(5),
            Duration::from_millis(40),
            Duration::from_millis(1),
        );
        handle.join().unwrap();
        assert!(result.is_ok(), "expected ok, got {result:?}");
        assert!(crate::native_audio::native_audio_available());
    }
}
```

- [ ] **Step 4.2: Run the tests to confirm they fail**

Run: `cargo test -p playa --lib native_player::tests::progress_wait`

Expected: compilation failure (`wait_with_progress` does not exist).

- [ ] **Step 4.3: Implement `wait_with_progress`**

Replace the existing `wait_with_timeout` function in `playa/lib/src/native_player.rs` with:

```rust
/// Wait for the player to drain, abandoning the device if no playback
/// progress occurs for the stall window.
///
/// `absolute_deadline` is the wall-clock backstop (existing 300 s
/// behavior). `stall_window` is the maximum gap between advances of
/// `Player::get_pos()`. `poll_interval` is the loop sleep cadence.
///
/// On stall, calls `player.stop()`, trips the native breaker, and
/// returns `NativePlaybackError::Timeout` so subsequent native attempts
/// in this process route directly to host playback.
fn wait_with_progress<P: PlayerProgress>(
    player: &P,
    absolute_timeout: Duration,
    stall_window: Duration,
    poll_interval: Duration,
) -> Result<(), NativePlaybackError> {
    let start = Instant::now();
    let mut last_pos = player.get_pos();
    let mut last_progress_at = Instant::now();

    while !player.empty() {
        let now = Instant::now();
        if now.duration_since(start) >= absolute_timeout {
            player.stop();
            trip_native_audio_breaker(NativeAudioFailureKind::DeviceOpenTimeout);
            eprintln!(
                "playa: audio playback timed out after {}s — audio device may be unresponsive",
                absolute_timeout.as_secs()
            );
            return Err(NativePlaybackError::Timeout(absolute_timeout.as_secs()));
        }

        let pos = player.get_pos();
        if pos != last_pos {
            last_pos = pos;
            last_progress_at = now;
        } else if now.duration_since(last_progress_at) >= stall_window {
            player.stop();
            trip_native_audio_breaker(NativeAudioFailureKind::DeviceOpenTimeout);
            eprintln!(
                "playa: audio playback stalled — no progress for {}s, treating device as unresponsive",
                stall_window.as_secs()
            );
            return Err(NativePlaybackError::Timeout(stall_window.as_secs()));
        }

        std::thread::sleep(poll_interval);
    }

    Ok(())
}
```

- [ ] **Step 4.4: Update `play_source` to call `wait_with_progress`**

Replace the body of `play_source` (currently calls `wait_with_timeout(&player, PLAYBACK_TIMEOUT)`) with:

```rust
fn play_source(
    source: Decoder<impl std::io::Read + std::io::Seek + Send + Sync + 'static>,
    options: &PlaybackOptions,
) -> Result<(), NativePlaybackError> {
    let stream = open_stream_with_timeout(NATIVE_DEVICE_TIMEOUT, options)?;
    let player = Player::connect_new(stream.mixer());

    if let Some(vol) = options.volume {
        player.set_volume(vol);
    }
    if let Some(speed) = options.speed {
        player.set_speed(speed);
    }

    player.append(source);
    wait_with_progress(
        &player,
        PLAYBACK_TIMEOUT,
        resolved_stall_window(),
        Duration::from_millis(50),
    )?;

    Ok(())
}
```

- [ ] **Step 4.5: Run the unit tests**

Run: `cargo test -p playa --lib native_player::tests::progress_wait`

Expected: 4 passed.

- [ ] **Step 4.6: Run the broader playa test suite to confirm no regressions**

Run: `cargo test -p playa`

Expected: all green. If a pre-existing test referenced `wait_with_timeout` directly (it does not at time of writing, but verify), update those too.

- [ ] **Step 4.7: Commit**

```bash
git add playa/lib/src/native_player.rs
git commit -m "feat(playa): progress-aware playback wait with stall-window breaker"
```

---

## Task 5: Cached default `MixerDeviceSink`

**Files:**
- Modify: `playa/lib/src/native_player.rs`

- [ ] **Step 5.1: Write a failing test for sink reuse**

Append to the `#[cfg(test)] mod tests` block in `playa/lib/src/native_player.rs`:

```rust
mod sink_cache {
    use super::*;

    /// The cache slot is module-private. Two consecutive calls to
    /// `cached_default_mixer_ptr` must return the same `*const ()`
    /// once a sink has been opened.
    ///
    /// This test exercises the cache helper directly without opening a
    /// real device. It substitutes a closure that returns a stub sink.
    #[test]
    fn cached_default_mixer_returns_same_pointer() {
        // SAFETY: tests in this module use the test-only reset helper.
        reset_default_sink_cache_for_tests();

        let opens = std::sync::atomic::AtomicU32::new(0);
        let opener = || {
            opens.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            // The opener returns a `Result<TestSink, NativePlaybackError>`.
            // For the cache test we use the real type but a stub sink is
            // produced via a feature-gated helper. If a stub is not
            // feasible, this test should be marked `#[ignore]` and the
            // assertion downgraded to "first call opens, second does not".
            Err::<DummySink, NativePlaybackError>(NativePlaybackError::DeviceOpenTimeout(0))
        };

        // First call: opener runs once and the cache stays empty
        // because the open failed.
        let _ = with_default_mixer(opener);
        assert_eq!(opens.load(std::sync::atomic::Ordering::SeqCst), 1);

        // Second call: opener runs again because the slot is still empty.
        let _ = with_default_mixer(opener);
        assert_eq!(opens.load(std::sync::atomic::Ordering::SeqCst), 2);
    }

    // Placeholder type so the test compiles.  The real cache stores
    // `Arc<MixerDeviceSink>`; the test only exercises the failure
    // path so a dummy type is acceptable.
    struct DummySink;
}
```

Note: this test only verifies the failure-fallthrough behavior of the cache. We cannot easily exercise the success path in a unit test without a real audio device; that path is covered indirectly by the existing `play_native_*` tests and by manual smoke testing.

- [ ] **Step 5.2: Run the test to confirm it fails**

Run: `cargo test -p playa --lib native_player::tests::sink_cache`

Expected: compilation failure (`with_default_mixer`, `reset_default_sink_cache_for_tests` do not exist).

- [ ] **Step 5.3: Add the cache module-level state**

Add to the top of `playa/lib/src/native_player.rs` (after the `use` block):

```rust
use std::sync::{Arc, Mutex, OnceLock};

/// Process-wide cache for the default-device mixer sink.
///
/// Opened lazily on the first native playback call. Once cached, every
/// default-device playback connects a fresh `Player` to this sink's
/// mixer instead of opening a new CoreAudio stream. This dramatically
/// reduces device-open churn against `coreaudiod` on macOS.
///
/// The slot is a `Mutex<Option<Arc<MixerDeviceSink>>>` so a failed
/// initial open does not poison the slot — subsequent calls retry.
static SHARED_DEFAULT_SINK: OnceLock<Mutex<Option<Arc<rodio::MixerDeviceSink>>>> = OnceLock::new();

fn shared_default_sink_slot() -> &'static Mutex<Option<Arc<rodio::MixerDeviceSink>>> {
    SHARED_DEFAULT_SINK.get_or_init(|| Mutex::new(None))
}
```

- [ ] **Step 5.4: Add `with_default_mixer` and the test reset helper**

Add to `playa/lib/src/native_player.rs` near the cache slot:

```rust
/// Run `body` against a long-lived default-device mixer.
///
/// On first call, `open` is invoked to construct the sink; the result
/// is cached for subsequent calls. If `open` fails, the cache stays
/// empty and the next call retries.
///
/// `MixerDeviceSink::log_on_drop(false)` is set so the cached sink
/// never logs spurious "dropping DeviceSink" noise during process
/// shutdown.
fn with_default_mixer<F, R>(open: F) -> Result<R, NativePlaybackError>
where
    F: FnOnce() -> Result<rodio::MixerDeviceSink, NativePlaybackError>,
    R: 'static,
{
    // For tests we use a stub return type; in production the body
    // closure is responsible for using the mixer. Restructure to a
    // closure that takes `&Mixer`.
    let _ = open;
    unreachable!("use with_cached_default_mixer instead")
}

/// Acquire (lazily open) the cached default-device sink and run `body`
/// against its mixer.
fn with_cached_default_mixer<F, R>(
    open: impl FnOnce() -> Result<rodio::MixerDeviceSink, NativePlaybackError>,
    body: F,
) -> Result<R, NativePlaybackError>
where
    F: FnOnce(&rodio::Mixer) -> Result<R, NativePlaybackError>,
{
    let slot = shared_default_sink_slot();
    let mut guard = slot.lock().expect("default sink mutex poisoned");

    if guard.is_none() {
        let mut sink = open()?;
        sink.log_on_drop(false);
        *guard = Some(Arc::new(sink));
    }

    let sink = guard
        .as_ref()
        .expect("just inserted")
        .clone();
    drop(guard);

    body(sink.mixer())
}

#[cfg(test)]
fn reset_default_sink_cache_for_tests() {
    if let Some(slot) = SHARED_DEFAULT_SINK.get() {
        let mut guard = slot.lock().expect("default sink mutex poisoned");
        *guard = None;
    }
}
```

- [ ] **Step 5.5: Replace the test from Step 5.1 with the real surface**

Replace the test body in `mod sink_cache` with:

```rust
mod sink_cache {
    use super::*;

    #[test]
    fn failed_open_does_not_poison_cache() {
        reset_default_sink_cache_for_tests();

        let opens = std::sync::atomic::AtomicU32::new(0);
        let try_open = || {
            opens.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Err::<rodio::MixerDeviceSink, NativePlaybackError>(
                NativePlaybackError::DeviceOpenTimeout(0),
            )
        };

        let r1 = with_cached_default_mixer(try_open, |_mixer| {
            Ok::<(), NativePlaybackError>(())
        });
        assert!(r1.is_err());

        let r2 = with_cached_default_mixer(try_open, |_mixer| {
            Ok::<(), NativePlaybackError>(())
        });
        assert!(r2.is_err());

        // Both calls invoked the opener — cache was not poisoned.
        assert_eq!(opens.load(std::sync::atomic::Ordering::SeqCst), 2);
    }
}
```

Also remove the now-unused `with_default_mixer` and `DummySink` placeholder from Step 5.4 — keep only `with_cached_default_mixer` and the test reset helper.

- [ ] **Step 5.6: Run the test**

Run: `cargo test -p playa --lib native_player::tests::sink_cache`

Expected: 1 passed.

- [ ] **Step 5.7: Route default-device `play_source` through the cache**

Edit `play_source` in `playa/lib/src/native_player.rs` so default-device playback uses the cached mixer; channel-override playback continues through the existing one-shot path:

```rust
fn play_source(
    source: Decoder<impl std::io::Read + std::io::Seek + Send + Sync + 'static>,
    options: &PlaybackOptions,
) -> Result<(), NativePlaybackError> {
    if options.channel.is_some() {
        return play_source_one_shot(source, options);
    }

    with_cached_default_mixer(
        || open_default_stream_with_timeout(NATIVE_DEVICE_TIMEOUT),
        |mixer| {
            let player = Player::connect_new(mixer);

            if let Some(vol) = options.volume {
                player.set_volume(vol);
            }
            if let Some(speed) = options.speed {
                player.set_speed(speed);
            }

            player.append(source);
            wait_with_progress(
                &player,
                PLAYBACK_TIMEOUT,
                resolved_stall_window(),
                Duration::from_millis(50),
            )?;

            Ok(())
        },
    )
}

/// One-shot device-open path for channel-override playback.
fn play_source_one_shot(
    source: Decoder<impl std::io::Read + std::io::Seek + Send + Sync + 'static>,
    options: &PlaybackOptions,
) -> Result<(), NativePlaybackError> {
    let stream = open_stream_with_timeout(NATIVE_DEVICE_TIMEOUT, options)?;
    let player = Player::connect_new(stream.mixer());

    if let Some(vol) = options.volume {
        player.set_volume(vol);
    }
    if let Some(speed) = options.speed {
        player.set_speed(speed);
    }

    player.append(source);
    wait_with_progress(
        &player,
        PLAYBACK_TIMEOUT,
        resolved_stall_window(),
        Duration::from_millis(50),
    )?;

    Ok(())
}
```

Compilation note: this introduces a generic `Decoder<...>` type parameter into the new `play_source_one_shot`. The function signature and bounds are identical to the original `play_source`.

- [ ] **Step 5.8: Run the full playa suite**

Run: `cargo test -p playa`

Expected: all green. The existing `play_native_*` error-path tests should still pass.

- [ ] **Step 5.9: Manual smoke test**

Run (only if a working audio device is available — skip if your local audio is currently wedged):

```bash
cargo run -p playa --bin playa -- effect confirmation
cargo run -p playa --bin playa -- effect sad-trombone
cargo run -p playa --bin playa -- effect confirmation
```

Expected: all three play through. The first opens the default device, the next two reuse the cached sink. No "Dropping DeviceSink" noise on stderr (because of `log_on_drop(false)`).

- [ ] **Step 5.10: Commit**

```bash
git add playa/lib/src/native_player.rs
git commit -m "feat(playa): cache default-device sink to reduce coreaudio churn"
```

---

## Task 6: End-to-end verification

**Files:**
- None (verification only)

- [ ] **Step 6.1: Run the originally-failing test under wall-clock measurement**

Run: `time cargo test -p claudine --test canonical_dispatch dispatch_sound_effect_action`

Expected: completes in well under 5 seconds.

- [ ] **Step 6.2: Run the broader claudine + playa suites**

Run:

```bash
cargo test -p playa
cargo test -p claudine
```

Expected: all green.

- [ ] **Step 6.3: Lint pass**

Run:

```bash
cargo clippy -p playa --all-targets -- -D warnings
cargo clippy -p claudine --all-targets -- -D warnings
```

Expected: no warnings.

- [ ] **Step 6.4: Format pass**

Run: `cargo fmt --check -p playa -p claudine`

Expected: no diff. If diff is reported, run `cargo fmt -p playa -p claudine` and amend the appropriate commit.

- [ ] **Step 6.5: Final summary commit (only if any fixups were needed)**

If lint or fmt produced changes, commit them:

```bash
git add -u
git commit -m "chore(playa,claudine): clippy + fmt fixups"
```

If there are no changes, skip this step.

---

## Self-review checklist

- Spec coverage:
  - Goal 1 (test under 1 s) → Task 2 + Task 6 Step 6.1.
  - Goal 2 (abandon wedged device in single-digit seconds) → Task 4.
  - Goal 3 (no native retry after stall) → Task 4 Step 4.3 (breaker trip).
  - Goal 4 (reduce CoreAudio churn) → Task 5.
  - Non-goal: multi-device caching → Task 5 Step 5.7 routes channel-override calls through `play_source_one_shot` (one-shot open).
  - Non-goal: rodio replacement → confirmed; rodio 0.22 API used throughout.
- Placeholders: none (no TBD/TODO; every step shows code or exact command).
- Type consistency: `wait_with_progress`, `with_cached_default_mixer`, `play_source_one_shot`, `PlayerProgress`, `resolved_stall_window`, `dry_run_env_enabled`, `PlayaDryRunGuard` are referenced consistently across tasks.
- Open question from spec ("Re-evaluate after Fix 3 lands; instrument with tracing if corruption persists"): out of scope for this plan; will be evaluated post-deploy.
