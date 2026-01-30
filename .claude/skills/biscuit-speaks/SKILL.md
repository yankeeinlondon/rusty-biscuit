---
name: biscuit-speaks
description: Cross-platform text-to-speech library with multi-provider support and automatic failover. Use when implementing TTS features, working with voice synthesis, integrating speech providers (ElevenLabs, macOS Say, eSpeak, Kokoro, Echogarden, gTTS, SAPI), or building the so-you-say CLI.
---

## Purpose

`biscuit-speaks` provides unified TTS functionality across multiple providers with:
- Automatic provider detection and failover
- OS-aware default provider stacks
- Voice capability caching
- Audio file caching (content-addressed via xxHash)
- Builder pattern APIs for ergonomic configuration

## Quick Start

```rust
use biscuit_speaks::{Speak, TtsConfig, Gender, VolumeLevel, SpeedLevel};

// Simple usage - speaks with best available provider
Speak::new("Hello, world!").play().await?;

// With configuration
Speak::new("Custom voice")
    .with_voice("Samantha")
    .with_gender(Gender::Female)
    .with_volume(VolumeLevel::Soft)
    .with_speed(SpeedLevel::Fast)
    .play()
    .await?;

// Get metadata about what was used
let result = Speak::new("Hello").play_with_result().await?;
println!("Used: {} via {:?}", result.voice.name, result.provider);
```

## Key Types

| Type | Purpose |
|------|---------|
| `Speak` | Builder for TTS operations |
| `TtsConfig` | Configuration container (voice, gender, language, volume, speed, failover) |
| `Voice` | Voice metadata (name, gender, quality, languages, identifier) |
| `TtsProvider` | Enum: `Host(HostTtsProvider)` or `Cloud(CloudTtsProvider)` |
| `TtsFailoverStrategy` | `FirstAvailable`, `PreferHost`, `PreferCloud`, `SpecificProvider(TtsProvider)` |
| `SpeakResult` | Metadata returned after speaking (provider, voice, model, cache_hit) |

## Traits

```rust
// Required for all providers
pub trait TtsExecutor: Send + Sync {
    async fn speak(&self, text: &str, config: &TtsConfig) -> Result<(), TtsError>;
    async fn speak_with_result(&self, text: &str, config: &TtsConfig) -> Result<SpeakResult, TtsError>;
    async fn is_ready(&self) -> bool;
    fn info(&self) -> &str;
}

// Optional for voice enumeration
pub trait TtsVoiceInventory: Send + Sync {
    async fn list_voices(&self) -> Result<Vec<Voice>, TtsError>;
}
```

## Provider Selection

Cloud providers (ElevenLabs) are NOT used automatically - must be explicitly requested via `TtsFailoverStrategy::SpecificProvider` or `PreferCloud`.

OS-specific default stacks (highest quality first):
- **macOS**: Say -> Kokoro -> EchoGarden -> Piper -> Sherpa -> ESpeak -> Gtts
- **Linux**: Kokoro -> EchoGarden -> Sherpa -> Piper -> Mimic3 -> ESpeak -> Festival -> SpdSay -> Gtts
- **Windows**: Sapi -> Piper -> EchoGarden -> Kokoro -> Sherpa -> ESpeak -> Gtts

## Environment Variables

| Variable | Purpose |
|----------|---------|
| `ELEVENLABS_API_KEY` / `ELEVEN_LABS_API_KEY` | ElevenLabs API authentication |
| `KOKORO_MODEL` | Path to `kokoro-v1.0.onnx` model file |
| `KOKORO_VOICES` | Path to `voices-v1.0.bin` voice embeddings |
| `TTS_PROVIDER` | Override default provider selection |
| `PREFER_LANGUAGE` | Default language preference |
| `PREFER_GENDER` | Default gender preference |
| `PREFER_VOICE` | Default voice preference |
| `PREFER_SPEED` | Default speed (`fast` or `slow`) |

## Caching

**Voice Capability Cache** (`~/.biscuit-speaks-cache.json`):
- Persists detected providers and available voices
- Prevents expensive re-enumeration

**Audio File Cache**:
- Content-addressed using xxHash
- Cache key: provider + voice_id + text + format + speed (conditional)
- Atomic writes to prevent corruption

## Detailed Documentation

- [Providers Reference](providers.md) - All supported TTS providers with configuration details
- [API Reference](api-reference.md) - Complete type definitions and usage patterns
- [Configuration Guide](configuration.md) - Builder patterns, failover strategies, caching

## Related Packages

- **so-you-say**: CLI that uses biscuit-speaks (`speak` binary)
- **playa**: Audio playback library (optional feature for `biscuit-speaks`)
- **sniff-lib**: System detection for available TTS providers
