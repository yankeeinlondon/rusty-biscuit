//! eSpeak/eSpeak-NG TTS provider.
//!
//! Uses the `espeak-ng` or `espeak` command for text-to-speech.
//! Common on Linux systems, also available on macOS and Windows.

use std::process::Stdio;

use tokio::io::AsyncWriteExt;

use crate::errors::TtsError;
use crate::traits::TtsExecutor;
use crate::types::{Gender, TtsConfig};

/// eSpeak/eSpeak-NG TTS provider.
///
/// This provider uses `espeak-ng` (preferred) or `espeak` for TTS.
///
/// ## Voice Selection
///
/// - `-v` flag sets the voice/language (e.g., "en", "en-us", "en+f3")
/// - Gender can be specified with suffixes: +m1..+m7 (male), +f1..+f5 (female)
///
/// ## Examples
///
/// ```ignore
/// use biscuit_speaks::providers::host::ESpeakProvider;
/// use biscuit_speaks::{TtsExecutor, TtsConfig};
///
/// let provider = ESpeakProvider::new();
/// provider.speak("Hello, world!", &TtsConfig::default()).await?;
/// ```
#[derive(Debug, Clone)]
pub struct ESpeakProvider {
    /// The binary to use (espeak-ng or espeak).
    binary: String,
}

impl Default for ESpeakProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl ESpeakProvider {
    /// Create a new ESpeakProvider, auto-detecting the available binary.
    pub fn new() -> Self {
        // Prefer espeak-ng over espeak
        let binary = if which::which("espeak-ng").is_ok() {
            "espeak-ng".to_string()
        } else {
            "espeak".to_string()
        };
        Self { binary }
    }

    /// Create a provider with a specific binary name.
    pub fn with_binary(binary: impl Into<String>) -> Self {
        Self {
            binary: binary.into(),
        }
    }

    /// Build the voice argument based on config.
    fn build_voice_arg(&self, config: &TtsConfig) -> String {
        // Start with language
        let lang = config.language.code_prefix();

        // If a specific voice is requested, use it directly
        if let Some(voice) = &config.requested_voice {
            return voice.clone();
        }

        // Otherwise, build from language + gender
        let gender_suffix = match config.gender {
            Gender::Male => "+m3",
            Gender::Female => "+f3",
            Gender::Any => "",
        };

        format!("{}{}", lang, gender_suffix)
    }
}

impl TtsExecutor for ESpeakProvider {
    async fn speak(&self, text: &str, config: &TtsConfig) -> Result<(), TtsError> {
        let mut cmd = tokio::process::Command::new(&self.binary);

        // Voice selection
        let voice = self.build_voice_arg(config);
        cmd.arg("-v").arg(&voice);

        // Speed (default is 175 wpm)
        // Could add speed configuration later

        // Use stdin for text input
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::null());
        cmd.stderr(Stdio::piped());

        let mut child = cmd.spawn().map_err(|e| TtsError::ProcessSpawnFailed {
            provider: self.binary.clone(),
            source: e,
        })?;

        // Write text to stdin
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| TtsError::StdinPipeError {
                provider: self.binary.clone(),
            })?;

        stdin
            .write_all(text.as_bytes())
            .await
            .map_err(|_| TtsError::StdinWriteError {
                provider: self.binary.clone(),
            })?;

        // CRITICAL: Drop stdin to send EOF signal
        drop(stdin);

        // Wait for completion
        let output = child.wait_with_output().await.map_err(|e| TtsError::IoError { source: e })?;

        if output.status.success() {
            Ok(())
        } else {
            Err(TtsError::ProcessFailed {
                provider: self.binary.clone(),
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
    use crate::types::Language;

    #[test]
    fn test_espeak_provider_default() {
        let provider = ESpeakProvider::default();
        assert!(provider.binary == "espeak-ng" || provider.binary == "espeak");
    }

    #[test]
    fn test_espeak_provider_with_binary() {
        let provider = ESpeakProvider::with_binary("espeak-ng");
        assert_eq!(provider.binary, "espeak-ng");
    }

    #[test]
    fn test_build_voice_arg_default() {
        let provider = ESpeakProvider::with_binary("espeak-ng");
        let config = TtsConfig::default();
        let voice = provider.build_voice_arg(&config);
        assert_eq!(voice, "en"); // English, any gender
    }

    #[test]
    fn test_build_voice_arg_with_gender() {
        let provider = ESpeakProvider::with_binary("espeak-ng");

        let config = TtsConfig::new().with_gender(Gender::Female);
        let voice = provider.build_voice_arg(&config);
        assert_eq!(voice, "en+f3");

        let config = TtsConfig::new().with_gender(Gender::Male);
        let voice = provider.build_voice_arg(&config);
        assert_eq!(voice, "en+m3");
    }

    #[test]
    fn test_build_voice_arg_with_language() {
        let provider = ESpeakProvider::with_binary("espeak-ng");
        let config = TtsConfig::new().with_language(Language::Custom("de".into()));
        let voice = provider.build_voice_arg(&config);
        assert_eq!(voice, "de");
    }

    #[test]
    fn test_build_voice_arg_explicit_voice() {
        let provider = ESpeakProvider::with_binary("espeak-ng");
        let config = TtsConfig::new().with_voice("en-gb+f4");
        let voice = provider.build_voice_arg(&config);
        assert_eq!(voice, "en-gb+f4");
    }
}
