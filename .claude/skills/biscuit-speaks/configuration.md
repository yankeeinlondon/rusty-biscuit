# Configuration Guide

## Builder Pattern Overview

Both `Speak` and `TtsConfig` use the builder pattern with `#[must_use]` attributes for ergonomic, chainable configuration.

```rust
// Via Speak builder (preferred for simple cases)
Speak::new("Hello!")
    .with_voice("Samantha")
    .with_gender(Gender::Female)
    .play()
    .await?;

// Via TtsConfig (for reusable configuration)
let config = TtsConfig::new()
    .with_voice("Samantha")
    .with_gender(Gender::Female)
    .with_volume(VolumeLevel::Soft);

// Use with convenience functions
speak("Hello!", &config).await?;
speak("Goodbye!", &config).await?;
```

---

## Failover Strategies

### FirstAvailable (Default)

Uses OS-specific default stack, trying providers in quality order until one succeeds.

```rust
// Implicit - this is the default
Speak::new("Hello!").play().await?;

// Explicit
Speak::new("Hello!")
    .with_failover(TtsFailoverStrategy::FirstAvailable)
    .play()
    .await?;
```

**OS Default Stacks** (highest quality first):

**macOS**:
1. Say
2. KokoroTts
3. EchoGarden
4. Piper
5. Sherpa
6. ESpeak
7. Gtts

**Linux**:
1. KokoroTts
2. EchoGarden
3. Sherpa
4. Piper
5. Mimic3
6. ESpeak
7. Festival
8. SpdSay
9. Gtts

**Windows**:
1. Sapi
2. Piper
3. EchoGarden
4. KokoroTts
5. Sherpa
6. ESpeak
7. Gtts

### PreferHost

Tries all host providers before considering cloud providers.

```rust
Speak::new("Hello!")
    .with_failover(TtsFailoverStrategy::PreferHost)
    .play()
    .await?;
```

### PreferCloud

Tries cloud providers first (requires API keys), then falls back to host.

```rust
Speak::new("Hello!")
    .with_failover(TtsFailoverStrategy::PreferCloud)
    .play()
    .await?;
```

### SpecificProvider

Uses only the specified provider, failing if unavailable.

```rust
use biscuit_speaks::{TtsProvider, CloudTtsProvider, HostTtsProvider};

// Use ElevenLabs specifically
Speak::new("Premium voice")
    .with_failover(TtsFailoverStrategy::SpecificProvider(
        TtsProvider::Cloud(CloudTtsProvider::ElevenLabs)
    ))
    .play()
    .await?;

// Use macOS Say specifically
Speak::new("Native voice")
    .with_failover(TtsFailoverStrategy::SpecificProvider(
        TtsProvider::Host(HostTtsProvider::Say)
    ))
    .play()
    .await?;
```

---

## Voice Selection

### By Name

```rust
Speak::new("Hello!")
    .with_voice("Samantha")  // macOS
    .play()
    .await?;

Speak::new("Hello!")
    .with_voice("af_heart")  // Kokoro voice ID
    .play()
    .await?;
```

### By Gender Preference

```rust
Speak::new("Hello!")
    .with_gender(Gender::Female)
    .play()
    .await?;
```

### With Language

```rust
Speak::new("Bonjour!")
    .with_language(Language::Custom("fr".into()))
    .play()
    .await?;
```

### Combined Selection

```rust
Speak::new("Hello!")
    .with_voice("Samantha")
    .with_gender(Gender::Female)
    .with_language(Language::English)
    .play()
    .await?;
```

**Note**: When a specific voice is set, gender is ignored.

---

## Volume and Speed Control

### Volume Levels

```rust
// Preset levels
Speak::new("Loud!").with_volume(VolumeLevel::Loud).play().await?;     // 1.0
Speak::new("Normal").with_volume(VolumeLevel::Normal).play().await?;  // 0.75
Speak::new("Soft").with_volume(VolumeLevel::Soft).play().await?;      // 0.5

// Explicit value (clamped 0.0-1.0)
Speak::new("Custom").with_volume(VolumeLevel::Explicit(0.3)).play().await?;
```

### Speed Levels

```rust
// Preset levels
Speak::new("Fast!").with_speed(SpeedLevel::Fast).play().await?;     // 1.25x
Speak::new("Normal").with_speed(SpeedLevel::Normal).play().await?;  // 1.0x
Speak::new("Slow").with_speed(SpeedLevel::Slow).play().await?;      // 0.75x

// Explicit value (clamped 0.25-4.0)
Speak::new("Custom").with_speed(SpeedLevel::Explicit(1.5)).play().await?;
```

**Provider-Specific Speed Handling**:
- **Kokoro, Gtts**: Speed handled at playback (by playa)
- **EchoGarden, ElevenLabs**: Speed baked into audio generation

---

## Environment Variable Configuration

### Provider Selection Override

```bash
# Force a specific provider
export TTS_PROVIDER=say
export TTS_PROVIDER=kokoro
export TTS_PROVIDER=elevenlabs
```

### Default Preferences

```bash
# Language preference
export PREFER_LANGUAGE=en

# Gender preference
export PREFER_GENDER=female

# Voice preference
export PREFER_VOICE=Samantha

# Speed preference
export PREFER_SPEED=fast
```

### Provider-Specific Variables

```bash
# ElevenLabs API key
export ELEVENLABS_API_KEY=your_key_here
# or
export ELEVEN_LABS_API_KEY=your_key_here

# Kokoro model paths
export KOKORO_MODEL=/path/to/kokoro-v1.0.onnx
export KOKORO_VOICES=/path/to/voices-v1.0.bin
```

---

## Caching Configuration

### Voice Capability Cache

**Location**: `~/.biscuit-speaks-cache.json`

**Purpose**: Stores detected providers and available voices to avoid expensive re-enumeration.

**Manual Cache Operations**:

```rust
use biscuit_speaks::{
    read_from_cache,
    bust_host_capability_cache,
    populate_cache_for_all_providers,
    populate_cache_for_provider,
    update_provider_in_cache,
};

// Read cache
let cache = read_from_cache()?;

// Clear cache
bust_host_capability_cache()?;

// Repopulate from all providers
populate_cache_for_all_providers().await?;

// Update specific provider
update_provider_in_cache(TtsProvider::Host(HostTtsProvider::Say)).await?;
```

### Audio File Cache

**Purpose**: Content-addressed cache for generated audio to avoid re-synthesis.

**Cache Key Components**:
- Provider
- Voice ID
- Text content
- Audio format
- Speed (conditional by provider)

**Speed Inclusion by Provider**:
| Provider | Speed in Cache Key | Reason |
|----------|-------------------|--------|
| Kokoro | No | Playa handles speed at playback |
| Gtts | No | Playa handles speed at playback |
| EchoGarden | Yes | Speed baked into audio |
| ElevenLabs | Yes | Speed baked into audio |

**Atomic Writes**: Uses temp file + rename pattern to prevent corruption.

---

## ElevenLabs-Specific Configuration

### Model Selection

```rust
// Set model explicitly
let config = TtsConfig::new()
    .with_model("eleven_multilingual_v2");

// Or let voice determine optimal model
let provider = ElevenLabsProvider::new()?;
let voices = provider.list_voices().await?;
let voice = voices.iter().find(|v| v.name == "Rachel").unwrap();

let config = TtsConfig::new()
    .with_voice(&voice.identifier.clone().unwrap())
    .with_model(voice.recommended_model().unwrap_or("eleven_multilingual_v2"));
```

### Available Models

| Model ID | Description |
|----------|-------------|
| `eleven_multilingual_v2` | Best for multilingual support (default) |
| `eleven_monolingual_v1` | Optimized for English |
| `eleven_turbo_v2` | Faster generation, slightly lower quality |

### Speed Range

ElevenLabs supports speed adjustment between 0.7x and 1.2x. Values outside this range are clamped.

```rust
// Within range
SpeedLevel::Explicit(0.9)  // OK
SpeedLevel::Explicit(1.1)  // OK

// Clamped to range
SpeedLevel::Explicit(0.5)  // Becomes 0.7
SpeedLevel::Explicit(2.0)  // Becomes 1.2
```

---

## Error Handling Patterns

### Basic Error Handling

```rust
match Speak::new("Hello!").play().await {
    Ok(()) => println!("Spoke successfully"),
    Err(TtsError::NoProvidersAvailable) => {
        eprintln!("No TTS providers found on this system");
    }
    Err(TtsError::AllProvidersFailed(errors)) => {
        eprintln!("All providers failed:");
        for (provider, error) in &errors.errors {
            eprintln!("  {:?}: {}", provider, error);
        }
    }
    Err(e) => eprintln!("TTS error: {}", e),
}
```

### Fire-and-Forget (Ignore Errors)

```rust
// Logs errors but doesn't propagate them
speak_when_able("Task complete!", &TtsConfig::default()).await;
```

### Getting Metadata

```rust
match Speak::new("Hello!").play_with_result().await {
    Ok(result) => {
        println!("Provider: {:?}", result.provider);
        println!("Voice: {}", result.voice.name);
        println!("Cache hit: {}", result.cache_hit);
        if let Some(model) = result.model_used {
            println!("Model: {}", model);
        }
    }
    Err(e) => eprintln!("Error: {}", e),
}
```

---

## Feature Flags

### Default Features

```toml
[dependencies]
biscuit-speaks = { path = "../biscuit-speaks" }
```

### With Playa Integration

```toml
[dependencies]
biscuit-speaks = { path = "../biscuit-speaks", features = ["playa"] }
```

Enables:
- `play_audio_bytes()` function
- `play_audio_file()` function
- Direct audio playback without external players
