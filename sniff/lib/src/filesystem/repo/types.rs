use crate::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use tracing::instrument;

use crate::filesystem::file_types::{
    FileAssociationStats, FileInventory, FrameworkStats, ProgrammingLanguage,
    ProgrammingLanguageStats,
};
use crate::filesystem::repo::detection::canonicalize_path;
use crate::package::DependencyEntry;

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
    let options = super::super::system_view::SharedWalkOptions {
        collect_manifests: true,
        collect_inventory: true,
        collect_docs: false,
    };
    let view = super::super::system_view::build_filesystem_system_view(root, options);
    super::detection::detect_repo_inner_with_shared(
        root,
        false,
        view.manifest_index.as_ref(),
        view.inventory.as_ref(),
    )
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
            monorepo_tool: None,
            workspace_tools: Vec::new(),
            root: PathBuf::from("/repo"),
            dependencies: None,
            dev_dependencies: None,
            peer_dependencies: None,
            optional_dependencies: None,
            packages: Some(vec![
                Package {
                    path: PathBuf::from("/repo/crates"),
                    relative: "crates".to_string(),
                    package_area: "root".to_string(),
                    name: "crates".to_string(),
                    ecosystem: PackageEcosystem::Cargo,
                    discovery_sources: Vec::new(),
                    nested_packages: Vec::new(),
                    primary_language: None,
                    secondary_languages: Vec::new(),
                    languages: Vec::new(),
                    frameworks: Vec::new(),
                    file_associations: Vec::new(),
                    configuration: Vec::new(),
                    documentation: Vec::new(),
                    editor_config: None,
                    command_runner: Vec::new(),
                    package_managers: Vec::new(),
                    version: None,
                    features: Vec::new(),
                    depends_on: Vec::new(),
                    used_by: Vec::new(),
                    dependencies: None,
                    dev_dependencies: None,
                    peer_dependencies: None,
                    optional_dependencies: None,
                    is_updatable: None,
                    has_major_update: None,
                    is_excluded: false,
                },
                Package {
                    path: PathBuf::from("/repo/crates/pkg-a"),
                    relative: "crates/pkg-a".to_string(),
                    package_area: "crates".to_string(),
                    name: "pkg-a".to_string(),
                    ecosystem: PackageEcosystem::Cargo,
                    discovery_sources: Vec::new(),
                    nested_packages: Vec::new(),
                    primary_language: None,
                    secondary_languages: Vec::new(),
                    languages: Vec::new(),
                    frameworks: Vec::new(),
                    file_associations: Vec::new(),
                    configuration: Vec::new(),
                    documentation: Vec::new(),
                    editor_config: None,
                    command_runner: Vec::new(),
                    package_managers: Vec::new(),
                    version: None,
                    features: Vec::new(),
                    depends_on: Vec::new(),
                    used_by: Vec::new(),
                    dependencies: None,
                    dev_dependencies: None,
                    peer_dependencies: None,
                    optional_dependencies: None,
                    is_updatable: None,
                    has_major_update: None,
                    is_excluded: false,
                },
            ]),
        };

        assert_eq!(
            repo.package_for_dir(Path::new("/repo/crates/pkg-a/src"))
                .map(|p| p.name.as_str()),
            Some("pkg-a")
        );
    }
}
