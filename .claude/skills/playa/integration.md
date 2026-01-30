# Integration Patterns

## Architecture

```
playa/
├── lib/           # Core library
│   ├── audio.rs       # Audio wrapper with pause tracking
│   ├── player.rs      # 13 players with capability scoring
│   ├── detection.rs   # Format detection (infer + extension fallback)
│   ├── playback.rs    # Sync/async playback functions
│   ├── playa.rs       # Builder API (Playa struct)
│   ├── types.rs       # Codec, AudioFileFormat, PlaybackOptions
│   ├── effects.rs     # 53 embedded sound effects
│   └── error.rs       # Error types
├── cli/           # Binary: `playa`
└── effects/       # 53 embedded audio files (~30MB)
```

## TTS Integration (so-you-say)

```rust
// Generate TTS audio, then play via playa
let audio_bytes = tts_provider.synthesize("Hello")?;
Playa::from_bytes(audio_bytes)?
    .speed(1.1)  // Slightly faster for TTS
    .play()?;
```

## Notification Sounds

```rust
#[cfg(feature = "sfx-reactions")]
fn notify_complete() {
    if let Some(effect) = SoundEffect::from_name("small-group-cheer") {
        let _ = effect.play();
    }
}
```

## sniff-lib Integration

Playa uses `sniff-lib` for player detection:

```rust
use sniff_lib::programs::InstalledHeadlessAudio;

let installed = InstalledHeadlessAudio::new();
if installed.is_installed(player.as_headless_audio()) {
    // Player available
}
```

## Error Handling

```rust
use playa::{DetectionError, InvalidAudio, PlaybackError};

// DetectionError: Format detection failed
// InvalidAudio: Cannot create Audio from source
// PlaybackError: Player execution failed
```

## Async Playback

Enable with `async` feature:

```rust
// Cargo.toml: playa = { features = ["async"] }

let audio = Audio::from_path("audio.wav")?;
audio.play_async().await?;
```
