---
name: playa
description: Audio playback via native OS backends or host CLI players, with format detection, capability-ranked player matching, 88 embedded sound effects, output-channel routing, and optional OS-specific audio ducking. Use when working with audio playback, the playa package, so-you-say TTS CLI, or implementing sound effects.
---

# playa

Audio playback library that prefers native OS playback when available, falls back to host CLI players when needed, provides 88 embedded sound effects, and supports optional OS-specific audio ducking.

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

## Playback Model

Playa is no longer just a host-player wrapper.

- Native-first: use the built-in decoder/device path when the build and the current OS support it
- Host fallback: delegate to ranked installed players when native playback is unavailable, unsupported, timed out, or forced off
- Escape hatch: `--force-host` or `.force_host()` skips the native path

Native audio uses bounded device-open deadlines. If a native device-open operation times out, Playa trips a process-local circuit breaker and future native playback attempts fall back directly to host playback for the rest of the process.

The library keeps `native-playback` opt-in; the Playa CLI and speech-producing
consumers enable it explicitly. All automatic builder/free-function entry points
then use this pipeline. APIs that explicitly name an `AudioPlayer` remain
host-only.

`play_with_report` and `play_async_with_report` return the selected route,
expected and elapsed duration, and `Complete`, `Truncated`, or `Unverified`.
Truncation is diagnostic and does not replay. mpv receives
`--audio-channels=stereo` only for positively probed mono input; unknown,
stereo, and multichannel sources keep their original layout.

## Detached playback

`Playa::play_detached`, `playa --background`, and effect background playback
publish to one private per-user spool. Publication returns after an existing or
new scheduler owns the job. The scheduler preserves global sequence without
overlap, survives requester exit, and delegates each job to the absolute
executable that enqueued it so options and compile-time capabilities are not
lost. Delivery is best-effort and at-most-once after playback starts.

The queue rejects linked/reparse-point records and enforces private ownership
and `0700` permissions on Unix. Windows paths use lossless wide-string encoding;
Unix paths preserve raw bytes. `playa spool` displays only redacted state and
journal outcomes; private speech text never appears there. `PLAYA_DRY_RUN=1`
short-circuits before source, cache, spool, journal, or subprocess side effects.

## Player Selection

Host players are ranked by capability (speed, volume, streaming). Top tier (score 9): mpv, FFplay, SoX.

```rust
// Get ranked compatible players
let players = match_available_players(format);
let best = players.first().expect("No compatible player installed");
```

## Sound Effects

```rust
// Feature-gated: sfx-ui, sfx-cartoon, sfx-reactions, etc.
let effect = SoundEffect::from_name("sad-trombone").expect("effect enabled");
effect.play()?;
```

Native SFX playback is OS-specific when enabled:

- macOS: route through the configured system sound device when possible
- Windows: use WASAPI `AudioCategory_SoundEffects`
- Linux: use PulseAudio/PipeWire with `media.role=event`

If the native SFX path fails, Playa falls back to regular playback or host-player delegation.

## CLI

```bash
playa audio.wav                     # Play file
playa play --fast audio.mp3         # 1.25x speed
playa effect sad-trombone           # Built-in effect
playa list-effects                  # List all effects
playa list-effects cartoon          # Filter effects
playa players                       # Show host player table
playa output-channels               # Show native output devices (with `sfx-native`)
playa --channel "<device>" tone.wav # Route to a specific output device
playa duck-info                     # Audio ducking backend info
playa spool                         # Redacted detached queue/journal status
playa --no-duck audio.wav           # Disable ducking
playa --force-host audio.wav        # Skip native playback
```

Shell completions are available for Bash, Zsh, and Fish. Effect names, audio files, volume levels, and output channels autocomplete.

## Audio Ducking

Feature-gated via `audio-ducking` plus the OS slice for the target platform.

```rust
use playa::ducking::DuckConfig;

let config = DuckConfig::new(750, 0.25)?;
Playa::from_path("audio.mp3")?
    .with_ducked_audio(config)
    .play_async()
    .await?;
```

Current backend behavior:

- macOS: CoreAudio virtual master volume when the output device exposes software volume; otherwise media-key pause/resume fallback
- Windows: WASAPI per-session ducking on the default render endpoint, excluding Playa's own process; active sessions fade in lockstep within one shared ramp window
- Linux: PulseAudio/PipeWire per-application ducking, excluding Playa by PID/name; ALSA fallback is coarse and ducks Playa's own output too

Practical notes:

- `playa duck-info` is the fastest way to inspect the selected backend on the current machine
- Ducking failures should degrade to normal playback rather than blocking audio
- Windows and Linux only duck sessions/applications visible when playback starts

## Detailed Topics

- [Players](./players.md) - Host player scoring and matching
- [Sound Effects](./effects.md) - 88 effects, feature flags, native SFX routing
- [Integration](./integration.md) - native playback, channel routing, ducking, and fallback patterns
- [Audio Programming by OS](./audio-programming/SKILL.md) - Deep background on the platform audio subsystems Playa sits on top of (macOS CoreAudio, Windows WASAPI, Linux PulseAudio/PipeWire, iOS, Android), plus the Rust crate and TypeScript library landscapes. Reach for this when implementing or troubleshooting a native backend, output-channel routing, or ducking on a specific OS.

## See Also

- [playa/README.md](../../../playa/README.md) - Package overview
- [playa/docs/audio-ducking.md](../../../playa/docs/audio-ducking.md) - Ducking design notes
