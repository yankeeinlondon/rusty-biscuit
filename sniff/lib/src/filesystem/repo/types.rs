use crate::Result;
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use tracing::instrument;

use crate::filesystem::file_types::{
    FileAssociationStats, FileInventory, FrameworkStats, ProgrammingLanguage,
    ProgrammingLanguageStats,
};
use crate::filesystem::repo::detection::canonicalize_path;
use crate::filesystem::repo::standard::{
    DetectedStandard, MonorepoLayer, MonorepoStandard, PackageProvenance,
};
use crate::package::DependencyEntry;

/// The primary ecosystem associated with a package boundary.
///
/// This is a property of the individual package, inferred from its own
/// manifest in both `structure()` and `full()` modes. It is distinct from
/// [`MonorepoStandard::spec`]`().primary_language` (a property of the *standard*
/// that owns the package) and from [`Package::primary_language`] (a rich-mode
/// file-scan result).
///
/// ## Notes
///
/// A Cargo workspace's standard has `primary_language = Rust`, but a member
/// package with a `package.json` may report `ecosystem = Node`. Conversely, a
/// Rust-only member still has no `primary_language` in `structure()` mode.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum PackageEcosystem {
    /// Rust/Cargo package
    Cargo,
    /// Node.js package
    Node,
    /// Python package
    Python,
    /// Go module
    Go,
    /// Unknown or mixed ecosystem package
    #[default]
    Unknown,
}

/// External dependency entry annotated with the package that declared it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExternalDependency {
    /// Package name that declares this dependency.
    pub package: String,
    /// Dependency family from the manifest.
    pub family: ExternalDependencyFamily,
    /// Dependency details parsed from the package manifest.
    #[serde(flatten)]
    pub dependency: DependencyEntry,
}

/// Dependency families exposed by `sniff repo dependencies`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ExternalDependencyFamily {
    /// Runtime dependencies.
    Dependencies,
    /// Development dependencies.
    DevDependencies,
    /// Peer dependencies.
    PeerDependencies,
    /// Optional dependencies.
    OptionalDependencies,
}

/// Filters selecting which dependency families to include.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExternalDependencyFilter {
    pub dependencies: bool,
    pub dev_dependencies: bool,
    pub peer_dependencies: bool,
    pub optional_dependencies: bool,
}

impl ExternalDependencyFilter {
    /// All dependency families.
    pub const fn all() -> Self {
        Self {
            dependencies: true,
            dev_dependencies: true,
            peer_dependencies: true,
            optional_dependencies: true,
        }
    }

    /// Use all families when no explicit family flag was selected.
    pub const fn normalize(self) -> Self {
        if self.dependencies
            || self.dev_dependencies
            || self.peer_dependencies
            || self.optional_dependencies
        {
            self
        } else {
            Self::all()
        }
    }

    pub const fn includes(self, family: ExternalDependencyFamily) -> bool {
        match family {
            ExternalDependencyFamily::Dependencies => self.dependencies,
            ExternalDependencyFamily::DevDependencies => self.dev_dependencies,
            ExternalDependencyFamily::PeerDependencies => self.peer_dependencies,
            ExternalDependencyFamily::OptionalDependencies => self.optional_dependencies,
        }
    }
}

/// Information about a detected repository
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RepoInfo {
    /// Whether this is a monorepo
    pub is_monorepo: bool,
    /// Root directory of the repository
    pub root: PathBuf,
    /// Dependencies (for non-monorepo projects only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dependencies: Option<Vec<DependencyEntry>>,
    /// Dev dependencies (for non-monorepo projects only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dev_dependencies: Option<Vec<DependencyEntry>>,
    /// Peer dependencies (for non-monorepo projects only, JS ecosystem)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub peer_dependencies: Option<Vec<DependencyEntry>>,
    /// Optional dependencies (for non-monorepo projects only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub optional_dependencies: Option<Vec<DependencyEntry>>,
    /// Packages within the monorepo (only present when is_monorepo is true)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub packages: Option<Vec<Package>>,
    /// Standards detected at the repo root, each with its acting binary and
    /// detection confidence.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub monorepo_standards: Vec<DetectedStandard>,
    /// Membership layers: each authority that declares packages plus any
    /// orchestrators riding on top. A forest, even for single-root repos.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub monorepo_layers: Vec<MonorepoLayer>,
}

/// A package within a monorepo.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Package {
    /// Absolute path to the package
    pub path: PathBuf,
    /// Path relative to the repo root (e.g., "sniff/lib")
    pub relative: String,
    /// Directory path between repo root and package root (e.g., "sniff" for "sniff/lib",
    /// "apps/browser" for "apps/browser/my_package", "root" for top-level packages)
    pub package_area: String,
    /// Native package name from manifest (Cargo.toml `[package]`.name or package.json name)
    pub name: String,
    /// The package ecosystem inferred from its manifests.
    #[serde(default)]
    pub ecosystem: PackageEcosystem,
    /// The monorepo standard that owns this package.
    #[serde(default)]
    pub standard: MonorepoStandard,
    /// How this package boundary was derived.
    #[serde(default)]
    pub provenance: PackageProvenance,
    /// Nested package names detected beneath this package root.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub nested_packages: Vec<String>,
    /// The primary programming language detected in this package
    pub primary_language: Option<ProgrammingLanguage>,
    /// Secondary programming languages detected in this package.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub secondary_languages: Vec<ProgrammingLanguage>,
    /// Structured programming language statistics.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub languages: Vec<ProgrammingLanguageStats>,
    /// Structured framework statistics.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub frameworks: Vec<FrameworkStats>,
    /// Broad file-association statistics.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub file_associations: Vec<FileAssociationStats>,
    /// Configuration files found in the package tree.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub configuration: Vec<PathBuf>,
    /// Documentation files found in the package tree.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub documentation: Vec<PathBuf>,
    /// EditorConfig file path (if present)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub editor_config: Option<PathBuf>,
    /// Command runner files (justfile, Makefile, etc.)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub command_runner: Vec<PathBuf>,
    /// Detected package managers (e.g., "cargo", "npm", "pnpm")
    pub package_managers: Vec<String>,
    /// Test runners declared by this package, with the evidence source for
    /// each detection. Populated by
    /// [`detect_test_runners`](super::test_runner_usage::detect_test_runners).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub test_runners: Vec<crate::filesystem::repo::test_runner_usage::TestRunnerUsage>,
    /// Package version from manifest (Cargo.toml `[package]`.version or package.json version)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Feature flags defined by this package (e.g., Cargo `[features]`)
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub features: Vec<String>,
    /// Names of workspace-internal packages this package depends on
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub depends_on: Vec<String>,
    /// Names of workspace-internal packages that depend on this package
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub used_by: Vec<String>,
    /// Dependencies for this package
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dependencies: Option<Vec<DependencyEntry>>,
    /// Dev dependencies for this package
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dev_dependencies: Option<Vec<DependencyEntry>>,
    /// Peer dependencies for this package (JS ecosystem)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub peer_dependencies: Option<Vec<DependencyEntry>>,
    /// Optional dependencies for this package
    #[serde(skip_serializing_if = "Option::is_none")]
    pub optional_dependencies: Option<Vec<DependencyEntry>>,
    /// Whether any dependency can be updated (deep-mode only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_updatable: Option<bool>,
    /// Whether any dependency has a major version update available (deep-mode only)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub has_major_update: Option<bool>,
    /// Whether this package is excluded from the workspace
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub is_excluded: bool,
}

impl RepoInfo {
    /// The layer that best represents the repository as a whole.
    ///
    /// ## Returns
    ///
    /// Selection rule:
    /// 1. The layer whose `root` equals the repo root, if one exists.
    /// 2. Otherwise the layer with the shallowest root (fewest path
    ///    components).
    /// 3. Ties are broken by `MonorepoStandard` enum-declaration order
    ///    (Cargo first … Unknown last) — **not** detector push-order, so
    ///    reordering detectors cannot silently change the primary layer.
    ///
    /// For the canonical Cargo + uv-at-repo-root case this selects Cargo,
    /// matching today's `.first()` output.
    pub fn primary_layer(&self) -> Option<&MonorepoLayer> {
        if self.monorepo_layers.is_empty() {
            return None;
        }
        let root = canonicalize_path(&self.root);
        self.monorepo_layers.iter().min_by_key(|layer| {
            let layer_root = canonicalize_path(&layer.root);
            let root_match = layer_root == root;
            let depth = layer_root.components().count();
            let authority_order = layer.authority;
            (!root_match, depth, authority_order)
        })
    }

    /// Find the package whose directory tree contains `dir`.
    ///
    /// Returns `None` when `dir` is not inside any package.
    pub fn package_for_dir(&self, dir: &Path) -> Option<&Package> {
        let packages = self.packages.as_ref()?;
        let dir = canonicalize_path(dir);

        packages
            .iter()
            .filter(|pkg| dir.starts_with(canonicalize_path(&pkg.path)))
            .max_by_key(|pkg| canonicalize_path(&pkg.path).components().count())
    }

    /// Resolve the area name for `dir`, combining package and package-area into
    /// a single value useful inside a monorepo.
    ///
    /// The rule: if `dir` is inside a package, the area is that package's name;
    /// otherwise the area is the surrounding package-area string. When no
    /// discovered package names the area — e.g. a freshly scaffolded area whose
    /// crates are not yet listed in `[workspace] members` — the area is taken
    /// from the directory structure (see [`directory_area_fallback`]). Falls
    /// back to `"root"` only at the repo root.
    ///
    /// ## Examples
    ///
    /// Given a monorepo with a `sniff/lib` package whose `package_area` is
    /// `"sniff"`:
    ///
    /// - CWD inside `sniff/lib/src` → `"sniff-lib"` (the package name)
    /// - CWD at `sniff/` but outside any package → `"sniff"`
    /// - CWD in `reaper/lib` while `reaper/*` is not yet a workspace member →
    ///   `"reaper"` (directory-structure fallback)
    /// - CWD at repo root → `"root"`
    ///
    /// [`directory_area_fallback`]: Self::directory_area_fallback
    pub fn area_for_dir(&self, dir: &Path) -> Cow<'_, str> {
        if let Some(pkg) = self.package_for_dir(dir) {
            return Cow::Borrowed(&pkg.name);
        }
        if let Some(area) = self.package_area_for_dir(dir) {
            return Cow::Borrowed(area);
        }
        self.directory_area_fallback(dir)
            .map_or(Cow::Borrowed("root"), Cow::Owned)
    }

    /// Resolve the package-area label for `dir`, falling back to the directory
    /// structure when no discovered package names the area.
    ///
    /// Behaves like [`package_area_for_dir`](Self::package_area_for_dir) but, in
    /// a monorepo, also resolves the area of a directory that holds no workspace
    /// member yet — e.g. a freshly scaffolded area not listed in
    /// `[workspace] members`. Returns `None` at the repo root or outside the repo.
    pub fn package_area_label_for_dir(&self, dir: &Path) -> Option<Cow<'_, str>> {
        if let Some(area) = self.package_area_for_dir(dir) {
            return Some(Cow::Borrowed(area));
        }
        self.directory_area_fallback(dir).map(Cow::Owned)
    }

    /// Derive an area name for `dir` from the directory structure alone.
    ///
    /// Used as the last resort when no discovered package identifies the area,
    /// so that a scaffolded area resolves before its crates are wired into the
    /// workspace `members`. The area is the first path component of `dir`
    /// relative to the repo root.
    ///
    /// Returns `None` outside a monorepo, at the repo root, or when `dir` is not
    /// under the root — the area concept is monorepo-only, mirroring the
    /// `is_monorepo` gate the reporting commands already apply.
    fn directory_area_fallback(&self, dir: &Path) -> Option<String> {
        if !self.is_monorepo {
            return None;
        }
        let dir = canonicalize_path(dir);
        let root = canonicalize_path(&self.root);
        let rel = dir.strip_prefix(&root).ok()?;
        match rel.components().next()? {
            std::path::Component::Normal(name) => Some(name.to_string_lossy().into_owned()),
            _ => None,
        }
    }

    /// Find the package area that contains `dir`.
    ///
    /// First checks if `dir` is inside a specific package, then falls back to
    /// checking whether it sits anywhere within a package area directory.
    /// Returns `None` when `dir` is outside every known package area.
    pub fn package_area_for_dir(&self, dir: &Path) -> Option<&str> {
        let packages = self.packages.as_ref()?;
        let dir = canonicalize_path(dir);

        // Check if inside a specific package first
        if let Some(pkg) = self.package_for_dir(&dir) {
            return Some(&pkg.package_area);
        }

        // Fall back to checking package area directories
        let root = canonicalize_path(&self.root);
        let areas: HashSet<&str> = packages
            .iter()
            .map(|p| p.package_area.as_str())
            .filter(|a| *a != "root")
            .collect();

        for area in &areas {
            let area_path = root.join(area);
            if dir.starts_with(&area_path) {
                // Return a reference with the right lifetime by finding the
                // original &str in the packages vec
                return packages
                    .iter()
                    .find(|p| p.package_area == *area)
                    .map(|p| p.package_area.as_str());
            }
        }

        None
    }
}

/// Deprecated type alias for backward compatibility.
#[deprecated(note = "Use `Package` instead")]
pub type PackageLocation = Package;

/// Categorized files found in a package directory.
#[derive(Default)]
pub(crate) struct PackageFiles {
    pub(crate) configuration: Vec<PathBuf>,
    pub(crate) documentation: Vec<PathBuf>,
    pub(crate) editor_config: Option<PathBuf>,
    pub(crate) command_runner: Vec<PathBuf>,
}

#[derive(Default)]
pub(crate) struct PackageScanResult {
    pub(crate) language_breakdown: crate::filesystem::languages::LanguageBreakdown,
    pub(crate) file_breakdown: crate::filesystem::file_types::FileAssociationBreakdown,
    pub(crate) compatibility: PackageFiles,
}

/// Detect repository configuration in the given directory.
///
/// ## Examples
///
/// ```no_run
/// use std::path::Path;
/// use sniff::filesystem::repo::detect_repo;
///
/// let root = Path::new("/path/to/project");
/// if let Some(info) = detect_repo(root).unwrap() {
///     if info.is_monorepo {
///         println!("Packages: {}", info.packages.as_ref().map(|p| p.len()).unwrap_or(0));
///         if let Some(layer) = info.primary_layer() {
///             println!("Authority: {:?}", layer.authority);
///         }
///     }
/// }
/// ```
///
/// ## Returns
///
/// - `Ok(Some(RepoInfo))` if a repository is detected
/// - `Ok(None)` if no repository configuration is found
/// - `Err(SniffError)` if there's an error reading files
pub fn detect_repo(root: &Path) -> Result<Option<RepoInfo>> {
    super::detection::detect_repo_inner(root, false).map(|(info, _inventory)| info)
}

/// Shallow repository detection for topology and package identity.
///
/// Package managers, dependencies, test runners, features, languages,
/// frameworks, and file lists are empty. Call [`detect_repo_with_request`]
/// with [`RepoRequest::focused`] for selected manifest-backed details, or
/// [`detect_repo`] for complete enrichment.
///
/// [`RepoRequest::structure`]: crate::request::RepoRequest::structure
#[instrument(skip_all, fields(root = %root.display()))]
pub fn detect_repo_structure(root: &Path) -> Result<Option<RepoInfo>> {
    super::detection::detect_repo_inner(root, true).map(|(info, _inventory)| info)
}

/// Detect a repository using a caller-selected detail request.
pub fn detect_repo_with_request(
    root: &Path,
    request: &crate::request::RepoRequest,
) -> Result<Option<RepoInfo>> {
    super::detection::detect_repo_inner_with_request(root, request)
        .map(|(info, _inventory)| info)
}

/// Like [`detect_repo_structure`], but synthesizes a single-package `RepoInfo`
/// from the root manifest when no workspace structure is detected.
///
/// [`detect_repo_structure`] returns `Ok(None)` for an ordinary single-package
/// project (a `Cargo.toml` with `[package]` but no `[workspace]`, or a lone
/// `package.json`, `pyproject.toml`, or `go.mod`). This function preserves the
/// shallow semantics of [`detect_repo_structure`]. Use
/// [`detect_repo_with_request_or_root_package`] when selected package details
/// are required.
///
/// ## Returns
///
/// - The workspace `RepoInfo` from [`detect_repo_structure`] when one is found.
/// - A single-package, non-monorepo `RepoInfo` when only a root manifest exists.
/// - `Ok(None)` when `root` has no recognizable package manifest.
#[instrument(skip_all, fields(root = %root.display()))]
pub fn detect_repo_structure_or_root_package(root: &Path) -> Result<Option<RepoInfo>> {
    if let Some(info) = detect_repo_structure(root)? {
        return Ok(Some(info));
    }
    Ok(super::detection::synthesize_root_package_repo(root))
}

/// Detect a repository using `request`, synthesizing a standalone root package
/// when no workspace structure is present.
pub fn detect_repo_with_request_or_root_package(
    root: &Path,
    request: &crate::request::RepoRequest,
) -> Result<Option<RepoInfo>> {
    if let Some(info) = detect_repo_with_request(root, request)? {
        return Ok(Some(info));
    }
    Ok(super::detection::synthesize_root_package_repo_with_request(
        root, request,
    ))
}

/// Full repo detection that also returns the shared file inventory.
///
/// The returned inventory is the same one used internally to enrich
/// per-package language/framework data, so callers that need both repo
/// info and a top-level file inventory can avoid scanning the tree twice.
pub fn detect_repo_with_inventory(
    root: &Path,
) -> Result<(Option<RepoInfo>, Option<FileInventory>)> {
    super::detection::detect_repo_inner(root, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================================
    // RepoInfo helper tests
    // ============================================================================

    #[test]
    fn repo_info_package_for_dir_finds_deepest_match() {
        let repo = RepoInfo {
            is_monorepo: true,
            root: PathBuf::from("/repo"),
            dependencies: None,
            dev_dependencies: None,
            peer_dependencies: None,
            optional_dependencies: None,
            monorepo_standards: Vec::new(),
            monorepo_layers: Vec::new(),
            packages: Some(vec![
                Package {
                    path: PathBuf::from("/repo/crates"),
                    relative: "crates".to_string(),
                    package_area: "root".to_string(),
                    name: "crates".to_string(),
                    ecosystem: PackageEcosystem::Cargo,
                    ..Package::default()
                },
                Package {
                    path: PathBuf::from("/repo/crates/pkg-a"),
                    relative: "crates/pkg-a".to_string(),
                    package_area: "crates".to_string(),
                    name: "pkg-a".to_string(),
                    ecosystem: PackageEcosystem::Cargo,
                    ..Package::default()
                },
            ]),
        };

        assert_eq!(
            repo.package_for_dir(Path::new("/repo/crates/pkg-a/src"))
                .map(|p| p.name.as_str()),
            Some("pkg-a")
        );
    }

    // ============================================================================
    // area_for_dir tests
    // ============================================================================

    fn monorepo_with_areas() -> RepoInfo {
        RepoInfo {
            is_monorepo: true,
            root: PathBuf::from("/repo"),
            dependencies: None,
            dev_dependencies: None,
            peer_dependencies: None,
            optional_dependencies: None,
            monorepo_standards: Vec::new(),
            monorepo_layers: Vec::new(),
            packages: Some(vec![
                Package {
                    path: PathBuf::from("/repo/sniff/lib"),
                    relative: "sniff/lib".to_string(),
                    package_area: "sniff".to_string(),
                    name: "sniff-lib".to_string(),
                    ..Package::default()
                },
                Package {
                    path: PathBuf::from("/repo/sniff/cli"),
                    relative: "sniff/cli".to_string(),
                    package_area: "sniff".to_string(),
                    name: "sniff-cli".to_string(),
                    ..Package::default()
                },
                Package {
                    path: PathBuf::from("/repo/top-pkg"),
                    relative: "top-pkg".to_string(),
                    package_area: "root".to_string(),
                    name: "top-pkg".to_string(),
                    ..Package::default()
                },
            ]),
        }
    }

    #[test]
    fn area_for_dir_returns_package_name_when_inside_package() {
        let repo = monorepo_with_areas();
        assert_eq!(
            repo.area_for_dir(Path::new("/repo/sniff/lib/src")),
            "sniff-lib"
        );
    }

    #[test]
    fn area_for_dir_returns_package_name_for_top_level_package() {
        let repo = monorepo_with_areas();
        assert_eq!(repo.area_for_dir(Path::new("/repo/top-pkg/src")), "top-pkg");
    }

    #[test]
    fn area_for_dir_falls_back_to_package_area_when_outside_any_package() {
        let repo = monorepo_with_areas();
        // /repo/sniff exists as an area dir but no package lives at that path
        assert_eq!(repo.area_for_dir(Path::new("/repo/sniff")), "sniff");
    }

    #[test]
    fn area_for_dir_returns_root_at_repo_root() {
        let repo = monorepo_with_areas();
        assert_eq!(repo.area_for_dir(Path::new("/repo")), "root");
    }

    #[test]
    fn area_for_dir_falls_back_to_directory_name_for_unwired_area() {
        // `reaper/lib` exists on disk but is not yet a workspace member, so no
        // package carries the "reaper" area. The area still resolves from the
        // directory structure.
        let repo = monorepo_with_areas();
        assert_eq!(
            repo.area_for_dir(Path::new("/repo/reaper/lib/src")),
            "reaper"
        );
        assert_eq!(repo.area_for_dir(Path::new("/repo/reaper")), "reaper");
    }

    #[test]
    fn package_area_label_falls_back_to_directory_name_for_unwired_area() {
        let repo = monorepo_with_areas();
        assert_eq!(
            repo.package_area_label_for_dir(Path::new("/repo/reaper/lib"))
                .as_deref(),
            Some("reaper")
        );
        // Member-discovered areas still resolve exactly as before.
        assert_eq!(
            repo.package_area_label_for_dir(Path::new("/repo/sniff"))
                .as_deref(),
            Some("sniff")
        );
        // Repo root has no area.
        assert_eq!(repo.package_area_label_for_dir(Path::new("/repo")), None);
    }

    #[test]
    fn directory_fallback_disabled_outside_monorepo() {
        let mut repo = monorepo_with_areas();
        repo.is_monorepo = false;
        // Without a monorepo the area concept does not apply: no directory-name
        // fallback, so an unwired path resolves to the "root" sentinel.
        assert_eq!(repo.area_for_dir(Path::new("/repo/reaper/lib")), "root");
        assert_eq!(
            repo.package_area_label_for_dir(Path::new("/repo/reaper/lib")),
            None
        );
    }

    // ============================================================================
    // primary_layer tests
    // ============================================================================

    fn layer_at(root: &str, authority: MonorepoStandard) -> MonorepoLayer {
        MonorepoLayer {
            root: PathBuf::from(root),
            authority,
            orchestrators: Vec::new(),
            provenance: crate::filesystem::repo::standard::PackageProvenance::Globbed,
            lockfile_match: None,
            root_is_package: false,
            packages: Vec::new(),
        }
    }

    #[test]
    fn primary_layer_returns_none_when_no_layers() {
        let repo = RepoInfo {
            root: PathBuf::from("/repo"),
            ..RepoInfo::default()
        };
        assert!(repo.primary_layer().is_none());
    }

    #[test]
    fn primary_layer_selects_single_root_layer() {
        let repo = RepoInfo {
            root: PathBuf::from("/repo"),
            monorepo_layers: vec![layer_at("/repo", MonorepoStandard::CargoWorkspace)],
            ..RepoInfo::default()
        };
        let layer = repo.primary_layer().unwrap();
        assert_eq!(layer.authority, MonorepoStandard::CargoWorkspace);
    }

    #[test]
    fn primary_layer_selects_cargo_over_uv_at_shared_root() {
        // The canonical shared-root case: Cargo + uv both at the repo root.
        // Enum-declaration order must break the tie in favor of Cargo.
        let repo = RepoInfo {
            root: PathBuf::from("/repo"),
            monorepo_layers: vec![
                layer_at("/repo", MonorepoStandard::UvWorkspace),
                layer_at("/repo", MonorepoStandard::CargoWorkspace),
            ],
            ..RepoInfo::default()
        };
        let layer = repo.primary_layer().unwrap();
        assert_eq!(layer.authority, MonorepoStandard::CargoWorkspace);
    }

    #[test]
    fn primary_layer_selects_shallowest_root() {
        let repo = RepoInfo {
            root: PathBuf::from("/repo"),
            monorepo_layers: vec![
                layer_at("/repo/nested", MonorepoStandard::PnpmWorkspaces),
                layer_at("/repo", MonorepoStandard::CargoWorkspace),
            ],
            ..RepoInfo::default()
        };
        let layer = repo.primary_layer().unwrap();
        assert_eq!(layer.authority, MonorepoStandard::CargoWorkspace);
        assert_eq!(layer.root, PathBuf::from("/repo"));
    }

    #[test]
    fn primary_layer_selects_shallowest_nested_layer_when_none_at_repo_root() {
        // No layer is rooted at the repo root, so selection rule 2 (shallowest
        // root) applies. `/repo/apps` (2 components) is shallower than
        // `/repo/tools/nested` (3 components) and must win — even though the
        // deeper layer's authority (Cargo) has higher enum priority than pnpm.
        let repo = RepoInfo {
            root: PathBuf::from("/repo"),
            monorepo_layers: vec![
                layer_at("/repo/tools/nested", MonorepoStandard::CargoWorkspace),
                layer_at("/repo/apps", MonorepoStandard::PnpmWorkspaces),
            ],
            ..RepoInfo::default()
        };
        let layer = repo.primary_layer().unwrap();
        assert_eq!(layer.authority, MonorepoStandard::PnpmWorkspaces);
        assert_eq!(layer.root, PathBuf::from("/repo/apps"));
    }

    #[test]
    fn primary_layer_breaks_nested_ties_by_enum_order() {
        // Two nested layers at the same depth (2 components each), neither at
        // the repo root. Listed opposite the expected enum order (pnpm before
        // cargo) so the tie-break by `MonorepoStandard` declaration order — not
        // iteration/push order — is what selects Cargo.
        let repo = RepoInfo {
            root: PathBuf::from("/repo"),
            monorepo_layers: vec![
                layer_at("/repo/apps", MonorepoStandard::PnpmWorkspaces),
                layer_at("/repo/tools", MonorepoStandard::CargoWorkspace),
            ],
            ..RepoInfo::default()
        };
        let layer = repo.primary_layer().unwrap();
        assert_eq!(layer.authority, MonorepoStandard::CargoWorkspace);
        assert_eq!(layer.root, PathBuf::from("/repo/tools"));
    }

    #[test]
    fn primary_layer_reproduces_first_on_rusty_biscuit_repo() {
        // Regression: on the rusty-biscuit repo, `primary_layer()` must agree
        // with the first layer — and select Cargo over the pnpm workspace that
        // also lives at the repo root.
        let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
        let repo_root = manifest_dir.parent().unwrap().parent().unwrap();
        let info = detect_repo_structure(repo_root)
            .expect("detect_repo_structure should succeed")
            .expect("rusty-biscuit should be a repo");
        assert!(info.is_monorepo, "rusty-biscuit should be a monorepo");

        let primary = info
            .primary_layer()
            .expect("primary_layer must resolve on rusty-biscuit");
        let first = info
            .monorepo_layers
            .first()
            .expect("rusty-biscuit has at least one layer");

        assert_eq!(primary.authority, first.authority);
        assert_eq!(primary.authority, MonorepoStandard::CargoWorkspace);
        assert_eq!(primary.orchestrators, first.orchestrators);
    }
}
