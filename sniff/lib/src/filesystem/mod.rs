use crate::Result;
use crate::performance;
use crate::request::{FilesystemRequest, GitRequest};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
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
pub mod path_kind;
pub mod repo;
mod system_view;

pub use docs::{
    MarkdownMeta, RepoDocuments, TitleSource, UpdatedSource, collect_markdown_paths, detect_docs,
};
pub use file_types::{
    FileAssociation, FileAssociationBreakdown, FileAssociationStats, FileClassification,
    FileInventory, FrameworkKind, FrameworkStats, ProgrammingLanguage, ProgrammingLanguageStats,
    ProgrammingLanguageType,
};
pub use formatting::{EditorConfigSection, FormattingConfig, detect_formatting};
pub use git::{
    BehindStatus, CommitDesc, CommitDescSet, CommitInfo, DeltaKind, GitHostingProvider, GitInfo,
    GitRepo, LocalBranchInfo, PeriodSpecifier, RemoteInfo, RepoStatus, commit_browser_url,
    commit_by_sha_at, commit_files_at, commits_for_branch_at, commits_for_path_at, detect_git,
    detect_git_with_request, detect_merge_conflicts, get_commit_by_sha, get_commit_files,
    get_commits_for_branch, get_commits_for_path, get_recent_commits_by_count,
    get_recent_commits_by_date, get_recent_commits_by_duration, get_recent_commits_by_hash,
    get_recent_commits_in_range, merge_conflicts_at, merge_conflicts_with_branch_at, parse_period,
    preferred_remote_url, remote_url, repo_root,
};
pub use just::{JustRecipe, JustRecipeParam, JustfileInfo, detect_justfiles};
pub use languages::{LanguageBreakdown, LanguageStats, detect_languages};
pub use repo::{
    DependencyEntry, DependencyKind, DetectedStandard, DetectionConfidence, MonorepoLayer,
    MonorepoStandard, MonorepoStandardSpec, Package, PackageEcosystem, PackageProvenance, RepoInfo,
    detect_repo, detect_repo_structure,
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
    let collector = performance::current_collector();
    let need_repo_context = request.repo.is_some() || request.include_docs;
    let need_repo_full = request
        .repo
        .as_ref()
        .is_some_and(|repo| !repo.structure_only);
    let need_shared_view = need_repo_full
        || request.include_file_inventory
        || request.include_docs
        || request.include_formatting;

    // Discover the repository once up front when git detection is requested,
    // then thread that single handle into the git stage and reuse its work
    // directory as the shared-walk root. This removes the redundant second
    // discovery (parent walk + repo open) the git stage used to perform on
    // every invocation that renders git alongside repo/docs/inventory. When git
    // detection is not requested, the provided root is always the walk root.
    let discovered_git = match request.git.as_ref() {
        Some(_) => GitRepo::discover(root)?,
        None => None,
    };
    let shared_root = match &discovered_git {
        // A bare repository's `repo_root` is its git directory. Walking that as
        // a source tree would inventory git internals, so keep the caller's root.
        Some(handle) if !handle.is_bare() => handle.repo_root().to_path_buf(),
        _ => root.to_path_buf(),
    };

    std::thread::scope(|scope| {
        let git_handle = request.git.as_ref().map(|git_request| {
            let collector = collector.clone();
            let discovered = discovered_git;
            scope.spawn(move || {
                performance::with_current_collector(collector, || {
                    let git_started = Instant::now();
                    let git = match discovered {
                        Some(repo) => repo.detect_with_request(git_request).map(Some),
                        None => Ok(None),
                    };
                    performance::record_logged_stage(
                        "filesystem.git",
                        git_started.elapsed(),
                        Level::DEBUG,
                    );
                    git
                })
            })
        });

        let formatting_handle = request.include_formatting.then(|| {
            let collector = collector.clone();
            scope.spawn(move || {
                performance::with_current_collector(collector, || {
                    let formatting_started = Instant::now();
                    let formatting = detect_formatting(root).ok().flatten();
                    performance::record_logged_stage(
                        "filesystem.formatting",
                        formatting_started.elapsed(),
                        Level::DEBUG,
                    );
                    formatting
                })
            })
        });

        let shared_view_handle = need_shared_view.then(|| {
            let collector = collector.clone();
            let shared_root = shared_root.clone();
            let options = system_view::SharedWalkOptions {
                collect_manifests: need_repo_full || request.include_docs,
                collect_inventory: request.include_file_inventory || need_repo_full,
                collect_docs: request.include_docs,
            };
            scope.spawn(move || {
                performance::with_current_collector(collector, || {
                    system_view::build_filesystem_system_view(&shared_root, options)
                })
            })
        });

        let git = match git_handle {
            Some(handle) => handle.join().unwrap()?,
            None => None,
        };
        let formatting = formatting_handle.and_then(|handle| handle.join().unwrap());
        let shared_view = shared_view_handle.map(|handle| handle.join().unwrap());

        let repo_detection_root = if request.repo.is_some() {
            git.as_ref().map(|g| g.repo_root.as_path()).unwrap_or(root)
        } else {
            shared_view
                .as_ref()
                .map(|view| view.root.as_path())
                .unwrap_or(root)
        };
        let shared_repo_data = shared_view
            .as_ref()
            .filter(|view| view.root == repo_detection_root);

        let repo_started = Instant::now();
        let repo_context = if need_repo_context {
            let structure_only = request
                .repo
                .as_ref()
                .map(|repo| repo.structure_only)
                .unwrap_or(true);
            let (repo_info, _) = repo::detection::detect_repo_inner_with_shared(
                repo_detection_root,
                structure_only,
                shared_repo_data.and_then(|view| view.manifest_index.as_ref()),
                shared_repo_data.and_then(|view| view.inventory.as_ref()),
            )?;
            repo_info
        } else {
            None
        };
        performance::record_logged_stage("filesystem.repo", repo_started.elapsed(), Level::DEBUG);
        let repo = if request.repo.is_some() {
            repo_context.clone()
        } else {
            None
        };

        let inventory_started = Instant::now();
        let (files, languages) = if request.include_file_inventory {
            let inventory = match repo_context.as_ref().and_then(|r| r.package_for_dir(root)) {
                Some(package) => {
                    let exclude_roots = repo_context
                        .as_ref()
                        .and_then(|r| r.packages.as_ref())
                        .map(|packages| {
                            packages
                                .iter()
                                .filter(|candidate| candidate.path != package.path)
                                .filter(|candidate| candidate.path.starts_with(&package.path))
                                .map(|candidate| candidate.path.clone())
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();
                    match shared_view
                        .as_ref()
                        .and_then(|view| view.inventory.as_ref())
                    {
                        Some(inventory) => {
                            Some(filter_inventory(inventory, &package.path, &exclude_roots))
                        }
                        None => file_types::scan_file_inventory_with_exclusions(
                            &package.path,
                            &exclude_roots,
                        )
                        .ok(),
                    }
                }
                None => match shared_view
                    .as_ref()
                    .and_then(|view| view.inventory.as_ref())
                {
                    Some(inventory) if inventory.scope.root == root => Some(inventory.clone()),
                    Some(inventory) => Some(filter_inventory(inventory, root, &[])),
                    None => file_types::scan_file_inventory(root).ok(),
                },
            };

            match inventory {
                Some(inventory) => {
                    let (fab, lang_summary) = file_types::summarize_file_inventory(&inventory);
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

        let docs_started = Instant::now();
        let docs = if request.include_docs {
            let mut docs = shared_view
                .as_ref()
                .and_then(|view| view.docs.clone())
                .unwrap_or_default();

            if let Some(packages) = repo_context
                .as_ref()
                .and_then(|repo| repo.packages.as_ref())
            {
                let package_paths: Vec<(String, PathBuf)> = packages
                    .iter()
                    .map(|package| (package.name.clone(), PathBuf::from(&package.relative)))
                    .collect();
                let docs_root = shared_view
                    .as_ref()
                    .map(|view| view.root.as_path())
                    .or_else(|| git.as_ref().map(|info| info.repo_root.as_path()))
                    .unwrap_or(root);
                docs::assign_packages(&mut docs, &package_paths, docs_root);
            }

            if docs.is_empty() { None } else { Some(docs) }
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

    // Use Arc::make_mut to share the underlying data when possible.
    // If no filtering is needed, return a clone of the Arc (zero-copy).
    if target_prefix == Path::new("") && exclude_prefixes.is_empty() {
        return file_types::FileInventory {
            scope: file_types::FileScanScope {
                root: target_root.to_path_buf(),
                exclude_roots: exclude_roots.to_vec(),
            },
            total_files_scanned: source.classifications.len(),
            classifications: source.classifications.clone(),
        };
    }

    let classifications: Vec<file_types::FileClassification> = source
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
        classifications: Arc::new(classifications),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::request::{DetectionPlan, GitRequest, RepoRequest};

    /// Creates a temporary git repo with a committed file and an uncommitted
    /// modification plus an untracked file, suitable for testing status-walk
    /// behavior.
    fn create_dirty_git_repo() -> (tempfile::TempDir, PathBuf) {
        use git2::{Repository, Signature};
        use std::fs;

        let dir = tempfile::TempDir::new().unwrap();
        let repo = Repository::init(dir.path()).unwrap();

        let file_path = dir.path().join("hello.txt");
        fs::write(&file_path, "hello world\n").unwrap();

        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new("hello.txt")).unwrap();
        index.write().unwrap();

        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let sig = Signature::now("Test", "test@test.com").unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "initial commit", &tree, &[])
            .unwrap();

        fs::write(&file_path, "hello world\nmodified line\n").unwrap();
        fs::write(dir.path().join("untracked.txt"), "new file\n").unwrap();

        let path = dir.path().to_path_buf();
        (dir, path)
    }

    #[test]
    fn identity_plan_does_not_walk_status_end_to_end() {
        let (_dir, path) = create_dirty_git_repo();

        // Measure walks for this repo's path as a before/after delta. The plan's
        // git stage runs on a scoped thread, but walks are recorded per repo path
        // regardless of thread, so no cross-thread counter propagation is needed.
        let before = crate::filesystem::git::status::status_walk_count(&path);

        let plan = DetectionPlan::new()
            .base_dir(path.clone())
            .without_os()
            .without_hardware()
            .without_network()
            .filesystem(
                FilesystemRequest::new()
                    .git(GitRequest::identity())
                    .repo(RepoRequest::structure())
                    .without_file_inventory()
                    .without_formatting()
                    .without_docs(),
            );

        let result = crate::detect_with_plan(plan).unwrap();
        let fs = result.filesystem.expect("filesystem should be present");
        let git = fs.git.expect("git should be present");

        assert_eq!(
            crate::filesystem::git::status::status_walk_count(&path),
            before,
            "identity plan must not trigger a working-tree status walk"
        );
        assert!(
            git.status.is_none(),
            "identity plan must yield status == None"
        );
        assert_eq!(
            git.repo_root.canonicalize().unwrap(),
            path.canonicalize().unwrap(),
            "repo_root should match the fixture directory"
        );
        assert!(
            git.current_branch.is_some(),
            "current_branch should be set on a branch"
        );
    }

    #[test]
    fn need_shared_view_includes_formatting() {
        // When only formatting is requested, shared view should still be triggered
        let mut request = FilesystemRequest::new();
        request.include_formatting = true;
        let need_repo_full = request
            .repo
            .as_ref()
            .is_some_and(|repo| !repo.structure_only);
        let need_shared_view = need_repo_full
            || request.include_file_inventory
            || request.include_docs
            || request.include_formatting;

        assert!(
            need_shared_view,
            "include_formatting should trigger shared view"
        );
    }

    #[test]
    fn need_shared_view_true_for_repo_detection() {
        let request = FilesystemRequest::new().repo(RepoRequest::full());
        let need_repo_full = request
            .repo
            .as_ref()
            .is_some_and(|repo| !repo.structure_only);
        let need_shared_view = need_repo_full
            || request.include_file_inventory
            || request.include_docs
            || request.include_formatting;

        assert!(
            need_shared_view,
            "repo detection should trigger shared view"
        );
    }

    #[test]
    fn need_shared_view_false_for_structure_only_repo() {
        let request = FilesystemRequest::new()
            .git(GitRequest::summary())
            .repo(RepoRequest::structure())
            .without_docs()
            .without_formatting()
            .without_file_inventory();
        let need_repo_full = request
            .repo
            .as_ref()
            .is_some_and(|repo| !repo.structure_only);
        let need_shared_view = need_repo_full
            || request.include_file_inventory
            || request.include_docs
            || request.include_formatting;

        assert!(
            !need_shared_view,
            "structure-only repo without other shared features should not trigger shared view"
        );
    }

    #[test]
    fn need_shared_view_true_for_file_inventory() {
        let mut request = FilesystemRequest::new();
        request.include_file_inventory = true;
        let need_repo_full = request
            .repo
            .as_ref()
            .is_some_and(|repo| !repo.structure_only);
        let need_shared_view = need_repo_full
            || request.include_file_inventory
            || request.include_docs
            || request.include_formatting;

        assert!(
            need_shared_view,
            "file inventory should trigger shared view"
        );
    }

    #[test]
    fn need_shared_view_false_when_all_disabled() {
        let request = FilesystemRequest::new()
            .without_git()
            .without_repo()
            .without_docs()
            .without_formatting()
            .without_file_inventory();
        let need_repo_full = request
            .repo
            .as_ref()
            .is_some_and(|repo| !repo.structure_only);
        let need_shared_view = need_repo_full
            || request.include_file_inventory
            || request.include_docs
            || request.include_formatting;

        assert!(
            !need_shared_view,
            "request with all features disabled should not trigger shared view"
        );
    }

    // ============================================================================
    // filter_inventory tests (issue #21)
    // ============================================================================

    #[test]
    fn filter_inventory_zero_copy_when_no_filtering_needed() {
        use crate::filesystem::file_types::{FileClassification, FileScanScope};
        use std::sync::Arc;

        let root = PathBuf::from("/repo");
        let classifications = Arc::new(vec![
            FileClassification {
                path: PathBuf::from("src/main.rs"),
                ..Default::default()
            },
            FileClassification {
                path: PathBuf::from("README.md"),
                ..Default::default()
            },
        ]);

        let inventory = file_types::FileInventory {
            scope: FileScanScope {
                root: root.clone(),
                exclude_roots: vec![],
            },
            total_files_scanned: 2,
            classifications: classifications.clone(),
        };

        // When target_root == source_root and no excludes, filter_inventory
        // should return a clone of the Arc (zero-copy).
        let filtered = filter_inventory(&inventory, &root, &[]);
        assert_eq!(filtered.total_files_scanned, 2);
        assert!(Arc::ptr_eq(
            &inventory.classifications,
            &filtered.classifications
        ));
    }

    #[test]
    fn filter_inventory_clones_when_filtering_needed() {
        use crate::filesystem::file_types::{FileClassification, FileScanScope};
        use std::sync::Arc;

        let root = PathBuf::from("/repo");
        let classifications = Arc::new(vec![
            FileClassification {
                path: PathBuf::from("packages/foo/src/lib.rs"),
                ..Default::default()
            },
            FileClassification {
                path: PathBuf::from("packages/bar/src/lib.rs"),
                ..Default::default()
            },
        ]);

        let inventory = file_types::FileInventory {
            scope: FileScanScope {
                root: root.clone(),
                exclude_roots: vec![],
            },
            total_files_scanned: 2,
            classifications,
        };

        let target = PathBuf::from("/repo/packages/foo");
        let filtered = filter_inventory(&inventory, &target, &[]);
        assert_eq!(filtered.total_files_scanned, 1);
        assert_eq!(
            filtered.classifications[0].path,
            PathBuf::from("packages/foo/src/lib.rs")
        );
    }
}
