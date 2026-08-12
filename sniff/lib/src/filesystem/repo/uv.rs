//! uv workspace (`pyproject.toml` `[tool.uv.workspace]`) detection.

use std::path::Path;

use biscuit_file::toml_crate;

use crate::Result;

use super::detection::{DetectorOutcome, ManifestStore, RepoEvidence, probe_exists};
use super::glob::expand_membership_globs;
use super::seed::{PackageSeed, merge_seeds};
use super::standard::{GlobDialect, MonorepoStandard, PackageProvenance};

pub(super) fn detect_uv_workspace(
    root: &Path,
    evidence: RepoEvidence<'_>,
    manifests: &ManifestStore,
) -> Result<Option<DetectorOutcome>> {
    let pyproject = root.join("pyproject.toml");
    if !probe_exists(&pyproject) {
        return Ok(None);
    }

    let parsed = manifests.required_pyproject(&pyproject)?;
    let members = uv_workspace_members_from_value(&parsed);
    if members.is_empty() {
        return Ok(None);
    }

    let dialect = MonorepoStandard::UvWorkspace
        .glob_dialect()
        .unwrap_or(GlobDialect::Minimatch);
    let mut seeds = expand_membership_globs(
        root,
        &members,
        dialect,
        MonorepoStandard::UvWorkspace,
        None,
        evidence,
    );

    // uv's `RootMembership::Always`: the root `[project]` is itself a workspace
    // member, so the root directory is counted alongside the globbed children.
    seeds.push(PackageSeed::new(
        root,
        root,
        MonorepoStandard::UvWorkspace,
        PackageProvenance::Globbed,
    ));


    Ok(Some(DetectorOutcome {
        standard: MonorepoStandard::UvWorkspace,
        root: root.to_path_buf(),
        seeds: merge_seeds(seeds),
    }))
}

/// Extract the `[tool.uv.workspace] members` array from a parsed
/// `pyproject.toml`.
///
/// Returns an empty vector when the table or field is absent, so callers treat
/// a missing or empty `members` list as "not a uv workspace".
pub(super) fn uv_workspace_members_from_value(parsed: &toml_crate::Value) -> Vec<String> {
    parsed
        .get("tool")
        .and_then(|t| t.get("uv"))
        .and_then(|uv| uv.get("workspace"))
        .and_then(|ws| ws.get("members"))
        .and_then(|m| m.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn members_reads_tool_uv_workspace_members() {
        let parsed: toml_crate::Value = toml_crate::from_str(
            "[project]\nname = \"root\"\nversion = \"0.1.0\"\n\n\
             [tool.uv.workspace]\nmembers = [\"packages/*\"]\n",
        )
        .unwrap();

        assert_eq!(
            uv_workspace_members_from_value(&parsed),
            vec!["packages/*".to_string()]
        );
    }

    #[test]
    fn members_empty_when_table_absent() {
        let parsed: toml_crate::Value =
            toml_crate::from_str("[project]\nname = \"solo\"\n").unwrap();

        assert!(uv_workspace_members_from_value(&parsed).is_empty());
    }
}
