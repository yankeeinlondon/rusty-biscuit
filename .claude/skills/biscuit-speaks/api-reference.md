# API Reference

## Core Types

### Speak Builder

The main entry point for TTS operations.

```rust
pub struct Speak {
    text: String,
    config: TtsConfig,
}

impl Speak {
    /// Create a new Speak builder with the given text.
    pub fn new(text: impl Into<String>) -> Self;

    /// Set the voice by name.
    pub fn with_voice(self, voice: impl Into<String>) -> Self;

    /// Set the preferred gender.
    pub fn with_gender(self, gender: Gender) -> Self;

    /// Set the language.
    pub fn with_language(self, language: Language) -> Self;

    /// Set the volume level.
    pub fn with_volume(self, volume: VolumeLevel) -> Self;

    /// Set the speech speed.
    pub fn with_speed(self, speed: SpeedLevel) -> Self;

    /// Set the failover strategy.
    pub fn with_failover(self, strategy: TtsFailoverStrategy) -> Self;

    /// Speak the text asynchronously.
    pub async fn play(self) -> Result<(), TtsError>;

    /// Speak and return metadata about what was used.
    pub async fn play_with_result(self) -> Result<SpeakResult, TtsError>;

    /// Pre-generate audio for later playback.
    pub async fn prepare(self) -> Result<PreparedSpeech, TtsError>;
}
```

**Usage Examples**:

```rust
// Minimal usage
Speak::new("Hello!").play().await?;

// Full configuration
Speak::new("Hello!")
    .with_voice("Samantha")
    .with_gender(Gender::Female)
    .with_language(Language::English)
    .with_volume(VolumeLevel::Soft)
    .with_speed(SpeedLevel::Fast)
    .with_failover(TtsFailoverStrategy::PreferHost)
    .play()
    .await?;

// Get metadata
let result = Speak::new("Hello!").play_with_result().await?;
println!("Provider: {:?}", result.provider);
println!("Voice: {}", result.voice.name);
println!("Cache hit: {}", result.cache_hit);
```

---

### TtsConfig

Configuration container for TTS operations.

```rust
pub struct TtsConfig {
    pub requested_voice: Option<String>,
    pub requested_model: Option<String>,  // For ElevenLabs
    pub gender: Gender,
    pub language: Language,
    pub volume: VolumeLevel,
    pub speed: SpeedLevel,
    pub failover_strategy: TtsFailoverStrategy,
}

impl TtsConfig {
    pub fn new() -> Self;
    pub fn with_voice(self, voice: impl Into<String>) -> Self;
    pub fn with_model(self, model: impl Into<String>) -> Self;
    pub fn with_gender(self, gender: Gender) -> Self;
    pub fn with_language(self, language: Language) -> Self;
    pub fn with_volume(self, volume: VolumeLevel) -> Self;
    pub fn with_speed(self, speed: SpeedLevel) -> Self;
    pub fn with_failover(self, strategy: TtsFailoverStrategy) -> Self;
}
```

---

### Voice

Voice metadata structure.

```rust
pub struct Voice {
    pub name: String,
    pub gender: Gender,
    pub quality: VoiceQuality,
    pub languages: Vec<Language>,
    pub identifier: Option<String>,      // Provider-specific ID
    pub description: Option<String>,
    pub priority: u8,                    // Selection priority
    pub model_file: Option<String>,      // Path for Piper/Sherpa
    pub recommended_models: Vec<String>, // ElevenLabs model IDs
}

impl Voice {
    pub fn new(name: impl Into<String>) -> Self;
    pub fn with_gender(self, gender: Gender) -> Self;
    pub fn with_quality(self, quality: VoiceQuality) -> Self;
    pub fn with_language(self, language: Language) -> Self;
    pub fn with_languages(self, languages: Vec<Language>) -> Self;
    pub fn with_identifier(self, id: impl Into<String>) -> Self;
    pub fn with_description(self, desc: impl Into<String>) -> Self;
    pub fn with_priority(self, priority: u8) -> Self;
    pub fn with_model_file(self, path: impl Into<String>) -> Self;
    pub fn with_recommended_models(self, models: Vec<String>) -> Self;

    /// Get the first recommended model, if any.
    pub fn recommended_model(&self) -> Option<&str>;
}
```

---

### SpeakResult

Metadata returned after speaking.

```rust
pub struct SpeakResult {
    pub provider: TtsProvider,
    pub voice: Voice,
    pub model_used: Option<String>,        // Model ID if applicable
    pub audio_file_path: Option<PathBuf>,  // Path to generated audio
    pub audio_codec: Option<String>,       // Codec/format used
    pub cache_hit: bool,                   // Whether from cache
}
```

---

## Enums

### Gender

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Gender {
    Male,
    Female,
    Any,
}
```

### Language

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Language {
    English,
    Custom(String),  // Any language code (e.g., "fr", "de-DE")
}
```

### VolumeLevel

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VolumeLevel {
    Loud,              // 1.0
    Normal,            // 0.75
    Soft,              // 0.5
    Explicit(f32),     // Custom (clamped 0.0-1.0)
}
```

### SpeedLevel

```rust
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SpeedLevel {
    Fast,              // 1.25x
    Normal,            // 1.0x
    Slow,              // 0.75x
    Explicit(f32),     // Custom (clamped 0.25-4.0)
}
```

### VoiceQuality

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum VoiceQuality {
    Excellent,
    Good,
    Moderate,
    Low,
    Unknown,
}
```

### AudioFormat

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AudioFormat {
    Wav,
    Mp3,
    Pcm,
    Ogg,
}
```

### TtsProvider

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TtsProvider {
    Host(HostTtsProvider),
    Cloud(CloudTtsProvider),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostTtsProvider {
    Say,        // macOS
    ESpeak,     // Cross-platform
    Sapi,       // Windows
    Piper,      // Cross-platform
    EchoGarden, // Cross-platform
    Sherpa,     // Cross-platform
    Mimic3,     // Cross-platform
    Festival,   // Linux
    Gtts,       // Cross-platform (network)
    KokoroTts,  // Cross-platform
    Pico2Wave,  // Linux
    SpdSay,     // Linux
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CloudTtsProvider {
    ElevenLabs,
}
```

### TtsFailoverStrategy

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TtsFailoverStrategy {
    FirstAvailable,                  // Default: try OS default stack
    PreferHost,                      // Host providers first, then cloud
    PreferCloud,                     // Cloud providers first, then host
    SpecificProvider(TtsProvider),   // Only one provider
}
```

---

## Traits

### TtsExecutor

Required trait for all TTS providers.

```rust
pub trait TtsExecutor: Send + Sync {
    /// Speak the given text with the provided configuration.
    async fn speak(&self, text: &str, config: &TtsConfig) -> Result<(), TtsError>;

    /// Speak and return metadata about what was used.
    async fn speak_with_result(
        &self,
        text: &str,
        config: &TtsConfig
    ) -> Result<SpeakResult, TtsError>;

    /// Check if the provider is ready to speak.
    async fn is_ready(&self) -> bool { true }

    /// Return provider information string.
    fn info(&self) -> &str { "Unknown TTS Provider" }
}
```

### TtsVoiceInventory

Optional trait for voice enumeration.

```rust
pub trait TtsVoiceInventory: Send + Sync {
    /// List all available voices for this provider.
    async fn list_voices(&self) -> Result<Vec<Voice>, TtsError>;
}
```

---

## Convenience Functions

```rust
/// Simple fire-and-forget TTS.
pub async fn speak(text: &str, config: &TtsConfig) -> Result<(), TtsError>;

/// TTS returning metadata.
pub async fn speak_with_result(
    text: &str,
    config: &TtsConfig
) -> Result<SpeakResult, TtsError>;

/// Error-ignored TTS (logs errors but doesn't return them).
pub async fn speak_when_able(text: &str, config: &TtsConfig);
```

---

## Error Types

```rust
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum TtsError {
    #[error("No TTS providers available")]
    NoProvidersAvailable,

    #[error("All providers failed")]
    AllProvidersFailed(AllProvidersFailed),

    #[error("Provider {provider} failed: {message}")]
    ProviderFailed { provider: String, message: String },

    #[error("Failed to spawn process for {provider}")]
    ProcessSpawnFailed { provider: String, source: std::io::Error },

    #[error("Process failed for {provider}: {stderr}")]
    ProcessFailed { provider: String, stderr: String },

    #[error("Missing environment variable {variable} for {provider}")]
    MissingEnvironment { provider: String, variable: String },

    #[error("Missing API key for {provider}")]
    MissingApiKey { provider: String },

    #[error("Model file not found: {path}")]
    ModelFileNotFound { path: String },

    #[error("HTTP error: {0}")]
    HttpError(String),

    #[error("API error (status {status}): {message}")]
    ApiError { status: u16, message: String },

    #[error("No audio player available")]
    NoAudioPlayer,

    #[error("Playback failed for {player}: {stderr}")]
    PlaybackFailed { player: String, stderr: String },

    #[error("Voice selection failed: {message}")]
    VoiceSelectionFailed { message: String },

    #[error("No suitable voice found")]
    NoSuitableVoice,

    #[error("Voice enumeration failed for {provider}: {message}")]
    VoiceEnumerationFailed { provider: String, message: String },

    // ... additional error variants
}

#[derive(Debug)]
pub struct AllProvidersFailed {
    pub errors: Vec<(TtsProvider, TtsError)>,
}
```

---

## Cache Functions

```rust
/// Read the voice capability cache.
pub fn read_from_cache() -> Result<HostTtsCapabilities, TtsError>;

/// Clear the cache file.
pub fn bust_host_capability_cache() -> Result<(), TtsError>;

/// Populate cache for all available providers.
pub async fn populate_cache_for_all_providers() -> Result<(), TtsError>;

/// Populate cache for a specific provider.
pub async fn populate_cache_for_provider(provider: TtsProvider) -> Result<(), TtsError>;

/// Update a specific provider in the cache.
pub async fn update_provider_in_cache(provider: TtsProvider) -> Result<(), TtsError>;
```

---

## Detection Functions

```rust
/// Get list of available TTS providers on the host.
pub fn get_available_providers() -> Vec<&'static TtsProvider>;

/// Get providers ordered by failover strategy.
pub fn get_providers_for_strategy(
    strategy: &TtsFailoverStrategy
) -> Vec<&'static TtsProvider>;

/// Parse a provider name string to TtsProvider.
pub fn parse_provider_name(name: &str) -> Option<TtsProvider>;
```

---

## Playback Functions (Feature-Gated)

Requires `playa` feature.

```rust
/// Play raw audio bytes.
pub async fn play_audio_bytes(
    bytes: &[u8],
    format: AudioFormat,
    config: &TtsConfig
) -> Result<(), TtsError>;

/// Play audio from a file path.
pub async fn play_audio_file(
    path: &Path,
    format: AudioFormat,
    config: &TtsConfig
) -> Result<(), TtsError>;
```

---

## Gender Inference

```rust
/// Infer gender from a voice name using name-based heuristics.
pub fn infer_gender(name: &str) -> Gender;
```

Uses the `gender_guesser` crate:
- Male/MayBeMale -> `Gender::Male`
- Female/MayBeFemale -> `Gender::Female`
- Unknown/Andy -> `Gender::Any`
