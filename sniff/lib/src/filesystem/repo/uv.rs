//! uv workspace (`pyproject.toml` `[tool.uv.workspace]`) detection.

use std::path::Path;

use biscuit_file::toml_crate;

use crate::Result;

use super::detection::{DetectorOutcome, ManifestStore, RepoEvidence, probe_exists};
use super::glob::{DeclaredPatterns, expand_membership_globs};
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
    let Some(members) = uv_workspace_members_from_value(&parsed) else {
        return Ok(None);
    };

    let dialect = MonorepoStandard::UvWorkspace
        .glob_dialect()
        .unwrap_or(GlobDialect::Minimatch);
    // uv expands every member entry as a glob, so a missing literal path
    // matches nothing rather than leaving the set incomplete.
    let expansion = expand_membership_globs(
        root,
        &members.patterns,
        dialect,
        MonorepoStandard::UvWorkspace,
        None,
        evidence,
    );
    let mut seeds = expansion.seeds;

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
        incomplete: !expansion.patterns_resolved || members.has_invalid,
    }))
}

/// Extract the `[tool.uv.workspace] members` array from a parsed
/// `pyproject.toml`.
///
/// Returns `None` when the table or the `members` key is absent. An explicit
/// empty array is a declared root-only workspace, which still gets a layer so
/// its lockfile is observed; a `members` value that is not an array is an
/// invalid declaration, not an absent one.
pub(super) fn uv_workspace_members_from_value(
    parsed: &toml_crate::Value,
) -> Option<DeclaredPatterns> {
    let members = parsed
        .get("tool")
        .and_then(|t| t.get("uv"))
        .and_then(|uv| uv.get("workspace"))
        .and_then(|ws| ws.get("members"))?;
    Some(match members.as_array() {
        Some(arr) => arr.iter().map(|v| v.as_str()).collect(),
        None => DeclaredPatterns::invalid(),
    })
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
            Some(DeclaredPatterns {
                patterns: vec!["packages/*".to_string()],
                has_invalid: false,
            })
        );
    }

    #[test]
    fn a_non_string_member_is_recorded_as_invalid_rather_than_dropped() {
        for (source, patterns) in [
            ("members = [\"packages/alpha\", 123]", vec!["packages/alpha".to_string()]),
            ("members = [123]", Vec::new()),
        ] {
            let parsed: toml_crate::Value =
                toml_crate::from_str(&format!("[tool.uv.workspace]\n{source}\n")).unwrap();

            assert_eq!(
                uv_workspace_members_from_value(&parsed),
                Some(DeclaredPatterns {
                    patterns,
                    has_invalid: true,
                }),
                "{source}"
            );
        }
    }

    #[test]
    fn a_non_array_members_value_is_recorded_as_invalid_rather_than_absent() {
        for source in ["members = \"packages/*\"", "members = 123", "members = { a = 1 }"] {
            let parsed: toml_crate::Value =
                toml_crate::from_str(&format!("[tool.uv.workspace]\n{source}\n")).unwrap();

            assert_eq!(
                uv_workspace_members_from_value(&parsed),
                Some(DeclaredPatterns::invalid()),
                "{source}"
            );
        }
    }

    #[test]
    fn members_absent_when_table_or_key_absent() {
        for source in [
            "[project]\nname = \"solo\"\n",
            "[project]\nname = \"solo\"\n\n[tool.uv.workspace]\nexclude = [\"x\"]\n",
        ] {
            let parsed: toml_crate::Value = toml_crate::from_str(source).unwrap();

            assert_eq!(uv_workspace_members_from_value(&parsed), None, "{source}");
        }
    }

    #[test]
    fn an_explicit_empty_members_array_declares_a_root_only_workspace() {
        let parsed: toml_crate::Value = toml_crate::from_str(
            "[project]\nname = \"solo\"\n\n[tool.uv.workspace]\nmembers = []\n",
        )
        .unwrap();

        assert_eq!(
            uv_workspace_members_from_value(&parsed),
            Some(DeclaredPatterns::default())
        );
    }
}
