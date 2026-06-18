use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::common::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TreeItem {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub item_type: String,
    pub path: String,
    #[serde(default)]
    pub mode: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct FileContent {
    pub file_name: String,
    pub file_path: String,
    pub size: u64,
    pub encoding: String,
    pub content: String,
    #[serde(rename = "ref")]
    pub ref_name: String,
    pub blob_id: String,
    pub commit_id: String,
    #[serde(default)]
    pub content_sha256: Option<String>,
    #[serde(default)]
    pub last_commit_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Namespace {
    pub id: u64,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub full_path: Option<String>,
    #[serde(default)]
    pub parent_id: Option<u64>,
    #[serde(default)]
    pub avatar_url: Option<String>,
    #[serde(default)]
    pub web_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Project {
    pub id: u64,
    pub name: String,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub path_with_namespace: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub default_branch: Option<String>,
    #[serde(default)]
    pub visibility: Option<String>,
    #[serde(default)]
    pub web_url: Option<String>,
    #[serde(default)]
    pub ssh_url_to_repo: Option<String>,
    #[serde(default)]
    pub http_url_to_repo: Option<String>,
    #[serde(default)]
    pub namespace: Option<Namespace>,
    #[serde(default)]
    pub star_count: Option<u64>,
    #[serde(default)]
    pub forks_count: Option<u64>,
    #[serde(default)]
    pub open_issues_count: Option<u64>,
    #[serde(default)]
    pub empty_repo: Option<bool>,
    #[serde(default)]
    pub archived: bool,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub last_activity_at: Option<String>,
    #[serde(default)]
    pub owner: Option<User>,
    #[serde(default)]
    pub topics: Vec<String>,
    #[serde(default)]
    pub issues_enabled: Option<bool>,
    #[serde(default)]
    pub merge_requests_enabled: Option<bool>,
    #[serde(default)]
    pub wiki_enabled: Option<bool>,
    #[serde(default)]
    pub avatar_url: Option<String>,
}
