use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OutputFormat {
    #[serde(rename = "mp3_22050_32")]
    Mp3_22050_32,
    #[serde(rename = "mp3_44100_32")]
    Mp3_44100_32,
    #[serde(rename = "mp3_44100_64")]
    Mp3_44100_64,
    #[serde(rename = "mp3_44100_96")]
    Mp3_44100_96,
    #[serde(rename = "mp3_44100_128")]
    #[default]
    Mp3_44100_128,
    #[serde(rename = "mp3_44100_192")]
    Mp3_44100_192,
    #[serde(rename = "pcm_8000")]
    Pcm8000,
    #[serde(rename = "pcm_16000")]
    Pcm16000,
    #[serde(rename = "pcm_22050")]
    Pcm22050,
    #[serde(rename = "pcm_24000")]
    Pcm24000,
    #[serde(rename = "pcm_44100")]
    Pcm44100,
    #[serde(rename = "ulaw_8000")]
    Ulaw8000,
    #[serde(rename = "alaw_8000")]
    Alaw8000,
    #[serde(rename = "opus_48000_32")]
    Opus48000_32,
    #[serde(rename = "opus_48000_64")]
    Opus48000_64,
    #[serde(rename = "opus_48000_96")]
    Opus48000_96,
    #[serde(rename = "opus_48000_128")]
    Opus48000_128,
    #[serde(rename = "opus_48000_192")]
    Opus48000_192,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum VoiceCategory {
    Premade,
    Cloned,
    Generated,
    Professional,
    Famous,
    HighQuality,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum VoiceType {
    Personal,
    Community,
    Default,
    Workspace,
    NonDefault,
    Saved,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TextNormalization {
    #[default]
    Auto,
    On,
    Off,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SubscriptionStatus {
    Trialing,
    Active,
    Incomplete,
    PastDue,
    Free,
    FreeDisabled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum BillingPeriod {
    #[serde(rename = "monthly")]
    Monthly,
    #[serde(rename = "3-month")]
    ThreeMonth,
    #[serde(rename = "6-month")]
    SixMonth,
    #[serde(rename = "annual")]
    Annual,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum Currency {
    USD,
    EUR,
    INR,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TokenType {
    RealtimeScribe,
    TtsWebsocket,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum HistorySource {
    TTS,
    STS,
    Projects,
    Dubbing,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum WebhookAuthType {
    #[default]
    Hmac,
    Oauth2,
    Mtls,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SortDirection {
    Asc,
    Desc,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AccessLevel {
    #[default]
    Admin,
    Editor,
    Commenter,
    Viewer,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FineTuningState {
    NotStarted,
    Queued,
    FineTuning,
    FineTuned,
    Failed,
    Delayed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum SpeakerSeparationStatus {
    NotStarted,
    Pending,
    Completed,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum SafetyControl {
    #[serde(rename = "NONE")]
    None,
    #[serde(rename = "BAN")]
    Ban,
    #[serde(rename = "CAPTCHA")]
    Captcha,
    #[serde(rename = "ENTERPRISE_BAN")]
    EnterpriseBan,
    #[serde(rename = "ENTERPRISE_CAPTCHA")]
    EnterpriseCaptcha,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct VoiceSettings {
    #[serde(default = "default_stability")]
    pub stability: f64,
    #[serde(default = "default_similarity_boost")]
    pub similarity_boost: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub style: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub speed: Option<f64>,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PronunciationDictionaryLocator {
    pub pronunciation_dictionary_id: String,
    pub version_id: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HttpAlignment {
    pub characters: Vec<String>,
    pub character_start_times_seconds: Vec<f64>,
    pub character_end_times_seconds: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WebSocketAlignment {
    pub chars: Vec<String>,
    pub char_start_times_ms: Vec<i64>,
    pub char_durations_ms: Vec<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct StatusResponse {
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct GenerationConfig {
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
