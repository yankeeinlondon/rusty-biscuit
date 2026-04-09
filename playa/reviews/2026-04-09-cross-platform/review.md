# Cross-Platform Review: Playa Library

**Date:** 2026-04-09
**Reviewer:** AI Code Review (Opus 4.6)
**Scope:** All OS-specific code paths (Windows, Linux) in playa-lib
**Context:** Primary testing done on macOS; evaluating Windows and Linux code paths for correctness, safety, and completeness

---

## Summary

The playa library uses a well-structured `cfg(target_os)` + feature-flag architecture to separate platform-specific code. The macOS path is thoroughly exercised and serves as the reference implementation. This review evaluates the **Windows** and **Linux** code paths for bugs, safety issues, and design gaps.

**15 issues** identified across 4 functional areas: SFX playback, audio ducking, host player delegation, and temp file handling.

| Severity | Count |
|----------|-------|
| **Critical** | 2 |
| **High** | 3 |
| **Medium** | 5 |
| **Low** | 5 |

---

## Critical Issues

### C1. Windows ducking is completely unimplemented

**File:** `lib/src/ducking/factory.rs:61-64`

```rust
#[cfg(target_os = "windows")]
{
    // Phase 4 will implement this
    return Box::new(NoopBackend::new());
}
```

The factory always returns `NoopBackend` on Windows. The `SessionId::WasapiSession` variant exists in `types.rs` (suggesting the design was planned), the `windows` crate is configured in `Cargo.toml`, and WASAPI SFX playback works in `sfx_player.rs` — but the ducking backend was never written.

Similarly, `backend_name()` returns `"noop"` on Windows (line 127).

**Impact:** Audio ducking silently does nothing on Windows. Users who enable `audio-ducking` get no warning that it's a no-op.

**Suggested fix:** Either implement a `WindowsBackend` using WASAPI `ISimpleAudioVolume` per-session ducking, or log a clear warning when ducking is requested on Windows:

```rust
#[cfg(target_os = "windows")]
{
    eprintln!("Warning: audio ducking is not yet implemented on Windows");
    return Box::new(NoopBackend::new());
}
```

### C2. Windows WASAPI SFX: COM initialization is unchecked and leaked

**File:** `lib/src/sfx_player.rs:639`

```rust
let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
```

Two problems:

1. **The result is discarded.** If the calling thread already initialized COM with `COINIT_APARTMENTTHREADED` (STA), this call fails with `RPC_E_CHANGED_MODE`. COM is left uninitialized for this thread, and all subsequent WASAPI calls fail with `CO_E_NOTINITIALIZED`. This is not theoretical — GUI frameworks and some Rust crates initialize COM as STA.

2. **`CoUninitialize` is never called.** Each successful `CoInitializeEx` increments a per-thread reference count; the missing `CoUninitialize` leaks it. For long-running processes that play many sound effects, this accumulates.

**Impact:** On threads with STA COM (common in GUI apps embedding playa), every WASAPI SFX attempt silently fails and falls through to the rodio default path. The user gets the wrong audio category routing with no diagnostic.

**Suggested fix:**

```rust
let hr = CoInitializeEx(None, COINIT_MULTITHREADED);
let needs_uninit = hr.is_ok(); // S_OK or S_FALSE
// ... WASAPI work ...
if needs_uninit {
    CoUninitialize();
}
```

Handle `RPC_E_CHANGED_MODE` by proceeding (COM is already initialized) rather than failing.

---

## High Issues

### H1. Linux PulseAudio SFX: no connection or playback timeout

**File:** `lib/src/sfx_player.rs:863-872` and `894-903`

The PulseAudio context connection and stream connection loops have **no timeout**:

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

If the PulseAudio daemon is running but unresponsive, `iterate(true)` blocks per iteration, and this loop runs indefinitely. The same pattern repeats for stream connection (line 894) and drain (line 913).

Compare: the macOS and Windows SFX paths have explicit timeouts. The native_player and rodio SFX paths use `open_sfx_stream_with_timeout()` with a bounded deadline. The PulseAudio path bypasses all of that.

**Impact:** On a system with a hung PulseAudio daemon, `play_sfx_as_event` blocks the calling thread forever. Since `SoundEffect::play_with_options()` calls this synchronously, the entire process hangs.

**Suggested fix:** Add an `Instant::now() + timeout` deadline check inside each wait loop:

```rust
let deadline = Instant::now() + Duration::from_secs(5);
loop {
    if Instant::now() >= deadline {
        return Err("PulseAudio context connection timed out".into());
    }
    iterate_or_fail(&mut mainloop)?;
    // ...
}
```

### H2. Linux PulseAudio ducking: volume delta rounding causes drift

**File:** `lib/src/ducking/linux.rs:193-207`

The `fade_to_floor` and `fade_restore` methods re-read the current volume on each fade step and compute a relative delta:

```rust
if let Ok(Some(app)) = controller.get_app_by_index(*index) {
    let current_percent = app.volume.avg().0 as f64 / 65536.0 * 100.0;
    let delta = percent - current_percent;
    if delta < 0.0 {
        let _ = controller.decrease_app_volume_by_percent(*index, -delta);
    }
}
```

PulseAudio internally quantizes volume to a `u32` on a 0-65536 scale. Each `increase/decrease_app_volume_by_percent` call introduces rounding error that accumulates across fade steps. After a full duck-and-restore cycle, the restored volume may noticeably differ from the original.

**Impact:** Repeated ducking cycles gradually shift application volumes away from user-set levels.

**Suggested fix:** Use absolute volume setting if pulsectl-rs exposes it. If only relative adjustments are available, compute the final step as the delta from current to the exact original cached in `original_volumes`, ensuring the last step always lands on the correct value:

```rust
// On the final step, snap to the exact original
if is_last_step {
    let exact_delta = original_percent - current;
    // apply exact_delta
}
```

### H3. Linux ALSA ducking: ducks Playa's own audio

**File:** `lib/src/ducking/linux.rs:293-395`

The `AlsaBackend` adjusts the Master/PCM/Speaker/Headphone mixer control, which affects **all audio output** system-wide, including Playa's own playback stream. Unlike the PulseAudio backend (which excludes Playa's streams by PID), ALSA has no per-application volume concept.

The factory logs a warning at selection time (`factory.rs:78`):

```rust
eprintln!("Warning: PulseAudio not available, using ALSA fallback (affects all audio)");
```

But the actual problem is more severe than "affects all audio" — it means the audio Playa is trying to play will also be ducked, potentially making it inaudible at low floor values.

**Impact:** On ALSA-only systems, ducking makes Playa's own output quieter alongside everything else, defeating the purpose.

**Suggested fix:** Document this clearly in `AlsaBackend` doc comments. Consider skipping the ALSA fallback entirely when ducking would affect the playback device, or boost Playa's player volume to compensate for the system-wide duck.

---

## Medium Issues

### M1. Linux PulseAudio SFX blocks the calling thread

**File:** `lib/src/sfx_player.rs:812-925`

The entire `play_sfx_as_event()` function is synchronous — it uses `Mainloop::new()` (standard, not threaded) with `iterate(true)` which blocks until events arrive. This means context connection, stream setup, audio writing, and drain are all blocking the calling thread.

Compare: the macOS path returns quickly after queuing audio in CoreAudio/rodio. The Windows path is also synchronous but bounded by the WASAPI drain timeout. The PulseAudio path has no upper bound on how long it blocks (see H1).

When called from `SoundEffect::play_with_options()`, which is a sync function, this blocks the thread. If the caller is in an async context and didn't use `spawn_blocking`, the Tokio runtime is blocked.

**Suggested fix:** Either:

1. Wrap the PulseAudio operation in `std::thread::spawn` with a timeout (matching how `open_sfx_stream_with_timeout` already works), or
2. Document that `play_sfx_as_event` is blocking and should not be called from async contexts without `spawn_blocking`.

### M2. Windows and Linux SFX speed control: pitch shift vs. time stretch

**File:** `lib/src/sfx_player.rs:631-635` (Windows) and `837-839` (Linux)

Both platforms implement speed control by lying about the sample rate:

```rust
let effective_rate = (sample_rate_u32 as f32 * speed) as u32;
```

This causes pitch-shifted playback (chipmunk effect at 2x). The rodio fallback path uses `player.set_speed()`, which may use a different algorithm. The macOS path doesn't have this issue because it uses the rodio/default path for SFX.

**Impact:** Speed control sounds different on Windows/Linux (pitch-shifted) vs. the rodio fallback (potentially time-stretched). Users may notice inconsistent audio quality across platforms.

**Suggested fix:** Document this behavior as a known limitation. If consistent behavior is important, always use the rodio default path when speed control is requested, falling back to platform-specific paths only for volume and routing:

```rust
if options.speed.is_some() {
    // Skip platform-specific path to get consistent speed behavior
    // Fall through to rodio default path
} else {
    // Use WASAPI/PulseAudio for audio category routing
}
```

### M3. Temp files are never cleaned up

**File:** `lib/src/playback.rs:413-436`

`write_temp_audio()` creates files like `/tmp/playa-<pid>-<timestamp>.audio` but never deletes them. Neither the sync nor async paths clean up after the player process finishes.

```rust
fn write_temp_audio(bytes: &[u8]) -> Result<PathBuf, PlaybackError> {
    // ... writes file ...
    std::fs::write(&path, bytes)?;
    return Ok(path);
    // No cleanup registered
}
```

**Impact:** On all platforms, repeated playback of `AudioData::Bytes` accumulates temp files in the system temp directory. On Linux servers with small `/tmp` partitions, this could fill the filesystem. On Windows, `%TEMP%` is never cleaned by the OS.

**Suggested fix:** Delete the temp file after the player process exits:

```rust
// In playa_with_player_and_options, after child.wait():
if let ResolvedSource::Path(ref path) = source {
    if path.starts_with(std::env::temp_dir()) {
        let _ = std::fs::remove_file(path);
    }
}
```

Or use `tempfile::NamedTempFile` with auto-cleanup on drop.

### M4. Windows player detection: no guidance when no player found

**File:** `lib/src/error.rs:44-48` and `lib/src/player.rs`

The `AudioPlayer` enum contains no Windows-specific player. Windows has no built-in CLI audio player equivalent to macOS's `afplay` or Linux's `aplay`. On a bare Windows installation without mpv, ffplay, VLC, or SoX, `match_available_players()` returns an empty list, and the user gets:

```
no compatible player available for Mp3
```

This error gives no hint about what to install.

**Impact:** Windows users with native playback disabled (or for formats that don't decode natively) get an opaque error with no recovery path.

**Suggested fix:** Enhance the `NoCompatiblePlayer` error message on Windows to suggest installing mpv or FFmpeg:

```rust
#[cfg(target_os = "windows")]
{
    eprintln!("Hint: install mpv (https://mpv.io) or FFmpeg (ffplay) for audio playback");
}
```

### M5. ALSA ducking only sets 3 channels

**File:** `lib/src/ducking/linux.rs:452-458`

```rust
for channel in [
    SelemChannelId::FrontLeft,
    SelemChannelId::FrontRight,
    SelemChannelId::FrontCenter,
] {
    let _ = elem.set_playback_volume(channel, target);
}
```

On surround sound setups (5.1, 7.1), additional channels (RearLeft, RearRight, LFE, SideLeft, SideRight) are not ducked. The rear and LFE channels continue at full volume while front channels are attenuated.

**Impact:** Inconsistent ducking on surround sound systems. Rear channels stay at full volume, creating an unbalanced mix during ducking.

**Suggested fix:** Iterate over all channels that have playback volume:

```rust
use alsa::mixer::SelemChannelId::*;
for channel in [FrontLeft, FrontRight, FrontCenter, RearLeft, RearRight,
                FrontLeftOfCenter, FrontRightOfCenter, SideLeft, SideRight, Woofer] {
    let _ = elem.set_playback_volume(channel, target);
}
```

Or use the ALSA API to query which channels exist on the element and set all of them.

---

## Low Issues

### L1. `channels.rs` SFX device detection is a no-op on Linux/Windows

**File:** `lib/src/channels.rs:100-101`

```rust
#[cfg(not(all(target_os = "macos", feature = "sfx-native-macos")))]
let sfx_device_name = default_audio_name.clone();
```

On Linux and Windows, `is_default_sfx` always equals `is_default_audio`. The `is_default_sfx` field becomes misleading in the `OutputChannel` struct — it's supposed to indicate whether the device is the system sound effects device, but on non-macOS platforms it's just a mirror of `is_default_audio`.

**Suggested fix:** Either implement platform-specific SFX device detection, or set `is_default_sfx = false` on non-macOS platforms to honestly indicate that SFX routing is macOS-only. Document this in the `OutputChannel` struct.

### L2. Windows WASAPI SFX: temp audio `.audio` extension may confuse players

**File:** `lib/src/playback.rs:421`

```rust
let filename = format!("playa-{}-{}.audio", std::process::id(), timestamp);
```

The `.audio` extension has no file association on any OS. On macOS/Linux, host players read file headers (magic bytes), so this works. On Windows, some players may rely on file extensions for format detection. If native playback fails and the SFX falls through to host player delegation via `Playa::from_bytes()`, the temp file's `.audio` extension could cause the player to reject it.

**Suggested fix:** This is unlikely to cause real issues since playa detects format from bytes and passes the detected format to the player. But for robustness, consider using the detected format's extension (`.wav`, `.mp3`, etc.) in the temp filename.

### L3. `pulsectl-rs` 0.3 compatibility with PipeWire

**File:** `lib/Cargo.toml:68`

```toml
pulsectl-rs = { version = "0.3", optional = true }
```

PipeWire is now the default audio server on most modern Linux distributions (Fedora 34+, Ubuntu 22.10+, Arch). PipeWire exposes a PulseAudio compatibility layer, but `pulsectl-rs` 0.3 may not handle all PipeWire-specific behaviors (e.g., different sink input indexing, module-role-ducking semantics).

**Suggested fix:** Test the ducking path on a PipeWire system. If issues arise, consider updating `pulsectl-rs` or switching to direct `libpulse-binding` calls for the ducking backend (matching what the SFX path already does).

### L4. Windows SFX: unsafe blocks lack safety comments

**File:** `lib/src/sfx_player.rs:637-733`

The Windows WASAPI `play_sfx_with_category()` function contains a large `unsafe` block spanning nearly 100 lines. The safety invariants are not documented:

```rust
unsafe {
    let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    // ... 90 lines of WASAPI calls ...
    client.Stop()?;
}
```

While the `windows` crate provides relatively safe wrappers, the raw pointer arithmetic in the render loop (`std::slice::from_raw_parts_mut` at line 705) does require safety documentation:

```rust
let dst = std::slice::from_raw_parts_mut(
    buf_ptr as *mut f32,
    frames_to_write as usize * ch,
);
```

**Suggested fix:** Add `// SAFETY:` comments for the `from_raw_parts_mut` call explaining that `buf_ptr` is valid for `frames_to_write * channels * sizeof(f32)` bytes as guaranteed by `GetBuffer`, and that no other references to the buffer exist during the write.

### L5. Linux install path: missing audio-ducking features

**File:** `justfile:250-266`

The `just install` recipe enables `sfx-native-linux` on Linux but never enables `audio-ducking-linux`:

```bash
Linux)
    if pkg-config --exists alsa 2>/dev/null; then
        EXTRA_FEATURES="--features sfx-native-linux"
    else
        INSTALL_FLAGS="--no-default-features"
    fi
    ;;
```

On macOS, the ducking features are similarly not enabled in the install recipe — ducking is presumably opt-in. But the comment at line 234 says "enables native playback on macOS/Windows" without mentioning ducking, which could confuse users who expect ducking to work after install.

**Suggested fix:** Add a comment in the install recipe or README noting that ducking requires explicit feature flags:

```bash
# Audio ducking requires additional feature flags:
#   --features audio-ducking-linux (Linux, requires PulseAudio or ALSA dev headers)
#   --features audio-ducking-macos (macOS)
```

---

## Architecture Observations

### Native SFX circuit breaker coverage

The recently added native audio circuit breaker (`native_audio.rs`) correctly gates the rodio default path in both `play_native()` and `play_sfx()`. However, the platform-specific SFX paths (Windows WASAPI at line 174, Linux PulseAudio at line 183) run **before** the rodio default path and are **not** gated by the circuit breaker:

```rust
pub fn play_sfx(bytes: &[u8], options: &PlaybackOptions) -> Result<(), SfxPlaybackError> {
    if !native_audio_available() { ... }  // gates rodio path

    #[cfg(all(target_os = "windows", feature = "sfx-native-windows"))]
    {
        if windows_sfx::play_sfx_with_category(bytes, options).is_ok() {
            return Ok(());  // bypasses circuit breaker check
        }
    }
    // ...
}
```

Wait — re-reading the code, the `native_audio_available()` check is at line 168, which is **before** the Windows/Linux platform paths at lines 174/183. So the circuit breaker does correctly gate all native paths. The platform-specific paths are only reached if native audio is still enabled. This is correct.

### Feature flag matrix

The feature flag layering is well-designed:

| Feature | Requires | Platform |
|---------|----------|----------|
| `sfx-native` | `rodio` | All |
| `sfx-native-macos` | `sfx-native` + `coreaudio-sys` | macOS |
| `sfx-native-windows` | `sfx-native` + `windows` | Windows |
| `sfx-native-linux` | `sfx-native` + `libpulse-binding` | Linux |
| `native-playback` | `sfx-native` + symphonia codecs | All |
| `audio-ducking` | `tokio` | All |
| `audio-ducking-macos` | `audio-ducking` + `coreaudio-sys` | macOS |
| `audio-ducking-linux` | `audio-ducking` + `pulsectl-rs` + `alsa` | Linux |

Notable: there is no `audio-ducking-windows` feature. Windows ducking is a planned but unimplemented gap.

### Error handling consistency

The platform-specific SFX paths (Windows WASAPI, Linux PulseAudio) use `Box<dyn std::error::Error>` for their error types, while the rest of the library uses typed `thiserror` enums (`SfxPlaybackError`, `NativePlaybackError`). This works because the callers only check `.is_ok()`, but it means platform-specific errors are opaque in logs.

---

## Files Reviewed

| File | Lines | OS-Specific Code |
|------|-------|-----------------|
| `lib/src/sfx_player.rs` | 1097 | macOS CoreAudio FFI, Windows WASAPI, Linux PulseAudio |
| `lib/src/ducking/factory.rs` | 231 | Platform backend selection |
| `lib/src/ducking/linux.rs` | 582 | PulseAudio sink inputs, ALSA mixer |
| `lib/src/ducking/macos.rs` | 393 | CoreAudio volume control |
| `lib/src/ducking/media_keys.rs` | 440 | macOS AppleScript media keys |
| `lib/src/ducking/types.rs` | 111 | SessionId variants per platform |
| `lib/src/ducking/guard.rs` | 265 | RAII guard (platform-agnostic) |
| `lib/src/ducking/mod.rs` | 68 | Platform module gating |
| `lib/src/channels.rs` | 189 | macOS system sound device, cpal enumeration |
| `lib/src/native_player.rs` | 398 | rodio playback (platform-agnostic via cpal) |
| `lib/src/native_audio.rs` | 234 | Circuit breaker (platform-agnostic) |
| `lib/src/playback.rs` | 1050 | Command building, temp files |
| `lib/src/player.rs` | 692 | Player definitions (platform-aware) |
| `lib/src/error.rs` | 118 | Error types (platform-agnostic) |
| `lib/src/detection.rs` | 162 | Format detection (platform-agnostic) |
| `lib/Cargo.toml` | 80 | Platform-specific dependencies |
| `cli/Cargo.toml` | 30 | Feature flag wiring |
| `justfile` | 271 | Platform-specific install |

---

## Recommendations (Priority Order)

1. **Fix COM initialization** in Windows SFX path (C2) — silent failure on STA threads
2. **Add timeouts** to Linux PulseAudio SFX connection/drain loops (H1)
3. **Implement or warn** about Windows ducking gap (C1)
4. **Use absolute volume** in Linux PulseAudio ducking to prevent drift (H2)
5. **Document ALSA self-ducking** limitation clearly (H3)
6. **Add temp file cleanup** after host player playback (M3)
7. **Add player install hints** on Windows for `NoCompatiblePlayer` error (M4)
8. **Document speed/pitch behavior** difference across platforms (M2)
9. **Add surround channels** to ALSA ducking backend (M5)
10. **Add PulseAudio blocking documentation** or wrap in thread (M1)
