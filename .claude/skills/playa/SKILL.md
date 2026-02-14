---
name: playa
description: Audio playback via host CLI players with format detection, capability-ranked player matching, 88 embedded sound effects, and optional audio ducking. Use when working with audio playback, the playa package, so-you-say TTS CLI, or implementing sound effects.
---

# playa

Audio playback library that detects formats, matches the best available player, provides 88 embedded sound effects, and supports optional audio ducking.

## Quick Start

```rust
// Simple playback
let audio = Audio::from_path("audio.wav")?;
audio.play()?;

// Builder API with options
Playa::from_path("audio.mp3")?
    .speed(1.25)
    .volume(0.8)
    .play()?;
```

## Player Selection

Players are ranked by capability (speed, volume, streaming). Top tier (score 9): mpv, FFplay, SoX.

```rust
// Get ranked compatible players
let players = match_available_players(format);
let best = players.first().expect("No player");
```

## Sound Effects

```rust
// Feature-gated: sfx-ui, sfx-cartoon, sfx-reactions, etc.
let effect = SoundEffect::from_name("sad-trombone")?;
effect.play()?;
```

## CLI

```bash
playa audio.wav                # Play file
playa play --fast audio.mp3    # 1.25x speed
playa effect sad-trombone      # Built-in effect
playa list-effects             # List all effects
playa list-effects cartoon     # Filter effects
playa players                  # Show player table
playa duck-info                # Audio ducking backend info
playa --no-duck audio.wav      # Disable ducking
```

Shell completions are available for Bash, Zsh, and Fish. Effect names autocomplete at `playa effect <TAB>`.

## Audio Ducking

Feature-gated (`audio-ducking`). Automatically lowers system volume during playback.

```rust
Playa::from_path("audio.mp3")?
    .with_ducked_audio()  // Lower system volume during playback
    .play_async().await?;
```

## Detailed Topics

- [Players](./players.md) - Capability scoring, 13 supported players
- [Sound Effects](./effects.md) - 88 effects, feature flags
- [Integration](./integration.md) - TTS, sniff-lib, audio ducking, patterns

## See Also

- [playa/README.md](../../../playa/README.md) - Package overview
