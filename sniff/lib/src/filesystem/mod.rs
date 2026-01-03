use serde::{Deserialize, Serialize};
use std::path::Path;
use crate::Result;

pub mod languages;
pub mod git;
pub mod monorepo;
pub mod dependencies;

pub use languages::{LanguageBreakdown, LanguageStats, detect_languages};
pub use git::{GitInfo, RepoStatus, RemoteInfo, HostingProvider, CommitInfo, detect_git};
pub use monorepo::{MonorepoInfo, MonorepoTool, PackageLocation, detect_monorepo};
pub use dependencies::{PackageManager, DependencyReport, detect_dependencies};

/// Complete filesystem analysis for a directory.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FilesystemInfo {
    /// Programming language breakdown
    pub languages: Option<LanguageBreakdown>,
    /// Git repository information
    pub git: Option<GitInfo>,
    /// Monorepo detection results
    pub monorepo: Option<MonorepoInfo>,
    /// Dependency analysis
    pub dependencies: Option<DependencyReport>,
}

/// Detect all filesystem information for a directory.
pub fn detect_filesystem(root: &Path) -> Result<FilesystemInfo> {
    let languages = detect_languages(root).ok();
    let git = detect_git(root)?;
    let monorepo = detect_monorepo(root)?;
    let dependencies = detect_dependencies(root).ok();

    Ok(FilesystemInfo {
        languages,
        git,
        monorepo,
        dependencies,
    })
}
