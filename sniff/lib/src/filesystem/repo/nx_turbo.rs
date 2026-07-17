//! Nx, Turborepo, and Lerna monorepo detection.

use std::path::Path;

use tracing::debug;

use crate::Result;
use crate::performance;
use crate::performance::counters;

use super::detection::{
    DetectorOutcome, ManifestStore, RepoEvidence, collect_default_workspace_patterns,
    dedupe_patterns, discover_seeds_with_optional_index, probe_exists,
};
use super::glob::expand_membership_globs;
use super::seed::merge_seeds;
use super::standard::{GlobDialect, MonorepoStandard, PackageProvenance};

pub(super) fn detect_nx(
    root: &Path,
    evidence: RepoEvidence<'_>,
    manifests: &ManifestStore,
) -> Result<Option<DetectorOutcome>> {
    let nx_json = root.join("nx.json");
    if !probe_exists(&nx_json) {
        return Ok(None);
    }

    let mut patterns = collect_default_workspace_patterns(root, manifests);
    patterns.extend(parse_nx_layout_patterns(&nx_json));
    patterns = dedupe_patterns(patterns);

    let mut seeds = if patterns.is_empty() {
        discover_seeds_with_optional_index(
            root,
            MonorepoStandard::Nx,
            PackageProvenance::LeafMarkers,
            evidence.manifest_index,
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
            evidence,
        )
    };
    if seeds.is_empty() {
        seeds = discover_seeds_with_optional_index(
            root,
            MonorepoStandard::Nx,
            PackageProvenance::LeafMarkers,
            evidence.manifest_index,
        );
    }

    Ok(Some(DetectorOutcome {
        standard: MonorepoStandard::Nx,
        root: root.to_path_buf(),
        seeds: merge_seeds(seeds),
    }))
}

pub(super) fn detect_turborepo(
    root: &Path,
    evidence: RepoEvidence<'_>,
    manifests: &ManifestStore,
) -> Result<Option<DetectorOutcome>> {
    let turbo_json = root.join("turbo.json");
    if !probe_exists(&turbo_json) {
        return Ok(None);
    }

    let patterns = collect_default_workspace_patterns(root, manifests);
    let mut seeds = if patterns.is_empty() {
        discover_seeds_with_optional_index(
            root,
            MonorepoStandard::Turborepo,
            PackageProvenance::Globbed,
            evidence.manifest_index,
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
            evidence,
        )
    };
    if seeds.is_empty() {
        seeds = discover_seeds_with_optional_index(
            root,
            MonorepoStandard::Turborepo,
            PackageProvenance::Globbed,
            evidence.manifest_index,
        );
    }

    Ok(Some(DetectorOutcome {
        standard: MonorepoStandard::Turborepo,
        root: root.to_path_buf(),
        seeds: merge_seeds(seeds),
    }))
}

pub(super) fn detect_lerna(
    root: &Path,
    evidence: RepoEvidence<'_>,
    manifests: &ManifestStore,
) -> Result<Option<DetectorOutcome>> {
    let lerna_json = root.join("lerna.json");
    if !probe_exists(&lerna_json) {
        return Ok(None);
    }

    let mut patterns = parse_lerna_workspace_patterns(&lerna_json).unwrap_or_default();
    patterns.extend(collect_default_workspace_patterns(root, manifests));
    patterns = dedupe_patterns(patterns);

    let mut seeds = if patterns.is_empty() {
        discover_seeds_with_optional_index(
            root,
            MonorepoStandard::Lerna,
            PackageProvenance::Globbed,
            evidence.manifest_index,
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
            evidence,
        )
    };
    if seeds.is_empty() {
        seeds = discover_seeds_with_optional_index(
            root,
            MonorepoStandard::Lerna,
            PackageProvenance::Globbed,
            evidence.manifest_index,
        );
    }

    Ok(Some(DetectorOutcome {
        standard: MonorepoStandard::Lerna,
        root: root.to_path_buf(),
        seeds: merge_seeds(seeds),
    }))
}

pub(super) fn parse_lerna_workspace_patterns(lerna_json_path: &Path) -> Option<Vec<String>> {
    performance::increment_counter(counters::FS_FILE_OPENS, 1);
    let content = std::fs::read_to_string(lerna_json_path)
        .map_err(|e| {
            debug!(path = %lerna_json_path.display(), error = %e, "could not read file");
            e
        })
        .ok()?;
    performance::increment_counter(counters::FS_BYTES_READ, content.len() as u64);
    performance::increment_counter(counters::REPO_CONFIG_PARSES, 1);
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
    performance::increment_counter(counters::FS_FILE_OPENS, 1);
    let content = match std::fs::read_to_string(nx_json_path) {
        Ok(content) => content,
        Err(_) => return Vec::new(),
    };
    performance::increment_counter(counters::FS_BYTES_READ, content.len() as u64);
    performance::increment_counter(counters::REPO_CONFIG_PARSES, 1);
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
