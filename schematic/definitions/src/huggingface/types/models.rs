use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::common::*;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ModelInfo {
    #[serde(rename = "modelId", alias = "id")]
    pub model_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sha: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(rename = "lastModified", skip_serializing_if = "Option::is_none")]
    pub last_modified: Option<String>,
    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(default)]
    pub private: bool,
    #[serde(default)]
    pub disabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gated: Option<GatedStatus>,
    #[serde(default)]
    pub downloads: u64,
    #[serde(rename = "downloadsAllTime", skip_serializing_if = "Option::is_none")]
    pub downloads_all_time: Option<u64>,
    #[serde(default)]
    pub likes: u64,
    #[serde(rename = "pipeline_tag", skip_serializing_if = "Option::is_none")]
    pub pipeline_tag: Option<String>,
    #[serde(rename = "library_name", skip_serializing_if = "Option::is_none")]
    pub library_name: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub siblings: Vec<RepoFile>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub spaces: Vec<String>,
    #[serde(rename = "cardData", skip_serializing_if = "Option::is_none")]
    pub card_data: Option<CardData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub safetensors: Option<SafetensorsInfo>,
    #[serde(rename = "transformersInfo", skip_serializing_if = "Option::is_none")]
    pub transformers_info: Option<TransformersInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config: Option<HashMap<String, serde_json::Value>>,
    #[serde(rename = "trendingScore", skip_serializing_if = "Option::is_none")]
    pub trending_score: Option<f64>,
    #[serde(rename = "inference", skip_serializing_if = "Option::is_none")]
    pub inference: Option<InferenceStatus>,
    #[serde(rename = "mask_token", skip_serializing_if = "Option::is_none")]
    pub mask_token: Option<String>,
    #[serde(rename = "widgetData", skip_serializing_if = "Option::is_none")]
    pub widget_data: Option<Vec<WidgetConfig>>,
    #[serde(rename = "model-index", skip_serializing_if = "Option::is_none")]
    pub model_index: Option<Vec<ModelIndexEntry>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ModelSummary {
    #[serde(rename = "modelId", alias = "_id", alias = "id")]
    pub model_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(rename = "lastModified", skip_serializing_if = "Option::is_none")]
    pub last_modified: Option<String>,
    #[serde(default)]
    pub downloads: u64,
    #[serde(default)]
    pub likes: u64,
    #[serde(rename = "pipeline_tag", skip_serializing_if = "Option::is_none")]
    pub pipeline_tag: Option<String>,
    #[serde(rename = "library_name", skip_serializing_if = "Option::is_none")]
    pub library_name: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default)]
    pub private: bool,
}
