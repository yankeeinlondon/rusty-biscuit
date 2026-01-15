use crate::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

pub mod formatting;
pub mod git;
pub mod languages;
pub mod repo;

pub use formatting::{EditorConfigSection, FormattingConfig, detect_formatting};
pub use git::{
    BehindStatus, CommitInfo, GitInfo, HostingProvider, RemoteInfo, RepoStatus, detect_git,
};
pub use languages::{LanguageBreakdown, LanguageStats, detect_languages};
pub use repo::{
    DependencyEntry, DependencyKind, MonorepoTool, PackageLocation, RepoInfo, detect_repo,
};

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
}

/// Detect all filesystem information for a directory.
pub fn detect_filesystem(root: &Path, deep: bool) -> Result<FilesystemInfo> {
    let languages = detect_languages(root).ok();
    let git = detect_git(root, deep)?;
    let repo = detect_repo(root)?;
    let formatting = detect_formatting(root).ok().flatten();

    Ok(FilesystemInfo {
        languages,
        git,
        repo,
        formatting,
    })
}
