use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::common::*;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct WebhookPayload {
    pub event: WebhookEvent,
    pub repo: WebhookRepo,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub discussion: Option<WebhookDiscussion>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<WebhookComment>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub webhook: Option<WebhookConfig>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct WebhookEvent {
    pub action: String,
    pub scope: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct WebhookRepo {
    #[serde(rename = "type")]
    pub repo_type: RepoType,
    pub name: String,
    pub id: String,
    pub private: bool,
    #[serde(rename = "headSha", skip_serializing_if = "Option::is_none")]
    pub head_sha: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct WebhookDiscussion {
    pub num: u64,
    pub title: String,
    pub status: DiscussionStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct WebhookComment {
    pub id: String,
    pub author: String,
    pub hidden: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct WebhookConfig {
    pub id: String,
}
