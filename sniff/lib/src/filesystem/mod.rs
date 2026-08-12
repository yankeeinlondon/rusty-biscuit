use crate::Result;
use crate::performance;
use crate::request::{FilesystemRequest, GitRequest, RepoRequest};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
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
    FileChange, GitHostingProvider, GitInfo, GitRepo, LocalBranchInfo, PathHistoryOptions,
    PathHistoryResult, PeriodSpecifier, RemoteInfo, RepoStatus, commit_browser_url,
    commit_by_sha_at, commit_files_at, commits_for_branch_at, commits_for_path_at, detect_git,
    detect_git_with_request,
    detect_merge_conflicts, get_commit_by_sha, get_commit_files, get_commits_for_branch,
    get_commits_for_path, get_recent_commits_by_count, get_recent_commits_by_date,
    get_recent_commits_by_duration, get_recent_commits_by_hash, get_recent_commits_in_range,
    merge_conflicts_at, merge_conflicts_with_branch_at, parse_commit_message, parse_period,
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

/// Immutable path identity for a discovered Git repository.
///
/// `git_dir` is worktree-specific, while `common_dir` may be shared by linked
/// worktrees. Consumers that key observations must therefore include
/// `git_dir` rather than collapsing repositories by `common_dir` alone.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitRepositoryIdentity {
    repo_root: PathBuf,
    git_dir: PathBuf,
    common_dir: PathBuf,
    is_bare: bool,
}

impl GitRepositoryIdentity {
    /// Working-tree root, or the Git directory for a bare repository.
    pub fn repo_root(&self) -> &Path {
        &self.repo_root
    }

    /// Worktree-specific Git directory.
    pub fn git_dir(&self) -> &Path {
        &self.git_dir
    }

    /// Git directory shared by all linked worktrees.
    pub fn common_dir(&self) -> &Path {
        &self.common_dir
    }

    /// Whether the repository has no working tree.
    pub fn is_bare(&self) -> bool {
        self.is_bare
    }
}

#[derive(Debug)]
struct ObservedGitRepository {
    identity: GitRepositoryIdentity,
    handle: Mutex<GitRepo>,
}

#[derive(Debug, Clone)]
enum GitObservation {
    Present(Arc<ObservedGitRepository>),
    Absent,
    Failed(Arc<crate::SniffError>),
}

/// A reusable, request-scoped Git filesystem observation.
///
/// Clones share one discovered repository handle. Git queries are serialized
/// because the underlying gix handle has request-local mutable caches and is
/// intentionally not `Sync`. Absence and discovery failures are retained too,
/// so later consumers do not repeat an upward repository search.
#[derive(Debug, Clone)]
pub struct FilesystemObservation {
    observed_root: PathBuf,
    git: GitObservation,
}

impl FilesystemObservation {
    /// Discover and retain the Git repository containing `root`.
    ///
    /// Discovery failures are stored in the returned observation. Consumers
    /// surface the retained typed failure through [`Self::discovery_error`] or
    /// a [`crate::SniffError::RetainedObservation`] without retrying discovery.
    pub fn discover(root: &Path) -> Self {
        let git = match GitRepo::discover(root) {
            Ok(Some(handle)) => {
                let identity = GitRepositoryIdentity {
                    repo_root: handle.repo_root().to_path_buf(),
                    git_dir: handle.git_dir().to_path_buf(),
                    common_dir: handle.common_dir().to_path_buf(),
                    is_bare: handle.is_bare(),
                };
                GitObservation::Present(Arc::new(ObservedGitRepository {
                    identity,
                    handle: Mutex::new(handle),
                }))
            }
            Ok(None) => GitObservation::Absent,
            Err(error) => GitObservation::Failed(Arc::new(error)),
        };
        Self {
            observed_root: root.to_path_buf(),
            git,
        }
    }

    /// Path from which this observation was acquired.
    pub fn observed_root(&self) -> &Path {
        &self.observed_root
    }

    /// Rebase this observation to another directory in the same worktree.
    ///
    /// Present repositories permit descendants of their working-tree root as
    /// long as no nested `.git` boundary intervenes. Bare repositories,
    /// explicit absence, and failed discovery remain exact-root observations.
    /// This operation performs bounded path validation but no Git discovery.
    pub fn for_root(&self, root: &Path) -> Result<Self> {
        if root == self.observed_root {
            return Ok(self.clone());
        }
        let GitObservation::Present(repository) = &self.git else {
            return Err(observation_rebase_error(&self.observed_root, root));
        };
        if repository.identity.is_bare() {
            return Err(observation_rebase_error(&self.observed_root, root));
        }

        let root_canonical = canonicalize_observation_path(root)?;
        let repo_canonical = canonicalize_observation_path(repository.identity.repo_root())?;
        if !root_canonical.starts_with(&repo_canonical)
            || has_nested_git_boundary(&root_canonical, &repo_canonical)?
        {
            return Err(observation_rebase_error(&self.observed_root, root));
        }

        Ok(Self {
            observed_root: root.to_path_buf(),
            git: self.git.clone(),
        })
    }

    /// Repository identity, explicit absence, or the retained discovery error.
    pub fn repository_identity(&self) -> Result<Option<&GitRepositoryIdentity>> {
        match &self.git {
            GitObservation::Present(repository) => Ok(Some(&repository.identity)),
            GitObservation::Absent => Ok(None),
            GitObservation::Failed(error) => Err(retained_observation_error(error)),
        }
    }

    /// Original typed discovery failure, if acquisition failed.
    pub fn discovery_error(&self) -> Option<&crate::SniffError> {
        match &self.git {
            GitObservation::Failed(error) => Some(error.as_ref()),
            GitObservation::Present(_) | GitObservation::Absent => None,
        }
    }

    /// Detect Git facts with the retained repository handle.
    ///
    /// Returns `Ok(None)` for a retained non-repository observation.
    pub fn detect_git(&self, request: &GitRequest) -> Result<Option<GitInfo>> {
        self.with_repository(|repo| repo.detect_with_request(request))
    }

    /// Capture per-file working-tree changes with the retained repository handle.
    ///
    /// The projection includes staged, unstaged, and untracked paths plus line
    /// statistics, but not unified diff payloads. Returns `Ok(None)` for a
    /// retained non-repository observation.
    pub fn detect_file_changes(&self) -> Result<Option<Vec<FileChange>>> {
        self.with_repository(GitRepo::file_changes)
    }

    fn with_repository<T>(&self, f: impl FnOnce(&GitRepo) -> Result<T>) -> Result<Option<T>> {
        match &self.git {
            GitObservation::Present(repository) => {
                let handle = repository
                    .handle
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                f(&handle).map(Some)
            }
            GitObservation::Absent => Ok(None),
            GitObservation::Failed(error) => Err(retained_observation_error(error)),
        }
    }
}

fn retained_observation_error(error: &Arc<crate::SniffError>) -> crate::SniffError {
    crate::SniffError::RetainedObservation {
        source: error.clone(),
    }
}

fn canonicalize_observation_path(path: &Path) -> Result<PathBuf> {
    performance::increment_counter(performance::counters::FS_CANONICALIZATIONS, 1);
    std::fs::canonicalize(path).map_err(crate::SniffError::Io)
}

fn has_nested_git_boundary(root: &Path, repository_root: &Path) -> Result<bool> {
    let mut current = root;
    while current != repository_root {
        performance::increment_counter(performance::counters::FS_METADATA_PROBES, 1);
        match std::fs::symlink_metadata(current.join(".git")) {
            Ok(_) => return Ok(true),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(crate::SniffError::Io(error)),
        }
        let Some(parent) = current.parent() else {
            return Ok(true);
        };
        current = parent;
    }
    Ok(false)
}

fn observation_rebase_error(observed_root: &Path, requested_root: &Path) -> crate::SniffError {
    crate::SniffError::SystemInfo {
        domain: "filesystem",
        message: format!(
            "filesystem observation for '{}' cannot be reused at '{}'",
            observed_root.display(),
            requested_root.display()
        ),
    }
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
    Ok(detect_filesystem_with_request_inner(root, request, false, None)?.filesystem)
}

/// Detect filesystem information using an already acquired Git observation.
///
/// The observation must have been acquired for `root`. Supplying it prevents
/// any additional upward Git discovery and leaves the shared handle available
/// to the caller for later request-scoped Git facts.
pub fn detect_filesystem_with_observation(
    root: &Path,
    request: &FilesystemRequest,
    observation: &FilesystemObservation,
) -> Result<FilesystemInfo> {
    validate_observation_root(root, observation)?;
    Ok(
        detect_filesystem_with_request_inner(root, request, false, Some(observation))?
            .filesystem,
    )
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
    detect_filesystem_with_request_inner(root, request, true, None)
}

fn detect_filesystem_with_request_inner(
    root: &Path,
    request: &FilesystemRequest,
    collect_aggregate: bool,
    observation: Option<&FilesystemObservation>,
) -> Result<AggregateFilesystemDetection> {
    let collector = performance::current_collector();
    let need_repo_context = request.repo.is_some() || request.include_docs;
    let consumers = WalkConsumers::of(request);
    let walk_scope = WalkScope::of(&consumers);

    // Ambient requests acquire one observation here. Seeded requests reuse the
    // caller's retained observation, including repository absence or failure.
    // Both paths use that same handle for Git facts and shared-walk root
    // selection. Requests without Git keep the provided root as their walk root.
    let acquired_observation;
    let git_observation = match (request.git.as_ref(), observation) {
        (Some(_), Some(observation)) => Some(observation),
        (Some(_), None) => {
            acquired_observation = FilesystemObservation::discover(root);
            Some(&acquired_observation)
        }
        (None, _) => None,
    };
    let shared_root = match git_observation
        .map(FilesystemObservation::repository_identity)
        .transpose()?
        .flatten()
    {
        // A bare repository's `repo_root` is its git directory. Walking that as
        // a source tree would inventory git internals, so keep the caller's root.
        Some(identity) if !identity.is_bare() => identity.repo_root().to_path_buf(),
        _ => root.to_path_buf(),
    };

    std::thread::scope(|scope| {
        let git_handle = request.git.as_ref().map(|git_request| {
            let collector = collector.clone();
            let observation = git_observation.cloned();
            scope.spawn(move || {
                performance::with_current_collector(collector, || {
                    let git_started = Instant::now();
                    let detected: Result<(
                        Option<GitInfo>,
                        Option<git::types::GitAggregateEvidence>,
                    )> = observation
                        .expect("Git request always has an observation")
                        .with_repository(|repo| {
                            let info = repo.detect_with_request(git_request)?;
                            let aggregate = collect_aggregate
                                .then(|| repo.observe_aggregate_evidence())
                                .transpose()?;
                            Ok((info, aggregate))
                        })
                        .map(|detected| match detected {
                            Some((info, aggregate)) => (Some(info), aggregate),
                            None => (None, None),
                        });
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

fn validate_observation_root(root: &Path, observation: &FilesystemObservation) -> Result<()> {
    if root == observation.observed_root() {
        return Ok(());
    }
    Err(crate::SniffError::SystemInfo {
        domain: "filesystem",
        message: format!(
            "filesystem observation for '{}' cannot seed request rooted at '{}'",
            observation.observed_root().display(),
            root.display()
        ),
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
    use crate::performance::{counters, testing};
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

    fn seeded_fixture_request() -> FilesystemRequest {
        FilesystemRequest::new()
            .git(GitRequest::identity())
            .repo(RepoRequest::structure())
            .without_docs()
            .without_formatting()
            .without_file_inventory()
    }

    fn assert_seeded_git_semantics(root: &Path, expected_root: &Path, expected_bare: bool) {
        let observation = FilesystemObservation::discover(root);
        let identity = observation
            .repository_identity()
            .expect("discovery should not fail")
            .expect("fixture should be a repository");
        assert_eq!(
            identity.repo_root().canonicalize().unwrap(),
            expected_root.canonicalize().unwrap()
        );
        assert_eq!(identity.is_bare(), expected_bare);

        let (result, counts) = testing::measure(|| {
            detect_filesystem_with_observation(root, &seeded_fixture_request(), &observation)
        });
        let filesystem = result.expect("seeded detection should succeed");
        assert!(filesystem.git.is_some());
        assert_eq!(
            counts.get(counters::GIT_DISCOVERIES),
            0,
            "seeded execution must not rediscover the repository; counters were {:?}",
            counts.all()
        );
    }

    #[test]
    fn seeded_and_ordinary_detection_are_equivalent() {
        let (_dir, path) = create_dirty_git_repo();
        std::fs::write(
            path.join("Cargo.toml"),
            "[package]\nname = \"seed-equivalence\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        let request = seeded_fixture_request();

        let ordinary = detect_filesystem_with_request(&path, &request).unwrap();
        let observation = FilesystemObservation::discover(&path);
        let (seeded, counts) = testing::measure(|| {
            detect_filesystem_with_observation(&path, &request, &observation)
        });
        let seeded = seeded.unwrap();

        assert_eq!(
            serde_json::to_value(&seeded.git).unwrap(),
            serde_json::to_value(&ordinary.git).unwrap()
        );
        assert_eq!(
            serde_json::to_value(&seeded.repo).unwrap(),
            serde_json::to_value(&ordinary.repo).unwrap()
        );
        assert_eq!(
            seeded.git.as_ref().map(|git| &git.repo_root),
            ordinary.git.as_ref().map(|git| &git.repo_root)
        );
        assert_eq!(counts.get(counters::GIT_DISCOVERIES), 0);
    }

    #[test]
    fn seeded_and_ordinary_non_repository_detection_are_equivalent() {
        let dir = tempfile::tempdir().unwrap();
        let request = seeded_fixture_request();
        let ordinary = detect_filesystem_with_request(dir.path(), &request).unwrap();
        let observation = FilesystemObservation::discover(dir.path());
        assert!(observation.repository_identity().unwrap().is_none());

        let (seeded, counts) = testing::measure(|| {
            detect_filesystem_with_observation(dir.path(), &request, &observation)
        });
        let seeded = seeded.unwrap();
        assert!(ordinary.git.is_none());
        assert!(ordinary.repo.is_none());
        assert!(seeded.git.is_none());
        assert!(seeded.repo.is_none());
        assert_eq!(counts.get(counters::GIT_DISCOVERIES), 0);
    }

    #[test]
    fn seeded_observation_preserves_normal_linked_bare_and_nested_identity() {
        let (normal_dir, normal_path) = create_dirty_git_repo();
        assert_seeded_git_semantics(&normal_path, &normal_path, false);

        let linked_path = normal_path.join("linked-worktree");
        let repo = git2::Repository::open(&normal_path).unwrap();
        repo.worktree("linked-worktree", &linked_path, None)
            .unwrap();
        assert!(linked_path.join(".git").is_file());
        let linked = FilesystemObservation::discover(&linked_path);
        let linked_identity = linked.repository_identity().unwrap().unwrap();
        assert_ne!(linked_identity.git_dir(), linked_identity.common_dir());
        assert_seeded_git_semantics(&linked_path, &linked_path, false);

        let bare_parent = tempfile::tempdir().unwrap();
        let bare_path = bare_parent.path().join("bare.git");
        git2::Repository::init_bare(&bare_path).unwrap();
        assert_seeded_git_semantics(&bare_path, &bare_path, true);

        let nested_path = normal_path.join("nested");
        std::fs::create_dir_all(&nested_path).unwrap();
        git2::Repository::init(&nested_path).unwrap();
        let nested = FilesystemObservation::discover(&nested_path);
        let nested_identity = nested.repository_identity().unwrap().unwrap();
        assert_eq!(
            nested_identity.repo_root().canonicalize().unwrap(),
            nested_path.canonicalize().unwrap()
        );
        assert_ne!(
            nested_identity.git_dir().canonicalize().unwrap(),
            normal_path.join(".git").canonicalize().unwrap()
        );
        assert_seeded_git_semantics(&nested_path, &nested_path, false);

        drop(normal_dir);
    }

    #[test]
    fn failed_observation_is_reused_without_retry() {
        let dir = tempfile::tempdir().unwrap();
        git2::Repository::init(dir.path()).unwrap();
        std::fs::write(dir.path().join(".git").join("config"), "[core\n").unwrap();

        let ((observation, first, second), counts) = testing::measure(|| {
            let observation = FilesystemObservation::discover(dir.path());
            let first = detect_filesystem_with_observation(
                dir.path(),
                &seeded_fixture_request(),
                &observation,
            );
            let second = observation.detect_git(&GitRequest::identity());
            (observation, first, second)
        });

        assert!(observation.discovery_error().is_some());
        assert!(first.is_err());
        assert!(second.is_err());
        assert_eq!(
            counts.get(counters::GIT_DISCOVERIES),
            1,
            "captured failure must be reused rather than retried; counters were {:?}",
            counts.all()
        );
    }

    #[test]
    fn seeded_filesystem_worker_propagates_status_counters() {
        let (_dir, path) = create_dirty_git_repo();
        let observation = FilesystemObservation::discover(&path);
        let request = FilesystemRequest::new()
            .git(GitRequest::full())
            .without_repo()
            .without_docs()
            .without_formatting()
            .without_file_inventory();

        let (result, counts) = testing::measure(|| {
            detect_filesystem_with_observation(&path, &request, &observation)
        });
        assert!(
            result
                .unwrap()
                .git
                .and_then(|git| git.status)
                .is_some_and(|status| status.is_dirty)
        );
        assert_eq!(counts.get(counters::GIT_DISCOVERIES), 0);
        assert_eq!(
            counts.get(counters::GIT_STATUS_WALKS),
            1,
            "filesystem worker work must reach its parent's collector; counters were {:?}",
            counts.all()
        );
    }

    #[test]
    fn seeded_file_changes_reuse_the_observed_handle() {
        let (_dir, path) = create_dirty_git_repo();
        let observation = FilesystemObservation::discover(&path);

        let (changes, counts) = testing::measure(|| observation.detect_file_changes());
        let changes = changes
            .unwrap()
            .expect("fixture should have a repository observation");
        assert!(
            changes.iter().any(|change| change.path == Path::new("hello.txt")),
            "modified tracked file should be projected: {changes:?}"
        );
        assert!(
            changes
                .iter()
                .any(|change| change.path == Path::new("untracked.txt")),
            "untracked file should be projected: {changes:?}"
        );
        assert_eq!(counts.get(counters::GIT_DISCOVERIES), 0);
        assert_eq!(counts.get(counters::GIT_STATUS_WALKS), 1);
    }

    #[test]
    fn rebased_observation_reuses_same_worktree_without_discovery() {
        let (_dir, path) = create_dirty_git_repo();
        let source_root = path.join("packages").join("example");
        std::fs::create_dir_all(&source_root).unwrap();
        let observation = FilesystemObservation::discover(&path);

        let (result, counts) = testing::measure(|| {
            let source_observation = observation.for_root(&source_root)?;
            detect_filesystem_with_observation(
                &source_root,
                &FilesystemRequest::new()
                    .git(GitRequest::identity())
                    .without_repo()
                    .without_docs()
                    .without_formatting()
                    .without_file_inventory(),
                &source_observation,
            )
        });

        let git = result
            .unwrap()
            .git
            .expect("rebased same-worktree observation should retain Git");
        assert_eq!(
            git.repo_root.canonicalize().unwrap(),
            path.canonicalize().unwrap()
        );
        assert_eq!(counts.get(counters::GIT_DISCOVERIES), 0);

        let outside = tempfile::tempdir().unwrap();
        assert!(observation.for_root(outside.path()).is_err());

        let nested = path.join("nested-repository");
        std::fs::create_dir_all(&nested).unwrap();
        git2::Repository::init(&nested).unwrap();
        assert!(observation.for_root(&nested).is_err());
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
