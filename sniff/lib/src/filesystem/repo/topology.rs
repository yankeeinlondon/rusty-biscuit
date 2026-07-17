//! Monorepo topology builder: turns per-detector results into membership
//! [`MonorepoLayer`]s and derives the honest `is_monorepo` predicate.
//!
//! Each detector reports a single [`MonorepoStandard`] plus the packages its
//! membership model resolved. This module groups those outcomes by root, splits
//! each root's standards into the authority that declares membership and the
//! orchestrators riding on top, and decides whether the resulting forest is
//! rich enough to call the repo a monorepo.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use biscuit_file::toml_crate;

use crate::performance;
use crate::performance::counters;

use super::detection::probe_exists;
use super::standard::{
    DetectedStandard, DetectionConfidence, MonorepoLayer, MonorepoStandard, RootMembership,
};
use super::seed::PackageSeed;

/// One detector's contribution to the topology: the standard it matched, the
/// root its marker lives at, and the boundaries its membership model resolved.
///
/// ## Notes
///
/// Carries [`PackageSeed`]s rather than enriched `Package`s so the topology is
/// built — and duplicate boundaries collapsed — before any manifest is parsed
/// for them. `relative` is the only field this module ever needed.
pub(crate) struct DetectorOutcome {
    pub(crate) standard: MonorepoStandard,
    pub(crate) root: PathBuf,
    pub(crate) seeds: Vec<PackageSeed>,
}

/// Build the membership layers from detector outcomes.
///
/// Outcomes are grouped by root. Within a root, every standard that declares
/// membership becomes a layer authority; orchestrator-only standards (Nx,
/// Turborepo, Lerna) are attached to each authority at that root. A root whose
/// only standards orchestrate tasks yields no layer — it has no membership
/// authority to own packages.
pub(crate) fn build_monorepo_layers(outcomes: &[DetectorOutcome]) -> Vec<MonorepoLayer> {
    let mut by_root: BTreeMap<&Path, Vec<&DetectorOutcome>> = BTreeMap::new();
    for outcome in outcomes {
        by_root
            .entry(outcome.root.as_path())
            .or_default()
            .push(outcome);
    }

    let mut layers = Vec::new();
    for (root, group) in by_root {
        let orchestrators: Vec<MonorepoStandard> = group
            .iter()
            .map(|outcome| outcome.standard)
            .filter(|standard| standard.orchestrates_tasks_only())
            .collect();

        for outcome in group.iter().filter(|o| o.standard.defines_membership()) {
            debug_assert!(
                outcome.standard.defines_membership(),
                "is_monorepo implies primary authority defines membership (never Unknown)"
            );
            let provenance = outcome.standard.membership_provenance();
            let root_is_package = root_declares_package(outcome.standard, root);
            let packages = outcome
                .seeds
                .iter()
                .map(|seed| seed.relative.clone())
                .collect();
            layers.push(MonorepoLayer {
                root: root.to_path_buf(),
                authority: outcome.standard,
                orchestrators: orchestrators.clone(),
                provenance,
                lockfile_match: None,
                root_is_package,
                packages,
            });
        }
    }
    layers
}

/// Whether the root manifest of `standard` at `root` also declares a package,
/// per the standard's [`RootMembership`] policy.
///
/// `Always` standards (uv) always count the root. `Never` standards (pnpm,
/// npm, Maven, ...) never do. `WhenManifestDeclaresPackage` standards (Cargo)
/// inspect the root manifest: a `[workspace]` plus `[package]` in the same
/// `Cargo.toml` makes the root a package; a virtual `[workspace]`-only
/// manifest does not.
fn root_declares_package(standard: MonorepoStandard, root: &Path) -> bool {
    match standard.spec().root_membership {
        RootMembership::Always => true,
        RootMembership::Never => false,
        RootMembership::WhenManifestDeclaresPackage => {
            // Today only Cargo uses this policy. The check reads the root
            // manifest's `[package]` table without spawning — purely a TOML
            // parse — so the no-subprocess detection boundary is preserved.
            if standard != MonorepoStandard::CargoWorkspace {
                return false;
            }
            performance::increment_counter(counters::FS_FILE_OPENS, 1);
            let Ok(content) = std::fs::read_to_string(root.join("Cargo.toml")) else {
                return false;
            };
            performance::increment_counter(counters::FS_BYTES_READ, content.len() as u64);
            performance::increment_counter(counters::REPO_MANIFEST_PARSES, 1);
            let Ok(parsed) = toml_crate::from_str::<toml_crate::Value>(&content) else {
                return false;
            };
            parsed.get("package").is_some()
        }
    }
}

/// Whether any layer's membership resolves non-degenerately — the honest
/// replacement for the old "any workspace tool present" heuristic.
pub(crate) fn layers_imply_monorepo(layers: &[MonorepoLayer]) -> bool {
    layers
        .iter()
        .any(|layer| layer.authority.membership_resolves_non_degenerately(layer))
}

/// Build the flat list of detected standards, each with its matched markers and
/// detection confidence.
///
/// When the repo is not a monorepo yet an orchestrator (e.g. a bare `nx.json`)
/// matched without any membership authority, an inferred [`MonorepoStandard::Unknown`]
/// entry is appended so the downgrade is observable in JSON.
pub(crate) fn build_detected_standards(
    root: &Path,
    outcomes: &[DetectorOutcome],
    layers: &[MonorepoLayer],
    is_monorepo: bool,
) -> Vec<DetectedStandard> {
    let mut standards: Vec<DetectedStandard> = outcomes
        .iter()
        .map(|outcome| DetectedStandard {
            standard: outcome.standard,
            root: outcome.root.clone(),
            matched_markers: matched_markers(outcome.standard, &outcome.root),
            binary: None,
            confidence: standard_confidence(outcome.standard, &outcome.root, layers),
        })
        .collect();

    let has_authority = outcomes.iter().any(|o| o.standard.defines_membership());
    let has_orchestrator = outcomes
        .iter()
        .any(|o| o.standard.orchestrates_tasks_only());
    if !is_monorepo && has_orchestrator && !has_authority {
        standards.push(DetectedStandard {
            standard: MonorepoStandard::Unknown,
            root: root.to_path_buf(),
            matched_markers: Vec::new(),
            binary: None,
            confidence: DetectionConfidence::Inferred,
        });
    }

    standards
}

/// A detected standard is marker-confirmed when, at its own root, it owns a
/// layer whose membership resolves non-degenerately; otherwise the detection
/// is merely inferred.
///
/// Matching both `authority` and `root` keeps confidence per detected
/// instance: in a forest that hosts the same standard at two roots — one
/// non-degenerate and one degenerate — only the non-degenerate root is
/// reported as confirmed. Without the root tie-breaker a degenerate sibling
/// would inherit `MarkerConfirmed` from its non-degenerate twin.
fn standard_confidence(
    standard: MonorepoStandard,
    root: &Path,
    layers: &[MonorepoLayer],
) -> DetectionConfidence {
    let confirmed = standard.defines_membership()
        && layers.iter().any(|layer| {
            layer.authority == standard
                && layer.root.as_path() == root
                && standard.membership_resolves_non_degenerately(layer)
        });
    if confirmed {
        DetectionConfidence::MarkerConfirmed
    } else {
        DetectionConfidence::Inferred
    }
}

/// The standard's marker files that actually exist at `root`.
fn matched_markers(standard: MonorepoStandard, root: &Path) -> Vec<PathBuf> {
    standard
        .spec()
        .markers
        .iter()
        .map(|marker| root.join(marker.file))
        .filter(|path| probe_exists(path))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filesystem::repo::standard::{DetectionConfidence, PackageProvenance};

    fn pkg(name: &str) -> PackageSeed {
        PackageSeed::new(
            &PathBuf::from("/repo/packages").join(name),
            Path::new("/repo"),
            MonorepoStandard::Unknown,
            PackageProvenance::ManifestScan,
        )
    }

    fn outcome(standard: MonorepoStandard, seeds: Vec<PackageSeed>) -> DetectorOutcome {
        DetectorOutcome {
            standard,
            root: PathBuf::from("/repo"),
            seeds,
        }
    }

    fn outcome_at(
        standard: MonorepoStandard,
        root: &str,
        seeds: Vec<PackageSeed>,
    ) -> DetectorOutcome {
        DetectorOutcome {
            standard,
            root: PathBuf::from(root),
            seeds,
        }
    }

    #[test]
    fn authority_and_orchestrator_collapse_into_one_layer() {
        let outcomes = vec![
            outcome(MonorepoStandard::PnpmWorkspaces, vec![pkg("a"), pkg("b")]),
            outcome(MonorepoStandard::Nx, vec![pkg("a"), pkg("b")]),
        ];
        let layers = build_monorepo_layers(&outcomes);
        assert_eq!(layers.len(), 1);
        assert_eq!(layers[0].authority, MonorepoStandard::PnpmWorkspaces);
        assert_eq!(layers[0].orchestrators, vec![MonorepoStandard::Nx]);
        assert_eq!(layers[0].provenance, PackageProvenance::Globbed);
        assert_eq!(layers[0].packages.len(), 2);
    }

    #[test]
    fn two_authorities_at_one_root_yield_two_layers() {
        let outcomes = vec![
            outcome(MonorepoStandard::CargoWorkspace, vec![pkg("server")]),
            outcome(MonorepoStandard::PnpmWorkspaces, vec![pkg("frontend")]),
        ];
        let layers = build_monorepo_layers(&outcomes);
        assert_eq!(layers.len(), 2);
    }

    #[test]
    fn orchestrator_without_authority_yields_no_layer() {
        let outcomes = vec![outcome(MonorepoStandard::Nx, vec![pkg("a"), pkg("b")])];
        let layers = build_monorepo_layers(&outcomes);
        assert!(layers.is_empty());
        assert!(!layers_imply_monorepo(&layers));
    }

    #[test]
    fn nx_only_downgrade_records_inferred_unknown() {
        let outcomes = vec![outcome(MonorepoStandard::Nx, vec![pkg("a")])];
        let layers = build_monorepo_layers(&outcomes);
        let is_monorepo = layers_imply_monorepo(&layers);
        assert!(!is_monorepo);
        let standards =
            build_detected_standards(Path::new("/repo"), &outcomes, &layers, is_monorepo);
        assert!(
            standards
                .iter()
                .any(|s| s.standard == MonorepoStandard::Unknown
                    && s.confidence == DetectionConfidence::Inferred)
        );
    }

    #[test]
    fn confirmed_authority_is_marker_confirmed() {
        let outcomes = vec![outcome(
            MonorepoStandard::PnpmWorkspaces,
            vec![pkg("a"), pkg("b")],
        )];
        let layers = build_monorepo_layers(&outcomes);
        let standards = build_detected_standards(Path::new("/repo"), &outcomes, &layers, true);
        let pnpm = standards
            .iter()
            .find(|s| s.standard == MonorepoStandard::PnpmWorkspaces)
            .unwrap();
        assert_eq!(pnpm.confidence, DetectionConfidence::MarkerConfirmed);
    }

    #[test]
    fn per_root_confidence_keeps_degenerate_sibling_inferred() {
        // Same standard at two roots: the non-degenerate root must be
        // `MarkerConfirmed`, the degenerate sibling must stay `Inferred`.
        // pnpm uses `RootMembership::Never`, so a single package is degenerate.
        let outcomes = vec![
            outcome_at(
                MonorepoStandard::PnpmWorkspaces,
                "/repo",
                vec![pkg("a"), pkg("b")],
            ),
            outcome_at(
                MonorepoStandard::PnpmWorkspaces,
                "/repo/nested",
                vec![pkg("solo")],
            ),
        ];
        let layers = build_monorepo_layers(&outcomes);
        assert!(layers_imply_monorepo(&layers));
        let standards = build_detected_standards(Path::new("/repo"), &outcomes, &layers, true);

        let root_pnpm = standards
            .iter()
            .find(|s| {
                s.standard == MonorepoStandard::PnpmWorkspaces && s.root == Path::new("/repo")
            })
            .expect("root pnpm standard must be present");
        assert_eq!(
            root_pnpm.confidence,
            DetectionConfidence::MarkerConfirmed,
            "non-degenerate root must be confirmed"
        );

        let nested_pnpm = standards
            .iter()
            .find(|s| {
                s.standard == MonorepoStandard::PnpmWorkspaces
                    && s.root == Path::new("/repo/nested")
            })
            .expect("nested pnpm standard must be present");
        assert_eq!(
            nested_pnpm.confidence,
            DetectionConfidence::Inferred,
            "degenerate sibling must stay inferred"
        );
    }

    #[test]
    fn layer_package_paths_resolve_to_canonical_catalog() {
        // Every path in `MonorepoLayer.packages` must resolve to exactly one
        // entry in the repo-root-relative `RepoInfo.packages` catalog.
        let outcomes = vec![
            outcome_at(
                MonorepoStandard::CargoWorkspace,
                "/repo",
                vec![pkg("server")],
            ),
            outcome_at(
                MonorepoStandard::PnpmWorkspaces,
                "/repo",
                vec![pkg("web"), pkg("api")],
            ),
        ];
        let layers = build_monorepo_layers(&outcomes);

        let catalog: std::collections::HashMap<String, usize> = outcomes
            .iter()
            .flat_map(|o| o.seeds.iter())
            .fold(std::collections::HashMap::new(), |mut acc, seed| {
                *acc.entry(seed.relative.clone()).or_insert(0) += 1;
                acc
            });

        for layer in &layers {
            for rel in &layer.packages {
                let key = normalize_path_for_catalog(rel);
                let count = catalog.get(&key).copied().unwrap_or(0);
                assert_eq!(
                    count, 1,
                    "layer package {key:?} must resolve to exactly one catalog entry"
                );
            }
        }
    }

    fn normalize_path_for_catalog(path: &str) -> String {
        path.replace('\\', "/")
    }
}
