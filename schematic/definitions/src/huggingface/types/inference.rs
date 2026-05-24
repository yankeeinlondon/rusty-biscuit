use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct InferenceRequest {
    pub inputs: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<InferenceParameters>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<InferenceOptions>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct InferenceParameters {
    #[serde(rename = "max_new_tokens", skip_serializing_if = "Option::is_none")]
    pub max_new_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f64>,
    #[serde(rename = "top_p", skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f64>,
    #[serde(rename = "top_k", skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,
    #[serde(rename = "repetition_penalty", skip_serializing_if = "Option::is_none")]
    pub repetition_penalty: Option<f64>,
    #[serde(rename = "do_sample", skip_serializing_if = "Option::is_none")]
    pub do_sample: Option<bool>,
    #[serde(rename = "return_full_text", skip_serializing_if = "Option::is_none")]
    pub return_full_text: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<u64>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub stop: Vec<String>,
    #[serde(rename = "candidate_labels", skip_serializing_if = "Option::is_none")]
    pub candidate_labels: Option<Vec<String>>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct InferenceOptions {
    #[serde(rename = "wait_for_model", default)]
    pub wait_for_model: bool,
    #[serde(rename = "use_cache", default = "default_true")]
    pub use_cache: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(untagged)]
pub enum InferenceResponse {
    TextGeneration(Vec<TextGenerationResult>),
    Classification(Vec<ClassificationResult>),
    TokenClassification(Vec<TokenClassificationResult>),
    ZeroShotClassification(ZeroShotResult),
    QuestionAnswering(QuestionAnsweringResult),
    FeatureExtraction(Vec<Vec<f64>>),
    Raw(serde_json::Value),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TextGenerationResult {
    #[serde(rename = "generated_text")]
    pub generated_text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ClassificationResult {
    pub label: String,
    pub score: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TokenClassificationResult {
    #[serde(rename = "entity_group", alias = "entity")]
    pub entity_group: String,
    pub score: f64,
    pub word: String,
    pub start: u64,
    pub end: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ZeroShotResult {
    pub sequence: String,
    pub labels: Vec<String>,
    pub scores: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct QuestionAnsweringResult {
    pub answer: String,
    pub score: f64,
    pub start: u64,
    pub end: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ApiError {
    pub error: String,
    #[serde(rename = "error_type", skip_serializing_if = "Option::is_none")]
    pub error_type: Option<String>,
    #[serde(rename = "estimated_time", skip_serializing_if = "Option::is_none")]
    pub estimated_time: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub warnings: Option<Vec<String>>,
}
