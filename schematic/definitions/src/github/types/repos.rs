use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::common::UserSummary;

/// Repository information from `GET /repos/{owner}/{repo}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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

/// Response from `GET /repos/{owner}/{repo}/git/trees/{tree_sha}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
