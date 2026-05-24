use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::common::{Content, Link, User};
use super::pull_requests::RepositoryRef;

/// An issue from `GET /repositories/{workspace}/{repo_slug}/issues`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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

/// Milestone reference.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ChangeDetail {
    /// Old value.
    #[serde(default)]
    pub old: Option<String>,

    /// New value.
    #[serde(default)]
    pub new: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
