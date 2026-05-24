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

pub mod common;
pub mod issues;
pub mod merge_requests;
pub mod pipelines;
pub mod projects;
pub mod releases;

pub use common::*;
pub use issues::*;
pub use merge_requests::*;
pub use pipelines::*;
pub use projects::*;
pub use releases::*;

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
