use crate::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

pub mod docs;
pub mod formatting;
pub mod git;
pub mod languages;
pub mod repo;

pub use docs::{MarkdownMeta, RepoDocuments, detect_docs};
pub use formatting::{EditorConfigSection, FormattingConfig, detect_formatting};
pub use git::{
    BehindStatus, CommitInfo, GitInfo, HostingProvider, LocalBranchInfo, RemoteInfo, RepoStatus,
    detect_git,
};
pub use languages::{LanguageBreakdown, LanguageStats, detect_languages};
pub use repo::{DependencyEntry, DependencyKind, MonorepoTool, Package, RepoInfo, detect_repo};

#[deprecated(note = "Use `Package` instead")]
pub type PackageLocation = Package;

/// Complete filesystem analysis for a directory.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FilesystemInfo {
    /// Programming language breakdown
    pub languages: Option<LanguageBreakdown>,
    /// Git repository information
    pub git: Option<GitInfo>,
    /// Repository detection results (monorepo or single-package repo)
    pub repo: Option<RepoInfo>,
    /// EditorConfig formatting configuration
    pub formatting: Option<FormattingConfig>,
    /// Markdown documents in the repository
    #[serde(skip_serializing_if = "Option::is_none")]
    pub docs: Option<Vec<MarkdownMeta>>,
}

/// Detect all filesystem information for a directory.
///
/// ## Arguments
///
/// * `root` - The root directory to analyze
/// * `deep` - Enable network operations for enhanced git info
/// * `commit_count` - Number of recent commits to retrieve
pub fn detect_filesystem(root: &Path, deep: bool, commit_count: usize) -> Result<FilesystemInfo> {
    let languages = detect_languages(root).ok();
    let git = detect_git(root, deep, commit_count)?;
    // Use git repo root (if available) for repo detection so that running
    // from a subdirectory still finds workspace markers at the repo root.
    let repo_root_path = git.as_ref().map(|g| g.repo_root.as_path()).unwrap_or(root);
    let repo = detect_repo(repo_root_path)?;
    let formatting = detect_formatting(root).ok().flatten();
    let docs = detect_docs(root);

    Ok(FilesystemInfo {
        languages,
        git,
        repo,
        formatting,
        docs,
    })
}
