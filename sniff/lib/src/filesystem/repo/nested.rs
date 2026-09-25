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
use super::seed::PackageSeed;
use super::standard::{MonorepoStandard, NestingPolicy};
use super::topology::DetectorOutcome;
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
/// already yields, rather than re-probing the filesystem per directory. Two
/// long-standing consequences, relative to the older probe-based loop this
/// replaced:
///
/// - A gitignored marker file inside a non-gitignored directory is not
///   detected, because the walker honors `git_ignore`. Marker files are
///   conventionally committed, so this is judged negligible.
/// - A directory whose name matches a marker file (e.g.
///   `nested/package.json/`) is not evidence: only non-directory entries are,
///   which is the true marker contract. Its descendants are still walked.
///
/// The walk runs on `ignore`'s parallel walker with its default worker count
/// (available parallelism, capped at 12). The result does not depend on
/// scheduling: every matching marker is kept and
/// [`candidates_from_marker_paths`] sorts what it returns.
///
/// The starting root itself is never evidence. `ignore`'s serial walker
/// yields a symlinked root with the link's (non-directory) file type, so the
/// pre-parallel walk registered the link's *parent*, outside the repository,
/// when the link was named like a marker; that is no longer possible.
fn walk_for_nested_markers(root: &Path) -> Vec<Candidate> {
    walk_for_nested_markers_with_threads(root, None)
}

/// [`walk_for_nested_markers`] with an explicit worker count.
///
/// `None` keeps `ignore`'s default policy and is what production passes; tests
/// pass `Some(n)` to pin a one-worker or multi-worker run.
fn walk_for_nested_markers_with_threads(root: &Path, threads: Option<usize>) -> Vec<Candidate> {
    use ignore::WalkState;
    use std::sync::{Arc, Mutex};

    /// Per-visitor state: marker paths are buffered locally and merged into
    /// `shared` once, when the visitor drops, so no lock is taken per entry.
    ///
    /// The collector makes work recorded on this visitor's thread visible to
    /// the request; it flushes when dropped on that thread.
    struct MarkerWorker {
        shared: Arc<Mutex<Vec<PathBuf>>>,
        collector: performance::WorkerCollector,
        local: Vec<PathBuf>,
    }

    impl Drop for MarkerWorker {
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

    performance::increment_counter(counters::FS_READ_DIRS, 1);
    performance::increment_counter(counters::REPO_NESTED_MARKER_WALKS, 1);

    let shared: Arc<Mutex<Vec<PathBuf>>> = Arc::new(Mutex::new(Vec::new()));
    let mut builder = WalkBuilder::new(root);
    builder
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
        });
    if let Some(threads) = threads {
        builder.threads(threads);
    }
    builder.build_parallel().run(|| {
        let mut worker = MarkerWorker {
            shared: Arc::clone(&shared),
            collector: performance::WorkerCollector::inherit(),
            local: Vec::new(),
        };
        Box::new(move |result| {
            // Unlike `ManifestIndex::build`, activate only after the error
            // check: `ignore` runs its first visitor on the *calling* thread
            // and hands it only root errors (e.g. a missing root). Activating
            // there would clear the caller's buffered counters, and dropping
            // it would uninstall the caller's collector.
            let Ok(entry) = result else {
                return WalkState::Continue;
            };
            worker.collector.activate();
            // Marker evidence is a file contract: skip directories so a
            // directory whose name happens to match a marker does not count.
            if entry.depth() == 0 || entry.file_type().is_some_and(|ft| ft.is_dir()) {
                return WalkState::Continue;
            }
            #[cfg(test)]
            tests::record_admitted_entry();
            if !is_nested_marker_path(entry.path()) {
                return WalkState::Continue;
            }
            worker.local.push(entry.into_path());
            WalkState::Continue
        })
    });

    // `run` returns only after every worker has joined, so every visitor has
    // dropped and merged its batch.
    let paths = std::mem::take(
        &mut *shared
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()),
    );
    candidates_from_marker_paths(root, &paths)
}

/// Every non-directory entry below `root` the pre-parallelization serial walk
/// admitted.
///
/// A frozen copy of the serial collect-all loop `walk_for_nested_markers`
/// used before the `2026-09-20-repo-perf` fix. It is kept independent of the
/// production walker on purpose: it is both the parity oracle for the
/// parallel walk and the in-process "before" side of its measurements, so a
/// change to the production builder must not change it too. It records no
/// work counters.
///
/// Its one deliberate difference from that loop is skipping the depth-0 root
/// entry, which the loop admitted for a symlinked root; see
/// [`walk_for_nested_markers`].
#[cfg(any(test, feature = "bench-internals"))]
fn serial_reference_paths(root: &Path) -> Vec<PathBuf> {
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
        .build()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.depth() != 0 && !entry.file_type().is_some_and(|ft| ft.is_dir()))
        .map(|entry| entry.path().to_path_buf())
        .collect()
}

/// Measurement access to the nested-marker fallback walk.
///
/// Only compiled for tests and the `bench-internals` feature; it is not part of
/// the public API.
#[cfg(any(test, feature = "bench-internals"))]
#[doc(hidden)]
pub mod benchmark {
    use std::path::{Path, PathBuf};

    use super::MonorepoStandard;

    /// A nested candidate as an ordered `(root, matched_standards)` pair.
    pub type CandidateFields = (PathBuf, Vec<MonorepoStandard>);

    /// Sizes of the tree the serial reference walk sees under one root.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct NestedWalkCorpus {
        /// Non-directory entries admitted by the walk's ignore and prune rules.
        pub walked_entries: usize,
        /// Of those, the entries whose file name is a nested-workspace marker.
        pub marker_entries: usize,
    }

    /// The production fallback walk, including its work counters.
    pub fn production_walk(root: &Path) -> Vec<CandidateFields> {
        into_fields(super::walk_for_nested_markers(root))
    }

    /// The frozen pre-parallelization serial walk, without work counters.
    pub fn serial_reference_walk(root: &Path) -> Vec<CandidateFields> {
        let paths = super::serial_reference_paths(root);
        into_fields(super::candidates_from_marker_paths(root, &paths))
    }

    /// Count what the serial reference walk visits under `root`.
    pub fn corpus(root: &Path) -> NestedWalkCorpus {
        let paths = super::serial_reference_paths(root);
        NestedWalkCorpus {
            walked_entries: paths.len(),
            marker_entries: paths
                .iter()
                .filter(|path| super::is_nested_marker_path(path))
                .count(),
        }
    }

    fn into_fields(candidates: Vec<super::Candidate>) -> Vec<CandidateFields> {
        candidates
            .into_iter()
            .map(|candidate| (candidate.root, candidate.matched_standards))
            .collect()
    }
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
        MonorepoStandard::CargoWorkspace => detect_cargo_workspace(target, evidence, manifests)?,
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

    /// The measurement seam's serial reference and the production walk agree,
    /// and both match candidates spelled out independently of the shared
    /// projection, so the "before" side of the walk benchmarks is the walk it
    /// claims to be.
    #[test]
    fn serial_reference_walk_matches_the_production_walk() {
        use crate::filesystem::repo::nested_benchmark;

        let dir = TempDir::new().expect("create temp dir");
        let root = dir.path();
        std::fs::write(root.join("package.json"), "{}\n").expect("write root marker");
        for (sub, file) in [
            ("apps/web", "package.json"),
            ("apps/web", "pnpm-workspace.yaml"),
            ("crates/core", "Cargo.toml"),
            ("node_modules/dep", "package.json"),
            ("marker/package.json", "notes.txt"),
        ] {
            let nested = root.join(sub);
            std::fs::create_dir_all(&nested).expect("create nested dir");
            std::fs::write(nested.join(file), "{}\n").expect("write fixture file");
        }

        let mut expected = vec![
            (
                root.join("apps/web"),
                vec![
                    MonorepoStandard::NpmWorkspaces,
                    MonorepoStandard::YarnWorkspaces,
                    MonorepoStandard::PnpmWorkspaces,
                    MonorepoStandard::BunWorkspaces,
                ],
            ),
            (
                root.join("crates/core"),
                vec![MonorepoStandard::CargoWorkspace],
            ),
        ];
        for (_, standards) in &mut expected {
            standards.sort_by_key(|s| s.spec().id);
        }

        assert_eq!(nested_benchmark::serial_reference_walk(root), expected);
        assert_eq!(nested_benchmark::production_walk(root), expected);
        assert_eq!(
            nested_benchmark::corpus(root),
            nested_benchmark::NestedWalkCorpus {
                walked_entries: 5,
                marker_entries: 4,
            },
            "the pruned node_modules marker is not walked"
        );
    }

    /// One fallback invocation is one logical walk, whatever the worker
    /// count: the counters sit at the chokepoint, not in the visitors.
    #[test]
    fn fallback_walk_records_one_logical_walk_per_invocation() {
        use crate::performance::testing;

        let dir = TempDir::new().expect("create temp dir");
        let root = dir.path();
        for sub in ["a", "b/c", "d"] {
            let nested = root.join(sub);
            std::fs::create_dir_all(&nested).expect("create nested dir");
            std::fs::write(nested.join("Cargo.toml"), "[workspace]\n").expect("write marker");
        }

        for threads in [None, Some(1), Some(4)] {
            let (candidates, counts) =
                testing::measure(|| walk_for_nested_markers_with_threads(root, threads));

            assert_eq!(
                candidates.iter().map(|c| c.root.clone()).collect::<Vec<_>>(),
                vec![root.join("a"), root.join("b/c"), root.join("d")],
                "threads = {threads:?}"
            );
            assert_eq!(
                counts.get(counters::FS_READ_DIRS),
                1,
                "threads = {threads:?}: {:?}",
                counts.all()
            );
            assert_eq!(
                counts.get(counters::REPO_NESTED_MARKER_WALKS),
                1,
                "threads = {threads:?}: {:?}",
                counts.all()
            );
        }
    }

    /// A missing root reaches `ignore`'s first visitor as an error on the
    /// *calling* thread. That visitor must not disturb the caller's collector:
    /// work recorded before the walk, the walk's own counters, and work
    /// recorded after it must all survive.
    #[test]
    fn a_missing_root_keeps_the_callers_counters() {
        use crate::performance::testing;

        let dir = TempDir::new().expect("create temp dir");
        let missing = dir.path().join("does-not-exist");

        let (candidates, counts) = testing::measure(|| {
            performance::increment_counter(counters::REPO_MANIFEST_PARSES, 1);
            let candidates = walk_for_nested_markers(&missing);
            performance::increment_counter(counters::FS_FILE_OPENS, 1);
            candidates
        });

        assert!(candidates.is_empty(), "a missing root yields no candidates");
        assert_eq!(counts.get(counters::FS_READ_DIRS), 1, "{:?}", counts.all());
        assert_eq!(
            counts.get(counters::REPO_NESTED_MARKER_WALKS),
            1,
            "{:?}",
            counts.all()
        );
        assert_eq!(
            counts.get(counters::REPO_MANIFEST_PARSES),
            1,
            "work recorded before the walk was lost: {:?}",
            counts.all()
        );
        assert_eq!(
            counts.get(counters::FS_FILE_OPENS),
            1,
            "work recorded after the walk was lost: {:?}",
            counts.all()
        );
    }

    /// A symlinked starting root named like a marker is not evidence: before
    /// the parallel walk, the serial walker admitted the link itself and
    /// registered its parent — a directory outside the walked tree.
    #[cfg(unix)]
    #[test]
    fn a_marker_named_symlinked_root_registers_no_candidate_outside_the_root() {
        use crate::filesystem::repo::nested_benchmark;

        let dir = TempDir::new().expect("create temp dir");
        let target = dir.path().join("target-tree");
        std::fs::create_dir_all(target.join("apps/web")).expect("create nested dir");
        std::fs::write(target.join("apps/web/package.json"), "{}\n").expect("write marker");
        let link = dir.path().join("package.json");
        std::os::unix::fs::symlink(&target, &link).expect("create root symlink");

        let mut standards = vec![
            MonorepoStandard::NpmWorkspaces,
            MonorepoStandard::YarnWorkspaces,
            MonorepoStandard::BunWorkspaces,
        ];
        standards.sort_by_key(|s| s.spec().id);
        let expected = vec![(link.join("apps/web"), standards)];

        assert_eq!(nested_benchmark::production_walk(&link), expected);
        assert_eq!(nested_benchmark::serial_reference_walk(&link), expected);
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

    /// Test-only work counter (ruling R2 of `2026-09-20-repo-perf`): one
    /// increment per entry the parallel walk admits past its root and directory
    /// filters. It goes through the ordinary thread-buffered counter path, so a
    /// visitor whose `WorkerCollector` never flushed makes the total read short.
    const ADMITTED_ENTRIES: &str = "test.nested_walk.admitted_entries";

    /// Prefix of the diagnostic counters recording which threads admitted
    /// entries: one key per distinct thread, each with the value one.
    const WORKER_THREAD_PREFIX: &str = "test.nested_walk.worker_thread.";

    /// Called by the parallel walk's visitor for every admitted entry.
    pub(super) fn record_admitted_entry() {
        use std::cell::Cell;

        thread_local! {
            // `ignore` runs every walk on freshly spawned scoped threads, so a
            // thread only ever admits entries for one walk.
            static RECORDED_THREAD: Cell<bool> = const { Cell::new(false) };
        }

        performance::increment_counter(ADMITTED_ENTRIES, 1);
        if !RECORDED_THREAD.replace(true) {
            performance::increment_counter_dynamic(
                format!("{WORKER_THREAD_PREFIX}{:?}", std::thread::current().id()),
                1,
            );
        }
    }

    /// Worker configurations every parity check runs: one worker, `ignore`'s
    /// default policy (what production uses), and an explicit multi-worker
    /// count that is parallel even on a small runner.
    const WORKER_CONFIGS: [Option<usize>; 3] = [Some(1), None, Some(4)];

    type Fields = crate::filesystem::repo::nested_benchmark::CandidateFields;

    fn fields(candidates: Vec<Candidate>) -> Vec<Fields> {
        candidates
            .into_iter()
            .map(|candidate| (candidate.root, candidate.matched_standards))
            .collect()
    }

    /// Standards in the order a candidate reports them.
    fn sorted(mut standards: Vec<MonorepoStandard>) -> Vec<MonorepoStandard> {
        standards.sort_by_key(|s| s.spec().id);
        standards
    }

    fn js_family() -> Vec<MonorepoStandard> {
        sorted(vec![
            MonorepoStandard::NpmWorkspaces,
            MonorepoStandard::YarnWorkspaces,
            MonorepoStandard::BunWorkspaces,
        ])
    }

    fn write_fixture_file(root: &Path, relative: &str) {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().expect("fixture file has a parent"))
            .expect("create fixture dir");
        std::fs::write(&path, "{}\n").expect("write fixture file");
    }

    /// Assert that the parallel walk, in every [`WORKER_CONFIGS`] entry, equals
    /// the serial reference as complete ordered `(root, matched_standards)`
    /// values, and return those values for the caller's independent check.
    fn assert_parity(root: &Path) -> Vec<Fields> {
        let reference = crate::filesystem::repo::nested_benchmark::serial_reference_walk(root);
        for threads in WORKER_CONFIGS {
            assert_eq!(
                fields(walk_for_nested_markers_with_threads(root, threads)),
                reference,
                "threads = {threads:?}: the parallel walk diverged from the serial reference under {}",
                root.display()
            );
        }
        reference
    }

    /// Create a symlink, or return `false` where the platform requires a
    /// privilege this process lacks (ruling R5: native Windows without
    /// Developer Mode). The skip is printed so it shows in captured output.
    fn try_symlink(target: &Path, link: &Path, target_is_dir: bool) -> bool {
        #[cfg(unix)]
        {
            let _ = target_is_dir;
            std::os::unix::fs::symlink(target, link).expect("create symlink");
            true
        }
        #[cfg(windows)]
        {
            let result = if target_is_dir {
                std::os::windows::fs::symlink_dir(target, link)
            } else {
                std::os::windows::fs::symlink_file(target, link)
            };
            match result {
                Ok(()) => true,
                // ERROR_PRIVILEGE_NOT_HELD
                Err(error) if error.raw_os_error() == Some(1314) => {
                    eprintln!(
                        "SKIP: creating a symlink needs Developer Mode or elevation here ({error})"
                    );
                    false
                }
                Err(error) => panic!("create symlink {}: {error}", link.display()),
            }
        }
    }

    /// A tree `ignore` should split across workers: many sibling directories
    /// at three depths, markers placed by index so the expected candidates are
    /// derived from the placement rule rather than from the walk.
    fn build_wide_fixture(root: &Path) -> Vec<Fields> {
        let mut expected = Vec::new();
        for index in 0..40 {
            let top = format!("w{index:02}");
            for filler in 0..8 {
                write_fixture_file(root, &format!("{top}/pkg/f{filler}.txt"));
            }
            for filler in 0..4 {
                write_fixture_file(root, &format!("{top}/pkg/sub/g{filler}.rs"));
            }
            if index % 2 == 0 {
                write_fixture_file(root, &format!("{top}/Cargo.toml"));
                expected.push((root.join(&top), vec![MonorepoStandard::CargoWorkspace]));
            }
            if index % 3 == 0 {
                write_fixture_file(root, &format!("{top}/pkg/package.json"));
                expected.push((root.join(&top).join("pkg"), js_family()));
            }
            if index % 5 == 0 {
                write_fixture_file(root, &format!("{top}/pkg/sub/go.work"));
                expected.push((
                    root.join(&top).join("pkg").join("sub"),
                    vec![MonorepoStandard::GoWorkspace],
                ));
            }
        }
        expected.sort_by(|a, b| a.0.cmp(&b.0));
        expected
    }

    /// AC2 marker matrix: every fixed marker name, both solution suffixes,
    /// several depths and siblings, two markers mapping to one standard, one
    /// marker mapping to several, and several markers in one directory.
    #[test]
    fn every_marker_name_registers_its_standards_at_any_depth() {
        let dir = TempDir::new().expect("create temp dir");
        let root = dir.path();
        for file in [
            // Root markers and look-alike names are never candidates.
            "Cargo.toml",
            "package.json",
            "notes/package.jsonx",
            "notes/xpom.xml",
            "notes/Cargo.lock",
            "notes/App.sln.bak",
            // One directory per fixed marker name and solution suffix.
            "m01/Cargo.toml",
            "m02/pnpm-workspace.yaml",
            "m03/package.json",
            "m04/pyproject.toml",
            "m05/go.work",
            "m06/settings.gradle",
            "m07/settings.gradle.kts",
            "m08/pom.xml",
            "m09/rush.json",
            "m10/nx.json",
            "m11/turbo.json",
            "m12/lerna.json",
            "m13/App.sln",
            "m14/App.slnx",
            "m01/README.md",
            "deep/a/b/c/d/Cargo.toml",
            "gradle/settings.gradle",
            "gradle/settings.gradle.kts",
            "multi/package.json",
            "multi/pnpm-workspace.yaml",
            "multi/turbo.json",
            "multi/One.sln",
            "multi/Two.slnx",
        ] {
            write_fixture_file(root, file);
        }

        let expected: Vec<Fields> = vec![
            (
                root.join("deep/a/b/c/d"),
                vec![MonorepoStandard::CargoWorkspace],
            ),
            (
                root.join("gradle"),
                vec![MonorepoStandard::GradleMultiProject],
            ),
            (root.join("m01"), vec![MonorepoStandard::CargoWorkspace]),
            (root.join("m02"), vec![MonorepoStandard::PnpmWorkspaces]),
            (root.join("m03"), js_family()),
            (root.join("m04"), vec![MonorepoStandard::UvWorkspace]),
            (root.join("m05"), vec![MonorepoStandard::GoWorkspace]),
            (root.join("m06"), vec![MonorepoStandard::GradleMultiProject]),
            (root.join("m07"), vec![MonorepoStandard::GradleMultiProject]),
            (root.join("m08"), vec![MonorepoStandard::MavenMultiModule]),
            (root.join("m09"), vec![MonorepoStandard::RushStack]),
            (root.join("m10"), vec![MonorepoStandard::Nx]),
            (root.join("m11"), vec![MonorepoStandard::Turborepo]),
            (root.join("m12"), vec![MonorepoStandard::Lerna]),
            (root.join("m13"), vec![MonorepoStandard::DotNetSolution]),
            (root.join("m14"), vec![MonorepoStandard::DotNetSolution]),
            (
                root.join("multi"),
                sorted(vec![
                    MonorepoStandard::NpmWorkspaces,
                    MonorepoStandard::YarnWorkspaces,
                    MonorepoStandard::BunWorkspaces,
                    MonorepoStandard::PnpmWorkspaces,
                    MonorepoStandard::Turborepo,
                    MonorepoStandard::DotNetSolution,
                ]),
            ),
        ];

        assert_eq!(assert_parity(root), expected);
    }

    /// Empty, root-only, and missing roots have nothing nested to find.
    #[test]
    fn empty_root_only_and_missing_roots_register_no_candidates() {
        let empty = TempDir::new().expect("create temp dir");
        assert_eq!(assert_parity(empty.path()), Vec::<Fields>::new());

        let root_only = TempDir::new().expect("create temp dir");
        for file in [
            "Cargo.toml",
            "pnpm-workspace.yaml",
            "package.json",
            "pyproject.toml",
            "go.work",
            "settings.gradle",
            "settings.gradle.kts",
            "pom.xml",
            "rush.json",
            "nx.json",
            "turbo.json",
            "lerna.json",
            "App.sln",
            "App.slnx",
        ] {
            write_fixture_file(root_only.path(), file);
        }
        assert_eq!(assert_parity(root_only.path()), Vec::<Fields>::new());

        let missing = empty.path().join("does-not-exist");
        assert_eq!(assert_parity(&missing), Vec::<Fields>::new());
    }

    /// Scheduling variance must not change the result: the wide tree is
    /// compared 20 times in every worker configuration.
    #[test]
    fn wide_tree_parity_holds_across_repeats_and_worker_counts() {
        let dir = TempDir::new().expect("create temp dir");
        let root = dir.path();
        let expected = build_wide_fixture(root);
        assert_eq!(expected.len(), 20 + 14 + 8, "fixture placement rule");

        for _ in 0..20 {
            assert_eq!(assert_parity(root), expected);
        }
    }

    /// Hidden directories stay eligible, the named-directory prune stays
    /// authoritative at any depth, and a directory named like a marker is not
    /// evidence though its descendants are still walked.
    #[test]
    fn prune_hidden_and_marker_named_directories_keep_their_semantics() {
        let dir = TempDir::new().expect("create temp dir");
        let root = dir.path();
        for pruned in [
            "node_modules",
            "target",
            "vendor",
            "dist",
            "build",
            "out",
            "bin",
            "__pycache__",
            ".venv",
            ".turbo",
            ".next",
            ".cache",
        ] {
            write_fixture_file(root, &format!("{pruned}/package.json"));
            write_fixture_file(root, &format!("{pruned}/pkg/Cargo.toml"));
            write_fixture_file(root, &format!("lib/{pruned}/deep/pom.xml"));
        }
        for file in [
            ".config/tool/package.json",
            ".hidden/Cargo.toml",
            "weird/package.json/inner/Cargo.toml",
            "odd/Cargo.toml/pom.xml",
        ] {
            write_fixture_file(root, file);
        }

        let expected: Vec<Fields> = vec![
            (root.join(".config/tool"), js_family()),
            (root.join(".hidden"), vec![MonorepoStandard::CargoWorkspace]),
            (
                root.join("odd/Cargo.toml"),
                vec![MonorepoStandard::MavenMultiModule],
            ),
            (
                root.join("weird/package.json/inner"),
                vec![MonorepoStandard::CargoWorkspace],
            ),
        ];

        assert_eq!(assert_parity(root), expected);
    }

    /// A non-directory symlink named like a marker is admitted on its name
    /// alone — even when dangling — while a directory link is never followed.
    /// Collected paths keep the spelling they were walked under.
    #[test]
    fn non_directory_symlinks_are_admitted_and_directory_links_are_not_followed() {
        let dir = TempDir::new().expect("create temp dir");
        let root = dir.path();
        write_fixture_file(root, "shared/templates/package.json.tmpl");
        write_fixture_file(root, "real/pkg/Cargo.toml");
        std::fs::create_dir_all(root.join("apps/web")).expect("create link dir");
        std::fs::create_dir_all(root.join("dangling")).expect("create link dir");

        if !try_symlink(
            &root.join("shared/templates/package.json.tmpl"),
            &root.join("apps/web/package.json"),
            false,
        ) || !try_symlink(
            &root.join("missing.xml"),
            &root.join("dangling/pom.xml"),
            false,
        ) || !try_symlink(&root.join("real"), &root.join("linked"), true)
        {
            return;
        }

        let expected: Vec<Fields> = vec![
            (root.join("apps/web"), js_family()),
            (
                root.join("dangling"),
                vec![MonorepoStandard::MavenMultiModule],
            ),
            (root.join("real/pkg"), vec![MonorepoStandard::CargoWorkspace]),
        ];

        let candidates = assert_parity(root);
        assert_eq!(candidates, expected);
        assert!(
            candidates
                .iter()
                .all(|(candidate, _)| candidate.starts_with(root)),
            "collected paths must keep the walked root's spelling, not a canonical one"
        );
    }

    /// A symlinked starting root walks like its target, under the link's
    /// spelling, in the serial reference and every parallel configuration.
    #[test]
    fn a_symlinked_starting_root_walks_like_its_target_under_the_link_spelling() {
        let dir = TempDir::new().expect("create temp dir");
        let target = dir.path().join("repo");
        for file in [
            "Cargo.toml",
            "apps/web/package.json",
            "crates/core/Cargo.toml",
            "node_modules/dep/package.json",
        ] {
            write_fixture_file(&target, file);
        }
        let link = dir.path().join("repo-link");
        if !try_symlink(&target, &link, true) {
            return;
        }

        let expected: Vec<Fields> = vec![
            (link.join("apps/web"), js_family()),
            (
                link.join("crates/core"),
                vec![MonorepoStandard::CargoWorkspace],
            ),
        ];

        assert_eq!(assert_parity(&link), expected);
    }

    /// Walk-level check of the platform case rules the matcher tests pin:
    /// fixed marker names fold ASCII case only on Windows, and the solution
    /// suffixes are case-sensitive everywhere.
    #[test]
    fn walked_marker_names_follow_the_platform_case_rules() {
        let dir = TempDir::new().expect("create temp dir");
        let root = dir.path();
        for file in [
            "upper/Package.json",
            "shout/CARGO.TOML",
            "sol/App.SLN",
            "solx/App.SLNX",
            "exact/App.sln",
        ] {
            write_fixture_file(root, file);
        }

        let expected: Vec<Fields> = if cfg!(windows) {
            vec![
                (root.join("exact"), vec![MonorepoStandard::DotNetSolution]),
                (root.join("shout"), vec![MonorepoStandard::CargoWorkspace]),
                (root.join("upper"), js_family()),
            ]
        } else {
            vec![(root.join("exact"), vec![MonorepoStandard::DotNetSolution])]
        };

        assert_eq!(assert_parity(root), expected);
    }

    /// A marker name followed by bytes that are not valid Unicode.
    fn non_unicode_marker_like_name() -> std::ffi::OsString {
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStrExt;
            std::ffi::OsStr::from_bytes(b"package.json\xff").to_os_string()
        }
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStringExt;
            let mut wide: Vec<u16> = "package.json".encode_utf16().collect();
            // An unpaired surrogate.
            wide.push(0xD800);
            std::ffi::OsString::from_wide(&wide)
        }
    }

    /// A non-Unicode basename is never a marker, and a walk that meets one
    /// keeps going. Filesystems that refuse such names (APFS) skip the walk
    /// half and say so.
    #[test]
    fn non_unicode_basenames_are_not_markers() {
        let dir = TempDir::new().expect("create temp dir");
        let root = dir.path();
        let name = non_unicode_marker_like_name();
        let path = root.join("a").join(&name);

        assert!(!is_nested_marker_path(&path));
        assert!(candidates_from_marker_paths(root, std::slice::from_ref(&path)).is_empty());

        write_fixture_file(root, "a/Cargo.toml");
        if let Err(error) = std::fs::write(&path, "{}\n") {
            eprintln!("SKIP walk half: this filesystem refuses non-Unicode names ({error})");
            return;
        }
        assert_eq!(
            assert_parity(root),
            vec![(root.join("a"), vec![MonorepoStandard::CargoWorkspace])]
        );
    }

    /// Restores a directory's mode so `TempDir` can delete it.
    #[cfg(unix)]
    struct RestoreMode(PathBuf);

    #[cfg(unix)]
    impl Drop for RestoreMode {
        fn drop(&mut self) {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&self.0, std::fs::Permissions::from_mode(0o755));
        }
    }

    /// Walk errors are best effort: an unreadable directory is skipped and the
    /// rest of the tree is still searched (ruling R6).
    ///
    /// Unix only, where a mode-000 directory is an unprivileged read denial.
    /// Native Windows would need ACL editing this crate does not do, so it is
    /// not asserted there; a privileged Unix run (root) skips and says so.
    #[cfg(unix)]
    #[test]
    fn an_unreadable_directory_is_skipped_and_the_walk_continues() {
        use std::os::unix::fs::PermissionsExt;

        let dir = TempDir::new().expect("create temp dir");
        let root = dir.path();
        write_fixture_file(root, "locked/inner/Cargo.toml");
        write_fixture_file(root, "open/Cargo.toml");
        let locked = root.join("locked");
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000))
            .expect("lock directory");
        let _restore = RestoreMode(locked.clone());

        if std::fs::read_dir(&locked).is_ok() {
            eprintln!("SKIP: this process can read a mode-000 directory (privileged)");
            return;
        }

        assert_eq!(
            assert_parity(root),
            vec![(root.join("open"), vec![MonorepoStandard::CargoWorkspace])]
        );
    }

    /// Environment variable naming the isolated home the Git-ignore child must
    /// run under; its absence means the child was started by hand.
    const ISOLATED_HOME_ENV: &str = "SNIFF_NESTED_WALK_ISOLATED_HOME";

    const GIT_IGNORE_CHILD: &str = "filesystem::repo::nested::tests::git_ignore_rules_child";

    /// Printed by the child once every assertion has run.
    const GIT_IGNORE_CHILD_SENTINEL: &str = "nested-walk git-ignore child: all assertions ran";

    /// Git ignore semantics under a controlled Git configuration.
    ///
    /// `ignore` reads the global excludes file from the process's home
    /// directory, so the assertions run in a child test process with `HOME`
    /// (and `USERPROFILE`) pointed at a disposable home. Changing this
    /// process's environment instead would race every concurrently running
    /// test.
    #[test]
    fn git_ignore_rules_apply_under_an_isolated_git_configuration() {
        let home = TempDir::new().expect("create temp home");
        write_fixture_file(home.path(), ".config/git/ignore");
        std::fs::write(
            home.path().join(".config/git/ignore"),
            "globally-ignored/\n",
        )
        .expect("write global excludes");

        let executable = std::env::current_exe().expect("current test executable should resolve");
        let output = std::process::Command::new(executable)
            .args([GIT_IGNORE_CHILD, "--exact", "--ignored", "--nocapture"])
            .env("HOME", home.path())
            .env("USERPROFILE", home.path())
            .env_remove("XDG_CONFIG_HOME")
            .env(ISOLATED_HOME_ENV, home.path())
            .output()
            .expect("run the git-ignore child test");
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        assert!(
            output.status.success(),
            "child failed:\nstdout:\n{stdout}\nstderr:\n{stderr}"
        );
        assert!(
            stdout.contains(GIT_IGNORE_CHILD_SENTINEL),
            "the child test did not run its assertions:\nstdout:\n{stdout}\nstderr:\n{stderr}"
        );
    }

    /// Child half of `git_ignore_rules_apply_under_an_isolated_git_configuration`.
    #[test]
    #[ignore = "subprocess fixture invoked by the git-ignore isolation test"]
    fn git_ignore_rules_child() {
        // Started by hand (e.g. a blanket ignored-test run), the host's own Git
        // configuration would leak in, so there is nothing sound to assert.
        let Some(home) = std::env::var_os(ISOLATED_HOME_ENV) else {
            return;
        };
        assert_eq!(std::env::var_os("HOME"), Some(home));

        let files = [
            "found/Cargo.toml",
            "generated/Cargo.toml",
            "drop/pom.xml",
            "keep/pom.xml",
            "tool-cache/package.json",
            "local-only/go.work",
            "globally-ignored/nx.json",
            "sub/private/lerna.json",
            "sub/public/lerna.json",
        ];
        let write_ignore_files = |root: &Path| {
            std::fs::write(
                root.join(".gitignore"),
                "generated/\n**/pom.xml\n!keep/pom.xml\n",
            )
            .expect("write .gitignore");
            std::fs::write(root.join(".ignore"), "tool-cache/\n").expect("write .ignore");
            std::fs::write(root.join("sub/.gitignore"), "private/\n")
                .expect("write nested .gitignore");
        };

        // In a Git repository every rule applies. Nothing is committed, so an
        // untracked but unignored marker must still be found.
        let git_dir = TempDir::new().expect("create temp dir");
        let git_root = git_dir.path();
        git2::Repository::init(git_root).expect("init fixture repository");
        for file in files {
            write_fixture_file(git_root, file);
        }
        write_ignore_files(git_root);
        std::fs::create_dir_all(git_root.join(".git/info")).expect("create .git/info");
        std::fs::write(git_root.join(".git/info/exclude"), "local-only/\n")
            .expect("write info/exclude");

        assert_eq!(
            assert_parity(git_root),
            vec![
                (
                    git_root.join("found"),
                    vec![MonorepoStandard::CargoWorkspace]
                ),
                (
                    git_root.join("keep"),
                    vec![MonorepoStandard::MavenMultiModule]
                ),
                (git_root.join("sub/public"), vec![MonorepoStandard::Lerna]),
            ],
            "Git root"
        );

        // Outside a repository only `.ignore` applies; `.gitignore` and the
        // global excludes need Git.
        let plain_dir = TempDir::new().expect("create temp dir");
        let plain_root = plain_dir.path();
        for file in files {
            write_fixture_file(plain_root, file);
        }
        write_ignore_files(plain_root);

        assert_eq!(
            assert_parity(plain_root),
            vec![
                (
                    plain_root.join("drop"),
                    vec![MonorepoStandard::MavenMultiModule]
                ),
                (
                    plain_root.join("found"),
                    vec![MonorepoStandard::CargoWorkspace]
                ),
                (
                    plain_root.join("generated"),
                    vec![MonorepoStandard::CargoWorkspace]
                ),
                (
                    plain_root.join("globally-ignored"),
                    vec![MonorepoStandard::Nx]
                ),
                (
                    plain_root.join("keep"),
                    vec![MonorepoStandard::MavenMultiModule]
                ),
                (
                    plain_root.join("local-only"),
                    vec![MonorepoStandard::GoWorkspace]
                ),
                (plain_root.join("sub/private"), vec![MonorepoStandard::Lerna]),
                (plain_root.join("sub/public"), vec![MonorepoStandard::Lerna]),
            ],
            "non-Git root"
        );
        println!("{GIT_IGNORE_CHILD_SENTINEL}");
    }

    /// An empty root is still one logical walk.
    #[test]
    fn an_empty_root_records_one_logical_walk() {
        use crate::performance::testing;

        let dir = TempDir::new().expect("create temp dir");
        for threads in WORKER_CONFIGS {
            let (candidates, counts) =
                testing::measure(|| walk_for_nested_markers_with_threads(dir.path(), threads));

            assert!(candidates.is_empty(), "threads = {threads:?}");
            assert_eq!(
                counts.get(counters::FS_READ_DIRS),
                1,
                "threads = {threads:?}: {:?}",
                counts.all()
            );
            assert_eq!(
                counts.get(counters::REPO_NESTED_MARKER_WALKS),
                1,
                "threads = {threads:?}: {:?}",
                counts.all()
            );
        }
    }

    /// Supplied marker evidence — including an empty list — starts no
    /// fallback walk, and a populated list reaches the same detector outcomes
    /// the fallback walk does.
    #[test]
    fn supplied_evidence_starts_no_fallback_walk() {
        use super::super::detection::RepoEvidence;
        use crate::performance::testing;

        let dir = TempDir::new().expect("create temp dir");
        let root = dir.path();
        std::fs::create_dir_all(root.join("tools/a")).expect("create nested workspace");
        std::fs::write(
            root.join("tools/Cargo.toml"),
            "[workspace]\nmembers = [\"a\"]\n",
        )
        .expect("write nested workspace manifest");
        std::fs::write(
            root.join("tools/a/Cargo.toml"),
            "[package]\nname = \"a\"\nversion = \"0.1.0\"\n",
        )
        .expect("write member manifest");

        let discover = |evidence: RepoEvidence<'_>| {
            let mut seeds = Vec::new();
            let mut outcomes = Vec::new();
            discover_nested_workspace_outcomes(
                root,
                evidence,
                &[],
                &ManifestStore::default(),
                &mut seeds,
                &mut outcomes,
            )
            .expect("nested discovery");
            let summary: Vec<(MonorepoStandard, PathBuf, usize)> = outcomes
                .into_iter()
                .map(|outcome| (outcome.standard, outcome.root, outcome.seeds.len()))
                .collect();
            (summary, seeds.len())
        };

        let (fallback, fallback_counts) = testing::measure(|| discover(RepoEvidence::default()));
        assert_eq!(
            fallback_counts.get(counters::REPO_NESTED_MARKER_WALKS),
            1,
            "{:?}",
            fallback_counts.all()
        );
        assert_eq!(
            fallback.0,
            vec![(MonorepoStandard::CargoWorkspace, root.join("tools"), 1)],
            "the fallback walk finds the nested workspace"
        );

        let markers = vec![root.join("tools/Cargo.toml"), root.join("tools/a/Cargo.toml")];
        let (supplied, supplied_counts) = testing::measure(|| {
            discover(RepoEvidence {
                nested_markers: Some(&markers),
                ..RepoEvidence::default()
            })
        });
        assert_eq!(
            supplied_counts.get(counters::REPO_NESTED_MARKER_WALKS),
            0,
            "{:?}",
            supplied_counts.all()
        );
        assert_eq!(
            supplied, fallback,
            "supplied evidence reaches the fallback's outcomes and seeds"
        );

        let (empty, empty_counts) = testing::measure(|| {
            discover(RepoEvidence {
                nested_markers: Some(&[]),
                ..RepoEvidence::default()
            })
        });
        assert_eq!(empty, (Vec::new(), 0), "empty evidence means no candidates");
        assert_eq!(
            empty_counts.get(counters::REPO_NESTED_MARKER_WALKS),
            0,
            "{:?}",
            empty_counts.all()
        );
        assert_eq!(
            empty_counts.get(counters::FS_READ_DIRS),
            0,
            "{:?}",
            empty_counts.all()
        );
    }

    /// Every visitor's `WorkerCollector` flushes into the request (ruling
    /// R2): the per-entry test counter recorded on worker threads sums to the
    /// serial reference's entry count. A lost flush reads short.
    ///
    /// Also prints how many distinct threads admitted entries, the
    /// "actual worker count" evidence for this fixture on this host.
    #[test]
    fn every_visitor_flushes_its_work_into_the_request() {
        use crate::performance::testing;

        let dir = TempDir::new().expect("create temp dir");
        let root = dir.path();
        build_wide_fixture(root);
        let entries = serial_reference_paths(root).len() as u64;
        assert_eq!(entries, 40 * 12 + 20 + 14 + 8, "fixture entry count");

        for threads in WORKER_CONFIGS {
            let (_, counts) =
                testing::measure(|| walk_for_nested_markers_with_threads(root, threads));

            assert_eq!(
                counts.get(ADMITTED_ENTRIES),
                entries,
                "threads = {threads:?}: a visitor's work did not reach the collector"
            );
            let worker_threads = counts
                .all()
                .keys()
                .filter(|name| name.starts_with(WORKER_THREAD_PREFIX))
                .count();
            eprintln!(
                "threads = {threads:?}: {worker_threads} distinct worker thread(s) admitted {entries} entries"
            );
            let cap = threads.unwrap_or(12);
            assert!(
                (1..=cap).contains(&worker_threads),
                "threads = {threads:?}: {worker_threads} worker threads"
            );
            if threads == Some(1) {
                assert_eq!(worker_threads, 1, "a one-worker walk runs on one thread");
            }
        }
    }
}
