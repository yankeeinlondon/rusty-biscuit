//! Traits for the biscuit-speaks TTS abstraction layer.
//!
//! This module defines the core traits that TTS providers implement:
//!
//! - [`TtsExecutor`] - Core trait for text-to-speech operations (required)
//! - [`TtsVoiceInventory`] - Optional trait for voice enumeration and provider info

use crate::errors::TtsError;
use crate::types::{Gender, SpeakResult, TtsConfig, Voice};

/// Executor trait for TTS providers.
///
/// All TTS providers (both host and cloud) implement this trait to provide
/// a unified interface for text-to-speech operations.
///
/// ## Native Async Traits
///
/// This trait uses native Rust async functions in traits (AFIT), available
/// since Rust 1.75. No `async-trait` crate is needed.
///
/// ## Implementation Requirements
///
/// Implementations must be `Send + Sync` to allow concurrent usage across
/// tasks and threads.
///
/// ## Examples
///
/// ```ignore
/// use biscuit_speaks::{TtsExecutor, TtsConfig, TtsError};
///
/// struct MyProvider;
///
/// impl TtsExecutor for MyProvider {
///     async fn speak(&self, text: &str, config: &TtsConfig) -> Result<(), TtsError> {
///         // Generate and play TTS audio
///         Ok(())
///     }
/// }
/// ```
pub trait TtsExecutor: Send + Sync {
    /// Generate and play TTS audio for the given text.
    ///
    /// ## Arguments
    ///
    /// * `text` - The text to synthesize and play.
    /// * `config` - Configuration for voice selection, volume, etc.
    ///
    /// ## Errors
    ///
    /// Returns `TtsError` if synthesis or playback fails.
    fn speak(
        &self,
        text: &str,
        config: &TtsConfig,
    ) -> impl std::future::Future<Output = Result<(), TtsError>> + Send;

    /// Check if this provider is ready to generate speech.
    ///
    /// This performs a more thorough check than simple availability detection.
    /// For host providers, this may verify that required model files exist.
    /// For cloud providers, this may verify API key validity.
    ///
    /// ## Default Implementation
    ///
    /// Returns `true` by default. Providers should override this if they
    /// have additional readiness requirements beyond basic availability.
    ///
    /// ## Examples
    ///
    /// ```ignore
    /// if provider.is_ready().await {
    ///     provider.speak("Hello", &config).await?;
    /// }
    /// ```
    fn is_ready(&self) -> impl std::future::Future<Output = bool> + Send {
        async { true }
    }

    /// Get human-readable information about this provider.
    ///
    /// Returns a description suitable for display in user interfaces or
    /// diagnostic output.
    ///
    /// ## Default Implementation
    ///
    /// Returns `"Unknown TTS Provider"`. Providers should override this
    /// to return meaningful information.
    fn info(&self) -> &str {
        "Unknown TTS Provider"
    }

    /// Generate and play TTS audio, returning metadata about what was used.
    ///
    /// This method is like `speak()` but also returns a `SpeakResult` containing
    /// information about the provider and voice that were actually used.
    ///
    /// ## Arguments
    ///
    /// * `text` - The text to synthesize and play.
    /// * `config` - Configuration for voice selection, volume, etc.
    ///
    /// ## Returns
    ///
    /// A `SpeakResult` containing the provider and voice used for synthesis.
    ///
    /// ## Errors
    ///
    /// Returns `TtsError` if synthesis or playback fails.
    ///
    /// ## Examples
    ///
    /// ```ignore
    /// let result = provider.speak_with_result("Hello", &config).await?;
    /// println!("Used voice: {}", result.voice.name);
    /// ```
    fn speak_with_result(
        &self,
        text: &str,
        config: &TtsConfig,
    ) -> impl std::future::Future<Output = Result<SpeakResult, TtsError>> + Send;

    /// Build a durable playback operation for this provider.
    ///
    /// File-producing implementations may synthesize into their shared cache;
    /// callers that require immediate return run this method in the detached
    /// preparation helper after reserving the ordered spool slot.
    #[cfg(feature = "playa")]
    fn detached_job(
        &self,
        _text: &str,
        _config: &TtsConfig,
    ) -> impl std::future::Future<Output = Result<playa::detached::SpoolJob, TtsError>> + Send {
        async {
            Err(TtsError::DetachedUnsupported {
                provider: self.info().to_string(),
            })
        }
    }

    /// Return a ready detached job only when synthesis output is already cached.
    #[cfg(feature = "playa")]
    fn cached_detached_job(
        &self,
        _text: &str,
        _config: &TtsConfig,
    ) -> impl std::future::Future<Output = Result<Option<playa::detached::SpoolJob>, TtsError>> + Send
    {
        async { Ok(None) }
    }
}

/// Trait for TTS providers that support voice enumeration.
///
/// Not all TTS providers support listing available voices. This trait
/// is separate from [`TtsExecutor`] so that voice inventory features
/// can be implemented incrementally for each provider.
///
/// ## Examples
///
/// ```ignore
/// use biscuit_speaks::{TtsVoiceInventory, TtsError};
///
/// async fn list_provider_voices<P: TtsVoiceInventory>(provider: &P) -> Result<(), TtsError> {
///     let voices = provider.list_voices().await?;
///     for voice in voices {
///         println!("{}: {:?}", voice.name, voice.gender);
///     }
///     Ok(())
/// }
/// ```
pub trait TtsVoiceInventory: Send + Sync {
    /// List all available voices for this provider.
    ///
    /// Returns a list of voices that can be used with this provider.
    /// For host providers, this returns installed voices.
    /// For cloud providers, this may make an API call to fetch available voices.
    ///
    /// ## Errors
    ///
    /// Returns `TtsError::VoiceEnumerationFailed` if voice listing fails.
    fn list_voices(&self)
    -> impl std::future::Future<Output = Result<Vec<Voice>, TtsError>> + Send;

    /// Return the provider's default voice for the given gender.
    ///
    /// When `gender` is `Gender::Any`, returns the provider's overall
    /// best default voice regardless of gender.
    /// Always returns a concrete `Voice`.
    ///
    /// ## Errors
    ///
    /// Returns `TtsError` if voice resolution fails (e.g., API call failure
    /// for cloud providers, or no voices available for dynamic providers).
    fn default_voice(
        &self,
        gender: Gender,
    ) -> impl std::future::Future<Output = Result<Voice, TtsError>> + Send;
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Gender, HostTtsProvider, TtsProvider, VoiceQuality};

    // Test that we can define a mock implementation
    struct MockExecutor {
        should_fail: bool,
        is_ready: bool,
    }

    impl TtsExecutor for MockExecutor {
        async fn speak(&self, _text: &str, _config: &TtsConfig) -> Result<(), TtsError> {
            if self.should_fail {
                Err(TtsError::ProviderFailed {
                    provider: "mock".into(),
                    message: "intentional failure".into(),
                })
            } else {
                Ok(())
            }
        }

        async fn speak_with_result(
            &self,
            text: &str,
            config: &TtsConfig,
        ) -> Result<SpeakResult, TtsError> {
            self.speak(text, config).await?;
            Ok(SpeakResult::new(
                TtsProvider::Host(HostTtsProvider::Say),
                Voice::new("MockVoice").with_gender(Gender::Female),
            ))
        }

        async fn is_ready(&self) -> bool {
            self.is_ready
        }

        fn info(&self) -> &str {
            "Mock TTS Provider for testing"
        }
    }

    impl TtsVoiceInventory for MockExecutor {
        async fn list_voices(&self) -> Result<Vec<Voice>, TtsError> {
            if self.should_fail {
                Err(TtsError::VoiceEnumerationFailed {
                    provider: "mock".into(),
                    message: "intentional failure".into(),
                })
            } else {
                Ok(vec![
                    Voice::new("MockVoice1")
                        .with_gender(Gender::Female)
                        .with_quality(VoiceQuality::Good),
                    Voice::new("MockVoice2")
                        .with_gender(Gender::Male)
                        .with_quality(VoiceQuality::Moderate),
                ])
            }
        }

        async fn default_voice(&self, gender: Gender) -> Result<Voice, TtsError> {
            if self.should_fail {
                Err(TtsError::VoiceEnumerationFailed {
                    provider: "mock".into(),
                    message: "intentional failure".into(),
                })
            } else {
                match gender {
                    Gender::Male => Ok(Voice::new("MockVoice2")
                        .with_gender(Gender::Male)
                        .with_quality(VoiceQuality::Moderate)),
                    Gender::Female | Gender::Any => Ok(Voice::new("MockVoice1")
                        .with_gender(Gender::Female)
                        .with_quality(VoiceQuality::Good)),
                }
            }
        }
    }

    #[tokio::test]
    async fn test_mock_executor_success() {
        let executor = MockExecutor {
            should_fail: false,
            is_ready: true,
        };
        let config = TtsConfig::default();
        let result = executor.speak("test", &config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_executor_failure() {
        let executor = MockExecutor {
            should_fail: true,
            is_ready: true,
        };
        let config = TtsConfig::default();
        let result = executor.speak("test", &config).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mock_executor_is_ready() {
        let ready_executor = MockExecutor {
            should_fail: false,
            is_ready: true,
        };
        assert!(ready_executor.is_ready().await);

        let not_ready_executor = MockExecutor {
            should_fail: false,
            is_ready: false,
        };
        assert!(!not_ready_executor.is_ready().await);
    }

    #[test]
    fn test_mock_executor_info() {
        let executor = MockExecutor {
            should_fail: false,
            is_ready: true,
        };
        assert_eq!(executor.info(), "Mock TTS Provider for testing");
    }

    #[tokio::test]
    async fn test_mock_executor_list_voices() {
        let executor = MockExecutor {
            should_fail: false,
            is_ready: true,
        };
        let voices = executor.list_voices().await.unwrap();
        assert_eq!(voices.len(), 2);
        assert_eq!(voices[0].name, "MockVoice1");
        assert_eq!(voices[0].gender, Gender::Female);
        assert_eq!(voices[1].name, "MockVoice2");
        assert_eq!(voices[1].gender, Gender::Male);
    }

    #[tokio::test]
    async fn test_mock_executor_list_voices_failure() {
        let executor = MockExecutor {
            should_fail: true,
            is_ready: true,
        };
        let result = executor.list_voices().await;
        assert!(result.is_err());
        match result {
            Err(TtsError::VoiceEnumerationFailed { provider, .. }) => {
                assert_eq!(provider, "mock");
            }
            _ => panic!("Expected VoiceEnumerationFailed error"),
        }
    }

    #[tokio::test]
    async fn test_mock_executor_default_voice_male() {
        let executor = MockExecutor {
            should_fail: false,
            is_ready: true,
        };
        let voice = executor.default_voice(Gender::Male).await.unwrap();
        assert_eq!(voice.name, "MockVoice2");
        assert_eq!(voice.gender, Gender::Male);
    }

    #[tokio::test]
    async fn test_mock_executor_default_voice_female() {
        let executor = MockExecutor {
            should_fail: false,
            is_ready: true,
        };
        let voice = executor.default_voice(Gender::Female).await.unwrap();
        assert_eq!(voice.name, "MockVoice1");
        assert_eq!(voice.gender, Gender::Female);
    }

    #[tokio::test]
    async fn test_mock_executor_default_voice_any() {
        let executor = MockExecutor {
            should_fail: false,
            is_ready: true,
        };
        let voice = executor.default_voice(Gender::Any).await.unwrap();
        assert_eq!(voice.name, "MockVoice1");
        assert_eq!(voice.gender, Gender::Female);
    }

    #[tokio::test]
    async fn test_mock_executor_default_voice_failure() {
        let executor = MockExecutor {
            should_fail: true,
            is_ready: true,
        };
        let result = executor.default_voice(Gender::Any).await;
        assert!(result.is_err());
    }

    // Test that default implementations work
    struct MinimalExecutor;

    impl TtsExecutor for MinimalExecutor {
        async fn speak(&self, _text: &str, _config: &TtsConfig) -> Result<(), TtsError> {
            Ok(())
        }

        async fn speak_with_result(
            &self,
            text: &str,
            config: &TtsConfig,
        ) -> Result<SpeakResult, TtsError> {
            self.speak(text, config).await?;
            Ok(SpeakResult::new(
                TtsProvider::Host(HostTtsProvider::Say),
                Voice::new("MinimalVoice"),
            ))
        }
        // Use default implementations for is_ready() and info()
    }

    #[tokio::test]
    async fn test_default_is_ready() {
        let executor = MinimalExecutor;
        // Default is_ready returns true
        assert!(executor.is_ready().await);
    }

    #[test]
    fn test_default_info() {
        let executor = MinimalExecutor;
        // Default info returns "Unknown TTS Provider"
        assert_eq!(executor.info(), "Unknown TTS Provider");
    }
}
