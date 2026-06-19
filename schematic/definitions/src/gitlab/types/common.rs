use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct User {
    pub id: u64,
    pub username: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub avatar_url: Option<String>,
    #[serde(default)]
    pub web_url: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct References {
    pub short: String,
    pub relative: String,
    pub full: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TimeStats {
    #[serde(default)]
    pub time_estimate: u64,
    #[serde(default)]
    pub total_time_spent: u64,
    #[serde(default)]
    pub human_time_estimate: Option<String>,
    #[serde(default)]
    pub human_total_time_spent: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TaskCompletionStatus {
    #[serde(default)]
    pub count: u64,
    #[serde(default)]
    pub completed_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Milestone {
    pub id: u64,
    pub iid: u64,
    #[serde(default)]
    pub project_id: Option<u64>,
    pub title: String,
    #[serde(default)]
    pub description: Option<String>,
    pub state: String,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
    #[serde(default)]
    pub due_date: Option<String>,
    #[serde(default)]
    pub start_date: Option<String>,
    #[serde(default)]
    pub expired: Option<bool>,
    #[serde(default)]
    pub web_url: Option<String>,
}
