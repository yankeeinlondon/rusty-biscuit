//! Gitea REST API types.
//!
//! This module contains request and response types for the Gitea REST API,
//! including repository metadata, pull requests, issues, tags, and releases.
//!
//! ## Design Notes
//!
//! - All types use `Option<_>` liberally for forward-compatibility across Gitea versions
//! - Fields that are frequently omitted or permission-dependent are optional
//! - `serde(default)` is used where empty/null values should deserialize cleanly
//! - Modeled after GitHub types but adapted for Gitea's API responses

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

// =============================================================================
// Common Types
// =============================================================================

/// A Gitea user summary (common across many responses).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct UserSummary {
    /// The user's login handle.
    #[serde(default)]
    pub login: Option<String>,

    /// Alternative field name for login (Gitea sometimes uses `username`).
    #[serde(default)]
    pub username: Option<String>,

    /// The user's unique ID.
    #[serde(default)]
    pub id: Option<i64>,

    /// The user's full name.
    #[serde(default)]
    pub full_name: Option<String>,

    /// URL to the user's avatar image.
    #[serde(default)]
    pub avatar_url: Option<String>,
}

impl UserSummary {
    /// Returns the user's login name, preferring `login` over `username`.
    pub fn login_name(&self) -> Option<&str> {
        self.login.as_deref().or(self.username.as_deref())
    }
}

// =============================================================================
// Repository Types
// =============================================================================

/// Repository information from `GET /repos/{owner}/{repo}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RepositoryInfo {
    /// Repository ID.
    #[serde(default)]
    pub id: Option<i64>,

    /// Repository name.
    #[serde(default)]
    pub name: Option<String>,

    /// Full name in "owner/repo" format.
    #[serde(default)]
    pub full_name: Option<String>,

    /// Repository description.
    #[serde(default)]
    pub description: Option<String>,

    /// Whether the repository is private.
    #[serde(default)]
    pub private: bool,

    /// Repository owner.
    #[serde(default)]
    pub owner: Option<UserSummary>,

    /// HTML URL to the repository.
    #[serde(default)]
    pub html_url: Option<String>,

    /// Clone URL (HTTPS).
    #[serde(default)]
    pub clone_url: Option<String>,

    /// SSH URL for cloning.
    #[serde(default)]
    pub ssh_url: Option<String>,

    /// Default branch name.
    #[serde(default)]
    pub default_branch: Option<String>,

    /// Primary programming language.
    #[serde(default)]
    pub language: Option<String>,

    /// Number of stars.
    #[serde(default)]
    pub stars_count: Option<i64>,

    /// Number of watchers.
    #[serde(default)]
    pub watchers_count: Option<i64>,

    /// Number of forks.
    #[serde(default)]
    pub forks_count: Option<i64>,

    /// Number of open issues.
    #[serde(default)]
    pub open_issues_count: Option<i64>,

    /// Creation timestamp (ISO 8601).
    #[serde(default)]
    pub created_at: Option<String>,

    /// Last update timestamp (ISO 8601).
    #[serde(default)]
    pub updated_at: Option<String>,

    /// Whether the repository is archived.
    #[serde(default)]
    pub archived: bool,

    /// Whether the repository is a mirror.
    #[serde(default)]
    pub mirror: bool,

    /// Whether the repository is empty.
    #[serde(default)]
    pub empty: bool,
}

// =============================================================================
// Git Tree Types
// =============================================================================

/// Response from `GET /repos/{owner}/{repo}/git/trees/{sha}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct GitTreeResponse {
    /// SHA of the tree.
    #[serde(default)]
    pub sha: Option<String>,

    /// API URL for this tree.
    #[serde(default)]
    pub url: Option<String>,

    /// Tree entries (files and directories).
    #[serde(default)]
    pub tree: Vec<GitTreeEntry>,

    /// Whether the response was truncated.
    #[serde(default)]
    pub truncated: bool,

    /// Total count of entries (when paginated).
    #[serde(default)]
    pub total_count: Option<i64>,

    /// Current page (when paginated).
    #[serde(default)]
    pub page: Option<i64>,
}

/// A single entry in a Git tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct GitTreeEntry {
    /// File or directory path.
    #[serde(default)]
    pub path: Option<String>,

    /// Entry type: "blob" (file), "tree" (directory), or "commit" (submodule).
    #[serde(rename = "type", default)]
    pub entry_type: Option<String>,

    /// Git file mode (e.g., "100644" for regular file).
    #[serde(default)]
    pub mode: Option<String>,

    /// SHA of the entry.
    #[serde(default)]
    pub sha: Option<String>,

    /// Size in bytes (only for blobs).
    #[serde(default)]
    pub size: Option<i64>,

    /// API URL for the entry.
    #[serde(default)]
    pub url: Option<String>,
}

// =============================================================================
// Pull Request Types
// =============================================================================

/// Branch reference for PR head/base.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PullRef {
    /// Branch name.
    #[serde(rename = "ref", default)]
    pub ref_name: Option<String>,

    /// Commit SHA.
    #[serde(default)]
    pub sha: Option<String>,
}

/// Pull request summary from list endpoints.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct PullRequestSummary {
    /// PR ID.
    #[serde(default)]
    pub id: Option<i64>,

    /// PR number (index).
    #[serde(default)]
    pub number: Option<i64>,

    /// PR state: "open", "closed".
    #[serde(default)]
    pub state: Option<String>,

    /// PR title.
    #[serde(default)]
    pub title: Option<String>,

    /// PR body/description.
    #[serde(default)]
    pub body: Option<String>,

    /// Whether this is a draft PR.
    #[serde(default)]
    pub draft: Option<bool>,

    /// Whether the PR is mergeable.
    #[serde(default)]
    pub mergeable: Option<bool>,

    /// Whether the PR has been merged.
    #[serde(default)]
    pub has_merged: Option<bool>,

    /// Creation timestamp.
    #[serde(default)]
    pub created_at: Option<String>,

    /// Last update timestamp.
    #[serde(default)]
    pub updated_at: Option<String>,

    /// Merge timestamp (if merged).
    #[serde(default)]
    pub merged_at: Option<String>,

    /// HTML URL to the PR.
    #[serde(default)]
    pub html_url: Option<String>,

    /// API URL for the PR.
    #[serde(default)]
    pub url: Option<String>,

    /// PR author.
    #[serde(default)]
    pub user: Option<UserSummary>,

    /// Base branch info.
    #[serde(default)]
    pub base: Option<PullRef>,

    /// Head branch info.
    #[serde(default)]
    pub head: Option<PullRef>,
}

/// A file changed in a pull request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct PullRequestFile {
    /// Filename (full path).
    #[serde(default)]
    pub filename: Option<String>,

    /// Change status: "added", "removed", "modified", "renamed".
    #[serde(default)]
    pub status: Option<String>,

    /// Number of additions.
    #[serde(default)]
    pub additions: Option<i64>,

    /// Number of deletions.
    #[serde(default)]
    pub deletions: Option<i64>,

    /// Total number of changes.
    #[serde(default)]
    pub changes: Option<i64>,
}

// =============================================================================
// Issue Types
// =============================================================================

/// Issue summary from list/get endpoints.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct IssueSummary {
    /// Issue ID.
    #[serde(default)]
    pub id: Option<i64>,

    /// Issue index (number).
    #[serde(default)]
    pub index: Option<i64>,

    /// Alternative field name for index (some endpoints use `number`).
    #[serde(default)]
    pub number: Option<i64>,

    /// Issue state: "open", "closed".
    #[serde(default)]
    pub state: Option<String>,

    /// Issue title.
    #[serde(default)]
    pub title: Option<String>,

    /// Issue body/description.
    #[serde(default)]
    pub body: Option<String>,

    /// Number of comments.
    #[serde(default)]
    pub comments: Option<i64>,

    /// Creation timestamp.
    #[serde(default)]
    pub created_at: Option<String>,

    /// Last update timestamp.
    #[serde(default)]
    pub updated_at: Option<String>,

    /// Close timestamp (if closed).
    #[serde(default)]
    pub closed_at: Option<String>,

    /// Issue author.
    #[serde(default)]
    pub user: Option<UserSummary>,

    /// Pull request info (present if this issue is a PR).
    #[serde(default)]
    pub pull_request: Option<serde_json::Value>,

    /// API URL for the issue.
    #[serde(default)]
    pub url: Option<String>,

    /// HTML URL to the issue.
    #[serde(default)]
    pub html_url: Option<String>,
}

impl IssueSummary {
    /// Returns `true` if this issue is actually a pull request.
    pub fn is_pull_request(&self) -> bool {
        self.pull_request.is_some()
    }

    /// Returns the issue number, preferring `index` over `number`.
    pub fn issue_number(&self) -> Option<i64> {
        self.index.or(self.number)
    }
}

/// A comment on an issue.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct IssueComment {
    /// Comment ID.
    #[serde(default)]
    pub id: Option<i64>,

    /// Comment body.
    #[serde(default)]
    pub body: Option<String>,

    /// Comment author.
    #[serde(default)]
    pub user: Option<UserSummary>,

    /// Creation timestamp.
    #[serde(default)]
    pub created_at: Option<String>,

    /// Last update timestamp.
    #[serde(default)]
    pub updated_at: Option<String>,
}

/// A timeline event on an issue.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct TimelineEvent {
    /// Event ID.
    #[serde(default)]
    pub id: Option<i64>,

    /// Event type.
    #[serde(rename = "type", default)]
    pub event_type: Option<String>,

    /// Event timestamp.
    #[serde(default)]
    pub created: Option<String>,

    /// Additional event data varies by event type.
    #[serde(flatten)]
    pub extra: serde_json::Value,
}

// =============================================================================
// Tag and Release Types
// =============================================================================

/// Commit info embedded in a tag.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct TagCommit {
    /// Commit SHA.
    #[serde(default)]
    pub sha: Option<String>,
}

/// A repository tag from `GET /repos/{owner}/{repo}/tags`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct RepoTag {
    /// Tag name.
    #[serde(default)]
    pub name: Option<String>,

    /// Tag ID (Gitea-specific).
    #[serde(default)]
    pub id: Option<String>,

    /// Tag message (for annotated tags).
    #[serde(default)]
    pub message: Option<String>,

    /// Commit the tag points to.
    #[serde(default)]
    pub commit: Option<TagCommit>,

    /// URL to download tarball.
    #[serde(default)]
    pub tarball_url: Option<String>,

    /// URL to download zipball.
    #[serde(default)]
    pub zipball_url: Option<String>,
}

/// A Gitea release from `GET /repos/{owner}/{repo}/releases`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Release {
    /// Release ID.
    #[serde(default)]
    pub id: Option<i64>,

    /// Tag name associated with the release.
    #[serde(default)]
    pub tag_name: Option<String>,

    /// Release name/title.
    #[serde(default)]
    pub name: Option<String>,

    /// Release body (markdown).
    #[serde(default)]
    pub body: Option<String>,

    /// Whether this is a draft.
    #[serde(default)]
    pub draft: Option<bool>,

    /// Whether this is a prerelease.
    #[serde(default)]
    pub prerelease: Option<bool>,

    /// Creation timestamp.
    #[serde(default)]
    pub created_at: Option<String>,

    /// Publish timestamp.
    #[serde(default)]
    pub published_at: Option<String>,

    /// HTML URL to the release.
    #[serde(default)]
    pub html_url: Option<String>,

    /// Author of the release.
    #[serde(default)]
    pub author: Option<UserSummary>,
}

// =============================================================================
// Git Reference Types
// =============================================================================

/// Object pointed to by a Git reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct GitRefObject {
    /// Object type: "commit" (lightweight tag) or "tag" (annotated tag).
    #[serde(rename = "type", default)]
    pub object_type: Option<String>,

    /// Object SHA.
    #[serde(default)]
    pub sha: Option<String>,

    /// API URL for the object.
    #[serde(default)]
    pub url: Option<String>,
}

impl GitRefObject {
    /// Returns `true` if this is an annotated tag.
    pub fn is_annotated_tag(&self) -> bool {
        self.object_type.as_deref() == Some("tag")
    }

    /// Returns `true` if this is a lightweight tag (points directly to commit).
    pub fn is_lightweight_tag(&self) -> bool {
        self.object_type.as_deref() == Some("commit")
    }
}

/// A Git reference from `GET /repos/{owner}/{repo}/git/refs/{ref}`.
///
/// Note: Gitea returns an **array** of refs for this endpoint, unlike GitHub.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct GitRef {
    /// Full ref name (e.g., "refs/tags/v1.0.0").
    #[serde(rename = "ref", default)]
    pub ref_name: Option<String>,

    /// API URL.
    #[serde(default)]
    pub url: Option<String>,

    /// Object the ref points to.
    #[serde(default)]
    pub object: Option<GitRefObject>,
}

/// An annotated tag object from `GET /repos/{owner}/{repo}/git/tags/{sha}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct AnnotatedTagObject {
    /// Tag object SHA.
    #[serde(default)]
    pub sha: Option<String>,

    /// Tag name.
    #[serde(default)]
    pub tag: Option<String>,

    /// Tag message.
    #[serde(default)]
    pub message: Option<String>,

    /// API URL.
    #[serde(default)]
    pub url: Option<String>,
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_summary_deserialization_with_login() {
        let json = r#"{
            "login": "gitea_user",
            "id": 1,
            "full_name": "Gitea User",
            "avatar_url": "https://gitea.example.com/avatars/1"
        }"#;

        let user: UserSummary = serde_json::from_str(json).unwrap();
        assert_eq!(user.login, Some("gitea_user".to_string()));
        assert_eq!(user.login_name(), Some("gitea_user"));
    }

    #[test]
    fn user_summary_deserialization_with_username() {
        let json = r#"{
            "username": "gitea_user",
            "id": 1
        }"#;

        let user: UserSummary = serde_json::from_str(json).unwrap();
        assert_eq!(user.username, Some("gitea_user".to_string()));
        assert_eq!(user.login_name(), Some("gitea_user"));
    }

    #[test]
    fn repository_info_deserialization() {
        let json = r#"{
            "id": 1,
            "name": "test-repo",
            "full_name": "owner/test-repo",
            "private": false,
            "default_branch": "main",
            "archived": false
        }"#;

        let repo: RepositoryInfo = serde_json::from_str(json).unwrap();
        assert_eq!(repo.name, Some("test-repo".to_string()));
        assert_eq!(repo.default_branch, Some("main".to_string()));
        assert!(!repo.archived);
    }

    #[test]
    fn git_tree_response_deserialization() {
        let json = r#"{
            "sha": "abc123",
            "url": "https://gitea.example.com/api/v1/repos/owner/repo/git/trees/abc123",
            "tree": [
                {"path": "README.md", "type": "blob", "sha": "def456", "size": 1234}
            ],
            "truncated": false
        }"#;

        let tree: GitTreeResponse = serde_json::from_str(json).unwrap();
        assert_eq!(tree.sha, Some("abc123".to_string()));
        assert_eq!(tree.tree.len(), 1);
        assert!(!tree.truncated);
    }

    #[test]
    fn git_tree_entry_deserialization() {
        let json = r#"{
            "path": "src/main.rs",
            "type": "blob",
            "mode": "100644",
            "sha": "abc123",
            "size": 500
        }"#;

        let entry: GitTreeEntry = serde_json::from_str(json).unwrap();
        assert_eq!(entry.path, Some("src/main.rs".to_string()));
        assert_eq!(entry.entry_type, Some("blob".to_string()));
        assert_eq!(entry.size, Some(500));
    }

    #[test]
    fn pull_request_summary_deserialization() {
        let json = r#"{
            "id": 1,
            "number": 42,
            "state": "open",
            "title": "Add new feature",
            "draft": false,
            "mergeable": true,
            "created_at": "2026-01-15T10:00:00Z",
            "updated_at": "2026-01-15T12:00:00Z",
            "html_url": "https://gitea.example.com/owner/repo/pulls/42",
            "user": {"login": "author"},
            "base": {"ref": "main", "sha": "abc"},
            "head": {"ref": "feature", "sha": "def"}
        }"#;

        let pr: PullRequestSummary = serde_json::from_str(json).unwrap();
        assert_eq!(pr.number, Some(42));
        assert_eq!(pr.state, Some("open".to_string()));
        assert_eq!(pr.draft, Some(false));
    }

    #[test]
    fn pull_request_file_deserialization() {
        let json = r#"{
            "filename": "src/lib.rs",
            "status": "modified",
            "additions": 10,
            "deletions": 5,
            "changes": 15
        }"#;

        let file: PullRequestFile = serde_json::from_str(json).unwrap();
        assert_eq!(file.filename, Some("src/lib.rs".to_string()));
        assert_eq!(file.status, Some("modified".to_string()));
        assert_eq!(file.additions, Some(10));
    }

    #[test]
    fn issue_summary_deserialization() {
        let json = r#"{
            "id": 1,
            "index": 123,
            "state": "open",
            "title": "Bug report",
            "comments": 5,
            "created_at": "2026-01-15T10:00:00Z",
            "updated_at": "2026-01-15T12:00:00Z",
            "url": "https://gitea.example.com/api/v1/repos/owner/repo/issues/123",
            "html_url": "https://gitea.example.com/owner/repo/issues/123"
        }"#;

        let issue: IssueSummary = serde_json::from_str(json).unwrap();
        assert_eq!(issue.index, Some(123));
        assert_eq!(issue.issue_number(), Some(123));
        assert!(!issue.is_pull_request());
    }

    #[test]
    fn issue_with_pull_request_field() {
        let json = r#"{
            "id": 1,
            "index": 42,
            "state": "open",
            "title": "A PR",
            "pull_request": {"url": "https://gitea.example.com/api/v1/repos/owner/repo/pulls/42"}
        }"#;

        let issue: IssueSummary = serde_json::from_str(json).unwrap();
        assert!(issue.is_pull_request());
    }

    #[test]
    fn issue_comment_deserialization() {
        let json = r#"{
            "id": 1,
            "body": "This looks good!",
            "user": {"login": "reviewer"},
            "created_at": "2026-01-15T14:00:00Z",
            "updated_at": "2026-01-15T14:00:00Z"
        }"#;

        let comment: IssueComment = serde_json::from_str(json).unwrap();
        assert_eq!(comment.id, Some(1));
        assert_eq!(comment.body, Some("This looks good!".to_string()));
    }

    #[test]
    fn timeline_event_deserialization() {
        let json = r#"{
            "id": 1,
            "type": "comment",
            "created": "2026-01-15T14:00:00Z",
            "body": "A comment"
        }"#;

        let event: TimelineEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.event_type, Some("comment".to_string()));
        assert!(event.extra.get("body").is_some());
    }

    #[test]
    fn repo_tag_deserialization() {
        let json = r#"{
            "name": "v1.0.0",
            "id": "abc123",
            "message": "Release v1.0.0",
            "commit": {"sha": "def456"},
            "tarball_url": "https://gitea.example.com/owner/repo/archive/v1.0.0.tar.gz",
            "zipball_url": "https://gitea.example.com/owner/repo/archive/v1.0.0.zip"
        }"#;

        let tag: RepoTag = serde_json::from_str(json).unwrap();
        assert_eq!(tag.name, Some("v1.0.0".to_string()));
        assert_eq!(tag.message, Some("Release v1.0.0".to_string()));
    }

    #[test]
    fn release_deserialization() {
        let json = r#"{
            "id": 1,
            "tag_name": "v1.0.0",
            "name": "Version 1.0.0",
            "draft": false,
            "prerelease": false,
            "created_at": "2026-01-15T10:00:00Z",
            "html_url": "https://gitea.example.com/owner/repo/releases/tag/v1.0.0"
        }"#;

        let release: Release = serde_json::from_str(json).unwrap();
        assert_eq!(release.tag_name, Some("v1.0.0".to_string()));
        assert_eq!(release.draft, Some(false));
    }

    #[test]
    fn git_ref_deserialization() {
        let json = r#"{
            "ref": "refs/tags/v1.0.0",
            "url": "https://gitea.example.com/api/v1/repos/owner/repo/git/refs/tags/v1.0.0",
            "object": {
                "type": "commit",
                "sha": "abc123",
                "url": "https://gitea.example.com/api/v1/repos/owner/repo/git/commits/abc123"
            }
        }"#;

        let git_ref: GitRef = serde_json::from_str(json).unwrap();
        assert_eq!(git_ref.ref_name, Some("refs/tags/v1.0.0".to_string()));
        assert!(git_ref.object.as_ref().unwrap().is_lightweight_tag());
    }

    #[test]
    fn git_ref_annotated_tag() {
        let json = r#"{
            "ref": "refs/tags/v1.0.0",
            "object": {
                "type": "tag",
                "sha": "abc123"
            }
        }"#;

        let git_ref: GitRef = serde_json::from_str(json).unwrap();
        assert!(git_ref.object.as_ref().unwrap().is_annotated_tag());
    }

    #[test]
    fn annotated_tag_object_deserialization() {
        let json = r#"{
            "sha": "abc123",
            "tag": "v1.0.0",
            "message": "Release v1.0.0\n\nInitial release.",
            "url": "https://gitea.example.com/api/v1/repos/owner/repo/git/tags/abc123"
        }"#;

        let tag: AnnotatedTagObject = serde_json::from_str(json).unwrap();
        assert_eq!(tag.tag, Some("v1.0.0".to_string()));
        assert!(tag.message.as_ref().unwrap().contains("Initial release"));
    }

    #[test]
    fn empty_json_deserializes_cleanly() {
        // All fields are optional, so empty objects should work
        let _user: UserSummary = serde_json::from_str("{}").unwrap();
        let _repo: RepositoryInfo = serde_json::from_str("{}").unwrap();
        let _tree: GitTreeResponse = serde_json::from_str("{}").unwrap();
        let _pr: PullRequestSummary = serde_json::from_str("{}").unwrap();
        let _issue: IssueSummary = serde_json::from_str("{}").unwrap();
        let _tag: RepoTag = serde_json::from_str("{}").unwrap();
        let _release: Release = serde_json::from_str("{}").unwrap();
        let _git_ref: GitRef = serde_json::from_str("{}").unwrap();
    }
}
