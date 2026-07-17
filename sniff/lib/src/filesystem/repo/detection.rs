use crate::Result;
use crate::performance;
use crate::performance::counters;
use crate::request::RepoRequest;
use biscuit_file::serde_yaml_ng;
use biscuit_file::toml_crate;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::filesystem::file_types::{
    FileAssociation, FileInventory, FrameworkAccumulator, FrameworkKind, LanguageAccumulator,
    ProgrammingLanguage, accumulate_language_classification, build_association_breakdown,
    build_language_summary, is_command_runner_filename,
};

use super::cargo::{
    cargo_dependencies_from_value, cargo_features_from_value, cargo_package_name,
    cargo_package_version_with_source, detect_cargo_workspace,
};
use super::dotnet::{detect_dotnet_solution, root_has_solution_file};
use super::go::{
    detect_go_workspace, go_mod_dependencies_from_content, go_module_name_from_content,
};
use super::gradle::detect_gradle_workspace;
use super::manifest_index::{
    CargoLockVersions, ManifestIndex, discover_seeds_from_index,
    discover_seeds_with_optional_index as mi_discover_seeds_with_optional_index,
};
use super::maven::detect_maven_workspace;
use super::nested::discover_nested_workspace_outcomes;
use super::seed::{PackageSeed, merge_seeds};
use super::npm::{
    detect_bun_workspace, detect_npm_workspace, detect_pnpm_workspace, detect_rush_workspace,
    detect_yarn_workspace, npm_package_name, npm_package_version,
    package_json_dependencies_from_value, parse_package_json_workspace_patterns,
    parse_pnpm_workspace_patterns, resolve_js_package_manager,
};
use super::nx_turbo::{detect_lerna, detect_nx, detect_turborepo, parse_lerna_workspace_patterns};
use super::polyglot::{detect_bazel_workspace, detect_buck2_workspace, detect_pants_workspace};
use super::python::{
    parse_requirements_txt_dependencies, pyproject_dependencies_from_value, pyproject_package_name,
    pyproject_package_version,
};
use super::standard::{MonorepoLayer, MonorepoStandard, PackageProvenance, resolve_acting_binary};
pub(crate) use super::topology::DetectorOutcome;
use super::topology::{build_detected_standards, build_monorepo_layers, layers_imply_monorepo};
use super::types::{Package, PackageEcosystem, PackageFiles, PackageScanResult, RepoInfo};
use super::uv::detect_uv_workspace;

/// Borrowed, request-scoped evidence from one shared observation walk.
///
/// ## Notes
///
/// Every field is what some detector would otherwise re-enumerate the tree to
/// learn. A `None` field means "not observed", never "observed and empty", so a
/// detector falls back to its own walk rather than silently reporting nothing.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct RepoEvidence<'a> {
    pub(crate) manifest_index: Option<&'a ManifestIndex>,
    /// Directories holding a package manifest, exactly as observed.
    ///
    /// Deliberately unfiltered where [`RepoEvidence::manifest_index`] is not:
    /// membership globs resolve a boundary by marker presence alone and have
    /// never applied the index's generated/fixture exclusions.
    pub(crate) manifest_dirs: Option<&'a [PathBuf]>,
    pub(crate) nested_markers: Option<&'a [PathBuf]>,
    pub(crate) inventory: Option<&'a FileInventory>,
}

impl<'a> RepoEvidence<'a> {
    /// Borrow every evidence kind a view collected.
    ///
    /// The only constructor from a walk, so evidence and the root it was
    /// observed from cannot drift apart at a call site.
    pub(crate) fn from_view(view: &'a crate::filesystem::system_view::FilesystemSystemView) -> Self {
        Self {
            manifest_index: view.manifest_index.as_ref(),
            manifest_dirs: view.manifest_dirs.as_deref(),
            nested_markers: view.nested_markers.as_deref(),
            inventory: view.inventory.as_ref(),
        }
    }

    /// Evidence with the manifest index replaced.
    ///
    /// Used only by the lazy index build, which runs after the nested walk has
    /// already established that a nested-only topology exists.
    fn with_manifest_index(self, manifest_index: Option<&'a ManifestIndex>) -> Self {
        Self {
            manifest_index,
            ..self
        }
    }
}

/// Detect a repository at `root`, observing the tree once when full evidence is
/// required.
///
/// ## Notes
///
/// The `has_workspace_marker` gate is what keeps a full `detect_repo` over a
/// large non-repository directory — a system temp dir — from enumerating
/// unrelated subtrees. Without a root marker every root-level detector returns
/// `None`, so the observation index would be built and discarded. Nested
/// discovery below still walks for markers deeper in the tree in that case.
pub(crate) fn detect_repo_inner(
    root: &Path,
    structure_only: bool,
) -> Result<(Option<RepoInfo>, Option<FileInventory>)> {
    let request = if structure_only {
        RepoRequest::structure()
    } else {
        RepoRequest::full()
    };
    detect_repo_inner_with_request(root, &request)
}

/// Detect a repository using the complete repository request.
pub(crate) fn detect_repo_inner_with_request(
    root: &Path,
    request: &RepoRequest,
) -> Result<(Option<RepoInfo>, Option<FileInventory>)> {
    // Structure-only detection consumes no manifest index, inventory, or docs,
    // so observing the tree for them would classify every file for evidence it
    // discards. It uses the nested walk's smaller evidence set instead.
    if request.structure_only || !has_workspace_marker(root) {
        return detect_repo_inner_with_shared_request(root, request, RepoEvidence::default());
    }

    let view = crate::filesystem::system_view::build_filesystem_system_view(
        root,
        crate::filesystem::system_view::SharedWalkOptions::full_repo(),
    );
    detect_repo_inner_with_shared_request(root, request, RepoEvidence::from_view(&view))
}

/// Synthesize a single-package, non-monorepo [`RepoInfo`] from the manifest at
/// `root`, or `None` when `root` declares no recognizable package.
///
/// [`detect_repo_inner_with_shared`] returns `None` for an ordinary
/// single-package project — a `Cargo.toml` with `[package]` but no
/// `[workspace]`, or a lone `package.json` / `pyproject.toml` / `go.mod` —
/// because every workspace detector requires a membership marker. The
/// package-manager, dependency, and aggregate reporting paths still need that
/// root package's facts, so this builds the one-package catalog they consume.
pub(crate) fn synthesize_root_package_repo(root: &Path) -> Option<RepoInfo> {
    synthesize_root_package_repo_with_request(root, &RepoRequest::structure())
}

/// Synthesize a root-package repository using focused package details.
pub(crate) fn synthesize_root_package_repo_with_request(
    root: &Path,
    request: &RepoRequest,
) -> Option<RepoInfo> {
    if detect_package_ecosystem(root) == PackageEcosystem::Unknown {
        return None;
    }
    let lock_versions = if request.wants_dependencies() {
        CargoLockVersions::parse(&root.join("Cargo.lock"))
    } else {
        None
    };
    let package = create_package_with_request(
        root,
        root,
        MonorepoStandard::Unknown,
        PackageProvenance::ManifestScan,
        &lock_versions,
        request,
    );
    Some(RepoInfo {
        is_monorepo: false,
        root: root.to_path_buf(),
        packages: Some(vec![package]),
        ..RepoInfo::default()
    })
}

/// Cache for parsed manifest files to avoid redundant I/O during repo detection.
///
/// Each manifest file is read and parsed at most once per `create_package`
/// invocation. Returned values are owned so multiple downstream helpers can
/// inspect them without re-reading from disk.
#[derive(Default)]
pub(crate) struct ManifestCache {
    cargo: HashMap<PathBuf, Option<toml_crate::Value>>,
    npm: HashMap<PathBuf, Option<serde_json::Value>>,
    pyproject: HashMap<PathBuf, Option<toml_crate::Value>>,
    go_mod: HashMap<PathBuf, Option<String>>,
    raw_text: HashMap<PathBuf, Option<String>>,
}

impl ManifestCache {
    pub(crate) fn cargo(&mut self, path: &Path) -> Option<&toml_crate::Value> {
        let entry = self.cargo.entry(path.to_path_buf()).or_insert_with(|| {
            read_counted_manifest(path).and_then(|c| toml_crate::from_str(&c).ok())
        });
        entry.as_ref()
    }

    pub(crate) fn npm(&mut self, path: &Path) -> Option<&serde_json::Value> {
        let entry = self.npm.entry(path.to_path_buf()).or_insert_with(|| {
            read_counted_manifest(path).and_then(|c| serde_json::from_str(&c).ok())
        });
        entry.as_ref()
    }

    pub(crate) fn pyproject(&mut self, path: &Path) -> Option<&toml_crate::Value> {
        let entry = self.pyproject.entry(path.to_path_buf()).or_insert_with(|| {
            read_counted_manifest(path).and_then(|c| toml_crate::from_str(&c).ok())
        });
        entry.as_ref()
    }

    pub(crate) fn go_mod(&mut self, path: &Path) -> Option<&str> {
        let entry = self
            .go_mod
            .entry(path.to_path_buf())
            .or_insert_with(|| read_counted_manifest(path));
        entry.as_deref()
    }

    /// Generic raw-text cache for manifests that do not warrant a typed parser
    /// in the cache yet (composer.json, Gemfile, pom.xml, build.gradle,
    /// mix.exs, *.csproj, *.gemspec, requirements.txt). Used by test-runner
    /// repo detection to do substring searches over the declared dependency
    /// keys without re-reading the file per runner.
    pub(crate) fn raw_text(&mut self, path: &Path) -> Option<&str> {
        let entry = self
            .raw_text
            .entry(path.to_path_buf())
            .or_insert_with(|| read_counted_manifest(path));
        entry.as_deref()
    }
}

/// Read a manifest and record the work against the repo and filesystem
/// counters.
///
/// Every [`ManifestCache`] accessor funnels its miss path through here, and the
/// `or_insert_with` closures only run on a miss, so the counters measure unique
/// manifests read rather than the far larger number of accessor calls. Counting
/// at the accessor call sites instead would multiply-count every manifest that
/// name, version, dependency, and test-runner resolution each ask for.
fn read_counted_manifest(path: &Path) -> Option<String> {
    performance::increment_counter(counters::FS_FILE_OPENS, 1);
    let content = std::fs::read_to_string(path).ok()?;
    performance::increment_counter(counters::FS_BYTES_READ, content.len() as u64);
    performance::increment_counter(counters::REPO_MANIFEST_PARSES, 1);
    Some(content)
}

/// Probe for a path's existence, recording one metadata probe.
///
/// Repo detection decides almost everything by marker-file presence, so these
/// probes are the dominant syscall class here and are counted rather than the
/// individual `exists` calls being left invisible.
pub(crate) fn probe_exists(path: &Path) -> bool {
    performance::increment_counter(counters::FS_METADATA_PROBES, 1);
    path.exists()
}

/// Probe whether a path is a directory, recording one metadata probe.
pub(crate) fn probe_is_dir(path: &Path) -> bool {
    performance::increment_counter(counters::FS_METADATA_PROBES, 1);
    path.is_dir()
}

/// Build context shared while constructing `Package` values.
///
/// Bundles a per-call manifest cache with the optional `Cargo.lock` version
/// resolver so each manifest file is read at most once per `create_package`
/// invocation. The `ManifestCache` is cleared for each package so cache
/// growth is bounded by the manifests of a single package.
pub(crate) struct PackageBuildContext<'a> {
    pub(crate) manifests: ManifestCache,
    pub(crate) lock_versions: &'a Option<CargoLockVersions>,
}

impl<'a> PackageBuildContext<'a> {
    pub(crate) fn new(lock_versions: &'a Option<CargoLockVersions>) -> Self {
        Self {
            manifests: ManifestCache::default(),
            lock_versions,
        }
    }
}

/// Returns `true` when `root` contains at least one file that could make a
/// workspace detector succeed.
///
/// Every detector in [`detect_repo_inner_with_shared`] short-circuits to
/// `None` unless its marker file is present directly in `root`. Checking these
/// names up front lets us avoid building the manifest index (a full recursive
/// walk) for directories that cannot possibly be a workspace root.
fn has_workspace_marker(root: &Path) -> bool {
    const MARKERS: [&str; 18] = [
        "Cargo.toml",          // cargo workspace
        "nx.json",             // nx
        "turbo.json",          // turborepo
        "pnpm-workspace.yaml", // pnpm workspaces
        "package.json",        // npm / yarn / bun workspaces
        "yarn.lock",           // yarn workspaces
        "lerna.json",          // lerna
        "pyproject.toml",      // uv workspace
        "go.work",             // go workspace
        "settings.gradle",     // gradle multi-project
        "settings.gradle.kts", // gradle multi-project (kotlin dsl)
        "pom.xml",             // maven multi-module
        "rush.json",           // rush stack
        "WORKSPACE",           // bazel
        "WORKSPACE.bazel",     // bazel
        "MODULE.bazel",        // bazel (bzlmod)
        "pants.toml",          // pants
        ".buckconfig",         // buck2
    ];
    if MARKERS.iter().any(|name| probe_exists(&root.join(name))) {
        return true;
    }
    // .NET solution files have arbitrary names, so they are matched by extension
    // rather than by a fixed marker name.
    root_has_solution_file(root)
}

/// `evidence` carries whatever a shared observation walk already learned about
/// `root`'s tree. Absent fields fall back to a specialized walk.
pub(crate) fn detect_repo_inner_with_shared(
    root: &Path,
    structure_only: bool,
    evidence: RepoEvidence<'_>,
) -> Result<(Option<RepoInfo>, Option<FileInventory>)> {
    let request = if structure_only {
        RepoRequest::structure()
    } else {
        RepoRequest::full()
    };
    detect_repo_inner_with_shared_request(root, &request, evidence)
}

/// Detect a repository using shared observation evidence and focused detail
/// controls from `request`.
pub(crate) fn detect_repo_inner_with_shared_request(
    root: &Path,
    request: &RepoRequest,
    evidence: RepoEvidence<'_>,
) -> Result<(Option<RepoInfo>, Option<FileInventory>)> {
    let mut seeds: Vec<PackageSeed> = Vec::new();
    let mut outcomes: Vec<DetectorOutcome> = Vec::new();

    collect_outcome(
        detect_cargo_workspace(root, evidence)?,
        &mut seeds,
        &mut outcomes,
    );
    collect_outcome(detect_nx(root, evidence)?, &mut seeds, &mut outcomes);
    collect_outcome(
        detect_turborepo(root, evidence)?,
        &mut seeds,
        &mut outcomes,
    );
    collect_outcome(
        detect_pnpm_workspace(root, evidence)?,
        &mut seeds,
        &mut outcomes,
    );
    collect_outcome(
        detect_yarn_workspace(root, evidence)?,
        &mut seeds,
        &mut outcomes,
    );
    collect_outcome(
        detect_npm_workspace(root, evidence)?,
        &mut seeds,
        &mut outcomes,
    );
    collect_outcome(detect_lerna(root, evidence)?, &mut seeds, &mut outcomes);
    collect_outcome(
        detect_bun_workspace(root, evidence)?,
        &mut seeds,
        &mut outcomes,
    );
    collect_outcome(
        detect_uv_workspace(root, evidence)?,
        &mut seeds,
        &mut outcomes,
    );
    collect_outcome(detect_go_workspace(root)?, &mut seeds, &mut outcomes);
    collect_outcome(detect_gradle_workspace(root)?, &mut seeds, &mut outcomes);
    collect_outcome(detect_maven_workspace(root)?, &mut seeds, &mut outcomes);
    collect_outcome(detect_dotnet_solution(root)?, &mut seeds, &mut outcomes);
    collect_outcome(detect_rush_workspace(root)?, &mut seeds, &mut outcomes);

    // Polyglot leaf-marker build systems contribute one outcome per workspace
    // root (Bazel segments nested `WORKSPACE` subtrees into their own layer).
    collect_outcomes(
        root,
        detect_bazel_workspace(root)?,
        &mut seeds,
        &mut outcomes,
    );
    collect_outcomes(
        root,
        detect_pants_workspace(root)?,
        &mut seeds,
        &mut outcomes,
    );
    collect_outcomes(
        root,
        detect_buck2_workspace(root)?,
        &mut seeds,
        &mut outcomes,
    );

    // Root-manifest standards (pnpm, npm, Go, Gradle, Maven, ...) only fired
    // at the supplied root above. Walk the tree once for their marker files
    // and dispatch the matching detectors at each non-root candidate so the
    // topology is a real forest — e.g. a Cargo workspace at the root with a
    // pnpm workspace several directories down produces two layers, not one.
    //
    // `forbids_nested_roots` is computed up front because a `ForbidsNested`
    // standard (Cargo, uv) only blocks nested instances of itself — the
    // walker needs read access to the root outcomes while mutating the
    // shared outcomes list.
    let forbids_nested_roots: Vec<(PathBuf, MonorepoStandard)> = outcomes
        .iter()
        .filter(|o| {
            matches!(
                o.standard.spec().nesting_policy,
                super::standard::NestingPolicy::ForbidsNested
            )
        })
        .map(|o| (o.root.clone(), o.standard))
        .collect();
    discover_nested_workspace_outcomes(
        root,
        evidence,
        &forbids_nested_roots,
        &mut seeds,
        &mut outcomes,
    )?;

    if outcomes.is_empty() {
        return Ok((None, None));
    }

    // Full-mode nested package enrichment requires the manifest index.
    // `detect_repo_inner` only observes the tree for a root that carries a
    // workspace marker, but nested discovery may still have produced outcomes
    // from markers deeper down (e.g. `web/pnpm-workspace.yaml` under an
    // otherwise bare root). Whether that topology exists is unknowable until
    // the nested walk has run, so the index is built lazily here rather than
    // speculatively above. Structure-only mode never consults it.
    let lazy_manifest_index = if !request.structure_only && evidence.manifest_index.is_none() {
        Some(ManifestIndex::build(root))
    } else {
        None
    };
    let evidence = match &lazy_manifest_index {
        Some(index) => evidence.with_manifest_index(Some(index)),
        None => evidence,
    };

    // Build the topology from membership-declaring detectors. `is_monorepo` is
    // now the honest predicate: at least one layer must resolve non-degenerately.
    let mut monorepo_layers = build_monorepo_layers(&outcomes);
    let is_monorepo = layers_imply_monorepo(&monorepo_layers);
    let mut monorepo_standards =
        build_detected_standards(root, &outcomes, &monorepo_layers, is_monorepo);

    // Resolve the acting binary for every detected standard. A PATH-only index
    // is sufficient: wrapper scripts are checked directly against `root`.
    let executable_index = crate::executable_index::ExecutableIndex::build_path_only();
    for standard in &mut monorepo_standards {
        standard.binary =
            resolve_acting_binary(standard.standard, &standard.root, &executable_index);
    }

    // Upgrade provenance to `Lockfile` for ecosystems where the committed
    // lockfile is a high-fidelity membership source.
    for layer in &mut monorepo_layers {
        upgrade_provenance_with_lockfile(layer, &mut seeds);
    }

    if !request.structure_only {
        // Full mode: pick up packages nested inside an already-discovered
        // member's subtree. The query also returns the searched member itself;
        // that occurrence merges away below rather than being enriched twice.
        let workspace_seeds = seeds.clone();
        let index = evidence
            .manifest_index
            .expect("full repo detection requires a manifest index");
        for seed in &workspace_seeds {
            seeds.extend(discover_seeds_from_index(
                &seed.path,
                root,
                MonorepoStandard::Unknown,
                PackageProvenance::ManifestScan,
                index,
            ));
        }
    }

    // Discovery is complete. Collapse duplicate boundaries *before* enrichment:
    // every seed surviving here is enriched exactly once (R5).
    let seeds = merge_seeds(seeds);
    let mut locks = LockStore::default();
    let mut packages: Vec<Package> = seeds
        .iter()
        .map(|seed| {
            let lock_versions = if request.wants_dependencies() {
                locks.for_seed(seed)
            } else {
                &NO_LOCK_VERSIONS
            };
            create_package_from_seed(seed, lock_versions, request)
        })
        .collect();
    packages.sort_by(|a, b| a.relative.cmp(&b.relative));

    let repo_inventory = if !request.structure_only {
        // Build shared repo-level file inventory once for all packages
        let inventory = evidence
            .inventory
            .cloned()
            .or_else(|| crate::filesystem::file_types::scan_file_inventory(root).ok());
        refresh_package_boundaries(&mut packages, inventory.as_ref());
        inventory
    } else {
        None
    };
    if request.wants_dependencies() {
        resolve_internal_deps(&mut packages);
    }

    Ok((
        Some(RepoInfo {
            is_monorepo,
            root: root.to_path_buf(),
            dependencies: None,
            dev_dependencies: None,
            peer_dependencies: None,
            optional_dependencies: None,
            monorepo_standards,
            monorepo_layers,
            packages: Some(packages),
        }),
        repo_inventory,
    ))
}

/// Fold a single detector outcome into the shared collections.
///
/// Only membership-authority outcomes contribute seeds to the canonical
/// `RepoInfo.packages` catalog. Orchestrator-only standards (Nx, Turborepo,
/// Lerna) are still recorded as outcomes so they appear in
/// `monorepo_standards` and as orchestrators on layers, but their packages do
/// not own entries in the flat catalog.
fn collect_outcome(
    outcome: Option<DetectorOutcome>,
    seeds: &mut Vec<PackageSeed>,
    outcomes: &mut Vec<DetectorOutcome>,
) {
    let Some(outcome) = outcome else {
        return;
    };
    if outcome.standard.defines_membership() {
        seeds.extend(outcome.seeds.clone());
    }
    outcomes.push(outcome);
}

/// Fold a multi-root detector's outcomes into the shared collections.
///
/// Leaf-marker detectors (Bazel/Pants/Buck2) may report more than one workspace
/// root, so they hand back a list of [`DetectorOutcome`]s. Each outcome's seeds
/// are layer-root-relative; before joining the flat seed list they are rebased
/// to repo-root-relative so `RepoInfo.packages` stays uniformly framed.
///
/// Like [`collect_outcome`], only membership-authority outcomes contribute
/// seeds; polyglot standards define membership and therefore keep theirs.
fn collect_outcomes(
    repo_root: &Path,
    detected: Vec<DetectorOutcome>,
    seeds: &mut Vec<PackageSeed>,
    outcomes: &mut Vec<DetectorOutcome>,
) {
    for outcome in detected {
        if outcome.standard.defines_membership() {
            let mut flat_seeds = outcome.seeds.clone();
            for seed in &mut flat_seeds {
                seed.rebase_to_root(repo_root);
            }
            seeds.extend(flat_seeds);
        }
        outcomes.push(outcome);
    }
}

/// Upgrade a layer's provenance to `Lockfile` when the committed lockfile
/// corroborates the manifest-derived package set.
///
/// The manifest remains the authority when lockfile and manifest disagree; the
/// mismatch is recorded in `lockfile_match` so consumers can spot stale lockfiles.
fn upgrade_provenance_with_lockfile(layer: &mut MonorepoLayer, seeds: &mut [PackageSeed]) {
    let authority = layer.authority;
    let lockfile_result = match authority {
        MonorepoStandard::PnpmWorkspaces => pnpm_lockfile_matches(layer, seeds),
        MonorepoStandard::UvWorkspace => uv_lockfile_matches(layer, seeds),
        MonorepoStandard::CargoWorkspace => cargo_lockfile_matches(layer, seeds),
        _ => return,
    };

    let Some(matches) = lockfile_result else {
        return;
    };

    layer.lockfile_match = Some(matches);
    if matches {
        layer.provenance = PackageProvenance::Lockfile;
        for relative in &layer.packages {
            let key = normalize_layer_package_relative(relative);
            for seed in seeds.iter_mut().filter(|s| s.relative == key) {
                seed.provenance = PackageProvenance::Lockfile;
            }
        }
    }
}

/// Normalize a repo-relative layer package path for comparison with
/// [`Package::relative`]. Empty paths are preserved so they match root
/// packages (e.g. uv's always-counted workspace root).
fn normalize_layer_package_relative(path: &str) -> String {
    path.replace('\\', "/")
}

/// Compute a path relative to a layer root, normalizing separators.
///
/// Returns `None` when `path` is not under `layer_root`.
fn layer_relative_path(path: &Path, layer_root: &Path) -> Option<String> {
    let rel = path.strip_prefix(layer_root).ok()?;
    rel.to_str().map(normalize_layer_package_relative)
}

/// Parse `pnpm-lock.yaml` and compare its `importers:` keys to the layer's
/// package set. Returns `Some(true)` only when the two sets are equal, so a
/// stale lockfile with extra importers is reported as a mismatch rather than
/// silently upgrading provenance to `Lockfile`. Returns `None` when the
/// lockfile is absent or unparseable.
///
/// The pnpm root importer key `"."` is normalized away because the manifest
/// globs never list the root — both sides are compared as member sets without
/// the workspace root.
fn pnpm_lockfile_matches(layer: &MonorepoLayer, seeds: &[PackageSeed]) -> Option<bool> {
    let lock_path = layer.root.join("pnpm-lock.yaml");
    performance::increment_counter(counters::FS_FILE_OPENS, 1);
    let content = std::fs::read_to_string(&lock_path).ok()?;
    performance::increment_counter(counters::FS_BYTES_READ, content.len() as u64);
    performance::increment_counter(counters::REPO_LOCKFILE_PARSES, 1);
    let parsed: serde_yaml_ng::Value = serde_yaml_ng::from_str(&content).ok()?;

    let importers = parsed.get("importers")?.as_mapping()?;
    let lock_members: std::collections::HashSet<String> = importers
        .keys()
        .filter_map(|k| k.as_str())
        .map(|s| {
            s.trim_start_matches('.')
                .trim_start_matches('/')
                .to_string()
        })
        .filter(|s| !s.is_empty())
        .collect();

    if lock_members.is_empty() {
        return Some(false);
    }

    let manifest_members: std::collections::HashSet<String> = layer
        .packages
        .iter()
        .filter_map(|rel| {
            let key = normalize_layer_package_relative(rel);
            seeds
                .iter()
                .find(|s| s.relative == key)
                .and_then(|s| layer_relative_path(&s.path, &layer.root))
        })
        .collect();

    Some(manifest_members == lock_members)
}

/// Parse `uv.lock` and compare its `workspace.members` entries to the layer's
/// package set. Returns `Some(true)` only on set equality, so a stale lockfile
/// with extra or missing members is reported as a mismatch. Returns `None`
/// when the lockfile is absent or unparseable.
///
/// The uv root member (`"."`) is kept on both sides because uv's
/// `RootMembership::Always` adds the root to `layer.packages`, so both sets
/// include the root.
fn uv_lockfile_matches(layer: &MonorepoLayer, seeds: &[PackageSeed]) -> Option<bool> {
    let lock_path = layer.root.join("uv.lock");
    performance::increment_counter(counters::FS_FILE_OPENS, 1);
    let content = std::fs::read_to_string(&lock_path).ok()?;
    performance::increment_counter(counters::FS_BYTES_READ, content.len() as u64);
    performance::increment_counter(counters::REPO_LOCKFILE_PARSES, 1);
    let parsed: toml_crate::Value = toml_crate::from_str(&content).ok()?;

    let members = parsed.get("workspace")?.get("members")?.as_array()?;

    let lock_members: std::collections::HashSet<String> = members
        .iter()
        .filter_map(|v| {
            if let Some(s) = v.as_str() {
                return Some(s.trim_end_matches('/').to_string());
            }
            v.get("root")
                .and_then(|r| r.as_str())
                .map(|s| s.trim_end_matches('/').to_string())
        })
        .collect();

    if lock_members.is_empty() {
        return Some(false);
    }

    let manifest_members: std::collections::HashSet<String> = layer
        .packages
        .iter()
        .filter_map(|rel| {
            let key = normalize_layer_package_relative(rel);
            seeds.iter().find(|s| s.relative == key).and_then(|seed| {
                let s = layer_relative_path(&seed.path, &layer.root)?;
                // uv counts the workspace root (`.`) as a member; represent the
                // empty relative path the same way the lockfile does.
                Some(if s.is_empty() { ".".to_string() } else { s })
            })
        })
        .collect();

    Some(manifest_members == lock_members)
}

/// Check that every globbed Cargo member has a `[package].name` present in the
/// root `Cargo.lock` `[[package]]` table.
fn cargo_lockfile_matches(layer: &MonorepoLayer, seeds: &[PackageSeed]) -> Option<bool> {
    let lock_path = layer.root.join("Cargo.lock");
    let lock_versions = CargoLockVersions::parse(&lock_path)?;

    for relative in &layer.packages {
        let key = normalize_layer_package_relative(relative);
        let seed = seeds.iter().find(|s| s.relative == key)?;
        let cargo_toml = seed.path.join("Cargo.toml");
        let name = read_counted_manifest(&cargo_toml)
            .and_then(|content| toml_crate::from_str::<toml_crate::Value>(&content).ok())
            .and_then(|parsed| {
                parsed
                    .get("package")?
                    .get("name")?
                    .as_str()
                    .map(String::from)
            });

        let Some(name) = name else {
            return Some(false);
        };
        if lock_versions.resolve(&name).is_none() {
            return Some(false);
        }
    }

    Some(true)
}

pub(crate) fn collect_default_workspace_patterns(root: &Path) -> Vec<String> {
    let mut patterns = Vec::new();

    if let Ok(Some(package_json_patterns)) =
        parse_package_json_workspace_patterns(&root.join("package.json"))
    {
        patterns.extend(package_json_patterns);
    }

    if let Ok(pnpm_patterns) = parse_pnpm_workspace_patterns(&root.join("pnpm-workspace.yaml")) {
        patterns.extend(pnpm_patterns);
    }

    if let Some(lerna_patterns) = parse_lerna_workspace_patterns(&root.join("lerna.json")) {
        patterns.extend(lerna_patterns);
    }

    dedupe_patterns(patterns)
}

// ============================================================================
// File categorization
// ============================================================================

/// Categorizes files in a package root directory into configuration, documentation,
/// editor config, and command runner files.
///
/// All paths are stored relative to the repo root for portability.
/// Only performs a shallow scan of the package root directory (no recursion).
fn detect_package_files(package_relative: &str, inventory: &FileInventory) -> PackageFiles {
    let mut files = PackageFiles::default();

    for classification in inventory.classifications.iter() {
        let repo_relative = package_relative_path(package_relative, &classification.path);
        let file_name = classification
            .path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default();

        if file_name == ".editorconfig" {
            files.editor_config = Some(repo_relative.clone());
            continue;
        }
        if is_command_runner_filename(file_name) {
            files.command_runner.push(repo_relative.clone());
            continue;
        }

        match classification.association {
            FileAssociation::Configuration => files.configuration.push(repo_relative),
            FileAssociation::Documentation => files.documentation.push(repo_relative),
            _ => {}
        }
    }

    files.configuration.sort();
    files.configuration.dedup();
    files.documentation.sort();
    files.documentation.dedup();
    files.command_runner.sort();
    files.command_runner.dedup();

    files
}

fn package_relative_path(package_relative: &str, path: &Path) -> PathBuf {
    if package_relative.is_empty() {
        path.to_path_buf()
    } else {
        PathBuf::from(package_relative).join(path)
    }
}

// ============================================================================
// Package name resolution
// ============================================================================

/// Determines the native package name based on manifests present in the package.
fn resolve_package_name(ctx: &mut PackageBuildContext<'_>, path: &Path, root: &Path) -> String {
    let cargo_toml = path.join("Cargo.toml");
    if probe_exists(&cargo_toml)
        && let Some(name) = ctx
            .manifests
            .cargo(&cargo_toml)
            .and_then(cargo_package_name)
    {
        return name;
    }

    let package_json = path.join("package.json");
    if probe_exists(&package_json)
        && let Some(name) = ctx.manifests.npm(&package_json).and_then(npm_package_name)
    {
        return name;
    }

    let pyproject_toml = path.join("pyproject.toml");
    if probe_exists(&pyproject_toml)
        && let Some(name) = ctx
            .manifests
            .pyproject(&pyproject_toml)
            .and_then(pyproject_package_name)
    {
        return name;
    }

    let go_mod = path.join("go.mod");
    if probe_exists(&go_mod)
        && let Some(name) = ctx
            .manifests
            .go_mod(&go_mod)
            .and_then(go_module_name_from_content)
    {
        return name;
    }

    // Fallback: relative path from root
    make_relative_path(path, root)
}

/// Determines the package version based on manifests present in the package.
fn resolve_package_version(
    ctx: &mut PackageBuildContext<'_>,
    path: &Path,
    root: &Path,
) -> Option<String> {
    let cargo_toml = path.join("Cargo.toml");
    if probe_exists(&cargo_toml) {
        // Resolve `version.workspace = true` against the root manifest so the
        // package catalog reports inherited versions, matching what
        // `aggregate_versions` reports for `sniff repo version`. The source and
        // `inherited` flag are unused here; the catalog stores only the string.
        return ctx
            .manifests
            .cargo(&cargo_toml)
            .and_then(|parsed| cargo_package_version_with_source(parsed, &cargo_toml, root))
            .map(|(version, _, _)| version);
    }

    let package_json = path.join("package.json");
    if probe_exists(&package_json) {
        if let Some(version) = ctx
            .manifests
            .npm(&package_json)
            .and_then(npm_package_version)
        {
            return Some(version);
        }
        if path != root {
            let root_package_json = root.join("package.json");
            if probe_exists(&root_package_json) {
                return ctx
                    .manifests
                    .npm(&root_package_json)
                    .and_then(npm_package_version);
            }
        }
    }

    let pyproject_toml = path.join("pyproject.toml");
    if probe_exists(&pyproject_toml)
        && let Some(version) = ctx
            .manifests
            .pyproject(&pyproject_toml)
            .and_then(pyproject_package_version)
    {
        return Some(version);
    }

    None
}

// ============================================================================
// Internal dependency graph
// ============================================================================

/// Resolves internal dependencies between packages in a workspace.
///
/// Two-pass algorithm:
/// 1. Scan each package's dependency lists for names matching other package names → `depends_on`
/// 2. Invert the relationship → `used_by`
pub(crate) fn resolve_internal_deps(packages: &mut [Package]) {
    let package_names: HashSet<String> = packages.iter().map(|p| p.name.clone()).collect();

    // Pass 1: populate depends_on
    for pkg in packages.iter_mut() {
        let mut internal_deps = Vec::new();
        let mut seen = HashSet::new();
        for dep_list in [
            pkg.dependencies.as_ref(),
            pkg.dev_dependencies.as_ref(),
            pkg.peer_dependencies.as_ref(),
            pkg.optional_dependencies.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            for dep in dep_list {
                if package_names.contains(&dep.name)
                    && dep.name != pkg.name
                    && seen.insert(dep.name.as_str())
                {
                    internal_deps.push(dep.name.clone());
                }
            }
        }
        internal_deps.sort();
        pkg.depends_on = internal_deps;
    }

    // Pass 2: invert to populate used_by
    let mut used_by_map: HashMap<String, Vec<String>> = HashMap::new();
    for pkg in packages.iter() {
        for dep in &pkg.depends_on {
            used_by_map
                .entry(dep.clone())
                .or_default()
                .push(pkg.name.clone());
        }
    }

    for pkg in packages.iter_mut() {
        let mut used_by = used_by_map.remove(&pkg.name).unwrap_or_default();
        used_by.sort();
        pkg.used_by = used_by;
    }
}

// ============================================================================
// Helper functions
// ============================================================================

/// Creates a relative path string from root.
pub(crate) fn make_relative_path(path: &Path, root: &Path) -> String {
    path.strip_prefix(root)
        .ok()
        .and_then(|rel| rel.to_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            path.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default()
        })
}

/// Normalizes a path for deduplication without resolving syscalls.
///
/// Uses `std::fs::canonicalize` as a best-effort optimization, but falls back
/// to simple path cleanup when the filesystem call fails. This avoids
/// unnecessary syscalls during monorepo package merging where symlink
/// resolution is not strictly required.
pub(crate) fn canonicalize_path(path: &Path) -> PathBuf {
    performance::increment_counter(counters::FS_CANONICALIZATIONS, 1);
    std::fs::canonicalize(path).unwrap_or_else(|_| normalize_path(path))
}

/// Lightweight path normalization without filesystem access.
///
/// Cleans up `.` and `..` components and resolves relative paths against
/// the current working directory when needed.
pub(crate) fn normalize_path(path: &Path) -> PathBuf {
    if path.is_absolute() {
        let mut normalized = PathBuf::new();
        for component in path.components() {
            match component {
                std::path::Component::Prefix(p) => normalized.push(p.as_os_str()),
                std::path::Component::RootDir => normalized.push("/"),
                std::path::Component::CurDir => {}
                std::path::Component::ParentDir => {
                    normalized.pop();
                }
                std::path::Component::Normal(name) => normalized.push(name),
            }
        }
        normalized
    } else {
        std::env::current_dir()
            .unwrap_or_default()
            .join(path)
            .components()
            .fold(PathBuf::new(), |mut acc, component| {
                match component {
                    std::path::Component::Prefix(p) => acc.push(p.as_os_str()),
                    std::path::Component::RootDir => acc.push("/"),
                    std::path::Component::CurDir => {}
                    std::path::Component::ParentDir => {
                        acc.pop();
                    }
                    std::path::Component::Normal(name) => acc.push(name),
                }
                acc
            })
    }
}

/// Derives the package area from a relative path.
///
/// The area is the directory path between the repo root and the package directory.
/// Returns "root" when the package sits directly under the repo root.
fn make_package_area(relative: &str) -> String {
    let path = Path::new(relative);
    match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent.to_string_lossy().to_string(),
        _ => "root".to_string(),
    }
}

/// Detects structured file metadata in a package directory.
fn detect_package_languages(
    package_relative: &str,
    path: &Path,
    exclude_roots: &[PathBuf],
    repo_inventory: Option<&FileInventory>,
) -> PackageScanResult {
    let inventory = if let Some(repo_inv) = repo_inventory {
        crate::filesystem::file_types::project_package_inventory(repo_inv, path, exclude_roots)
    } else {
        match crate::filesystem::file_types::scan_file_inventory_with_exclusions(
            path,
            exclude_roots,
        ) {
            Ok(inv) => inv,
            Err(_) => return PackageScanResult::default(),
        }
    };

    let (file_breakdown, language_breakdown) =
        crate::filesystem::file_types::summarize_file_inventory(&inventory);
    PackageScanResult {
        language_breakdown,
        file_breakdown,
        compatibility: detect_package_files(package_relative, &inventory),
    }
}

/// Detects dependency managers present in a package directory.
fn detect_package_managers(path: &Path) -> Vec<String> {
    let mut managers = Vec::new();

    if probe_exists(&path.join("Cargo.toml")) {
        managers.push("cargo".to_string());
    }

    let has_package_json = probe_exists(&path.join("package.json"));
    let has_pnpm_lock = probe_exists(&path.join("pnpm-lock.yaml"));
    let has_yarn_lock = probe_exists(&path.join("yarn.lock"));

    if has_pnpm_lock {
        managers.push("pnpm".to_string());
    } else if has_yarn_lock {
        managers.push("yarn".to_string());
    } else if has_package_json {
        managers.push("npm".to_string());
    }

    if probe_exists(&path.join("requirements.txt")) || probe_exists(&path.join("pyproject.toml")) {
        managers.push("pip".to_string());
    }

    if probe_exists(&path.join("go.mod")) {
        managers.push("go".to_string());
    }

    managers
}

fn detect_package_ecosystem(path: &Path) -> PackageEcosystem {
    if probe_exists(&path.join("Cargo.toml")) {
        return PackageEcosystem::Cargo;
    }
    if probe_exists(&path.join("package.json")) {
        return PackageEcosystem::Node;
    }
    if probe_exists(&path.join("pyproject.toml")) || probe_exists(&path.join("requirements.txt")) {
        return PackageEcosystem::Python;
    }
    if probe_exists(&path.join("go.mod")) {
        return PackageEcosystem::Go;
    }

    PackageEcosystem::Unknown
}

pub(crate) fn dedupe_patterns(patterns: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut deduped = Vec::new();

    for pattern in patterns {
        if seen.insert(pattern.clone()) {
            deduped.push(pattern);
        }
    }

    deduped
}

/// Re-compute language/file stats for every package using a shared repo
/// inventory.
///
/// ## Algorithm
///
/// 1. Build a HashMap from canonical package path → index.
/// 2. Derive nested-package relationships by walking parent directories.
/// 3. When a repo inventory is provided, do a single pass over all
///    classifications, assign each to the deepest containing package, and
///    accumulate stats directly into per-package buckets.
///
/// This is O(F·depth + P·depth) instead of the previous O(P·F) + O(P²).
#[doc(hidden)]
pub fn refresh_package_boundaries(
    packages: &mut [Package],
    repo_inventory: Option<&FileInventory>,
) {
    if packages.is_empty() {
        return;
    }

    let package_rel_to_index: HashMap<&Path, usize> = packages
        .iter()
        .enumerate()
        .map(|(i, pkg)| (Path::new(&pkg.relative), i))
        .collect();

    let mut nested_packages: Vec<Vec<String>> = vec![Vec::new(); packages.len()];
    for (child_idx, package) in packages.iter().enumerate() {
        let mut current = Path::new(&package.relative);
        while let Some(parent) = current.parent() {
            if let Some(&parent_idx) = package_rel_to_index.get(parent)
                && parent_idx != child_idx
            {
                nested_packages[parent_idx].push(package.name.clone());
                break;
            }
            current = parent;
        }
    }
    for list in &mut nested_packages {
        list.sort();
    }

    if let Some(inventory) = repo_inventory {
        let mut per_pkg_lang: Vec<HashMap<ProgrammingLanguage, LanguageAccumulator>> =
            (0..packages.len()).map(|_| HashMap::new()).collect();
        let mut per_pkg_fw: Vec<HashMap<FrameworkKind, FrameworkAccumulator>> =
            (0..packages.len()).map(|_| HashMap::new()).collect();
        let mut per_pkg_assoc: Vec<HashMap<FileAssociation, Vec<PathBuf>>> =
            (0..packages.len()).map(|_| HashMap::new()).collect();
        let mut per_pkg_files: Vec<PackageFiles> = (0..packages.len())
            .map(|_| PackageFiles::default())
            .collect();
        let mut per_pkg_total: Vec<usize> = vec![0; packages.len()];

        for classification in inventory.classifications.iter() {
            let mut assigned_pkg = None;
            let mut current = classification.path.as_path();
            while let Some(parent) = current.parent() {
                if let Some(&pkg_idx) = package_rel_to_index.get(parent) {
                    assigned_pkg = Some(pkg_idx);
                    break;
                }
                current = parent;
            }

            let Some(pkg_idx) = assigned_pkg else {
                continue;
            };

            per_pkg_total[pkg_idx] += 1;

            accumulate_language_classification(
                classification,
                &mut per_pkg_lang[pkg_idx],
                &mut per_pkg_fw[pkg_idx],
            );

            per_pkg_assoc[pkg_idx]
                .entry(classification.association)
                .or_default()
                .push(classification.path.clone());

            let package_relative = &packages[pkg_idx].relative;
            let repo_relative = package_relative_path(package_relative, &classification.path);
            let file_name = classification
                .path
                .file_name()
                .and_then(|v| v.to_str())
                .unwrap_or_default();

            if file_name == ".editorconfig" {
                per_pkg_files[pkg_idx].editor_config = Some(repo_relative);
            } else if is_command_runner_filename(file_name) {
                per_pkg_files[pkg_idx].command_runner.push(repo_relative);
            } else {
                match classification.association {
                    FileAssociation::Configuration => {
                        per_pkg_files[pkg_idx].configuration.push(repo_relative);
                    }
                    FileAssociation::Documentation => {
                        per_pkg_files[pkg_idx].documentation.push(repo_relative);
                    }
                    _ => {}
                }
            }
        }

        for (i, package) in packages.iter_mut().enumerate() {
            package.nested_packages = nested_packages[i].clone();

            let lang_summary = build_language_summary(
                std::mem::take(&mut per_pkg_lang[i]),
                std::mem::take(&mut per_pkg_fw[i]),
                per_pkg_total[i],
            );
            package.primary_language = lang_summary.primary;
            package.secondary_languages = lang_summary.secondary;
            package.languages = lang_summary.languages;
            package.frameworks = lang_summary.frameworks;

            package.file_associations = build_association_breakdown(
                std::mem::take(&mut per_pkg_assoc[i]),
                per_pkg_total[i],
            );

            per_pkg_files[i].configuration.sort();
            per_pkg_files[i].configuration.dedup();
            per_pkg_files[i].documentation.sort();
            per_pkg_files[i].documentation.dedup();
            per_pkg_files[i].command_runner.sort();
            per_pkg_files[i].command_runner.dedup();

            package.configuration = std::mem::take(&mut per_pkg_files[i].configuration);
            package.documentation = std::mem::take(&mut per_pkg_files[i].documentation);
            package.editor_config = per_pkg_files[i].editor_config.take();
            package.command_runner = std::mem::take(&mut per_pkg_files[i].command_runner);
        }
    } else {
        let name_to_path: HashMap<&str, PathBuf> = packages
            .iter()
            .map(|p| (p.name.as_str(), p.path.clone()))
            .collect();
        let mut nested_roots_by_index: Vec<Vec<PathBuf>> = vec![Vec::new(); packages.len()];
        for (index, names) in nested_packages.iter().enumerate() {
            for name in names {
                if let Some(path) = name_to_path.get(name.as_str()) {
                    nested_roots_by_index[index].push(path.clone());
                }
            }
            nested_roots_by_index[index].sort();
        }

        for (index, package) in packages.iter_mut().enumerate() {
            let scan = detect_package_languages(
                &package.relative,
                &package.path,
                &nested_roots_by_index[index],
                None,
            );
            package.primary_language = scan.language_breakdown.primary;
            package.secondary_languages = scan.language_breakdown.secondary;
            package.languages = scan.language_breakdown.languages;
            package.frameworks = scan.language_breakdown.frameworks;
            package.file_associations = scan.file_breakdown.by_association;
            package.configuration = scan.compatibility.configuration;
            package.documentation = scan.compatibility.documentation;
            package.editor_config = scan.compatibility.editor_config;
            package.command_runner = scan.compatibility.command_runner;
            package.nested_packages = nested_packages[index].clone();
        }
    }
}

/// Re-export wrapper for use by sibling submodules.
pub(crate) fn discover_seeds_with_optional_index(
    root: &Path,
    standard: MonorepoStandard,
    provenance: PackageProvenance,
    index: Option<&ManifestIndex>,
) -> Vec<PackageSeed> {
    mi_discover_seeds_with_optional_index(root, standard, provenance, index)
}

/// `Cargo.lock` versions, parsed at most once per workspace root per detection.
///
/// ## Notes
///
/// Which lockfile resolves a member's dependency versions is a property of the
/// *owning* workspace, not the repository: a Cargo workspace nested under a pnpm
/// root resolves against its own `Cargo.lock`. Keying by `owner_root` preserves
/// that, and collapses what was previously one parse per detector plus one more
/// for the full-mode manifest scan.
#[derive(Default)]
struct LockStore {
    by_root: HashMap<PathBuf, Option<CargoLockVersions>>,
}

impl LockStore {
    /// The lockfile versions to enrich `seed` with.
    ///
    /// Only Cargo-owned and manifest-scanned boundaries consult a lockfile;
    /// every other standard enriches with `None`, which is the contract each
    /// detector expressed by passing a `None` lockfile of its own.
    fn for_seed(&mut self, seed: &PackageSeed) -> &Option<CargoLockVersions> {
        if !matches!(
            seed.standard,
            MonorepoStandard::CargoWorkspace | MonorepoStandard::Unknown
        ) {
            return &NO_LOCK_VERSIONS;
        }
        self.by_root
            .entry(seed.owner_root.clone())
            .or_insert_with(|| CargoLockVersions::parse(&seed.owner_root.join("Cargo.lock")))
    }
}

/// The absent-lockfile constant [`LockStore::for_seed`] hands to standards that
/// never resolved versions from one.
static NO_LOCK_VERSIONS: Option<CargoLockVersions> = None;

/// Enrich one deduplicated boundary into a catalog [`Package`].
///
/// ## Notes
///
/// Enrichment runs in the seed's `owner_root` frame so workspace-inherited
/// versions resolve against the manifest that actually declares them; the
/// catalog-facing `relative` / `package_area` then come from the seed, which
/// nested and polyglot discovery already re-framed against the repo root. This
/// is the same two-step the pre-seed code performed as "create against the
/// detector's root, then `rebase_package_to_root`".
fn create_package_from_seed(
    seed: &PackageSeed,
    lock_versions: &Option<CargoLockVersions>,
    request: &RepoRequest,
) -> Package {
    let mut package = create_package_with_request(
        &seed.path,
        &seed.owner_root,
        seed.standard,
        seed.provenance,
        lock_versions,
        request,
    );
    package.relative = seed.relative.clone();
    package.package_area = make_package_area(&seed.relative);
    package.is_excluded = seed.is_excluded;
    package
}

/// Creates a Package with all metadata and parsed dependencies.
///
/// ## Notes
///
/// Every workspace detector builds its packages through this function, so it is
/// the one place that sees each package boundary exactly once — which is why
/// the enrichment counter lives here rather than in the detectors.
/// [`refresh_package_boundaries`] deliberately does not count: it re-derives
/// languages for packages this function already produced, so counting there too
/// would report every boundary twice.
pub(crate) fn create_package(
    path: &Path,
    root: &Path,
    standard: MonorepoStandard,
    provenance: PackageProvenance,
    lock_versions: &Option<CargoLockVersions>,
) -> Package {
    create_package_with_request(
        path,
        root,
        standard,
        provenance,
        lock_versions,
        &RepoRequest::full(),
    )
}

/// Creates a package at the detail level selected by `request`.
fn create_package_with_request(
    path: &Path,
    root: &Path,
    standard: MonorepoStandard,
    provenance: PackageProvenance,
    lock_versions: &Option<CargoLockVersions>,
    request: &RepoRequest,
) -> Package {
    if request.wants_package_enrichment() {
        performance::increment_counter(counters::REPO_PACKAGE_ENRICHMENTS, 1);
    }
    let mut ctx = PackageBuildContext::new(lock_versions);
    let relative = make_relative_path(path, root);
    let package_area = make_package_area(&relative);
    let ecosystem = detect_package_ecosystem(path);
    let detected_package_managers = if request.wants_package_managers()
        || request.wants_dependencies()
    {
        detect_package_managers(path)
    } else {
        Vec::new()
    };
    let package_managers = if request.wants_package_managers() {
        detected_package_managers.clone()
    } else {
        Vec::new()
    };
    let test_runners = if request.wants_test_runners() {
        crate::filesystem::repo::test_runner_usage::detect_test_runners(
            path,
            root,
            &mut ctx.manifests,
        )
    } else {
        Vec::new()
    };
    let name = resolve_package_name(&mut ctx, path, root);
    let version = resolve_package_version(&mut ctx, path, root);

    let cargo_toml = path.join("Cargo.toml");
    let features = if !request.structure_only && probe_exists(&cargo_toml) {
        ctx.manifests
            .cargo(&cargo_toml)
            .map(cargo_features_from_value)
            .unwrap_or_default()
    } else {
        Vec::new()
    };

    let mut dependencies = Vec::new();
    let mut dev_dependencies = Vec::new();
    let mut peer_dependencies = Vec::new();
    let mut optional_dependencies = Vec::new();

    if request.wants_dependencies()
        && probe_exists(&cargo_toml)
        && let Some(parsed) = ctx.manifests.cargo(&cargo_toml)
    {
        let (normal, dev, build) = cargo_dependencies_from_value(parsed, ctx.lock_versions);
        let mut all_deps = normal;
        all_deps.extend(build);

        let (optional, regular): (Vec<_>, Vec<_>) = all_deps.into_iter().partition(|d| d.optional);

        dependencies.extend(regular);
        dev_dependencies.extend(dev);
        optional_dependencies.extend(optional);
    }

    // Parse package.json dependency sections when available.
    let package_json = path.join("package.json");
    if request.wants_dependencies() && probe_exists(&package_json) {
        let js_package_manager =
            resolve_js_package_manager(standard, root, &detected_package_managers);
        if let Some(parsed) = ctx.manifests.npm(&package_json) {
            let (normal, dev, peer, optional) =
                package_json_dependencies_from_value(parsed, js_package_manager);
            dependencies.extend(normal);
            dev_dependencies.extend(dev);
            peer_dependencies.extend(peer);
            optional_dependencies.extend(optional);
        }
    }

    // Parse Python dependencies from pyproject.toml / requirements.txt.
    let pyproject_toml = path.join("pyproject.toml");
    if request.wants_dependencies()
        && probe_exists(&pyproject_toml)
        && let Some((normal, optional)) = ctx
            .manifests
            .pyproject(&pyproject_toml)
            .and_then(pyproject_dependencies_from_value)
    {
        dependencies.extend(normal);
        optional_dependencies.extend(optional);
    }
    let requirements_txt = path.join("requirements.txt");
    if request.wants_dependencies()
        && probe_exists(&requirements_txt)
        && let Some(req_deps) = parse_requirements_txt_dependencies(&requirements_txt)
    {
        dependencies.extend(req_deps);
    }

    // Parse Go module dependencies when available.
    let go_mod = path.join("go.mod");
    if request.wants_dependencies()
        && probe_exists(&go_mod)
        && let Some(go_deps) = ctx
            .manifests
            .go_mod(&go_mod)
            .and_then(go_mod_dependencies_from_content)
    {
        dependencies.extend(go_deps);
    };

    let dependencies = if dependencies.is_empty() {
        None
    } else {
        Some(dependencies)
    };
    let dev_dependencies = if dev_dependencies.is_empty() {
        None
    } else {
        Some(dev_dependencies)
    };
    let peer_dependencies = if peer_dependencies.is_empty() {
        None
    } else {
        Some(peer_dependencies)
    };
    let optional_dependencies = if optional_dependencies.is_empty() {
        None
    } else {
        Some(optional_dependencies)
    };

    Package {
        path: path.to_path_buf(),
        relative,
        package_area,
        name,
        ecosystem,
        standard,
        provenance,
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
        package_managers,
        test_runners,
        version,
        features,
        depends_on: Vec::new(),
        used_by: Vec::new(),
        dependencies,
        dev_dependencies,
        peer_dependencies,
        optional_dependencies,
        is_updatable: None,
        has_major_update: None,
        is_excluded: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filesystem::repo::cargo::cargo_package_version;
    use crate::package::{DependencyEntry, DependencyKind};

    fn make_test_package(name: &str, deps: Vec<DependencyEntry>) -> Package {
        Package {
            name: name.to_string(),
            path: PathBuf::from(name),
            relative: name.to_string(),
            package_area: "root".to_string(),
            ecosystem: PackageEcosystem::Unknown,
            standard: MonorepoStandard::Unknown,
            provenance: PackageProvenance::ManifestScan,
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
            test_runners: Vec::new(),
            version: None,
            features: Vec::new(),
            depends_on: Vec::new(),
            used_by: Vec::new(),
            dependencies: if deps.is_empty() { None } else { Some(deps) },
            dev_dependencies: None,
            peer_dependencies: None,
            optional_dependencies: None,
            is_updatable: None,
            has_major_update: None,
            is_excluded: false,
        }
    }

    fn dep(name: &str) -> DependencyEntry {
        DependencyEntry {
            name: name.to_string(),
            targeted_version: "^1.0".to_string(),
            kind: DependencyKind::Normal,
            optional: false,
            actual_version: None,
            package_manager: None,
            latest_version: None,
            target: None,
            features: Vec::new(),
            is_updatable: false,
            has_major_update: false,
        }
    }

    #[test]
    fn make_package_area_returns_root_for_top_level_package() {
        assert_eq!(make_package_area("model_id"), "root");
    }

    #[test]
    fn make_package_area_uses_top_level_parent_for_lib_cli_split() {
        assert_eq!(make_package_area("sniff/lib"), "sniff");
    }

    #[test]
    fn make_package_area_preserves_nested_area_parent() {
        assert_eq!(make_package_area("apps/browser/my_package"), "apps/browser");
    }

    #[test]
    fn resolve_internal_deps_populates_depends_on() {
        let mut packages = vec![
            make_test_package("pkg-a", vec![dep("pkg-b"), dep("pkg-c")]),
            make_test_package("pkg-b", vec![dep("pkg-c")]),
            make_test_package("pkg-c", vec![]),
        ];

        resolve_internal_deps(&mut packages);

        assert_eq!(packages[0].depends_on, vec!["pkg-b", "pkg-c"]);
        assert_eq!(packages[1].depends_on, vec!["pkg-c"]);
        assert!(packages[2].depends_on.is_empty());
    }

    #[test]
    fn resolve_internal_deps_populates_used_by() {
        let mut packages = vec![
            make_test_package("pkg-a", vec![dep("pkg-b"), dep("pkg-c")]),
            make_test_package("pkg-b", vec![dep("pkg-c")]),
            make_test_package("pkg-c", vec![]),
        ];

        resolve_internal_deps(&mut packages);

        assert_eq!(packages[0].used_by, Vec::<String>::new());
        assert_eq!(packages[1].used_by, vec!["pkg-a"]);
        assert_eq!(packages[2].used_by, vec!["pkg-a", "pkg-b"]);
    }

    #[test]
    fn resolve_internal_deps_skips_external_deps() {
        let mut packages = vec![
            make_test_package("pkg-a", vec![dep("pkg-b"), dep("external-lib")]),
            make_test_package("pkg-b", vec![]),
        ];

        resolve_internal_deps(&mut packages);

        assert_eq!(packages[0].depends_on, vec!["pkg-b"]);
        assert!(!packages[0].depends_on.contains(&"external-lib".to_string()));
    }

    #[test]
    fn resolve_internal_deps_skips_self_references() {
        let mut packages = vec![
            make_test_package("pkg-a", vec![dep("pkg-a"), dep("pkg-b")]),
            make_test_package("pkg-b", vec![]),
        ];

        resolve_internal_deps(&mut packages);

        assert_eq!(packages[0].depends_on, vec!["pkg-b"]);
    }

    #[test]
    fn resolve_internal_deps_deduplicates() {
        let mut packages = vec![
            make_test_package("pkg-a", vec![dep("pkg-b"), dep("pkg-b")]),
            make_test_package("pkg-b", vec![]),
        ];

        resolve_internal_deps(&mut packages);

        assert_eq!(packages[0].depends_on, vec!["pkg-b"]);
    }

    // ============================================================================
    // normalize_path tests
    // ============================================================================

    #[test]
    fn normalize_path_cleans_dot_components() {
        let path = Path::new("/foo/bar/./baz");
        let normalized = normalize_path(path);
        assert_eq!(normalized, PathBuf::from("/foo/bar/baz"));
    }

    #[test]
    fn normalize_path_cleans_dotdot_components() {
        let path = Path::new("/foo/bar/../baz");
        let normalized = normalize_path(path);
        assert_eq!(normalized, PathBuf::from("/foo/baz"));
    }

    #[test]
    fn normalize_path_is_idempotent() {
        let path = Path::new("/foo/./bar/../baz");
        let once = normalize_path(path);
        let twice = normalize_path(&once);
        assert_eq!(once, twice);
    }

    #[test]
    fn normalize_path_handles_multiple_dotdots() {
        let path = Path::new("/a/b/c/../../d");
        let normalized = normalize_path(path);
        assert_eq!(normalized, PathBuf::from("/a/d"));
    }

    // ============================================================================
    // ManifestCache tests
    // ============================================================================

    #[test]
    fn manifest_cache_parses_cargo_toml_once() {
        use std::io::Write;
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        let cargo_toml = dir.path().join("Cargo.toml");
        let mut file = std::fs::File::create(&cargo_toml).unwrap();
        file.write_all(
            b"[package]\nname = \"test-pkg\"\nversion = \"1.2.3\"\nedition = \"2021\"\n\n\
             [dependencies]\nserde = \"1.0\"\n\n\
             [features]\ndefault = [\"std\"]\nstd = []\n",
        )
        .unwrap();
        drop(file);

        let mut cache = ManifestCache::default();

        let parsed = cache.cargo(&cargo_toml);
        assert!(parsed.is_some());
        assert_eq!(
            parsed
                .unwrap()
                .get("package")
                .and_then(|p| p.get("name"))
                .and_then(|n| n.as_str()),
            Some("test-pkg")
        );

        let parsed = cache.cargo(&cargo_toml);
        assert!(parsed.is_some());
        assert_eq!(
            parsed
                .unwrap()
                .get("package")
                .and_then(|p| p.get("name"))
                .and_then(|n| n.as_str()),
            Some("test-pkg")
        );
    }

    #[test]
    fn manifest_cache_parses_package_json_once() {
        use std::io::Write;
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        let package_json = dir.path().join("package.json");
        let mut file = std::fs::File::create(&package_json).unwrap();
        file.write_all(
            br#"{"name":"test-app","version":"2.0.0","dependencies":{"lodash":"^4.0.0"}}"#,
        )
        .unwrap();
        drop(file);

        let mut cache = ManifestCache::default();

        let parsed = cache.npm(&package_json);
        assert!(parsed.is_some());
        assert_eq!(
            parsed.unwrap().get("name").and_then(|n| n.as_str()),
            Some("test-app")
        );

        let parsed = cache.npm(&package_json);
        assert!(parsed.is_some());
        assert_eq!(
            parsed.unwrap().get("name").and_then(|n| n.as_str()),
            Some("test-app")
        );
    }

    #[test]
    fn manifest_cache_parses_pyproject_toml_once() {
        use std::io::Write;
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        let pyproject = dir.path().join("pyproject.toml");
        let mut file = std::fs::File::create(&pyproject).unwrap();
        file.write_all(b"[project]\nname = \"test-py\"\nversion = \"3.0.0\"\n")
            .unwrap();
        drop(file);

        let mut cache = ManifestCache::default();

        let parsed = cache.pyproject(&pyproject);
        assert!(parsed.is_some());
        assert_eq!(
            parsed
                .unwrap()
                .get("project")
                .and_then(|p| p.get("name"))
                .and_then(|n| n.as_str()),
            Some("test-py")
        );

        let parsed = cache.pyproject(&pyproject);
        assert!(parsed.is_some());
        assert_eq!(
            parsed
                .unwrap()
                .get("project")
                .and_then(|p| p.get("name"))
                .and_then(|n| n.as_str()),
            Some("test-py")
        );
    }

    #[test]
    fn manifest_cache_reads_go_mod_once() {
        use std::io::Write;
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        let go_mod = dir.path().join("go.mod");
        let mut file = std::fs::File::create(&go_mod).unwrap();
        file.write_all(b"module example.com/test\n\ngo 1.21\n")
            .unwrap();
        drop(file);

        let mut cache = ManifestCache::default();

        let content = cache.go_mod(&go_mod);
        assert!(content.is_some());
        assert!(content.unwrap().contains("example.com/test"));

        let content = cache.go_mod(&go_mod);
        assert!(content.is_some());
        assert!(content.unwrap().contains("example.com/test"));
    }

    #[test]
    fn package_build_context_derives_all_fields_from_cached_manifests() {
        use std::io::Write;
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        let cargo_toml = dir.path().join("Cargo.toml");
        let mut file = std::fs::File::create(&cargo_toml).unwrap();
        file.write_all(
            b"[package]\nname = \"ctx-pkg\"\nversion = \"0.5.0\"\nedition = \"2021\"\n\n\
             [dependencies]\nserde = \"1.0\"\n\n\
             [features]\ndefault = [\"std\"]\nstd = []\n",
        )
        .unwrap();
        drop(file);

        let lock_versions: Option<CargoLockVersions> = None;
        let mut ctx = PackageBuildContext::new(&lock_versions);

        let name = ctx
            .manifests
            .cargo(&cargo_toml)
            .and_then(cargo_package_name);
        assert_eq!(name, Some("ctx-pkg".to_string()));

        let version = ctx
            .manifests
            .cargo(&cargo_toml)
            .and_then(cargo_package_version);
        assert_eq!(version, Some("0.5.0".to_string()));

        let features = ctx
            .manifests
            .cargo(&cargo_toml)
            .map(cargo_features_from_value)
            .unwrap_or_default();
        assert!(features.contains(&"default".to_string()));
        assert!(features.contains(&"std".to_string()));

        if let Some(parsed) = ctx.manifests.cargo(&cargo_toml) {
            let (normal, _dev, _build) = cargo_dependencies_from_value(parsed, ctx.lock_versions);
            assert_eq!(normal.len(), 1);
            assert_eq!(normal[0].name, "serde");
        }
    }

    #[test]
    fn create_package_resolves_cargo_workspace_inherited_version() {
        use tempfile::tempdir;

        // A member crate that inherits its version from the workspace root must
        // land in the package catalog with a resolved `version`, not `None` —
        // the catalog feeds `repo structure --json` and every consumer of
        // `Package.version`, not just `repo version`.
        let dir = tempdir().unwrap();
        let root = dir.path();
        std::fs::write(
            root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"member\"]\n\n[workspace.package]\nversion = \"9.9.9\"\n",
        )
        .unwrap();
        let member = root.join("member");
        std::fs::create_dir_all(&member).unwrap();
        std::fs::write(
            member.join("Cargo.toml"),
            "[package]\nname = \"member\"\nversion.workspace = true\n",
        )
        .unwrap();

        let package = create_package(
            &member,
            root,
            MonorepoStandard::CargoWorkspace,
            PackageProvenance::Globbed,
            &None,
        );
        assert_eq!(
            package.version,
            Some("9.9.9".to_string()),
            "inherited workspace version must populate the catalog, got {:?}",
            package.version
        );
    }

    // ============================================================================
    // refresh_package_boundaries tests
    // ============================================================================

    #[test]
    fn refresh_boundaries_assigns_files_to_deepest_package() {
        use std::fs;
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        let root = dir.path();

        fs::create_dir_all(root.join("packages/foo/src")).unwrap();
        fs::create_dir_all(root.join("packages/bar/src")).unwrap();
        fs::write(root.join("packages/foo/src/main.rs"), "fn main() {}").unwrap();
        fs::write(
            root.join("packages/foo/Cargo.toml"),
            "[package]\nname = 'foo'\n",
        )
        .unwrap();
        fs::write(root.join("packages/bar/src/lib.rs"), "pub fn lib() {}").unwrap();
        fs::write(
            root.join("packages/bar/Cargo.toml"),
            "[package]\nname = 'bar'\n",
        )
        .unwrap();
        fs::write(root.join("README.md"), "# Repo").unwrap();

        let inventory = crate::filesystem::file_types::scan_file_inventory(root).unwrap();
        assert_eq!(inventory.total_files_scanned, 5);

        let mut packages = vec![
            Package {
                name: "foo".to_string(),
                path: root.join("packages/foo"),
                relative: "packages/foo".to_string(),
                package_area: "packages".to_string(),
                ..Default::default()
            },
            Package {
                name: "bar".to_string(),
                path: root.join("packages/bar"),
                relative: "packages/bar".to_string(),
                package_area: "packages".to_string(),
                ..Default::default()
            },
        ];

        refresh_package_boundaries(&mut packages, Some(&inventory));

        assert_eq!(packages[0].languages.len(), 1);
        assert_eq!(packages[0].languages[0].language, ProgrammingLanguage::Rust);
        assert_eq!(packages[0].languages[0].direct_file_count, 1);

        assert_eq!(packages[1].languages.len(), 1);
        assert_eq!(packages[1].languages[0].language, ProgrammingLanguage::Rust);
        assert_eq!(packages[1].languages[0].direct_file_count, 1);
    }

    #[test]
    fn refresh_boundaries_excludes_nested_package_files_from_parent() {
        use std::fs;
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        let root = dir.path();

        fs::create_dir_all(root.join("apps/browser/src")).unwrap();
        fs::create_dir_all(root.join("apps/browser/embedded/src")).unwrap();
        fs::write(
            root.join("apps/browser/package.json"),
            r#"{"name":"browser"}"#,
        )
        .unwrap();
        fs::write(
            root.join("apps/browser/embedded/Cargo.toml"),
            "[package]\nname = 'embedded'\n",
        )
        .unwrap();
        fs::write(
            root.join("apps/browser/embedded/src/main.rs"),
            "fn main() {}",
        )
        .unwrap();

        let inventory = crate::filesystem::file_types::scan_file_inventory(root).unwrap();

        let mut packages = vec![
            Package {
                name: "browser".to_string(),
                path: root.join("apps/browser"),
                relative: "apps/browser".to_string(),
                package_area: "apps".to_string(),
                ..Default::default()
            },
            Package {
                name: "embedded".to_string(),
                path: root.join("apps/browser/embedded"),
                relative: "apps/browser/embedded".to_string(),
                package_area: "apps/browser".to_string(),
                ..Default::default()
            },
        ];

        refresh_package_boundaries(&mut packages, Some(&inventory));

        assert_eq!(packages[0].nested_packages, vec!["embedded"]);
        assert!(packages[0].languages.is_empty());

        assert_eq!(packages[1].languages.len(), 1);
        assert_eq!(packages[1].languages[0].language, ProgrammingLanguage::Rust);
    }

    #[test]
    fn refresh_boundaries_root_package_does_not_steal_all_files() {
        use std::fs;
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        let root = dir.path();

        fs::create_dir_all(root.join("src")).unwrap();
        fs::create_dir_all(root.join("sub/src")).unwrap();
        fs::write(root.join("Cargo.toml"), "[package]\nname = 'root'\n").unwrap();
        fs::write(root.join("src/main.rs"), "fn main() {}").unwrap();
        fs::write(root.join("sub/Cargo.toml"), "[package]\nname = 'sub'\n").unwrap();
        fs::write(root.join("sub/src/lib.rs"), "pub fn lib() {}").unwrap();

        let inventory = crate::filesystem::file_types::scan_file_inventory(root).unwrap();

        let mut packages = vec![
            Package {
                name: "root".to_string(),
                path: root.to_path_buf(),
                relative: "".to_string(),
                package_area: "root".to_string(),
                ..Default::default()
            },
            Package {
                name: "sub".to_string(),
                path: root.join("sub"),
                relative: "sub".to_string(),
                package_area: "root".to_string(),
                ..Default::default()
            },
        ];

        refresh_package_boundaries(&mut packages, Some(&inventory));

        assert_eq!(packages[0].languages.len(), 1);
        assert_eq!(packages[0].languages[0].direct_file_count, 1);

        assert_eq!(packages[1].languages.len(), 1);
        assert_eq!(packages[1].languages[0].direct_file_count, 1);

        assert_eq!(packages[0].nested_packages, vec!["sub"]);
    }

    #[test]
    fn refresh_boundaries_file_associations_and_compatibility() {
        use std::fs;
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        let root = dir.path();

        fs::create_dir_all(root.join("pkg/src")).unwrap();
        fs::write(root.join("pkg/src/main.rs"), "fn main() {}").unwrap();
        fs::write(root.join("pkg/README.md"), "# pkg").unwrap();
        fs::write(root.join("pkg/.editorconfig"), "root = true").unwrap();
        fs::write(root.join("pkg/justfile"), "build:").unwrap();

        let inventory = crate::filesystem::file_types::scan_file_inventory(root).unwrap();

        let mut packages = vec![Package {
            name: "pkg".to_string(),
            path: root.join("pkg"),
            relative: "pkg".to_string(),
            package_area: "root".to_string(),
            ..Default::default()
        }];

        refresh_package_boundaries(&mut packages, Some(&inventory));

        assert!(!packages[0].documentation.is_empty());
        assert!(packages[0].editor_config.is_some());
        assert!(!packages[0].command_runner.is_empty());

        let doc_assoc = packages[0]
            .file_associations
            .iter()
            .find(|a| a.association == FileAssociation::Documentation);
        assert!(doc_assoc.is_some());
    }
}

/// R4 — the request-scoped observation index.
///
/// These tests are counter-based on purpose: the phase's claim is that full
/// detection stopped enumerating the same tree three times, and only a work
/// count can distinguish "walked once" from "walked three times and got the
/// same answer".
#[cfg(test)]
mod observation_index {
    use super::*;
    use crate::filesystem::file_types::MAX_FILES;
    use crate::performance::testing;
    use std::fs;
    use tempfile::TempDir;

    /// A Cargo workspace root with globbed members and a nested pnpm workspace
    /// several directories down.
    ///
    /// The nested workspace is what makes the marker-evidence assertions
    /// meaningful: it is only discoverable by enumerating below the root, which
    /// is exactly the walk this phase folds into the shared one.
    fn workspace_fixture() -> TempDir {
        let dir = TempDir::new().expect("tempdir");
        let root = dir.path();

        fs::write(
            root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/*\"]\nresolver = \"2\"\n",
        )
        .expect("write root manifest");

        for name in ["alpha", "beta"] {
            let pkg = root.join("crates").join(name);
            fs::create_dir_all(pkg.join("src")).expect("create member");
            fs::write(
                pkg.join("Cargo.toml"),
                format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\n"),
            )
            .expect("write member manifest");
            fs::write(pkg.join("src").join("lib.rs"), "pub fn f() {}\n").expect("write source");
        }

        // A solution file exercises the suffix-matched marker path, which stays
        // byte-exact on every platform where the fixed marker names do not.
        let desktop = root.join("desktop");
        fs::create_dir_all(desktop.join("App")).expect("create solution dir");
        fs::write(
            desktop.join("App.sln"),
            "Microsoft Visual Studio Solution File, Format Version 12.00\n\
             Project(\"{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}\") = \"App\", \
             \"App\\App.csproj\", \"{11111111-1111-1111-1111-111111111111}\"\n\
             EndProject\n",
        )
        .expect("write solution");
        fs::write(
            desktop.join("App").join("App.csproj"),
            "<Project Sdk=\"Microsoft.NET.Sdk\"></Project>\n",
        )
        .expect("write project");
        fs::write(root.join("README.md"), "# fixture\n").expect("write doc");

        let web = root.join("web");
        fs::create_dir_all(web.join("packages").join("site")).expect("create nested");
        fs::write(
            web.join("pnpm-workspace.yaml"),
            "packages:\n  - 'packages/*'\n",
        )
        .expect("write nested marker");
        fs::write(
            web.join("packages").join("site").join("package.json"),
            "{\"name\": \"site\", \"version\": \"1.0.0\"}\n",
        )
        .expect("write nested member");

        dir
    }

    fn package_names(repo: &RepoInfo) -> Vec<String> {
        let mut names: Vec<String> = repo
            .packages
            .as_ref()
            .expect("packages")
            .iter()
            .map(|package| package.name.clone())
            .collect();
        names.sort();
        names
    }

    /// R4's headline: one repository-wide non-Git enumeration for a full
    /// standalone detection.
    ///
    /// Before this phase the same tree was enumerated three times — the
    /// manifest index, the nested-marker walk, and the file inventory — plus a
    /// bounded subtree walk per membership glob.
    #[test]
    fn standalone_full_detection_enumerates_the_tree_once() {
        let dir = workspace_fixture();
        let (repo, counts) = testing::measure(|| detect_repo_inner(dir.path(), false));
        let (repo, _inventory) = repo.expect("detection should succeed");
        let repo = repo.expect("the fixture is a workspace");

        assert_eq!(
            counts.get(counters::FS_WALK_STARTS),
            1,
            "full detection must observe the tree exactly once; counters were {:?}",
            counts.all()
        );
        assert_eq!(
            counts.get(counters::REPO_NESTED_MARKER_WALKS),
            0,
            "nested markers must come from the observation index, not a second walk"
        );
        assert_eq!(
            counts.get(counters::REPO_MEMBERSHIP_GLOB_WALKS),
            0,
            "membership globs must query observed manifest dirs, not walk prefix trees"
        );
        assert!(
            package_names(&repo).contains(&"alpha".to_string()),
            "the globbed members must still be found: {:?}",
            package_names(&repo)
        );
    }

    /// R5's headline: every unique boundary is enriched exactly once.
    ///
    /// Full detection used to enrich each package **twice** — once by the
    /// workspace detector and once by the full-mode manifest scan, which
    /// re-discovers each member inside its own subtree — and then dropped the
    /// duplicate in `merge_packages`. The entire second enrichment (manifest
    /// reads, ecosystem probes, the per-package test-runner search) was
    /// discarded work, invisible in the output.
    ///
    /// Asserting enrichments **equals** the package count is what makes that
    /// unrepresentable: seeds are merged before enrichment, so a re-introduced
    /// duplicate discovery cannot hide behind a post-hoc merge.
    #[test]
    fn every_unique_boundary_is_enriched_exactly_once() {
        let dir = workspace_fixture();
        let (repo, counts) = testing::measure(|| detect_repo_inner(dir.path(), false));
        let (repo, _inventory) = repo.expect("detection should succeed");
        let repo = repo.expect("the fixture is a workspace");
        let package_count = repo.packages.as_ref().expect("packages").len() as u64;

        assert!(package_count > 0, "fixture must discover packages");
        assert_eq!(
            counts.get(counters::REPO_PACKAGE_ENRICHMENTS),
            package_count,
            "each of the {package_count} boundaries must be enriched once, not once \
             per detector that resolved it; counters were {:?}",
            counts.all()
        );
    }

    /// Structure-only detection enriches each boundary once too, and full mode
    /// must not enrich more boundaries than structure mode discovers.
    ///
    /// The two modes disagreeing here would mean full mode is counting a
    /// boundary structure mode never resolved — the doubling defect's signature.
    #[test]
    fn full_mode_enriches_no_more_boundaries_than_structure_mode() {
        let dir = workspace_fixture();
        let (_, structure) = testing::measure(|| detect_repo_inner(dir.path(), true));
        let (_, full) = testing::measure(|| detect_repo_inner(dir.path(), false));

        assert_eq!(
            full.get(counters::REPO_PACKAGE_ENRICHMENTS),
            structure.get(counters::REPO_PACKAGE_ENRICHMENTS),
            "full mode discovers the same boundaries as structure mode, so it must \
             enrich the same number; structure {:?} vs full {:?}",
            structure.all(),
            full.all()
        );
    }

    /// R4.2 — integrated and standalone full detection must agree.
    ///
    /// They now share one evidence model, so a divergence here means one of the
    /// two routes is observing something the other is not.
    #[test]
    fn integrated_and_standalone_full_detection_agree() {
        use crate::filesystem::detect_filesystem_with_request;
        use crate::request::{FilesystemRequest, RepoRequest};

        let dir = workspace_fixture();

        let (standalone, standalone_inventory) =
            detect_repo_inner(dir.path(), false).expect("standalone detection");
        let standalone = standalone.expect("the fixture is a workspace");

        let integrated = detect_filesystem_with_request(
            dir.path(),
            &FilesystemRequest::new()
                .without_git()
                .repo(RepoRequest::full())
                .without_docs()
                .without_formatting(),
        )
        .expect("integrated detection");

        assert_eq!(
            serde_json::to_value(&standalone).expect("serialize standalone"),
            serde_json::to_value(integrated.repo.as_ref().expect("repo was requested"))
                .expect("serialize integrated"),
            "integrated and standalone full detection must return equivalent topology, \
             nested workspaces, solutions, and leaf packages"
        );

        // Both a solution and a nested workspace resolve here, so the equality
        // above is over a topology with something to disagree about.
        let authorities: Vec<_> = standalone
            .monorepo_layers
            .iter()
            .map(|layer| layer.authority)
            .collect();
        assert!(
            authorities.contains(&MonorepoStandard::DotNetSolution)
                && authorities.contains(&MonorepoStandard::PnpmWorkspaces),
            "the fixture must exercise a solution and a nested workspace; authorities were \
             {authorities:?}"
        );

        let (standalone_files, _) = crate::filesystem::file_types::summarize_file_inventory(
            &standalone_inventory.expect("full detection returns its inventory"),
        );
        assert_eq!(
            serde_json::to_value(&standalone_files).expect("serialize standalone inventory"),
            serde_json::to_value(integrated.files.as_ref().expect("inventory was requested"))
                .expect("serialize integrated inventory"),
            "both routes must project the same inventory from their one observation"
        );
    }

    /// The nested pnpm workspace is real evidence, not an artifact of the
    /// fallback walk — it must survive being derived from the index.
    #[test]
    fn nested_workspaces_are_discovered_from_observed_markers() {
        let dir = workspace_fixture();
        let (repo, counts) = testing::measure(|| detect_repo_inner(dir.path(), false));
        let (repo, _) = repo.expect("detection should succeed");
        let repo = repo.expect("the fixture is a workspace");

        assert_eq!(counts.get(counters::REPO_NESTED_MARKER_WALKS), 0);
        assert!(
            repo.monorepo_layers
                .iter()
                .any(|layer| layer.authority == MonorepoStandard::PnpmWorkspaces),
            "the nested pnpm workspace must be discovered from observed marker evidence; \
             layers were {:?}",
            repo.monorepo_layers
                .iter()
                .map(|layer| layer.authority)
                .collect::<Vec<_>>()
        );
    }

    /// R4.5 — structure-only keeps the smallest evidence set Phase 2 gave it.
    ///
    /// It consumes no manifest index, inventory, or docs, so building the
    /// observation index for it would classify every file in the tree for
    /// evidence it discards. Its fallback walk is explicit rather than silent.
    #[test]
    fn structure_only_detection_uses_the_explicit_marker_fallback() {
        let dir = workspace_fixture();
        let (repo, counts) = testing::measure(|| detect_repo_inner(dir.path(), true));
        let (repo, inventory) = repo.expect("detection should succeed");
        repo.expect("the fixture is a workspace");

        assert!(inventory.is_none(), "structure mode collects no inventory");
        assert_eq!(
            counts.get(counters::FS_WALK_STARTS),
            0,
            "structure mode must not build the observation index; counters were {:?}",
            counts.all()
        );
        assert_eq!(
            counts.get(counters::REPO_NESTED_MARKER_WALKS),
            1,
            "structure mode's marker walk must be visible, not silent"
        );
        assert_eq!(
            counts.get(counters::FS_INVENTORY_ACCEPTED),
            0,
            "structure mode must classify nothing"
        );
    }

    /// The `has_workspace_marker` gate: a full `detect_repo` over a directory
    /// that cannot be a workspace root must not classify its contents.
    ///
    /// This is what keeps `detect_repo` on a large system temp dir cheap, and
    /// routing standalone detection through the observation builder must not
    /// regress it.
    #[test]
    fn markerless_root_is_not_observed() {
        let dir = TempDir::new().expect("tempdir");
        fs::write(dir.path().join("notes.txt"), "not a manifest\n").expect("write");

        let (result, counts) = testing::measure(|| detect_repo_inner(dir.path(), false));
        let (repo, _) = result.expect("detection should succeed");
        assert!(repo.is_none(), "a markerless directory is not a repository");

        assert_eq!(
            counts.get(counters::FS_WALK_STARTS),
            0,
            "a directory with no workspace marker must not be observed; counters were {:?}",
            counts.all()
        );
        assert_eq!(
            counts.get(counters::FS_INVENTORY_ACCEPTED),
            0,
            "nothing may be classified for a directory that cannot be a workspace root"
        );
    }

    /// `ManifestIndex::build`'s parallel workers must report their reads.
    ///
    /// Its walker carried no `WorkerCollector`, so every `is_generated_manifest`
    /// read it performed landed in a thread-local buffer nothing ever drained.
    /// Phase 1 closed this class of defect for the shared walk and the Rayon
    /// pools but did not reach this walker, so the archived Phase 1 baselines
    /// under-report `file_opens`/`bytes_read` for every case that built the
    /// index. Without this, `observing_once_changes_work_not_results` would
    /// read a phantom +3 opens for the arm that is doing strictly less work.
    #[test]
    fn manifest_index_build_reports_its_reads() {
        let dir = workspace_fixture();
        let ((), counts) = testing::measure(|| {
            ManifestIndex::build(dir.path());
        });

        assert!(
            counts.get(counters::FS_FILE_OPENS) > 0,
            "the index reads every Cargo.toml to screen generated manifests; those              reads must appear in the report. counters were {:?}",
            counts.all()
        );
        assert!(counts.get(counters::FS_BYTES_READ) > 0);
    }

    /// The observation index must change work, not results.
    ///
    /// Both arms run in the same process on the same tree, so this is a
    /// drift-free comparison rather than a cross-run one. The pre-index path is
    /// still reachable by construction: `detect_repo_inner_with_shared` with no
    /// evidence is exactly what standalone full detection used to do.
    #[test]
    fn observing_once_changes_work_not_results() {
        let dir = workspace_fixture();

        let (unshared, unshared_counts) = testing::measure(|| {
            detect_repo_inner_with_shared(dir.path(), false, RepoEvidence::default())
        });
        let (shared, shared_counts) = testing::measure(|| detect_repo_inner(dir.path(), false));

        let (unshared_repo, _) = unshared.expect("unshared detection");
        let (shared_repo, _) = shared.expect("shared detection");
        assert_eq!(
            serde_json::to_value(unshared_repo.expect("workspace")).expect("serialize"),
            serde_json::to_value(shared_repo.expect("workspace")).expect("serialize"),
            "routing through the observation index must not change results"
        );

        // The saving: three whole-tree enumerations become one. `read_dirs`
        // falls by the manifest index's walk, the nested-marker walk, and the
        // membership-glob subtree walks; `metadata_probes` falls by the
        // `dir_has_manifest` probe storm those glob walks performed.
        assert_eq!(shared_counts.get(counters::REPO_NESTED_MARKER_WALKS), 0);
        assert_eq!(shared_counts.get(counters::REPO_MEMBERSHIP_GLOB_WALKS), 0);
        assert!(
            unshared_counts.get(counters::REPO_NESTED_MARKER_WALKS) > 0
                && unshared_counts.get(counters::REPO_MEMBERSHIP_GLOB_WALKS) > 0,
            "the pre-index arm must actually perform the walks this phase removes,              or the comparison proves nothing"
        );
        assert!(
            shared_counts.get(counters::FS_READ_DIRS) < unshared_counts.get(counters::FS_READ_DIRS),
            "observing once must enumerate fewer directories: {} -> {}",
            unshared_counts.get(counters::FS_READ_DIRS),
            shared_counts.get(counters::FS_READ_DIRS)
        );
        assert!(
            shared_counts.get(counters::FS_METADATA_PROBES)
                < unshared_counts.get(counters::FS_METADATA_PROBES),
            "dropping the per-candidate manifest probes must lower metadata probes"
        );

        // Reading is unchanged: the same manifests are read the same number of
        // times, just from one walk instead of three.
        assert_eq!(
            shared_counts.get(counters::FS_FILE_OPENS),
            unshared_counts.get(counters::FS_FILE_OPENS),
            "observing once must not change how many files are read"
        );
        assert_eq!(
            shared_counts.get(counters::REPO_MANIFEST_PARSES),
            unshared_counts.get(counters::REPO_MANIFEST_PARSES),
        );
        assert_eq!(
            shared_counts.get(counters::FS_INVENTORY_ACCEPTED),
            unshared_counts.get(counters::FS_INVENTORY_ACCEPTED),
        );
    }

    /// R4.4 — committed-marker ignore semantics survive the index.
    ///
    /// `detect_repo_structure` already covers this for the fallback walk
    /// (`test_gitignored_nested_marker_is_not_detected`). Full detection now
    /// reads markers from the observation index instead, so the same semantics
    /// need their own assertion on this path: the shared walk honors
    /// `git_ignore`, exactly as `walk_for_nested_markers` does.
    #[test]
    fn gitignored_markers_are_absent_from_observed_evidence() {
        let dir = TempDir::new().expect("tempdir");
        let root = dir.path();
        git2::Repository::init(root).expect("init repo");

        fs::write(
            root.join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/*\"]\n",
        )
        .expect("write root manifest");
        let member = root.join("crates").join("alpha");
        fs::create_dir_all(&member).expect("create member");
        fs::write(
            member.join("Cargo.toml"),
            "[package]\nname = \"alpha\"\nversion = \"0.1.0\"\n",
        )
        .expect("write member manifest");

        fs::write(root.join(".gitignore"), "web/\n").expect("write gitignore");
        let web = root.join("web");
        fs::create_dir_all(&web).expect("create ignored dir");
        fs::write(web.join("pnpm-workspace.yaml"), "packages:\n  - '*'\n")
            .expect("write ignored marker");

        let view = crate::filesystem::system_view::build_filesystem_system_view(
            root,
            crate::filesystem::system_view::SharedWalkOptions::full_repo(),
        );
        let markers = view.nested_markers.as_ref().expect("markers requested");
        assert!(
            !markers.iter().any(|path| path.starts_with(&web)),
            "a gitignored marker must not become observed evidence; markers were {markers:?}"
        );
    }

    /// R4.4 — the shared walk prunes the same directory names repo detection
    /// has always pruned, so a vendored tree cannot inject marker evidence.
    #[test]
    fn pruned_directories_yield_no_evidence() {
        let dir = TempDir::new().expect("tempdir");
        let root = dir.path();
        fs::write(root.join("Cargo.toml"), "[workspace]\nmembers = []\n")
            .expect("write root manifest");

        let vendored = root.join("node_modules").join("dep");
        fs::create_dir_all(&vendored).expect("create vendored dir");
        fs::write(vendored.join("package.json"), "{\"name\": \"dep\"}\n")
            .expect("write vendored marker");

        let view = crate::filesystem::system_view::build_filesystem_system_view(
            root,
            crate::filesystem::system_view::SharedWalkOptions::full_repo(),
        );

        // The root `Cargo.toml` is legitimately observed — marker evidence is
        // collected for the whole tree and the root exception is applied later,
        // when candidates are grouped. Only the pruned subtree must be absent.
        let pruned = root.join("node_modules");
        let markers = view.nested_markers.as_ref().expect("markers requested");
        assert!(
            !markers.iter().any(|path| path.starts_with(&pruned)),
            "a marker inside a pruned directory must not be observed; markers were {markers:?}"
        );
        let dirs = view.manifest_dirs.as_ref().expect("manifest dirs requested");
        assert!(
            !dirs.iter().any(|path| path.starts_with(&pruned)),
            "a pruned directory must not contribute manifest evidence; dirs were {dirs:?}"
        );
    }

    /// R4.6 — the index retains evidence, never entries or file bodies, and is
    /// bounded by the same inventory cap as every other classification path.
    #[test]
    fn observed_evidence_is_bounded_by_the_inventory_cap() {
        let dir = workspace_fixture();
        let view = crate::filesystem::system_view::build_filesystem_system_view(
            dir.path(),
            crate::filesystem::system_view::SharedWalkOptions::full_repo(),
        );

        let inventory = view.inventory.as_ref().expect("inventory requested");
        assert!(inventory.classifications.len() <= MAX_FILES);
        assert!(!inventory.truncated, "the fixture is far below the cap");

        let markers = view.nested_markers.as_ref().expect("markers requested");
        assert!(
            markers
                .iter()
                .any(|path| path.ends_with("web/pnpm-workspace.yaml")),
            "the nested marker must be observed; markers were {markers:?}"
        );
        assert!(
            markers.iter().all(|path| path.is_absolute()),
            "marker evidence must stay native absolute paths"
        );

        let dirs = view.manifest_dirs.as_ref().expect("manifest dirs requested");
        assert!(
            dirs.windows(2).all(|w| w[0] < w[1]),
            "manifest dirs must be sorted and deduped for prefix queries"
        );
    }
}
