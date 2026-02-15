//! GitHub REST API types.
//!
//! This module contains request and response types for the GitHub REST API,
//! including repository metadata, pull requests, issues, tags, and releases.
//!
//! ## Design Notes
//!
//! - All types use `Option<_>` liberally for forward-compatibility with API evolution
//! - Fields that are frequently omitted or permission-dependent are optional
//! - `serde(default)` is used where empty/null values should deserialize cleanly

use serde::{Deserialize, Serialize};

// =============================================================================
// Common Types
// =============================================================================

/// A GitHub user summary (common across many responses).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserSummary {
    /// The user's login handle.
    pub login: String,

    /// The user's unique ID.
    #[serde(default)]
    pub id: Option<u64>,

    /// URL to the user's avatar image.
    #[serde(default)]
    pub avatar_url: Option<String>,

    /// API URL for the user resource.
    #[serde(default)]
    pub url: Option<String>,

    /// HTML URL to the user's profile.
    #[serde(default)]
    pub html_url: Option<String>,

    /// User type (e.g., "User", "Organization", "Bot").
    #[serde(rename = "type", default)]
    pub user_type: Option<String>,
}

// =============================================================================
// Repository Types
// =============================================================================

/// Repository information from `GET /repos/{owner}/{repo}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepositoryInfo {
    /// Repository ID.
    pub id: u64,

    /// Repository name.
    pub name: String,

    /// Full name in "owner/repo" format.
    pub full_name: String,

    /// Repository description.
    #[serde(default)]
    pub description: Option<String>,

    /// Whether the repository is private.
    #[serde(default)]
    pub private: bool,

    /// Repository owner.
    pub owner: UserSummary,

    /// HTML URL to the repository.
    pub html_url: String,

    /// API URL for the repository.
    pub url: String,

    /// Default branch name.
    pub default_branch: String,

    /// Primary programming language.
    #[serde(default)]
    pub language: Option<String>,

    /// Number of stargazers.
    #[serde(default)]
    pub stargazers_count: Option<u64>,

    /// Number of watchers.
    #[serde(default)]
    pub watchers_count: Option<u64>,

    /// Number of forks.
    #[serde(default)]
    pub forks_count: Option<u64>,

    /// Number of open issues.
    #[serde(default)]
    pub open_issues_count: Option<u64>,

    /// Creation timestamp (ISO 8601).
    #[serde(default)]
    pub created_at: Option<String>,

    /// Last update timestamp (ISO 8601).
    #[serde(default)]
    pub updated_at: Option<String>,

    /// Last push timestamp (ISO 8601).
    #[serde(default)]
    pub pushed_at: Option<String>,

    /// Whether issues are enabled.
    #[serde(default)]
    pub has_issues: Option<bool>,

    /// Whether the wiki is enabled.
    #[serde(default)]
    pub has_wiki: Option<bool>,

    /// Whether the repository is archived.
    #[serde(default)]
    pub archived: bool,

    /// Whether the repository is disabled.
    #[serde(default)]
    pub disabled: bool,

    /// License information.
    #[serde(default)]
    pub license: Option<LicenseInfo>,

    /// Topics/tags on the repository.
    #[serde(default)]
    pub topics: Vec<String>,
}

/// License information for a repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LicenseInfo {
    /// SPDX license identifier.
    pub key: String,

    /// License name.
    pub name: String,

    /// SPDX ID.
    #[serde(default)]
    pub spdx_id: Option<String>,

    /// URL to license details.
    #[serde(default)]
    pub url: Option<String>,
}

// =============================================================================
// Git Tree Types
// =============================================================================

/// Response from `GET /repos/{owner}/{repo}/git/trees/{tree_sha}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitTreeResponse {
    /// SHA of the tree.
    pub sha: String,

    /// API URL for this tree.
    pub url: String,

    /// Tree entries (files and directories).
    pub tree: Vec<GitTreeEntry>,

    /// Whether the response was truncated (> 100k entries or 7MB).
    #[serde(default)]
    pub truncated: bool,
}

/// A single entry in a Git tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitTreeEntry {
    /// File or directory path.
    pub path: String,

    /// Entry type: "blob" (file), "tree" (directory), or "commit" (submodule).
    #[serde(rename = "type")]
    pub entry_type: String,

    /// Git file mode (e.g., "100644" for regular file).
    #[serde(default)]
    pub mode: Option<String>,

    /// SHA of the entry.
    #[serde(default)]
    pub sha: Option<String>,

    /// Size in bytes (only for blobs).
    #[serde(default)]
    pub size: Option<u64>,

    /// API URL for the entry.
    #[serde(default)]
    pub url: Option<String>,
}

// =============================================================================
// Pull Request Types
// =============================================================================

/// Branch reference for PR head/base.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
}

/// A file changed in a pull request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

// =============================================================================
// Issue Types
// =============================================================================

/// Issue summary from list/get endpoints.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IssueSummary {
    /// Issue ID.
    pub id: u64,

    /// Issue number.
    pub number: u64,

    /// Issue state: "open", "closed".
    pub state: String,

    /// Issue title.
    pub title: String,

    /// Issue body/description.
    #[serde(default)]
    pub body: Option<String>,

    /// Issue author.
    pub user: UserSummary,

    /// Number of comments.
    #[serde(default)]
    pub comments: u64,

    /// Creation timestamp.
    pub created_at: String,

    /// Last update timestamp.
    pub updated_at: String,

    /// Close timestamp (if closed).
    #[serde(default)]
    pub closed_at: Option<String>,

    /// API URL for the issue.
    pub url: String,

    /// HTML URL to the issue.
    pub html_url: String,

    /// API URL for comments.
    #[serde(default)]
    pub comments_url: Option<String>,

    /// API URL for events.
    #[serde(default)]
    pub events_url: Option<String>,

    /// Pull request info (present if this issue is a PR).
    ///
    /// Use `.is_some()` to distinguish issues from PRs.
    #[serde(default)]
    pub pull_request: Option<serde_json::Value>,

    /// Labels on the issue.
    #[serde(default)]
    pub labels: Vec<IssueLabel>,

    /// Assigned users.
    #[serde(default)]
    pub assignees: Vec<UserSummary>,

    /// Milestone (if any).
    #[serde(default)]
    pub milestone: Option<Milestone>,
}

impl IssueSummary {
    /// Returns `true` if this issue is actually a pull request.
    pub fn is_pull_request(&self) -> bool {
        self.pull_request.is_some()
    }
}

/// A label on an issue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssueLabel {
    /// Label name.
    pub name: String,

    /// Label color (hex without #).
    #[serde(default)]
    pub color: Option<String>,

    /// Label description.
    #[serde(default)]
    pub description: Option<String>,
}

/// A milestone on an issue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Milestone {
    /// Milestone number.
    pub number: u64,

    /// Milestone title.
    pub title: String,

    /// Milestone description.
    #[serde(default)]
    pub description: Option<String>,

    /// Milestone state: "open", "closed".
    pub state: String,
}

/// A comment on an issue.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IssueComment {
    /// Comment ID.
    pub id: u64,

    /// Comment author.
    pub user: UserSummary,

    /// Comment body.
    pub body: String,

    /// Creation timestamp.
    pub created_at: String,

    /// Last update timestamp.
    pub updated_at: String,

    /// HTML URL to the comment.
    #[serde(default)]
    pub html_url: Option<String>,

    /// API URL for the comment.
    #[serde(default)]
    pub url: Option<String>,
}

/// A timeline event on an issue.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TimelineEvent {
    /// Event ID (may be absent for some event types).
    #[serde(default)]
    pub id: Option<u64>,

    /// Event type (e.g., "commented", "committed", "labeled", "closed").
    #[serde(default)]
    pub event: Option<String>,

    /// Event timestamp.
    #[serde(default)]
    pub created_at: Option<String>,

    /// Actor who triggered the event.
    #[serde(default)]
    pub actor: Option<UserSummary>,

    /// Additional event data varies by event type.
    #[serde(flatten)]
    pub extra: serde_json::Value,
}

// =============================================================================
// Tag and Release Types
// =============================================================================

/// A repository tag from `GET /repos/{owner}/{repo}/tags`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepoTag {
    /// Tag name.
    pub name: String,

    /// Commit the tag points to.
    pub commit: TagCommit,

    /// URL to download tarball.
    pub tarball_url: String,

    /// URL to download zipball.
    pub zipball_url: String,

    /// Node ID.
    #[serde(default)]
    pub node_id: Option<String>,
}

/// Commit info embedded in a tag.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TagCommit {
    /// Commit SHA.
    pub sha: String,

    /// API URL for the commit.
    pub url: String,
}

/// A GitHub release from `GET /repos/{owner}/{repo}/releases`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Release {
    /// Release ID.
    pub id: u64,

    /// Tag name associated with the release.
    pub tag_name: String,

    /// Release name/title.
    #[serde(default)]
    pub name: Option<String>,

    /// Release body (markdown).
    #[serde(default)]
    pub body: Option<String>,

    /// Whether this is a draft.
    #[serde(default)]
    pub draft: bool,

    /// Whether this is a prerelease.
    #[serde(default)]
    pub prerelease: bool,

    /// Whether this release is immutable.
    #[serde(default)]
    pub immutable: Option<bool>,

    /// Creation timestamp.
    pub created_at: String,

    /// Publish timestamp.
    #[serde(default)]
    pub published_at: Option<String>,

    /// HTML URL to the release.
    pub html_url: String,

    /// API URL for the release.
    #[serde(default)]
    pub url: Option<String>,

    /// Author of the release.
    #[serde(default)]
    pub author: Option<UserSummary>,

    /// Release assets.
    #[serde(default)]
    pub assets: Vec<ReleaseAsset>,
}

/// An asset attached to a release.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReleaseAsset {
    /// Asset ID.
    pub id: u64,

    /// Asset filename.
    pub name: String,

    /// Asset content type.
    #[serde(default)]
    pub content_type: Option<String>,

    /// Asset size in bytes.
    #[serde(default)]
    pub size: Option<u64>,

    /// Download URL.
    #[serde(default)]
    pub browser_download_url: Option<String>,

    /// Download count.
    #[serde(default)]
    pub download_count: Option<u64>,
}

// =============================================================================
// Git Reference Types
// =============================================================================

/// A Git reference from `GET /repos/{owner}/{repo}/git/ref/tags/{tag}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitRef {
    /// Full ref name (e.g., "refs/tags/v1.0.0").
    #[serde(rename = "ref")]
    pub ref_name: String,

    /// Node ID.
    #[serde(default)]
    pub node_id: Option<String>,

    /// API URL.
    pub url: String,

    /// Object the ref points to.
    pub object: GitRefObject,
}

/// Object pointed to by a Git reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitRefObject {
    /// Object type: "commit" (lightweight tag) or "tag" (annotated tag).
    #[serde(rename = "type")]
    pub object_type: String,

    /// Object SHA.
    pub sha: String,

    /// API URL for the object.
    pub url: String,
}

impl GitRefObject {
    /// Returns `true` if this is an annotated tag.
    pub fn is_annotated_tag(&self) -> bool {
        self.object_type == "tag"
    }

    /// Returns `true` if this is a lightweight tag (points directly to commit).
    pub fn is_lightweight_tag(&self) -> bool {
        self.object_type == "commit"
    }
}

/// An annotated tag object from `GET /repos/{owner}/{repo}/git/tags/{sha}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnnotatedTagObject {
    /// Tag object SHA.
    pub sha: String,

    /// Tag name.
    pub tag: String,

    /// Tag message.
    pub message: String,

    /// Tagger information.
    #[serde(default)]
    pub tagger: Option<AnnotatedTagger>,

    /// Object the tag points to.
    pub object: TagObjectRef,

    /// GPG signature verification.
    #[serde(default)]
    pub verification: Option<TagVerification>,

    /// Node ID.
    #[serde(default)]
    pub node_id: Option<String>,

    /// API URL.
    #[serde(default)]
    pub url: Option<String>,
}

/// Object reference from an annotated tag.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TagObjectRef {
    /// Object type (usually "commit").
    #[serde(rename = "type")]
    pub object_type: String,

    /// Object SHA.
    pub sha: String,

    /// API URL.
    pub url: String,
}

/// Tagger information for annotated tags.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnnotatedTagger {
    /// Tagger's name.
    pub name: String,

    /// Tagger's email.
    pub email: String,

    /// Tagging timestamp (ISO 8601).
    pub date: String,
}

/// GPG signature verification for tags.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TagVerification {
    /// Whether the signature is verified.
    pub verified: bool,

    /// Verification reason/status.
    pub reason: String,

    /// Signature data.
    #[serde(default)]
    pub signature: Option<String>,

    /// Payload that was signed.
    #[serde(default)]
    pub payload: Option<String>,

    /// Verification timestamp.
    #[serde(default)]
    pub verified_at: Option<String>,
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_summary_deserialization() {
        let json = r#"{
            "login": "octocat",
            "id": 1,
            "avatar_url": "https://github.com/images/error/octocat_happy.gif",
            "type": "User"
        }"#;

        let user: UserSummary = serde_json::from_str(json).unwrap();
        assert_eq!(user.login, "octocat");
        assert_eq!(user.id, Some(1));
        assert_eq!(user.user_type, Some("User".to_string()));
    }

    #[test]
    fn repository_info_deserialization() {
        let json = r#"{
            "id": 1296269,
            "name": "Hello-World",
            "full_name": "octocat/Hello-World",
            "owner": { "login": "octocat" },
            "private": false,
            "html_url": "https://github.com/octocat/Hello-World",
            "url": "https://api.github.com/repos/octocat/Hello-World",
            "default_branch": "main",
            "archived": false,
            "disabled": false,
            "topics": ["octocat", "atom", "electron"]
        }"#;

        let repo: RepositoryInfo = serde_json::from_str(json).unwrap();
        assert_eq!(repo.name, "Hello-World");
        assert_eq!(repo.default_branch, "main");
        assert_eq!(repo.topics.len(), 3);
    }

    #[test]
    fn git_tree_entry_deserialization() {
        let json = r#"{
            "path": "README.md",
            "type": "blob",
            "mode": "100644",
            "sha": "abc123",
            "size": 1234
        }"#;

        let entry: GitTreeEntry = serde_json::from_str(json).unwrap();
        assert_eq!(entry.path, "README.md");
        assert_eq!(entry.entry_type, "blob");
        assert_eq!(entry.size, Some(1234));
    }

    #[test]
    fn git_tree_response_deserialization() {
        let json = r#"{
            "sha": "abc123",
            "url": "https://api.github.com/repos/octocat/Hello-World/git/trees/abc123",
            "tree": [
                {"path": "README.md", "type": "blob", "sha": "def456"}
            ],
            "truncated": false
        }"#;

        let tree: GitTreeResponse = serde_json::from_str(json).unwrap();
        assert_eq!(tree.sha, "abc123");
        assert_eq!(tree.tree.len(), 1);
        assert!(!tree.truncated);
    }

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
    }

    #[test]
    fn issue_summary_deserialization() {
        let json = r#"{
            "id": 1,
            "number": 1347,
            "state": "open",
            "title": "Found a bug",
            "user": { "login": "octocat" },
            "comments": 5,
            "created_at": "2011-04-22T13:33:48Z",
            "updated_at": "2011-04-22T13:33:48Z",
            "url": "https://api.github.com/repos/octocat/Hello-World/issues/1347",
            "html_url": "https://github.com/octocat/Hello-World/issues/1347",
            "labels": [{"name": "bug", "color": "d73a4a"}],
            "assignees": []
        }"#;

        let issue: IssueSummary = serde_json::from_str(json).unwrap();
        assert_eq!(issue.number, 1347);
        assert!(!issue.is_pull_request());
        assert_eq!(issue.labels.len(), 1);
    }

    #[test]
    fn issue_with_pull_request_field() {
        let json = r#"{
            "id": 1,
            "number": 1347,
            "state": "open",
            "title": "A PR",
            "user": { "login": "octocat" },
            "comments": 0,
            "created_at": "2011-04-22T13:33:48Z",
            "updated_at": "2011-04-22T13:33:48Z",
            "url": "https://api.github.com/repos/octocat/Hello-World/issues/1347",
            "html_url": "https://github.com/octocat/Hello-World/issues/1347",
            "pull_request": {"url": "https://api.github.com/repos/octocat/Hello-World/pulls/1347"},
            "labels": [],
            "assignees": []
        }"#;

        let issue: IssueSummary = serde_json::from_str(json).unwrap();
        assert!(issue.is_pull_request());
    }

    #[test]
    fn release_deserialization() {
        let json = r#"{
            "id": 1,
            "tag_name": "v1.0.0",
            "name": "Version 1.0.0",
            "draft": false,
            "prerelease": false,
            "created_at": "2013-02-27T19:35:32Z",
            "html_url": "https://github.com/octocat/Hello-World/releases/v1.0.0",
            "assets": []
        }"#;

        let release: Release = serde_json::from_str(json).unwrap();
        assert_eq!(release.tag_name, "v1.0.0");
        assert!(!release.draft);
        assert!(!release.prerelease);
    }

    #[test]
    fn git_ref_lightweight_tag() {
        let json = r#"{
            "ref": "refs/tags/v1.0.0",
            "url": "https://api.github.com/repos/octocat/Hello-World/git/ref/tags/v1.0.0",
            "object": {
                "type": "commit",
                "sha": "abc123",
                "url": "https://api.github.com/repos/octocat/Hello-World/git/commits/abc123"
            }
        }"#;

        let git_ref: GitRef = serde_json::from_str(json).unwrap();
        assert!(git_ref.object.is_lightweight_tag());
        assert!(!git_ref.object.is_annotated_tag());
    }

    #[test]
    fn git_ref_annotated_tag() {
        let json = r#"{
            "ref": "refs/tags/v1.0.0",
            "url": "https://api.github.com/repos/octocat/Hello-World/git/ref/tags/v1.0.0",
            "object": {
                "type": "tag",
                "sha": "abc123",
                "url": "https://api.github.com/repos/octocat/Hello-World/git/tags/abc123"
            }
        }"#;

        let git_ref: GitRef = serde_json::from_str(json).unwrap();
        assert!(git_ref.object.is_annotated_tag());
        assert!(!git_ref.object.is_lightweight_tag());
    }

    #[test]
    fn annotated_tag_object_deserialization() {
        let json = r#"{
            "sha": "abc123",
            "tag": "v1.0.0",
            "message": "Release v1.0.0\n\nInitial release.",
            "tagger": {
                "name": "Octocat",
                "email": "octocat@github.com",
                "date": "2014-11-07T22:01:45Z"
            },
            "object": {
                "type": "commit",
                "sha": "def456",
                "url": "https://api.github.com/repos/octocat/Hello-World/git/commits/def456"
            }
        }"#;

        let tag: AnnotatedTagObject = serde_json::from_str(json).unwrap();
        assert_eq!(tag.tag, "v1.0.0");
        assert!(tag.message.contains("Initial release"));
        assert!(tag.tagger.is_some());
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

    #[test]
    fn repo_tag_deserialization() {
        let json = r#"{
            "name": "v1.0.0",
            "commit": {
                "sha": "abc123",
                "url": "https://api.github.com/repos/octocat/Hello-World/commits/abc123"
            },
            "tarball_url": "https://api.github.com/repos/octocat/Hello-World/tarball/v1.0.0",
            "zipball_url": "https://api.github.com/repos/octocat/Hello-World/zipball/v1.0.0"
        }"#;

        let tag: RepoTag = serde_json::from_str(json).unwrap();
        assert_eq!(tag.name, "v1.0.0");
        assert_eq!(tag.commit.sha, "abc123");
    }

    #[test]
    fn issue_comment_deserialization() {
        let json = r#"{
            "id": 1,
            "user": { "login": "octocat" },
            "body": "This looks great!",
            "created_at": "2011-04-14T16:00:49Z",
            "updated_at": "2011-04-14T16:00:49Z"
        }"#;

        let comment: IssueComment = serde_json::from_str(json).unwrap();
        assert_eq!(comment.id, 1);
        assert_eq!(comment.body, "This looks great!");
    }

    #[test]
    fn timeline_event_deserialization() {
        let json = r#"{
            "id": 123,
            "event": "labeled",
            "created_at": "2011-04-14T16:00:49Z",
            "actor": { "login": "octocat" },
            "label": { "name": "bug", "color": "d73a4a" }
        }"#;

        let event: TimelineEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.event, Some("labeled".to_string()));
        assert!(event.extra.get("label").is_some());
    }
}
