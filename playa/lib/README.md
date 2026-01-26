# Playa Library

Playa is a Rust library for playing audio using the host's installed command-line
players. It detects file formats and selects the best available player based on
capabilities.

## Features

- Audio format detection from files, URLs, or bytes
- Capability-ranked player matching
- Simple playback helpers for common players
- Stateful `Audio` wrapper with pause position tracking

## Usage

```rust
use playa::Audio;

let audio = Audio::from_path("audio.wav")?;
audio.play()?;
# Ok::<(), Box<dyn std::error::Error>>(())
```

## API Highlights

- `Audio::from_path`, `Audio::from_url`, `Audio::from_bytes`
- `playa::playa` / `playa::playa_explicit`
- `match_players` / `match_available_players`
