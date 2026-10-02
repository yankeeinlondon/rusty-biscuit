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
    /// Home and environment are read through the same biscuit-file helpers
    /// [`FileResolutionContext::new`] uses, so `USERPROFILE` on Windows and
    /// the skipping of non-UTF-8 variables are unchanged. This is the only
    /// reader of process state on a request path; only binaries call it.
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
}

impl ContextBuildError {
    /// The request directory the failing build was anchored at.
    pub fn request_dir(&self) -> &Path {
        match self {
            Self::Discovery { dir, .. } | Self::Invalid { dir, .. } => dir,
        }
    }

    /// The failure class, as file-reference errors report it.
    ///
    /// A discovery failure is [`ResolutionFailure::MissingContext`], the class
    /// biscuit-file gives its own Git discovery errors.
    pub fn resolution_failure(&self) -> ResolutionFailure {
        match self {
            Self::Discovery { .. } => ResolutionFailure::MissingContext,
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
        capture_runtime_context_for_groups(dir, &[ContextGroup::Repo]);
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
    /// ## Errors
    ///
    /// Returns the builder's [`ContextBuildError`].
    pub fn prepare(
        mut options: ComposeOptions,
        snapshot: &RequestSnapshot,
    ) -> Result<Self, ContextBuildError> {
        let dir = snapshot.request_dir();
        options.anchor_unanchored_context(dir);
        let observation = match options.request_repository_containing(dir) {
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
        Ok(Self::assemble(options, context))
    }

    /// Pairs `options` with a context the caller built or derived from one
    /// that was built.
    ///
    /// For callers that hold a context already: a source derivation, a source
    /// in another repository, or an embedder's own built context.
    ///
    /// ## Errors
    ///
    /// Returns [`ContextBuildError::Invalid`] when `context` fails
    /// [`validate`](FileResolutionContext::validate).
    pub fn with_context(
        options: ComposeOptions,
        context: FileResolutionContext,
    ) -> Result<Self, ContextBuildError> {
        context.validate().map_err(|source| ContextBuildError::Invalid {
            dir: context.request_cwd().to_path_buf(),
            source,
        })?;
        options.establish_request_repository();
        Ok(Self::assemble(options, context))
    }

    fn assemble(mut options: ComposeOptions, context: FileResolutionContext) -> Self {
        options.align_context_environment(context.env());
        Self { options, context }
    }

    /// This request with its options changed by `change`, keeping its
    /// context.
    ///
    /// For per-run settings decided after preparation, such as the approval
    /// set pre-flight computes.
    #[must_use]
    pub fn map_options(self, change: impl FnOnce(ComposeOptions) -> ComposeOptions) -> Self {
        Self::assemble(change(self.options), self.context)
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

    /// This request with its options changed by `change`, keeping its context
    /// and the options' own `ctx.*` environment.
    ///
    /// For the pipeline's own derivations (a child source, an inline pass);
    /// [`map_options`](Self::map_options) is the caller-facing form.
    #[must_use]
    pub(crate) fn derive(mut self, change: impl FnOnce(ComposeOptions) -> ComposeOptions) -> Self {
        self.options = change(self.options);
        self
    }

    /// This request composing the file at `path`; see
    /// [`ComposeOptions::with_source_file`].
    #[must_use]
    pub(crate) fn with_source_file(self, path: impl Into<PathBuf>) -> Self {
        self.derive(|options| options.with_source_file(path))
    }

    /// This request composing `url`; see [`ComposeOptions::with_source_url`].
    #[must_use]
    pub(crate) fn with_source_url(self, url: url::Url) -> Self {
        self.derive(|options| options.with_source_url(url))
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
        self.derive(|options| options.with_accepted_source_file_in(path, opening, &context))
    }

    /// This request as the compose pass of `document` will see it, after
    /// [`ComposeOptions::extend_context_for`].
    ///
    /// Pre-flight discovery evaluates a document before that pass runs, so it
    /// must read the same context or it would observe groups the real pass has.
    pub(crate) fn extended_for(
        &self,
        document: &crate::markdown::Markdown,
    ) -> std::borrow::Cow<'_, Self> {
        if !self.options.needs_extension_for(document) {
            return std::borrow::Cow::Borrowed(self);
        }
        let mut extended = self.clone();
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
    /// process and the environment from the options' own context, so `ctx.*`
    /// is unchanged.
    pub(crate) fn request(options: ComposeOptions) -> ComposeRequest {
        let dir = legacy_request_dir(&options);
        let snapshot = RequestSnapshot::new(dir)
            .with_home(biscuit_file::home_dir())
            .with_env(options.context().env().clone());
        ComposeRequest::prepare(options, &snapshot).expect("test request")
    }

    /// A request that resolves through `context`, as the options' old
    /// context field did.
    pub(crate) fn request_in(options: ComposeOptions, context: FileResolutionContext) -> ComposeRequest {
        ComposeRequest::with_context(options, context).expect("test request context")
    }

    /// A file source's directory, else the context's anchor, else the
    /// process directory.
    pub(crate) fn legacy_request_dir(options: &ComposeOptions) -> PathBuf {
        let process = || std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        match options.source() {
            super::super::options::ComposeSource::File(path) => {
                let absolute = if path.is_absolute() { path.clone() } else { process().join(path) };
                absolute.parent().map(Path::to_path_buf).unwrap_or_else(process)
            }
            _ if options.context().anchor().is_absolute() => options.context().anchor().to_path_buf(),
            _ => process(),
        }
    }
}
