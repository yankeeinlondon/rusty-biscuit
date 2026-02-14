# Playa Library

Playa is a Rust library for playing audio using the host's installed command-line
players. It detects file formats and selects the best available player based on
capabilities.

## Features

- Audio format detection from files, URLs, or bytes
- Capability-ranked player matching (speed +4, volume +3, stream +2)
- Simple playback helpers for common players
- Stateful `Audio` wrapper with pause position tracking
- Builder API with fluent interface (`Playa`)
- Optional async support via `async` feature
- 88 embedded sound effects (feature-gated)
- Optional audio ducking via `audio-ducking` feature

## Usage

```rust
use playa::Audio;

let audio = Audio::from_path("audio.wav")?;
audio.play()?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

Builder API with options:

```rust
use playa::Playa;

Playa::from_path("audio.mp3")?
    .speed(1.25)
    .volume(0.8)
    .show_meta()
    .play()?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

## API Highlights

### Audio Sources

- `Audio::from_path`, `Audio::from_url`, `Audio::from_bytes`
- `Playa::from_path`, `Playa::from_bytes` (builder API)
- `Playa::play_async`, `Playa::with_ducked_audio` (requires `audio-ducking`)

### Detection

- `detect_audio_format_from_path`
- `detect_audio_format_from_url`
- `detect_audio_format_from_bytes`

### Playback

- `playa`, `playa_explicit`, `playa_explicit_with_options`
- `playa_with_player`, `playa_with_player_and_options`
- Async variants (feature-gated): `playa_async`, `playa_explicit_async`, etc.

### Player Matching

- `match_players(format)` - All compatible players, ranked
- `match_available_players(format)` - Installed players only
- `all_players()` - All 13 supported players

### Types

- `AudioPlayer` - 13 supported players enum
- `AudioData` - File path, URL, or bytes input
- `Codec` - PCM, FLAC, ALAC, MP3, AAC, Vorbis, Opus
- `AudioFileFormat` - WAV, AIFF, FLAC, MP3, OGG, M4A, WebM
- `PlaybackOptions` - Volume and speed control
- `SoundEffect` - 88 embedded sound effects (feature-gated)
- `DetectionError`, `InvalidAudio`, `PlaybackError` - Error types
