//! Bitbucket Cloud REST API types.
//!
//! This module contains request and response types for the Bitbucket Cloud REST API,
//! including repository metadata, pull requests, issues, tags, and downloads.
//!
//! ## Design Notes
//!
//! - All types use `Option<_>` liberally for forward-compatibility with API evolution
//! - Fields that are frequently omitted or permission-dependent are optional
//! - `serde(default)` is used where empty/null values should deserialize cleanly
//! - Bitbucket uses `created_on`/`updated_on` timestamps (not `created_at`/`updated_at`)
//! - Bitbucket uses cursor-based pagination with a `next` URL field
//! - Tags + Downloads = "Releases" (no first-class release concept)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// =============================================================================
// Common Types
// =============================================================================

/// A Bitbucket user summary (common across many responses).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct User {
    /// The user's UUID (e.g., "{abc-123}").
    #[serde(default)]
    pub uuid: Option<String>,

    /// The user's display name.
    #[serde(default)]
    pub display_name: Option<String>,

    /// The user's nickname/username.
    #[serde(default)]
    pub nickname: Option<String>,

    /// The user's account ID.
    #[serde(default)]
    pub account_id: Option<String>,

    /// Object type (e.g., "user").
    #[serde(rename = "type", default)]
    pub user_type: Option<String>,

    /// HATEOAS links for this user.
    #[serde(default)]
    pub links: Option<HashMap<String, Link>>,
}

impl User {
    /// Returns the user's name, preferring `display_name` over `nickname`.
    pub fn name(&self) -> Option<&str> {
        self.display_name.as_deref().or(self.nickname.as_deref())
    }
}

/// A HATEOAS link with href field.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Link {
    /// The URL this link points to.
    pub href: String,
}

/// A paginated response wrapper.
///
/// Bitbucket uses cursor-based pagination. Follow the `next` URL to get more results.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PaginatedResponse<T> {
    /// The items in this page.
    #[serde(default)]
    pub values: Vec<T>,

    /// URL to the next page of results, if any.
    #[serde(default)]
    pub next: Option<String>,

    /// URL to the previous page of results, if any.
    #[serde(default)]
    pub previous: Option<String>,

    /// Total number of items across all pages.
    #[serde(default)]
    pub size: Option<u64>,

    /// Number of items per page.
    #[serde(default)]
    pub pagelen: Option<u64>,

    /// Current page number (1-indexed).
    #[serde(default)]
    pub page: Option<u64>,
}

impl<T> PaginatedResponse<T> {
    /// Returns `true` if there are more pages available.
    pub fn has_next(&self) -> bool {
        self.next.is_some()
    }
}

// =============================================================================
// Repository Types
// =============================================================================

/// Repository information from `GET /repositories/{workspace}/{repo_slug}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Repository {
    /// The repository's UUID.
    #[serde(default)]
    pub uuid: Option<String>,

    /// The repository slug (URL-friendly name).
    #[serde(default)]
    pub slug: Option<String>,

    /// The repository name.
    #[serde(default)]
    pub name: Option<String>,

    /// Full name in "workspace/repo_slug" format.
    #[serde(default)]
    pub full_name: Option<String>,

    /// Repository description.
    #[serde(default)]
    pub description: Option<String>,

    /// Whether the repository is private.
    #[serde(default)]
    pub is_private: bool,

    /// Repository owner.
    #[serde(default)]
    pub owner: Option<User>,

    /// Workspace the repository belongs to.
    #[serde(default)]
    pub workspace: Option<Workspace>,

    /// Project the repository belongs to.
    #[serde(default)]
    pub project: Option<Project>,

    /// Primary programming language.
    #[serde(default)]
    pub language: Option<String>,

    /// Creation timestamp (ISO 8601).
    #[serde(default)]
    pub created_on: Option<String>,

    /// Last update timestamp (ISO 8601).
    #[serde(default)]
    pub updated_on: Option<String>,

    /// Size of the repository in bytes.
    #[serde(default)]
    pub size: Option<u64>,

    /// Whether the repository has issues enabled.
    #[serde(default)]
    pub has_issues: Option<bool>,

    /// Whether the repository has a wiki enabled.
    #[serde(default)]
    pub has_wiki: Option<bool>,

    /// Fork policy: "allow_forks", "no_public_forks", "no_forks".
    #[serde(default)]
    pub fork_policy: Option<String>,

    /// Main branch information.
    #[serde(default)]
    pub mainbranch: Option<BranchInfo>,

    /// Object type (e.g., "repository").
    #[serde(rename = "type", default)]
    pub repo_type: Option<String>,

    /// HATEOAS links for this repository.
    #[serde(default)]
    pub links: Option<HashMap<String, Link>>,
}

impl Repository {
    /// Returns the default branch name if available.
    pub fn default_branch(&self) -> Option<&str> {
        self.mainbranch.as_ref().and_then(|b| b.name.as_deref())
    }
}

/// Simple branch information (for mainbranch).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BranchInfo {
    /// Branch name.
    #[serde(default)]
    pub name: Option<String>,

    /// Object type (e.g., "branch").
    #[serde(rename = "type", default)]
    pub branch_type: Option<String>,
}

/// Workspace information.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Workspace {
    /// The workspace slug.
    #[serde(default)]
    pub slug: Option<String>,

    /// The workspace UUID.
    #[serde(default)]
    pub uuid: Option<String>,

    /// The workspace name.
    #[serde(default)]
    pub name: Option<String>,

    /// Object type (e.g., "workspace").
    #[serde(rename = "type", default)]
    pub workspace_type: Option<String>,

    /// HATEOAS links.
    #[serde(default)]
    pub links: Option<HashMap<String, Link>>,
}

/// Project information.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Project {
    /// The project key.
    #[serde(default)]
    pub key: Option<String>,

    /// The project UUID.
    #[serde(default)]
    pub uuid: Option<String>,

    /// The project name.
    #[serde(default)]
    pub name: Option<String>,

    /// Object type (e.g., "project").
    #[serde(rename = "type", default)]
    pub project_type: Option<String>,

    /// HATEOAS links.
    #[serde(default)]
    pub links: Option<HashMap<String, Link>>,
}

// =============================================================================
// Source/Content Types
// =============================================================================

/// A source entry from directory listing.
///
/// Returned by `GET /repositories/{workspace}/{repo_slug}/src/{commit}/{path}`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceEntry {
    /// Full path from repository root.
    #[serde(default)]
    pub path: Option<String>,

    /// Entry type: "commit_directory" or "commit_file".
    #[serde(rename = "type", default)]
    pub entry_type: Option<String>,

    /// Size in bytes (only for files).
    #[serde(default)]
    pub size: Option<u64>,

    /// Commit hash (for files).
    #[serde(default)]
    pub commit: Option<CommitInfo>,

    /// HATEOAS links including "self" for raw content.
    #[serde(default)]
    pub links: Option<HashMap<String, Link>>,

    /// MIME type for files.
    #[serde(default)]
    pub mimetype: Option<String>,

    /// File attributes.
    #[serde(default)]
    pub attributes: Option<Vec<String>>,
}

impl SourceEntry {
    /// Returns `true` if this entry is a directory.
    pub fn is_directory(&self) -> bool {
        self.entry_type.as_deref() == Some("commit_directory")
    }

    /// Returns `true` if this entry is a file.
    pub fn is_file(&self) -> bool {
        self.entry_type.as_deref() == Some("commit_file")
    }

    /// Returns the filename (last component of path).
    pub fn filename(&self) -> Option<&str> {
        self.path.as_deref().and_then(|p| p.rsplit('/').next())
    }
}

/// Simple commit info embedded in other types.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommitInfo {
    /// Commit hash.
    #[serde(default)]
    pub hash: Option<String>,

    /// Object type (e.g., "commit").
    #[serde(rename = "type", default)]
    pub commit_type: Option<String>,

    /// HATEOAS links.
    #[serde(default)]
    pub links: Option<HashMap<String, Link>>,
}

// =============================================================================
// Pull Request Types
// =============================================================================

/// Pull request from `GET /repositories/{workspace}/{repo_slug}/pullrequests`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommentParent {
    /// Parent comment ID.
    #[serde(default)]
    pub id: Option<u64>,
}

/// Inline context for code comments.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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

// =============================================================================
// Issue Types
// =============================================================================

/// An issue from `GET /repositories/{workspace}/{repo_slug}/issues`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Issue {
    /// Issue ID (unique within the repository).
    #[serde(default)]
    pub id: Option<u64>,

    /// Issue title.
    #[serde(default)]
    pub title: Option<String>,

    /// Issue type: "bug", "enhancement", "proposal", "task".
    #[serde(default)]
    pub kind: Option<String>,

    /// Issue priority: "trivial", "minor", "major", "critical", "blocker".
    #[serde(default)]
    pub priority: Option<String>,

    /// Issue state: "new", "open", "resolved", "on hold", "invalid", "duplicate", "wontfix", "closed".
    #[serde(default)]
    pub state: Option<String>,

    /// Creation timestamp (ISO 8601).
    #[serde(default)]
    pub created_on: Option<String>,

    /// Last update timestamp (ISO 8601).
    #[serde(default)]
    pub updated_on: Option<String>,

    /// Last edit timestamp (ISO 8601).
    #[serde(default)]
    pub edited_on: Option<String>,

    /// Issue reporter.
    #[serde(default)]
    pub reporter: Option<User>,

    /// Assigned user.
    #[serde(default)]
    pub assignee: Option<User>,

    /// Issue content/description.
    #[serde(default)]
    pub content: Option<Content>,

    /// Number of comments.
    #[serde(default)]
    pub comment_count: Option<u32>,

    /// Number of votes.
    #[serde(default)]
    pub votes: Option<u32>,

    /// Number of watchers.
    #[serde(default)]
    pub watchers: Option<u32>,

    /// Repository this issue belongs to.
    #[serde(default)]
    pub repository: Option<RepositoryRef>,

    /// Component (if any).
    #[serde(default)]
    pub component: Option<String>,

    /// Version (if any).
    #[serde(default)]
    pub version: Option<String>,

    /// Milestone (if any).
    #[serde(default)]
    pub milestone: Option<Milestone>,

    /// Object type.
    #[serde(rename = "type", default)]
    pub issue_type: Option<String>,

    /// HATEOAS links.
    #[serde(default)]
    pub links: Option<HashMap<String, Link>>,
}

impl Issue {
    /// Returns `true` if this issue is open.
    pub fn is_open(&self) -> bool {
        matches!(self.state.as_deref(), Some("new" | "open"))
    }

    /// Returns `true` if this issue is resolved.
    pub fn is_resolved(&self) -> bool {
        self.state.as_deref() == Some("resolved")
    }

    /// Returns `true` if this issue is a bug.
    pub fn is_bug(&self) -> bool {
        self.kind.as_deref() == Some("bug")
    }
}

/// Content with raw, markup, and HTML forms.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Content {
    /// Raw content (markdown or plain text).
    #[serde(default)]
    pub raw: Option<String>,

    /// Markup type: "markdown", "creole", "plaintext".
    #[serde(default)]
    pub markup: Option<String>,

    /// Rendered HTML.
    #[serde(default)]
    pub html: Option<String>,
}

/// Milestone reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Milestone {
    /// Milestone ID.
    #[serde(default)]
    pub id: Option<u64>,

    /// Milestone name.
    #[serde(default)]
    pub name: Option<String>,

    /// HATEOAS links.
    #[serde(default)]
    pub links: Option<HashMap<String, Link>>,
}

/// A comment on an issue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssueComment {
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

    /// Object type.
    #[serde(rename = "type", default)]
    pub comment_type: Option<String>,

    /// HATEOAS links.
    #[serde(default)]
    pub links: Option<HashMap<String, Link>>,
}

/// An issue change record from the change history.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssueChange {
    /// Change timestamp (ISO 8601).
    #[serde(default)]
    pub created_on: Option<String>,

    /// User who made the change.
    #[serde(default)]
    pub user: Option<User>,

    /// The changes made (field name -> change detail).
    #[serde(default)]
    pub changes: Option<HashMap<String, ChangeDetail>>,

    /// Message associated with the change (if any).
    #[serde(default)]
    pub message: Option<Content>,

    /// Object type.
    #[serde(rename = "type", default)]
    pub change_type: Option<String>,

    /// HATEOAS links.
    #[serde(default)]
    pub links: Option<HashMap<String, Link>>,
}

/// Detail of a field change.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChangeDetail {
    /// Old value.
    #[serde(default)]
    pub old: Option<String>,

    /// New value.
    #[serde(default)]
    pub new: Option<String>,
}

// =============================================================================
// Tag Types
// =============================================================================

/// A repository tag from `GET /repositories/{workspace}/{repo_slug}/refs/tags`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tag {
    /// Tag name.
    #[serde(default)]
    pub name: Option<String>,

    /// Object type (e.g., "tag").
    #[serde(rename = "type", default)]
    pub tag_type: Option<String>,

    /// Commit this tag points to.
    #[serde(default)]
    pub target: Option<TagTarget>,

    /// Creation timestamp (ISO 8601, for annotated tags).
    #[serde(default)]
    pub date: Option<String>,

    /// Tag message (for annotated tags).
    #[serde(default)]
    pub message: Option<String>,

    /// Tagger information (for annotated tags).
    #[serde(default)]
    pub tagger: Option<Tagger>,

    /// HATEOAS links.
    #[serde(default)]
    pub links: Option<HashMap<String, Link>>,
}

impl Tag {
    /// Returns `true` if this is an annotated tag (has a message).
    pub fn is_annotated(&self) -> bool {
        self.message.is_some() || self.tagger.is_some()
    }
}

/// Target commit for a tag.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TagTarget {
    /// Commit hash.
    #[serde(default)]
    pub hash: Option<String>,

    /// Object type (e.g., "commit").
    #[serde(rename = "type", default)]
    pub target_type: Option<String>,

    /// Commit date (ISO 8601).
    #[serde(default)]
    pub date: Option<String>,

    /// Commit message.
    #[serde(default)]
    pub message: Option<String>,

    /// Commit author.
    #[serde(default)]
    pub author: Option<CommitAuthor>,

    /// HATEOAS links.
    #[serde(default)]
    pub links: Option<HashMap<String, Link>>,
}

/// Commit author information.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommitAuthor {
    /// Raw author string (e.g., "Name <email>").
    #[serde(default)]
    pub raw: Option<String>,

    /// Linked user account (if any).
    #[serde(default)]
    pub user: Option<User>,
}

impl CommitAuthor {
    /// Extracts the name from the raw author string.
    pub fn name(&self) -> Option<&str> {
        self.raw.as_deref().and_then(|r| r.split('<').next()).map(|s| s.trim())
    }

    /// Extracts the email from the raw author string.
    pub fn email(&self) -> Option<&str> {
        self.raw.as_deref().and_then(|r| {
            r.find('<').and_then(|start| {
                r.find('>').map(|end| &r[start + 1..end])
            })
        })
    }
}

/// Tagger information for annotated tags.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tagger {
    /// Raw tagger string (e.g., "Name <email>").
    #[serde(default)]
    pub raw: Option<String>,

    /// Linked user account (if any).
    #[serde(default)]
    pub user: Option<User>,
}

// =============================================================================
// Download Types
// =============================================================================

/// A download artifact from `GET /repositories/{workspace}/{repo_slug}/downloads`.
///
/// Downloads serve as release artifacts in Bitbucket (there's no first-class release concept).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Download {
    /// Filename.
    #[serde(default)]
    pub name: Option<String>,

    /// Creation timestamp (ISO 8601).
    #[serde(default)]
    pub created_on: Option<String>,

    /// User who uploaded this download.
    #[serde(default)]
    pub user: Option<User>,

    /// File size in bytes.
    #[serde(default)]
    pub size: Option<u64>,

    /// Number of times this file has been downloaded.
    #[serde(default)]
    pub downloads: Option<u64>,

    /// Object type.
    #[serde(rename = "type", default)]
    pub download_type: Option<String>,

    /// HATEOAS links (includes "self" for download URL).
    #[serde(default)]
    pub links: Option<HashMap<String, Link>>,
}

impl Download {
    /// Returns the download URL if available.
    pub fn download_url(&self) -> Option<&str> {
        self.links.as_ref()
            .and_then(|l| l.get("self"))
            .map(|link| link.href.as_str())
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_deserialization() {
        let json = r#"{
            "uuid": "{abc-123}",
            "display_name": "Test User",
            "nickname": "testuser",
            "type": "user"
        }"#;

        let user: User = serde_json::from_str(json).unwrap();
        assert_eq!(user.uuid, Some("{abc-123}".to_string()));
        assert_eq!(user.display_name, Some("Test User".to_string()));
        assert_eq!(user.name(), Some("Test User"));
    }

    #[test]
    fn user_name_fallback() {
        let json = r#"{"nickname": "testuser"}"#;

        let user: User = serde_json::from_str(json).unwrap();
        assert_eq!(user.name(), Some("testuser"));
    }

    #[test]
    fn link_deserialization() {
        let json = r#"{"href": "https://api.bitbucket.org/2.0/users/testuser"}"#;

        let link: Link = serde_json::from_str(json).unwrap();
        assert_eq!(link.href, "https://api.bitbucket.org/2.0/users/testuser");
    }

    #[test]
    fn paginated_response_deserialization() {
        let json = r#"{
            "values": [{"name": "item1"}, {"name": "item2"}],
            "next": "https://api.bitbucket.org/2.0/next-page",
            "size": 100,
            "pagelen": 50,
            "page": 1
        }"#;

        let response: PaginatedResponse<serde_json::Value> = serde_json::from_str(json).unwrap();
        assert_eq!(response.values.len(), 2);
        assert!(response.has_next());
        assert_eq!(response.size, Some(100));
    }

    #[test]
    fn paginated_response_no_next() {
        let json = r#"{"values": []}"#;

        let response: PaginatedResponse<serde_json::Value> = serde_json::from_str(json).unwrap();
        assert!(!response.has_next());
    }

    #[test]
    fn repository_deserialization() {
        let json = r#"{
            "uuid": "{repo-uuid}",
            "slug": "my-repo",
            "name": "My Repository",
            "full_name": "workspace/my-repo",
            "is_private": true,
            "language": "rust",
            "mainbranch": {"name": "main", "type": "branch"},
            "type": "repository"
        }"#;

        let repo: Repository = serde_json::from_str(json).unwrap();
        assert_eq!(repo.slug, Some("my-repo".to_string()));
        assert!(repo.is_private);
        assert_eq!(repo.default_branch(), Some("main"));
    }

    #[test]
    fn source_entry_directory_deserialization() {
        let json = r#"{
            "path": "src",
            "type": "commit_directory"
        }"#;

        let entry: SourceEntry = serde_json::from_str(json).unwrap();
        assert!(entry.is_directory());
        assert!(!entry.is_file());
        assert_eq!(entry.filename(), Some("src"));
    }

    #[test]
    fn source_entry_file_deserialization() {
        let json = r#"{
            "path": "src/main.rs",
            "type": "commit_file",
            "size": 1234,
            "mimetype": "text/x-rust"
        }"#;

        let entry: SourceEntry = serde_json::from_str(json).unwrap();
        assert!(entry.is_file());
        assert!(!entry.is_directory());
        assert_eq!(entry.filename(), Some("main.rs"));
        assert_eq!(entry.size, Some(1234));
    }

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
        assert_eq!(comment.content.as_ref().unwrap().raw, Some("Looks good!".to_string()));
    }

    #[test]
    fn issue_deserialization() {
        let json = r#"{
            "id": 1,
            "title": "Bug report",
            "kind": "bug",
            "priority": "major",
            "state": "open",
            "created_on": "2026-01-15T10:00:00Z",
            "updated_on": "2026-01-15T12:00:00Z",
            "reporter": {"display_name": "Reporter"},
            "content": {"raw": "Found a bug", "markup": "markdown"},
            "comment_count": 3,
            "votes": 10,
            "watchers": 5,
            "type": "issue"
        }"#;

        let issue: Issue = serde_json::from_str(json).unwrap();
        assert_eq!(issue.id, Some(1));
        assert!(issue.is_open());
        assert!(issue.is_bug());
        assert_eq!(issue.priority, Some("major".to_string()));
        assert_eq!(issue.comment_count, Some(3));
    }

    #[test]
    fn issue_resolved_state() {
        let json = r#"{"id": 1, "state": "resolved"}"#;

        let issue: Issue = serde_json::from_str(json).unwrap();
        assert!(issue.is_resolved());
        assert!(!issue.is_open());
    }

    #[test]
    fn issue_comment_deserialization() {
        let json = r#"{
            "id": 456,
            "content": {"raw": "Thanks for reporting!", "markup": "markdown"},
            "user": {"display_name": "Developer"},
            "created_on": "2026-01-15T15:00:00Z",
            "type": "issue_comment"
        }"#;

        let comment: IssueComment = serde_json::from_str(json).unwrap();
        assert_eq!(comment.id, Some(456));
    }

    #[test]
    fn issue_change_deserialization() {
        let json = r#"{
            "created_on": "2026-01-15T16:00:00Z",
            "user": {"display_name": "Developer"},
            "changes": {
                "state": {"old": "open", "new": "resolved"},
                "assignee": {"old": null, "new": "Developer"}
            },
            "type": "issue_change"
        }"#;

        let change: IssueChange = serde_json::from_str(json).unwrap();
        assert!(change.changes.is_some());
        let changes = change.changes.unwrap();
        assert!(changes.contains_key("state"));
        assert_eq!(changes["state"].old, Some("open".to_string()));
        assert_eq!(changes["state"].new, Some("resolved".to_string()));
    }

    #[test]
    fn tag_deserialization() {
        let json = r#"{
            "name": "v1.0.0",
            "type": "tag",
            "target": {
                "hash": "abc123def456",
                "type": "commit",
                "date": "2026-01-15T10:00:00Z",
                "message": "Release v1.0.0"
            },
            "message": "Version 1.0.0",
            "tagger": {"raw": "Developer <dev@example.com>"}
        }"#;

        let tag: Tag = serde_json::from_str(json).unwrap();
        assert_eq!(tag.name, Some("v1.0.0".to_string()));
        assert!(tag.is_annotated());
        assert_eq!(tag.target.as_ref().unwrap().hash, Some("abc123def456".to_string()));
    }

    #[test]
    fn tag_lightweight() {
        let json = r#"{
            "name": "v0.1.0",
            "type": "tag",
            "target": {"hash": "def456", "type": "commit"}
        }"#;

        let tag: Tag = serde_json::from_str(json).unwrap();
        assert!(!tag.is_annotated());
    }

    #[test]
    fn commit_author_parsing() {
        let author = CommitAuthor {
            raw: Some("John Doe <john@example.com>".to_string()),
            user: None,
        };

        assert_eq!(author.name(), Some("John Doe"));
        assert_eq!(author.email(), Some("john@example.com"));
    }

    #[test]
    fn download_deserialization() {
        let json = r#"{
            "name": "app-v1.0.0.tar.gz",
            "created_on": "2026-01-15T10:00:00Z",
            "user": {"display_name": "Releaser"},
            "size": 1048576,
            "downloads": 100,
            "type": "download",
            "links": {
                "self": {"href": "https://bitbucket.org/workspace/repo/downloads/app-v1.0.0.tar.gz"}
            }
        }"#;

        let download: Download = serde_json::from_str(json).unwrap();
        assert_eq!(download.name, Some("app-v1.0.0.tar.gz".to_string()));
        assert_eq!(download.size, Some(1048576));
        assert_eq!(download.downloads, Some(100));
        assert!(download.download_url().is_some());
    }

    #[test]
    fn content_deserialization() {
        let json = r##"{
            "raw": "# Hello World",
            "markup": "markdown",
            "html": "<h1>Hello World</h1>"
        }"##;

        let content: Content = serde_json::from_str(json).unwrap();
        assert_eq!(content.raw, Some("# Hello World".to_string()));
        assert_eq!(content.markup, Some("markdown".to_string()));
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
    fn empty_json_deserializes_cleanly() {
        // All fields are optional, so empty objects should work
        let _user: User = serde_json::from_str("{}").unwrap();
        let _repo: Repository = serde_json::from_str("{}").unwrap();
        let _pr: PullRequest = serde_json::from_str("{}").unwrap();
        let _issue: Issue = serde_json::from_str("{}").unwrap();
        let _tag: Tag = serde_json::from_str("{}").unwrap();
        let _download: Download = serde_json::from_str("{}").unwrap();
        let _source: SourceEntry = serde_json::from_str("{}").unwrap();
    }

    #[test]
    fn workspace_deserialization() {
        let json = r#"{
            "slug": "my-workspace",
            "uuid": "{workspace-uuid}",
            "name": "My Workspace",
            "type": "workspace"
        }"#;

        let workspace: Workspace = serde_json::from_str(json).unwrap();
        assert_eq!(workspace.slug, Some("my-workspace".to_string()));
        assert_eq!(workspace.name, Some("My Workspace".to_string()));
    }

    #[test]
    fn project_deserialization() {
        let json = r#"{
            "key": "PROJ",
            "uuid": "{project-uuid}",
            "name": "My Project",
            "type": "project"
        }"#;

        let project: Project = serde_json::from_str(json).unwrap();
        assert_eq!(project.key, Some("PROJ".to_string()));
        assert_eq!(project.name, Some("My Project".to_string()));
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
