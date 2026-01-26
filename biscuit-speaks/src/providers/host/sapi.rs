//! Windows SAPI (Speech API) provider.
//!
//! This provider uses Windows Speech API via PowerShell for text-to-speech
//! synthesis. It only works on Windows systems.

use crate::errors::TtsError;
use crate::traits::{TtsExecutor, TtsVoiceInventory};
use crate::types::{Gender, HostTtsProvider, Language, SpeakResult, TtsConfig, TtsProvider, Voice, VoiceQuality};

/// Windows SAPI provider using PowerShell commands.
///
/// This provider wraps the Windows Speech API (SAPI) using PowerShell
/// for cross-language compatibility. It provides access to all installed
/// Windows voices including SAPI5 and OneCore voices.
///
/// ## Platform Support
///
/// This provider only works on Windows. On other platforms, `is_ready()`
/// returns `false` and `list_voices()` returns an empty vector.
///
/// ## Examples
///
/// ```no_run
/// use biscuit_speaks::providers::host::SapiProvider;
/// use biscuit_speaks::TtsExecutor;
///
/// # async fn example() {
/// let provider = SapiProvider::new();
/// if provider.is_ready().await {
///     // Only true on Windows
/// }
/// # }
/// ```
#[derive(Debug, Clone, Default)]
pub struct SapiProvider;

impl SapiProvider {
    /// Create a new SAPI provider.
    pub fn new() -> Self {
        Self
    }

    /// Check if running on Windows.
    #[cfg(target_os = "windows")]
    fn is_windows() -> bool {
        true
    }

    /// Check if running on Windows.
    #[cfg(not(target_os = "windows"))]
    fn is_windows() -> bool {
        false
    }

    /// Parse PowerShell voice list output into Voice structs.
    #[allow(dead_code)]
    fn parse_voice_line(line: &str) -> Option<Voice> {
        let name = line.trim();
        if name.is_empty() {
            return None;
        }

        // Infer gender from common Windows voice naming patterns
        let gender = if name.contains("Zira")
            || name.contains("Hazel")
            || name.contains("Susan")
            || name.contains("Catherine")
        {
            Gender::Female
        } else if name.contains("David") || name.contains("Mark") || name.contains("James") {
            Gender::Male
        } else {
            Gender::Any
        };

        // Determine quality based on voice type
        // OneCore voices (newer) are better quality than SAPI5
        let quality = if name.contains("OneCore") || name.contains("Neural") {
            VoiceQuality::Excellent
        } else if name.contains("Desktop") {
            VoiceQuality::Good
        } else {
            VoiceQuality::Moderate
        };

        Some(
            Voice::new(name)
                .with_gender(gender)
                .with_quality(quality)
                .with_language(Language::English),
        )
    }
}

impl TtsExecutor for SapiProvider {
    async fn speak(&self, _text: &str, _config: &TtsConfig) -> Result<(), TtsError> {
        if !Self::is_windows() {
            return Err(TtsError::ProviderFailed {
                provider: "SAPI".to_string(),
                message: "SAPI is only available on Windows".to_string(),
            });
        }

        // Windows implementation would use PowerShell here:
        // Add-Type -AssemblyName System.Speech
        // (New-Object System.Speech.Synthesis.SpeechSynthesizer).Speak('text')

        Err(TtsError::ProviderFailed {
            provider: "SAPI".to_string(),
            message: "SAPI speak() not yet implemented".to_string(),
        })
    }

    async fn is_ready(&self) -> bool {
        Self::is_windows()
    }

    fn info(&self) -> &str {
        "Windows SAPI - Windows Speech API via PowerShell (Windows only)"
    }

    async fn speak_with_result(
        &self,
        text: &str,
        config: &TtsConfig,
    ) -> Result<SpeakResult, TtsError> {
        // Build a default voice (SAPI doesn't yet have voice selection)
        let voice = Voice::new("Windows SAPI")
            .with_gender(Gender::Any)
            .with_quality(VoiceQuality::Moderate)
            .with_language(Language::English);

        // Call speak
        self.speak(text, config).await?;

        // Return the result
        Ok(SpeakResult::new(
            TtsProvider::Host(HostTtsProvider::Sapi),
            voice,
        ))
    }
}

impl TtsVoiceInventory for SapiProvider {
    async fn list_voices(&self) -> Result<Vec<Voice>, TtsError> {
        if !Self::is_windows() {
            return Ok(Vec::new());
        }

        // Windows implementation would query via PowerShell:
        // Add-Type -AssemblyName System.Speech
        // (New-Object System.Speech.Synthesis.SpeechSynthesizer).GetInstalledVoices() |
        //   ForEach-Object { $_.VoiceInfo.Name }

        Ok(Vec::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sapi_provider_new() {
        let provider = SapiProvider::new();
        assert_eq!(
            provider.info(),
            "Windows SAPI - Windows Speech API via PowerShell (Windows only)"
        );
    }

    #[tokio::test]
    async fn test_is_ready_returns_false_on_non_windows() {
        let provider = SapiProvider::new();
        #[cfg(not(target_os = "windows"))]
        assert!(!provider.is_ready().await);
        #[cfg(target_os = "windows")]
        assert!(provider.is_ready().await);
    }

    #[test]
    fn test_parse_voice_line_female() {
        let voice = SapiProvider::parse_voice_line("Microsoft Zira Desktop").unwrap();
        assert_eq!(voice.name, "Microsoft Zira Desktop");
        assert_eq!(voice.gender, Gender::Female);
        assert_eq!(voice.quality, VoiceQuality::Good);
    }

    #[test]
    fn test_parse_voice_line_male() {
        let voice = SapiProvider::parse_voice_line("Microsoft David Desktop").unwrap();
        assert_eq!(voice.name, "Microsoft David Desktop");
        assert_eq!(voice.gender, Gender::Male);
        assert_eq!(voice.quality, VoiceQuality::Good);
    }

    #[test]
    fn test_parse_voice_line_onecore() {
        let voice = SapiProvider::parse_voice_line("Microsoft Zira OneCore").unwrap();
        assert_eq!(voice.quality, VoiceQuality::Excellent);
    }

    #[test]
    fn test_parse_voice_line_empty() {
        assert!(SapiProvider::parse_voice_line("").is_none());
        assert!(SapiProvider::parse_voice_line("   ").is_none());
    }

    #[tokio::test]
    async fn test_list_voices_empty_on_non_windows() {
        let provider = SapiProvider::new();
        let voices = provider.list_voices().await.unwrap();
        #[cfg(not(target_os = "windows"))]
        assert!(voices.is_empty());
    }
}
