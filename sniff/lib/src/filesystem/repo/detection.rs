use crate::{Result, SniffError};
use biscuit_file::serde_yaml_ng;
use biscuit_file::toml_crate;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use tracing::debug;

use crate::filesystem::file_types::{FileAssociation, FileInventory, is_command_runner_filename};

use super::types::*;

pub(crate) fn detect_repo_inner(
    root: &Path,
    structure_only: bool,
) -> Result<(Option<RepoInfo>, Option<FileInventory>)> {
    detect_repo_inner_with_shared(root, structure_only, None, None)
}

/// Cache for parsed manifest files to avoid redundant I/O during repo detection.
#[derive(Default)]
#[allow(dead_code)]
struct ManifestCache {
    cargo: HashMap<PathBuf, toml_crate::Value>,
    npm: HashMap<PathBuf, serde_json::Value>,
    pyproject: HashMap<PathBuf, toml_crate::Value>,
    go_mod: HashMap<PathBuf, String>,
}

#[allow(dead_code)]
impl ManifestCache {
    fn get_cargo(&mut self, path: &Path) -> Option<&toml_crate::Value> {
        if !self.cargo.contains_key(path) {
            let content = std::fs::read_to_string(path).ok()?;
            let parsed = toml_crate::from_str(&content).ok()?;
            self.cargo.insert(path.to_path_buf(), parsed);
        }
        self.cargo.get(path)
    }

    fn get_npm(&mut self, path: &Path) -> Option<&serde_json::Value> {
        if !self.npm.contains_key(path) {
            let content = std::fs::read_to_string(path).ok()?;
            let parsed = serde_json::from_str(&content).ok()?;
            self.npm.insert(path.to_path_buf(), parsed);
        }
        self.npm.get(path)
    }

    fn get_pyproject(&mut self, path: &Path) -> Option<&toml_crate::Value> {
        if !self.pyproject.contains_key(path) {
            let content = std::fs::read_to_string(path).ok()?;
            let parsed = toml_crate::from_str(&content).ok()?;
            self.pyproject.insert(path.to_path_buf(), parsed);
        }
        self.pyproject.get(path)
    }

    fn get_go_mod(&mut self, path: &Path) -> Option<&str> {
        if !self.go_mod.contains_key(path) {
            let content = std::fs::read_to_string(path).ok()?;
            self.go_mod.insert(path.to_path_buf(), content);
        }
        self.go_mod.get(path).map(|s| s.as_str())
    }
}

pub(crate) fn detect_repo_inner_with_shared(
    root: &Path,
    structure_only: bool,
    shared_manifest_index: Option<&ManifestIndex>,
    shared_repo_inventory: Option<&FileInventory>,
) -> Result<(Option<RepoInfo>, Option<FileInventory>)> {
    // Build manifest index once for the entire tree unless the caller already
    // provided the shared view from a higher-level walk.
    let manifest_index = if structure_only || shared_manifest_index.is_some() {
        None
    } else {
        Some(ManifestIndex::build(root))
    };
    let manifest_index = shared_manifest_index.or(manifest_index.as_ref());

    let mut workspace_tools = Vec::new();
    let mut packages = Vec::new();

    collect_repo_info(
        detect_cargo_workspace(root)?,
        &mut workspace_tools,
        &mut packages,
    );
    collect_repo_info(
        detect_nx(root, manifest_index)?,
        &mut workspace_tools,
        &mut packages,
    );
    collect_repo_info(
        detect_turborepo(root, manifest_index)?,
        &mut workspace_tools,
        &mut packages,
    );
    collect_repo_info(
        detect_pnpm_workspace(root)?,
        &mut workspace_tools,
        &mut packages,
    );
    collect_repo_info(
        detect_yarn_workspace(root)?,
        &mut workspace_tools,
        &mut packages,
    );
    collect_repo_info(
        detect_npm_workspace(root)?,
        &mut workspace_tools,
        &mut packages,
    );
    collect_repo_info(
        detect_lerna(root, manifest_index)?,
        &mut workspace_tools,
        &mut packages,
    );

    if workspace_tools.is_empty() {
        return Ok((None, None));
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
            is_monorepo: true,
            monorepo_tool: workspace_tools.first().copied(),
            workspace_tools,
            root: root.to_path_buf(),
            dependencies: None,
            dev_dependencies: None,
            peer_dependencies: None,
            optional_dependencies: None,
            packages: Some(packages),
        }),
        repo_inventory,
    ))
}

fn collect_repo_info(
    info: Option<RepoInfo>,
    workspace_tools: &mut Vec<MonorepoTool>,
    packages: &mut Vec<Package>,
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

    if let Some(mut detected_packages) = info.packages {
        packages.append(&mut detected_packages);
    }
}

fn detect_cargo_workspace(root: &Path) -> Result<Option<RepoInfo>> {
    let cargo_toml = root.join("Cargo.toml");
    if !cargo_toml.exists() {
        return Ok(None);
    }

    let content = std::fs::read_to_string(&cargo_toml)?;
    let parsed: toml_crate::Value =
        toml_crate::from_str(&content).map_err(|e| SniffError::SystemInfo {
            domain: "repo",
            message: e.to_string(),
        })?;

    let workspace = match parsed.get("workspace") {
        Some(w) => w,
        None => return Ok(None),
    };

    let members = workspace
        .get("members")
        .and_then(|m| m.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    if members.is_empty() {
        return Ok(None);
    }

    // Parse Cargo.lock once for version resolution
    let lock_versions = CargoLockVersions::parse(&root.join("Cargo.lock"));

    let excludes = workspace
        .get("exclude")
        .and_then(|m| m.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    // Expand globs and collect packages with dependencies
    let mut packages = expand_glob_patterns_with_deps(
        root,
        &members,
        MonorepoTool::CargoWorkspace,
        &lock_versions,
    );

    // Expand excluded patterns and mark them
    let mut excluded_packages = expand_glob_patterns_with_deps(
        root,
        &excludes,
        MonorepoTool::CargoWorkspace,
        &lock_versions,
    );
    for pkg in &mut excluded_packages {
        pkg.is_excluded = true;
    }
    packages.extend(excluded_packages);

    // Resolve internal dependency graph
    resolve_internal_deps(&mut packages);

    Ok(Some(RepoInfo {
        is_monorepo: true,
        monorepo_tool: Some(MonorepoTool::CargoWorkspace),
        workspace_tools: vec![MonorepoTool::CargoWorkspace],
        root: root.to_path_buf(),
        dependencies: None,
        dev_dependencies: None,
        peer_dependencies: None,
        optional_dependencies: None,
        packages: Some(packages),
    }))
}

/// Parses Cargo.toml dependencies into DependencyEntry structs.
fn parse_cargo_dependencies(
    toml_path: &Path,
    lock_versions: &Option<CargoLockVersions>,
) -> Option<(
    Vec<DependencyEntry>,
    Vec<DependencyEntry>,
    Vec<DependencyEntry>,
)> {
    let content = std::fs::read_to_string(toml_path)
        .map_err(|e| {
            debug!(path = %toml_path.display(), error = %e, "could not read file");
            e
        })
        .ok()?;
    let parsed: toml_crate::Value = toml_crate::from_str(&content).ok()?;

    let normal_deps = parse_cargo_dep_section(
        &parsed,
        "dependencies",
        DependencyKind::Normal,
        lock_versions,
    );
    let dev_deps = parse_cargo_dep_section(
        &parsed,
        "dev-dependencies",
        DependencyKind::Dev,
        lock_versions,
    );
    let build_deps = parse_cargo_dep_section(
        &parsed,
        "build-dependencies",
        DependencyKind::Build,
        lock_versions,
    );

    Some((normal_deps, dev_deps, build_deps))
}

/// Parses a single dependencies section from Cargo.toml.
fn parse_cargo_dep_section(
    parsed: &toml_crate::Value,
    section: &str,
    kind: DependencyKind,
    lock_versions: &Option<CargoLockVersions>,
) -> Vec<DependencyEntry> {
    let Some(deps) = parsed.get(section).and_then(|d| d.as_table()) else {
        return Vec::new();
    };

    deps.iter()
        .map(|(name, value)| {
            let (version_req, features, optional) = match value {
                toml_crate::Value::String(v) => (v.clone(), Vec::new(), false),
                toml_crate::Value::Table(t) => {
                    let version = t
                        .get("version")
                        .and_then(|v| v.as_str())
                        .unwrap_or("*")
                        .to_string();
                    let features = t
                        .get("features")
                        .and_then(|f| f.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(String::from))
                                .collect()
                        })
                        .unwrap_or_default();
                    let optional = t.get("optional").and_then(|o| o.as_bool()).unwrap_or(false);
                    (version, features, optional)
                }
                _ => ("*".to_string(), Vec::new(), false),
            };

            let actual_version = lock_versions.as_ref().and_then(|lv| lv.resolve(name));

            DependencyEntry {
                name: name.clone(),
                kind,
                targeted_version: version_req,
                actual_version,
                package_manager: Some("cargo".to_string()),
                latest_version: None,
                target: None,
                optional,
                features,
                is_updatable: false,
                has_major_update: false,
            }
        })
        .collect()
}

/// Parses a single dependency section from package.json.
fn parse_package_json_dep_section(
    parsed: &serde_json::Value,
    section: &str,
    kind: DependencyKind,
    package_manager: &str,
    optional: bool,
) -> Vec<DependencyEntry> {
    let Some(deps) = parsed.get(section).and_then(|d| d.as_object()) else {
        return Vec::new();
    };

    deps.iter()
        .map(|(name, value)| {
            let targeted_version = value
                .as_str()
                .map(String::from)
                .or_else(|| {
                    value
                        .get("version")
                        .and_then(|v| v.as_str())
                        .map(String::from)
                })
                .unwrap_or_else(|| "*".to_string());

            DependencyEntry {
                name: name.clone(),
                kind,
                targeted_version,
                actual_version: None,
                package_manager: Some(package_manager.to_string()),
                latest_version: None,
                target: None,
                optional,
                features: Vec::new(),
                is_updatable: false,
                has_major_update: false,
            }
        })
        .collect()
}

/// Parses package.json dependencies into category-specific vectors.
#[allow(clippy::type_complexity)]
fn parse_package_json_dependencies(
    package_json_path: &Path,
    package_manager: &str,
) -> Option<(
    Vec<DependencyEntry>,
    Vec<DependencyEntry>,
    Vec<DependencyEntry>,
    Vec<DependencyEntry>,
)> {
    let content = std::fs::read_to_string(package_json_path)
        .map_err(|e| {
            debug!(path = %package_json_path.display(), error = %e, "could not read file");
            e
        })
        .ok()?;
    let parsed: serde_json::Value = serde_json::from_str(&content).ok()?;

    let deps = parse_package_json_dep_section(
        &parsed,
        "dependencies",
        DependencyKind::Normal,
        package_manager,
        false,
    );
    let dev_deps = parse_package_json_dep_section(
        &parsed,
        "devDependencies",
        DependencyKind::Dev,
        package_manager,
        false,
    );
    let peer_deps = parse_package_json_dep_section(
        &parsed,
        "peerDependencies",
        DependencyKind::Normal,
        package_manager,
        false,
    );
    let optional_deps = parse_package_json_dep_section(
        &parsed,
        "optionalDependencies",
        DependencyKind::Optional,
        package_manager,
        true,
    );

    Some((deps, dev_deps, peer_deps, optional_deps))
}

/// Extracts package name from a PEP 508 requirement string.
fn parse_python_requirement_name(requirement: &str) -> Option<String> {
    let without_comment = requirement.split('#').next()?.trim();
    if without_comment.is_empty() {
        return None;
    }

    let before_marker = without_comment
        .split(';')
        .next()
        .unwrap_or(without_comment)
        .trim();
    if before_marker.is_empty() {
        return None;
    }

    let before_at = before_marker
        .split('@')
        .next()
        .unwrap_or(before_marker)
        .trim();
    if before_at.is_empty() {
        return None;
    }

    let end = before_at
        .find(|c: char| {
            c == '['
                || c == '<'
                || c == '>'
                || c == '='
                || c == '!'
                || c == '~'
                || c.is_whitespace()
        })
        .unwrap_or(before_at.len());
    let name = before_at[..end].trim();

    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

/// Parses pyproject.toml dependencies from PEP 621 `[project]` sections.
fn parse_pyproject_dependencies(
    pyproject_path: &Path,
) -> Option<(Vec<DependencyEntry>, Vec<DependencyEntry>)> {
    let content = std::fs::read_to_string(pyproject_path)
        .map_err(|e| {
            debug!(path = %pyproject_path.display(), error = %e, "could not read file");
            e
        })
        .ok()?;
    let parsed: toml_crate::Value = toml_crate::from_str(&content).ok()?;
    let project = parsed.get("project")?;

    let dependencies = project
        .get("dependencies")
        .and_then(|v| v.as_array())
        .map(|deps| {
            deps.iter()
                .filter_map(|entry| entry.as_str())
                .filter_map(|req| parse_python_requirement_name(req).map(|name| (name, req)))
                .map(|(name, req)| DependencyEntry {
                    name,
                    kind: DependencyKind::Normal,
                    targeted_version: req.to_string(),
                    actual_version: None,
                    package_manager: Some("pip".to_string()),
                    latest_version: None,
                    target: None,
                    optional: false,
                    features: Vec::new(),
                    is_updatable: false,
                    has_major_update: false,
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let optional_dependencies = project
        .get("optional-dependencies")
        .and_then(|v| v.as_table())
        .map(|groups| {
            groups
                .values()
                .filter_map(|value| value.as_array())
                .flat_map(|entries| entries.iter())
                .filter_map(|entry| entry.as_str())
                .filter_map(|req| parse_python_requirement_name(req).map(|name| (name, req)))
                .map(|(name, req)| DependencyEntry {
                    name,
                    kind: DependencyKind::Optional,
                    targeted_version: req.to_string(),
                    actual_version: None,
                    package_manager: Some("pip".to_string()),
                    latest_version: None,
                    target: None,
                    optional: true,
                    features: Vec::new(),
                    is_updatable: false,
                    has_major_update: false,
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    Some((dependencies, optional_dependencies))
}

/// Parses requirements.txt dependency lines.
fn parse_requirements_txt_dependencies(requirements_path: &Path) -> Option<Vec<DependencyEntry>> {
    let content = std::fs::read_to_string(requirements_path)
        .map_err(|e| {
            debug!(path = %requirements_path.display(), error = %e, "could not read file");
            e
        })
        .ok()?;
    let mut deps = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        let Some(name) = parse_python_requirement_name(trimmed) else {
            continue;
        };

        deps.push(DependencyEntry {
            name,
            kind: DependencyKind::Normal,
            targeted_version: trimmed.to_string(),
            actual_version: None,
            package_manager: Some("pip".to_string()),
            latest_version: None,
            target: None,
            optional: false,
            features: Vec::new(),
            is_updatable: false,
            has_major_update: false,
        });
    }

    if deps.is_empty() { None } else { Some(deps) }
}

/// Parses go.mod `require` entries.
fn parse_go_mod_dependencies(go_mod_path: &Path) -> Option<Vec<DependencyEntry>> {
    let content = std::fs::read_to_string(go_mod_path)
        .map_err(|e| {
            debug!(path = %go_mod_path.display(), error = %e, "could not read file");
            e
        })
        .ok()?;
    let mut deps = Vec::new();
    let mut in_require_block = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") {
            continue;
        }

        if in_require_block {
            if trimmed.starts_with(')') {
                in_require_block = false;
                continue;
            }

            let line_without_comment = trimmed.split("//").next().unwrap_or(trimmed).trim();
            let mut parts = line_without_comment.split_whitespace();
            let Some(name) = parts.next() else {
                continue;
            };
            let Some(version) = parts.next() else {
                continue;
            };

            deps.push(DependencyEntry {
                name: name.to_string(),
                kind: DependencyKind::Normal,
                targeted_version: version.to_string(),
                actual_version: None,
                package_manager: Some("go".to_string()),
                latest_version: None,
                target: None,
                optional: false,
                features: Vec::new(),
                is_updatable: false,
                has_major_update: false,
            });

            continue;
        }

        if trimmed.starts_with("require (") {
            in_require_block = true;
            continue;
        }

        if let Some(rest) = trimmed.strip_prefix("require ") {
            let line_without_comment = rest.split("//").next().unwrap_or(rest).trim();
            let mut parts = line_without_comment.split_whitespace();
            let Some(name) = parts.next() else {
                continue;
            };
            let Some(version) = parts.next() else {
                continue;
            };

            deps.push(DependencyEntry {
                name: name.to_string(),
                kind: DependencyKind::Normal,
                targeted_version: version.to_string(),
                actual_version: None,
                package_manager: Some("go".to_string()),
                latest_version: None,
                target: None,
                optional: false,
                features: Vec::new(),
                is_updatable: false,
                has_major_update: false,
            });
        }
    }

    if deps.is_empty() { None } else { Some(deps) }
}

fn detect_pnpm_workspace(root: &Path) -> Result<Option<RepoInfo>> {
    let pnpm_workspace = root.join("pnpm-workspace.yaml");
    if !pnpm_workspace.exists() {
        return Ok(None);
    }

    let packages = parse_pnpm_workspace_patterns(&pnpm_workspace)?;

    if packages.is_empty() {
        return Ok(None);
    }

    let lock_versions = None;
    let mut package_locations = expand_glob_patterns_with_deps(
        root,
        &packages,
        MonorepoTool::PnpmWorkspaces,
        &lock_versions,
    );
    package_locations = dedupe_packages(package_locations);
    resolve_internal_deps(&mut package_locations);

    Ok(Some(RepoInfo {
        is_monorepo: true,
        monorepo_tool: Some(MonorepoTool::PnpmWorkspaces),
        workspace_tools: vec![MonorepoTool::PnpmWorkspaces],
        root: root.to_path_buf(),
        dependencies: None,
        dev_dependencies: None,
        peer_dependencies: None,
        optional_dependencies: None,
        packages: Some(package_locations),
    }))
}

fn detect_npm_workspace(root: &Path) -> Result<Option<RepoInfo>> {
    let package_json = root.join("package.json");
    if !package_json.exists() {
        return Ok(None);
    }

    let workspaces = parse_package_json_workspace_patterns(&package_json)?.unwrap_or_default();

    if workspaces.is_empty() {
        return Ok(None);
    }

    let lock_versions = None;
    let mut packages = expand_glob_patterns_with_deps(
        root,
        &workspaces,
        MonorepoTool::NpmWorkspaces,
        &lock_versions,
    );
    packages = dedupe_packages(packages);
    resolve_internal_deps(&mut packages);

    Ok(Some(RepoInfo {
        is_monorepo: true,
        monorepo_tool: Some(MonorepoTool::NpmWorkspaces),
        workspace_tools: vec![MonorepoTool::NpmWorkspaces],
        root: root.to_path_buf(),
        dependencies: None,
        dev_dependencies: None,
        peer_dependencies: None,
        optional_dependencies: None,
        packages: Some(packages),
    }))
}

fn detect_yarn_workspace(root: &Path) -> Result<Option<RepoInfo>> {
    if !root.join("yarn.lock").exists() {
        return Ok(None);
    }

    let package_json = root.join("package.json");
    if !package_json.exists() {
        return Ok(None);
    }

    let workspaces = parse_package_json_workspace_patterns(&package_json)?.unwrap_or_default();

    if workspaces.is_empty() {
        return Ok(None);
    }

    let lock_versions = None;
    let mut packages = expand_glob_patterns_with_deps(
        root,
        &workspaces,
        MonorepoTool::YarnWorkspaces,
        &lock_versions,
    );
    packages = dedupe_packages(packages);
    resolve_internal_deps(&mut packages);

    Ok(Some(RepoInfo {
        is_monorepo: true,
        monorepo_tool: Some(MonorepoTool::YarnWorkspaces),
        workspace_tools: vec![MonorepoTool::YarnWorkspaces],
        root: root.to_path_buf(),
        dependencies: None,
        dev_dependencies: None,
        peer_dependencies: None,
        optional_dependencies: None,
        packages: Some(packages),
    }))
}

fn detect_nx(root: &Path, index: Option<&ManifestIndex>) -> Result<Option<RepoInfo>> {
    let nx_json = root.join("nx.json");
    if !nx_json.exists() {
        return Ok(None);
    }

    let mut patterns = collect_default_workspace_patterns(root);
    patterns.extend(parse_nx_layout_patterns(&nx_json));
    patterns = dedupe_patterns(patterns);

    let lock_versions = None;
    let mut packages = if patterns.is_empty() {
        discover_packages_with_optional_index(
            root,
            MonorepoTool::Nx,
            &lock_versions,
            PackageDiscoverySource::Nx,
            index,
        )
    } else {
        expand_glob_patterns_with_deps(root, &patterns, MonorepoTool::Nx, &lock_versions)
    };
    if packages.is_empty() {
        packages = discover_packages_with_optional_index(
            root,
            MonorepoTool::Nx,
            &lock_versions,
            PackageDiscoverySource::Nx,
            index,
        );
    }
    packages = dedupe_packages(packages);
    resolve_internal_deps(&mut packages);

    Ok(Some(RepoInfo {
        is_monorepo: true,
        monorepo_tool: Some(MonorepoTool::Nx),
        workspace_tools: vec![MonorepoTool::Nx],
        root: root.to_path_buf(),
        dependencies: None,
        dev_dependencies: None,
        peer_dependencies: None,
        optional_dependencies: None,
        packages: Some(packages),
    }))
}

fn detect_turborepo(root: &Path, index: Option<&ManifestIndex>) -> Result<Option<RepoInfo>> {
    let turbo_json = root.join("turbo.json");
    if !turbo_json.exists() {
        return Ok(None);
    }

    let patterns = collect_default_workspace_patterns(root);
    let lock_versions = None;
    let mut packages = if patterns.is_empty() {
        discover_packages_with_optional_index(
            root,
            MonorepoTool::Turborepo,
            &lock_versions,
            PackageDiscoverySource::Turborepo,
            index,
        )
    } else {
        expand_glob_patterns_with_deps(root, &patterns, MonorepoTool::Turborepo, &lock_versions)
    };
    if packages.is_empty() {
        packages = discover_packages_with_optional_index(
            root,
            MonorepoTool::Turborepo,
            &lock_versions,
            PackageDiscoverySource::Turborepo,
            index,
        );
    }
    packages = dedupe_packages(packages);
    resolve_internal_deps(&mut packages);

    Ok(Some(RepoInfo {
        is_monorepo: true,
        monorepo_tool: Some(MonorepoTool::Turborepo),
        workspace_tools: vec![MonorepoTool::Turborepo],
        root: root.to_path_buf(),
        dependencies: None,
        dev_dependencies: None,
        peer_dependencies: None,
        optional_dependencies: None,
        packages: Some(packages),
    }))
}

fn detect_lerna(root: &Path, index: Option<&ManifestIndex>) -> Result<Option<RepoInfo>> {
    let lerna_json = root.join("lerna.json");
    if !lerna_json.exists() {
        return Ok(None);
    }

    let mut patterns = parse_lerna_workspace_patterns(&lerna_json).unwrap_or_default();
    patterns.extend(collect_default_workspace_patterns(root));
    patterns = dedupe_patterns(patterns);

    let lock_versions = None;
    let mut packages = if patterns.is_empty() {
        discover_packages_with_optional_index(
            root,
            MonorepoTool::Lerna,
            &lock_versions,
            PackageDiscoverySource::Lerna,
            index,
        )
    } else {
        expand_glob_patterns_with_deps(root, &patterns, MonorepoTool::Lerna, &lock_versions)
    };
    if packages.is_empty() {
        packages = discover_packages_with_optional_index(
            root,
            MonorepoTool::Lerna,
            &lock_versions,
            PackageDiscoverySource::Lerna,
            index,
        );
    }
    packages = dedupe_packages(packages);
    resolve_internal_deps(&mut packages);

    Ok(Some(RepoInfo {
        is_monorepo: true,
        monorepo_tool: Some(MonorepoTool::Lerna),
        workspace_tools: vec![MonorepoTool::Lerna],
        root: root.to_path_buf(),
        dependencies: None,
        dev_dependencies: None,
        peer_dependencies: None,
        optional_dependencies: None,
        packages: Some(packages),
    }))
}

fn parse_pnpm_workspace_patterns(pnpm_workspace_path: &Path) -> Result<Vec<String>> {
    let content = std::fs::read_to_string(pnpm_workspace_path)?;
    let parsed: serde_yaml_ng::Value =
        serde_yaml_ng::from_str(&content).map_err(|e| SniffError::SystemInfo {
            domain: "repo",
            message: e.to_string(),
        })?;

    Ok(parsed
        .get("packages")
        .and_then(|p| p.as_sequence())
        .map(|seq| {
            seq.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default())
}

fn parse_package_json_workspace_patterns(package_json_path: &Path) -> Result<Option<Vec<String>>> {
    let content = std::fs::read_to_string(package_json_path)?;
    let parsed: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| SniffError::SystemInfo {
            domain: "repo",
            message: e.to_string(),
        })?;
    let Some(workspaces) = parsed.get("workspaces") else {
        return Ok(None);
    };

    if let Some(arr) = workspaces.as_array() {
        return Ok(Some(
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect(),
        ));
    }

    if let Some(obj) = workspaces.as_object() {
        return Ok(Some(
            obj.get("packages")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default(),
        ));
    }

    Ok(Some(Vec::new()))
}

fn parse_lerna_workspace_patterns(lerna_json_path: &Path) -> Option<Vec<String>> {
    let content = std::fs::read_to_string(lerna_json_path)
        .map_err(|e| {
            debug!(path = %lerna_json_path.display(), error = %e, "could not read file");
            e
        })
        .ok()?;
    let parsed: serde_json::Value = serde_json::from_str(&content).ok()?;

    parsed
        .get("packages")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
}

fn parse_nx_layout_patterns(nx_json_path: &Path) -> Vec<String> {
    let content = match std::fs::read_to_string(nx_json_path) {
        Ok(content) => content,
        Err(_) => return Vec::new(),
    };
    let parsed: serde_json::Value = match serde_json::from_str(&content) {
        Ok(parsed) => parsed,
        Err(_) => return Vec::new(),
    };

    let apps_dir = parsed
        .get("workspaceLayout")
        .and_then(|v| v.get("appsDir"))
        .and_then(|v| v.as_str())
        .unwrap_or("apps");
    let libs_dir = parsed
        .get("workspaceLayout")
        .and_then(|v| v.get("libsDir"))
        .and_then(|v| v.as_str())
        .unwrap_or("libs");

    vec![format!("{apps_dir}/*"), format!("{libs_dir}/*")]
}

fn collect_default_workspace_patterns(root: &Path) -> Vec<String> {
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
// Package name reading helpers
// ============================================================================

/// Reads the package name from a Cargo.toml file.
fn read_cargo_package_name(cargo_toml: &Path) -> Option<String> {
    let content = std::fs::read_to_string(cargo_toml).ok()?;
    let parsed: toml_crate::Value = toml_crate::from_str(&content).ok()?;
    parsed
        .get("package")
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
        .map(String::from)
}

/// Reads the package version from a Cargo.toml file.
fn read_cargo_package_version(cargo_toml: &Path) -> Option<String> {
    let content = std::fs::read_to_string(cargo_toml).ok()?;
    let parsed: toml_crate::Value = toml_crate::from_str(&content).ok()?;
    parsed
        .get("package")
        .and_then(|p| p.get("version"))
        .and_then(|v| v.as_str())
        .map(String::from)
}

/// Reads the package name from a package.json file.
fn read_npm_package_name(package_json: &Path) -> Option<String> {
    let content = std::fs::read_to_string(package_json).ok()?;
    let parsed: serde_json::Value = serde_json::from_str(&content).ok()?;
    parsed
        .get("name")
        .and_then(|n| n.as_str())
        .map(String::from)
}

/// Reads the package version from a package.json file.
fn read_npm_package_version(package_json: &Path) -> Option<String> {
    let content = std::fs::read_to_string(package_json).ok()?;
    let parsed: serde_json::Value = serde_json::from_str(&content).ok()?;
    parsed
        .get("version")
        .and_then(|v| v.as_str())
        .map(String::from)
}

/// Reads package name from a pyproject.toml `[project].name`.
fn read_pyproject_package_name(pyproject_toml: &Path) -> Option<String> {
    let content = std::fs::read_to_string(pyproject_toml).ok()?;
    let parsed: toml_crate::Value = toml_crate::from_str(&content).ok()?;
    parsed
        .get("project")
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
        .map(String::from)
}

/// Reads package version from a pyproject.toml `[project].version`.
fn read_pyproject_package_version(pyproject_toml: &Path) -> Option<String> {
    let content = std::fs::read_to_string(pyproject_toml).ok()?;
    let parsed: toml_crate::Value = toml_crate::from_str(&content).ok()?;
    parsed
        .get("project")
        .and_then(|p| p.get("version"))
        .and_then(|v| v.as_str())
        .map(String::from)
}

/// Reads module name from a go.mod file (module directive).
fn read_go_module_name(go_mod: &Path) -> Option<String> {
    let content = std::fs::read_to_string(go_mod).ok()?;
    content.lines().find_map(|line| {
        let trimmed = line.trim();
        trimmed
            .strip_prefix("module ")
            .map(|value| value.trim().to_string())
    })
}

/// Reads the feature flag names from a Cargo.toml `[features]` section.
fn read_cargo_features(cargo_toml: &Path) -> Vec<String> {
    let content = match std::fs::read_to_string(cargo_toml) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let parsed: toml_crate::Value = match toml_crate::from_str(&content) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    let Some(features) = parsed.get("features").and_then(|f| f.as_table()) else {
        return Vec::new();
    };
    let mut names: Vec<String> = features.keys().cloned().collect();
    names.sort();
    names
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
fn resolve_package_name(path: &Path, root: &Path) -> String {
    let cargo_toml = path.join("Cargo.toml");
    if cargo_toml.exists()
        && let Some(name) = read_cargo_package_name(&cargo_toml)
    {
        return name;
    }

    let package_json = path.join("package.json");
    if package_json.exists()
        && let Some(name) = read_npm_package_name(&package_json)
    {
        return name;
    }

    let pyproject_toml = path.join("pyproject.toml");
    if pyproject_toml.exists()
        && let Some(name) = read_pyproject_package_name(&pyproject_toml)
    {
        return name;
    }

    let go_mod = path.join("go.mod");
    if go_mod.exists()
        && let Some(name) = read_go_module_name(&go_mod)
    {
        return name;
    }

    // Fallback: relative path from root
    make_relative_path(path, root)
}

/// Determines the package version based on manifests present in the package.
fn resolve_package_version(path: &Path, root: &Path) -> Option<String> {
    let cargo_toml = path.join("Cargo.toml");
    if cargo_toml.exists() {
        return read_cargo_package_version(&cargo_toml);
    }

    let package_json = path.join("package.json");
    if package_json.exists() {
        if let Some(version) = read_npm_package_version(&package_json) {
            return Some(version);
        }
        if path != root {
            let root_package_json = root.join("package.json");
            if root_package_json.exists() {
                return read_npm_package_version(&root_package_json);
            }
        }
    }

    let pyproject_toml = path.join("pyproject.toml");
    if pyproject_toml.exists()
        && let Some(version) = read_pyproject_package_version(&pyproject_toml)
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
fn resolve_internal_deps(packages: &mut [Package]) {
    // Collect all package names into owned Strings first to avoid borrow issues
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
    // Build a HashMap from dependency name → list of packages that depend on it.
    // We collect into owned data first to avoid borrow issues during mutation.
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
        let mut used_by = used_by_map
            .remove(&pkg.name)
            .unwrap_or_default();
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
fn normalize_path(path: &Path) -> PathBuf {
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
        // For relative paths, resolve against current dir then normalize
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
        // Use shared repo inventory and project to this package
        crate::filesystem::file_types::project_package_inventory(repo_inv, path, exclude_roots)
    } else {
        // Fall back to per-package scanning
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

fn resolve_js_package_manager(
    tool: MonorepoTool,
    root: &Path,
    package_managers: &[String],
) -> &'static str {
    match tool {
        MonorepoTool::PnpmWorkspaces => return "pnpm",
        MonorepoTool::YarnWorkspaces => return "yarn",
        _ => {}
    }

    if package_managers.iter().any(|manager| manager == "pnpm")
        || root.join("pnpm-lock.yaml").exists()
    {
        return "pnpm";
    }
    if package_managers.iter().any(|manager| manager == "yarn") || root.join("yarn.lock").exists() {
        return "yarn";
    }
    if root.join("bun.lock").exists() || root.join("bun.lockb").exists() {
        return "bun";
    }

    "npm"
}

fn discovery_source_for_tool(tool: MonorepoTool) -> PackageDiscoverySource {
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

fn dedupe_patterns(patterns: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut deduped = Vec::new();

    for pattern in patterns {
        if seen.insert(pattern.clone()) {
            deduped.push(pattern);
        }
    }

    deduped
}

fn dedupe_packages(packages: Vec<Package>) -> Vec<Package> {
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

fn refresh_package_boundaries(packages: &mut [Package], repo_inventory: Option<&FileInventory>) {
    // Build sorted list of (canonical_root, original_path, name, index) for efficient lookup
    let mut sorted_packages: Vec<(PathBuf, PathBuf, String, usize)> = packages
        .iter()
        .enumerate()
        .map(|(i, pkg)| {
            (
                canonicalize_path(&pkg.path),
                pkg.path.clone(),
                pkg.name.clone(),
                i,
            )
        })
        .collect();
    // Sort by path depth (shorter paths first = ancestors first)
    sorted_packages.sort_by_key(|a| a.0.components().count());

    // Build a map from original index to its descendants
    let mut descendants: Vec<Vec<(PathBuf, String)>> = vec![Vec::new(); packages.len()];

    // For each package, only check packages that come AFTER it in the sorted list
    // (which are guaranteed to be at least as deep)
    for i in 0..sorted_packages.len() {
        let (root_i, _, _, idx_i) = &sorted_packages[i];
        for (root_j, path_j, name_j, idx_j) in sorted_packages.iter().skip(i + 1) {
            if root_j.starts_with(root_i) && idx_i != idx_j {
                descendants[*idx_i].push((path_j.clone(), name_j.clone()));
            }
        }
    }

    for (index, package) in packages.iter_mut().enumerate() {
        let mut nested_roots: Vec<PathBuf> = Vec::new();
        let mut nested_packages: Vec<String> = Vec::new();

        for (path, name) in &descendants[index] {
            nested_roots.push(path.clone());
            nested_packages.push(name.clone());
        }

        nested_roots.sort();
        nested_packages.sort();

        let scan = detect_package_languages(
            &package.relative,
            &package.path,
            &nested_roots,
            repo_inventory,
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
        package.nested_packages = nested_packages;
    }
}

fn discover_packages_from_manifests(
    root: &Path,
    tool: MonorepoTool,
    lock_versions: &Option<CargoLockVersions>,
    discovery_source: PackageDiscoverySource,
) -> Vec<Package> {
    discover_packages_from_manifests_in_tree(root, root, tool, lock_versions, discovery_source)
}

/// Discover packages using manifest index if available, otherwise walk filesystem.
fn discover_packages_with_optional_index(
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

fn discover_packages_from_manifests_in_tree(
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
fn discover_packages_from_index(
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

#[allow(dead_code)]
/// Expand glob patterns and parse dependencies for Cargo workspaces.
fn expand_glob_patterns_with_deps(
    root: &Path,
    patterns: &[String],
    tool: MonorepoTool,
    lock_versions: &Option<CargoLockVersions>,
) -> Vec<Package> {
    let mut packages = Vec::new();

    for pattern in patterns {
        if pattern.contains('*') {
            let parts: Vec<&str> = pattern.split('*').collect();
            if let Some(prefix) = parts.first() {
                let search_dir = root.join(prefix.trim_end_matches('/'));
                if let Ok(entries) = std::fs::read_dir(&search_dir) {
                    for entry in entries.filter_map(|e| e.ok()) {
                        if entry.path().is_dir() {
                            let path = entry.path();
                            packages.push(create_package(
                                &path,
                                root,
                                tool,
                                lock_versions,
                                discovery_source_for_tool(tool),
                            ));
                        }
                    }
                }
            }
        } else {
            let path = root.join(pattern);
            if path.exists() {
                packages.push(create_package(
                    &path,
                    root,
                    tool,
                    lock_versions,
                    discovery_source_for_tool(tool),
                ));
            }
        }
    }

    packages
}

/// Creates a Package with all metadata and parsed dependencies.
fn create_package(
    path: &Path,
    root: &Path,
    tool: MonorepoTool,
    lock_versions: &Option<CargoLockVersions>,
    discovery_source: PackageDiscoverySource,
) -> Package {
    let relative = make_relative_path(path, root);
    let package_area = make_package_area(&relative);
    let name = resolve_package_name(path, root);
    let ecosystem = detect_package_ecosystem(path);
    let package_managers = detect_package_managers(path);
    let version = resolve_package_version(path, root);

    // Read feature flags (Cargo only for now)
    let cargo_toml = path.join("Cargo.toml");
    let features = if cargo_toml.exists() {
        read_cargo_features(&cargo_toml)
    } else {
        Vec::new()
    };

    let mut dependencies = Vec::new();
    let mut dev_dependencies = Vec::new();
    let mut peer_dependencies = Vec::new();
    let mut optional_dependencies = Vec::new();

    if cargo_toml.exists()
        && let Some((normal, dev, build)) = parse_cargo_dependencies(&cargo_toml, lock_versions)
    {
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
        if let Some((normal, dev, peer, optional)) =
            parse_package_json_dependencies(&package_json, js_package_manager)
        {
            dependencies.extend(normal);
            dev_dependencies.extend(dev);
            peer_dependencies.extend(peer);
            optional_dependencies.extend(optional);
        }
    }

    // Parse Python dependencies from pyproject.toml / requirements.txt.
    let pyproject_toml = path.join("pyproject.toml");
    if pyproject_toml.exists()
        && let Some((normal, optional)) = parse_pyproject_dependencies(&pyproject_toml)
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
        && let Some(go_deps) = parse_go_mod_dependencies(&go_mod)
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
    // normalize_path tests (issue #18)
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
}
