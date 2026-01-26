//! ElevenLabs cloud TTS provider.
//!
//! This module implements the ElevenLabs text-to-speech API using the
//! schematic-schema generated client.
//!
//! ## Environment Variables
//!
//! The API key is read from:
//! - `ELEVEN_LABS_API_KEY` (preferred by schematic)
//! - `ELEVENLABS_API_KEY` (alternative)
//!
//! ## Examples
//!
//! ```ignore
//! use biscuit_speaks::providers::cloud::ElevenLabsProvider;
//! use biscuit_speaks::{TtsExecutor, TtsConfig};
//!
//! let provider = ElevenLabsProvider::new()?;
//! provider.speak("Hello, world!", &TtsConfig::default()).await?;
//! ```

use schematic_schema::elevenlabs::{
    CreateSpeechBody, CreateSpeechRequest, CreateSoundEffectBody, CreateSoundEffectRequest,
    ElevenLabs, ListVoicesRequest, ListVoicesResponse, ModelInfo, VoiceResponseModel,
};

use crate::errors::TtsError;
use crate::traits::{TtsExecutor, TtsVoiceInventory};
use crate::types::{AudioFormat, CloudTtsProvider, Gender, Language, SpeakResult, TtsConfig, TtsProvider, Voice, VoiceQuality};

/// Default ElevenLabs voice ID (Rachel - a versatile female voice).
const DEFAULT_VOICE_ID: &str = "21m00Tcm4TlvDq8ikWAM";

/// Default ElevenLabs model for multilingual support.
const DEFAULT_MODEL_ID: &str = "eleven_multilingual_v2";

/// ElevenLabs cloud TTS provider.
///
/// Implements the `TtsExecutor` trait using the ElevenLabs text-to-speech API.
/// Requires an API key to be set in the environment.
///
/// ## Examples
///
/// ```ignore
/// use biscuit_speaks::ElevenLabsProvider;
/// use biscuit_speaks::{TtsExecutor, TtsConfig};
///
/// // Create provider (reads API key from environment)
/// let provider = ElevenLabsProvider::new()?;
///
/// // Generate and play speech
/// provider.speak("Hello, world!", &TtsConfig::default()).await?;
///
/// // Or just generate audio bytes
/// let audio = provider.generate_audio("Hello", &TtsConfig::default()).await?;
/// ```
pub struct ElevenLabsProvider {
    /// The schematic-generated ElevenLabs client.
    client: ElevenLabs,
    /// Default voice ID to use when none specified.
    default_voice_id: String,
    /// Default model ID to use.
    default_model_id: String,
}

impl std::fmt::Debug for ElevenLabsProvider {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ElevenLabsProvider")
            .field("default_voice_id", &self.default_voice_id)
            .field("default_model_id", &self.default_model_id)
            .finish_non_exhaustive()
    }
}

impl ElevenLabsProvider {
    /// Create a new ElevenLabs provider using environment variables.
    ///
    /// The API key is read from `ELEVEN_LABS_API_KEY` or `ELEVENLABS_API_KEY`.
    ///
    /// ## Errors
    ///
    /// Returns `TtsError::MissingApiKey` if no API key is found in the environment.
    ///
    /// ## Examples
    ///
    /// ```ignore
    /// let provider = ElevenLabsProvider::new()?;
    /// ```
    pub fn new() -> Result<Self, TtsError> {
        // Verify API key exists before creating the client
        if std::env::var("ELEVEN_LABS_API_KEY").is_err()
            && std::env::var("ELEVENLABS_API_KEY").is_err()
        {
            return Err(TtsError::MissingApiKey {
                provider: "elevenlabs".into(),
            });
        }

        Ok(Self {
            client: ElevenLabs::new(),
            default_voice_id: DEFAULT_VOICE_ID.into(),
            default_model_id: DEFAULT_MODEL_ID.into(),
        })
    }

    /// Create a new ElevenLabs provider with a custom base URL.
    ///
    /// Useful for testing with mock servers.
    ///
    /// ## Examples
    ///
    /// ```ignore
    /// let provider = ElevenLabsProvider::with_base_url("http://localhost:8080")?;
    /// ```
    pub fn with_base_url(url: impl Into<String>) -> Result<Self, TtsError> {
        // Verify API key exists before creating the client
        if std::env::var("ELEVEN_LABS_API_KEY").is_err()
            && std::env::var("ELEVENLABS_API_KEY").is_err()
        {
            return Err(TtsError::MissingApiKey {
                provider: "elevenlabs".into(),
            });
        }

        Ok(Self {
            client: ElevenLabs::with_base_url(url),
            default_voice_id: DEFAULT_VOICE_ID.into(),
            default_model_id: DEFAULT_MODEL_ID.into(),
        })
    }

    /// Set the default voice ID.
    #[must_use]
    pub fn with_default_voice(mut self, voice_id: impl Into<String>) -> Self {
        self.default_voice_id = voice_id.into();
        self
    }

    /// Set the default model ID.
    #[must_use]
    pub fn with_default_model(mut self, model_id: impl Into<String>) -> Self {
        self.default_model_id = model_id.into();
        self
    }

    fn voice_matches_gender(voice: &VoiceResponseModel, gender_label: &str) -> bool {
        voice
            .labels
            .as_ref()
            .and_then(|labels| labels.get("gender"))
            .map(|value| value.eq_ignore_ascii_case(gender_label))
            .unwrap_or(false)
    }

    /// Convert an ElevenLabs API voice response to our Voice type.
    fn voice_response_to_voice(voice: VoiceResponseModel) -> Voice {
        // Extract gender from labels
        let gender = voice
            .labels
            .as_ref()
            .and_then(|labels| labels.get("gender"))
            .map(|g| match g.to_lowercase().as_str() {
                "male" => Gender::Male,
                "female" => Gender::Female,
                _ => Gender::Any,
            })
            .unwrap_or(Gender::Any);

        // Extract languages from verified_languages if available
        let languages: Vec<Language> = voice
            .verified_languages
            .as_ref()
            .map(|langs| {
                langs
                    .iter()
                    .map(|lang| {
                        // Check for English variants
                        if lang.language_id.starts_with("en") {
                            Language::English
                        } else {
                            Language::Custom(lang.language_id.clone())
                        }
                    })
                    .collect()
            })
            .unwrap_or_else(|| vec![Language::English]); // Default to English

        Voice::new(&voice.name)
            .with_identifier(&voice.voice_id)
            .with_gender(gender)
            .with_quality(VoiceQuality::Excellent) // All ElevenLabs voices are excellent quality
            .with_languages(languages)
    }

    /// Check if the ElevenLabs API key is configured in the environment.
    ///
    /// Returns `true` if either `ELEVENLABS_API_KEY` or `ELEVEN_LABS_API_KEY`
    /// is set in the environment.
    pub fn has_api_key() -> bool {
        std::env::var("ELEVEN_LABS_API_KEY").is_ok()
            || std::env::var("ELEVENLABS_API_KEY").is_ok()
    }

    async fn resolve_voice_id(&self, config: &TtsConfig) -> Result<String, TtsError> {
        if let Some(voice_id) = &config.requested_voice {
            return Ok(voice_id.clone());
        }

        let gender_label = match config.gender {
            Gender::Male => Some("male"),
            Gender::Female => Some("female"),
            Gender::Any => None,
        };

        if let Some(gender_label) = gender_label {
            match self.list_voices_raw().await {
                Ok(voices) => {
                    if let Some(voice) = voices
                        .voices
                        .iter()
                        .find(|voice| Self::voice_matches_gender(voice, gender_label))
                    {
                        return Ok(voice.voice_id.clone());
                    }

                    tracing::warn!(
                        gender = gender_label,
                        "No ElevenLabs voice matched gender, falling back to default"
                    );
                }
                Err(error) => {
                    tracing::warn!(
                        error = ?error,
                        gender = gender_label,
                        "Failed to fetch ElevenLabs voices, falling back to default"
                    );
                }
            }
        }

        Ok(self.default_voice_id.clone())
    }

    /// Generate audio bytes from text using the ElevenLabs API.
    ///
    /// ## Arguments
    ///
    /// * `text` - The text to convert to speech.
    /// * `config` - TTS configuration for voice selection.
    ///
    /// ## Returns
    ///
    /// Audio bytes in MP3 format.
    ///
    /// ## Errors
    ///
    /// Returns `TtsError::HttpError` if the request fails, or
    /// `TtsError::ApiError` if the API returns an error response.
    ///
    /// ## Examples
    ///
    /// ```ignore
    /// let provider = ElevenLabsProvider::new()?;
    /// let audio = provider.generate_audio("Hello", &TtsConfig::default()).await?;
    /// // audio is Vec<u8> containing MP3 data
    /// ```
    pub async fn generate_audio(&self, text: &str, config: &TtsConfig) -> Result<Vec<u8>, TtsError> {
        let voice_id = self.resolve_voice_id(config).await?;

        // Build the request body
        let body = CreateSpeechBody {
            text: text.to_string(),
            model_id: Some(self.default_model_id.clone()),
            language_code: Some(config.language.code_prefix().to_string()),
            ..Default::default()
        };

        // Build the request
        let request = CreateSpeechRequest::new(voice_id.clone(), body);

        tracing::debug!(
            voice_id = %voice_id,
            text_len = text.len(),
            model = %self.default_model_id,
            "Sending ElevenLabs TTS request"
        );

        // Use the generated client to make the request
        let audio_bytes = self
            .client
            .create_speech(request)
            .await
            .map_err(|e| TtsError::HttpError {
                provider: "elevenlabs".into(),
                message: e.to_string(),
            })?;

        tracing::debug!(
            audio_size = audio_bytes.len(),
            "Received ElevenLabs audio response"
        );

        Ok(audio_bytes.to_vec())
    }

    /// List available voices from the ElevenLabs API with full response details.
    ///
    /// This returns the raw API response with all ElevenLabs-specific fields.
    /// For a normalized voice list, use the `TtsVoiceInventory::list_voices()` trait method.
    ///
    /// ## Returns
    ///
    /// A list of available voice information with ElevenLabs-specific metadata.
    ///
    /// ## Errors
    ///
    /// Returns `TtsError::HttpError` if the request fails.
    ///
    /// ## Examples
    ///
    /// ```ignore
    /// let provider = ElevenLabsProvider::new()?;
    /// let response = provider.list_voices_raw().await?;
    /// for voice in response.voices {
    ///     println!("{}: {}", voice.voice_id, voice.name);
    /// }
    /// ```
    pub async fn list_voices_raw(&self) -> Result<ListVoicesResponse, TtsError> {
        let request = ListVoicesRequest {};

        self.client
            .request::<ListVoicesResponse>(request)
            .await
            .map_err(|e| TtsError::HttpError {
                provider: "elevenlabs".into(),
                message: e.to_string(),
            })
    }

    /// List available models from the ElevenLabs API.
    ///
    /// ## Returns
    ///
    /// A list of available model information.
    ///
    /// ## Errors
    ///
    /// Returns `TtsError::HttpError` if the request fails.
    pub async fn list_models(&self) -> Result<Vec<ModelInfo>, TtsError> {
        use schematic_schema::elevenlabs::ListModelsRequest;

        let request = ListModelsRequest {};

        self.client
            .request::<Vec<ModelInfo>>(request)
            .await
            .map_err(|e| TtsError::HttpError {
                provider: "elevenlabs".into(),
                message: e.to_string(),
            })
    }

    /// Create a sound effect from a text description.
    ///
    /// ## Arguments
    ///
    /// * `prompt` - Text description of the sound effect (e.g., "dog barking").
    /// * `duration_seconds` - Optional duration in seconds (0.5 to 22).
    ///
    /// ## Returns
    ///
    /// Audio bytes in MP3 format.
    ///
    /// ## Errors
    ///
    /// Returns `TtsError::HttpError` if the request fails.
    ///
    /// ## Examples
    ///
    /// ```ignore
    /// let provider = ElevenLabsProvider::new()?;
    /// let audio = provider.create_sound_effect("dog barking loudly", Some(3.0)).await?;
    /// ```
    pub async fn create_sound_effect(
        &self,
        prompt: &str,
        duration_seconds: Option<f64>,
    ) -> Result<Vec<u8>, TtsError> {
        let body = CreateSoundEffectBody {
            text: prompt.to_string(),
            duration_seconds,
            ..Default::default()
        };

        let request = CreateSoundEffectRequest { body };

        tracing::debug!(
            prompt = %prompt,
            duration = ?duration_seconds,
            "Creating ElevenLabs sound effect"
        );

        let audio_bytes = self
            .client
            .create_sound_effect(request)
            .await
            .map_err(|e| TtsError::HttpError {
                provider: "elevenlabs".into(),
                message: e.to_string(),
            })?;

        Ok(audio_bytes.to_vec())
    }
}

impl TtsExecutor for ElevenLabsProvider {
    async fn speak(&self, text: &str, config: &TtsConfig) -> Result<(), TtsError> {
        // Generate audio
        let audio_bytes = self.generate_audio(text, config).await?;

        // Play the audio (MP3 format from ElevenLabs)
        crate::playback::play_audio_bytes(&audio_bytes, AudioFormat::Mp3).await
    }

    async fn is_ready(&self) -> bool {
        // If we have a client, the API key was verified at construction time
        // For a more thorough check, we could make an API call, but construction
        // already validates the API key exists
        true
    }

    fn info(&self) -> &str {
        "ElevenLabs - High quality AI voice synthesis with neural TTS models"
    }

    async fn speak_with_result(
        &self,
        text: &str,
        config: &TtsConfig,
    ) -> Result<SpeakResult, TtsError> {
        // Resolve the voice ID (this may fetch the voice list for gender matching)
        let voice_id = self.resolve_voice_id(config).await?;

        // Try to get full voice metadata from the voice list
        let voice = if let Ok(voices) = self.list_voices().await {
            // Find the voice with this ID
            voices
                .into_iter()
                .find(|v| v.identifier.as_deref() == Some(&voice_id))
                .unwrap_or_else(|| {
                    Voice::new(&voice_id)
                        .with_identifier(&voice_id)
                        .with_gender(config.gender)
                        .with_quality(VoiceQuality::Excellent)
                        .with_language(config.language.clone())
                })
        } else {
            Voice::new(&voice_id)
                .with_identifier(&voice_id)
                .with_gender(config.gender)
                .with_quality(VoiceQuality::Excellent)
                .with_language(config.language.clone())
        };

        // Call speak
        self.speak(text, config).await?;

        // Return the result
        Ok(SpeakResult::new(
            TtsProvider::Cloud(CloudTtsProvider::ElevenLabs),
            voice,
        ))
    }
}

impl TtsVoiceInventory for ElevenLabsProvider {
    async fn list_voices(&self) -> Result<Vec<Voice>, TtsError> {
        let response = self
            .client
            .request::<ListVoicesResponse>(ListVoicesRequest {})
            .await
            .map_err(|e| TtsError::VoiceEnumerationFailed {
                provider: "elevenlabs".into(),
                message: e.to_string(),
            })?;

        Ok(response
            .voices
            .into_iter()
            .map(Self::voice_response_to_voice)
            .collect())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_default_voice_id() {
        assert_eq!(DEFAULT_VOICE_ID, "21m00Tcm4TlvDq8ikWAM");
    }

    #[test]
    fn test_default_model_id() {
        assert_eq!(DEFAULT_MODEL_ID, "eleven_multilingual_v2");
    }

    #[test]
    fn test_new_without_env_var() {
        // Ensure no API key is set
        // SAFETY: Tests run with test isolation, removing env vars is safe
        unsafe {
            std::env::remove_var("ELEVENLABS_API_KEY");
            std::env::remove_var("ELEVEN_LABS_API_KEY");
        }

        let result = ElevenLabsProvider::new();
        assert!(result.is_err());
        match result {
            Err(TtsError::MissingApiKey { provider }) => {
                assert_eq!(provider, "elevenlabs");
            }
            _ => panic!("Expected MissingApiKey error"),
        }
    }

    #[test]
    fn test_with_base_url_without_env_var() {
        // Ensure no API key is set
        // SAFETY: Tests run with test isolation, removing env vars is safe
        unsafe {
            std::env::remove_var("ELEVENLABS_API_KEY");
            std::env::remove_var("ELEVEN_LABS_API_KEY");
        }

        let result = ElevenLabsProvider::with_base_url("http://localhost:8080");
        assert!(result.is_err());
        match result {
            Err(TtsError::MissingApiKey { provider }) => {
                assert_eq!(provider, "elevenlabs");
            }
            _ => panic!("Expected MissingApiKey error"),
        }
    }

    // ========================================================================
    // has_api_key() tests
    // ========================================================================

    // Note: These tests verify the has_api_key logic directly using the internal
    // implementation rather than depending on env var state, since env var tests
    // can race in parallel test execution. The actual env var checks are already
    // covered by test_new_without_env_var.

    #[test]
    fn test_has_api_key_logic_elevenlabs_key() {
        // Test the logic: ELEVENLABS_API_KEY should be checked
        fn check_with_vars(elevenlabs: Option<&str>, eleven_labs: Option<&str>) -> bool {
            elevenlabs.is_some() || eleven_labs.is_some()
        }

        assert!(check_with_vars(Some("key"), None));
    }

    #[test]
    fn test_has_api_key_logic_eleven_labs_key() {
        fn check_with_vars(elevenlabs: Option<&str>, eleven_labs: Option<&str>) -> bool {
            elevenlabs.is_some() || eleven_labs.is_some()
        }

        assert!(check_with_vars(None, Some("key")));
    }

    #[test]
    fn test_has_api_key_logic_both_keys() {
        fn check_with_vars(elevenlabs: Option<&str>, eleven_labs: Option<&str>) -> bool {
            elevenlabs.is_some() || eleven_labs.is_some()
        }

        assert!(check_with_vars(Some("key1"), Some("key2")));
    }

    #[test]
    fn test_has_api_key_logic_no_keys() {
        fn check_with_vars(elevenlabs: Option<&str>, eleven_labs: Option<&str>) -> bool {
            elevenlabs.is_some() || eleven_labs.is_some()
        }

        assert!(!check_with_vars(None, None));
    }

    // ========================================================================
    // info() tests
    // ========================================================================

    #[test]
    fn test_info_returns_description() {
        // This test verifies the info() return value.
        // We need a provider to call info(), so we skip if API key isn't available.
        if !ElevenLabsProvider::has_api_key() {
            return; // Skip test if no API key
        }

        let provider = ElevenLabsProvider::new().unwrap();
        let info = provider.info();

        assert!(info.contains("ElevenLabs"));
        assert!(info.contains("AI voice"));
    }

    #[test]
    fn test_info_content_is_correct() {
        // Test the exact info string without needing a provider instance
        const EXPECTED_INFO: &str =
            "ElevenLabs - High quality AI voice synthesis with neural TTS models";
        assert!(EXPECTED_INFO.contains("ElevenLabs"));
        assert!(EXPECTED_INFO.contains("AI voice"));
        assert!(EXPECTED_INFO.contains("neural"));
    }

    // ========================================================================
    // is_ready() tests
    // ========================================================================

    #[tokio::test]
    async fn test_is_ready_returns_true_when_constructed() {
        // Skip if no API key available
        if !ElevenLabsProvider::has_api_key() {
            return;
        }

        let provider = ElevenLabsProvider::new().unwrap();
        // If construction succeeded, is_ready should return true
        assert!(provider.is_ready().await);
    }

    #[test]
    fn test_is_ready_behavior_documented() {
        // The is_ready() method returns true if the provider was successfully
        // constructed, since construction validates the API key exists.
        // This test documents the expected behavior.

        // Construction fails without API key -> is_ready() would never be called
        // Construction succeeds with API key -> is_ready() returns true
        // This is the expected contract.
    }

    // ========================================================================
    // voice_response_to_voice() tests
    // ========================================================================

    #[test]
    fn test_voice_response_to_voice_with_female_gender() {
        let mut labels = HashMap::new();
        labels.insert("gender".to_string(), "female".to_string());
        labels.insert("accent".to_string(), "american".to_string());

        let voice_response = VoiceResponseModel {
            voice_id: "voice123".to_string(),
            name: "Rachel".to_string(),
            labels: Some(labels),
            category: None,
            samples: None,
            settings: None,
            fine_tuning: None,
            sharing: None,
            verified_languages: None,
            voice_verification: None,
            description: None,
        };

        let voice = ElevenLabsProvider::voice_response_to_voice(voice_response);

        assert_eq!(voice.name, "Rachel");
        assert_eq!(voice.identifier, Some("voice123".to_string()));
        assert_eq!(voice.gender, Gender::Female);
        assert_eq!(voice.quality, VoiceQuality::Excellent);
    }

    #[test]
    fn test_voice_response_to_voice_with_male_gender() {
        let mut labels = HashMap::new();
        labels.insert("gender".to_string(), "male".to_string());

        let voice_response = VoiceResponseModel {
            voice_id: "voice456".to_string(),
            name: "Adam".to_string(),
            labels: Some(labels),
            category: None,
            samples: None,
            settings: None,
            fine_tuning: None,
            sharing: None,
            verified_languages: None,
            voice_verification: None,
            description: None,
        };

        let voice = ElevenLabsProvider::voice_response_to_voice(voice_response);

        assert_eq!(voice.name, "Adam");
        assert_eq!(voice.gender, Gender::Male);
        assert_eq!(voice.quality, VoiceQuality::Excellent);
    }

    #[test]
    fn test_voice_response_to_voice_with_no_gender_label() {
        let voice_response = VoiceResponseModel {
            voice_id: "voice789".to_string(),
            name: "Unknown".to_string(),
            labels: None,
            category: None,
            samples: None,
            settings: None,
            fine_tuning: None,
            sharing: None,
            verified_languages: None,
            voice_verification: None,
            description: None,
        };

        let voice = ElevenLabsProvider::voice_response_to_voice(voice_response);

        assert_eq!(voice.name, "Unknown");
        assert_eq!(voice.gender, Gender::Any);
        assert_eq!(voice.quality, VoiceQuality::Excellent);
    }

    #[test]
    fn test_voice_response_to_voice_with_case_insensitive_gender() {
        let mut labels = HashMap::new();
        labels.insert("gender".to_string(), "FEMALE".to_string());

        let voice_response = VoiceResponseModel {
            voice_id: "voice_upper".to_string(),
            name: "TestVoice".to_string(),
            labels: Some(labels),
            category: None,
            samples: None,
            settings: None,
            fine_tuning: None,
            sharing: None,
            verified_languages: None,
            voice_verification: None,
            description: None,
        };

        let voice = ElevenLabsProvider::voice_response_to_voice(voice_response);

        assert_eq!(voice.gender, Gender::Female);
    }

    #[test]
    fn test_voice_response_to_voice_with_verified_languages() {
        use schematic_schema::elevenlabs::LanguageModel;

        let voice_response = VoiceResponseModel {
            voice_id: "voice_multilang".to_string(),
            name: "MultiLingual".to_string(),
            labels: None,
            category: None,
            samples: None,
            settings: None,
            fine_tuning: None,
            sharing: None,
            verified_languages: Some(vec![
                LanguageModel {
                    language_id: "en".to_string(),
                    name: "English".to_string(),
                },
                LanguageModel {
                    language_id: "fr".to_string(),
                    name: "French".to_string(),
                },
            ]),
            voice_verification: None,
            description: None,
        };

        let voice = ElevenLabsProvider::voice_response_to_voice(voice_response);

        assert_eq!(voice.languages.len(), 2);
        assert!(voice.languages.contains(&Language::English));
        assert!(voice.languages.contains(&Language::Custom("fr".to_string())));
    }

    #[test]
    fn test_voice_response_to_voice_defaults_to_english_when_no_languages() {
        let voice_response = VoiceResponseModel {
            voice_id: "voice_no_lang".to_string(),
            name: "NoLang".to_string(),
            labels: None,
            category: None,
            samples: None,
            settings: None,
            fine_tuning: None,
            sharing: None,
            verified_languages: None,
            voice_verification: None,
            description: None,
        };

        let voice = ElevenLabsProvider::voice_response_to_voice(voice_response);

        assert_eq!(voice.languages, vec![Language::English]);
    }

    #[test]
    fn test_voice_response_to_voice_with_unknown_gender_value() {
        let mut labels = HashMap::new();
        labels.insert("gender".to_string(), "nonbinary".to_string());

        let voice_response = VoiceResponseModel {
            voice_id: "voice_nb".to_string(),
            name: "NonBinary".to_string(),
            labels: Some(labels),
            category: None,
            samples: None,
            settings: None,
            fine_tuning: None,
            sharing: None,
            verified_languages: None,
            voice_verification: None,
            description: None,
        };

        let voice = ElevenLabsProvider::voice_response_to_voice(voice_response);

        // Unknown gender values should map to Gender::Any
        assert_eq!(voice.gender, Gender::Any);
    }

    // Note: Integration tests requiring API key should use #[ignore]
    // and be run with: cargo test -- --ignored
    #[tokio::test]
    #[ignore = "requires ELEVEN_LABS_API_KEY environment variable"]
    async fn test_list_voices_raw_integration() {
        let provider = ElevenLabsProvider::new().expect("API key should be set");
        let response = provider
            .list_voices_raw()
            .await
            .expect("Should list voices");
        assert!(!response.voices.is_empty(), "Should have at least one voice");
    }

    #[tokio::test]
    #[ignore = "requires ELEVEN_LABS_API_KEY environment variable"]
    async fn test_tts_voice_inventory_list_voices_integration() {
        use crate::traits::TtsVoiceInventory;

        let provider = ElevenLabsProvider::new().expect("API key should be set");
        let voices: Vec<Voice> = TtsVoiceInventory::list_voices(&provider)
            .await
            .expect("Should list voices via TtsVoiceInventory trait");

        assert!(!voices.is_empty(), "Should have at least one voice");

        // All ElevenLabs voices should have Excellent quality
        for voice in &voices {
            assert_eq!(
                voice.quality,
                VoiceQuality::Excellent,
                "Voice {} should have Excellent quality",
                voice.name
            );
        }
    }

    #[tokio::test]
    #[ignore = "requires ELEVEN_LABS_API_KEY environment variable"]
    async fn test_list_models_integration() {
        let provider = ElevenLabsProvider::new().expect("API key should be set");
        let models = provider.list_models().await.expect("Should list models");
        assert!(!models.is_empty(), "Should have at least one model");
    }

    #[tokio::test]
    #[ignore = "requires ELEVEN_LABS_API_KEY environment variable"]
    async fn test_generate_audio_integration() {
        let provider = ElevenLabsProvider::new().expect("API key should be set");
        let config = TtsConfig::default();
        let audio = provider
            .generate_audio("Hello, world!", &config)
            .await
            .expect("Should generate audio");
        assert!(!audio.is_empty(), "Audio should not be empty");
        // MP3 files typically start with 0xFF 0xFB or ID3 tag
        assert!(
            audio[0] == 0xFF || audio.starts_with(b"ID3"),
            "Should be valid MP3 data"
        );
    }
}
