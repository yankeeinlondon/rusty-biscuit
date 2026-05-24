use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::common::*;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SpaceInfo {
    #[serde(rename = "id", alias = "spaceId")]
    pub id: String,
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
    #[serde(default)]
    pub likes: u64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sdk: Option<SpaceSdk>,
    #[serde(rename = "sdk_version", skip_serializing_if = "Option::is_none")]
    pub sdk_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime: Option<SpaceRuntime>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub models: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub datasets: Vec<String>,
    #[serde(rename = "cardData", skip_serializing_if = "Option::is_none")]
    pub card_data: Option<SpaceCardData>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub siblings: Vec<RepoFile>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoji: Option<String>,
    #[serde(rename = "colorFrom", skip_serializing_if = "Option::is_none")]
    pub color_from: Option<String>,
    #[serde(rename = "colorTo", skip_serializing_if = "Option::is_none")]
    pub color_to: Option<String>,
    #[serde(default)]
    pub pinned: bool,
    #[serde(rename = "trendingScore", skip_serializing_if = "Option::is_none")]
    pub trending_score: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SpaceRuntime {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stage: Option<SpaceStage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hardware: Option<SpaceHardware>,
    #[serde(rename = "requestedHardware", skip_serializing_if = "Option::is_none")]
    pub requested_hardware: Option<SpaceHardware>,
    #[serde(rename = "gcTimeout", skip_serializing_if = "Option::is_none")]
    pub gc_timeout: Option<u64>,
    #[serde(rename = "errorMessage", skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub replicas: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SpaceCardData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub emoji: Option<String>,
    #[serde(rename = "colorFrom", skip_serializing_if = "Option::is_none")]
    pub color_from: Option<String>,
    #[serde(rename = "colorTo", skip_serializing_if = "Option::is_none")]
    pub color_to: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sdk: Option<SpaceSdk>,
    #[serde(rename = "sdk_version", skip_serializing_if = "Option::is_none")]
    pub sdk_version: Option<String>,
    #[serde(rename = "app_file", skip_serializing_if = "Option::is_none")]
    pub app_file: Option<String>,
    #[serde(rename = "app_port", skip_serializing_if = "Option::is_none")]
    pub app_port: Option<u16>,
    #[serde(rename = "pinned", default)]
    pub pinned: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    #[serde(rename = "suggested_hardware", skip_serializing_if = "Option::is_none")]
    pub suggested_hardware: Option<SpaceHardware>,
    #[serde(rename = "suggested_storage", skip_serializing_if = "Option::is_none")]
    pub suggested_storage: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub models: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub datasets: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(rename = "short_description", skip_serializing_if = "Option::is_none")]
    pub short_description: Option<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SpaceSummary {
    #[serde(rename = "id", alias = "_id")]
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(rename = "lastModified", skip_serializing_if = "Option::is_none")]
    pub last_modified: Option<String>,
    #[serde(default)]
    pub likes: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sdk: Option<SpaceSdk>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default)]
    pub private: bool,
}
