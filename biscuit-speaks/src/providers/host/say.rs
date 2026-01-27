//! macOS Say TTS provider.
//!
//! Uses the built-in `say` command on macOS for text-to-speech.

use std::process::Stdio;

use tokio::io::AsyncWriteExt;
use tracing::{debug, trace};

use crate::errors::TtsError;
use crate::gender_inference::infer_gender;
use crate::traits::{TtsExecutor, TtsVoiceInventory};
use crate::types::{Gender, HostTtsProvider, Language, SpeedLevel, SpeakResult, TtsConfig, TtsProvider, Voice, VoiceQuality};

/// Default speaking rate for macOS `say` command in words per minute.
const DEFAULT_RATE_WPM: f32 = 175.0;

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
    /// Provider name constant for error messages.
    const PROVIDER_NAME: &'static str = "say";

    fn resolve_voice(config: &TtsConfig) -> Option<&str> {
        if let Some(voice) = &config.requested_voice {
            return Some(voice.as_str());
        }

        match config.gender {
            Gender::Male => Some("Alex"),
            Gender::Female => Some("Samantha"),
            Gender::Any => None,
        }
    }

    /// Convert a SpeedLevel to words per minute for the `-r` flag.
    ///
    /// Returns `None` for normal speed (use system default).
    fn resolve_rate(speed: SpeedLevel) -> Option<u32> {
        match speed {
            SpeedLevel::Normal => None, // Use default
            _ => {
                let rate = (DEFAULT_RATE_WPM * speed.value()).round() as u32;
                Some(rate)
            }
        }
    }

    /// Select the best voice from a list based on config constraints.
    ///
    /// Selection priority:
    /// 1. Filter by language (if specified)
    /// 2. Filter by gender (if specified)
    /// 3. Sort by quality (highest first)
    /// 4. Return the first match
    fn select_best_voice(voices: &[Voice], config: &TtsConfig) -> Option<Voice> {
        let mut candidates: Vec<&Voice> = voices.iter().collect();

        // Filter by language if specified
        let target_language = &config.language;
        candidates.retain(|v| {
            v.languages.iter().any(|lang| {
                match (lang, target_language) {
                    (Language::English, Language::English) => true,
                    (Language::Custom(a), Language::Custom(b)) => {
                        // Match if same language code or if one is a prefix of the other
                        let a_lower = a.to_lowercase();
                        let b_lower = b.to_lowercase();
                        a_lower == b_lower
                            || a_lower.starts_with(&format!("{}-", b_lower))
                            || b_lower.starts_with(&format!("{}-", a_lower))
                    }
                    (Language::English, Language::Custom(c))
                    | (Language::Custom(c), Language::English) => {
                        c.to_lowercase().starts_with("en")
                    }
                }
            })
        });

        // Filter by gender if specified (not Any)
        if config.gender != Gender::Any {
            let gender_matches: Vec<&Voice> = candidates
                .iter()
                .filter(|v| v.gender == config.gender)
                .copied()
                .collect();

            // Only apply gender filter if we have matches
            if !gender_matches.is_empty() {
                candidates = gender_matches;
            }
        }

        // Sort by quality (highest first)
        candidates.sort_by(|a, b| {
            let quality_rank = |q: VoiceQuality| match q {
                VoiceQuality::Excellent => 0,
                VoiceQuality::Good => 1,
                VoiceQuality::Moderate => 2,
                VoiceQuality::Low => 3,
                VoiceQuality::Unknown => 4,
            };
            quality_rank(a.quality).cmp(&quality_rank(b.quality))
        });

        candidates.first().cloned().cloned()
    }

    /// Parse a single line of `say -v '?'` output into a Voice.
    ///
    /// The format is:
    /// ```text
    /// VoiceName           locale    # Sample text
    /// VoiceName (Qualifier) locale  # Sample text
    /// ```
    ///
    /// Returns `None` for lines that:
    /// - Cannot be parsed
    /// - Contain "Eloquence" (low quality robotic voices to filter out)
    fn parse_voice_line(line: &str) -> Option<Voice> {
        // Skip empty lines
        let line = line.trim();
        if line.is_empty() {
            return None;
        }

        // Split on '#' to separate metadata from sample text
        let parts: Vec<&str> = line.splitn(2, '#').collect();
        if parts.is_empty() {
            return None;
        }

        let metadata = parts[0].trim();

        // Split metadata to get voice name and locale
        // The format uses multiple spaces as delimiter, locale is the last token
        let tokens: Vec<&str> = metadata.split_whitespace().collect();
        if tokens.len() < 2 {
            trace!(line = line, "Skipping line with insufficient tokens");
            return None;
        }

        // Last token is the locale (e.g., "en_US", "fr_FR")
        let locale = *tokens.last()?;

        // Everything before the locale is the voice name
        // Need to reconstruct it carefully because of potential parentheses
        let name_end = metadata.rfind(locale)?;
        let name = metadata[..name_end].trim();

        if name.is_empty() {
            trace!(line = line, "Skipping line with empty name");
            return None;
        }

        // Filter out Eloquence voices (low quality robotic voices)
        if name.contains("Eloquence") {
            debug!(name = name, "Filtering out Eloquence voice");
            return None;
        }

        // Determine quality based on name suffixes
        let quality = if name.contains("(Enhanced)") || name.contains("(Premium)") {
            VoiceQuality::Good
        } else {
            VoiceQuality::Moderate
        };

        // Extract the base name for gender inference
        // Remove qualifiers like "(Enhanced)", "(Premium)", "(English (US))"
        let base_name = Self::extract_base_name(name);
        let gender = infer_gender(&base_name);

        // Parse locale to Language
        let language = Self::parse_locale_to_language(locale);

        Some(
            Voice::new(name)
                .with_gender(gender)
                .with_quality(quality)
                .with_language(language),
        )
    }

    /// Extract the base name from a voice name, removing qualifiers.
    ///
    /// Examples:
    /// - "Samantha (Enhanced)" -> "Samantha"
    /// - "Eddy (English (US))" -> "Eddy"
    /// - "Albert" -> "Albert"
    fn extract_base_name(name: &str) -> String {
        // Find the first opening parenthesis and take everything before it
        if let Some(paren_pos) = name.find('(') {
            name[..paren_pos].trim().to_string()
        } else {
            name.trim().to_string()
        }
    }

    /// Parse a locale string (e.g., "en_US") into a Language.
    fn parse_locale_to_language(locale: &str) -> Language {
        // Common English locale prefixes
        if locale.starts_with("en") {
            Language::English
        } else {
            // Return the full locale as a custom language
            Language::Custom(locale.replace('_', "-"))
        }
    }

    /// Check if the `say` binary exists on the system.
    async fn say_binary_exists() -> bool {
        which::which("say").is_ok()
    }
}

impl TtsExecutor for SayProvider {
    async fn speak(&self, text: &str, config: &TtsConfig) -> Result<(), TtsError> {
        let mut cmd = tokio::process::Command::new("say");

        // Voice selection (NOT volume - macOS say has no volume flag)
        if let Some(voice) = Self::resolve_voice(config) {
            cmd.arg("-v").arg(voice);
        }

        // Rate (speed) selection
        if let Some(rate) = Self::resolve_rate(config.speed) {
            cmd.arg("-r").arg(rate.to_string());
        }

        // Use stdin for text input
        cmd.stdin(Stdio::piped());
        cmd.stdout(Stdio::null());
        cmd.stderr(Stdio::piped());

        let mut child = cmd.spawn().map_err(|e| TtsError::ProcessSpawnFailed {
            provider: Self::PROVIDER_NAME.into(),
            source: e,
        })?;

        // Write text to stdin
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| TtsError::StdinPipeError {
                provider: Self::PROVIDER_NAME.into(),
            })?;

        stdin
            .write_all(text.as_bytes())
            .await
            .map_err(|_| TtsError::StdinWriteError {
                provider: Self::PROVIDER_NAME.into(),
            })?;

        // CRITICAL: Drop stdin to send EOF signal
        drop(stdin);

        // Wait for completion
        let output = child
            .wait_with_output()
            .await
            .map_err(|e| TtsError::IoError { source: e })?;

        if output.status.success() {
            Ok(())
        } else {
            Err(TtsError::ProcessFailed {
                provider: Self::PROVIDER_NAME.into(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            })
        }
    }

    /// Check if the Say provider is ready.
    ///
    /// Returns `true` only on macOS when the `say` binary exists.
    async fn is_ready(&self) -> bool {
        // Only available on macOS
        if !cfg!(target_os = "macos") {
            return false;
        }

        Self::say_binary_exists().await
    }

    /// Get provider information.
    fn info(&self) -> &str {
        "macOS Say - Built-in speech synthesis using the `say` command"
    }

    async fn speak_with_result(
        &self,
        text: &str,
        config: &TtsConfig,
    ) -> Result<SpeakResult, TtsError> {
        // If a specific voice was requested, use it directly
        if let Some(voice_name) = &config.requested_voice {
            // Speak with the requested voice
            self.speak(text, config).await?;

            // Try to find the voice in the list for full metadata
            if let Ok(voices) = self.list_voices().await
                && let Some(voice) = voices.iter().find(|v| v.name == *voice_name)
            {
                return Ok(SpeakResult::new(
                    TtsProvider::Host(HostTtsProvider::Say),
                    voice.clone(),
                ));
            }

            // Fallback if voice not found in list
            return Ok(SpeakResult::new(
                TtsProvider::Host(HostTtsProvider::Say),
                Voice::new(voice_name.clone()).with_language(Language::English),
            ));
        }

        // No specific voice requested - select the best one based on constraints
        let voices = self.list_voices().await?;
        let selected_voice = Self::select_best_voice(&voices, config).ok_or_else(|| {
            TtsError::VoiceEnumerationFailed {
                provider: Self::PROVIDER_NAME.into(),
                message: "No matching voice found for the given constraints".into(),
            }
        })?;

        // Update config to use the selected voice
        let mut config_with_voice = config.clone();
        config_with_voice.requested_voice = Some(selected_voice.name.clone());

        // Speak with the selected voice
        self.speak(text, &config_with_voice).await?;

        Ok(SpeakResult::new(
            TtsProvider::Host(HostTtsProvider::Say),
            selected_voice,
        ))
    }
}

impl TtsVoiceInventory for SayProvider {
    /// List all available voices from macOS `say`.
    ///
    /// Parses the output of `say -v '?'` to extract voice metadata.
    /// Filters out Eloquence voices (low quality robotic voices).
    async fn list_voices(&self) -> Result<Vec<Voice>, TtsError> {
        // Run `say -v '?'` to list voices
        let output = tokio::process::Command::new("say")
            .arg("-v")
            .arg("?")
            .output()
            .await
            .map_err(|e| TtsError::ProcessSpawnFailed {
                provider: Self::PROVIDER_NAME.into(),
                source: e,
            })?;

        if !output.status.success() {
            return Err(TtsError::VoiceEnumerationFailed {
                provider: Self::PROVIDER_NAME.into(),
                message: String::from_utf8_lossy(&output.stderr).to_string(),
            });
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let voices: Vec<Voice> = stdout
            .lines()
            .filter_map(Self::parse_voice_line)
            .collect();

        debug!(
            provider = Self::PROVIDER_NAME,
            voice_count = voices.len(),
            "Enumerated voices"
        );

        Ok(voices)
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // Basic provider tests
    // ========================================================================

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

    #[test]
    fn test_resolve_rate_normal() {
        assert_eq!(SayProvider::resolve_rate(SpeedLevel::Normal), None);
    }

    #[test]
    fn test_resolve_rate_fast() {
        // Fast = 1.25x, so 175 * 1.25 = 219 (rounded)
        let rate = SayProvider::resolve_rate(SpeedLevel::Fast).unwrap();
        assert_eq!(rate, 219);
    }

    #[test]
    fn test_resolve_rate_slow() {
        // Slow = 0.75x, so 175 * 0.75 = 131 (rounded)
        let rate = SayProvider::resolve_rate(SpeedLevel::Slow).unwrap();
        assert_eq!(rate, 131);
    }

    #[test]
    fn test_resolve_rate_explicit() {
        // 2x speed = 175 * 2.0 = 350
        let rate = SayProvider::resolve_rate(SpeedLevel::Explicit(2.0)).unwrap();
        assert_eq!(rate, 350);
    }

    #[test]
    fn test_info() {
        let provider = SayProvider;
        assert!(provider.info().contains("macOS"));
        assert!(provider.info().contains("say"));
    }

    // ========================================================================
    // Voice line parsing tests
    // ========================================================================

    #[test]
    fn test_parse_simple_voice() {
        let line = "Albert              en_US    # Hello! My name is Albert.";
        let voice = SayProvider::parse_voice_line(line).unwrap();

        assert_eq!(voice.name, "Albert");
        assert_eq!(voice.gender, Gender::Male);
        assert_eq!(voice.quality, VoiceQuality::Moderate);
        assert_eq!(voice.languages, vec![Language::English]);
    }

    #[test]
    fn test_parse_enhanced_voice() {
        let line = "Samantha (Enhanced) en_US    # Hello! My name is Samantha.";
        let voice = SayProvider::parse_voice_line(line).unwrap();

        assert_eq!(voice.name, "Samantha (Enhanced)");
        assert_eq!(voice.gender, Gender::Female);
        assert_eq!(voice.quality, VoiceQuality::Good);
        assert_eq!(voice.languages, vec![Language::English]);
    }

    #[test]
    fn test_parse_premium_voice() {
        let line = "Karen (Premium)     en_AU    # Hello! My name is Karen.";
        let voice = SayProvider::parse_voice_line(line).unwrap();

        assert_eq!(voice.name, "Karen (Premium)");
        // Karen is classified as Male by gender_guesser (Danish male name)
        assert_eq!(voice.gender, Gender::Male);
        assert_eq!(voice.quality, VoiceQuality::Good);
        assert_eq!(voice.languages, vec![Language::English]);
    }

    #[test]
    fn test_parse_voice_with_language_qualifier() {
        let line = "Eddy (English (US)) en_US    # Hello! My name is Eddy.";
        let voice = SayProvider::parse_voice_line(line).unwrap();

        assert_eq!(voice.name, "Eddy (English (US))");
        // Eddy is an ambiguous name, so gender_guesser returns Any
        assert_eq!(voice.quality, VoiceQuality::Moderate);
        assert_eq!(voice.languages, vec![Language::English]);
    }

    #[test]
    fn test_parse_non_english_voice() {
        let line = "Alice               it_IT    # Ciao! Mi chiamo Alice.";
        let voice = SayProvider::parse_voice_line(line).unwrap();

        assert_eq!(voice.name, "Alice");
        assert_eq!(voice.gender, Gender::Female);
        assert_eq!(voice.quality, VoiceQuality::Moderate);
        assert_eq!(voice.languages, vec![Language::Custom("it-IT".into())]);
    }

    #[test]
    fn test_parse_french_voice() {
        let line = "Amélie              fr_CA    # Bonjour! Je m'appelle Amélie.";
        let voice = SayProvider::parse_voice_line(line).unwrap();

        assert_eq!(voice.name, "Amélie");
        assert_eq!(voice.quality, VoiceQuality::Moderate);
        assert_eq!(voice.languages, vec![Language::Custom("fr-CA".into())]);
    }

    #[test]
    fn test_parse_german_voice() {
        let line = "Anna                de_DE    # Hallo! Ich heiße Anna.";
        let voice = SayProvider::parse_voice_line(line).unwrap();

        assert_eq!(voice.name, "Anna");
        // Anna is classified as BothMaleFemale by gender_guesser (varies by culture)
        assert_eq!(voice.gender, Gender::Any);
        assert_eq!(voice.quality, VoiceQuality::Moderate);
        assert_eq!(voice.languages, vec![Language::Custom("de-DE".into())]);
    }

    #[test]
    fn test_parse_voice_en_gb() {
        let line = "Daniel              en_GB    # Hello! My name is Daniel.";
        let voice = SayProvider::parse_voice_line(line).unwrap();

        assert_eq!(voice.name, "Daniel");
        assert_eq!(voice.gender, Gender::Male);
        assert_eq!(voice.languages, vec![Language::English]);
    }

    #[test]
    fn test_parse_voice_en_in() {
        let line = "Aman (English (India)) en_IN    # Hello! My name is Aman.";
        let voice = SayProvider::parse_voice_line(line).unwrap();

        assert_eq!(voice.name, "Aman (English (India))");
        assert_eq!(voice.languages, vec![Language::English]);
    }

    #[test]
    fn test_filter_eloquence_voice() {
        let line = "Eloquence Clara     en_US    # Hello! My name is Clara.";
        let voice = SayProvider::parse_voice_line(line);

        assert!(voice.is_none(), "Eloquence voices should be filtered out");
    }

    #[test]
    fn test_parse_empty_line() {
        assert!(SayProvider::parse_voice_line("").is_none());
        assert!(SayProvider::parse_voice_line("   ").is_none());
    }

    #[test]
    fn test_parse_malformed_line() {
        // No locale - just a name
        assert!(SayProvider::parse_voice_line("BadVoice").is_none());
    }

    #[test]
    fn test_parse_novelty_voices() {
        // macOS includes some novelty/effect voices
        let line = "Bells               en_US    # Hello! My name is Bells.";
        let voice = SayProvider::parse_voice_line(line).unwrap();

        assert_eq!(voice.name, "Bells");
        // "Bells" is not a human name, so gender should be Any
        assert_eq!(voice.gender, Gender::Any);
        assert_eq!(voice.quality, VoiceQuality::Moderate);
    }

    #[test]
    fn test_parse_bad_news_voice() {
        let line = "Bad News            en_US    # Hello! My name is Bad News.";
        let voice = SayProvider::parse_voice_line(line).unwrap();

        assert_eq!(voice.name, "Bad News");
        assert_eq!(voice.quality, VoiceQuality::Moderate);
    }

    // ========================================================================
    // Base name extraction tests
    // ========================================================================

    #[test]
    fn test_extract_base_name_simple() {
        assert_eq!(SayProvider::extract_base_name("Albert"), "Albert");
    }

    #[test]
    fn test_extract_base_name_enhanced() {
        assert_eq!(
            SayProvider::extract_base_name("Samantha (Enhanced)"),
            "Samantha"
        );
    }

    #[test]
    fn test_extract_base_name_premium() {
        assert_eq!(SayProvider::extract_base_name("Karen (Premium)"), "Karen");
    }

    #[test]
    fn test_extract_base_name_nested_parens() {
        assert_eq!(
            SayProvider::extract_base_name("Eddy (English (US))"),
            "Eddy"
        );
    }

    #[test]
    fn test_extract_base_name_with_spaces() {
        assert_eq!(SayProvider::extract_base_name("Bad News"), "Bad News");
    }

    // ========================================================================
    // Locale parsing tests
    // ========================================================================

    #[test]
    fn test_parse_locale_english_us() {
        assert_eq!(
            SayProvider::parse_locale_to_language("en_US"),
            Language::English
        );
    }

    #[test]
    fn test_parse_locale_english_gb() {
        assert_eq!(
            SayProvider::parse_locale_to_language("en_GB"),
            Language::English
        );
    }

    #[test]
    fn test_parse_locale_english_au() {
        assert_eq!(
            SayProvider::parse_locale_to_language("en_AU"),
            Language::English
        );
    }

    #[test]
    fn test_parse_locale_french() {
        assert_eq!(
            SayProvider::parse_locale_to_language("fr_FR"),
            Language::Custom("fr-FR".into())
        );
    }

    #[test]
    fn test_parse_locale_german() {
        assert_eq!(
            SayProvider::parse_locale_to_language("de_DE"),
            Language::Custom("de-DE".into())
        );
    }

    #[test]
    fn test_parse_locale_japanese() {
        assert_eq!(
            SayProvider::parse_locale_to_language("ja_JP"),
            Language::Custom("ja-JP".into())
        );
    }

    #[test]
    fn test_parse_locale_chinese() {
        assert_eq!(
            SayProvider::parse_locale_to_language("zh_CN"),
            Language::Custom("zh-CN".into())
        );
    }

    // ========================================================================
    // Integration tests - macOS only
    // ========================================================================

    #[cfg(target_os = "macos")]
    #[tokio::test]
    #[ignore] // Produces audio - run manually
    async fn test_say_provider_speaks() {
        let provider = SayProvider;
        let config = TtsConfig::default();
        let result = provider
            .speak("Hello from the Say provider test.", &config)
            .await;
        assert!(result.is_ok());
    }

    #[cfg(target_os = "macos")]
    #[tokio::test]
    async fn test_is_ready_on_macos() {
        let provider = SayProvider;
        // On macOS, say should always be ready
        assert!(provider.is_ready().await);
    }

    #[cfg(target_os = "macos")]
    #[tokio::test]
    async fn test_list_voices_returns_voices() {
        let provider = SayProvider;
        let voices = provider.list_voices().await.unwrap();

        // Should have at least some voices
        assert!(!voices.is_empty(), "Expected at least one voice");

        // Check that we have expected common voices
        let voice_names: Vec<&str> = voices.iter().map(|v| v.name.as_str()).collect();

        // These voices are typically available on macOS
        let expected_voices = ["Samantha", "Alex", "Victoria"];
        for expected in expected_voices {
            let found = voice_names.iter().any(|name| name.contains(expected));
            // Note: Not all voices may be installed, so we just check a few
            if found {
                println!("Found expected voice: {}", expected);
            }
        }

        // Verify voice properties are populated
        for voice in &voices {
            assert!(!voice.name.is_empty(), "Voice name should not be empty");
            assert!(
                !voice.languages.is_empty(),
                "Voice should have at least one language"
            );
        }
    }

    #[cfg(target_os = "macos")]
    #[tokio::test]
    async fn test_list_voices_no_eloquence() {
        let provider = SayProvider;
        let voices = provider.list_voices().await.unwrap();

        // Verify no Eloquence voices are included
        for voice in &voices {
            assert!(
                !voice.name.contains("Eloquence"),
                "Eloquence voice should be filtered: {}",
                voice.name
            );
        }
    }

    #[cfg(target_os = "macos")]
    #[tokio::test]
    async fn test_enhanced_voices_have_good_quality() {
        let provider = SayProvider;
        let voices = provider.list_voices().await.unwrap();

        for voice in &voices {
            if voice.name.contains("(Enhanced)") || voice.name.contains("(Premium)") {
                assert_eq!(
                    voice.quality,
                    VoiceQuality::Good,
                    "Enhanced/Premium voice {} should have Good quality",
                    voice.name
                );
            }
        }
    }
}
