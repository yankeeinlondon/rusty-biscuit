//! Request-scoped launch and repository evidence for Claudine invocations.

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use biscuit_file::{FileResolutionContext, PathPosition, home_dir};
use sniff::filesystem::{FilesystemObservation, GitRepositoryIdentity};
use sniff::filesystem::docs::MarkdownMeta;
use sniff::filesystem::git::{FileChange, GitInfo};
use sniff::filesystem::repo::RepoInfo;
use sniff::filesystem::LanguageBreakdown;
use sniff::hardware::{GpuInfo, HardwareInfo};
use sniff::os::OsInfo;
use sniff::request::{
    DetectionPlan, FilesystemRequest, GitRequest, HardwareRequest, OsRequest, RepoRequest,
};

use crate::composition::{
    LaunchWorkspaceContext, prompt_magic_roots,
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
    /// The launch working directory could not be captured.
    #[error("failed to capture the launch working directory: {0}")]
    CurrentDirectory(#[source] std::io::Error),
    /// A resolved source does not have an authoring directory.
    #[error("resolved source path has no parent directory: {0}")]
    SourceWithoutParent(PathBuf),
}

/// Request-local accounting for discovery and preparation work.
///
/// `runtime_evidence_captures` counts, per group, the calls that ran the
/// group's initializer; `runtime_evidence_reuses` counts the calls a cache or
/// already-retained evidence answered. A group requested `n` times therefore
/// sums to `n` across the two maps, and a second capture is a duplicate
/// computation rather than an invisible repeat.
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
    pub launch_context_constructions: usize,
    pub launch_context_extensions: usize,
    pub runtime_evidence_captures: BTreeMap<String, usize>,
    pub runtime_evidence_reuses: BTreeMap<String, usize>,
    pub system_prompt_timings: BTreeMap<String, std::time::Duration>,
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
    runtime_evidence_captures: Mutex<BTreeMap<String, usize>>,
    runtime_evidence_reuses: Mutex<BTreeMap<String, usize>>,
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

#[derive(Debug, Default)]
struct SourceEvidence {
    file_changes: OnceLock<Result<Option<Vec<FileChange>>, Arc<sniff::SniffError>>>,
    languages: OnceLock<Result<Option<LanguageBreakdown>, Arc<sniff::SniffError>>>,
    documents: OnceLock<Result<Option<Vec<MarkdownMeta>>, Arc<sniff::SniffError>>>,
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

#[derive(Debug)]
struct InvocationInner {
    launch_cwd: PathBuf,
    home_dir: Option<PathBuf>,
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

impl InvocationContext {
    /// Capture a composition invocation from the ambient launch directory.
    pub fn capture() -> Result<Self, InvocationContextError> {
        let cwd = std::env::current_dir().map_err(InvocationContextError::CurrentDirectory)?;
        Ok(Self::capture_at(&cwd))
    }

    /// Capture a composition invocation at an explicit launch directory.
    pub fn capture_at(cwd: &Path) -> Self {
        Self::capture_inner(cwd, GitRequest::summary(), true)
    }

    /// Capture direct-wrapper startup evidence.
    ///
    /// Promptless root wrappers retain Git identity but omit topology. A later
    /// source that needs package evidence initializes the same entry's
    /// topology cell instead of discovering Git again.
    pub fn capture_for_wrapper(cwd: &Path, capture_git_status: bool) -> Self {
        let cwd = absolutize(cwd);
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
        Self::capture_with_observation(&cwd, request, !promptless_at_repo_root, observation)
    }

    fn capture_inner(cwd: &Path, git_request: GitRequest, include_topology: bool) -> Self {
        let cwd = absolutize(cwd);
        let observation = FilesystemObservation::discover(&cwd);
        Self::capture_with_observation(&cwd, git_request, include_topology, observation)
    }

    fn capture_with_observation(
        cwd: &Path,
        git_request: GitRequest,
        include_topology: bool,
        observation: FilesystemObservation,
    ) -> Self {
        let cwd = absolutize(cwd);
        let environment = std::env::vars().collect::<HashMap<_, _>>();
        let home_dir = home_dir();
        let work = InvocationWork::default();
        work.git_root_discoveries.fetch_add(1, Ordering::Relaxed);

        let (launch_repository, launch_result) = observe_repository(
            &cwd,
            observation,
            git_request,
            include_topology,
            &work,
        );

        let launch_repository_root = launch_repository.repo_root();
        let launch_repo_info = launch_repository.repo_info();
        let (package_area_root, package_root) = package_roots(&cwd, launch_repo_info);
        let launch_file_resolution = build_file_resolution_context(
            &cwd,
            None,
            home_dir.clone(),
            environment.clone(),
            launch_repository_root,
            package_area_root.as_deref(),
            package_root.as_deref(),
        );

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

        Self {
            inner: Arc::new(InvocationInner {
                launch_cwd: cwd,
                home_dir,
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
        }
    }

    pub fn launch_cwd(&self) -> &Path {
        &self.inner.launch_cwd
    }

    pub fn home_dir(&self) -> Option<&Path> {
        self.inner.home_dir.as_deref()
    }

    pub fn environment(&self) -> &HashMap<String, String> {
        &self.inner.environment
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
        let (package_area_root, package_root) = package_roots(&base_dir, entry.repo_info());
        let file_resolution = build_file_resolution_context(
            &base_dir,
            Some(&source_path),
            self.inner.home_dir.clone(),
            self.inner.environment.clone(),
            repository_root.as_deref(),
            package_area_root.as_deref(),
            package_root.as_deref(),
        );

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
    pub fn runtime_evidence(
        &self,
        source: &SourceContext,
        requirements: &darkmatter::markdown::compose::ContextRequirements,
    ) -> darkmatter::markdown::compose::ContextCaptureEvidence {
        self.runtime_evidence_for(
            &source.repository.inner,
            &source.base_dir,
            source.repository_root.as_deref(),
            requirements,
        )
    }

    /// Capture prepared plain `ctx.*` from the invocation's launch evidence.
    ///
    /// The launch directory and its retained repository observation are paired
    /// here so callers cannot accidentally combine launch coordinates with a
    /// document source's repository or topology.
    pub fn capture_launch_context(
        &self,
        requirements: &darkmatter::markdown::compose::ContextRequirements,
    ) -> darkmatter::markdown::compose::ComposeContext {
        self.ensure_launch_topology_for(requirements);
        let evidence = self.runtime_evidence_for(
            &self.inner.launch_repository,
            &self.inner.launch_cwd,
            self.inner.launch_repository.repo_root(),
            requirements,
        );
        self.inner
            .work
            .launch_context_constructions
            .fetch_add(1, Ordering::Relaxed);
        darkmatter::markdown::compose::ComposeContext::capture_with_evidence(
            &self.inner.launch_cwd,
            requirements,
            &evidence,
        )
    }

    /// Extend one launch snapshot with groups newly required inside its epoch.
    ///
    /// Only the difference between `required` and `captured` is projected. The
    /// existing context keeps its clock, environment, and target overrides;
    /// the retained launch observation supplies every added group.
    pub fn extend_launch_context(
        &self,
        context: &mut darkmatter::markdown::compose::ComposeContext,
        captured: &mut darkmatter::markdown::compose::ContextRequirements,
        required: &darkmatter::markdown::compose::ContextRequirements,
    ) {
        let missing = required.missing_from(captured);
        if missing.is_empty() {
            return;
        }

        self.ensure_launch_topology_for(&missing);
        let evidence = self.runtime_evidence_for(
            &self.inner.launch_repository,
            &self.inner.launch_cwd,
            self.inner.launch_repository.repo_root(),
            &missing,
        );
        context.extend_with_evidence(&self.inner.launch_cwd, &missing, &evidence);
        captured.extend(&missing);
        self.inner
            .work
            .launch_context_extensions
            .fetch_add(1, Ordering::Relaxed);
    }

    fn ensure_launch_topology_for(
        &self,
        requirements: &darkmatter::markdown::compose::ContextRequirements,
    ) {
        use darkmatter::markdown::compose::ContextGroup;

        let needs_repository = requirements.iter().any(|group| {
            matches!(
                group,
                ContextGroup::Repo
                    | ContextGroup::FileChanges
                    | ContextGroup::Languages
                    | ContextGroup::Documents
            )
        });
        if needs_repository && self.inner.launch_repository.topology.get().is_none() {
            self.ensure_topology(&self.inner.launch_repository);
        }
    }

    fn runtime_evidence_for(
        &self,
        repository: &Arc<RepositoryEntry>,
        base_dir: &Path,
        repository_root: Option<&Path>,
        requirements: &darkmatter::markdown::compose::ContextRequirements,
    ) -> darkmatter::markdown::compose::ContextCaptureEvidence {
        use darkmatter::markdown::compose::{ContextCaptureEvidence, ContextGroup};

        let mut evidence = ContextCaptureEvidence::new(self.inner.environment.clone());
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

        let needs_repository = requirements.iter().any(|group| {
            matches!(
                group,
                ContextGroup::Repo
                    | ContextGroup::FileChanges
                    | ContextGroup::Languages
                    | ContextGroup::Documents
            )
        });
        if (requirements.contains(ContextGroup::Git) || needs_repository)
            && repository.failure().is_none()
        {
            evidence = evidence.with_git(repository.git_info.clone());
        }
        if needs_repository
            && repository.failure().is_none()
            && !matches!(repository.topology.get(), Some(Err(_)))
        {
            evidence = evidence.with_repository(
                repository_root.map(Path::to_path_buf),
                repository.repo_info().cloned(),
            );
        }

        for group in requirements.iter() {
            // Set only from inside a cache initializer, so the counters below
            // separate the one call that computed the evidence from every call
            // the cache answered.
            let mut captured = false;
            match group {
                ContextGroup::DateTime | ContextGroup::Agent => {}
                ContextGroup::Git => {
                    if repository.failure().is_none() {
                        evidence = evidence.with_git(repository.git_info.clone());
                    }
                }
                ContextGroup::Repo => {
                    if repository.failure().is_none()
                        && !matches!(repository.topology.get(), Some(Err(_)))
                    {
                        evidence = evidence.with_repository(
                            repository_root.map(Path::to_path_buf),
                            repository.repo_info().cloned(),
                        );
                    }
                }
                ContextGroup::FileChanges => {
                    if let Ok(changes) = source_evidence
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
                    if let Ok(os) = self
                        .inner
                        .os
                        .get_or_init(|| {
                            captured = true;
                            // Deliberately the same request Darkmatter's ambient
                            // capture issues, so a supplied `ctx.os*` costs the
                            // same and reads the same as an ambient one. Locale
                            // and timezone are unread by `populate_os` and their
                            // probes are not free — locale shells out to
                            // PowerShell on a Windows host with no `LC_*` set.
                            let request = OsRequest::full()
                                .include_locale(false)
                                .include_timezone(false)
                                .include_ntp_status(false);
                            sniff::os::detect_os_with_request(&request).map_err(Arc::new)
                        })
                        .as_ref()
                    {
                        evidence = evidence.with_os(Some(os.clone()));
                    }
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
            if captured {
                self.record_runtime_evidence_capture(name);
            } else {
                self.record_runtime_evidence_reuse(name);
            }
        }
        evidence
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

        let git_info = match observation.detect_git(&GitRequest::summary()) {
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

    let filesystem = FilesystemRequest::new()
        .git(git_request)
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

fn context_group_name(group: darkmatter::markdown::compose::ContextGroup) -> &'static str {
    use darkmatter::markdown::compose::ContextGroup;

    match group {
        ContextGroup::DateTime => "datetime",
        ContextGroup::Git => "git",
        ContextGroup::Repo => "repo",
        ContextGroup::FileChanges => "file_changes",
        ContextGroup::Languages => "languages",
        ContextGroup::Documents => "documents",
        ContextGroup::Os => "os",
        ContextGroup::Hardware => "hardware",
        ContextGroup::Gpu => "gpu",
        ContextGroup::Agent => "agent",
    }
}

fn build_file_resolution_context(
    base_dir: &Path,
    source_path: Option<&Path>,
    home_dir: Option<PathBuf>,
    environment: HashMap<String, String>,
    repository_root: Option<&Path>,
    package_area_root: Option<&Path>,
    package_root: Option<&Path>,
) -> FileResolutionContext {
    let mut context = FileResolutionContext::from_snapshot(base_dir, home_dir, environment);
    if let Some(source_path) = source_path {
        context = context.with_source_path(source_path);
    }
    if let Some(repository_root) = repository_root {
        context = context.with_repository_root(repository_root);
    }
    if let Some(package_area_root) = package_area_root {
        context = context.with_package_area(package_area_root);
    }
    for root in prompt_magic_roots(
        repository_root,
        package_area_root,
        package_root,
        context.home_dir(),
    ) {
        context = context.add_magic_path(root, PathPosition::Start);
    }
    context
}

fn package_roots(base_dir: &Path, repo: Option<&RepoInfo>) -> (Option<PathBuf>, Option<PathBuf>) {
    let package_area = repo.and_then(|repo| {
        repo.package_area_label_for_dir(base_dir).map(|area| {
            if area.as_ref() == "root" {
                repo.root.clone()
            } else {
                repo.root.join(area.as_ref())
            }
        })
    });
    let package = repo
        .and_then(|repo| repo.package_for_dir(base_dir))
        .map(|package| package.path.clone());
    (package_area, package)
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
/// Keys are only ever compared against other keys. On Windows `canonicalize`
/// yields a `\\?\` verbatim path, which must never reach a projection or a
/// comparison against an authored path.
fn canonical_key(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn absolutize(path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|cwd| cwd.join(path))
            .unwrap_or_else(|_| path.to_path_buf())
    }
}

#[cfg(test)]
mod tests;
