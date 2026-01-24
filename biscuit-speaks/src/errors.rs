//! Error types for the biscuit-speaks TTS library.
//!
//! This module defines all error types that can occur during TTS operations,
//! using `thiserror` for ergonomic error handling.

use crate::types::TtsProvider;

/// Errors that can occur during TTS operations.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum TtsError {
    /// No TTS providers are available on this system.
    #[error("No TTS providers available")]
    NoProvidersAvailable,

    /// All TTS providers failed.
    #[error("All TTS providers failed")]
    AllProvidersFailed(AllProvidersFailed),

    /// A specific provider failed.
    #[error("TTS provider '{provider}' failed: {message}")]
    ProviderFailed {
        /// The provider that failed.
        provider: String,
        /// Description of the failure.
        message: String,
    },

    /// Failed to spawn a subprocess for a host TTS provider.
    #[error("Failed to spawn process for '{provider}': {source}")]
    ProcessSpawnFailed {
        /// The provider that failed.
        provider: String,
        /// The underlying IO error.
        #[source]
        source: std::io::Error,
    },

    /// The TTS subprocess exited with a non-zero status.
    #[error("TTS process '{provider}' failed: {stderr}")]
    ProcessFailed {
        /// The provider that failed.
        provider: String,
        /// The stderr output from the process.
        stderr: String,
    },

    /// Failed to write to the subprocess stdin.
    #[error("Failed to write to stdin for '{provider}'")]
    StdinWriteError {
        /// The provider that failed.
        provider: String,
    },

    /// Failed to open stdin pipe for subprocess.
    #[error("Failed to open stdin pipe for '{provider}'")]
    StdinPipeError {
        /// The provider that failed.
        provider: String,
    },

    /// A required environment variable is missing.
    #[error("Missing environment variable '{variable}' for provider '{provider}'")]
    MissingEnvironment {
        /// The provider requiring the variable.
        provider: String,
        /// The missing variable name.
        variable: String,
    },

    /// A required API key is missing.
    #[error("Missing API key for provider '{provider}'")]
    MissingApiKey {
        /// The provider requiring the API key.
        provider: String,
    },

    /// A required model file was not found.
    #[error("Model file not found: {path}")]
    ModelFileNotFound {
        /// Path to the missing model file.
        path: String,
    },

    /// HTTP request to a cloud provider failed.
    #[error("HTTP request failed for '{provider}': {message}")]
    HttpError {
        /// The provider that failed.
        provider: String,
        /// Description of the HTTP error.
        message: String,
    },

    /// API returned an error response.
    #[error("API error from '{provider}' (status {status}): {message}")]
    ApiError {
        /// The provider that failed.
        provider: String,
        /// HTTP status code.
        status: u16,
        /// Error message from the API.
        message: String,
    },

    /// No audio player is available for playback.
    #[error("No audio player available")]
    NoAudioPlayer,

    /// Audio playback failed.
    #[error("Audio playback failed with '{player}': {stderr}")]
    PlaybackFailed {
        /// The audio player that failed.
        player: String,
        /// The stderr output from the player.
        stderr: String,
    },

    /// Failed to create a temporary file for audio.
    #[error("Failed to create temporary file: {source}")]
    TempFileError {
        /// The underlying IO error.
        #[source]
        source: std::io::Error,
    },

    /// IO error during TTS operations.
    #[error("IO error: {source}")]
    IoError {
        /// The underlying IO error.
        #[source]
        source: std::io::Error,
    },

    /// Voice selection failed.
    #[error("Voice selection failed: {reason}")]
    VoiceSelectionFailed {
        /// Description of why voice selection failed.
        reason: String,
    },

    /// No suitable voice was found.
    #[error("No suitable voice found (language: {language})")]
    NoSuitableVoice {
        /// The language that was requested.
        language: String,
    },
}

impl From<std::io::Error> for TtsError {
    fn from(source: std::io::Error) -> Self {
        TtsError::IoError { source }
    }
}

/// Container for errors from all failed providers.
///
/// When failover is enabled and all providers fail, this struct
/// collects all the individual errors for debugging purposes.
#[derive(Debug)]
pub struct AllProvidersFailed {
    /// Errors from each attempted provider.
    pub errors: Vec<(TtsProvider, TtsError)>,
}

impl std::fmt::Display for AllProvidersFailed {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "All {} providers failed:", self.errors.len())?;
        for (provider, error) in &self.errors {
            writeln!(f, "  - {:?}: {}", provider, error)?;
        }
        Ok(())
    }
}

impl std::error::Error for AllProvidersFailed {}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::HostTtsProvider;

    #[test]
    fn test_no_providers_available_display() {
        let error = TtsError::NoProvidersAvailable;
        assert_eq!(error.to_string(), "No TTS providers available");
    }

    #[test]
    fn test_provider_failed_display() {
        let error = TtsError::ProviderFailed {
            provider: "say".into(),
            message: "voice not found".into(),
        };
        assert_eq!(
            error.to_string(),
            "TTS provider 'say' failed: voice not found"
        );
    }

    #[test]
    fn test_missing_api_key_display() {
        let error = TtsError::MissingApiKey {
            provider: "elevenlabs".into(),
        };
        assert_eq!(
            error.to_string(),
            "Missing API key for provider 'elevenlabs'"
        );
    }

    #[test]
    fn test_api_error_display() {
        let error = TtsError::ApiError {
            provider: "elevenlabs".into(),
            status: 401,
            message: "Unauthorized".into(),
        };
        assert_eq!(
            error.to_string(),
            "API error from 'elevenlabs' (status 401): Unauthorized"
        );
    }

    #[test]
    fn test_all_providers_failed_display() {
        let errors = vec![
            (
                TtsProvider::Host(HostTtsProvider::Say),
                TtsError::ProviderFailed {
                    provider: "say".into(),
                    message: "not found".into(),
                },
            ),
            (
                TtsProvider::Host(HostTtsProvider::ESpeak),
                TtsError::ProviderFailed {
                    provider: "espeak".into(),
                    message: "timeout".into(),
                },
            ),
        ];
        let error = AllProvidersFailed { errors };
        let display = error.to_string();
        assert!(display.contains("All 2 providers failed"));
        assert!(display.contains("Say"));
        assert!(display.contains("ESpeak"));
    }

    #[test]
    fn test_io_error_conversion() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let tts_error: TtsError = io_error.into();
        assert!(matches!(tts_error, TtsError::IoError { .. }));
    }
}
