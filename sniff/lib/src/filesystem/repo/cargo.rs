//! Cargo workspace detection and Cargo.toml parsing helpers.

use std::path::Path;

use biscuit_file::toml_crate;

use crate::package::{DependencyEntry, DependencyKind};
use crate::{Result, SniffError};

use super::detection::{DetectorOutcome, resolve_internal_deps};
use super::glob::expand_membership_globs;
use super::manifest_index::CargoLockVersions;
use super::standard::{GlobDialect, MonorepoStandard};

pub(super) fn detect_cargo_workspace(root: &Path) -> Result<Option<DetectorOutcome>> {
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

    let dialect = MonorepoStandard::CargoWorkspace
        .glob_dialect()
        .unwrap_or(GlobDialect::Cargo);

    // Expand globs and collect packages with dependencies
    let mut packages = expand_membership_globs(
        root,
        &members,
        dialect,
        MonorepoStandard::CargoWorkspace,
        None,
        &lock_versions,
    );

    // Expand excluded patterns and mark them
    let mut excluded_packages = expand_membership_globs(
        root,
        &excludes,
        dialect,
        MonorepoStandard::CargoWorkspace,
        None,
        &lock_versions,
    );
    for pkg in &mut excluded_packages {
        pkg.is_excluded = true;
    }
    packages.extend(excluded_packages);

    // Resolve internal dependency graph
    resolve_internal_deps(&mut packages);

    Ok(Some(DetectorOutcome {
        standard: MonorepoStandard::CargoWorkspace,
        root: root.to_path_buf(),
        packages,
    }))
}

/// Parses Cargo.toml dependencies from an already-parsed TOML value.
pub(super) fn cargo_dependencies_from_value(
    parsed: &toml_crate::Value,
    lock_versions: &Option<CargoLockVersions>,
) -> (
    Vec<DependencyEntry>,
    Vec<DependencyEntry>,
    Vec<DependencyEntry>,
) {
    let normal_deps = parse_cargo_dep_section(
        parsed,
        "dependencies",
        DependencyKind::Normal,
        lock_versions,
    );
    let dev_deps = parse_cargo_dep_section(
        parsed,
        "dev-dependencies",
        DependencyKind::Dev,
        lock_versions,
    );
    let build_deps = parse_cargo_dep_section(
        parsed,
        "build-dependencies",
        DependencyKind::Build,
        lock_versions,
    );

    (normal_deps, dev_deps, build_deps)
}

/// Parses a single dependencies section from Cargo.toml.
pub(super) fn parse_cargo_dep_section(
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

/// Extracts the package name from a parsed Cargo.toml value.
pub(super) fn cargo_package_name(parsed: &toml_crate::Value) -> Option<String> {
    parsed
        .get("package")
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
        .map(String::from)
}

/// Extracts the package version from a parsed Cargo.toml value.
pub(super) fn cargo_package_version(parsed: &toml_crate::Value) -> Option<String> {
    parsed
        .get("package")
        .and_then(|p| p.get("version"))
        .and_then(|v| v.as_str())
        .map(String::from)
}

/// Extracts the feature-flag names from a parsed Cargo.toml `[features]` section.
pub(super) fn cargo_features_from_value(parsed: &toml_crate::Value) -> Vec<String> {
    let Some(features) = parsed.get("features").and_then(|f| f.as_table()) else {
        return Vec::new();
    };
    let mut names: Vec<String> = features.keys().cloned().collect();
    names.sort();
    names
}
