---
name: playa
description: Audio playback via host CLI players with format detection, capability-ranked player matching, and embedded sound effects. Use when working with audio playback, the playa package, so-you-say TTS CLI, or implementing sound effects.
---

# playa

Audio playback library that detects formats, matches the best available player, and provides 53 embedded sound effects.

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
playa --fast audio.mp3         # 1.25x speed
playa --effect sad-trombone    # Built-in effect
playa --list-effects           # List all effects
playa --players                # Show player table
```

## Detailed Topics

- [Players](./players.md) - Capability scoring, 13 supported players
- [Sound Effects](./effects.md) - 53 effects, feature flags
- [Integration](./integration.md) - TTS, sniff-lib, patterns

## See Also

- [playa/README.md](../../../playa/README.md) - Package overview
