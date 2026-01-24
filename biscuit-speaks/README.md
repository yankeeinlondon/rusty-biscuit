# Biscuit Speaks TTS Library

<table>
<tr>
<td><img src="../assets/biscuit-speaks-512.png" style="max-width='25%'" width=200px /></td>
<td>
<h2>biscuit-speaks</h2>
<p>This library provides TTS functionality to Rust programs:</p>

<ul>
  <li>leverages the host's existing capabilities for small yet cross-platform TTS functionality</li>
  <li>includes <i>cloud</i> support for <a href="https://elevenlabs.io/docs/overview/intro">ElevenLabs</a> TTS</li>
  <li>automatic failover between providers</li>
</ul>

<p>
  This library is the TTS functionality behind the <code>so-you-say</code> CLI.
</p>
</td>
</tr>
</table>

## Usage Examples

```rust
use biscuit_speaks::{Speak, TtsConfig, Gender, VolumeLevel};

// Simple usage with defaults (async)
Speak::new("Hello, world!").play().await?;

// With custom voice selection
Speak::new("Custom voice")
    .with_voice("Samantha")
    .with_gender(Gender::Female)
    .with_volume(VolumeLevel::Soft)
    .play()
    .await?;

// Fire-and-forget (ignores errors)
biscuit_speaks::speak_when_able("Task complete!", &TtsConfig::default()).await;
```

## Features

- **Cross-platform system TTS**: macOS (`say`), Windows (SAPI), Linux (eSpeak, Festival)
- **Cloud TTS**: ElevenLabs API integration with high-quality voices
- **Automatic failover**: Tries providers in priority order until one succeeds
- **Async-first**: Built on tokio for non-blocking operations
- **Builder pattern**: Ergonomic configuration via `TtsConfig` or `Speak` builder

## Supported Providers

### Host Providers (Implemented)

| Provider | Platform | Binary | Notes |
|----------|----------|--------|-------|
| Say | macOS | `say` | Built-in macOS TTS |
| eSpeak | Cross-platform | `espeak-ng` | Open source synthesizer |

### Cloud Providers (Implemented)

| Provider | API Key Env Var | Notes |
|----------|-----------------|-------|
| ElevenLabs | `ELEVEN_LABS_API_KEY` or `ELEVENLABS_API_KEY` | High-quality AI voices, sound effects |

#### ElevenLabs Features

The ElevenLabs provider offers full API access:
- **Text-to-Speech**: Generate high-quality audio from text
- **Voice Listing**: Query available voices with metadata
- **Model Listing**: List available TTS models
- **Sound Effects**: Generate sound effects from text descriptions

### Deferred Providers (Not Yet Implemented)

The following providers are defined in the type system but not yet implemented:

| Provider | Platform | Requirements |
|----------|----------|--------------|
| SAPI | Windows | PowerShell with `System.Speech` assembly |
| Festival | Linux | `festival` binary installed |
| Pico2Wave | Linux | `pico2wave` binary (SVOX Pico) |
| Mimic3 | Cross-platform | Mycroft neural TTS, SSML support |
| KokoroTts | Cross-platform | Model files required |
| EchoGarden | Cross-platform | Model configuration required |
| Sherpa | Cross-platform | `SHERPA_MODEL` and `SHERPA_TOKENS` env vars |
| Gtts | Cross-platform | `gtts-cli`, requires network |
| SpdSay | Linux | Speech Dispatcher (`spd-say`) |
| Piper | Cross-platform | Fast local neural TTS with ONNX |

#### Implementation Requirements for Deferred Providers

- **SAPI (Windows)**: Implement PowerShell script invocation with `Add-Type -AssemblyName System.Speech`. Handle voice selection via `SelectVoice()`.

- **Festival**: Simple CLI wrapper, pipe text to `festival --tts`.

- **Pico2Wave**: Generate WAV file with `pico2wave -w /tmp/output.wav "text"`, then play via audio player.

- **Mimic3**: CLI wrapper with SSML support. Requires model download.

- **KokoroTts**: Requires model files and configuration. Supports voice blending syntax (e.g., `af_sarah:60,am_adam:40`).

- **EchoGarden**: Needs model discovery and configuration.

- **Sherpa**: Validate `SHERPA_MODEL` and `SHERPA_TOKENS` environment variables exist and point to valid files.

- **Gtts**: HTTP-based Google TTS via CLI. Handle network errors gracefully.

- **SpdSay**: Simple CLI wrapper for Speech Dispatcher on Linux desktops.

- **Piper**: CLI wrapper for ONNX-based neural TTS. Very fast local inference.

## Environment Variables

| Variable | Description |
|----------|-------------|
| `ELEVENLABS_API_KEY` | ElevenLabs API key (preferred) |
| `ELEVEN_LABS_API_KEY` | ElevenLabs API key (alternative) |
| `TTS_PROVIDER` | Override default provider selection |

## API Overview

### `Speak` Builder

The main API for TTS operations:

```rust
use biscuit_speaks::{Speak, TtsFailoverStrategy, TtsProvider, CloudTtsProvider};

// Use ElevenLabs specifically
Speak::new("High quality voice")
    .with_failover(TtsFailoverStrategy::SpecificProvider(
        TtsProvider::Cloud(CloudTtsProvider::ElevenLabs)
    ))
    .play()
    .await?;

// Prefer cloud providers
Speak::new("Cloud first")
    .with_failover(TtsFailoverStrategy::PreferCloud)
    .play()
    .await?;
```

### Direct Provider Access

```rust
use biscuit_speaks::{ElevenLabsProvider, TtsExecutor, TtsConfig};

// Direct ElevenLabs usage
let provider = ElevenLabsProvider::new()?;
provider.speak("Direct API call", &TtsConfig::default()).await?;

// Generate audio bytes without playing
let audio_bytes = provider.generate_audio("Get audio", &TtsConfig::default()).await?;

// List available voices
let voices = provider.list_voices().await?;
for voice in voices.voices {
    println!("{}: {}", voice.voice_id, voice.name);
}

// List available models
let models = provider.list_models().await?;
for model in models {
    println!("{}: {}", model.model_id, model.name);
}

// Create sound effects
let sfx = provider.create_sound_effect("dog barking loudly", Some(3.0)).await?;
```

### Available Providers

```rust
use biscuit_speaks::get_available_providers;

// List detected providers
for provider in get_available_providers() {
    println!("Available: {:?}", provider);
}
```
