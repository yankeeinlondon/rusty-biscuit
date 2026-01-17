//! ElevenLabs API types.
//!
//! This module contains all data types used in the ElevenLabs API,
//! including enums, request/response models, and shared types.

use serde::{Deserialize, Serialize};

// =============================================================================
// Core Enums
// =============================================================================

/// Audio output format for TTS and sound generation.
///
/// Supports MP3, PCM, Opus, and telephony formats at various quality levels.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputFormat {
    /// MP3 at 22.05 kHz, 32 kbps (low quality)
    #[serde(rename = "mp3_22050_32")]
    Mp3_22050_32,
    /// MP3 at 44.1 kHz, 32 kbps
    #[serde(rename = "mp3_44100_32")]
    Mp3_44100_32,
    /// MP3 at 44.1 kHz, 64 kbps
    #[serde(rename = "mp3_44100_64")]
    Mp3_44100_64,
    /// MP3 at 44.1 kHz, 96 kbps
    #[serde(rename = "mp3_44100_96")]
    Mp3_44100_96,
    /// MP3 at 44.1 kHz, 128 kbps (default)
    #[serde(rename = "mp3_44100_128")]
    #[default]
    Mp3_44100_128,
    /// MP3 at 44.1 kHz, 192 kbps (high quality)
    #[serde(rename = "mp3_44100_192")]
    Mp3_44100_192,
    /// PCM at 8 kHz
    #[serde(rename = "pcm_8000")]
    Pcm8000,
    /// PCM at 16 kHz
    #[serde(rename = "pcm_16000")]
    Pcm16000,
    /// PCM at 22.05 kHz
    #[serde(rename = "pcm_22050")]
    Pcm22050,
    /// PCM at 24 kHz
    #[serde(rename = "pcm_24000")]
    Pcm24000,
    /// PCM at 44.1 kHz
    #[serde(rename = "pcm_44100")]
    Pcm44100,
    /// u-law at 8 kHz (telephony)
    #[serde(rename = "ulaw_8000")]
    Ulaw8000,
    /// A-law at 8 kHz (telephony)
    #[serde(rename = "alaw_8000")]
    Alaw8000,
    /// Opus at 48 kHz, 32 kbps
    #[serde(rename = "opus_48000_32")]
    Opus48000_32,
    /// Opus at 48 kHz, 64 kbps
    #[serde(rename = "opus_48000_64")]
    Opus48000_64,
    /// Opus at 48 kHz, 96 kbps
    #[serde(rename = "opus_48000_96")]
    Opus48000_96,
    /// Opus at 48 kHz, 128 kbps
    #[serde(rename = "opus_48000_128")]
    Opus48000_128,
    /// Opus at 48 kHz, 192 kbps
    #[serde(rename = "opus_48000_192")]
    Opus48000_192,
}


/// Voice category classification.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VoiceCategory {
    /// ElevenLabs default/premade voices
    Premade,
    /// Instant voice clones
    Cloned,
    /// AI-generated voices
    Generated,
    /// Professional voice clones (PVC)
    Professional,
    /// Celebrity voices
    Famous,
    /// High-quality community voices
    HighQuality,
}

/// Voice type for filtering.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VoiceType {
    /// Personal voices
    Personal,
    /// Community-shared voices
    Community,
    /// Default ElevenLabs voices
    Default,
    /// Workspace voices
    Workspace,
    /// Non-default voices
    NonDefault,
    /// Saved/favorited voices
    Saved,
}

/// Text normalization mode.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextNormalization {
    /// Automatic text normalization
    #[default]
    Auto,
    /// Force text normalization on
    On,
    /// Disable text normalization
    Off,
}

/// Subscription status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SubscriptionStatus {
    /// Active trial period
    Trialing,
    /// Paid subscription
    Active,
    /// Payment pending
    Incomplete,
    /// Payment overdue
    PastDue,
    /// Free tier
    Free,
    /// Disabled free account
    FreeDisabled,
}

/// Billing period for subscriptions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum BillingPeriod {
    /// Monthly billing
    #[serde(rename = "monthly")]
    Monthly,
    /// 3-month billing
    #[serde(rename = "3-month")]
    ThreeMonth,
    /// 6-month billing
    #[serde(rename = "6-month")]
    SixMonth,
    /// Annual billing
    #[serde(rename = "annual")]
    Annual,
}

/// Supported currencies.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Currency {
    USD,
    EUR,
    INR,
}

/// Resource types for workspace management.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceType {
    #[default]
    Voice,
    VoiceCollection,
    PronunciationDictionary,
    Dubbing,
    Project,
    ConvaiConversation,
    ConvaiAgent,
    ConvaiSecret,
    ConvaiKnowledgeBase,
    ConvaiKnowledgeBaseDocument,
    ConvaiTool,
    ConvaiPhoneNumber,
    ConvaiWidget,
}

/// Token type for single-use tokens.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TokenType {
    /// For real-time transcription
    RealtimeScribe,
    /// For TTS WebSocket connections
    TtsWebsocket,
}

/// History item source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HistorySource {
    /// Text-to-speech
    TTS,
    /// Speech-to-speech
    STS,
    /// Projects
    Projects,
    /// Dubbing
    Dubbing,
}

/// Webhook authentication type.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WebhookAuthType {
    /// HMAC authentication
    #[default]
    Hmac,
    /// OAuth2 authentication
    Oauth2,
    /// Mutual TLS authentication
    Mtls,
}

/// Sort direction for list queries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SortDirection {
    /// Ascending order
    Asc,
    /// Descending order
    Desc,
}

/// Access level for resource sharing.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccessLevel {
    #[default]
    Admin,
    Editor,
    Commenter,
    Viewer,
}

/// Fine-tuning state for PVC voices.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FineTuningState {
    Draft,
    Queued,
    FineTuning,
    Completed,
    Failed,
}

/// Speaker separation status for PVC samples.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SpeakerSeparationStatus {
    NotStarted,
    Pending,
    Completed,
    Failed,
}

/// API permissions for service account keys.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApiPermission {
    TextToSpeech,
    SpeechToSpeech,
    SpeechToText,
    ModelsRead,
    ModelsWrite,
    VoicesRead,
    VoicesWrite,
    SpeechHistoryRead,
    SpeechHistoryWrite,
    SoundGeneration,
    AudioIsolation,
    VoiceGeneration,
    DubbingRead,
    DubbingWrite,
    PronunciationDictionariesRead,
    PronunciationDictionariesWrite,
    UserRead,
    UserWrite,
    ProjectsRead,
    ProjectsWrite,
    AudioNativeRead,
    AudioNativeWrite,
    WorkspaceRead,
    WorkspaceWrite,
    ForcedAlignment,
    ConvaiRead,
    ConvaiWrite,
    MusicGeneration,
}

// =============================================================================
// Common Structs
// =============================================================================

/// Voice settings for TTS generation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VoiceSettings {
    /// Voice stability/randomness (0-1).
    #[serde(default = "default_stability")]
    pub stability: f64,

    /// Adherence to original voice (0-1).
    #[serde(default = "default_similarity_boost")]
    pub similarity_boost: f64,

    /// Style exaggeration (V2+ models).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<f64>,

    /// Speech rate (0.7-1.2).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed: Option<f64>,

    /// Enhanced similarity (V2+ models).
    #[serde(default = "default_true")]
    pub use_speaker_boost: bool,
}

fn default_stability() -> f64 {
    0.5
}

fn default_similarity_boost() -> f64 {
    0.75
}

fn default_true() -> bool {
    true
}

impl Default for VoiceSettings {
    fn default() -> Self {
        Self {
            stability: default_stability(),
            similarity_boost: default_similarity_boost(),
            style: Some(0.0),
            speed: Some(1.0),
            use_speaker_boost: true,
        }
    }
}

/// Reference to a pronunciation dictionary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PronunciationDictionaryLocator {
    /// Dictionary identifier.
    pub pronunciation_dictionary_id: String,
    /// Dictionary version identifier.
    pub version_id: String,
}

/// HTTP alignment object for `/with-timestamps` endpoints.
///
/// Times are in **seconds** (f64).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpAlignment {
    /// Array of individual characters.
    pub characters: Vec<String>,
    /// Start time for each character in seconds.
    pub character_start_times_seconds: Vec<f64>,
    /// End time for each character in seconds.
    pub character_end_times_seconds: Vec<f64>,
}

/// WebSocket alignment object for streaming TTS.
///
/// Times are in **milliseconds** (i64).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebSocketAlignment {
    /// Array of individual characters.
    pub chars: Vec<String>,
    /// Start time for each character in milliseconds.
    pub char_start_times_ms: Vec<i64>,
    /// Duration of each character in milliseconds.
    pub char_durations_ms: Vec<i64>,
}

/// Generic status response for many API operations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatusResponse {
    /// Status message (typically "ok").
    pub status: String,
}

/// Generation config for WebSocket TTS.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenerationConfig {
    /// Chunk length schedule for buffering.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunk_length_schedule: Option<Vec<i32>>,
}

impl Default for GenerationConfig {
    fn default() -> Self {
        Self {
            chunk_length_schedule: Some(vec![120, 160, 250, 290]),
        }
    }
}

// =============================================================================
// Text-to-Speech Types (Phase 2)
// =============================================================================

/// Request body for text-to-speech endpoints.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CreateSpeechBody {
    /// Text to convert to speech.
    pub text: String,

    /// TTS model to use.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,

    /// ISO 639-1 language code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language_code: Option<String>,

    /// Voice configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice_settings: Option<VoiceSettings>,

    /// Pronunciation dictionaries (up to 3).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pronunciation_dictionary_locators: Option<Vec<PronunciationDictionaryLocator>>,

    /// Seed for deterministic generation (0-4294967295).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<u32>,

    /// Previous text for context continuity.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_text: Option<String>,

    /// Next text for context continuity.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_text: Option<String>,

    /// Previous request IDs for chaining.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_request_ids: Option<Vec<String>>,

    /// Next request IDs for chaining.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_request_ids: Option<Vec<String>>,

    /// Text normalization mode.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub apply_text_normalization: Option<TextNormalization>,

    /// Language-specific text normalization.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub apply_language_text_normalization: Option<bool>,
}

/// Response from TTS with timestamps endpoints.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpeechWithTimestampsResponse {
    /// Base64-encoded audio data.
    pub audio_base64: String,

    /// Timing data for original text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alignment: Option<HttpAlignment>,

    /// Timing data for normalized text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub normalized_alignment: Option<HttpAlignment>,
}

// =============================================================================
// Voice Types (Phase 3)
// =============================================================================

/// Voice sample model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SampleModel {
    /// Sample identifier.
    pub sample_id: String,

    /// Original file name.
    pub file_name: String,

    /// MIME type of the audio.
    pub mime_type: String,

    /// File size in bytes.
    pub size_bytes: i64,

    /// Content hash.
    pub hash: String,
}

/// Language model for voices.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LanguageModel {
    /// Language identifier.
    pub language_id: String,

    /// Display name.
    pub name: String,
}

/// Fine-tuning model for PVC voices.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FineTuningModel {
    /// Current fine-tuning state.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<FineTuningState>,

    /// Model used for fine-tuning.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,

    /// Whether verification is required.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verification_required: Option<bool>,
}

/// Sharing model for voices.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SharingModel {
    /// Public owner ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_owner_id: Option<String>,

    /// Whether voice is shared.
    #[serde(default)]
    pub is_public: bool,

    /// Number of clones.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cloned_by_count: Option<i64>,
}

/// Voice verification model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VoiceVerificationModel {
    /// Whether verification is required.
    pub requires_verification: bool,

    /// Whether voice is verified.
    pub is_verified: bool,
}

/// Full voice response model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VoiceResponseModel {
    /// Voice identifier.
    pub voice_id: String,

    /// Voice display name.
    pub name: String,

    /// Voice category.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<VoiceCategory>,

    /// Voice samples.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub samples: Option<Vec<SampleModel>>,

    /// Voice settings.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<VoiceSettings>,

    /// Fine-tuning information.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fine_tuning: Option<FineTuningModel>,

    /// Sharing information.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sharing: Option<SharingModel>,

    /// Verified languages.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verified_languages: Option<Vec<LanguageModel>>,

    /// Voice verification status.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice_verification: Option<VoiceVerificationModel>,

    /// Voice description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Voice labels (language, accent, gender, age).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<std::collections::HashMap<String, String>>,
}

/// Response from list voices endpoint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListVoicesResponse {
    /// List of voices.
    pub voices: Vec<VoiceResponseModel>,

    /// Whether more results exist.
    pub has_more: bool,

    /// Total count of matching voices.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_count: Option<i64>,

    /// Token for next page.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_page_token: Option<String>,
}

/// Library voice response model (for shared voices).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LibraryVoiceResponseModel {
    /// Voice identifier.
    pub voice_id: String,

    /// Voice display name.
    pub name: String,

    /// Voice category.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<VoiceCategory>,

    /// Public owner ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_owner_id: Option<String>,

    /// Voice description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Response from list shared voices endpoint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListSharedVoicesResponse {
    /// List of shared voices.
    pub voices: Vec<LibraryVoiceResponseModel>,

    /// Whether more results exist.
    pub has_more: bool,

    /// Cursor for pagination.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_sort_id: Option<String>,
}

/// Request to add a shared voice.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddSharedVoiceBody {
    /// New name for the voice.
    pub new_name: String,
}

/// Response from add shared voice endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddSharedVoiceResponse {
    /// The new voice ID in your library.
    pub voice_id: String,
}

/// Response from adding a voice sample.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AddSampleResponse {
    /// The sample ID.
    pub sample_id: String,
}

/// Request to create a PVC voice.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreatePvcVoiceBody {
    /// Voice name.
    pub name: String,

    /// Language used in samples.
    pub language: String,

    /// Voice description.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Labels (language, accent, gender, age).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<std::collections::HashMap<String, String>>,
}

/// Request to train a PVC voice.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TrainPvcVoiceBody {
    /// Model to use for training.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
}

// =============================================================================
// Core Resources & Sound Effects Types (Phase 4)
// =============================================================================

/// Model rate information.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelRates {
    /// Cost multiplier per character.
    pub character_cost_multiplier: f64,
}

/// Model information.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModelInfo {
    /// Model identifier.
    pub model_id: String,

    /// Display name.
    pub name: String,

    /// Description.
    pub description: String,

    /// Token cost factor.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_cost_factor: Option<f64>,

    /// TTS capability.
    #[serde(default)]
    pub can_do_text_to_speech: bool,

    /// Voice conversion capability.
    #[serde(default)]
    pub can_do_voice_conversion: bool,

    /// Fine-tuning capability.
    #[serde(default)]
    pub can_be_finetuned: bool,

    /// Style support.
    #[serde(default)]
    pub can_use_style: bool,

    /// Speaker boost support.
    #[serde(default)]
    pub can_use_speaker_boost: bool,

    /// Pro voice support.
    #[serde(default)]
    pub serves_pro_voices: bool,

    /// Supported languages.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub languages: Option<Vec<LanguageModel>>,

    /// Model rates.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_rates: Option<ModelRates>,

    /// Max text length per request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maximum_text_length_per_request: Option<i64>,

    /// Max characters for free users.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_characters_request_free_user: Option<i64>,

    /// Max characters for subscribed users.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_characters_request_subscribed_user: Option<i64>,

    /// Requires alpha access.
    #[serde(default)]
    pub requires_alpha_access: bool,

    /// Concurrency group.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub concurrency_group: Option<String>,
}

/// Response containing a single-use token.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SingleUseTokenResponse {
    /// The generated token.
    pub token: String,
}

/// Feedback model for history items.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FeedbackModel {
    /// Thumbs up/down.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbs_up: Option<bool>,

    /// Feedback text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub feedback: Option<String>,
}

/// Alignment model for history items.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AlignmentModel {
    /// Character-level alignment.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub characters: Option<Vec<String>>,

    /// Start times in seconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub character_start_times_seconds: Option<Vec<f64>>,

    /// End times in seconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub character_end_times_seconds: Option<Vec<f64>>,
}

/// Speech history item.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SpeechHistoryItemResponseModel {
    /// History item ID.
    pub history_item_id: String,

    /// Voice ID used.
    pub voice_id: String,

    /// Voice name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice_name: Option<String>,

    /// Voice category.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice_category: Option<VoiceCategory>,

    /// Model ID used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,

    /// The text that was converted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,

    /// Unix timestamp.
    pub date_unix: i64,

    /// Content type of audio.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,

    /// Generation state.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,

    /// User feedback.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub feedback: Option<FeedbackModel>,

    /// Source of generation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<HistorySource>,

    /// Character alignments.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alignments: Option<AlignmentModel>,
}

/// Response from get history endpoint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GetHistoryResponse {
    /// List of history items.
    pub history: Vec<SpeechHistoryItemResponseModel>,

    /// Last item ID for pagination.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_history_item_id: Option<String>,

    /// Whether more items exist.
    pub has_more: bool,

    /// Scan timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scanned_until: Option<i64>,
}

/// Request to download history items.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DownloadHistoryBody {
    /// History item IDs to download.
    pub history_item_ids: Vec<String>,

    /// Output format (wav or null for original).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_format: Option<String>,
}

/// Request to create a sound effect.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct CreateSoundEffectBody {
    /// Text prompt for the sound effect.
    pub text: String,

    /// Duration in seconds (0.5-30).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_seconds: Option<f64>,

    /// Create looping effect (v2 model only).
    /// Note: `loop` is a reserved keyword, so we rename it.
    #[serde(rename = "loop", skip_serializing_if = "Option::is_none")]
    pub loop_sound: Option<bool>,

    /// Prompt adherence (0-1).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_influence: Option<f64>,

    /// Sound generation model.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
}

// =============================================================================
// Workspace & Admin Types (Phase 5)
// =============================================================================

/// Usage statistics response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UsageStatsResponse {
    /// Time points.
    pub time: Vec<i64>,

    /// Usage data by category.
    pub usage: std::collections::HashMap<String, Vec<f64>>,
}

/// Invoice model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InvoiceModel {
    /// Invoice amount.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount_due: Option<f64>,

    /// Invoice currency.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency: Option<Currency>,

    /// Due date.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_date_unix: Option<i64>,
}

/// Subscription model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubscriptionModel {
    /// Subscription tier.
    pub tier: String,

    /// Characters used.
    pub character_count: i64,

    /// Character limit.
    pub character_limit: i64,

    /// Max extension limit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_character_limit_extension: Option<i64>,

    /// Voice slots used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice_slots_used: Option<i64>,

    /// Voice slot limit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice_limit: Option<i64>,

    /// Pro voice slots used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub professional_voice_slots_used: Option<i64>,

    /// Pro voice limit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub professional_voice_limit: Option<i64>,

    /// Subscription status.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<SubscriptionStatus>,

    /// Billing period.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub billing_period: Option<BillingPeriod>,

    /// Currency.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency: Option<Currency>,

    /// Next invoice.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_invoice: Option<InvoiceModel>,
}

/// User response model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserResponse {
    /// User identifier.
    pub user_id: String,

    /// Subscription info.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subscription: Option<SubscriptionModel>,

    /// Is new user.
    #[serde(default)]
    pub is_new_user: bool,

    /// API key (if visible).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub xi_api_key: Option<String>,

    /// Onboarding completion.
    #[serde(default)]
    pub is_onboarding_completed: bool,

    /// First name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_name: Option<String>,

    /// Account creation timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<i64>,
}

/// Share option model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShareOptionModel {
    /// User email (if sharing with user).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_email: Option<String>,

    /// Group ID (if sharing with group).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_id: Option<String>,

    /// Access level.
    pub role: AccessLevel,
}

/// Resource response model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResourceResponse {
    /// Resource identifier.
    pub resource_id: String,

    /// Resource type.
    pub resource_type: ResourceType,

    /// Creator user ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub creator_user_id: Option<String>,

    /// Anonymous access level override.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anonymous_access_level_override: Option<AccessLevel>,

    /// Role to group ID mapping.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role_to_group_ids: Option<std::collections::HashMap<String, Vec<String>>>,

    /// Share options.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub share_options: Option<Vec<ShareOptionModel>>,
}

/// Request to share a resource.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShareResourceBody {
    /// Access level to grant.
    pub role: AccessLevel,

    /// Resource type.
    pub resource_type: ResourceType,

    /// Target user email.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_email: Option<String>,

    /// Target group ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_id: Option<String>,

    /// Target API key ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_api_key_id: Option<String>,
}

/// Request to unshare a resource.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnshareResourceBody {
    /// Resource type.
    pub resource_type: ResourceType,

    /// Target user email.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_email: Option<String>,

    /// Target group ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_id: Option<String>,

    /// Target API key ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace_api_key_id: Option<String>,
}

/// Request to copy resource to workspace.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CopyResourceBody {
    /// Resource type.
    pub resource_type: ResourceType,

    /// Target user ID.
    pub target_user_id: String,
}

/// API key model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiKeyModel {
    /// Key name.
    pub name: String,

    /// Key hint (partial key).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hint: Option<String>,

    /// Key identifier.
    pub key_id: String,

    /// Service account user ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_account_user_id: Option<String>,

    /// Creation timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at_unix: Option<i64>,

    /// Whether key is disabled.
    #[serde(default)]
    pub is_disabled: bool,

    /// Permissions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permissions: Option<Vec<ApiPermission>>,

    /// Character limit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub character_limit: Option<i64>,

    /// Characters used.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub character_count: Option<i64>,
}

/// Service account model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ServiceAccountModel {
    /// Service account user ID.
    pub service_account_user_id: String,

    /// Account name.
    pub name: String,

    /// Creation timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at_unix: Option<i64>,

    /// Associated API keys.
    #[serde(rename = "api-keys", skip_serializing_if = "Option::is_none")]
    pub api_keys: Option<Vec<ApiKeyModel>>,
}

/// Response from list service accounts endpoint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListServiceAccountsResponse {
    /// List of service accounts.
    #[serde(rename = "service-accounts")]
    pub service_accounts: Vec<ServiceAccountModel>,
}

/// Response from list API keys endpoint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListApiKeysResponse {
    /// List of API keys.
    #[serde(rename = "api-keys")]
    pub api_keys: Vec<ApiKeyModel>,
}

/// Permission specification for API keys.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PermissionSpec {
    /// All permissions.
    All(String),
    /// Specific permissions.
    List(Vec<ApiPermission>),
}

impl Default for PermissionSpec {
    fn default() -> Self {
        Self::All("all".to_string())
    }
}

/// Request to create an API key.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateApiKeyBody {
    /// Key name.
    pub name: String,

    /// Permissions ("all" or list).
    pub permissions: PermissionSpec,

    /// Character limit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub character_limit: Option<i64>,
}

/// Response from create API key endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct CreateApiKeyResponse {
    /// The full API key (only shown once).
    pub xi_api_key: String,

    /// Key identifier.
    pub key_id: String,
}

/// Request to update an API key.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct UpdateApiKeyBody {
    /// Enable/disable the key.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_enabled: Option<bool>,

    /// New name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// New permissions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permissions: Option<PermissionSpec>,

    /// New character limit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub character_limit: Option<i64>,
}

/// Webhook product model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProductModel {
    /// Product identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub product_id: Option<String>,

    /// Product name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Webhook model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WebhookModel {
    /// Webhook name.
    pub name: String,

    /// Webhook identifier.
    pub webhook_id: String,

    /// Webhook URL.
    pub webhook_url: String,

    /// Whether webhook is disabled.
    #[serde(default)]
    pub is_disabled: bool,

    /// Whether webhook was auto-disabled.
    #[serde(default)]
    pub is_auto_disabled: bool,

    /// Creation timestamp.
    pub created_at_unix: i64,

    /// Authentication type.
    pub auth_type: WebhookAuthType,

    /// Active usages.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<Vec<ProductModel>>,

    /// Most recent failure error code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub most_recent_failure_error_code: Option<i64>,

    /// Most recent failure timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub most_recent_failure_timestamp: Option<i64>,
}

/// Response from list webhooks endpoint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ListWebhooksResponse {
    /// List of webhooks.
    pub webhooks: Vec<WebhookModel>,
}

/// Webhook settings for creation.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WebhookSettings {
    /// Authentication type.
    pub auth_type: WebhookAuthType,

    /// Webhook name.
    #[serde(default)]
    pub name: String,

    /// Webhook URL.
    pub webhook_url: String,
}

/// Request to create a webhook.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateWebhookBody {
    /// Webhook settings.
    pub settings: WebhookSettings,
}

/// Response from create webhook endpoint.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateWebhookResponse {
    /// Webhook identifier.
    pub webhook_id: String,

    /// Webhook secret (for HMAC auth).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub webhook_secret: Option<String>,
}

/// Request to update a webhook.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UpdateWebhookBody {
    /// Disable/enable the webhook.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_disabled: Option<bool>,

    /// New name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

// =============================================================================
// WebSocket Message Types (Phase 7)
// =============================================================================

/// WebSocket TTS initialization message (BOS).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TtsInitMessage {
    /// Space character to initialize.
    pub text: String,

    /// Voice settings.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice_settings: Option<VoiceSettings>,

    /// Generation config.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation_config: Option<GenerationConfig>,

    /// Pronunciation dictionaries.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pronunciation_dictionary_locators: Option<Vec<PronunciationDictionaryLocator>>,

    /// Inline API key (optional).
    #[serde(rename = "xi-api-key", skip_serializing_if = "Option::is_none")]
    pub xi_api_key: Option<String>,
}

impl Default for TtsInitMessage {
    fn default() -> Self {
        Self {
            text: " ".to_string(),
            voice_settings: None,
            generation_config: None,
            pronunciation_dictionary_locators: None,
            xi_api_key: None,
        }
    }
}

/// WebSocket TTS text message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TtsTextMessage {
    /// Text to synthesize (should end with space).
    pub text: String,

    /// Try to trigger generation immediately.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub try_trigger_generation: Option<bool>,

    /// Flush buffer.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flush: Option<bool>,
}

/// WebSocket TTS close message (EOS).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TtsCloseMessage {
    /// Empty string to signal close.
    pub text: String,
}

/// WebSocket TTS audio response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TtsAudioResponse {
    /// Base64-encoded audio data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio: Option<String>,

    /// Whether this is the final message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_final: Option<bool>,

    /// Alignment data for original text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alignment: Option<WebSocketAlignment>,

    /// Alignment data for normalized text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub normalized_alignment: Option<WebSocketAlignment>,
}

/// Multi-context WebSocket init message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MultiContextInitMessage {
    /// Space character to initialize.
    pub text: String,

    /// Context identifier.
    pub context_id: String,

    /// Voice settings.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice_settings: Option<VoiceSettings>,

    /// Generation config.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation_config: Option<GenerationConfig>,
}

/// Multi-context WebSocket text message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MultiContextTextMessage {
    /// Text to synthesize.
    pub text: String,

    /// Context identifier.
    pub context_id: String,

    /// Flush buffer.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flush: Option<bool>,
}

/// Multi-context close context message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MultiContextCloseMessage {
    /// Context identifier.
    pub context_id: String,

    /// Close this context.
    pub close_context: bool,
}

/// Multi-context close socket message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MultiContextCloseSocketMessage {
    /// Close the entire socket.
    pub close_socket: bool,
}

/// Multi-context audio response.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MultiContextAudioResponse {
    /// Base64-encoded audio data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio: Option<String>,

    /// Context identifier.
    pub context_id: String,

    /// Whether this is the final message for this context.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_final: Option<bool>,

    /// Alignment data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alignment: Option<WebSocketAlignment>,

    /// Normalized alignment data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub normalized_alignment: Option<WebSocketAlignment>,
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_format_serialization() {
        assert_eq!(
            serde_json::to_string(&OutputFormat::Mp3_44100_128).unwrap(),
            "\"mp3_44100_128\""
        );
        assert_eq!(
            serde_json::to_string(&OutputFormat::Pcm16000).unwrap(),
            "\"pcm_16000\""
        );
    }

    #[test]
    fn output_format_deserialization() {
        let format: OutputFormat = serde_json::from_str("\"mp3_44100_128\"").unwrap();
        assert_eq!(format, OutputFormat::Mp3_44100_128);

        let format: OutputFormat = serde_json::from_str("\"pcm_16000\"").unwrap();
        assert_eq!(format, OutputFormat::Pcm16000);
    }

    #[test]
    fn output_format_default() {
        assert_eq!(OutputFormat::default(), OutputFormat::Mp3_44100_128);
    }

    #[test]
    fn voice_category_serialization() {
        assert_eq!(
            serde_json::to_string(&VoiceCategory::Premade).unwrap(),
            "\"premade\""
        );
        assert_eq!(
            serde_json::to_string(&VoiceCategory::HighQuality).unwrap(),
            "\"high_quality\""
        );
    }

    #[test]
    fn text_normalization_default() {
        assert_eq!(TextNormalization::default(), TextNormalization::Auto);
    }

    #[test]
    fn subscription_status_serialization() {
        assert_eq!(
            serde_json::to_string(&SubscriptionStatus::Active).unwrap(),
            "\"active\""
        );
        assert_eq!(
            serde_json::to_string(&SubscriptionStatus::FreeDisabled).unwrap(),
            "\"free_disabled\""
        );
    }

    #[test]
    fn billing_period_serialization() {
        assert_eq!(
            serde_json::to_string(&BillingPeriod::Monthly).unwrap(),
            "\"monthly\""
        );
        assert_eq!(
            serde_json::to_string(&BillingPeriod::ThreeMonth).unwrap(),
            "\"3-month\""
        );
    }

    #[test]
    fn voice_settings_default() {
        let settings = VoiceSettings::default();
        assert_eq!(settings.stability, 0.5);
        assert_eq!(settings.similarity_boost, 0.75);
        assert!(settings.use_speaker_boost);
    }

    #[test]
    fn voice_settings_roundtrip() {
        let settings = VoiceSettings {
            stability: 0.7,
            similarity_boost: 0.8,
            style: Some(0.5),
            speed: Some(1.1),
            use_speaker_boost: false,
        };

        let json = serde_json::to_string(&settings).unwrap();
        let parsed: VoiceSettings = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.stability, settings.stability);
        assert_eq!(parsed.similarity_boost, settings.similarity_boost);
        assert_eq!(parsed.style, settings.style);
        assert_eq!(parsed.speed, settings.speed);
        assert_eq!(parsed.use_speaker_boost, settings.use_speaker_boost);
    }

    #[test]
    fn http_alignment_roundtrip() {
        let alignment = HttpAlignment {
            characters: vec!["H".into(), "i".into()],
            character_start_times_seconds: vec![0.0, 0.1],
            character_end_times_seconds: vec![0.1, 0.2],
        };

        let json = serde_json::to_string(&alignment).unwrap();
        assert!(json.contains("characterStartTimesSeconds"));

        let parsed: HttpAlignment = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.characters, alignment.characters);
    }

    #[test]
    fn websocket_alignment_roundtrip() {
        let alignment = WebSocketAlignment {
            chars: vec!["H".into(), "i".into()],
            char_start_times_ms: vec![0, 100],
            char_durations_ms: vec![100, 150],
        };

        let json = serde_json::to_string(&alignment).unwrap();
        assert!(json.contains("charStartTimesMs"));

        let parsed: WebSocketAlignment = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.chars, alignment.chars);
    }

    #[test]
    fn pronunciation_dictionary_locator_roundtrip() {
        let locator = PronunciationDictionaryLocator {
            pronunciation_dictionary_id: "dict-123".to_string(),
            version_id: "v1".to_string(),
        };

        let json = serde_json::to_string(&locator).unwrap();
        let parsed: PronunciationDictionaryLocator = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.pronunciation_dictionary_id, "dict-123");
        assert_eq!(parsed.version_id, "v1");
    }

    #[test]
    fn status_response_roundtrip() {
        let response = StatusResponse {
            status: "ok".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        let parsed: StatusResponse = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed.status, "ok");
    }

    #[test]
    fn generation_config_default() {
        let config = GenerationConfig::default();
        assert_eq!(
            config.chunk_length_schedule,
            Some(vec![120, 160, 250, 290])
        );
    }

    #[test]
    fn api_permission_serialization() {
        assert_eq!(
            serde_json::to_string(&ApiPermission::TextToSpeech).unwrap(),
            "\"text_to_speech\""
        );
        assert_eq!(
            serde_json::to_string(&ApiPermission::SpeechHistoryRead).unwrap(),
            "\"speech_history_read\""
        );
    }

    #[test]
    fn webhook_auth_type_serialization() {
        assert_eq!(
            serde_json::to_string(&WebhookAuthType::Hmac).unwrap(),
            "\"hmac\""
        );
        assert_eq!(
            serde_json::to_string(&WebhookAuthType::Oauth2).unwrap(),
            "\"oauth2\""
        );
    }

    #[test]
    fn token_type_serialization() {
        assert_eq!(
            serde_json::to_string(&TokenType::RealtimeScribe).unwrap(),
            "\"realtime_scribe\""
        );
        assert_eq!(
            serde_json::to_string(&TokenType::TtsWebsocket).unwrap(),
            "\"tts_websocket\""
        );
    }
}
