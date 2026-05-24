use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::common::*;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct MergeRequest {
    pub id: u64,
    pub iid: u64,
    pub project_id: u64,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    pub state: String,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub merged_at: Option<String>,
    #[serde(default)]
    pub closed_at: Option<String>,
    pub target_branch: String,
    pub source_branch: String,
    pub author: User,
    #[serde(default)]
    pub assignee: Option<User>,
    #[serde(default)]
    pub assignees: Vec<User>,
    #[serde(default)]
    pub reviewers: Vec<User>,
    #[serde(default)]
    pub source_project_id: Option<u64>,
    #[serde(default)]
    pub target_project_id: Option<u64>,
    #[serde(default)]
    pub labels: Vec<String>,
    #[serde(default)]
    pub draft: bool,
    #[serde(default)]
    pub work_in_progress: bool,
    #[serde(default)]
    pub milestone: Option<Milestone>,
    #[serde(default)]
    pub merge_when_pipeline_succeeds: bool,
    #[serde(default)]
    pub merge_status: Option<String>,
    #[serde(default)]
    pub sha: Option<String>,
    #[serde(default)]
    pub merge_commit_sha: Option<String>,
    #[serde(default)]
    pub squash_commit_sha: Option<String>,
    #[serde(default)]
    pub user_notes_count: u64,
    #[serde(default)]
    pub upvotes: u64,
    #[serde(default)]
    pub downvotes: u64,
    #[serde(default)]
    pub web_url: Option<String>,
    #[serde(default)]
    pub references: Option<References>,
    #[serde(default)]
    pub time_stats: Option<TimeStats>,
    #[serde(default)]
    pub squash: bool,
    #[serde(default)]
    pub changes_count: Option<String>,
    #[serde(default)]
    pub merged_by: Option<User>,
    #[serde(default)]
    pub closed_by: Option<User>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct MergeRequestChanges {
    pub id: u64,
    pub iid: u64,
    pub project_id: u64,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    pub state: String,
    #[serde(default)]
    pub merged_at: Option<String>,
    #[serde(default)]
    pub closed_at: Option<String>,
    pub target_branch: String,
    pub source_branch: String,
    #[serde(default)]
    pub changes: Vec<Diff>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Commit {
    pub id: String,
    pub short_id: String,
    pub created_at: String,
    #[serde(default)]
    pub parent_ids: Vec<String>,
    pub title: String,
    #[serde(default)]
    pub message: Option<String>,
    pub author_name: String,
    pub author_email: String,
    #[serde(default)]
    pub authored_date: Option<String>,
    #[serde(default)]
    pub committer_name: Option<String>,
    #[serde(default)]
    pub committer_email: Option<String>,
    #[serde(default)]
    pub committed_date: Option<String>,
    #[serde(default)]
    pub web_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Diff {
    pub old_path: String,
    pub new_path: String,
    #[serde(default)]
    pub a_mode: Option<String>,
    #[serde(default)]
    pub b_mode: Option<String>,
    #[serde(default)]
    pub diff: Option<String>,
    #[serde(default)]
    pub new_file: bool,
    #[serde(default)]
    pub renamed_file: bool,
    #[serde(default)]
    pub deleted_file: bool,
}
