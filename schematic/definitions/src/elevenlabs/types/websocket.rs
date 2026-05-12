use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::common::*;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TtsInitMessage {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice_settings: Option<VoiceSettings>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation_config: Option<GenerationConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pronunciation_dictionary_locators: Option<Vec<PronunciationDictionaryLocator>>,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TtsTextMessage {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub try_trigger_generation: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flush: Option<bool>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TtsCloseMessage {
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TtsAudioResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_final: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alignment: Option<WebSocketAlignment>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub normalized_alignment: Option<WebSocketAlignment>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct MultiContextInitMessage {
    pub text: String,
    pub context_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub voice_settings: Option<VoiceSettings>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generation_config: Option<GenerationConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct MultiContextTextMessage {
    pub text: String,
    pub context_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flush: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct MultiContextCloseMessage {
    pub context_id: String,
    pub close_context: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct MultiContextCloseSocketMessage {
    pub close_socket: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct MultiContextAudioResponse {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub audio: Option<String>,
    pub context_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_final: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alignment: Option<WebSocketAlignment>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub normalized_alignment: Option<WebSocketAlignment>,
}
