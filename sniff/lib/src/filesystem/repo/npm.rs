//! npm/pnpm/yarn workspace detection and package.json parsing helpers.

use std::path::Path;

use biscuit_file::serde_yaml_ng;
use serde::Deserialize;
use serde::de::IgnoredAny;

use crate::Result;
use crate::package::{DependencyEntry, DependencyKind};

use super::detection::{DetectorOutcome, ManifestStore, RepoEvidence, probe_exists};
use super::glob::expand_membership_globs;
use super::seed::{PackageSeed, merge_seeds};
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

pub(super) fn detect_pnpm_workspace(
    root: &Path,
    evidence: RepoEvidence<'_>,
    manifests: &ManifestStore,
) -> Result<Option<DetectorOutcome>> {
    let pnpm_workspace = root.join("pnpm-workspace.yaml");
    if !probe_exists(&pnpm_workspace) {
        return Ok(None);
    }

    let parsed = manifests.required_pnpm_workspace(&pnpm_workspace)?;
    let packages = pnpm_workspace_patterns_from_value(&parsed);

    if packages.is_empty() {
        return Ok(None);
    }

    let dialect = MonorepoStandard::PnpmWorkspaces
        .glob_dialect()
        .unwrap_or(GlobDialect::Minimatch);
    let package_locations = expand_membership_globs(
        root,
        &packages,
        dialect,
        MonorepoStandard::PnpmWorkspaces,
        None,
        evidence,
    );

    Ok(Some(DetectorOutcome {
        standard: MonorepoStandard::PnpmWorkspaces,
        root: root.to_path_buf(),
        seeds: merge_seeds(package_locations),
    }))
}

/// Whether a Bun lockfile (`bun.lock` or `bun.lockb`) is present at `root`.
///
/// Bun and npm/yarn all declare members via `package.json#workspaces`; the
/// lockfile is what disambiguates Bun so it wins the membership authority.
///
/// Probed through the store, so the Bun layer's lockfile observation reuses
/// these answers. A failed probe reads as absent here, as `Path::exists` did.
fn has_bun_lockfile(root: &Path, manifests: &ManifestStore) -> bool {
    manifests
        .lockfile_presence(&root.join("bun.lock"))
        .is_present()
        || manifests
            .lockfile_presence(&root.join("bun.lockb"))
            .is_present()
}

pub(super) fn detect_bun_workspace(
    root: &Path,
    evidence: RepoEvidence<'_>,
    manifests: &ManifestStore,
) -> Result<Option<DetectorOutcome>> {
    if !has_bun_lockfile(root, manifests) {
        return Ok(None);
    }

    let package_json = root.join("package.json");
    if !probe_exists(&package_json) {
        return Ok(None);
    }

    let parsed = manifests.required_npm(&package_json)?;
    let workspaces = package_json_workspace_patterns_from_value(&parsed).unwrap_or_default();

    if workspaces.is_empty() {
        return Ok(None);
    }

    let dialect = MonorepoStandard::BunWorkspaces
        .glob_dialect()
        .unwrap_or(GlobDialect::Minimatch);
    let packages = expand_membership_globs(
        root,
        &workspaces,
        dialect,
        MonorepoStandard::BunWorkspaces,
        None,
        evidence,
    );

    Ok(Some(DetectorOutcome {
        standard: MonorepoStandard::BunWorkspaces,
        root: root.to_path_buf(),
        seeds: merge_seeds(packages),
    }))
}

pub(super) fn detect_npm_workspace(
    root: &Path,
    evidence: RepoEvidence<'_>,
    manifests: &ManifestStore,
) -> Result<Option<DetectorOutcome>> {
    let package_json = root.join("package.json");
    if !probe_exists(&package_json) {
        return Ok(None);
    }

    // Bun reuses `package.json#workspaces`; when a Bun lockfile is present, the
    // Bun detector owns membership and npm must not also claim this root.
    if has_bun_lockfile(root, manifests) {
        return Ok(None);
    }

    let parsed = manifests.required_npm(&package_json)?;
    let workspaces = package_json_workspace_patterns_from_value(&parsed).unwrap_or_default();

    if workspaces.is_empty() {
        return Ok(None);
    }

    let dialect = MonorepoStandard::NpmWorkspaces
        .glob_dialect()
        .unwrap_or(GlobDialect::Minimatch);
    let packages = expand_membership_globs(
        root,
        &workspaces,
        dialect,
        MonorepoStandard::NpmWorkspaces,
        None,
        evidence,
    );

    Ok(Some(DetectorOutcome {
        standard: MonorepoStandard::NpmWorkspaces,
        root: root.to_path_buf(),
        seeds: merge_seeds(packages),
    }))
}

pub(super) fn detect_yarn_workspace(
    root: &Path,
    evidence: RepoEvidence<'_>,
    manifests: &ManifestStore,
) -> Result<Option<DetectorOutcome>> {
    if !manifests
        .lockfile_presence(&root.join("yarn.lock"))
        .is_present()
    {
        return Ok(None);
    }

    let package_json = root.join("package.json");
    if !probe_exists(&package_json) {
        return Ok(None);
    }

    let parsed = manifests.required_npm(&package_json)?;
    let workspaces = package_json_workspace_patterns_from_value(&parsed).unwrap_or_default();

    if workspaces.is_empty() {
        return Ok(None);
    }

    let dialect = MonorepoStandard::YarnWorkspaces
        .glob_dialect()
        .unwrap_or(GlobDialect::Minimatch);
    let packages = expand_membership_globs(
        root,
        &workspaces,
        dialect,
        MonorepoStandard::YarnWorkspaces,
        None,
        evidence,
    );

    Ok(Some(DetectorOutcome {
        standard: MonorepoStandard::YarnWorkspaces,
        root: root.to_path_buf(),
        seeds: merge_seeds(packages),
    }))
}

pub(super) fn detect_rush_workspace(
    root: &Path,
    manifests: &ManifestStore,
) -> Result<Option<DetectorOutcome>> {
    let rush_json = root.join("rush.json");
    if !probe_exists(&rush_json) {
        return Ok(None);
    }
    let Some(config) = manifests.rush_json(&rush_json)? else {
        return Ok(None);
    };
    let folders = config.project_folders();
    if folders.is_empty() {
        return Ok(None);
    }

    let mut seeds = Vec::new();
    for folder in folders {
        let member_path = root.join(folder);
        if !probe_exists(&member_path) {
            continue;
        }
        seeds.push(PackageSeed::new(
            &member_path,
            root,
            MonorepoStandard::RushStack,
            PackageProvenance::Explicit,
        ));
    }

    if seeds.is_empty() {
        return Ok(None);
    }

    Ok(Some(DetectorOutcome {
        standard: MonorepoStandard::RushStack,
        root: root.to_path_buf(),
        seeds: merge_seeds(seeds),
    }))
}

/// The parts of `rush.json` Sniff reads: the projects for membership, and
/// the package manager and install settings that classify the lockfile
/// layout (ruling R3 of `2026-09-26-lockfile-corroboration`).
///
/// Real `rush.json` files are JSON with comments. A field of an unexpected
/// type is kept as [`Lenient::Other`] rather than failing the document, so an
/// odd setting never hides the projects.
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RushJson {
    #[serde(default)]
    projects: Vec<Lenient<RushProject>>,
    pnpm_version: Option<IgnoredAny>,
    npm_version: Option<IgnoredAny>,
    yarn_version: Option<IgnoredAny>,
    pnpm_options: Option<Lenient<RushPnpmOptions>>,
    variants: Option<Lenient<Vec<IgnoredAny>>>,
}

/// The package manager `rush.json` selects.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RushManager {
    Pnpm,
    Npm,
    Yarn,
}

impl RushJson {
    /// The `projectFolder` of each project; an entry without a string
    /// `projectFolder` is skipped.
    fn project_folders(&self) -> Vec<&str> {
        self.projects
            .iter()
            .filter_map(|project| match project {
                Lenient::Value(RushProject {
                    project_folder: Some(Lenient::Value(folder)),
                }) => Some(folder.as_str()),
                _ => None,
            })
            .collect()
    }

    /// The manager, when exactly one of `pnpmVersion`, `npmVersion`, and
    /// `yarnVersion` is set.
    pub(crate) fn manager(&self) -> Option<RushManager> {
        match (
            self.pnpm_version.is_some(),
            self.npm_version.is_some(),
            self.yarn_version.is_some(),
        ) {
            (true, false, false) => Some(RushManager::Pnpm),
            (false, true, false) => Some(RushManager::Npm),
            (false, false, true) => Some(RushManager::Yarn),
            _ => None,
        }
    }

    /// The legacy `pnpmOptions.useWorkspaces`, for repositories without a
    /// `pnpm-config.json`.
    pub(crate) fn legacy_use_workspaces(&self) -> Option<bool> {
        match &self.pnpm_options {
            Some(Lenient::Value(options)) => options.use_workspaces.as_ref()?.value().copied(),
            _ => None,
        }
    }

    /// Whether `rush.json` declares installation variants, or declares them
    /// in a shape Sniff cannot read.
    pub(crate) fn declares_variants(&self) -> bool {
        match &self.variants {
            None => false,
            Some(Lenient::Value(variants)) => !variants.is_empty(),
            Some(Lenient::Other(_)) => true,
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RushProject {
    project_folder: Option<Lenient<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RushPnpmOptions {
    use_workspaces: Option<Lenient<bool>>,
}

/// A value of the expected type, or anything else.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub(crate) enum Lenient<T> {
    Value(T),
    Other(IgnoredAny),
}

impl<T> Lenient<T> {
    pub(crate) fn value(&self) -> Option<&T> {
        match self {
            Self::Value(value) => Some(value),
            Self::Other(_) => None,
        }
    }
}

/// Extract the `packages:` sequence from a parsed `pnpm-workspace.yaml`.
///
/// Returns an empty vector when the field is absent, so callers treat a
/// missing or empty `packages` list as "not a pnpm workspace".
pub(super) fn pnpm_workspace_patterns_from_value(parsed: &serde_yaml_ng::Value) -> Vec<String> {
    parsed
        .get("packages")
        .and_then(|p| p.as_sequence())
        .map(|seq| {
            seq.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

/// Extract the workspace patterns from a parsed `package.json`.
///
/// Returns `None` when the manifest declares no `workspaces` field, so callers
/// can distinguish "not a workspace root" from an empty pattern list.
pub(super) fn package_json_workspace_patterns_from_value(
    parsed: &serde_json::Value,
) -> Option<Vec<String>> {
    let workspaces = parsed.get("workspaces")?;

    if let Some(arr) = workspaces.as_array() {
        return Some(
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect(),
        );
    }

    if let Some(obj) = workspaces.as_object() {
        return Some(
            obj.get("packages")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default(),
        );
    }

    Some(Vec::new())
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
        || probe_exists(&root.join("pnpm-lock.yaml"))
    {
        return "pnpm";
    }
    if package_managers.iter().any(|manager| manager == "yarn")
        || probe_exists(&root.join("yarn.lock"))
    {
        return "yarn";
    }
    if probe_exists(&root.join("bun.lock")) || probe_exists(&root.join("bun.lockb")) {
        return "bun";
    }

    "npm"
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rush_json(content: &str) -> RushJson {
        crate::filesystem::repo::jsonc::from_str(content).expect("valid rush.json")
    }

    #[test]
    fn rush_json_reads_project_folders() {
        let content = r#"{
            "projects": [
                { "packageName": "@scope/app", "projectFolder": "apps/app" },
                { "packageName": "@scope/lib", "projectFolder": "libraries/lib" }
            ]
        }"#;
        assert_eq!(
            rush_json(content).project_folders(),
            vec!["apps/app", "libraries/lib"]
        );
    }

    #[test]
    fn rush_json_without_projects_has_no_folders() {
        assert!(rush_json(r#"{"rushVersion": "5.0.0"}"#).project_folders().is_empty());
    }

    #[test]
    fn rush_json_skips_projects_without_a_string_folder() {
        let content = r#"{"projects": [
            {"packageName": "a"},
            {"projectFolder": 3},
            "not a project",
            {"projectFolder": "apps/b"}
        ], "pnpmOptions": 7, "variants": {}}"#;
        let config = rush_json(content);
        assert_eq!(config.project_folders(), vec!["apps/b"]);
        assert_eq!(config.legacy_use_workspaces(), None);
        assert!(config.declares_variants(), "an unreadable variants value");
    }

    /// Regression: `rush init` writes comments throughout `rush.json`, and the
    /// former strict JSON parse found no projects in any real Rush repository.
    #[test]
    fn the_real_rush_json_is_read_through_its_comments() {
        let content = include_str!(
            "../../../tests/fixtures/lockfiles/rush-5.179.0/pnpm-workspace/rush.json"
        );
        assert!(serde_json::from_str::<serde_json::Value>(content).is_err());
        let config = rush_json(content);
        assert_eq!(
            config.project_folders(),
            vec!["packages/alpha", "packages/beta", ".tools/hidden"]
        );
        assert_eq!(config.manager(), Some(RushManager::Pnpm));
        assert!(!config.declares_variants());
    }

    #[test]
    fn exactly_one_version_field_selects_the_manager() {
        for (content, expected) in [
            (r#"{"pnpmVersion": "9.0.0"}"#, Some(RushManager::Pnpm)),
            (r#"{"npmVersion": "6.0.0"}"#, Some(RushManager::Npm)),
            (r#"{"yarnVersion": "1.22.0"}"#, Some(RushManager::Yarn)),
            (r#"{}"#, None),
            (r#"{"pnpmVersion": "9.0.0", "npmVersion": "6.0.0"}"#, None),
        ] {
            assert_eq!(rush_json(content).manager(), expected, "{content}");
        }
        assert_eq!(
            rush_json(r#"{"pnpmOptions": {"useWorkspaces": true}}"#).legacy_use_workspaces(),
            Some(true)
        );
    }
}
