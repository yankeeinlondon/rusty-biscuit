use crate::Result;
use serde::{Deserialize, Serialize};
use std::path::Path;

pub mod docs;
pub mod file_types;
pub mod formatting;
pub mod git;
pub mod just;
pub mod languages;
pub mod repo;

pub use docs::{MarkdownMeta, RepoDocuments, detect_docs};
pub use file_types::{
    FileAssociation, FileAssociationBreakdown, FileAssociationStats, FileClassification,
    FileInventory, FrameworkKind, FrameworkStats, ProgrammingLanguage, ProgrammingLanguageStats,
    ProgrammingLanguageType,
};
pub use formatting::{EditorConfigSection, FormattingConfig, detect_formatting};
pub use git::{
    BehindStatus, CommitInfo, DeltaKind, GitHostingProvider, GitInfo, LocalBranchInfo, RemoteInfo,
    RepoStatus, detect_git, get_commit_by_sha, get_commit_files, get_commits_for_path,
};
pub use just::{JustRecipe, JustRecipeParam, JustfileInfo, detect_justfiles};
pub use languages::{LanguageBreakdown, LanguageStats, detect_languages};
pub use repo::{
    DependencyEntry, DependencyKind, MonorepoTool, Package, PackageDiscoverySource,
    PackageEcosystem, RepoInfo, detect_repo,
};

#[deprecated(note = "Use `Package` instead")]
pub type PackageLocation = Package;

/// Complete filesystem analysis for a directory.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FilesystemInfo {
    /// Programming language breakdown
    pub languages: Option<LanguageBreakdown>,
    /// Broad file-association breakdown
    #[serde(skip_serializing_if = "Option::is_none")]
    pub files: Option<FileAssociationBreakdown>,
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
    let git = detect_git(root, deep, commit_count)?;

    // Use git repo root (if available) for repo detection so that running
    // from a subdirectory still finds workspace markers at the repo root.
    let repo_root_path = git.as_ref().map(|g| g.repo_root.as_path()).unwrap_or(root);
    let repo = detect_repo(repo_root_path)?;
    let inventory = match repo.as_ref().and_then(|repo| repo.package_for_dir(root)) {
        Some(package) => {
            let exclude_roots = repo
                .as_ref()
                .and_then(|repo| repo.packages.as_ref())
                .map(|packages| {
                    packages
                        .iter()
                        .filter(|candidate| candidate.path != package.path)
                        .filter(|candidate| candidate.path.starts_with(&package.path))
                        .map(|candidate| candidate.path.clone())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();

            file_types::scan_file_inventory_with_exclusions(&package.path, &exclude_roots).ok()
        }
        None => file_types::scan_file_inventory(root).ok(),
    };
    let languages = inventory.as_ref().map(file_types::summarize_languages);
    let files = inventory.as_ref().map(file_types::summarize_file_inventory);

    let formatting = detect_formatting(root).ok().flatten();
    let docs = detect_docs(root);

    Ok(FilesystemInfo {
        languages,
        files,
        git,
        repo,
        formatting,
        docs,
    })
}
