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
    /// `~foo` / `~/foo` -- pinned to the user's home directory.
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

/// Entry form for a partial completion token.
///
/// Corresponds to the subset of [`FileReferenceKind`] variants that
/// [`FileReference::complete_partial`] supports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionEntryForm {
    /// `@`-prefixed magic path. Roots are the enclosing git root and the
    /// user's home directory.
    Magic,
    /// Bare implicit-relative path. Roots are the caller-provided base
    /// directory and its enclosing git root (when distinct).
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
        ctx.validate()?;
        let internal = context::ResolutionContext::from_context(ctx);
        resolve::resolve(
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
    /// - Implicit relative: roots are `{base, git_root}` (distinct), each
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
