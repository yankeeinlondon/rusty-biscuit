# Cross-Platform Review: Playa Library

**Date:** 2026-04-08
**Reviewer:** AI Code Review
**Scope:** All OS-specific code paths (Windows, Linux, macOS)
**Context:** Primary testing done on macOS; evaluating Windows and Linux code for correctness

---

## Summary

The playa library has a well-structured cross-platform architecture with clear `cfg(target_os)` gating and feature-flag separation. The macOS path is thoroughly exercised. This review identifies **14 issues** across the Windows and Linux code paths, ranging from bugs to design concerns.

| Severity | Count |
|----------|-------|
| **Critical** | 1 |
| **High** | 4 |
| **Medium** | 5 |
| **Low** | 4 |

---

## Critical Issues

### C1. Windows ducking is completely unimplemented

**File:** `playa/lib/src/ducking/factory.rs:61-64`

```rust
#[cfg(target_os = "windows")]
{
    // Phase 4 will implement this
    return Box::new(NoopBackend::new());
}
```

Windows has the `SessionId::WasapiSession` variant defined in `types.rs`, the `windows` crate dependency configured in `Cargo.toml`, and WASAPI SFX playback implemented in `sfx_player.rs` — but the audio ducking backend for Windows was never written. The factory always returns `NoopBackend` on Windows.

Similarly, `backend_name()` on Windows always returns `"noop"` (line 127).

**Impact:** Audio ducking is completely non-functional on Windows. Users who enable `audio-ducking` feature will get silent no-op behavior with no warning.

**Suggested Fix:** Either:
1. Implement a `WindowsBackend` using WASAPI `ISimpleAudioVolume` per-session ducking (matching the design implied by `SessionId::WasapiSession`), or
2. Log a clear warning when ducking is requested on Windows but unavailable, and document this as unsupported.

---

## High Issues

### H1. Linux PulseAudio volume delta rounding causes cumulative drift

**File:** `playa/lib/src/ducking/linux.rs:193-207`

The `fade_to_floor` and `fade_restore` methods compute volume deltas and apply them using `increase_app_volume_by_percent` / `decrease_app_volume_by_percent`. These pulsectl methods take floating-point percentages, but the PulseAudio protocol internally converts to integer volume levels (0–65536 scale). Each delta application introduces rounding error that accumulates across fade steps.

After a full duck-and-restore cycle, the restored volume may noticeably differ from the original.

**Suggested Fix:** Instead of computing relative deltas, use `set_app_volume` (if available in `pulsectl-rs`) to set absolute volume on each step. If the library only supports relative adjustments, reduce the number of steps and verify the final restored volume matches the cached `original_volumes`.

### H2. Linux ALSA backend ducks ALL audio including Playa's own output

**File:** `playa/lib/src/ducking/linux.rs:293-395`

The `AlsaBackend` adjusts the Master/PCM/Speaker/Headphone mixer control, which affects all audio output system-wide. Unlike the PulseAudio backend (which can exclude Playa's own streams by PID/name), ALSA has no per-application volume concept.

The factory code at `factory.rs:78` does print a warning:
```rust
eprintln!("Warning: PulseAudio not available, using ALSA fallback (affects all audio)");
```

But this only appears when the backend is *selected*. The user may not see this if ducking is configured programmatically. More importantly, the Playa audio itself will be ducked along with everything else.

**Suggested Fix:** Document this limitation clearly in the `AlsaBackend` doc comments and consider returning an `Err` from `fade_to_floor` if Playa is playing through an external player (not native rodio), since the ducked audio would be inaudible. Alternatively, the ALSA backend could temporarily boost Playa's player process volume to compensate.

### H3. Windows SFX playback does not handle COM threading model correctly

**File:** `playa/lib/src/sfx_player.rs:606-607`

```rust
let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
```

The `CoInitializeEx` result is discarded. On Windows, COM initialization must match across all threads in the same apartment. If the calling thread has already initialized COM with `COINIT_APARTMENTTHREADED` (single-threaded apartment), this `COINIT_MULTITHREADED` call will fail with `RPC_E_CHANGED_MODE` and COM will not be initialized at all. The subsequent WASAPI calls will then fail with `CO_E_NOTINITIALIZED`.

Furthermore, the code never calls `CoUninitialize`, which leaks the COM reference count.

**Suggested Fix:**
1. Check the `CoInitializeEx` return value. If it returns `S_FALSE` (already initialized with same mode) that's fine. If it returns `RPC_E_CHANGED_MODE`, either proceed (COM is already initialized) or return an error.
2. Call `CoUninitialize()` at the end of the function (only if `CoInitializeEx` succeeded with `S_OK`).
3. Consider initializing COM once at application startup rather than per-call.

### H4. Windows WASAPI drain loop has no upper bound

**File:** `playa/lib/src/sfx_player.rs:691-698`

```rust
let drain_deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
loop {
    let padding = client.GetCurrentPadding()?;
    if padding == 0 || std::time::Instant::now() > drain_deadline {
        break;
    }
    std::thread::sleep(std::time::Duration::from_millis(10));
}
```

The 5-second drain timeout is reasonable, but `client.Stop()` is called after the drain loop regardless of whether the drain succeeded or timed out. If the drain timed out, there may still be audio in the buffer, and `Stop()` will cut it off abruptly. This is acceptable for short SFX, but the timeout should be documented.

More critically, if the WASAPI device enters an error state during drain, `GetCurrentPadding()` may keep returning the same non-zero value indefinitely until timeout. This is a minor issue since the timeout prevents a true hang.

**Suggested Fix:** Add a comment documenting the 5s drain timeout and the rationale. Consider adding a fallback that calls `client.Stop()` with `AUDCLNT_SHAREMODE_SHARED` flags to ensure clean shutdown.

---

## Medium Issues

### M1. Linux PulseAudio `play_sfx_as_event` blocks the calling thread with mainloop iteration

**File:** `playa/lib/src/sfx_player.rs:831-840`

The PulseAudio mainloop `iterate(true)` call is **blocking** — it blocks until an event arrives. This means the entire `play_sfx_as_event` function is synchronous and blocks the calling thread for the duration of context connection, stream setup, and audio playback. If called from an async context, this will block the Tokio runtime.

Compare with the macOS and Windows paths, which either use rodio (non-blocking sink) or WASAPI (buffered writes with short sleeps). The Linux PulseAudio path is architecturally different.

**Suggested Fix:** Either:
1. Document that this function is blocking and should be called from `tokio::task::spawn_blocking`, or
2. Use `libpulse-binding`'s async mainloop API, or
3. Run the entire PulseAudio operation on a background thread (similar to how `open_sfx_stream_with_timeout` already works).

### M2. Linux PulseAudio mainloop has no connection timeout

**File:** `playa/lib/src/sfx_player.rs:831-840`

The context connection and stream connection loops have no timeout:

```rust
loop {
    iterate_or_fail(&mut mainloop)?;
    match context.get_state() {
        pulse::context::State::Ready => break,
        pulse::context::State::Failed | pulse::context::State::Terminated => {
            return Err("PulseAudio context connection failed".into());
        }
        _ => {}
    }
}
```

If the PulseAudio daemon is running but unresponsive (e.g., hung, high load), this loop will block indefinitely. The `iterate_or_fail` function only catches mainloop errors, not timeouts.

**Suggested Fix:** Add an `Instant::now()` deadline check inside each wait loop (e.g., 10 seconds for context, 5 seconds for stream).

### M3. Windows SFX speed control uses pitch-shifting sample rate trick, inconsistent with rodio path

**File:** `playa/lib/src/sfx_player.rs:599-603`

```rust
let effective_rate = if let Some(speed) = options.speed {
    (sample_rate_u32 as f32 * speed) as u32
} else {
    sample_rate_u32
};
```

Both the Windows WASAPI and Linux PulseAudio SFX paths implement speed control by declaring a higher sample rate to the audio subsystem, which causes pitch-shifted faster playback. However, the rodio fallback path (used when platform-specific SFX fails) uses `player.set_speed(speed)` which may do proper time-stretching without pitch shift.

This means the same `PlaybackOptions` will sound different depending on whether the platform-specific SFX path or the rodio fallback is used. Users may notice different audio quality across platforms.

The same issue applies to the Linux PulseAudio SFX path (`sfx_player.rs:805-809`).

**Suggested Fix:** Document this behavior clearly. For a consistent experience, consider always using the rodio fallback for speed control, or implement proper time-stretching in the platform-specific paths.

### M4. Temp file cleanup missing in `playback.rs`

**File:** `playa/lib/src/playback.rs:413-436` and `450-469`

The `write_temp_audio` and `write_temp_audio_async` functions write audio bytes to a temp file (e.g., `/tmp/playa-12345-1234567890.audio`) but never clean them up. On all platforms, repeated playback of `AudioData::Bytes` will accumulate temp files in the system temp directory.

On Linux, this fills `/tmp`. On Windows, it fills `%TEMP%`. On macOS, it fills `$TMPDIR`.

**Suggested Fix:** Register the temp file for cleanup after playback completes. Options:
1. Use `tempfile::NamedTempFile` (auto-cleanup on drop).
2. Delete the file after the player process exits in `playa_with_player_and_options`.
3. Use a periodic cleanup of old `playa-*.audio` files.

### M5. `channels.rs` SFX device detection may not work on Linux/Windows

**File:** `playa/lib/src/channels.rs:86-93`

The `sfx_device_name` for non-macOS platforms always falls back to `default_audio_name.clone()`:

```rust
#[cfg(not(all(target_os = "macos", feature = "sfx-native-macos")))]
let sfx_device_name = default_audio_name.clone();
```

This means `OutputChannel.is_default_sfx` will always equal `is_default_audio` on Linux and Windows. The `is_default_sfx` field becomes misleading — it's supposed to indicate whether the device is the system sound effects device, but on Linux/Windows it just mirrors `is_default_audio`.

**Suggested Fix:** Either:
1. Implement platform-specific SFX device detection for Linux (PulseAudio default sink for event sounds) and Windows (WASAPI default communication device), or
2. Set `is_default_sfx` to `false` on non-macOS platforms and document that SFX routing is macOS-only.

---

## Low Issues

### L1. Linux PulseAudio SFX path: `libpulse-binding` crate name mismatch

**File:** `playa/lib/Cargo.toml:67`

```toml
libpulse-binding = { version = "2.28", optional = true }
```

The crate is imported as `libpulse_binding` in `sfx_player.rs:761`:
```rust
use libpulse_binding as pulse;
```

The actual crate name on crates.io is `libpulse-binding` which maps to `libpulse_binding` in Rust code. This works correctly but the alias `pulse` shadows common conventions. This is a style concern, not a bug.

### L2. Windows WASAPI test uses `unsafe` without `#[cfg(test)]` guard at module level

**File:** `playa/lib/src/sfx_player.rs:706-751`

The Windows test module is already gated by `#[cfg(all(target_os = "windows", feature = "sfx-native-windows"))]`, so it will only compile on Windows. However, the `unsafe` blocks in tests should have safety comments explaining why they're sound (even in tests). This is a best-practices concern.

### L3. Linux ALSA `SelemChannelId` only sets 3 channels

**File:** `playa/lib/src/ducking/linux.rs:453-458`

```rust
for channel in [
    SelemChannelId::FrontLeft,
    SelemChannelId::FrontRight,
    SelemChannelId::FrontCenter,
] {
```

Surround sound setups (5.1, 7.1) have additional channels (RearLeft, RearRight, LFE, etc.) that won't be ducked. This means on a 5.1 surround system using ALSA, only front channels would be attenuated during ducking.

**Suggested Fix:** Iterate over all available ALSA channels, or at minimum add `SelemChannelId::RearLeft`, `SelemChannelId::RearRight`, and `SelemChannelId::LowFrequencyEffect` (LFE).

### L4. `pulsectl-rs` version `0.3` may be outdated

**File:** `playa/lib/Cargo.toml:68`

The `pulsectl-rs` crate at version 0.3 may not support all PulseAudio/PipeWire features. Consider verifying compatibility with PipeWire's PulseAudio emulation layer, which is the default on modern Linux distributions (Fedora 34+, Ubuntu 22.10+).

---

## Architecture Observations

### No Windows Player Binary

The `AudioPlayer` enum has no Windows-specific player variant. Windows users rely entirely on cross-platform players (mpv, ffplay, VLC, MPlayer, SoX). This is a good design — Windows has no built-in CLI audio player comparable to macOS's `afplay` or Linux's `aplay`. However, it means Windows users **must** install at least one third-party player.

The `match_available_players` function will return an empty list on a bare Windows installation, and users will get `PlaybackError::NoCompatiblePlayer` with no guidance on what to install.

**Suggested Fix:** Enhance the `NoCompatiblePlayer` error message on Windows to suggest installing mpv or FFmpeg (ffplay), since no built-in option exists.

### Temp File Path on Windows

The temp file naming in `playback.rs:421` uses `format!("playa-{}-{}.audio", ...)` with `std::env::temp_dir()`. This works on Windows (`%TEMP%`) but the `.audio` extension has no association. On Linux/macOS this is fine since the player reads the file header, not the extension. Verify this works with Windows players (ffplay, mpv) since some Windows software is extension-sensitive.

### Feature Flag Consistency

The feature flags are well-structured:
- `audio-ducking-macos` = CoreAudio
- `audio-ducking-linux` = PulseAudio + ALSA
- No `audio-ducking-windows` feature exists (ducking always no-ops on Windows)

This is intentional (documented as "Phase 4") but should be tracked as a known gap.

---

## Files Reviewed

| File | Lines | OS-Specific Code |
|------|-------|-----------------|
| `lib/src/playback.rs` | 1049 | Temp file paths, Command building |
| `lib/src/sfx_player.rs` | 1043 | macOS CoreAudio, Windows WASAPI, Linux PulseAudio |
| `lib/src/channels.rs` | 184 | macOS system sound device, cpal host enumeration |
| `lib/src/ducking/mod.rs` | 67 | Platform module gating |
| `lib/src/ducking/factory.rs` | 230 | Platform backend selection |
| `lib/src/ducking/macos.rs` | 392 | CoreAudio volume control |
| `lib/src/ducking/linux.rs` | 581 | PulseAudio sink inputs, ALSA mixer |
| `lib/src/ducking/media_keys.rs` | 439 | macOS-only AppleScript media keys |
| `lib/src/ducking/types.rs` | 110 | SessionId variants per platform |
| `lib/src/ducking/backend.rs` | 144 | NoopBackend fallback |
| `lib/src/ducking/guard.rs` | 264 | RAII guard (platform-agnostic) |
| `lib/src/ducking/envelope.rs` | 185 | Fade math (platform-agnostic) |
| `lib/src/player.rs` | 691 | Player definitions including platform-specific ones |
| `lib/src/native_player.rs` | 325 | rodio playback (platform-agnostic via cpal) |
| `lib/src/detection.rs` | 161 | Format detection (platform-agnostic) |
| `lib/Cargo.toml` | 79 | Platform-specific dependencies |

---

## Recommendations (Priority Order)

1. **Implement Windows ducking** or document it as unsupported (C1)
2. **Fix COM initialization** in Windows SFX path (H3)
3. **Add timeouts** to Linux PulseAudio SFX connection loops (M2)
4. **Use absolute volume** in Linux ducking instead of relative deltas (H1)
5. **Add temp file cleanup** after playback (M4)
6. **Improve error message** for no-player-found on Windows (Architecture)
7. **Document ALSA limitation** clearly that it ducks all audio (H2)
8. **Add ALSA surround channels** to the ducking backend (L3)
