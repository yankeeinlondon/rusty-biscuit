//! Cargo workspace detection and Cargo.toml parsing helpers.

use std::path::Path;

use biscuit_file::toml_crate;

use crate::Result;
use crate::package::{DependencyEntry, DependencyKind};
use crate::performance;
use crate::performance::counters;

use super::detection::{DetectorOutcome, ManifestStore, RepoEvidence, probe_exists};
use super::glob::{DeclaredPatterns, expand_membership_globs};
use super::manifest_index::CargoLockVersions;
use super::standard::{GlobDialect, MonorepoStandard};

pub(super) fn detect_cargo_workspace(
    root: &Path,
    evidence: RepoEvidence<'_>,
    manifests: &ManifestStore,
) -> Result<Option<DetectorOutcome>> {
    let cargo_toml = root.join("Cargo.toml");
    if !probe_exists(&cargo_toml) {
        return Ok(None);
    }

    let parsed = manifests.required_cargo(&cargo_toml)?;

    let workspace = match parsed.get("workspace") {
        Some(w) => w,
        None => return Ok(None),
    };

    let members = declared_patterns(workspace, "members");
    if members.is_empty() {
        return Ok(None);
    }
    let excludes = declared_patterns(workspace, "exclude");

    let dialect = MonorepoStandard::CargoWorkspace
        .glob_dialect()
        .unwrap_or(GlobDialect::Cargo);

    let expansion = expand_membership_globs(
        root,
        &members.patterns,
        dialect,
        MonorepoStandard::CargoWorkspace,
        None,
        evidence,
    );

    // Cargo rejects a missing literal member, so it leaves the set incomplete;
    // excluding a missing path is harmless.
    let mut incomplete =
        !expansion.patterns_resolved || expansion.missing_literal || members.has_invalid;
    let mut seeds = expansion.seeds;

    // Expand excluded patterns and mark them. A directory matched by both an
    // include and an exclude pattern merges to one excluded seed.
    let excluded = expand_membership_globs(
        root,
        &excludes.patterns,
        dialect,
        MonorepoStandard::CargoWorkspace,
        None,
        evidence,
    );
    incomplete |= !excluded.patterns_resolved || excludes.has_invalid;
    let mut excluded_seeds = excluded.seeds;
    for seed in &mut excluded_seeds {
        seed.is_excluded = true;
    }
    seeds.extend(excluded_seeds);

    Ok(Some(DetectorOutcome {
        standard: MonorepoStandard::CargoWorkspace,
        root: root.to_path_buf(),
        seeds,
        incomplete,
    }))
}

/// The `[workspace]` array at `key`; absent reads as empty, and a value that
/// is not an array is an invalid declaration.
fn declared_patterns(workspace: &toml_crate::Value, key: &str) -> DeclaredPatterns {
    match workspace.get(key) {
        None => DeclaredPatterns::default(),
        Some(value) => match value.as_array() {
            Some(arr) => arr.iter().map(|v| v.as_str()).collect(),
            None => DeclaredPatterns::invalid(),
        },
    }
}

/// Parses Cargo.toml dependencies from an already-parsed TOML value.
pub(super) fn cargo_dependencies_from_value(
    parsed: &toml_crate::Value,
    lock_versions: Option<&CargoLockVersions>,
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
    lock_versions: Option<&CargoLockVersions>,
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

            let actual_version = lock_versions.and_then(|versions| versions.resolve(name));

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
pub(crate) fn cargo_package_name(parsed: &toml_crate::Value) -> Option<String> {
    parsed
        .get("package")
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
        .map(String::from)
}

/// Extracts the package version from a parsed Cargo.toml value.
///
/// Returns the literal `[package].version` string when present. Workspace
/// inheritance (`version = { workspace = true }`) is **not** resolved here;
/// callers that need the inherited value should use
/// [`cargo_package_version_with_source`] instead.
pub(crate) fn cargo_package_version(parsed: &toml_crate::Value) -> Option<String> {
    parsed
        .get("package")
        .and_then(|p| p.get("version"))
        .and_then(|v| v.as_str())
        .map(String::from)
}

/// Resolve a Cargo package version, including workspace inheritance.
///
/// Returns `(version, manifest_path, inherited)` where `manifest_path` is
/// the manifest whose `[package].version` (or `[workspace.package].version`)
/// produced the value (already a string, or `workspace = true` resolving to
/// the workspace root), and `inherited` is `true` only when the value came
/// from the root `[workspace.package].version` because the package
/// declared `version = { workspace = true }`.
///
/// A `version.workspace = true` with no root `[workspace.package].version`
/// returns `None` with `inherited: false` — the package inherits nothing.
/// The implementation never shells out to Cargo; both the package and
/// workspace manifests are parsed with the existing TOML stack.
pub(crate) fn cargo_package_version_with_source(
    parsed: &toml_crate::Value,
    package_manifest: &Path,
    repo_root: &Path,
) -> Option<(String, String, bool)> {
    if let Some(version) = cargo_package_version(parsed) {
        return Some((
            version,
            repo_relative_manifest_path(package_manifest, repo_root),
            false,
        ));
    }
    let root_manifest = repo_root.join("Cargo.toml");
    let root_manifest = if probe_exists(&root_manifest) {
        root_manifest
    } else {
        package_manifest.to_path_buf()
    };
    let root_parsed = read_toml_at(&root_manifest);
    cargo_package_version_with_root(
        parsed,
        package_manifest,
        repo_root,
        &root_manifest,
        root_parsed.as_ref(),
    )
}

/// Resolve a Cargo package version from caller-supplied workspace-root data.
///
/// This is the request-scoped counterpart to
/// [`cargo_package_version_with_source`]: repository detection obtains the root
/// value from its [`ManifestStore`] so every inheriting member shares one parse.
pub(crate) fn cargo_package_version_with_root(
    parsed: &toml_crate::Value,
    package_manifest: &Path,
    repo_root: &Path,
    root_manifest: &Path,
    root_parsed: Option<&toml_crate::Value>,
) -> Option<(String, String, bool)> {
    let version = parsed.get("package").and_then(|p| p.get("version"))?;
    if let Some(s) = version.as_str() {
        return Some((
            s.to_string(),
            repo_relative_manifest_path(package_manifest, repo_root),
            false,
        ));
    }
    let workspace_inherits = version
        .as_table()
        .and_then(|t| t.get("workspace"))
        .and_then(|w| w.as_bool())
        .unwrap_or(false);
    if !workspace_inherits {
        return None;
    }
    let root_parsed = if root_manifest == package_manifest {
        root_parsed.unwrap_or(parsed)
    } else {
        root_parsed?
    };
    let inherited = root_parsed
        .get("workspace")
        .and_then(|w| w.get("package"))
        .and_then(|p| p.get("version"))
        .and_then(|v| v.as_str())
        .map(String::from)?;
    Some((
        inherited,
        repo_relative_manifest_path(root_manifest, repo_root),
        true,
    ))
}

/// Read and parse a TOML manifest at `path`, returning `None` on any I/O or
/// parse failure. Used by standalone aggregation; repository detection routes
/// inherited workspace manifests through its request-scoped `ManifestStore`.
fn read_toml_at(path: &Path) -> Option<toml_crate::Value> {
    performance::increment_counter(counters::FS_FILE_OPENS, 1);
    let content = std::fs::read_to_string(path).ok()?;
    performance::increment_counter(counters::FS_BYTES_READ, content.len() as u64);
    performance::increment_counter(counters::REPO_MANIFEST_PARSES, 1);
    toml_crate::from_str(&content).ok()
}

/// Convert an absolute manifest path into the repo-relative string used by
/// `VersionSource.path`. Returns the original string form when `path` is not
/// under `repo_root` so callers always get a non-empty value.
fn repo_relative_manifest_path(path: &Path, repo_root: &Path) -> String {
    path.strip_prefix(repo_root)
        .ok()
        .and_then(|p| p.to_str())
        .map(|s| s.replace('\\', "/"))
        .unwrap_or_else(|| path.to_str().map(|s| s.to_string()).unwrap_or_default())
}

/// Extracts the feature-flag names from a parsed Cargo.toml `[features]` section.
pub(crate) fn cargo_features_from_value(parsed: &toml_crate::Value) -> Vec<String> {
    let Some(features) = parsed.get("features").and_then(|f| f.as_table()) else {
        return Vec::new();
    };
    let mut names: Vec<String> = features.keys().cloned().collect();
    names.sort();
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    fn workspace(source: &str) -> toml_crate::Value {
        let parsed: toml_crate::Value = toml_crate::from_str(source).unwrap();
        parsed["workspace"].clone()
    }

    #[test]
    fn an_absent_key_declares_nothing() {
        let workspace = workspace("[workspace]\nresolver = \"2\"\n");

        assert!(declared_patterns(&workspace, "members").is_empty());
        assert!(declared_patterns(&workspace, "exclude").is_empty());
    }

    #[test]
    fn a_non_array_value_is_recorded_as_invalid_rather_than_absent() {
        for key in ["members", "exclude"] {
            for value in ["\"crates/*\"", "123", "{ a = 1 }"] {
                let workspace = workspace(&format!("[workspace]\n{key} = {value}\n"));

                assert_eq!(
                    declared_patterns(&workspace, key),
                    DeclaredPatterns::invalid(),
                    "{key} = {value}"
                );
            }
        }
    }
}
