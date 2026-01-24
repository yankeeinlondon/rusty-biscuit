//! macOS Say TTS provider.
//!
//! Uses the built-in `say` command on macOS for text-to-speech.

use std::process::Stdio;

use tokio::io::AsyncWriteExt;

use crate::errors::TtsError;
use crate::traits::TtsExecutor;
use crate::types::{Gender, TtsConfig};

/// macOS Say TTS provider.
///
/// This provider uses the `say` command available on all macOS systems.
///
/// ## Voice Selection
///
/// The `-v` flag selects the voice by name (e.g., "Samantha", "Alex").
/// Note: macOS `say` does NOT have a volume flag.
///
/// ## Examples
///
/// ```ignore
/// use biscuit_speaks::providers::host::SayProvider;
/// use biscuit_speaks::{TtsExecutor, TtsConfig};
///
/// let provider = SayProvider;
/// provider.speak("Hello, world!", &TtsConfig::default()).await?;
/// ```
#[derive(Debug, Clone, Copy, Default)]
pub struct SayProvider;

impl SayProvider {
    fn resolve_voice<'a>(config: &'a TtsConfig) -> Option<&'a str> {
        if let Some(voice) = &config.requested_voice {
            return Some(voice.as_str());
        }

        match config.gender {
            Gender::Male => Some("Alex"),
            Gender::Female => Some("Samantha"),
            Gender::Any => None,
        }
    }
}

impl TtsExecutor for SayProvider {
    async fn speak(&self, text: &str, config: &TtsConfig) -> Result<(), TtsError> {
        let mut cmd = tokio::process::Command::new("say");

        // Voice selection (NOT volume - macOS say has no volume flag)
        if let Some(voice) = Self::resolve_voice(config) {
            cmd.arg("-v").arg(voice);
        }

        // Use stdin for text input
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::null());
        cmd.stderr(Stdio::piped());

        let mut child = cmd.spawn().map_err(|e| TtsError::ProcessSpawnFailed {
            provider: "say".into(),
            source: e,
        })?;

        // Write text to stdin
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| TtsError::StdinPipeError {
                provider: "say".into(),
            })?;

        stdin
            .write_all(text.as_bytes())
            .await
            .map_err(|_| TtsError::StdinWriteError {
                provider: "say".into(),
            })?;

        // CRITICAL: Drop stdin to send EOF signal
        drop(stdin);

        // Wait for completion
        let output = child.wait_with_output().await.map_err(|e| TtsError::IoError { source: e })?;

        if output.status.success() {
            Ok(())
        } else {
            Err(TtsError::ProcessFailed {
                provider: "say".into(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            })
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_say_provider_default() {
        let provider = SayProvider::default();
        let _ = provider; // Just ensure it compiles
    }

    #[test]
    fn test_resolve_voice_requested() {
        let config = TtsConfig::new().with_voice("Alex");
        assert_eq!(SayProvider::resolve_voice(&config), Some("Alex"));
    }

    #[test]
    fn test_resolve_voice_gender_male() {
        let config = TtsConfig::new().with_gender(Gender::Male);
        assert_eq!(SayProvider::resolve_voice(&config), Some("Alex"));
    }

    #[test]
    fn test_resolve_voice_gender_female() {
        let config = TtsConfig::new().with_gender(Gender::Female);
        assert_eq!(SayProvider::resolve_voice(&config), Some("Samantha"));
    }

    #[test]
    fn test_resolve_voice_gender_any() {
        let config = TtsConfig::new();
        assert_eq!(SayProvider::resolve_voice(&config), None);
    }

    // Integration test - only runs on macOS
    #[cfg(target_os = "macos")]
    #[tokio::test]
    #[ignore] // Produces audio - run manually
    async fn test_say_provider_speaks() {
        let provider = SayProvider;
        let config = TtsConfig::default();
        let result = provider.speak("Hello from the Say provider test.", &config).await;
        assert!(result.is_ok());
    }
}
