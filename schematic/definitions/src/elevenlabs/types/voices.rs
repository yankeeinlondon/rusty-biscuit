use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::common::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct SampleModel {
    pub sample_id: String,
    pub file_name: String,
    pub mime_type: String,
    pub size_bytes: i64,
    pub hash: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct LanguageModel {
    #[serde(alias = "language")]
    pub language_id: String,
    #[serde(default)]
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locale: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview_url: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct FineTuningModel {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<std::collections::HashMap<String, FineTuningState>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verification_required: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_allowed_to_fine_tune: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verification_failures: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verification_attempts_count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manual_verification_requested: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub progress: Option<std::collections::HashMap<String, serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<std::collections::HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dataset_duration_seconds: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_verification_attempts: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_max_verification_attempts_reset_unix_ms: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SharingModel {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_owner_id: Option<String>,
    #[serde(default)]
    pub is_public: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cloned_by_count: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct VoiceVerificationModel {
    pub requires_verification: bool,
    pub is_verified: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct VoiceResponseModel {
    pub voice_id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<VoiceCategory>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub samples: Option<Vec<SampleModel>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<VoiceSettings>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fine_tuning: Option<FineTuningModel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sharing: Option<SharingModel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verified_languages: Option<Vec<LanguageModel>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice_verification: Option<VoiceVerificationModel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<std::collections::HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub available_for_tiers: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub high_quality_base_model_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub collection_ids: Option<Vec<String>>,
    #[serde(default)]
    pub is_legacy: bool,
    #[serde(default)]
    pub is_mixed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_owner: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission_on_resource: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub favorited_at_unix: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at_unix: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safety_control: Option<SafetyControl>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ListVoicesResponse {
    pub voices: Vec<VoiceResponseModel>,
    #[serde(default)]
    pub has_more: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_count: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_page_token: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct LibraryVoiceResponseModel {
    pub voice_id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<VoiceCategory>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub public_owner_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ListSharedVoicesResponse {
    pub voices: Vec<LibraryVoiceResponseModel>,
    pub has_more: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_sort_id: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct AddSharedVoiceBody {
    pub new_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct AddSharedVoiceResponse {
    pub voice_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct AddSampleResponse {
    pub sample_id: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CreatePvcVoiceBody {
    pub name: String,
    pub language: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub labels: Option<std::collections::HashMap<String, String>>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TrainPvcVoiceBody {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
}
