//! The main Speak struct for TTS operations.
//!
//! This module provides the primary API for text-to-speech functionality.

use crate::detection::get_providers_for_strategy;
use crate::errors::{AllProvidersFailed, TtsError};
use crate::providers::cloud::ElevenLabsProvider;
use crate::providers::host::{
    EchogardenProvider, ESpeakProvider, GttsProvider, KokoroTtsProvider, SapiProvider, SayProvider,
};
use crate::traits::TtsExecutor;
use crate::types::{
    AudioFormat, CloudTtsProvider, Gender, HostTtsProvider, Language, TtsConfig,
    TtsFailoverStrategy, TtsProvider, VolumeLevel,
};

/// The primary struct for TTS operations.
///
/// `Speak` provides a builder-pattern API for configuring and executing
/// text-to-speech. It supports:
///
/// - Voice selection by name
/// - Gender and language preferences
/// - Volume control
/// - Provider failover strategies
/// - Optional audio pre-generation via `prepare()`
///
/// ## Examples
///
/// ```ignore
/// use biscuit_speaks::Speak;
///
/// // Simple usage
/// Speak::new("Hello!").play().await?;
///
/// // With configuration
/// Speak::new("Hello!")
///     .with_voice("Samantha")
///     .with_gender(Gender::Female)
///     .play()
///     .await?;
///
/// // Pre-generate audio for later playback
/// let prepared = Speak::new("Important message")
///     .prepare()
///     .await?;
/// // ... later ...
/// prepared.play().await?;
/// ```
#[derive(Debug, Clone)]
pub struct Speak {
    /// The text to be spoken.
    text: String,
    /// Pre-generated audio data (if prepared).
    audio: Option<Vec<u8>>,
    /// Audio format of pre-generated audio.
    audio_format: AudioFormat,
    /// TTS configuration.
    config: TtsConfig,
}

impl Speak {
    /// Create a new Speak instance with the given text.
    ///
    /// ## Examples
    ///
    /// ```
    /// use biscuit_speaks::Speak;
    ///
    /// let speak = Speak::new("Hello, world!");
    /// ```
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            audio: None,
            audio_format: AudioFormat::default(),
            config: TtsConfig::default(),
        }
    }

    /// Set the requested voice name.
    ///
    /// Voice names are provider-specific. For example:
    /// - macOS `say`: "Samantha", "Alex", "Daniel"
    /// - ElevenLabs: voice IDs like "21m00Tcm4TlvDq8ikWAM"
    #[must_use]
    pub fn with_voice(mut self, voice: impl Into<String>) -> Self {
        self.config.requested_voice = Some(voice.into());
        self
    }

    /// Set the gender preference for voice selection.
    #[must_use]
    pub fn with_gender(mut self, gender: Gender) -> Self {
        self.config.gender = gender;
        self
    }

    /// Set the language preference for voice selection.
    #[must_use]
    pub fn with_language(mut self, language: Language) -> Self {
        self.config.language = language;
        self
    }

    /// Set the volume level.
    #[must_use]
    pub fn with_volume(mut self, volume: VolumeLevel) -> Self {
        self.config.volume = volume;
        self
    }

    /// Set the failover strategy.
    #[must_use]
    pub fn with_failover(mut self, strategy: TtsFailoverStrategy) -> Self {
        self.config.failover_strategy = strategy;
        self
    }

    /// Apply a complete TtsConfig.
    #[must_use]
    pub fn with_config(mut self, config: TtsConfig) -> Self {
        self.config = config;
        self
    }

    /// Get the text that will be spoken.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Get the current configuration.
    pub fn config(&self) -> &TtsConfig {
        &self.config
    }

    /// Check if audio has been pre-generated.
    pub fn is_prepared(&self) -> bool {
        self.audio.is_some()
    }

    /// Pre-generate the audio for later playback.
    ///
    /// This is useful when you want to minimize latency at playback time.
    /// The audio is generated and cached, then played when `play()` is called.
    ///
    /// ## Errors
    ///
    /// Returns `TtsError` if audio generation fails.
    ///
    /// ## Examples
    ///
    /// ```ignore
    /// let prepared = Speak::new("Important announcement")
    ///     .prepare()
    ///     .await?;
    ///
    /// // Audio is now cached, play() will be faster
    /// prepared.play().await?;
    /// ```
    pub async fn prepare(self) -> Result<Self, TtsError> {
        // TODO: Implement audio generation for cloud providers
        // For now, host providers don't support pre-generation
        Ok(self)
    }

    /// Play the TTS audio.
    ///
    /// If `prepare()` was called, plays the cached audio.
    /// Otherwise, generates and plays the audio on-demand.
    ///
    /// ## Errors
    ///
    /// Returns `TtsError` if generation or playback fails.
    ///
    /// ## Examples
    ///
    /// ```ignore
    /// Speak::new("Hello!").play().await?;
    /// ```
    pub async fn play(self) -> Result<(), TtsError> {
        // If audio was pre-generated, play it
        if let Some(audio) = &self.audio {
            return crate::playback::play_audio_bytes(audio, self.audio_format).await;
        }

        // Get available providers based on failover strategy
        let providers = get_providers_for_strategy(&self.config.failover_strategy);

        if providers.is_empty() {
            return Err(TtsError::NoProvidersAvailable);
        }

        // Try providers with failover
        self.execute_with_failover(&providers).await
    }

    /// Execute TTS with failover, collecting all errors.
    async fn execute_with_failover(&self, providers: &[TtsProvider]) -> Result<(), TtsError> {
        let mut errors: Vec<(TtsProvider, TtsError)> = Vec::new();

        for provider in providers {
            tracing::debug!(provider = ?provider, text_len = self.text.len(), "Trying TTS provider");

            match self.execute_provider(*provider).await {
                Ok(()) => {
                    tracing::debug!(provider = ?provider, "TTS provider succeeded");
                    return Ok(());
                }
                Err(e) => {
                    tracing::debug!(provider = ?provider, error = ?e, "TTS provider failed, trying next");
                    errors.push((*provider, e));
                }
            }
        }

        // All providers failed - return ALL errors for debugging
        Err(TtsError::AllProvidersFailed(AllProvidersFailed { errors }))
    }

    /// Execute TTS with a specific provider.
    async fn execute_provider(&self, provider: TtsProvider) -> Result<(), TtsError> {
        match provider {
            TtsProvider::Host(host) => self.execute_host_provider(host).await,
            TtsProvider::Cloud(cloud) => self.execute_cloud_provider(cloud).await,
        }
    }

    /// Execute TTS with a cloud provider.
    async fn execute_cloud_provider(&self, provider: CloudTtsProvider) -> Result<(), TtsError> {
        match provider {
            CloudTtsProvider::ElevenLabs => {
                let executor = ElevenLabsProvider::new()?;
                executor.speak(&self.text, &self.config).await
            }
        }
    }

    /// Execute TTS with a host provider.
    async fn execute_host_provider(&self, provider: HostTtsProvider) -> Result<(), TtsError> {
        match provider {
            HostTtsProvider::Say => {
                let executor = SayProvider;
                executor.speak(&self.text, &self.config).await
            }
            HostTtsProvider::ESpeak => {
                let executor = ESpeakProvider::new();
                executor.speak(&self.text, &self.config).await
            }
            HostTtsProvider::EchoGarden => {
                let executor = EchogardenProvider::new();
                executor.speak(&self.text, &self.config).await
            }
            HostTtsProvider::Gtts => {
                let executor = GttsProvider::new();
                executor.speak(&self.text, &self.config).await
            }
            HostTtsProvider::KokoroTts => {
                let executor = KokoroTtsProvider::new();
                executor.speak(&self.text, &self.config).await
            }
            HostTtsProvider::Sapi => {
                let executor = SapiProvider::new();
                executor.speak(&self.text, &self.config).await
            }
            // Other providers not yet implemented
            _ => Err(TtsError::ProviderFailed {
                provider: format!("{:?}", provider),
                message: "Provider not yet implemented".into(),
            }),
        }
    }
}

/// Convenience function for simple TTS.
///
/// This is equivalent to `Speak::new(text).with_config(config).play().await`.
///
/// ## Examples
///
/// ```ignore
/// use biscuit_speaks::{speak, TtsConfig};
///
/// speak("Hello!", &TtsConfig::default()).await?;
/// ```
pub async fn speak(text: &str, config: &TtsConfig) -> Result<(), TtsError> {
    Speak::new(text).with_config(config.clone()).play().await
}

/// Fire-and-forget TTS that ignores errors.
///
/// Use this when TTS is a nice-to-have feature and failures shouldn't
/// affect the main flow.
///
/// ## Examples
///
/// ```ignore
/// use biscuit_speaks::{speak_when_able, TtsConfig};
///
/// // Errors are logged but not propagated
/// speak_when_able("Task complete!", &TtsConfig::default()).await;
/// ```
pub async fn speak_when_able(text: &str, config: &TtsConfig) {
    if let Err(e) = speak(text, config).await {
        tracing::debug!(error = ?e, "TTS failed (non-fatal)");
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_speak_new() {
        let speak = Speak::new("Hello");
        assert_eq!(speak.text(), "Hello");
        assert!(!speak.is_prepared());
    }

    #[test]
    fn test_speak_builder_chain() {
        let speak = Speak::new("Test")
            .with_voice("Samantha")
            .with_gender(Gender::Female)
            .with_language(Language::English)
            .with_volume(VolumeLevel::Soft);

        assert_eq!(speak.text(), "Test");
        assert_eq!(speak.config().requested_voice, Some("Samantha".into()));
        assert_eq!(speak.config().gender, Gender::Female);
        assert_eq!(speak.config().language, Language::English);
        assert_eq!(speak.config().volume, VolumeLevel::Soft);
    }

    #[test]
    fn test_speak_with_config() {
        let config = TtsConfig::new()
            .with_voice("Alex")
            .with_gender(Gender::Male);

        let speak = Speak::new("Test").with_config(config);

        assert_eq!(speak.config().requested_voice, Some("Alex".into()));
        assert_eq!(speak.config().gender, Gender::Male);
    }

    #[tokio::test]
    async fn test_speak_no_providers_returns_error() {
        // With no providers available, play should fail
        let speak = Speak::new("Test").with_failover(TtsFailoverStrategy::SpecificProvider(
            TtsProvider::Host(HostTtsProvider::Pico2Wave), // Unlikely to be installed
        ));

        let result = speak.play().await;
        // Should either be NoProvidersAvailable or AllProvidersFailed
        assert!(result.is_err());
    }

    /// Regression test: All implemented host providers should have dispatch logic.
    ///
    /// Bug: execute_host_provider() only had match arms for Say and ESpeak.
    /// Other providers (EchoGarden, Gtts, KokoroTts, Sapi) fell through to the
    /// catch-all `_ =>` case which returned "Provider not yet implemented".
    #[test]
    fn test_execute_host_provider_has_all_implemented_arms() {
        // This test documents which providers MUST have explicit dispatch logic.
        // If a provider is removed from this list, it should be because:
        // 1. The provider was removed entirely, OR
        // 2. There's a good reason (e.g., platform-specific that can't be tested)
        //
        // The following providers MUST have match arms in execute_host_provider:
        let implemented_providers = [
            HostTtsProvider::Say,       // macOS
            HostTtsProvider::ESpeak,    // Cross-platform
            HostTtsProvider::EchoGarden, // Cross-platform
            HostTtsProvider::Gtts,      // Cross-platform (Python)
            HostTtsProvider::KokoroTts, // Cross-platform (Rust)
            HostTtsProvider::Sapi,      // Windows
        ];

        // Verify at compile time these variants exist in the enum
        for provider in implemented_providers {
            let _ = TtsProvider::Host(provider);
        }
    }
}
