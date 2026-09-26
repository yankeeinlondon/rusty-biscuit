//! Manifest indexing and package discovery from filesystem walks.
//!
//! Provides:
//! - [`CargoLockVersions`] — resolved versions extracted from `Cargo.lock`.
//! - [`ManifestIndex`] — fast index of manifest-bearing directories under a tree.
//! - Package discovery helpers that build [`Package`] values from manifest paths.

use std::collections::{HashMap, HashSet};
use std::fmt;
use std::path::{Path, PathBuf};

use biscuit_file::toml_crate;
use serde::de::{self, IgnoredAny, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};
use tracing::debug;

use crate::performance;
use crate::performance::counters;

use super::detection::normalize_path;
use super::seed::PackageSeed;
use super::standard::{MonorepoStandard, PackageProvenance};

/// Resolved versions from Cargo.lock.
pub(crate) struct CargoLockVersions {
    versions: HashMap<String, Vec<String>>,
}

impl CargoLockVersions {
    /// Parse a Cargo.lock file and extract package versions.
    ///
    /// Only `package[].name` and `package[].version` are retained, with every
    /// version of a name kept in lockfile order. Entries that are not tables,
    /// or whose `name` or `version` is absent or not a string, are skipped; a
    /// missing or non-array `package` yields an empty index.
    ///
    /// ## Returns
    ///
    /// `None` only when the file cannot be read or is not valid TOML.
    pub fn parse(lock_path: &Path) -> Option<Self> {
        performance::increment_counter(counters::REPO_LOCKFILE_READS, 1);
        performance::increment_counter(counters::FS_FILE_OPENS, 1);
        let content = std::fs::read_to_string(lock_path)
            .map_err(|e| {
                debug!(path = %lock_path.display(), error = %e, "could not read file");
                e
            })
            .ok()?;
        performance::increment_counter(counters::FS_BYTES_READ, content.len() as u64);
        performance::increment_counter(counters::REPO_LOCKFILE_PARSES, 1);
        Self::from_lock_str(&content)
    }

    fn from_lock_str(content: &str) -> Option<Self> {
        let lock: CargoLockDocument = toml_crate::from_str(content).ok()?;

        let mut versions: HashMap<String, Vec<String>> = HashMap::new();
        for entry in lock.package.0 {
            if let (Some(name), Some(version)) = (entry.name, entry.version) {
                versions.entry(name).or_default().push(version);
            }
        }

        Some(Self { versions })
    }

    /// The generic `toml::Value` parser that [`Self::from_lock_str`] replaced,
    /// kept as the parity oracle.
    #[cfg(test)]
    fn parse_reference(content: &str) -> Option<Self> {
        let parsed: toml_crate::Value = toml_crate::from_str(content).ok()?;

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

/// The part of a `Cargo.lock` document that [`CargoLockVersions`] retains.
///
/// Every other key (`version`, `metadata`, and each entry's `source`,
/// `checksum`, and `dependencies`) is skipped through `IgnoredAny` rather than
/// allocated. Each level is lenient, matching the generic parser this replaced:
/// a value of the wrong shape becomes an absent value instead of an error.
#[derive(Deserialize)]
struct CargoLockDocument {
    #[serde(default)]
    package: LockedPackages,
}

#[derive(Default)]
struct LockedPackages(Vec<LockedPackage>);

#[derive(Deserialize)]
struct LockedPackage {
    #[serde(default, deserialize_with = "string_or_none")]
    name: Option<String>,
    #[serde(default, deserialize_with = "string_or_none")]
    version: Option<String>,
}

/// One `package` array element; `None` when the element is not a table.
struct LockedPackageEntry(Option<LockedPackage>);

fn string_or_none<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<String>, D::Error> {
    struct StringOrNone;

    impl<'de> Visitor<'de> for StringOrNone {
        type Value = Option<String>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("any TOML value")
        }

        fn visit_str<E: de::Error>(self, value: &str) -> Result<Self::Value, E> {
            Ok(Some(value.to_owned()))
        }

        fn visit_string<E: de::Error>(self, value: String) -> Result<Self::Value, E> {
            Ok(Some(value))
        }

        fn visit_bool<E: de::Error>(self, _: bool) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_i64<E: de::Error>(self, _: i64) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_u64<E: de::Error>(self, _: u64) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_f64<E: de::Error>(self, _: f64) -> Result<Self::Value, E> {
            Ok(None)
        }

        fn visit_seq<A: SeqAccess<'de>>(self, seq: A) -> Result<Self::Value, A::Error> {
            IgnoredAny.visit_seq(seq)?;
            Ok(None)
        }

        // Also reached by TOML datetimes, which serde sees as a map.
        fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<Self::Value, A::Error> {
            IgnoredAny.visit_map(map)?;
            Ok(None)
        }
    }

    deserializer.deserialize_any(StringOrNone)
}

impl<'de> Deserialize<'de> for LockedPackageEntry {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct EntryVisitor;

        impl<'de> Visitor<'de> for EntryVisitor {
            type Value = LockedPackageEntry;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("any TOML value")
            }

            fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<Self::Value, A::Error> {
                LockedPackage::deserialize(de::value::MapAccessDeserializer::new(map))
                    .map(|package| LockedPackageEntry(Some(package)))
            }

            fn visit_str<E: de::Error>(self, _: &str) -> Result<Self::Value, E> {
                Ok(LockedPackageEntry(None))
            }

            fn visit_bool<E: de::Error>(self, _: bool) -> Result<Self::Value, E> {
                Ok(LockedPackageEntry(None))
            }

            fn visit_i64<E: de::Error>(self, _: i64) -> Result<Self::Value, E> {
                Ok(LockedPackageEntry(None))
            }

            fn visit_u64<E: de::Error>(self, _: u64) -> Result<Self::Value, E> {
                Ok(LockedPackageEntry(None))
            }

            fn visit_f64<E: de::Error>(self, _: f64) -> Result<Self::Value, E> {
                Ok(LockedPackageEntry(None))
            }

            fn visit_seq<A: SeqAccess<'de>>(self, seq: A) -> Result<Self::Value, A::Error> {
                IgnoredAny.visit_seq(seq)?;
                Ok(LockedPackageEntry(None))
            }
        }

        deserializer.deserialize_any(EntryVisitor)
    }
}

impl<'de> Deserialize<'de> for LockedPackages {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct PackagesVisitor;

        impl<'de> Visitor<'de> for PackagesVisitor {
            type Value = LockedPackages;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("any TOML value")
            }

            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut packages = Vec::with_capacity(seq.size_hint().unwrap_or(0));
                while let Some(LockedPackageEntry(entry)) = seq.next_element()? {
                    packages.extend(entry);
                }
                Ok(LockedPackages(packages))
            }

            fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<Self::Value, A::Error> {
                IgnoredAny.visit_map(map)?;
                Ok(LockedPackages::default())
            }

            fn visit_str<E: de::Error>(self, _: &str) -> Result<Self::Value, E> {
                Ok(LockedPackages::default())
            }

            fn visit_bool<E: de::Error>(self, _: bool) -> Result<Self::Value, E> {
                Ok(LockedPackages::default())
            }

            fn visit_i64<E: de::Error>(self, _: i64) -> Result<Self::Value, E> {
                Ok(LockedPackages::default())
            }

            fn visit_u64<E: de::Error>(self, _: u64) -> Result<Self::Value, E> {
                Ok(LockedPackages::default())
            }

            fn visit_f64<E: de::Error>(self, _: f64) -> Result<Self::Value, E> {
                Ok(LockedPackages::default())
            }
        }

        deserializer.deserialize_any(PackagesVisitor)
    }
}

/// Manifest file type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
/// [`ManifestIndex::package_dirs_in_tree`] queries do not perform any syscalls.
///
/// ## Notes
///
/// `entries` is sorted by `canonical` at construction, which is what lets a
/// subtree query resolve as a binary-searched contiguous range instead of a scan
/// over every manifest in the repository. On a 375-package workspace the scan
/// form is O(packages²).
#[derive(Debug, Clone)]
pub(crate) struct ManifestIndex {
    /// Sorted by `canonical`. Invariant relied on by `subtree_range`.
    pub(crate) entries: Vec<ManifestEntry>,
}

impl ManifestIndex {
    /// Build manifest index by walking the directory tree once.
    pub(crate) fn build(root: &Path) -> Self {
        use crate::filesystem::file_types::should_skip_directory_name;
        use ignore::{WalkBuilder, WalkState};
        use std::sync::{Arc, Mutex};

        /// Per-worker state for one `build_parallel` thread.
        ///
        /// The collector is what makes this worker's reads visible: recording
        /// writes to a thread-local buffer that only a `WorkerCollector`'s drop
        /// drains, so without one every `is_generated_manifest` read this
        /// worker performs would be silently discarded from the report.
        struct ManifestWorker {
            shared: Arc<Mutex<Vec<(PathBuf, ManifestKind)>>>,
            collector: performance::WorkerCollector,
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

        performance::increment_counter(counters::FS_READ_DIRS, 1);
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
                    collector: performance::WorkerCollector::inherit(),
                    local: Vec::new(),
                };
                Box::new(move |result| {
                    // The first callback is the earliest point that runs on
                    // this worker's own thread, which is where the collector
                    // must live.
                    worker.collector.activate();
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
        let mut entries: Vec<ManifestEntry> = grouped
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
        entries.sort_by(|a, b| a.canonical.cmp(&b.canonical));
        Self { entries }
    }

    /// The contiguous entry range whose canonical paths lie under `search_root`.
    ///
    /// ## Notes
    ///
    /// Sound because `entries` is sorted by `canonical` and `Path`'s ordering is
    /// componentwise: every descendant of `search_root` sorts at or after it, and
    /// the first non-descendant ends the run. The membership test stays
    /// `Path::starts_with` (componentwise), so a sibling sharing a textual prefix
    /// — `crates/pkg-a2` against `crates/pkg-a` — is excluded rather than
    /// silently claimed, which a string-prefix range would get wrong.
    fn subtree_range(&self, search_root: &Path) -> &[ManifestEntry] {
        let start = self
            .entries
            .partition_point(|entry| entry.canonical.as_path() < search_root);
        let tail = &self.entries[start..];
        let len = tail.partition_point(|entry| entry.canonical.starts_with(search_root));
        &tail[..len]
    }

    /// Get directories containing manifests within a specific subtree.
    ///
    /// Uses the pre-canonicalized entries built at index construction time, so
    /// no filesystem syscalls occur during the query. The `search_root` and
    /// `root` parameters are lexically normalized (not canonicalized) so the
    /// comparison is syscall-free.
    pub(crate) fn package_dirs_in_tree(&self, search_root: &Path, root: &Path) -> Vec<&Path> {
        let search_root_normalized = normalize_path(search_root);
        let root_normalized = normalize_path(root);

        self.subtree_range(&search_root_normalized)
            .iter()
            .filter(|entry| entry.canonical != root_normalized)
            .map(|entry| entry.original.as_path())
            .collect()
    }

    /// The manifest kinds observed at `dir`, empty when nothing was observed.
    ///
    /// Presence is proof; absence is not. The index omits generated and fixture
    /// manifests by design, so an empty set never means "no manifest exists".
    pub(crate) fn kinds_at(&self, dir: &Path) -> HashSet<ManifestKind> {
        let normalized = normalize_path(dir);
        self.entries
            .binary_search_by(|entry| entry.canonical.cmp(&normalized))
            .map(|index| self.entries[index].kinds.clone())
            .unwrap_or_default()
    }
}

pub(crate) fn is_generated_manifest(path: &Path) -> bool {
    let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
        return false;
    };

    if !matches!(file_name, "Cargo.toml" | "pyproject.toml") {
        return false;
    }

    performance::increment_counter(counters::FS_FILE_OPENS, 1);
    let Ok(content) = std::fs::read_to_string(path) else {
        return false;
    };
    performance::increment_counter(counters::FS_BYTES_READ, content.len() as u64);
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

/// Discover boundaries using the manifest index if available, otherwise walk.
pub(super) fn discover_seeds_with_optional_index(
    root: &Path,
    standard: MonorepoStandard,
    provenance: PackageProvenance,
    index: Option<&ManifestIndex>,
) -> Vec<PackageSeed> {
    match index {
        // `package_dirs_in_tree` excludes the root itself, matching
        // `discover_seeds_from_manifests_in_tree`'s `parent == search_root` skip.
        Some(idx) => idx
            .package_dirs_in_tree(root, root)
            .iter()
            .map(|path| seed_from_index(path, root, standard, provenance, idx))
            .collect(),
        None => discover_seeds_from_manifests_in_tree(root, root, standard, provenance),
    }
}

/// A seed carrying the manifest kinds the index observed at `path`.
pub(super) fn seed_from_index(
    path: &Path,
    root: &Path,
    standard: MonorepoStandard,
    provenance: PackageProvenance,
    index: &ManifestIndex,
) -> PackageSeed {
    let mut seed = PackageSeed::new(path, root, standard, provenance);
    seed.evidence = index.kinds_at(path).into_iter().collect();
    seed
}

pub(super) fn discover_seeds_from_manifests_in_tree(
    search_root: &Path,
    repo_root: &Path,
    standard: MonorepoStandard,
    provenance: PackageProvenance,
) -> Vec<PackageSeed> {
    let mut discovered_dirs = HashSet::new();

    performance::increment_counter(counters::FS_READ_DIRS, 1);
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
        .map(|path| PackageSeed::new(path, repo_root, standard, provenance))
        .collect()
}

/// Discover boundaries from the manifest index (optimized path).
///
/// Uses the pre-built manifest index instead of walking the filesystem.
pub(crate) fn discover_seeds_from_index(
    search_root: &Path,
    repo_root: &Path,
    standard: MonorepoStandard,
    provenance: PackageProvenance,
    index: &ManifestIndex,
) -> Vec<PackageSeed> {
    index
        .package_dirs_in_tree(search_root, repo_root)
        .iter()
        .map(|path| seed_from_index(path, repo_root, standard, provenance, index))
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

    type VersionIndex = HashMap<String, Vec<String>>;

    fn typed_and_reference(content: &str) -> (Option<VersionIndex>, Option<VersionIndex>) {
        (
            CargoLockVersions::from_lock_str(content).map(|lock| lock.versions),
            CargoLockVersions::parse_reference(content).map(|lock| lock.versions),
        )
    }

    fn expected_map(entries: &[(&str, &[&str])]) -> VersionIndex {
        entries
            .iter()
            .map(|(name, versions)| {
                (
                    (*name).to_string(),
                    versions.iter().map(|v| (*v).to_string()).collect(),
                )
            })
            .collect()
    }

    #[test]
    fn cargo_lock_typed_parse_keeps_duplicate_name_versions_in_lockfile_order() {
        let content = r#"
version = 4

[[package]]
name = "syn"
version = "2.0.100"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "abc"
dependencies = ["proc-macro2", "quote"]

[[package]]
name = "serde"
version = "1.0.200"

[[package]]
name = "syn"
version = "1.0.109"

[[package]]
name = "syn"
version = "2.0.50"

[metadata]
"checksum syn 1.0.109" = "def"
"#;

        let (typed, reference) = typed_and_reference(content);

        let expected = expected_map(&[
            ("syn", &["2.0.100", "1.0.109", "2.0.50"]),
            ("serde", &["1.0.200"]),
        ]);
        assert_eq!(typed.as_ref(), Some(&expected));
        assert_eq!(reference.as_ref(), Some(&expected));
        let typed_lock = CargoLockVersions::from_lock_str(content).expect("valid TOML");
        assert_eq!(typed_lock.resolve("syn"), Some("2.0.100".to_string()));
    }

    #[test]
    fn cargo_lock_typed_parse_skips_malformed_entries_like_reference() {
        let content = r#"
package = [
    { version = "1.0.0" },
    { name = "int-version", version = 3 },
    { name = 7, version = "1.0.0" },
    { name = "table-version", version = { major = 1 } },
    { name = "date-version", version = 1979-05-27 },
    "not-a-table",
    42,
    ["nested", "array"],
    1979-05-27,
    { name = "no-version" },
    { name = "kept", version = "0.1.0", checksum = "abc" },
]
"#;

        let (typed, reference) = typed_and_reference(content);

        let expected = expected_map(&[("kept", &["0.1.0"])]);
        assert_eq!(typed.as_ref(), Some(&expected));
        assert_eq!(reference.as_ref(), Some(&expected));
    }

    #[test]
    fn cargo_lock_typed_parse_treats_non_array_package_as_empty_like_reference() {
        for content in [
            "version = 4\n",
            "package = \"serde\"\n",
            "package = 1\n",
            "package = 1979-05-27\n",
            "[package]\nname = \"serde\"\nversion = \"1.0.0\"\n",
        ] {
            let (typed, reference) = typed_and_reference(content);

            assert_eq!(typed, Some(HashMap::new()), "typed parse of {content:?}");
            assert_eq!(
                reference,
                Some(HashMap::new()),
                "reference parse of {content:?}"
            );
        }
    }

    #[test]
    fn cargo_lock_typed_parse_rejects_invalid_toml_like_reference() {
        for content in [
            "[[package]\nname = \"serde\"\n",
            "[[package]]\nname = \"serde\"\nname = \"dup\"\n",
            "package = [",
        ] {
            let (typed, reference) = typed_and_reference(content);

            assert_eq!(typed, None, "typed parse of {content:?}");
            assert_eq!(reference, None, "reference parse of {content:?}");
        }
    }

    #[test]
    fn cargo_lock_typed_parse_matches_reference_on_workspace_lockfile() {
        let content = include_str!("../../../../../Cargo.lock");

        let (typed, reference) = typed_and_reference(content);

        let typed = typed.expect("workspace Cargo.lock parses");
        assert!(
            typed.contains_key("sniff"),
            "workspace member missing from typed index"
        );
        assert_eq!(Some(typed), reference);
    }
}
