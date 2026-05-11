use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A Bitbucket user summary (common across many responses).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Link {
    /// The URL this link points to.
    pub href: String,
}

/// A paginated response wrapper.
///
/// Bitbucket uses cursor-based pagination. Follow the `next` URL to get more results.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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

/// Content with raw, markup, and HTML forms.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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
    fn empty_json_deserializes_cleanly() {
        use super::super::{Repository, PullRequest, Issue, Tag, Download, SourceEntry};
        let _user: User = serde_json::from_str("{}").unwrap();
        let _repo: Repository = serde_json::from_str("{}").unwrap();
        let _pr: PullRequest = serde_json::from_str("{}").unwrap();
        let _issue: Issue = serde_json::from_str("{}").unwrap();
        let _tag: Tag = serde_json::from_str("{}").unwrap();
        let _download: Download = serde_json::from_str("{}").unwrap();
        let _source: SourceEntry = serde_json::from_str("{}").unwrap();
    }
}
