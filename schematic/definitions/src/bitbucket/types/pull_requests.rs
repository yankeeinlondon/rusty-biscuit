use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::common::{Content, Link, User};
use super::repos::{BranchInfo, CommitInfo};

/// Pull request from `GET /repositories/{workspace}/{repo_slug}/pullrequests`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct PullRequest {
    /// PR ID (unique within the repository).
    #[serde(default)]
    pub id: Option<u64>,

    /// PR title.
    #[serde(default)]
    pub title: Option<String>,

    /// PR description (markdown).
    #[serde(default)]
    pub description: Option<String>,

    /// PR state: "OPEN", "MERGED", "DECLINED", "SUPERSEDED".
    #[serde(default)]
    pub state: Option<String>,

    /// Creation timestamp (ISO 8601).
    #[serde(default)]
    pub created_on: Option<String>,

    /// Last update timestamp (ISO 8601).
    #[serde(default)]
    pub updated_on: Option<String>,

    /// PR author.
    #[serde(default)]
    pub author: Option<User>,

    /// Source branch information.
    #[serde(default)]
    pub source: Option<BranchRef>,

    /// Destination branch information.
    #[serde(default)]
    pub destination: Option<BranchRef>,

    /// Number of comments on this PR.
    #[serde(default)]
    pub comment_count: Option<u32>,

    /// Number of tasks on this PR.
    #[serde(default)]
    pub task_count: Option<u32>,

    /// Whether to close the source branch after merge.
    #[serde(default)]
    pub close_source_branch: bool,

    /// Users who have been added as reviewers.
    #[serde(default)]
    pub reviewers: Vec<User>,

    /// Users who have participated in this PR.
    #[serde(default)]
    pub participants: Vec<Participant>,

    /// PR merge commit (if merged).
    #[serde(default)]
    pub merge_commit: Option<CommitInfo>,

    /// User who closed the PR.
    #[serde(default)]
    pub closed_by: Option<User>,

    /// Reason for PR state.
    #[serde(default)]
    pub reason: Option<String>,

    /// Summary of changes.
    #[serde(default)]
    pub summary: Option<Content>,

    /// Object type (e.g., "pullrequest").
    #[serde(rename = "type", default)]
    pub pr_type: Option<String>,

    /// HATEOAS links.
    #[serde(default)]
    pub links: Option<HashMap<String, Link>>,
}

impl PullRequest {
    /// Returns `true` if the PR is open.
    pub fn is_open(&self) -> bool {
        self.state.as_deref() == Some("OPEN")
    }

    /// Returns `true` if the PR has been merged.
    pub fn is_merged(&self) -> bool {
        self.state.as_deref() == Some("MERGED")
    }

    /// Returns `true` if the PR was declined.
    pub fn is_declined(&self) -> bool {
        self.state.as_deref() == Some("DECLINED")
    }
}

/// Branch reference for PR source/destination.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct BranchRef {
    /// Branch information.
    #[serde(default)]
    pub branch: Option<BranchInfo>,

    /// Commit information.
    #[serde(default)]
    pub commit: Option<CommitInfo>,

    /// Repository information.
    #[serde(default)]
    pub repository: Option<RepositoryRef>,
}

impl BranchRef {
    /// Returns the branch name if available.
    pub fn branch_name(&self) -> Option<&str> {
        self.branch.as_ref().and_then(|b| b.name.as_deref())
    }

    /// Returns the commit hash if available.
    pub fn commit_hash(&self) -> Option<&str> {
        self.commit.as_ref().and_then(|c| c.hash.as_deref())
    }
}

/// Minimal repository reference (for PRs).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RepositoryRef {
    /// Repository name.
    #[serde(default)]
    pub name: Option<String>,

    /// Full name in "workspace/repo_slug" format.
    #[serde(default)]
    pub full_name: Option<String>,

    /// Repository UUID.
    #[serde(default)]
    pub uuid: Option<String>,

    /// Object type.
    #[serde(rename = "type", default)]
    pub repo_type: Option<String>,

    /// HATEOAS links.
    #[serde(default)]
    pub links: Option<HashMap<String, Link>>,
}

/// A participant in a pull request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Participant {
    /// The participating user.
    #[serde(default)]
    pub user: Option<User>,

    /// Participant role: "REVIEWER" or "PARTICIPANT".
    #[serde(default)]
    pub role: Option<String>,

    /// Whether the participant has approved the PR.
    #[serde(default)]
    pub approved: bool,

    /// Whether the participant has requested changes.
    #[serde(default)]
    pub changes_requested: bool,

    /// State of participation: "approved", "changes_requested", null.
    #[serde(default)]
    pub state: Option<String>,

    /// Timestamp of participation.
    #[serde(default)]
    pub participated_on: Option<String>,

    /// Object type.
    #[serde(rename = "type", default)]
    pub participant_type: Option<String>,
}

impl Participant {
    /// Returns `true` if this participant is a reviewer.
    pub fn is_reviewer(&self) -> bool {
        self.role.as_deref() == Some("REVIEWER")
    }
}

/// A comment on a pull request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PullRequestComment {
    /// Comment ID.
    #[serde(default)]
    pub id: Option<u64>,

    /// Comment content.
    #[serde(default)]
    pub content: Option<Content>,

    /// Comment author.
    #[serde(default)]
    pub user: Option<User>,

    /// Creation timestamp (ISO 8601).
    #[serde(default)]
    pub created_on: Option<String>,

    /// Last update timestamp (ISO 8601).
    #[serde(default)]
    pub updated_on: Option<String>,

    /// Whether this is a deleted comment.
    #[serde(default)]
    pub deleted: bool,

    /// Parent comment ID (for threaded comments).
    #[serde(default)]
    pub parent: Option<CommentParent>,

    /// Inline context (for code comments).
    #[serde(default)]
    pub inline: Option<InlineContext>,

    /// Object type.
    #[serde(rename = "type", default)]
    pub comment_type: Option<String>,

    /// HATEOAS links.
    #[serde(default)]
    pub links: Option<HashMap<String, Link>>,
}

/// Parent comment reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct CommentParent {
    /// Parent comment ID.
    #[serde(default)]
    pub id: Option<u64>,
}

/// Inline context for code comments.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct InlineContext {
    /// File path.
    #[serde(default)]
    pub path: Option<String>,

    /// Line number (old version).
    #[serde(default)]
    pub from: Option<u64>,

    /// Line number (new version).
    #[serde(default)]
    pub to: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pull_request_deserialization() {
        let json = r#"{
            "id": 42,
            "title": "Add new feature",
            "description": "This PR adds a new feature",
            "state": "OPEN",
            "created_on": "2026-01-15T10:00:00Z",
            "updated_on": "2026-01-15T12:00:00Z",
            "author": {"display_name": "Test User"},
            "source": {"branch": {"name": "feature"}},
            "destination": {"branch": {"name": "main"}},
            "comment_count": 5,
            "task_count": 2,
            "close_source_branch": true,
            "type": "pullrequest"
        }"#;

        let pr: PullRequest = serde_json::from_str(json).unwrap();
        assert_eq!(pr.id, Some(42));
        assert!(pr.is_open());
        assert!(!pr.is_merged());
        assert_eq!(pr.source.as_ref().unwrap().branch_name(), Some("feature"));
        assert_eq!(pr.destination.as_ref().unwrap().branch_name(), Some("main"));
        assert_eq!(pr.comment_count, Some(5));
    }

    #[test]
    fn pull_request_merged_state() {
        let json = r#"{"id": 1, "state": "MERGED"}"#;

        let pr: PullRequest = serde_json::from_str(json).unwrap();
        assert!(pr.is_merged());
        assert!(!pr.is_open());
    }

    #[test]
    fn pull_request_declined_state() {
        let json = r#"{"id": 1, "state": "DECLINED"}"#;

        let pr: PullRequest = serde_json::from_str(json).unwrap();
        assert!(pr.is_declined());
    }

    #[test]
    fn participant_deserialization() {
        let json = r#"{
            "user": {"display_name": "Reviewer"},
            "role": "REVIEWER",
            "approved": true,
            "changes_requested": false,
            "state": "approved",
            "type": "participant"
        }"#;

        let participant: Participant = serde_json::from_str(json).unwrap();
        assert!(participant.is_reviewer());
        assert!(participant.approved);
        assert!(!participant.changes_requested);
    }

    #[test]
    fn pull_request_comment_deserialization() {
        let json = r#"{
            "id": 123,
            "content": {"raw": "Looks good!", "markup": "markdown"},
            "user": {"display_name": "Reviewer"},
            "created_on": "2026-01-15T14:00:00Z",
            "deleted": false,
            "type": "pullrequest_comment"
        }"#;

        let comment: PullRequestComment = serde_json::from_str(json).unwrap();
        assert_eq!(comment.id, Some(123));
        assert!(!comment.deleted);
        assert_eq!(
            comment.content.as_ref().unwrap().raw,
            Some("Looks good!".to_string())
        );
    }

    #[test]
    fn branch_ref_helpers() {
        let json = r#"{
            "branch": {"name": "feature-branch"},
            "commit": {"hash": "abc123"}
        }"#;

        let branch_ref: BranchRef = serde_json::from_str(json).unwrap();
        assert_eq!(branch_ref.branch_name(), Some("feature-branch"));
        assert_eq!(branch_ref.commit_hash(), Some("abc123"));
    }

    #[test]
    fn inline_context_deserialization() {
        let json = r#"{
            "path": "src/lib.rs",
            "from": 10,
            "to": 15
        }"#;

        let inline: InlineContext = serde_json::from_str(json).unwrap();
        assert_eq!(inline.path, Some("src/lib.rs".to_string()));
        assert_eq!(inline.from, Some(10));
        assert_eq!(inline.to, Some(15));
    }
}
