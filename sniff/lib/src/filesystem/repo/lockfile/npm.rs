//! npm `package-lock.json` and `npm-shrinkwrap.json` parser.
//!
//! A path outside `node_modules` is not automatically a workspace: npm
//! records a `file:` dependency exactly like a member. The locked member set
//! is therefore every package path, and every `link: true` target, that the
//! lockfile's own root `workspaces` declarations match. Reading the locked
//! declarations rather than the current manifest keeps deleted members
//! visible. Accepted versions are listed in `accepted-versions.md` of
//! `2026-09-26-lockfile-corroboration`.

use std::collections::{BTreeSet, HashSet};
use std::fmt;

use globset::{GlobBuilder, GlobSet, GlobSetBuilder};
use serde::Deserialize;
use serde::de::{self, IgnoredAny, MapAccess, Visitor};

use super::{Outcome, ParsedLockfile};

/// `lockfileVersion` values with a real-tool fixture (npm 11.6.4). Version 1
/// (npm 6) has no `packages` object and is unsupported.
const ACCEPTED_VERSIONS: [u64; 2] = [2, 3];

/// Parse an npm lockfile.
///
/// Every `packages` record is scanned in one pass; only the root record's
/// `workspaces`, each record's path, and `link`/`resolved` are retained.
///
/// ## Returns
///
/// [`ParsedLockfile::UnsupportedVersion`] for any version outside the
/// accepted matrix, [`ParsedLockfile::AmbiguousMembership`] when the lockfile
/// records local package paths but no workspace declarations (or a
/// declaration that is not a valid glob), and otherwise the locked member
/// paths.
///
/// ## Errors
///
/// Invalid JSON, trailing content, a missing `lockfileVersion`, and, for an
/// accepted version, a missing `packages` object or root record, a duplicate
/// `packages` key, or a mistyped membership field.
pub(super) fn parse(content: &str) -> Outcome {
    let document = match serde_json::from_str::<NpmLockDocument>(content) {
        Ok(document) => document,
        // An unfamiliar body under an unsupported version is still that
        // version, not a parse failure.
        Err(error) => {
            return match serde_json::from_str::<NpmLockHeader>(content) {
                Ok(NpmLockHeader {
                    lockfile_version: Some(version),
                }) if !version.is_accepted() => Ok(ParsedLockfile::UnsupportedVersion),
                _ => Err(error.to_string()),
            };
        }
    };
    let Some(version) = document.lockfile_version else {
        return Err("npm lockfile has no lockfileVersion".to_owned());
    };
    if !version.is_accepted() {
        return Ok(ParsedLockfile::UnsupportedVersion);
    }
    let Some(packages) = document.packages else {
        return Err("npm lockfile has no packages object".to_owned());
    };
    if let Some(why) = packages.invalid {
        return Err(why.to_owned());
    }
    let Some(root) = packages.root else {
        return Err("npm lockfile has no root package record".to_owned());
    };
    let Some(declarations) = root.workspaces else {
        // Without locked declarations a local package path could be either a
        // workspace or a `file:` dependency.
        return Ok(if packages.paths.is_empty() {
            ParsedLockfile::Members(Vec::new())
        } else {
            ParsedLockfile::AmbiguousMembership
        });
    };
    let Some(matcher) = WorkspaceMatcher::new(declarations.patterns()) else {
        return Ok(ParsedLockfile::AmbiguousMembership);
    };
    let members: BTreeSet<String> = packages
        .paths
        .into_iter()
        .chain(packages.links)
        .filter(|path| matcher.is_match(path))
        .collect();
    Ok(ParsedLockfile::Members(members.into_iter().collect()))
}

/// The part of an npm lockfile that [`parse`] reads.
#[derive(Deserialize)]
struct NpmLockDocument {
    #[serde(rename = "lockfileVersion")]
    lockfile_version: Option<LockfileVersion>,
    packages: Option<Packages>,
}

/// Only the version, for classifying a document whose body did not parse.
#[derive(Deserialize)]
struct NpmLockHeader {
    #[serde(rename = "lockfileVersion")]
    lockfile_version: Option<LockfileVersion>,
}

/// npm writes an integer; anything else is a recognizable but unsupported
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

/// The `packages` object, reduced to what membership needs.
#[derive(Default)]
struct Packages {
    /// The `""` record.
    root: Option<RootRecord>,
    /// Every record path other than `""` with no `node_modules` component.
    paths: Vec<String>,
    /// The `resolved` target of every `link: true` record.
    links: Vec<String>,
    invalid: Option<&'static str>,
}

#[derive(Deserialize)]
struct RootRecord {
    workspaces: Option<Declarations>,
}

/// `package.json#workspaces`, as npm copies it into the root record.
#[derive(Deserialize)]
#[serde(untagged)]
enum Declarations {
    List(Vec<String>),
    Object { packages: Vec<String> },
}

impl Declarations {
    fn patterns(&self) -> &[String] {
        match self {
            Self::List(patterns) | Self::Object { packages: patterns } => patterns,
        }
    }
}

#[derive(Deserialize)]
struct PackageRecord {
    link: Option<bool>,
    resolved: Option<String>,
}

impl<'de> Deserialize<'de> for Packages {
    fn deserialize<D: de::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct PackagesVisitor;

        impl<'de> Visitor<'de> for PackagesVisitor {
            type Value = Packages;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("the npm lockfile packages object")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
                let mut packages = Packages::default();
                // `serde_json` keeps the last of two equal keys silently.
                let mut seen = HashSet::new();
                while let Some(key) = map.next_key::<String>()? {
                    if !seen.insert(key.clone()) {
                        packages.invalid.get_or_insert("duplicate packages key");
                    }
                    if key.is_empty() {
                        packages.root = Some(map.next_value::<RootRecord>()?);
                        continue;
                    }
                    let record = map.next_value::<PackageRecord>()?;
                    if record.link == Some(true)
                        && let Some(resolved) = record.resolved
                    {
                        packages.links.push(resolved);
                    }
                    if !is_installed(&key) {
                        packages.paths.push(key);
                    }
                }
                Ok(packages)
            }
        }

        deserializer.deserialize_map(PackagesVisitor)
    }
}

/// Whether a record path lies under a `node_modules` directory.
fn is_installed(path: &str) -> bool {
    path.split('/').any(|component| component == "node_modules")
}

/// The locked workspace declarations as a pure string matcher: no path from
/// the lockfile is ever looked up on disk.
struct WorkspaceMatcher {
    include: GlobSet,
    exclude: GlobSet,
}

impl WorkspaceMatcher {
    /// `None` when a declaration is not a valid glob, because membership then
    /// cannot be decided.
    fn new(patterns: &[String]) -> Option<Self> {
        let mut include = GlobSetBuilder::new();
        let mut exclude = GlobSetBuilder::new();
        for pattern in patterns {
            let (builder, pattern) = match pattern.trim().strip_prefix('!') {
                Some(negated) => (&mut exclude, negated),
                None => (&mut include, pattern.trim()),
            };
            let pattern = normalize_pattern(pattern);
            if pattern.is_empty() {
                continue;
            }
            // `*` stops at `/` and `**` spans it, as in the manifest-side
            // expander.
            builder.add(GlobBuilder::new(&pattern).literal_separator(true).build().ok()?);
        }
        Some(Self {
            include: include.build().ok()?,
            exclude: exclude.build().ok()?,
        })
    }

    fn is_match(&self, path: &str) -> bool {
        let path = normalize_pattern(path);
        self.include.is_match(&path) && !self.exclude.is_match(&path)
    }
}

/// `/`-separated, without `./` prefixes or a trailing separator, so
/// `./packages/*/` and `packages/*` declare the same members.
fn normalize_pattern(pattern: &str) -> String {
    pattern
        .replace('\\', "/")
        .split('/')
        .filter(|component| !component.is_empty() && *component != ".")
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn members(paths: &[&str]) -> Option<ParsedLockfile> {
        Some(ParsedLockfile::Members(
            paths.iter().map(|path| (*path).to_owned()).collect(),
        ))
    }

    /// Every shipped npm fixture, with the parse the accepted-version matrix
    /// specifies. `None` is `parse_failed`.
    fn corpus() -> Vec<(&'static str, &'static str, Option<ParsedLockfile>)> {
        let workspace = [".tools/hidden", "packages/alpha", "packages/beta"];
        vec![
            (
                "npm-11.6.4/workspace",
                include_str!("../../../../tests/fixtures/lockfiles/npm-11.6.4/workspace/package-lock.json"),
                members(&workspace),
            ),
            (
                "npm-11.6.4/workspace-lockfile-v2",
                include_str!("../../../../tests/fixtures/lockfiles/npm-11.6.4/workspace-lockfile-v2/package-lock.json"),
                members(&workspace),
            ),
            (
                "npm-11.6.4/shrinkwrap",
                include_str!("../../../../tests/fixtures/lockfiles/npm-11.6.4/shrinkwrap/npm-shrinkwrap.json"),
                members(&workspace),
            ),
            (
                "npm-6.14.18/single-project",
                include_str!("../../../../tests/fixtures/lockfiles/npm-6.14.18/single-project/package-lock.json"),
                Some(ParsedLockfile::UnsupportedVersion),
            ),
            (
                "npm-11.6.4/workspace-edited-stale-extra",
                include_str!("../../../../tests/fixtures/lockfiles/npm-11.6.4/workspace-edited-stale-extra/package-lock.json"),
                members(&[".tools/hidden", "packages/alpha", "packages/beta", "packages/gamma"]),
            ),
            (
                "npm-11.6.4/workspace-edited-missing",
                include_str!("../../../../tests/fixtures/lockfiles/npm-11.6.4/workspace-edited-missing/package-lock.json"),
                members(&[".tools/hidden", "packages/alpha"]),
            ),
            (
                "npm-11.6.4/workspace-edited-malformed-trailing",
                include_str!("../../../../tests/fixtures/lockfiles/npm-11.6.4/workspace-edited-malformed-trailing/package-lock.json"),
                None,
            ),
            (
                "npm-11.6.4/workspace-edited-duplicate-key",
                include_str!("../../../../tests/fixtures/lockfiles/npm-11.6.4/workspace-edited-duplicate-key/package-lock.json"),
                None,
            ),
            (
                "npm-11.6.4/workspace-edited-unknown-version",
                include_str!("../../../../tests/fixtures/lockfiles/npm-11.6.4/workspace-edited-unknown-version/package-lock.json"),
                Some(ParsedLockfile::UnsupportedVersion),
            ),
            (
                "npm-11.6.4/workspace-edited-missing-required-field",
                include_str!("../../../../tests/fixtures/lockfiles/npm-11.6.4/workspace-edited-missing-required-field/package-lock.json"),
                None,
            ),
        ]
    }

    #[test]
    fn every_shipped_npm_fixture_parses_as_the_matrix_specifies() {
        for (fixture, content, expected) in corpus() {
            assert_eq!(parse(content).ok(), expected, "{fixture}");
        }
    }

    /// The generic `serde_json::Value` reading of the same rule, kept as the
    /// extraction-parity oracle for accepted fixtures.
    fn reference_members(content: &str) -> Vec<String> {
        let document: serde_json::Value = serde_json::from_str(content).expect("valid JSON");
        let packages = document["packages"].as_object().expect("packages");
        let patterns: Vec<String> = packages[""]["workspaces"]
            .as_array()
            .expect("workspaces")
            .iter()
            .map(|pattern| pattern.as_str().expect("string").to_owned())
            .collect();
        let matcher = WorkspaceMatcher::new(&patterns).expect("valid globs");
        let mut members = BTreeSet::new();
        for (key, record) in packages {
            if !key.is_empty() && !is_installed(key) && matcher.is_match(key) {
                members.insert(key.clone());
            }
            if record["link"] == serde_json::Value::Bool(true)
                && let Some(resolved) = record["resolved"].as_str()
                && matcher.is_match(resolved)
            {
                members.insert(resolved.to_owned());
            }
        }
        members.into_iter().collect()
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
    fn a_file_dependency_outside_the_declarations_is_not_a_member() {
        let content = include_str!(
            "../../../../tests/fixtures/lockfiles/npm-11.6.4/workspace/package-lock.json"
        );
        assert!(content.contains("\"local-lib\": {"), "the fixture records the file: dependency");
        let Ok(ParsedLockfile::Members(members)) = parse(content) else {
            panic!("the fixture parses");
        };
        assert!(!members.iter().any(|member| member.contains("local-lib")), "{members:?}");
    }

    #[test]
    fn a_link_record_alone_records_a_declared_member() {
        let content = r#"{"lockfileVersion": 3, "packages": {
            "": {"workspaces": ["packages/*"]},
            "node_modules/web": {"resolved": "packages/web", "link": true},
            "node_modules/lib": {"resolved": "vendor/lib", "link": true}
        }}"#;
        assert_eq!(parse(content).ok(), members(&["packages/web"]));
    }

    #[test]
    fn negated_and_dotted_declarations_follow_minimatch() {
        let content = r#"{"lockfileVersion": 3, "packages": {
            "": {"workspaces": ["./packages/*/", "!packages/skip", "tools/**"]},
            "packages/keep": {}, "packages/skip": {}, "packages/a/b": {},
            "tools/x/y": {}, "packages/keep/node_modules/dep": {}
        }}"#;
        assert_eq!(parse(content).ok(), members(&["packages/keep", "tools/x/y"]));
    }

    #[test]
    fn object_form_declarations_are_read() {
        let content = r#"{"lockfileVersion": 2, "packages": {
            "": {"workspaces": {"packages": ["apps/*"], "nohoist": ["**"]}},
            "apps/web": {}
        }, "dependencies": {}}"#;
        assert_eq!(parse(content).ok(), members(&["apps/web"]));
    }

    #[test]
    fn undeclared_local_paths_are_ambiguous_but_an_empty_lock_is_not() {
        let with_local = r#"{"lockfileVersion": 3, "packages": {"": {}, "local-lib": {}}}"#;
        assert_eq!(parse(with_local), Ok(ParsedLockfile::AmbiguousMembership));

        let installed_only =
            r#"{"lockfileVersion": 3, "packages": {"": {}, "node_modules/x": {"version": "1.0.0"}}}"#;
        assert_eq!(parse(installed_only), Ok(ParsedLockfile::Members(Vec::new())));
    }

    #[test]
    fn an_invalid_declaration_glob_is_ambiguous() {
        let content = r#"{"lockfileVersion": 3, "packages": {"": {"workspaces": ["packages/[a"]}, "packages/a": {}}}"#;
        assert_eq!(parse(content), Ok(ParsedLockfile::AmbiguousMembership));
    }

    #[test]
    fn unsupported_versions_never_yield_membership() {
        for version in ["1", "4", "99", "\"3\"", "3.5", "null"] {
            for body in [
                r#""packages": {"": {"workspaces": ["a"]}, "a": {}}"#,
                r#""dependencies": {}"#,
                r#""packages": 3"#,
                r#""packages": {"a": {"link": "yes"}}"#,
            ] {
                let content = format!("{{\"lockfileVersion\": {version}, {body}}}");
                let parsed = parse(&content);
                if version == "null" {
                    assert!(parsed.is_err(), "{content}");
                } else {
                    assert_eq!(parsed, Ok(ParsedLockfile::UnsupportedVersion), "{content}");
                }
            }
        }
    }

    #[test]
    fn rejects_malformed_documents() {
        for (label, content) in [
            ("empty", ""),
            ("not an object", "[]"),
            ("no version", r#"{"packages": {"": {}}}"#),
            ("no packages", r#"{"lockfileVersion": 3}"#),
            ("no root record", r#"{"lockfileVersion": 3, "packages": {"a": {}}}"#),
            ("scalar packages", r#"{"lockfileVersion": 3, "packages": 3}"#),
            ("scalar record", r#"{"lockfileVersion": 3, "packages": {"": {}, "a": 1}}"#),
            (
                "mistyped link",
                r#"{"lockfileVersion": 3, "packages": {"": {}, "a": {"link": "yes"}}}"#,
            ),
            (
                "mistyped workspaces",
                r#"{"lockfileVersion": 3, "packages": {"": {"workspaces": [1]}}}"#,
            ),
            (
                "duplicate root record",
                r#"{"lockfileVersion": 3, "packages": {"": {}, "": {}}}"#,
            ),
            (
                "duplicate packages object",
                r#"{"lockfileVersion": 3, "packages": {"": {}}, "packages": {"": {}}}"#,
            ),
            ("trailing garbage", r#"{"lockfileVersion": 3, "packages": {"": {}}} x"#),
        ] {
            assert!(parse(content).is_err(), "{label}");
        }
    }
}
