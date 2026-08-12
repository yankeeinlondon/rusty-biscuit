//! Cheap package-boundary descriptors, deduplicated before enrichment.
//!
//! A [`PackageSeed`] is what a workspace detector resolves: a directory that is
//! a package boundary, plus who says so. Producing one reads no file, parses no
//! manifest, and runs no test-runner search — it costs one normalization of a
//! path the detector already owns.
//!
//! ## Notes
//!
//! Seeds exist so deduplication happens *before* the expensive step rather than
//! after it. Detectors legitimately resolve the same boundary more than once —
//! overlapping globs, a nested marker under a globbed member, a manifest scan
//! over an already-declared member — and enriching each occurrence and merging
//! afterwards discards the duplicate's entire cost. Merging seeds first makes
//! that class of waste unrepresentable rather than merely absent; see the Phase
//! 4 sub-spec of the `2026-07-16-performance` feature.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use super::detection::canonicalize_path;
use super::manifest_index::ManifestKind;
use super::standard::{MonorepoStandard, PackageProvenance};

/// One package boundary, before enrichment.
#[derive(Debug, Clone)]
pub(crate) struct PackageSeed {
    /// Native path exactly as discovered; the enrichment input.
    pub(crate) path: PathBuf,
    /// Normalized absolute comparison key. See [`normalized_key`].
    pub(crate) key: PathBuf,
    /// The root the producing detector resolved this boundary against.
    ///
    /// Not the repo root for a nested workspace, and the distinction is
    /// load-bearing: Cargo `version.workspace = true` and npm's root-version
    /// fallback both resolve against the *owning workspace's* manifest, and the
    /// `Cargo.lock` that resolves a member's dependency versions is that
    /// workspace's. Enrichment therefore runs in this frame, and only
    /// `relative` is re-framed to the catalog's repo-root view.
    pub(crate) owner_root: PathBuf,
    /// Path relative to the catalog frame (repo root once rebased).
    pub(crate) relative: String,
    pub(crate) standard: MonorepoStandard,
    pub(crate) provenance: PackageProvenance,
    /// Manifest kinds observed at this boundary.
    ///
    /// Presence is proof; absence is not. The observation index deliberately
    /// omits generated and fixture manifests, so an empty set means "nothing
    /// observed here", never "no manifest exists".
    pub(crate) evidence: BTreeSet<ManifestKind>,
    pub(crate) is_excluded: bool,
}

impl PackageSeed {
    /// Resolve a boundary at `path`, framed relative to `root`.
    pub(crate) fn new(
        path: &Path,
        root: &Path,
        standard: MonorepoStandard,
        provenance: PackageProvenance,
    ) -> Self {
        Self {
            key: normalized_key(path),
            relative: super::detection::make_relative_path(path, root),
            path: path.to_path_buf(),
            owner_root: root.to_path_buf(),
            standard,
            provenance,
            evidence: BTreeSet::new(),
            is_excluded: false,
        }
    }

    /// Re-frame `relative` against `new_root`, deriving it from the absolute
    /// path.
    ///
    /// sniff keeps two path frames explicit: a detector resolves its packages
    /// relative to its own layer root, while the flat `RepoInfo.packages`
    /// catalog is repo-root-relative. This is the boundary between them.
    /// `owner_root` deliberately does not move — see its docs.
    pub(crate) fn rebase_to_root(&mut self, new_root: &Path) {
        self.relative = super::detection::make_relative_path(&self.path, new_root);
    }
}

/// The one operation that turns a path into a boundary comparison key.
///
/// ## Notes
///
/// Canonicalizes, falling back to lexical normalization when the path does not
/// resolve — matching the resolved-symlink semantics package merging has always
/// had, since `merge_packages` keyed on `canonicalize_path`. Native encoding and
/// separators survive: the key stays a `PathBuf` and is never routed through a
/// lossy string. Windows drive prefixes are normalized by `Component::Prefix`
/// handling in the lexical fallback rather than by ad hoc case folding, so
/// existing per-platform case behavior is preserved.
pub(crate) fn normalized_key(path: &Path) -> PathBuf {
    canonicalize_path(path)
}

/// Merge seeds that name the same boundary, keeping the first occurrence's
/// identity.
///
/// ## Notes
///
/// The precedence here reproduces `merge_package_into`'s, so collapsing before
/// enrichment yields the same catalog collapsing after it did: a non-`Unknown`
/// standard wins over `Unknown` and carries its provenance along, `is_excluded`
/// is the OR across occurrences, and evidence is the union. First-seen `path`
/// and `relative` win, which preserves detector ordering.
pub(crate) fn merge_seeds(seeds: Vec<PackageSeed>) -> Vec<PackageSeed> {
    let mut merged: Vec<PackageSeed> = Vec::new();
    let mut index_by_key: std::collections::HashMap<PathBuf, usize> =
        std::collections::HashMap::new();

    for seed in seeds {
        match index_by_key.get(&seed.key).copied() {
            Some(index) => merge_seed_into(&mut merged[index], seed),
            None => {
                index_by_key.insert(seed.key.clone(), merged.len());
                merged.push(seed);
            }
        }
    }

    merged
}

fn merge_seed_into(existing: &mut PackageSeed, incoming: PackageSeed) {
    if existing.standard == MonorepoStandard::Unknown
        && incoming.standard != MonorepoStandard::Unknown
    {
        existing.standard = incoming.standard;
        existing.provenance = incoming.provenance;
    }
    existing.is_excluded |= incoming.is_excluded;
    existing.evidence.extend(incoming.evidence);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seed(path: &str, standard: MonorepoStandard) -> PackageSeed {
        PackageSeed {
            path: PathBuf::from(path),
            key: PathBuf::from(path),
            owner_root: PathBuf::from("/repo"),
            relative: path.trim_start_matches("/repo/").to_string(),
            standard,
            provenance: PackageProvenance::ManifestScan,
            evidence: BTreeSet::new(),
            is_excluded: false,
        }
    }

    /// The R5 contract: two detectors resolving one boundary collapse to one
    /// seed, so enrichment runs once rather than twice-then-deduped.
    #[test]
    fn merge_seeds_collapses_the_same_boundary() {
        let merged = merge_seeds(vec![
            seed("/repo/crates/a", MonorepoStandard::CargoWorkspace),
            seed("/repo/crates/a", MonorepoStandard::Unknown),
            seed("/repo/crates/b", MonorepoStandard::CargoWorkspace),
        ]);

        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].path, PathBuf::from("/repo/crates/a"));
        assert_eq!(merged[1].path, PathBuf::from("/repo/crates/b"));
    }

    /// A real authority must survive a merge with an `Unknown` manifest-scan
    /// occurrence regardless of arrival order, and its provenance travels with
    /// it — the same precedence `merge_package_into` applied post-enrichment.
    #[test]
    fn merge_seeds_prefers_a_real_authority_over_unknown_either_way() {
        let mut authority = seed("/repo/crates/a", MonorepoStandard::CargoWorkspace);
        authority.provenance = PackageProvenance::Globbed;

        let scan_first = merge_seeds(vec![
            seed("/repo/crates/a", MonorepoStandard::Unknown),
            authority.clone(),
        ]);
        assert_eq!(scan_first[0].standard, MonorepoStandard::CargoWorkspace);
        assert_eq!(scan_first[0].provenance, PackageProvenance::Globbed);

        let authority_first = merge_seeds(vec![
            authority,
            seed("/repo/crates/a", MonorepoStandard::Unknown),
        ]);
        assert_eq!(
            authority_first[0].standard,
            MonorepoStandard::CargoWorkspace
        );
        assert_eq!(authority_first[0].provenance, PackageProvenance::Globbed);
    }

    /// Exclusion is sticky: a member matched by both an include and an exclude
    /// pattern stays excluded no matter which seed arrives first.
    #[test]
    fn merge_seeds_ors_exclusion() {
        let mut excluded = seed("/repo/crates/a", MonorepoStandard::CargoWorkspace);
        excluded.is_excluded = true;

        let merged = merge_seeds(vec![
            seed("/repo/crates/a", MonorepoStandard::CargoWorkspace),
            excluded,
        ]);

        assert_eq!(merged.len(), 1);
        assert!(merged[0].is_excluded);
    }

    #[test]
    fn merge_seeds_unions_evidence() {
        let mut cargo = seed("/repo/crates/a", MonorepoStandard::CargoWorkspace);
        cargo.evidence.insert(ManifestKind::Cargo);
        let mut node = seed("/repo/crates/a", MonorepoStandard::CargoWorkspace);
        node.evidence.insert(ManifestKind::Node);

        let merged = merge_seeds(vec![cargo, node]);

        assert_eq!(merged.len(), 1);
        assert!(merged[0].evidence.contains(&ManifestKind::Cargo));
        assert!(merged[0].evidence.contains(&ManifestKind::Node));
    }

    /// Ownership and merging compare whole components, so a sibling sharing a
    /// textual prefix is a different boundary.
    #[test]
    fn sibling_sharing_a_textual_prefix_is_a_distinct_boundary() {
        let merged = merge_seeds(vec![
            seed("/repo/crates/pkg-a", MonorepoStandard::CargoWorkspace),
            seed("/repo/crates/pkg-a2", MonorepoStandard::CargoWorkspace),
        ]);

        assert_eq!(merged.len(), 2);
        // Keys compare by whole component, so the shared textual prefix does not
        // make `pkg-a` an ancestor of `pkg-a2` — the hazard R6 names.
        assert!(!Path::new("/repo/crates/pkg-a2").starts_with("/repo/crates/pkg-a"));
        assert!(Path::new("/repo/crates/pkg-a/src").starts_with("/repo/crates/pkg-a"));
    }
}
