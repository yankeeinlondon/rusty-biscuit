//! gTTS (Google Text-to-Speech) provider.
//!
//! Uses the `gtts-cli` Python package for text-to-speech via Google's TTS API.
//! Requires network connectivity.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

use tempfile::NamedTempFile;
use tracing::debug;

use crate::errors::TtsError;
use crate::traits::{TtsExecutor, TtsVoiceInventory};
use crate::types::{AudioFormat, Gender, HostTtsProvider, Language, SpeakResult, TtsConfig, TtsProvider, Voice, VoiceQuality};

/// gTTS (Google Text-to-Speech) provider.
///
/// This provider uses the `gtts-cli` Python package to synthesize speech
/// via Google's Text-to-Speech API. It requires network connectivity.
///
/// ## Installation
///
/// ```bash
/// pip install gTTS
/// ```
///
/// ## Voice Selection
///
/// The `--lang` flag selects the language (e.g., "en", "fr", "de").
/// gTTS does not distinguish between male/female voices.
///
/// ## Examples
///
/// ```ignore
/// use biscuit_speaks::providers::host::GttsProvider;
/// use biscuit_speaks::{TtsExecutor, TtsConfig};
///
/// let provider = GttsProvider::new();
/// provider.speak("Hello, world!", &TtsConfig::default()).await?;
/// ```
#[derive(Debug)]
pub struct GttsProvider {
    /// Cached connectivity status (set to false after a connectivity failure).
    /// This provides a fast path to skip the provider when offline.
    connectivity_ok: AtomicBool,
}

impl Default for GttsProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for GttsProvider {
    fn clone(&self) -> Self {
        Self {
            connectivity_ok: AtomicBool::new(self.connectivity_ok.load(Ordering::Relaxed)),
        }
    }
}

impl GttsProvider {
    /// Provider name constant for error messages.
    const PROVIDER_NAME: &'static str = "gtts-cli";

    /// Create a new gTTS provider.
    pub fn new() -> Self {
        Self {
            connectivity_ok: AtomicBool::new(true),
        }
    }

    /// Get the binary path for gtts-cli.
    fn binary_path() -> PathBuf {
        which::which("gtts-cli").unwrap_or_else(|_| PathBuf::from("gtts-cli"))
    }

    /// Check if the gtts-cli binary exists.
    fn binary_exists() -> bool {
        which::which("gtts-cli").is_ok()
    }

    /// Check network connectivity by making a HEAD request to Google.
    ///
    /// This is a lightweight check to verify internet is available before
    /// attempting to use gTTS (which requires internet).
    async fn check_connectivity() -> bool {
        // Use a simple TCP connection check to Google's TTS endpoint
        // We don't want to add reqwest as a dependency just for this check,
        // so we do a simple DNS lookup + TCP connect
        use tokio::net::TcpStream;
        use tokio::time::{timeout, Duration};

        // Try to connect to Google's servers with a short timeout
        let connect_future = TcpStream::connect("translate.google.com:443");
        timeout(Duration::from_secs(2), connect_future)
            .await
            .is_ok()
    }

    /// Resolve the language code from config.
    fn resolve_language(config: &TtsConfig) -> &str {
        // If a specific voice is requested, use it as the language code
        if let Some(voice) = &config.requested_voice {
            return voice.as_str();
        }

        // Otherwise use the configured language
        config.language.code_prefix()
    }

    /// Parse a single line of `gtts-cli --all` output into a Voice.
    ///
    /// The format is:
    /// ```text
    ///  af: Afrikaans
    ///  ar: Arabic
    /// ```
    ///
    /// Returns `None` for lines that cannot be parsed.
    fn parse_voice_line(line: &str) -> Option<Voice> {
        let line = line.trim();
        if line.is_empty() {
            return None;
        }

        // Split on ':'
        let parts: Vec<&str> = line.splitn(2, ':').collect();
        if parts.len() != 2 {
            return None;
        }

        let lang_code = parts[0].trim();
        let lang_name = parts[1].trim();

        if lang_code.is_empty() || lang_name.is_empty() {
            return None;
        }

        // Determine language
        let language = if lang_code == "en" || lang_code.starts_with("en-") {
            Language::English
        } else {
            Language::Custom(lang_code.to_string())
        };

        Some(
            Voice::new(lang_name)
                .with_gender(Gender::Any) // gTTS doesn't distinguish gender
                .with_quality(VoiceQuality::Good) // Google TTS is good quality
                .with_language(language)
                .with_identifier(lang_code), // Use language code as identifier for gTTS
        )
    }
}

impl TtsExecutor for GttsProvider {
    async fn speak(&self, text: &str, config: &TtsConfig) -> Result<(), TtsError> {
        // Check cached connectivity status for fast fail
        if !self.connectivity_ok.load(Ordering::Relaxed) {
            return Err(TtsError::ProviderFailed {
                provider: Self::PROVIDER_NAME.into(),
                message: "Network connectivity unavailable (cached)".into(),
            });
        }

        let lang = Self::resolve_language(config);

        // Create temp file for audio output
        let temp_file = NamedTempFile::with_suffix(format!(".{}", AudioFormat::Mp3.extension()))
            .map_err(|e| TtsError::TempFileError { source: e })?;

        let output_path = temp_file.path().to_string_lossy().to_string();

        debug!(
            provider = Self::PROVIDER_NAME,
            lang = lang,
            output = %output_path,
            "Generating speech with gTTS"
        );

        // Run gtts-cli to generate audio
        let output = tokio::process::Command::new(Self::binary_path())
            .arg("--output")
            .arg(&output_path)
            .arg("--lang")
            .arg(lang)
            .arg(text)
            .output()
            .await
            .map_err(|e| TtsError::ProcessSpawnFailed {
                provider: Self::PROVIDER_NAME.into(),
                source: e,
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);

            // Check for connectivity-related errors
            if stderr.contains("Connection") || stderr.contains("Network") || stderr.contains("Timeout") {
                self.connectivity_ok.store(false, Ordering::Relaxed);
            }

            return Err(TtsError::ProcessFailed {
                provider: Self::PROVIDER_NAME.into(),
                stderr: stderr.to_string(),
            });
        }

        // Play the generated audio
        #[cfg(feature = "playa")]
        {
            crate::playback::play_audio_file(temp_file.path(), AudioFormat::Mp3, config).await
        }
        #[cfg(not(feature = "playa"))]
        {
            // Playback requires the playa feature
            let _ = temp_file;
            Err(TtsError::NoAudioPlayer)
        }
    }

    /// Check if the gTTS provider is ready.
    ///
    /// Returns `true` if gtts-cli binary exists AND internet is available.
    async fn is_ready(&self) -> bool {
        if !Self::binary_exists() {
            return false;
        }

        // Check connectivity and cache the result
        let is_connected = Self::check_connectivity().await;
        self.connectivity_ok.store(is_connected, Ordering::Relaxed);
        is_connected
    }

    /// Get provider information.
    fn info(&self) -> &str {
        "gTTS - Google Text-to-Speech via Python CLI (requires internet)"
    }

    async fn speak_with_result(
        &self,
        text: &str,
        config: &TtsConfig,
    ) -> Result<SpeakResult, TtsError> {
        // Resolve language code
        let lang_code = Self::resolve_language(config);

        // Try to get the voice from the voice list for better metadata
        let voice = if let Ok(voices) = self.list_voices().await {
            // Find the voice matching this language code
            voices
                .iter()
                .find(|v| v.identifier.as_deref() == Some(lang_code))
                .cloned()
                .unwrap_or_else(|| {
                    Voice::new(lang_code)
                        .with_gender(Gender::Any)
                        .with_quality(VoiceQuality::Good)
                        .with_language(config.language.clone())
                        .with_identifier(lang_code)
                })
        } else {
            // Fallback if voice list unavailable
            Voice::new(lang_code)
                .with_gender(Gender::Any)
                .with_quality(VoiceQuality::Good)
                .with_language(config.language.clone())
                .with_identifier(lang_code)
        };

        // Call speak
        self.speak(text, config).await?;

        // Return the result
        Ok(SpeakResult::new(
            TtsProvider::Host(HostTtsProvider::Gtts),
            voice,
        ))
    }
}

impl TtsVoiceInventory for GttsProvider {
    /// List all available languages from gTTS.
    ///
    /// Parses the output of `gtts-cli --all` to extract language metadata.
    /// Each language becomes a Voice with `VoiceQuality::Good`.
    async fn list_voices(&self) -> Result<Vec<Voice>, TtsError> {
        // Run `gtts-cli --all` to list languages
        let output = tokio::process::Command::new(Self::binary_path())
            .arg("--all")
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
            "Enumerated gTTS languages"
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
    fn test_gtts_provider_default() {
        let provider = GttsProvider::default();
        assert!(provider.connectivity_ok.load(Ordering::Relaxed));
    }

    #[test]
    fn test_gtts_provider_clone() {
        let provider = GttsProvider::new();
        provider.connectivity_ok.store(false, Ordering::Relaxed);

        let cloned = provider.clone();
        assert!(!cloned.connectivity_ok.load(Ordering::Relaxed));
    }

    #[test]
    fn test_resolve_language_default() {
        let config = TtsConfig::default();
        assert_eq!(GttsProvider::resolve_language(&config), "en");
    }

    #[test]
    fn test_resolve_language_explicit_voice() {
        let config = TtsConfig::new().with_voice("fr");
        assert_eq!(GttsProvider::resolve_language(&config), "fr");
    }

    #[test]
    fn test_resolve_language_custom() {
        let config = TtsConfig::new().with_language(Language::Custom("de".into()));
        assert_eq!(GttsProvider::resolve_language(&config), "de");
    }

    #[test]
    fn test_info() {
        let provider = GttsProvider::new();
        assert!(provider.info().contains("gTTS"));
        assert!(provider.info().contains("Google"));
        assert!(provider.info().contains("internet"));
    }

    // ========================================================================
    // Voice line parsing tests
    // ========================================================================

    #[test]
    fn test_parse_voice_line_afrikaans() {
        let line = " af: Afrikaans";
        let voice = GttsProvider::parse_voice_line(line).unwrap();

        assert_eq!(voice.name, "Afrikaans");
        assert_eq!(voice.gender, Gender::Any);
        assert_eq!(voice.quality, VoiceQuality::Good);
        assert_eq!(voice.languages, vec![Language::Custom("af".into())]);
        assert_eq!(voice.identifier, Some("af".into()));
    }

    #[test]
    fn test_parse_voice_line_english() {
        let line = " en: English";
        let voice = GttsProvider::parse_voice_line(line).unwrap();

        assert_eq!(voice.name, "English");
        assert_eq!(voice.languages, vec![Language::English]);
        assert_eq!(voice.identifier, Some("en".into()));
    }

    #[test]
    fn test_parse_voice_line_english_variant() {
        let line = " en-au: English (Australia)";
        let voice = GttsProvider::parse_voice_line(line).unwrap();

        assert_eq!(voice.name, "English (Australia)");
        assert_eq!(voice.languages, vec![Language::English]);
        assert_eq!(voice.identifier, Some("en-au".into()));
    }

    #[test]
    fn test_parse_voice_line_french() {
        let line = " fr: French";
        let voice = GttsProvider::parse_voice_line(line).unwrap();

        assert_eq!(voice.name, "French");
        assert_eq!(voice.languages, vec![Language::Custom("fr".into())]);
    }

    #[test]
    fn test_parse_voice_line_chinese() {
        let line = " zh-CN: Chinese (Simplified)";
        let voice = GttsProvider::parse_voice_line(line).unwrap();

        assert_eq!(voice.name, "Chinese (Simplified)");
        assert_eq!(voice.languages, vec![Language::Custom("zh-CN".into())]);
        assert_eq!(voice.identifier, Some("zh-CN".into()));
    }

    #[test]
    fn test_parse_voice_line_all_good_quality() {
        let lines = [
            " de: German",
            " es: Spanish",
            " ja: Japanese",
            " ko: Korean",
        ];

        for line in lines {
            let voice = GttsProvider::parse_voice_line(line).unwrap();
            assert_eq!(
                voice.quality,
                VoiceQuality::Good,
                "Voice {} should have Good quality",
                voice.name
            );
        }
    }

    #[test]
    fn test_parse_voice_line_all_gender_any() {
        let lines = [
            " de: German",
            " es: Spanish",
            " fr: French",
        ];

        for line in lines {
            let voice = GttsProvider::parse_voice_line(line).unwrap();
            assert_eq!(
                voice.gender,
                Gender::Any,
                "Voice {} should have Any gender (gTTS doesn't distinguish)",
                voice.name
            );
        }
    }

    #[test]
    fn test_parse_voice_line_empty() {
        assert!(GttsProvider::parse_voice_line("").is_none());
        assert!(GttsProvider::parse_voice_line("   ").is_none());
    }

    #[test]
    fn test_parse_voice_line_no_colon() {
        assert!(GttsProvider::parse_voice_line("invalid line without colon").is_none());
    }

    #[test]
    fn test_parse_voice_line_empty_parts() {
        assert!(GttsProvider::parse_voice_line(":").is_none());
        assert!(GttsProvider::parse_voice_line("en:").is_none());
        assert!(GttsProvider::parse_voice_line(": English").is_none());
    }

    // ========================================================================
    // Voice parsing from sample output
    // ========================================================================

    const GTTS_CLI_ALL_SAMPLE: &str = "\
 af: Afrikaans
 ar: Arabic
 bg: Bulgarian
 bn: Bengali
 ca: Catalan
 cs: Czech
 cy: Welsh
 da: Danish
 de: German
 el: Greek
 en: English
 en-au: English (Australia)
 en-ca: English (Canada)
 en-co.uk: English (co.uk)
 en-gb: English (UK)
 en-gh: English (Ghana)
 en-ie: English (Ireland)
 en-in: English (India)
 en-ke: English (Kenya)
 en-ng: English (Nigeria)
 en-nz: English (New Zealand)
 en-ph: English (Philippines)
 en-sg: English (Singapore)
 en-tz: English (Tanzania)
 en-us: English (US)
 en-za: English (South Africa)
 es: Spanish
 es-es: Spanish (Spain)
 es-us: Spanish (United States)
 et: Estonian
 fi: Finnish
 fr: French
 fr-ca: French (Canada)
 fr-fr: French (France)
 gu: Gujarati
 hi: Hindi
 hr: Croatian
 hu: Hungarian
 id: Indonesian
 is: Icelandic
 it: Italian
 ja: Japanese
 jw: Javanese
 kn: Kannada
 ko: Korean
 la: Latin
 lv: Latvian
 ml: Malayalam
 mr: Marathi
 ms: Malay
 my: Myanmar (Burmese)
 ne: Nepali
 nl: Dutch
 no: Norwegian
 pl: Polish
 pt: Portuguese
 pt-br: Portuguese (Brazil)
 ro: Romanian
 ru: Russian
 si: Sinhala
 sk: Slovak
 sq: Albanian
 sr: Serbian
 su: Sundanese
 sv: Swedish
 sw: Swahili
 ta: Tamil
 te: Telugu
 th: Thai
 tl: Filipino
 tr: Turkish
 uk: Ukrainian
 ur: Urdu
 vi: Vietnamese
 zh-CN: Chinese (Simplified)
 zh-TW: Chinese (Mandarin/Taiwan)
";

    #[test]
    fn test_parse_gtts_voices_sample() {
        let voices: Vec<Voice> = GTTS_CLI_ALL_SAMPLE
            .lines()
            .filter_map(GttsProvider::parse_voice_line)
            .collect();

        // Should parse all lines
        assert!(!voices.is_empty());
        assert!(voices.len() >= 70, "Expected at least 70 voices, got {}", voices.len());
    }

    #[test]
    fn test_parse_gtts_voices_english_variants() {
        let voices: Vec<Voice> = GTTS_CLI_ALL_SAMPLE
            .lines()
            .filter_map(GttsProvider::parse_voice_line)
            .collect();

        // Count English voices (should be many variants)
        let english_voices: Vec<_> = voices
            .iter()
            .filter(|v| v.languages.contains(&Language::English))
            .collect();

        assert!(
            english_voices.len() >= 10,
            "Expected at least 10 English variants, got {}",
            english_voices.len()
        );
    }

    #[test]
    fn test_parse_gtts_voices_all_have_identifier() {
        let voices: Vec<Voice> = GTTS_CLI_ALL_SAMPLE
            .lines()
            .filter_map(GttsProvider::parse_voice_line)
            .collect();

        for voice in &voices {
            assert!(
                voice.identifier.is_some(),
                "Voice {} should have an identifier",
                voice.name
            );
        }
    }

    // ========================================================================
    // Integration tests (require gtts-cli to be installed)
    // ========================================================================

    #[tokio::test]
    async fn test_is_ready_without_binary() {
        // Create a provider - if gtts-cli is not installed, is_ready should return false
        let provider = GttsProvider::new();

        // This test just verifies the function doesn't panic
        // The result depends on whether gtts-cli is installed
        let _is_ready = provider.is_ready().await;
    }

    #[tokio::test]
    #[ignore] // Only run manually when gtts-cli is installed
    async fn test_list_voices_integration() {
        let provider = GttsProvider::new();

        if !GttsProvider::binary_exists() {
            eprintln!("Skipping test: gtts-cli not installed");
            return;
        }

        let voices = provider.list_voices().await.unwrap();

        // Should have voices
        assert!(!voices.is_empty(), "Expected at least one voice");

        // All should be good quality
        for voice in &voices {
            assert_eq!(voice.quality, VoiceQuality::Good);
        }

        // Should have an English voice
        assert!(
            voices.iter().any(|v| v.languages.contains(&Language::English)),
            "Expected at least one English voice"
        );

        println!("Found {} voices", voices.len());
        for voice in voices.iter().take(10) {
            println!(
                "  - {} ({:?}): {:?}",
                voice.name, voice.identifier, voice.languages
            );
        }
    }

    #[tokio::test]
    #[ignore] // Produces audio and requires internet - run manually
    async fn test_gtts_provider_speaks() {
        let provider = GttsProvider::new();

        if !provider.is_ready().await {
            eprintln!("Skipping test: gtts-cli not ready (not installed or no internet)");
            return;
        }

        let config = TtsConfig::default();
        let result = provider
            .speak("Hello from the gTTS provider test.", &config)
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[ignore] // Requires internet - run manually
    async fn test_check_connectivity() {
        let is_connected = GttsProvider::check_connectivity().await;
        println!("Connectivity check result: {}", is_connected);
        // Can't assert true/false since it depends on network state
    }
}
