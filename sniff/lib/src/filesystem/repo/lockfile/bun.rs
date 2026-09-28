//! Bun text `bun.lock` parser.
//!
//! The format is JSON with comments and trailing commas. Membership is the
//! key set of the top-level `workspaces` object; `""` is the root. Binary
//! `bun.lockb` is never read (it is selected by name and reported from
//! metadata). Accepted versions are listed in `accepted-versions.md` of
//! `2026-09-26-lockfile-corroboration`.

use std::collections::HashSet;
use std::fmt;

use serde::Deserialize;
use serde::de::{self, IgnoredAny, MapAccess, Visitor};

use super::super::jsonc;
use super::{Outcome, ParsedLockfile};

/// `lockfileVersion` values with a real-tool fixture: Bun 1.2.0 and 1.3.3
/// both write 1.
const ACCEPTED_VERSIONS: [u64; 1] = [1];

/// Parse a `bun.lock` document.
///
/// Only `lockfileVersion` and the `workspaces` keys are retained; workspace
/// bodies and `packages` are skipped while the whole document is checked.
///
/// ## Returns
///
/// [`ParsedLockfile::UnsupportedVersion`] for any version outside the
/// accepted matrix, and otherwise every workspace key, including the root's
/// `""`.
///
/// ## Errors
///
/// Invalid JSONC (including trailing content), a missing `lockfileVersion`,
/// and, for an accepted version, a missing or non-object `workspaces` or a
/// duplicate workspace key.
pub(super) fn parse(content: &str) -> Outcome {
    let document = match jsonc::from_str::<BunLock>(content) {
        Ok(document) => document,
        // An unfamiliar body under an unsupported version is still that
        // version, not a parse failure.
        Err(error) => {
            return match jsonc::from_str::<BunLockHeader>(content) {
                Ok(BunLockHeader {
                    lockfile_version: Some(version),
                }) if !version.is_accepted() => Ok(ParsedLockfile::UnsupportedVersion),
                _ => Err(error),
            };
        }
    };
    let Some(version) = document.lockfile_version else {
        return Err("bun.lock has no lockfileVersion".to_owned());
    };
    if !version.is_accepted() {
        return Ok(ParsedLockfile::UnsupportedVersion);
    }
    match document.workspaces {
        None => Err("bun.lock has no workspaces object".to_owned()),
        Some(Workspaces { duplicate: true, .. }) => Err("duplicate workspace key".to_owned()),
        Some(Workspaces { keys, .. }) => Ok(ParsedLockfile::Members(keys)),
    }
}

/// The part of a `bun.lock` document that [`parse`] reads.
#[derive(Deserialize)]
struct BunLock {
    #[serde(rename = "lockfileVersion")]
    lockfile_version: Option<LockfileVersion>,
    workspaces: Option<Workspaces>,
}

/// Only the version, for classifying a document whose body did not parse.
#[derive(Deserialize)]
struct BunLockHeader {
    #[serde(rename = "lockfileVersion")]
    lockfile_version: Option<LockfileVersion>,
}

/// Bun writes an integer; anything else is a recognizable but unsupported
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

/// The `workspaces` keys. A plain map would keep the last of two equal keys
/// silently.
struct Workspaces {
    keys: Vec<String>,
    duplicate: bool,
}

impl<'de> Deserialize<'de> for Workspaces {
    fn deserialize<D: de::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct KeysVisitor;

        impl<'de> Visitor<'de> for KeysVisitor {
            type Value = Workspaces;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("the bun.lock workspaces object")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut keys = Vec::new();
                let mut seen = HashSet::new();
                let mut duplicate = false;
                while let Some(key) = map.next_key::<String>()? {
                    map.next_value::<IgnoredAny>()?;
                    duplicate |= !seen.insert(key.clone());
                    keys.push(key);
                }
                Ok(Workspaces { keys, duplicate })
            }
        }

        deserializer.deserialize_map(KeysVisitor)
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

    /// Every shipped `bun.lock` fixture, with the parse the accepted-version
    /// matrix specifies. `None` is `parse_failed`.
    fn corpus() -> Vec<(&'static str, &'static str, Option<ParsedLockfile>)> {
        let workspace = ["", ".tools/hidden", "packages/alpha", "packages/beta"];
        vec![
            (
                "bun-1.2.0/workspace",
                include_str!("../../../../tests/fixtures/lockfiles/bun-1.2.0/workspace/bun.lock"),
                members(&workspace),
            ),
            (
                "bun-1.3.3/workspace",
                include_str!("../../../../tests/fixtures/lockfiles/bun-1.3.3/workspace/bun.lock"),
                members(&workspace),
            ),
            (
                "bun-1.3.3/precedence-both",
                include_str!("../../../../tests/fixtures/lockfiles/bun-1.3.3/precedence-both/bun.lock"),
                members(&workspace),
            ),
            (
                "bun-1.3.3/workspace-edited-comments-trailing-commas",
                include_str!("../../../../tests/fixtures/lockfiles/bun-1.3.3/workspace-edited-comments-trailing-commas/bun.lock"),
                members(&workspace),
            ),
            (
                "bun-1.3.3/workspace-edited-stale-extra",
                include_str!("../../../../tests/fixtures/lockfiles/bun-1.3.3/workspace-edited-stale-extra/bun.lock"),
                members(&["", ".tools/hidden", "packages/alpha", "packages/beta", "packages/gamma"]),
            ),
            (
                "bun-1.3.3/workspace-edited-missing",
                include_str!("../../../../tests/fixtures/lockfiles/bun-1.3.3/workspace-edited-missing/bun.lock"),
                members(&["", ".tools/hidden", "packages/alpha"]),
            ),
            (
                "bun-1.3.3/workspace-edited-malformed-trailing",
                include_str!("../../../../tests/fixtures/lockfiles/bun-1.3.3/workspace-edited-malformed-trailing/bun.lock"),
                None,
            ),
            (
                "bun-1.3.3/workspace-edited-duplicate-key",
                include_str!("../../../../tests/fixtures/lockfiles/bun-1.3.3/workspace-edited-duplicate-key/bun.lock"),
                None,
            ),
            (
                "bun-1.3.3/workspace-edited-unknown-version",
                include_str!("../../../../tests/fixtures/lockfiles/bun-1.3.3/workspace-edited-unknown-version/bun.lock"),
                Some(ParsedLockfile::UnsupportedVersion),
            ),
            (
                "bun-1.3.3/workspace-edited-missing-required-field",
                include_str!("../../../../tests/fixtures/lockfiles/bun-1.3.3/workspace-edited-missing-required-field/bun.lock"),
                None,
            ),
        ]
    }

    #[test]
    fn every_shipped_bun_fixture_parses_as_the_matrix_specifies() {
        for (fixture, content, expected) in corpus() {
            assert_eq!(parse(content).ok(), expected, "{fixture}");
        }
    }

    #[test]
    fn strict_json_rejects_real_bun_output() {
        let content =
            include_str!("../../../../tests/fixtures/lockfiles/bun-1.3.3/workspace/bun.lock");
        assert!(serde_json::from_str::<serde_json::Value>(content).is_err());
        assert!(parse(content).is_ok());
    }

    #[test]
    fn unsupported_versions_never_yield_membership() {
        for version in ["0", "2", "99", "\"1\"", "1.5"] {
            for body in [
                r#""workspaces": {"": {}, "a": {}}"#,
                r#""packages": {}"#,
                r#""workspaces": 3"#,
            ] {
                let content = format!("{{\"lockfileVersion\": {version}, {body},}}");
                assert_eq!(parse(&content), Ok(ParsedLockfile::UnsupportedVersion), "{content}");
            }
        }
    }

    #[test]
    fn rejects_malformed_documents() {
        for (label, content) in [
            ("empty", ""),
            ("not an object", "[]"),
            ("no version", r#"{"workspaces": {"": {}}}"#),
            ("no workspaces", r#"{"lockfileVersion": 1}"#),
            ("array workspaces", r#"{"lockfileVersion": 1, "workspaces": [""]}"#),
            ("duplicate workspace", r#"{"lockfileVersion": 1, "workspaces": {"a": {}, "a": {}}}"#),
            ("single quotes", r#"{'lockfileVersion': 1, "workspaces": {}}"#),
            ("trailing garbage", r#"{"lockfileVersion": 1, "workspaces": {}} x"#),
        ] {
            assert!(parse(content).is_err(), "{label}");
        }
    }

    #[test]
    fn empty_workspaces_is_an_empty_member_set() {
        assert_eq!(
            parse(r#"{"lockfileVersion": 1, "workspaces": {},}"#),
            Ok(ParsedLockfile::Members(Vec::new()))
        );
    }
}
