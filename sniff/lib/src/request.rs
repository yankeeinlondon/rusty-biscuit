//! Per-domain request types for controlling detection detail levels.
//!
//! These types let callers express exactly which detection subsections
//! they need, avoiding the cost of expensive operations they don't use.
//!
//! ## Examples
//!
//! ```
//! use sniff::request::*;
//!
//! // Fast context: OS summary + hardware summary, skip network/filesystem
//! let plan = DetectionPlan::new()
//!     .os(OsRequest::summary())
//!     .hardware(HardwareRequest::summary())
//!     .without_network()
//!     .without_filesystem();
//!
//! // Git identity only: repo root, branch, HEAD id, worktree flag, and
//! // org/repo. No working-tree status walk, no commits, no remotes.
//! let plan = DetectionPlan::new()
//!     .without_os()
//!     .without_hardware()
//!     .without_network()
//!     .filesystem(
//!         FilesystemRequest::new()
//!             .git(GitRequest::identity())
//!             .repo(RepoRequest::structure())
//!             .without_file_inventory()
//!             .without_formatting()
//!             .without_docs(),
//!     );
//!
//! // Full audit with deep git
//! let plan = DetectionPlan::new()
//!     .filesystem(FilesystemRequest::new().git(GitRequest::deep()));
//! ```

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Top-level detection plan controlling which domains are collected
/// and at what detail level.
///
/// By default, all domains are included at **safe defaults** — which is not the
/// same as every domain at `full()`. The one difference is the live NTP probe:
/// [`DetectionPlan::default`] disables it so that observing the OS never initiates
/// an implicit network request. Ask for it explicitly with
/// `.os(OsRequest::full())`.
///
/// Use builder methods or `without_*` to exclude domains, and pass domain-specific
/// request types to control detail within each domain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectionPlan {
    /// Base directory for filesystem analysis. Falls back to cwd if None.
    pub base_dir: Option<PathBuf>,
    /// OS detection request. None skips OS detection entirely.
    pub os: Option<OsRequest>,
    /// Hardware detection request. None skips hardware detection entirely.
    pub hardware: Option<HardwareRequest>,
    /// Network detection request. None skips network detection entirely.
    pub network: Option<NetworkRequest>,
    /// Filesystem detection request. None skips filesystem detection entirely.
    pub filesystem: Option<FilesystemRequest>,
    /// Include structured performance metrics in the response payload.
    #[serde(default)]
    pub include_performance: bool,
}

impl Default for DetectionPlan {
    fn default() -> Self {
        Self {
            base_dir: None,
            // Full OS observation minus the live NTP probe: it is the only default
            // field that reaches the network, and it costs a subprocess plus a
            // round trip for a fact almost no caller of `detect()` reads. NTP is
            // one `.os(OsRequest::full())` away.
            os: Some(OsRequest::full().include_ntp_status(false)),
            hardware: Some(HardwareRequest::full()),
            network: Some(NetworkRequest::full()),
            filesystem: Some(FilesystemRequest::default()),
            include_performance: false,
        }
    }
}

impl DetectionPlan {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn base_dir(mut self, path: PathBuf) -> Self {
        self.base_dir = Some(path);
        self
    }

    pub fn os(mut self, request: OsRequest) -> Self {
        self.os = Some(request);
        self
    }

    pub fn hardware(mut self, request: HardwareRequest) -> Self {
        self.hardware = Some(request);
        self
    }

    pub fn network(mut self, request: NetworkRequest) -> Self {
        self.network = Some(request);
        self
    }

    pub fn filesystem(mut self, request: FilesystemRequest) -> Self {
        self.filesystem = Some(request);
        self
    }

    pub fn without_os(mut self) -> Self {
        self.os = None;
        self
    }

    pub fn without_hardware(mut self) -> Self {
        self.hardware = None;
        self
    }

    pub fn without_network(mut self) -> Self {
        self.network = None;
        self
    }

    pub fn without_filesystem(mut self) -> Self {
        self.filesystem = None;
        self
    }

    pub fn performance(mut self, include: bool) -> Self {
        self.include_performance = include;
        self
    }
}

/// Controls which OS subsections are collected.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OsRequest {
    /// Include system package manager detection (can be slow on Linux)
    pub include_package_managers: bool,
    /// Include locale detection
    pub include_locale: bool,
    /// Include timezone, UTC offset, and DST detection (local, cheap)
    pub include_timezone: bool,
    /// Include NTP synchronization status (bounded at 3s; network round trip
    /// on macOS only). Off in `DetectionPlan::default()`, on in `full()`.
    pub include_ntp_status: bool,
}

impl OsRequest {
    /// Core identity only: OS type, name, version, kernel, hostname.
    pub fn summary() -> Self {
        Self {
            include_package_managers: false,
            include_locale: false,
            include_timezone: false,
            include_ntp_status: false,
        }
    }

    /// Everything including package managers, locale, timezone, and NTP.
    pub fn full() -> Self {
        Self {
            include_package_managers: true,
            include_locale: true,
            include_timezone: true,
            include_ntp_status: true,
        }
    }

    pub fn include_package_managers(mut self, include: bool) -> Self {
        self.include_package_managers = include;
        self
    }

    pub fn include_locale(mut self, include: bool) -> Self {
        self.include_locale = include;
        self
    }

    pub fn include_timezone(mut self, include: bool) -> Self {
        self.include_timezone = include;
        self
    }

    pub fn include_ntp_status(mut self, include: bool) -> Self {
        self.include_ntp_status = include;
        self
    }
}

/// Controls which hardware subsections are collected.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareRequest {
    /// Include CPU detection
    pub include_cpu: bool,
    /// Include memory detection
    pub include_memory: bool,
    /// Include storage device inventory
    pub include_storage: bool,
    /// Include GPU detection
    pub include_gpu: bool,
    /// Include audio device enumeration (~1.5s on macOS)
    pub include_audio: bool,
}

impl HardwareRequest {
    /// CPU and memory only. Skips storage, GPU, and audio (~1.5s savings on macOS).
    pub fn summary() -> Self {
        Self {
            include_cpu: true,
            include_memory: true,
            include_storage: false,
            include_gpu: false,
            include_audio: false,
        }
    }

    /// Full hardware detection including storage, GPU, and audio devices.
    pub fn full() -> Self {
        Self {
            include_cpu: true,
            include_memory: true,
            include_storage: true,
            include_gpu: true,
            include_audio: true,
        }
    }

    pub fn include_cpu(mut self, include: bool) -> Self {
        self.include_cpu = include;
        self
    }

    pub fn include_memory(mut self, include: bool) -> Self {
        self.include_memory = include;
        self
    }

    pub fn include_storage(mut self, include: bool) -> Self {
        self.include_storage = include;
        self
    }

    pub fn include_gpu(mut self, include: bool) -> Self {
        self.include_gpu = include;
        self
    }

    pub fn include_audio(mut self, include: bool) -> Self {
        self.include_audio = include;
        self
    }
}

/// Controls which network subsections are collected.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkRequest {
    /// Include WAN IP lookup (HTTP call to external service)
    pub include_wan_ip: bool,
    /// Force a fresh WAN IP lookup, bypassing the TTL cache.
    /// Only meaningful when `include_wan_ip` is true.
    #[serde(default)]
    pub force_refresh: bool,
}

impl NetworkRequest {
    /// Local interfaces only. No external HTTP call.
    pub fn interfaces_only() -> Self {
        Self {
            include_wan_ip: false,
            force_refresh: false,
        }
    }

    /// Full network detection including WAN IP lookup.
    pub fn full() -> Self {
        Self {
            include_wan_ip: true,
            force_refresh: false,
        }
    }

    pub fn include_wan_ip(mut self, include: bool) -> Self {
        self.include_wan_ip = include;
        self
    }

    /// Bypass the WAN IP cache and perform a fresh lookup.
    pub fn force_refresh(mut self, refresh: bool) -> Self {
        self.force_refresh = refresh;
        self
    }
}

/// Fine-grained controls for the repository metadata a git request collects.
///
/// Each flag names one independently expensive observation. A caller that wants
/// recent commits but not, say, the per-branch divergence walk sets only what it
/// renders.
///
/// This is only consulted when a [`GitRequest`] carries it. Absence means
/// "derive the legacy behavior from the coarse fields", never "collect nothing"
/// — see [`GitRequest::metadata`].
///
/// ## Examples
///
/// ```
/// use sniff::request::{GitMetadataRequest, GitRequest};
///
/// // Ten commits, and none of the branch/remote/config/worktree work that
/// // `full()` would otherwise imply.
/// let request = GitRequest::full()
///     .metadata(GitMetadataRequest::none().commits(true));
/// assert!(request.wants_commits());
/// assert!(!request.wants_branches());
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitMetadataRequest {
    /// Collect recent commits (bounded by [`GitRequest::commit_count`]).
    pub commits: bool,
    /// Attach ref decorations (branches/tags pointing at a returned commit).
    pub ref_decorations: bool,
    /// Collect the local branch list.
    pub branches: bool,
    /// Compute per-branch ahead/behind against the current branch.
    ///
    /// Separate from [`branches`](Self::branches) because it walks the commit
    /// graph once per branch, which dominates latency on branch-heavy repos.
    pub branch_divergence: bool,
    /// Collect configured remotes.
    pub remotes: bool,
    /// Compute per-remote tracking (ahead/behind) status.
    pub tracking: bool,
    /// Read git config (user identity, signing, pager).
    pub config: bool,
    /// Enumerate linked worktrees.
    pub worktrees: bool,
    /// Collect the library-owned evidence for bare aggregate projection.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub aggregate: bool,
}

impl GitMetadataRequest {
    /// No metadata at all — the base for opting in to exactly one thing.
    pub fn none() -> Self {
        Self::default()
    }

    /// Every metadata observation.
    pub fn all() -> Self {
        Self {
            commits: true,
            ref_decorations: true,
            branches: true,
            branch_divergence: true,
            remotes: true,
            tracking: true,
            config: true,
            worktrees: true,
            aggregate: false,
        }
    }

    /// Collect recent commits.
    pub fn commits(mut self, include: bool) -> Self {
        self.commits = include;
        self
    }

    /// Attach ref decorations to returned commits.
    pub fn ref_decorations(mut self, include: bool) -> Self {
        self.ref_decorations = include;
        self
    }

    /// Collect the local branch list.
    pub fn branches(mut self, include: bool) -> Self {
        self.branches = include;
        self
    }

    /// Compute per-branch ahead/behind counts.
    pub fn branch_divergence(mut self, include: bool) -> Self {
        self.branch_divergence = include;
        self
    }

    /// Collect configured remotes.
    pub fn remotes(mut self, include: bool) -> Self {
        self.remotes = include;
        self
    }

    /// Compute per-remote tracking status.
    pub fn tracking(mut self, include: bool) -> Self {
        self.tracking = include;
        self
    }

    /// Read git config.
    pub fn config(mut self, include: bool) -> Self {
        self.config = include;
        self
    }

    /// Enumerate linked worktrees.
    pub fn worktrees(mut self, include: bool) -> Self {
        self.worktrees = include;
        self
    }

    /// Collect aggregate-only branch, worktree, and file-aware history shapes.
    pub fn aggregate(mut self, include: bool) -> Self {
        self.aggregate = include;
        self
    }
}

/// Controls git repository detection detail level.
///
/// Presets range from [`identity()`](Self::identity) — the cheapest,
/// status-free floor — through [`summary()`](Self::summary),
/// [`full()`](Self::full), and [`deep()`](Self::deep). The
/// [`identity_only`](Self::identity_only) field discriminates the
/// identity level from the all-false `minimal()`/`summary()` presets; see
/// [`is_identity_only`](Self::is_identity_only) for the public predicate.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitRequest {
    /// Number of recent commits to retrieve (0 = skip commit history)
    pub commit_count: usize,
    /// Include per-file change details (paths, status, line counts)
    pub include_file_changes: bool,
    /// Include full unified diff payloads in `RepoStatus.dirty` and `RepoStatus.untracked`.
    /// Only effective when `include_file_changes` is also true.
    pub include_file_diffs: bool,
    /// Include worktree enumeration and status
    pub include_worktrees: bool,
    /// Fetch remote tracking refs (requires network)
    pub refresh_remote_tracking: bool,
    /// Include remote branch details (requires refresh_remote_tracking)
    pub include_remote_branch_details: bool,
    /// Check which remotes contain recent commits (requires refresh_remote_tracking)
    pub include_commit_remote_containment: bool,
    /// Maximum number of remote-tracking branches to inspect for commit
    /// containment. Lower values reduce deep-git latency for repos with many
    /// remote branches. `None` means no limit.
    pub max_remote_branches: Option<usize>,
    /// Compute ahead/behind, merge status, and conflict detection for every
    /// linked worktree. When `false` (default), only the current worktree
    /// receives full detail; all other linked worktrees skip the expensive
    /// commit graph walks.
    pub full_worktree_details: bool,
    /// When true, return only repository identity (root, branch, HEAD id,
    /// worktree flag, base root, org/repo) and skip every expensive walk,
    /// including the working-tree status walk.
    ///
    /// This is set by [`Self::identity`]; setting it manually on any other
    /// preset yields identity semantics (early return). Defaults to `false`
    /// so older serialized plans deserialize to the pre-identity behavior.
    #[serde(default)]
    pub identity_only: bool,
    /// Fine-grained metadata controls, or `None` to derive them from the coarse
    /// fields above.
    ///
    /// `None` means "legacy behavior", not "want nothing" — the distinction the
    /// `wants_*` accessors exist to enforce. Every preset leaves this `None`, so
    /// `skip_serializing_if` keeps their serialized JSON byte-identical to what
    /// it was before this field existed, and `default` lets plans serialized
    /// before it existed deserialize unchanged.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<GitMetadataRequest>,
}

impl GitRequest {
    /// Absolute minimum: branch name and dirty yes/no.
    /// No counts, no branches, no remotes, no config, no tracking.
    pub fn minimal() -> Self {
        Self {
            commit_count: 0,
            include_file_changes: false,
            include_file_diffs: false,
            include_worktrees: false,
            refresh_remote_tracking: false,
            include_remote_branch_details: false,
            include_commit_remote_containment: false,
            max_remote_branches: None,
            full_worktree_details: false,
            identity_only: false,
            metadata: None,
        }
    }

    /// Repository identity only: root, branch, HEAD id, worktree flag, base
    /// repo root, and org/repo from the preferred remote. No working-tree
    /// status walk, no commits, no branches, no remotes, no config.
    ///
    /// This is the cheapest git request level and is the new floor below
    /// [`minimal`](Self::minimal) and [`summary`](Self::summary).
    pub fn identity() -> Self {
        Self {
            commit_count: 0,
            include_file_changes: false,
            include_file_diffs: false,
            include_worktrees: false,
            refresh_remote_tracking: false,
            include_remote_branch_details: false,
            include_commit_remote_containment: false,
            max_remote_branches: None,
            full_worktree_details: false,
            identity_only: true,
            metadata: None,
        }
    }

    /// Branch + dirty yes/no flag. No per-category counts, no commits, no file
    /// details, no worktrees.
    ///
    /// Currently produces the same field set as [`minimal`](Self::minimal):
    /// both satisfy [`is_minimal`](Self::is_minimal), so detection takes the
    /// dirty-flag-only path and leaves `staged_count` / `unstaged_count` /
    /// `untracked_count` at `0`. Use a request that is *not* minimal (e.g. with
    /// `commit_count > 0`) to populate per-category counts.
    pub fn summary() -> Self {
        Self {
            commit_count: 0,
            include_file_changes: false,
            include_file_diffs: false,
            include_worktrees: false,
            refresh_remote_tracking: false,
            include_remote_branch_details: false,
            include_commit_remote_containment: false,
            max_remote_branches: None,
            full_worktree_details: false,
            identity_only: false,
            metadata: None,
        }
    }

    /// Standard detection with 10 commits, file change stats (paths and line counts),
    /// worktrees, but no unified diff payloads and no remote refresh.
    ///
    /// Worktrees are enumerated, but expensive ahead/behind and merge-conflict
    /// probes are skipped for non-current linked worktrees. Use
    /// [`Self::deep()`] or call `.full_worktree_details(true)` to force full
    /// detail for every worktree.
    pub fn full() -> Self {
        Self {
            commit_count: 10,
            include_file_changes: true,
            include_file_diffs: false,
            include_worktrees: true,
            refresh_remote_tracking: false,
            include_remote_branch_details: false,
            include_commit_remote_containment: false,
            max_remote_branches: None,
            full_worktree_details: false,
            identity_only: false,
            metadata: None,
        }
    }

    /// Deep detection: refreshes remote tracking refs, populates remote info on
    /// commits, and includes full unified diff payloads for dirty and untracked files.
    pub fn deep() -> Self {
        Self {
            commit_count: 10,
            include_file_changes: true,
            include_file_diffs: true,
            include_worktrees: true,
            refresh_remote_tracking: true,
            include_remote_branch_details: true,
            include_commit_remote_containment: true,
            max_remote_branches: Some(50),
            full_worktree_details: true,
            identity_only: false,
            metadata: None,
        }
    }

    pub fn commit_count(mut self, count: usize) -> Self {
        self.commit_count = count;
        self
    }

    pub fn include_file_changes(mut self, include: bool) -> Self {
        self.include_file_changes = include;
        self
    }

    pub fn include_file_diffs(mut self, include: bool) -> Self {
        self.include_file_diffs = include;
        self
    }

    pub fn include_worktrees(mut self, include: bool) -> Self {
        self.include_worktrees = include;
        self
    }

    pub fn refresh_remote_tracking(mut self, include: bool) -> Self {
        self.refresh_remote_tracking = include;
        self
    }

    /// Returns true when the request is so minimal that branches, remotes,
    /// config, and tracking can be skipped entirely.
    pub fn is_minimal(&self) -> bool {
        !self.identity_only
            && self.commit_count == 0
            && !self.include_file_changes
            && !self.include_worktrees
            && !self.refresh_remote_tracking
    }

    /// Returns true when only repository identity should be returned, with no
    /// working-tree status walk and no metadata collection.
    pub fn is_identity_only(&self) -> bool {
        self.identity_only
    }

    /// Whether this request needs repo metadata enrichment — remotes, git
    /// config, local branches, and tracking status.
    ///
    /// Computing local branches walks `graph_ahead_behind` once per branch,
    /// which dominates latency on repos with many branches. A pure file-change
    /// query (`include_file_changes` with no commits, worktrees, or remote
    /// refresh) maps changed files to packages and never renders any of that
    /// metadata, so it must skip the walk — unlike [`is_minimal`], whose
    /// contract treats `include_file_changes` as non-minimal.
    ///
    /// [`is_minimal`]: Self::is_minimal
    pub fn wants_repo_metadata(&self) -> bool {
        self.commit_count > 0 || self.include_worktrees || self.refresh_remote_tracking
    }

    pub fn max_remote_branches(mut self, limit: Option<usize>) -> Self {
        self.max_remote_branches = limit;
        self
    }

    pub fn full_worktree_details(mut self, full: bool) -> Self {
        self.full_worktree_details = full;
        self
    }

    /// Opt into fine-grained metadata controls.
    ///
    /// Once set, the controls decide what metadata is collected instead of the
    /// coarse fields; the coarse fields still govern *how much* (e.g.
    /// [`commit_count`](Self::commit_count) still bounds the commit walk).
    pub fn metadata(mut self, metadata: GitMetadataRequest) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Collect recent commits?
    pub fn wants_commits(&self) -> bool {
        match self.metadata {
            Some(m) => m.commits && self.commit_count > 0,
            None => self.commit_count > 0,
        }
    }

    /// Attach ref decorations to returned commits?
    ///
    /// Legacy behavior decorates whenever commits are collected.
    pub fn wants_ref_decorations(&self) -> bool {
        match self.metadata {
            Some(m) => m.ref_decorations && self.wants_commits(),
            None => self.wants_commits(),
        }
    }

    /// Collect the local branch list?
    pub fn wants_branches(&self) -> bool {
        match self.metadata {
            Some(m) => m.branches,
            None => self.wants_repo_metadata(),
        }
    }

    /// Compute per-branch ahead/behind counts?
    ///
    /// Legacy behavior ties divergence to the branch list, and the presets'
    /// published values depend on it, so `full()` keeps paying for it unless a
    /// caller explicitly opts out.
    pub fn wants_branch_divergence(&self) -> bool {
        match self.metadata {
            Some(m) => m.branch_divergence,
            None => self.wants_repo_metadata(),
        }
    }

    /// Collect configured remotes?
    pub fn wants_remotes(&self) -> bool {
        match self.metadata {
            Some(m) => m.remotes,
            None => self.wants_repo_metadata(),
        }
    }

    /// Compute per-remote tracking status?
    pub fn wants_tracking(&self) -> bool {
        match self.metadata {
            Some(m) => m.tracking,
            None => self.wants_repo_metadata(),
        }
    }

    /// Read git config?
    pub fn wants_config(&self) -> bool {
        match self.metadata {
            Some(m) => m.config,
            None => self.wants_repo_metadata(),
        }
    }

    /// Enumerate linked worktrees?
    pub fn wants_worktrees(&self) -> bool {
        match self.metadata {
            Some(m) => m.worktrees && self.include_worktrees,
            None => self.include_worktrees,
        }
    }

    /// Collect the library-owned bare aggregate evidence?
    pub fn wants_aggregate(&self) -> bool {
        self.metadata.is_some_and(|metadata| metadata.aggregate)
    }
}

/// Focused package details that can be added to a shallow repository request.
///
/// Identity and topology are always collected. These controls opt into facts
/// that otherwise belong to [`RepoRequest::full`] without enabling its file
/// inventory, language, framework, feature, or file-list work.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RepoDetailRequest {
    /// Detect package managers from ecosystem markers and lockfiles.
    pub package_managers: bool,
    /// Parse external and workspace-internal dependencies.
    pub dependencies: bool,
    /// Detect declared test runners and their evidence.
    pub test_runners: bool,
}

impl RepoDetailRequest {
    /// Package-manager detection only.
    pub fn package_managers() -> Self {
        Self {
            package_managers: true,
            ..Self::default()
        }
    }

    /// Dependency parsing only.
    pub fn dependencies() -> Self {
        Self {
            dependencies: true,
            ..Self::default()
        }
    }

    /// Test-runner detection only.
    pub fn test_runners() -> Self {
        Self {
            test_runners: true,
            ..Self::default()
        }
    }

    /// Every focused detail used by repository aggregate projections.
    pub fn all() -> Self {
        Self {
            package_managers: true,
            dependencies: true,
            test_runners: true,
        }
    }
}

/// Controls repo/monorepo detection detail level.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoRequest {
    /// When true, skip inventory-backed package enrichment.
    /// When false, collect the complete repository detail set.
    pub structure_only: bool,
    /// Optional enrichment for callers that need selected package facts but
    /// not inventory-backed full repository detail.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<RepoDetailRequest>,
}

impl RepoRequest {
    /// Workspace topology and minimum package identity only.
    ///
    /// Package managers, dependencies, test runners, features, languages,
    /// frameworks, and file lists are empty. Use [`Self::focused`] for one of
    /// the inexpensive manifest-backed detail sets, or [`Self::full`] for all
    /// enriched fields.
    pub fn structure() -> Self {
        Self {
            structure_only: true,
            details: None,
        }
    }

    /// Full repo detection with per-package language and framework scanning.
    pub fn full() -> Self {
        Self {
            structure_only: false,
            details: None,
        }
    }

    /// Structure plus selected manifest-backed package details.
    pub fn focused(details: RepoDetailRequest) -> Self {
        Self {
            structure_only: true,
            details: Some(details),
        }
    }

    /// Whether package-manager detection is requested.
    pub fn wants_package_managers(&self) -> bool {
        !self.structure_only
            || self
                .details
                .is_some_and(|details| details.package_managers)
    }

    /// Whether dependency parsing is requested.
    pub fn wants_dependencies(&self) -> bool {
        !self.structure_only || self.details.is_some_and(|details| details.dependencies)
    }

    /// Whether declared test-runner detection is requested.
    pub fn wants_test_runners(&self) -> bool {
        !self.structure_only || self.details.is_some_and(|details| details.test_runners)
    }

    /// Whether any manifest-backed package enrichment is requested.
    pub fn wants_package_enrichment(&self) -> bool {
        self.wants_package_managers() || self.wants_dependencies() || self.wants_test_runners()
    }
}

/// Controls filesystem detection composition.
///
/// Composes sub-requests for git, repo, file inventory, formatting, and docs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesystemRequest {
    /// Git detection request. None skips git entirely.
    pub git: Option<GitRequest>,
    /// Repo/monorepo detection request. None skips repo detection.
    pub repo: Option<RepoRequest>,
    /// Include file inventory and language breakdown
    pub include_file_inventory: bool,
    /// Include EditorConfig formatting detection
    pub include_formatting: bool,
    /// Include markdown document discovery
    pub include_docs: bool,
}

impl Default for FilesystemRequest {
    fn default() -> Self {
        Self {
            git: Some(GitRequest::full()),
            repo: Some(RepoRequest::full()),
            include_file_inventory: true,
            include_formatting: true,
            include_docs: true,
        }
    }
}

impl FilesystemRequest {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn git(mut self, request: GitRequest) -> Self {
        self.git = Some(request);
        self
    }

    pub fn repo(mut self, request: RepoRequest) -> Self {
        self.repo = Some(request);
        self
    }

    pub fn without_git(mut self) -> Self {
        self.git = None;
        self
    }

    pub fn without_repo(mut self) -> Self {
        self.repo = None;
        self
    }

    pub fn without_docs(mut self) -> Self {
        self.include_docs = false;
        self
    }

    pub fn without_formatting(mut self) -> Self {
        self.include_formatting = false;
        self
    }

    pub fn without_file_inventory(mut self) -> Self {
        self.include_file_inventory = false;
        self
    }

    pub fn include_docs(mut self, include: bool) -> Self {
        self.include_docs = include;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The `metadata` field must be invisible in every preset's JSON.
    ///
    /// This is the R9.1/R9.8 compatibility contract: adding fine-grained
    /// controls must not change the wire shape of any existing preset.
    #[test]
    fn preset_json_omits_the_metadata_field() {
        for (label, request) in [
            ("identity", GitRequest::identity()),
            ("minimal", GitRequest::minimal()),
            ("summary", GitRequest::summary()),
            ("full", GitRequest::full()),
            ("deep", GitRequest::deep()),
        ] {
            let value: serde_json::Value =
                serde_json::from_str(&serde_json::to_string(&request).unwrap()).unwrap();
            assert!(
                !value.as_object().unwrap().contains_key("metadata"),
                "{label}() must serialize without a metadata field"
            );
        }
    }

    /// A plan serialized before `metadata` existed must still deserialize, and
    /// must derive the behavior it had then.
    #[test]
    fn legacy_request_json_without_metadata_derives_legacy_behavior() {
        let legacy = r#"{
            "commit_count": 10,
            "include_file_changes": true,
            "include_file_diffs": false,
            "include_worktrees": true,
            "refresh_remote_tracking": false,
            "include_remote_branch_details": false,
            "include_commit_remote_containment": false,
            "max_remote_branches": null,
            "full_worktree_details": false
        }"#;

        let request: GitRequest = serde_json::from_str(legacy).unwrap();
        assert!(request.metadata.is_none(), "absent field must stay absent");
        // Absence means "derive from the coarse fields", not "want nothing".
        assert!(request.wants_commits());
        assert!(request.wants_branches());
        assert!(request.wants_remotes());
        assert!(request.wants_config());
        assert!(request.wants_worktrees());
    }

    #[test]
    fn focused_metadata_controls_round_trip() {
        let request = GitRequest::full().metadata(
            GitMetadataRequest::none()
                .commits(true)
                .ref_decorations(true),
        );
        let round_tripped: GitRequest =
            serde_json::from_str(&serde_json::to_string(&request).unwrap()).unwrap();

        assert_eq!(round_tripped.metadata, request.metadata);
        assert!(round_tripped.wants_commits());
        assert!(round_tripped.wants_ref_decorations());
        assert!(!round_tripped.wants_branches());
        assert!(!round_tripped.wants_config());
    }

    /// R9.2: asking for commits must not drag in branch, remote, tracking,
    /// config, or worktree work.
    #[test]
    fn focused_commit_request_wants_nothing_else() {
        let request = GitRequest::full().metadata(GitMetadataRequest::none().commits(true));

        assert!(request.wants_commits());
        for (label, wanted) in [
            ("branches", request.wants_branches()),
            ("branch_divergence", request.wants_branch_divergence()),
            ("remotes", request.wants_remotes()),
            ("tracking", request.wants_tracking()),
            ("config", request.wants_config()),
            ("worktrees", request.wants_worktrees()),
        ] {
            assert!(!wanted, "focused commit request must not want {label}");
        }
    }

    /// The coarse fields still bound the controls: opting into commits cannot
    /// resurrect a walk a `commit_count` of zero already ruled out.
    #[test]
    fn metadata_controls_cannot_widen_past_coarse_fields() {
        let request = GitRequest::summary().metadata(GitMetadataRequest::all());
        assert!(!request.wants_commits(), "commit_count 0 still wins");
        assert!(!request.wants_worktrees(), "include_worktrees false still wins");
    }

    #[test]
    fn zero_scan_limit_is_rejected_in_favor_of_the_default() {
        use crate::filesystem::git::{DEFAULT_PATH_HISTORY_SCAN_LIMIT, PathHistoryOptions};

        // A zero bound would return an empty history indistinguishable from
        // "never touched" — the exact confusion the bounded API removes.
        let opts = PathHistoryOptions::new(10).scan_limit(0);
        assert_eq!(opts.scan_limit_value(), DEFAULT_PATH_HISTORY_SCAN_LIMIT);
    }

    #[test]
    fn detection_plan_defaults_to_all_domains() {
        let plan = DetectionPlan::default();
        assert!(plan.os.is_some());
        assert!(plan.hardware.is_some());
        assert!(plan.network.is_some());
        assert!(plan.filesystem.is_some());
    }

    /// The default plan is "all domains at safe defaults", not "every domain at
    /// `full()`". The live NTP probe is the sole difference: it is the only OS
    /// default that reaches the network.
    #[test]
    fn default_plan_makes_no_ntp_request() {
        let os = DetectionPlan::default().os.expect("os domain is on by default");

        assert!(!os.include_ntp_status, "default plan must not probe NTP");
        // Every other full() field survives — this gates one probe, it does not
        // downgrade the default plan to summary().
        assert!(os.include_package_managers);
        assert!(os.include_locale);
        assert!(os.include_timezone);
    }

    /// R12.1: gating the default must not change what an explicit `full()` means.
    #[test]
    fn explicit_full_os_request_retains_ntp() {
        let plan = DetectionPlan::new().os(OsRequest::full());
        assert!(plan.os.expect("explicitly set").include_ntp_status);
    }

    #[test]
    fn detection_plan_skip_sections() {
        let plan = DetectionPlan::new().without_os().without_hardware();
        assert!(plan.os.is_none());
        assert!(plan.hardware.is_none());
        assert!(plan.network.is_some());
        assert!(plan.filesystem.is_some());
    }

    #[test]
    fn os_request_summary_vs_full() {
        let summary = OsRequest::summary();
        assert!(!summary.include_package_managers);
        assert!(!summary.include_locale);
        assert!(!summary.include_timezone);
        assert!(!summary.include_ntp_status);

        let full = OsRequest::full();
        assert!(full.include_package_managers);
        assert!(full.include_locale);
        assert!(full.include_timezone);
        assert!(full.include_ntp_status);
    }

    #[test]
    fn hardware_request_detail_levels() {
        let summary = HardwareRequest::summary();
        assert!(summary.include_cpu);
        assert!(summary.include_memory);
        assert!(!summary.include_storage);
        assert!(!summary.include_gpu);
        assert!(!summary.include_audio);

        let full = HardwareRequest::full();
        assert!(full.include_cpu);
        assert!(full.include_memory);
        assert!(full.include_storage);
        assert!(full.include_gpu);
        assert!(full.include_audio);

        let gpu_only = HardwareRequest::summary()
            .include_gpu(true)
            .include_cpu(false)
            .include_memory(false);
        assert!(!gpu_only.include_cpu);
        assert!(!gpu_only.include_memory);
        assert!(gpu_only.include_gpu);
    }

    #[test]
    fn network_request_interfaces_only() {
        let req = NetworkRequest::interfaces_only();
        assert!(!req.include_wan_ip);

        let full = NetworkRequest::full();
        assert!(full.include_wan_ip);
    }

    #[test]
    fn git_request_identity_preset() {
        let identity = GitRequest::identity();
        assert!(identity.identity_only);
        assert!(identity.is_identity_only());
        assert!(!identity.is_minimal());
        assert!(!identity.wants_repo_metadata());

        // Existing presets stay non-identity.
        assert!(!GitRequest::minimal().is_identity_only());
        assert!(!GitRequest::summary().is_identity_only());
        assert!(!GitRequest::full().is_identity_only());
        assert!(!GitRequest::deep().is_identity_only());
    }

    #[test]
    fn git_request_detail_levels() {
        let minimal = GitRequest::minimal();
        assert_eq!(minimal.commit_count, 0);
        assert!(!minimal.include_file_changes);
        assert!(!minimal.include_file_diffs);
        assert!(!minimal.include_worktrees);
        assert!(!minimal.refresh_remote_tracking);
        assert!(minimal.is_minimal());

        let summary = GitRequest::summary();
        assert_eq!(summary.commit_count, 0);
        assert!(!summary.include_file_changes);
        assert!(!summary.include_file_diffs);
        assert!(!summary.include_worktrees);
        assert!(!summary.refresh_remote_tracking);

        let full = GitRequest::full();
        assert_eq!(full.commit_count, 10);
        assert!(full.include_file_changes);
        assert!(!full.include_file_diffs);
        assert!(full.include_worktrees);
        assert!(!full.refresh_remote_tracking);
        assert!(!full.is_minimal());

        let deep = GitRequest::deep();
        assert!(deep.include_file_changes);
        assert!(deep.include_file_diffs);
        assert!(deep.refresh_remote_tracking);
        assert!(deep.include_remote_branch_details);
        assert!(deep.include_commit_remote_containment);
        assert!(!deep.is_minimal());
    }

    #[test]
    fn git_request_is_minimal_flags() {
        assert!(GitRequest::minimal().is_minimal());
        assert!(GitRequest::summary().is_minimal());
        assert!(!GitRequest::full().is_minimal());
        assert!(!GitRequest::deep().is_minimal());

        let with_commits = GitRequest::minimal().commit_count(1);
        assert!(!with_commits.is_minimal());

        let with_changes = GitRequest::minimal().include_file_changes(true);
        assert!(!with_changes.is_minimal());
    }

    #[test]
    fn git_request_wants_repo_metadata_flags() {
        // A pure file-change query must NOT pull repo metadata (branches etc.),
        // even though `include_file_changes` makes it non-minimal.
        let changes_only = GitRequest::summary().include_file_changes(true);
        assert!(!changes_only.is_minimal());
        assert!(!changes_only.wants_repo_metadata());

        // Plain summary and minimal want no metadata either.
        assert!(!GitRequest::summary().wants_repo_metadata());
        assert!(!GitRequest::minimal().wants_repo_metadata());

        // full() (worktrees) and deep() (remote refresh) still render branches
        // and remotes, so they must keep the metadata enrichment.
        assert!(GitRequest::full().wants_repo_metadata());
        assert!(GitRequest::deep().wants_repo_metadata());
        assert!(GitRequest::minimal().commit_count(1).wants_repo_metadata());
    }

    #[test]
    fn filesystem_request_composition() {
        let req = FilesystemRequest::new()
            .git(GitRequest::summary())
            .without_docs()
            .without_repo();
        assert!(req.git.is_some());
        assert!(req.repo.is_none());
        assert!(!req.include_docs);
    }

    #[test]
    fn repo_request_structure_vs_full() {
        let structure = RepoRequest::structure();
        assert!(structure.structure_only);
        assert!(structure.details.is_none());
        assert!(!structure.wants_package_enrichment());

        let full = RepoRequest::full();
        assert!(!full.structure_only);
        assert!(full.details.is_none());
        assert!(full.wants_package_managers());
        assert!(full.wants_dependencies());
        assert!(full.wants_test_runners());
    }

    #[test]
    fn focused_repo_details_narrow_package_enrichment() {
        let cases = [
            (
                RepoDetailRequest::package_managers(),
                (true, false, false),
            ),
            (RepoDetailRequest::dependencies(), (false, true, false)),
            (RepoDetailRequest::test_runners(), (false, false, true)),
            (RepoDetailRequest::all(), (true, true, true)),
        ];

        for (details, expected) in cases {
            let request = RepoRequest::focused(details);
            assert!(request.structure_only);
            assert_eq!(
                (
                    request.wants_package_managers(),
                    request.wants_dependencies(),
                    request.wants_test_runners(),
                ),
                expected
            );
        }
    }

    #[test]
    fn legacy_repo_request_json_omits_and_defaults_focused_details() {
        let structure_json = serde_json::to_value(RepoRequest::structure()).unwrap();
        let full_json = serde_json::to_value(RepoRequest::full()).unwrap();
        assert!(structure_json.get("details").is_none());
        assert!(full_json.get("details").is_none());

        let legacy: RepoRequest =
            serde_json::from_str(r#"{"structure_only":true}"#).unwrap();
        assert_eq!(legacy.details, None);
        assert!(!legacy.wants_package_enrichment());

        let focused = RepoRequest::focused(RepoDetailRequest::dependencies());
        let roundtrip: RepoRequest =
            serde_json::from_value(serde_json::to_value(focused).unwrap()).unwrap();
        assert!(roundtrip.wants_dependencies());
        assert!(!roundtrip.wants_package_managers());
    }

    #[test]
    fn detection_plan_serialization_roundtrip() {
        let plan = DetectionPlan::new()
            .os(OsRequest::summary())
            .without_network()
            .filesystem(
                FilesystemRequest::new()
                    .git(GitRequest::deep().commit_count(5))
                    .repo(RepoRequest::structure()),
            );

        let json = serde_json::to_string(&plan).unwrap();
        let parsed: DetectionPlan = serde_json::from_str(&json).unwrap();

        assert!(parsed.os.is_some());
        assert!(parsed.network.is_none());
        let fs = parsed.filesystem.unwrap();
        assert_eq!(fs.git.unwrap().commit_count, 5);
        assert!(fs.repo.unwrap().structure_only);
    }

    #[test]
    fn git_request_identity_deserializes_from_old_payload() {
        // Old serialized plans omit identity_only; serde(default) must keep
        // them working instead of erroring.
        let json = r#"{
            "commit_count": 0,
            "include_file_changes": false,
            "include_file_diffs": false,
            "include_worktrees": false,
            "refresh_remote_tracking": false,
            "include_remote_branch_details": false,
            "include_commit_remote_containment": false,
            "max_remote_branches": null,
            "full_worktree_details": false
        }"#;
        let parsed: GitRequest = serde_json::from_str(json).unwrap();
        assert!(!parsed.is_identity_only());
        assert!(parsed.is_minimal());
    }

    #[test]
    fn hardware_request_serialization_roundtrip() {
        let gpu_only = HardwareRequest {
            include_cpu: false,
            include_memory: false,
            include_storage: false,
            include_gpu: true,
            include_audio: false,
        };
        let json = serde_json::to_string(&gpu_only).unwrap();
        let parsed: HardwareRequest = serde_json::from_str(&json).unwrap();
        assert!(!parsed.include_cpu);
        assert!(!parsed.include_memory);
        assert!(parsed.include_gpu);
    }
}
