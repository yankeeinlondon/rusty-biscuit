//! Traits for the biscuit-speaks TTS abstraction layer.
//!
//! This module defines the core trait that all TTS providers must implement.

use crate::errors::TtsError;
use crate::types::TtsConfig;

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
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // Test that we can define a mock implementation
    struct MockExecutor {
        should_fail: bool,
    }

    impl TtsExecutor for MockExecutor {
        /// Uses the text, and optionally gender/voice/volume settings to generate audio and
        /// send that audio the host's default audio device.
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
    }

    #[tokio::test]
    async fn test_mock_executor_success() {
        let executor = MockExecutor { should_fail: false };
        let config = TtsConfig::default();
        let result = executor.speak("test", &config).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_mock_executor_failure() {
        let executor = MockExecutor { should_fail: true };
        let config = TtsConfig::default();
        let result = executor.speak("test", &config).await;
        assert!(result.is_err());
    }
}
