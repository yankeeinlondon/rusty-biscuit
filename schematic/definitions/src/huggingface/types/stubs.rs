use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::common::*;

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct StatusResponse {
    pub ok: Option<bool>,
    pub message: Option<String>,
}

pub type Commit = CommitInfo;

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct FileMetadata {
    pub path: String,
    pub size: Option<u64>,
    pub oid: Option<String>,
    #[serde(rename = "type")]
    pub file_type: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct RepoInfo {
    pub id: String,
    #[serde(rename = "type")]
    pub repo_type: Option<RepoType>,
    pub private: Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct RepoUrl {
    pub url: String,
}
