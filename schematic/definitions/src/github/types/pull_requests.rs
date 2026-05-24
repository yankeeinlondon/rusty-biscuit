use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::common::UserSummary;
use super::issues::IssueLabel;
use super::repos::RepositoryInfo;

/// Branch reference for PR head/base.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct BranchRef {
    /// Branch name.
    #[serde(rename = "ref")]
    pub ref_name: String,

    /// Commit SHA.
    pub sha: String,

    /// Repository the branch belongs to.
    #[serde(default)]
    pub repo: Option<Box<RepositoryInfo>>,
}

/// Pull request summary from list endpoints.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct PullRequestSummary {
    /// PR number.
    pub number: u64,

    /// PR state: "open", "closed".
    pub state: String,

    /// PR title.
    pub title: String,

    /// PR body/description.
    #[serde(default)]
    pub body: Option<String>,

    /// Whether this is a draft PR.
    #[serde(default)]
    pub draft: Option<bool>,

    /// PR author.
    pub user: UserSummary,

    /// Base branch info.
    pub base: BranchRef,

    /// Head branch info.
    pub head: BranchRef,

    /// Creation timestamp.
    pub created_at: String,

    /// Last update timestamp.
    pub updated_at: String,

    /// Merge timestamp (if merged).
    #[serde(default)]
    pub merged_at: Option<String>,

    /// Close timestamp (if closed).
    #[serde(default)]
    pub closed_at: Option<String>,

    /// HTML URL to the PR.
    pub html_url: String,

    /// API URL for the PR.
    pub url: String,

    /// API URL for issue resource.
    #[serde(default)]
    pub issue_url: Option<String>,

    /// API URL for PR comments.
    #[serde(default)]
    pub comments_url: Option<String>,

    /// API URL for review comments.
    #[serde(default)]
    pub review_comments_url: Option<String>,

    /// API URL for commits.
    #[serde(default)]
    pub commits_url: Option<String>,

    /// API URL for statuses.
    #[serde(default)]
    pub statuses_url: Option<String>,

    /// Labels applied to the PR.
    ///
    /// GitHub returns labels on PRs through the same `labels` array shape
    /// used by issues. The list endpoint includes this field by default,
    /// but `#[serde(default)]` keeps older fixtures (or alternative APIs
    /// that omit it) deserializable.
    #[serde(default)]
    pub labels: Vec<IssueLabel>,
}

/// A file changed in a pull request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PullRequestFile {
    /// Filename (full path).
    pub filename: String,

    /// Change status: "added", "removed", "modified", "renamed", "copied", "changed", "unchanged".
    pub status: String,

    /// Number of additions.
    #[serde(default)]
    pub additions: i64,

    /// Number of deletions.
    #[serde(default)]
    pub deletions: i64,

    /// Total number of changes.
    #[serde(default)]
    pub changes: i64,

    /// SHA of the file.
    #[serde(default)]
    pub sha: Option<String>,

    /// API URL for the blob.
    #[serde(default)]
    pub blob_url: Option<String>,

    /// Raw content URL.
    #[serde(default)]
    pub raw_url: Option<String>,

    /// URL to view the file at this commit.
    #[serde(default)]
    pub contents_url: Option<String>,

    /// Patch/diff content.
    #[serde(default)]
    pub patch: Option<String>,

    /// Previous filename (if renamed).
    #[serde(default)]
    pub previous_filename: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pull_request_summary_deserialization() {
        let json = r#"{
            "number": 1347,
            "state": "open",
            "title": "Amazing feature",
            "user": { "login": "octocat" },
            "base": { "ref": "main", "sha": "abc123" },
            "head": { "ref": "feature", "sha": "def456" },
            "created_at": "2011-01-26T19:01:12Z",
            "updated_at": "2011-01-26T19:01:12Z",
            "html_url": "https://github.com/octocat/Hello-World/pull/1347",
            "url": "https://api.github.com/repos/octocat/Hello-World/pulls/1347"
        }"#;

        let pr: PullRequestSummary = serde_json::from_str(json).unwrap();
        assert_eq!(pr.number, 1347);
        assert_eq!(pr.state, "open");
        assert_eq!(pr.base.ref_name, "main");
        assert!(pr.labels.is_empty());
    }

    #[test]
    fn pull_request_summary_with_labels_deserialization() {
        let json = r#"{
            "number": 1347,
            "state": "open",
            "title": "Amazing feature",
            "user": { "login": "octocat" },
            "base": { "ref": "main", "sha": "abc123" },
            "head": { "ref": "feature", "sha": "def456" },
            "created_at": "2011-01-26T19:01:12Z",
            "updated_at": "2011-01-26T19:01:12Z",
            "html_url": "https://github.com/octocat/Hello-World/pull/1347",
            "url": "https://api.github.com/repos/octocat/Hello-World/pulls/1347",
            "labels": [
                {"name": "bug", "color": "d73a4a"},
                {"name": "enhancement"}
            ]
        }"#;

        let pr: PullRequestSummary = serde_json::from_str(json).unwrap();
        assert_eq!(pr.labels.len(), 2);
        assert_eq!(pr.labels[0].name, "bug");
        assert_eq!(pr.labels[0].color.as_deref(), Some("d73a4a"));
        assert_eq!(pr.labels[1].name, "enhancement");
        assert!(pr.labels[1].color.is_none());
    }

    #[test]
    fn pull_request_file_deserialization() {
        let json = r#"{
            "filename": "src/main.rs",
            "status": "modified",
            "additions": 10,
            "deletions": 5,
            "changes": 15,
            "sha": "abc123"
        }"#;

        let file: PullRequestFile = serde_json::from_str(json).unwrap();
        assert_eq!(file.filename, "src/main.rs");
        assert_eq!(file.status, "modified");
        assert_eq!(file.additions, 10);
    }
}
