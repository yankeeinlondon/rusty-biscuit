//! The request snapshot, the context builder, and the prepared request.
//!
//! A request path obtains its [`FileResolutionContext`] in exactly one way:
//! [`build_resolution_context`] over a [`RequestSnapshot`], or a derivation of
//! a context that was built that way. [`ComposeRequest`] pairs that context
//! with the [`ComposeOptions`] it composes under, and every compose entry
//! point takes one.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use biscuit_file::{
    FileReference, FileReferenceError, FileResolutionContext, MagicPathTier, PathPosition,
    RepositoryScopeCatalog, ResolutionFailure,
};

use super::ContextMergeDiagnostic;
use super::capture::{ContextGroup, capture_runtime_context_for_groups};
use super::options::ComposeOptions;
use super::repository_scope::RepositoryObservation;

/// The process state one request resolves file references against.
///
/// It names the request directory and carries the home directory, the
/// environment, the caller's extra `@` roots, and the reference that opened
/// the source document. [`new`](Self::new) reads nothing from the process:
/// it starts with no home and an empty environment, so a library call names
/// every input it depends on. Only a binary's entry point calls
/// [`from_process`](Self::from_process).
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::compose::{RequestSnapshot, build_resolution_context};
///
/// let dir = std::env::temp_dir();
/// let snapshot = RequestSnapshot::new(&dir).with_home(Some(dir.clone()));
/// let context = build_resolution_context(&snapshot).unwrap();
/// assert_eq!(context.request_cwd(), dir.as_path());
/// ```
#[derive(Debug, Clone)]
pub struct RequestSnapshot {
    request_dir: PathBuf,
    home: Option<PathBuf>,
    env: HashMap<String, String>,
    magic_roots: Vec<(PathBuf, PathPosition, MagicPathTier)>,
    opening_reference: Option<(FileReference, PathBuf)>,
}

impl RequestSnapshot {
    /// A snapshot anchored at `request_dir`, with no home directory and an
    /// empty environment.
    pub fn new(request_dir: impl Into<PathBuf>) -> Self {
        Self {
            request_dir: request_dir.into(),
            home: None,
            env: HashMap::new(),
            magic_roots: Vec::new(),
            opening_reference: None,
        }
    }

    /// The running process's current directory, home directory, and
    /// environment.
    ///
    /// The home is read by [`biscuit_file::home_dir`]: `HOME`, or
    /// `USERPROFILE` on native Windows, with the platform's profile lookup as
    /// the fallback; a relative home is no home. A caller that launches a
    /// binary therefore chooses its home through the child's environment. The
    /// environment is read through [`biscuit_file::capture_env`], which skips
    /// non-UTF-8 variables. This is the only reader of process state on a
    /// request path; only binaries call it.
    ///
    /// ## Errors
    ///
    /// Returns the I/O error when the current directory cannot be read.
    pub fn from_process() -> std::io::Result<Self> {
        Ok(Self::new(std::env::current_dir()?)
            .with_home(biscuit_file::home_dir())
            .with_env(biscuit_file::capture_env()))
    }

    /// Sets the home directory `~` references resolve against.
    #[must_use]
    pub fn with_home(mut self, home: Option<PathBuf>) -> Self {
        self.home = home;
        self
    }

    /// Sets the environment `{{VAR}}` references, `ctx.env`, and `env.*` read.
    #[must_use]
    pub fn with_env(mut self, env: HashMap<String, String>) -> Self {
        self.env = env;
        self
    }

    /// Adds an `@` search root whose tier is inferred from its location.
    #[must_use]
    pub fn with_magic_root(self, path: impl Into<PathBuf>, position: PathPosition) -> Self {
        self.with_magic_root_tier(path, position, MagicPathTier::Inferred)
    }

    /// Adds an `@` search root with an explicit tier.
    ///
    /// Roots are registered in the order they are added.
    #[must_use]
    pub fn with_magic_root_tier(
        mut self,
        path: impl Into<PathBuf>,
        position: PathPosition,
        tier: MagicPathTier,
    ) -> Self {
        self.magic_roots.push((path.into(), position, tier));
        self
    }

    /// Records the reference that opened the source document and the path it
    /// resolved to, so a document opened as `~/notes/x.md` keeps `~` as its
    /// tree root.
    #[must_use]
    pub fn with_opening_reference(mut self, reference: FileReference, resolved: PathBuf) -> Self {
        self.opening_reference = Some((reference, resolved));
        self
    }

    /// This snapshot anchored at `request_dir` instead.
    ///
    /// Home, environment, and extra `@` roots are kept. The opening reference
    /// is dropped: it was resolved for the original request directory.
    #[must_use]
    pub fn at_request_dir(&self, request_dir: impl Into<PathBuf>) -> Self {
        Self {
            request_dir: request_dir.into(),
            home: self.home.clone(),
            env: self.env.clone(),
            magic_roots: self.magic_roots.clone(),
            opening_reference: None,
        }
    }

    /// The directory the request is anchored at.
    pub fn request_dir(&self) -> &Path {
        &self.request_dir
    }

    /// The home directory, when the snapshot has one.
    pub fn home(&self) -> Option<&Path> {
        self.home.as_deref()
    }

    /// The environment.
    pub fn env(&self) -> &HashMap<String, String> {
        &self.env
    }

    /// The extra `@` roots, in registration order.
    pub fn magic_roots(&self) -> &[(PathBuf, PathPosition, MagicPathTier)] {
        &self.magic_roots
    }

    /// The reference that opened the source document and its resolved path.
    pub fn opening_reference(&self) -> Option<(&FileReference, &Path)> {
        self.opening_reference
            .as_ref()
            .map(|(reference, resolved)| (reference, resolved.as_path()))
    }
}

/// Why [`build_resolution_context`] could not build a context.
#[derive(Debug, thiserror::Error)]
pub enum ContextBuildError {
    /// Repository discovery at the request directory failed, as opposed to
    /// finding no repository (which is not an error).
    #[error("repository discovery failed at `{}`: {detail}", dir.display())]
    Discovery {
        /// The request directory discovery started from.
        dir: PathBuf,
        /// The discovery error.
        detail: String,
    },
    /// The built context failed validation.
    #[error("invalid file-resolution context for request directory `{}`: {source}", dir.display())]
    Invalid {
        /// The request directory the context was built for.
        dir: PathBuf,
        /// The validation failure.
        #[source]
        source: FileReferenceError,
    },
    /// [`ComposeRequest::with_context`] was given a context that names a
    /// repository but no configuration snapshot
    /// ([`ComposeOptions::with_config_snapshot`], or a host capture on the
    /// options' `ctx` that answers for that repository). The request would need the
    /// repository's committed policy, which only the caller's capture holds;
    /// it is never rediscovered and never skipped.
    #[error(
        "the request at `{}` names the repository `{}` but was given no configuration snapshot for it",
        dir.display(),
        repository.display()
    )]
    ConfigSnapshotRequired {
        /// The request directory.
        dir: PathBuf,
        /// The repository root the context names.
        repository: PathBuf,
    },
    /// The request's configuration could not be loaded.
    #[error("{source}")]
    Config {
        /// The request directory.
        dir: PathBuf,
        /// The load failure, with every located problem.
        #[source]
        source: crate::config::load::ConfigError,
    },
}

impl ContextBuildError {
    /// The request directory the failing build was anchored at.
    pub fn request_dir(&self) -> &Path {
        match self {
            Self::Discovery { dir, .. }
            | Self::Invalid { dir, .. }
            | Self::ConfigSnapshotRequired { dir, .. }
            | Self::Config { dir, .. } => dir,
        }
    }

    /// The failure class, as file-reference errors report it.
    ///
    /// A discovery or configuration failure is
    /// [`ResolutionFailure::MissingContext`], the class biscuit-file gives its
    /// own Git discovery errors.
    pub fn resolution_failure(&self) -> ResolutionFailure {
        match self {
            Self::Discovery { .. } | Self::ConfigSnapshotRequired { .. } | Self::Config { .. } => {
                ResolutionFailure::MissingContext
            }
            Self::Invalid { source, .. } => source.resolution_failure(),
        }
    }
}

/// Builds the file-resolution context for one request.
///
/// In order: the snapshot's directory, home, and environment; repository
/// discovery from the request directory and its package scope catalog (which
/// also fixes the launch `@` scope); the snapshot's extra `@` roots; the
/// opening reference's derivation; and validation. A successful build emits
/// one `debug` event naming the request directory and the `base_dir` origin.
///
/// ## Errors
///
/// - [`ContextBuildError::Discovery`] when repository discovery fails. No
///   repository is not an error.
/// - [`ContextBuildError::Invalid`] when the built context fails
///   [`validate`](FileResolutionContext::validate), for example a relative
///   request directory or an opening reference outside the request's tree.
pub fn build_resolution_context(
    snapshot: &RequestSnapshot,
) -> Result<FileResolutionContext, ContextBuildError> {
    let observation = discover_repository(snapshot.request_dir())?.observation;
    build_with_observation(snapshot, observation.as_deref())
}

/// [`build_resolution_context`], also returning the tree committed at `HEAD`
/// of the repository its discovery found, pinned by that same discovery.
///
/// For a host that keeps the context and loads the request's configuration
/// itself ([`ConfigSnapshot::load_captured`]), so the repository is
/// discovered once. The tree is `None` outside a (non-bare) repository.
///
/// ## Errors
///
/// As [`build_resolution_context`]. A failure to read the committed tree is
/// not an error here; it is returned for the configuration load to report.
///
/// [`ConfigSnapshot::load_captured`]: crate::config::load::ConfigSnapshot::load_captured
pub fn capture_resolution_context(
    snapshot: &RequestSnapshot,
) -> Result<(FileResolutionContext, Option<crate::config::load::CommittedTreeCapture>), ContextBuildError> {
    let observation = discover_repository(snapshot.request_dir())?.observation;
    let context = build_with_observation(snapshot, observation.as_deref())?;
    let committed = observation.and_then(|observation| observation.committed.clone());
    Ok((context, committed))
}

/// One `Repo`-group capture at a directory: the projected `ctx` values and the
/// observation behind them.
pub(crate) struct RepositoryDiscovery {
    pub(crate) values: serde_json::Map<String, serde_json::Value>,
    pub(crate) observation: Option<Arc<RepositoryObservation>>,
}

/// Discovers the repository containing `dir` through the same capture
/// `ctx.repo*` uses, so a request can share the observation.
pub(crate) fn discover_repository(dir: &Path) -> Result<RepositoryDiscovery, ContextBuildError> {
    // A relative directory is rejected by validation with a typed error;
    // discovering from it would read the process directory.
    if !dir.is_absolute() {
        return Ok(RepositoryDiscovery { values: serde_json::Map::new(), observation: None });
    }
    let (values, diagnostics, _, _, observations) =
        capture_runtime_context_for_groups(dir, &[ContextGroup::Repo], &std::collections::HashMap::new());
    // With only the `Repo` group requested, the sole `git` diagnostic the
    // capture raises is the discovery failure itself.
    if let Some(detail) = diagnostics.into_iter().find_map(|diagnostic| match diagnostic {
        ContextMergeDiagnostic::PartialRuntimeCapture { area: "git", detail } => Some(detail),
        _ => None,
    }) {
        return Err(ContextBuildError::Discovery { dir: dir.to_path_buf(), detail });
    }
    Ok(RepositoryDiscovery { values, observation: observations.repository().cloned() })
}

/// The snapshot the caller supplied: on the options, else from the host's
/// configuration capture on their `ctx` ([`ComposeContext::with_config_source`]).
///
/// [`ComposeContext::with_config_source`]: super::runtime::ComposeContext::with_config_source
fn supplied_config(
    options: &ComposeOptions,
    context: &FileResolutionContext,
) -> Result<Option<Arc<crate::config::load::ConfigSnapshot>>, ContextBuildError> {
    if let Some(config) = options.config_snapshot() {
        return Ok(Some(Arc::clone(config)));
    }
    options
        .context()
        .config_source()
        .and_then(|source| source.config_snapshot_for(context))
        .transpose()
        .map_err(|source| ContextBuildError::Config { dir: context.request_cwd().to_path_buf(), source })
}

/// The passive configuration load of a request whose caller supplied no
/// snapshot.
fn load_passively(
    context: &FileResolutionContext,
    committed: Option<&crate::config::load::CommittedTreeCapture>,
) -> Result<Arc<crate::config::load::ConfigSnapshot>, ContextBuildError> {
    crate::config::load::ConfigSnapshot::load_captured(context, committed, crate::config::load::ConfigLoadMode::Passive)
        .map(Arc::new)
        .map_err(|source| ContextBuildError::Config { dir: context.request_cwd().to_path_buf(), source })
}

/// [`build_resolution_context`] over a repository observation already made
/// for the request directory.
pub(crate) fn build_with_observation(
    snapshot: &RequestSnapshot,
    observation: Option<&RepositoryObservation>,
) -> Result<FileResolutionContext, ContextBuildError> {
    build_resolution_context_with_catalog(
        snapshot,
        observation.and_then(RepositoryObservation::scope_catalog),
    )
}

/// [`build_resolution_context`] for a caller that has already discovered the
/// repository containing the request directory.
///
/// For an embedder that keeps its own repository observations (Claudine's
/// invocation cache), so one request does not discover the same repository
/// twice. Every step except discovery is the builder's. `catalog` must be the
/// scope catalog of the repository that contains the snapshot's request
/// directory, or `None` when there is no repository; a catalog for another
/// repository fails validation like any other misplaced anchor.
///
/// ## Errors
///
/// [`ContextBuildError::Invalid`] when the built context fails
/// [`validate`](FileResolutionContext::validate). No discovery runs, so this
/// never returns [`ContextBuildError::Discovery`].
pub fn build_resolution_context_with_catalog(
    snapshot: &RequestSnapshot,
    catalog: Option<RepositoryScopeCatalog>,
) -> Result<FileResolutionContext, ContextBuildError> {
    let dir = snapshot.request_dir();
    let mut context = FileResolutionContext::from_snapshot(
        dir,
        snapshot.home().map(Path::to_path_buf),
        snapshot.env().clone(),
    );
    if let Some(catalog) = catalog {
        context = context.with_repository_scope_catalog(catalog);
    }
    for (path, position, tier) in snapshot.magic_roots() {
        context = context.add_magic_path_with_tier(path.clone(), *position, *tier);
    }
    if let Some((reference, resolved)) = snapshot.opening_reference() {
        context = context.for_source_reference(reference, resolved);
    }
    context
        .validate()
        .map_err(|source| ContextBuildError::Invalid { dir: dir.to_path_buf(), source })?;
    tracing::debug!(
        request_dir = %dir.display(),
        base_dir = %context.base_dir().display(),
        base_dir_origin = ?context.base_dir_origin(),
        "built file-resolution context",
    );
    Ok(context)
}

/// The context a source in another Git repository resolves its own
/// references against: `own` is a context built at the source's directory,
/// which found that repository, and `launch` is the context of the request
/// or invocation that reached the source.
///
/// The source is derived from `own` as a trusted external source (keeping
/// the `~` or `{{VAR}}` anchor of `opening`), so `./`, bare, `&`, and `^`
/// references, the package anchors, and the schema roots are its own
/// repository's, while `@` keeps searching `launch`'s launch scope. The one
/// rule `md`'s directly opened documents and every source a request admits
/// across a file boundary share.
pub fn derive_in_own_repository(
    own: &FileResolutionContext,
    launch: &FileResolutionContext,
    resolved: &Path,
    opening: Option<&FileReference>,
) -> FileResolutionContext {
    let source = match opening {
        Some(reference) => own.for_trusted_external_source_reference(reference, resolved),
        None => own.for_trusted_external_source(resolved),
    };
    source.with_launch_scope(launch.launch_scope().clone())
}

/// The working-tree root of the Git checkout nearest to `dir`: the closest
/// ancestor holding a `.git` directory or a `.git` file (a submodule or a
/// `git init --separate-git-dir` checkout). `None` outside every checkout and
/// when discovery fails, so a caller keeps the derivation it would make
/// without one.
///
/// This, never containment under another checkout's root, decides which
/// repository a source belongs to: a checkout nested inside another is its
/// own repository.
pub fn nearest_repository_root(dir: &Path) -> Option<PathBuf> {
    // A relative directory would be searched from the process directory.
    if !dir.is_absolute() {
        return None;
    }
    biscuit_file::find_git_root(dir)
        .inspect_err(|error| tracing::debug!(%error, dir = %dir.display(), "nearest repository root not found"))
        .ok()
        .flatten()
}

/// Whether the checkout rooted at `nearest` (from [`nearest_repository_root`])
/// is a repository other than `root`, the one a request or launch context
/// names; `true` when that context names none. Roots spelled differently
/// (macOS `/var` versus `/private/var`) are compared canonically.
pub fn names_another_repository(nearest: &Path, root: Option<&Path>) -> bool {
    let Some(root) = root else {
        return true;
    };
    if nearest == root {
        return false;
    }
    match (biscuit_file::canonicalize_simplified(nearest), biscuit_file::canonicalize_simplified(root)) {
        (Ok(nearest), Ok(root)) => nearest != root,
        _ => true,
    }
}

/// The Git repositories a request found its admitted sources in: the nearest
/// checkout root of each source directory, looked up once per directory, and
/// one repository observation per checkout root, for the whole request.
///
/// A request captures its own repository when it is prepared. A source it
/// later admits across a file boundary (a `::file`, `prologue:`, or
/// `epilogue:` child, a `::toc-linking` or fragment target, the root of a
/// request whose context names another repository) can lie in a different
/// checkout, including one nested inside the request's own tree; that is the
/// one place a request discovers another repository, and every consumer of
/// the source then reads the context decided there
/// ([`SourceDerivation::OwnRepository`]). Shared by every derivation of the
/// request. Observations are keyed by exact checkout root, never by
/// containment, so the order sources are visited in cannot change which
/// repository a source gets.
///
/// [`SourceDerivation::OwnRepository`]: super::options::SourceDerivation::OwnRepository
#[derive(Debug, Default)]
pub(crate) struct SourceRepositories {
    known: std::sync::Mutex<KnownRepositories>,
}

#[derive(Debug, Default)]
struct KnownRepositories {
    /// Source directory to its nearest checkout root (`None`: outside Git).
    nearest: std::collections::HashMap<PathBuf, Option<PathBuf>>,
    /// Checkout root to its observation (`None`: discovery failed).
    observed: std::collections::HashMap<PathBuf, Option<Arc<RepositoryObservation>>>,
}

impl SourceRepositories {
    /// The memo for a request over `context`: the request directory's
    /// checkout (the one `context` names, or none) is known, so it is not
    /// looked up again.
    pub(crate) fn for_request(context: &FileResolutionContext) -> Self {
        let mut known = KnownRepositories::default();
        known
            .nearest
            .insert(context.request_cwd().to_path_buf(), context.repository_root().map(Path::to_path_buf));
        Self { known: std::sync::Mutex::new(known) }
    }

    /// [`SourceDerivation::OwnRepository`] for the source at `path` when its
    /// nearest checkout is a Git repository `context` does not name, holding
    /// its [`derive_in_own_repository`] context; `None` when it is in no
    /// repository, in `context`'s own, or its context cannot be built, so the
    /// caller keeps its ordinary derivation.
    ///
    /// [`SourceDerivation::OwnRepository`]: super::options::SourceDerivation::OwnRepository
    pub(crate) fn own_repository_derivation(
        &self,
        context: &FileResolutionContext,
        path: &Path,
        opening: Option<(&FileReference, &Path)>,
    ) -> Option<super::options::SourceDerivation> {
        let dir = path.parent()?;
        let observation = self.other_repository(dir, context.repository_root())?;
        let mut snapshot = RequestSnapshot::new(dir)
            .with_home(context.home_dir().map(Path::to_path_buf))
            .with_env(context.env().clone());
        for registration in context.magic_path_registrations() {
            snapshot = snapshot.with_magic_root_tier(
                registration.path().to_path_buf(),
                registration.position(),
                registration.tier(),
            );
        }
        let own = build_with_observation(&snapshot, Some(&observation))
            .inspect_err(|error| tracing::debug!(%error, "source repository context not built"))
            .ok()?;
        let resolved = opening.map_or(path, |(_, resolved)| resolved);
        let derived = derive_in_own_repository(&own, context, resolved, opening.map(|(reference, _)| reference));
        derived
            .validate()
            .is_ok()
            .then(|| super::options::SourceDerivation::OwnRepository(Arc::new(derived)))
    }

    /// The observation of `dir`'s nearest checkout when that checkout is not
    /// `own`, discovering it on the first source there. A discovery failure
    /// counts as no repository: the source keeps the request's derivation.
    fn other_repository(&self, dir: &Path, own: Option<&Path>) -> Option<Arc<RepositoryObservation>> {
        let mut known = self.known.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let nearest = known.nearest.entry(dir.to_path_buf()).or_insert_with(|| nearest_repository_root(dir)).clone()?;
        if !names_another_repository(&nearest, own) {
            return None;
        }
        known
            .observed
            .entry(nearest)
            .or_insert_with_key(|root| {
                discover_repository(root)
                    .inspect_err(|error| tracing::debug!(%error, "source repository discovery failed"))
                    .ok()
                    .and_then(|discovery| discovery.observation)
                    .filter(|observation| observation.root.is_some())
            })
            .clone()
    }
}

/// Compose options paired with the request's required file-resolution
/// context.
///
/// Every compose entry point takes one: [`Markdown::compose_with`],
/// pre-flight, link resolution, and shell execution. Building one fixes the
/// request's repository observation, so every phase run with it, and every
/// transcluded child, answers `current.repo*` and `&`/`^` from the same
/// repository.
///
/// `ctx.env`, `env.*`, and `ctx.agent`/`ctx.model` read the context's
/// environment, so an expression and a `{{VAR}}` file reference in one
/// request see the same value.
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::Markdown;
/// use darkmatter::markdown::compose::{ComposeOptions, ComposeRequest, RequestSnapshot};
///
/// let snapshot = RequestSnapshot::new(std::env::temp_dir());
/// let request = ComposeRequest::prepare(ComposeOptions::new(), &snapshot).unwrap();
/// let md: Markdown = "# Title\n".into();
/// let (composed, _report) = md.compose_with(&request).unwrap();
/// assert!(composed.content().contains("# Title"));
/// ```
///
/// [`Markdown::compose_with`]: crate::markdown::Markdown::compose_with
///
/// A request dereferences to its [`ComposeOptions`], so every stage reads
/// settings through it while the type guarantees a context is present.
#[derive(Clone)]
pub struct ComposeRequest {
    options: ComposeOptions,
    context: FileResolutionContext,
    /// The request's one configuration snapshot; `options` mirrors it so
    /// every stage reads it through `Deref` too.
    config: Arc<crate::config::load::ConfigSnapshot>,
    /// The repositories admitted sources were found in, shared by every
    /// derivation of this request.
    repositories: Arc<SourceRepositories>,
}

impl std::ops::Deref for ComposeRequest {
    type Target = ComposeOptions;

    fn deref(&self) -> &ComposeOptions {
        &self.options
    }
}

impl std::ops::DerefMut for ComposeRequest {
    fn deref_mut(&mut self) -> &mut ComposeOptions {
        &mut self.options
    }
}

impl std::fmt::Debug for ComposeRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ComposeRequest")
            .field("options", &self.options)
            .field("context", &self.context)
            .field("config", &self.config)
            .field("repositories", &self.repositories)
            .finish()
    }
}

impl ComposeRequest {
    /// Prepares `options` for the request `snapshot` describes, building its
    /// context through [`build_resolution_context`].
    ///
    /// A Darkmatter-owned context that has captured nothing anchored to a
    /// directory is re-anchored on the request directory, so `ctx.cwd` and
    /// later captures describe the same directory file references start
    /// from. The repository discovery the builder makes becomes the request's
    /// observation when the request has none yet.
    ///
    /// Without a supplied snapshot ([`ComposeOptions::with_config_snapshot`],
    /// or the host capture on the options' `ctx`), the configuration is loaded
    /// passively once the context exists, reading the repository's committed
    /// policy from the tree that observation pinned.
    ///
    /// ## Errors
    ///
    /// Returns the builder's [`ContextBuildError`], or
    /// [`ContextBuildError::Config`] when the configuration cannot be loaded.
    pub fn prepare(
        mut options: ComposeOptions,
        snapshot: &RequestSnapshot,
    ) -> Result<Self, ContextBuildError> {
        let dir = snapshot.request_dir();
        options.anchor_unanchored_context(dir);
        let observation = match options.request_repository_for(dir) {
            Some(observation) => Some(observation),
            None => {
                let discovery = discover_repository(dir)?;
                let observation = discovery.observation.clone();
                options.adopt_request_repository(dir, discovery);
                observation
            }
        };
        options.establish_request_repository();
        let context = build_with_observation(snapshot, observation.as_deref())?;
        let config = match supplied_config(&options, &context)? {
            Some(config) => config,
            None => load_passively(&context, observation.as_ref().and_then(|observation| observation.committed.as_ref()))?,
        };
        Ok(Self::assemble(options, context, config))
    }

    /// Pairs `options` with a context the caller built or derived from one
    /// that was built.
    ///
    /// For callers that hold a context already: a source derivation, a source
    /// in another repository, or an embedder's own built context. A context
    /// that names a repository needs the caller's configuration snapshot
    /// ([`ComposeOptions::with_config_snapshot`], or the host capture its
    /// `ctx` carries, [`ComposeContext::with_config_source`]); one that names
    /// none loads the user configuration passively when no snapshot was
    /// supplied.
    ///
    /// [`ComposeContext::with_config_source`]: super::runtime::ComposeContext::with_config_source
    ///
    /// ## Errors
    ///
    /// - [`ContextBuildError::Invalid`] when `context` fails
    ///   [`validate`](FileResolutionContext::validate);
    /// - [`ContextBuildError::ConfigSnapshotRequired`] when `context` names a
    ///   repository and `options` holds no configuration snapshot;
    /// - [`ContextBuildError::Config`] when the passive load fails.
    pub fn with_context(
        options: ComposeOptions,
        context: FileResolutionContext,
    ) -> Result<Self, ContextBuildError> {
        context.validate().map_err(|source| ContextBuildError::Invalid {
            dir: context.request_cwd().to_path_buf(),
            source,
        })?;
        let config = match (supplied_config(&options, &context)?, context.repository_root()) {
            (Some(config), _) => config,
            (None, Some(repository)) => {
                return Err(ContextBuildError::ConfigSnapshotRequired {
                    dir: context.request_cwd().to_path_buf(),
                    repository: repository.to_path_buf(),
                });
            }
            (None, None) => load_passively(&context, None)?,
        };
        options.establish_request_repository();
        Ok(Self::assemble(options, context, config))
    }

    /// The local-only expression context `options`' source evaluates in when
    /// it resolves through `context`, without building a request.
    ///
    /// For hosts that evaluate document-authored expressions outside the
    /// compose pipeline (Claudine's lifecycle, loop, and sequence
    /// expressions), as [`Self::local_expression_resolution_context`] does for
    /// a request. A local-only context fetches nothing remote, runs no shell
    /// command, and evaluates no schema slot, so it consults no configuration
    /// and needs no snapshot, whichever repository `context` names.
    ///
    /// ## Errors
    ///
    /// [`ContextBuildError::Invalid`] when `context` fails
    /// [`validate`](FileResolutionContext::validate).
    pub fn local_expression_context(
        mut options: ComposeOptions,
        context: &FileResolutionContext,
    ) -> Result<crate::markdown::compose::expression::ResolutionContext, ContextBuildError> {
        context.validate().map_err(|source| ContextBuildError::Invalid {
            dir: context.request_cwd().to_path_buf(),
            source,
        })?;
        options.establish_request_repository();
        options.align_context_environment(context.env());
        options.admit_source_in(context, &SourceRepositories::for_request(context));
        Ok(options.local_expression_resolution_context_in(context))
    }

    fn assemble(
        options: ComposeOptions,
        context: FileResolutionContext,
        config: Arc<crate::config::load::ConfigSnapshot>,
    ) -> Self {
        let repositories = Arc::new(SourceRepositories::for_request(&context));
        Self::assemble_with(options, context, config, repositories)
    }

    fn assemble_with(
        mut options: ComposeOptions,
        context: FileResolutionContext,
        config: Arc<crate::config::load::ConfigSnapshot>,
        repositories: Arc<SourceRepositories>,
    ) -> Self {
        options.align_context_environment(context.env());
        options.admit_source_in(&context, &repositories);
        options.config_snapshot = Some(Arc::clone(&config));
        options.align_remote_network();
        Self { options, context, config, repositories }
    }

    /// This request with its options changed by `change`, keeping its
    /// context.
    ///
    /// For per-run settings decided after preparation, such as the approval
    /// set pre-flight computes.
    #[must_use]
    pub fn map_options(self, change: impl FnOnce(ComposeOptions) -> ComposeOptions) -> Self {
        Self::assemble_with(change(self.options), self.context, self.config, self.repositories)
    }

    /// The configuration snapshot this request was captured with, shared by
    /// every derivation of it (transcluded children, inline passes,
    /// `as_markdown`).
    pub fn config_snapshot(&self) -> &Arc<crate::config::load::ConfigSnapshot> {
        &self.config
    }

    /// The options this request composes under.
    pub fn options(&self) -> &ComposeOptions {
        &self.options
    }

    /// The request's file-resolution context.
    ///
    /// Named apart from [`ComposeOptions::context`], the captured `ctx.*`
    /// values, which a request also exposes through `Deref`.
    pub fn resolution_context(&self) -> &FileResolutionContext {
        &self.context
    }

    /// The local-only expression context for the request's source, resolving
    /// file references through the request's context.
    ///
    /// For hosts that evaluate document-authored expressions outside the
    /// compose pipeline (Claudine's lifecycle and sequence expressions). It
    /// keeps the request's source derivation and caller provenance.
    pub fn local_expression_resolution_context(
        &self,
    ) -> crate::markdown::compose::expression::ResolutionContext {
        self.options.local_expression_resolution_context_in(&self.context)
    }

    /// The files and URLs this request's data runtime recorded so far, in
    /// lexical order; empty when no runtime is attached
    /// ([`ComposeOptions::with_shared_data_runtime`]).
    pub fn read_set(&self) -> Vec<String> {
        self.options.data_runtime_handle().map(crate::markdown::compose::DataRuntime::read_set).unwrap_or_default()
    }

    /// This request with its options changed by `change`, keeping its context
    /// and the options' own `ctx.*` environment.
    ///
    /// For the pipeline's own derivations (a child source, an inline pass);
    /// [`map_options`](Self::map_options) is the caller-facing form.
    #[must_use]
    pub(crate) fn derive(mut self, change: impl FnOnce(ComposeOptions) -> ComposeOptions) -> Self {
        self.options = change(self.options);
        self.options.config_snapshot = Some(Arc::clone(&self.config));
        self
    }

    /// This request rooted at the file `path`, admitted as the request's own
    /// source is when it is prepared ([`ComposeOptions::admit_source_in`]): a
    /// relative path is taken from the request directory, and the file's
    /// nearest checkout decides its repository. The request's own source,
    /// however it is spelled, keeps the derivation it was admitted with
    /// (and the reference that opened it).
    #[must_use]
    pub(crate) fn with_source_file(self, path: impl Into<PathBuf>) -> Self {
        let mut path = path.into();
        if path.is_relative() {
            path = self.context.request_cwd().join(path);
        }
        if matches!(&self.options.source, super::options::ComposeSource::File(current) if *current == path) {
            return self;
        }
        let context = self.context.clone();
        let repositories = Arc::clone(&self.repositories);
        self.derive(|options| {
            let mut options = options.with_source_file(path);
            options.admit_source_in(&context, &repositories);
            options
        })
    }

    /// This request composing `url`; see [`ComposeOptions::with_source_url`].
    #[must_use]
    pub(crate) fn with_source_url(self, url: url::Url) -> Self {
        self.derive(|options| options.with_source_url(url))
    }

    /// This request rooted at `source`, a root document's own source: a file
    /// through [`Self::with_source_file`], a URL as is; an unknown source
    /// leaves the request unchanged.
    #[must_use]
    pub(crate) fn with_root_source(self, source: &super::options::ComposeSource) -> Self {
        match source {
            super::options::ComposeSource::File(path) => self.with_source_file(path),
            super::options::ComposeSource::Url(url) => self.with_source_url(url.clone()),
            super::options::ComposeSource::Unknown => self,
        }
    }

    /// This request as the root pass over `document` uses it: a request
    /// with no source of its own takes the source `document` was loaded
    /// from ([`Self::with_root_source`]), so a caller that supplies the
    /// path only on the [`Markdown`](crate::markdown::Markdown) gets the
    /// same checkout as one that puts it on the request.
    pub(crate) fn rooted_at(&self, document: &crate::markdown::Markdown) -> std::borrow::Cow<'_, Self> {
        match (&self.options.source, document.source()) {
            (super::options::ComposeSource::Unknown, Some(source)) if *source != super::options::ComposeSource::Unknown => {
                std::borrow::Cow::Owned(self.clone().with_root_source(source))
            }
            _ => std::borrow::Cow::Borrowed(self),
        }
    }

    /// Sets a child source that has already passed file-reference resolution;
    /// see [`ComposeOptions::with_accepted_source_file_in`].
    #[must_use]
    pub(crate) fn with_accepted_source_file(
        self,
        path: impl Into<PathBuf>,
        opening: Option<super::options::SourceOpening>,
    ) -> Self {
        let context = self.context.clone();
        let repositories = Arc::clone(&self.repositories);
        self.derive(|options| options.with_accepted_source_file_in(path, opening, &context, &repositories))
    }

    /// `document` with a relative file source made absolute at the request
    /// directory, as [`Self::with_source_file`] admits it; borrowed when its
    /// source is already absolute or not a file. Schema preparation reads
    /// the document's own path (its `./` references and its `$schema`
    /// position), so it must name the same file the request admitted.
    pub(crate) fn root_document<'d>(
        &self,
        document: &'d crate::markdown::Markdown,
    ) -> std::borrow::Cow<'d, crate::markdown::Markdown> {
        match document.source() {
            Some(super::options::ComposeSource::File(path)) if path.is_relative() => std::borrow::Cow::Owned(
                document
                    .clone()
                    .with_source(super::options::ComposeSource::File(self.context.request_cwd().join(path))),
            ),
            _ => std::borrow::Cow::Borrowed(document),
        }
    }

    /// `document` carrying the file identity this request admits for it: its
    /// own file source as [`Self::root_document`] admits it, or, when it
    /// carries no source, the file this request was prepared with.
    ///
    /// This is the one rule a root entry point reads before deciding file
    /// identity (schema roots and trigger eligibility, the reference graph's
    /// root and origins, local link and inclusion targets), so a path given
    /// on the request, on the `Markdown`, or on both names one file. A
    /// document with its own source keeps it (each consumer keeps its
    /// precedence when the two name different files). Only the path is
    /// attached: the buffer is never re-read, so an unsaved editor buffer
    /// stays the text that is judged.
    ///
    /// Only for a request's root document that is the file's text: content
    /// composed against the root source without being that file
    /// (`as_markdown`, or text a host generated and named with
    /// [`Markdown::with_context_identity`](crate::markdown::Markdown::with_context_identity))
    /// keeps having no source of its own.
    pub(crate) fn admitted_document<'d>(
        &self,
        document: &'d crate::markdown::Markdown,
    ) -> std::borrow::Cow<'d, crate::markdown::Markdown> {
        match (document.source(), &self.options.source) {
            (None | Some(super::options::ComposeSource::Unknown), super::options::ComposeSource::File(path))
                if document.generated_identity().is_none() =>
            {
                std::borrow::Cow::Owned(document.clone().with_source(super::options::ComposeSource::File(
                    self.context.request_cwd().join(path),
                )))
            }
            _ => self.root_document(document),
        }
    }

    /// This request as the compose pass of `document` will see it: rooted at
    /// its source ([`Self::rooted_at`]), after
    /// [`ComposeOptions::extend_context_for`].
    ///
    /// Pre-flight discovery evaluates a document before that pass runs, so it
    /// must read the same context or it would observe groups the real pass has.
    pub(crate) fn extended_for(
        &self,
        document: &crate::markdown::Markdown,
    ) -> std::borrow::Cow<'_, Self> {
        let rooted = self.rooted_at(document);
        if !rooted.options.needs_extension_for(document) {
            return rooted;
        }
        let mut extended = rooted.into_owned();
        extended.options.extend_context_for(document);
        std::borrow::Cow::Owned(extended)
    }

    /// The current source's context, derived from the request's context.
    pub(crate) fn source_file_resolution_context(&self) -> FileResolutionContext {
        self.options.source_file_resolution_context_in(&self.context)
    }

    /// The transclusion view of this request.
    pub(crate) fn transclusion_options(&self) -> super::options::TransclusionOptions {
        self.options.transclusion_options_in(&self.context)
    }

    /// The expression context body interpolation evaluates in.
    pub(crate) fn expression_resolution_context(
        &self,
        remote_fetch: &crate::markdown::compose::remote_fetch::RemoteFetchRuntime,
    ) -> crate::markdown::compose::expression::ResolutionContext {
        self.options.expression_resolution_context_in(&self.context, remote_fetch)
    }

    /// The expression context frontmatter interpolation evaluates in.
    pub(crate) fn frontmatter_resolution_context(
        &self,
    ) -> crate::markdown::compose::expression::ResolutionContext {
        self.options.frontmatter_resolution_context_in(&self.context)
    }

    /// The compose-cache fingerprint of the options and the context.
    pub(crate) fn compose_cache_fingerprint(&self) -> u64 {
        self.options.compose_cache_fingerprint(&self.context)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_snapshot_reads_nothing_from_the_process() {
        let snapshot = RequestSnapshot::new("/request");

        assert_eq!(snapshot.request_dir(), Path::new("/request"));
        assert_eq!(snapshot.home(), None);
        assert!(snapshot.env().is_empty());
        assert!(snapshot.magic_roots().is_empty());
        assert!(snapshot.opening_reference().is_none());
    }

    #[test]
    fn at_request_dir_keeps_home_environment_and_roots_but_not_the_opening() {
        let env = HashMap::from([("NOTES".to_string(), "/notes".to_string())]);
        let snapshot = RequestSnapshot::new("/request")
            .with_home(Some(PathBuf::from("/home/me")))
            .with_env(env.clone())
            .with_magic_root("/roots/a", PathPosition::Start)
            .with_magic_root_tier("/roots/b", PathPosition::End, MagicPathTier::User)
            .with_opening_reference(
                FileReference::new("~/doc.md").unwrap(),
                PathBuf::from("/home/me/doc.md"),
            );

        let moved = snapshot.at_request_dir("/elsewhere");

        assert_eq!(moved.request_dir(), Path::new("/elsewhere"));
        assert_eq!(moved.home(), Some(Path::new("/home/me")));
        assert_eq!(moved.env(), &env);
        assert_eq!(
            moved.magic_roots(),
            &[
                (PathBuf::from("/roots/a"), PathPosition::Start, MagicPathTier::Inferred),
                (PathBuf::from("/roots/b"), PathPosition::End, MagicPathTier::User),
            ]
        );
        assert!(moved.opening_reference().is_none());
        assert!(snapshot.opening_reference().is_some(), "the original keeps its opening");
    }

    /// A successful build emits exactly one `debug` event naming the request
    /// directory and the `base_dir` origin.
    #[test]
    #[tracing_test::traced_test]
    fn a_successful_build_emits_one_debug_event() {
        let dir = tempfile::tempdir().unwrap();
        let request_dir = dir.path().canonicalize().unwrap();

        build_resolution_context(&RequestSnapshot::new(&request_dir)).unwrap();

        let shown = request_dir.display().to_string();
        logs_assert(|lines: &[&str]| {
            let events: Vec<&&str> =
                lines.iter().filter(|line| line.contains("built file-resolution context")).collect();
            match events.as_slice() {
                [event]
                    if event.contains("DEBUG")
                        && event.contains(&format!("request_dir={shown}"))
                        && event.contains("base_dir_origin=Fallback") =>
                {
                    Ok(())
                }
                other => Err(format!("expected one debug event naming {shown}, got {other:?}")),
            }
        });
    }

    /// One discovery serves the context, the repository observation, and the
    /// committed policy, and every derivation keeps the one snapshot.
    #[test]
    fn prepare_discovers_once_and_derivations_share_the_snapshot() {
        use std::sync::atomic::Ordering;

        use super::super::capture::GIT_DISCOVERY_COUNT;

        let temp = tempfile::tempdir().unwrap();
        let root = biscuit_file::canonicalize_simplified(temp.path()).unwrap();
        gix::init(&root).unwrap();
        GIT_DISCOVERY_COUNT.store(0, Ordering::Relaxed);

        let request = ComposeRequest::prepare(ComposeOptions::new(), &RequestSnapshot::new(&root)).unwrap();

        assert_eq!(GIT_DISCOVERY_COUNT.load(Ordering::Relaxed), 1);
        assert_eq!(request.config_snapshot().repository().map(|repository| repository.root.clone()), Some(root.clone()));
        let child = request.clone().with_source_file(root.join("child.md"));
        let accepted = request.clone().with_accepted_source_file(root.join("other.md"), None);
        let derived = request.clone().derive(|_| ComposeOptions::new());
        for view in [&child, &accepted, &derived] {
            assert!(Arc::ptr_eq(request.config_snapshot(), view.config_snapshot()));
            assert!(Arc::ptr_eq(request.config_snapshot(), view.options().config_snapshot().unwrap()));
        }
        assert_eq!(GIT_DISCOVERY_COUNT.load(Ordering::Relaxed), 1, "no derivation rediscovers");
    }

    /// A source admitted from another checkout gets a context in its own
    /// repository with the request's launch `@` scope and configuration, and
    /// the request discovers each other repository once; a directory outside
    /// every repository, or in the launch checkout, discovers none.
    #[test]
    fn admitted_sources_discover_another_repository_once_per_request() {
        use std::sync::atomic::Ordering;

        use super::super::capture::GIT_DISCOVERY_COUNT;

        let temp = tempfile::tempdir().unwrap();
        let base = biscuit_file::canonicalize_simplified(temp.path()).unwrap();
        let (app, notes, loose) = (base.join("app"), base.join("notes"), base.join("loose"));
        for dir in [&app, &notes.join("deep"), &loose] {
            std::fs::create_dir_all(dir).unwrap();
        }
        gix::init(&app).unwrap();
        gix::init(&notes).unwrap();
        let snapshot = RequestSnapshot::new(&app).with_home(Some(base.clone()));
        let request = ComposeRequest::prepare(ComposeOptions::new(), &snapshot).unwrap();
        GIT_DISCOVERY_COUNT.store(0, Ordering::Relaxed);

        for source in [notes.join("a.md"), notes.join("deep/b.md"), notes.join("a.md")] {
            let accepted = request.clone().with_accepted_source_file(&source, None);
            let context = accepted.source_file_resolution_context();
            assert_eq!(context.repository_root(), Some(notes.as_path()), "{}", source.display());
            assert_eq!(context.launch_scope(), request.resolution_context().launch_scope());
            assert!(Arc::ptr_eq(request.config_snapshot(), accepted.config_snapshot()));
        }
        assert_eq!(GIT_DISCOVERY_COUNT.load(Ordering::Relaxed), 1, "one discovery for the notes checkout");

        for _ in 0..2 {
            let accepted = request.clone().with_accepted_source_file(loose.join("c.md"), None);
            assert_eq!(accepted.source_file_resolution_context().repository_root(), None);
        }
        let resident = request.clone().with_accepted_source_file(app.join("d.md"), None);
        assert_eq!(resident.source_file_resolution_context().repository_root(), Some(app.as_path()));
        assert_eq!(GIT_DISCOVERY_COUNT.load(Ordering::Relaxed), 1, "none for the folder outside Git or the launch checkout");
    }

    /// Git membership, not containment, names a source's repository: a
    /// checkout nested in the launch checkout, or in another checkout the
    /// request has already visited, is its own repository whichever source
    /// the request admits first. A `.git` file (a submodule's layout) marks a
    /// checkout as a `.git` directory does.
    #[test]
    fn nested_checkouts_are_their_own_repository_in_either_order() {
        let temp = tempfile::tempdir().unwrap();
        let base = biscuit_file::canonicalize_simplified(temp.path()).unwrap();
        let (app, notes) = (base.join("app"), base.join("notes"));
        let (app_vendor, notes_vendor) = (app.join("vendor"), notes.join("vendor"));
        for dir in [&app, &notes, &app_vendor, &notes_vendor] {
            std::fs::create_dir_all(dir).unwrap();
        }
        gix::init(&app).unwrap();
        gix::init(&notes).unwrap();
        gix::init(&notes_vendor).unwrap();
        // `git init --separate-git-dir`: the checkout holds a `.git` file.
        let separate = base.join("separate.git");
        gix::init_bare(&separate).unwrap();
        std::fs::write(app_vendor.join(".git"), format!("gitdir: {}\n", separate.display())).unwrap();
        let mut config = std::fs::read_to_string(separate.join("config")).unwrap();
        // No `worktree` key: a Windows path's backslashes are escapes in Git
        // configuration, and discovery takes the checkout from the `.git` file.
        config = config.replace("bare = true", "bare = false");
        std::fs::write(separate.join("config"), config).unwrap();

        let snapshot = RequestSnapshot::new(&app).with_home(Some(base.clone()));
        let roots = |sources: &[PathBuf]| -> Vec<Option<PathBuf>> {
            let request = ComposeRequest::prepare(ComposeOptions::new(), &snapshot).unwrap();
            sources
                .iter()
                .map(|source| {
                    let accepted = request.clone().with_accepted_source_file(source, None);
                    accepted.source_file_resolution_context().repository_root().map(Path::to_path_buf)
                })
                .collect()
        };
        let (seed, nested, component) = (notes.join("seed.md"), notes_vendor.join("review.md"), app_vendor.join("review.md"));
        assert_eq!(roots(&[component]), [Some(app_vendor)], "nested in the launch checkout");
        assert_eq!(
            roots(&[seed.clone(), nested.clone()]),
            [Some(notes.clone()), Some(notes_vendor.clone())],
            "seed first"
        );
        assert_eq!(roots(&[nested, seed]), [Some(notes_vendor), Some(notes)], "nested first");
        assert_eq!(roots(&[app.join("d.md")]), [Some(app)], "the launch checkout's own source");
    }

    #[test]
    #[tracing_test::traced_test]
    fn a_failed_build_emits_no_debug_event() {
        build_resolution_context(&RequestSnapshot::new("relative/dir")).unwrap_err();

        assert!(!logs_contain("built file-resolution context"));
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    use super::*;

    /// A request for unit tests that preserves the request directory the
    /// pre-`ComposeRequest` pipeline chose: a file source's directory, else
    /// the context's anchor, else the process directory. Home comes from the
    /// process and the environment from the options' own context (empty
    /// unless the test put values there), so no test sees the process
    /// environment by accident.
    pub(crate) fn request(options: ComposeOptions) -> ComposeRequest {
        let dir = legacy_request_dir(&options);
        let snapshot = RequestSnapshot::new(dir)
            .with_home(biscuit_file::home_dir())
            .with_env(options.context().env().clone());
        ComposeRequest::prepare(options, &snapshot).expect("test request")
    }

    /// A request that resolves through `context`, as the options' old
    /// context field did. A context naming a repository gets the user-only
    /// configuration of its home (see [`user_config`]).
    ///
    /// Options whose context is anchored nowhere ([`ComposeOptions::new`])
    /// are anchored on `context`'s request directory first, as
    /// [`ComposeRequest::prepare`] anchors them: `with_context` discovers the
    /// request's repository at that anchor, and an empty one is the process
    /// directory, inside this monorepo (~70 ms per request on Linux, seconds
    /// on Windows under load).
    pub(crate) fn request_in(mut options: ComposeOptions, context: FileResolutionContext) -> ComposeRequest {
        if !options.context().anchor().is_absolute() {
            options.anchor_unanchored_context(context.request_cwd());
        }
        let options = match (options.config_snapshot(), context.repository_root()) {
            (None, Some(_)) => options.with_config_snapshot(user_config(&context)),
            _ => options,
        };
        ComposeRequest::with_context(options, context).expect("test request context")
    }

    /// The configuration of `context`'s home and no repository: what a test
    /// whose repository is a synthetic catalog (no Git) supplies, since such a
    /// repository has no committed policy to read.
    pub(crate) fn user_config(context: &FileResolutionContext) -> Arc<crate::config::load::ConfigSnapshot> {
        let snapshot = RequestSnapshot::new(context.request_cwd())
            .with_home(context.home_dir().map(Path::to_path_buf))
            .with_env(context.env().clone());
        Arc::new(
            crate::config::load::ConfigSnapshot::load_user(&snapshot, crate::config::load::ConfigLoadMode::Passive)
                .expect("test configuration"),
        )
    }

    /// A file source's directory, else the context's anchor, else a
    /// per-process temp directory outside any Git repository. The fallback is
    /// deliberately not the process directory: under the test runner that is
    /// inside this monorepo, and preparing a request there runs repository
    /// discovery over all of it (~0.4s per call, more under load). Relative
    /// file sources still join onto the process directory.
    pub(crate) fn legacy_request_dir(options: &ComposeOptions) -> PathBuf {
        static SCRATCH: std::sync::OnceLock<tempfile::TempDir> = std::sync::OnceLock::new();
        let process = || std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        match options.source() {
            super::super::options::ComposeSource::File(path) => {
                let absolute = if path.is_absolute() { path.clone() } else { process().join(path) };
                absolute.parent().map(Path::to_path_buf).unwrap_or_else(process)
            }
            _ if options.context().anchor().is_absolute() => options.context().anchor().to_path_buf(),
            _ => SCRATCH
                .get_or_init(|| tempfile::tempdir().expect("scratch request dir"))
                .path()
                .to_path_buf(),
        }
    }
}
