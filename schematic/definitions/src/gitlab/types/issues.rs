use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::common::*;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Issue {
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
    pub closed_at: Option<String>,
    #[serde(default)]
    pub closed_by: Option<User>,
    #[serde(default)]
    pub labels: Vec<String>,
    #[serde(default)]
    pub milestone: Option<Milestone>,
    #[serde(default)]
    pub assignees: Vec<User>,
    pub author: User,
    #[serde(default)]
    pub assignee: Option<User>,
    #[serde(default)]
    pub user_notes_count: u64,
    #[serde(default)]
    pub upvotes: u64,
    #[serde(default)]
    pub downvotes: u64,
    #[serde(default)]
    pub due_date: Option<String>,
    #[serde(default)]
    pub confidential: bool,
    #[serde(default)]
    pub discussion_locked: Option<bool>,
    #[serde(default)]
    pub issue_type: Option<String>,
    #[serde(default)]
    pub web_url: Option<String>,
    #[serde(default)]
    pub time_stats: Option<TimeStats>,
    #[serde(default)]
    pub task_completion_status: Option<TaskCompletionStatus>,
    #[serde(default)]
    pub weight: Option<u64>,
    #[serde(default)]
    pub has_tasks: Option<bool>,
    #[serde(default)]
    pub task_status: Option<String>,
    #[serde(default)]
    pub references: Option<References>,
    #[serde(default)]
    pub severity: Option<String>,
    #[serde(default)]
    pub moved_to_id: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Note {
    pub id: u64,
    #[serde(rename = "type", default)]
    pub note_type: Option<String>,
    pub body: String,
    #[serde(default)]
    pub attachment: Option<String>,
    pub author: User,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub system: bool,
    #[serde(default)]
    pub noteable_id: Option<u64>,
    #[serde(default)]
    pub noteable_type: Option<String>,
    #[serde(default)]
    pub resolvable: bool,
    #[serde(default)]
    pub resolved: Option<bool>,
    #[serde(default)]
    pub resolved_by: Option<User>,
    #[serde(default)]
    pub confidential: bool,
    #[serde(default)]
    pub internal: bool,
}
