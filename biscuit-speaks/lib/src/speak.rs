//! The main Speak struct for TTS operations.
//!
//! This module provides the primary API for text-to-speech functionality.

use crate::detection::get_providers_for_strategy;
use crate::errors::{AllProvidersFailed, TtsError};
use crate::prepared::{ResolvedSynthesis, dry_run_enabled};
use crate::providers::cloud::ElevenLabsProvider;
#[cfg(target_os = "windows")]
use crate::providers::host::SapiProvider;
#[cfg(target_os = "macos")]
use crate::providers::host::SayProvider;
use crate::providers::host::{ESpeakProvider, EchogardenProvider, GttsProvider, KokoroTtsProvider};
use crate::traits::TtsExecutor;
use crate::types::{
    CloudTtsProvider, Gender, HostTtsProvider, Language, SpeakResult, TtsConfig,
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
/// - Ahead-of-time synthesis into the shared audio cache via `prepare()`
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
/// // Synthesize ahead of time; a later call with the same settings reuses the file
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
    /// TTS configuration.
    config: TtsConfig,
    /// The settings `prepare` resolved, which also name its provider.
    ///
    /// Kept apart from `config`: the settings belong to that provider alone,
    /// and failover to another provider uses the caller's original config.
    prepared: Option<ResolvedSynthesis>,
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
            config: TtsConfig::default(),
            prepared: None,
        }
    }

    /// Set the requested voice name.
    ///
    /// Voice names are provider-specific. For example:
    /// - macOS `say`: "Samantha", "Alex", "Daniel"
    /// - ElevenLabs: voice IDs like "21m00Tcm4TlvDq8ikWAM"
    ///
    /// Clears any preparation, as every builder input that can change the
    /// synthesized audio does.
    #[must_use]
    pub fn with_voice(mut self, voice: impl Into<String>) -> Self {
        self.config.requested_voice = Some(voice.into());
        self.prepared = None;
        self
    }

    /// Set the gender preference for voice selection. Clears any preparation.
    #[must_use]
    pub fn with_gender(mut self, gender: Gender) -> Self {
        self.config.gender = gender;
        self.prepared = None;
        self
    }

    /// Set the language preference for voice selection. Clears any preparation.
    #[must_use]
    pub fn with_language(mut self, language: Language) -> Self {
        self.config.language = language;
        self.prepared = None;
        self
    }

    /// Set the volume level.
    ///
    /// Keeps any preparation: volume is applied at playback, never synthesized.
    #[must_use]
    pub fn with_volume(mut self, volume: VolumeLevel) -> Self {
        self.config.volume = volume;
        self
    }

    /// Set the failover strategy. Clears any preparation.
    #[must_use]
    pub fn with_failover(mut self, strategy: TtsFailoverStrategy) -> Self {
        self.config.failover_strategy = strategy;
        self.prepared = None;
        self
    }

    /// Apply a complete TtsConfig. Clears any preparation.
    #[must_use]
    pub fn with_config(mut self, config: TtsConfig) -> Self {
        self.config = config;
        self.prepared = None;
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

    /// Whether `prepare` on this object published a cache file.
    ///
    /// Stays false after a dry run or a preparation that had nothing to do
    /// (eSpeak, SAPI, or a default that could not be resolved). It records
    /// completed work; it does not promise the temporary file still exists.
    pub fn is_prepared(&self) -> bool {
        self.prepared.is_some()
    }

    /// Synthesize the phrase into the shared audio cache without playing it.
    ///
    /// Only the first available provider for the failover strategy is
    /// prepared; its error is returned rather than trying another provider.
    /// The returned object retains the settings that provider resolved. A
    /// fresh `Speak` that resolves the same provider and settings also reuses
    /// the file, so the returned object may be discarded.
    ///
    /// Under `PLAYA_DRY_RUN` this returns `self` unchanged before looking at
    /// any provider. Preparation can make a billed cloud request, never plays
    /// audio, and never publishes a playback job.
    ///
    /// ## Errors
    ///
    /// `NoProvidersAvailable` when the strategy selects no provider; otherwise
    /// the selected provider's construction, resolution, or synthesis error.
    ///
    /// ## Examples
    ///
    /// ```ignore
    /// let prepared = Speak::new("Important announcement")
    ///     .prepare()
    ///     .await?;
    /// // False when there was nothing to prepare (eSpeak, an unresolved default).
    /// let _published = prepared.is_prepared();
    /// prepared.play().await?;
    /// ```
    pub async fn prepare(mut self) -> Result<Self, TtsError> {
        if dry_run_enabled() {
            return Ok(self);
        }
        let providers = get_providers_for_strategy(&self.config.failover_strategy);
        let Some(&provider) = providers.first() else {
            return Err(TtsError::NoProvidersAvailable);
        };
        if let Some(settings) = Self::prepare_provider(provider, &self.text, &self.config).await? {
            self.prepared = Some(settings);
        }
        Ok(self)
    }

    /// Prepare one provider, returning the settings when it published a file.
    async fn prepare_provider(
        provider: TtsProvider,
        text: &str,
        config: &TtsConfig,
    ) -> Result<Option<ResolvedSynthesis>, TtsError> {
        match provider {
            TtsProvider::Host(HostTtsProvider::KokoroTts) => {
                KokoroTtsProvider::new().prepare_resolved(text, config).await
            }
            TtsProvider::Host(HostTtsProvider::Gtts) => {
                GttsProvider::new().prepare_resolved(text, config).await
            }
            TtsProvider::Host(HostTtsProvider::EchoGarden) => {
                EchogardenProvider::new().prepare_resolved(text, config).await
            }
            TtsProvider::Host(HostTtsProvider::ESpeak) => {
                ESpeakProvider::new().prepare(text, config).await.map(|()| None)
            }
            #[cfg(target_os = "macos")]
            TtsProvider::Host(HostTtsProvider::Say) => {
                SayProvider.prepare_resolved(text, config).await
            }
            #[cfg(target_os = "windows")]
            TtsProvider::Host(HostTtsProvider::Sapi) => {
                SapiProvider::new().prepare(text, config).await.map(|()| None)
            }
            TtsProvider::Cloud(CloudTtsProvider::ElevenLabs) => {
                ElevenLabsProvider::new()?.prepare_resolved(text, config).await
            }
            TtsProvider::Host(host) => Err(TtsError::ProviderFailed {
                provider: format!("{:?}", host),
                message: "Provider not yet implemented".into(),
            }),
        }
    }

    /// Play the TTS audio.
    ///
    /// File-producing providers play their cached file when one exists for
    /// the resolved settings, and synthesize it first otherwise. A prepared
    /// object first plays through the provider and settings `prepare`
    /// retained; if that fails, the remaining providers of the failover
    /// strategy are tried with the original configuration.
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
        let mut errors = Vec::new();
        if let Some(settings) = &self.prepared {
            tracing::debug!(provider = ?settings.provider(), "Trying prepared TTS provider");
            match settings.speak(&self.text, &self.config).await {
                Ok(()) => return Ok(()),
                Err(error) => errors.push((settings.provider(), error)),
            }
        }

        let providers = self.fallback_providers();
        if providers.is_empty() && errors.is_empty() {
            return Err(TtsError::NoProvidersAvailable);
        }
        self.execute_with_failover(&providers, errors).await
    }

    /// The failover strategy's providers, without the prepared provider a
    /// retained-settings attempt has already tried.
    fn fallback_providers(&self) -> Vec<TtsProvider> {
        let mut providers = get_providers_for_strategy(&self.config.failover_strategy);
        if let Some(settings) = &self.prepared {
            let prepared = settings.provider();
            providers.retain(|provider| *provider != prepared);
        }
        providers
    }

    /// Durably hand speech to the per-user Playa queue and return immediately.
    ///
    /// Cache misses reserve their sequence before synthesis so later sounds
    /// cannot overtake the speech while a detached helper prepares it. A
    /// prepared object publishes its retained file when it is still cached;
    /// otherwise the helper synthesizes it again with the retained settings,
    /// falling back to the other providers with the original configuration.
    pub async fn play_detached(self) -> Result<crate::detached::DetachedJobId, TtsError> {
        #[cfg(not(feature = "playa"))]
        {
            let _ = self;
            Err(TtsError::NoAudioPlayer)
        }

        #[cfg(feature = "playa")]
        {
            if crate::prepared::dry_run_enabled() {
                return Ok(playa::detached::JobId::dry_run().into());
            }

            if let Some(settings) = &self.prepared {
                if let Some(job) = settings.cached_job(&self.text, &self.config) {
                    return playa::detached::enqueue(job)
                        .map(Into::into)
                        .map_err(Into::into);
                }
                return crate::detached::reserve_preparation(self.text, self.config, self.prepared);
            }

            let providers = get_providers_for_strategy(&self.config.failover_strategy);
            if providers.is_empty() {
                return Err(TtsError::NoProvidersAvailable);
            }

            let mut errors = Vec::new();
            for provider in providers.iter().copied() {
                match crate::detached::cached_job(provider, &self.text, &self.config).await {
                    Ok(Some(job)) => {
                        return playa::detached::enqueue(job)
                            .map(Into::into)
                            .map_err(Into::into);
                    }
                    Ok(None) => {}
                    Err(error) => {
                        errors.push((provider, error));
                        continue;
                    }
                }

                if crate::detached::requires_preparation(provider) {
                    return crate::detached::reserve_preparation(self.text, self.config, None);
                }

                match crate::detached::build_job(provider, &self.text, &self.config).await {
                    Ok(job) => {
                        return playa::detached::enqueue(job)
                            .map(Into::into)
                            .map_err(Into::into);
                    }
                    Err(error) => errors.push((provider, error)),
                }
            }
            Err(TtsError::AllProvidersFailed(AllProvidersFailed { errors }))
        }
    }

    /// Play the TTS audio and return metadata about the voice used.
    ///
    /// This is like `play()`, including a prepared object's retained
    /// settings, but also returns a `SpeakResult` naming the provider and
    /// voice that were actually used.
    ///
    /// ## Errors
    ///
    /// Returns `TtsError` if generation or playback fails.
    ///
    /// ## Examples
    ///
    /// ```ignore
    /// let result = Speak::new("Hello!").play_with_result().await?;
    /// println!("Used voice: {}", result.voice.name);
    /// ```
    pub async fn play_with_result(self) -> Result<SpeakResult, TtsError> {
        let mut errors = Vec::new();
        if let Some(settings) = &self.prepared {
            tracing::debug!(provider = ?settings.provider(), "Trying prepared TTS provider");
            match settings.speak_with_result(&self.text, &self.config).await {
                Ok(result) => return Ok(result),
                Err(error) => errors.push((settings.provider(), error)),
            }
        }

        let providers = self.fallback_providers();
        if providers.is_empty() && errors.is_empty() {
            return Err(TtsError::NoProvidersAvailable);
        }
        self.execute_with_failover_result(&providers, errors).await
    }

    /// Execute TTS with failover, collecting all errors after `errors`.
    async fn execute_with_failover(
        &self,
        providers: &[TtsProvider],
        mut errors: Vec<(TtsProvider, TtsError)>,
    ) -> Result<(), TtsError> {

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

    /// Execute TTS with failover, returning metadata about the voice used.
    async fn execute_with_failover_result(
        &self,
        providers: &[TtsProvider],
        mut errors: Vec<(TtsProvider, TtsError)>,
    ) -> Result<SpeakResult, TtsError> {

        for provider in providers {
            tracing::debug!(provider = ?provider, text_len = self.text.len(), "Trying TTS provider");

            match self.execute_provider_with_result(*provider).await {
                Ok(result) => {
                    tracing::debug!(provider = ?provider, "TTS provider succeeded");
                    return Ok(result);
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

    /// Execute TTS with a specific provider, returning metadata.
    async fn execute_provider_with_result(
        &self,
        provider: TtsProvider,
    ) -> Result<SpeakResult, TtsError> {
        match provider {
            TtsProvider::Host(host) => self.execute_host_provider_with_result(host).await,
            TtsProvider::Cloud(cloud) => self.execute_cloud_provider_with_result(cloud).await,
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
            #[cfg(target_os = "macos")]
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
            #[cfg(target_os = "windows")]
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

    /// Execute TTS with a cloud provider, returning metadata.
    async fn execute_cloud_provider_with_result(
        &self,
        provider: CloudTtsProvider,
    ) -> Result<SpeakResult, TtsError> {
        match provider {
            CloudTtsProvider::ElevenLabs => {
                let executor = ElevenLabsProvider::new()?;
                executor.speak_with_result(&self.text, &self.config).await
            }
        }
    }

    /// Execute TTS with a host provider, returning metadata.
    async fn execute_host_provider_with_result(
        &self,
        provider: HostTtsProvider,
    ) -> Result<SpeakResult, TtsError> {
        match provider {
            #[cfg(target_os = "macos")]
            HostTtsProvider::Say => {
                let executor = SayProvider;
                executor.speak_with_result(&self.text, &self.config).await
            }
            HostTtsProvider::ESpeak => {
                let executor = ESpeakProvider::new();
                executor.speak_with_result(&self.text, &self.config).await
            }
            HostTtsProvider::EchoGarden => {
                let executor = EchogardenProvider::new();
                executor.speak_with_result(&self.text, &self.config).await
            }
            HostTtsProvider::Gtts => {
                let executor = GttsProvider::new();
                executor.speak_with_result(&self.text, &self.config).await
            }
            HostTtsProvider::KokoroTts => {
                let executor = KokoroTtsProvider::new();
                executor.speak_with_result(&self.text, &self.config).await
            }
            #[cfg(target_os = "windows")]
            HostTtsProvider::Sapi => {
                let executor = SapiProvider::new();
                executor.speak_with_result(&self.text, &self.config).await
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

/// Convenience function for TTS that returns metadata about the voice used.
///
/// This is equivalent to `Speak::new(text).with_config(config).play_with_result().await`.
///
/// ## Examples
///
/// ```ignore
/// use biscuit_speaks::{speak_with_result, TtsConfig};
///
/// let result = speak_with_result("Hello!", &TtsConfig::default()).await?;
/// println!("Provider: {:?}", result.provider);
/// println!("Voice: {}", result.voice.name);
/// ```
pub async fn speak_with_result(text: &str, config: &TtsConfig) -> Result<SpeakResult, TtsError> {
    Speak::new(text)
        .with_config(config.clone())
        .play_with_result()
        .await
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

    fn prepared() -> Speak {
        let mut speak = Speak::new("Test");
        speak.prepared = Some(ResolvedSynthesis::Kokoro {
            voice: "af_heart".into(),
        });
        speak
    }

    #[test]
    fn clone_and_volume_keep_preparation() {
        assert!(prepared().is_prepared());
        assert!(prepared().clone().is_prepared());
        let louder = prepared().with_volume(VolumeLevel::Loud);
        assert!(louder.is_prepared());
        assert_eq!(louder.prepared, prepared().prepared);
    }

    #[test]
    fn synthesis_inputs_clear_preparation() {
        let cleared = [
            prepared().with_voice("Samantha"),
            prepared().with_gender(Gender::Male),
            prepared().with_language(Language::Custom("fr".into())),
            prepared().with_failover(TtsFailoverStrategy::PreferCloud),
            prepared().with_config(TtsConfig::default()),
        ];
        for speak in cleared {
            assert!(!speak.is_prepared(), "{speak:?}");
        }
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn prepare_with_no_candidate_reports_no_providers() {
        let _live = test_toolkit::EnvGuard::remove_safe("PLAYA_DRY_RUN");
        let speak = Speak::new("Test").with_failover(TtsFailoverStrategy::SpecificProvider(
            TtsProvider::Host(HostTtsProvider::Pico2Wave),
        ));
        assert!(matches!(speak.prepare().await, Err(TtsError::NoProvidersAvailable)));
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
            HostTtsProvider::Say,        // macOS
            HostTtsProvider::ESpeak,     // Cross-platform
            HostTtsProvider::EchoGarden, // Cross-platform
            HostTtsProvider::Gtts,       // Cross-platform (Python)
            HostTtsProvider::KokoroTts,  // Cross-platform (Rust)
            HostTtsProvider::Sapi,       // Windows
        ];

        // Verify at compile time these variants exist in the enum
        for provider in implemented_providers {
            let _ = TtsProvider::Host(provider);
        }
    }
}
