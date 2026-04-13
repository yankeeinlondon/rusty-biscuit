use crate::Result;
use biscuit_file::toml_crate;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use tracing::{debug, instrument};

use crate::filesystem::file_types::{
    FileAssociationStats, FileInventory, FrameworkStats, ProgrammingLanguage,
    ProgrammingLanguageStats,
};
use crate::filesystem::repo::detection::{
    canonicalize_path, is_fixture_manifest, is_generated_manifest,
};

/// Supported monorepo tools and package managers
#[non_exhaustive]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum MonorepoTool {
    /// Rust Cargo workspace
    CargoWorkspace,
    /// npm workspaces
    NpmWorkspaces,
    /// pnpm workspaces
    PnpmWorkspaces,
    /// Yarn workspaces
    YarnWorkspaces,
    /// Nx monorepo tool
    Nx,
    /// Turborepo
    Turborepo,
    /// Lerna
    Lerna,
    /// Unknown monorepo tool
    Unknown,
}

/// The primary ecosystem associated with a package boundary.
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

/// Describes how a package boundary was discovered.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum PackageDiscoverySource {
    /// Cargo workspace member
    CargoWorkspace,
    /// pnpm workspace package
    PnpmWorkspace,
    /// npm workspace package
    NpmWorkspace,
    /// Yarn workspace package
    YarnWorkspace,
    /// Nx-discovered package
    Nx,
    /// Turborepo-discovered package
    Turborepo,
    /// Lerna-discovered package
    Lerna,
    /// Directly discovered from a package manifest
    ManifestScan,
}

/// The type/category of a dependency.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum DependencyKind {
    /// Normal runtime dependency
    #[default]
    Normal,
    /// Development-only dependency (testing, building docs, etc.)
    Dev,
    /// Build script dependency (Cargo's build-dependencies)
    Build,
    /// Optional dependency (enabled via features)
    Optional,
    /// Target-specific dependency (e.g., platform-specific)
    Target,
}

/// A single dependency entry with version information.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DependencyEntry {
    /// The package/crate name
    pub name: String,
    /// The kind of dependency (internal use only, hidden from JSON)
    #[serde(skip)]
    pub kind: DependencyKind,
    /// Version requirement as specified in the manifest (e.g., "^1.0", ">=2.0, <3.0")
    #[serde(alias = "version_req")]
    pub targeted_version: String,
    /// Actual resolved version from the lockfile (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual_version: Option<String>,
    /// The package manager used for this dependency (e.g., "cargo", "npm")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package_manager: Option<String>,
    /// Latest version available from the registry (only populated with --deep flag)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest_version: Option<String>,
    /// Target specification for target-specific dependencies (e.g., "cfg(target_os = \"macos\")")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target: Option<String>,
    /// Whether this dependency is optional (feature-gated)
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub optional: bool,
    /// Features enabled for this dependency
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub features: Vec<String>,
    /// Whether this dependency can be updated (latest != actual)
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub is_updatable: bool,
    /// Whether the available update is a major version bump.
    ///
    /// Only set when `is_updatable` is true and both versions follow
    /// semantic versioning (`major.minor.patch`). Considered major when:
    /// - The major version is 0 and a newer minor version exists, or
    /// - A newer major version exists.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub has_major_update: bool,
}

/// Information about a detected repository
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoInfo {
    /// Whether this is a monorepo
    pub is_monorepo: bool,
    /// The tool managing the monorepo (if is_monorepo is true)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub monorepo_tool: Option<MonorepoTool>,
    /// All workspace tools detected at the repo root.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub workspace_tools: Vec<MonorepoTool>,
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
}

/// A package within a monorepo.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Package {
    /// Absolute path to the package
    pub path: PathBuf,
    /// Path relative to the repo root (e.g., "sniff/lib")
    pub relative: String,
    /// Directory path between repo root and package root (e.g., "sniff" for "sniff/lib",
    /// "apps/browser" for "apps/browser/my_package", "root" for top-level packages)
    pub package_area: String,
    /// Native package name from manifest (Cargo.toml [package].name or package.json name)
    pub name: String,
    /// The package ecosystem inferred from its manifests.
    #[serde(default)]
    pub ecosystem: PackageEcosystem,
    /// How this package boundary was discovered.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub discovery_sources: Vec<PackageDiscoverySource>,
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
    /// Package version from manifest (Cargo.toml [package].version or package.json version)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Feature flags defined by this package (e.g., Cargo [features])
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

/// Resolved versions from Cargo.lock.
pub(crate) struct CargoLockVersions {
    versions: HashMap<String, Vec<String>>,
}

impl CargoLockVersions {
    /// Parse a Cargo.lock file and extract package versions.
    pub fn parse(lock_path: &Path) -> Option<Self> {
        let content = std::fs::read_to_string(lock_path)
            .map_err(|e| {
                debug!(path = %lock_path.display(), error = %e, "could not read file");
                e
            })
            .ok()?;
        let parsed: toml_crate::Value = toml_crate::from_str(&content).ok()?;

        let mut versions: HashMap<String, Vec<String>> = HashMap::new();

        if let Some(packages) = parsed.get("package").and_then(|p| p.as_array()) {
            for pkg in packages {
                if let (Some(name), Some(version)) = (
                    pkg.get("name").and_then(|n| n.as_str()),
                    pkg.get("version").and_then(|v| v.as_str()),
                ) {
                    versions
                        .entry(name.to_string())
                        .or_default()
                        .push(version.to_string());
                }
            }
        }

        Some(Self { versions })
    }

    /// Resolve the version for a dependency name.
    ///
    /// Returns the first resolved version if available.
    pub fn resolve(&self, name: &str) -> Option<String> {
        self.versions.get(name).and_then(|v| v.first()).cloned()
    }
}

/// Manifest file type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum ManifestKind {
    Cargo,
    Node,
    Python,
    Go,
}

/// Index of all manifest files in a directory tree.
///
/// Performs a single directory walk and caches manifest locations,
/// avoiding redundant filesystem traversals during package discovery.
#[derive(Debug, Clone)]
pub(crate) struct ManifestIndex {
    /// Map from parent directory to set of manifest kinds found there
    manifests: HashMap<PathBuf, HashSet<ManifestKind>>,
}

impl ManifestIndex {
    /// Build manifest index by walking the directory tree once.
    pub(crate) fn build(root: &Path) -> Self {
        let mut manifests: HashMap<PathBuf, HashSet<ManifestKind>> = HashMap::new();

        let walker = walkdir::WalkDir::new(root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|entry| {
                if !entry.file_type().is_dir() {
                    return true;
                }

                let name = entry.file_name().to_string_lossy();
                name != ".git"
                    && name != "node_modules"
                    && name != "target"
                    && name != ".turbo"
                    && name != "dist"
                    && name != "build"
            });

        for entry in walker.filter_map(|e| e.ok()) {
            if !entry.file_type().is_file() {
                continue;
            }

            let file_name = entry.file_name().to_string_lossy();
            let kind = match file_name.as_ref() {
                "Cargo.toml" => ManifestKind::Cargo,
                "package.json" => ManifestKind::Node,
                "pyproject.toml" => ManifestKind::Python,
                "go.mod" => ManifestKind::Go,
                _ => continue,
            };

            if is_generated_manifest(entry.path()) {
                continue;
            }
            if is_fixture_manifest(entry.path()) {
                continue;
            }

            let Some(parent) = entry.path().parent() else {
                continue;
            };

            manifests
                .entry(parent.to_path_buf())
                .or_default()
                .insert(kind);
        }

        Self { manifests }
    }

    pub(crate) fn from_manifest_paths(paths: impl IntoIterator<Item = PathBuf>) -> Self {
        let mut manifests: HashMap<PathBuf, HashSet<ManifestKind>> = HashMap::new();

        for path in paths {
            let Some(file_name) = path.file_name().and_then(|value| value.to_str()) else {
                continue;
            };

            let kind = match file_name {
                "Cargo.toml" => ManifestKind::Cargo,
                "package.json" => ManifestKind::Node,
                "pyproject.toml" => ManifestKind::Python,
                "go.mod" => ManifestKind::Go,
                _ => continue,
            };

            let Some(parent) = path.parent() else {
                continue;
            };

            manifests
                .entry(parent.to_path_buf())
                .or_default()
                .insert(kind);
        }

        Self { manifests }
    }

    /// Get directories containing manifests within a specific subtree.
    pub(crate) fn package_dirs_in_tree(&self, search_root: &Path, root: &Path) -> Vec<&Path> {
        let search_root_canonical = canonicalize_path(search_root);
        let root_canonical = canonicalize_path(root);

        let mut dirs: Vec<&Path> = self
            .manifests
            .keys()
            .filter_map(|path| {
                let canonical = canonicalize_path(path);
                // Must be within search_root but not equal to root
                if canonical.starts_with(&search_root_canonical) && canonical != root_canonical {
                    Some(path.as_path())
                } else {
                    None
                }
            })
            .collect();

        dirs.sort();
        dirs
    }
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
///         println!("Monorepo tool: {:?}", info.monorepo_tool);
///         if let Some(ref packages) = info.packages {
///             println!("Packages: {}", packages.len());
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

/// Lightweight repo detection that skips per-package language scanning.
///
/// Returns the same package structure (names, paths, areas) but without
/// `primary_language`, `frameworks`, or `file_associations` per package.
/// Typically 10-50x faster than `detect_repo` on large monorepos.
#[instrument(skip_all, fields(root = %root.display()))]
pub fn detect_repo_structure(root: &Path) -> Result<Option<RepoInfo>> {
    super::detection::detect_repo_inner(root, true).map(|(info, _inventory)| info)
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
