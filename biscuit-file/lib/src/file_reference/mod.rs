//! File reference parsing and resolution.
//!
//! A file reference is a compact string descriptor (e.g. `@docs/spec.md`,
//! `!README.md`, `vault:notes/today.md`) that can be resolved lazily against
//! runtime context such as the working directory, git repository root,
//! Cargo workspace layout, and configured paths.
//!
//! ## Examples
//!
//! ```rust,no_run
//! use biscuit_file::FileReference;
//!
//! // Magic reference -- searches repo root, then HOME
//! let file_ref = FileReference::new("@docs/spec.md")?;
//! let resolved = file_ref.resolve()?;
//! if let Some(path) = resolved {
//!     println!("Found: {}", path.display());
//! }
//! # Ok::<(), biscuit_file::FileReferenceError>(())
//! ```

mod context;
pub mod error;
#[cfg(feature = "fetch")]
pub mod fetch;
mod parse;
mod resolve;

use std::path::{Path, PathBuf};

pub use error::FileReferenceError;

#[cfg(feature = "fetch")]
pub use error::FetchError;

/// Pure path-discovery helpers shared by magic (`@`) resolution. Exposed so
/// callers can register convention magic search roots (e.g. a tool's
/// `prompts/` directories) computed from the same git-root / package-area /
/// home anchors the resolver itself uses.
pub use context::{FileResolutionContext, find_git_root, find_package_area, home_dir};

/// The classified kind of a file reference.
///
/// Exposed publicly so callers can branch on reference semantics without
/// re-deriving the grammar with prefix checks such as `starts_with('@')`.
/// The recursive (`%`) modifier is carried separately by
/// [`FileReferenceClass`] because it modifies a kind rather than competing
/// with one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileReferenceKind {
    /// `./foo`, `../foo` (and their `.\`/`..\` spellings). Pinned to the base.
    ExplicitRelative,
    /// A bare path (`foo`, `path/to/foo`) with no base-pinning sigil.
    ImplicitRelative,
    /// A platform-native absolute path.
    Absolute,
    /// `@foo` -- magic-root search.
    Magic,
    /// `!foo` -- package-area resolution.
    Package,
    /// `~`, `~/foo` (and the Windows `~\foo` spelling) -- pinned to the user's
    /// home directory. `~user` is unsupported.
    Home,
    /// `vault:foo` -- configured-vault resolution.
    Vault,
    /// `http://...` / `https://...` -- a remote URL, never a local path.
    Url,
}

/// A file reference's public classification: its [`FileReferenceKind`] plus
/// the recursive (`%`) modifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileReferenceClass {
    /// The reference kind.
    pub kind: FileReferenceKind,
    /// Whether the reference carried the recursive (`%`) prefix.
    pub recursive: bool,
}

/// The provenance of the root a resolution candidate was built from.
///
/// Provenance is carried as data alongside every candidate so diagnostics and
/// completion never infer a candidate's origin from string prefixes. `Absolute`
/// marks a candidate that is the authored absolute path itself, with no joined
/// root.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootProvenance {
    /// The repository (worktree) root.
    Repository,
    /// The source document/base directory.
    Source,
    /// The Cargo package area.
    Package,
    /// The user's home directory.
    Home,
    /// A configured magic (`@`) search root.
    Magic,
    /// A configured vault root.
    Vault,
    /// The authored absolute path (no joined root).
    Absolute,
}

/// One ordered resolution candidate: a concrete path plus the provenance of the
/// root it was derived from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolutionCandidate {
    path: PathBuf,
    provenance: RootProvenance,
}

impl ResolutionCandidate {
    /// The candidate path (not yet normalized to absolute).
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The provenance of the root this candidate was built from.
    pub fn provenance(&self) -> RootProvenance {
        self.provenance
    }
}

pub(crate) fn make_candidate(path: PathBuf, provenance: RootProvenance) -> ResolutionCandidate {
    ResolutionCandidate { path, provenance }
}

/// The outcome of probing a single candidate path.
///
/// A `Missing` or `NonFile` candidate advances the ordered search; an `Io`
/// disposition stops it (the underlying error is retained on the
/// [`DetailedResolution`]). `SearchRoot` marks a recursive search root rather
/// than a directly probed file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeDisposition {
    /// The path does not exist.
    Missing,
    /// The path exists but is not a regular file (e.g. a directory).
    NonFile,
    /// The path is a regular file (or a symlink to one) -- the winner.
    Matched,
    /// Probing failed with an I/O error other than not-found; the search stops.
    Io(std::io::ErrorKind),
    /// A recursive search root walked for matches, not a direct file probe.
    SearchRoot,
}

/// One attempted candidate paired with its probe disposition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbedCandidate {
    candidate: ResolutionCandidate,
    disposition: ProbeDisposition,
}

impl ProbedCandidate {
    /// The candidate that was probed.
    pub fn candidate(&self) -> &ResolutionCandidate {
        &self.candidate
    }

    /// The disposition of the probe.
    pub fn disposition(&self) -> ProbeDisposition {
        self.disposition
    }
}

pub(crate) fn make_probed(
    candidate: ResolutionCandidate,
    disposition: ProbeDisposition,
) -> ProbedCandidate {
    ProbedCandidate {
        candidate,
        disposition,
    }
}

/// The typed classification of a resolution that did not match.
///
/// This is the failure taxonomy the error-propagation pipeline projects into
/// `err.detail.failure`; it must not be re-derived from [`FileReferenceKind`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolutionFailure {
    /// The reference syntax or an anchoring invariant was invalid (e.g. an
    /// interpolated absolute reference that did not expand to an absolute path).
    InvalidReference,
    /// A required context anchor was absent (home, vault, repository, package,
    /// or an interpolation variable).
    MissingContext,
    /// No candidate resolved to a regular file.
    NoMatch,
    /// A candidate probe hit a permission or other I/O failure.
    Io,
    /// A remote URL reference has no local filesystem candidate.
    UnsupportedRemote,
}

/// The terminal outcome of a detailed resolution.
#[derive(Debug)]
pub enum DetailedOutcome {
    /// A candidate matched; carries the resolved absolute path.
    Matched(PathBuf),
    /// Resolution did not produce a match, with a typed classification.
    Failed(ResolutionFailure),
}

/// The detailed outcome of resolving a reference against a
/// [`FileResolutionContext`].
///
/// Retains everything the typed error-propagation pipeline needs to render an
/// ordered, candidate-aware diagnostic without parsing prose: the raw authored
/// reference, its parsed class, the base/source anchors, the repository root the
/// search anchored on (when a root was used), every attempted candidate with its
/// probe disposition, and the underlying [`FileReferenceError`] when one exists.
///
/// A no-match is a typed [`DetailedOutcome::Failed`] carrying
/// [`ResolutionFailure::NoMatch`], never a silent `Ok(None)`.
#[derive(Debug)]
pub struct DetailedResolution {
    raw: String,
    class: FileReferenceClass,
    effective_kind: FileReferenceKind,
    base_dir: PathBuf,
    source_path: Option<PathBuf>,
    repository_root: Option<PathBuf>,
    candidates: Vec<ProbedCandidate>,
    outcome: DetailedOutcome,
    error: Option<FileReferenceError>,
}

impl DetailedResolution {
    /// The raw authored reference string.
    pub fn raw(&self) -> &str {
        &self.raw
    }

    /// The parsed classification of the reference.
    pub fn class(&self) -> FileReferenceClass {
        self.class
    }

    /// The filesystem-anchoring kind that actually drove resolution.
    ///
    /// Equal to `class().kind` for most references. It differs only when
    /// environment interpolation reclassified a local anchoring reference: an
    /// implicit `{{DIR}}/x.md` whose `DIR` expanded to an absolute path is
    /// authored [`FileReferenceKind::ImplicitRelative`] yet resolves as
    /// [`FileReferenceKind::Absolute`]. Diagnostics expose both so the effective
    /// behavior is honest without hiding the authored intent.
    pub fn effective_kind(&self) -> FileReferenceKind {
        self.effective_kind
    }

    /// The base directory references resolved against.
    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    /// The source document/file, when one was supplied on the context.
    pub fn source_path(&self) -> Option<&Path> {
        self.source_path.as_deref()
    }

    /// The repository root the search anchored on, when a root was used or
    /// supplied.
    pub fn repository_root(&self) -> Option<&Path> {
        self.repository_root.as_deref()
    }

    /// The ordered candidates attempted, each with its probe disposition.
    ///
    /// For a recursive reference these are the search roots
    /// ([`ProbeDisposition::SearchRoot`]) rather than direct file probes.
    pub fn candidates(&self) -> &[ProbedCandidate] {
        &self.candidates
    }

    /// The terminal outcome.
    pub fn outcome(&self) -> &DetailedOutcome {
        &self.outcome
    }

    /// The underlying [`FileReferenceError`], present for every failure except
    /// [`ResolutionFailure::NoMatch`].
    pub fn error(&self) -> Option<&FileReferenceError> {
        self.error.as_ref()
    }

    /// The resolved absolute path when the reference matched.
    pub fn matched_path(&self) -> Option<&Path> {
        match &self.outcome {
            DetailedOutcome::Matched(p) => Some(p),
            DetailedOutcome::Failed(_) => None,
        }
    }

    /// Project the detailed outcome onto the legacy convenience shape.
    ///
    /// A match yields `Ok(Some(path))`, a no-match yields `Ok(None)`, and every
    /// other failure yields its underlying `Err`. The ordered candidate plan is
    /// discarded; callers needing it must hold the [`DetailedResolution`].
    pub fn into_convenience(self) -> Result<Option<PathBuf>, FileReferenceError> {
        match self.outcome {
            DetailedOutcome::Matched(p) => Ok(Some(p)),
            DetailedOutcome::Failed(ResolutionFailure::NoMatch) => Ok(None),
            // A non-`NoMatch` failure always carries its underlying error; the
            // `raw`-based fallback only guards against an internally
            // inconsistent construction.
            DetailedOutcome::Failed(_) => {
                Err(self.error.unwrap_or(FileReferenceError::InvalidSyntax(self.raw)))
            }
        }
    }
}

/// Entry form for a partial completion token.
///
/// Corresponds to the subset of [`FileReferenceKind`] variants that
/// [`FileReference::complete_partial`] supports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionEntryForm {
    /// `@`-prefixed magic path. Roots are the enclosing git root and the
    /// user's home directory.
    Magic,
    /// Bare implicit-relative path. Roots are the enclosing git root and the
    /// caller-provided base directory (in that order, when distinct).
    ImplicitRelative,
}

/// Expansion of a partial completion token.
///
/// Returned by [`FileReference::complete_partial`] for tokens in the two
/// supported entry forms. Exposes the absolute directories a completion
/// consumer should enumerate, the partial filename being matched, and the
/// prefix the shell will re-insert in front of each emitted candidate.
#[derive(Debug, Clone)]
pub struct PartialCompletion {
    entry_form: CompletionEntryForm,
    roots: Vec<PathBuf>,
    active_segment: String,
    rendered_prefix: String,
}

impl PartialCompletion {
    /// The classified entry form of the token.
    pub fn entry_form(&self) -> CompletionEntryForm {
        self.entry_form
    }

    /// Absolute directories the caller should enumerate candidates from.
    ///
    /// Each root already has the scope (everything before the active
    /// segment) appended. Callers should not assume these directories
    /// exist on the filesystem; non-existent roots should simply yield
    /// no candidates.
    pub fn roots(&self) -> &[PathBuf] {
        &self.roots
    }

    /// The partial filename after the last `/` in the token.
    ///
    /// Empty for tokens that end with `/` or for the bare-sigil token `@`.
    pub fn active_segment(&self) -> &str {
        &self.active_segment
    }

    /// The string to render before each matched filename in the emitted
    /// completion candidate.
    ///
    /// For `Magic` tokens this preserves the leading `@` and the scope
    /// (e.g. `@prompts/`); for `ImplicitRelative` tokens it is just the
    /// scope (e.g. `prompts/`), and may be empty when the user has typed
    /// no `/`.
    pub fn rendered_prefix(&self) -> &str {
        &self.rendered_prefix
    }
}

pub(crate) fn make_partial_completion(
    entry_form: CompletionEntryForm,
    roots: Vec<PathBuf>,
    active_segment: String,
    rendered_prefix: String,
) -> PartialCompletion {
    PartialCompletion {
        entry_form,
        roots,
        active_segment,
        rendered_prefix,
    }
}

/// The resolved target of a file reference.
///
/// Distinguishes between local filesystem paths and remote URLs so callers
/// can handle each appropriately.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolved {
    /// A local filesystem path.
    Local(PathBuf),
    /// A remote HTTP(S) URL.
    #[cfg(feature = "url")]
    Remote(::url::Url),
}

/// Position for magic path insertion.
///
/// ## Examples
///
/// ```rust,no_run
/// use biscuit_file::{FileReference, PathPosition};
///
/// let file_ref = FileReference::new("@config.toml")?
///     .add_magic_path("/opt/configs", PathPosition::Start)
///     .add_magic_path("/etc/defaults", PathPosition::End);
/// # Ok::<(), biscuit_file::FileReferenceError>(())
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathPosition {
    /// Insert before the default search roots.
    Start,
    /// Insert after the default search roots.
    End,
}

/// A parsed file reference with lazy resolution.
///
/// Construction parses the reference string syntactically. Resolution is
/// deferred to the `resolve()` or `resolve_relative()` methods, which
/// inspect the filesystem and environment at call time.
///
/// ## Examples
///
/// ```rust,no_run
/// use biscuit_file::FileReference;
///
/// // Relative path
/// let file_ref = FileReference::new("./README.md")?;
/// assert_eq!(file_ref.raw(), "./README.md");
///
/// // Magic reference with vault
/// let file_ref = FileReference::new("vault:notes/today.md")?
///     .add_vault("/path/to/vault");
/// let resolved = file_ref.resolve()?;
/// # Ok::<(), biscuit_file::FileReferenceError>(())
/// ```
#[derive(Debug, Clone)]
pub struct FileReference {
    raw: String,
    parsed: ParsedReference,
    magic_paths: MagicPathList,
    vault_roots: Vec<PathBuf>,
}

impl FileReference {
    /// Parse a file reference string.
    ///
    /// ## Errors
    ///
    /// Returns `FileReferenceError::InvalidSyntax` if the string cannot be parsed.
    pub fn new(raw: &str) -> Result<Self, FileReferenceError> {
        let parsed = parse::parse(raw)?;
        Ok(Self {
            raw: raw.to_string(),
            parsed,
            magic_paths: MagicPathList::default(),
            vault_roots: Vec::new(),
        })
    }

    /// The original reference string.
    pub fn raw(&self) -> &str {
        &self.raw
    }

    /// The public classification of this reference.
    ///
    /// Callers must use this rather than re-parsing the raw string with prefix
    /// checks; `FileReference` is the single grammar authority.
    pub fn class(&self) -> FileReferenceClass {
        FileReferenceClass {
            kind: self.parsed.kind.public_kind(),
            recursive: self.parsed.recursive,
        }
    }

    /// Add a custom search path for magic (`@`) references.
    pub fn add_magic_path(mut self, path: impl Into<PathBuf>, position: PathPosition) -> Self {
        match position {
            PathPosition::Start => self.magic_paths.prepend.push(path.into()),
            PathPosition::End => self.magic_paths.append.push(path.into()),
        }
        self
    }

    /// Prepend the current Cargo workspace package area as a magic search root.
    ///
    /// When the current working directory lives inside a Cargo workspace member
    /// (or at a workspace area root), magic (`@`) lookups will first search the
    /// package area before falling back to the git repository root and HOME.
    /// This is useful for monorepos where prompts or config files live alongside
    /// the package, e.g. `@prompts/commit.md` resolving to
    /// `<workspace>/<area>/prompts/commit.md` when invoked from inside that area.
    ///
    /// If the current directory is not inside a Cargo workspace (or metadata
    /// cannot be loaded), this is a no-op.
    pub fn with_package_area_magic_path(self) -> Self {
        let Ok(cwd) = std::env::current_dir() else {
            return self;
        };
        let Ok(Some(git_root)) = context::find_git_root(&cwd) else {
            return self;
        };
        match context::find_package_area(&git_root, &cwd) {
            Ok(Some(area)) => self.add_magic_path(area, PathPosition::Start),
            _ => self,
        }
    }

    /// Add a vault root for `vault:` references.
    pub fn add_vault(mut self, path: impl Into<PathBuf>) -> Self {
        self.vault_roots.push(path.into());
        self
    }

    /// Resolve the reference to an absolute filesystem path.
    ///
    /// Uses the ambient process working directory for relative, `@`, and
    /// `!` lookups. When the reference comes from a document or file whose
    /// own location should drive resolution, prefer [`resolve_from`].
    ///
    /// ## Returns
    ///
    /// - `Ok(Some(path))` -- the reference resolved to an existing file
    /// - `Ok(None)` -- the reference is well-formed but no matching file was found
    ///
    /// ## Errors
    ///
    /// Returns an error if resolution requires state that cannot be determined
    /// (e.g. missing environment variable, vault not configured).
    ///
    /// [`resolve_from`]: Self::resolve_from
    pub fn resolve(&self) -> Result<Option<PathBuf>, FileReferenceError> {
        let ctx = context::ResolutionContext::from_ambient()?;
        resolve::resolve(&self.parsed, &self.magic_paths, &self.vault_roots, &ctx)
    }

    /// Resolve the reference treating `base` as the working directory.
    ///
    /// This overrides the ambient process CWD used for relative, `@`
    /// (magic), and `!` (package) lookups. Use this when a reference
    /// appears inside a document or file and should be resolved relative
    /// to *that file's* location rather than wherever the current process
    /// happens to be running.
    ///
    /// HOME and environment variables are still read from the live
    /// process state.
    ///
    /// ## Returns
    ///
    /// - `Ok(Some(path))` -- the reference resolved to an existing file
    /// - `Ok(None)` -- the reference is well-formed but no matching file was found
    ///
    /// ## Errors
    ///
    /// Returns an error if resolution requires state that cannot be
    /// determined (e.g. missing environment variable, vault not configured).
    pub fn resolve_from(&self, base: &Path) -> Result<Option<PathBuf>, FileReferenceError> {
        let ctx = context::ResolutionContext::from_base(base)?;
        resolve::resolve(&self.parsed, &self.magic_paths, &self.vault_roots, &ctx)
    }

    /// Resolve the reference against an explicit [`FileResolutionContext`].
    ///
    /// Unlike [`resolve`] and [`resolve_from`], this reads **no** ambient
    /// process state during candidate construction: the base directory, home
    /// directory, environment snapshot, repository root, and magic/vault roots
    /// all come from the context. This is the document-backed resolution entry
    /// point that Claudine and Darkmatter drive with a `sniff`-discovered
    /// worktree root.
    ///
    /// The context's magic and vault roots are authoritative here; any roots
    /// configured on the `FileReference` itself (via [`add_magic_path`] /
    /// [`add_vault`]) apply only to the ambient [`resolve`]/[`resolve_from`]
    /// paths.
    ///
    /// ## Returns
    ///
    /// - `Ok(Some(path))` -- the reference resolved to an existing file
    /// - `Ok(None)` -- the reference is well-formed but no matching file was found
    ///
    /// ## Errors
    ///
    /// Returns [`FileReferenceError::RepositoryRootNotContainingSource`] when a
    /// caller-supplied repository root does not contain the base, and typed
    /// missing-context errors (e.g. [`FileReferenceError::MissingHomeContext`])
    /// when a required anchor is absent.
    ///
    /// [`resolve`]: Self::resolve
    /// [`resolve_from`]: Self::resolve_from
    /// [`add_magic_path`]: Self::add_magic_path
    /// [`add_vault`]: Self::add_vault
    pub fn resolve_in_context(
        &self,
        ctx: &FileResolutionContext,
    ) -> Result<Option<PathBuf>, FileReferenceError> {
        self.resolve_detailed(ctx).into_convenience()
    }

    /// Resolve against an explicit context, returning the full detailed outcome.
    ///
    /// This is the context-aware detailed operation: it retains the parsed
    /// class, the ordered candidate plan with per-candidate probe dispositions,
    /// the repository root the search anchored on, and the underlying typed
    /// error. Diagnostics and completion consume this rather than the
    /// `Ok(None)`-collapsing convenience methods, which discard the candidate
    /// plan D8 requires.
    ///
    /// Unlike [`resolve_in_context`], this never returns `Err`: every failure --
    /// including an invalid context, a missing anchor, or an I/O probe failure --
    /// is a typed [`DetailedOutcome::Failed`] with the underlying error attached.
    ///
    /// [`resolve_in_context`]: Self::resolve_in_context
    pub fn resolve_detailed(&self, ctx: &FileResolutionContext) -> DetailedResolution {
        let class = self.class();
        let base_dir = ctx.base_dir().to_path_buf();
        let source_path = ctx.source_path().map(Path::to_path_buf);

        if let Err(error) = ctx.validate() {
            return DetailedResolution {
                raw: self.raw.clone(),
                class,
                // Validation fails before any interpolation, so the authored
                // kind is the effective kind.
                effective_kind: class.kind,
                base_dir,
                source_path,
                repository_root: ctx.repository_root().map(Path::to_path_buf),
                candidates: Vec::new(),
                outcome: DetailedOutcome::Failed(ResolutionFailure::MissingContext),
                error: Some(error),
            };
        }

        let internal = context::ResolutionContext::from_context(ctx);
        let core = resolve::resolve_core(
            &self.parsed,
            ctx.magic_paths(),
            ctx.vault_roots(),
            &internal,
        );

        let effective_kind = core.effective_kind.unwrap_or(class.kind);
        let (outcome, error) = core.outcome.into_parts();
        DetailedResolution {
            raw: self.raw.clone(),
            class,
            effective_kind,
            base_dir,
            source_path,
            repository_root: core.repository_root,
            candidates: core.candidates,
            outcome,
            error,
        }
    }

    /// Build the ordered candidate plan for this reference without probing the
    /// filesystem.
    ///
    /// Exposes the exact candidates (or, for a recursive reference, the search
    /// roots) that resolution would attempt, each carrying its
    /// [`RootProvenance`]. This is the separable builder diagnostics,
    /// completion, and tests inspect without reimplementing the algorithm.
    ///
    /// ## Errors
    ///
    /// Returns the typed error when the context is invalid or a required anchor
    /// (interpolation variable, vault, home, repository) cannot be established.
    pub fn candidate_plan(
        &self,
        ctx: &FileResolutionContext,
    ) -> Result<Vec<ResolutionCandidate>, FileReferenceError> {
        ctx.validate()?;
        let internal = context::ResolutionContext::from_context(ctx);
        resolve::candidate_plan(
            &self.parsed,
            ctx.magic_paths(),
            ctx.vault_roots(),
            &internal,
        )
    }

    /// Expand a partial completion token into its implied roots and segments.
    ///
    /// Given a (possibly incomplete) reference string like `@prompts/p` and
    /// a base directory, returns the absolute roots a completion consumer
    /// should enumerate, the active segment (partial filename after the
    /// last `/`), and the prefix the shell will insert in front of each
    /// candidate.
    ///
    /// Only two entry forms are supported: `@`-prefixed magic paths and
    /// implicit-relative paths. All other forms (`!`, `/`, `./`, `../`,
    /// `vault:`, `%`, `{{...}}`) return `Ok(None)` so callers can cleanly
    /// opt out rather than silently re-interpret them.
    ///
    /// Resolution rules:
    ///
    /// - Path-separator reset: the active segment is the portion after the
    ///   last `/`. Everything up to and including that `/` is the
    ///   "scope", which is appended to each implied root.
    /// - Magic form: roots are `{git_root, home_dir}` (in that order),
    ///   each with the scope appended.
    /// - Implicit relative: roots are `{git_root, base}` (distinct), each
    ///   with the scope appended.
    ///
    /// Markdown filtering, ranking, and typed-length policy are not
    /// applied here -- they are the caller's responsibility.
    ///
    /// ## Returns
    ///
    /// - `Ok(Some(completion))` -- the token is completable
    /// - `Ok(None)` -- the token uses an unsupported entry form
    ///
    /// ## Errors
    ///
    /// Returns an error only if the caller passes a relative `base` and
    /// the ambient CWD cannot be read, or if git discovery itself fails.
    /// A missing git repository is **not** an error.
    pub fn complete_partial(
        token: &str,
        base: &Path,
    ) -> Result<Option<PartialCompletion>, FileReferenceError> {
        resolve::complete_partial(token, base)
    }

    /// Expand a partial completion token against an explicit
    /// [`FileResolutionContext`], reading no ambient process state.
    ///
    /// This is the document-backed counterpart to [`complete_partial`]: instead
    /// of discovering the repository root and home directory live from `base`,
    /// it consumes the captured repository root, home directory, and configured
    /// magic roots the context supplies. Completion and execution therefore
    /// share one candidate builder and one context, so a value emitted here
    /// enumerates from the same roots [`resolve_detailed`] probes (D9). The
    /// `Magic` form in particular now surfaces the context's configured magic
    /// roots, which the ambient [`complete_partial`] cannot see.
    ///
    /// The two supported entry forms and the returned shape are identical to
    /// [`complete_partial`].
    ///
    /// ## Returns
    ///
    /// - `Ok(Some(completion))` -- the token is completable
    /// - `Ok(None)` -- the token uses an unsupported entry form
    ///
    /// ## Errors
    ///
    /// Returns [`FileReferenceError::RepositoryRootNotContainingSource`] when the
    /// context's repository root does not contain its base directory.
    ///
    /// [`complete_partial`]: Self::complete_partial
    /// [`resolve_detailed`]: Self::resolve_detailed
    pub fn complete_partial_in_context(
        token: &str,
        ctx: &FileResolutionContext,
    ) -> Result<Option<PartialCompletion>, FileReferenceError> {
        resolve::complete_partial_in_context(token, ctx)
    }

    /// Resolve the reference and return a path relative to `base`.
    ///
    /// If `base` is `None`, the current working directory is used.
    ///
    /// ## Returns
    ///
    /// - `Ok(Some(path))` -- the relative path to the resolved file
    /// - `Ok(None)` -- no matching file was found
    ///
    /// ## Errors
    ///
    /// Returns an error if resolution fails or if a relative path cannot
    /// be computed between the two locations.
    pub fn resolve_relative(
        &self,
        base: Option<&Path>,
    ) -> Result<Option<PathBuf>, FileReferenceError> {
        let resolved = self.resolve()?;

        let resolved = match resolved {
            Some(p) => p,
            None => return Ok(None),
        };

        let base_dir = match base {
            Some(b) => b.to_path_buf(),
            None => std::env::current_dir().map_err(FileReferenceError::CurrentDirectory)?,
        };

        let relative = resolve::diff_paths(&resolved, &base_dir).ok_or_else(|| {
            FileReferenceError::RelativePath {
                from: base_dir,
                to: resolved.clone(),
            }
        })?;

        Ok(Some(relative))
    }

    /// Resolve the reference to a typed target.
    ///
    /// Unlike [`resolve()`], which only returns local filesystem paths,
    /// this returns a [`Resolved`] that distinguishes between local paths
    /// and remote URLs.
    ///
    /// ## Returns
    ///
    /// - `Ok(Some(Resolved::Local(path)))` -- a local file was found
    /// - `Ok(Some(Resolved::Remote(url)))` -- a remote URL was classified
    /// - `Ok(None)` -- the reference is well-formed but no local file was found
    ///
    /// ## Errors
    ///
    /// Returns an error if the URL is malformed or if resolution requires
    /// state that cannot be determined.
    ///
    /// [`resolve()`]: Self::resolve
    #[cfg(feature = "url")]
    pub fn resolve_target(&self) -> Result<Option<Resolved>, FileReferenceError> {
        resolve::resolve_target(&self.parsed, &self.magic_paths, &self.vault_roots)
    }
}

// --- Internal types ---

#[derive(Debug, Clone)]
pub(crate) struct ParsedReference {
    pub recursive: bool,
    pub kind: ReferenceKind,
}

#[derive(Debug, Clone)]
pub(crate) enum ReferenceKind {
    Relative(PathTemplate),
    ImplicitRelative(PathTemplate),
    Absolute(PathTemplate),
    Magic(PathTemplate),
    Package(PathTemplate),
    Home(PathTemplate),
    Vault(PathTemplate),
    Url(PathTemplate),
}

impl ReferenceKind {
    pub(crate) fn template(&self) -> &PathTemplate {
        match self {
            Self::Relative(t)
            | Self::ImplicitRelative(t)
            | Self::Absolute(t)
            | Self::Magic(t)
            | Self::Package(t)
            | Self::Home(t)
            | Self::Vault(t)
            | Self::Url(t) => t,
        }
    }

    /// Project the internal kind onto the public [`FileReferenceKind`].
    pub(crate) fn public_kind(&self) -> FileReferenceKind {
        match self {
            Self::Relative(_) => FileReferenceKind::ExplicitRelative,
            Self::ImplicitRelative(_) => FileReferenceKind::ImplicitRelative,
            Self::Absolute(_) => FileReferenceKind::Absolute,
            Self::Magic(_) => FileReferenceKind::Magic,
            Self::Package(_) => FileReferenceKind::Package,
            Self::Home(_) => FileReferenceKind::Home,
            Self::Vault(_) => FileReferenceKind::Vault,
            Self::Url(_) => FileReferenceKind::Url,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct PathTemplate {
    pub segments: Vec<TemplateSegment>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TemplateSegment {
    Literal(String),
    EnvVar(String),
}

#[derive(Debug, Clone, Default)]
pub(crate) struct MagicPathList {
    pub prepend: Vec<PathBuf>,
    pub append: Vec<PathBuf>,
}
