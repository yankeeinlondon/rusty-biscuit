use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::common::*;
use super::voices::LanguageModel;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ModelRates {
    pub character_cost_multiplier: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ModelInfo {
    pub model_id: String,
    pub name: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_cost_factor: Option<f64>,
    #[serde(default)]
    pub can_do_text_to_speech: bool,
    #[serde(default)]
    pub can_do_voice_conversion: bool,
    #[serde(default)]
    pub can_be_finetuned: bool,
    #[serde(default)]
    pub can_use_style: bool,
    #[serde(default)]
    pub can_use_speaker_boost: bool,
    #[serde(default)]
    pub serves_pro_voices: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub languages: Option<Vec<LanguageModel>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_rates: Option<ModelRates>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maximum_text_length_per_request: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_characters_request_free_user: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_characters_request_subscribed_user: Option<i64>,
    #[serde(default)]
    pub requires_alpha_access: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub concurrency_group: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct SingleUseTokenResponse {
    pub token: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct FeedbackModel {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumbs_up: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub feedback: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct AlignmentModel {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub characters: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub character_start_times_seconds: Option<Vec<f64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub character_end_times_seconds: Option<Vec<f64>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SpeechHistoryItemResponseModel {
    pub history_item_id: String,
    pub voice_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice_category: Option<VoiceCategory>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    pub date_unix: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub feedback: Option<FeedbackModel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<HistorySource>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alignments: Option<AlignmentModel>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct GetHistoryResponse {
    pub history: Vec<SpeechHistoryItemResponseModel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_history_item_id: Option<String>,
    pub has_more: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scanned_until: Option<i64>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DownloadHistoryBody {
    pub history_item_ids: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_format: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct CreateSoundEffectBody {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_seconds: Option<f64>,
    #[serde(rename = "loop", skip_serializing_if = "Option::is_none")]
    pub loop_sound: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_influence: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
}
