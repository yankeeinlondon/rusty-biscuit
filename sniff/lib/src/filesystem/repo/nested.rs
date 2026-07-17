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

use crate::performance;
use crate::performance::counters;

use super::cargo::detect_cargo_workspace;
use super::detection::ManifestStore;
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
use super::seed::PackageSeed;
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
///
/// `evidence` carries marker paths the request-scoped observation index already
/// collected. When it supplies them, no walk happens here: the shared walk used
/// the same ignore, prune, and marker-name rules, so re-enumerating the tree
/// would only re-derive evidence that is already in hand.
pub(crate) fn discover_nested_workspace_outcomes(
    root: &Path,
    evidence: super::detection::RepoEvidence<'_>,
    forbids_nested_roots: &[(PathBuf, MonorepoStandard)],
    manifests: &ManifestStore,
    seeds: &mut Vec<PackageSeed>,
    outcomes: &mut Vec<DetectorOutcome>,
) -> Result<()> {
    let candidates = match evidence.nested_markers {
        Some(markers) => candidates_from_marker_paths(root, markers),
        None => walk_for_nested_markers(root),
    };
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
                evidence,
                manifests,
                seeds,
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
///
/// Marker evidence is matched in-memory against the filenames the walker
/// already yields, rather than re-probing the filesystem per directory. This
/// collapses the per-directory `path.join(marker).exists()` + `read_dir`
/// syscall storm into the single batched `ignore` walk, with two intentional
/// deltas from the older probe-based loop:
///
/// - A gitignored marker file inside a non-gitignored directory is no longer
///   detected (the walker honors `git_ignore` and skips it). Marker files are
///   conventionally committed, so this is judged negligible; see the spec's
///   "Intentional Behavior Change" section.
/// - A directory whose name happens to match a marker file (e.g.
///   `nested/package.json/`) is no longer treated as evidence. The loop now
///   inspects non-directory entries only, which is the true marker contract.
fn walk_for_nested_markers(root: &Path) -> Vec<Candidate> {
    performance::increment_counter(counters::FS_READ_DIRS, 1);
    performance::increment_counter(counters::REPO_NESTED_MARKER_WALKS, 1);
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

    let paths: Vec<PathBuf> = walker
        .filter_map(|entry| entry.ok())
        // Marker evidence is a file contract: skip directories so a directory
        // whose name happens to match a marker does not count as evidence.
        .filter(|entry| !entry.file_type().is_some_and(|ft| ft.is_dir()))
        .map(|entry| entry.path().to_path_buf())
        .collect();

    candidates_from_marker_paths(root, &paths)
}

/// Whether `path` names a nested-workspace marker file.
///
/// This is the predicate the shared observation walk applies when it records
/// marker evidence, so an indexed walk and [`walk_for_nested_markers`] admit
/// exactly the same files.
pub(crate) fn is_nested_marker_path(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| !standards_for_marker_name(name).is_empty())
}

/// The standards a marker file name could indicate.
///
/// Empty when the name is not a marker.
fn standards_for_marker_name(name: &str) -> Vec<MonorepoStandard> {
    let mut matched: Vec<MonorepoStandard> = Vec::new();
    for mapping in NESTED_MARKERS {
        if marker_name_matches(name, mapping.file) {
            for &standard in mapping.standards {
                if !matched.contains(&standard) {
                    matched.push(standard);
                }
            }
        }
    }

    // Suffix match for .NET solution files. Intentionally case-sensitive even
    // on Windows: this inspects names returned by the walker and preserves the
    // historical byte-exact contract from the old `read_dir`-based loop.
    if (name.ends_with(SOLUTION_SUFFIX) || name.ends_with(".slnx"))
        && !matched.contains(&MonorepoStandard::DotNetSolution)
    {
        matched.push(MonorepoStandard::DotNetSolution);
    }

    matched
}

/// Group observed marker file paths into non-root candidate directories.
///
/// Shared by the fallback walk and by callers supplying marker evidence from
/// the request-scoped observation index, so the two cannot drift apart on
/// root exclusion, per-directory dedup, or ordering.
fn candidates_from_marker_paths(root: &Path, paths: &[PathBuf]) -> Vec<Candidate> {
    let mut by_root: HashMap<PathBuf, Vec<MonorepoStandard>> = HashMap::new();

    for path in paths {
        // Nested discovery is non-root only: skip entries whose parent is the
        // repo root itself. A walker also yields `root`, whose parent is
        // `Some("")`/`None`/`Some(parent_of_root)` — none of those equal
        // `Some(root)`, so the root entry naturally fails this check too.
        let Some(parent) = path.parent() else {
            continue;
        };
        if parent == root {
            continue;
        }

        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            // Non-Unicode filenames cannot be marker names: all
            // `NESTED_MARKERS[*].file` literals are ASCII.
            continue;
        };

        let matched_for_entry = standards_for_marker_name(name);
        if matched_for_entry.is_empty() {
            continue;
        }

        // Dedup per parent root so multiple markers in the same directory
        // (e.g. several `*.sln` files, or `package.json` next to
        // `pnpm-workspace.yaml`) dispatch each detector at most once.
        let standards = by_root.entry(parent.to_path_buf()).or_default();
        for standard in matched_for_entry {
            if !standards.contains(&standard) {
                standards.push(standard);
            }
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

/// Marker-name equality for the fixed [`NESTED_MARKERS`] file literals.
///
/// On Unix-like platforms the comparison is byte-exact. On Windows it is
/// ASCII case-insensitive, mirroring what the older
/// `path.join(marker.file).exists()` lookup accepted on case-insensitive
/// filesystems. All `NESTED_MARKERS[*].file` literals are ASCII, so
/// case-folding is sound.
///
/// Intentionally **not** used for the `*.sln` / `*.slnx` suffix match, which
/// preserves its historical byte-exact contract on every platform.
fn marker_name_matches(name: &str, marker: &str) -> bool {
    if cfg!(windows) {
        name.eq_ignore_ascii_case(marker)
    } else {
        name == marker
    }
}

/// Dispatch the detector for `standard` at `target`, folding any
/// [`DetectorOutcome`] it returns into the shared collections.
///
/// `repo_root` is the outer repository root, used to rebase packages from the
/// detector's layer-root-relative frame into the repo-root-relative frame the
/// flat `packages` list requires. Outcome packages stay layer-root-relative.
fn dispatch_detector_at(
    standard: MonorepoStandard,
    target: &Path,
    repo_root: &Path,
    evidence: super::detection::RepoEvidence<'_>,
    manifests: &ManifestStore,
    seeds: &mut Vec<PackageSeed>,
    outcomes: &mut Vec<DetectorOutcome>,
) -> Result<()> {
    let outcome = match standard {
        MonorepoStandard::CargoWorkspace => {
            detect_cargo_workspace(target, evidence, manifests)?
        }
        MonorepoStandard::NpmWorkspaces => detect_npm_workspace(target, evidence, manifests)?,
        MonorepoStandard::PnpmWorkspaces => detect_pnpm_workspace(target, evidence, manifests)?,
        MonorepoStandard::YarnWorkspaces => detect_yarn_workspace(target, evidence, manifests)?,
        MonorepoStandard::BunWorkspaces => detect_bun_workspace(target, evidence, manifests)?,
        MonorepoStandard::UvWorkspace => detect_uv_workspace(target, evidence, manifests)?,
        MonorepoStandard::GoWorkspace => detect_go_workspace(target)?,
        MonorepoStandard::GradleMultiProject => detect_gradle_workspace(target)?,
        MonorepoStandard::MavenMultiModule => detect_maven_workspace(target)?,
        MonorepoStandard::DotNetSolution => detect_dotnet_solution(target)?,
        MonorepoStandard::RushStack => detect_rush_workspace(target)?,
        MonorepoStandard::Nx => detect_nx(target, evidence, manifests)?,
        MonorepoStandard::Turborepo => detect_turborepo(target, evidence, manifests)?,
        MonorepoStandard::Lerna => detect_lerna(target, evidence, manifests)?,
        // Bazel/Pants/Buck2 self-walk; Unknown is not a real detector.
        MonorepoStandard::Bazel
        | MonorepoStandard::Pants
        | MonorepoStandard::Buck2
        | MonorepoStandard::Unknown => None,
    };
    collect_outcome(outcome, repo_root, seeds, outcomes);
    Ok(())
}

/// Fold a detector's [`DetectorOutcome`] into the shared collections.
///
/// Outcome seeds are rebased to `repo_root` so `MonorepoLayer.packages` carries
/// repo-relative paths matching the canonical `RepoInfo.packages` catalog; the
/// same rebased clones populate the flat seed list. Each seed's `owner_root`
/// stays at the nested detector's root, which is the frame its manifests must be
/// enriched in.
fn collect_outcome(
    outcome: Option<DetectorOutcome>,
    repo_root: &Path,
    seeds: &mut Vec<PackageSeed>,
    outcomes: &mut Vec<DetectorOutcome>,
) {
    let Some(mut outcome) = outcome else {
        return;
    };
    for seed in &mut outcome.seeds {
        seed.rebase_to_root(repo_root);
    }
    seeds.extend(outcome.seeds.clone());
    outcomes.push(outcome);
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::*;

    /// Nested discovery is non-root only: a marker placed directly at the repo
    /// root must not register the root itself as a candidate. With only the
    /// root marker present (and no nested markers), the candidate set is exactly
    /// empty — so this fails fast if the `parent == root` skip ever regresses.
    #[test]
    fn root_marker_does_not_register_a_candidate() {
        let dir = TempDir::new().expect("create temp dir");
        std::fs::write(
            dir.path().join("pnpm-workspace.yaml"),
            "packages:\n  - 'packages/*'\n",
        )
        .expect("write root marker");

        let candidates = walk_for_nested_markers(dir.path());

        assert!(
            candidates.is_empty(),
            "a marker at the repo root must not register a nested candidate, got {} candidate(s)",
            candidates.len()
        );
    }

    /// Supplied marker evidence and the fallback walk must produce identical
    /// candidates.
    ///
    /// The shared observation walk applies the same ignore, prune, and
    /// marker-name rules, so consuming its evidence is meant to be a pure work
    /// saving. This is the assertion that would fail if the two ever drift.
    #[test]
    fn supplied_evidence_and_the_fallback_walk_agree() {
        let dir = TempDir::new().expect("create temp dir");
        let root = dir.path();

        // A root marker (must not become a candidate), two nested markers, and
        // a solution file whose suffix match is separate from the fixed names.
        std::fs::write(root.join("Cargo.toml"), "[workspace]\n").expect("write root marker");
        for (sub, file) in [
            ("web", "pnpm-workspace.yaml"),
            ("api", "package.json"),
            ("desktop", "App.sln"),
        ] {
            let nested = root.join(sub);
            std::fs::create_dir_all(&nested).expect("create nested dir");
            std::fs::write(nested.join(file), "{}\n").expect("write nested marker");
        }

        let walked = walk_for_nested_markers(root);

        // The evidence a shared walk would hand over: every marker file it saw.
        let observed: Vec<PathBuf> = walkdir::WalkDir::new(root)
            .into_iter()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_type().is_file())
            .map(|entry| entry.path().to_path_buf())
            .filter(|path| is_nested_marker_path(path))
            .collect();
        let supplied = candidates_from_marker_paths(root, &observed);

        assert_eq!(
            walked.iter().map(|c| &c.root).collect::<Vec<_>>(),
            supplied.iter().map(|c| &c.root).collect::<Vec<_>>(),
            "supplied evidence must yield the same candidate roots as the walk"
        );
        assert_eq!(
            walked
                .iter()
                .map(|c| c.matched_standards.clone())
                .collect::<Vec<_>>(),
            supplied
                .iter()
                .map(|c| c.matched_standards.clone())
                .collect::<Vec<_>>(),
            "supplied evidence must dispatch the same standards as the walk"
        );
        assert_eq!(supplied.len(), 3, "root marker must not be a candidate");
    }

    /// The root-marker exception survives the supplied-evidence path.
    #[test]
    fn root_marker_from_supplied_evidence_registers_no_candidate() {
        let dir = TempDir::new().expect("create temp dir");
        let markers = vec![dir.path().join("pnpm-workspace.yaml")];

        assert!(
            candidates_from_marker_paths(dir.path(), &markers).is_empty(),
            "a marker at the observation root must not register a nested candidate"
        );
    }

    /// `is_nested_marker_path` is the predicate the shared walk records with, so
    /// it must admit exactly the names the candidate grouping later matches.
    #[test]
    fn nested_marker_predicate_matches_fixed_names_and_solution_suffixes() {
        assert!(is_nested_marker_path(Path::new("a/package.json")));
        assert!(is_nested_marker_path(Path::new("a/pnpm-workspace.yaml")));
        assert!(is_nested_marker_path(Path::new("a/App.sln")));
        assert!(is_nested_marker_path(Path::new("a/App.slnx")));
        assert!(!is_nested_marker_path(Path::new("a/README.md")));
        assert!(!is_nested_marker_path(Path::new("a/Cargo.lock")));
    }

    /// `marker_name_matches` is byte-exact on Unix and ASCII case-insensitive
    /// on Windows, mirroring what the older `Path::exists()` lookup accepted
    /// on case-insensitive filesystems. A path-like input never matches a bare
    /// marker name on either platform.
    #[test]
    fn marker_name_matches_is_exact_on_unix_and_case_insensitive_on_windows() {
        // Exact match succeeds on every platform.
        assert!(marker_name_matches("package.json", "package.json"));

        // A path-like input is never a marker name.
        assert!(!marker_name_matches("packages/x", "package.json"));

        // ASCII case-folding contract differs by platform.
        let case_variant = marker_name_matches("Package.json", "package.json");
        if cfg!(windows) {
            assert!(
                case_variant,
                "Windows must accept ASCII case variants for fixed marker names"
            );
        } else {
            assert!(
                !case_variant,
                "Unix-like platforms must compare marker names byte-exactly"
            );
        }
    }
}
