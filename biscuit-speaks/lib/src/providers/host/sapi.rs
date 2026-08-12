//! Windows SAPI (Speech API) provider.
//!
//! This provider uses Windows Speech API via PowerShell for text-to-speech
//! synthesis. It only works on Windows systems.

use crate::errors::TtsError;
use crate::traits::{TtsExecutor, TtsVoiceInventory};
use crate::types::{
    Gender, HostTtsProvider, Language, SpeakResult, TtsConfig, TtsProvider, Voice, VoiceQuality,
};

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

    /// Select the best default voice from a list, filtering by gender with fallback.
    ///
    /// Sort by quality descending, then name ascending. If no voices match the
    /// requested gender, falls back to the best voice regardless of gender.
    fn select_best_default_voice(voices: &[Voice], gender: Gender) -> Option<Voice> {
        let mut candidates: Vec<&Voice> = voices.iter().collect();

        // Filter by gender if specified (not Any)
        if gender != Gender::Any {
            let gender_matches: Vec<&Voice> = candidates
                .iter()
                .filter(|v| v.gender == gender)
                .copied()
                .collect();

            if !gender_matches.is_empty() {
                candidates = gender_matches;
            }
        }

        // Sort by quality descending, then name ascending
        candidates.sort_by(|a, b| {
            a.quality
                .rank()
                .cmp(&b.quality.rank())
                .then_with(|| a.name.cmp(&b.name))
        });

        candidates.first().cloned().cloned()
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

    async fn default_voice(&self, gender: Gender) -> Result<Voice, TtsError> {
        let voices = self.list_voices().await?;

        Self::select_best_default_voice(&voices, gender).ok_or_else(|| {
            TtsError::VoiceEnumerationFailed {
                provider: "SAPI".into(),
                message: "No voices available".into(),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // select_best_default_voice tests
    // ========================================================================

    #[test]
    fn test_select_best_default_voice_by_quality() {
        let voices = vec![
            Voice::new("Microsoft David Desktop")
                .with_gender(Gender::Male)
                .with_quality(VoiceQuality::Good)
                .with_language(Language::English),
            Voice::new("Microsoft Mark OneCore")
                .with_gender(Gender::Male)
                .with_quality(VoiceQuality::Excellent)
                .with_language(Language::English),
        ];

        let best = SapiProvider::select_best_default_voice(&voices, Gender::Male).unwrap();
        assert_eq!(best.name, "Microsoft Mark OneCore");
    }

    #[test]
    fn test_select_best_default_voice_gender_fallback() {
        let voices = vec![
            Voice::new("Microsoft Zira Desktop")
                .with_gender(Gender::Female)
                .with_quality(VoiceQuality::Good)
                .with_language(Language::English),
        ];

        let best = SapiProvider::select_best_default_voice(&voices, Gender::Male).unwrap();
        assert_eq!(best.name, "Microsoft Zira Desktop");
    }

    #[test]
    fn test_select_best_default_voice_alphabetical_tiebreak() {
        let voices = vec![
            Voice::new("Microsoft Zira OneCore")
                .with_gender(Gender::Female)
                .with_quality(VoiceQuality::Excellent)
                .with_language(Language::English),
            Voice::new("Microsoft Catherine OneCore")
                .with_gender(Gender::Female)
                .with_quality(VoiceQuality::Excellent)
                .with_language(Language::English),
        ];

        let best = SapiProvider::select_best_default_voice(&voices, Gender::Female).unwrap();
        assert_eq!(best.name, "Microsoft Catherine OneCore");
    }

    #[test]
    fn test_select_best_default_voice_any_gender() {
        let voices = vec![
            Voice::new("Microsoft Zira Desktop")
                .with_gender(Gender::Female)
                .with_quality(VoiceQuality::Good)
                .with_language(Language::English),
            Voice::new("Microsoft David Desktop")
                .with_gender(Gender::Male)
                .with_quality(VoiceQuality::Good)
                .with_language(Language::English),
        ];

        let best = SapiProvider::select_best_default_voice(&voices, Gender::Any).unwrap();
        assert_eq!(best.name, "Microsoft David Desktop");
    }

    #[test]
    fn test_select_best_default_voice_empty_list() {
        let voices: Vec<Voice> = vec![];
        let result = SapiProvider::select_best_default_voice(&voices, Gender::Any);
        assert!(result.is_none());
    }

    // ========================================================================
    // Basic provider tests
    // ========================================================================

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

    #[cfg(not(target_os = "windows"))]
    #[tokio::test]
    async fn test_list_voices_empty_on_non_windows() {
        let provider = SapiProvider::new();
        let voices = provider.list_voices().await.unwrap();
        assert!(voices.is_empty());
    }
}
