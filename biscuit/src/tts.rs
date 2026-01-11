//! Text-to-Speech utilities
//!
//! Provides cross-platform text-to-speech functionality using the system's
//! native TTS engine. Includes voice selection and blocking speech.
//!
//! ## Usage
//!
//! ```ignore
//! use shared::tts::{speak_when_able, VoiceConfig};
//!
//! // Simple usage with defaults
//! speak_when_able("Hello, world!", &VoiceConfig::default());
//!
//! // With custom voice selection
//! speak_when_able(
//!     "Custom voice",
//!     &VoiceConfig::new()
//!         .with_voice(VoiceSelector::ByName("Samantha".into()))
//!         .with_volume(0.8),
//! );
//! ```

use tts::Tts;

// ============================================================================
// Error Types
// ============================================================================

/// Errors that can occur during TTS operations.
#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum TtsError {
    /// TTS engine initialization failed.
    #[error("TTS initialization failed")]
    InitFailed {
        /// The underlying error from the TTS engine.
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// Voice selection failed.
    #[error("Voice selection failed: {reason}")]
    VoiceSelectionFailed {
        /// Description of why voice selection failed.
        reason: String,
    },

    /// Speech synthesis failed.
    #[error("Speech failed")]
    SpeechFailed {
        /// The underlying error from the TTS engine.
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// No suitable voice was found for the requested language.
    #[error("No suitable voice found (language: {language})")]
    NoSuitableVoice {
        /// The language that was requested.
        language: String,
    },
}

// ============================================================================
// Enums
// ============================================================================

/// Gender preference for voice selection.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum Gender {
    /// Prefer a male voice.
    Male,
    /// Prefer a female voice.
    Female,
    /// No gender preference (use any available voice).
    #[default]
    Any,
}

/// Language preference for voice selection.
#[non_exhaustive]
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Language {
    /// English language (any variant: en-US, en-GB, etc.).
    #[default]
    English,
    /// Custom language code (BCP-47 format recommended, e.g., "fr-FR", "es-MX").
    Custom(String),
}

impl Language {
    /// Returns the language code prefix for voice matching.
    ///
    /// ## Examples
    ///
    /// ```
    /// use shared::tts::Language;
    ///
    /// assert_eq!(Language::English.code_prefix(), "en");
    /// assert_eq!(Language::Custom("fr-FR".into()).code_prefix(), "fr-FR");
    /// ```
    pub fn code_prefix(&self) -> &str {
        match self {
            Language::English => "en",
            Language::Custom(code) => code,
        }
    }
}

/// Selector for choosing a specific voice from the system.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VoiceSelector {
    /// Select a system voice by its exact ID.
    ById(String),
    /// Select a system voice by its display name.
    ByName(String),
}

// ============================================================================
// Newtypes
// ============================================================================

/// Validated volume level for TTS output.
///
/// Volume is clamped to the valid range [0.0, 1.0] at construction time.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Volume(f32);

impl Volume {
    /// Create a new Volume, clamping to valid range [0.0, 1.0].
    ///
    /// ## Examples
    ///
    /// ```
    /// use shared::tts::Volume;
    ///
    /// assert_eq!(Volume::new(0.5).get(), 0.5);
    /// assert_eq!(Volume::new(1.5).get(), 1.0); // Clamped
    /// assert_eq!(Volume::new(-0.5).get(), 0.0); // Clamped
    /// ```
    pub fn new(value: f32) -> Self {
        Self(value.clamp(0.0, 1.0))
    }

    /// Get the volume value.
    pub fn get(&self) -> f32 {
        self.0
    }
}

impl Default for Volume {
    fn default() -> Self {
        Self(1.0)
    }
}

// ============================================================================
// Configuration Structs
// ============================================================================

/// Configuration for voice selection and TTS behavior.
///
/// Use the builder pattern to construct a `VoiceConfig`:
///
/// ```
/// use shared::tts::{VoiceConfig, VoiceSelector, Gender, Language};
///
/// let config = VoiceConfig::new()
///     .with_voice(VoiceSelector::ByName("Samantha".into()))
///     .of_gender(Gender::Female)
///     .with_volume(0.8)
///     .with_language(Language::English);
/// ```
#[derive(Debug, Clone, Default)]
pub struct VoiceConfig {
    /// Language preference for voice selection.
    pub language: Language,
    /// Ordered list of voice selectors to try (first match wins).
    pub voice_stack: Vec<VoiceSelector>,
    /// Gender preference for voice selection.
    pub gender: Gender,
    /// Volume level for TTS output.
    pub volume: Volume,
}

impl VoiceConfig {
    /// Create a new VoiceConfig with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a VoiceConfig with a specific voice by name.
    ///
    /// ## Examples
    ///
    /// ```
    /// use shared::tts::VoiceConfig;
    ///
    /// let config = VoiceConfig::with_name("Samantha");
    /// assert_eq!(config.voice_stack.len(), 1);
    /// ```
    pub fn with_name(name: impl Into<String>) -> Self {
        Self::new().with_voice(VoiceSelector::ByName(name.into()))
    }

    /// Create a VoiceConfig with a specific voice by ID.
    ///
    /// ## Examples
    ///
    /// ```
    /// use shared::tts::VoiceConfig;
    ///
    /// let config = VoiceConfig::with_id("com.apple.voice.Alex");
    /// assert_eq!(config.voice_stack.len(), 1);
    /// ```
    pub fn with_id(id: impl Into<String>) -> Self {
        Self::new().with_voice(VoiceSelector::ById(id.into()))
    }

    /// Add a voice selector to the voice stack.
    ///
    /// Voice selectors are tried in order; the first match wins.
    ///
    /// ## Examples
    ///
    /// ```
    /// use shared::tts::{VoiceConfig, VoiceSelector};
    ///
    /// let config = VoiceConfig::new()
    ///     .with_voice(VoiceSelector::ByName("Samantha".into()))
    ///     .with_voice(VoiceSelector::ByName("Alex".into()));
    /// assert_eq!(config.voice_stack.len(), 2);
    /// ```
    #[must_use]
    pub fn with_voice(mut self, voice: VoiceSelector) -> Self {
        self.voice_stack.push(voice);
        self
    }

    /// Set the gender preference for voice selection.
    ///
    /// ## Examples
    ///
    /// ```
    /// use shared::tts::{VoiceConfig, Gender};
    ///
    /// let config = VoiceConfig::new().of_gender(Gender::Female);
    /// assert_eq!(config.gender, Gender::Female);
    /// ```
    #[must_use]
    pub fn of_gender(mut self, gender: Gender) -> Self {
        self.gender = gender;
        self
    }

    /// Set the volume level.
    ///
    /// Values outside [0.0, 1.0] are clamped.
    ///
    /// ## Examples
    ///
    /// ```
    /// use shared::tts::VoiceConfig;
    ///
    /// let config = VoiceConfig::new().with_volume(0.5);
    /// assert_eq!(config.volume.get(), 0.5);
    ///
    /// // Values are clamped
    /// let config = VoiceConfig::new().with_volume(1.5);
    /// assert_eq!(config.volume.get(), 1.0);
    /// ```
    #[must_use]
    pub fn with_volume(mut self, volume: f32) -> Self {
        self.volume = Volume::new(volume);
        self
    }

    /// Set the language preference for voice selection.
    ///
    /// ## Examples
    ///
    /// ```
    /// use shared::tts::{VoiceConfig, Language};
    ///
    /// let config = VoiceConfig::new().with_language(Language::Custom("fr-FR".into()));
    /// assert_eq!(config.language.code_prefix(), "fr-FR");
    /// ```
    #[must_use]
    pub fn with_language(mut self, language: Language) -> Self {
        self.language = language;
        self
    }
}

/// Information about a system voice.
///
/// This is a stable wrapper around the internal `tts::Voice` type,
/// avoiding leaking implementation details in the public API.
#[derive(Debug, Clone)]
pub struct SystemVoiceInfo {
    /// The unique identifier for this voice.
    pub id: String,
    /// The display name of this voice.
    pub name: String,
    /// The language code for this voice (e.g., "en-US").
    pub language: String,
    /// The gender of this voice, if known.
    pub gender: Option<Gender>,
}

impl SystemVoiceInfo {
    /// Returns a display string for the gender.
    pub fn gender_str(&self) -> &'static str {
        match self.gender {
            Some(Gender::Male) => "Male",
            Some(Gender::Female) => "Female",
            Some(Gender::Any) => "Unknown",
            None => "Unknown",
        }
    }
}

// ============================================================================
// Speech Functions
// ============================================================================

/// Speak a message using the system's TTS engine.
///
/// This function blocks the current thread until speech completes.
///
/// ## Voice Selection Algorithm
///
/// 1. Try each `VoiceSelector` in the voice stack order (first match wins)
/// 2. Fall back to language filtering (excluding "compact" and "eloquence" voices)
/// 3. Fall back to any English voice
/// 4. Return `TtsError::NoSuitableVoice` if no voice matches
///
/// ## Errors
///
/// Returns `TtsError` if TTS initialization, voice selection, or speech fails.
///
/// ## Examples
///
/// ```ignore
/// use shared::tts::{speak, VoiceConfig};
///
/// // Simple usage with defaults
/// speak("Hello, world!", &VoiceConfig::default())?;
///
/// // With custom voice
/// speak("Custom voice", &VoiceConfig::with_name("Samantha"))?;
/// ```
#[tracing::instrument(skip(config), fields(message_len = message.len()))]
pub fn speak(message: &str, config: &VoiceConfig) -> Result<(), TtsError> {
    let mut tts = Tts::default().map_err(|e| TtsError::InitFailed {
        source: Box::new(e),
    })?;

    // Select voice using the algorithm
    select_voice(&mut tts, config)?;

    // Speak and wait for completion
    tts.speak(message, false).map_err(|e| TtsError::SpeechFailed {
        source: Box::new(e),
    })?;

    // Block until speech completes
    std::thread::sleep(std::time::Duration::from_millis(100));
    while tts.is_speaking().unwrap_or(false) {
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    Ok(())
}

/// Speak a message, ignoring any errors.
///
/// This is the fire-and-forget variant of `speak()`. Use when TTS is
/// a nice-to-have feature and failures shouldn't affect the main flow.
///
/// This function blocks the current thread until speech completes (if successful).
///
/// ## Examples
///
/// ```ignore
/// use shared::tts::{speak_when_able, VoiceConfig};
///
/// // Fire and forget - errors are logged but ignored
/// speak_when_able("Task complete!", &VoiceConfig::default());
/// ```
pub fn speak_when_able(message: &str, config: &VoiceConfig) {
    if let Err(e) = speak(message, config) {
        tracing::debug!(error = ?e, "TTS failed (non-fatal)");
    }
}

/// Query the system's available TTS voices.
///
/// Returns information about all voices available on the system.
///
/// ## Errors
///
/// Returns `TtsError::InitFailed` if TTS initialization fails.
///
/// ## Examples
///
/// ```ignore
/// use shared::tts::available_system_voices;
///
/// for voice in available_system_voices()? {
///     println!("{}: {} ({})", voice.id, voice.name, voice.language);
/// }
/// ```
pub fn available_system_voices() -> Result<Vec<SystemVoiceInfo>, TtsError> {
    let tts = Tts::default().map_err(|e| TtsError::InitFailed {
        source: Box::new(e),
    })?;

    let voices = tts.voices().map_err(|e| TtsError::InitFailed {
        source: Box::new(e),
    })?;

    Ok(voices
        .into_iter()
        .map(|v| {
            let gender = match v.gender() {
                Some(tts::Gender::Male) => Some(Gender::Male),
                Some(tts::Gender::Female) => Some(Gender::Female),
                _ => None,
            };
            SystemVoiceInfo {
                id: v.id().to_string(),
                name: v.name().to_string(),
                language: v.language().to_string(),
                gender,
            }
        })
        .collect())
}

/// Check if a voice matches the requested gender.
fn matches_gender(voice: &tts::Voice, requested: Gender) -> bool {
    match requested {
        Gender::Any => true,
        Gender::Male => voice.gender() == Some(tts::Gender::Male),
        Gender::Female => voice.gender() == Some(tts::Gender::Female),
    }
}

/// Check if a voice ID should be excluded (compact/eloquence voices).
fn is_excluded_voice(id: &str) -> bool {
    let lower = id.to_lowercase();
    lower.contains("compact") || lower.contains("eloquence")
}

/// Check if a voice is Premium quality (highest on macOS).
fn is_premium_voice(name: &str) -> bool {
    name.contains("(Premium)")
}

/// Check if a voice is Enhanced quality (high on macOS).
fn is_enhanced_voice(name: &str) -> bool {
    name.contains("(Enhanced)")
}

/// Find a voice matching criteria, preferring Premium > Enhanced > regular.
fn find_best_voice<'a>(
    voices: &'a [tts::Voice],
    lang_prefix: &str,
    gender: Gender,
) -> Option<&'a tts::Voice> {
    let matches_criteria = |v: &&tts::Voice| {
        !is_excluded_voice(&v.id())
            && v.language().starts_with(lang_prefix)
            && matches_gender(v, gender)
    };

    // Try Premium first
    if let Some(voice) = voices
        .iter()
        .filter(matches_criteria)
        .find(|v| is_premium_voice(&v.name()))
    {
        return Some(voice);
    }

    // Try Enhanced next
    if let Some(voice) = voices
        .iter()
        .filter(matches_criteria)
        .find(|v| is_enhanced_voice(&v.name()))
    {
        return Some(voice);
    }

    // Fall back to any matching voice
    voices.iter().find(matches_criteria)
}

/// Select a voice based on the VoiceConfig.
///
/// Algorithm:
/// 1. Try each VoiceSelector in voice_stack order
/// 2. Try language + gender filtering, preferring Premium > Enhanced > regular
/// 3. Fall back to language filtering only (any gender), same quality preference
/// 4. Fall back to any English voice
/// 5. Return error if no voice found
fn select_voice(tts: &mut Tts, config: &VoiceConfig) -> Result<(), TtsError> {
    let voices = tts.voices().map_err(|e| TtsError::VoiceSelectionFailed {
        reason: format!("Failed to query voices: {}", e),
    })?;

    // Step 1: Try each VoiceSelector in voice_stack order
    for selector in &config.voice_stack {
        match selector {
            VoiceSelector::ById(id) => {
                if let Some(voice) = voices.iter().find(|v| v.id() == id.as_str()) {
                    return tts.set_voice(voice).map_err(|e| TtsError::VoiceSelectionFailed {
                        reason: format!("Failed to set voice by ID '{}': {}", id, e),
                    });
                }
            }
            VoiceSelector::ByName(name) => {
                if let Some(voice) = voices.iter().find(|v| v.name() == name.as_str()) {
                    return tts.set_voice(voice).map_err(|e| TtsError::VoiceSelectionFailed {
                        reason: format!("Failed to set voice by name '{}': {}", name, e),
                    });
                }
            }
        }
    }

    let lang_prefix = config.language.code_prefix();

    // Step 2: Try language + gender filtering with quality preference
    if config.gender != Gender::Any
        && let Some(voice) = find_best_voice(&voices, lang_prefix, config.gender)
    {
        return tts.set_voice(voice).map_err(|e| TtsError::VoiceSelectionFailed {
            reason: format!(
                "Failed to set {:?} voice for language '{}': {}",
                config.gender, lang_prefix, e
            ),
        });
    }

    // Step 3: Fall back to language filtering only (any gender) with quality preference
    if let Some(voice) = find_best_voice(&voices, lang_prefix, Gender::Any) {
        return tts.set_voice(voice).map_err(|e| TtsError::VoiceSelectionFailed {
            reason: format!("Failed to set voice for language '{}': {}", lang_prefix, e),
        });
    }

    // Step 4: Final fallback - any English voice
    if let Some(voice) = voices.iter().find(|v| v.language().starts_with("en")) {
        return tts.set_voice(voice).map_err(|e| TtsError::VoiceSelectionFailed {
            reason: format!("Failed to set fallback English voice: {}", e),
        });
    }

    // Step 5: No suitable voice found
    Err(TtsError::NoSuitableVoice {
        language: lang_prefix.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================================================
    // TtsError tests
    // ========================================================================

    #[test]
    fn test_tts_error_init_failed_display() {
        let error = TtsError::InitFailed {
            source: "mock error".into(),
        };
        assert_eq!(error.to_string(), "TTS initialization failed");
    }

    #[test]
    fn test_tts_error_no_suitable_voice_display() {
        let error = TtsError::NoSuitableVoice {
            language: "fr".to_string(),
        };
        assert_eq!(error.to_string(), "No suitable voice found (language: fr)");
    }

    #[test]
    fn test_tts_error_voice_selection_failed_display() {
        let error = TtsError::VoiceSelectionFailed {
            reason: "no voices available".to_string(),
        };
        assert_eq!(
            error.to_string(),
            "Voice selection failed: no voices available"
        );
    }

    #[test]
    fn test_tts_error_speech_failed_display() {
        let error = TtsError::SpeechFailed {
            source: "audio device error".into(),
        };
        assert_eq!(error.to_string(), "Speech failed");
    }

    // ========================================================================
    // Gender tests
    // ========================================================================

    #[test]
    fn test_gender_default_is_any() {
        assert_eq!(Gender::default(), Gender::Any);
    }

    #[test]
    fn test_gender_variants_exist() {
        let _ = Gender::Male;
        let _ = Gender::Female;
        let _ = Gender::Any;
    }

    // ========================================================================
    // Language tests
    // ========================================================================

    #[test]
    fn test_language_default_is_english() {
        assert_eq!(Language::default(), Language::English);
    }

    #[test]
    fn test_language_code_prefix_english() {
        assert_eq!(Language::English.code_prefix(), "en");
    }

    #[test]
    fn test_language_code_prefix_custom() {
        assert_eq!(Language::Custom("fr-FR".into()).code_prefix(), "fr-FR");
        assert_eq!(Language::Custom("es-MX".into()).code_prefix(), "es-MX");
        assert_eq!(Language::Custom("de".into()).code_prefix(), "de");
    }

    // ========================================================================
    // VoiceSelector tests
    // ========================================================================

    #[test]
    fn test_voice_selector_by_id() {
        let selector = VoiceSelector::ById("com.apple.voice.Alex".into());
        if let VoiceSelector::ById(id) = selector {
            assert_eq!(id, "com.apple.voice.Alex");
        } else {
            panic!("Expected ById variant");
        }
    }

    #[test]
    fn test_voice_selector_by_name() {
        let selector = VoiceSelector::ByName("Samantha".into());
        if let VoiceSelector::ByName(name) = selector {
            assert_eq!(name, "Samantha");
        } else {
            panic!("Expected ByName variant");
        }
    }

    // ========================================================================
    // Volume tests
    // ========================================================================

    #[test]
    fn test_volume_default_is_one() {
        assert_eq!(Volume::default().get(), 1.0);
    }

    #[test]
    fn test_volume_new_valid_value() {
        assert_eq!(Volume::new(0.5).get(), 0.5);
        assert_eq!(Volume::new(0.0).get(), 0.0);
        assert_eq!(Volume::new(1.0).get(), 1.0);
    }

    #[test]
    fn test_volume_clamps_high_values() {
        assert_eq!(Volume::new(1.5).get(), 1.0);
        assert_eq!(Volume::new(100.0).get(), 1.0);
    }

    #[test]
    fn test_volume_clamps_low_values() {
        assert_eq!(Volume::new(-0.5).get(), 0.0);
        assert_eq!(Volume::new(-100.0).get(), 0.0);
    }

    // ========================================================================
    // VoiceConfig tests
    // ========================================================================

    #[test]
    fn test_voice_config_default_has_empty_voice_stack() {
        let config = VoiceConfig::default();
        assert!(config.voice_stack.is_empty());
    }

    #[test]
    fn test_voice_config_default_volume_is_one() {
        let config = VoiceConfig::default();
        assert_eq!(config.volume.get(), 1.0);
    }

    #[test]
    fn test_voice_config_default_language_is_english() {
        let config = VoiceConfig::default();
        assert_eq!(config.language, Language::English);
    }

    #[test]
    fn test_voice_config_default_gender_is_any() {
        let config = VoiceConfig::default();
        assert_eq!(config.gender, Gender::Any);
    }

    // ========================================================================
    // SystemVoiceInfo tests
    // ========================================================================

    #[test]
    fn test_system_voice_info_construction() {
        let voice = SystemVoiceInfo {
            id: "com.apple.voice.Alex".into(),
            name: "Alex".into(),
            language: "en-US".into(),
            gender: Some(Gender::Male),
        };
        assert_eq!(voice.id, "com.apple.voice.Alex");
        assert_eq!(voice.name, "Alex");
        assert_eq!(voice.language, "en-US");
        assert_eq!(voice.gender, Some(Gender::Male));
    }

    #[test]
    fn test_system_voice_info_gender_str() {
        let male = SystemVoiceInfo {
            id: "".into(),
            name: "".into(),
            language: "".into(),
            gender: Some(Gender::Male),
        };
        assert_eq!(male.gender_str(), "Male");

        let female = SystemVoiceInfo {
            id: "".into(),
            name: "".into(),
            language: "".into(),
            gender: Some(Gender::Female),
        };
        assert_eq!(female.gender_str(), "Female");

        let unknown = SystemVoiceInfo {
            id: "".into(),
            name: "".into(),
            language: "".into(),
            gender: None,
        };
        assert_eq!(unknown.gender_str(), "Unknown");
    }

    // ========================================================================
    // VoiceConfig builder tests (Phase 2)
    // ========================================================================

    #[test]
    fn test_voice_config_new_returns_default() {
        let config = VoiceConfig::new();
        assert!(config.voice_stack.is_empty());
        assert_eq!(config.volume.get(), 1.0);
        assert_eq!(config.language, Language::English);
        assert_eq!(config.gender, Gender::Any);
    }

    #[test]
    fn test_voice_config_chained_builder_calls() {
        let config = VoiceConfig::new()
            .of_gender(Gender::Female)
            .with_volume(0.8)
            .with_language(Language::Custom("fr-FR".into()));

        assert_eq!(config.gender, Gender::Female);
        assert_eq!(config.volume.get(), 0.8);
        assert_eq!(config.language.code_prefix(), "fr-FR");
    }

    #[test]
    fn test_voice_config_multiple_with_voice_preserves_order() {
        let config = VoiceConfig::new()
            .with_voice(VoiceSelector::ByName("First".into()))
            .with_voice(VoiceSelector::ByName("Second".into()))
            .with_voice(VoiceSelector::ById("third-id".into()));

        assert_eq!(config.voice_stack.len(), 3);
        assert_eq!(config.voice_stack[0], VoiceSelector::ByName("First".into()));
        assert_eq!(
            config.voice_stack[1],
            VoiceSelector::ByName("Second".into())
        );
        assert_eq!(config.voice_stack[2], VoiceSelector::ById("third-id".into()));
    }

    #[test]
    fn test_voice_config_with_name_creates_config() {
        let config = VoiceConfig::with_name("Alice");
        assert_eq!(config.voice_stack.len(), 1);
        assert_eq!(
            config.voice_stack[0],
            VoiceSelector::ByName("Alice".into())
        );
    }

    #[test]
    fn test_voice_config_with_id_creates_config() {
        let config = VoiceConfig::with_id("com.apple.voice.Alex");
        assert_eq!(config.voice_stack.len(), 1);
        assert_eq!(
            config.voice_stack[0],
            VoiceSelector::ById("com.apple.voice.Alex".into())
        );
    }

    #[test]
    fn test_voice_config_with_volume_clamps_high() {
        let config = VoiceConfig::new().with_volume(1.5);
        assert_eq!(config.volume.get(), 1.0);
    }

    #[test]
    fn test_voice_config_with_volume_clamps_low() {
        let config = VoiceConfig::new().with_volume(-0.5);
        assert_eq!(config.volume.get(), 0.0);
    }

    #[test]
    fn test_voice_config_builder_all_options() {
        let config = VoiceConfig::new()
            .with_voice(VoiceSelector::ByName("Samantha".into()))
            .with_voice(VoiceSelector::ById("fallback-id".into()))
            .of_gender(Gender::Female)
            .with_volume(0.7)
            .with_language(Language::English);

        assert_eq!(config.voice_stack.len(), 2);
        assert_eq!(config.gender, Gender::Female);
        assert_eq!(config.volume.get(), 0.7);
        assert_eq!(config.language, Language::English);
    }
}
