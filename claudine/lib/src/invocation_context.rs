//! Request-scoped launch and repository evidence for Claudine invocations.

use std::collections::{BTreeMap, HashMap};
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use biscuit_file::{FileResolutionContext, LaunchMagicScope, PackageAreaFallback, RepositoryScopeCatalog};
use sniff::filesystem::{FilesystemObservation, GitRepositoryIdentity};
use sniff::filesystem::docs::MarkdownMeta;
use sniff::filesystem::git::{FileChange, GitInfo, RecentCommits};
use sniff::filesystem::repo::RepoInfo;
use sniff::filesystem::LanguageBreakdown;
use sniff::hardware::{GpuInfo, HardwareInfo};
use sniff::os::OsInfo;
use sniff::request::{
    DetectionPlan, FilesystemRequest, GitMetadataRequest, GitRequest, HardwareRequest, OsRequest, RepoRequest,
};

use darkmatter::markdown::compose::{
    ContextBuildError, CurrentProvider, CurrentRefresh, RequestSnapshot,
    build_resolution_context_with_catalog,
};

use crate::composition::{
    LaunchWorkspaceContext, with_prompt_magic_roots,
};
use crate::diagnostics::DiagnosticSnapshot;
use crate::error::ClaudineError;
use crate::events::{
    EnvironmentContext, environment_context_from_sniff_result_and_env,
};
use crate::system_prompt::LaunchContext;

/// Failure to establish immutable invocation inputs.
#[derive(Debug, thiserror::Error)]
pub enum InvocationContextError {
    /// The launch or source file-resolution context could not be built.
    #[error(transparent)]
    ResolutionContext(#[from] ContextBuildError),
    /// A resolved source does not have an authoring directory.
    #[error("resolved source path has no parent directory: {0}")]
    SourceWithoutParent(PathBuf),
}

/// Canonical seams that consume a document epoch's prepared context.
///
/// These names are part of invocation work accounting: tests compare the
/// observed set for a route with its expected semantic consumers. Recording a
/// seam at its owner proves the snapshot was populated there; aggregate launch
/// construction counts alone cannot distinguish reuse from equal recapture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PreparedContextConsumer {
    /// Resolve-once preparation and shell discovery.
    Preflight,
    /// Prompt body composition.
    Body,
    /// Effective-frontmatter composition.
    EffectiveFrontmatter,
    /// A loop gate using the prepared context.
    LoopCondition,
    /// Event-time lifecycle interpolation.
    Lifecycle,
}

impl PreparedContextConsumer {
    /// These names are emitted in performance reports and form the stable
    /// vocabulary used by exact route-consumer assertions.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Preflight => "preflight",
            Self::Body => "body",
            Self::EffectiveFrontmatter => "effective-frontmatter",
            Self::LoopCondition => "loop-condition",
            Self::Lifecycle => "lifecycle",
        }
    }
}

/// Request-local accounting for discovery and preparation work.
///
/// Every requested group lands in exactly one of three per-group maps:
///
/// - `runtime_evidence_captures`: calls that ran a *stable* group's
///   initializer. Stable evidence (host facts, languages, documents) is
///   observed at most once per invocation, so a second capture is a duplicate
///   computation rather than an invisible repeat.
/// - `volatile_observations`: calls that observed a *volatile* group (Git
///   working state: `git`, `file_changes`, `git_history`) for a composition
///   run. These are expected once per requesting run, never more.
/// - `runtime_evidence_reuses`: calls that retained evidence answered.
///
/// A group requested `n` times therefore sums to `n` across the three maps.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InvocationWorkSnapshot {
    pub git_root_discoveries: usize,
    pub topology_probes: usize,
    pub topology_reuses: usize,
    pub system_prompt_lookups: usize,
    pub compose_operations: usize,
    pub harness_eligibility_parses: usize,
    pub harness_materializations: usize,
    pub ambient_fallbacks: usize,
    /// Launch-capture constructions: full `ComposeContext` snapshots built by
    /// [`InvocationContext::capture_launch_context`]. One per document
    /// preparation epoch is the contract; more means a route recaptured.
    pub launch_context_constructions: usize,
    /// Same-epoch group extensions: missing-requirement projections applied to
    /// an already-constructed snapshot by
    /// [`InvocationContext::extend_launch_context`]. Expected only from a
    /// stabilized reread whose document grew new `ctx.*` groups.
    pub launch_context_extensions: usize,
    /// Populated prepared-context observations grouped by canonical consumer.
    ///
    /// Values retain repeat observations while callers that need the semantic
    /// route contract can compare the deterministic map keys as a set.
    pub prepared_context_consumers: BTreeMap<String, usize>,
    /// Work keyed by its intrinsically attributable document epoch.
    ///
    /// Unlike invocation-global deltas, these records remain exact when
    /// sibling sequence tasks prepare concurrently.
    pub document_epochs: BTreeMap<usize, DocumentEpochWork>,
    pub runtime_evidence_captures: BTreeMap<String, usize>,
    pub runtime_evidence_reuses: BTreeMap<String, usize>,
    pub volatile_observations: BTreeMap<String, usize>,
    pub system_prompt_timings: BTreeMap<String, std::time::Duration>,
}

/// Work shape for one document preparation epoch.
///
/// [`DocumentEpoch::work_snapshot`] is the exact, concurrency-safe source.
/// Invocation snapshot deltas use the same shape only as aggregate interval
/// diagnostics and cannot attribute overlapping workers.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DocumentEpochWork {
    pub launch_context_constructions: usize,
    pub launch_context_extensions: usize,
    pub ambient_fallbacks: usize,
    pub prepared_context_consumers: BTreeMap<String, usize>,
    /// Volatile groups this epoch's own calls observed. A group its run had
    /// already observed (a parallel sibling's shared capture, or an earlier
    /// stage of the same run) is a reuse and does not appear here.
    pub volatile_observations: BTreeMap<String, usize>,
}

#[derive(Debug, Default)]
struct DocumentEpochRecorder {
    launch_context_constructions: AtomicUsize,
    launch_context_extensions: AtomicUsize,
    ambient_fallbacks: AtomicUsize,
    prepared_context_consumers: Mutex<BTreeMap<String, usize>>,
    volatile_observations: Mutex<BTreeMap<String, usize>>,
}

impl DocumentEpochRecorder {
    fn snapshot(&self) -> DocumentEpochWork {
        DocumentEpochWork {
            launch_context_constructions: self
                .launch_context_constructions
                .load(Ordering::Relaxed),
            launch_context_extensions: self.launch_context_extensions.load(Ordering::Relaxed),
            ambient_fallbacks: self.ambient_fallbacks.load(Ordering::Relaxed),
            prepared_context_consumers: self
                .prepared_context_consumers
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .clone(),
            volatile_observations: self
                .volatile_observations
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .clone(),
        }
    }
}

impl InvocationWorkSnapshot {
    /// Return aggregate preparation work performed since `before`.
    ///
    /// Both snapshots must belong to the same invocation and `before` must
    /// have been captured first. Violating either condition is a caller bug.
    /// This interval is not an exact epoch boundary when work overlaps; use
    /// [`DocumentEpoch::work_snapshot`] for attributable assertions.
    pub fn document_epoch_since(&self, before: &Self) -> DocumentEpochWork {
        DocumentEpochWork {
            launch_context_constructions: monotonic_delta(
                self.launch_context_constructions,
                before.launch_context_constructions,
            ),
            launch_context_extensions: monotonic_delta(
                self.launch_context_extensions,
                before.launch_context_extensions,
            ),
            ambient_fallbacks: monotonic_delta(
                self.ambient_fallbacks,
                before.ambient_fallbacks,
            ),
            prepared_context_consumers: map_delta(
                &self.prepared_context_consumers,
                &before.prepared_context_consumers,
            ),
            volatile_observations: map_delta(
                &self.volatile_observations,
                &before.volatile_observations,
            ),
        }
    }
}

fn monotonic_delta(after: usize, before: usize) -> usize {
    after
        .checked_sub(before)
        .expect("invocation work snapshots must be ordered and share an owner")
}

fn map_delta(
    after: &BTreeMap<String, usize>,
    before: &BTreeMap<String, usize>,
) -> BTreeMap<String, usize> {
    let mut delta = BTreeMap::new();
    for (name, after_count) in after {
        let count = monotonic_delta(*after_count, before.get(name).copied().unwrap_or_default());
        if count > 0 {
            delta.insert(name.clone(), count);
        }
    }
    for (name, before_count) in before {
        assert!(
            after.contains_key(name) || *before_count == 0,
            "invocation consumer counts must be monotonic"
        );
    }
    delta
}

#[derive(Debug, Default)]
struct InvocationWork {
    git_root_discoveries: AtomicUsize,
    topology_probes: AtomicUsize,
    topology_reuses: AtomicUsize,
    system_prompt_lookups: AtomicUsize,
    compose_operations: AtomicUsize,
    harness_eligibility_parses: AtomicUsize,
    harness_materializations: AtomicUsize,
    ambient_fallbacks: AtomicUsize,
    launch_context_constructions: AtomicUsize,
    launch_context_extensions: AtomicUsize,
    prepared_context_consumers: Mutex<BTreeMap<String, usize>>,
    next_document_epoch: AtomicUsize,
    document_epochs: Mutex<BTreeMap<usize, Arc<DocumentEpochRecorder>>>,
    runtime_evidence_captures: Mutex<BTreeMap<String, usize>>,
    runtime_evidence_reuses: Mutex<BTreeMap<String, usize>>,
    volatile_observations: Mutex<BTreeMap<String, usize>>,
    system_prompt_timings: Mutex<BTreeMap<String, std::time::Duration>>,
}

impl InvocationWork {
    fn snapshot(&self) -> InvocationWorkSnapshot {
        InvocationWorkSnapshot {
            git_root_discoveries: self.git_root_discoveries.load(Ordering::Relaxed),
            topology_probes: self.topology_probes.load(Ordering::Relaxed),
            topology_reuses: self.topology_reuses.load(Ordering::Relaxed),
            system_prompt_lookups: self.system_prompt_lookups.load(Ordering::Relaxed),
            compose_operations: self.compose_operations.load(Ordering::Relaxed),
            harness_eligibility_parses: self
                .harness_eligibility_parses
                .load(Ordering::Relaxed),
            harness_materializations: self.harness_materializations.load(Ordering::Relaxed),
            ambient_fallbacks: self.ambient_fallbacks.load(Ordering::Relaxed),
            launch_context_constructions: self
                .launch_context_constructions
                .load(Ordering::Relaxed),
            launch_context_extensions: self.launch_context_extensions.load(Ordering::Relaxed),
            prepared_context_consumers: self
                .prepared_context_consumers
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .clone(),
            document_epochs: self
                .document_epochs
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .iter()
                .map(|(id, recorder)| (*id, recorder.snapshot()))
                .collect(),
            runtime_evidence_captures: self
                .runtime_evidence_captures
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .clone(),
            runtime_evidence_reuses: self
                .runtime_evidence_reuses
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .clone(),
            volatile_observations: self
                .volatile_observations
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .clone(),
            system_prompt_timings: self
                .system_prompt_timings
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .clone(),
        }
    }
}

/// Canonical cache identity for one repository.
///
/// Both paths are [`canonical_key`] form so that a launch reached through a
/// symlinked ancestor keys the same entry a later lookup finds. The key is
/// never projected: [`RepositoryEntry::repo_root`] keeps the authored root Git
/// reported, and `worktree_root` is only ever compared against another key.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct RepositoryKey {
    worktree_root: PathBuf,
    git_dir: PathBuf,
}

impl RepositoryKey {
    fn from_identity(identity: &GitRepositoryIdentity) -> Self {
        Self {
            worktree_root: canonical_key(identity.repo_root()),
            git_dir: canonical_key(identity.git_dir()),
        }
    }
}

#[derive(Debug)]
struct RepositoryEntry {
    observation: FilesystemObservation,
    identity: Option<GitRepositoryIdentity>,
    git_info: Option<GitInfo>,
    topology: OnceLock<Result<Option<RepoInfo>, Arc<sniff::SniffError>>>,
    failure: Option<Arc<sniff::SniffError>>,
    diagnostic: OnceLock<DiagnosticSnapshot>,
    source_evidence: Mutex<HashMap<PathBuf, Arc<SourceEvidence>>>,
}

/// Source-scanned evidence that keeps its invocation lifetime.
///
/// Languages and documents describe the repository's contents at a
/// granularity no single run changes in a way a prompt depends on, so one scan
/// per source directory serves the whole invocation. Git working state is not
/// here: it belongs to [`RunEvidence`].
#[derive(Debug, Default)]
struct SourceEvidence {
    languages: OnceLock<Result<Option<LanguageBreakdown>, Arc<sniff::SniffError>>>,
    documents: OnceLock<Result<Option<Vec<MarkdownMeta>>, Arc<sniff::SniffError>>>,
}

/// Volatile repository evidence observed for one composition run.
///
/// Git working state — the branch, worktree, and merge-conflict facts, the
/// staged/dirty/untracked file changes, and the recent commit history — can
/// change between runs, including by another process, so an invocation never
/// retains it. A run observes each volatile group at most once, on the first
/// request for it, and every later request in the same run (another stage,
/// the post-`initialize` reread, a transcluded source naming the group)
/// reuses that observation. A new run starts from an empty scope, so which
/// run first mentioned a property has no effect on what a later run sees.
///
/// Cloning shares the scope: the siblings of a parallel sequence group hold
/// clones of one run so they begin from one view of the working tree.
#[derive(Debug, Clone, Default)]
pub struct RunEvidence {
    repositories: Arc<Mutex<Vec<RunRepository>>>,
}

/// One repository's volatile cells within a run.
type RunRepository = (Arc<RepositoryEntry>, Arc<VolatileEvidence>);

#[derive(Debug, Default)]
struct VolatileEvidence {
    git: OnceLock<Result<Option<GitInfo>, Arc<sniff::SniffError>>>,
    file_changes: OnceLock<Result<Option<Vec<FileChange>>, Arc<sniff::SniffError>>>,
    /// `Ok(None)` when the repository has no root to walk history from.
    recent_commits: OnceLock<Result<Option<RecentCommits>, Arc<sniff::SniffError>>>,
}

impl RunEvidence {
    /// This run's cells for one repository; entries are compared by identity
    /// because the invocation hands out exactly one entry per repository.
    fn for_repository(&self, repository: &Arc<RepositoryEntry>) -> Arc<VolatileEvidence> {
        let mut repositories = self
            .repositories
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if let Some((_, cells)) = repositories
            .iter()
            .find(|(entry, _)| Arc::ptr_eq(entry, repository))
        {
            return Arc::clone(cells);
        }
        let cells = Arc::new(VolatileEvidence::default());
        repositories.push((Arc::clone(repository), Arc::clone(&cells)));
        cells
    }
}

impl RepositoryEntry {
    fn absent(observation: FilesystemObservation) -> Self {
        Self {
            observation,
            identity: None,
            git_info: None,
            topology: OnceLock::new(),
            failure: None,
            diagnostic: OnceLock::new(),
            source_evidence: Mutex::new(HashMap::new()),
        }
    }

    fn failed(observation: FilesystemObservation, error: sniff::SniffError) -> Self {
        let failure = Arc::new(error);
        let diagnostic = DiagnosticSnapshot::from_diagnostic(
            &ClaudineError::LaunchContextDetection(failure.clone()),
        );
        let diagnostic_cell = OnceLock::new();
        let _ = diagnostic_cell.set(diagnostic);
        Self {
            observation,
            identity: None,
            git_info: None,
            topology: OnceLock::new(),
            failure: Some(failure),
            diagnostic: diagnostic_cell,
            source_evidence: Mutex::new(HashMap::new()),
        }
    }

    fn present(
        observation: FilesystemObservation,
        identity: GitRepositoryIdentity,
        git_info: Option<GitInfo>,
        topology: Option<Result<Option<RepoInfo>, Arc<sniff::SniffError>>>,
    ) -> Self {
        let cell = OnceLock::new();
        if let Some(topology) = topology {
            let _ = cell.set(topology);
        }
        Self {
            observation,
            identity: Some(identity),
            git_info,
            topology: cell,
            failure: None,
            diagnostic: OnceLock::new(),
            source_evidence: Mutex::new(HashMap::new()),
        }
    }

    fn repo_root(&self) -> Option<&Path> {
        self.identity.as_ref().map(GitRepositoryIdentity::repo_root)
    }

    fn repo_info(&self) -> Option<&RepoInfo> {
        self.topology
            .get()
            .and_then(|result| result.as_ref().ok())
            .and_then(Option::as_ref)
    }

    fn failure(&self) -> Option<&sniff::SniffError> {
        self.failure.as_deref().or_else(|| {
            self.topology
                .get()
                .and_then(|result| result.as_ref().err())
                .map(Arc::as_ref)
        })
    }

    fn failure_arc(&self) -> Option<Arc<sniff::SniffError>> {
        self.failure.clone().or_else(|| {
            self.topology
                .get()
                .and_then(|result| result.as_ref().err())
                .cloned()
        })
    }

    fn diagnostic(&self) -> Option<&DiagnosticSnapshot> {
        if let Some(diagnostic) = self.diagnostic.get() {
            return Some(diagnostic);
        }
        let failure = self.failure_arc()?;
        let diagnostic = DiagnosticSnapshot::from_diagnostic(
            &ClaudineError::LaunchContextDetection(failure),
        );
        let _ = self.diagnostic.set(diagnostic);
        self.diagnostic.get()
    }
}

/// One immutable repository observation retained by an invocation.
#[derive(Debug, Clone)]
pub struct RepositoryObservation {
    inner: Arc<RepositoryEntry>,
}

impl RepositoryObservation {
    pub fn repo_root(&self) -> Option<&Path> {
        self.inner.repo_root()
    }

    pub fn git_info(&self) -> Option<&GitInfo> {
        self.inner.git_info.as_ref()
    }

    pub fn repo_info(&self) -> Option<&RepoInfo> {
        self.inner.repo_info()
    }

    pub fn filesystem_observation(&self) -> &FilesystemObservation {
        &self.inner.observation
    }

    pub fn failure(&self) -> Option<&sniff::SniffError> {
        self.inner.failure()
    }

    pub fn diagnostic(&self) -> Option<&DiagnosticSnapshot> {
        self.inner.diagnostic()
    }

    pub fn is_absent(&self) -> bool {
        self.inner.identity.is_none() && self.inner.failure().is_none()
    }
}

/// Definitive request context for one resolved document source.
#[derive(Debug, Clone)]
pub struct SourceContext {
    source_path: PathBuf,
    base_dir: PathBuf,
    repository: RepositoryObservation,
    repository_root: Option<PathBuf>,
    package_area_root: Option<PathBuf>,
    package_root: Option<PathBuf>,
    file_resolution: FileResolutionContext,
}

impl SourceContext {
    pub fn source_path(&self) -> &Path {
        &self.source_path
    }

    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    pub fn repository(&self) -> &RepositoryObservation {
        &self.repository
    }

    pub fn repository_root(&self) -> Option<&Path> {
        self.repository_root.as_deref()
    }

    pub fn package_area_root(&self) -> Option<&Path> {
        self.package_area_root.as_deref()
    }

    pub fn package_root(&self) -> Option<&Path> {
        self.package_root.as_deref()
    }

    pub fn file_resolution_context(&self) -> &FileResolutionContext {
        &self.file_resolution
    }
}

/// Request-local repository evidence, keyed entirely in [`canonical_key`] form.
///
/// Every insert and every lookup on both maps must canonicalize first; an
/// asymmetric key is a silent cache miss that re-discovers Git and swaps the
/// projected repository root mid-invocation.
#[derive(Debug, Default)]
struct RepositoryCache {
    repositories: HashMap<RepositoryKey, Arc<RepositoryEntry>>,
    exact_non_repositories: HashMap<PathBuf, Arc<RepositoryEntry>>,
}

/// Environment variables that name a user home on a supported platform.
///
/// Unix launches populate `HOME`; native Windows populates `USERPROFILE` and
/// the `HOMEDRIVE`/`HOMEPATH` pair; an MSYS, Git-Bash, or WSL-interop launch can
/// carry both sets at once. All four are recorded on every platform so a
/// consumer never has to decide which names its host "should" have had.
pub const HOME_VARIABLES: [&str; 4] = ["HOME", "USERPROFILE", "HOMEDRIVE", "HOMEPATH"];

/// Whether two environment-variable names address the same variable.
///
/// Windows resolves names case-insensitively, so a lookup that compares bytes
/// would miss a `Path`/`PATH` or `UserProfile`/`USERPROFILE` spelling the OS
/// considers identical.
fn env_names_match(left: &OsStr, right: &OsStr) -> bool {
    #[cfg(windows)]
    {
        left.to_string_lossy()
            .eq_ignore_ascii_case(&right.to_string_lossy())
    }
    #[cfg(not(windows))]
    {
        left == right
    }
}

/// The user home as it stood when the invocation was captured.
///
/// Absence is `None` and never an empty string: an unset `HOME` and an empty
/// `HOME` are different launch states, and a child process can tell them apart.
/// `resolved` is the same authority [`InvocationContext::home_dir`] reports;
/// the raw variables are what a child environment must reproduce byte for byte.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HomeBaseline {
    resolved: Option<PathBuf>,
    variables: [Option<OsString>; HOME_VARIABLES.len()],
}

impl HomeBaseline {
    /// Capture the home state of the current process, resolved as `snapshot`
    /// resolves it.
    ///
    /// The resolved home is the request snapshot's, so `~` and every child
    /// projection agree. The raw variables are read from the process because
    /// a child must reproduce them byte for byte, which the snapshot's UTF-8
    /// view cannot.
    pub fn capture(snapshot: &RequestSnapshot) -> Self {
        Self {
            resolved: snapshot.home().map(Path::to_path_buf),
            variables: HOME_VARIABLES.map(std::env::var_os),
        }
    }

    /// Build a baseline from explicit values, `variables` in
    /// [`HOME_VARIABLES`] order.
    ///
    /// The twin of [`EnvBaseline::from_entries`]: a consumer that has to plan
    /// against a home other than this process's — a test fixture, a replayed
    /// invocation — states it rather than mutating the process to make
    /// [`HomeBaseline::capture`] observe it.
    pub fn from_parts(
        resolved: Option<PathBuf>,
        variables: [Option<OsString>; HOME_VARIABLES.len()],
    ) -> Self {
        Self {
            resolved,
            variables,
        }
    }

    /// The resolved user home directory, when the host has one.
    pub fn resolved(&self) -> Option<&Path> {
        self.resolved.as_deref()
    }

    /// The captured raw value of one [`HOME_VARIABLES`] entry.
    ///
    /// Returns `None` both for a variable that was unset at capture and for a
    /// name outside the home vocabulary.
    pub fn variable(&self, name: &str) -> Option<&OsStr> {
        HOME_VARIABLES
            .iter()
            .position(|candidate| env_names_match(OsStr::new(candidate), OsStr::new(name)))
            .and_then(|index| self.variables[index].as_deref())
    }

    /// Every home variable in [`HOME_VARIABLES`] order, present or absent.
    pub fn variables(&self) -> impl Iterator<Item = (&'static str, Option<&OsStr>)> + '_ {
        HOME_VARIABLES
            .iter()
            .zip(self.variables.iter())
            .map(|(name, value)| (*name, value.as_deref()))
    }
}

/// The complete launch environment, preserved exactly as the OS supplied it.
///
/// [`InvocationContext::environment`] remains the lossy `String` view that
/// context projection and templating consume. This record exists beside it
/// because child-environment assembly must reproduce values that no UTF-8
/// conversion survives, and because *absence* of a variable is load-bearing
/// when a provider-owned key has to be removed or restored.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct EnvBaseline {
    entries: Vec<(OsString, OsString)>,
}

impl EnvBaseline {
    /// Capture the ambient environment of the current process.
    pub fn capture() -> Self {
        Self {
            entries: std::env::vars_os().collect(),
        }
    }

    /// Build a baseline from explicit entries.
    pub fn from_entries<I, K, V>(entries: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<OsString>,
        V: Into<OsString>,
    {
        Self {
            entries: entries
                .into_iter()
                .map(|(key, value)| (key.into(), value.into()))
                .collect(),
        }
    }

    /// Every captured variable, in the order the OS enumerated it.
    pub fn iter(&self) -> impl Iterator<Item = (&OsStr, &OsStr)> + '_ {
        self.entries
            .iter()
            .map(|(key, value)| (key.as_os_str(), value.as_os_str()))
    }

    /// The captured raw value of one variable.
    pub fn get<K: AsRef<OsStr>>(&self, name: K) -> Option<&OsStr> {
        let name = name.as_ref();
        self.entries
            .iter()
            .find(|(key, _)| env_names_match(key, name))
            .map(|(_, value)| value.as_os_str())
    }

    /// The number of captured variables.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether nothing was captured.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[derive(Debug)]
struct InvocationInner {
    launch_cwd: PathBuf,
    snapshot: RequestSnapshot,
    home_baseline: HomeBaseline,
    env_baseline: EnvBaseline,
    environment: HashMap<String, String>,
    launch_repository: Arc<RepositoryEntry>,
    launch_result: sniff::SniffResult,
    launch_file_resolution: FileResolutionContext,
    repositories: Mutex<RepositoryCache>,
    os: OnceLock<Result<OsInfo, Arc<sniff::SniffError>>>,
    hardware: OnceLock<Result<HardwareInfo, Arc<sniff::SniffError>>>,
    gpus: OnceLock<Vec<GpuInfo>>,
    work: InvocationWork,
}

/// Owner of immutable launch facts and request-local repository evidence.
#[derive(Debug, Clone)]
pub struct InvocationContext {
    inner: Arc<InvocationInner>,
}

/// The [`CurrentProvider`] an invocation installs for the lazy `current` root.
///
/// Holds the whole invocation rather than a copy of its evidence: the launch
/// repository handle, the launch anchor, and the host caches all live there,
/// and a refresh has to reach the *handle* — copying the evidence would be the
/// replay this provider exists to avoid.
#[derive(Debug)]
struct LaunchRefresh {
    invocation: InvocationContext,
}

impl CurrentProvider for LaunchRefresh {
    fn refresh(&self, key: &str) -> CurrentRefresh {
        self.invocation.refresh_current(key)
    }
}

/// Attribution token for one canonical document preparation epoch.
///
/// The recorder belongs to the token, not to a before/after interval on the
/// invocation. Overlapping sequence workers therefore cannot contribute to
/// one another's exact construction, extension, fallback, or consumer map.
///
/// An epoch also carries the [`RunEvidence`] of the composition run it
/// belongs to. Every capture and extension through the epoch observes
/// volatile Git state from that run; stable evidence still comes from the
/// invocation.
#[derive(Debug, Clone)]
pub struct DocumentEpoch {
    id: usize,
    invocation: InvocationContext,
    work: Arc<DocumentEpochRecorder>,
    run: RunEvidence,
}

impl DocumentEpoch {
    /// This epoch's invocation refresh capability for the lazy `current` root.
    ///
    /// Refresh is invocation-scoped, not epoch-scoped: `current.<key>` observes
    /// a fact now, and an epoch boundary does not change what "now" means.
    #[must_use]
    pub fn current_provider(&self) -> Arc<dyn CurrentProvider> {
        self.invocation.current_provider()
    }

    /// [`Self::current_provider`] wrapped in the authority
    /// `EffectiveStateBuilder::with_current_authority` takes.
    #[must_use]
    pub fn current_authority(&self) -> darkmatter::markdown::compose::CurrentAuthority {
        self.invocation.current_authority()
    }

    /// Stable request-local identity used to correlate diagnostic snapshots.
    pub fn id(&self) -> usize {
        self.id
    }

    /// The composition run this epoch observes volatile evidence from.
    pub fn run_evidence(&self) -> &RunEvidence {
        &self.run
    }

    /// Capture this epoch's one launch-anchored context construction.
    pub fn capture_launch_context(
        &self,
        requirements: &darkmatter::markdown::compose::ContextRequirements,
    ) -> darkmatter::markdown::compose::ComposeContext {
        let context =
            self.invocation
                .capture_launch_context_in(&self.run, Some(&self.work), requirements);
        self.work
            .launch_context_constructions
            .fetch_add(1, Ordering::Relaxed);
        context
    }

    /// Extend this epoch's retained context from the invocation's launch
    /// evidence and this epoch's run.
    ///
    /// Only groups the context is missing are projected, so an already
    /// observed value is never replaced: extension within a run is not a
    /// refresh.
    pub fn extend_launch_context(
        &self,
        context: &mut darkmatter::markdown::compose::ComposeContext,
        requirements: &darkmatter::markdown::compose::ContextRequirements,
    ) -> bool {
        let extended = self.invocation.extend_launch_context_in(
            &self.run,
            Some(&self.work),
            context,
            requirements,
        );
        if extended {
            self.work
                .launch_context_extensions
                .fetch_add(1, Ordering::Relaxed);
        }
        extended
    }

    /// Lets composition grow this epoch's launch context when a transcluded
    /// source names a group the snapshot lacks, from the same launch evidence.
    pub fn compose_context_authority(&self) -> darkmatter::markdown::compose::ContextAuthority {
        darkmatter::markdown::compose::ContextAuthority::CallerExtended(Arc::new(self.clone()))
    }

    /// Record a production seam that consumed this epoch's populated context.
    pub fn record_prepared_context_consumer(&self, consumer: PreparedContextConsumer) {
        self.invocation.record_prepared_context_consumer(consumer);
        record_group(
            &self.work.prepared_context_consumers,
            consumer.as_str().to_string(),
        );
    }

    /// Record a compatibility capture taken instead of this epoch's context.
    pub fn record_ambient_fallback(&self) {
        self.invocation.record_ambient_fallback();
        self.work.ambient_fallbacks.fetch_add(1, Ordering::Relaxed);
    }

    /// Return exact work owned by this epoch.
    pub fn work_snapshot(&self) -> DocumentEpochWork {
        self.work.snapshot()
    }
}

impl InvocationContext {
    /// Begin one independently attributable canonical document epoch that
    /// opens a new composition run.
    ///
    /// Use this at every run boundary: a document invoked or adopted, a
    /// sequence step or serial group member, a loop iteration, a retry, and a
    /// resume.
    pub fn begin_document_epoch(&self) -> DocumentEpoch {
        self.begin_document_epoch_in(&RunEvidence::default())
    }

    /// Begin a document epoch inside an existing composition run.
    ///
    /// For a second document epoch that is not a new run: a sequence step's
    /// task and its prompt document, or a parallel group sibling's first
    /// preparation, which shares the group's capture.
    pub fn begin_document_epoch_in(&self, run: &RunEvidence) -> DocumentEpoch {
        let id = self
            .inner
            .work
            .next_document_epoch
            .fetch_add(1, Ordering::Relaxed);
        let work = Arc::new(DocumentEpochRecorder::default());
        self.inner
            .work
            .document_epochs
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(id, Arc::clone(&work));
        DocumentEpoch {
            id,
            invocation: self.clone(),
            work,
            run: run.clone(),
        }
    }

    /// Capture a composition invocation launched at the snapshot's request
    /// directory.
    ///
    /// ## Errors
    ///
    /// Returns [`InvocationContextError::ResolutionContext`] when the launch
    /// file-resolution context cannot be built.
    pub fn capture(snapshot: &RequestSnapshot) -> Result<Self, InvocationContextError> {
        Self::capture_inner(snapshot, GitRequest::summary(), true)
    }

    /// Capture a composition invocation launched at `cwd`, keeping the
    /// snapshot's home and environment.
    ///
    /// ## Errors
    ///
    /// As [`capture`](Self::capture).
    pub fn capture_at(snapshot: &RequestSnapshot, cwd: &Path) -> Result<Self, InvocationContextError> {
        Self::capture(&snapshot.at_request_dir(absolutize(snapshot, cwd)))
    }

    /// Capture direct-wrapper startup evidence.
    ///
    /// Promptless root wrappers retain Git identity but omit topology. A later
    /// source that needs package evidence initializes the same entry's
    /// topology cell instead of discovering Git again.
    ///
    /// ## Errors
    ///
    /// As [`capture`](Self::capture).
    pub fn capture_for_wrapper(
        snapshot: &RequestSnapshot,
        cwd: &Path,
        capture_git_status: bool,
    ) -> Result<Self, InvocationContextError> {
        let snapshot = snapshot.at_request_dir(absolutize(snapshot, cwd));
        let cwd = snapshot.request_dir().to_path_buf();
        let request = if capture_git_status {
            GitRequest::summary()
        } else {
            GitRequest::identity()
        };
        let observation = FilesystemObservation::discover(&cwd);
        let promptless_at_repo_root = !capture_git_status
            && observation
                .repository_identity()
                .ok()
                .flatten()
                .is_some_and(|identity| paths_equivalent(identity.repo_root(), &cwd));
        Self::capture_with_observation(&snapshot, request, !promptless_at_repo_root, observation)
    }

    fn capture_inner(
        snapshot: &RequestSnapshot,
        git_request: GitRequest,
        include_topology: bool,
    ) -> Result<Self, InvocationContextError> {
        let observation = FilesystemObservation::discover(snapshot.request_dir());
        Self::capture_with_observation(snapshot, git_request, include_topology, observation)
    }

    fn capture_with_observation(
        snapshot: &RequestSnapshot,
        git_request: GitRequest,
        include_topology: bool,
        observation: FilesystemObservation,
    ) -> Result<Self, InvocationContextError> {
        let cwd = snapshot.request_dir().to_path_buf();
        // `ctx.*`, `env.*`, and file references read the snapshot's
        // environment. The raw baseline is kept beside it only for child
        // processes, which must reproduce values no UTF-8 view survives.
        let env_baseline = EnvBaseline::capture();
        let environment = snapshot.env().clone();
        let home_baseline = HomeBaseline::capture(snapshot);
        let work = InvocationWork::default();
        work.git_root_discoveries.fetch_add(1, Ordering::Relaxed);

        let (launch_repository, launch_result) = observe_repository(
            &cwd,
            observation,
            git_request,
            include_topology,
            &work,
        );

        let launch_file_resolution = build_file_resolution_context(
            snapshot,
            None,
            launch_repository.repo_root(),
            launch_repository.repo_info(),
            None,
        )?;

        let mut cache = RepositoryCache::default();
        if let Some(identity) = launch_repository.identity.as_ref() {
            cache.repositories.insert(
                RepositoryKey::from_identity(identity),
                launch_repository.clone(),
            );
        } else {
            cache
                .exact_non_repositories
                .insert(canonical_key(&cwd), launch_repository.clone());
        }

        Ok(Self {
            inner: Arc::new(InvocationInner {
                launch_cwd: cwd,
                snapshot: snapshot.clone(),
                home_baseline,
                env_baseline,
                environment,
                launch_repository,
                launch_result,
                launch_file_resolution,
                repositories: Mutex::new(cache),
                os: OnceLock::new(),
                hardware: OnceLock::new(),
                gpus: OnceLock::new(),
                work,
            }),
        })
    }

    pub fn launch_cwd(&self) -> &Path {
        &self.inner.launch_cwd
    }

    pub fn home_dir(&self) -> Option<&Path> {
        self.inner.home_baseline.resolved()
    }

    /// The immutable launch home snapshot.
    ///
    /// Later ambient mutation cannot move it, which is what lets child-process
    /// assembly project the *launch* home rather than whatever a wrapper stage
    /// has since written into its own process.
    pub fn home_baseline(&self) -> &HomeBaseline {
        &self.inner.home_baseline
    }

    /// The immutable launch environment snapshot, in raw [`OsString`] form.
    pub fn env_baseline(&self) -> &EnvBaseline {
        &self.inner.env_baseline
    }

    pub fn environment(&self) -> &HashMap<String, String> {
        &self.inner.environment
    }

    /// The request snapshot this invocation was captured from, anchored at
    /// the launch directory.
    ///
    /// A context for a directory in another repository is built from this
    /// snapshot ([`RequestSnapshot::at_request_dir`]), so it keeps the
    /// invocation's home and environment.
    pub fn request_snapshot(&self) -> &RequestSnapshot {
        &self.inner.snapshot
    }

    pub fn launch_file_resolution_context(&self) -> &FileResolutionContext {
        &self.inner.launch_file_resolution
    }

    pub fn launch_repository(&self) -> RepositoryObservation {
        RepositoryObservation {
            inner: self.inner.launch_repository.clone(),
        }
    }

    pub fn launch_context(&self) -> LaunchContext {
        LaunchContext::from_sniff_result(&self.inner.launch_result, &self.inner.launch_cwd)
    }

    pub fn environment_context(&self) -> EnvironmentContext {
        environment_context_from_sniff_result_and_env(
            self.inner.launch_result.clone(),
            &self.inner.environment,
        )
    }

    /// Project event metadata for a resolved source from retained evidence.
    pub fn environment_context_for_source(&self, source: &SourceContext) -> EnvironmentContext {
        let filesystem = sniff::filesystem::FilesystemInfo {
            git: source.repository.inner.git_info.clone(),
            repo: source.repository.inner.repo_info().cloned(),
            ..Default::default()
        };
        let result = sniff::SniffResult {
            filesystem: Some(filesystem),
            ..Default::default()
        };
        environment_context_from_sniff_result_and_env(result, &self.inner.environment)
    }

    pub fn launch_workspace_context(
        &self,
        source_repo_root: Option<&Path>,
    ) -> LaunchWorkspaceContext {
        LaunchWorkspaceContext::from_repo_info(
            &self.inner.launch_cwd,
            self.inner.launch_repository.repo_root(),
            self.inner.launch_repository.repo_info(),
            source_repo_root,
        )
    }

    /// Derive the definitive request context for one resolved document source.
    ///
    /// Repository evidence is shared — a second source in an already observed
    /// repository reuses that entry and its topology — but the returned value
    /// is not memoized: calling this twice for one path rebuilds the package
    /// projection and the resolution context. A caller that needs the same
    /// source context at two stages should carry the value rather than derive
    /// it again.
    ///
    /// The launch `@` scope is carried into the rebuilt context: a source in
    /// another repository or an external prompt directory keeps its own
    /// repository anchors for `&`/`^` and its own authoring base for `./`
    /// and bare references, while nested `@` references keep searching the
    /// launch tree first (ruling 2 of 2026-09-23-local-before-home).
    pub fn derive_source(
        &self,
        source_path: &Path,
    ) -> Result<SourceContext, InvocationContextError> {
        let source_path = if source_path.is_absolute() {
            source_path.to_path_buf()
        } else {
            self.inner.launch_cwd.join(source_path)
        };
        let base_dir = source_path
            .parent()
            .map(Path::to_path_buf)
            .ok_or_else(|| InvocationContextError::SourceWithoutParent(source_path.clone()))?;
        let entry = self.repository_for_base(&base_dir);
        self.ensure_topology(&entry);

        let repository_root = entry
            .repo_root()
            .map(|root| repo_root_in_base_spelling(&base_dir, root));
        let file_resolution = build_file_resolution_context(
            &self.inner.snapshot.at_request_dir(&base_dir),
            Some(&source_path),
            repository_root.as_deref(),
            entry.repo_info(),
            Some(self.inner.launch_file_resolution.launch_magic_scope()),
        )?;
        let package_area_root = file_resolution.package_area().map(Path::to_path_buf);
        let package_root = file_resolution.package_root().map(Path::to_path_buf);

        Ok(SourceContext {
            source_path,
            base_dir,
            repository: RepositoryObservation {
                inner: entry,
            },
            repository_root,
            package_area_root,
            package_root,
            file_resolution,
        })
    }

    /// [`derive_source`](Self::derive_source) for a top-level source, keeping
    /// the anchor of the reference that opened it.
    ///
    /// A prompt opened as `~/.claudine/prompts/x.md` (or through a leading
    /// `{{VAR}}`) outside any repository or vault takes that anchor as its
    /// tree root, so its relative references may move around the anchor but
    /// not leave it. A source a repository contains keeps the repository as
    /// its tree. A reference with no such anchor (`@x.md`, an absolute path)
    /// derives exactly as [`derive_source`](Self::derive_source) does.
    pub fn derive_composition_source(
        &self,
        source: &crate::composition::ResolvedCompositionSource,
    ) -> Result<SourceContext, InvocationContextError> {
        let mut context = self.derive_source(&source.resolved_path)?;
        if let Ok(reference) = biscuit_file::FileReference::new(&source.original_ref) {
            context.file_resolution = context
                .file_resolution
                .for_source_reference(&reference, &context.source_path);
        }
        Ok(context)
    }

    /// Capture the runtime groups requested by a document from retained facts.
    ///
    /// The returned evidence is complete for every requested group. Repository
    /// and host observations are cached by this invocation; no ambient CWD,
    /// HOME, environment, or Git discovery is performed.
    ///
    /// Every requested group lands in exactly one work counter: the group is a
    /// capture when this call ran the initializer behind its cache, and a reuse
    /// when retained evidence answered it. Groups that hold no evidence at all
    /// (`datetime`, `agent`) and groups that only clone what the repository
    /// entry already carries (`git`, `repo`) are therefore always reuses.
    ///
    /// Volatile groups are observed once per `run`, never per invocation.
    ///
    /// This is the *source* projection: repository facts come from the supplied
    /// document's repository and package area. Canonical prepared `ctx.*` must
    /// instead use [`Self::capture_launch_context`], which pairs the launch
    /// anchor with launch evidence as one operation.
    pub fn runtime_evidence(
        &self,
        run: &RunEvidence,
        source: &SourceContext,
        requirements: &darkmatter::markdown::compose::ContextRequirements,
    ) -> darkmatter::markdown::compose::ContextCaptureEvidence {
        self.project_evidence(
            &source.repository.inner,
            &source.base_dir,
            source.repository_root.as_deref(),
            run,
            None,
            requirements,
        )
    }

    /// Capture the launch-anchored prepared-`ctx.*` snapshot for one document
    /// preparation epoch.
    ///
    /// The returned `ComposeContext` is anchored at [`Self::launch_cwd`] and
    /// populated exclusively from the invocation's retained launch repository,
    /// launch package topology, environment, and host evidence. The anchor and
    /// the evidence are inseparable here by construction: a caller cannot pair
    /// a launch directory with a prompt-derived repository.
    ///
    /// Source-scanned groups (`file_changes`, `languages`, `documents`) scan
    /// from the launch base when requested, and Git/repository facts project
    /// the launch repository — never a document source's repository. No
    /// ambient CWD, HOME, environment, or topology discovery runs; the
    /// retained caches answer every stable group.
    ///
    /// Called on the invocation rather than a [`DocumentEpoch`], the capture
    /// is its own composition run: volatile Git state is observed for this
    /// call and retained nowhere. A caller whose stages must agree on one
    /// observation captures and extends through one epoch instead.
    ///
    /// Callers apply the resolved target's `env_overrides` (agent/model
    /// identity) on the returned snapshot through `env_mut`; that is the
    /// existing target-identity precedence, not part of the capture.
    ///
    /// Counts one `launch_context_constructions` in the work snapshot.
    pub fn capture_launch_context(
        &self,
        requirements: &darkmatter::markdown::compose::ContextRequirements,
    ) -> darkmatter::markdown::compose::ComposeContext {
        self.capture_launch_context_in(&RunEvidence::default(), None, requirements)
    }

    fn capture_launch_context_in(
        &self,
        run: &RunEvidence,
        epoch: Option<&DocumentEpochRecorder>,
        requirements: &darkmatter::markdown::compose::ContextRequirements,
    ) -> darkmatter::markdown::compose::ComposeContext {
        let evidence = self.project_evidence(
            &self.inner.launch_repository,
            &self.inner.launch_cwd,
            self.launch_repository_root_spelling().as_deref(),
            run,
            epoch,
            requirements,
        );
        let context = darkmatter::markdown::compose::ComposeContext::capture_with_evidence(
            &self.inner.launch_cwd,
            requirements,
            &evidence,
        );
        self.inner
            .work
            .launch_context_constructions
            .fetch_add(1, Ordering::Relaxed);
        context
    }

    /// Extend an existing epoch snapshot with requirement groups it is
    /// missing, from the same retained launch evidence.
    ///
    /// This is the same-epoch operation the post-`initialize` stabilized
    /// reread uses when the rewritten document demands context groups the
    /// stored snapshot was not captured with: only the missing groups are
    /// projected, and the snapshot's capture anchor, environment capture, and
    /// already-applied target overrides are preserved. A requirements set the
    /// snapshot already satisfies is a no-op that records no extension.
    ///
    /// Counts one `launch_context_extensions` per extension that populated at
    /// least one missing group.
    ///
    /// Like [`Self::capture_launch_context`], an invocation-level extension
    /// observes a missing volatile group for this call alone.
    pub fn extend_launch_context(
        &self,
        context: &mut darkmatter::markdown::compose::ComposeContext,
        requirements: &darkmatter::markdown::compose::ContextRequirements,
    ) -> bool {
        self.extend_launch_context_in(&RunEvidence::default(), None, context, requirements)
    }

    fn extend_launch_context_in(
        &self,
        run: &RunEvidence,
        epoch: Option<&DocumentEpochRecorder>,
        context: &mut darkmatter::markdown::compose::ComposeContext,
        requirements: &darkmatter::markdown::compose::ContextRequirements,
    ) -> bool {
        let missing = context.missing_requirements(requirements);
        if missing.iter().next().is_none() {
            return false;
        }
        let evidence = self.project_evidence(
            &self.inner.launch_repository,
            &self.inner.launch_cwd,
            self.launch_repository_root_spelling().as_deref(),
            run,
            epoch,
            &missing,
        );
        if context.extend_with_evidence(requirements, &evidence) {
            self.inner
                .work
                .launch_context_extensions
                .fetch_add(1, Ordering::Relaxed);
            true
        } else {
            false
        }
    }

    /// Lets composition grow a launch context when a transcluded source names
    /// a group the snapshot lacks, from this invocation's launch evidence.
    pub fn compose_context_authority(&self) -> darkmatter::markdown::compose::ContextAuthority {
        darkmatter::markdown::compose::ContextAuthority::CallerExtended(Arc::new(self.clone()))
    }

    /// The launch repository root projected into the launch directory's
    /// spelling family, mirroring what [`Self::derive_source`] retains for a
    /// source context.
    fn launch_repository_root_spelling(&self) -> Option<PathBuf> {
        self.inner
            .launch_repository
            .repo_root()
            .map(|root| repo_root_in_base_spelling(&self.inner.launch_cwd, root))
    }

    /// Project one requirements set's evidence from a retained repository
    /// entry, a scanning base, and that base's repository-root spelling.
    ///
    /// `runtime_evidence` (source projection) and `capture_launch_context` /
    /// `extend_launch_context` (launch projection) share this walk so both pay
    /// the same caches and record the same per-group work counters; only the
    /// entry and base differ.
    /// Repository-dependent groups also receive retained Git identity and
    /// topology evidence, even without an explicit Git or Repo group request.
    ///
    /// Stable groups read the invocation's caches. Volatile groups (`Git`,
    /// `FileChanges`, `GitHistory`) read `run`, observing on the run's first
    /// request; `epoch` is credited with any observation this call makes.
    #[allow(clippy::too_many_lines)]
    fn project_evidence(
        &self,
        repository: &Arc<RepositoryEntry>,
        base_dir: &Path,
        repository_root: Option<&Path>,
        run: &RunEvidence,
        epoch: Option<&DocumentEpochRecorder>,
        requirements: &darkmatter::markdown::compose::ContextRequirements,
    ) -> darkmatter::markdown::compose::ContextCaptureEvidence {
        use darkmatter::markdown::compose::{ContextCaptureEvidence, ContextGroup};

        let mut evidence = ContextCaptureEvidence::new(self.inner.environment.clone());
        let volatile = run.for_repository(repository);
        let needs_repository = requirements.iter().any(|group| {
            matches!(
                group,
                ContextGroup::Repo
                    | ContextGroup::FileChanges
                    | ContextGroup::Languages
                    | ContextGroup::Documents
            )
        });
        let mut observed_git = false;
        // `ctx.branch`, `ctx.worktree`, and `ctx.merge_conflicts` are this
        // run's observation. Identity-only consumers (`ctx.repo`, the
        // document hash) keep the launch observation, so a run that names no
        // Git fact pays for no Git status.
        let git_info = match (repository.failure(), requirements.contains(ContextGroup::Git)) {
            (Some(_), _) => None,
            (None, false) => repository.git_info.clone(),
            (None, true) => match volatile
                .git
                .get_or_init(|| {
                    observed_git = true;
                    repository
                        .observation
                        .detect_git(&launch_git_request())
                        .map_err(Arc::new)
                })
                .as_ref()
            {
                Ok(git) => git.clone(),
                // A failed re-observation keeps the launch observation rather
                // than dropping the group.
                Err(_) => repository.git_info.clone(),
            },
        };
        if repository.failure().is_none() {
            if needs_repository || requirements.contains(ContextGroup::Git) {
                evidence = evidence.with_git(git_info.clone());
            }
            if needs_repository && !matches!(repository.topology.get(), Some(Err(_))) {
                evidence = evidence.with_repository(
                    repository_root.map(Path::to_path_buf),
                    repository.repo_info().cloned(),
                );
            }
        }
        let source_evidence = {
            let mut cache = repository
                .source_evidence
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            cache
                .entry(base_dir.to_path_buf())
                .or_insert_with(|| Arc::new(SourceEvidence::default()))
                .clone()
        };

        for group in requirements.iter() {
            // Set only from inside a cache initializer, so the counters below
            // separate the one call that computed the evidence from every call
            // the cache answered.
            let mut captured = false;
            match group {
                ContextGroup::Invocation => {
                    evidence = evidence.with_invocation_cwd(Some(self.inner.launch_cwd.clone()));
                }
                ContextGroup::DateTime | ContextGroup::Agent | ContextGroup::Repo => {}
                ContextGroup::Git => captured = observed_git,
                ContextGroup::FileChanges => {
                    if let Ok(changes) = volatile
                        .file_changes
                        .get_or_init(|| {
                            captured = true;
                            repository
                                .observation
                                .detect_file_changes()
                                .map_err(Arc::new)
                        })
                        .as_ref()
                    {
                        evidence = evidence.with_file_changes(
                            changes.clone().unwrap_or_default(),
                        );
                    }
                }
                ContextGroup::Languages => {
                    if let Ok(languages) = source_evidence
                        .languages
                        .get_or_init(|| {
                            captured = true;
                            detect_source_filesystem(repository, base_dir, true, false)
                                .map(|filesystem| filesystem.languages)
                                .map_err(Arc::new)
                        })
                        .as_ref()
                    {
                        evidence = evidence.with_languages(languages.clone());
                    }
                }
                ContextGroup::Documents => {
                    if let Ok(documents) = source_evidence
                        .documents
                        .get_or_init(|| {
                            captured = true;
                            detect_source_filesystem(repository, base_dir, false, true)
                                .map(|filesystem| filesystem.docs)
                                .map_err(Arc::new)
                        })
                        .as_ref()
                    {
                        evidence = evidence.with_documents_for_source(
                            documents.clone(),
                            base_dir,
                            repository_root,
                            repository.repo_info(),
                        );
                    }
                }
                ContextGroup::Os => {
                    if let Ok(os) = self.cached_os(&mut captured) {
                        evidence = evidence.with_os(Some(os.clone()));
                    }
                }
                // Document identity hashes the host name and repository name.
                // The root document itself is retained by the Darkmatter
                // context, never supplied as evidence.
                ContextGroup::Document => {
                    if let Ok(os) = self.cached_os(&mut captured) {
                        evidence = evidence.with_os(Some(os.clone()));
                    }
                    if repository.failure().is_none() {
                        evidence = evidence.with_git(git_info.clone());
                    }
                }
                ContextGroup::GitHistory => {
                    if repository.failure().is_none()
                        && let Ok(commits) = volatile
                            .recent_commits
                            .get_or_init(|| {
                                captured = true;
                                repository_root
                                    .map(|root| recent_commits_at(root, 10))
                                    .transpose()
                                    .map_err(Arc::new)
                            })
                            .as_ref()
                    {
                        evidence = evidence.with_recent_commits(commits.clone());
                    }
                }
                ContextGroup::Network => {
                    captured = true;
                    evidence = evidence
                        .with_network_interfaces(
                            sniff::network::detect_network_with_request(
                                &sniff::request::NetworkRequest::interfaces_only(),
                            )
                            .ok()
                            .map(|network| network.interfaces),
                        )
                        .with_gateways(sniff::network::detect_default_gateways().ok());
                }
                ContextGroup::Hardware => {
                    if let Ok(hardware) = self
                        .inner
                        .hardware
                        .get_or_init(|| {
                            captured = true;
                            sniff::hardware::detect_hardware_with_request(&HardwareRequest::summary())
                                .map_err(Arc::new)
                        })
                        .as_ref()
                    {
                        evidence = evidence.with_hardware(Some(hardware.clone()));
                    }
                }
                ContextGroup::Gpu => {
                    let gpus = self
                        .inner
                        .gpus
                        .get_or_init(|| {
                            captured = true;
                            sniff::hardware::detect_gpus()
                        })
                        .clone();
                    evidence = evidence.with_gpus(gpus);
                }
            }
            let name = context_group_name(group);
            match (captured, is_volatile(group)) {
                (true, true) => {
                    record_group(&self.inner.work.volatile_observations, name);
                    if let Some(epoch) = epoch {
                        record_group(&epoch.volatile_observations, name);
                    }
                }
                (true, false) => self.record_runtime_evidence_capture(name),
                (false, _) => self.record_runtime_evidence_reuse(name),
            }
        }
        evidence
    }

    /// This invocation's refresh capability for Darkmatter's lazy `current`
    /// root.
    ///
    /// Install it on every `ComposeOptions` and every lifecycle
    /// `EffectiveState` Claudine builds, so a `current.<key>` reference
    /// observes the fact against the retained launch roots instead of taking
    /// Darkmatter's ambient anchored refresh. A capability this invocation does
    /// not hold answers [`CurrentRefresh::Unsupported`], which is `null` plus a
    /// `PartialRuntimeCapture` diagnostic — never a host probe.
    #[must_use]
    pub fn current_provider(&self) -> Arc<dyn CurrentProvider> {
        Arc::new(LaunchRefresh {
            invocation: self.clone(),
        })
    }

    /// [`Self::current_provider`] wrapped in the authority
    /// `EffectiveStateBuilder::with_current_authority` takes.
    #[must_use]
    pub fn current_authority(&self) -> darkmatter::markdown::compose::CurrentAuthority {
        darkmatter::markdown::compose::CurrentAuthority::default()
            .with_provider(self.current_provider())
    }

    /// Observe one cataloged `ctx` key as it stands now.
    ///
    /// Repository topology, the repository root, and the launch directory are
    /// invocation-owned and never rediscovered here; only the mutable facts
    /// layered over them are re-observed, through the *retained* Git handle.
    fn refresh_current(&self, key: &str) -> CurrentRefresh {
        use darkmatter::markdown::compose::{ComposeContext, ContextGroup, ContextRequirements};

        let Some(group) = ContextGroup::for_key(key) else {
            return CurrentRefresh::Unsupported;
        };
        let Some(evidence) = self.refresh_evidence(group) else {
            return CurrentRefresh::Unsupported;
        };
        let requirements = ContextRequirements::from_groups([group]);
        let context =
            ComposeContext::capture_with_evidence(&self.inner.launch_cwd, &requirements, &evidence);
        match context.values().get(key) {
            Some(value) => CurrentRefresh::Observed(value.clone()),
            None => CurrentRefresh::Unsupported,
        }
    }

    /// Re-observe one group's evidence against the retained launch roots.
    ///
    /// Deliberately bypasses the invocation's evidence caches: those exist so
    /// the eager `ctx.*` snapshot is captured once, and reusing them here would
    /// replay the launch observation rather than refresh it.
    ///
    /// ## Returns
    ///
    /// `None` when this invocation holds no capability for `group`, which the
    /// caller reports as [`CurrentRefresh::Unsupported`].
    fn refresh_evidence(
        &self,
        group: darkmatter::markdown::compose::ContextGroup,
    ) -> Option<darkmatter::markdown::compose::ContextCaptureEvidence> {
        use darkmatter::markdown::compose::{ContextCaptureEvidence, ContextGroup};

        // The live process environment, not the frozen launch snapshot: `agent`
        // and `model` derive from `AGENT`/`MODEL`, and the whole point of the
        // lazy mirror is that a wrapper stage which re-exported them since
        // launch is observable. This matches `current_env.<KEY>`, which rereads
        // the process environment for the same reason.
        let evidence = ContextCaptureEvidence::new(std::env::vars().collect());
        let repository = &self.inner.launch_repository;
        let repository_root = self.launch_repository_root_spelling();
        let base_dir = self.inner.launch_cwd.clone();

        let evidence = match group {
            // Fixed for the request; Darkmatter answers these from its own
            // capture and never consults a provider for them.
            ContextGroup::Invocation | ContextGroup::Document => {
                evidence.with_invocation_cwd(Some(base_dir.clone()))
            }
            // Zero-evidence groups: the capture itself is the observation.
            ContextGroup::DateTime | ContextGroup::Agent => evidence,
            ContextGroup::Git => {
                if repository.failure().is_some() {
                    return None;
                }
                evidence.with_git(
                    repository
                        .observation
                        .detect_git(&launch_git_request())
                        .ok()?,
                )
            }
            ContextGroup::FileChanges => {
                if repository.failure().is_some() {
                    return None;
                }
                evidence
                    .with_git(repository.git_info.clone())
                    .with_repository(repository_root.clone(), repository.repo_info().cloned())
                    .with_file_changes(
                        repository
                            .observation
                            .detect_file_changes()
                            .ok()?
                            .unwrap_or_default(),
                    )
            }
            ContextGroup::GitHistory => {
                if repository.failure().is_some() {
                    return None;
                }
                let commits = match repository_root.as_deref() {
                    Some(root) => Some(recent_commits_at(root, 10).ok()?),
                    None => None,
                };
                evidence.with_recent_commits(commits)
            }
            // Package topology is invocation-owned (D3): refreshing it would
            // mean rediscovering the repository, which is exactly what the
            // launch anchor forbids.
            ContextGroup::Repo => {
                if repository.failure().is_some() {
                    return None;
                }
                evidence
                    .with_git(repository.git_info.clone())
                    .with_repository(repository_root.clone(), repository.repo_info().cloned())
            }
            ContextGroup::Languages | ContextGroup::Documents => {
                if repository.failure().is_some() {
                    return None;
                }
                let wants_languages = matches!(group, ContextGroup::Languages);
                let filesystem = detect_source_filesystem(
                    repository,
                    &base_dir,
                    wants_languages,
                    !wants_languages,
                )
                .ok()?;
                let evidence = evidence
                    .with_git(repository.git_info.clone())
                    .with_repository(repository_root.clone(), repository.repo_info().cloned());
                match wants_languages {
                    true => evidence.with_languages(filesystem.languages),
                    false => evidence.with_documents_for_source(
                        filesystem.docs,
                        &base_dir,
                        repository_root.as_deref(),
                        repository.repo_info(),
                    ),
                }
            }
            // Host identity does not change within a run, so the invocation's
            // one detection is also the current observation.
            ContextGroup::Os => {
                let mut captured = false;
                evidence.with_os(Some(self.cached_os(&mut captured).as_ref().ok()?.clone()))
            }
            ContextGroup::Hardware => evidence.with_hardware(Some(
                self.inner
                    .hardware
                    .get_or_init(|| {
                        sniff::hardware::detect_hardware_with_request(&HardwareRequest::summary())
                            .map_err(Arc::new)
                    })
                    .as_ref()
                    .ok()?
                    .clone(),
            )),
            ContextGroup::Gpu => evidence.with_gpus(
                self.inner
                    .gpus
                    .get_or_init(sniff::hardware::detect_gpus)
                    .clone(),
            ),
            ContextGroup::Network => evidence
                .with_network_interfaces(
                    sniff::network::detect_network_with_request(
                        &sniff::request::NetworkRequest::interfaces_only(),
                    )
                    .ok()
                    .map(|network| network.interfaces),
                )
                .with_gateways(sniff::network::detect_default_gateways().ok()),
        };
        Some(evidence)
    }

    /// The invocation's OS observation, detected at most once.
    ///
    /// Deliberately the same request Darkmatter's ambient capture issues, so a
    /// supplied `ctx.os*` costs the same and reads the same as an ambient one.
    /// Locale and timezone are unread by `populate_os` and their probes are not
    /// free — locale shells out to PowerShell on a Windows host with no `LC_*`
    /// set.
    fn cached_os(&self, captured: &mut bool) -> &Result<OsInfo, Arc<sniff::SniffError>> {
        self.inner.os.get_or_init(|| {
            *captured = true;
            let request = OsRequest::full()
                .include_locale(false)
                .include_timezone(false)
                .include_ntp_status(false);
            sniff::os::detect_os_with_request(&request).map_err(Arc::new)
        })
    }

    /// Resolve the repository entry enclosing an authored source directory.
    ///
    /// `base_dir` stays authored: it is what Git discovers against, so a newly
    /// observed repository projects the root the caller named rather than a
    /// resolved-symlink alias. Only `lookup_key` is canonicalized, and only to
    /// address the cache and to test containment against another key.
    fn repository_for_base(&self, base_dir: &Path) -> Arc<RepositoryEntry> {
        let lookup_key = canonical_key(base_dir);
        let mut cache = self
            .inner
            .repositories
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        if let Some(entry) = cache.exact_non_repositories.get(&lookup_key) {
            return entry.clone();
        }

        let mut enclosing = cache
            .repositories
            .iter()
            .map(|(key, entry)| (key.worktree_root.as_path(), entry))
            .filter(|(root, _)| lookup_key.starts_with(root))
            .collect::<Vec<_>>();
        enclosing.sort_by_key(|(root, _)| std::cmp::Reverse(root.components().count()));
        if let Some((_, entry)) = enclosing
            .into_iter()
            .find(|(root, _)| !contains_nested_repository_boundary(&lookup_key, root))
        {
            return entry.clone();
        }

        self.inner
            .work
            .git_root_discoveries
            .fetch_add(1, Ordering::Relaxed);
        let observation = FilesystemObservation::discover(base_dir);
        let identity = match observation.repository_identity() {
            Ok(Some(identity)) => identity.clone(),
            Ok(None) => {
                let entry = Arc::new(RepositoryEntry::absent(observation));
                cache
                    .exact_non_repositories
                    .insert(lookup_key, entry.clone());
                return entry;
            }
            Err(error) => {
                let entry = Arc::new(RepositoryEntry::failed(observation, error));
                cache
                    .exact_non_repositories
                    .insert(lookup_key, entry.clone());
                return entry;
            }
        };
        let key = RepositoryKey::from_identity(&identity);
        if let Some(entry) = cache.repositories.get(&key) {
            return entry.clone();
        }

        let git_info = match observation.detect_git(&launch_git_request()) {
            Ok(git_info) => git_info,
            Err(error) => {
                let mut entry = RepositoryEntry::failed(observation, error);
                entry.identity = Some(identity);
                let entry = Arc::new(entry);
                cache.repositories.insert(key, entry.clone());
                return entry;
            }
        };
        let entry = Arc::new(RepositoryEntry::present(
            observation,
            identity,
            git_info,
            None,
        ));
        cache.repositories.insert(key, entry.clone());
        entry
    }

    fn ensure_topology(&self, entry: &Arc<RepositoryEntry>) {
        if entry.identity.is_none() || entry.failure.is_some() {
            return;
        }
        let mut initialized_here = false;
        let _ = entry.topology.get_or_init(|| {
            initialized_here = true;
            self.inner
                .work
                .topology_probes
                .fetch_add(1, Ordering::Relaxed);
            topology_with_observation(&entry.observation).map_err(Arc::new)
        });
        if !initialized_here {
            self.inner
                .work
                .topology_reuses
                .fetch_add(1, Ordering::Relaxed);
        }
    }

    pub fn work_snapshot(&self) -> InvocationWorkSnapshot {
        self.inner.work.snapshot()
    }

    /// Record one runtime-evidence group whose value this call computed.
    pub fn record_runtime_evidence_capture(&self, group: impl Into<String>) {
        record_group(&self.inner.work.runtime_evidence_captures, group);
    }

    /// Record one runtime-evidence group answered without computing anything.
    pub fn record_runtime_evidence_reuse(&self, group: impl Into<String>) {
        record_group(&self.inner.work.runtime_evidence_reuses, group);
    }

    pub fn record_system_prompt_lookup(&self) {
        self.inner
            .work
            .system_prompt_lookups
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_system_prompt_timing(
        &self,
        stage: impl Into<String>,
        elapsed: std::time::Duration,
    ) {
        let mut timings = self
            .inner
            .work
            .system_prompt_timings
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *timings.entry(stage.into()).or_default() += elapsed;
    }

    pub fn record_compose_operation(&self) {
        self.inner
            .work
            .compose_operations
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_harness_eligibility_parse(&self) {
        self.inner
            .work
            .harness_eligibility_parses
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_harness_materialization(&self) {
        self.inner
            .work
            .harness_materializations
            .fetch_add(1, Ordering::Relaxed);
    }

    pub fn record_ambient_fallback(&self) {
        self.inner
            .work
            .ambient_fallbacks
            .fetch_add(1, Ordering::Relaxed);
    }

    /// Record that a canonical consumer received a populated epoch snapshot.
    pub fn record_prepared_context_consumer(&self, consumer: PreparedContextConsumer) {
        let mut consumers = self
            .inner
            .work
            .prepared_context_consumers
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *consumers.entry(consumer.as_str().to_string()).or_default() += 1;
    }
}

fn observe_repository(
    root: &Path,
    observation: FilesystemObservation,
    git_request: GitRequest,
    include_topology: bool,
    work: &InvocationWork,
) -> (Arc<RepositoryEntry>, sniff::SniffResult) {
    let identity = match observation.repository_identity() {
        Ok(Some(identity)) => identity.clone(),
        Ok(None) => {
            return (
                Arc::new(RepositoryEntry::absent(observation)),
                sniff::SniffResult::default(),
            );
        }
        Err(error) => {
            return (
                Arc::new(RepositoryEntry::failed(observation, error)),
                sniff::SniffResult::default(),
            );
        }
    };

    // Keep repository identity in summary captures without branch/tracking scans.
    let filesystem = FilesystemRequest::new()
        .git(git_request.metadata(GitMetadataRequest::none().remotes(true)))
        .without_file_inventory()
        .without_docs()
        .without_formatting();
    let filesystem = if include_topology {
        work.topology_probes.fetch_add(1, Ordering::Relaxed);
        filesystem.repo(RepoRequest::structure())
    } else {
        filesystem.without_repo()
    };
    let plan = DetectionPlan::new()
        .base_dir(root.to_path_buf())
        .without_os()
        .without_hardware()
        .without_network()
        .filesystem(filesystem);

    match sniff::detect_with_plan_and_filesystem_observation(plan, observation.clone()) {
        Ok(observed) => {
            let git_info = observed
                .result
                .filesystem
                .as_ref()
                .and_then(|filesystem| filesystem.git.clone());
            let repo_info = observed
                .result
                .filesystem
                .as_ref()
                .and_then(|filesystem| filesystem.repo.clone());
            let topology = include_topology.then_some(Ok(repo_info));
            (
                Arc::new(RepositoryEntry::present(
                    observed.filesystem_observation,
                    identity,
                    git_info,
                    topology,
                )),
                observed.result,
            )
        }
        Err(error) => (
            Arc::new(RepositoryEntry::failed(observation, error)),
            sniff::SniffResult::default(),
        ),
    }
}

fn topology_with_observation(
    observation: &FilesystemObservation,
) -> Result<Option<RepoInfo>, sniff::SniffError> {
    let request = FilesystemRequest::new()
        .git(GitRequest::identity())
        .repo(RepoRequest::structure())
        .without_file_inventory()
        .without_docs()
        .without_formatting();
    sniff::filesystem::detect_filesystem_with_observation(
        observation.observed_root(),
        &request,
        observation,
    )
    .map(|filesystem| filesystem.repo)
}

/// The Git request every launch observation and every `current.*` Git refresh
/// issues.
///
/// Summary omits remotes by default, but `ctx.repo` needs their identity.
fn launch_git_request() -> GitRequest {
    GitRequest::summary().metadata(GitMetadataRequest::none().remotes(true))
}

fn detect_source_filesystem(
    repository: &RepositoryEntry,
    base_dir: &Path,
    languages: bool,
    documents: bool,
) -> Result<sniff::filesystem::FilesystemInfo, sniff::SniffError> {
    let request = FilesystemRequest::new()
        .git(GitRequest::identity())
        .without_repo()
        .without_formatting()
        .include_docs(documents);
    let request = if languages {
        request
    } else {
        request.without_file_inventory()
    };
    let observation = repository.observation.for_root(base_dir)?;
    sniff::filesystem::detect_filesystem_with_observation(base_dir, &request, &observation)
}

fn record_group(counts: &Mutex<BTreeMap<String, usize>>, group: impl Into<String>) {
    let mut counts = counts
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    *counts.entry(group.into()).or_insert(0) += 1;
}

/// Whether `group` is Git working state, observed per composition run rather
/// than per invocation.
fn is_volatile(group: darkmatter::markdown::compose::ContextGroup) -> bool {
    use darkmatter::markdown::compose::ContextGroup;

    matches!(
        group,
        ContextGroup::Git | ContextGroup::FileChanges | ContextGroup::GitHistory
    )
}

fn context_group_name(group: darkmatter::markdown::compose::ContextGroup) -> &'static str {
    use darkmatter::markdown::compose::ContextGroup;

    match group {
        ContextGroup::Invocation => "invocation",
        ContextGroup::DateTime => "datetime",
        ContextGroup::Git => "git",
        ContextGroup::GitHistory => "git_history",
        ContextGroup::Repo => "repo",
        ContextGroup::FileChanges => "file_changes",
        ContextGroup::Languages => "languages",
        ContextGroup::Documents => "documents",
        ContextGroup::Os => "os",
        ContextGroup::Hardware => "hardware",
        ContextGroup::Gpu => "gpu",
        ContextGroup::Agent => "agent",
        ContextGroup::Document => "document",
        ContextGroup::Network => "network",
    }
}

/// Build a file-resolution context through Darkmatter's builder, registering
/// Claudine's prompt conventions as the snapshot's extra `@` roots.
///
/// `snapshot` is anchored at the context's request directory. The repository
/// comes from the invocation's own observation cache, so the builder does not
/// discover it again.
///
/// `launch_scope` is `None` for the launch context itself, whose scope the
/// builder captures naturally from the request directory and the supplied
/// repository. A context rebuilt around a source in another repository or an
/// external prompt directory passes the launch context's captured scope
/// instead: the conventions are registered against the launch local root and
/// the scope is restored last, so `@` keeps searching the launch tree
/// (ruling 2 of 2026-09-23-local-before-home) while `./`, bare, `&`, and `^`
/// references keep the source anchors.
pub(crate) fn build_file_resolution_context(
    snapshot: &RequestSnapshot,
    source_path: Option<&Path>,
    repository_root: Option<&Path>,
    repo_info: Option<&RepoInfo>,
    launch_scope: Option<&LaunchMagicScope>,
) -> Result<FileResolutionContext, ContextBuildError> {
    let base_dir = snapshot.request_dir();
    let catalog = repository_root.map(|root| match repo_info {
        Some(repo_info) => darkmatter::markdown::compose::repository_scope_catalog(repo_info, root)
            .expect("retained repository topology must project to valid absolute scopes"),
        None => RepositoryScopeCatalog::new(root, Vec::new(), Vec::new(), PackageAreaFallback::None)
            .expect("a discovered repository root is absolute"),
    });
    let (local_root, package_area_root, package_root) = match launch_scope {
        Some(scope) => (
            scope.local_root().to_path_buf(),
            scope.package_area().map(Path::to_path_buf),
            scope.package_root().map(Path::to_path_buf),
        ),
        None => {
            let scope = catalog.as_ref().map(|catalog| catalog.scope_for(base_dir));
            (
                repository_root.unwrap_or(base_dir).to_path_buf(),
                scope.as_ref().and_then(|scope| scope.package_area_root()).map(Path::to_path_buf),
                scope.as_ref().and_then(|scope| scope.package_root()).map(Path::to_path_buf),
            )
        }
    };
    let snapshot = with_prompt_magic_roots(
        snapshot.clone(),
        &local_root,
        package_area_root.as_deref(),
        package_root.as_deref(),
        snapshot.home(),
    );
    let mut context = build_resolution_context_with_catalog(&snapshot, catalog)?;
    if let Some(source_path) = source_path {
        context = context.with_source_path(source_path);
    }
    if let Some(scope) = launch_scope {
        context = context.with_launch_magic_scope(scope.clone());
    }
    Ok(context)
}

/// Whether a `.git` boundary separates a directory from its enclosing root.
///
/// Both arguments must be [`canonical_key`] form; the walk compares components
/// against `enclosing_root` to terminate, so a canonical/authored mix would
/// walk past the root and probe unrelated ancestors.
fn contains_nested_repository_boundary(base_dir: &Path, enclosing_root: &Path) -> bool {
    let mut cursor = base_dir;
    while cursor != enclosing_root && cursor.starts_with(enclosing_root) {
        if std::fs::symlink_metadata(cursor.join(".git")).is_ok() {
            return true;
        }
        let Some(parent) = cursor.parent() else {
            break;
        };
        cursor = parent;
    }
    false
}

fn paths_equivalent(left: &Path, right: &Path) -> bool {
    canonical_key(left) == canonical_key(right)
}

/// Project a repository root into the spelling family of `base_dir`.
///
/// A cached [`RepositoryEntry`] keeps the root as authored by whichever call
/// first observed the repository. A later source can reach the same entry
/// through a different spelling of the tree (macOS `/var` → `/private/var`,
/// a symlinked launch directory), and `FileResolutionContext` containment is
/// lexical — pairing that base with a foreign-spelled root would reject a
/// valid source. The ancestor of `base_dir` that names the root carries the
/// caller's spelling; the authored root is kept when the spellings already
/// agree, or when no ancestor matches (a foreign base cannot be made valid).
fn repo_root_in_base_spelling(base_dir: &Path, authored_root: &Path) -> PathBuf {
    if base_dir.starts_with(authored_root) {
        return authored_root.to_path_buf();
    }
    let root_key = canonical_key(authored_root);
    base_dir
        .ancestors()
        .find(|ancestor| canonical_key(ancestor) == root_key)
        .map_or_else(|| authored_root.to_path_buf(), Path::to_path_buf)
}

/// Cache-key form of a path, canonicalized when the filesystem allows.
///
/// Canonicalization fails for a missing path or an unreadable ancestor; the
/// authored path is then its own key, degrading to the per-path cache misses
/// that predate symmetric keying rather than failing the invocation.
///
/// Keys are only ever compared against other keys. A fallback key is an
/// authored spelling, so a canonical key must share that spelling family: a
/// Windows verbatim (`\\?\`) key would never equal the fallback key for the
/// same directory.
fn canonical_key(path: &Path) -> PathBuf {
    biscuit_file::canonicalize_simplified(path).unwrap_or_else(|_| path.to_path_buf())
}

/// `path` made absolute against the snapshot's request directory.
fn absolutize(snapshot: &RequestSnapshot, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        snapshot.request_dir().join(path)
    }
}

impl darkmatter::markdown::compose::ContextExtension for InvocationContext {
    fn extend(
        &self,
        context: &mut darkmatter::markdown::compose::ComposeContext,
        required: &darkmatter::markdown::compose::ContextRequirements,
    ) -> bool {
        self.extend_launch_context(context, required)
    }
}

impl darkmatter::markdown::compose::ContextExtension for DocumentEpoch {
    fn extend(
        &self,
        context: &mut darkmatter::markdown::compose::ComposeContext,
        required: &darkmatter::markdown::compose::ContextRequirements,
    ) -> bool {
        self.extend_launch_context(context, required)
    }
}

#[cfg(test)]
mod tests;

/// The newest `count` commits of the repository containing `root`.
///
/// A `root` outside any repository is an error, not an empty set: both callers
/// have already established that a repository is present.
fn recent_commits_at(
    root: &std::path::Path,
    count: usize,
) -> sniff::Result<sniff::filesystem::git::RecentCommits> {
    use sniff::filesystem::git::{GitRepo, RecentCommits, RecentCommitsOptions};
    let repo = GitRepo::discover(root)?
        .ok_or_else(|| sniff::SniffError::NotARepository(root.to_path_buf()))?;
    RecentCommits::collect(&repo, &RecentCommitsOptions::new().count(count))
}
