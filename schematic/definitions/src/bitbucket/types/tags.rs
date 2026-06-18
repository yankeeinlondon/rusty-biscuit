use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::common::Link;
use super::common::User;

/// A repository tag from `GET /repositories/{workspace}/{repo_slug}/refs/tags`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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
        self.raw
            .as_deref()
            .and_then(|r| r.split('<').next())
            .map(|s| s.trim())
    }

    /// Extracts the email from the raw author string.
    pub fn email(&self) -> Option<&str> {
        self.raw.as_deref().and_then(|r| {
            r.find('<')
                .and_then(|start| r.find('>').map(|end| &r[start + 1..end]))
        })
    }
}

/// Tagger information for annotated tags.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct Tagger {
    /// Raw tagger string (e.g., "Name <email>").
    #[serde(default)]
    pub raw: Option<String>,

    /// Linked user account (if any).
    #[serde(default)]
    pub user: Option<User>,
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert_eq!(
            tag.target.as_ref().unwrap().hash,
            Some("abc123def456".to_string())
        );
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
}
