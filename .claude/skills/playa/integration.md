# Integration Patterns

## Architecture

```text
playa/
├── lib/               # Core library
│   ├── audio.rs           # Audio wrapper with pause tracking
│   ├── player.rs          # Host player ranking and matching
│   ├── detection.rs       # Format detection (infer + extension fallback)
│   ├── playback.rs        # Sync/async playback entry points
│   ├── playa.rs           # Builder API (Playa struct)
│   ├── metadata.rs        # Runtime duration/channel probing
│   ├── report.rs          # Serializable completion reports
│   ├── detached/          # Private spool, protocol, scheduler, delegation
│   ├── native_player.rs   # Native file/bytes playback path
│   ├── native_audio.rs    # Shared native timeout + circuit-breaker logic
│   ├── sfx_player.rs      # Native sound effects playback
│   ├── channels.rs        # Output device enumeration and selection
│   ├── ducking/           # Audio ducking (feature-gated)
│   ├── windows_com.rs     # Shared Windows COM helpers
│   ├── effects.rs         # 88 embedded sound effects
│   └── error.rs           # Error types
├── cli/               # Binary: `playa`
└── effects/           # Embedded audio files
```

## Playback Strategy

Prefer these rules when building on top of Playa:

- Default to native playback first for lower latency and output-device routing
- Fall back to host players when native decode/device open fails or when the format/path is better handled externally
- Use `.force_host()` or `--force-host` when consistency with external players matters more than native routing
- Use `play_with_report` when route and completion evidence matter
- Use `play_detached` for durable, serialized fire-and-forget delivery

Native playback is intentionally defensive:

- device enumeration and open operations run with bounded deadlines
- a native device-open timeout trips a process-local breaker
- after the breaker trips, future native playback attempts fail fast and callers should fall back to host playback

All automatic APIs share this strategy. Explicit-player APIs are the only
host-only family. The `native-playback` library feature is opt-in even though
the Playa CLI and speech consumers enable it. The mpv fallback adds a stereo
output request only when metadata positively identifies mono input.

## Detached delivery

The `detached` module publishes versioned jobs to a private per-user spool.
Queue and worker locks guarantee ordered, non-overlapping playback and close the
final-empty handoff race. Delegation re-executes the losslessly encoded absolute
enqueuer executable, preserving feature capabilities; incompatible or replaced
executables fail the job without degrading options. Delivery is best-effort and
at-most-once after a job moves in flight.

Never expose preparation payloads: they can contain speech text and non-secret
TTS configuration. Journal and `playa spool` projections are redacted.
`PLAYA_DRY_RUN=1` must return before source reads or any cache/spool/process I/O.

## TTS Integration (`so-you-say`)

```rust
// Generate TTS audio, then play via Playa
let audio_bytes = tts_provider.synthesize("Hello")?;
Playa::from_bytes(audio_bytes)?
    .speed(1.1)
    .play()?;
```

Guidance:

- Use native playback when you want device selection and lower startup latency
- Use host-player fallback when you need maximum codec/tool availability on the target machine
- TTS playback often benefits from `speed(1.05..1.25)` rather than resynthesizing

## Notification Sounds

```rust
#[cfg(feature = "sfx-reactions")]
fn notify_complete() {
    if let Some(effect) = SoundEffect::from_name("small-group-cheer") {
        let _ = effect.play();
    }
}
```

For notification and UI sounds, prefer the native SFX path when available:

- macOS: system sound output device
- Windows: WASAPI `AudioCategory_SoundEffects`
- Linux: PulseAudio/PipeWire event-role stream

This keeps short cues aligned with platform mixer behavior instead of treating them like long-form media playback.

## Output Channel Routing

```rust
let options = playa::PlaybackOptions::new().with_channel("Built-in Output");

Playa::from_path("audio.wav")?
    .with_options(options)
    .play()?;
```

CLI equivalents:

- `playa output-channels` (with `sfx-native`)
- `playa --channel "<device>" audio.wav`
- `playa effect click --channel "<device>"`

Use channel routing when the caller cares about alerts vs. music outputs, multi-output setups, or explicit device targeting.

## Host Player Detection

Playa uses the `sniff` crate for host player detection:

```rust
use sniff::programs::InstalledHeadlessAudio;

let installed = InstalledHeadlessAudio::new();
if installed.is_installed(player.as_headless_audio()) {
    // Host player available
}
```

## Error Handling

```rust
use playa::{DetectionError, InvalidAudio, PlaybackError};
```

Operational guidance:

- treat native playback timeouts as a signal to fall back, not to retry in a tight loop
- do not assume ducking succeeded just because playback started
- for user-facing CLIs, surface whether playback used native or host fallback when debugging matters

## Audio Ducking

Feature-gated via `audio-ducking`. OS slices are selected by target platform:

- `audio-ducking-macos`
- `audio-ducking-windows`
- `audio-ducking-linux`

```rust
use playa::ducking::DuckConfig;

let config = DuckConfig::new(1000, 0.2)?;

Playa::from_path("audio.mp3")?
    .with_ducked_audio(config)
    .play_async()
    .await?;
```

CLI flags:

- `--no-duck`
- `--duck-ramp-ms <MS>` default `1000`
- `--duck-floor <LEVEL>` default `0.2`
- `duck-info` for backend diagnostics

Backend matrix:

- macOS: CoreAudio virtual master volume; fallback to media-key pause/resume when the device does not expose software volume
- Windows: WASAPI per-session ducking, excluding Playa's own PID; active sessions fade in lockstep within one shared ramp window
- Linux PulseAudio/PipeWire: per-application sink-input ducking with Playa self-exclusion
- Linux ALSA fallback: system-wide mixer attenuation that also ducks Playa's own output

Important caveats:

- Windows and Linux only affect sessions/applications present when playback starts
- Linux ALSA fallback is best-effort and changes all audio, not just competing apps
- ducking should degrade to normal playback on backend failure

## Async Playback

Enable with `async`:

```rust
Playa::from_path("audio.wav")?
    .play_async()
    .await?;
```

Use async when you need ducking, coordinated restore, or integration with a Tokio-based CLI/service.
