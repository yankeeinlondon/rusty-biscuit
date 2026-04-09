# Playa Package Review

**Date:** 2026-04-08  
**Reviewer:** Senior Rust Code Review  
**Packages Reviewed:** `playa` (lib), `playa-cli` (cli)  
**Repository:** rusty-biscuit monorepo

---

## 1. Executive Summary

The `playa` package is a well-structured audio playback library with dual-mode playback (native rodio/symphonia + host player delegation), comprehensive format detection, 13 supported players with capability-based ranking, 88 embedded sound effects, and platform-specific audio ducking backends. Error handling uses `thiserror` correctly throughout library code. The ducking subsystem is architecturally sound with a channel-based RAII guard pattern, though it has a latent race condition. The `MacOsAfplay` player is unimplemented in `build_player_command`, representing a missing match arm that will cause silent failure. Unsafe FFI is confined to `sfx_player.rs` (CoreAudio, WASAPI, PulseAudio) with watchdog protection. Code is idiomatic, tests pass (41 lib + 6 CLI), and Clippy shows only 2 minor warnings in the CLI.

**Overall Risk Level:** Medium

**Production Readiness:** Mostly production-ready; the `MacOsAfplay` gap and ducking race condition should be addressed before heavy use on macOS.

---

## 2. Key Findings

#### [Severity: High] `MacOsAfplay` has no implementation in `build_player_command`

- **Location:** `playa/lib/src/playback.rs:264-400` (`build_player_command`), `playa/lib/src/playback.rs:488-617` (`build_player_args`)
- **Why it matters:** When `MacOsAfplay` is selected as the best available player (WAV/PCM on macOS when native playback is disabled or unavailable), playback will fail silently or hang because `build_player_command` has no match arm for `AudioPlayer::MacOsAfplay`. The command will be spawned with no source argument, causing `afplay` to fail immediately.
- **Evidence:** `build_player_command` has match arms for `Mpv`, `FfPlay`, `Vlc`, `MPlayer`, `GstreamerGstPlay`, `Sox`, `Mpg123`, `Ogg123`, `AlsaAplay`, `MacOsAfplay`, `PulseaudioPaplay`, `PulseaudioPacat`, `Pipewire` — but `MacOsAfplay` is absent. Tests at lines 912-950 exist for `MacOsAfplay` (confirming intent), but they test a non-existent code path.
- **Reproduction path:** `Playa::from_bytes(wav_bytes).force_host().play()` on macOS where `mpv`/`ffplay` are uninstalled and `MacOsAfplay` is the highest-ranked installed player.
- **Recommendation:** Add match arms for `AudioPlayer::MacOsAfplay` in both `build_player_command` and `build_player_args`:

  ```rust
  AudioPlayer::MacOsAfplay => {
      if let Some(vol) = options.volume {
          command.arg("-v").arg(vol.to_string());
      }
      if let Some(speed) = options.speed {
          let clamped = speed.clamp(0.4, 3.0);
          command.arg("-r").arg(clamped.to_string());
      }
      source.apply(command);
  }
  ```

- **Confidence:** High

---

#### [Severity: High] `DuckGuard` restoration can silently fail if spawned task hasn't reached `recv()`

- **Location:** `playa/lib/src/ducking/guard.rs:129-144` (`Drop::drop`), `guard.rs:147-162` (`restoration_task`)
- **Why it matters:** When `DuckGuard` is dropped (early return, panic, or scope exit) before the spawned `restoration_task` has reached `rx.recv().await`, the `try_send` in `Drop::drop` fails (unbuffered channel, receiver not yet polling) and the task receives `None` from `rx.recv().await`, exiting without calling `fade_restore`. System audio remains in a permanently ducked state with no error signal.
- **Evidence:** The channel is unbuffered (`mpsc::channel(1)`), `try_send` fails if the receiver isn't ready, and `rx.recv().await` returning `None` causes the task to exit without restoration.
- **Scenario:** On a heavily loaded system, if `playa.play_async()` returns early due to playback failure, the guard's `Drop` runs, `try_send` is called, but the tokio scheduler hasn't moved the `restoration_task` to a running state yet. `try_send` fails, `rx.recv().await` returns `None`, task exits, audio stays ducked.
- **Recommendation:** Use a buffered channel (e.g., `mpsc::channel(1)` → `mpsc::channel(8)`) so `try_send` succeeds even if the task hasn't reached `recv` yet. Additionally, set `restored.store(true)` in the `None` branch of `restoration_task` so at minimum the flag reflects the intent, and log a warning when `rx.recv().await` returns `None`.
- **Confidence:** Medium (race condition, not guaranteed in normal use)

---

#### [Severity: Medium] `DeviceOpenWatchdog` can `process::exit(1)` from library code

- **Location:** `playa/lib/src/native_player.rs:41-59` (`DeviceOpenWatchdog`), `playa/lib/src/sfx_player.rs:60-78`
- **Why it matters:** The watchdog spawns a thread that calls `std::process::exit(1)` if the audio device doesn't open within 5 seconds. This terminates the entire process unconditionally, bypassing Rust's drop semantics, panic hooks, and any caller cleanup. This is extremely aggressive for a library.
- **Evidence:** `std::thread::spawn` + `std::process::exit(1)` with no guard in the calling code path. The 5-second timeout is intended to handle CoreAudio run-loop requirements, but `process::exit` is irreversible.
- **Recommendation:** Return an error instead of exiting. CoreAudio requires a run loop on the calling thread, which is why the watchdog exists — but the correct fix is to run the device-opening on a dedicated thread and move the actual playback to the main thread. Alternatively, use `std::panic::panic_any` with a custom panic payload, or propagate a `TimedOut` error that the caller handles. At minimum, warn that `process::exit` will be called and document the invariant that the device open must complete within the timeout.
- **Confidence:** High (deterministic behavior when device is unresponsive)

---

#### [Severity: Medium] Temp audio files not cleaned up on crash

- **Location:** `playa/lib/src/playback.rs:413-436` (`write_temp_audio`), `playa/lib/src/playback.rs:450-469` (`write_temp_audio_async`)
- **Why it matters:** Bytes audio (`AudioData::Bytes`) is written to `std::env::temp_dir()/playa-{pid}-{timestamp}.audio`. If the process crashes or is killed after writing but before playback completes, these files accumulate in the temp directory.
- **Evidence:** `write_temp_audio` and `write_temp_audio_async` create files but have no cleanup mechanism. `playback.rs` has no `Drop` handler for `AudioData`.
- **Recommendation:** Use `tempfile` crate's `NamedTempFile` for automatic cleanup on drop, or at minimum document the lifetime of temp files and that users should periodically clean `playa-*.audio` from their temp directory. For short-lived CLI invocations, this is not a concern.
- **Confidence:** High (confirmed by code inspection)

---

#### [Severity: Low] `detect_audio_format_from_url` creates a new `reqwest::Client` on every call

- **Location:** `playa/lib/src/detection.rs:56-75`
- **Why it matters:** For repeated URL detections, creating a new `Client` each time avoids connection reuse and adds latency. `reqwest::Client` is designed to be reused.
- **Evidence:** `let client = Client::new();` inside the async function.
- **Recommendation:** Consider a `LazyLock<Client>` at module level, or accept the minor overhead for single-invocation CLI use.
- **Confidence:** High

---

#### [Severity: Low] Clippy warnings in `playa-cli`

- **Location:** `playa/cli/src/main.rs:157` and `playa/cli/src/main.rs:161`
- **Evidence:**
  1. `question_mark` lint: `if std::env::var_os("COMPLETE").is_none() { return None; }` should use `?` operator
  2. `double_ended_iterator_last`: `std::env::args().last()` should use `next_back()` instead
- **Recommendation:** Minor fixes — use `std::env::var_os("COMPLETE")?;` and `std::env::args().next_back()`.
- **Confidence:** High

---

## 3. Rust-Idiomaticity Notes

**Good patterns observed:**

- `PLAYER_LOOKUP: LazyLock<HashMap<AudioPlayer, Player>>` — correct use of `LazyLock` for static initialization with potentially expensive computation
- `AudioData::Bytes(Arc<Vec<u8>>)` — correct use of `Arc` for shared ownership of byte buffers across sync and async paths
- `DuckGuard` with `Arc<AtomicBool>` — correct for cross-thread communication without a join handle
- `thiserror` derive for all error types — consistent and idiomatic
- `#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]` on enums — appropriate use of derive macros
- Feature-gated modules with `#[cfg(...)]` — clean conditional compilation
- RAII guard pattern for ducking (`DuckGuard`) — idiomatic resource management

**Areas for improvement:**

- `MacOsAfplay` not handled in `build_player_command` — confirmed gap in match arm completeness
- `fn slugify(s: &str) -> String` in `channels.rs` is `pub(crate)` but the enclosing module is `#[cfg(feature = "sfx-native")]`. Minor inconsistency in visibility given the conditional compilation.
- `format_codec` and `format_file_format` are private helper functions at module level — could be `impl AudioFormat` methods or kept private with a `Debug` derive for formatted output.

---

## 4. Testing Gaps

**Missing or incomplete test coverage:**

1. **`build_player_command` with `MacOsAfplay`**: The tests at `playback.rs:912-950` test the `afplay` command building, but `build_player_command` has no match arm for `MacOsAfplay`. These tests pass trivially because they test a non-existent code path. Real integration testing with `MacOsAfplay` as the selected player is needed.

2. **Temp file collision retry**: `write_temp_audio` retries up to 3 times on filename collision, but there's no test for the retry logic or the `AlreadyExists` error path.

3. **Ducking graceful degradation**: When `DuckGuard::new()` fails (backend error), the code falls back gracefully but there's no test verifying that `playa.play_async()` proceeds without ducking in this case.

4. **`DeviceOpenWatchdog` timeout**: No test for the watchdog behavior when device open times out. This would require mocking or a very slow device.

5. **`Playa::from_bytes` → `play()` with native playback failure**: The fallback path from native to host player via bytes is not directly tested in integration style (only unit tests for `play_native` error paths).

6. **URL format detection with extension fallback**: `detect_audio_format_from_url` is only tested via unit tests for bytes detection, not with actual HTTP responses or extension fallback from URLs.

---

## 5. Unsafe Code Review

The `playa` package has unsafe code confined to `sfx_player.rs` for three platform backends. All unsafe usage is justified and properly contained.

### `sfx_player.rs` — macOS CoreAudio FFI (lines ~206-480)

**What it does:** Direct CoreAudio syscalls via `coreaudio_sys` crate to:

- Query hardware device IDs (`AudioObjectGetPropertyData`)
- Get device names and UIDs (`get_device_cfstring_property`)
- Enumerate all output devices (`get_all_device_ids`)
- Check device stream capabilities (`has_output_streams`)

**Safety assessment:**

- `unsafe extern "C"` blocks for CFString FFI functions (`CFStringGetLength`, `CFStringGetCString`, `CFRelease`) — correct, these are C APIs
- `AudioObjectGetPropertyData` calls with valid pointers checked for null and status codes — correct
- Four `unsafe` blocks overall, each with clear contracts documented in comments
- `CFRelease(value)` called after use to avoid memory leaks
- Property selector constants manually computed (e.g., `SYSTEM_OUTPUT_DEVICE_SELECTOR = 'sOut'`) with test assertions verifying correctness
- **Verdict:** Acceptable. Safety invariants are documented and upheld.

### `sfx_player.rs` — Windows WASAPI (lines ~547-757)

**What it does:** COM initialization, WASAPI device enumeration, shared-mode stream setup with `AudioCategory_SoundEffects`, render loop writing decoded samples.

**Safety assessment (lines 611-707):**

- `CoInitializeEx(None, COINIT_MULTITHREADED)` — guarded with `let _ =` to ignore "already initialized" errors. Correct.
- `CoCreateInstance`, `device.Activate`, `client.Initialize`, `GetBuffer`, `GetCurrentPadding`, `Stop` — all COM/WASAPI calls with error propagation
- `std::slice::from_raw_parts_mut(buf_ptr as *mut f32, frames_to_write as usize * ch)` — **critical**: The `buf_ptr` comes from `IAudioRenderClient::GetBuffer` which returns a valid pointer to a buffer of at least `frames_to_write * ch * sizeof(f32)` bytes. The slice length computation matches. This is safe if the WASAPI contract holds.
- Buffer padding/drain loop with 5-second deadline — correct
- **Verdict:** Acceptable. WASAPI invariants are upheld by the COM contract.

### `sfx_player.rs` — Linux PulseAudio (lines ~763-909)

**What it does:** PulseAudio context creation, stream with `media.role=event` proplist, write and drain.

**Safety assessment:**

- All PulseAudio operations use the `libpulse_binding` safe API
- No raw pointers or FFI calls
- Mainloop iteration with `iterate_or_fail` blocking call — correct
- **Verdict:** No unsafe code. Safe Rust via bindings.

### `native_player.rs` — `DeviceOpenWatchdog`

**Not unsafe, but notable:** Spawns a thread that calls `std::process::exit(1)`. This is discussed in Key Findings (High severity).

### Summary

| Location | Lines | Unsafe Blocks | Soundness Risk |
|----------|-------|---------------|----------------|
| `sfx_player.rs` macOS | 206-480 | 4 | Low |
| `sfx_player.rs` Windows | 547-757 | 1 (block at 611) | Low |
| `sfx_player.rs` Linux | 763-909 | 0 | N/A |
| `native_player.rs` watchdog | 41-59 | 0 (thread exit) | Medium (design issue) |

---

## 6. Prioritized Next Steps

1. **[High] Add `MacOsAfplay` match arms to `build_player_command` and `build_player_args`** — This is a confirmed missing feature causing playback failure on macOS when this player is selected.

2. **[High] Fix `DuckGuard` restoration race condition** — Change the channel from unbuffered to buffered (e.g., 8 slots) and set `restored.store(true)` before `rx.recv().await` returns `None`, with a `warn!` log message.

3. **[Medium] Replace `std::process::exit(1)` in `DeviceOpenWatchdog`** — Return a `TimedOut` error from the device open operation instead of exiting. This requires restructuring the device open to happen on a background thread with a timeout-wrapped join handle.

4. **[Medium] Add integration test for bytes-to-host-player fallback path** — `Playa::from_bytes` → native fails → host player selected → playback succeeds.

5. **[Medium] Use `NamedTempFile` or document temp file lifetime** — Consider using the `tempfile` crate for automatic cleanup, or document that `playa-*.audio` files in the temp directory may need periodic cleanup.

6. **[Low] Fix Clippy warnings in `playa-cli`** — Replace `if std::env::var_os("COMPLETE").is_none() { return None; }` with `std::env::var_os("COMPLETE")?;` and `std::env::args().last()` with `std::env::args().next_back()`.

7. **[Low] Cache `reqwest::Client` in `detect_audio_format_from_url`** — Use a `LazyLock` or `once_cell` for the HTTP client if URL-based detection is called repeatedly.
