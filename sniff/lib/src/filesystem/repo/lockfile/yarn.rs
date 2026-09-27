//! Yarn Berry `yarn.lock` parser.
//!
//! Membership is every entry whose `resolution` is
//! `<name>@workspace:<path>`; the root resolves to `.`. Entry keys can hold
//! combined descriptors such as `workspace:^`, so only `resolution` is read.
//! Yarn Classic records no workspaces and is recognized by its header.
//! Accepted versions are listed in `accepted-versions.md` of
//! `2026-09-26-lockfile-corroboration`.

use std::collections::{BTreeMap, HashSet};
use std::fmt;

use biscuit_file::serde_yaml_ng;
use serde::Deserialize;
use serde::de::{self, IgnoredAny, MapAccess, Visitor};

use super::{Outcome, ParsedLockfile};

/// `__metadata.version` values with a real-tool fixture: Yarn 3.8.7 writes
/// 6, Yarn 4.0.2 writes 8, and Yarn 4.18.1 writes 10.
const ACCEPTED_VERSIONS: [u64; 3] = [6, 8, 10];

/// Yarn Classic's signature line. Its body is not YAML, so it is recognized
/// before parsing rather than reported as a parse failure.
const CLASSIC_SIGNATURE: &str = "# yarn lockfile v1";

const WORKSPACE_PROTOCOL: &str = "@workspace:";

/// Parse a `yarn.lock` document.
///
/// ## Returns
///
/// [`ParsedLockfile::UnsupportedVersion`] for Yarn Classic and any Berry
/// version outside the accepted matrix, and otherwise every workspace path,
/// including the root's `.`.
///
/// ## Errors
///
/// Invalid YAML, a missing `__metadata.version`, and, for an accepted
/// version, a duplicate entry key, an entry without a string `resolution`, a
/// workspace path resolved under two names, or no root workspace entry.
pub(super) fn parse(content: &str) -> Outcome {
    if is_classic(content) {
        return Ok(ParsedLockfile::UnsupportedVersion);
    }
    let document = match serde_yaml_ng::from_str::<YarnLock>(content) {
        Ok(document) => document,
        // An unfamiliar body under an unsupported version is still that
        // version, not a parse failure.
        Err(error) => {
            return match serde_yaml_ng::from_str::<YarnLockHeader>(content) {
                Ok(YarnLockHeader {
                    metadata: Some(Metadata {
                        version: Some(version),
                    }),
                }) if !version.is_accepted() => Ok(ParsedLockfile::UnsupportedVersion),
                _ => Err(error.to_string()),
            };
        }
    };
    let Some(version) = document.metadata.and_then(|metadata| metadata.version) else {
        return Err("yarn.lock has no __metadata.version".to_owned());
    };
    if !version.is_accepted() {
        return Ok(ParsedLockfile::UnsupportedVersion);
    }
    if let Some(why) = document.invalid {
        return Err(why.to_owned());
    }
    // Two entries may resolve to one workspace; two names for one path are
    // conflicting identities.
    let mut workspaces: BTreeMap<String, String> = BTreeMap::new();
    for resolution in document.resolutions {
        let Some((name, path)) = workspace_resolution(&resolution) else {
            continue;
        };
        if let Some(existing) = workspaces.insert(path.to_owned(), name.to_owned())
            && existing != name
        {
            return Err(format!("workspace {path} resolves under two names"));
        }
    }
    if !workspaces.contains_key(".") {
        return Err("yarn.lock has no root workspace entry".to_owned());
    }
    Ok(ParsedLockfile::Members(workspaces.into_keys().collect()))
}

/// Whether the leading comment block carries Yarn Classic's signature.
fn is_classic(content: &str) -> bool {
    content
        .lines()
        .map(str::trim)
        .take_while(|line| line.is_empty() || line.starts_with('#'))
        .any(|line| line == CLASSIC_SIGNATURE)
}

/// Split `<name>@workspace:<path>` at the last marker, so a scoped name's
/// leading `@` is never mistaken for the separator. A name with a protocol
/// (`patch:`, `link:`) is not a workspace resolution.
fn workspace_resolution(resolution: &str) -> Option<(&str, &str)> {
    let at = resolution.rfind(WORKSPACE_PROTOCOL)?;
    let (name, path) = (&resolution[..at], &resolution[at + WORKSPACE_PROTOCOL.len()..]);
    (!name.is_empty() && !name.contains(':') && !path.is_empty()).then_some((name, path))
}

/// The part of a `yarn.lock` document that [`parse`] reads.
#[derive(Default)]
struct YarnLock {
    metadata: Option<Metadata>,
    /// Every entry's `resolution`, in document order.
    resolutions: Vec<String>,
    invalid: Option<&'static str>,
}

/// Only `__metadata`, for classifying a document whose body did not parse.
#[derive(Deserialize)]
struct YarnLockHeader {
    #[serde(rename = "__metadata")]
    metadata: Option<Metadata>,
}

#[derive(Deserialize)]
struct Metadata {
    version: Option<LockfileVersion>,
}

/// Berry writes an integer; anything else is a recognizable but unsupported
/// version rather than a parse failure.
#[derive(Deserialize)]
#[serde(untagged)]
enum LockfileVersion {
    Number(u64),
    Other(IgnoredAny),
}

impl LockfileVersion {
    fn is_accepted(&self) -> bool {
        matches!(self, Self::Number(version) if ACCEPTED_VERSIONS.contains(version))
    }
}

#[derive(Deserialize)]
struct Entry {
    resolution: Option<String>,
}

impl<'de> Deserialize<'de> for YarnLock {
    fn deserialize<D: de::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct EntriesVisitor;

        impl<'de> Visitor<'de> for EntriesVisitor {
            type Value = YarnLock;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("a yarn.lock mapping")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut lock = YarnLock::default();
                let mut seen = HashSet::new();
                while let Some(key) = map.next_key::<String>()? {
                    if !seen.insert(key.clone()) {
                        lock.invalid.get_or_insert("duplicate yarn.lock entry key");
                    }
                    if key == "__metadata" {
                        lock.metadata = Some(map.next_value::<Metadata>()?);
                        continue;
                    }
                    match map.next_value::<Entry>()?.resolution {
                        Some(resolution) => lock.resolutions.push(resolution),
                        None => {
                            lock.invalid.get_or_insert("yarn.lock entry has no resolution");
                        }
                    }
                }
                Ok(lock)
            }
        }

        deserializer.deserialize_map(EntriesVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn members(paths: &[&str]) -> Option<ParsedLockfile> {
        Some(ParsedLockfile::Members(
            paths.iter().map(|path| (*path).to_owned()).collect(),
        ))
    }

    /// Every shipped Yarn fixture, with the parse the accepted-version matrix
    /// specifies. `None` is `parse_failed`.
    fn corpus() -> Vec<(&'static str, &'static str, Option<ParsedLockfile>)> {
        let workspace = [".", ".tools/hidden", "packages/alpha", "packages/beta"];
        vec![
            (
                "yarn-3.8.7/workspace",
                include_str!("../../../../tests/fixtures/lockfiles/yarn-3.8.7/workspace/yarn.lock"),
                members(&workspace),
            ),
            (
                "yarn-4.0.2/workspace",
                include_str!("../../../../tests/fixtures/lockfiles/yarn-4.0.2/workspace/yarn.lock"),
                members(&workspace),
            ),
            (
                "yarn-4.18.1/workspace",
                include_str!("../../../../tests/fixtures/lockfiles/yarn-4.18.1/workspace/yarn.lock"),
                members(&workspace),
            ),
            (
                "yarn-1.22.22/workspace",
                include_str!("../../../../tests/fixtures/lockfiles/yarn-1.22.22/workspace/yarn.lock"),
                Some(ParsedLockfile::UnsupportedVersion),
            ),
            (
                "yarn-4.18.1/workspace-edited-stale-extra",
                include_str!("../../../../tests/fixtures/lockfiles/yarn-4.18.1/workspace-edited-stale-extra/yarn.lock"),
                members(&[".", ".tools/hidden", "packages/alpha", "packages/beta", "packages/gamma"]),
            ),
            (
                "yarn-4.18.1/workspace-edited-missing",
                include_str!("../../../../tests/fixtures/lockfiles/yarn-4.18.1/workspace-edited-missing/yarn.lock"),
                members(&[".", ".tools/hidden", "packages/alpha"]),
            ),
            (
                "yarn-4.18.1/workspace-edited-malformed-trailing",
                include_str!("../../../../tests/fixtures/lockfiles/yarn-4.18.1/workspace-edited-malformed-trailing/yarn.lock"),
                None,
            ),
            (
                "yarn-4.18.1/workspace-edited-duplicate-key",
                include_str!("../../../../tests/fixtures/lockfiles/yarn-4.18.1/workspace-edited-duplicate-key/yarn.lock"),
                None,
            ),
            (
                "yarn-4.18.1/workspace-edited-unknown-version",
                include_str!("../../../../tests/fixtures/lockfiles/yarn-4.18.1/workspace-edited-unknown-version/yarn.lock"),
                Some(ParsedLockfile::UnsupportedVersion),
            ),
            (
                "yarn-4.18.1/workspace-edited-missing-required-field",
                include_str!("../../../../tests/fixtures/lockfiles/yarn-4.18.1/workspace-edited-missing-required-field/yarn.lock"),
                None,
            ),
        ]
    }

    #[test]
    fn every_shipped_yarn_fixture_parses_as_the_matrix_specifies() {
        for (fixture, content, expected) in corpus() {
            assert_eq!(parse(content).ok(), expected, "{fixture}");
        }
    }

    /// The generic `serde_yaml_ng::Value` reading of the same rule, kept as
    /// the extraction-parity oracle for accepted fixtures.
    fn reference_members(content: &str) -> Vec<String> {
        let document: serde_yaml_ng::Value = serde_yaml_ng::from_str(content).expect("valid YAML");
        let mut members: Vec<String> = document
            .as_mapping()
            .expect("mapping")
            .values()
            .filter_map(|entry| entry.get("resolution")?.as_str())
            .filter_map(workspace_resolution)
            .map(|(_, path)| path.to_owned())
            .collect();
        members.sort();
        members.dedup();
        members
    }

    #[test]
    fn typed_parse_matches_the_generic_reference_on_accepted_fixtures() {
        for (fixture, content, expected) in corpus() {
            if let Some(ParsedLockfile::Members(_)) = expected {
                assert_eq!(
                    parse(content),
                    Ok(ParsedLockfile::Members(reference_members(content))),
                    "{fixture}"
                );
            }
        }
    }

    #[test]
    fn scoped_names_split_at_the_last_workspace_marker() {
        assert_eq!(
            workspace_resolution("@fixture/beta@workspace:packages/beta"),
            Some(("@fixture/beta", "packages/beta"))
        );
        assert_eq!(workspace_resolution("root@workspace:."), Some(("root", ".")));
        for not_a_workspace in [
            "local-lib@link:./local-lib::locator=fixture-root%40workspace%3A.",
            "portal-lib@portal:./portal-lib::locator=fixture-root%40workspace%3A.",
            "left-pad@npm:1.3.0",
            "x@patch:x@workspace:packages/x#./fix.patch",
            "@workspace:packages/x",
            "x@workspace:",
        ] {
            assert_eq!(workspace_resolution(not_a_workspace), None, "{not_a_workspace}");
        }
    }

    #[test]
    fn link_and_portal_entries_are_not_members() {
        let content = include_str!(
            "../../../../tests/fixtures/lockfiles/yarn-4.18.1/workspace/yarn.lock"
        );
        assert!(content.contains("@link:") && content.contains("@portal:"));
        let Ok(ParsedLockfile::Members(members)) = parse(content) else {
            panic!("the fixture parses");
        };
        assert!(
            !members.iter().any(|member| member.contains("local-lib") || member.contains("portal-lib")),
            "{members:?}"
        );
    }

    #[test]
    fn the_same_workspace_under_two_entries_collapses() {
        let content = "__metadata:\n  version: 8\n\"a@workspace:.\":\n  resolution: \"a@workspace:.\"\n\"b@workspace:^\":\n  resolution: \"b@workspace:p/b\"\n\"b@workspace:p/b\":\n  resolution: \"b@workspace:p/b\"\n";
        assert_eq!(parse(content).ok(), members(&[".", "p/b"]));
    }

    #[test]
    fn rejects_conflicting_identities_and_missing_resolutions() {
        for (label, content) in [
            (
                "two names for one path",
                "__metadata:\n  version: 8\n\"a@workspace:.\":\n  resolution: \"a@workspace:.\"\n\"b@workspace:p\":\n  resolution: \"b@workspace:p\"\n\"c@workspace:p\":\n  resolution: \"c@workspace:p\"\n",
            ),
            (
                "entry without resolution",
                "__metadata:\n  version: 8\n\"a@workspace:.\":\n  resolution: \"a@workspace:.\"\n\"b@npm:1\":\n  version: 1.0.0\n",
            ),
            (
                "mistyped resolution",
                "__metadata:\n  version: 8\n\"a@workspace:.\":\n  resolution: [a]\n",
            ),
            ("no root workspace", "__metadata:\n  version: 8\n\"b@workspace:p\":\n  resolution: \"b@workspace:p\"\n"),
            ("no metadata", "\"a@workspace:.\":\n  resolution: \"a@workspace:.\"\n"),
            ("no metadata version", "__metadata:\n  cacheKey: 8\n"),
            ("empty", ""),
            ("scalar", "just text\n"),
        ] {
            assert!(parse(content).is_err(), "{label}");
        }
    }

    #[test]
    fn unsupported_versions_never_yield_membership() {
        for version in ["4", "7", "99", "\"8\"", "8.5"] {
            for body in [
                "\"a@workspace:.\":\n  resolution: \"a@workspace:.\"\n",
                "",
                "\"x\": 3\n",
                "\"b@workspace:p\":\n  resolution: [1]\n",
            ] {
                let content = format!("__metadata:\n  version: {version}\n{body}");
                assert_eq!(parse(&content), Ok(ParsedLockfile::UnsupportedVersion), "{content:?}");
            }
        }
    }

    #[test]
    fn the_classic_signature_is_recognized_before_parsing() {
        let content = "# THIS IS AN AUTOGENERATED FILE. DO NOT EDIT THIS FILE DIRECTLY.\n# yarn lockfile v1\n\n\nleft-pad@^1.3.0:\n  version \"1.3.0\"\n";
        assert_eq!(parse(content), Ok(ParsedLockfile::UnsupportedVersion));
        // A signature-like line after the header is ordinary content.
        let late = "__metadata:\n  version: 8\n\"a@workspace:.\":\n  resolution: \"a@workspace:.\"\n# yarn lockfile v1\n";
        assert_eq!(parse(late).ok(), members(&["."]));
    }
}
