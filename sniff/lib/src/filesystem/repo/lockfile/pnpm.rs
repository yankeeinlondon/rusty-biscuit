//! pnpm `pnpm-lock.yaml` parser.
//!
//! Membership is the key set of the top-level `importers` mapping; `.` is the
//! root. Accepted versions are listed in `accepted-versions.md` of
//! `2026-09-26-lockfile-corroboration`.

use std::collections::HashSet;
use std::fmt;

use biscuit_file::serde_yaml_ng;
use serde::Deserialize;
use serde::de::{self, IgnoredAny, MapAccess, SeqAccess, Visitor};

use super::{Outcome, ParsedLockfile};

/// `lockfileVersion` values with a real-tool fixture: pnpm 8 writes `6.0`,
/// pnpm 9 and 10 write `9.0`.
const ACCEPTED_VERSIONS: [f64; 2] = [6.0, 9.0];
const ACCEPTED_TEXT: [&str; 2] = ["6.0", "9.0"];

/// Parse a `pnpm-lock.yaml` document.
///
/// Only `lockfileVersion` and the `importers` keys are retained; importer
/// bodies and every other section are skipped through `IgnoredAny`, while the
/// whole document is still checked for YAML syntax. A duplicate key inside a
/// skipped section (for example `packages:`) is tolerated, unlike a generic
/// `serde_yaml_ng::Value` parse, which rejects it; pnpm never writes one.
///
/// ## Returns
///
/// [`ParsedLockfile::UnsupportedVersion`] for any version outside the
/// accepted matrix, whatever the rest of the document holds, and otherwise
/// every importer key.
///
/// ## Errors
///
/// Invalid YAML, a missing or non-scalar `lockfileVersion`, and, for an
/// accepted version, a missing or non-mapping `importers` or an importer key
/// that is duplicated or not a string.
pub(super) fn parse(content: &str) -> Outcome {
    let document: PnpmLockDocument =
        serde_yaml_ng::from_str(content).map_err(|error| error.to_string())?;
    let Some(version) = document.lockfile_version else {
        return Err("pnpm-lock.yaml has no lockfileVersion".to_owned());
    };
    if !version.is_accepted() {
        return Ok(ParsedLockfile::UnsupportedVersion);
    }
    match document.importers {
        None => Err("pnpm-lock.yaml has no importers mapping".to_owned()),
        Some(Importers::Invalid(why)) => Err(why.to_owned()),
        Some(Importers::Keys(keys)) => Ok(ParsedLockfile::Members(keys)),
    }
}

/// The part of a `pnpm-lock.yaml` document that [`parse`] reads.
#[derive(Deserialize)]
struct PnpmLockDocument {
    #[serde(rename = "lockfileVersion")]
    lockfile_version: Option<LockfileVersion>,
    #[serde(default)]
    importers: Option<Importers>,
}

/// pnpm 6+ quotes the version; pnpm 5 wrote a bare number such as `5.4`.
#[derive(Deserialize)]
#[serde(untagged)]
enum LockfileVersion {
    Text(String),
    Number(f64),
}

impl LockfileVersion {
    /// A quoted version must be spelled exactly as pnpm writes it; a bare
    /// number compares by value.
    fn is_accepted(&self) -> bool {
        match self {
            Self::Text(text) => ACCEPTED_TEXT.contains(&text.as_str()),
            Self::Number(value) => ACCEPTED_VERSIONS.contains(value),
        }
    }
}

/// The importer keys, or why they cannot be membership evidence. Deciding
/// after the version keeps an unsupported version from reading as a parse
/// failure.
enum Importers {
    Keys(Vec<String>),
    Invalid(&'static str),
}

impl<'de> Deserialize<'de> for Importers {
    fn deserialize<D: de::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ImporterKeys;

        impl<'de> Visitor<'de> for ImporterKeys {
            type Value = Importers;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("any YAML value")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut keys = Vec::new();
                let mut seen = HashSet::new();
                let mut invalid = None;
                // Keys go through `Value`: deserializing them as `String`
                // would coerce the plain scalars `1`, `~`, and `true` into
                // strings instead of rejecting them.
                while let Some(key) = map.next_key::<serde_yaml_ng::Value>()? {
                    map.next_value::<IgnoredAny>()?;
                    match key {
                        serde_yaml_ng::Value::String(key) => {
                            if !seen.insert(key.clone()) {
                                invalid.get_or_insert("duplicate importer key");
                            }
                            keys.push(key);
                        }
                        _ => {
                            invalid.get_or_insert("importer key is not a string");
                        }
                    }
                }
                Ok(invalid.map_or(Importers::Keys(keys), Importers::Invalid))
            }

            fn visit_seq<A: SeqAccess<'de>>(self, seq: A) -> Result<Self::Value, A::Error> {
                IgnoredAny.visit_seq(seq)?;
                Ok(Importers::Invalid("importers is not a mapping"))
            }

            fn visit_bool<E: de::Error>(self, _: bool) -> Result<Self::Value, E> {
                Ok(Importers::Invalid("importers is not a mapping"))
            }

            fn visit_i64<E: de::Error>(self, _: i64) -> Result<Self::Value, E> {
                Ok(Importers::Invalid("importers is not a mapping"))
            }

            fn visit_u64<E: de::Error>(self, _: u64) -> Result<Self::Value, E> {
                Ok(Importers::Invalid("importers is not a mapping"))
            }

            fn visit_f64<E: de::Error>(self, _: f64) -> Result<Self::Value, E> {
                Ok(Importers::Invalid("importers is not a mapping"))
            }

            fn visit_str<E: de::Error>(self, _: &str) -> Result<Self::Value, E> {
                Ok(Importers::Invalid("importers is not a mapping"))
            }
        }

        deserializer.deserialize_any(ImporterKeys)
    }
}

/// The generic `serde_yaml_ng::Value` parse that [`parse`] replaces, kept as
/// the extraction-parity oracle.
#[cfg(test)]
fn parse_reference(content: &str) -> Outcome {
    use serde_yaml_ng::Value;

    let document: Value = serde_yaml_ng::from_str(content).map_err(|error| error.to_string())?;
    let accepted = match document.get("lockfileVersion") {
        Some(Value::String(text)) => ACCEPTED_TEXT.contains(&text.as_str()),
        Some(Value::Number(number)) => number
            .as_f64()
            .is_some_and(|value| ACCEPTED_VERSIONS.contains(&value)),
        _ => return Err("no scalar lockfileVersion".to_owned()),
    };
    if !accepted {
        return Ok(ParsedLockfile::UnsupportedVersion);
    }
    let importers = document
        .get("importers")
        .and_then(Value::as_mapping)
        .ok_or("no importers mapping")?;
    let mut keys = Vec::new();
    for key in importers.keys() {
        match key {
            Value::String(key) => keys.push(key.clone()),
            _ => return Err("importer key is not a string".to_owned()),
        }
    }
    Ok(ParsedLockfile::Members(keys))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every shipped pnpm-format fixture, with the parse the accepted-version
    /// matrix specifies. `None` is `parse_failed`.
    fn corpus() -> Vec<(&'static str, &'static str, Option<ParsedLockfile>)> {
        let members = |keys: &[&str]| {
            Some(ParsedLockfile::Members(
                keys.iter().map(|key| (*key).to_owned()).collect(),
            ))
        };
        let workspace = [".", ".tools/hidden", "packages/alpha", "packages/beta"];
        vec![
            (
                "pnpm-8.15.9/workspace",
                include_str!("../../../../tests/fixtures/lockfiles/pnpm-8.15.9/workspace/pnpm-lock.yaml"),
                members(&workspace),
            ),
            (
                "pnpm-9.15.9/workspace",
                include_str!("../../../../tests/fixtures/lockfiles/pnpm-9.15.9/workspace/pnpm-lock.yaml"),
                members(&workspace),
            ),
            (
                "pnpm-10.32.1/workspace",
                include_str!("../../../../tests/fixtures/lockfiles/pnpm-10.32.1/workspace/pnpm-lock.yaml"),
                members(&workspace),
            ),
            (
                "pnpm-10.32.1/workspace-edited-stale-extra",
                include_str!("../../../../tests/fixtures/lockfiles/pnpm-10.32.1/workspace-edited-stale-extra/pnpm-lock.yaml"),
                members(&[".", ".tools/hidden", "packages/alpha", "packages/beta", "packages/gamma"]),
            ),
            (
                "pnpm-10.32.1/workspace-edited-missing",
                include_str!("../../../../tests/fixtures/lockfiles/pnpm-10.32.1/workspace-edited-missing/pnpm-lock.yaml"),
                members(&[".", ".tools/hidden", "packages/alpha"]),
            ),
            (
                "pnpm-10.32.1/workspace-edited-malformed-trailing",
                include_str!("../../../../tests/fixtures/lockfiles/pnpm-10.32.1/workspace-edited-malformed-trailing/pnpm-lock.yaml"),
                None,
            ),
            (
                "pnpm-10.32.1/workspace-edited-duplicate-key",
                include_str!("../../../../tests/fixtures/lockfiles/pnpm-10.32.1/workspace-edited-duplicate-key/pnpm-lock.yaml"),
                None,
            ),
            (
                "pnpm-10.32.1/workspace-edited-unknown-version",
                include_str!("../../../../tests/fixtures/lockfiles/pnpm-10.32.1/workspace-edited-unknown-version/pnpm-lock.yaml"),
                Some(ParsedLockfile::UnsupportedVersion),
            ),
            (
                "pnpm-10.32.1/workspace-edited-missing-required-field",
                include_str!("../../../../tests/fixtures/lockfiles/pnpm-10.32.1/workspace-edited-missing-required-field/pnpm-lock.yaml"),
                None,
            ),
            (
                "rush-5.179.0/pnpm-workspace",
                include_str!("../../../../tests/fixtures/lockfiles/rush-5.179.0/pnpm-workspace/common/config/rush/pnpm-lock.yaml"),
                members(&[".", "../../.tools/hidden", "../../packages/alpha", "../../packages/beta"]),
            ),
        ]
    }

    #[test]
    fn every_shipped_pnpm_fixture_parses_as_the_matrix_specifies() {
        for (fixture, content, expected) in corpus() {
            assert_eq!(parse(content).ok(), expected, "{fixture}");
        }
    }

    #[test]
    fn typed_parse_matches_the_generic_reference_on_every_shipped_fixture() {
        for (fixture, content, _) in corpus() {
            assert_eq!(
                parse(content).ok(),
                parse_reference(content).ok(),
                "{fixture}"
            );
        }
    }

    #[test]
    fn typed_parse_matches_the_generic_reference_on_the_workspace_lockfile() {
        let content = include_str!("../../../../../../pnpm-lock.yaml");
        let typed = parse(content).expect("the workspace lockfile parses");
        assert_eq!(Some(&typed), parse_reference(content).ok().as_ref());
        let ParsedLockfile::Members(keys) = typed else {
            panic!("the workspace lockfile is an accepted version: {typed:?}");
        };
        assert!(keys.iter().any(|key| key == "."), "{keys:?}");
    }

    #[test]
    fn accepts_quoted_and_numeric_spellings_of_accepted_versions() {
        for version in ["'6.0'", "'9.0'", "6.0", "9.0", "9"] {
            let content = format!("lockfileVersion: {version}\nimporters:\n  .: {{}}\n");
            assert_eq!(
                parse(&content),
                Ok(ParsedLockfile::Members(vec![".".to_owned()])),
                "{version}"
            );
        }
    }

    #[test]
    fn unsupported_versions_never_yield_membership() {
        for version in ["'5.4'", "5.4", "'99.0'", "'9'", "'9.0.0'", "'v9'", "'5.3'"] {
            for body in ["importers:\n  .: {}\n", "", "importers: 3\n", "importers:\n  1: {}\n"] {
                let content = format!("lockfileVersion: {version}\n{body}");
                assert_eq!(
                    parse(&content),
                    Ok(ParsedLockfile::UnsupportedVersion),
                    "{content:?}"
                );
                assert_eq!(
                    parse_reference(&content).ok(),
                    parse(&content).ok(),
                    "{content:?}"
                );
            }
        }
    }

    #[test]
    fn rejects_non_string_importer_keys_instead_of_dropping_them() {
        for key in ["1", "true", "~", "[a]", "1.5"] {
            let content =
                format!("lockfileVersion: '9.0'\nimporters:\n  {key}: {{}}\n  packages/web: {{}}\n");
            assert!(parse(&content).is_err(), "{key}");
            assert!(parse_reference(&content).is_err(), "{key}");
        }
    }

    #[test]
    fn rejects_malformed_documents() {
        for (label, content) in [
            ("invalid YAML", "lockfileVersion: '9.0'\nimporters:\n  .: {\n"),
            (
                "invalid YAML after importers",
                "lockfileVersion: '9.0'\nimporters:\n  .: {}\npackages: [\n",
            ),
            ("empty document", ""),
            ("scalar document", "just a string\n"),
            ("sequence document", "- importers\n"),
            ("no version", "importers:\n  .: {}\n"),
            ("mapping version", "lockfileVersion: {a: 1}\nimporters:\n  .: {}\n"),
            ("no importers", "lockfileVersion: '9.0'\npackages: {}\n"),
            ("null importers", "lockfileVersion: '9.0'\nimporters:\n"),
            ("scalar importers", "lockfileVersion: '9.0'\nimporters: 3\n"),
            ("string importers", "lockfileVersion: '9.0'\nimporters: packages/web\n"),
            ("sequence importers", "lockfileVersion: '9.0'\nimporters:\n  - packages/web\n"),
            (
                "duplicate importer key",
                "lockfileVersion: '9.0'\nimporters:\n  a: {}\n  a: {}\n",
            ),
            (
                "duplicate importers key",
                "lockfileVersion: '9.0'\nimporters: {}\nimporters: {}\n",
            ),
        ] {
            assert!(parse(content).is_err(), "{label}");
            assert!(parse_reference(content).is_err(), "{label}");
        }
    }

    #[test]
    fn empty_importers_is_an_empty_member_set() {
        assert_eq!(
            parse("lockfileVersion: '9.0'\nimporters: {}\n"),
            Ok(ParsedLockfile::Members(Vec::new()))
        );
    }

    #[test]
    fn tolerates_duplicate_keys_in_skipped_sections_unlike_the_reference() {
        let content = "lockfileVersion: '9.0'\nimporters:\n  a: {}\npackages:\n  x: 1\n  x: 2\n";

        assert!(parse_reference(content).is_err());
        assert_eq!(
            parse(content),
            Ok(ParsedLockfile::Members(vec!["a".to_owned()]))
        );
    }
}
