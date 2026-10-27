use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::common::*;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct CreateSpeechBody {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language_code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice_settings: Option<VoiceSettings>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pronunciation_dictionary_locators: Option<Vec<PronunciationDictionaryLocator>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub previous_request_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_request_ids: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub apply_text_normalization: Option<TextNormalization>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub apply_language_text_normalization: Option<bool>,
}
impl schematic_define::Registration for CreateSpeechBody {
    const RUST_IDENTIFIER: Option<&'static str> = Some("CreateSpeechBody");
    fn register(ctx: &mut schematic_define::DefinitionContext) -> Result<schematic_define::TypeDesc, schematic_define::DefinitionError> {
        ctx.register_nominal::<Self>()
    }
}


/// One with-timestamps reply, or one line of the streaming variant. Field names
/// are snake_case on the wire (`audio_base64`), as the provider reference lists.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SpeechWithTimestampsResponse {
    pub audio_base64: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alignment: Option<HttpAlignment>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub normalized_alignment: Option<HttpAlignment>,
}
impl schematic_define::Registration for SpeechWithTimestampsResponse {
    const RUST_IDENTIFIER: Option<&'static str> = Some("SpeechWithTimestampsResponse");
    fn register(ctx: &mut schematic_define::DefinitionContext) -> Result<schematic_define::TypeDesc, schematic_define::DefinitionError> {
        ctx.register_nominal::<Self>()
    }
}
