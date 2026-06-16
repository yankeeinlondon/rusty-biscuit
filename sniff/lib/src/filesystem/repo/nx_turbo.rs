//! Nx, Turborepo, and Lerna monorepo detection.

use std::path::Path;

use tracing::debug;

use crate::Result;

use super::detection::{
    DetectorOutcome, collect_default_workspace_patterns, dedupe_packages, dedupe_patterns,
    discover_packages_with_optional_index, resolve_internal_deps,
};
use super::glob::expand_membership_globs;
use super::manifest_index::ManifestIndex;
use super::standard::{GlobDialect, MonorepoStandard, PackageProvenance};

pub(super) fn detect_nx(
    root: &Path,
    index: Option<&ManifestIndex>,
) -> Result<Option<DetectorOutcome>> {
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
            MonorepoStandard::Nx,
            PackageProvenance::LeafMarkers,
            &lock_versions,
            index,
        )
    } else {
        let dialect = MonorepoStandard::Nx
            .glob_dialect()
            .unwrap_or(GlobDialect::Minimatch);
        expand_membership_globs(
            root,
            &patterns,
            dialect,
            MonorepoStandard::Nx,
            None,
            &lock_versions,
        )
    };
    if packages.is_empty() {
        packages = discover_packages_with_optional_index(
            root,
            MonorepoStandard::Nx,
            PackageProvenance::LeafMarkers,
            &lock_versions,
            index,
        );
    }
    packages = dedupe_packages(packages);
    resolve_internal_deps(&mut packages);

    Ok(Some(DetectorOutcome {
        standard: MonorepoStandard::Nx,
        root: root.to_path_buf(),
        packages,
    }))
}

pub(super) fn detect_turborepo(
    root: &Path,
    index: Option<&ManifestIndex>,
) -> Result<Option<DetectorOutcome>> {
    let turbo_json = root.join("turbo.json");
    if !turbo_json.exists() {
        return Ok(None);
    }

    let patterns = collect_default_workspace_patterns(root);
    let lock_versions = None;
    let mut packages = if patterns.is_empty() {
        discover_packages_with_optional_index(
            root,
            MonorepoStandard::Turborepo,
            PackageProvenance::Globbed,
            &lock_versions,
            index,
        )
    } else {
        let dialect = MonorepoStandard::Turborepo
            .glob_dialect()
            .unwrap_or(GlobDialect::Minimatch);
        expand_membership_globs(
            root,
            &patterns,
            dialect,
            MonorepoStandard::Turborepo,
            None,
            &lock_versions,
        )
    };
    if packages.is_empty() {
        packages = discover_packages_with_optional_index(
            root,
            MonorepoStandard::Turborepo,
            PackageProvenance::Globbed,
            &lock_versions,
            index,
        );
    }
    packages = dedupe_packages(packages);
    resolve_internal_deps(&mut packages);

    Ok(Some(DetectorOutcome {
        standard: MonorepoStandard::Turborepo,
        root: root.to_path_buf(),
        packages,
    }))
}

pub(super) fn detect_lerna(
    root: &Path,
    index: Option<&ManifestIndex>,
) -> Result<Option<DetectorOutcome>> {
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
            MonorepoStandard::Lerna,
            PackageProvenance::Globbed,
            &lock_versions,
            index,
        )
    } else {
        let dialect = MonorepoStandard::Lerna
            .glob_dialect()
            .unwrap_or(GlobDialect::Minimatch);
        expand_membership_globs(
            root,
            &patterns,
            dialect,
            MonorepoStandard::Lerna,
            None,
            &lock_versions,
        )
    };
    if packages.is_empty() {
        packages = discover_packages_with_optional_index(
            root,
            MonorepoStandard::Lerna,
            PackageProvenance::Globbed,
            &lock_versions,
            index,
        );
    }
    packages = dedupe_packages(packages);
    resolve_internal_deps(&mut packages);

    Ok(Some(DetectorOutcome {
        standard: MonorepoStandard::Lerna,
        root: root.to_path_buf(),
        packages,
    }))
}

pub(super) fn parse_lerna_workspace_patterns(lerna_json_path: &Path) -> Option<Vec<String>> {
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

pub(super) fn parse_nx_layout_patterns(nx_json_path: &Path) -> Vec<String> {
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
