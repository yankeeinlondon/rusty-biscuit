use crate::Result;
use crate::request::{FilesystemRequest, GitRequest};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

pub mod blast_radius;
pub mod docs;
pub mod file_types;
pub mod formatting;
pub mod git;
pub mod just;
pub mod languages;
pub mod repo;

pub use docs::{MarkdownMeta, RepoDocuments, TitleSource, UpdatedSource, detect_docs};
pub use file_types::{
    FileAssociation, FileAssociationBreakdown, FileAssociationStats, FileClassification,
    FileInventory, FrameworkKind, FrameworkStats, ProgrammingLanguage, ProgrammingLanguageStats,
    ProgrammingLanguageType,
};
pub use formatting::{EditorConfigSection, FormattingConfig, detect_formatting};
pub use git::{
    BehindStatus, CommitInfo, DeltaKind, GitHostingProvider, GitInfo, GitRepo, LocalBranchInfo,
    RemoteInfo, RepoStatus, detect_git, detect_git_with_request, get_commit_by_sha,
    get_commit_files, get_commits_for_path,
};
pub use just::{JustRecipe, JustRecipeParam, JustfileInfo, detect_justfiles};
pub use languages::{LanguageBreakdown, LanguageStats, detect_languages};
pub use repo::{
    DependencyEntry, DependencyKind, MonorepoTool, Package, PackageDiscoverySource,
    PackageEcosystem, RepoInfo, detect_repo, detect_repo_structure,
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

/// Detect filesystem information according to the given request.
///
/// Controls which subsections are collected: git, repo, file inventory,
/// formatting, and document discovery.
pub fn detect_filesystem_with_request(
    root: &Path,
    request: &FilesystemRequest,
) -> Result<FilesystemInfo> {
    // Stage 1: Git detection
    let git = match &request.git {
        Some(git_request) => git::detect_git_with_request(root, git_request)?,
        None => None,
    };

    // Stage 2: Repo detection
    let repo_root_path = git.as_ref().map(|g| g.repo_root.as_path()).unwrap_or(root);
    let repo = match &request.repo {
        Some(repo_request) => {
            if repo_request.structure_only {
                detect_repo_structure(repo_root_path)?
            } else {
                detect_repo(repo_root_path)?
            }
        }
        None => None,
    };

    // Stage 3: File inventory and language breakdown
    let (files, languages) = if request.include_file_inventory {
        let inventory = match repo.as_ref().and_then(|r| r.package_for_dir(root)) {
            Some(package) => {
                let exclude_roots = repo
                    .as_ref()
                    .and_then(|r| r.packages.as_ref())
                    .map(|packages| {
                        packages
                            .iter()
                            .filter(|c| c.path != package.path)
                            .filter(|c| c.path.starts_with(&package.path))
                            .map(|c| c.path.clone())
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                file_types::scan_file_inventory_with_exclusions(&package.path, &exclude_roots).ok()
            }
            None => file_types::scan_file_inventory(root).ok(),
        };

        match inventory {
            Some(inv) => {
                let (fab, lang_summary) = file_types::summarize_file_inventory(&inv);
                (Some(fab), Some(lang_summary))
            }
            None => (None, None),
        }
    } else {
        (None, None)
    };

    // Stage 4: Formatting
    let formatting = if request.include_formatting {
        detect_formatting(root).ok().flatten()
    } else {
        None
    };

    // Stage 5: Docs
    let docs = if request.include_docs {
        match (
            git.as_ref(),
            repo.as_ref().and_then(|r| r.packages.as_ref()),
        ) {
            (Some(git_info), Some(packages)) => {
                let pkg_tuples: Vec<(String, PathBuf)> = packages
                    .iter()
                    .map(|p| (p.name.clone(), PathBuf::from(&p.relative)))
                    .collect();
                docs::detect_docs_with_packages(&git_info.repo_root, &pkg_tuples)
            }
            _ => detect_docs(root),
        }
    } else {
        None
    };

    Ok(FilesystemInfo {
        languages,
        files,
        git,
        repo,
        formatting,
        docs,
    })
}

/// Detect all filesystem information for a directory.
///
/// ## Arguments
///
/// * `root` - The root directory to analyze
/// * `deep` - Enable network operations for enhanced git info
/// * `commit_count` - Number of recent commits to retrieve
pub fn detect_filesystem(root: &Path, deep: bool, commit_count: usize) -> Result<FilesystemInfo> {
    let git_request = if deep {
        GitRequest::deep().commit_count(commit_count)
    } else {
        GitRequest::full().commit_count(commit_count)
    };
    detect_filesystem_with_request(root, &FilesystemRequest::new().git(git_request))
}
