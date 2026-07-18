use crate::Result;
use crate::performance;
use crate::request::{FilesystemRequest, GitRequest, RepoRequest};
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
    BehindStatus, CommitDesc, CommitDescSet, CommitInfo, DEFAULT_PATH_HISTORY_SCAN_LIMIT, DeltaKind,
    GitHostingProvider, GitInfo, GitRepo, LocalBranchInfo, PathHistoryOptions, PathHistoryResult,
    PeriodSpecifier, RemoteInfo, RepoStatus, commit_browser_url, commit_by_sha_at, commit_files_at,
    commits_for_branch_at, commits_for_path_at, detect_git, detect_git_with_request,
    detect_merge_conflicts, get_commit_by_sha, get_commit_files, get_commits_for_branch,
    get_commits_for_path, get_recent_commits_by_count, get_recent_commits_by_date,
    get_recent_commits_by_duration, get_recent_commits_by_hash, get_recent_commits_in_range,
    merge_conflicts_at, parse_commit_message, parse_period, preferred_remote_url, remote_url,
    repo_root,
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

/// Which consumers of a request need evidence from a descendant walk.
///
/// Formatting is deliberately absent: `detect_formatting` probes the requested
/// `.editorconfig` chain directly and reads nothing the walk produces, so a
/// formatting-only request must start no walker at all.
#[derive(Debug, Clone, Copy)]
struct WalkConsumers {
    repo_full: bool,
    docs: bool,
    inventory: bool,
}

impl WalkConsumers {
    fn of(request: &FilesystemRequest) -> Self {
        Self {
            repo_full: request
                .repo
                .as_ref()
                .is_some_and(|repo| !repo.structure_only),
            docs: request.include_docs,
            inventory: request.include_file_inventory,
        }
    }

    /// Whether a consumer needs evidence from outside the requested directory.
    ///
    /// Full repository detection and repository-wide document discovery both
    /// describe the repository, not the caller's directory. Inventory does not:
    /// it is scoped to the package or base directory that was asked for.
    fn need_repository_scope(&self) -> bool {
        self.repo_full || self.docs
    }

    fn need_any_walk(&self) -> bool {
        self.repo_full || self.docs || self.inventory
    }
}

/// Which tree, if any, the shared descendant walk enumerates.
///
/// Chosen from [`WalkConsumers`] alone. A Git handle never appears here: a
/// repository being *present* is not a consumer of repository-wide evidence,
/// and treating it as one silently widened package-scoped inventory requests to
/// the whole monorepo.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WalkScope {
    /// No consumer needs a descendant walk.
    None,
    /// A repository-wide consumer is active; enumerate from the repository root.
    Repository,
    /// Only inventory is active; enumerate from the resolved package/base root.
    Package,
}

impl WalkScope {
    fn of(consumers: &WalkConsumers) -> Self {
        if !consumers.need_any_walk() {
            Self::None
        } else if consumers.need_repository_scope() {
            Self::Repository
        } else {
            Self::Package
        }
    }
}

/// Run repository detection for the shared repo context, reusing walk evidence.
///
/// Returns an empty context when no consumer needs repository context, so callers can
/// keep one call site per walk scope rather than repeating the `need` check.
#[derive(Default)]
struct DetectedRepoContext {
    info: Option<RepoInfo>,
    ownership_index: Option<repo::ownership::PackageOwnershipIndex>,
}

fn detect_repo_context(
    need_repo_context: bool,
    root: &Path,
    request: &RepoRequest,
    view: Option<&system_view::FilesystemSystemView>,
) -> Result<DetectedRepoContext> {
    if !need_repo_context {
        return Ok(DetectedRepoContext::default());
    }
    let evidence = view
        .map(repo::detection::RepoEvidence::from_view)
        .unwrap_or_default();
    let (info, _, ownership_index) =
        repo::detection::detect_repo_inner_with_shared_request_and_ownership(
            root,
            request,
            evidence,
            request.details.is_some(),
        )?;
    Ok(DetectedRepoContext {
        info,
        ownership_index,
    })
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
    Ok(detect_filesystem_with_request_inner(root, request, false)?.filesystem)
}

pub(crate) struct AggregateFilesystemDetection {
    pub(crate) filesystem: FilesystemInfo,
    pub(crate) git: Option<git::types::GitAggregateEvidence>,
}

/// Run filesystem detection with the private repository-aggregate companion.
///
/// This is reserved for library projections that need shapes outside the
/// stable [`GitInfo`] contract. Ordinary callers use
/// [`detect_filesystem_with_request`] and never observe the companion.
pub(crate) fn detect_filesystem_for_aggregate(
    root: &Path,
    request: &FilesystemRequest,
) -> Result<AggregateFilesystemDetection> {
    detect_filesystem_with_request_inner(root, request, true)
}

fn detect_filesystem_with_request_inner(
    root: &Path,
    request: &FilesystemRequest,
    collect_aggregate: bool,
) -> Result<AggregateFilesystemDetection> {
    let collector = performance::current_collector();
    let need_repo_context = request.repo.is_some() || request.include_docs;
    let consumers = WalkConsumers::of(request);
    let walk_scope = WalkScope::of(&consumers);

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
        Some(handle) => handle.repo_root().to_path_buf(),
        None => root.to_path_buf(),
    };

    std::thread::scope(|scope| {
        let git_handle = request.git.as_ref().map(|git_request| {
            let collector = collector.clone();
            let discovered = discovered_git;
            scope.spawn(move || {
                performance::with_current_collector(collector, || {
                    let git_started = Instant::now();
                    let detected: Result<(
                        Option<GitInfo>,
                        Option<git::types::GitAggregateEvidence>,
                    )> = match discovered {
                        Some(repo) => {
                            let info = repo.detect_with_request(git_request)?;
                            let aggregate = collect_aggregate
                                .then(|| repo.observe_aggregate_evidence())
                                .transpose()?;
                            Ok((Some(info), aggregate))
                        }
                        None => Ok((None, None)),
                    };
                    performance::record_logged_stage(
                        "filesystem.git",
                        git_started.elapsed(),
                        Level::DEBUG,
                    );
                    detected
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

        let options = system_view::SharedWalkOptions {
            collect_manifests: consumers.repo_full || consumers.docs,
            collect_inventory: consumers.inventory || consumers.repo_full,
            collect_docs: consumers.docs,
            // Nested-workspace markers are repo-structure evidence, so only a
            // repository-scoped walk can supply them. Collecting them on a
            // package-scoped walk would hand repo detection a set that is
            // missing every marker outside the package.
            collect_nested_markers: consumers.repo_full,
        };
        let structure_request = RepoRequest::structure();
        let repo_request = request.repo.as_ref().unwrap_or(&structure_request);

        // The walk and repo detection run here on the calling thread, alongside
        // the git and formatting threads. Neither waits on the git result: the
        // repository root comes from the discovery handle above, so joining git
        // first would serialize the two for nothing.
        let repo_started = Instant::now();
        let (mut shared_view, mut repo_context) = match walk_scope {
            WalkScope::None => (
                None,
                detect_repo_context(need_repo_context, &shared_root, repo_request, None)?,
            ),
            WalkScope::Repository => {
                let view = system_view::build_filesystem_system_view(&shared_root, options);
                let repo_context = detect_repo_context(
                    need_repo_context,
                    &shared_root,
                    repo_request,
                    Some(&view),
                )?;
                (Some(view), repo_context)
            }
            WalkScope::Package => {
                // Inventory is the only consumer, so the walk is scoped to the
                // package owning `root`. Finding that package needs repository
                // membership, which structure-only detection resolves without a
                // descendant walk of its own — so it must run first, and that
                // ordering is the point rather than an accident.
                let repo_context =
                    detect_repo_context(need_repo_context, &shared_root, repo_request, None)?;
                let walk_root = repo_context
                    .info
                    .as_ref()
                    .and_then(|repo| repo.package_for_dir(root))
                    .map(|package| package.path.clone())
                    .unwrap_or_else(|| root.to_path_buf());
                let view = system_view::build_filesystem_system_view(&walk_root, options);
                (Some(view), repo_context)
            }
        };
        performance::record_logged_stage("filesystem.repo", repo_started.elapsed(), Level::DEBUG);

        let (git, aggregate_git) = match git_handle {
            Some(handle) => handle.join().unwrap()?,
            None => (None, None),
        };
        let formatting = formatting_handle.and_then(|handle| handle.join().unwrap());

        let inventory_started = Instant::now();
        let (files, languages) = if request.include_file_inventory {
            let inventory = match repo_context
                .info
                .as_ref()
                .and_then(|repo| repo.package_for_dir(root))
            {
                Some(package) => {
                    let exclude_roots = repo_context
                        .info
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
            // Move the collected documents out of the view rather than cloning:
            // the view is discarded at the end of this scope and nothing else
            // reads `docs` after this point.
            let docs_root = shared_view
                .as_ref()
                .map(|view| view.root.clone())
                .or_else(|| git.as_ref().map(|info| info.repo_root.clone()))
                .unwrap_or_else(|| root.to_path_buf());
            let mut docs = shared_view
                .as_mut()
                .and_then(|view| view.docs.take())
                .unwrap_or_default();

            if let (Some(repo), Some(ownership_index)) = (
                repo_context.info.as_ref(),
                repo_context.ownership_index.as_ref(),
            ) {
                docs::assign_packages_from_repo(
                    &mut docs,
                    repo,
                    ownership_index,
                    &docs_root,
                );
            }

            if docs.is_empty() { None } else { Some(docs) }
        } else {
            None
        };
        performance::record_logged_stage("filesystem.docs", docs_started.elapsed(), Level::DEBUG);

        // Move the completed `RepoInfo` out; a request that only needed it as
        // docs context drops it here instead of returning it.
        let repo = match request.repo.is_some() {
            true => repo_context.info.take(),
            false => None,
        };

        Ok(AggregateFilesystemDetection {
            filesystem: FilesystemInfo {
                languages,
                files,
                git,
                repo,
                formatting,
                docs,
            },
            git: aggregate_git,
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
///
/// ## Notes
///
/// When no filtering is required the returned inventory shares `source`'s
/// classifications through the `Arc` rather than copying them; any narrowing
/// filter necessarily copies the retained classifications.
///
/// The subset inherits `source`'s completeness: files the cap discarded may
/// have fallen under `target_root`, so a subset of a truncated inventory is
/// itself truncated.
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

    if target_prefix == Path::new("") && exclude_prefixes.is_empty() {
        return file_types::FileInventory {
            scope: file_types::FileScanScope {
                root: target_root.to_path_buf(),
                exclude_roots: exclude_roots.to_vec(),
            },
            total_files_scanned: source.classifications.len(),
            classifications: source.classifications.clone(),
            truncated: source.truncated,
            limit: source.limit,
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
        truncated: source.truncated,
        limit: source.limit,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::request::{DetectionPlan, GitRequest, RepoRequest};

    /// Creates a temporary git repo with a committed file and an uncommitted
    /// modification plus an untracked file, suitable for testing status-walk
    /// behavior.
    pub(super) fn create_dirty_git_repo() -> (tempfile::TempDir, PathBuf) {
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

    /// The walk-scope decision table (Phase 2, sub-spec C4).
    ///
    /// Asserts the real `WalkScope::of` rather than re-deriving the rule in the
    /// test: the previous `need_shared_view_*` tests recomputed the boolean
    /// locally and so passed regardless of what the planner actually did.
    #[test]
    fn walk_scope_table() {
        let cases: Vec<(&str, FilesystemRequest, WalkScope)> = vec![
            (
                "formatting only starts no walker",
                FilesystemRequest::new()
                    .without_git()
                    .without_repo()
                    .without_docs()
                    .without_file_inventory(),
                WalkScope::None,
            ),
            (
                "structure-only repo needs no walk",
                FilesystemRequest::new()
                    .git(GitRequest::summary())
                    .repo(RepoRequest::structure())
                    .without_docs()
                    .without_formatting()
                    .without_file_inventory(),
                WalkScope::None,
            ),
            (
                "everything disabled needs no walk",
                FilesystemRequest::new()
                    .without_git()
                    .without_repo()
                    .without_docs()
                    .without_formatting()
                    .without_file_inventory(),
                WalkScope::None,
            ),
            (
                "full repo detection is repository-scoped",
                FilesystemRequest::new()
                    .repo(RepoRequest::full())
                    .without_docs()
                    .without_formatting()
                    .without_file_inventory(),
                WalkScope::Repository,
            ),
            (
                "repo-wide docs are repository-scoped",
                FilesystemRequest::new()
                    .without_git()
                    .without_repo()
                    .without_formatting()
                    .without_file_inventory(),
                WalkScope::Repository,
            ),
            (
                "inventory alone stays package-scoped",
                FilesystemRequest::new()
                    .without_git()
                    .without_repo()
                    .without_docs()
                    .without_formatting(),
                WalkScope::Package,
            ),
            (
                "git presence alone does not widen an inventory request",
                FilesystemRequest::new()
                    .git(GitRequest::summary())
                    .repo(RepoRequest::structure())
                    .without_docs()
                    .without_formatting(),
                WalkScope::Package,
            ),
            (
                "a mixed request takes the widest scope its consumers require",
                FilesystemRequest::new()
                    .git(GitRequest::summary())
                    .repo(RepoRequest::full())
                    .without_formatting(),
                WalkScope::Repository,
            ),
        ];

        for (name, request, expected) in cases {
            let actual = WalkScope::of(&WalkConsumers::of(&request));
            assert_eq!(actual, expected, "{name}");
        }
    }

    #[test]
    fn filesystem_detection_projects_a_standalone_package_without_reparsing_manifest() {
        use crate::performance::{counters, testing};

        let temp = tempfile::tempdir().unwrap();
        std::fs::write(
            temp.path().join("Cargo.toml"),
            "[package]\nname = \"standalone\"\nversion = \"1.2.3\"\n",
        )
        .unwrap();
        let request = FilesystemRequest::new()
            .without_git()
            .repo(RepoRequest::focused(
                crate::request::RepoDetailRequest::all(),
            ))
            .without_docs()
            .without_formatting()
            .without_file_inventory();

        let (result, counts) =
            testing::measure(|| detect_filesystem_with_request(temp.path(), &request));
        let repo = result
            .expect("filesystem detection succeeds")
            .repo
            .expect("filesystem detection carries the standalone package");
        let packages = repo.packages.expect("standalone package catalog");

        assert_eq!(packages.len(), 1);
        assert_eq!(packages[0].name, "standalone");
        assert_eq!(packages[0].version.as_deref(), Some("1.2.3"));
        assert_eq!(
            counts.get(counters::REPO_MANIFEST_PARSES),
            1,
            "workspace detection and standalone projection must share one manifest parse; \
             counters were {:?}",
            counts.all()
        );
        assert_eq!(counts.get(counters::REPO_PACKAGE_ENRICHMENTS), 1);
    }

    /// R3.1: a formatting-only request must enumerate no descendants.
    ///
    /// Asserted with the walk counter rather than the decision enum, so it
    /// still fails if the planner is right but some stage walks anyway.
    #[test]
    fn formatting_only_request_starts_no_walker() {
        use crate::performance::counters;
        use crate::performance::testing;

        let (_dir, path) = create_dirty_git_repo();
        let request = FilesystemRequest::new()
            .without_git()
            .without_repo()
            .without_docs()
            .without_file_inventory();

        let (result, counts) =
            testing::measure(|| detect_filesystem_with_request(&path, &request));
        result.expect("formatting-only detection should succeed");

        assert_eq!(
            counts.get(counters::FS_WALK_STARTS),
            0,
            "formatting reads the .editorconfig chain directly and must start no \
             descendant walk; counters were {:?}",
            counts.all()
        );
        assert_eq!(counts.get(counters::FS_WALK_ENTRIES), 0);
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
            ..Default::default()
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
            ..Default::default()
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

#[cfg(test)]
mod planner_counter_propagation {
    use super::*;
    use crate::performance::{counters, testing};
    use crate::request::GitRequest;

    /// Counters recorded on the planner's scoped stage threads must reach the
    /// request's report.
    ///
    /// Recording writes to a thread-local buffer that only `snapshot()` (on the
    /// requesting thread) and `WorkerCollector`'s drop ever drain. The git stage
    /// runs on a `std::thread::scope` thread that has neither, so before
    /// `with_current_collector` learned to flush, this whole family read zero
    /// for a request that demonstrably walked status once — the exact
    /// count-nothing-and-call-it-zero failure work accounting exists to prevent.
    #[test]
    fn git_stage_counters_survive_the_scoped_thread() {
        let (_dir, path) = super::tests::create_dirty_git_repo();
        let request = FilesystemRequest::new()
            .git(GitRequest::full())
            .without_repo()
            .without_docs()
            .without_formatting()
            .without_file_inventory();

        let (result, counts) =
            testing::measure(|| detect_filesystem_with_request(&path, &request));
        let git = result
            .expect("detection should succeed")
            .git
            .expect("git was requested");
        assert!(
            git.status.is_some_and(|status| status.is_dirty),
            "the fixture must be dirty, or this test proves nothing"
        );

        assert_eq!(
            counts.get(counters::GIT_STATUS_WALKS),
            1,
            "the git stage's status walk must appear in the report; counters were {:?}",
            counts.all()
        );
        assert_eq!(counts.get(counters::GIT_DISCOVERIES), 1);
    }
}
