# Player Capability Scoring

Players are automatically ranked by capability for format-appropriate selection.

## Scoring System

| Capability | Score | Why |
|------------|-------|-----|
| Speed control | +4 | Most valuable for TTS playback rate |
| Volume control | +3 | Commonly needed |
| Stream input | +2 | Enables piping without disk I/O |

## Player Tiers

**Tier 1 (score 9)**: mpv, FFplay, SoX - full controllability
**Tier 2 (score 5-7)**: VLC, MPlayer, GStreamer, afplay - volume + some features
**Tier 3 (score 2-3)**: mpg123, paplay, PipeWire - limited control
**Tier 4 (score 0)**: aplay, ogg123 - no controllability

## Supported Players (13)

| Player | OS | Speed | Volume | Stream |
|--------|:--:|:-----:|:------:|:------:|
| mpv | All | Y | Y | Y |
| FFplay | All | Y | Y | Y |
| SoX | All | Y | Y | Y |
| afplay | macOS | Y | Y | N |
| VLC | All | N | Y | Y |
| MPlayer | All | N | Y | Y |
| GStreamer | All | N | Y | Y |
| paplay | Linux | N | Y | N |
| PipeWire | Linux | N | Y | N |
| mpg123 | All | N | N | Y |
| pacat | Linux | N | N | Y |
| ogg123 | All | N | N | N |
| aplay | Linux | N | N | N |

## Player Matching API

```rust
// Get all compatible players, ranked by capability
let players = match_players(format);

// Get only installed players
let available = match_available_players(format);

// Check specific player
use sniff_lib::programs::InstalledHeadlessAudio;
let installed = InstalledHeadlessAudio::new();
if installed.is_installed(player.as_headless_audio()) {
    // Player available
}
```
