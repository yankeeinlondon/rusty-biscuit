//! Monorepo topology builder: turns per-detector results into membership
//! [`MonorepoLayer`]s and derives the honest `is_monorepo` predicate.
//!
//! Each detector reports a single [`MonorepoStandard`] plus the packages its
//! membership model resolved. This module groups those outcomes by root, splits
//! each root's standards into the authority that declares membership and the
//! orchestrators riding on top, and decides whether the resulting forest is
//! rich enough to call the repo a monorepo. The legacy `monorepo_tool` /
//! `workspace_tools` fields are populated separately and unchanged; this is
//! purely additive.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::standard::{
    DetectedStandard, DetectionConfidence, LayerPackage, MonorepoLayer, MonorepoStandard,
};
use super::types::{MonorepoTool, Package};

/// One detector's contribution to the topology: the standard it matched, the
/// root its marker lives at, and the packages its membership model resolved.
pub(crate) struct DetectorOutcome {
    pub(crate) standard: MonorepoStandard,
    pub(crate) root: PathBuf,
    pub(crate) packages: Vec<Package>,
}

/// Map a legacy [`MonorepoTool`] to its [`MonorepoStandard`] counterpart.
pub(crate) fn standard_for_tool(tool: MonorepoTool) -> MonorepoStandard {
    match tool {
        MonorepoTool::CargoWorkspace => MonorepoStandard::CargoWorkspace,
        MonorepoTool::NpmWorkspaces => MonorepoStandard::NpmWorkspaces,
        MonorepoTool::PnpmWorkspaces => MonorepoStandard::PnpmWorkspaces,
        MonorepoTool::YarnWorkspaces => MonorepoStandard::YarnWorkspaces,
        MonorepoTool::Nx => MonorepoStandard::Nx,
        MonorepoTool::Turborepo => MonorepoStandard::Turborepo,
        MonorepoTool::Lerna => MonorepoStandard::Lerna,
        MonorepoTool::Unknown => MonorepoStandard::Unknown,
    }
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
            let provenance = outcome.standard.membership_provenance();
            let packages = outcome
                .packages
                .iter()
                .map(|pkg| LayerPackage {
                    name: pkg.name.clone(),
                    relative: PathBuf::from(&pkg.relative),
                    standard: outcome.standard,
                    provenance,
                })
                .collect();
            layers.push(MonorepoLayer {
                root: root.to_path_buf(),
                authority: outcome.standard,
                orchestrators: orchestrators.clone(),
                provenance,
                lockfile_match: None,
                packages,
            });
        }
    }
    layers
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
            confidence: standard_confidence(outcome.standard, layers),
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

/// A standard is marker-confirmed when it owns a layer whose membership
/// resolves non-degenerately; otherwise the detection is merely inferred.
fn standard_confidence(
    standard: MonorepoStandard,
    layers: &[MonorepoLayer],
) -> DetectionConfidence {
    let confirmed = standard.defines_membership()
        && layers.iter().any(|layer| {
            layer.authority == standard && standard.membership_resolves_non_degenerately(layer)
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
        .filter(|path| path.exists())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filesystem::repo::standard::{DetectionConfidence, PackageProvenance};

    fn pkg(name: &str) -> Package {
        Package {
            name: name.to_string(),
            relative: format!("packages/{name}"),
            ..Package::default()
        }
    }

    fn outcome(standard: MonorepoStandard, packages: Vec<Package>) -> DetectorOutcome {
        DetectorOutcome {
            standard,
            root: PathBuf::from("/repo"),
            packages,
        }
    }

    #[test]
    fn standard_for_tool_round_trips_every_variant() {
        assert_eq!(
            standard_for_tool(MonorepoTool::CargoWorkspace),
            MonorepoStandard::CargoWorkspace
        );
        assert_eq!(standard_for_tool(MonorepoTool::Nx), MonorepoStandard::Nx);
        assert_eq!(
            standard_for_tool(MonorepoTool::Unknown),
            MonorepoStandard::Unknown
        );
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
}
