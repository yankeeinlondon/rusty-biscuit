use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::common::*;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Discussion {
    pub num: u64,
    pub title: String,
    pub status: DiscussionStatus,
    #[serde(rename = "isPullRequest", default)]
    pub is_pull_request: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<Author>,
    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(rename = "numComments", skip_serializing_if = "Option::is_none")]
    pub num_comments: Option<u64>,
    #[serde(default)]
    pub pinned: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub labels: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct DiscussionComment {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<Author>,
    pub content: String,
    #[serde(rename = "createdAt", skip_serializing_if = "Option::is_none")]
    pub created_at: Option<String>,
    #[serde(rename = "updatedAt", skip_serializing_if = "Option::is_none")]
    pub updated_at: Option<String>,
    #[serde(default)]
    pub hidden: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct DiscussionList {
    pub discussions: Vec<Discussion>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<u64>,
}
