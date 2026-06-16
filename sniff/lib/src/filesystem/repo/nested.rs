//! Bounded workspace-root discovery for nested root-manifest standards.
//!
//! The root-level detectors in [`super::detection`] only consult the supplied
//! `root`. Real repos often host additional workspace standards deeper in the
//! tree — e.g. a Cargo workspace at the root with a pnpm workspace several
//! directories down — and the topology forest must surface those nested roots
//! as their own [`super::topology::DetectorOutcome`]s rather than silently
//! dropping them.
//!
//! This module walks the tree once, looking for the marker files the
//! [`MonorepoStandard`] descriptor table declares. For each non-root directory
//! containing such a marker, the corresponding detector is dispatched at that
//! nested root. Detectors self-filter when the marker does not actually match
//! their stronger checks (e.g. a `package.json` without a `workspaces` field),
//! so the walk stays cheap: only directories that *might* be a workspace root
//! trigger any parsing.
//!
//! The leaf-marker polyglot detectors (Bazel, Pants, Buck2) are intentionally
//! absent from the marker table: they already perform their own tree walk and
//! segment nested workspace roots internally.
//!
//! [`NestingPolicy::ForbidsNested`] standards (Cargo, uv) are still walked:
//! `ForbidsNested` only forbids nested instances of the *same* standard, so a
//! Cargo workspace nested under a pnpm root is a valid separate layer. The
//! same-standard case (e.g. Cargo under Cargo) is suppressed by the caller
//! via the `forbids_nested_roots` set, which the detectors self-filter cannot
//! express because individual markers carry no ancestor context.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use ignore::WalkBuilder;
use tracing::debug;

use super::cargo::detect_cargo_workspace;
use super::dotnet::detect_dotnet_solution;
use super::go::detect_go_workspace;
use super::gradle::detect_gradle_workspace;
use super::maven::detect_maven_workspace;
use super::npm::{
    detect_bun_workspace, detect_npm_workspace, detect_pnpm_workspace, detect_rush_workspace,
    detect_yarn_workspace,
};
use super::nx_turbo::{detect_lerna, detect_nx, detect_turborepo};
use super::standard::{MonorepoStandard, NestingPolicy};
use super::topology::DetectorOutcome;
use super::types::{MonorepoTool, Package, RepoInfo};
use super::uv::detect_uv_workspace;
use crate::Result;
use crate::filesystem::file_types::should_skip_directory_name;

/// Marker files whose presence at a non-root directory marks it as a
/// candidate nested-workspace root.
///
/// Each entry maps a marker file to the standards whose detectors accept it.
/// A single marker (notably `package.json`) can map to several JS-family
/// detectors; those detectors self-disambiguate by lockfile.
struct MarkerMapping {
    file: &'static str,
    /// Standards this marker could indicate. Each is consulted when the
    /// marker is present at a candidate root.
    standards: &'static [MonorepoStandard],
}

/// A glob-style suffix match (`*.sln`) used for marker files whose names are
/// not fixed.
const SOLUTION_SUFFIX: &str = ".sln";

static NESTED_MARKERS: &[MarkerMapping] = &[
    MarkerMapping {
        file: "Cargo.toml",
        standards: &[MonorepoStandard::CargoWorkspace],
    },
    MarkerMapping {
        file: "pnpm-workspace.yaml",
        standards: &[MonorepoStandard::PnpmWorkspaces],
    },
    MarkerMapping {
        file: "package.json",
        standards: &[
            MonorepoStandard::NpmWorkspaces,
            MonorepoStandard::YarnWorkspaces,
            MonorepoStandard::BunWorkspaces,
        ],
    },
    MarkerMapping {
        file: "pyproject.toml",
        standards: &[MonorepoStandard::UvWorkspace],
    },
    MarkerMapping {
        file: "go.work",
        standards: &[MonorepoStandard::GoWorkspace],
    },
    MarkerMapping {
        file: "settings.gradle",
        standards: &[MonorepoStandard::GradleMultiProject],
    },
    MarkerMapping {
        file: "settings.gradle.kts",
        standards: &[MonorepoStandard::GradleMultiProject],
    },
    MarkerMapping {
        file: "pom.xml",
        standards: &[MonorepoStandard::MavenMultiModule],
    },
    MarkerMapping {
        file: "rush.json",
        standards: &[MonorepoStandard::RushStack],
    },
    MarkerMapping {
        file: "nx.json",
        standards: &[MonorepoStandard::Nx],
    },
    MarkerMapping {
        file: "turbo.json",
        standards: &[MonorepoStandard::Turborepo],
    },
    MarkerMapping {
        file: "lerna.json",
        standards: &[MonorepoStandard::Lerna],
    },
];

/// Walk the repo tree from `root` and dispatch detectors at every non-root
/// directory that contains a marker file from the descriptor table.
///
/// [`NestingPolicy::ForbidsNested`] standards (Cargo, uv) only forbid their
/// own nested instances: a nested Cargo workspace under a root Cargo workspace
/// is invalid Cargo, but a uv workspace nested under a *different* standard's
/// root (e.g. pnpm) is perfectly valid. `forbids_nested_roots` carries the
/// pre-computed set of `(root, standard)` pairs whose standard is
/// `ForbidsNested`; a nested candidate of the same standard whose directory is
/// under one of those roots is dropped.
///
/// The leaf-marker polyglot detectors (Bazel, Pants, Buck2) are intentionally
/// absent from [`NESTED_MARKERS`]: they already perform their own tree walk
/// and segment nested workspace roots internally.
pub(crate) fn discover_nested_workspace_outcomes(
    root: &Path,
    manifest_index: Option<&super::manifest_index::ManifestIndex>,
    forbids_nested_roots: &[(PathBuf, MonorepoStandard)],
    workspace_tools: &mut Vec<MonorepoTool>,
    packages: &mut Vec<Package>,
    outcomes: &mut Vec<DetectorOutcome>,
) -> Result<()> {
    let candidates = walk_for_nested_markers(root);
    if candidates.is_empty() {
        return Ok(());
    }

    for candidate in candidates {
        for standard in candidate.matched_standards {
            if !should_dispatch_nested(standard) {
                debug!(
                    standard = ?standard,
                    "skipping nested dispatch for standard that does not accept nested roots"
                );
                continue;
            }

            // `ForbidsNested` only blocks same-standard nesting. A nested
            // Cargo under a root Cargo is invalid; a nested uv under a root
            // pnpm is fine.
            if matches!(standard.spec().nesting_policy, NestingPolicy::ForbidsNested)
                && forbids_nested_roots
                    .iter()
                    .any(|(root, s)| *s == standard && candidate.root.starts_with(root))
            {
                debug!(
                    standard = ?standard,
                    candidate = %candidate.root.display(),
                    "skipping nested dispatch: same-standard ancestor forbids nested instances"
                );
                continue;
            }

            dispatch_detector_at(
                standard,
                &candidate.root,
                root,
                manifest_index,
                workspace_tools,
                packages,
                outcomes,
            )?;
        }
    }

    Ok(())
}

/// Whether the standard is eligible for nested dispatch at all.
///
/// The leaf-marker polyglot detectors handle their own walk; everything else
/// in [`NESTED_MARKERS`] is eligible (subject to the same-standard
/// `ForbidsNested` check the caller performs).
fn should_dispatch_nested(standard: MonorepoStandard) -> bool {
    !matches!(
        standard,
        MonorepoStandard::Bazel | MonorepoStandard::Pants | MonorepoStandard::Buck2
    )
}

/// A directory that contains at least one marker file from [`NESTED_MARKERS`].
struct Candidate {
    root: PathBuf,
    matched_standards: Vec<MonorepoStandard>,
}

/// Walk `root` once and collect every non-root directory that contains a
/// marker file from [`NESTED_MARKERS`] (or a `*.sln` / `*.slnx` solution file
/// for the .NET detector).
///
/// The walk honors `.gitignore` and skips the same directory names
/// (`node_modules`, `target`, `dist`, `build`) the rest of the repo detection
/// skips, so a JS monorepo's deeply-nested `node_modules` subtrees do not
/// explode the candidate set.
fn walk_for_nested_markers(root: &Path) -> Vec<Candidate> {
    let walker = WalkBuilder::new(root)
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
        .build();

    let mut by_root: HashMap<PathBuf, Vec<MonorepoStandard>> = HashMap::new();
    for entry in walker.filter_map(|entry| entry.ok()) {
        if !entry.file_type().is_some_and(|ft| ft.is_dir()) {
            continue;
        }
        let path = entry.path();
        if path == root {
            continue;
        }

        let mut matched: Vec<MonorepoStandard> = Vec::new();
        for mapping in NESTED_MARKERS {
            if path.join(mapping.file).exists() {
                for &standard in mapping.standards {
                    if !matched.contains(&standard) {
                        matched.push(standard);
                    }
                }
            }
        }

        // .NET solution files have arbitrary names, so match by suffix.
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let Some(name) = name.to_str() else { continue };
                if (name.ends_with(SOLUTION_SUFFIX) || name.ends_with(".slnx"))
                    && !matched.contains(&MonorepoStandard::DotNetSolution)
                {
                    matched.push(MonorepoStandard::DotNetSolution);
                    break;
                }
            }
        }

        if !matched.is_empty() {
            by_root
                .entry(path.to_path_buf())
                .or_default()
                .extend(matched);
        }
    }

    let mut candidates: Vec<Candidate> = by_root
        .into_iter()
        .map(|(root, mut standards)| {
            standards.sort_by_key(|s| s.spec().id);
            Candidate {
                root,
                matched_standards: standards,
            }
        })
        .collect();
    candidates.sort_by(|a, b| a.root.cmp(&b.root));
    candidates
}

/// Dispatch the detector for `standard` at `target`, folding any
/// [`RepoInfo`] it returns into the shared collections.
///
/// `repo_root` is the outer repository root, used to rebase packages from the
/// detector's layer-root-relative frame into the repo-root-relative frame the
/// flat `packages` list requires. Outcome packages stay layer-root-relative.
fn dispatch_detector_at(
    standard: MonorepoStandard,
    target: &Path,
    repo_root: &Path,
    manifest_index: Option<&super::manifest_index::ManifestIndex>,
    workspace_tools: &mut Vec<MonorepoTool>,
    packages: &mut Vec<Package>,
    outcomes: &mut Vec<DetectorOutcome>,
) -> Result<()> {
    match standard {
        MonorepoStandard::CargoWorkspace => {
            collect_repo_info(
                detect_cargo_workspace(target)?,
                repo_root,
                workspace_tools,
                packages,
                outcomes,
                standard,
            );
        }
        MonorepoStandard::NpmWorkspaces => {
            collect_repo_info(
                detect_npm_workspace(target)?,
                repo_root,
                workspace_tools,
                packages,
                outcomes,
                standard,
            );
        }
        MonorepoStandard::PnpmWorkspaces => {
            collect_repo_info(
                detect_pnpm_workspace(target)?,
                repo_root,
                workspace_tools,
                packages,
                outcomes,
                standard,
            );
        }
        MonorepoStandard::YarnWorkspaces => {
            collect_repo_info(
                detect_yarn_workspace(target)?,
                repo_root,
                workspace_tools,
                packages,
                outcomes,
                standard,
            );
        }
        MonorepoStandard::BunWorkspaces => {
            collect_standard_outcome(
                detect_bun_workspace(target)?,
                standard,
                repo_root,
                packages,
                outcomes,
            );
        }
        MonorepoStandard::UvWorkspace => {
            collect_standard_outcome(
                detect_uv_workspace(target)?,
                standard,
                repo_root,
                packages,
                outcomes,
            );
        }
        MonorepoStandard::GoWorkspace => {
            collect_standard_outcome(
                detect_go_workspace(target)?,
                standard,
                repo_root,
                packages,
                outcomes,
            );
        }
        MonorepoStandard::GradleMultiProject => {
            collect_standard_outcome(
                detect_gradle_workspace(target)?,
                standard,
                repo_root,
                packages,
                outcomes,
            );
        }
        MonorepoStandard::MavenMultiModule => {
            collect_standard_outcome(
                detect_maven_workspace(target)?,
                standard,
                repo_root,
                packages,
                outcomes,
            );
        }
        MonorepoStandard::DotNetSolution => {
            collect_standard_outcome(
                detect_dotnet_solution(target)?,
                standard,
                repo_root,
                packages,
                outcomes,
            );
        }
        MonorepoStandard::RushStack => {
            collect_standard_outcome(
                detect_rush_workspace(target)?,
                standard,
                repo_root,
                packages,
                outcomes,
            );
        }
        MonorepoStandard::Nx => {
            collect_repo_info(
                detect_nx(target, manifest_index)?,
                repo_root,
                workspace_tools,
                packages,
                outcomes,
                standard,
            );
        }
        MonorepoStandard::Turborepo => {
            collect_repo_info(
                detect_turborepo(target, manifest_index)?,
                repo_root,
                workspace_tools,
                packages,
                outcomes,
                standard,
            );
        }
        MonorepoStandard::Lerna => {
            collect_repo_info(
                detect_lerna(target, manifest_index)?,
                repo_root,
                workspace_tools,
                packages,
                outcomes,
                standard,
            );
        }
        // Bazel/Pants/Buck2 self-walk; Unknown is not a real detector.
        MonorepoStandard::Bazel
        | MonorepoStandard::Pants
        | MonorepoStandard::Buck2
        | MonorepoStandard::Unknown => {}
    }
    Ok(())
}

/// Fold a detector's [`RepoInfo`] into the shared collections, attributing the
/// outcome to `standard` regardless of the legacy [`MonorepoTool`] the
/// detector reported.
///
/// Outcome packages stay layer-root-relative (relative to the detector's own
/// root); the flat `packages` list receives repo-root-relative clones so
/// `RepoInfo.packages` stays uniformly framed for dirty/staged matching and
/// `--package-area` filtering.
fn collect_repo_info(
    info: Option<RepoInfo>,
    repo_root: &Path,
    workspace_tools: &mut Vec<MonorepoTool>,
    packages: &mut Vec<Package>,
    outcomes: &mut Vec<DetectorOutcome>,
    standard: MonorepoStandard,
) {
    let Some(info) = info else {
        return;
    };

    if let Some(tool) = info.monorepo_tool
        && !workspace_tools.contains(&tool)
    {
        workspace_tools.push(tool);
    }
    for tool in info.workspace_tools {
        if !workspace_tools.contains(&tool) {
            workspace_tools.push(tool);
        }
    }

    let detected_packages = info.packages.unwrap_or_default();
    outcomes.push(DetectorOutcome {
        standard,
        root: info.root,
        packages: detected_packages.clone(),
    });
    let mut flat_packages = detected_packages;
    for pkg in &mut flat_packages {
        super::detection::rebase_package_to_root(pkg, repo_root);
    }
    packages.extend(flat_packages);
}

/// Fold a new-standard detector's result into the topology collections.
///
/// Outcome packages stay layer-root-relative; the flat `packages` list receives
/// repo-root-relative clones (see [`collect_repo_info`]).
fn collect_standard_outcome(
    info: Option<RepoInfo>,
    standard: MonorepoStandard,
    repo_root: &Path,
    packages: &mut Vec<Package>,
    outcomes: &mut Vec<DetectorOutcome>,
) {
    let Some(info) = info else {
        return;
    };
    let detected_packages = info.packages.unwrap_or_default();
    outcomes.push(DetectorOutcome {
        standard,
        root: info.root,
        packages: detected_packages.clone(),
    });
    let mut flat_packages = detected_packages;
    for pkg in &mut flat_packages {
        super::detection::rebase_package_to_root(pkg, repo_root);
    }
    packages.extend(flat_packages);
}
