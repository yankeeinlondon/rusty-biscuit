use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::common::*;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CreateRepoBody {
    pub name: String,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub repo_type: Option<RepoType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization: Option<String>,
    #[serde(default)]
    pub private: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sdk: Option<SpaceSdk>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hardware: Option<SpaceHardware>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage: Option<String>,
    #[serde(rename = "sleepTimeSeconds", skip_serializing_if = "Option::is_none")]
    pub sleep_time_seconds: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secrets: Option<Vec<SpaceSecret>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variables: Option<Vec<SpaceVariable>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct SpaceSecret {
    pub key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct SpaceVariable {
    pub key: String,
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CreateRepoResponse {
    pub url: String,
    #[serde(rename = "repoId", skip_serializing_if = "Option::is_none")]
    pub repo_id: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct DeleteRepoBody {
    #[serde(rename = "repoId")]
    pub repo_id: String,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub repo_type: Option<RepoType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub organization: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct MoveRepoBody {
    #[serde(rename = "fromRepo")]
    pub from_repo: String,
    #[serde(rename = "toRepo")]
    pub to_repo: String,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub repo_type: Option<RepoType>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct UpdateRepoSettingsBody {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gated: Option<GatedStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub private: Option<bool>,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct UpdateSpaceSettingsRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hardware: Option<SpaceHardware>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub storage: Option<String>,
    #[serde(rename = "sleepTimeSeconds", skip_serializing_if = "Option::is_none")]
    pub sleep_time_seconds: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secrets: Option<Vec<SpaceSecret>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variables: Option<Vec<SpaceVariable>>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct UploadFileRequest {
    pub path: String,
    #[serde(rename = "commitMessage", skip_serializing_if = "Option::is_none")]
    pub commit_message: Option<String>,
    #[serde(rename = "commitDescription", skip_serializing_if = "Option::is_none")]
    pub commit_description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    #[serde(rename = "createBranch", default)]
    pub create_branch: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct UploadFileResponse {
    #[serde(rename = "commitOid", skip_serializing_if = "Option::is_none")]
    pub commit_oid: Option<String>,
    #[serde(rename = "commitUrl", skip_serializing_if = "Option::is_none")]
    pub commit_url: Option<String>,
}
