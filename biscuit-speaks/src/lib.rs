//! Biscuit Speaks
//!
//! Provides cross-platform text-to-speech functionality using the host system's
//! native TTS solutions. Includes voice selection and blocking speech.
//!
//! ## Usage
//!
//! #TODO
//! ```


pub mod speak;
pub mod types;
pub mod errors;

// ============================================================================
// Error Types
// ============================================================================



// ============================================================================
// Enums
// ============================================================================







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
    /// use biscuit_speaks::Volume;
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
/// use biscuit_speaks::{VoiceConfig, VoiceSelector, Gender, Language};
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
    /// use biscuit_speaks::VoiceConfig;
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
    /// use biscuit_speaks::VoiceConfig;
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
    /// use biscuit_speaks::{VoiceConfig, VoiceSelector};
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
    /// use biscuit_speaks::{VoiceConfig, Gender};
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
    /// use biscuit_speaks::VoiceConfig;
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
    /// use biscuit_speaks::{VoiceConfig, Language};
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
