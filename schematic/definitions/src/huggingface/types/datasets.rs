use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::common::*;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct DatasetInfo {
    #[serde(rename = "id", alias = "datasetId")]
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gated: Option<GatedStatus>,
    #[serde(default)]
    pub downloads: u64,
    #[serde(default)]
    pub likes: u64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub siblings: Vec<RepoFile>,
    #[serde(rename = "cardData", skip_serializing_if = "Option::is_none")]
    pub card_data: Option<DatasetCardData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub citation: Option<String>,
    #[serde(rename = "trendingScore", skip_serializing_if = "Option::is_none")]
    pub trending_score: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct DatasetCardData {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub language: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(
        rename = "task_categories",
        default,
        skip_serializing_if = "Vec::is_empty"
    )]
    pub task_categories: Vec<String>,
    #[serde(rename = "task_ids", default, skip_serializing_if = "Vec::is_empty")]
    pub task_ids: Vec<String>,
    #[serde(
        rename = "size_categories",
        default,
        skip_serializing_if = "Vec::is_empty"
    )]
    pub size_categories: Vec<String>,
    #[serde(rename = "pretty_name", skip_serializing_if = "Option::is_none")]
    pub pretty_name: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub configs: Vec<DatasetConfig>,
    #[serde(flatten)]
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct DatasetConfig {
    #[serde(rename = "config_name", skip_serializing_if = "Option::is_none")]
    pub config_name: Option<String>,
    #[serde(rename = "data_files", skip_serializing_if = "Option::is_none")]
    pub data_files: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub splits: Vec<DatasetSplit>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct DatasetSplit {
    pub name: String,
    #[serde(rename = "num_examples", skip_serializing_if = "Option::is_none")]
    pub num_examples: Option<u64>,
    #[serde(rename = "num_bytes", skip_serializing_if = "Option::is_none")]
    pub num_bytes: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct DatasetSummary {
    #[serde(rename = "id", alias = "_id")]
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(rename = "lastModified", skip_serializing_if = "Option::is_none")]
    pub last_modified: Option<String>,
    #[serde(default)]
    pub downloads: u64,
    #[serde(default)]
    pub likes: u64,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default)]
    pub private: bool,
}
