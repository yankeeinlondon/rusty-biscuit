//! Echogarden TTS provider.
//!
//! Uses the Echogarden speech processing engine for text-to-speech.
//! Supports multiple TTS backends, with focus on `vits` and `kokoro` engines.
//!
//! Echogarden is an npm package: <https://github.com/echogarden-project/echogarden>

use std::process::Stdio;

use tracing::{debug, trace};

use crate::errors::TtsError;
use crate::playback::play_audio_file;
use crate::traits::{TtsExecutor, TtsVoiceInventory};
use crate::types::{Gender, HostTtsProvider, Language, SpeakResult, TtsConfig, TtsProvider, Voice, VoiceQuality};

/// Echogarden TTS engine identifier.
///
/// We focus on the high-quality local engines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EchogardenEngine {
    /// VITS - high-quality end-to-end neural speech synthesis.
    Vits,
    /// Kokoro - neural speech synthesis based on StyleTTS 2.
    Kokoro,
}

impl EchogardenEngine {
    /// Returns the CLI identifier for this engine.
    pub fn as_str(&self) -> &'static str {
        match self {
            EchogardenEngine::Vits => "vits",
            EchogardenEngine::Kokoro => "kokoro",
        }
    }

    /// Returns the voice quality for this engine.
    pub fn quality(&self) -> VoiceQuality {
        match self {
            EchogardenEngine::Kokoro => VoiceQuality::Excellent,
            EchogardenEngine::Vits => VoiceQuality::Good,
        }
    }

    /// Parse engine from string identifier.
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "vits" => Some(EchogardenEngine::Vits),
            "kokoro" => Some(EchogardenEngine::Kokoro),
            _ => None,
        }
    }

    /// All supported engines.
    pub fn all() -> &'static [EchogardenEngine] {
        &[EchogardenEngine::Kokoro, EchogardenEngine::Vits]
    }
}

/// Echogarden TTS provider.
///
/// This provider uses the `echogarden` npm package for TTS.
///
/// ## Engine Selection
///
/// By default, uses the Kokoro engine for best quality. Can be configured
/// to use VITS for broader language support.
///
/// ## Examples
///
/// ```ignore
/// use biscuit_speaks::providers::host::EchogardenProvider;
/// use biscuit_speaks::{TtsExecutor, TtsConfig};
///
/// let provider = EchogardenProvider::new();
/// provider.speak("Hello, world!", &TtsConfig::default()).await?;
/// ```
#[derive(Debug, Clone)]
pub struct EchogardenProvider {
    /// The engine to use for synthesis.
    engine: EchogardenEngine,
}

impl Default for EchogardenProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl EchogardenProvider {
    /// Provider name constant for error messages.
    const PROVIDER_NAME: &'static str = "echogarden";

    /// Create a new EchogardenProvider with default Kokoro engine.
    pub fn new() -> Self {
        Self {
            engine: EchogardenEngine::Kokoro,
        }
    }

    /// Create a provider with a specific engine.
    pub fn with_engine(engine: EchogardenEngine) -> Self {
        Self { engine }
    }

    /// Get the current engine.
    pub fn engine(&self) -> EchogardenEngine {
        self.engine
    }

    /// Check if the `echogarden` command exists on the system.
    async fn echogarden_binary_exists() -> bool {
        which::which("echogarden").is_ok()
    }

    /// Resolve the voice to use based on config.
    fn resolve_voice(&self, config: &TtsConfig) -> Option<String> {
        // If a specific voice is requested, use it directly
        if let Some(voice) = &config.requested_voice {
            return Some(voice.clone());
        }

        // For Kokoro, provide gender-based defaults
        if self.engine == EchogardenEngine::Kokoro {
            return Some(
                match config.gender {
                    Gender::Male => "Michael",
                    Gender::Female => "Heart",
                    Gender::Any => "Heart", // Default to Heart for best quality
                }
                .to_string(),
            );
        }

        // For VITS, let echogarden pick the default
        None
    }

    /// Select the best voice from a list based on config constraints.
    fn select_best_voice(voices: &[&Voice], config: &TtsConfig) -> Option<Voice> {
        let mut candidates: Vec<&Voice> = voices.iter().copied().collect();

        // Filter by language if specified
        let target_language = &config.language;
        let filtered_by_lang: Vec<&Voice> = candidates
            .iter()
            .filter(|v| {
                v.languages.iter().any(|lang| {
                    match (lang, target_language) {
                        (Language::English, Language::English) => true,
                        (Language::Custom(a), Language::Custom(b)) => {
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
            })
            .copied()
            .collect();

        if !filtered_by_lang.is_empty() {
            candidates = filtered_by_lang;
        }

        // Filter by gender if specified (not Any)
        if config.gender != Gender::Any {
            let gender_matches: Vec<&Voice> = candidates
                .iter()
                .filter(|v| v.gender == config.gender)
                .copied()
                .collect();

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
}

impl TtsExecutor for EchogardenProvider {
    async fn speak(&self, text: &str, config: &TtsConfig) -> Result<(), TtsError> {
        // Create a temporary directory to hold the output file
        // Note: We can't use NamedTempFile because echogarden doesn't overwrite
        // existing files properly - it produces empty output when the file exists.
        let temp_dir = tempfile::tempdir().map_err(|e| TtsError::TempFileError { source: e })?;
        let output_path = temp_dir.path().join("echogarden_output.wav");

        let mut cmd = tokio::process::Command::new("echogarden");
        cmd.arg("speak");
        cmd.arg(text);
        cmd.arg(&output_path);

        // Engine selection
        cmd.arg(format!("--engine={}", self.engine.as_str()));

        // Voice selection
        if let Some(voice) = self.resolve_voice(config) {
            cmd.arg(format!("--voice={}", voice));
        }

        cmd.stdout(Stdio::null());
        cmd.stderr(Stdio::piped());

        debug!(
            engine = self.engine.as_str(),
            text_len = text.len(),
            "Synthesizing with echogarden"
        );

        let output = cmd
            .output()
            .await
            .map_err(|e| TtsError::ProcessSpawnFailed {
                provider: Self::PROVIDER_NAME.into(),
                source: e,
            })?;

        if !output.status.success() {
            return Err(TtsError::ProcessFailed {
                provider: Self::PROVIDER_NAME.into(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            });
        }

        // Verify the file was created and has content
        let metadata = std::fs::metadata(&output_path).map_err(|e| TtsError::TempFileError {
            source: std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Echogarden output file not found: {}", e),
            ),
        })?;

        if metadata.len() == 0 {
            return Err(TtsError::ProcessFailed {
                provider: Self::PROVIDER_NAME.into(),
                stderr: "Echogarden produced an empty audio file".into(),
            });
        }

        debug!(
            file_size = metadata.len(),
            path = %output_path.display(),
            "Echogarden synthesis complete, playing audio"
        );

        // Play the generated audio file
        play_audio_file(&output_path).await?;

        Ok(())
    }

    async fn is_ready(&self) -> bool {
        Self::echogarden_binary_exists().await
    }

    fn info(&self) -> &str {
        "Echogarden - Speech processing engine with multiple high-quality TTS backends (VITS, Kokoro)"
    }

    async fn speak_with_result(
        &self,
        text: &str,
        config: &TtsConfig,
    ) -> Result<SpeakResult, TtsError> {
        // If a specific voice is requested, use it directly
        if let Some(voice_name) = &config.requested_voice {
            self.speak(text, config).await?;

            // Try to find the voice in the list for full metadata
            if let Ok(voices) = self.list_voices().await {
                if let Some(voice) = voices.iter().find(|v| v.name == *voice_name) {
                    return Ok(SpeakResult::new(
                        TtsProvider::Host(HostTtsProvider::EchoGarden),
                        voice.clone(),
                    ));
                }
            }

            // Fallback with constructed metadata
            let voice = Voice::new(voice_name)
                .with_gender(config.gender)
                .with_quality(self.engine.quality())
                .with_language(config.language.clone())
                .with_identifier(format!("{}:{}", self.engine.as_str(), voice_name));

            return Ok(SpeakResult::new(
                TtsProvider::Host(HostTtsProvider::EchoGarden),
                voice,
            ));
        }

        // No specific voice requested - select from voice list
        let voices = self.list_voices().await?;

        // Filter voices by engine
        let engine_prefix = format!("{}:", self.engine.as_str());
        let engine_voices: Vec<&Voice> = voices
            .iter()
            .filter(|v| v.identifier.as_ref().map(|i| i.starts_with(&engine_prefix)).unwrap_or(false))
            .collect();

        // Select best voice based on constraints (gender, language, quality)
        let selected_voice = Self::select_best_voice(&engine_voices, config).ok_or_else(|| {
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
            TtsProvider::Host(HostTtsProvider::EchoGarden),
            selected_voice,
        ))
    }
}

impl TtsVoiceInventory for EchogardenProvider {
    async fn list_voices(&self) -> Result<Vec<Voice>, TtsError> {
        let mut all_voices = Vec::new();

        // List voices for each supported engine
        for engine in EchogardenEngine::all() {
            match list_voices_for_engine(*engine).await {
                Ok(voices) => {
                    // Process VITS voices: filter low quality, set correct VoiceQuality
                    let voices = if *engine == EchogardenEngine::Vits {
                        process_vits_voices(voices)
                    } else {
                        voices
                    };
                    all_voices.extend(voices);
                }
                Err(e) => {
                    debug!(
                        engine = engine.as_str(),
                        error = %e,
                        "Failed to list voices for engine"
                    );
                }
            }
        }

        debug!(
            provider = Self::PROVIDER_NAME,
            voice_count = all_voices.len(),
            "Enumerated echogarden voices"
        );

        Ok(all_voices)
    }
}

/// List voices for a specific echogarden engine.
///
/// Note: Echogarden outputs to stderr, not stdout (unusual behavior).
async fn list_voices_for_engine(engine: EchogardenEngine) -> Result<Vec<Voice>, TtsError> {
    let output = tokio::process::Command::new("echogarden")
        .arg("list-voices")
        .arg(engine.as_str())
        .output()
        .await
        .map_err(|e| TtsError::VoiceEnumerationFailed {
            provider: format!("echogarden/{}", engine.as_str()),
            message: format!("Failed to run 'echogarden list-voices {}': {}", engine.as_str(), e),
        })?;

    if !output.status.success() {
        return Err(TtsError::VoiceEnumerationFailed {
            provider: format!("echogarden/{}", engine.as_str()),
            message: format!(
                "Command failed with status {}: {}",
                output.status,
                String::from_utf8_lossy(&output.stderr)
            ),
        });
    }

    // Echogarden outputs to stderr, not stdout (quirky behavior)
    let stderr = String::from_utf8_lossy(&output.stderr);
    parse_echogarden_voices(&stderr, engine)
}

/// Parse the output of `echogarden list-voices <engine>`.
///
/// The output format is:
/// ```text
/// Echogarden v2.10.1
///
/// Identifier: Heart
/// Languages: American English (en-US), English (en)
/// Gender: female
///
/// Identifier: Michael
/// Languages: American English (en-US), English (en)
/// Gender: male
/// ```
fn parse_echogarden_voices(output: &str, engine: EchogardenEngine) -> Result<Vec<Voice>, TtsError> {
    let mut voices = Vec::new();
    let mut current_identifier: Option<String> = None;
    let mut current_languages: Vec<Language> = Vec::new();
    let mut current_gender = Gender::Any;

    for line in output.lines() {
        let line = line.trim();

        if line.starts_with("Identifier:") {
            // Save previous voice if we have one
            if let Some(identifier) = current_identifier.take() {
                let voice = Voice::new(&identifier)
                    .with_identifier(format!("{}:{}", engine.as_str(), identifier))
                    .with_gender(current_gender)
                    .with_quality(engine.quality())
                    .with_languages(current_languages.clone());
                voices.push(voice);
            }

            // Start new voice
            current_identifier = Some(line.trim_start_matches("Identifier:").trim().to_string());
            current_languages.clear();
            current_gender = Gender::Any;
        } else if line.starts_with("Languages:") {
            let langs_str = line.trim_start_matches("Languages:").trim();
            current_languages = parse_languages(langs_str);
        } else if line.starts_with("Gender:") {
            let gender_str = line.trim_start_matches("Gender:").trim().to_lowercase();
            current_gender = match gender_str.as_str() {
                "male" => Gender::Male,
                "female" => Gender::Female,
                _ => Gender::Any,
            };
        }
    }

    // Don't forget the last voice
    if let Some(identifier) = current_identifier {
        let voice = Voice::new(&identifier)
            .with_identifier(format!("{}:{}", engine.as_str(), identifier))
            .with_gender(current_gender)
            .with_quality(engine.quality())
            .with_languages(current_languages);
        voices.push(voice);
    }

    trace!(
        engine = engine.as_str(),
        voice_count = voices.len(),
        "Parsed echogarden voices"
    );

    Ok(voices)
}

/// Quality tier extracted from VITS voice names.
///
/// VITS voice names follow the pattern: `{locale}-{name}-{quality}`
/// e.g., `nl_BE-nathalie-medium`, `nl_BE-nathalie-x_low`
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum VitsQualityTier {
    /// Highest quality
    High = 3,
    /// Medium quality
    Medium = 2,
    /// Lower quality - filtered out
    Low = 1,
    /// Lowest quality - filtered out
    XLow = 0,
}

impl VitsQualityTier {
    /// Parse the quality tier from a VITS voice name suffix.
    fn from_suffix(name: &str) -> Option<Self> {
        if name.ends_with("-high") {
            Some(VitsQualityTier::High)
        } else if name.ends_with("-medium") {
            Some(VitsQualityTier::Medium)
        } else if name.ends_with("-x_low") {
            Some(VitsQualityTier::XLow)
        } else if name.ends_with("-low") {
            Some(VitsQualityTier::Low)
        } else {
            None
        }
    }

    /// Extract the base name (without quality suffix) from a VITS voice name.
    fn base_name(name: &str) -> &str {
        if let Some(stripped) = name.strip_suffix("-high") {
            stripped
        } else if let Some(stripped) = name.strip_suffix("-medium") {
            stripped
        } else if let Some(stripped) = name.strip_suffix("-x_low") {
            stripped
        } else if let Some(stripped) = name.strip_suffix("-low") {
            stripped
        } else {
            name
        }
    }

    /// Convert to VoiceQuality.
    ///
    /// Mapping:
    /// - `high` → `VoiceQuality::Good`
    /// - `medium` → `VoiceQuality::Moderate`
    /// - `low` / `x_low` → `VoiceQuality::Low`
    fn to_voice_quality(self) -> VoiceQuality {
        match self {
            VitsQualityTier::High => VoiceQuality::Good,
            VitsQualityTier::Medium => VoiceQuality::Moderate,
            VitsQualityTier::Low | VitsQualityTier::XLow => VoiceQuality::Low,
        }
    }

    /// Returns true if this tier should be filtered out.
    ///
    /// Low and x_low quality voices are filtered out entirely.
    fn should_filter(self) -> bool {
        matches!(self, VitsQualityTier::Low | VitsQualityTier::XLow)
    }
}

/// Process VITS voices: filter low quality and set appropriate VoiceQuality.
///
/// This function:
/// 1. Filters out `low` and `x_low` quality voices entirely
/// 2. Sets the correct VoiceQuality based on the tier (high→Good, medium→Moderate)
/// 3. Deduplicates voices with the same base name, keeping only the highest quality
///
/// For example, given:
/// - `nl_BE-nathalie-medium`
/// - `nl_BE-nathalie-x_low`
///
/// Only `nl_BE-nathalie-medium` will be kept (with VoiceQuality::Moderate).
fn process_vits_voices(voices: Vec<Voice>) -> Vec<Voice> {
    use std::collections::HashMap;

    // Group voices by base name, keeping track of the best quality
    let mut best_voices: HashMap<String, (VitsQualityTier, Voice)> = HashMap::new();

    for mut voice in voices {
        let tier = VitsQualityTier::from_suffix(&voice.name);

        // If no quality suffix found, keep the voice as-is (e.g., Kokoro voices)
        let Some(tier) = tier else {
            let key = voice.name.clone();
            best_voices
                .entry(key)
                .or_insert((VitsQualityTier::High, voice));
            continue;
        };

        // Set the VoiceQuality based on the tier
        voice.quality = tier.to_voice_quality();

        let base = VitsQualityTier::base_name(&voice.name).to_string();

        match best_voices.get(&base) {
            Some((existing_tier, _)) if *existing_tier >= tier => {
                // Existing voice is same or better quality, skip
            }
            _ => {
                // New voice or better quality, replace
                best_voices.insert(base, (tier, voice));
            }
        }
    }

    // Extract voices, filtering out low quality ones
    best_voices
        .into_values()
        .filter(|(tier, _)| !tier.should_filter())
        .map(|(_, voice)| voice)
        .collect()
}

/// Parse the Languages field from echogarden output.
///
/// Format: "American English (en-US), English (en)"
fn parse_languages(langs_str: &str) -> Vec<Language> {
    let mut languages = Vec::new();

    for part in langs_str.split(',') {
        let part = part.trim();

        // Extract the language code from parentheses
        if let (Some(start), Some(end)) = (part.rfind('('), part.rfind(')')) {
            let code = &part[start + 1..end];
            // Map common codes
            if code.starts_with("en") {
                if !languages.contains(&Language::English) {
                    languages.push(Language::English);
                }
            } else if !languages.iter().any(|l| matches!(l, Language::Custom(c) if c == code)) {
                languages.push(Language::Custom(code.to_string()));
            }
        }
    }

    // Ensure at least one language
    if languages.is_empty() {
        languages.push(Language::English);
    }

    languages
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
    fn test_echogarden_provider_default() {
        let provider = EchogardenProvider::default();
        assert_eq!(provider.engine, EchogardenEngine::Kokoro);
    }

    #[test]
    fn test_echogarden_provider_with_engine() {
        let provider = EchogardenProvider::with_engine(EchogardenEngine::Vits);
        assert_eq!(provider.engine, EchogardenEngine::Vits);
    }

    #[test]
    fn test_engine_as_str() {
        assert_eq!(EchogardenEngine::Vits.as_str(), "vits");
        assert_eq!(EchogardenEngine::Kokoro.as_str(), "kokoro");
    }

    #[test]
    fn test_engine_parse() {
        assert_eq!(
            EchogardenEngine::parse("vits"),
            Some(EchogardenEngine::Vits)
        );
        assert_eq!(
            EchogardenEngine::parse("KOKORO"),
            Some(EchogardenEngine::Kokoro)
        );
        assert_eq!(EchogardenEngine::parse("unknown"), None);
    }

    #[test]
    fn test_engine_quality() {
        assert_eq!(EchogardenEngine::Kokoro.quality(), VoiceQuality::Excellent);
        assert_eq!(EchogardenEngine::Vits.quality(), VoiceQuality::Good);
    }

    #[test]
    fn test_info() {
        let provider = EchogardenProvider::new();
        assert!(provider.info().contains("Echogarden"));
        assert!(provider.info().contains("VITS"));
        assert!(provider.info().contains("Kokoro"));
    }

    // ========================================================================
    // Voice resolution tests
    // ========================================================================

    #[test]
    fn test_resolve_voice_explicit() {
        let provider = EchogardenProvider::new();
        let config = TtsConfig::new().with_voice("CustomVoice");
        assert_eq!(provider.resolve_voice(&config), Some("CustomVoice".into()));
    }

    #[test]
    fn test_resolve_voice_kokoro_male() {
        let provider = EchogardenProvider::with_engine(EchogardenEngine::Kokoro);
        let config = TtsConfig::new().with_gender(Gender::Male);
        assert_eq!(provider.resolve_voice(&config), Some("Michael".into()));
    }

    #[test]
    fn test_resolve_voice_kokoro_female() {
        let provider = EchogardenProvider::with_engine(EchogardenEngine::Kokoro);
        let config = TtsConfig::new().with_gender(Gender::Female);
        assert_eq!(provider.resolve_voice(&config), Some("Heart".into()));
    }

    #[test]
    fn test_resolve_voice_kokoro_any() {
        let provider = EchogardenProvider::with_engine(EchogardenEngine::Kokoro);
        let config = TtsConfig::new();
        assert_eq!(provider.resolve_voice(&config), Some("Heart".into()));
    }

    #[test]
    fn test_resolve_voice_vits_default() {
        let provider = EchogardenProvider::with_engine(EchogardenEngine::Vits);
        let config = TtsConfig::new();
        assert_eq!(provider.resolve_voice(&config), None);
    }

    // ========================================================================
    // Voice parsing tests
    // ========================================================================

    const ECHOGARDEN_KOKORO_SAMPLE: &str = r#"Echogarden v2.10.1

Identifier: Heart
Languages: American English (en-US), English (en)
Gender: female

Identifier: Bella
Languages: American English (en-US), English (en)
Gender: female

Identifier: Michael
Languages: American English (en-US), English (en)
Gender: male

Identifier: Emma
Languages: British English (en-GB), English (en)
Gender: female
"#;

    const ECHOGARDEN_VITS_SAMPLE: &str = r#"Echogarden v2.10.1

Identifier: ar_JO-kareem-low
Languages: Arabic (Jordan) (ar-JO), Arabic (ar)
Gender: male

Identifier: de_DE-thorsten-medium
Languages: German (Germany) (de-DE), German (de)
Gender: male

Identifier: en_US-lessac-medium
Languages: American English (en-US), English (en)
Gender: female
"#;

    #[test]
    fn test_parse_kokoro_voices() {
        let voices = parse_echogarden_voices(ECHOGARDEN_KOKORO_SAMPLE, EchogardenEngine::Kokoro)
            .unwrap();

        assert_eq!(voices.len(), 4);

        let heart = voices.iter().find(|v| v.name == "Heart").unwrap();
        assert_eq!(heart.gender, Gender::Female);
        assert_eq!(heart.quality, VoiceQuality::Excellent);
        assert!(heart.languages.contains(&Language::English));
        assert_eq!(heart.identifier, Some("kokoro:Heart".into()));

        let michael = voices.iter().find(|v| v.name == "Michael").unwrap();
        assert_eq!(michael.gender, Gender::Male);
        assert_eq!(michael.quality, VoiceQuality::Excellent);
    }

    #[test]
    fn test_parse_vits_voices() {
        let voices =
            parse_echogarden_voices(ECHOGARDEN_VITS_SAMPLE, EchogardenEngine::Vits).unwrap();

        assert_eq!(voices.len(), 3);

        let kareem = voices
            .iter()
            .find(|v| v.name == "ar_JO-kareem-low")
            .unwrap();
        assert_eq!(kareem.gender, Gender::Male);
        assert_eq!(kareem.quality, VoiceQuality::Good);
        assert!(kareem
            .languages
            .contains(&Language::Custom("ar-JO".into())));
        assert_eq!(kareem.identifier, Some("vits:ar_JO-kareem-low".into()));

        let thorsten = voices
            .iter()
            .find(|v| v.name == "de_DE-thorsten-medium")
            .unwrap();
        assert_eq!(thorsten.gender, Gender::Male);
        assert!(thorsten
            .languages
            .contains(&Language::Custom("de-DE".into())));
    }

    #[test]
    fn test_parse_empty_output() {
        let voices =
            parse_echogarden_voices("Echogarden v2.10.1\n", EchogardenEngine::Kokoro).unwrap();
        assert!(voices.is_empty());
    }

    #[test]
    fn test_parse_version_only() {
        let voices = parse_echogarden_voices("", EchogardenEngine::Kokoro).unwrap();
        assert!(voices.is_empty());
    }

    // ========================================================================
    // Language parsing tests
    // ========================================================================

    #[test]
    fn test_parse_languages_english_variants() {
        let langs = parse_languages("American English (en-US), English (en)");
        assert_eq!(langs, vec![Language::English]);
    }

    #[test]
    fn test_parse_languages_non_english() {
        let langs = parse_languages("German (Germany) (de-DE), German (de)");
        assert!(langs.contains(&Language::Custom("de-DE".into())));
        assert!(langs.contains(&Language::Custom("de".into())));
    }

    #[test]
    fn test_parse_languages_mixed() {
        let langs = parse_languages("British English (en-GB), English (en)");
        assert_eq!(langs, vec![Language::English]);
    }

    #[test]
    fn test_parse_languages_empty() {
        let langs = parse_languages("");
        assert_eq!(langs, vec![Language::English]); // Default fallback
    }

    #[test]
    fn test_parse_languages_no_parens() {
        let langs = parse_languages("Unknown Language");
        assert_eq!(langs, vec![Language::English]); // Default fallback
    }

    // ========================================================================
    // Engine enumeration tests
    // ========================================================================

    #[test]
    fn test_engine_all() {
        let engines = EchogardenEngine::all();
        assert_eq!(engines.len(), 2);
        assert!(engines.contains(&EchogardenEngine::Kokoro));
        assert!(engines.contains(&EchogardenEngine::Vits));
    }

    // ========================================================================
    // VITS voice deduplication tests
    // ========================================================================

    #[test]
    fn test_vits_quality_tier_from_suffix() {
        assert_eq!(
            VitsQualityTier::from_suffix("en_US-lessac-high"),
            Some(VitsQualityTier::High)
        );
        assert_eq!(
            VitsQualityTier::from_suffix("nl_BE-nathalie-medium"),
            Some(VitsQualityTier::Medium)
        );
        assert_eq!(
            VitsQualityTier::from_suffix("nl_BE-nathalie-low"),
            Some(VitsQualityTier::Low)
        );
        assert_eq!(
            VitsQualityTier::from_suffix("nl_BE-nathalie-x_low"),
            Some(VitsQualityTier::XLow)
        );
        assert_eq!(VitsQualityTier::from_suffix("Heart"), None);
    }

    #[test]
    fn test_vits_quality_tier_base_name() {
        assert_eq!(
            VitsQualityTier::base_name("en_US-lessac-high"),
            "en_US-lessac"
        );
        assert_eq!(
            VitsQualityTier::base_name("nl_BE-nathalie-medium"),
            "nl_BE-nathalie"
        );
        assert_eq!(
            VitsQualityTier::base_name("nl_BE-nathalie-low"),
            "nl_BE-nathalie"
        );
        assert_eq!(
            VitsQualityTier::base_name("nl_BE-nathalie-x_low"),
            "nl_BE-nathalie"
        );
        assert_eq!(VitsQualityTier::base_name("Heart"), "Heart");
    }

    #[test]
    fn test_vits_quality_tier_ordering() {
        assert!(VitsQualityTier::High > VitsQualityTier::Medium);
        assert!(VitsQualityTier::Medium > VitsQualityTier::Low);
        assert!(VitsQualityTier::Low > VitsQualityTier::XLow);
        assert!(VitsQualityTier::High > VitsQualityTier::XLow);
    }

    #[test]
    fn test_vits_quality_tier_to_voice_quality() {
        assert_eq!(
            VitsQualityTier::High.to_voice_quality(),
            VoiceQuality::Good
        );
        assert_eq!(
            VitsQualityTier::Medium.to_voice_quality(),
            VoiceQuality::Moderate
        );
        assert_eq!(
            VitsQualityTier::Low.to_voice_quality(),
            VoiceQuality::Low
        );
        assert_eq!(
            VitsQualityTier::XLow.to_voice_quality(),
            VoiceQuality::Low
        );
    }

    #[test]
    fn test_vits_quality_tier_should_filter() {
        assert!(!VitsQualityTier::High.should_filter());
        assert!(!VitsQualityTier::Medium.should_filter());
        assert!(VitsQualityTier::Low.should_filter());
        assert!(VitsQualityTier::XLow.should_filter());
    }

    #[test]
    fn test_process_vits_voices_filters_low_quality() {
        // Low and x_low voices should be filtered out entirely
        let voices = vec![
            Voice::new("nl_BE-nathalie-medium")
                .with_identifier("vits:nl_BE-nathalie-medium")
                .with_quality(VoiceQuality::Good),
            Voice::new("nl_BE-nathalie-x_low")
                .with_identifier("vits:nl_BE-nathalie-x_low")
                .with_quality(VoiceQuality::Good),
        ];

        let processed = process_vits_voices(voices);
        assert_eq!(processed.len(), 1);
        assert_eq!(processed[0].name, "nl_BE-nathalie-medium");
        // Quality should be set to Moderate for medium tier
        assert_eq!(processed[0].quality, VoiceQuality::Moderate);
    }

    #[test]
    fn test_process_vits_voices_filters_out_low_only_voices() {
        // If only low/x_low versions exist, they should be filtered out entirely
        let voices = vec![
            Voice::new("nl_BE-nathalie-low")
                .with_identifier("vits:nl_BE-nathalie-low")
                .with_quality(VoiceQuality::Good),
            Voice::new("nl_BE-nathalie-x_low")
                .with_identifier("vits:nl_BE-nathalie-x_low")
                .with_quality(VoiceQuality::Good),
        ];

        let processed = process_vits_voices(voices);
        // Both are low quality, should be filtered out
        assert!(processed.is_empty());
    }

    #[test]
    fn test_process_vits_voices_keeps_high_quality() {
        let voices = vec![
            Voice::new("en_US-lessac-high")
                .with_identifier("vits:en_US-lessac-high")
                .with_quality(VoiceQuality::Good),
            Voice::new("en_US-lessac-medium")
                .with_identifier("vits:en_US-lessac-medium")
                .with_quality(VoiceQuality::Good),
        ];

        let processed = process_vits_voices(voices);
        assert_eq!(processed.len(), 1);
        assert_eq!(processed[0].name, "en_US-lessac-high");
        // Quality should be set to Good for high tier
        assert_eq!(processed[0].quality, VoiceQuality::Good);
    }

    #[test]
    fn test_process_vits_voices_multiple_voices() {
        // Simulates real-world scenario with multiple voices
        let voices = vec![
            Voice::new("nl_BE-nathalie-medium").with_quality(VoiceQuality::Good),
            Voice::new("nl_BE-nathalie-x_low").with_quality(VoiceQuality::Good),
            Voice::new("nl_BE-rdh-medium").with_quality(VoiceQuality::Good),
            Voice::new("nl_BE-rdh-x_low").with_quality(VoiceQuality::Good),
            Voice::new("uk_UA-lada-x_low").with_quality(VoiceQuality::Good), // Only x_low - filtered
            Voice::new("de_DE-thorsten-medium").with_quality(VoiceQuality::Good),
            Voice::new("en_US-lessac-high").with_quality(VoiceQuality::Good),
        ];

        let processed = process_vits_voices(voices);

        // Should have 4 voices: nathalie-medium, rdh-medium, thorsten-medium, lessac-high
        // lada-x_low should be filtered out (low quality)
        assert_eq!(processed.len(), 4);

        let names: Vec<&str> = processed.iter().map(|v| v.name.as_str()).collect();
        assert!(names.contains(&"nl_BE-nathalie-medium"));
        assert!(names.contains(&"nl_BE-rdh-medium"));
        assert!(names.contains(&"de_DE-thorsten-medium"));
        assert!(names.contains(&"en_US-lessac-high"));

        // Low quality voices should be filtered out
        assert!(!names.contains(&"uk_UA-lada-x_low"));
        assert!(!names.contains(&"nl_BE-nathalie-x_low"));
        assert!(!names.contains(&"nl_BE-rdh-x_low"));
    }

    #[test]
    fn test_process_vits_voices_preserves_kokoro_voices() {
        // Kokoro voices don't have quality suffixes, they should pass through unchanged
        let voices = vec![
            Voice::new("Heart").with_quality(VoiceQuality::Excellent),
            Voice::new("Michael").with_quality(VoiceQuality::Excellent),
        ];

        let processed = process_vits_voices(voices);
        assert_eq!(processed.len(), 2);

        let names: Vec<&str> = processed.iter().map(|v| v.name.as_str()).collect();
        assert!(names.contains(&"Heart"));
        assert!(names.contains(&"Michael"));

        // Quality should be preserved for Kokoro voices
        for voice in &processed {
            assert_eq!(voice.quality, VoiceQuality::Excellent);
        }
    }

    #[test]
    fn test_process_vits_voices_sets_correct_quality() {
        let voices = vec![
            Voice::new("en_US-lessac-high").with_quality(VoiceQuality::Unknown),
            Voice::new("de_DE-thorsten-medium").with_quality(VoiceQuality::Unknown),
        ];

        let processed = process_vits_voices(voices);

        let high_voice = processed.iter().find(|v| v.name == "en_US-lessac-high");
        let medium_voice = processed.iter().find(|v| v.name == "de_DE-thorsten-medium");

        assert_eq!(high_voice.unwrap().quality, VoiceQuality::Good);
        assert_eq!(medium_voice.unwrap().quality, VoiceQuality::Moderate);
    }

    #[test]
    fn test_process_vits_voices_order_independent() {
        // Should work regardless of which quality version comes first
        let voices_high_first = vec![
            Voice::new("en_US-lessac-high").with_quality(VoiceQuality::Good),
            Voice::new("en_US-lessac-medium").with_quality(VoiceQuality::Good),
        ];

        let voices_medium_first = vec![
            Voice::new("en_US-lessac-medium").with_quality(VoiceQuality::Good),
            Voice::new("en_US-lessac-high").with_quality(VoiceQuality::Good),
        ];

        let processed1 = process_vits_voices(voices_high_first);
        let processed2 = process_vits_voices(voices_medium_first);

        assert_eq!(processed1.len(), 1);
        assert_eq!(processed2.len(), 1);
        assert_eq!(processed1[0].name, "en_US-lessac-high");
        assert_eq!(processed2[0].name, "en_US-lessac-high");
    }

    // ========================================================================
    // Integration tests - only run if echogarden is installed
    // ========================================================================

    #[tokio::test]
    async fn test_is_ready_check() {
        let provider = EchogardenProvider::new();
        // This will return true if echogarden is installed, false otherwise
        // We can't assert either way since it depends on the system
        let _ = provider.is_ready().await;
    }

    #[tokio::test]
    #[ignore] // Only run manually when echogarden is installed
    async fn test_list_voices_integration() {
        let provider = EchogardenProvider::new();

        if !provider.is_ready().await {
            eprintln!("Skipping test: echogarden not installed");
            return;
        }

        let voices = provider.list_voices().await.unwrap();

        // Should have voices from at least one engine
        assert!(!voices.is_empty(), "Expected at least one voice");

        // Verify voice properties
        for voice in &voices {
            assert!(!voice.name.is_empty(), "Voice name should not be empty");
            assert!(
                voice.identifier.is_some(),
                "Voice should have identifier"
            );
            assert!(
                !voice.languages.is_empty(),
                "Voice should have at least one language"
            );
        }

        println!("Found {} echogarden voices", voices.len());
        for voice in voices.iter().take(10) {
            println!(
                "  - {} ({:?}, {:?}): {:?}",
                voice.name, voice.gender, voice.quality, voice.languages
            );
        }
    }

    #[tokio::test]
    #[ignore] // Produces audio - run manually
    async fn test_speak_integration() {
        let provider = EchogardenProvider::new();

        if !provider.is_ready().await {
            eprintln!("Skipping test: echogarden not installed");
            return;
        }

        let config = TtsConfig::default();
        let result = provider
            .speak("Hello from the Echogarden provider test.", &config)
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[ignore] // Produces audio - run manually
    async fn test_speak_with_voice() {
        let provider = EchogardenProvider::with_engine(EchogardenEngine::Kokoro);

        if !provider.is_ready().await {
            eprintln!("Skipping test: echogarden not installed");
            return;
        }

        let config = TtsConfig::new().with_voice("Heart");
        let result = provider.speak("Testing with the Heart voice.", &config).await;
        assert!(result.is_ok());
    }

    /// Regression test: Output file must not pre-exist for echogarden.
    ///
    /// Bug: The speak() method used NamedTempFile which creates the file.
    /// Echogarden doesn't properly overwrite existing files - it produces
    /// empty (0-byte) output when the file already exists. The fix uses
    /// tempdir() instead and defines a path that doesn't exist yet.
    #[test]
    fn test_speak_uses_tempdir_not_namedtempfile() {
        // This test documents the requirement: we must NOT create the output
        // file before echogarden writes to it. The implementation should use
        // tempfile::tempdir() and join() a path, not NamedTempFile.
        //
        // The following is a compile-time check that the correct imports exist:
        let _tempdir: fn() -> std::io::Result<tempfile::TempDir> = tempfile::tempdir;

        // NamedTempFile should NOT be used for output files in echogarden
        // (This comment serves as documentation for future maintainers)
    }
}
