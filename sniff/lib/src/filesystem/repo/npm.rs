//! npm/pnpm/yarn workspace detection and package.json parsing helpers.

use std::path::Path;

use biscuit_file::serde_yaml_ng;

use crate::package::{DependencyEntry, DependencyKind};
use crate::{Result, SniffError};

use super::detection::{DetectorOutcome, create_package, dedupe_packages, resolve_internal_deps};
use super::glob::expand_membership_globs;
use super::standard::{GlobDialect, MonorepoStandard, PackageProvenance};

/// Parses a single dependency section from package.json.
pub(super) fn parse_package_json_dep_section(
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

/// Parses package.json dependencies from an already-parsed JSON value.
#[allow(clippy::type_complexity)]
pub(super) fn package_json_dependencies_from_value(
    parsed: &serde_json::Value,
    package_manager: &str,
) -> (
    Vec<DependencyEntry>,
    Vec<DependencyEntry>,
    Vec<DependencyEntry>,
    Vec<DependencyEntry>,
) {
    let deps = parse_package_json_dep_section(
        parsed,
        "dependencies",
        DependencyKind::Normal,
        package_manager,
        false,
    );
    let dev_deps = parse_package_json_dep_section(
        parsed,
        "devDependencies",
        DependencyKind::Dev,
        package_manager,
        false,
    );
    let peer_deps = parse_package_json_dep_section(
        parsed,
        "peerDependencies",
        DependencyKind::Normal,
        package_manager,
        false,
    );
    let optional_deps = parse_package_json_dep_section(
        parsed,
        "optionalDependencies",
        DependencyKind::Optional,
        package_manager,
        true,
    );

    (deps, dev_deps, peer_deps, optional_deps)
}

/// Extracts the package name from a parsed package.json value.
pub(crate) fn npm_package_name(parsed: &serde_json::Value) -> Option<String> {
    parsed
        .get("name")
        .and_then(|n| n.as_str())
        .map(String::from)
}

/// Extracts the package version from a parsed package.json value.
pub(crate) fn npm_package_version(parsed: &serde_json::Value) -> Option<String> {
    parsed
        .get("version")
        .and_then(|v| v.as_str())
        .map(String::from)
}

pub(super) fn detect_pnpm_workspace(root: &Path) -> Result<Option<DetectorOutcome>> {
    let pnpm_workspace = root.join("pnpm-workspace.yaml");
    if !pnpm_workspace.exists() {
        return Ok(None);
    }

    let packages = parse_pnpm_workspace_patterns(&pnpm_workspace)?;

    if packages.is_empty() {
        return Ok(None);
    }

    let lock_versions = None;
    let dialect = MonorepoStandard::PnpmWorkspaces
        .glob_dialect()
        .unwrap_or(GlobDialect::Minimatch);
    let mut package_locations = expand_membership_globs(
        root,
        &packages,
        dialect,
        MonorepoStandard::PnpmWorkspaces,
        None,
        &lock_versions,
    );
    package_locations = dedupe_packages(package_locations);
    resolve_internal_deps(&mut package_locations);

    Ok(Some(DetectorOutcome {
        standard: MonorepoStandard::PnpmWorkspaces,
        root: root.to_path_buf(),
        packages: package_locations,
    }))
}

/// Whether a Bun lockfile (`bun.lock` or `bun.lockb`) is present at `root`.
///
/// Bun and npm/yarn all declare members via `package.json#workspaces`; the
/// lockfile is what disambiguates Bun so it wins the membership authority.
fn has_bun_lockfile(root: &Path) -> bool {
    root.join("bun.lock").exists() || root.join("bun.lockb").exists()
}

pub(super) fn detect_bun_workspace(root: &Path) -> Result<Option<DetectorOutcome>> {
    if !has_bun_lockfile(root) {
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
    let dialect = MonorepoStandard::BunWorkspaces
        .glob_dialect()
        .unwrap_or(GlobDialect::Minimatch);
    let mut packages = expand_membership_globs(
        root,
        &workspaces,
        dialect,
        MonorepoStandard::BunWorkspaces,
        None,
        &lock_versions,
    );
    packages = dedupe_packages(packages);
    resolve_internal_deps(&mut packages);

    Ok(Some(DetectorOutcome {
        standard: MonorepoStandard::BunWorkspaces,
        root: root.to_path_buf(),
        packages,
    }))
}

pub(super) fn detect_npm_workspace(root: &Path) -> Result<Option<DetectorOutcome>> {
    let package_json = root.join("package.json");
    if !package_json.exists() {
        return Ok(None);
    }

    // Bun reuses `package.json#workspaces`; when a Bun lockfile is present, the
    // Bun detector owns membership and npm must not also claim this root.
    if has_bun_lockfile(root) {
        return Ok(None);
    }

    let workspaces = parse_package_json_workspace_patterns(&package_json)?.unwrap_or_default();

    if workspaces.is_empty() {
        return Ok(None);
    }

    let lock_versions = None;
    let dialect = MonorepoStandard::NpmWorkspaces
        .glob_dialect()
        .unwrap_or(GlobDialect::Minimatch);
    let mut packages = expand_membership_globs(
        root,
        &workspaces,
        dialect,
        MonorepoStandard::NpmWorkspaces,
        None,
        &lock_versions,
    );
    packages = dedupe_packages(packages);
    resolve_internal_deps(&mut packages);

    Ok(Some(DetectorOutcome {
        standard: MonorepoStandard::NpmWorkspaces,
        root: root.to_path_buf(),
        packages,
    }))
}

pub(super) fn detect_yarn_workspace(root: &Path) -> Result<Option<DetectorOutcome>> {
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
    let dialect = MonorepoStandard::YarnWorkspaces
        .glob_dialect()
        .unwrap_or(GlobDialect::Minimatch);
    let mut packages = expand_membership_globs(
        root,
        &workspaces,
        dialect,
        MonorepoStandard::YarnWorkspaces,
        None,
        &lock_versions,
    );
    packages = dedupe_packages(packages);
    resolve_internal_deps(&mut packages);

    Ok(Some(DetectorOutcome {
        standard: MonorepoStandard::YarnWorkspaces,
        root: root.to_path_buf(),
        packages,
    }))
}

pub(super) fn detect_rush_workspace(root: &Path) -> Result<Option<DetectorOutcome>> {
    let rush_json = root.join("rush.json");
    if !rush_json.exists() {
        return Ok(None);
    }

    let content = std::fs::read_to_string(&rush_json)?;
    let folders = parse_rush_project_folders(&content);
    if folders.is_empty() {
        return Ok(None);
    }

    let lock_versions = None;
    let mut packages = Vec::new();
    for folder in folders {
        let member_path = root.join(&folder);
        if !member_path.exists() {
            continue;
        }
        packages.push(create_package(
            &member_path,
            root,
            MonorepoStandard::RushStack,
            PackageProvenance::Explicit,
            &lock_versions,
        ));
    }

    if packages.is_empty() {
        return Ok(None);
    }

    packages = dedupe_packages(packages);
    resolve_internal_deps(&mut packages);

    Ok(Some(DetectorOutcome {
        standard: MonorepoStandard::RushStack,
        root: root.to_path_buf(),
        packages,
    }))
}

/// Parse the `projectFolder` of each entry in `rush.json#projects`.
///
/// Rush's `projects` array lists `{ projectFolder, packageName }` objects whose
/// `projectFolder` is a repo-relative directory. Entries without a string
/// `projectFolder` are skipped.
fn parse_rush_project_folders(content: &str) -> Vec<String> {
    let Ok(parsed) = serde_json::from_str::<serde_json::Value>(content) else {
        return Vec::new();
    };
    parsed
        .get("projects")
        .and_then(|v| v.as_array())
        .map(|projects| {
            projects
                .iter()
                .filter_map(|project| {
                    project
                        .get("projectFolder")
                        .and_then(|v| v.as_str())
                        .map(String::from)
                })
                .collect()
        })
        .unwrap_or_default()
}

pub(super) fn parse_pnpm_workspace_patterns(pnpm_workspace_path: &Path) -> Result<Vec<String>> {
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

pub(super) fn parse_package_json_workspace_patterns(
    package_json_path: &Path,
) -> Result<Option<Vec<String>>> {
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

pub(super) fn resolve_js_package_manager(
    standard: MonorepoStandard,
    root: &Path,
    package_managers: &[String],
) -> &'static str {
    match standard {
        MonorepoStandard::PnpmWorkspaces => return "pnpm",
        MonorepoStandard::YarnWorkspaces => return "yarn",
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_rush_projects_reads_project_folders() {
        let content = r#"{
            "projects": [
                { "packageName": "@scope/app", "projectFolder": "apps/app" },
                { "packageName": "@scope/lib", "projectFolder": "libraries/lib" }
            ]
        }"#;
        assert_eq!(
            parse_rush_project_folders(content),
            vec!["apps/app".to_string(), "libraries/lib".to_string()]
        );
    }

    #[test]
    fn parse_rush_projects_empty_when_absent() {
        assert!(parse_rush_project_folders(r#"{"rushVersion": "5.0.0"}"#).is_empty());
    }
}
