use std::collections::HashMap;
use std::path::{Path, PathBuf};

use tracing::{debug, trace};

use crate::file_reference::MagicPathList;
use crate::file_reference::error::FileReferenceError;
use crate::file_reference::resolve::normalize_components;

/// Policy for assigning a package-area root when a monorepo directory is not
/// covered by an explicitly cataloged area.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageAreaFallback {
    /// Do not infer an area for uncataloged directories.
    None,
    /// Treat the first component below the repository root as the area.
    FirstComponent,
}

/// Validation failure while constructing a [`RepositoryScopeCatalog`].
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum RepositoryScopeCatalogError {
    /// Every catalog root must be absolute.
    #[error("{root_kind} root must be absolute: `{path}`")]
    RootNotAbsolute {
        root_kind: &'static str,
        path: PathBuf,
    },
    /// Catalog roots must not retain `.` or `..` components.
    #[error("{root_kind} root must be lexically normalized: `{path}`")]
    RootNotNormalized {
        root_kind: &'static str,
        path: PathBuf,
    },
    /// Package and package-area roots must stay inside their repository.
    #[error("{root_kind} root `{path}` is outside repository `{repository_root}`")]
    RootOutsideRepository {
        root_kind: &'static str,
        path: PathBuf,
        repository_root: PathBuf,
    },
}

/// Repository, package-area, and package roots captured by an observing caller.
///
/// Construction and scope selection are purely lexical. Neither operation
/// reads the filesystem or performs repository discovery.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryScopeCatalog {
    repository_root: PathBuf,
    package_area_roots: Vec<PathBuf>,
    package_roots: Vec<PathBuf>,
    package_area_fallback: PackageAreaFallback,
}

impl RepositoryScopeCatalog {
    /// Validate and construct a repository scope catalog.
    pub fn new(
        repository_root: impl Into<PathBuf>,
        package_area_roots: Vec<PathBuf>,
        package_roots: Vec<PathBuf>,
        package_area_fallback: PackageAreaFallback,
    ) -> Result<Self, RepositoryScopeCatalogError> {
        let repository_root = repository_root.into();
        validate_catalog_root("repository", &repository_root, None)?;
        let package_area_roots = validate_catalog_roots(
            "package-area",
            package_area_roots,
            &repository_root,
        )?;
        let package_roots =
            validate_catalog_roots("package", package_roots, &repository_root)?;
        Ok(Self {
            repository_root,
            package_area_roots,
            package_roots,
            package_area_fallback,
        })
    }

    /// The repository root represented by this catalog.
    pub fn repository_root(&self) -> &Path {
        &self.repository_root
    }

    /// Explicit package-area roots, deduplicated in caller order.
    pub fn package_area_roots(&self) -> &[PathBuf] {
        &self.package_area_roots
    }

    /// Explicit package roots, deduplicated in caller order.
    pub fn package_roots(&self) -> &[PathBuf] {
        &self.package_roots
    }

    /// Select the most-specific cataloged scope containing `base`.
    #[must_use]
    pub fn scope_for(&self, base: &Path) -> RepositoryScope {
        let base = normalize_components(base);
        if !base.starts_with(&self.repository_root) {
            return RepositoryScope::default();
        }
        let package_root = most_specific_containing(&self.package_roots, &base);
        let package_area_root = most_specific_containing(&self.package_area_roots, &base)
            .or_else(|| package_root.is_none().then(|| self.fallback_area_for(&base)).flatten());
        RepositoryScope {
            repository_root: Some(self.repository_root.clone()),
            package_area_root,
            package_root,
        }
    }

    fn fallback_area_for(&self, base: &Path) -> Option<PathBuf> {
        if self.package_area_fallback == PackageAreaFallback::None {
            return None;
        }
        let relative = base.strip_prefix(&self.repository_root).ok()?;
        let first = relative.components().next()?;
        Some(self.repository_root.join(first.as_os_str()))
    }
}

/// Repository scopes selected for one reference base.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RepositoryScope {
    repository_root: Option<PathBuf>,
    package_area_root: Option<PathBuf>,
    package_root: Option<PathBuf>,
}

impl RepositoryScope {
    /// Selected repository root, absent when the base is outside the catalog.
    pub fn repository_root(&self) -> Option<&Path> {
        self.repository_root.as_deref()
    }

    /// Selected package-area root.
    pub fn package_area_root(&self) -> Option<&Path> {
        self.package_area_root.as_deref()
    }

    /// Selected package root.
    pub fn package_root(&self) -> Option<&Path> {
        self.package_root.as_deref()
    }
}

fn validate_catalog_roots(
    root_kind: &'static str,
    roots: Vec<PathBuf>,
    repository_root: &Path,
) -> Result<Vec<PathBuf>, RepositoryScopeCatalogError> {
    let mut validated = Vec::with_capacity(roots.len());
    for root in roots {
        validate_catalog_root(root_kind, &root, Some(repository_root))?;
        if !validated.contains(&root) {
            validated.push(root);
        }
    }
    Ok(validated)
}

fn validate_catalog_root(
    root_kind: &'static str,
    root: &Path,
    repository_root: Option<&Path>,
) -> Result<(), RepositoryScopeCatalogError> {
    if !root.is_absolute() {
        return Err(RepositoryScopeCatalogError::RootNotAbsolute {
            root_kind,
            path: root.to_path_buf(),
        });
    }
    if normalize_components(root) != root {
        return Err(RepositoryScopeCatalogError::RootNotNormalized {
            root_kind,
            path: root.to_path_buf(),
        });
    }
    if let Some(repository_root) = repository_root
        && !root.starts_with(repository_root)
    {
        return Err(RepositoryScopeCatalogError::RootOutsideRepository {
            root_kind,
            path: root.to_path_buf(),
            repository_root: repository_root.to_path_buf(),
        });
    }
    Ok(())
}

fn most_specific_containing(roots: &[PathBuf], base: &Path) -> Option<PathBuf> {
    roots
        .iter()
        .filter(|root| base.starts_with(root))
        .max_by_key(|root| root.components().count())
        .cloned()
}

/// Runtime state captured for file reference resolution.
pub(crate) struct ResolutionContext {
    pub cwd: PathBuf,
    pub home_dir: Option<PathBuf>,
    pub env: HashMap<String, String>,
    /// Caller-supplied repository root. When `Some`, resolution anchors the
    /// implicit/magic/package roots on it. When `None`, the ambient
    /// compatibility methods fall back to live `gix` discovery (see
    /// `allow_ambient_discovery`); the explicit document-backed context does
    /// not, so `None` means "no repository root" rather than "discover one".
    pub repository_root: Option<PathBuf>,
    pub package_root: Option<PathBuf>,
    pub package_area: Option<PathBuf>,
    /// Whether the resolver may fall back to a live `gix` repository-root walk
    /// when an anchor was not supplied.
    ///
    /// `true` for the `resolve()`/`resolve_from()` compatibility methods, which
    /// resolve against live process state. `false` for the explicit,
    /// request-scoped [`FileResolutionContext`] path
    /// (`resolve_detailed`/`candidate_plan`/`resolve_in_context`): that context
    /// is authoritative (D2/D10), so a missing anchor is consumed as absent
    /// rather than re-probed from the filesystem after construction.
    pub allow_ambient_discovery: bool,
}

impl ResolutionContext {
    /// Build from live process state.
    pub fn from_ambient() -> Result<Self, FileReferenceError> {
        let cwd = std::env::current_dir().map_err(FileReferenceError::CurrentDirectory)?;
        let home_dir = home_dir();
        let env: HashMap<String, String> = std::env::vars().collect();

        debug!(
            ?cwd,
            home_dir_set = home_dir.is_some(),
            env_var_count = env.len(),
            "built resolution context"
        );

        Ok(Self {
            cwd,
            home_dir,
            env,
            repository_root: None,
            package_root: None,
            package_area: None,
            allow_ambient_discovery: true,
        })
    }

    /// Build a context that treats `base` as the working directory, while
    /// still reading HOME and environment variables from the live process
    /// state.
    ///
    /// If `base` is a relative path, it is joined onto the ambient CWD so
    /// that repository discovery always operates on an absolute location.
    pub fn from_base(base: &Path) -> Result<Self, FileReferenceError> {
        let cwd = if base.is_absolute() {
            base.to_path_buf()
        } else {
            let ambient = std::env::current_dir().map_err(FileReferenceError::CurrentDirectory)?;
            ambient.join(base)
        };
        let home_dir = home_dir();
        let env = std::env::vars().collect();

        Ok(Self {
            cwd,
            home_dir,
            env,
            repository_root: None,
            package_root: None,
            package_area: None,
            allow_ambient_discovery: true,
        })
    }

    /// Build the internal resolution context from an explicit, caller-owned
    /// [`FileResolutionContext`], reading no ambient process state.
    ///
    /// The context is authoritative: `allow_ambient_discovery` is `false`, so
    /// resolution consumes only the anchors the caller supplied and never falls
    /// back to a live repository-root walk (D2/D10).
    pub fn from_context(ctx: &FileResolutionContext) -> Self {
        Self {
            cwd: ctx.base_dir.clone(),
            home_dir: ctx.home_dir.clone(),
            env: ctx.env.clone(),
            repository_root: ctx.repository_root.clone(),
            package_root: ctx.package_root.clone(),
            package_area: ctx.package_area.clone(),
            allow_ambient_discovery: false,
        }
    }
}

/// Explicit, request-scoped inputs for document-backed file-reference
/// resolution.
///
/// Carries the filesystem anchors, environment snapshot, and configured
/// magic/vault roots a resolution needs so candidate construction never
/// rereads ambient process state (CWD, `$HOME`, environment, git root, or
/// package area) after the context is captured. Callers discover the trusted
/// worktree root (for example via `sniff::filesystem::git::repo_root`) and
/// supply it with [`with_repository_root`]; `biscuit-file` does not depend on
/// `sniff`, because `sniff` already depends on `biscuit-file`.
///
/// [`FileResolutionContext::new`] snapshots the ambient environment and home
/// directory once at construction; builder methods override individual
/// anchors.
///
/// ## Notes
///
/// `base_dir` is a directory: for a file-backed source, pass the source
/// file's parent. A supplied `repository_root` is accepted only when it
/// lexically contains the initial `base_dir` (see [`validate`]). Contexts
/// derived with [`for_source`](Self::for_source) must remain inside that root.
/// A caller that deliberately crosses into a configured external trust root
/// must use [`for_trusted_external_source`](Self::for_trusted_external_source)
/// or [`for_trusted_external_base`](Self::for_trusted_external_base).
///
/// [`with_repository_root`]: Self::with_repository_root
/// [`validate`]: Self::validate
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileResolutionContext {
    source_path: Option<PathBuf>,
    base_dir: PathBuf,
    repository_root: Option<PathBuf>,
    package_root: Option<PathBuf>,
    package_area: Option<PathBuf>,
    repository_scope_catalog: Option<RepositoryScopeCatalog>,
    home_dir: Option<PathBuf>,
    env: HashMap<String, String>,
    magic_paths: MagicPathList,
    vault_roots: Vec<PathBuf>,
    /// The request boundary whose containment must remain valid across every
    /// derivation, including explicitly trusted external ones.
    request_base_dir: PathBuf,
    /// Whether the current authoring base intentionally crosses the request
    /// repository boundary.
    trusted_external_authoring_base: bool,
}

impl FileResolutionContext {
    /// Create a context from request state captured by the caller.
    ///
    /// Unlike [`Self::new`], this constructor performs no ambient HOME or
    /// environment reads. It is intended for a request that must change its
    /// filesystem anchor after resolving a top-level source while retaining
    /// the original process-state snapshot.
    #[must_use]
    pub fn from_snapshot(
        base_dir: impl Into<PathBuf>,
        home_dir: Option<PathBuf>,
        env: HashMap<String, String>,
    ) -> Self {
        let base_dir = base_dir.into();
        Self {
            source_path: None,
            request_base_dir: base_dir.clone(),
            base_dir,
            repository_root: None,
            package_root: None,
            package_area: None,
            repository_scope_catalog: None,
            home_dir,
            env,
            magic_paths: MagicPathList::default(),
            vault_roots: Vec::new(),
            trusted_external_authoring_base: false,
        }
    }

    /// Create a context anchored at `base_dir`, snapshotting the ambient
    /// environment and home directory.
    ///
    /// `base_dir` should be an absolutized directory. Builder methods layer
    /// the remaining anchors on top.
    pub fn new(base_dir: impl Into<PathBuf>) -> Self {
        let base_dir = base_dir.into();
        Self {
            source_path: None,
            request_base_dir: base_dir.clone(),
            base_dir,
            repository_root: None,
            package_root: None,
            package_area: None,
            repository_scope_catalog: None,
            home_dir: home_dir(),
            env: std::env::vars().collect(),
            magic_paths: MagicPathList::default(),
            vault_roots: Vec::new(),
            trusted_external_authoring_base: false,
        }
    }

    /// Derive a document context from this request snapshot.
    ///
    /// Only the authoring source and its base directory change. Repository,
    /// package-area, home, environment, magic-root, and vault-root inputs are
    /// cloned from the captured request without reading process state or
    /// performing discovery.
    ///
    /// Both the request boundary and the derived source directory must remain
    /// inside the supplied repository root. Use
    /// [`for_trusted_external_source`](Self::for_trusted_external_source) only
    /// after another policy has accepted an external trust root.
    #[must_use]
    pub fn for_source(&self, source_path: impl Into<PathBuf>) -> Self {
        let source_path = source_path.into();
        let base_dir = source_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| self.base_dir.clone());
        let mut derived = self.clone();
        derived.source_path = Some(source_path);
        derived.base_dir = base_dir;
        derived.trusted_external_authoring_base = false;
        derived.recompute_repository_scopes();
        derived
    }

    /// Derive a document context across an explicitly accepted trust boundary.
    ///
    /// The originating request must still satisfy repository containment, but
    /// the external source directory may live outside that repository. This is
    /// intended for documents already accepted through a configured home,
    /// magic, or vault root; it does not establish a filesystem sandbox.
    #[must_use]
    pub fn for_trusted_external_source(&self, source_path: impl Into<PathBuf>) -> Self {
        let mut derived = self.for_source(source_path);
        derived.trusted_external_authoring_base = true;
        derived
    }

    /// Derive a context with a different authoring base but no source file.
    ///
    /// This is the in-memory-document counterpart to [`for_source`](Self::for_source).
    /// All request-scoped inputs remain unchanged and no ambient state is read.
    ///
    /// Both the request boundary and the new base must remain inside the
    /// supplied repository root.
    #[must_use]
    pub fn for_base(&self, base_dir: impl Into<PathBuf>) -> Self {
        let mut derived = self.clone();
        derived.source_path = None;
        derived.base_dir = base_dir.into();
        derived.trusted_external_authoring_base = false;
        derived.recompute_repository_scopes();
        derived
    }

    /// Derive an in-memory document base across an explicitly accepted trust
    /// boundary.
    ///
    /// The originating request must still satisfy repository containment. Use
    /// this only after another policy has accepted the external base.
    #[must_use]
    pub fn for_trusted_external_base(&self, base_dir: impl Into<PathBuf>) -> Self {
        let mut derived = self.for_base(base_dir);
        derived.trusted_external_authoring_base = true;
        derived
    }

    /// Record the source document/file that authored the references being
    /// resolved.
    #[must_use]
    pub fn with_source_path(mut self, source_path: impl Into<PathBuf>) -> Self {
        self.source_path = Some(source_path.into());
        self
    }

    /// Supply the trusted repository (worktree) root.
    #[must_use]
    pub fn with_repository_root(mut self, repository_root: impl Into<PathBuf>) -> Self {
        self.repository_root = Some(repository_root.into());
        self
    }

    /// Supply a captured repository topology and select scopes for this base.
    #[must_use]
    pub fn with_repository_scope_catalog(mut self, catalog: RepositoryScopeCatalog) -> Self {
        self.repository_scope_catalog = Some(catalog);
        self.recompute_repository_scopes();
        self
    }

    /// Supply the package root used by intrinsic magic and repository-scoped searches.
    #[must_use]
    pub fn with_package_root(mut self, package_root: impl Into<PathBuf>) -> Self {
        self.package_root = Some(package_root.into());
        self
    }

    /// Supply the package-area root.
    #[must_use]
    pub fn with_package_area(mut self, package_area: impl Into<PathBuf>) -> Self {
        self.package_area = Some(package_area.into());
        self
    }

    /// Override the home directory (otherwise snapshotted at construction).
    #[must_use]
    pub fn with_home_dir(mut self, home_dir: impl Into<PathBuf>) -> Self {
        self.home_dir = Some(home_dir.into());
        self
    }

    /// Clear the snapshotted home directory.
    ///
    /// Explicit request contexts use this when their authoritative discovery
    /// result has no home directory. Resolution must then report missing home
    /// context rather than falling back to the ambient value captured by
    /// [`new`](Self::new).
    #[must_use]
    pub fn without_home_dir(mut self) -> Self {
        self.home_dir = None;
        self
    }

    /// Replace the environment snapshot used for `{{VAR}}` interpolation.
    #[must_use]
    pub fn with_env(mut self, env: HashMap<String, String>) -> Self {
        self.env = env;
        self
    }

    /// Add a magic (`@`) search root at the given position.
    #[must_use]
    pub fn add_magic_path(mut self, path: impl Into<PathBuf>, position: super::PathPosition) -> Self {
        match position {
            super::PathPosition::Start => self.magic_paths.prepend.push(path.into()),
            super::PathPosition::End => self.magic_paths.append.push(path.into()),
        }
        self
    }

    /// Add a vault root for `vault:` references.
    #[must_use]
    pub fn add_vault(mut self, path: impl Into<PathBuf>) -> Self {
        self.vault_roots.push(path.into());
        self
    }

    /// The source document/file, when one was supplied.
    pub fn source_path(&self) -> Option<&Path> {
        self.source_path.as_deref()
    }

    /// The base directory references resolve against.
    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    /// The supplied repository (worktree) root, when one was supplied.
    pub fn repository_root(&self) -> Option<&Path> {
        self.repository_root.as_deref()
    }

    /// The selected package root, when the reference base is inside one.
    pub fn package_root(&self) -> Option<&Path> {
        self.package_root.as_deref()
    }

    /// The supplied package-area root, when one was supplied.
    pub fn package_area(&self) -> Option<&Path> {
        self.package_area.as_deref()
    }

    /// The home directory used for `~` references.
    pub fn home_dir(&self) -> Option<&Path> {
        self.home_dir.as_deref()
    }

    /// The captured environment used for interpolation and vault roots.
    pub fn env(&self) -> &HashMap<String, String> {
        &self.env
    }

    /// The original request directory whose repository containment is retained
    /// across derived document contexts.
    pub fn request_base_dir(&self) -> &Path {
        &self.request_base_dir
    }

    /// Whether this context intentionally crosses the request repository
    /// boundary for an externally trusted authoring source.
    pub fn is_trusted_external_authoring_base(&self) -> bool {
        self.trusted_external_authoring_base
    }

    /// Magic roots searched before repository and home defaults.
    pub fn prepended_magic_paths(&self) -> &[PathBuf] {
        &self.magic_paths.prepend
    }

    /// Magic roots searched after repository and home defaults.
    pub fn appended_magic_paths(&self) -> &[PathBuf] {
        &self.magic_paths.append
    }

    /// Explicit roots searched for `vault:` references.
    pub fn vault_roots(&self) -> &[PathBuf] {
        &self.vault_roots
    }

    pub(crate) fn magic_paths(&self) -> &MagicPathList {
        &self.magic_paths
    }

    fn recompute_repository_scopes(&mut self) {
        let Some(catalog) = &self.repository_scope_catalog else {
            return;
        };
        let scope = catalog.scope_for(&self.base_dir);
        self.repository_root = scope.repository_root;
        self.package_area = scope.package_area_root;
        self.package_root = scope.package_root;
    }

    /// Validate repository containment for the request and authoring bases.
    ///
    /// Containment is component-aware and lexical after `.`/`..` normalization
    /// and Windows verbatim-prefix reduction, so a root and a base that name the
    /// same tree in different spellings still validate. It does **not**
    /// canonicalize through symlinks, so the authored/worktree identity is
    /// preserved. This is a trust check on the caller-provided root, not a
    /// sandbox boundary. When no repository root is supplied, the context is
    /// trivially valid.
    ///
    /// Normal derivations must keep their authoring base inside the repository.
    /// Trusted-external derivations exempt only the current authoring base; the
    /// originating request boundary is always checked, so derivation cannot
    /// make an invalid request snapshot valid.
    ///
    /// ## Errors
    ///
    /// Returns [`FileReferenceError::RepositoryRootNotContainingSource`] when a
    /// repository root is supplied but does not contain a required base.
    pub fn validate(&self) -> Result<(), FileReferenceError> {
        if self.repository_scope_catalog.is_some() {
            if let Some(repo) = &self.repository_root
                && !normalize_components(&self.base_dir).starts_with(normalize_components(repo))
            {
                return Err(FileReferenceError::RepositoryRootNotContainingSource {
                    repository_root: repo.clone(),
                    source_path: self.base_dir.clone(),
                });
            }
            return Ok(());
        }
        if let Some(repo) = &self.repository_root {
            let repo_norm = normalize_components(repo);
            let required_bases = if self.trusted_external_authoring_base {
                [Some(&self.request_base_dir), None]
            } else {
                [Some(&self.request_base_dir), Some(&self.base_dir)]
            };
            for base in required_bases.into_iter().flatten() {
                if !normalize_components(base).starts_with(&repo_norm) {
                    return Err(FileReferenceError::RepositoryRootNotContainingSource {
                        repository_root: repo.clone(),
                        source_path: base.clone(),
                    });
                }
            }
        }
        Ok(())
    }
}

/// Find the git repository root starting from `from`.
///
/// Returns `Ok(None)` if no git repository is found.
pub fn find_git_root(from: &Path) -> Result<Option<PathBuf>, FileReferenceError> {
    use gix::discover::upwards::Error as Up;
    trace!(?from, "searching for git root");
    match gix::discover(from) {
        Ok(repo) => {
            let workdir = repo
                .workdir()
                .ok_or(FileReferenceError::BareRepository)?;
            debug!(?workdir, "found git root");
            Ok(Some(workdir.to_path_buf()))
        }
        // Upward-search exhaustion is the only "not a repository" outcome;
        // trust, permission, and corruption failures propagate as errors.
        Err(gix::discover::Error::Discover(
            Up::NoGitRepository { .. }
            | Up::NoGitRepositoryWithinCeiling { .. }
            | Up::NoGitRepositoryWithinFs { .. },
        )) => {
            trace!("no git repository found");
            Ok(None)
        }
        Err(e) => Err(FileReferenceError::Git(Box::new(e))),
    }
}

/// Get the user's home directory from the cross-platform provider.
///
/// On POSIX this honors `$HOME` (with a passwd fallback); on native Windows it
/// resolves the profile directory through the OS known-folder API rather than
/// the frequently-unset `HOME` variable, so `~` and the HOME leg of magic
/// search stay valid there (D11).
pub fn home_dir() -> Option<PathBuf> {
    dirs::home_dir()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_ambient_succeeds() {
        let ctx = ResolutionContext::from_ambient().unwrap();
        assert!(ctx.cwd.is_absolute());
    }

    #[test]
    fn from_base_absolute_path_is_preserved() {
        // Windows has no drive-less absolute path: `/tmp` is *rooted* there but
        // `is_absolute()` is false, so the literal must be selected per platform.
        #[cfg(windows)]
        let abs = Path::new(r"C:\tmp");
        #[cfg(not(windows))]
        let abs = Path::new("/tmp");

        let ctx = ResolutionContext::from_base(abs).unwrap();
        assert_eq!(ctx.cwd, abs);
    }

    #[test]
    fn from_base_relative_path_is_joined_to_ambient_cwd() {
        let ctx = ResolutionContext::from_base(Path::new("sub/dir")).unwrap();
        assert!(ctx.cwd.is_absolute());
        assert!(ctx.cwd.ends_with("sub/dir"));
    }

    #[test]
    fn find_git_root_inside_repo() {
        // We're inside the rusty-biscuit repo
        let root = find_git_root(&std::env::current_dir().unwrap()).unwrap();
        assert!(root.is_some());
    }

    #[test]
    fn find_git_root_outside_repo() {
        // The OS temp directory is the portable stand-in for a real directory
        // that is unlikely to sit inside a git repository. A POSIX `/tmp`
        // literal does not exist on Windows, where discovery would fail on an
        // inaccessible directory rather than report "no repository".
        let root = find_git_root(&std::env::temp_dir()).unwrap();
        assert!(root.is_none());
    }
}
