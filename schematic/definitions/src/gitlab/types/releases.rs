use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::common::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Tag {
    pub name: String,
    #[serde(default)]
    pub message: Option<String>,
    pub target: String,
    pub commit: TagCommit,
    #[serde(default)]
    pub release: Option<TagRelease>,
    #[serde(default)]
    pub protected: bool,
    #[serde(default)]
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TagCommit {
    pub id: String,
    pub short_id: String,
    pub title: String,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub parent_ids: Vec<String>,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub author_name: Option<String>,
    #[serde(default)]
    pub author_email: Option<String>,
    #[serde(default)]
    pub authored_date: Option<String>,
    #[serde(default)]
    pub committer_name: Option<String>,
    #[serde(default)]
    pub committer_email: Option<String>,
    #[serde(default)]
    pub committed_date: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TagRelease {
    pub tag_name: String,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Release {
    pub name: String,
    pub tag_name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub description_html: Option<String>,
    pub created_at: String,
    pub released_at: String,
    pub author: User,
    pub commit: ReleaseCommit,
    #[serde(default)]
    pub milestones: Vec<Milestone>,
    #[serde(default)]
    pub commit_path: Option<String>,
    #[serde(default)]
    pub tag_path: Option<String>,
    pub assets: ReleaseAssets,
    #[serde(default)]
    pub evidences: Vec<ReleaseEvidence>,
    #[serde(rename = "_links", default)]
    pub links: Option<ReleaseLinks>,
    #[serde(default)]
    pub upcoming_release: Option<bool>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ReleaseCommit {
    pub id: String,
    pub short_id: String,
    pub title: String,
    pub created_at: String,
    #[serde(default)]
    pub parent_ids: Vec<String>,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub author_name: Option<String>,
    #[serde(default)]
    pub author_email: Option<String>,
    #[serde(default)]
    pub authored_date: Option<String>,
    #[serde(default)]
    pub committer_name: Option<String>,
    #[serde(default)]
    pub committer_email: Option<String>,
    #[serde(default)]
    pub committed_date: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ReleaseAssets {
    #[serde(default)]
    pub count: u64,
    #[serde(default)]
    pub sources: Vec<ReleaseSource>,
    #[serde(default)]
    pub links: Vec<ReleaseLink>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ReleaseSource {
    pub format: String,
    pub url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ReleaseLink {
    pub id: u64,
    pub name: String,
    pub url: String,
    #[serde(default)]
    pub external: bool,
    #[serde(default)]
    pub link_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ReleaseEvidence {
    pub sha: String,
    pub filepath: String,
    pub collected_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ReleaseLinks {
    #[serde(rename = "self")]
    pub self_url: String,
    #[serde(default)]
    pub edit_url: Option<String>,
    #[serde(default)]
    pub closed_issues_url: Option<String>,
    #[serde(default)]
    pub closed_merge_requests_url: Option<String>,
    #[serde(default)]
    pub merged_merge_requests_url: Option<String>,
    #[serde(default)]
    pub opened_issues_url: Option<String>,
    #[serde(default)]
    pub opened_merge_requests_url: Option<String>,
}
