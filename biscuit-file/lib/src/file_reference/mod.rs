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
mod parse;
mod resolve;

use std::path::{Path, PathBuf};

pub use error::FileReferenceError;

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
    Vault(PathTemplate),
}

impl ReferenceKind {
    pub(crate) fn template(&self) -> &PathTemplate {
        match self {
            Self::Relative(t)
            | Self::ImplicitRelative(t)
            | Self::Absolute(t)
            | Self::Magic(t)
            | Self::Package(t)
            | Self::Vault(t) => t,
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
