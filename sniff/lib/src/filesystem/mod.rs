use crate::Result;
use crate::performance;
use crate::request::{FilesystemRequest, GitRequest};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Instant;
use tracing::Level;
use tracing::instrument;

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
    BehindStatus, CommitDesc, CommitDescSet, CommitInfo, DeltaKind, GitHostingProvider, GitInfo,
    GitRepo, LocalBranchInfo, PeriodSpecifier, RemoteInfo, RepoStatus, detect_git,
    detect_git_with_request, detect_merge_conflicts, get_commit_by_sha, get_commit_files,
    get_commits_for_path, get_recent_commits_by_date, get_recent_commits_by_duration,
    get_recent_commits_by_hash, get_recent_commits_in_range, parse_period,
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
#[instrument(skip(request), fields(
    git = request.git.is_some(),
    repo = request.repo.is_some(),
    files = request.include_file_inventory,
    docs = request.include_docs,
))]
pub fn detect_filesystem_with_request(
    root: &Path,
    request: &FilesystemRequest,
) -> Result<FilesystemInfo> {
    // Stage 1: Git detection
    let git_started = Instant::now();
    let git = match &request.git {
        Some(git_request) => git::detect_git_with_request(root, git_request)?,
        None => None,
    };
    performance::record_logged_stage("filesystem.git", git_started.elapsed(), Level::DEBUG);

    // Stage 2: Repo detection
    // When full repo detection is requested, also capture the shared file
    // inventory it already builds so Stage 3 can reuse it instead of
    // rescanning the tree.
    let repo_root_path = git.as_ref().map(|g| g.repo_root.as_path()).unwrap_or(root);
    let repo_started = Instant::now();
    let (repo, repo_inventory) = match &request.repo {
        Some(repo_request) => {
            if repo_request.structure_only {
                (detect_repo_structure(repo_root_path)?, None)
            } else {
                repo::detect_repo_with_inventory(repo_root_path)?
            }
        }
        None => (None, None),
    };
    performance::record_logged_stage("filesystem.repo", repo_started.elapsed(), Level::DEBUG);

    // Stage 3: File inventory and language breakdown
    // When full repo detection already scanned the tree, reuse that
    // inventory (filtered to the target scope) instead of walking again.
    let inventory_started = Instant::now();
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
                match repo_inventory {
                    Some(ref inv) => Some(filter_inventory(inv, &package.path, &exclude_roots)),
                    None => file_types::scan_file_inventory_with_exclusions(
                        &package.path,
                        &exclude_roots,
                    )
                    .ok(),
                }
            }
            None => match repo_inventory {
                Some(ref inv) if inv.scope.root == root => Some(inv.clone()),
                _ => file_types::scan_file_inventory(root).ok(),
            },
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
    performance::record_logged_stage(
        "filesystem.inventory",
        inventory_started.elapsed(),
        Level::DEBUG,
    );

    // Stage 4: Formatting
    let formatting_started = Instant::now();
    let formatting = if request.include_formatting {
        detect_formatting(root).ok().flatten()
    } else {
        None
    };
    performance::record_logged_stage(
        "filesystem.formatting",
        formatting_started.elapsed(),
        Level::DEBUG,
    );

    // Stage 5: Docs
    let docs_started = Instant::now();
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
    performance::record_logged_stage("filesystem.docs", docs_started.elapsed(), Level::DEBUG);

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

/// Creates a filtered subset of a repo-wide inventory scoped to `target_root`,
/// excluding files under any of `exclude_roots`.
///
/// Inventory paths are relative to the source scan root, so all comparisons
/// use the relative prefix of `target_root` within that root.
fn filter_inventory(
    source: &file_types::FileInventory,
    target_root: &Path,
    exclude_roots: &[PathBuf],
) -> file_types::FileInventory {
    let source_root = &source.scope.root;

    // Convert absolute target/exclude paths to relative prefixes within the inventory
    let target_prefix = target_root
        .strip_prefix(source_root)
        .unwrap_or(Path::new(""));
    let exclude_prefixes: Vec<&Path> = exclude_roots
        .iter()
        .filter_map(|ex| ex.strip_prefix(source_root).ok())
        .collect();

    let classifications: Vec<_> = source
        .classifications
        .iter()
        .filter(|c| {
            if target_prefix == Path::new("") {
                true
            } else {
                c.path.starts_with(target_prefix)
            }
        })
        .filter(|c| !exclude_prefixes.iter().any(|ex| c.path.starts_with(ex)))
        .cloned()
        .collect();
    let total = classifications.len();
    file_types::FileInventory {
        scope: file_types::FileScanScope {
            root: target_root.to_path_buf(),
            exclude_roots: exclude_roots.to_vec(),
        },
        total_files_scanned: total,
        classifications,
    }
}
