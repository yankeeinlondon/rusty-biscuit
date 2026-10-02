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

/// The immutable, request-scoped launch scope `@` (magic) references search.
///
/// Captured once from the directory a request was launched from: that
/// directory plus the repository, package, and package-area roots selected
/// for it. Derivations ([`for_source`](FileResolutionContext::for_source),
/// [`for_cwd`](FileResolutionContext::for_cwd), and the trusted-external
/// forms) preserve this snapshot unchanged, so a nested `@` reference keeps
/// searching the launch tree even when the authoring source lives in another
/// repository or an external prompt directory, while `./`, bare, `&`, and `^`
/// references keep their source-specific anchors.
///
/// The [`local_root`](Self::local_root) — the repository root when one was
/// selected for the request directory, else the request directory itself —
/// anchors the local tier of the `@` root chain.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchMagicScope {
    request_dir: PathBuf,
    repository_root: Option<PathBuf>,
    package_root: Option<PathBuf>,
    package_area: Option<PathBuf>,
}

impl LaunchMagicScope {
    /// The request directory the scope was captured from.
    pub fn request_dir(&self) -> &Path {
        &self.request_dir
    }

    /// The repository root selected for the request directory, when one was.
    pub fn repository_root(&self) -> Option<&Path> {
        self.repository_root.as_deref()
    }

    /// The package root selected for the request directory, when one was.
    pub fn package_root(&self) -> Option<&Path> {
        self.package_root.as_deref()
    }

    /// The package-area root selected for the request directory, when one was.
    pub fn package_area(&self) -> Option<&Path> {
        self.package_area.as_deref()
    }

    /// The root of the local tree for `@` references: the launch repository
    /// root when one exists, otherwise the request directory itself.
    pub fn local_root(&self) -> &Path {
        self.repository_root.as_deref().unwrap_or(&self.request_dir)
    }
}

/// One configured magic (`@`) root registration, as supplied to
/// [`add_magic_path`](FileResolutionContext::add_magic_path) or
/// [`add_magic_path_with_tier`](FileResolutionContext::add_magic_path_with_tier).
///
/// Exposed so cache and graph identity can encode exactly what a context
/// registered — including its tier policy, which can change the winning
/// candidate without changing any other context field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MagicPathRegistration {
    path: PathBuf,
    position: super::PathPosition,
    tier: super::MagicPathTier,
}

impl MagicPathRegistration {
    /// The registered root, exactly as supplied (possibly relative).
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The registered position.
    pub fn position(&self) -> super::PathPosition {
        self.position
    }

    /// The registered tier policy.
    pub fn tier(&self) -> super::MagicPathTier {
        self.tier
    }
}

/// Where a [`FileResolutionContext`]'s tree root
/// ([`base_dir`](FileResolutionContext::base_dir)) came from.
///
/// Every origin except [`Fallback`](Self::Fallback) is a boundary: a relative
/// reference may not leave that tree unless the reader opted in with
/// [`allow_external_relative`](FileResolutionContext::allow_external_relative).
/// A caller must read the origin rather than compare `base_dir` with `cwd`,
/// because an explicit root can equal `cwd` and still enforce containment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BaseDirOrigin {
    /// The supplied repository root (directly or through a scope catalog).
    Repository,
    /// A root supplied with [`with_base_dir`](FileResolutionContext::with_base_dir).
    Explicit,
    /// The deepest configured vault root containing `cwd`.
    Vault,
    /// The captured home directory of a `~`-anchored opening reference.
    Home,
    /// The captured value of the `{{name}}`-anchored opening reference.
    Environment { name: String },
    /// Nothing named the tree, so `base_dir` is `cwd` and enforces no boundary.
    Fallback,
}

impl BaseDirOrigin {
    /// Whether a tree with this origin rejects relative references that leave it.
    pub fn is_boundary(&self) -> bool {
        !matches!(self, Self::Fallback)
    }
}

/// The role of a [`FileResolutionContext`] directory that must be absolute,
/// reported by [`FileReferenceError::RelativeContextDirectory`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContextAnchor {
    /// The directory the request was captured in
    /// ([`request_cwd`](FileResolutionContext::request_cwd)), or the request
    /// directory of the launch `@` scope
    /// ([`LaunchMagicScope::request_dir`]).
    RequestDirectory,
    /// The current authoring directory ([`cwd`](FileResolutionContext::cwd)).
    WorkingDirectory,
    /// A root supplied with [`with_base_dir`](FileResolutionContext::with_base_dir).
    BaseDir,
    RepositoryRoot,
    PackageRoot,
    PackageArea,
    /// The captured home directory.
    HomeDir,
}

impl std::fmt::Display for ContextAnchor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::RequestDirectory => "request directory",
            Self::WorkingDirectory => "working directory",
            Self::BaseDir => "base directory",
            Self::RepositoryRoot => "repository root",
            Self::PackageRoot => "package root",
            Self::PackageArea => "package area",
            Self::HomeDir => "home directory",
        })
    }
}

/// A selected tree root and its origin.
#[derive(Debug, Clone, PartialEq, Eq)]
struct TreeRoot {
    path: PathBuf,
    origin: BaseDirOrigin,
}

impl TreeRoot {
    fn fallback(cwd: &Path) -> Self {
        Self {
            path: cwd.to_path_buf(),
            origin: BaseDirOrigin::Fallback,
        }
    }

    /// Lexical, component-aware containment of `dir` in a boundary tree.
    /// A fallback tree contains everything.
    fn contains(&self, dir: &Path) -> bool {
        !self.origin.is_boundary()
            || normalize_components(dir).starts_with(normalize_components(&self.path))
    }

    /// The containment failure for a `dir` this tree does not contain.
    fn not_containing(&self, dir: &Path) -> ContextFailure {
        match self.origin {
            BaseDirOrigin::Repository => ContextFailure::RepositoryRootNotContainingSource {
                repository_root: self.path.clone(),
                source_path: dir.to_path_buf(),
            },
            _ => ContextFailure::CwdOutsideBaseDir {
                base_dir: self.path.clone(),
                cwd: dir.to_path_buf(),
            },
        }
    }
}

/// A [`FileResolutionContext::validate`] failure, kept in a cloneable form so
/// a derived context can carry the failure of the context it came from.
/// Each variant converts to the [`FileReferenceError`] variant of the same
/// name.
#[derive(Debug, Clone, PartialEq, Eq)]
enum ContextFailure {
    RelativeContextDirectory {
        anchor: ContextAnchor,
        path: PathBuf,
    },
    BaseDirNotRepositoryRoot {
        base_dir: PathBuf,
        repository_root: PathBuf,
    },
    RepositoryRootNotContainingSource {
        repository_root: PathBuf,
        source_path: PathBuf,
    },
    CwdOutsideBaseDir {
        base_dir: PathBuf,
        cwd: PathBuf,
    },
}

impl From<ContextFailure> for FileReferenceError {
    fn from(failure: ContextFailure) -> Self {
        match failure {
            ContextFailure::RelativeContextDirectory { anchor, path } => {
                Self::RelativeContextDirectory { anchor, path }
            }
            ContextFailure::BaseDirNotRepositoryRoot {
                base_dir,
                repository_root,
            } => Self::BaseDirNotRepositoryRoot {
                base_dir,
                repository_root,
            },
            ContextFailure::RepositoryRootNotContainingSource {
                repository_root,
                source_path,
            } => Self::RepositoryRootNotContainingSource {
                repository_root,
                source_path,
            },
            ContextFailure::CwdOutsideBaseDir { base_dir, cwd } => {
                Self::CwdOutsideBaseDir { base_dir, cwd }
            }
        }
    }
}

/// The tree root an opening reference's `~` or leading `{{VAR}}` anchor
/// supplies, when it is a captured absolute directory containing
/// `resolved_source`.
///
/// Containment is lexical (ruling R6), which preserves the authored identity
/// of the anchor. A relative, unset, or foreign-host value supplies nothing.
fn opening_anchor(
    reference: &super::FileReference,
    resolved_source: &Path,
    home_dir: Option<&Path>,
    env: &HashMap<String, String>,
) -> Option<TreeRoot> {
    use super::{ReferenceKind, TemplateSegment};

    let (path, origin) = match &reference.parsed.kind {
        ReferenceKind::Home(_) => (home_dir?.to_path_buf(), BaseDirOrigin::Home),
        // A leading `{{VAR}}` always parses as implicit relative.
        ReferenceKind::ImplicitRelative(template) => match template.segments.first()? {
            TemplateSegment::EnvVar(name) => (
                PathBuf::from(env.get(name)?),
                BaseDirOrigin::Environment { name: name.clone() },
            ),
            TemplateSegment::Literal(_) => return None,
        },
        _ => return None,
    };
    if !path.is_absolute() {
        return None;
    }
    let path = normalize_components(&path);
    normalize_components(resolved_source)
        .starts_with(&path)
        .then_some(TreeRoot { path, origin })
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
    /// The immutable launch `@` scope captured by the originating
    /// [`FileResolutionContext`]. `None` for the ambient compatibility
    /// methods, whose live anchors *are* the launch scope. `@` resolution
    /// reads the local-root anchors from here rather than from the
    /// source-derived anchors above.
    pub launch_magic_scope: Option<LaunchMagicScope>,
    /// The tree root relative references must stay inside, as written and
    /// where they land. `None` when no boundary applies: the ambient
    /// compatibility methods, a fallback tree root, or a reader that opted in
    /// with [`FileResolutionContext::allow_external_relative`].
    pub relative_boundary: Option<PathBuf>,
}

impl ResolutionContext {
    /// Build from live process state.
    pub fn from_ambient() -> Result<Self, FileReferenceError> {
        let cwd = std::env::current_dir().map_err(FileReferenceError::CurrentDirectory)?;
        let home_dir = home_dir();
        let env = capture_env();

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
            launch_magic_scope: None,
            relative_boundary: None,
        })
    }

    /// Build a context that treats `cwd` as the working directory, while
    /// still reading HOME and environment variables from the live process
    /// state.
    ///
    /// If `cwd` is a relative path, it is joined onto the ambient CWD so
    /// that repository discovery always operates on an absolute location.
    pub fn from_cwd(cwd: &Path) -> Result<Self, FileReferenceError> {
        let cwd = if cwd.is_absolute() {
            cwd.to_path_buf()
        } else {
            let ambient = std::env::current_dir().map_err(FileReferenceError::CurrentDirectory)?;
            ambient.join(cwd)
        };
        let home_dir = home_dir();
        let env = capture_env();

        Ok(Self {
            cwd,
            home_dir,
            env,
            repository_root: None,
            package_root: None,
            package_area: None,
            allow_ambient_discovery: true,
            launch_magic_scope: None,
            relative_boundary: None,
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
            cwd: ctx.cwd.clone(),
            home_dir: ctx.home_dir.clone(),
            env: ctx.env.clone(),
            repository_root: ctx.repository_root.clone(),
            package_root: ctx.package_root.clone(),
            package_area: ctx.package_area.clone(),
            allow_ambient_discovery: false,
            launch_magic_scope: Some(ctx.launch_magic_scope.clone()),
            relative_boundary: (ctx.base_dir_is_boundary() && !ctx.allow_external_relative)
                .then(|| ctx.tree.path.clone()),
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
/// `cwd` is a directory: for a file-backed source, pass the source
/// file's parent. It is where `./`, `../`, and bare references start.
///
/// [`base_dir`](Self::base_dir) is the root of the whole file tree and the
/// boundary relative references must stay inside. It is selected, in order,
/// from the supplied repository root, an explicit
/// [`with_base_dir`](Self::with_base_dir), the deepest configured vault
/// containing `cwd`, the `~`/`{{VAR}}` anchor of the reference that opened the
/// document ([`for_source_reference`](Self::for_source_reference)), and
/// finally `cwd` itself, which is a [`BaseDirOrigin::Fallback`] and enforces
/// no boundary. `cwd` must stay inside `base_dir`, and every directory and
/// tree anchor must be absolute (see [`validate`]). Builders and derivations
/// are infallible; an invalid context fails `validate`, which every resolver
/// entry point and [`PortablePath`](super::PortablePath) run first. A derived
/// context keeps the failure of the context it was derived from, so deriving
/// a document never turns an invalid request into a valid one.
///
/// [`for_source`](Self::for_source) and [`for_cwd`](Self::for_cwd) keep the
/// tree: a derived document must remain inside it. A caller that deliberately
/// crosses into a configured external trust root must use
/// [`for_trusted_external_source`](Self::for_trusted_external_source) or
/// [`for_trusted_external_cwd`](Self::for_trusted_external_cwd), which select
/// a new tree for the external document.
///
/// The context also captures an immutable launch `@` scope (see
/// [`LaunchMagicScope`]): the construction directory plus the repository,
/// package, and package-area roots selected for it. `@` resolution anchors on
/// that snapshot across every derivation, so a nested `@` reference keeps
/// searching the launch tree; the direct builder methods
/// ([`with_repository_root`](Self::with_repository_root),
/// [`with_repository_scope_catalog`](Self::with_repository_scope_catalog),
/// [`with_package_root`](Self::with_package_root),
/// [`with_package_area`](Self::with_package_area)) update it, while
/// derivations never do.
///
/// [`with_repository_root`]: Self::with_repository_root
/// [`validate`]: Self::validate
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileResolutionContext {
    source_path: Option<PathBuf>,
    cwd: PathBuf,
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
    request_cwd: PathBuf,
    /// Whether the current authoring `cwd` intentionally crosses the request
    /// repository boundary.
    trusted_external_authoring_cwd: bool,
    /// The immutable launch `@` scope; see the struct documentation.
    launch_magic_scope: LaunchMagicScope,
    explicit_base_dir: Option<PathBuf>,
    /// The `~`/`{{VAR}}` tree root supplied by the reference that opened the
    /// current document, kept so a later builder call can reselect the tree.
    opening_anchor: Option<TreeRoot>,
    /// The selected tree root for `cwd`.
    tree: TreeRoot,
    /// The tree selected for `request_cwd`. Builders keep it in step with
    /// `tree` until the first derivation; after that it is frozen, so the
    /// original request is validated against its own tree independently of
    /// whatever tree a derived document selected.
    request_tree: TreeRoot,
    /// The validation failure of the context this one was derived from,
    /// captured before derivation discarded or recomputed any of its
    /// settings. Builders never clear it.
    inherited_failure: Option<ContextFailure>,
    derived: bool,
    allow_external_relative: bool,
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
        cwd: impl Into<PathBuf>,
        home_dir: Option<PathBuf>,
        env: HashMap<String, String>,
    ) -> Self {
        let cwd = cwd.into();
        let launch_magic_scope = LaunchMagicScope {
            request_dir: cwd.clone(),
            repository_root: None,
            package_root: None,
            package_area: None,
        };
        let tree = TreeRoot::fallback(&cwd);
        let mut context = Self {
            source_path: None,
            request_cwd: cwd.clone(),
            cwd,
            repository_root: None,
            package_root: None,
            package_area: None,
            repository_scope_catalog: None,
            home_dir,
            env,
            magic_paths: MagicPathList::default(),
            vault_roots: Vec::new(),
            trusted_external_authoring_cwd: false,
            launch_magic_scope,
            explicit_base_dir: None,
            opening_anchor: None,
            request_tree: tree.clone(),
            tree,
            inherited_failure: None,
            derived: false,
            allow_external_relative: false,
        };
        // A captured `VAULT` value can already contain `cwd`.
        context.reselect_tree();
        context
    }

    /// Create a context anchored at `cwd`, snapshotting the ambient
    /// environment and home directory.
    ///
    /// `cwd` must be an absolute directory, or [`validate`](Self::validate)
    /// fails. Builder methods layer the remaining anchors on top.
    pub fn new(cwd: impl Into<PathBuf>) -> Self {
        Self::from_snapshot(cwd, home_dir(), capture_env())
    }

    /// Derive a document context from this request snapshot.
    ///
    /// Only the authoring source and its `cwd` change. Repository,
    /// package-area, home, environment, magic-root, and vault-root inputs are
    /// cloned from the captured request without reading process state or
    /// performing discovery. The tree root, its origin, and the reader opt-in
    /// carry over unchanged. The launch `@` scope
    /// ([`launch_magic_scope`](Self::launch_magic_scope)) is likewise
    /// preserved verbatim: a nested `@` reference keeps searching the launch
    /// tree while the source-relative kinds re-anchor on the new `cwd`.
    ///
    /// Both the request boundary and the derived source directory must remain
    /// inside the tree root. Use
    /// [`for_trusted_external_source`](Self::for_trusted_external_source) only
    /// after another policy has accepted an external trust root. When the tree
    /// root is a [`BaseDirOrigin::Fallback`] there is no tree to keep, so one
    /// is selected for the new `cwd` (a containing vault, else `cwd`).
    #[must_use]
    pub fn for_source(&self, source_path: impl Into<PathBuf>) -> Self {
        let source_path = source_path.into();
        let cwd = self.source_cwd(&source_path);
        self.derive(Some(source_path), cwd, None, false)
    }

    /// Derive a document context across an explicitly accepted trust boundary.
    ///
    /// The originating request must still satisfy its own containment, but
    /// the external source directory may live outside the current tree. This
    /// is intended for documents already accepted through a configured home,
    /// magic, or vault root; it does not establish a filesystem sandbox.
    ///
    /// When the source is still inside the current tree this behaves like
    /// [`for_source`](Self::for_source). Otherwise it selects a new tree: the
    /// repository a scope catalog assigns to the source, else a containing
    /// vault, else the new `cwd`. Repository, package, and package-area
    /// anchors no catalog assigns are dropped, an explicit
    /// [`with_base_dir`](Self::with_base_dir) of the originating tree does not
    /// carry over, and no repository is discovered. The launch `@` scope is
    /// unchanged. A validation failure of this context is kept regardless (see
    /// [`validate`](Self::validate)).
    #[must_use]
    pub fn for_trusted_external_source(&self, source_path: impl Into<PathBuf>) -> Self {
        let source_path = source_path.into();
        let cwd = self.source_cwd(&source_path);
        self.derive(Some(source_path), cwd, None, true)
    }

    /// Derive a document context, keeping the `~` or leading `{{VAR}}` anchor
    /// of the reference that opened it.
    ///
    /// `resolved_source` must already have been accepted by the caller using
    /// this snapshot; this does not re-resolve `reference` or grant permission
    /// to open the file. Behaves like [`for_source`](Self::for_source), except
    /// that when this context has no tree to keep
    /// ([`BaseDirOrigin::Fallback`]) the anchor can become the tree root. The
    /// anchor qualifies only when it is a captured absolute home directory or
    /// environment value that lexically contains `resolved_source`. An anchor
    /// never replaces a tree that already contains the document.
    #[must_use]
    pub fn for_source_reference(
        &self,
        reference: &super::FileReference,
        resolved_source: impl Into<PathBuf>,
    ) -> Self {
        let source_path = resolved_source.into();
        let cwd = self.source_cwd(&source_path);
        let anchor = opening_anchor(reference, &source_path, self.home_dir(), &self.env);
        self.derive(Some(source_path), cwd, anchor, false)
    }

    /// The trusted-external counterpart of
    /// [`for_source_reference`](Self::for_source_reference).
    ///
    /// Behaves like
    /// [`for_trusted_external_source`](Self::for_trusted_external_source),
    /// with the qualifying opening anchor ranked after a catalog repository
    /// and a containing vault when the new tree is selected.
    #[must_use]
    pub fn for_trusted_external_source_reference(
        &self,
        reference: &super::FileReference,
        resolved_source: impl Into<PathBuf>,
    ) -> Self {
        let source_path = resolved_source.into();
        let cwd = self.source_cwd(&source_path);
        let anchor = opening_anchor(reference, &source_path, self.home_dir(), &self.env);
        self.derive(Some(source_path), cwd, anchor, true)
    }

    /// Derive a context with a different authoring `cwd` but no source file.
    ///
    /// This is the in-memory-document counterpart to [`for_source`](Self::for_source),
    /// with the same tree rules. All request-scoped inputs remain unchanged
    /// and no ambient state is read.
    #[must_use]
    pub fn for_cwd(&self, cwd: impl Into<PathBuf>) -> Self {
        self.derive(None, cwd.into(), None, false)
    }

    /// Derive an in-memory document `cwd` across an explicitly accepted trust
    /// boundary.
    ///
    /// The in-memory counterpart to
    /// [`for_trusted_external_source`](Self::for_trusted_external_source), with
    /// the same tree rules. Use this only after another policy has accepted
    /// the external directory.
    #[must_use]
    pub fn for_trusted_external_cwd(&self, cwd: impl Into<PathBuf>) -> Self {
        self.derive(None, cwd.into(), None, true)
    }

    fn source_cwd(&self, source_path: &Path) -> PathBuf {
        source_path
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| self.cwd.clone())
    }

    /// The one derivation rule behind every `for_*` method.
    fn derive(
        &self,
        source_path: Option<PathBuf>,
        cwd: PathBuf,
        anchor: Option<TreeRoot>,
        trusted: bool,
    ) -> Self {
        let mut derived = self.clone();
        // Captured first: the steps below can clear the explicit root and
        // replace the repository and package anchors this check reads.
        derived.inherited_failure = self.check().err();
        derived.source_path = source_path;
        derived.cwd = cwd;
        derived.trusted_external_authoring_cwd = trusted;
        derived.derived = true;
        let keeps_tree = self.tree.origin.is_boundary();
        if keeps_tree && (!trusted || self.tree.contains(&derived.cwd)) {
            // A normal derivation keeps the tree even when the new `cwd` left
            // it, so `validate` reports the escape instead of a new tree
            // silently absorbing it.
            derived.recompute_repository_scopes();
            return derived;
        }
        if trusted {
            derived.explicit_base_dir = None;
            derived.opening_anchor = anchor;
            if derived.repository_scope_catalog.is_some() {
                derived.recompute_repository_scopes();
            } else {
                derived.repository_root = None;
                derived.package_root = None;
                derived.package_area = None;
            }
        } else {
            derived.recompute_repository_scopes();
            if anchor.is_some() {
                derived.opening_anchor = anchor;
            }
        }
        derived.tree = derived.select_tree();
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
    ///
    /// Updates both the current repository anchor and the launch `@` scope's
    /// repository root.
    #[must_use]
    pub fn with_repository_root(mut self, repository_root: impl Into<PathBuf>) -> Self {
        self.repository_root = Some(repository_root.into());
        self.sync_launch_magic_scope();
        self.reselect_tree();
        self
    }

    /// Supply a captured repository topology and select scopes for this base.
    ///
    /// The selected repository, package-area, and package roots become both
    /// the current anchors and the launch `@` scope's anchors.
    #[must_use]
    pub fn with_repository_scope_catalog(mut self, catalog: RepositoryScopeCatalog) -> Self {
        self.repository_scope_catalog = Some(catalog);
        self.recompute_repository_scopes();
        self.sync_launch_magic_scope();
        self.reselect_tree();
        self
    }

    /// Supply the root of the file tree outside a repository.
    ///
    /// An explicit root outranks a containing vault and an opening-reference
    /// anchor, and it is a boundary even when it equals `cwd`. Inside a
    /// repository the tree root is always the repository root: a `dir` equal
    /// to it is accepted, and any other makes [`validate`](Self::validate)
    /// fail with [`FileReferenceError::BaseDirNotRepositoryRoot`].
    #[must_use]
    pub fn with_base_dir(mut self, dir: impl Into<PathBuf>) -> Self {
        self.explicit_base_dir = Some(dir.into());
        self.reselect_tree();
        self
    }

    /// Let relative references resolve to targets outside the tree root,
    /// as written or through a symlink.
    ///
    /// Off by default and copied to every derived context. It does not exempt
    /// an invalid request or document `cwd`, authorize file access, or relax
    /// the repository-only `&` and `^` sigils.
    #[must_use]
    pub fn allow_external_relative(mut self) -> Self {
        self.allow_external_relative = true;
        self
    }

    /// Supply the package root used by intrinsic magic and repository-scoped searches.
    ///
    /// Updates both the current package anchor and the launch `@` scope's
    /// package root.
    #[must_use]
    pub fn with_package_root(mut self, package_root: impl Into<PathBuf>) -> Self {
        self.package_root = Some(package_root.into());
        self.sync_launch_magic_scope();
        self
    }

    /// Supply the package-area root.
    ///
    /// Updates both the current package-area anchor and the launch `@`
    /// scope's package-area root.
    #[must_use]
    pub fn with_package_area(mut self, package_area: impl Into<PathBuf>) -> Self {
        self.package_area = Some(package_area.into());
        self.sync_launch_magic_scope();
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
        // The `VAULT` entry takes part in tree-root selection.
        self.reselect_tree();
        self
    }

    /// Add a magic (`@`) search root at the given position, with its tier
    /// inferred from containment in the local root.
    ///
    /// Tier is decided at resolution time against the launch `@` scope's
    /// local root (see [`LaunchMagicScope::local_root`]): a configured root
    /// that lexically lies inside it — compared after `.`/`..` and Windows
    /// verbatim normalization — joins the local tier searched before every
    /// home-based root; every other root (including one under `$HOME` but
    /// outside the local tree, and one outside both trees such as
    /// `/opt/configs`) joins the user tier searched after the local tier.
    /// Registration order is preserved within each tier position. Use
    /// [`add_magic_path_with_tier`](Self::add_magic_path_with_tier) when a
    /// caller knows a root is a user convention regardless of layout.
    ///
    /// A relative root is interpreted against the captured request directory
    /// (the launch `@` scope's `request_dir`), never against the process
    /// working directory: the joined candidate and the tier test use that
    /// same absolute spelling. For the ambient `resolve_from(cwd)` form the
    /// captured request directory is `cwd`, so a relative root follows
    /// `cwd` rather than the process working directory.
    #[must_use]
    pub fn add_magic_path(self, path: impl Into<PathBuf>, position: super::PathPosition) -> Self {
        self.add_configured_root(path, position, super::MagicPathTier::Inferred)
    }

    /// Add a magic (`@`) search root with an explicit tier policy.
    ///
    /// [`super::MagicPathTier::User`] forces the user tier regardless of where the
    /// root lies, winning over containment inference in every layout —
    /// including a launch directory equal to `$HOME` and `$HOME` itself being
    /// a repository, where a path-only rule cannot tell a user convention
    /// from a local one. [`super::MagicPathTier::Inferred`] behaves exactly like
    /// [`add_magic_path`](Self::add_magic_path).
    #[must_use]
    pub fn add_magic_path_with_tier(
        self,
        path: impl Into<PathBuf>,
        position: super::PathPosition,
        tier: super::MagicPathTier,
    ) -> Self {
        self.add_configured_root(path, position, tier)
    }

    fn add_configured_root(
        mut self,
        path: impl Into<PathBuf>,
        position: super::PathPosition,
        tier: super::MagicPathTier,
    ) -> Self {
        let entry = (path.into(), tier);
        match position {
            super::PathPosition::Start => self.magic_paths.prepend.push(entry),
            super::PathPosition::End => self.magic_paths.append.push(entry),
        }
        self
    }

    /// Add a vault root for `vault:` references.
    #[must_use]
    pub fn add_vault(mut self, path: impl Into<PathBuf>) -> Self {
        self.vault_roots.push(path.into());
        self.reselect_tree();
        self
    }

    /// The source document/file, when one was supplied.
    pub fn source_path(&self) -> Option<&Path> {
        self.source_path.as_deref()
    }

    /// The working directory relative references resolve against.
    pub fn cwd(&self) -> &Path {
        &self.cwd
    }

    /// The root of the file tree: the boundary relative references stay
    /// inside. Equal to [`repository_root`](Self::repository_root) inside a
    /// repository; see the struct notes for how it is selected elsewhere.
    pub fn base_dir(&self) -> &Path {
        &self.tree.path
    }

    /// Where [`base_dir`](Self::base_dir) came from.
    pub fn base_dir_origin(&self) -> &BaseDirOrigin {
        &self.tree.origin
    }

    /// Whether [`base_dir`](Self::base_dir) rejects relative references that
    /// leave it; `false` only for a [`BaseDirOrigin::Fallback`].
    pub fn base_dir_is_boundary(&self) -> bool {
        self.tree.origin.is_boundary()
    }

    /// Whether [`allow_external_relative`](Self::allow_external_relative)
    /// was set on this context or one it was derived from.
    pub fn external_relative_allowed(&self) -> bool {
        self.allow_external_relative
    }

    /// The supplied repository (worktree) root, when one was supplied.
    pub fn repository_root(&self) -> Option<&Path> {
        self.repository_root.as_deref()
    }

    /// The selected package root, when `cwd` is inside one.
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
    pub fn request_cwd(&self) -> &Path {
        &self.request_cwd
    }

    /// Whether this context intentionally crosses the request repository
    /// boundary for an externally trusted authoring source.
    pub fn is_trusted_external_authoring_cwd(&self) -> bool {
        self.trusted_external_authoring_cwd
    }

    /// The immutable launch `@` scope captured at construction: the request
    /// directory plus the repository, package, and package-area roots
    /// selected for it.
    ///
    /// `@` resolution, recursive `%@` traversal, and `@` completion all read
    /// their local-root anchors from this snapshot, which derivations never
    /// rewrite. It is also a cache-identity input: two contexts differing
    /// only in this scope can resolve the same `@` reference to different
    /// files.
    pub fn launch_magic_scope(&self) -> &LaunchMagicScope {
        &self.launch_magic_scope
    }

    /// Replace the launch `@` scope with an explicitly captured one.
    ///
    /// Every directory the scope carries, its request directory included,
    /// must be absolute or [`validate`](Self::validate) fails.
    ///
    /// For requests that rebuild their resolution context around a source in
    /// another repository or an external prompt directory: the context's own
    /// anchors may then be re-anchored on that source (giving `./`, bare,
    /// `&`, and `^` references their source-specific meanings) while `@`
    /// keeps searching the invocation's launch tree. The natural source is
    /// [`launch_magic_scope`](Self::launch_magic_scope) on the launch
    /// context, captured before derivation.
    #[must_use]
    pub fn with_launch_magic_scope(mut self, scope: LaunchMagicScope) -> Self {
        self.launch_magic_scope = scope;
        self
    }

    /// Magic roots registered at [`PathPosition::Start`](super::PathPosition::Start),
    /// in registration order.
    ///
    /// Roots are reported exactly as registered (possibly relative); tier is
    /// not included — use [`magic_path_registrations`](Self::magic_path_registrations)
    /// for the tier-aware view resolution orders them by.
    pub fn prepended_magic_paths(&self) -> Vec<PathBuf> {
        self.magic_paths.prepend_paths().collect()
    }

    /// Magic roots registered at [`PathPosition::End`](super::PathPosition::End),
    /// in registration order.
    ///
    /// Roots are reported exactly as registered (possibly relative); tier is
    /// not included — use [`magic_path_registrations`](Self::magic_path_registrations)
    /// for the tier-aware view resolution orders them by.
    pub fn appended_magic_paths(&self) -> Vec<PathBuf> {
        self.magic_paths.append_paths().collect()
    }

    /// Every configured magic root with its registration position and tier
    /// policy, prepended roots first in registration order, then appended
    /// roots in registration order.
    ///
    /// This is the tier-aware view cache and graph identity should encode: a
    /// tier override alone can change the winning candidate without changing
    /// any other context field.
    pub fn magic_path_registrations(&self) -> Vec<MagicPathRegistration> {
        let mut registrations = Vec::with_capacity(
            self.magic_paths.prepend.len() + self.magic_paths.append.len(),
        );
        let mut extend =
            |entries: &[(PathBuf, super::MagicPathTier)], position: super::PathPosition| {
                for (path, tier) in entries {
                    registrations.push(MagicPathRegistration {
                        path: path.clone(),
                        position,
                        tier: *tier,
                    });
                }
            };
        extend(&self.magic_paths.prepend, super::PathPosition::Start);
        extend(&self.magic_paths.append, super::PathPosition::End);
        registrations
    }

    /// The ordered, deduplicated `@` search roots for this context, each
    /// carrying its provenance.
    ///
    /// Built by the same chain resolver `@` resolution, `%@` traversal, and
    /// completion use, in local-before-user tier order (see
    /// [`add_magic_path`](Self::add_magic_path)). Intrinsic roots carry their
    /// kind provenance (package, package area, repository or
    /// [`super::RootProvenance::LocalRoot`], home); configured roots carry
    /// [`super::RootProvenance::Magic`].
    pub fn magic_search_roots(&self) -> Vec<super::MagicSearchRoot> {
        crate::file_reference::resolve::magic_root_chain_for_context(self)
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
        let scope = catalog.scope_for(&self.cwd);
        self.repository_root = scope.repository_root;
        self.package_area = scope.package_area_root;
        self.package_root = scope.package_root;
    }

    /// Select the tree root for `cwd` from the current inputs, in precedence
    /// order: repository, explicit, deepest containing vault, opening anchor,
    /// fallback.
    fn select_tree(&self) -> TreeRoot {
        if let Some(repository_root) = &self.repository_root {
            return TreeRoot {
                path: repository_root.clone(),
                origin: BaseDirOrigin::Repository,
            };
        }
        if let Some(base_dir) = &self.explicit_base_dir {
            return TreeRoot {
                path: base_dir.clone(),
                origin: BaseDirOrigin::Explicit,
            };
        }
        if let Some(vault) = self.containing_vault() {
            return TreeRoot {
                path: vault,
                origin: BaseDirOrigin::Vault,
            };
        }
        if let Some(anchor) = &self.opening_anchor
            && anchor.contains(&self.cwd)
        {
            return anchor.clone();
        }
        TreeRoot::fallback(&self.cwd)
    }

    /// The deepest vault root containing `cwd`. Configured roots come before
    /// captured `VAULT` roots, matching the resolver's order, and the first
    /// root wins a depth tie.
    fn containing_vault(&self) -> Option<PathBuf> {
        let cwd = normalize_components(&self.cwd);
        let captured = self
            .env
            .get("VAULT")
            .map(|value| std::env::split_paths(value).collect::<Vec<_>>())
            .unwrap_or_default();
        let mut best: Option<(usize, PathBuf)> = None;
        for root in self.vault_roots.iter().chain(captured.iter()) {
            let normalized = normalize_components(root);
            if !root.is_absolute() || !cwd.starts_with(&normalized) {
                continue;
            }
            let depth = normalized.components().count();
            if best.as_ref().is_none_or(|(best_depth, _)| depth > *best_depth) {
                best = Some((depth, root.clone()));
            }
        }
        best.map(|(_, root)| root)
    }

    /// Reselect the tree after a builder changed a selection input.
    fn reselect_tree(&mut self) {
        self.tree = self.select_tree();
        if !self.derived {
            self.request_tree = self.tree.clone();
        }
    }

    /// Mirror the current repository/package anchors into the launch `@`
    /// scope.
    ///
    /// Called only by the direct builder methods, never by the
    /// `for_source`/`for_cwd` derivations: those re-anchor the
    /// source-relative kinds through `recompute_repository_scopes` while the
    /// launch `@` scope stays frozen at what the request captured.
    fn sync_launch_magic_scope(&mut self) {
        self.launch_magic_scope.repository_root = self.repository_root.clone();
        self.launch_magic_scope.package_root = self.package_root.clone();
        self.launch_magic_scope.package_area = self.package_area.clone();
    }

    /// Validate the absolute-anchor invariant and tree containment for the
    /// request and authoring `cwd`s.
    ///
    /// The request and authoring `cwd`s, an explicit
    /// [`with_base_dir`](Self::with_base_dir), the request directory,
    /// repository, package, and package-area roots of both the current anchors
    /// and the launch `@` scope, and a captured home
    /// directory must all be absolute host paths, whether or not a later
    /// resolution reads them. This is checked first, and trusted derivations
    /// are not exempt: trust lets a document change trees, not use a relative
    /// directory. Captured environment values and configured magic and vault
    /// roots keep their own rules and may be relative.
    ///
    /// Containment is component-aware and lexical after `.`/`..` normalization
    /// and Windows verbatim-prefix reduction, so a root and a `cwd` that name the
    /// same tree in different spellings still validate. It does **not**
    /// canonicalize through symlinks, so the authored/worktree identity is
    /// preserved. This is a trust check on the caller-provided root, not a
    /// sandbox boundary. A [`BaseDirOrigin::Fallback`] tree contains every
    /// `cwd`.
    ///
    /// Normal derivations must keep their authoring `cwd` inside the tree.
    /// Trusted-external derivations exempt only the current authoring `cwd`;
    /// the originating request is always checked against the tree it was
    /// captured with.
    ///
    /// A derived context first reports the failure the context it was derived
    /// from would have reported, captured before the derivation cleared an
    /// explicit root or reselected repository and package anchors. A builder
    /// applied before derivation can correct a setting; one applied to the
    /// derived context cannot clear the retained failure, because it describes
    /// the originating request. Derivation therefore cannot make an invalid
    /// request snapshot valid.
    ///
    /// ## Errors
    ///
    /// - [`FileReferenceError::RelativeContextDirectory`] when a directory or
    ///   anchor listed above, or the launch `@` scope's request directory, is
    ///   relative
    /// - [`FileReferenceError::BaseDirNotRepositoryRoot`] when an explicit
    ///   [`with_base_dir`](Self::with_base_dir) differs from the repository root
    /// - [`FileReferenceError::RepositoryRootNotContainingSource`] when a
    ///   repository tree does not contain a required `cwd`
    /// - [`FileReferenceError::CwdOutsideBaseDir`] when any other boundary
    ///   tree does not contain a required `cwd`
    pub fn validate(&self) -> Result<(), FileReferenceError> {
        self.check().map_err(FileReferenceError::from)
    }

    fn check(&self) -> Result<(), ContextFailure> {
        if let Some(failure) = &self.inherited_failure {
            return Err(failure.clone());
        }
        self.validate_absolute_anchors()?;
        if let (Some(base_dir), Some(repository_root)) =
            (&self.explicit_base_dir, &self.repository_root)
            && normalize_components(base_dir) != normalize_components(repository_root)
        {
            return Err(ContextFailure::BaseDirNotRepositoryRoot {
                base_dir: base_dir.clone(),
                repository_root: repository_root.clone(),
            });
        }
        if !self.request_tree.contains(&self.request_cwd) {
            return Err(self.request_tree.not_containing(&self.request_cwd));
        }
        if !self.trusted_external_authoring_cwd && !self.tree.contains(&self.cwd) {
            return Err(self.tree.not_containing(&self.cwd));
        }
        Ok(())
    }

    /// Every directory and tree anchor must be an absolute host path, whether
    /// or not a later resolution reads it. Trusted derivations are not exempt.
    /// The launch `@` scope is checked separately: a trusted derivation can
    /// drop the current repository anchors while that scope keeps them, and
    /// [`with_launch_magic_scope`](Self::with_launch_magic_scope) replaces it
    /// wholesale, request directory included.
    fn validate_absolute_anchors(&self) -> Result<(), ContextFailure> {
        let scope = &self.launch_magic_scope;
        let anchors = [
            (ContextAnchor::RequestDirectory, Some(&self.request_cwd)),
            (ContextAnchor::RequestDirectory, Some(&scope.request_dir)),
            (ContextAnchor::WorkingDirectory, Some(&self.cwd)),
            (ContextAnchor::BaseDir, self.explicit_base_dir.as_ref()),
            (ContextAnchor::RepositoryRoot, self.repository_root.as_ref()),
            (ContextAnchor::PackageRoot, self.package_root.as_ref()),
            (ContextAnchor::PackageArea, self.package_area.as_ref()),
            (ContextAnchor::RepositoryRoot, scope.repository_root.as_ref()),
            (ContextAnchor::PackageRoot, scope.package_root.as_ref()),
            (ContextAnchor::PackageArea, scope.package_area.as_ref()),
            (ContextAnchor::HomeDir, self.home_dir.as_ref()),
        ];
        for (anchor, path) in anchors {
            if let Some(path) = path
                && !path.is_absolute()
            {
                return Err(ContextFailure::RelativeContextDirectory {
                    anchor,
                    path: path.clone(),
                });
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
///
/// A relative `$HOME` is reported as no home directory: it is not a usable
/// anchor, and capturing it would make every [`FileResolutionContext::new`]
/// fail [`validate`](FileResolutionContext::validate).
pub fn home_dir() -> Option<PathBuf> {
    dirs::home_dir().filter(|home| home.is_absolute())
}

/// Snapshot the process environment for `{{VAR}}` interpolation.
///
/// `std::env::vars()` panics when any variable's name or value is not valid
/// Unicode, which POSIX permits, so one stray variable would crash every
/// context construction. Such variables are skipped instead; a reference
/// naming one fails with `MissingEnvironmentVariable`.
///
/// This is the environment [`FileResolutionContext::new`] snapshots; a caller
/// that builds through [`FileResolutionContext::from_snapshot`] reads it here
/// so both see the same variables.
pub fn capture_env() -> HashMap<String, String> {
    utf8_env(std::env::vars_os())
}

fn utf8_env(
    vars: impl IntoIterator<Item = (std::ffi::OsString, std::ffi::OsString)>,
) -> HashMap<String, String> {
    vars.into_iter()
        .filter_map(|(name, value)| Some((name.into_string().ok()?, value.into_string().ok()?)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utf8_env_skips_non_unicode_variables() {
        use std::ffi::OsString;
        #[cfg(unix)]
        let invalid = {
            use std::os::unix::ffi::OsStringExt;
            OsString::from_vec(vec![0x66, 0x6f, 0x80])
        };
        #[cfg(windows)]
        let invalid = {
            use std::os::windows::ffi::OsStringExt;
            OsString::from_wide(&[0x0066, 0xD800])
        };

        let env = utf8_env([
            (OsString::from("KEPT"), OsString::from("value")),
            (OsString::from("BAD_VALUE"), invalid.clone()),
            (invalid, OsString::from("bad name")),
        ]);

        assert_eq!(env, HashMap::from([("KEPT".to_string(), "value".to_string())]));
    }

    #[test]
    fn from_ambient_succeeds() {
        let ctx = ResolutionContext::from_ambient().unwrap();
        assert!(ctx.cwd.is_absolute());
    }

    #[test]
    fn from_cwd_absolute_path_is_preserved() {
        // Windows has no drive-less absolute path: `/tmp` is *rooted* there but
        // `is_absolute()` is false, so the literal must be selected per platform.
        #[cfg(windows)]
        let abs = Path::new(r"C:\tmp");
        #[cfg(not(windows))]
        let abs = Path::new("/tmp");

        let ctx = ResolutionContext::from_cwd(abs).unwrap();
        assert_eq!(ctx.cwd, abs);
    }

    #[test]
    fn from_cwd_relative_path_is_joined_to_ambient_cwd() {
        let ctx = ResolutionContext::from_cwd(Path::new("sub/dir")).unwrap();
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
