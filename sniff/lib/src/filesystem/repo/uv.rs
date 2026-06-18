//! uv workspace (`pyproject.toml` `[tool.uv.workspace]`) detection.

use std::path::Path;

use biscuit_file::toml_crate;

use crate::{Result, SniffError};

use super::detection::{DetectorOutcome, create_package, dedupe_packages, resolve_internal_deps};
use super::glob::expand_membership_globs;
use super::standard::{GlobDialect, MonorepoStandard, PackageProvenance};

pub(super) fn detect_uv_workspace(root: &Path) -> Result<Option<DetectorOutcome>> {
    let pyproject = root.join("pyproject.toml");
    if !pyproject.exists() {
        return Ok(None);
    }

    let members = parse_uv_workspace_members(&pyproject)?;
    if members.is_empty() {
        return Ok(None);
    }

    let lock_versions = None;
    let dialect = MonorepoStandard::UvWorkspace
        .glob_dialect()
        .unwrap_or(GlobDialect::Minimatch);
    let mut packages = expand_membership_globs(
        root,
        &members,
        dialect,
        MonorepoStandard::UvWorkspace,
        None,
        &lock_versions,
    );

    // uv's `RootMembership::Always`: the root `[project]` is itself a workspace
    // member, so the root directory is counted alongside the globbed children.
    packages.push(create_package(
        root,
        root,
        MonorepoStandard::UvWorkspace,
        PackageProvenance::Globbed,
        &lock_versions,
    ));

    packages = dedupe_packages(packages);
    resolve_internal_deps(&mut packages);

    Ok(Some(DetectorOutcome {
        standard: MonorepoStandard::UvWorkspace,
        root: root.to_path_buf(),
        packages,
    }))
}

/// Parse the `[tool.uv.workspace] members` array from a `pyproject.toml`.
///
/// Returns an empty vector when the table or field is absent, so callers treat a
/// missing or empty `members` list as "not a uv workspace".
pub(super) fn parse_uv_workspace_members(pyproject_path: &Path) -> Result<Vec<String>> {
    let content = std::fs::read_to_string(pyproject_path)?;
    let parsed: toml_crate::Value =
        toml_crate::from_str(&content).map_err(|e| SniffError::SystemInfo {
            domain: "repo",
            message: e.to_string(),
        })?;

    Ok(parsed
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
        .unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn parse_members_reads_tool_uv_workspace_members() {
        let dir = tempdir().unwrap();
        let pyproject = dir.path().join("pyproject.toml");
        fs::write(
            &pyproject,
            "[project]\nname = \"root\"\nversion = \"0.1.0\"\n\n\
             [tool.uv.workspace]\nmembers = [\"packages/*\"]\n",
        )
        .unwrap();

        let members = parse_uv_workspace_members(&pyproject).unwrap();
        assert_eq!(members, vec!["packages/*".to_string()]);
    }

    #[test]
    fn parse_members_empty_when_table_absent() {
        let dir = tempdir().unwrap();
        let pyproject = dir.path().join("pyproject.toml");
        fs::write(&pyproject, "[project]\nname = \"solo\"\n").unwrap();

        assert!(parse_uv_workspace_members(&pyproject).unwrap().is_empty());
    }
}
