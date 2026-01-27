//! Core types for the biscuit-speaks TTS abstraction layer.
//!
//! This module defines the fundamental types used throughout the TTS system:
//! - Provider enums for host and cloud TTS services
//! - Configuration structs with builder pattern
//! - Audio format and failover strategy types

use std::path::PathBuf;
use std::sync::LazyLock;

use serde::{Deserialize, Serialize};
use sniff_lib::programs::InstalledTtsClients;

// ============================================================================
// Volume Level
// ============================================================================

/// Volume level for TTS audio output.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum VolumeLevel {
    /// Full volume (1.0)
    Loud,
    /// Reduced volume (0.5)
    Soft,
    /// Default volume (0.75)
    #[default]
    Normal,
    /// Explicit volume value (clamped to 0.0-1.0)
    Explicit(f32),
}

impl VolumeLevel {
    /// Get the numeric volume value (0.0 to 1.0).
    pub fn value(&self) -> f32 {
        match self {
            VolumeLevel::Loud => 1.0,
            VolumeLevel::Soft => 0.5,
            VolumeLevel::Normal => 0.75,
            VolumeLevel::Explicit(v) => v.clamp(0.0, 1.0),
        }
    }
}


// ============================================================================
// Speed Level
// ============================================================================

/// Speed level for TTS speech rate.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum SpeedLevel {
    /// Fast speech (1.25x normal)
    Fast,
    /// Slow speech (0.75x normal)
    Slow,
    /// Default speech rate (1.0x)
    #[default]
    Normal,
    /// Explicit speed multiplier (clamped to 0.25-4.0)
    Explicit(f32),
}

impl SpeedLevel {
    /// Get the numeric speed multiplier.
    ///
    /// Returns a value where 1.0 is normal speed, values > 1.0 are faster,
    /// and values < 1.0 are slower.
    pub fn value(&self) -> f32 {
        match self {
            SpeedLevel::Fast => 1.25,
            SpeedLevel::Slow => 0.75,
            SpeedLevel::Normal => 1.0,
            SpeedLevel::Explicit(v) => v.clamp(0.25, 4.0),
        }
    }
}


/// The quality of a specific voice (on a specific provider).
///
/// Quality is subjective but provides a rough categorization for
/// comparing voices across different TTS providers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum VoiceQuality {
    /// Low quality (robotic, limited prosody).
    Low,
    /// Moderate quality (understandable but artificial).
    Moderate,
    /// Good quality (natural-sounding with occasional artifacts).
    Good,
    /// Excellent quality (near-human, minimal artifacts).
    Excellent,
    /// Quality is unknown or cannot be determined.
    ///
    /// Avoid using this unless it REALLY is a complete unknown.
    /// In most cases, the TTS solution or model being used should
    /// be enough to generalize quality.
    Unknown,
}

/// A specific voice on a specific TTS provider.
///
/// `Voice` represents a named voice with associated metadata such as
/// gender, quality, and supported languages. Each provider may have
/// multiple voices with different characteristics.
///
/// ## Examples
///
/// ```
/// use biscuit_speaks::types::{Voice, Gender, VoiceQuality, Language};
///
/// let voice = Voice::new("Samantha")
///     .with_gender(Gender::Female)
///     .with_quality(VoiceQuality::Excellent)
///     .with_language(Language::English);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Voice {
    /// The display name of the voice.
    pub name: String,
    /// The gender of the voice.
    pub gender: Gender,
    /// The quality rating of this voice.
    pub quality: VoiceQuality,
    /// Languages supported by this voice.
    pub languages: Vec<Language>,
    /// Provider-specific voice identifier (e.g., ElevenLabs voice ID).
    ///
    /// Some providers use identifiers that differ from the display name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identifier: Option<String>,
    /// Optional description or tagline for this voice.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Priority for voice selection (higher = preferred).
    ///
    /// Used when multiple voices match selection criteria.
    #[serde(default)]
    pub priority: u8,
    /// Path to a voice model file, if applicable.
    ///
    /// Required for providers like Piper or Sherpa that use local model files.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_file: Option<String>,
    /// Recommended TTS model IDs for this voice (provider-specific).
    ///
    /// For ElevenLabs, this contains the `high_quality_base_model_ids` from the API.
    /// The first model in the list is typically the best match for this voice.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub recommended_models: Vec<String>,
}

impl Voice {
    /// Create a new voice with the given name.
    ///
    /// Uses default values for all other fields:
    /// - `gender`: `Gender::Any`
    /// - `quality`: `VoiceQuality::Unknown`
    /// - `languages`: empty
    /// - `identifier`: `None`
    /// - `description`: `None`
    /// - `priority`: 0
    /// - `model_file`: `None`
    /// - `recommended_models`: empty
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            gender: Gender::Any,
            quality: VoiceQuality::Unknown,
            languages: Vec::new(),
            identifier: None,
            description: None,
            priority: 0,
            model_file: None,
            recommended_models: Vec::new(),
        }
    }

    /// Set the gender of this voice.
    #[must_use]
    pub fn with_gender(mut self, gender: Gender) -> Self {
        self.gender = gender;
        self
    }

    /// Set the quality rating of this voice.
    #[must_use]
    pub fn with_quality(mut self, quality: VoiceQuality) -> Self {
        self.quality = quality;
        self
    }

    /// Add a supported language to this voice.
    #[must_use]
    pub fn with_language(mut self, language: Language) -> Self {
        self.languages.push(language);
        self
    }

    /// Set the supported languages for this voice.
    #[must_use]
    pub fn with_languages(mut self, languages: Vec<Language>) -> Self {
        self.languages = languages;
        self
    }

    /// Set the provider-specific identifier.
    #[must_use]
    pub fn with_identifier(mut self, identifier: impl Into<String>) -> Self {
        self.identifier = Some(identifier.into());
        self
    }

    /// Set the voice description or tagline.
    #[must_use]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Set the selection priority.
    #[must_use]
    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    /// Set the model file path.
    #[must_use]
    pub fn with_model_file(mut self, path: impl Into<String>) -> Self {
        self.model_file = Some(path.into());
        self
    }

    /// Set the recommended TTS models for this voice.
    #[must_use]
    pub fn with_recommended_models(mut self, models: Vec<String>) -> Self {
        self.recommended_models = models;
        self
    }

    /// Get the first recommended model for this voice, if any.
    pub fn recommended_model(&self) -> Option<&str> {
        self.recommended_models.first().map(|s| s.as_str())
    }
}


/// Capability information for a specific TTS provider on the host system.
///
/// `HostTtsCapability` tracks both the currently installed/available voices
/// and any additional voices that could be installed for a provider.
///
/// ## Examples
///
/// ```
/// use biscuit_speaks::types::{HostTtsCapability, TtsProvider, HostTtsProvider, Voice, Gender, VoiceQuality, Language};
///
/// let capability = HostTtsCapability::new(TtsProvider::Host(HostTtsProvider::Say))
///     .with_voice(Voice::new("Samantha")
///         .with_gender(Gender::Female)
///         .with_quality(VoiceQuality::Good));
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HostTtsCapability {
    /// The TTS provider this capability describes.
    pub provider: TtsProvider,
    /// Voices currently available (installed) on the host for this provider.
    pub voices: Vec<Voice>,
    /// Voices that could be installed but are not yet available locally.
    ///
    /// This is useful for providers like macOS `say` where additional
    /// voices can be downloaded from System Settings.
    pub available_voices: Vec<Voice>,
}

impl HostTtsCapability {
    /// Create a new capability record for a provider.
    pub fn new(provider: TtsProvider) -> Self {
        Self {
            provider,
            voices: Vec::new(),
            available_voices: Vec::new(),
        }
    }

    /// Add an installed voice to this provider's capability.
    #[must_use]
    pub fn with_voice(mut self, voice: Voice) -> Self {
        self.voices.push(voice);
        self
    }

    /// Add multiple installed voices.
    #[must_use]
    pub fn with_voices(mut self, voices: Vec<Voice>) -> Self {
        self.voices.extend(voices);
        self
    }

    /// Add an available (not yet installed) voice.
    #[must_use]
    pub fn with_available_voice(mut self, voice: Voice) -> Self {
        self.available_voices.push(voice);
        self
    }

    /// Add multiple available voices.
    #[must_use]
    pub fn with_available_voices(mut self, voices: Vec<Voice>) -> Self {
        self.available_voices.extend(voices);
        self
    }
}

/// Complete TTS capabilities for a host system.
///
/// `HostTtsCapabilities` aggregates capability information from all
/// TTS providers discovered on the system. This struct is designed
/// to be serialized to disk as a cache for quick lookup.
///
/// ## Caching
///
/// The capabilities can be expensive to enumerate (especially for
/// cloud providers), so this struct supports JSON serialization for
/// persistent caching.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct HostTtsCapabilities {
    /// Capability information for each discovered provider.
    pub providers: Vec<HostTtsCapability>,
    /// Timestamp when this cache was last updated (Unix epoch seconds).
    #[serde(default)]
    pub last_updated: u64,
}

impl HostTtsCapabilities {
    /// Create an empty capabilities struct.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a provider's capability information.
    #[must_use]
    pub fn with_provider(mut self, capability: HostTtsCapability) -> Self {
        self.providers.push(capability);
        self
    }

    /// Set the last updated timestamp.
    #[must_use]
    pub fn with_timestamp(mut self, timestamp: u64) -> Self {
        self.last_updated = timestamp;
        self
    }

    /// Get all installed voices across all providers.
    pub fn all_voices(&self) -> impl Iterator<Item = &Voice> {
        self.providers.iter().flat_map(|p| p.voices.iter())
    }

    /// Find a provider's capability by provider type.
    pub fn get_provider(&self, provider: &TtsProvider) -> Option<&HostTtsCapability> {
        self.providers.iter().find(|p| &p.provider == provider)
    }
}


// ============================================================================
// Language
// ============================================================================

/// Language preference for voice selection.
#[non_exhaustive]
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    /// English language (any variant: en-US, en-GB, etc.).
    #[default]
    English,
    /// Custom language code (BCP-47 format recommended, e.g., "fr-FR", "es-MX").
    Custom(String),
}

impl Language {
    /// Returns the language code prefix for voice matching.
    ///
    /// ## Examples
    ///
    /// ```
    /// use biscuit_speaks::types::Language;
    ///
    /// assert_eq!(Language::English.code_prefix(), "en");
    /// assert_eq!(Language::Custom("fr-FR".into()).code_prefix(), "fr-FR");
    /// ```
    pub fn code_prefix(&self) -> &str {
        match self {
            Language::English => "en",
            Language::Custom(code) => code,
        }
    }
}

// ============================================================================
// Gender
// ============================================================================

/// Gender preference for voice selection.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Gender {
    /// Prefer a male voice.
    Male,
    /// Prefer a female voice.
    Female,
    /// No gender preference (use any available voice).
    #[default]
    Any,
}

// ============================================================================
// Audio Format
// ============================================================================

/// Audio format for TTS output.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AudioFormat {
    /// WAV format (uncompressed, widely supported)
    #[default]
    Wav,
    /// MP3 format (compressed, used by ElevenLabs)
    Mp3,
    /// Raw PCM audio data
    Pcm,
    /// Ogg Vorbis format
    Ogg,
}

impl AudioFormat {
    /// Returns the file extension for this audio format.
    pub fn extension(&self) -> &'static str {
        match self {
            AudioFormat::Wav => "wav",
            AudioFormat::Mp3 => "mp3",
            AudioFormat::Pcm => "raw",
            AudioFormat::Ogg => "ogg",
        }
    }

    /// Returns the MIME type for this audio format.
    pub fn mime_type(&self) -> &'static str {
        match self {
            AudioFormat::Wav => "audio/wav",
            AudioFormat::Mp3 => "audio/mpeg",
            AudioFormat::Pcm => "audio/pcm",
            AudioFormat::Ogg => "audio/ogg",
        }
    }
}

// ============================================================================
// Host TTS Provider
// ============================================================================

/// TTS providers that may reside on a host system.
///
/// These are CLI-based TTS solutions that can be invoked via subprocess.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HostTtsProvider {
    /// macOS built-in speech synthesis (`say` command).
    /// Available on all macOS systems.
    Say,

    /// EchoGarden - high quality speech processing engine.
    /// [Website](https://echogarden.io/)
    EchoGarden,

    /// Sherpa-ONNX offline TTS.
    /// Requires `SHERPA_MODEL` and `SHERPA_TOKENS` environment variables.
    /// [Website](https://k2-fsa.github.io/sherpa/onnx/)
    Sherpa,

    /// eSpeak/eSpeak-NG - open source speech synthesizer.
    /// Common on Linux, available cross-platform.
    /// [Website](https://github.com/espeak-ng/espeak-ng)
    ESpeak,

    /// Windows Speech API (SAPI).
    /// Available on all Windows systems via PowerShell.
    Sapi,

    /// Festival - general multi-lingual speech synthesis.
    /// [Website](http://www.cstr.ed.ac.uk/projects/festival/)
    Festival,

    /// SVOX Pico TTS (`pico2wave` command).
    /// Lightweight TTS for embedded systems.
    Pico2Wave,

    /// Mimic3 - Mycroft's neural TTS engine.
    /// Supports SSML input.
    /// [Website](https://github.com/MycroftAI/mycroft-mimic3-tts)
    Mimic3,

    /// Kokoro TTS - high quality neural TTS.
    /// Requires model files.
    /// [Website](https://github.com/nazdridoy/kokoro-tts)
    KokoroTts,

    /// Google Text-to-Speech CLI (`gtts-cli`).
    /// Requires network connectivity.
    /// [Website](https://github.com/pndurette/gTTS)
    Gtts,

    /// Speech Dispatcher client (`spd-say`).
    /// Routes to system TTS on Linux desktops.
    SpdSay,

    /// Piper - fast local neural TTS using ONNX.
    /// [Website](https://github.com/rhasspy/piper)
    Piper,
}

impl HostTtsProvider {
    /// Check if this provider is available on the host system.
    pub fn is_available(&self, installed: &InstalledTtsClients) -> bool {
        match self {
            HostTtsProvider::Say => installed.say(),
            HostTtsProvider::EchoGarden => installed.echogarden(),
            HostTtsProvider::Sherpa => installed.sherpa_onnx(),
            HostTtsProvider::ESpeak => installed.espeak() || installed.espeak_ng(),
            HostTtsProvider::Sapi => installed.windows_sapi(),
            HostTtsProvider::Festival => installed.festival(),
            HostTtsProvider::Mimic3 => installed.mimic3(),
            HostTtsProvider::KokoroTts => installed.kokoro_tts(),
            HostTtsProvider::Gtts => installed.gtts_cli(),
            HostTtsProvider::SpdSay => false, // Not yet detected by sniff-lib
            HostTtsProvider::Piper => installed.piper(),
            HostTtsProvider::Pico2Wave => installed.pico2wave(),
        }
    }

    /// Returns the binary name for this provider.
    pub fn binary_name(&self) -> &'static str {
        match self {
            HostTtsProvider::Say => "say",
            HostTtsProvider::EchoGarden => "echogarden",
            HostTtsProvider::Sherpa => "sherpa-onnx-offline-tts",
            HostTtsProvider::ESpeak => "espeak-ng",
            HostTtsProvider::Sapi => "powershell",
            HostTtsProvider::Festival => "festival",
            HostTtsProvider::Mimic3 => "mimic3",
            HostTtsProvider::KokoroTts => "kokoro-tts",
            HostTtsProvider::Gtts => "gtts-cli",
            HostTtsProvider::SpdSay => "spd-say",
            HostTtsProvider::Piper => "piper",
            HostTtsProvider::Pico2Wave => "pico2wave",
        }
    }

    /// A more thorough check then `is_available()`, to pass the TTS provider must
    /// not only have the required executable program available on the host but
    /// the underlying `TtsExecutor`'s `is_ready()` function must evaluate to to
    /// true.
    pub fn is_ready(&self) -> bool {
        todo!()
    }
}

// ============================================================================
// Cloud TTS Provider
// ============================================================================

/// Cloud-based TTS providers requiring API keys.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CloudTtsProvider {
    /// ElevenLabs - high quality AI voice synthesis.
    /// Requires `ELEVENLABS_API_KEY` or `ELEVEN_LABS_API_KEY`.
    /// [Website](https://elevenlabs.io/)
    ElevenLabs,
}

impl CloudTtsProvider {
    /// Check if this provider is available (has API key configured).
    pub fn is_available(&self) -> bool {
        match self {
            CloudTtsProvider::ElevenLabs => {
                std::env::var("ELEVENLABS_API_KEY").is_ok()
                    || std::env::var("ELEVEN_LABS_API_KEY").is_ok()
            }
        }
    }

    /// Returns the environment variable name(s) for the API key.
    pub fn api_key_env_vars(&self) -> &'static [&'static str] {
        match self {
            CloudTtsProvider::ElevenLabs => &["ELEVENLABS_API_KEY", "ELEVEN_LABS_API_KEY"],
        }
    }

    /// A more thorough check then `is_available()`, to pass the TTS provider must
    /// not only have the required executable program available on the host but
    /// the underlying `TtsExecutor`'s `is_ready()` function must evaluate to to
    /// true.
    pub fn is_ready(&self) -> bool {
        todo!()
    }
}

// ============================================================================
// Unified TTS Provider
// ============================================================================

/// Unified TTS provider enum combining host and cloud providers.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TtsProvider {
    /// Host-based TTS provider (CLI subprocess).
    Host(HostTtsProvider),
    /// Cloud-based TTS provider (HTTP API).
    Cloud(CloudTtsProvider),
}

impl TtsProvider {
    /// Check if this provider is available.
    pub fn is_available(&self, installed: &InstalledTtsClients) -> bool {
        match self {
            TtsProvider::Host(h) => h.is_available(installed),
            TtsProvider::Cloud(c) => c.is_available(),
        }
    }
}

impl From<HostTtsProvider> for TtsProvider {
    fn from(p: HostTtsProvider) -> Self {
        TtsProvider::Host(p)
    }
}

impl From<CloudTtsProvider> for TtsProvider {
    fn from(p: CloudTtsProvider) -> Self {
        TtsProvider::Cloud(p)
    }
}

// ============================================================================
// Failover Strategy
// ============================================================================

/// Strategy for handling TTS provider failures.
#[non_exhaustive]
#[derive(Debug, Clone, Default)]
pub enum TtsFailoverStrategy {
    /// Try providers in priority order until one succeeds.
    #[default]
    FirstAvailable,
    /// Prefer host providers, fall back to cloud.
    PreferHost,
    /// Prefer cloud providers, fall back to host.
    PreferCloud,
    /// Use a specific provider only, no failover.
    SpecificProvider(TtsProvider),
}

// ============================================================================
// TTS Configuration
// ============================================================================

/// Configuration for TTS operations.
///
/// Use the builder pattern to construct:
///
/// ```
/// use biscuit_speaks::types::{TtsConfig, Gender, Language};
///
/// let config = TtsConfig::new()
///     .with_voice("Samantha")
///     .with_gender(Gender::Female)
///     .with_language(Language::English);
/// ```
#[derive(Debug, Clone, Default)]
pub struct TtsConfig {
    /// Requested voice name (provider-specific).
    pub requested_voice: Option<String>,
    /// Requested TTS model (provider-specific, e.g., ElevenLabs model ID).
    ///
    /// If set, this model will be used instead of the provider's default.
    /// For ElevenLabs, this should be a model ID like "eleven_multilingual_v2".
    pub requested_model: Option<String>,
    /// Gender preference for voice selection.
    pub gender: Gender,
    /// Language preference for voice selection.
    pub language: Language,
    /// Volume level for TTS output.
    pub volume: VolumeLevel,
    /// Speed level for TTS speech rate.
    pub speed: SpeedLevel,
    /// Failover strategy when providers fail.
    pub failover_strategy: TtsFailoverStrategy,
}

impl TtsConfig {
    /// Create a new TtsConfig with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the requested voice name.
    #[must_use]
    pub fn with_voice(mut self, voice: impl Into<String>) -> Self {
        self.requested_voice = Some(voice.into());
        self
    }

    /// Set the requested TTS model.
    ///
    /// For ElevenLabs, this should be a model ID like "eleven_multilingual_v2".
    /// Use `Voice::recommended_model()` to get the best model for a specific voice.
    #[must_use]
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.requested_model = Some(model.into());
        self
    }

    /// Set the gender preference.
    #[must_use]
    pub fn with_gender(mut self, gender: Gender) -> Self {
        self.gender = gender;
        self
    }

    /// Set the language preference.
    #[must_use]
    pub fn with_language(mut self, language: Language) -> Self {
        self.language = language;
        self
    }

    /// Set the volume level.
    #[must_use]
    pub fn with_volume(mut self, volume: VolumeLevel) -> Self {
        self.volume = volume;
        self
    }

    /// Set the speed level.
    #[must_use]
    pub fn with_speed(mut self, speed: SpeedLevel) -> Self {
        self.speed = speed;
        self
    }

    /// Set the failover strategy.
    #[must_use]
    pub fn with_failover(mut self, strategy: TtsFailoverStrategy) -> Self {
        self.failover_strategy = strategy;
        self
    }
}

// ============================================================================
// OS-Specific Provider Stacks
// ============================================================================

/// Default TTS provider stack for Linux systems.
pub static LINUX_TTS_STACK: LazyLock<Vec<TtsProvider>> = LazyLock::new(|| {
    vec![
        TtsProvider::Host(HostTtsProvider::KokoroTts),
        TtsProvider::Host(HostTtsProvider::EchoGarden),
        TtsProvider::Host(HostTtsProvider::Sherpa),
        TtsProvider::Host(HostTtsProvider::Piper),
        TtsProvider::Host(HostTtsProvider::Mimic3),
        TtsProvider::Host(HostTtsProvider::ESpeak),
        TtsProvider::Host(HostTtsProvider::Festival),
        TtsProvider::Host(HostTtsProvider::Gtts),
        TtsProvider::Host(HostTtsProvider::SpdSay),
        TtsProvider::Cloud(CloudTtsProvider::ElevenLabs),
    ]
});

/// Default TTS provider stack for macOS systems.
pub static MACOS_TTS_STACK: LazyLock<Vec<TtsProvider>> = LazyLock::new(|| {
    vec![
        TtsProvider::Host(HostTtsProvider::KokoroTts),
        TtsProvider::Host(HostTtsProvider::EchoGarden),
        TtsProvider::Host(HostTtsProvider::Say),
        TtsProvider::Host(HostTtsProvider::Piper),
        TtsProvider::Host(HostTtsProvider::Sherpa),
        TtsProvider::Host(HostTtsProvider::ESpeak),
        TtsProvider::Host(HostTtsProvider::Gtts),
        TtsProvider::Cloud(CloudTtsProvider::ElevenLabs),
    ]
});

/// Default TTS provider stack for Windows systems.
pub static WINDOWS_TTS_STACK: LazyLock<Vec<TtsProvider>> = LazyLock::new(|| {
    vec![
        TtsProvider::Host(HostTtsProvider::Sapi),
        TtsProvider::Host(HostTtsProvider::Piper),
        TtsProvider::Host(HostTtsProvider::EchoGarden),
        TtsProvider::Host(HostTtsProvider::KokoroTts),
        TtsProvider::Host(HostTtsProvider::Sherpa),
        TtsProvider::Host(HostTtsProvider::ESpeak),
        TtsProvider::Host(HostTtsProvider::Gtts),
        TtsProvider::Cloud(CloudTtsProvider::ElevenLabs),
    ]
});

// ============================================================================
// SpeakResult
// ============================================================================

/// Result of a TTS operation containing metadata about what was actually used.
///
/// This struct is returned by `speak_with_result()` and contains information
/// about the provider and voice that were actually used for synthesis.
///
/// ## Examples
///
/// ```
/// use biscuit_speaks::types::{SpeakResult, TtsProvider, HostTtsProvider, Voice, Gender, VoiceQuality};
///
/// let result = SpeakResult::new(
///     TtsProvider::Host(HostTtsProvider::Say),
///     Voice::new("Samantha")
///         .with_gender(Gender::Female)
///         .with_quality(VoiceQuality::Good),
/// );
///
/// assert!(matches!(result.provider, TtsProvider::Host(HostTtsProvider::Say)));
/// assert_eq!(result.voice.name, "Samantha");
/// ```
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpeakResult {
    /// The TTS provider that was used.
    pub provider: TtsProvider,
    /// The voice that was used for synthesis.
    pub voice: Voice,
    /// The TTS model that was used (provider-specific).
    ///
    /// For ElevenLabs, this is the model ID like "eleven_multilingual_v2".
    /// For other providers, this may be None.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_used: Option<String>,
    /// Path to the audio file that was used (cached or newly generated).
    ///
    /// For file-based providers, this contains the path to the audio file
    /// in the system temp directory.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_file_path: Option<PathBuf>,
    /// The audio codec/format used (e.g., "wav", "mp3").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio_codec: Option<String>,
    /// Whether the audio was served from cache.
    #[serde(default)]
    pub cache_hit: bool,
}

impl SpeakResult {
    /// Create a new SpeakResult.
    pub fn new(provider: TtsProvider, voice: Voice) -> Self {
        Self {
            provider,
            voice,
            model_used: None,
            audio_file_path: None,
            audio_codec: None,
            cache_hit: false,
        }
    }

    /// Create a new SpeakResult with a model.
    pub fn with_model(provider: TtsProvider, voice: Voice, model: impl Into<String>) -> Self {
        Self {
            provider,
            voice,
            model_used: Some(model.into()),
            audio_file_path: None,
            audio_codec: None,
            cache_hit: false,
        }
    }

    /// Set the audio file path.
    #[must_use]
    pub fn with_audio_file(mut self, path: PathBuf) -> Self {
        self.audio_file_path = Some(path);
        self
    }

    /// Set the audio codec.
    #[must_use]
    pub fn with_codec(mut self, codec: impl Into<String>) -> Self {
        self.audio_codec = Some(codec.into());
        self
    }

    /// Set whether this was a cache hit.
    #[must_use]
    pub fn with_cache_hit(mut self, hit: bool) -> Self {
        self.cache_hit = hit;
        self
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_volume_level_values() {
        assert_eq!(VolumeLevel::Loud.value(), 1.0);
        assert_eq!(VolumeLevel::Soft.value(), 0.5);
        assert_eq!(VolumeLevel::Normal.value(), 0.75);
        assert_eq!(VolumeLevel::Explicit(0.3).value(), 0.3);
    }

    #[test]
    fn test_volume_level_clamping() {
        assert_eq!(VolumeLevel::Explicit(1.5).value(), 1.0);
        assert_eq!(VolumeLevel::Explicit(-0.5).value(), 0.0);
    }

    #[test]
    fn test_speed_level_values() {
        assert_eq!(SpeedLevel::Fast.value(), 1.25);
        assert_eq!(SpeedLevel::Slow.value(), 0.75);
        assert_eq!(SpeedLevel::Normal.value(), 1.0);
        assert_eq!(SpeedLevel::Explicit(1.5).value(), 1.5);
    }

    #[test]
    fn test_speed_level_clamping() {
        assert_eq!(SpeedLevel::Explicit(5.0).value(), 4.0);
        assert_eq!(SpeedLevel::Explicit(0.1).value(), 0.25);
    }

    #[test]
    fn test_language_code_prefix() {
        assert_eq!(Language::English.code_prefix(), "en");
        assert_eq!(Language::Custom("fr-FR".into()).code_prefix(), "fr-FR");
    }

    #[test]
    fn test_gender_default() {
        assert_eq!(Gender::default(), Gender::Any);
    }

    #[test]
    fn test_audio_format_extension() {
        assert_eq!(AudioFormat::Wav.extension(), "wav");
        assert_eq!(AudioFormat::Mp3.extension(), "mp3");
        assert_eq!(AudioFormat::Pcm.extension(), "raw");
        assert_eq!(AudioFormat::Ogg.extension(), "ogg");
    }

    #[test]
    fn test_audio_format_mime_type() {
        assert_eq!(AudioFormat::Wav.mime_type(), "audio/wav");
        assert_eq!(AudioFormat::Mp3.mime_type(), "audio/mpeg");
    }

    #[test]
    fn test_tts_config_builder() {
        let config = TtsConfig::new()
            .with_voice("Samantha")
            .with_gender(Gender::Female)
            .with_language(Language::Custom("en-US".into()))
            .with_volume(VolumeLevel::Soft)
            .with_speed(SpeedLevel::Fast);

        assert_eq!(config.requested_voice, Some("Samantha".into()));
        assert_eq!(config.gender, Gender::Female);
        assert_eq!(config.language, Language::Custom("en-US".into()));
        assert_eq!(config.volume, VolumeLevel::Soft);
        assert_eq!(config.speed, SpeedLevel::Fast);
    }

    #[test]
    fn test_tts_provider_from_host() {
        let provider: TtsProvider = HostTtsProvider::Say.into();
        assert!(matches!(provider, TtsProvider::Host(HostTtsProvider::Say)));
    }

    #[test]
    fn test_tts_provider_from_cloud() {
        let provider: TtsProvider = CloudTtsProvider::ElevenLabs.into();
        assert!(matches!(
            provider,
            TtsProvider::Cloud(CloudTtsProvider::ElevenLabs)
        ));
    }

    #[test]
    fn test_host_provider_binary_name() {
        assert_eq!(HostTtsProvider::Say.binary_name(), "say");
        assert_eq!(HostTtsProvider::ESpeak.binary_name(), "espeak-ng");
        assert_eq!(HostTtsProvider::Sapi.binary_name(), "powershell");
    }

    // ========================================================================
    // Voice tests
    // ========================================================================

    #[test]
    fn test_voice_new() {
        let voice = Voice::new("TestVoice");
        assert_eq!(voice.name, "TestVoice");
        assert_eq!(voice.gender, Gender::Any);
        assert_eq!(voice.quality, VoiceQuality::Unknown);
        assert!(voice.languages.is_empty());
        assert!(voice.identifier.is_none());
        assert_eq!(voice.priority, 0);
        assert!(voice.model_file.is_none());
    }

    #[test]
    fn test_voice_builder() {
        let voice = Voice::new("Samantha")
            .with_gender(Gender::Female)
            .with_quality(VoiceQuality::Excellent)
            .with_language(Language::English)
            .with_identifier("voice_123")
            .with_priority(10)
            .with_model_file("/path/to/model.onnx");

        assert_eq!(voice.name, "Samantha");
        assert_eq!(voice.gender, Gender::Female);
        assert_eq!(voice.quality, VoiceQuality::Excellent);
        assert_eq!(voice.languages, vec![Language::English]);
        assert_eq!(voice.identifier, Some("voice_123".into()));
        assert_eq!(voice.priority, 10);
        assert_eq!(voice.model_file, Some("/path/to/model.onnx".into()));
    }

    #[test]
    fn test_voice_with_languages() {
        let voice = Voice::new("MultiLang")
            .with_languages(vec![Language::English, Language::Custom("fr-FR".into())]);

        assert_eq!(voice.languages.len(), 2);
    }

    #[test]
    fn test_voice_serialization() {
        let voice = Voice::new("TestVoice")
            .with_gender(Gender::Female)
            .with_quality(VoiceQuality::Good)
            .with_language(Language::English);

        let json = serde_json::to_string(&voice).unwrap();
        assert!(json.contains("\"name\":\"TestVoice\""));
        assert!(json.contains("\"gender\":\"female\""));
        assert!(json.contains("\"quality\":\"good\""));

        // Roundtrip test
        let deserialized: Voice = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, voice);
    }

    #[test]
    fn test_voice_serialization_skips_none() {
        let voice = Voice::new("Simple");
        let json = serde_json::to_string(&voice).unwrap();

        // Optional fields with None should be skipped
        assert!(!json.contains("identifier"));
        assert!(!json.contains("model_file"));
    }

    // ========================================================================
    // VoiceQuality tests
    // ========================================================================

    #[test]
    fn test_voice_quality_serialization() {
        assert_eq!(serde_json::to_string(&VoiceQuality::Low).unwrap(), "\"low\"");
        assert_eq!(
            serde_json::to_string(&VoiceQuality::Excellent).unwrap(),
            "\"excellent\""
        );
        assert_eq!(
            serde_json::to_string(&VoiceQuality::Unknown).unwrap(),
            "\"unknown\""
        );
    }

    // ========================================================================
    // Gender tests
    // ========================================================================

    #[test]
    fn test_gender_serialization() {
        assert_eq!(serde_json::to_string(&Gender::Male).unwrap(), "\"male\"");
        assert_eq!(serde_json::to_string(&Gender::Female).unwrap(), "\"female\"");
        assert_eq!(serde_json::to_string(&Gender::Any).unwrap(), "\"any\"");
    }

    // ========================================================================
    // Language tests
    // ========================================================================

    #[test]
    fn test_language_serialization() {
        assert_eq!(
            serde_json::to_string(&Language::English).unwrap(),
            "\"english\""
        );
        let custom = Language::Custom("de-DE".into());
        let json = serde_json::to_string(&custom).unwrap();
        assert!(json.contains("de-DE"));
    }

    // ========================================================================
    // HostTtsProvider tests
    // ========================================================================

    #[test]
    fn test_host_provider_serialization() {
        assert_eq!(
            serde_json::to_string(&HostTtsProvider::Say).unwrap(),
            "\"say\""
        );
        assert_eq!(
            serde_json::to_string(&HostTtsProvider::ESpeak).unwrap(),
            "\"e_speak\""
        );
        assert_eq!(
            serde_json::to_string(&HostTtsProvider::Piper).unwrap(),
            "\"piper\""
        );
    }

    #[test]
    fn test_cloud_provider_serialization() {
        assert_eq!(
            serde_json::to_string(&CloudTtsProvider::ElevenLabs).unwrap(),
            "\"eleven_labs\""
        );
    }

    // ========================================================================
    // TtsProvider tests
    // ========================================================================

    #[test]
    fn test_tts_provider_serialization() {
        let host_provider = TtsProvider::Host(HostTtsProvider::Say);
        let json = serde_json::to_string(&host_provider).unwrap();
        assert!(json.contains("host"));
        assert!(json.contains("say"));

        let cloud_provider = TtsProvider::Cloud(CloudTtsProvider::ElevenLabs);
        let json = serde_json::to_string(&cloud_provider).unwrap();
        assert!(json.contains("cloud"));
        assert!(json.contains("eleven_labs"));
    }

    // ========================================================================
    // HostTtsCapability tests
    // ========================================================================

    #[test]
    fn test_host_tts_capability_new() {
        let cap = HostTtsCapability::new(TtsProvider::Host(HostTtsProvider::Say));
        assert!(matches!(
            cap.provider,
            TtsProvider::Host(HostTtsProvider::Say)
        ));
        assert!(cap.voices.is_empty());
        assert!(cap.available_voices.is_empty());
    }

    #[test]
    fn test_host_tts_capability_builder() {
        let cap = HostTtsCapability::new(TtsProvider::Host(HostTtsProvider::Say))
            .with_voice(Voice::new("Samantha"))
            .with_available_voice(Voice::new("Alex"));

        assert_eq!(cap.voices.len(), 1);
        assert_eq!(cap.voices[0].name, "Samantha");
        assert_eq!(cap.available_voices.len(), 1);
        assert_eq!(cap.available_voices[0].name, "Alex");
    }

    #[test]
    fn test_host_tts_capability_with_voices() {
        let cap = HostTtsCapability::new(TtsProvider::Host(HostTtsProvider::Say))
            .with_voices(vec![Voice::new("V1"), Voice::new("V2")])
            .with_available_voices(vec![Voice::new("V3")]);

        assert_eq!(cap.voices.len(), 2);
        assert_eq!(cap.available_voices.len(), 1);
    }

    #[test]
    fn test_host_tts_capability_serialization() {
        let cap = HostTtsCapability::new(TtsProvider::Host(HostTtsProvider::Say))
            .with_voice(Voice::new("Samantha").with_gender(Gender::Female));

        let json = serde_json::to_string(&cap).unwrap();
        let deserialized: HostTtsCapability = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, cap);
    }

    // ========================================================================
    // HostTtsCapabilities tests
    // ========================================================================

    #[test]
    fn test_host_tts_capabilities_new() {
        let caps = HostTtsCapabilities::new();
        assert!(caps.providers.is_empty());
        assert_eq!(caps.last_updated, 0);
    }

    #[test]
    fn test_host_tts_capabilities_builder() {
        let caps = HostTtsCapabilities::new()
            .with_provider(
                HostTtsCapability::new(TtsProvider::Host(HostTtsProvider::Say))
                    .with_voice(Voice::new("Samantha")),
            )
            .with_timestamp(1234567890);

        assert_eq!(caps.providers.len(), 1);
        assert_eq!(caps.last_updated, 1234567890);
    }

    #[test]
    fn test_host_tts_capabilities_all_voices() {
        let caps = HostTtsCapabilities::new()
            .with_provider(
                HostTtsCapability::new(TtsProvider::Host(HostTtsProvider::Say))
                    .with_voices(vec![Voice::new("V1"), Voice::new("V2")]),
            )
            .with_provider(
                HostTtsCapability::new(TtsProvider::Host(HostTtsProvider::ESpeak))
                    .with_voice(Voice::new("V3")),
            );

        let all_voices: Vec<_> = caps.all_voices().collect();
        assert_eq!(all_voices.len(), 3);
    }

    #[test]
    fn test_host_tts_capabilities_get_provider() {
        let caps = HostTtsCapabilities::new()
            .with_provider(HostTtsCapability::new(TtsProvider::Host(HostTtsProvider::Say)));

        assert!(caps
            .get_provider(&TtsProvider::Host(HostTtsProvider::Say))
            .is_some());
        assert!(caps
            .get_provider(&TtsProvider::Host(HostTtsProvider::ESpeak))
            .is_none());
    }

    #[test]
    fn test_host_tts_capabilities_serialization() {
        let caps = HostTtsCapabilities::new()
            .with_provider(
                HostTtsCapability::new(TtsProvider::Host(HostTtsProvider::Say))
                    .with_voice(Voice::new("Test")),
            )
            .with_timestamp(1234567890);

        let json = serde_json::to_string_pretty(&caps).unwrap();
        let deserialized: HostTtsCapabilities = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, caps);
    }
}
