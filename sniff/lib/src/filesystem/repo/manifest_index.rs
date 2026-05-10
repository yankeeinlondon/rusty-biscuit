//! Manifest indexing and package discovery from filesystem walks.
//!
//! Provides:
//! - [`CargoLockVersions`] — resolved versions extracted from `Cargo.lock`.
//! - [`ManifestIndex`] — fast index of manifest-bearing directories under a tree.
//! - Package discovery helpers that build [`Package`] values from manifest paths.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use biscuit_file::toml_crate;
use tracing::debug;

use super::detection::{create_package, normalize_path};
use super::types::{MonorepoTool, Package, PackageDiscoverySource};

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
pub(crate) enum ManifestKind {
    Cargo,
    Node,
    Python,
    Go,
}

/// Manifest entry storing both the original path and its canonicalized form.
///
/// Pre-normalizing at index-build time eliminates per-query `canonicalize`
/// syscalls during `package_dirs_in_tree` lookups.
#[derive(Debug, Clone)]
pub(crate) struct ManifestEntry {
    /// Original parent directory path (as discovered).
    pub(crate) original: PathBuf,
    /// Canonicalized parent directory path (resolved at build time).
    pub(crate) canonical: PathBuf,
    /// Kinds of manifests detected in this directory.
    #[allow(dead_code)]
    pub(crate) kinds: HashSet<ManifestKind>,
}

/// Index of all manifest files in a directory tree.
///
/// Performs a single directory walk and caches manifest locations,
/// avoiding redundant filesystem traversals during package discovery.
/// Each entry is canonicalized once at build time so that downstream
/// `package_dirs_in_tree` queries do not perform any syscalls.
#[derive(Debug, Clone)]
pub(crate) struct ManifestIndex {
    pub(crate) entries: Vec<ManifestEntry>,
}

impl ManifestIndex {
    /// Build manifest index by walking the directory tree once.
    pub(crate) fn build(root: &Path) -> Self {
        use crate::filesystem::file_types::should_skip_directory_name;
        use ignore::{WalkBuilder, WalkState};
        use std::sync::{Arc, Mutex};

        struct ManifestWorker {
            shared: Arc<Mutex<Vec<(PathBuf, ManifestKind)>>>,
            local: Vec<(PathBuf, ManifestKind)>,
        }

        impl Drop for ManifestWorker {
            fn drop(&mut self) {
                if self.local.is_empty() {
                    return;
                }
                let mut shared = self
                    .shared
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                shared.append(&mut self.local);
            }
        }

        let shared: Arc<Mutex<Vec<(PathBuf, ManifestKind)>>> = Arc::new(Mutex::new(Vec::new()));

        WalkBuilder::new(root)
            .hidden(false)
            .git_ignore(true)
            .git_global(true)
            .git_exclude(true)
            .filter_entry(|entry| {
                if !entry.file_type().is_some_and(|ft| ft.is_dir()) {
                    return true;
                }
                !entry
                    .file_name()
                    .to_str()
                    .is_some_and(should_skip_directory_name)
            })
            .build_parallel()
            .run(|| {
                let mut worker = ManifestWorker {
                    shared: Arc::clone(&shared),
                    local: Vec::new(),
                };
                Box::new(move |result| {
                    let Ok(entry) = result else {
                        return WalkState::Continue;
                    };
                    if !entry.file_type().is_some_and(|ft| ft.is_file()) {
                        return WalkState::Continue;
                    }

                    let file_name = entry.file_name().to_string_lossy();
                    let kind = match file_name.as_ref() {
                        "Cargo.toml" => ManifestKind::Cargo,
                        "package.json" => ManifestKind::Node,
                        "pyproject.toml" => ManifestKind::Python,
                        "go.mod" => ManifestKind::Go,
                        _ => return WalkState::Continue,
                    };

                    if is_generated_manifest(entry.path()) {
                        return WalkState::Continue;
                    }
                    if is_fixture_manifest(entry.path()) {
                        return WalkState::Continue;
                    }

                    let Some(parent) = entry.path().parent() else {
                        return WalkState::Continue;
                    };

                    worker.local.push((parent.to_path_buf(), kind));
                    WalkState::Continue
                })
            });

        let mut accumulated = shared
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let mut grouped: HashMap<PathBuf, HashSet<ManifestKind>> = HashMap::new();
        for (path, kind) in accumulated.drain(..) {
            grouped.entry(path).or_default().insert(kind);
        }
        Self::from_grouped(grouped)
    }

    pub(crate) fn from_manifest_paths(paths: impl IntoIterator<Item = PathBuf>) -> Self {
        let mut grouped: HashMap<PathBuf, HashSet<ManifestKind>> = HashMap::new();

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

            grouped
                .entry(parent.to_path_buf())
                .or_default()
                .insert(kind);
        }

        Self::from_grouped(grouped)
    }

    fn from_grouped(grouped: HashMap<PathBuf, HashSet<ManifestKind>>) -> Self {
        let entries = grouped
            .into_iter()
            .map(|(original, kinds)| {
                let canonical = normalize_path(&original);
                ManifestEntry {
                    original,
                    canonical,
                    kinds,
                }
            })
            .collect();
        Self { entries }
    }

    /// Get directories containing manifests within a specific subtree.
    ///
    /// Uses the pre-canonicalized entries built at index construction time, so
    /// no filesystem syscalls occur during the query.  The `search_root` and
    /// `root` parameters are lexically normalized (not canonicalized) so the
    /// comparison is syscall-free.
    pub(crate) fn package_dirs_in_tree(&self, search_root: &Path, root: &Path) -> Vec<&Path> {
        let search_root_normalized = normalize_path(search_root);
        let root_normalized = normalize_path(root);

        let mut dirs: Vec<&Path> = self
            .entries
            .iter()
            .filter_map(|entry| {
                if entry.canonical.starts_with(&search_root_normalized)
                    && entry.canonical != root_normalized
                {
                    Some(entry.original.as_path())
                } else {
                    None
                }
            })
            .collect();

        dirs.sort();
        dirs
    }
}

pub(crate) fn is_generated_manifest(path: &Path) -> bool {
    let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };

    if !matches!(file_name, "Cargo.toml" | "pyproject.toml") {
        return false;
    }

    let Ok(content) = std::fs::read_to_string(path) else {
        return false;
    };
    let content_lower = content.to_lowercase();

    content_lower.contains("automatically generated")
        || content_lower.contains("auto-generated")
        || content_lower.contains("do not edit manually")
}

pub(crate) fn is_fixture_manifest(path: &Path) -> bool {
    let components: Vec<String> = path
        .components()
        .filter_map(|component| component.as_os_str().to_str().map(|s| s.to_lowercase()))
        .collect();

    if components
        .iter()
        .any(|component| matches!(component.as_str(), "__fixtures__" | "testdata"))
    {
        return true;
    }

    components.windows(2).any(|window| {
        matches!(window[0].as_str(), "test" | "tests" | "spec" | "specs") && window[1] == "fixtures"
    })
}

pub(super) fn discover_packages_from_manifests(
    root: &Path,
    tool: MonorepoTool,
    lock_versions: &Option<CargoLockVersions>,
    discovery_source: PackageDiscoverySource,
) -> Vec<Package> {
    discover_packages_from_manifests_in_tree(root, root, tool, lock_versions, discovery_source)
}

/// Discover packages using manifest index if available, otherwise walk filesystem.
pub(super) fn discover_packages_with_optional_index(
    root: &Path,
    tool: MonorepoTool,
    lock_versions: &Option<CargoLockVersions>,
    discovery_source: PackageDiscoverySource,
    index: Option<&ManifestIndex>,
) -> Vec<Package> {
    if let Some(idx) = index {
        // Use package_dirs_in_tree to exclude root itself (matches original
        // discover_packages_from_manifests_in_tree which skips search_root)
        idx.package_dirs_in_tree(root, root)
            .iter()
            .map(|path| create_package(path, root, tool, lock_versions, discovery_source))
            .collect()
    } else {
        discover_packages_from_manifests(root, tool, lock_versions, discovery_source)
    }
}

pub(super) fn discover_packages_from_manifests_in_tree(
    search_root: &Path,
    repo_root: &Path,
    tool: MonorepoTool,
    lock_versions: &Option<CargoLockVersions>,
    discovery_source: PackageDiscoverySource,
) -> Vec<Package> {
    let mut discovered_dirs = HashSet::new();

    let walker = walkdir::WalkDir::new(search_root)
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

    for entry in walker.filter_map(|entry| entry.ok()) {
        if !entry.file_type().is_file() {
            continue;
        }

        let file_name = entry.file_name().to_string_lossy();
        let is_manifest = matches!(
            file_name.as_ref(),
            "Cargo.toml" | "package.json" | "pyproject.toml" | "go.mod"
        );
        if !is_manifest {
            continue;
        }
        if is_generated_manifest(entry.path()) {
            continue;
        }
        if is_fixture_manifest(entry.path()) {
            continue;
        }

        let Some(parent) = entry.path().parent() else {
            continue;
        };
        if parent == search_root {
            continue;
        }
        discovered_dirs.insert(parent.to_path_buf());
    }

    let mut dirs: Vec<PathBuf> = discovered_dirs.into_iter().collect();
    dirs.sort();

    dirs.iter()
        .map(|path| create_package(path, repo_root, tool, lock_versions, discovery_source))
        .collect()
}

/// Discover packages from manifest index (optimized path).
///
/// Uses pre-built manifest index instead of walking the filesystem.
pub(crate) fn discover_packages_from_index(
    search_root: &Path,
    repo_root: &Path,
    tool: MonorepoTool,
    lock_versions: &Option<CargoLockVersions>,
    discovery_source: PackageDiscoverySource,
    index: &ManifestIndex,
) -> Vec<Package> {
    let dirs = index.package_dirs_in_tree(search_root, repo_root);

    dirs.iter()
        .map(|path| create_package(path, repo_root, tool, lock_versions, discovery_source))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================================
    // ManifestIndex normalization tests (Phase 3)
    // ============================================================================

    #[test]
    fn manifest_index_package_dirs_in_tree_uses_normalized_paths() {
        let index = ManifestIndex::from_manifest_paths(vec![
            PathBuf::from("/repo/crates/pkg-a/Cargo.toml"),
            PathBuf::from("/repo/crates/pkg-b/Cargo.toml"),
            PathBuf::from("/repo/apps/app-a/package.json"),
            PathBuf::from("/repo/vendor/some-lib/Cargo.toml"),
        ]);

        let search_root = Path::new("/repo/crates/../crates");
        let root = Path::new("/repo");
        let dirs = index.package_dirs_in_tree(search_root, root);

        assert_eq!(dirs.len(), 2);
        assert!(dirs.contains(&Path::new("/repo/crates/pkg-a")));
        assert!(dirs.contains(&Path::new("/repo/crates/pkg-b")));
    }

    #[test]
    fn manifest_index_package_dirs_excludes_root() {
        let index = ManifestIndex::from_manifest_paths(vec![
            PathBuf::from("/repo/Cargo.toml"),
            PathBuf::from("/repo/crates/pkg-a/Cargo.toml"),
        ]);

        let dirs = index.package_dirs_in_tree(Path::new("/repo"), Path::new("/repo"));

        assert_eq!(dirs.len(), 1);
        assert!(dirs.contains(&Path::new("/repo/crates/pkg-a")));
    }

    #[test]
    fn manifest_index_package_dirs_returns_empty_for_no_match() {
        let index = ManifestIndex::from_manifest_paths(vec![PathBuf::from(
            "/repo/crates/pkg-a/Cargo.toml",
        )]);

        let dirs = index.package_dirs_in_tree(Path::new("/other"), Path::new("/repo"));
        assert!(dirs.is_empty());
    }

    #[test]
    fn manifest_index_preserves_original_paths_in_output() {
        let index = ManifestIndex::from_manifest_paths(vec![PathBuf::from(
            "/repo/crates/pkg-a/Cargo.toml",
        )]);

        let dirs = index.package_dirs_in_tree(Path::new("/repo"), Path::new("/repo"));
        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0], Path::new("/repo/crates/pkg-a"));
    }

    #[test]
    fn manifest_index_build_deduplicates_same_dir_different_manifests() {
        let index = ManifestIndex::from_manifest_paths(vec![
            PathBuf::from("/repo/crates/pkg-a/Cargo.toml"),
            PathBuf::from("/repo/crates/pkg-a/package.json"),
        ]);

        assert_eq!(index.entries.len(), 1);
        let entry = &index.entries[0];
        assert_eq!(entry.original, PathBuf::from("/repo/crates/pkg-a"));
        assert!(entry.kinds.contains(&ManifestKind::Cargo));
        assert!(entry.kinds.contains(&ManifestKind::Node));
    }

    // ============================================================================
    // CargoLockVersions tests
    // ============================================================================

    #[test]
    fn cargo_lock_versions_resolve_finds_first_match() {
        let versions = CargoLockVersions {
            versions: {
                let mut m = HashMap::new();
                m.insert(
                    "serde".to_string(),
                    vec!["1.0.0".to_string(), "1.0.1".to_string()],
                );
                m
            },
        };

        assert_eq!(versions.resolve("serde"), Some("1.0.0".to_string()));
        assert_eq!(versions.resolve("missing"), None);
    }
}
