//! Core types for the biscuit-speaks TTS abstraction layer.
//!
//! This module defines the fundamental types used throughout the TTS system:
//! - Provider enums for host and cloud TTS services
//! - Configuration structs with builder pattern
//! - Audio format and failover strategy types

use std::sync::LazyLock;

use sniff_lib::programs::InstalledTtsClients;

// ============================================================================
// Volume Level
// ============================================================================

/// Volume level for TTS audio output.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VolumeLevel {
    /// Full volume (1.0)
    Loud,
    /// Reduced volume (0.5)
    Soft,
    /// Default volume (0.75)
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

impl Default for VolumeLevel {
    fn default() -> Self {
        VolumeLevel::Normal
    }
}

/// The quality of a specific voice (on a specific provider)
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VoiceQuality {
    Low,
    Moderate,
    Good,
    Excellent,
    /// avoid using this unless it REALLY is a complete unknown
    /// in most cases, however, the TTS solution or the TTS model
    /// being used should be enough to generalize this
    Unknown
}

/// The `Voice` struct defines a specific voice on a specific
/// provider.
#[derive(Debug, Clone, PartialEq)]
pub struct Voice {
    name: String,
    gender: String,
    quality: VoiceQuality,
    languages: Vec<Language>
}


/// `HostTtsCapability` defines the capability of a particular
/// provider on the Host system.
#[derive(Debug, Clone, PartialEq)]
pub struct HostTtsCapability {
    provider: TtsProvider,
    /// voices which the host already has access to for the given provider
    voices: Vec<Voice>,
    /// the voices available for the given provider which are NOT yet installed
    /// on the local host but can be
    available_voices: Vec<Voice>
}


/// The `HostTtsCapabilities` documents all of the providers (and their
/// voice capabilities) on the host. This struct is serialized to disk
/// to act as a cache for a given host's capabilities.
#[derive(Debug, Clone, PartialEq)]
pub struct HostTtsCapabilities (Vec<HostTtsCapability>);


// ============================================================================
// Language
// ============================================================================

/// Language preference for voice selection.
#[non_exhaustive]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
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
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
    // Pico2Wave,

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
            HostTtsProvider::Say => installed.say,
            HostTtsProvider::EchoGarden => installed.echogarden,
            HostTtsProvider::Sherpa => installed.sherpa_onnx,
            HostTtsProvider::ESpeak => installed.espeak || installed.espeak_ng,
            HostTtsProvider::Sapi => installed.windows_sapi,
            HostTtsProvider::Festival => installed.festival,
            HostTtsProvider::Mimic3 => installed.mimic3,
            HostTtsProvider::KokoroTts => installed.kokoro_tts,
            HostTtsProvider::Gtts => installed.gtts_cli,
            HostTtsProvider::SpdSay => false, // Not yet detected by sniff-lib
            HostTtsProvider::Piper => installed.piper,
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
    /// Gender preference for voice selection.
    pub gender: Gender,
    /// Language preference for voice selection.
    pub language: Language,
    /// Volume level for TTS output.
    pub volume: VolumeLevel,
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
        TtsProvider::Host(HostTtsProvider::Piper),
        TtsProvider::Host(HostTtsProvider::EchoGarden),
        TtsProvider::Host(HostTtsProvider::KokoroTts),
        TtsProvider::Host(HostTtsProvider::Sherpa),
        TtsProvider::Host(HostTtsProvider::Mimic3),
        TtsProvider::Host(HostTtsProvider::ESpeak),
        TtsProvider::Host(HostTtsProvider::Festival),
        TtsProvider::Host(HostTtsProvider::SpdSay),
        TtsProvider::Cloud(CloudTtsProvider::ElevenLabs),
    ]
});

/// Default TTS provider stack for macOS systems.
pub static MACOS_TTS_STACK: LazyLock<Vec<TtsProvider>> = LazyLock::new(|| {
    vec![
        TtsProvider::Host(HostTtsProvider::Say),
        TtsProvider::Host(HostTtsProvider::Piper),
        TtsProvider::Host(HostTtsProvider::EchoGarden),
        TtsProvider::Host(HostTtsProvider::KokoroTts),
        TtsProvider::Host(HostTtsProvider::Sherpa),
        TtsProvider::Host(HostTtsProvider::ESpeak),
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
        TtsProvider::Cloud(CloudTtsProvider::ElevenLabs),
    ]
});

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
            .with_volume(VolumeLevel::Soft);

        assert_eq!(config.requested_voice, Some("Samantha".into()));
        assert_eq!(config.gender, Gender::Female);
        assert_eq!(config.language, Language::Custom("en-US".into()));
        assert_eq!(config.volume, VolumeLevel::Soft);
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
}
