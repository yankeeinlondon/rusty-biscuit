use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::common::Link;
use super::common::User;

/// A download artifact from `GET /repositories/{workspace}/{repo_slug}/downloads`.
///
/// Downloads serve as release artifacts in Bitbucket (there's no first-class release concept).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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
        self.links
            .as_ref()
            .and_then(|l| l.get("self"))
            .map(|link| link.href.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
