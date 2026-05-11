use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::common::UserSummary;

/// Issue summary from list/get endpoints.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
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

#[cfg(test)]
mod tests {
    use super::*;

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
