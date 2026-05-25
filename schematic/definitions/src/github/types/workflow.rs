use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::common::UserSummary;

/// Response from `GET /repos/{owner}/{repo}/actions/runs`.
///
/// Contains a total count and a list of workflow runs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct WorkflowRunsResponse {
    /// Total number of workflow runs matching the query.
    #[serde(default)]
    pub total_count: Option<u64>,

    /// The workflow runs.
    #[serde(default)]
    pub workflow_runs: Vec<WorkflowRun>,
}

/// A single workflow run from GitHub Actions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct WorkflowRun {
    /// Workflow run ID.
    pub id: u64,

    /// Run display name.
    #[serde(default)]
    pub name: Option<String>,

    /// Run number (incrementing per workflow).
    #[serde(default)]
    pub run_number: Option<u64>,

    /// Run attempt number.
    #[serde(default)]
    pub run_attempt: Option<u64>,

    /// Run status (e.g., "queued", "in_progress", "completed").
    #[serde(default)]
    pub status: Option<String>,

    /// Run conclusion (e.g., "success", "failure", "cancelled", "skipped").
    #[serde(default)]
    pub conclusion: Option<String>,

    /// The workflow ID this run belongs to.
    #[serde(default)]
    pub workflow_id: Option<u64>,

    /// The event that triggered the workflow (e.g., "push", "pull_request").
    #[serde(default)]
    pub event: Option<String>,

    /// The branch or tag ref that triggered the run.
    #[serde(default)]
    pub head_branch: Option<String>,

    /// The commit SHA at the head of the run.
    #[serde(default)]
    pub head_sha: Option<String>,

    /// HTML URL to the workflow run.
    #[serde(default)]
    pub html_url: Option<String>,

    /// API URL.
    #[serde(default)]
    pub url: Option<String>,

    /// Creation timestamp (ISO 8601).
    #[serde(default)]
    pub created_at: Option<String>,

    /// Last update timestamp (ISO 8601).
    #[serde(default)]
    pub updated_at: Option<String>,

    /// The actor who triggered the run.
    #[serde(default)]
    pub actor: Option<UserSummary>,

    /// The actor who triggered the re-run (if applicable).
    #[serde(default)]
    pub triggering_actor: Option<UserSummary>,
}

#[cfg(test)]
mod tests {}
