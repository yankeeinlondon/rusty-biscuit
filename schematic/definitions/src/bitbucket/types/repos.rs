use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::common::{Link, User};

/// Repository information from `GET /repositories/{workspace}/{repo_slug}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct BranchInfo {
    /// Branch name.
    #[serde(default)]
    pub name: Option<String>,

    /// Object type (e.g., "branch").
    #[serde(rename = "type", default)]
    pub branch_type: Option<String>,
}

/// Workspace information.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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

/// A source entry from directory listing.
///
/// Returned by `GET /repositories/{workspace}/{repo_slug}/src/{commit}/{path}`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
