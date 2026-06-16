use crate::Result;
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
    cargo_package_version, detect_cargo_workspace,
};
use super::dotnet::{detect_dotnet_solution, root_has_solution_file};
use super::go::{
    detect_go_workspace, go_mod_dependencies_from_content, go_module_name_from_content,
};
use super::gradle::detect_gradle_workspace;
use super::manifest_index::{
    CargoLockVersions, ManifestIndex, discover_packages_from_index,
    discover_packages_with_optional_index as mi_discover_packages_with_optional_index,
};
use super::maven::detect_maven_workspace;
use super::nested::discover_nested_workspace_outcomes;
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
use super::topology::{
    DetectorOutcome, build_detected_standards, build_monorepo_layers, layers_imply_monorepo,
    standard_for_tool,
};
use super::types::{
    MonorepoTool, Package, PackageDiscoverySource, PackageEcosystem, PackageFiles,
    PackageScanResult, RepoInfo,
};
use super::uv::detect_uv_workspace;

pub(crate) fn detect_repo_inner(
    root: &Path,
    structure_only: bool,
) -> Result<(Option<RepoInfo>, Option<FileInventory>)> {
    detect_repo_inner_with_shared(root, structure_only, None, None)
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
}

impl ManifestCache {
    pub(crate) fn cargo(&mut self, path: &Path) -> Option<&toml_crate::Value> {
        let entry = self.cargo.entry(path.to_path_buf()).or_insert_with(|| {
            std::fs::read_to_string(path)
                .ok()
                .and_then(|content| toml_crate::from_str(&content).ok())
        });
        entry.as_ref()
    }

    pub(crate) fn npm(&mut self, path: &Path) -> Option<&serde_json::Value> {
        let entry = self.npm.entry(path.to_path_buf()).or_insert_with(|| {
            std::fs::read_to_string(path)
                .ok()
                .and_then(|content| serde_json::from_str(&content).ok())
        });
        entry.as_ref()
    }

    pub(crate) fn pyproject(&mut self, path: &Path) -> Option<&toml_crate::Value> {
        let entry = self.pyproject.entry(path.to_path_buf()).or_insert_with(|| {
            std::fs::read_to_string(path)
                .ok()
                .and_then(|content| toml_crate::from_str(&content).ok())
        });
        entry.as_ref()
    }

    pub(crate) fn go_mod(&mut self, path: &Path) -> Option<&str> {
        let entry = self
            .go_mod
            .entry(path.to_path_buf())
            .or_insert_with(|| std::fs::read_to_string(path).ok());
        entry.as_deref()
    }
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
    if MARKERS.iter().any(|name| root.join(name).exists()) {
        return true;
    }
    // .NET solution files have arbitrary names, so they are matched by extension
    // rather than by a fixed marker name.
    root_has_solution_file(root)
}

pub(crate) fn detect_repo_inner_with_shared(
    root: &Path,
    structure_only: bool,
    shared_manifest_index: Option<&ManifestIndex>,
    shared_repo_inventory: Option<&FileInventory>,
) -> Result<(Option<RepoInfo>, Option<FileInventory>)> {
    // Build manifest index once for the entire tree unless the caller already
    // provided the shared view from a higher-level walk. Skip the (potentially
    // very expensive) tree walk entirely when `root` has no workspace marker
    // file: without one, every *root-level* detector below returns `None`, so
    // the index would never be consumed at this stage. This keeps `detect_repo`
    // on a non-repo directory -- e.g. a large system temp dir -- from walking
    // unrelated subtrees.
    //
    // The skip is eager only: nested workspace discovery below may still
    // produce outcomes from markers deeper in the tree (e.g.
    // `web/pnpm-workspace.yaml` under an otherwise bare root). When that
    // happens and full package enrichment is requested, the index is built
    // lazily after the early-return check so full detection does not panic.
    let manifest_index =
        if structure_only || shared_manifest_index.is_some() || !has_workspace_marker(root) {
            None
        } else {
            Some(ManifestIndex::build(root))
        };
    let manifest_index = shared_manifest_index.or(manifest_index.as_ref());

    let mut workspace_tools = Vec::new();
    let mut packages = Vec::new();
    let mut outcomes: Vec<DetectorOutcome> = Vec::new();

    collect_repo_info(
        detect_cargo_workspace(root)?,
        &mut workspace_tools,
        &mut packages,
        &mut outcomes,
    );
    collect_repo_info(
        detect_nx(root, manifest_index)?,
        &mut workspace_tools,
        &mut packages,
        &mut outcomes,
    );
    collect_repo_info(
        detect_turborepo(root, manifest_index)?,
        &mut workspace_tools,
        &mut packages,
        &mut outcomes,
    );
    collect_repo_info(
        detect_pnpm_workspace(root)?,
        &mut workspace_tools,
        &mut packages,
        &mut outcomes,
    );
    collect_repo_info(
        detect_yarn_workspace(root)?,
        &mut workspace_tools,
        &mut packages,
        &mut outcomes,
    );
    collect_repo_info(
        detect_npm_workspace(root)?,
        &mut workspace_tools,
        &mut packages,
        &mut outcomes,
    );
    collect_repo_info(
        detect_lerna(root, manifest_index)?,
        &mut workspace_tools,
        &mut packages,
        &mut outcomes,
    );

    // New membership authorities (Bun, uv, Go) have no legacy `MonorepoTool`
    // counterpart, so they contribute only to the new topology collections via an
    // explicit standard rather than through `workspace_tools`.
    collect_standard_outcome(
        detect_bun_workspace(root)?,
        MonorepoStandard::BunWorkspaces,
        &mut packages,
        &mut outcomes,
    );
    collect_standard_outcome(
        detect_uv_workspace(root)?,
        MonorepoStandard::UvWorkspace,
        &mut packages,
        &mut outcomes,
    );
    collect_standard_outcome(
        detect_go_workspace(root)?,
        MonorepoStandard::GoWorkspace,
        &mut packages,
        &mut outcomes,
    );
    collect_standard_outcome(
        detect_gradle_workspace(root)?,
        MonorepoStandard::GradleMultiProject,
        &mut packages,
        &mut outcomes,
    );
    collect_standard_outcome(
        detect_maven_workspace(root)?,
        MonorepoStandard::MavenMultiModule,
        &mut packages,
        &mut outcomes,
    );
    collect_standard_outcome(
        detect_dotnet_solution(root)?,
        MonorepoStandard::DotNetSolution,
        &mut packages,
        &mut outcomes,
    );
    collect_standard_outcome(
        detect_rush_workspace(root)?,
        MonorepoStandard::RushStack,
        &mut packages,
        &mut outcomes,
    );

    // Polyglot leaf-marker build systems contribute one outcome per workspace
    // root (Bazel segments nested `WORKSPACE` subtrees into their own layer).
    collect_outcomes(detect_bazel_workspace(root)?, &mut packages, &mut outcomes);
    collect_outcomes(detect_pants_workspace(root)?, &mut packages, &mut outcomes);
    collect_outcomes(detect_buck2_workspace(root)?, &mut packages, &mut outcomes);

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
        manifest_index,
        &forbids_nested_roots,
        &mut workspace_tools,
        &mut packages,
        &mut outcomes,
    )?;

    if workspace_tools.is_empty() && outcomes.is_empty() {
        return Ok((None, None));
    }

    // Full-mode nested package enrichment requires the manifest index. The
    // eager build above is skipped when `root` has no workspace marker, but
    // nested discovery may still have produced outcomes (e.g.
    // `web/pnpm-workspace.yaml` under an otherwise bare root). Build the index
    // lazily here so full `detect_repo` does not panic on a nested-only
    // topology. Structure-only mode never consults the index, so it stays `None`.
    let lazy_manifest_index = if !structure_only && manifest_index.is_none() {
        Some(ManifestIndex::build(root))
    } else {
        None
    };
    let manifest_index = manifest_index.or(lazy_manifest_index.as_ref());

    // Build the topology from membership-declaring detectors. `is_monorepo` is
    // now the honest predicate: at least one layer must resolve non-degenerately.
    // The legacy `monorepo_tool` / `workspace_tools` fields below stay populated
    // exactly as before so existing JSON consumers see no change.
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
        upgrade_provenance_with_lockfile(layer);
    }

    if !structure_only {
        // Full mode: discover nested packages using the manifest index
        let lock_versions = CargoLockVersions::parse(&root.join("Cargo.lock"));
        let workspace_packages = packages.clone();
        let index = manifest_index.expect("full repo detection requires a manifest index");
        for package in &workspace_packages {
            packages.extend(discover_packages_from_index(
                &package.path,
                root,
                MonorepoTool::Unknown,
                &lock_versions,
                PackageDiscoverySource::ManifestScan,
                index,
            ));
        }
    }

    let mut packages = merge_packages(packages);
    let repo_inventory = if !structure_only {
        // Build shared repo-level file inventory once for all packages
        let inventory = shared_repo_inventory
            .cloned()
            .or_else(|| crate::filesystem::file_types::scan_file_inventory(root).ok());
        refresh_package_boundaries(&mut packages, inventory.as_ref());
        inventory
    } else {
        None
    };
    resolve_internal_deps(&mut packages);

    Ok((
        Some(RepoInfo {
            is_monorepo,
            monorepo_tool: workspace_tools.first().copied(),
            workspace_tools,
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

fn collect_repo_info(
    info: Option<RepoInfo>,
    workspace_tools: &mut Vec<MonorepoTool>,
    packages: &mut Vec<Package>,
    outcomes: &mut Vec<DetectorOutcome>,
) {
    let Some(info) = info else {
        return;
    };

    if let Some(tool) = info.monorepo_tool
        && !workspace_tools.contains(&tool)
    {
        workspace_tools.push(tool);
    }

    if !info.workspace_tools.is_empty() {
        for tool in info.workspace_tools {
            if !workspace_tools.contains(&tool) {
                workspace_tools.push(tool);
            }
        }
    }

    let detected_packages = info.packages.unwrap_or_default();
    // Record this detector's membership outcome for the topology builder before
    // the packages are folded into the legacy flat list. Each detector reports a
    // single tool, so one outcome is produced per matched standard.
    if let Some(tool) = info.monorepo_tool {
        outcomes.push(DetectorOutcome {
            standard: standard_for_tool(tool),
            root: info.root.clone(),
            packages: detected_packages.clone(),
        });
    }
    packages.extend(detected_packages);
}

/// Fold a new-standard detector's result into the topology collections.
///
/// Unlike [`collect_repo_info`], this does not touch the legacy
/// `workspace_tools` list: Bun, uv, and Go have no [`MonorepoTool`] counterpart,
/// so they are reported purely through `monorepo_standards` / `monorepo_layers`.
/// The detected packages still join the flat `packages` list so `RepoInfo`
/// continues to surface them.
fn collect_standard_outcome(
    info: Option<RepoInfo>,
    standard: MonorepoStandard,
    packages: &mut Vec<Package>,
    outcomes: &mut Vec<DetectorOutcome>,
) {
    let Some(info) = info else {
        return;
    };

    let detected_packages = info.packages.unwrap_or_default();
    outcomes.push(DetectorOutcome {
        standard,
        root: info.root.clone(),
        packages: detected_packages.clone(),
    });
    packages.extend(detected_packages);
}

/// Fold a multi-root detector's outcomes into the topology collections.
///
/// Leaf-marker detectors (Bazel/Pants/Buck2) may report more than one workspace
/// root, so they hand back a list of [`DetectorOutcome`]s rather than a single
/// [`RepoInfo`]. Each outcome's packages also join the flat `packages` list so
/// `RepoInfo` continues to surface them.
fn collect_outcomes(
    detected: Vec<DetectorOutcome>,
    packages: &mut Vec<Package>,
    outcomes: &mut Vec<DetectorOutcome>,
) {
    for outcome in detected {
        packages.extend(outcome.packages.clone());
        outcomes.push(outcome);
    }
}

/// Upgrade a layer's provenance to `Lockfile` when the committed lockfile
/// corroborates the manifest-derived package set.
///
/// The manifest remains the authority when lockfile and manifest disagree; the
/// mismatch is recorded in `lockfile_match` so consumers can spot stale lockfiles.
fn upgrade_provenance_with_lockfile(layer: &mut MonorepoLayer) {
    let authority = layer.authority;
    let lockfile_result = match authority {
        MonorepoStandard::PnpmWorkspaces => pnpm_lockfile_matches(layer),
        MonorepoStandard::UvWorkspace => uv_lockfile_matches(layer),
        MonorepoStandard::CargoWorkspace => cargo_lockfile_matches(layer),
        _ => return,
    };

    let Some(matches) = lockfile_result else {
        return;
    };

    layer.lockfile_match = Some(matches);
    if matches {
        layer.provenance = PackageProvenance::Lockfile;
        for pkg in &mut layer.packages {
            pkg.provenance = PackageProvenance::Lockfile;
        }
    }
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
fn pnpm_lockfile_matches(layer: &MonorepoLayer) -> Option<bool> {
    let lock_path = layer.root.join("pnpm-lock.yaml");
    let content = std::fs::read_to_string(&lock_path).ok()?;
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
        .map(|p| p.relative.to_string_lossy().into_owned().replace('\\', "/"))
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
fn uv_lockfile_matches(layer: &MonorepoLayer) -> Option<bool> {
    let lock_path = layer.root.join("uv.lock");
    let content = std::fs::read_to_string(&lock_path).ok()?;
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
        .map(|p| {
            let s = p.relative.to_string_lossy().into_owned().replace('\\', "/");
            // uv counts the workspace root (`.`) as a member; represent the
            // empty relative path the same way the lockfile does.
            if s.is_empty() { ".".to_string() } else { s }
        })
        .collect();

    Some(manifest_members == lock_members)
}

/// Check that every globbed Cargo member has a `[package].name` present in the
/// root `Cargo.lock` `[[package]]` table.
fn cargo_lockfile_matches(layer: &MonorepoLayer) -> Option<bool> {
    let lock_path = layer.root.join("Cargo.lock");
    let lock_versions = CargoLockVersions::parse(&lock_path)?;

    for pkg in &layer.packages {
        let cargo_toml = layer.root.join(&pkg.relative).join("Cargo.toml");
        let name = std::fs::read_to_string(&cargo_toml)
            .ok()
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
    if cargo_toml.exists()
        && let Some(name) = ctx
            .manifests
            .cargo(&cargo_toml)
            .and_then(cargo_package_name)
    {
        return name;
    }

    let package_json = path.join("package.json");
    if package_json.exists()
        && let Some(name) = ctx.manifests.npm(&package_json).and_then(npm_package_name)
    {
        return name;
    }

    let pyproject_toml = path.join("pyproject.toml");
    if pyproject_toml.exists()
        && let Some(name) = ctx
            .manifests
            .pyproject(&pyproject_toml)
            .and_then(pyproject_package_name)
    {
        return name;
    }

    let go_mod = path.join("go.mod");
    if go_mod.exists()
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
    if cargo_toml.exists() {
        return ctx
            .manifests
            .cargo(&cargo_toml)
            .and_then(cargo_package_version);
    }

    let package_json = path.join("package.json");
    if package_json.exists() {
        if let Some(version) = ctx
            .manifests
            .npm(&package_json)
            .and_then(npm_package_version)
        {
            return Some(version);
        }
        if path != root {
            let root_package_json = root.join("package.json");
            if root_package_json.exists() {
                return ctx
                    .manifests
                    .npm(&root_package_json)
                    .and_then(npm_package_version);
            }
        }
    }

    let pyproject_toml = path.join("pyproject.toml");
    if pyproject_toml.exists()
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
fn make_relative_path(path: &Path, root: &Path) -> String {
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

    if path.join("Cargo.toml").exists() {
        managers.push("cargo".to_string());
    }

    let has_package_json = path.join("package.json").exists();
    let has_pnpm_lock = path.join("pnpm-lock.yaml").exists();
    let has_yarn_lock = path.join("yarn.lock").exists();

    if has_pnpm_lock {
        managers.push("pnpm".to_string());
    } else if has_yarn_lock {
        managers.push("yarn".to_string());
    } else if has_package_json {
        managers.push("npm".to_string());
    }

    if path.join("requirements.txt").exists() || path.join("pyproject.toml").exists() {
        managers.push("pip".to_string());
    }

    if path.join("go.mod").exists() {
        managers.push("go".to_string());
    }

    managers
}

pub(crate) fn discovery_source_for_tool(tool: MonorepoTool) -> PackageDiscoverySource {
    match tool {
        MonorepoTool::CargoWorkspace => PackageDiscoverySource::CargoWorkspace,
        MonorepoTool::PnpmWorkspaces => PackageDiscoverySource::PnpmWorkspace,
        MonorepoTool::NpmWorkspaces => PackageDiscoverySource::NpmWorkspace,
        MonorepoTool::YarnWorkspaces => PackageDiscoverySource::YarnWorkspace,
        MonorepoTool::Nx => PackageDiscoverySource::Nx,
        MonorepoTool::Turborepo => PackageDiscoverySource::Turborepo,
        MonorepoTool::Lerna => PackageDiscoverySource::Lerna,
        MonorepoTool::Unknown => PackageDiscoverySource::ManifestScan,
    }
}

fn detect_package_ecosystem(path: &Path) -> PackageEcosystem {
    if path.join("Cargo.toml").exists() {
        return PackageEcosystem::Cargo;
    }
    if path.join("package.json").exists() {
        return PackageEcosystem::Node;
    }
    if path.join("pyproject.toml").exists() || path.join("requirements.txt").exists() {
        return PackageEcosystem::Python;
    }
    if path.join("go.mod").exists() {
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

pub(crate) fn dedupe_packages(packages: Vec<Package>) -> Vec<Package> {
    let mut seen = HashSet::new();
    let mut deduped = Vec::new();

    for package in packages {
        if seen.insert(package.relative.clone()) {
            deduped.push(package);
        }
    }

    deduped
}

fn merge_packages(packages: Vec<Package>) -> Vec<Package> {
    let mut merged = Vec::new();
    let mut index_by_path: HashMap<PathBuf, usize> = HashMap::new();

    for package in packages {
        let key = canonicalize_path(&package.path);
        if let Some(index) = index_by_path.get(&key).copied() {
            merge_package_into(&mut merged[index], package);
        } else {
            index_by_path.insert(key, merged.len());
            merged.push(package);
        }
    }

    merged.sort_by(|a, b| a.relative.cmp(&b.relative));
    merged
}

fn merge_package_into(existing: &mut Package, incoming: Package) {
    if existing.name == existing.relative && incoming.name != incoming.relative {
        existing.name = incoming.name.clone();
    }

    if existing.version.is_none() {
        existing.version = incoming.version.clone();
    }

    if existing.ecosystem == PackageEcosystem::Unknown {
        existing.ecosystem = incoming.ecosystem;
    }

    for source in incoming.discovery_sources {
        if !existing.discovery_sources.contains(&source) {
            existing.discovery_sources.push(source);
        }
    }

    for manager in incoming.package_managers {
        if !existing.package_managers.contains(&manager) {
            existing.package_managers.push(manager);
        }
    }

    for feature in incoming.features {
        if !existing.features.contains(&feature) {
            existing.features.push(feature);
        }
    }

    if existing.primary_language.is_none() {
        existing.primary_language = incoming.primary_language;
    }

    for language in incoming.secondary_languages {
        if !existing.secondary_languages.contains(&language) {
            existing.secondary_languages.push(language);
        }
    }

    existing.configuration = merge_path_lists(&existing.configuration, &incoming.configuration);
    existing.documentation = merge_path_lists(&existing.documentation, &incoming.documentation);
    existing.command_runner = merge_path_lists(&existing.command_runner, &incoming.command_runner);

    if existing.editor_config.is_none() {
        existing.editor_config = incoming.editor_config;
    }

    if existing.dependencies.is_none() {
        existing.dependencies = incoming.dependencies;
    }
    if existing.dev_dependencies.is_none() {
        existing.dev_dependencies = incoming.dev_dependencies;
    }
    if existing.peer_dependencies.is_none() {
        existing.peer_dependencies = incoming.peer_dependencies;
    }
    if existing.optional_dependencies.is_none() {
        existing.optional_dependencies = incoming.optional_dependencies;
    }

    existing.is_excluded |= incoming.is_excluded;
    existing.package_managers.sort();
    existing.features.sort();
}

fn merge_path_lists(existing: &[PathBuf], incoming: &[PathBuf]) -> Vec<PathBuf> {
    let mut merged = existing.to_vec();
    for path in incoming {
        if !merged.contains(path) {
            merged.push(path.clone());
        }
    }
    merged.sort();
    merged
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
pub(crate) fn discover_packages_with_optional_index(
    root: &Path,
    tool: MonorepoTool,
    lock_versions: &Option<CargoLockVersions>,
    discovery_source: PackageDiscoverySource,
    index: Option<&ManifestIndex>,
) -> Vec<Package> {
    mi_discover_packages_with_optional_index(root, tool, lock_versions, discovery_source, index)
}

/// Creates a Package with all metadata and parsed dependencies.
pub(crate) fn create_package(
    path: &Path,
    root: &Path,
    tool: MonorepoTool,
    lock_versions: &Option<CargoLockVersions>,
    discovery_source: PackageDiscoverySource,
) -> Package {
    let mut ctx = PackageBuildContext::new(lock_versions);
    let relative = make_relative_path(path, root);
    let package_area = make_package_area(&relative);
    let ecosystem = detect_package_ecosystem(path);
    let package_managers = detect_package_managers(path);
    let name = resolve_package_name(&mut ctx, path, root);
    let version = resolve_package_version(&mut ctx, path, root);

    let cargo_toml = path.join("Cargo.toml");
    let features = if cargo_toml.exists() {
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

    if cargo_toml.exists()
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
    if package_json.exists() {
        let js_package_manager = resolve_js_package_manager(tool, root, &package_managers);
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
    if pyproject_toml.exists()
        && let Some((normal, optional)) = ctx
            .manifests
            .pyproject(&pyproject_toml)
            .and_then(pyproject_dependencies_from_value)
    {
        dependencies.extend(normal);
        optional_dependencies.extend(optional);
    }
    let requirements_txt = path.join("requirements.txt");
    if requirements_txt.exists()
        && let Some(req_deps) = parse_requirements_txt_dependencies(&requirements_txt)
    {
        dependencies.extend(req_deps);
    }

    // Parse Go module dependencies when available.
    let go_mod = path.join("go.mod");
    if go_mod.exists()
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
        discovery_sources: vec![discovery_source],
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
    use crate::package::{DependencyEntry, DependencyKind};

    fn make_test_package(name: &str, deps: Vec<DependencyEntry>) -> Package {
        Package {
            name: name.to_string(),
            path: PathBuf::from(name),
            relative: name.to_string(),
            package_area: "root".to_string(),
            ecosystem: PackageEcosystem::Unknown,
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
