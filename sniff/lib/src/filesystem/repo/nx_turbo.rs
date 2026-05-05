//! Nx, Turborepo, and Lerna monorepo detection.

use std::path::Path;

use tracing::debug;

use crate::Result;

use super::detection::{
    collect_default_workspace_patterns, dedupe_packages, dedupe_patterns,
    discover_packages_with_optional_index, expand_glob_patterns_with_deps, resolve_internal_deps,
};
use super::manifest_index::ManifestIndex;
use super::types::{MonorepoTool, PackageDiscoverySource, RepoInfo};

pub(super) fn detect_nx(
    root: &Path,
    index: Option<&ManifestIndex>,
) -> Result<Option<RepoInfo>> {
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

pub(super) fn detect_turborepo(
    root: &Path,
    index: Option<&ManifestIndex>,
) -> Result<Option<RepoInfo>> {
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

pub(super) fn detect_lerna(
    root: &Path,
    index: Option<&ManifestIndex>,
) -> Result<Option<RepoInfo>> {
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
