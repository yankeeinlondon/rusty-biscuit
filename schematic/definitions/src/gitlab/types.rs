//! GitLab REST API types.
//!
//! This module contains request and response types for the GitLab REST API,
//! including repository metadata, merge requests, issues, tags, and releases.
//!
//! ## Design Notes
//!
//! - All types use `Option<_>` liberally for forward-compatibility with API evolution
//! - Fields that are frequently omitted or permission-dependent are optional
//! - `serde(default)` is used where empty/null values should deserialize cleanly
//! - GitLab uses `iid` (internal ID) scoped to projects, not global `id`

use serde::{Deserialize, Serialize};

// =============================================================================
// Common Types
// =============================================================================

/// A GitLab user summary (common across many responses).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct User {
    /// The user's unique ID.
    pub id: u64,

    /// The user's username handle.
    pub username: String,

    /// The user's display name.
    #[serde(default)]
    pub name: Option<String>,

    /// User state (e.g., "active", "blocked").
    #[serde(default)]
    pub state: Option<String>,

    /// URL to the user's avatar image.
    #[serde(default)]
    pub avatar_url: Option<String>,

    /// HTML URL to the user's profile.
    #[serde(default)]
    pub web_url: Option<String>,
}

// =============================================================================
// Repository Tree Types
// =============================================================================

/// A single entry in a repository tree from `GET /projects/:id/repository/tree`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TreeItem {
    /// Object ID (SHA).
    pub id: String,

    /// File or directory name.
    pub name: String,

    /// Entry type: "blob" (file) or "tree" (directory).
    #[serde(rename = "type")]
    pub item_type: String,

    /// Full path from repository root.
    pub path: String,

    /// Git file mode (e.g., "100644" for regular file).
    #[serde(default)]
    pub mode: Option<String>,
}

// =============================================================================
// Repository File Types
// =============================================================================

/// File content from `GET /projects/:id/repository/files/:path`.
///
/// Note: The `content` field is Base64 encoded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileContent {
    /// Filename.
    pub file_name: String,

    /// Full file path.
    pub file_path: String,

    /// File size in bytes.
    pub size: u64,

    /// Content encoding (typically "base64").
    pub encoding: String,

    /// Base64-encoded file content.
    pub content: String,

    /// Git ref (branch/tag) the file was retrieved from.
    #[serde(rename = "ref")]
    pub ref_name: String,

    /// Blob ID (SHA).
    pub blob_id: String,

    /// Commit ID (SHA).
    pub commit_id: String,

    /// Content SHA256 hash.
    #[serde(default)]
    pub content_sha256: Option<String>,

    /// Last commit ID that modified this file.
    #[serde(default)]
    pub last_commit_id: Option<String>,
}

// =============================================================================
// Merge Request Types
// =============================================================================

/// Merge request summary from list/get endpoints.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MergeRequest {
    /// Global merge request ID.
    pub id: u64,

    /// Project-scoped internal ID (use this for API calls).
    pub iid: u64,

    /// Project ID.
    pub project_id: u64,

    /// MR title.
    pub title: String,

    /// MR description/body.
    #[serde(default)]
    pub description: Option<String>,

    /// MR state: "opened", "closed", "merged", "locked".
    pub state: String,

    /// Creation timestamp (ISO 8601).
    pub created_at: String,

    /// Last update timestamp (ISO 8601).
    pub updated_at: String,

    /// Merge timestamp (ISO 8601, if merged).
    #[serde(default)]
    pub merged_at: Option<String>,

    /// Close timestamp (ISO 8601, if closed without merge).
    #[serde(default)]
    pub closed_at: Option<String>,

    /// Target branch name.
    pub target_branch: String,

    /// Source branch name.
    pub source_branch: String,

    /// MR author.
    pub author: User,

    /// Assigned user (single).
    #[serde(default)]
    pub assignee: Option<User>,

    /// Assigned users (multiple).
    #[serde(default)]
    pub assignees: Vec<User>,

    /// Reviewers.
    #[serde(default)]
    pub reviewers: Vec<User>,

    /// Source project ID.
    #[serde(default)]
    pub source_project_id: Option<u64>,

    /// Target project ID.
    #[serde(default)]
    pub target_project_id: Option<u64>,

    /// Labels on the MR.
    #[serde(default)]
    pub labels: Vec<String>,

    /// Whether this is a draft MR.
    #[serde(default)]
    pub draft: bool,

    /// Work in progress (legacy, prefer `draft`).
    #[serde(default)]
    pub work_in_progress: bool,

    /// Associated milestone.
    #[serde(default)]
    pub milestone: Option<Milestone>,

    /// Whether to merge when pipeline succeeds.
    #[serde(default)]
    pub merge_when_pipeline_succeeds: bool,

    /// Merge status (e.g., "can_be_merged", "cannot_be_merged").
    #[serde(default)]
    pub merge_status: Option<String>,

    /// HEAD commit SHA.
    #[serde(default)]
    pub sha: Option<String>,

    /// Merge commit SHA (if merged).
    #[serde(default)]
    pub merge_commit_sha: Option<String>,

    /// Squash commit SHA (if squash merged).
    #[serde(default)]
    pub squash_commit_sha: Option<String>,

    /// Number of user comments/notes.
    #[serde(default)]
    pub user_notes_count: u64,

    /// Upvotes count.
    #[serde(default)]
    pub upvotes: u64,

    /// Downvotes count.
    #[serde(default)]
    pub downvotes: u64,

    /// HTML URL to the MR.
    #[serde(default)]
    pub web_url: Option<String>,

    /// Reference information.
    #[serde(default)]
    pub references: Option<References>,

    /// Time tracking statistics.
    #[serde(default)]
    pub time_stats: Option<TimeStats>,

    /// Whether to squash on merge.
    #[serde(default)]
    pub squash: bool,

    /// Number of changes.
    #[serde(default)]
    pub changes_count: Option<String>,

    /// User who merged the MR.
    #[serde(default)]
    pub merged_by: Option<User>,

    /// User who closed the MR.
    #[serde(default)]
    pub closed_by: Option<User>,
}

/// Merge request with changes/diffs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MergeRequestChanges {
    /// Global merge request ID.
    pub id: u64,

    /// Project-scoped internal ID.
    pub iid: u64,

    /// Project ID.
    pub project_id: u64,

    /// MR title.
    pub title: String,

    /// MR description.
    #[serde(default)]
    pub description: Option<String>,

    /// MR state.
    pub state: String,

    /// Merge timestamp.
    #[serde(default)]
    pub merged_at: Option<String>,

    /// Close timestamp.
    #[serde(default)]
    pub closed_at: Option<String>,

    /// Target branch name.
    pub target_branch: String,

    /// Source branch name.
    pub source_branch: String,

    /// File changes/diffs.
    #[serde(default)]
    pub changes: Vec<Diff>,
}

/// A commit in a merge request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Commit {
    /// Commit SHA.
    pub id: String,

    /// Short commit SHA.
    pub short_id: String,

    /// Creation timestamp (ISO 8601).
    pub created_at: String,

    /// Parent commit SHAs.
    #[serde(default)]
    pub parent_ids: Vec<String>,

    /// Commit title (first line of message).
    pub title: String,

    /// Full commit message.
    #[serde(default)]
    pub message: Option<String>,

    /// Author name.
    pub author_name: String,

    /// Author email.
    pub author_email: String,

    /// Authored timestamp (ISO 8601).
    #[serde(default)]
    pub authored_date: Option<String>,

    /// Committer name.
    #[serde(default)]
    pub committer_name: Option<String>,

    /// Committer email.
    #[serde(default)]
    pub committer_email: Option<String>,

    /// Committed timestamp (ISO 8601).
    #[serde(default)]
    pub committed_date: Option<String>,

    /// HTML URL to the commit.
    #[serde(default)]
    pub web_url: Option<String>,
}

/// A file diff in a merge request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diff {
    /// Old file path (before rename).
    pub old_path: String,

    /// New file path (after rename).
    pub new_path: String,

    /// Old file mode.
    #[serde(default)]
    pub a_mode: Option<String>,

    /// New file mode.
    #[serde(default)]
    pub b_mode: Option<String>,

    /// Diff content (unified diff format).
    #[serde(default)]
    pub diff: Option<String>,

    /// Whether this is a new file.
    #[serde(default)]
    pub new_file: bool,

    /// Whether this file was renamed.
    #[serde(default)]
    pub renamed_file: bool,

    /// Whether this file was deleted.
    #[serde(default)]
    pub deleted_file: bool,
}

// =============================================================================
// Issue Types
// =============================================================================

/// Issue summary from list/get endpoints.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Issue {
    /// Global issue ID.
    pub id: u64,

    /// Project-scoped internal ID (use this for API calls).
    pub iid: u64,

    /// Project ID.
    pub project_id: u64,

    /// Issue title.
    pub title: String,

    /// Issue description/body.
    #[serde(default)]
    pub description: Option<String>,

    /// Issue state: "opened", "closed".
    pub state: String,

    /// Creation timestamp (ISO 8601).
    pub created_at: String,

    /// Last update timestamp (ISO 8601).
    pub updated_at: String,

    /// Close timestamp (ISO 8601, if closed).
    #[serde(default)]
    pub closed_at: Option<String>,

    /// User who closed the issue.
    #[serde(default)]
    pub closed_by: Option<User>,

    /// Labels on the issue.
    #[serde(default)]
    pub labels: Vec<String>,

    /// Associated milestone.
    #[serde(default)]
    pub milestone: Option<Milestone>,

    /// Assigned users.
    #[serde(default)]
    pub assignees: Vec<User>,

    /// Issue author.
    pub author: User,

    /// Single assigned user (legacy).
    #[serde(default)]
    pub assignee: Option<User>,

    /// Number of user comments/notes.
    #[serde(default)]
    pub user_notes_count: u64,

    /// Upvotes count.
    #[serde(default)]
    pub upvotes: u64,

    /// Downvotes count.
    #[serde(default)]
    pub downvotes: u64,

    /// Due date (YYYY-MM-DD).
    #[serde(default)]
    pub due_date: Option<String>,

    /// Whether this issue is confidential.
    #[serde(default)]
    pub confidential: bool,

    /// Whether discussion is locked.
    #[serde(default)]
    pub discussion_locked: Option<bool>,

    /// Issue type: "issue", "incident", "test_case".
    #[serde(default)]
    pub issue_type: Option<String>,

    /// HTML URL to the issue.
    #[serde(default)]
    pub web_url: Option<String>,

    /// Time tracking statistics.
    #[serde(default)]
    pub time_stats: Option<TimeStats>,

    /// Task completion status.
    #[serde(default)]
    pub task_completion_status: Option<TaskCompletionStatus>,

    /// Issue weight (for weighted issues).
    #[serde(default)]
    pub weight: Option<u64>,

    /// Whether the issue has tasks.
    #[serde(default)]
    pub has_tasks: Option<bool>,

    /// Task status string.
    #[serde(default)]
    pub task_status: Option<String>,

    /// Reference information.
    #[serde(default)]
    pub references: Option<References>,

    /// Severity for incidents.
    #[serde(default)]
    pub severity: Option<String>,

    /// ID of issue this was moved to.
    #[serde(default)]
    pub moved_to_id: Option<u64>,
}

/// A comment/note on an issue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Note {
    /// Note ID.
    pub id: u64,

    /// Note type (e.g., "DiffNote", "DiscussionNote").
    #[serde(rename = "type", default)]
    pub note_type: Option<String>,

    /// Note body/content.
    pub body: String,

    /// Attachment URL (if any).
    #[serde(default)]
    pub attachment: Option<String>,

    /// Note author.
    pub author: User,

    /// Creation timestamp (ISO 8601).
    pub created_at: String,

    /// Last update timestamp (ISO 8601).
    pub updated_at: String,

    /// Whether this is a system-generated note.
    #[serde(default)]
    pub system: bool,

    /// ID of the noteable (issue/MR).
    #[serde(default)]
    pub noteable_id: Option<u64>,

    /// Type of noteable ("Issue", "MergeRequest").
    #[serde(default)]
    pub noteable_type: Option<String>,

    /// Whether this note is resolvable.
    #[serde(default)]
    pub resolvable: bool,

    /// Whether this note is resolved.
    #[serde(default)]
    pub resolved: Option<bool>,

    /// User who resolved this note.
    #[serde(default)]
    pub resolved_by: Option<User>,

    /// Whether this is a confidential note.
    #[serde(default)]
    pub confidential: bool,

    /// Whether this is an internal note.
    #[serde(default)]
    pub internal: bool,
}

// =============================================================================
// Time Tracking Types
// =============================================================================

/// Time tracking statistics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeStats {
    /// Estimated time in seconds.
    #[serde(default)]
    pub time_estimate: u64,

    /// Total time spent in seconds.
    #[serde(default)]
    pub total_time_spent: u64,

    /// Human-readable time estimate.
    #[serde(default)]
    pub human_time_estimate: Option<String>,

    /// Human-readable total time spent.
    #[serde(default)]
    pub human_total_time_spent: Option<String>,
}

/// Task completion status for issues.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskCompletionStatus {
    /// Total number of tasks.
    #[serde(default)]
    pub count: u64,

    /// Number of completed tasks.
    #[serde(default)]
    pub completed_count: u64,
}

// =============================================================================
// Milestone Type
// =============================================================================

/// A project milestone.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Milestone {
    /// Milestone ID.
    pub id: u64,

    /// Project-scoped internal ID.
    pub iid: u64,

    /// Project ID.
    #[serde(default)]
    pub project_id: Option<u64>,

    /// Milestone title.
    pub title: String,

    /// Milestone description.
    #[serde(default)]
    pub description: Option<String>,

    /// Milestone state: "active", "closed".
    pub state: String,

    /// Creation timestamp (ISO 8601).
    #[serde(default)]
    pub created_at: Option<String>,

    /// Last update timestamp (ISO 8601).
    #[serde(default)]
    pub updated_at: Option<String>,

    /// Due date (YYYY-MM-DD).
    #[serde(default)]
    pub due_date: Option<String>,

    /// Start date (YYYY-MM-DD).
    #[serde(default)]
    pub start_date: Option<String>,

    /// Whether the milestone is expired.
    #[serde(default)]
    pub expired: Option<bool>,

    /// HTML URL to the milestone.
    #[serde(default)]
    pub web_url: Option<String>,
}

/// Reference information (short, relative, full).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct References {
    /// Short reference (e.g., "#1").
    pub short: String,

    /// Relative reference (e.g., "group/project#1").
    pub relative: String,

    /// Full reference (e.g., "https://gitlab.com/group/project/-/issues/1").
    pub full: String,
}

// =============================================================================
// Tag and Release Types
// =============================================================================

/// A repository tag from `GET /projects/:id/repository/tags`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tag {
    /// Tag name.
    pub name: String,

    /// Tag message (for annotated tags).
    #[serde(default)]
    pub message: Option<String>,

    /// Target commit SHA.
    pub target: String,

    /// Commit the tag points to.
    pub commit: TagCommit,

    /// Associated release info (if this is a release tag).
    #[serde(default)]
    pub release: Option<TagRelease>,

    /// Whether the tag is protected.
    #[serde(default)]
    pub protected: bool,

    /// Creation timestamp (ISO 8601).
    #[serde(default)]
    pub created_at: Option<String>,
}

/// Commit info embedded in a tag.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TagCommit {
    /// Commit SHA.
    pub id: String,

    /// Short commit SHA.
    pub short_id: String,

    /// Commit title.
    pub title: String,

    /// Creation timestamp.
    #[serde(default)]
    pub created_at: Option<String>,

    /// Parent commit SHAs.
    #[serde(default)]
    pub parent_ids: Vec<String>,

    /// Full commit message.
    #[serde(default)]
    pub message: Option<String>,

    /// Author name.
    #[serde(default)]
    pub author_name: Option<String>,

    /// Author email.
    #[serde(default)]
    pub author_email: Option<String>,

    /// Authored timestamp (ISO 8601).
    #[serde(default)]
    pub authored_date: Option<String>,

    /// Committer name.
    #[serde(default)]
    pub committer_name: Option<String>,

    /// Committer email.
    #[serde(default)]
    pub committer_email: Option<String>,

    /// Committed timestamp (ISO 8601).
    #[serde(default)]
    pub committed_date: Option<String>,
}

/// Brief release info embedded in a tag.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TagRelease {
    /// Tag name associated with the release.
    pub tag_name: String,

    /// Release description/notes.
    #[serde(default)]
    pub description: Option<String>,
}

/// A full release from `GET /projects/:id/releases`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Release {
    /// Release name/title.
    pub name: String,

    /// Tag name associated with the release.
    pub tag_name: String,

    /// Release description (markdown).
    #[serde(default)]
    pub description: Option<String>,

    /// Release description (rendered HTML).
    #[serde(default)]
    pub description_html: Option<String>,

    /// Creation timestamp (ISO 8601).
    pub created_at: String,

    /// Release timestamp (ISO 8601).
    pub released_at: String,

    /// Release author.
    pub author: User,

    /// Commit the release points to.
    pub commit: ReleaseCommit,

    /// Associated milestones.
    #[serde(default)]
    pub milestones: Vec<Milestone>,

    /// Path to the commit.
    #[serde(default)]
    pub commit_path: Option<String>,

    /// Path to the tag.
    #[serde(default)]
    pub tag_path: Option<String>,

    /// Release assets.
    pub assets: ReleaseAssets,

    /// Release evidences.
    #[serde(default)]
    pub evidences: Vec<ReleaseEvidence>,

    /// Related links.
    #[serde(rename = "_links", default)]
    pub links: Option<ReleaseLinks>,

    /// Whether this is an upcoming release.
    #[serde(default)]
    pub upcoming_release: Option<bool>,
}

/// Commit info embedded in a release.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseCommit {
    /// Commit SHA.
    pub id: String,

    /// Short commit SHA.
    pub short_id: String,

    /// Commit title.
    pub title: String,

    /// Creation timestamp (ISO 8601).
    pub created_at: String,

    /// Parent commit SHAs.
    #[serde(default)]
    pub parent_ids: Vec<String>,

    /// Full commit message.
    #[serde(default)]
    pub message: Option<String>,

    /// Author name.
    #[serde(default)]
    pub author_name: Option<String>,

    /// Author email.
    #[serde(default)]
    pub author_email: Option<String>,

    /// Authored timestamp (ISO 8601).
    #[serde(default)]
    pub authored_date: Option<String>,

    /// Committer name.
    #[serde(default)]
    pub committer_name: Option<String>,

    /// Committer email.
    #[serde(default)]
    pub committer_email: Option<String>,

    /// Committed timestamp (ISO 8601).
    #[serde(default)]
    pub committed_date: Option<String>,
}

/// Release assets (sources and links).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseAssets {
    /// Number of assets.
    #[serde(default)]
    pub count: u64,

    /// Source archive downloads.
    #[serde(default)]
    pub sources: Vec<ReleaseSource>,

    /// Custom asset links.
    #[serde(default)]
    pub links: Vec<ReleaseLink>,
}

/// A source archive for a release.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseSource {
    /// Archive format (e.g., "zip", "tar.gz").
    pub format: String,

    /// Download URL.
    pub url: String,
}

/// A custom asset link in a release.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseLink {
    /// Link ID.
    pub id: u64,

    /// Link name.
    pub name: String,

    /// Link URL.
    pub url: String,

    /// Whether this is an external link.
    #[serde(default)]
    pub external: bool,

    /// Link type: "other", "runbook", "image", "package".
    #[serde(default)]
    pub link_type: Option<String>,
}

/// Release evidence for compliance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseEvidence {
    /// Evidence SHA.
    pub sha: String,

    /// Evidence file path.
    pub filepath: String,

    /// Collection timestamp (ISO 8601).
    pub collected_at: String,
}

/// Related links in a release.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReleaseLinks {
    /// Self URL.
    #[serde(rename = "self")]
    pub self_url: String,

    /// Edit URL.
    #[serde(default)]
    pub edit_url: Option<String>,

    /// URL to closed issues.
    #[serde(default)]
    pub closed_issues_url: Option<String>,

    /// URL to closed merge requests.
    #[serde(default)]
    pub closed_merge_requests_url: Option<String>,

    /// URL to merged merge requests.
    #[serde(default)]
    pub merged_merge_requests_url: Option<String>,

    /// URL to opened issues.
    #[serde(default)]
    pub opened_issues_url: Option<String>,

    /// URL to opened merge requests.
    #[serde(default)]
    pub opened_merge_requests_url: Option<String>,
}

// =============================================================================
// Project Types
// =============================================================================

/// A GitLab namespace (group or user).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Namespace {
    /// Namespace ID.
    pub id: u64,

    /// Namespace name.
    #[serde(default)]
    pub name: Option<String>,

    /// URL-safe path.
    #[serde(default)]
    pub path: Option<String>,

    /// Namespace kind: "user" or "group".
    #[serde(default)]
    pub kind: Option<String>,

    /// Full path (including parent groups).
    #[serde(default)]
    pub full_path: Option<String>,

    /// Parent namespace ID.
    #[serde(default)]
    pub parent_id: Option<u64>,

    /// URL to the namespace's avatar image.
    #[serde(default)]
    pub avatar_url: Option<String>,

    /// HTML URL to the namespace.
    #[serde(default)]
    pub web_url: Option<String>,
}

/// Project metadata from `GET /projects/{id}`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Project {
    /// Project ID.
    pub id: u64,

    /// Project name.
    pub name: String,

    /// URL-safe path.
    #[serde(default)]
    pub path: Option<String>,

    /// Full path with namespace (e.g., "group/subgroup/project").
    #[serde(default)]
    pub path_with_namespace: Option<String>,

    /// Project description.
    #[serde(default)]
    pub description: Option<String>,

    /// Default branch name.
    #[serde(default)]
    pub default_branch: Option<String>,

    /// Visibility level: "private", "internal", "public".
    #[serde(default)]
    pub visibility: Option<String>,

    /// HTML URL to the project.
    #[serde(default)]
    pub web_url: Option<String>,

    /// SSH URL for cloning.
    #[serde(default)]
    pub ssh_url_to_repo: Option<String>,

    /// HTTPS URL for cloning.
    #[serde(default)]
    pub http_url_to_repo: Option<String>,

    /// Namespace the project belongs to.
    #[serde(default)]
    pub namespace: Option<Namespace>,

    /// Star count.
    #[serde(default)]
    pub star_count: Option<u64>,

    /// Fork count.
    #[serde(default)]
    pub forks_count: Option<u64>,

    /// Number of open issues.
    #[serde(default)]
    pub open_issues_count: Option<u64>,

    /// Whether the repository is empty.
    #[serde(default)]
    pub empty_repo: Option<bool>,

    /// Whether the project is archived.
    #[serde(default)]
    pub archived: bool,

    /// Creation timestamp (ISO 8601).
    #[serde(default)]
    pub created_at: Option<String>,

    /// Last activity timestamp (ISO 8601).
    #[serde(default)]
    pub last_activity_at: Option<String>,

    /// Project owner.
    #[serde(default)]
    pub owner: Option<User>,

    /// Topics/tags on the project.
    #[serde(default)]
    pub topics: Vec<String>,

    /// Whether issues are enabled.
    #[serde(default)]
    pub issues_enabled: Option<bool>,

    /// Whether merge requests are enabled.
    #[serde(default)]
    pub merge_requests_enabled: Option<bool>,

    /// Whether the wiki is enabled.
    #[serde(default)]
    pub wiki_enabled: Option<bool>,

    /// URL to the project's avatar image.
    #[serde(default)]
    pub avatar_url: Option<String>,
}

// =============================================================================
// Pipeline Types
// =============================================================================

/// A CI/CD pipeline from `GET /projects/{id}/pipelines`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pipeline {
    /// Pipeline ID.
    pub id: u64,

    /// Project ID.
    #[serde(default)]
    pub project_id: Option<u64>,

    /// Pipeline status (e.g., "created", "pending", "running", "success", "failed", "canceled", "skipped", "manual").
    #[serde(default)]
    pub status: Option<String>,

    /// Pipeline source (e.g., "push", "merge_request_event", "schedule", "api", "web").
    #[serde(default)]
    pub source: Option<String>,

    /// Git ref (branch or tag name).
    #[serde(rename = "ref", default)]
    pub ref_name: Option<String>,

    /// Commit SHA.
    #[serde(default)]
    pub sha: Option<String>,

    /// HTML URL to the pipeline.
    #[serde(default)]
    pub web_url: Option<String>,

    /// Creation timestamp (ISO 8601).
    #[serde(default)]
    pub created_at: Option<String>,

    /// Last update timestamp (ISO 8601).
    #[serde(default)]
    pub updated_at: Option<String>,

    /// Pipeline name.
    #[serde(default)]
    pub name: Option<String>,
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
            "id": 1,
            "username": "admin",
            "name": "Administrator",
            "state": "active",
            "avatar_url": "https://gitlab.example.com/uploads/-/system/user/avatar/1/avatar.png",
            "web_url": "https://gitlab.example.com/admin"
        }"#;

        let user: User = serde_json::from_str(json).unwrap();
        assert_eq!(user.id, 1);
        assert_eq!(user.username, "admin");
        assert_eq!(user.name, Some("Administrator".to_string()));
    }

    #[test]
    fn tree_item_deserialization() {
        let json = r#"{
            "id": "abc123",
            "name": "README.md",
            "type": "blob",
            "path": "README.md",
            "mode": "100644"
        }"#;

        let item: TreeItem = serde_json::from_str(json).unwrap();
        assert_eq!(item.name, "README.md");
        assert_eq!(item.item_type, "blob");
        assert_eq!(item.mode, Some("100644".to_string()));
    }

    #[test]
    fn file_content_deserialization() {
        let json = r#"{
            "file_name": "README.md",
            "file_path": "README.md",
            "size": 1234,
            "encoding": "base64",
            "content": "SGVsbG8gV29ybGQh",
            "ref": "main",
            "blob_id": "abc123",
            "commit_id": "def456"
        }"#;

        let file: FileContent = serde_json::from_str(json).unwrap();
        assert_eq!(file.file_name, "README.md");
        assert_eq!(file.encoding, "base64");
        assert_eq!(file.ref_name, "main");
    }

    #[test]
    fn merge_request_minimal_deserialization() {
        let json = r#"{
            "id": 1,
            "iid": 1,
            "project_id": 3,
            "title": "Test MR",
            "state": "opened",
            "created_at": "2024-01-01T00:00:00Z",
            "updated_at": "2024-01-01T00:00:00Z",
            "target_branch": "main",
            "source_branch": "feature",
            "author": {"id": 1, "username": "admin"}
        }"#;

        let mr: MergeRequest = serde_json::from_str(json).unwrap();
        assert_eq!(mr.iid, 1);
        assert_eq!(mr.state, "opened");
        assert_eq!(mr.target_branch, "main");
        assert_eq!(mr.author.username, "admin");
    }

    #[test]
    fn commit_deserialization() {
        let json = r#"{
            "id": "abc123def456",
            "short_id": "abc123d",
            "created_at": "2024-01-01T00:00:00Z",
            "parent_ids": ["parent1", "parent2"],
            "title": "Initial commit",
            "author_name": "Admin",
            "author_email": "admin@example.com"
        }"#;

        let commit: Commit = serde_json::from_str(json).unwrap();
        assert_eq!(commit.short_id, "abc123d");
        assert_eq!(commit.parent_ids.len(), 2);
    }

    #[test]
    fn diff_deserialization() {
        let json = r#"{
            "old_path": "README.md",
            "new_path": "README.md",
            "a_mode": "100644",
            "b_mode": "100644",
            "diff": "--- a/README.md\n+++ b/README.md",
            "new_file": false,
            "renamed_file": false,
            "deleted_file": false
        }"#;

        let diff: Diff = serde_json::from_str(json).unwrap();
        assert_eq!(diff.old_path, "README.md");
        assert!(!diff.new_file);
    }

    #[test]
    fn issue_minimal_deserialization() {
        let json = r#"{
            "id": 1,
            "iid": 42,
            "project_id": 3,
            "title": "Bug report",
            "state": "opened",
            "created_at": "2024-01-01T00:00:00Z",
            "updated_at": "2024-01-01T00:00:00Z",
            "author": {"id": 1, "username": "admin"}
        }"#;

        let issue: Issue = serde_json::from_str(json).unwrap();
        assert_eq!(issue.iid, 42);
        assert_eq!(issue.state, "opened");
    }

    #[test]
    fn note_deserialization() {
        let json = r#"{
            "id": 1,
            "body": "This looks good!",
            "author": {"id": 1, "username": "admin"},
            "created_at": "2024-01-01T00:00:00Z",
            "updated_at": "2024-01-01T00:00:00Z",
            "system": false,
            "resolvable": false,
            "confidential": false,
            "internal": false
        }"#;

        let note: Note = serde_json::from_str(json).unwrap();
        assert_eq!(note.body, "This looks good!");
        assert!(!note.system);
    }

    #[test]
    fn time_stats_deserialization() {
        let json = r#"{
            "time_estimate": 3600,
            "total_time_spent": 1800,
            "human_time_estimate": "1h",
            "human_total_time_spent": "30m"
        }"#;

        let stats: TimeStats = serde_json::from_str(json).unwrap();
        assert_eq!(stats.time_estimate, 3600);
        assert_eq!(stats.human_time_estimate, Some("1h".to_string()));
    }

    #[test]
    fn tag_with_release_deserialization() {
        let json = r#"{
            "name": "v1.0.0",
            "message": "Release v1.0.0",
            "target": "abc123",
            "commit": {
                "id": "abc123",
                "short_id": "abc12",
                "title": "Version 1.0.0"
            },
            "release": {
                "tag_name": "v1.0.0",
                "description": "First release"
            },
            "protected": true
        }"#;

        let tag: Tag = serde_json::from_str(json).unwrap();
        assert_eq!(tag.name, "v1.0.0");
        assert!(tag.release.is_some());
        assert!(tag.protected);
    }

    #[test]
    fn tag_without_release_deserialization() {
        let json = r#"{
            "name": "v0.1.0",
            "target": "def456",
            "commit": {
                "id": "def456",
                "short_id": "def45",
                "title": "Initial"
            },
            "protected": false
        }"#;

        let tag: Tag = serde_json::from_str(json).unwrap();
        assert_eq!(tag.name, "v0.1.0");
        assert!(tag.release.is_none());
    }

    #[test]
    fn release_deserialization() {
        let json = r#"{
            "name": "v1.0.0",
            "tag_name": "v1.0.0",
            "description": "First stable release",
            "created_at": "2024-01-01T00:00:00Z",
            "released_at": "2024-01-02T00:00:00Z",
            "author": {"id": 1, "username": "admin"},
            "commit": {
                "id": "abc123",
                "short_id": "abc12",
                "title": "Version 1.0.0",
                "created_at": "2024-01-01T00:00:00Z"
            },
            "assets": {
                "count": 2,
                "sources": [
                    {"format": "zip", "url": "https://example.com/v1.0.0.zip"},
                    {"format": "tar.gz", "url": "https://example.com/v1.0.0.tar.gz"}
                ],
                "links": []
            }
        }"#;

        let release: Release = serde_json::from_str(json).unwrap();
        assert_eq!(release.tag_name, "v1.0.0");
        assert_eq!(release.assets.sources.len(), 2);
    }

    #[test]
    fn milestone_deserialization() {
        let json = r#"{
            "id": 1,
            "iid": 1,
            "project_id": 3,
            "title": "Q1 2024",
            "description": "Q1 Goals",
            "state": "active",
            "due_date": "2024-03-31"
        }"#;

        let milestone: Milestone = serde_json::from_str(json).unwrap();
        assert_eq!(milestone.title, "Q1 2024");
        assert_eq!(milestone.state, "active");
    }

    #[test]
    fn references_deserialization() {
        let json = r##"{
            "short": "#1",
            "relative": "group/project#1",
            "full": "https://gitlab.example.com/group/project/-/issues/1"
        }"##;

        let refs: References = serde_json::from_str(json).unwrap();
        assert_eq!(refs.short, "#1");
    }
}
