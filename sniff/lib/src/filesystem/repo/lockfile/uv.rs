//! uv `uv.lock` parser.
//!
//! `[manifest].members` lists workspace member **names**, the root's
//! included. Each name maps to a path through its package's local `editable`
//! or `virtual` source. uv omits `[manifest]` when the root is the only
//! member. uv does not promise a stable lockfile format, so only the version
//! and revision with a real-tool fixture are accepted.

use std::collections::{BTreeMap, BTreeSet};

use biscuit_file::toml_crate;
use serde::Deserialize;

use super::{Outcome, ParsedLockfile};

/// `(version, revision)` written by uv 0.9.5.
const ACCEPTED: (i64, i64) = (1, 3);

/// Parse a `uv.lock` document.
///
/// ## Returns
///
/// - [`ParsedLockfile::UnsupportedVersion`] unless `version = 1` and
///   `revision = 3`, including when a newer layout no longer fits the
///   accepted shape.
/// - [`ParsedLockfile::AmbiguousMembership`] when a member name has no local
///   package path, or more than one.
/// - Otherwise the member paths; an absent `[manifest]` is the root alone,
///   which is the empty set once the root is excluded.
///
/// ## Errors
///
/// Invalid TOML, a missing or non-integer `version`, and, for the accepted
/// version, fields of the wrong type or a package without a name.
pub(super) fn parse(content: &str) -> Outcome {
    let document: UvLockDocument = match toml_crate::from_str(content) {
        Ok(document) => document,
        // A layout that no longer fits the typed shape is still classified
        // by its version, so an unsupported version is never a parse failure.
        Err(error) => {
            return match toml_crate::from_str::<UvLockHeader>(content) {
                Ok(header)
                    if header.version.is_some() && !is_accepted(header.version, header.revision) =>
                {
                    Ok(ParsedLockfile::UnsupportedVersion)
                }
                _ => Err(error.to_string()),
            };
        }
    };
    if document.version.is_none() {
        return Err("uv.lock has no version".to_owned());
    }
    if !is_accepted(document.version, document.revision) {
        return Ok(ParsedLockfile::UnsupportedVersion);
    }

    let mut local_paths: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
    for package in &document.package {
        let Some(source) = &package.source else {
            continue;
        };
        for path in [&source.editable, &source.virtual_path].into_iter().flatten() {
            local_paths
                .entry(package.name.as_str())
                .or_default()
                .insert(path.as_str());
        }
    }

    let names: BTreeSet<&str> = document
        .manifest
        .iter()
        .flat_map(|manifest| manifest.members.iter().map(String::as_str))
        .collect();
    let mut members = Vec::with_capacity(names.len());
    for name in names {
        match local_paths.get(name) {
            Some(paths) if paths.len() == 1 => {
                members.extend(paths.iter().map(|path| (*path).to_owned()));
            }
            _ => return Ok(ParsedLockfile::AmbiguousMembership),
        }
    }
    Ok(ParsedLockfile::Members(members))
}

fn is_accepted(version: Option<i64>, revision: Option<i64>) -> bool {
    (version, revision) == (Some(ACCEPTED.0), Some(ACCEPTED.1))
}

/// The part of a `uv.lock` document that [`parse`] reads.
#[derive(Deserialize)]
struct UvLockDocument {
    version: Option<i64>,
    revision: Option<i64>,
    manifest: Option<UvManifest>,
    #[serde(default)]
    package: Vec<UvPackage>,
}

/// The version fields alone, read only after the typed parse failed.
#[derive(Deserialize)]
struct UvLockHeader {
    version: Option<i64>,
    revision: Option<i64>,
}

#[derive(Deserialize)]
struct UvManifest {
    #[serde(default)]
    members: Vec<String>,
}

#[derive(Deserialize)]
struct UvPackage {
    name: String,
    source: Option<UvSource>,
}

/// A package's source. Only local workspace-style sources carry a member
/// path; `directory`, `registry`, `git`, and `url` sources never do.
#[derive(Deserialize)]
struct UvSource {
    editable: Option<String>,
    #[serde(rename = "virtual")]
    virtual_path: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn members(paths: &[&str]) -> Option<ParsedLockfile> {
        Some(ParsedLockfile::Members(
            paths.iter().map(|path| (*path).to_owned()).collect(),
        ))
    }

    /// Every shipped `uv.lock` fixture, with the parse the accepted-version
    /// matrix specifies. `None` is `parse_failed`. Member paths come out
    /// sorted by member name.
    #[test]
    fn every_shipped_uv_fixture_parses_as_the_matrix_specifies() {
        let workspace = &[
            "packages/alpha",
            "packages/beta",
            ".",
            ".tools/hidden",
        ];
        let cases: [(&str, &str, Option<ParsedLockfile>); 9] = [
            (
                "workspace",
                include_str!("../../../../tests/fixtures/lockfiles/uv-0.9.5/workspace/uv.lock"),
                members(workspace),
            ),
            (
                "virtual-root",
                include_str!("../../../../tests/fixtures/lockfiles/uv-0.9.5/virtual-root/uv.lock"),
                members(&["packages/alpha", "packages/beta"]),
            ),
            (
                "root-only-workspace",
                include_str!("../../../../tests/fixtures/lockfiles/uv-0.9.5/root-only-workspace/uv.lock"),
                members(&[]),
            ),
            (
                "single-project",
                include_str!("../../../../tests/fixtures/lockfiles/uv-0.9.5/single-project/uv.lock"),
                members(&[]),
            ),
            (
                "workspace-edited-stale-extra",
                include_str!("../../../../tests/fixtures/lockfiles/uv-0.9.5/workspace-edited-stale-extra/uv.lock"),
                members(&[
                    "packages/alpha",
                    "packages/beta",
                    ".",
                    "packages/gamma",
                    ".tools/hidden",
                ]),
            ),
            (
                "workspace-edited-missing",
                include_str!("../../../../tests/fixtures/lockfiles/uv-0.9.5/workspace-edited-missing/uv.lock"),
                members(&["packages/alpha", ".", ".tools/hidden"]),
            ),
            (
                "workspace-edited-malformed-trailing",
                include_str!("../../../../tests/fixtures/lockfiles/uv-0.9.5/workspace-edited-malformed-trailing/uv.lock"),
                None,
            ),
            (
                "workspace-edited-unknown-version",
                include_str!("../../../../tests/fixtures/lockfiles/uv-0.9.5/workspace-edited-unknown-version/uv.lock"),
                Some(ParsedLockfile::UnsupportedVersion),
            ),
            (
                "workspace-edited-missing-required-field",
                include_str!("../../../../tests/fixtures/lockfiles/uv-0.9.5/workspace-edited-missing-required-field/uv.lock"),
                members(&[]),
            ),
        ];
        for (fixture, content, expected) in cases {
            assert_eq!(parse(content).ok(), expected, "uv-0.9.5/{fixture}");
        }
    }

    const HEADER: &str = "version = 1\nrevision = 3\n";

    #[test]
    fn a_non_member_local_dependency_is_never_a_member() {
        let content = format!(
            "{HEADER}[manifest]\nmembers = [\"alpha\"]\n\n\
             [[package]]\nname = \"alpha\"\nsource = {{ editable = \"packages/alpha\" }}\n\n\
             [[package]]\nname = \"local-lib\"\nsource = {{ directory = \"local-lib\" }}\n\n\
             [[package]]\nname = \"path-dep\"\nsource = {{ editable = \"vendor/path-dep\" }}\n"
        );
        assert_eq!(parse(&content).ok(), members(&["packages/alpha"]));
    }

    #[test]
    fn an_unmapped_or_doubly_mapped_member_name_is_ambiguous() {
        let unmapped = format!(
            "{HEADER}[manifest]\nmembers = [\"alpha\", \"ghost\"]\n\n\
             [[package]]\nname = \"alpha\"\nsource = {{ editable = \"packages/alpha\" }}\n"
        );
        assert_eq!(parse(&unmapped), Ok(ParsedLockfile::AmbiguousMembership));

        let registry_only = format!(
            "{HEADER}[manifest]\nmembers = [\"alpha\"]\n\n\
             [[package]]\nname = \"alpha\"\nsource = {{ registry = \"https://pypi.org/simple\" }}\n"
        );
        assert_eq!(parse(&registry_only), Ok(ParsedLockfile::AmbiguousMembership));

        let doubly_mapped = format!(
            "{HEADER}[manifest]\nmembers = [\"alpha\"]\n\n\
             [[package]]\nname = \"alpha\"\nsource = {{ editable = \"packages/alpha\" }}\n\n\
             [[package]]\nname = \"alpha\"\nsource = {{ editable = \"other/alpha\" }}\n"
        );
        assert_eq!(parse(&doubly_mapped), Ok(ParsedLockfile::AmbiguousMembership));
    }

    #[test]
    fn a_member_listed_twice_with_one_path_is_one_member() {
        let content = format!(
            "{HEADER}[manifest]\nmembers = [\"alpha\", \"alpha\"]\n\n\
             [[package]]\nname = \"alpha\"\nsource = {{ editable = \"packages/alpha\" }}\n"
        );
        assert_eq!(parse(&content).ok(), members(&["packages/alpha"]));
    }

    #[test]
    fn only_version_one_revision_three_is_accepted() {
        for header in [
            "version = 2\nrevision = 3\n",
            "version = 1\nrevision = 2\n",
            "version = 1\n",
            "version = 99\n",
        ] {
            let content = format!("{header}[manifest]\nmembers = [\"alpha\"]\n");
            assert_eq!(
                parse(&content),
                Ok(ParsedLockfile::UnsupportedVersion),
                "{header:?}"
            );
        }
    }

    #[test]
    fn an_unsupported_version_with_an_unfamiliar_layout_is_not_a_parse_failure() {
        let content = "version = 2\nrevision = 1\n[manifest]\nmembers = \"alpha\"\n\
                       [[package]]\nsource = \"somewhere\"\n";
        assert_eq!(parse(content), Ok(ParsedLockfile::UnsupportedVersion));
    }

    #[test]
    fn rejects_malformed_accepted_documents() {
        for (label, content) in [
            ("invalid TOML", "version = 1\nrevision = 3\n[manifest\n"),
            ("no version", "revision = 3\n[manifest]\nmembers = []\n"),
            ("string version", "version = \"1\"\nrevision = 3\n"),
            (
                "members not a list",
                "version = 1\nrevision = 3\n[manifest]\nmembers = \"alpha\"\n",
            ),
            (
                "non-string member",
                "version = 1\nrevision = 3\n[manifest]\nmembers = [1]\n",
            ),
            (
                "package without a name",
                "version = 1\nrevision = 3\n[[package]]\nversion = \"0.1.0\"\n",
            ),
            (
                "editable path not a string",
                "version = 1\nrevision = 3\n[[package]]\nname = \"a\"\nsource = { editable = 1 }\n",
            ),
        ] {
            assert!(parse(content).is_err(), "{label}: {:?}", parse(content));
        }
    }
}
