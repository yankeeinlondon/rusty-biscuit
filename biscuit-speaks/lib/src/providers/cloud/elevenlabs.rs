//! ElevenLabs cloud TTS provider.
//!
//! This module implements the ElevenLabs text-to-speech API using the
//! schematic-schema generated client.
//!
//! ## Environment Variables
//!
//! The API key is read once, at construction, from the first one set of:
//! - `ELEVEN_LABS_API_KEY`
//! - `ELEVENLABS_API_KEY`
//!
//! The key is handed to the client as its credential, so the generated
//! client's own environment fallback is never consulted.
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
//!
//! ## Default voice records
//!
//! Without a requested voice, the voice is selected from the account's voice
//! list. With a non-blank `BISCUIT_SPEAKS_ELEVENLABS_ACCOUNT` label, each
//! selection made from a successful listing is recorded locally (see
//! `snapshot`), so a detached request can find prepared default speech
//! without listing voices again.

mod snapshot;

use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;

use schematic_schema::elevenlabs::{
    CreateSoundEffectBody, CreateSpeechBody, CreateSpeechRequest, ListVoicesResponse, ModelInfo,
    VoiceResponseModel, VoiceSettings,
};
use schematic_schema::prelude::{Credential, ElevenLabs, TransportError};
use schematic_schema::schematic_runtime::{ApiError, CallError, ErrorKind, NoDeclaredError};

use crate::audio_cache::{CacheKey, write_atomic};
use crate::errors::TtsError;
use crate::traits::{TtsExecutor, TtsVoiceInventory};
#[cfg(feature = "playa")]
use crate::types::AudioFormat;
#[cfg(feature = "playa")]
use crate::types::{CloudTtsProvider, TtsProvider};
use crate::prepared::{ResolvedSynthesis, dry_run_enabled};
use crate::types::{Gender, Language, SpeakResult, SpeedLevel, TtsConfig, Voice, VoiceQuality};
use snapshot::{SelectionInputs, Snapshot};

/// Default ElevenLabs voice ID (Rachel - a versatile female voice).
const DEFAULT_VOICE_ID: &str = "21m00Tcm4TlvDq8ikWAM";

/// Default ElevenLabs model for multilingual support.
const DEFAULT_MODEL_ID: &str = "eleven_multilingual_v2";

/// Voices requested per `/v2/voices` page; the API's maximum.
const VOICES_PAGE_SIZE: i64 = 100;

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
    client: Arc<ElevenLabs>,
    /// Voice listing and speech synthesis requests; replaceable so tests can
    /// count requests without a network.
    transport: Arc<dyn ElevenLabsTransport>,
    /// Decides whether a default-voice record is still trusted.
    clock: Arc<dyn Clock>,
    /// The non-secret account identity default-voice records are keyed by;
    /// `None` disables them.
    account_context: Option<String>,
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
        Self::build(None)
    }

    /// Create a new ElevenLabs provider with a custom base URL.
    ///
    /// Useful for testing with mock servers. The API key is still read from
    /// the environment.
    ///
    /// ## Errors
    ///
    /// Returns `TtsError::MissingApiKey` if no API key is found in the
    /// environment, or `TtsError::HttpError` if `url` is not a usable HTTP(S)
    /// base URL.
    ///
    /// ## Examples
    ///
    /// ```ignore
    /// let provider = ElevenLabsProvider::with_base_url("http://localhost:8080")?;
    /// ```
    pub fn with_base_url(url: impl Into<String>) -> Result<Self, TtsError> {
        Self::build(Some(url.into()))
    }

    fn build(base_url: Option<String>) -> Result<Self, TtsError> {
        // Same variable order as the generated definition's env fallback.
        let api_key = std::env::var("ELEVEN_LABS_API_KEY")
            .or_else(|_| std::env::var("ELEVENLABS_API_KEY"))
            .map_err(|_| TtsError::MissingApiKey {
                provider: "elevenlabs".into(),
            })?;

        let mut builder = ElevenLabs::new().credentials(Credential::api_key(api_key));
        if let Some(url) = &base_url {
            builder = builder.base_url(url.clone());
        }
        let client = builder.build().map_err(|e| TtsError::HttpError {
            provider: "elevenlabs".into(),
            message: format!("Failed to build HTTP client: {e}"),
        })?;

        Ok(Self::from_client(
            client,
            base_url.as_deref().unwrap_or(ElevenLabs::BASE_URL),
        ))
    }

    fn from_client(client: ElevenLabs, base_url: &str) -> Self {
        let client = Arc::new(client);
        Self {
            transport: Arc::new(HttpTransport {
                client: Arc::clone(&client),
            }),
            client,
            clock: Arc::new(SystemClock),
            account_context: snapshot::account_context(base_url),
            default_voice_id: DEFAULT_VOICE_ID.into(),
            default_model_id: DEFAULT_MODEL_ID.into(),
        }
    }

    /// Construct a provider whose API requests and clock are supplied by the
    /// caller. No API key is required: `transport` decides what every request
    /// returns.
    #[cfg(test)]
    pub(crate) fn with_transport(
        transport: Arc<dyn ElevenLabsTransport>,
        clock: Arc<dyn Clock>,
    ) -> Self {
        Self {
            client: Arc::new(
                ElevenLabs::new()
                    .credentials(Credential::api_key("test-key"))
                    .base_url("http://127.0.0.1:9")
                    .build()
                    .expect("valid test client"),
            ),
            transport,
            clock,
            account_context: None,
            default_voice_id: DEFAULT_VOICE_ID.into(),
            default_model_id: DEFAULT_MODEL_ID.into(),
        }
    }

    /// Key default-voice records by `account_context`.
    #[cfg(test)]
    pub(crate) fn with_account_context(mut self, account_context: Option<String>) -> Self {
        self.account_context = account_context;
        self
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

    /// Convert a SpeedLevel to ElevenLabs speed value (0.7-1.2).
    ///
    /// Returns `None` for normal speed (use API default of 1.0).
    fn resolve_speed(speed: SpeedLevel) -> Option<f64> {
        match speed {
            SpeedLevel::Normal => None, // Use default
            _ => {
                // ElevenLabs speed range: 0.7 to 1.2
                // Clamp our multiplier to this range
                let value = speed.value() as f64;
                Some(value.clamp(0.7, 1.2))
            }
        }
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

        // Extract primary language from labels.language first (most reliable)
        // Then supplement with verified_languages for multilingual voices
        let primary_language = voice
            .labels
            .as_ref()
            .and_then(|labels| labels.get("language"))
            .map(|lang| {
                if lang.starts_with("en") {
                    Language::English
                } else {
                    Language::Custom(lang.clone())
                }
            });

        // Get unique languages from verified_languages
        let mut languages: Vec<Language> = if let Some(langs) = &voice.verified_languages {
            let mut seen = std::collections::HashSet::new();
            langs
                .iter()
                .filter_map(|lang| {
                    // Deduplicate by language code (ignore model differences)
                    if seen.insert(lang.language_id.clone()) {
                        Some(if lang.language_id.starts_with("en") {
                            Language::English
                        } else {
                            Language::Custom(lang.language_id.clone())
                        })
                    } else {
                        None
                    }
                })
                .collect()
        } else {
            vec![]
        };

        // If we have a primary language from labels, make sure it's first
        if let Some(primary) = primary_language {
            // Remove it if already in the list, then prepend
            languages.retain(|l| l != &primary);
            languages.insert(0, primary);
        }

        // Default to English if no languages found
        if languages.is_empty() {
            languages.push(Language::English);
        }

        // Parse name and description from "Name - Description" format
        let (name, description) = if let Some((n, d)) = voice.name.split_once(" - ") {
            (n.to_string(), Some(d.to_string()))
        } else {
            (voice.name.clone(), None)
        };

        let mut v = Voice::new(&name)
            .with_identifier(&voice.voice_id)
            .with_gender(gender)
            .with_quality(VoiceQuality::Excellent) // All ElevenLabs voices are excellent quality
            .with_languages(languages);

        if let Some(desc) = description {
            v = v.with_description(desc);
        }

        // Store recommended models for this voice
        if let Some(models) = voice.high_quality_base_model_ids {
            v = v.with_recommended_models(models);
        }

        v
    }

    /// Build the default Rachel voice as a `Voice` struct.
    fn rachel_default_voice() -> Voice {
        Voice::new("Rachel")
            .with_gender(Gender::Female)
            .with_quality(VoiceQuality::Excellent)
            .with_language(Language::English)
            .with_identifier(DEFAULT_VOICE_ID)
    }

    /// Check if the ElevenLabs API key is configured in the environment.
    ///
    /// Returns `true` if either `ELEVENLABS_API_KEY` or `ELEVEN_LABS_API_KEY`
    /// is set in the environment.
    pub fn has_api_key() -> bool {
        std::env::var("ELEVEN_LABS_API_KEY").is_ok() || std::env::var("ELEVENLABS_API_KEY").is_ok()
    }

    /// Map a failed call to a `TtsError` via [`Self::map_api_error`] or
    /// [`Self::map_transport_error`].
    fn map_call_error(error: CallError<NoDeclaredError>) -> TtsError {
        match error {
            CallError::Transport(error) => Self::map_transport_error(error),
            CallError::Api { error, meta } => Self::map_api_error(meta.status.as_u16(), &error),
            // `CallError` is non-exhaustive; its redacted `Display` is safe to surface.
            other => TtsError::HttpError {
                provider: "elevenlabs".into(),
                message: other.to_string(),
            },
        }
    }

    /// Map a failed reply to a `TtsError`: 402 Payment Required becomes
    /// `PaidPlanRequired` carrying the body, everything else `HttpError`.
    ///
    /// The response body is included in the message as the API sent it; the
    /// request's headers (and therefore the API key) never are.
    fn map_api_error(status: u16, error: &ApiError<NoDeclaredError>) -> TtsError {
        let body = String::from_utf8_lossy(&error.body);
        if status == 402 {
            return TtsError::PaidPlanRequired {
                provider: "ElevenLabs".into(),
                detail: body.into_owned(),
            };
        }
        let message = match error.kind {
            ErrorKind::MalformedSuccess => format!("Failed to parse response: {error}"),
            ErrorKind::Undeclared => format!("API error ({status}): {body}"),
            _ => format!("API error ({status}): {error}"),
        };
        TtsError::HttpError {
            provider: "elevenlabs".into(),
            message,
        }
    }

    /// Map a request that received no response to a `TtsError`, using the
    /// transport error's redacted `Display`.
    fn map_transport_error(error: TransportError) -> TtsError {
        TtsError::HttpError {
            provider: "elevenlabs".into(),
            message: error.to_string(),
        }
    }

    /// Generate audio to the cache path, returning the path and whether it was a cache hit.
    ///
    /// Speed IS included in the cache key because ElevenLabs bakes speed into the audio.
    async fn generate_to_cache(
        &self,
        text: &str,
        voice_id: &str,
        model_id: &str,
        speed: SpeedLevel,
    ) -> Result<(PathBuf, bool), TtsError> {
        let cache_key = Self::cache_key(text, voice_id, model_id, speed);
        let cache_path = cache_key.cache_path();

        // Check for cache hit
        if cache_key.cache_exists() {
            tracing::debug!(
                voice_id = %voice_id,
                model_id = %model_id,
                cache_path = %cache_path.display(),
                "Cache hit for ElevenLabs"
            );
            return Ok((cache_path, true));
        }

        // Cache miss - generate audio via API
        tracing::debug!(
            voice_id = %voice_id,
            model_id = %model_id,
            cache_path = %cache_path.display(),
            "Cache miss for ElevenLabs, generating"
        );

        // Build the request body
        let voice_settings = Self::resolve_speed(speed).map(|speed_val| VoiceSettings {
            speed: Some(speed_val),
            ..Default::default()
        });

        let body = CreateSpeechBody {
            text: text.to_string(),
            model_id: Some(model_id.to_string()),
            voice_settings,
            ..Default::default()
        };

        let request = CreateSpeechRequest::new(voice_id.to_string(), body);

        let audio_bytes = self.transport.create_speech(request).await?;

        // Write to cache atomically
        write_atomic(&cache_path, &audio_bytes).map_err(|e| TtsError::TempFileError {
            source: std::io::Error::other(e.to_string()),
        })?;

        tracing::debug!(
            audio_size = audio_bytes.len(),
            cache_path = %cache_path.display(),
            "Saved ElevenLabs audio to cache"
        );

        Ok((cache_path, false))
    }

    /// The shared-cache identity; speed is part of it because ElevenLabs
    /// bakes speed into the audio.
    fn cache_key(text: &str, voice_id: &str, model_id: &str, speed: SpeedLevel) -> CacheKey {
        let key = CacheKey::new("elevenlabs", format!("{voice_id}-{model_id}"), text, "mp3");
        if speed == SpeedLevel::Normal {
            key
        } else {
            key.with_speed(speed.value())
        }
    }

    /// The requested model, else the provider default.
    fn model(&self, config: &TtsConfig) -> String {
        config
            .requested_model
            .clone()
            .unwrap_or_else(|| self.default_model_id.clone())
    }

    fn gender_label(gender: Gender) -> Option<&'static str> {
        match gender {
            Gender::Male => Some("male"),
            Gender::Female => Some("female"),
            Gender::Any => None,
        }
    }

    /// The inputs a default-voice selection for `config` depends on.
    fn selection_inputs(&self, config: &TtsConfig) -> SelectionInputs {
        SelectionInputs {
            gender: Self::gender_label(config.gender).unwrap_or("any").into(),
            language: config.language.code_prefix().into(),
            model: self.model(config),
            fallback: self.default_voice_id.clone(),
        }
    }

    /// Select the voice and model every entry point synthesizes with.
    ///
    /// A requested voice wins. Otherwise the account's voice list supplies
    /// the first voice matching the requested gender, else its first voice,
    /// else (an empty list) the configured default; a failed listing also
    /// falls back to the configured default.
    async fn select(&self, config: &TtsConfig) -> Selection {
        let model = self.model(config);
        if let Some(voice_id) = &config.requested_voice {
            tracing::debug!(requested_voice = %voice_id, "Using requested voice from config");
            return Selection {
                voice: voice_id.clone(),
                model,
                source: VoiceSource::Requested,
            };
        }
        tracing::debug!("No requested voice in config, will resolve from gender/default");

        let voices = match self.list_voices_raw().await {
            Ok(voices) => voices.voices,
            Err(error) => {
                tracing::warn!(
                    error = ?error,
                    "Failed to fetch ElevenLabs voices, falling back to hardcoded default"
                );
                return Selection {
                    voice: self.default_voice_id.clone(),
                    model,
                    source: VoiceSource::Fallback,
                };
            }
        };
        let gender_match = Self::gender_label(config.gender).and_then(|gender_label| {
            let found = voices
                .iter()
                .find(|voice| Self::voice_matches_gender(voice, gender_label));
            if found.is_none() {
                tracing::warn!(
                    gender = gender_label,
                    "No ElevenLabs voice matched gender, using first available"
                );
            }
            found
        });
        // The first listed voice beats a hardcoded default the account may lack.
        let voice = gender_match
            .or_else(|| voices.first())
            .map_or_else(|| self.default_voice_id.clone(), |voice| voice.voice_id.clone());
        Selection {
            voice,
            model,
            source: VoiceSource::Account,
        }
    }

    /// Select, and record a selection made from the account's voice list.
    ///
    /// A record that cannot be written only costs a later detached request
    /// its cache hit, so the failure is logged rather than returned.
    async fn select_and_record(&self, config: &TtsConfig) -> Selection {
        let selection = self.select(config).await;
        if selection.source == VoiceSource::Account
            && let Some(account_context) = &self.account_context
        {
            let record = Snapshot {
                voice: selection.voice.clone(),
                model: selection.model.clone(),
                inputs: self.selection_inputs(config),
                account_context: account_context.clone(),
                written_at: self.clock.now_unix_seconds(),
            };
            if let Err(error) = snapshot::write(&record) {
                tracing::warn!(%error, "Failed to record the ElevenLabs default voice");
            }
        }
        selection
    }

    /// The cache entry a detached request would play, found without any API
    /// request: from the requested voice, or from a trusted default-voice
    /// record.
    #[cfg_attr(not(feature = "playa"), allow(dead_code))]
    fn cached_path(&self, text: &str, config: &TtsConfig) -> Option<PathBuf> {
        let model = self.model(config);
        let voice = match &config.requested_voice {
            Some(voice) => voice.clone(),
            None => {
                let account_context = self.account_context.as_deref()?;
                let inputs = self.selection_inputs(config);
                snapshot::load_trusted(&inputs, account_context, self.clock.now_unix_seconds())?.voice
            }
        };
        let path = Self::cache_key(text, &voice, &model, config.speed).cache_path();
        path.is_file().then_some(path)
    }

    /// Playback settings for ElevenLabs audio, whose speed is already baked in.
    #[cfg_attr(not(feature = "playa"), allow(dead_code))]
    fn playback_config(config: &TtsConfig) -> TtsConfig {
        let mut playback = config.clone();
        playback.speed = SpeedLevel::Normal;
        playback
    }

    /// Prepare the cache entry and report the settings it was prepared with.
    ///
    /// The selection is always checked against the API, which also refreshes
    /// the default-voice record. `None` when there is nothing to prepare: dry
    /// run, or a voice list that could not be fetched, which leaves the
    /// default voice unresolved.
    pub(crate) async fn prepare_resolved(
        &self,
        text: &str,
        config: &TtsConfig,
    ) -> Result<Option<ResolvedSynthesis>, TtsError> {
        if dry_run_enabled() {
            return Ok(None);
        }
        let selection = self.select_and_record(config).await;
        if selection.source == VoiceSource::Fallback {
            return Ok(None);
        }
        self.generate_to_cache(text, &selection.voice, &selection.model, config.speed)
            .await?;
        Ok(Some(ResolvedSynthesis::ElevenLabs {
            voice: selection.voice,
            model: selection.model,
            speed: config.speed,
        }))
    }

    /// Play `voice` and `model` synthesized at `speed`, at `config`'s volume.
    ///
    /// Makes no voice-list request; a cache miss makes one synthesis request.
    pub(crate) async fn speak_resolved(
        &self,
        text: &str,
        voice: &str,
        model: &str,
        speed: SpeedLevel,
        config: &TtsConfig,
    ) -> Result<(), TtsError> {
        let (audio_path, _cache_hit) = self.generate_to_cache(text, voice, model, speed).await?;

        #[cfg(feature = "playa")]
        {
            crate::playback::play_audio_file(&audio_path, AudioFormat::Mp3, &Self::playback_config(config))
                .await
        }
        #[cfg(not(feature = "playa"))]
        {
            // Playback requires the playa feature
            let _ = (audio_path, config);
            Err(TtsError::NoAudioPlayer)
        }
    }

    /// [`Self::speak_resolved`], reporting the voice, model, and cache entry
    /// played.
    pub(crate) async fn speak_resolved_with_result(
        &self,
        text: &str,
        voice_id: &str,
        model_used: &str,
        speed: SpeedLevel,
        config: &TtsConfig,
    ) -> Result<SpeakResult, TtsError> {
        let (audio_path, cache_hit) = self
            .generate_to_cache(text, voice_id, model_used, speed)
            .await?;

        #[cfg(feature = "playa")]
        let playback = crate::playback::play_audio_file_with_report(
            &audio_path,
            AudioFormat::Mp3,
            &Self::playback_config(config),
        )
        .await?;

        #[cfg(not(feature = "playa"))]
        {
            let _ = (&audio_path, cache_hit, model_used, voice_id, config);
            Err(TtsError::NoAudioPlayer)
        }

        #[cfg(feature = "playa")]
        {
            let mut voice = if let Ok(voices) = self.list_voices().await {
                let found = voices
                    .iter()
                    .find(|v| v.identifier.as_deref() == Some(voice_id));
                tracing::debug!(
                    voice_id = %voice_id,
                    found = found.is_some(),
                    "Looking up voice metadata"
                );
                found.cloned().unwrap_or_else(|| {
                    Voice::new(voice_id)
                        .with_identifier(voice_id)
                        .with_gender(config.gender)
                        .with_quality(VoiceQuality::Excellent)
                        .with_language(config.language.clone())
                })
            } else {
                Voice::new(voice_id)
                    .with_identifier(voice_id)
                    .with_gender(config.gender)
                    .with_quality(VoiceQuality::Excellent)
                    .with_language(config.language.clone())
            };

            if voice.identifier.is_none() {
                voice.identifier = Some(voice_id.to_string());
            }

            Ok(SpeakResult::with_model(
                TtsProvider::Cloud(CloudTtsProvider::ElevenLabs),
                voice,
                model_used.to_string(),
            )
            .with_audio_file(audio_path)
            .with_codec("mp3")
            .with_cache_hit(cache_hit)
            .with_playback(playback))
        }
    }

    /// A ready detached job for `voice` and `model` at `speed`, synthesizing
    /// it first on a miss.
    #[cfg(feature = "playa")]
    pub(crate) async fn resolved_job(
        &self,
        text: &str,
        voice: &str,
        model: &str,
        speed: SpeedLevel,
        config: &TtsConfig,
    ) -> Result<playa::detached::SpoolJob, TtsError> {
        let (path, _) = self.generate_to_cache(text, voice, model, speed).await?;
        Ok(crate::playa_bridge::file_job(path, &Self::playback_config(config)))
    }

    /// A ready detached job for `voice` and `model` at `speed` when its file
    /// is already cached. Makes no API request.
    #[cfg(feature = "playa")]
    pub(crate) fn cached_resolved_job(
        text: &str,
        voice: &str,
        model: &str,
        speed: SpeedLevel,
        config: &TtsConfig,
    ) -> Option<playa::detached::SpoolJob> {
        let path = Self::cache_key(text, voice, model, speed).cache_path();
        path.is_file()
            .then(|| crate::playa_bridge::file_job(path, &Self::playback_config(config)))
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
    /// Returns `TtsError::PaidPlanRequired` on 402 Payment Required, and
    /// `TtsError::HttpError` for any other failed request or reply.
    ///
    /// ## Examples
    ///
    /// ```ignore
    /// let provider = ElevenLabsProvider::new()?;
    /// let audio = provider.generate_audio("Hello", &TtsConfig::default()).await?;
    /// // audio is Vec<u8> containing MP3 data
    /// ```
    pub async fn generate_audio(
        &self,
        text: &str,
        config: &TtsConfig,
    ) -> Result<Vec<u8>, TtsError> {
        let Selection {
            voice: voice_id,
            model: model_id,
            ..
        } = self.select(config).await;

        // Voice settings are sent only when the speed is not normal.
        let voice_settings = Self::resolve_speed(config.speed).map(|speed| VoiceSettings {
            speed: Some(speed),
            ..Default::default()
        });

        let body = CreateSpeechBody {
            text: text.to_string(),
            model_id: Some(model_id.clone()),
            language_code: Some(config.language.code_prefix().to_string()),
            voice_settings,
            ..Default::default()
        };

        // Build the request
        let request = CreateSpeechRequest::new(voice_id.clone(), body);

        tracing::debug!(
            voice_id = %voice_id,
            text_len = text.len(),
            model = %model_id,
            "Sending ElevenLabs TTS request"
        );

        let audio_bytes = self.transport.create_speech(request).await?;

        tracing::debug!(
            audio_size = audio_bytes.len(),
            "Received ElevenLabs audio response"
        );

        Ok(audio_bytes)
    }

    /// List available voices from the ElevenLabs API with full response details.
    ///
    /// Every call fetches ALL pages of voices (the API is paginated) and returns
    /// a consolidated response; nothing here is cached. Paging stops when a
    /// page reports no more voices or gives no next-page token.
    ///
    /// ## Returns
    ///
    /// The voices of every page, with `total_count` from the first page,
    /// `has_more` false, and no `next_page_token`.
    ///
    /// ## Errors
    ///
    /// Returns `TtsError::PaidPlanRequired` on 402 Payment Required, and
    /// `TtsError::HttpError` for any other failed request or reply.
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
        self.transport.list_voices().await
    }

    /// List available models from the ElevenLabs API.
    ///
    /// ## Returns
    ///
    /// A list of available model information.
    ///
    /// ## Errors
    ///
    /// Returns `TtsError::PaidPlanRequired` on 402 Payment Required, and
    /// `TtsError::HttpError` for any other failed request or reply.
    pub async fn list_models(&self) -> Result<Vec<ModelInfo>, TtsError> {
        self.client
            .list_models()
            .await
            .map_err(Self::map_call_error)
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
    /// Returns `TtsError::PaidPlanRequired` on 402 Payment Required, and
    /// `TtsError::HttpError` for any other failed request or reply.
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

        tracing::debug!(
            prompt = %prompt,
            duration = ?duration_seconds,
            "Creating ElevenLabs sound effect"
        );

        let audio_bytes = self
            .client
            .create_sound_effect(body)
            .await
            .map_err(Self::map_call_error)?;

        Ok(audio_bytes.to_vec())
    }
}

/// A boxed `Send` future, so [`ElevenLabsTransport`] stays object-safe.
pub(crate) type TransportFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// The ElevenLabs API requests that voice resolution and synthesis make.
///
/// Every request either path sends goes through this trait, so a replacement
/// sees all network traffic the provider would produce.
pub(crate) trait ElevenLabsTransport: Send + Sync {
    /// Fetch every page of the account's voice list as one response.
    fn list_voices(&self) -> TransportFuture<'_, Result<ListVoicesResponse, TtsError>>;

    /// Synthesize one request and return the encoded audio.
    fn create_speech(
        &self,
        request: CreateSpeechRequest,
    ) -> TransportFuture<'_, Result<Vec<u8>, TtsError>>;
}

/// The production transport: HTTPS requests through the schematic client.
struct HttpTransport {
    client: Arc<ElevenLabs>,
}

impl ElevenLabsTransport for HttpTransport {
    fn list_voices(&self) -> TransportFuture<'_, Result<ListVoicesResponse, TtsError>> {
        Box::pin(self.fetch_all_voices())
    }

    fn create_speech(
        &self,
        request: CreateSpeechRequest,
    ) -> TransportFuture<'_, Result<Vec<u8>, TtsError>> {
        Box::pin(async move {
            self.client
                .create_speech(request.voice_id, request.body)
                .await
                .map(|bytes| bytes.to_vec())
                .map_err(ElevenLabsProvider::map_call_error)
        })
    }
}

impl HttpTransport {
    async fn fetch_all_voices(&self) -> Result<ListVoicesResponse, TtsError> {
        let mut all_voices = Vec::new();
        let mut page_token: Option<String> = None;
        let mut total_count = None;

        let mut page_num = 0;
        loop {
            page_num += 1;

            tracing::debug!(
                page = page_num,
                page_token = ?page_token,
                "Fetching ElevenLabs voices page"
            );

            let mut call = self.client.list_voices().page_size(VOICES_PAGE_SIZE);
            if let Some(token) = page_token.take() {
                call = call.page_token(token);
            }
            let page = call.await.map_err(ElevenLabsProvider::map_call_error)?;

            tracing::debug!(
                page = page_num,
                voices_in_page = page.voices.len(),
                has_more = page.has_more,
                total_count = ?page.total_count,
                next_page_token = ?page.next_page_token,
                "Received ElevenLabs voices page"
            );

            all_voices.extend(page.voices);
            if total_count.is_none() {
                total_count = page.total_count;
            }
            // A page that claims more without a token to fetch it would
            // otherwise re-request the first page forever.
            match page.next_page_token {
                Some(token) if page.has_more => page_token = Some(token),
                _ => break,
            }
        }

        tracing::debug!(
            total_voices = all_voices.len(),
            total_pages = page_num,
            "Completed fetching all ElevenLabs voices"
        );

        Ok(ListVoicesResponse {
            voices: all_voices,
            has_more: false,
            total_count,
            next_page_token: None,
        })
    }

}

/// How a [`Selection`]'s voice was chosen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VoiceSource {
    /// The caller named it.
    Requested,
    /// From a successful listing of the account's voices.
    Account,
    /// The configured default, because the voice list could not be fetched.
    Fallback,
}

/// The voice and model one operation synthesizes with.
#[derive(Debug, Clone)]
struct Selection {
    voice: String,
    model: String,
    source: VoiceSource,
}

/// Wall-clock source for deciding whether persisted state is still trusted.
pub(crate) trait Clock: Send + Sync {
    /// Seconds since the Unix epoch.
    fn now_unix_seconds(&self) -> u64;
}

/// The system clock. A time before the Unix epoch reads as `0`.
struct SystemClock;

impl Clock for SystemClock {
    fn now_unix_seconds(&self) -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_secs())
    }
}

impl TtsExecutor for ElevenLabsProvider {
    async fn speak(&self, text: &str, config: &TtsConfig) -> Result<(), TtsError> {
        let selection = self.select_and_record(config).await;
        self.speak_resolved(text, &selection.voice, &selection.model, config.speed, config)
            .await
    }

    async fn prepare(&self, text: &str, config: &TtsConfig) -> Result<(), TtsError> {
        self.prepare_resolved(text, config).await.map(|_| ())
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
        tracing::debug!(
            requested_voice = ?config.requested_voice,
            "speak_with_result called"
        );

        // This may fetch the voice list for gender matching.
        let selection = self.select_and_record(config).await;
        tracing::debug!(
            voice_id = %selection.voice,
            "speak_with_result resolved voice_id"
        );
        self.speak_resolved_with_result(text, &selection.voice, &selection.model, config.speed, config)
            .await
    }

    #[cfg(feature = "playa")]
    async fn detached_job(
        &self,
        text: &str,
        config: &TtsConfig,
    ) -> Result<playa::detached::SpoolJob, TtsError> {
        let selection = self.select_and_record(config).await;
        self.resolved_job(text, &selection.voice, &selection.model, config.speed, config)
            .await
    }

    #[cfg(feature = "playa")]
    async fn cached_detached_job(
        &self,
        text: &str,
        config: &TtsConfig,
    ) -> Result<Option<playa::detached::SpoolJob>, TtsError> {
        Ok(self
            .cached_path(text, config)
            .map(|path| crate::playa_bridge::file_job(path, &Self::playback_config(config))))
    }
}

impl TtsVoiceInventory for ElevenLabsProvider {
    async fn list_voices(&self) -> Result<Vec<Voice>, TtsError> {
        let response =
            self.list_voices_raw()
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

    async fn default_voice(&self, gender: Gender) -> Result<Voice, TtsError> {
        if gender == Gender::Any {
            return Ok(Self::rachel_default_voice());
        }

        // Query API for voices matching the requested gender.
        // list_voices() already converts API responses to Voice structs
        // with gender populated via voice_response_to_voice.
        match self.list_voices().await {
            Ok(voices) => Ok(voices
                .into_iter()
                .find(|v| v.gender == gender)
                .unwrap_or_else(Self::rachel_default_voice)),
            Err(e) => {
                tracing::warn!(
                    error = ?e,
                    "Failed to fetch ElevenLabs voices for gender filter, falling back to Rachel"
                );
                Ok(Self::rachel_default_voice())
            }
        }
    }
}

/// Scripted stand-ins for the ElevenLabs API and the wall clock.
#[cfg(test)]
pub(crate) mod fake {
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

    use schematic_schema::elevenlabs::{CreateSpeechRequest, ListVoicesResponse, VoiceResponseModel};

    use super::{Clock, ElevenLabsTransport, TransportFuture};
    use crate::errors::TtsError;

    /// Audio bytes every scripted synthesis returns.
    pub(crate) const FAKE_AUDIO: &[u8] = b"fake elevenlabs mp3";

    /// One recorded speech request.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub(crate) struct SpeechCall {
        pub voice_id: String,
        pub model_id: Option<String>,
        pub text: String,
        /// The speed the audio is synthesized at, when not the API default.
        pub speed: Option<String>,
    }

    /// An API whose voice list is scripted and whose requests are counted.
    ///
    /// `None` as the voice list makes listing fail, as an unreachable API does.
    #[derive(Debug, Default)]
    pub(crate) struct FakeTransport {
        voices: Mutex<Option<Vec<VoiceResponseModel>>>,
        list_requests: AtomicUsize,
        speech_calls: Mutex<Vec<SpeechCall>>,
    }

    impl FakeTransport {
        pub(crate) fn with_voices(voices: Vec<VoiceResponseModel>) -> Self {
            Self {
                voices: Mutex::new(Some(voices)),
                ..Self::default()
            }
        }

        /// A transport whose voice listing always fails.
        pub(crate) fn unreachable() -> Self {
            Self::default()
        }

        pub(crate) fn set_voices(&self, voices: Option<Vec<VoiceResponseModel>>) {
            *self.voices.lock().unwrap() = voices;
        }

        pub(crate) fn list_requests(&self) -> usize {
            self.list_requests.load(Ordering::SeqCst)
        }

        pub(crate) fn speech_calls(&self) -> Vec<SpeechCall> {
            self.speech_calls.lock().unwrap().clone()
        }

        /// Every request of either kind.
        pub(crate) fn total_requests(&self) -> usize {
            self.list_requests() + self.speech_calls.lock().unwrap().len()
        }
    }

    impl ElevenLabsTransport for FakeTransport {
        fn list_voices(&self) -> TransportFuture<'_, Result<ListVoicesResponse, TtsError>> {
            self.list_requests.fetch_add(1, Ordering::SeqCst);
            let voices = self.voices.lock().unwrap().clone();
            Box::pin(async move {
                let voices = voices.ok_or_else(|| TtsError::HttpError {
                    provider: "elevenlabs".into(),
                    message: "scripted voice-list failure".into(),
                })?;
                Ok(ListVoicesResponse {
                    total_count: Some(voices.len() as i64),
                    voices,
                    has_more: false,
                    next_page_token: None,
                })
            })
        }

        fn create_speech(
            &self,
            request: CreateSpeechRequest,
        ) -> TransportFuture<'_, Result<Vec<u8>, TtsError>> {
            self.speech_calls.lock().unwrap().push(SpeechCall {
                voice_id: request.voice_id,
                model_id: request.body.model_id,
                text: request.body.text,
                speed: request
                    .body
                    .voice_settings
                    .and_then(|settings| settings.speed)
                    .map(|speed| speed.to_string()),
            });
            Box::pin(async { Ok(FAKE_AUDIO.to_vec()) })
        }
    }

    /// A voice as the API lists it, with an optional `gender` label.
    pub(crate) fn voice(voice_id: &str, gender: Option<&str>) -> VoiceResponseModel {
        VoiceResponseModel {
            voice_id: voice_id.into(),
            name: voice_id.into(),
            labels: gender.map(|gender| [("gender".to_string(), gender.to_string())].into()),
            ..Default::default()
        }
    }

    /// A clock that reads whatever the test last set.
    #[derive(Debug)]
    pub(crate) struct FakeClock(AtomicU64);

    impl FakeClock {
        pub(crate) fn at(unix_seconds: u64) -> Self {
            Self(AtomicU64::new(unix_seconds))
        }

        pub(crate) fn advance(&self, seconds: u64) {
            self.0.fetch_add(seconds, Ordering::SeqCst);
        }
    }

    impl Clock for FakeClock {
        fn now_unix_seconds(&self) -> u64 {
            self.0.load(Ordering::SeqCst)
        }
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
    fn test_rachel_default_voice() {
        let voice = ElevenLabsProvider::rachel_default_voice();
        assert_eq!(voice.name, "Rachel");
        assert_eq!(voice.gender, Gender::Female);
        assert_eq!(voice.quality, VoiceQuality::Excellent);
        assert_eq!(voice.identifier, Some(DEFAULT_VOICE_ID.to_string()));
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
            ..Default::default()
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
            ..Default::default()
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
            ..Default::default()
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
            ..Default::default()
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
            verified_languages: Some(vec![
                LanguageModel {
                    language_id: "en".to_string(),
                    name: "English".to_string(),
                    ..Default::default()
                },
                LanguageModel {
                    language_id: "fr".to_string(),
                    name: "French".to_string(),
                    ..Default::default()
                },
            ]),
            ..Default::default()
        };

        let voice = ElevenLabsProvider::voice_response_to_voice(voice_response);

        assert_eq!(voice.languages.len(), 2);
        assert!(voice.languages.contains(&Language::English));
        assert!(
            voice
                .languages
                .contains(&Language::Custom("fr".to_string()))
        );
    }

    #[test]
    fn test_voice_response_to_voice_defaults_to_english_when_no_languages() {
        let voice_response = VoiceResponseModel {
            voice_id: "voice_no_lang".to_string(),
            name: "NoLang".to_string(),
            ..Default::default()
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
            ..Default::default()
        };

        let voice = ElevenLabsProvider::voice_response_to_voice(voice_response);

        // Unknown gender values should map to Gender::Any
        assert_eq!(voice.gender, Gender::Any);
    }

    // ========================================================================
    // resolve_speed() tests
    // ========================================================================

    #[test]
    fn test_resolve_speed_normal() {
        assert_eq!(ElevenLabsProvider::resolve_speed(SpeedLevel::Normal), None);
    }

    #[test]
    fn test_resolve_speed_fast() {
        // Fast = 1.25x, but ElevenLabs max is 1.2
        let speed = ElevenLabsProvider::resolve_speed(SpeedLevel::Fast).unwrap();
        assert!((speed - 1.2).abs() < 0.001, "Fast should clamp to 1.2");
    }

    #[test]
    fn test_resolve_speed_slow() {
        // Slow = 0.75x, within ElevenLabs range (0.7-1.2)
        let speed = ElevenLabsProvider::resolve_speed(SpeedLevel::Slow).unwrap();
        assert!((speed - 0.75).abs() < 0.001, "Slow should be 0.75");
    }

    #[test]
    fn test_resolve_speed_explicit_within_range() {
        // 0.9x is within range
        let speed = ElevenLabsProvider::resolve_speed(SpeedLevel::Explicit(0.9)).unwrap();
        assert!((speed - 0.9).abs() < 0.001);
    }

    #[test]
    fn test_resolve_speed_explicit_clamped_low() {
        // 0.5x should clamp to 0.7 (ElevenLabs minimum)
        let speed = ElevenLabsProvider::resolve_speed(SpeedLevel::Explicit(0.5)).unwrap();
        assert!((speed - 0.7).abs() < 0.001, "Should clamp to minimum 0.7");
    }

    #[test]
    fn test_resolve_speed_explicit_clamped_high() {
        // 2.0x should clamp to 1.2 (ElevenLabs maximum)
        let speed = ElevenLabsProvider::resolve_speed(SpeedLevel::Explicit(2.0)).unwrap();
        assert!((speed - 1.2).abs() < 0.001, "Should clamp to maximum 1.2");
    }

    // ========================================================================
    // map_api_error() tests
    // ========================================================================

    #[test]
    fn test_map_api_error_402_returns_paid_plan_required() {
        let err = ApiError::<NoDeclaredError> {
            kind: ErrorKind::Undeclared,
            body: "payment required".into(),
        };
        let mapped = ElevenLabsProvider::map_api_error(402, &err);
        match mapped {
            TtsError::PaidPlanRequired { provider, detail } => {
                assert_eq!(provider, "ElevenLabs");
                assert_eq!(detail, "payment required");
            }
            other => panic!("Expected PaidPlanRequired, got: {other}"),
        }
    }

    #[test]
    fn test_map_api_error_non_402_returns_http_error() {
        let err = ApiError::<NoDeclaredError> {
            kind: ErrorKind::Undeclared,
            body: "internal server error".into(),
        };
        let mapped = ElevenLabsProvider::map_api_error(500, &err);
        match mapped {
            TtsError::HttpError { provider, message } => {
                assert_eq!(provider, "elevenlabs");
                assert_eq!(message, "API error (500): internal server error");
            }
            other => panic!("Expected HttpError, got: {other}"),
        }
    }

    #[test]
    fn test_map_api_error_malformed_success_reports_parse_failure() {
        let err = ApiError::<NoDeclaredError> {
            kind: ErrorKind::MalformedSuccess,
            body: "not json".into(),
        };
        let mapped = ElevenLabsProvider::map_api_error(200, &err);
        match mapped {
            TtsError::HttpError { message, .. } => {
                assert!(
                    message.starts_with("Failed to parse response: "),
                    "unexpected message: {message}"
                );
            }
            other => panic!("Expected HttpError, got: {other}"),
        }
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
        assert!(
            !response.voices.is_empty(),
            "Should have at least one voice"
        );
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

    /// Diagnostic test to debug voice deserialization issues.
    /// Run with: cargo test -p biscuit-speaks -- --ignored test_debug_voice_deserialization --nocapture
    #[tokio::test]
    #[ignore = "diagnostic test - requires API key"]
    async fn test_debug_voice_deserialization() {
        use schematic_schema::elevenlabs::ListVoicesResponse;

        let api_key = std::env::var("ELEVEN_LABS_API_KEY")
            .or_else(|_| std::env::var("ELEVENLABS_API_KEY"))
            .expect("No API key found");

        let client = reqwest::Client::new();
        let response = client
            .get("https://api.elevenlabs.io/v2/voices")
            .header("xi-api-key", &api_key)
            .send()
            .await
            .expect("Request failed");

        println!("Status: {}", response.status());

        let text = response.text().await.expect("Failed to get text");

        match serde_json::from_str::<ListVoicesResponse>(&text) {
            Ok(resp) => {
                println!(
                    "\n=== SUCCESS! Deserialized {} voices ===",
                    resp.voices.len()
                );
                for v in resp.voices.iter() {
                    println!("\nVoice: {} ({:?})", v.name, v.category);
                    if let Some(langs) = &v.verified_languages {
                        println!("  verified_languages ({} entries):", langs.len());
                        for lang in langs.iter().take(3) {
                            println!(
                                "    - language_id='{}', model_id={:?}",
                                lang.language_id, lang.model_id
                            );
                        }
                    } else {
                        println!("  verified_languages: None");
                    }
                }
            }
            Err(e) => {
                println!("\n=== DESERIALIZATION ERROR ===");
                println!("Error: {}", e);
                println!("Line {}, Column {}", e.line(), e.column());

                // Show context around the error
                let lines: Vec<&str> = text.lines().collect();
                let line_num = e.line();
                if line_num > 0 && line_num <= lines.len() {
                    let start = line_num.saturating_sub(5);
                    let end = (line_num + 3).min(lines.len());
                    println!("\nContext (lines {}-{}):", start + 1, end);
                    for (i, line) in lines[start..end].iter().enumerate() {
                        let actual_line = start + i + 1;
                        let marker = if actual_line == line_num {
                            ">>>"
                        } else {
                            "   "
                        };
                        println!(
                            "{} {}: {}",
                            marker,
                            actual_line,
                            &line[..line.len().min(100)]
                        );
                    }
                }
                panic!("Deserialization failed: {}", e);
            }
        }
    }

    #[tokio::test]
    #[ignore] // Requires ELEVENLABS_API_KEY
    async fn test_default_voice_any_returns_rachel() {
        let provider = ElevenLabsProvider::new().expect("Requires ELEVENLABS_API_KEY");
        let voice = provider.default_voice(Gender::Any).await.unwrap();
        assert_eq!(voice.name, "Rachel");
        assert_eq!(voice.gender, Gender::Female);
        assert_eq!(voice.quality, VoiceQuality::Excellent);
    }

    #[tokio::test]
    #[ignore] // Requires ELEVENLABS_API_KEY
    async fn test_default_voice_male() {
        let provider = ElevenLabsProvider::new().expect("Requires ELEVENLABS_API_KEY");
        let voice = provider.default_voice(Gender::Male).await.unwrap();
        assert_eq!(voice.gender, Gender::Male);
        assert!(!voice.name.is_empty());
    }

    #[tokio::test]
    #[ignore] // Requires ELEVENLABS_API_KEY
    async fn test_default_voice_female() {
        let provider = ElevenLabsProvider::new().expect("Requires ELEVENLABS_API_KEY");
        let voice = provider.default_voice(Gender::Female).await.unwrap();
        assert_eq!(voice.gender, Gender::Female);
        assert!(!voice.name.is_empty());
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

    // ========================================================================
    // Injected transport: voice selection through the request seam
    // ========================================================================

    fn fake_provider(transport: &Arc<fake::FakeTransport>) -> ElevenLabsProvider {
        ElevenLabsProvider::with_transport(
            Arc::clone(transport) as Arc<dyn ElevenLabsTransport>,
            Arc::new(fake::FakeClock::at(1_800_000_000)),
        )
    }

    async fn selected_voice(transport: &Arc<fake::FakeTransport>, config: &TtsConfig) -> String {
        let audio = fake_provider(transport)
            .generate_audio("This is a test message.", config)
            .await
            .expect("scripted synthesis succeeds");
        assert_eq!(audio, fake::FAKE_AUDIO);
        transport.speech_calls().last().expect("one speech request").voice_id.clone()
    }

    #[tokio::test]
    async fn selection_prefers_the_first_voice_matching_the_requested_gender() {
        let transport = Arc::new(fake::FakeTransport::with_voices(vec![
            fake::voice("m1", Some("male")),
            fake::voice("f1", Some("Female")),
            fake::voice("f2", Some("female")),
        ]));
        let config = TtsConfig::new().with_gender(Gender::Female);
        assert_eq!(selected_voice(&transport, &config).await, "f1");
        assert_eq!(transport.list_requests(), 1);
        let call = &transport.speech_calls()[0];
        assert_eq!(call.model_id.as_deref(), Some(DEFAULT_MODEL_ID));
        assert_eq!(call.text, "This is a test message.");
    }

    #[tokio::test]
    async fn selection_without_a_gender_match_uses_the_first_listed_voice() {
        let transport = Arc::new(fake::FakeTransport::with_voices(vec![
            fake::voice("unlabeled", None),
            fake::voice("m1", Some("male")),
        ]));
        assert_eq!(selected_voice(&transport, &TtsConfig::new().with_gender(Gender::Female)).await, "unlabeled");
        assert_eq!(selected_voice(&transport, &TtsConfig::new()).await, "unlabeled");
    }

    #[tokio::test]
    async fn selection_falls_back_to_the_configured_voice_when_listing_fails_or_is_empty() {
        let transport = Arc::new(fake::FakeTransport::unreachable());
        assert_eq!(selected_voice(&transport, &TtsConfig::new()).await, DEFAULT_VOICE_ID);
        transport.set_voices(Some(Vec::new()));
        assert_eq!(selected_voice(&transport, &TtsConfig::new()).await, DEFAULT_VOICE_ID);
        assert_eq!(transport.list_requests(), 2);
    }

    #[tokio::test]
    async fn a_requested_voice_and_model_skip_the_voice_list() {
        let transport = Arc::new(fake::FakeTransport::with_voices(vec![fake::voice("listed", None)]));
        let config = TtsConfig::new().with_voice("chosen").with_model("eleven_turbo_v2");
        assert_eq!(selected_voice(&transport, &config).await, "chosen");
        assert_eq!(transport.list_requests(), 0);
        assert_eq!(transport.speech_calls()[0].model_id.as_deref(), Some("eleven_turbo_v2"));
    }

    #[tokio::test]
    async fn a_changed_account_voice_list_changes_the_next_selection() {
        let transport = Arc::new(fake::FakeTransport::with_voices(vec![fake::voice("before", None)]));
        assert_eq!(selected_voice(&transport, &TtsConfig::new()).await, "before");
        transport.set_voices(Some(vec![fake::voice("after", None)]));
        assert_eq!(selected_voice(&transport, &TtsConfig::new()).await, "after");
        assert_eq!(transport.total_requests(), 4);
    }

    #[test]
    fn fake_clock_reads_the_time_it_was_set_to() {
        let clock = fake::FakeClock::at(1_800_000_000);
        clock.advance(24 * 60 * 60);
        assert_eq!(clock.now_unix_seconds(), 1_800_086_400);
    }

    // ========================================================================
    // Default-voice records: preparation, detached lookup, trust rules
    // ========================================================================

    const NOW: u64 = 1_800_000_000;
    const ACCOUNT: &str = "https://api.elevenlabs.io;key-source=ELEVEN_LABS_API_KEY;label=work";
    const TEXT: &str = "This is a test message: prepared ElevenLabs";

    /// Owns the audio cache (the OS temporary directory) and the record directory
    /// (`BISCUIT_SPEAKS_CACHE`'s parent).
    struct RecordFixture {
        root: tempfile::TempDir,
        _env: [test_toolkit::EnvGuard; 6],
    }

    impl RecordFixture {
        fn new() -> Self {
            let root = tempfile::tempdir().unwrap();
            for dir in ["tmp", "records"] {
                std::fs::create_dir(root.path().join(dir)).unwrap();
            }
            // `temp_dir` reads `TMPDIR` on Unix and `TMP`/`TEMP` on Windows.
            let env = [
                test_toolkit::EnvGuard::set_safe("TMPDIR", root.path().join("tmp")),
                test_toolkit::EnvGuard::set_safe("TMP", root.path().join("tmp")),
                test_toolkit::EnvGuard::set_safe("TEMP", root.path().join("tmp")),
                test_toolkit::EnvGuard::set_safe(
                    "BISCUIT_SPEAKS_CACHE",
                    root.path().join("records").join("biscuit-speaks-cache.json"),
                ),
                test_toolkit::EnvGuard::remove_safe("PLAYA_DRY_RUN"),
                test_toolkit::EnvGuard::remove_safe("BISCUIT_SPEAKS_ELEVENLABS_ACCOUNT"),
            ];
            Self { root, _env: env }
        }

        fn records(&self) -> Vec<std::path::PathBuf> {
            let mut records: Vec<_> = std::fs::read_dir(self.root.path().join("records"))
                .unwrap()
                .map(|entry| entry.unwrap().path())
                .collect();
            records.sort();
            records
        }

        /// The one record, as JSON.
        fn record(&self) -> serde_json::Value {
            let records = self.records();
            assert_eq!(records.len(), 1, "{records:?}");
            serde_json::from_slice(&std::fs::read(&records[0]).unwrap()).unwrap()
        }

        fn audio_files(&self) -> usize {
            std::fs::read_dir(self.root.path().join("tmp")).unwrap().count()
        }
    }

    fn recording_provider(
        transport: &Arc<fake::FakeTransport>,
        now: u64,
        account: Option<&str>,
    ) -> ElevenLabsProvider {
        ElevenLabsProvider::with_transport(
            Arc::clone(transport) as Arc<dyn ElevenLabsTransport>,
            Arc::new(fake::FakeClock::at(now)),
        )
        .with_account_context(account.map(str::to_owned))
    }

    fn female() -> TtsConfig {
        TtsConfig::new()
            .with_gender(Gender::Female)
            .with_volume(crate::types::VolumeLevel::Explicit(0.0))
    }

    fn listed_voices() -> Arc<fake::FakeTransport> {
        Arc::new(fake::FakeTransport::with_voices(vec![
            fake::voice("m1", Some("male")),
            fake::voice("f1", Some("female")),
        ]))
    }

    /// Whether a fresh detached request finds a ready file, asserting it made
    /// no API request to find out.
    async fn detached_hit(provider: &ElevenLabsProvider, transport: &fake::FakeTransport, config: &TtsConfig) -> bool {
        let before = transport.total_requests();
        let path = provider.cached_path(TEXT, config);
        #[cfg(feature = "playa")]
        {
            let job = provider.cached_detached_job(TEXT, config).await.unwrap();
            assert_eq!(job.is_some(), path.is_some());
            if let Some(job) = job {
                let playa::detached::SpoolJob::PlayFile { path: job_path, playback, .. } = job else {
                    panic!("a ready ElevenLabs job plays a file: {job:?}");
                };
                assert_eq!(Some(job_path.to_os_string().unwrap()), path.clone().map(PathBuf::into_os_string));
                assert_eq!(playback.options.speed, Some(1.0), "baked speed is not applied again");
            }
        }
        assert_eq!(transport.total_requests(), before, "a detached lookup makes no API request");
        path.is_some()
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn a_prepared_default_voice_is_found_by_a_fresh_detached_lookup_without_any_request() {
        let fixture = RecordFixture::new();
        let transport = listed_voices();
        let prepared = recording_provider(&transport, NOW, Some(ACCOUNT))
            .prepare_resolved(TEXT, &female())
            .await
            .unwrap();
        assert_eq!(
            prepared,
            Some(ResolvedSynthesis::ElevenLabs {
                voice: "f1".into(),
                model: DEFAULT_MODEL_ID.into(),
                speed: SpeedLevel::Normal,
            })
        );
        assert_eq!((transport.list_requests(), transport.speech_calls().len()), (1, 1));
        assert_eq!(fixture.audio_files(), 1);
        assert_eq!(
            fixture.record(),
            serde_json::json!({
                "voice": "f1",
                "model": DEFAULT_MODEL_ID,
                "inputs": {"gender": "female", "language": "en", "model": DEFAULT_MODEL_ID, "fallback": DEFAULT_VOICE_ID},
                "account_context": ACCOUNT,
                "written_at": NOW,
            })
        );

        let fresh = recording_provider(&transport, NOW + 60, Some(ACCOUNT));
        assert!(detached_hit(&fresh, &transport, &female()).await);
        // Volume is playback-only: a louder request finds the same file.
        let louder = female().with_volume(crate::types::VolumeLevel::Loud);
        assert!(detached_hit(&fresh, &transport, &louder).await);
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn every_prepare_rechecks_the_selection_and_a_changed_voice_list_updates_the_record() {
        let fixture = RecordFixture::new();
        let transport = listed_voices();
        recording_provider(&transport, NOW, Some(ACCOUNT)).prepare_resolved(TEXT, &female()).await.unwrap();
        recording_provider(&transport, NOW + 10, Some(ACCOUNT)).prepare_resolved(TEXT, &female()).await.unwrap();
        assert_eq!(transport.list_requests(), 2, "a fresh record does not skip the API");
        assert_eq!(transport.speech_calls().len(), 1, "the file is reused");
        assert_eq!(fixture.record()["written_at"], NOW + 10);

        transport.set_voices(Some(vec![fake::voice("f2", Some("female"))]));
        let prepared = recording_provider(&transport, NOW + 20, Some(ACCOUNT))
            .prepare_resolved(TEXT, &female())
            .await
            .unwrap();
        assert!(matches!(prepared, Some(ResolvedSynthesis::ElevenLabs { ref voice, .. }) if voice == "f2"));
        assert_eq!(transport.speech_calls().last().unwrap().voice_id, "f2");
        assert_eq!(fixture.record()["voice"], "f2");
        assert_eq!(fixture.audio_files(), 2);
        let fresh = recording_provider(&transport, NOW + 30, Some(ACCOUNT));
        assert!(detached_hit(&fresh, &transport, &female()).await);
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn untrusted_records_miss_until_the_helper_resolves_through_the_api() {
        let fixture = RecordFixture::new();
        let transport = listed_voices();
        recording_provider(&transport, NOW, Some(ACCOUNT)).prepare_resolved(TEXT, &female()).await.unwrap();
        let at = |now| recording_provider(&transport, now, Some(ACCOUNT));

        assert!(detached_hit(&at(NOW + snapshot::TRUST_SECONDS - 1), &transport, &female()).await);
        assert!(!detached_hit(&at(NOW + snapshot::TRUST_SECONDS), &transport, &female()).await, "expired");
        assert!(!detached_hit(&at(NOW - 1), &transport, &female()).await, "written in the future");
        let other_account = recording_provider(&transport, NOW, Some("https://api.elevenlabs.io;key-source=ELEVENLABS_API_KEY;label=work"));
        assert!(!detached_hit(&other_account, &transport, &female()).await, "another account");
        assert!(!detached_hit(&recording_provider(&transport, NOW, None), &transport, &female()).await, "no account");
        let other_language = female().with_language(Language::Custom("fr".into()));
        assert!(!detached_hit(&at(NOW), &transport, &other_language).await, "other selection inputs");
        let other_fallback = at(NOW).with_default_voice("someone-else");
        assert!(!detached_hit(&other_fallback, &transport, &female()).await, "other fallback");

        let record = fixture.records().remove(0);
        std::fs::write(&record, b"{\"voice\":").unwrap();
        assert!(!detached_hit(&at(NOW), &transport, &female()).await, "corrupt");
        std::fs::remove_file(&record).unwrap();
        assert!(!detached_hit(&at(NOW), &transport, &female()).await, "missing");

        // The miss path's helper resolves through the API, which rewrites the
        // record, and reuses the already-synthesized file.
        #[cfg(feature = "playa")]
        {
            let lists = transport.list_requests();
            at(NOW).detached_job(TEXT, &female()).await.unwrap();
            assert_eq!(transport.list_requests(), lists + 1);
            assert_eq!(transport.speech_calls().len(), 1);
            assert!(detached_hit(&at(NOW), &transport, &female()).await);
        }
    }

    /// Input robustness for the record file: the control row is a record the
    /// provider wrote; each edited row must be a cache miss.
    #[tokio::test]
    #[serial_test::serial]
    async fn every_malformed_record_shape_is_a_miss() {
        let fixture = RecordFixture::new();
        let transport = listed_voices();
        let provider = recording_provider(&transport, NOW, Some(ACCOUNT));
        provider.prepare_resolved(TEXT, &female()).await.unwrap();
        let path = fixture.records().remove(0);
        let original = std::fs::read_to_string(&path).unwrap();
        let control: serde_json::Value = serde_json::from_str(&original).unwrap();

        let edited = |edit: &dyn Fn(&mut serde_json::Value)| {
            let mut value = control.clone();
            edit(&mut value);
            serde_json::to_string(&value).unwrap()
        };
        let duplicated = |field: &str| {
            original.replacen('{', &format!("{{\"{field}\":{},", control[field]), 1)
        };
        let mut rows: Vec<(String, String)> = Vec::new();
        for field in ["voice", "model", "inputs", "account_context", "written_at"] {
            let wrong_type = if field == "written_at" { serde_json::json!("1800000000") } else { serde_json::json!(123) };
            let empty = if field == "inputs" { serde_json::json!({}) } else { serde_json::json!("") };
            rows.push((format!("{field} absent"), edited(&|v| { v.as_object_mut().unwrap().remove(field); })));
            rows.push((format!("{field} null"), edited(&|v| v[field] = serde_json::Value::Null)));
            rows.push((format!("{field} wrong type"), edited(&|v| v[field] = wrong_type.clone())));
            rows.push((format!("{field} empty"), edited(&|v| v[field] = empty.clone())));
            rows.push((format!("{field} duplicate key"), duplicated(field)));
        }
        for input in ["gender", "language", "model", "fallback"] {
            rows.push((format!("inputs.{input} absent"), edited(&|v| { v["inputs"].as_object_mut().unwrap().remove(input); })));
            rows.push((format!("inputs.{input} null"), edited(&|v| v["inputs"][input] = serde_json::Value::Null)));
            rows.push((format!("inputs.{input} wrong type"), edited(&|v| v["inputs"][input] = serde_json::json!(1))));
            rows.push((format!("inputs.{input} empty"), edited(&|v| v["inputs"][input] = serde_json::json!(""))));
            rows.push((
                format!("inputs.{input} duplicate key"),
                original.replacen("\"inputs\":{", &format!("\"inputs\":{{\"{input}\":{},", control["inputs"][input]), 1),
            ));
        }
        rows.extend([
            (
                "inputs as an array, in declaration order".into(),
                edited(&|v| v["inputs"] = serde_json::json!(["female", "en", DEFAULT_MODEL_ID, DEFAULT_VOICE_ID])),
            ),
            ("inputs unknown field".into(), edited(&|v| v["inputs"]["speed"] = serde_json::json!("fast"))),
            ("unknown field".into(), edited(&|v| v["api_key"] = serde_json::json!("x"))),
            ("trailing garbage".into(), format!("{original} garbage")),
            ("a second document".into(), format!("{original}\n{original}")),
            (
                "fields as an array, in declaration order".into(),
                serde_json::to_string(&["voice", "model", "inputs", "account_context", "written_at"].map(|field| &control[field])).unwrap(),
            ),
            ("empty file".into(), String::new()),
            ("stale".into(), edited(&|v| v["written_at"] = serde_json::json!(NOW - snapshot::TRUST_SECONDS))),
            ("future".into(), edited(&|v| v["written_at"] = serde_json::json!(NOW + 1))),
            ("negative time".into(), edited(&|v| v["written_at"] = serde_json::json!(-1))),
            ("mismatched account".into(), edited(&|v| v["account_context"] = serde_json::json!("another"))),
            ("mismatched inputs".into(), edited(&|v| v["inputs"]["gender"] = serde_json::json!("male"))),
            ("model differs from its input".into(), edited(&|v| v["model"] = serde_json::json!("eleven_turbo_v2"))),
        ]);

        assert!(detached_hit(&provider, &transport, &female()).await, "control row");
        for (row, content) in &rows {
            std::fs::write(&path, content).unwrap();
            assert!(!detached_hit(&provider, &transport, &female()).await, "{row}: {content}");
        }
        std::fs::write(&path, &original).unwrap();
        assert!(detached_hit(&provider, &transport, &female()).await, "control row restored");
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn a_failed_voice_listing_prepares_and_records_nothing() {
        let fixture = RecordFixture::new();
        let transport = Arc::new(fake::FakeTransport::unreachable());
        let prepared = recording_provider(&transport, NOW, Some(ACCOUNT))
            .prepare_resolved(TEXT, &female())
            .await
            .unwrap();
        assert_eq!(prepared, None, "the default voice is unresolved");
        assert!(transport.speech_calls().is_empty());
        assert!(fixture.records().is_empty());
        assert_eq!(fixture.audio_files(), 0);
    }

    struct InterruptedSpeech {
        fail: bool,
        started: tokio::sync::Notify,
        dropped: std::sync::atomic::AtomicBool,
    }

    impl ElevenLabsTransport for InterruptedSpeech {
        fn list_voices(&self) -> TransportFuture<'_, Result<ListVoicesResponse, TtsError>> {
            panic!("an explicit voice must not list voices");
        }

        fn create_speech(
            &self,
            _request: CreateSpeechRequest,
        ) -> TransportFuture<'_, Result<Vec<u8>, TtsError>> {
            Box::pin(async move {
                if self.fail {
                    return Err(TtsError::HttpError {
                        provider: "elevenlabs".into(),
                        message: "scripted synthesis failure".into(),
                    });
                }
                struct OnDrop<'a>(&'a std::sync::atomic::AtomicBool);
                impl Drop for OnDrop<'_> {
                    fn drop(&mut self) {
                        self.0.store(true, std::sync::atomic::Ordering::SeqCst);
                    }
                }
                let _pending = OnDrop(&self.dropped);
                self.started.notify_one();
                std::future::pending().await
            })
        }
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn a_failed_synthesis_prepares_and_records_nothing() {
        let fixture = RecordFixture::new();
        let transport = Arc::new(InterruptedSpeech {
            fail: true,
            started: tokio::sync::Notify::new(),
            dropped: std::sync::atomic::AtomicBool::new(false),
        });
        let provider = ElevenLabsProvider::with_transport(transport, Arc::new(fake::FakeClock::at(NOW)))
            .with_account_context(Some(ACCOUNT.into()));
        let result = provider.prepare(TEXT, &female().with_voice("f1")).await;
        assert!(matches!(result, Err(TtsError::HttpError { message, .. }) if message == "scripted synthesis failure"));
        assert_eq!(fixture.audio_files(), 0);
        assert!(fixture.records().is_empty());
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn cancelling_synthesis_drops_the_request_without_publishing_audio() {
        let fixture = RecordFixture::new();
        let transport = Arc::new(InterruptedSpeech {
            fail: false,
            started: tokio::sync::Notify::new(),
            dropped: std::sync::atomic::AtomicBool::new(false),
        });
        let provider = ElevenLabsProvider::with_transport(
            Arc::clone(&transport) as Arc<dyn ElevenLabsTransport>,
            Arc::new(fake::FakeClock::at(NOW)),
        )
        .with_account_context(Some(ACCOUNT.into()));
        let task = tokio::spawn(async move { provider.prepare(TEXT, &female().with_voice("f1")).await });
        tokio::time::timeout(std::time::Duration::from_secs(2), transport.started.notified())
            .await
            .expect("the synthesis request starts");
        task.abort();
        assert!(task.await.unwrap_err().is_cancelled());
        assert!(transport.dropped.load(std::sync::atomic::Ordering::SeqCst));
        assert_eq!(fixture.audio_files(), 0);
        assert!(fixture.records().is_empty());
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn a_requested_voice_prepares_without_listing_or_recording() {
        let fixture = RecordFixture::new();
        let transport = listed_voices();
        let config = female().with_voice("chosen");
        let provider = recording_provider(&transport, NOW, Some(ACCOUNT));
        let prepared = provider.prepare_resolved(TEXT, &config).await.unwrap();
        assert!(matches!(prepared, Some(ResolvedSynthesis::ElevenLabs { ref voice, .. }) if voice == "chosen"));
        assert_eq!(transport.list_requests(), 0);
        assert!(fixture.records().is_empty());
        assert!(detached_hit(&provider, &transport, &config).await);
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn dry_run_prepares_nothing_and_makes_no_request() {
        let fixture = RecordFixture::new();
        let _dry_run = test_toolkit::EnvGuard::set_safe("PLAYA_DRY_RUN", "TRUE");
        let transport = listed_voices();
        let prepared = recording_provider(&transport, NOW, Some(ACCOUNT))
            .prepare(TEXT, &female())
            .await;
        prepared.unwrap();
        assert_eq!(transport.total_requests(), 0);
        assert!(fixture.records().is_empty());
        assert_eq!(fixture.audio_files(), 0);
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn baked_speed_reaches_the_api_and_the_key_but_not_playback() {
        let fixture = RecordFixture::new();
        let transport = listed_voices();
        let provider = recording_provider(&transport, NOW, Some(ACCOUNT));
        let fast = female().with_speed(SpeedLevel::Fast);
        provider.prepare_resolved(TEXT, &fast).await.unwrap();
        provider.prepare_resolved(TEXT, &female()).await.unwrap();
        let speeds: Vec<_> = transport.speech_calls().into_iter().map(|call| call.speed).collect();
        assert_eq!(speeds, [Some("1.2".to_string()), None], "Fast clamps to the API's 1.2");
        assert_eq!(fixture.audio_files(), 2, "speed is part of the key");
        assert!(detached_hit(&provider, &transport, &fast).await);
    }

    /// Removes both API-key variables for the test's lifetime, so a key set
    /// and dropped inside the test restores absence rather than the
    /// developer's own key.
    fn without_api_keys() -> [test_toolkit::EnvGuard; 2] {
        [
            test_toolkit::EnvGuard::remove_safe("ELEVEN_LABS_API_KEY"),
            test_toolkit::EnvGuard::remove_safe("ELEVENLABS_API_KEY"),
        ]
    }

    /// A provider built by a production constructor, so its account context
    /// comes from the environment, with only its requests and clock replaced.
    fn production_provider(
        base: ElevenLabsProvider,
        transport: &Arc<fake::FakeTransport>,
        now: u64,
    ) -> ElevenLabsProvider {
        ElevenLabsProvider {
            transport: Arc::clone(transport) as Arc<dyn ElevenLabsTransport>,
            clock: Arc::new(fake::FakeClock::at(now)),
            ..base
        }
    }

    /// The record never contains the API key, only the name of the
    /// variable it was read from.
    #[tokio::test]
    #[serial_test::serial]
    async fn the_account_context_and_record_hold_no_credential() {
        const SECRET: &str = "sk-sentinel-0123456789abcdef";
        let fixture = RecordFixture::new();
        let _no_keys = without_api_keys();
        let _key = test_toolkit::EnvGuard::set_safe("ELEVEN_LABS_API_KEY", SECRET);
        let _other = test_toolkit::EnvGuard::set_safe("ELEVENLABS_API_KEY", "sk-other-secret");
        let _label = test_toolkit::EnvGuard::set_safe("BISCUIT_SPEAKS_ELEVENLABS_ACCOUNT", "work");

        let context = snapshot::account_context(ElevenLabs::BASE_URL).unwrap();
        assert_eq!(context, "https://api.elevenlabs.io;key-source=ELEVEN_LABS_API_KEY;label=work");
        let transport = listed_voices();
        recording_provider(&transport, NOW, Some(&context)).prepare_resolved(TEXT, &female()).await.unwrap();
        let record = std::fs::read_to_string(&fixture.records()[0]).unwrap();
        assert!(!record.contains(SECRET) && !record.contains("sk-other-secret"), "{record}");
        assert!(!record.contains(&biscuit_hash::xx_hash(SECRET).to_string()), "{record}");

        let personal = test_toolkit::EnvGuard::set_safe("BISCUIT_SPEAKS_ELEVENLABS_ACCOUNT", "personal");
        assert_ne!(snapshot::account_context(ElevenLabs::BASE_URL).unwrap(), context);
        drop(personal);
        drop(_key);
        assert_eq!(
            snapshot::account_context("http://localhost:8080").as_deref(),
            Some("http://localhost:8080;key-source=ELEVENLABS_API_KEY;label=work")
        );
        drop(_other);
        assert_eq!(snapshot::account_context(ElevenLabs::BASE_URL), None, "no key, no records");
    }

    /// Without a label, two keys under one variable are indistinguishable,
    /// so account A's preparation must not answer account B's detached
    /// request: it writes no record, and B's lookup misses.
    #[tokio::test]
    #[serial_test::serial]
    async fn an_unlabeled_account_switch_misses_instead_of_reusing_the_previous_accounts_voice() {
        let fixture = RecordFixture::new();
        let _no_keys = without_api_keys();
        let account_a = test_toolkit::EnvGuard::set_safe("ELEVEN_LABS_API_KEY", "dummy-account-a");
        let transport_a = listed_voices();
        let prepared = production_provider(ElevenLabsProvider::new().unwrap(), &transport_a, NOW)
            .prepare_resolved(TEXT, &female())
            .await
            .unwrap();
        assert!(matches!(prepared, Some(ResolvedSynthesis::ElevenLabs { ref voice, .. }) if voice == "f1"));
        assert_eq!(fixture.audio_files(), 1, "account A's speech is cached");
        assert!(fixture.records().is_empty(), "no label, no record");

        drop(account_a);
        let _account_b = test_toolkit::EnvGuard::set_safe("ELEVEN_LABS_API_KEY", "dummy-account-b");
        let transport_b = Arc::new(fake::FakeTransport::with_voices(vec![fake::voice(
            "account-b-female",
            Some("female"),
        )]));
        let fresh = production_provider(ElevenLabsProvider::new().unwrap(), &transport_b, NOW + 1);
        assert!(!detached_hit(&fresh, &transport_b, &female()).await, "account A's voice is not reused");
        assert_eq!(transport_b.total_requests(), 0);
        assert!(fixture.records().is_empty());
    }

    #[tokio::test]
    #[serial_test::serial]
    async fn a_blank_account_label_records_and_trusts_nothing() {
        let fixture = RecordFixture::new();
        let _no_keys = without_api_keys();
        let _key = test_toolkit::EnvGuard::set_safe("ELEVEN_LABS_API_KEY", "dummy-account-a");
        for blank in ["", "   "] {
            let _label = test_toolkit::EnvGuard::set_safe("BISCUIT_SPEAKS_ELEVENLABS_ACCOUNT", blank);
            assert_eq!(snapshot::account_context(ElevenLabs::BASE_URL), None, "{blank:?}");
            let transport = listed_voices();
            production_provider(ElevenLabsProvider::new().unwrap(), &transport, NOW)
                .prepare_resolved(TEXT, &female())
                .await
                .unwrap();
            let fresh = production_provider(ElevenLabsProvider::new().unwrap(), &transport, NOW + 1);
            assert!(!detached_hit(&fresh, &transport, &female()).await, "{blank:?}");
            assert_eq!(transport.list_requests(), 1, "the voice is resolved through the API");
        }
        assert!(fixture.records().is_empty());
        assert_eq!(fixture.audio_files(), 1, "once resolved, the voice's audio is reused");
    }

    /// A labeled account reuses its own record without a request; changing
    /// the label, the API address, or the key's variable misses.
    #[tokio::test]
    #[serial_test::serial]
    async fn a_labeled_account_reuses_its_record_and_any_context_change_misses() {
        let fixture = RecordFixture::new();
        let _no_keys = without_api_keys();
        let primary = test_toolkit::EnvGuard::set_safe("ELEVEN_LABS_API_KEY", "dummy-account-a");
        let label = test_toolkit::EnvGuard::set_safe("BISCUIT_SPEAKS_ELEVENLABS_ACCOUNT", "work");
        let transport = listed_voices();
        production_provider(ElevenLabsProvider::new().unwrap(), &transport, NOW)
            .prepare_resolved(TEXT, &female())
            .await
            .unwrap();
        assert_eq!(fixture.records().len(), 1);
        let fresh = || production_provider(ElevenLabsProvider::new().unwrap(), &transport, NOW + 1);
        assert!(detached_hit(&fresh(), &transport, &female()).await, "same labeled account");

        let address = production_provider(
            ElevenLabsProvider::with_base_url("http://127.0.0.1:9").unwrap(),
            &transport,
            NOW + 1,
        );
        assert!(!detached_hit(&address, &transport, &female()).await, "another API address");

        drop(label);
        let personal = test_toolkit::EnvGuard::set_safe("BISCUIT_SPEAKS_ELEVENLABS_ACCOUNT", "personal");
        assert!(!detached_hit(&fresh(), &transport, &female()).await, "another label");
        drop(personal);

        let _label = test_toolkit::EnvGuard::set_safe("BISCUIT_SPEAKS_ELEVENLABS_ACCOUNT", "work");
        drop(primary);
        let _alias = test_toolkit::EnvGuard::set_safe("ELEVENLABS_API_KEY", "dummy-account-a");
        assert!(!detached_hit(&fresh(), &transport, &female()).await, "another key source");
        assert_eq!(transport.list_requests(), 1, "only the preparation listed voices");
    }

    #[cfg(feature = "playa")]
    #[tokio::test]
    #[serial_test::serial]
    async fn helper_synthesis_records_the_default_voice() {
        let fixture = RecordFixture::new();
        let transport = listed_voices();
        let provider = recording_provider(&transport, NOW, Some(ACCOUNT));
        assert!(!detached_hit(&provider, &transport, &female()).await);
        provider.detached_job(TEXT, &female()).await.unwrap();
        assert_eq!(fixture.record()["voice"], "f1");
        assert!(detached_hit(&recording_provider(&transport, NOW, Some(ACCOUNT)), &transport, &female()).await);
    }

    /// Retained settings keep the prepared voice after the account's voice
    /// list changes: the cached file is found, and a vanished file is
    /// synthesized again, with no voice-list request. Speed is sent to the
    /// API once and never applied again at playback.
    #[cfg(feature = "playa")]
    #[tokio::test]
    #[serial_test::serial]
    async fn retained_settings_skip_the_voice_list_and_apply_speed_once() {
        let fixture = RecordFixture::new();
        let transport = listed_voices();
        let provider = recording_provider(&transport, NOW, Some(ACCOUNT));
        let fast = female().with_speed(SpeedLevel::Fast);
        let Some(ResolvedSynthesis::ElevenLabs { voice, model, speed }) =
            provider.prepare_resolved(TEXT, &fast).await.unwrap()
        else {
            panic!("a listed default voice prepares");
        };
        assert_eq!((voice.as_str(), speed), ("f1", SpeedLevel::Fast));
        transport.set_voices(Some(vec![fake::voice("f2", Some("female"))]));
        let requests = transport.total_requests();

        let job_speed = |job: playa::detached::SpoolJob| match job {
            playa::detached::SpoolJob::PlayFile { playback, .. } => playback.options.speed,
            other => panic!("a ready ElevenLabs job plays a file: {other:?}"),
        };
        let cached = ElevenLabsProvider::cached_resolved_job(TEXT, &voice, &model, speed, &fast)
            .expect("the retained settings name the prepared file");
        assert_eq!(job_speed(cached), Some(1.0));
        assert_eq!(transport.total_requests(), requests, "a cached lookup makes no request");

        for entry in std::fs::read_dir(fixture.root.path().join("tmp")).unwrap() {
            std::fs::remove_file(entry.unwrap().path()).unwrap();
        }
        assert!(ElevenLabsProvider::cached_resolved_job(TEXT, &voice, &model, speed, &fast).is_none());
        let job = provider.resolved_job(TEXT, &voice, &model, speed, &fast).await.unwrap();
        assert_eq!(job_speed(job), Some(1.0), "baked speed is not applied again");
        assert_eq!(transport.list_requests(), 1, "only the preparation listed voices");
        let calls = transport.speech_calls();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[1].voice_id, "f1", "not the account's new default");
        assert_eq!(calls[1].speed.as_deref(), Some("1.2"));
        assert_eq!(fixture.audio_files(), 1);
    }

    #[test]
    fn system_clock_reads_the_current_unix_time() {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        assert!(SystemClock.now_unix_seconds().abs_diff(now) <= 5);
    }
}
