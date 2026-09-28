//! `Cargo.lock` partial evidence (ruling R1).
//!
//! `Cargo.lock` records packages, not workspace paths, and local non-members
//! are source-less too, so exact membership cannot be recovered. A member
//! counts as present when a source-less entry has its name and resolved
//! manifest version. The result is `members_present` or `members_missing`,
//! never `match` or `mismatch`, and it never upgrades provenance.

use std::collections::{BTreeSet, HashSet};
use std::rc::Rc;

use biscuit_file::toml_crate;

use super::super::cargo::{cargo_package_name, cargo_package_version_with_root};
use super::super::detection::ManifestStore;
use super::super::manifest_index::{CargoLockDocument, CargoLockVersions, LockVersion};
use super::super::seed::PackageSeed;
use super::super::standard::MonorepoLayer;
use super::{LockfileObservation, LockfileReason, LockfileStatus, Outcome, ParsedLockfile, membership};

/// One `Cargo.lock` parse, shared by dependency-version enrichment and
/// corroboration so the file is read and parsed once per request.
pub(crate) struct CargoLock {
    /// The lenient version index dependency enrichment has always used.
    pub(crate) versions: CargoLockVersions,
    /// The strict corroboration view; `Err` is parse-failure detail.
    pub(crate) membership: Result<Rc<ParsedLockfile>, String>,
}

impl CargoLock {
    /// Parse `Cargo.lock` content.
    ///
    /// ## Errors
    ///
    /// Only invalid TOML. A document the version index accepts can still carry
    /// a corroboration failure in [`CargoLock::membership`].
    pub(crate) fn parse(content: &str) -> Result<Self, String> {
        let document: CargoLockDocument =
            toml_crate::from_str(content).map_err(|error| error.to_string())?;
        Ok(Self {
            membership: membership(&document).map(Rc::new),
            versions: CargoLockVersions::from_document(&document),
        })
    }
}

/// The corroboration view of a `Cargo.lock`: its source-less entries.
///
/// v3 and v4 carry `version`; a lockfile without it is v2 unless a
/// `[metadata]` table marks it as v1, which is unsupported.
fn membership(document: &CargoLockDocument) -> Outcome {
    match document.version {
        LockVersion::Integer(3 | 4) => {}
        LockVersion::Integer(_) => return Ok(ParsedLockfile::UnsupportedVersion),
        LockVersion::Other => return Err("Cargo.lock version is not an integer".to_owned()),
        LockVersion::Absent if document.metadata.is_some() => {
            return Ok(ParsedLockfile::UnsupportedVersion);
        }
        LockVersion::Absent => {}
    }
    let packages = &document.package;
    // Cargo writes an entry for every workspace member, so a lockfile without
    // one proves nothing, whereas the version index reads it as empty.
    if !packages.is_array {
        return Err("Cargo.lock has no [[package]] array".to_owned());
    }
    if packages.skipped > 0 {
        return Err("a Cargo.lock package entry is not a table".to_owned());
    }
    let mut sourceless = Vec::new();
    for entry in &packages.entries {
        let (Some(name), Some(version)) = (&entry.name, &entry.version) else {
            return Err("a Cargo.lock package entry has no string name or version".to_owned());
        };
        if entry.source.is_none() {
            sourceless.push((name.clone(), version.clone()));
        }
    }
    Ok(ParsedLockfile::CargoPackages(sourceless))
}

/// Check every non-root, non-excluded manifest member against the
/// source-less entries.
pub(super) fn compare(
    sourceless: &[(String, String)],
    layer: &MonorepoLayer,
    owned: Option<&[PackageSeed]>,
    store: &ManifestStore,
    paths: Vec<String>,
) -> LockfileObservation {
    let Some(owned) = owned else {
        return LockfileObservation::unverifiable(
            paths,
            LockfileReason::IncompleteManifestDiscovery,
        );
    };
    let locked: HashSet<(&str, &str)> = sourceless
        .iter()
        .map(|(name, version)| (name.as_str(), version.as_str()))
        .collect();
    let mut missing = BTreeSet::new();
    // `[workspace].exclude` directories are catalogued but are not members,
    // so Cargo never locks them as members. A directory matched by both
    // `members` and `exclude` has one seed of each here (they merge later), and
    // exclusion wins.
    let excluded: HashSet<&std::path::Path> = owned
        .iter()
        .filter(|seed| seed.is_excluded)
        .map(|seed| seed.path.as_path())
        .collect();
    for seed in owned
        .iter()
        .filter(|seed| !excluded.contains(seed.path.as_path()))
    {
        let member = match membership::manifest_member(&layer.root, &seed.path) {
            Ok(member) => member,
            Err(reason) => return LockfileObservation::unverifiable(paths, reason),
        };
        if member.is_empty() {
            continue;
        }
        let Some((name, version)) = resolved_identity(seed, layer, store) else {
            return LockfileObservation::unverifiable(
                paths,
                LockfileReason::IncompleteManifestDiscovery,
            );
        };
        if !locked.contains(&(name.as_str(), version.as_str())) {
            missing.insert(member);
        }
    }
    let status = if missing.is_empty() {
        LockfileStatus::MembersPresent
    } else {
        LockfileStatus::MembersMissing
    };
    LockfileObservation {
        missing: missing.into_iter().collect(),
        ..LockfileObservation::new(status, paths, Some(LockfileReason::SubsetOnly))
    }
}

/// A member's `(name, version)` as `Cargo.lock` records it, with workspace
/// version inheritance resolved and Cargo's `0.0.0` default for an omitted
/// version. `None` when the manifest cannot be read or parsed or the identity
/// cannot be resolved: the manifest-side set is then incomplete.
fn resolved_identity(
    seed: &PackageSeed,
    layer: &MonorepoLayer,
    store: &ManifestStore,
) -> Option<(String, String)> {
    let manifest = seed.path.join("Cargo.toml");
    let parsed = store.cargo(&manifest)?;
    let name = cargo_package_name(&parsed)?;
    let declares_version = parsed
        .get("package")
        .and_then(|package| package.get("version"))
        .is_some();
    if !declares_version {
        return Some((name, "0.0.0".to_owned()));
    }
    let root_manifest = layer.root.join("Cargo.toml");
    let root_parsed = store.cargo(&root_manifest);
    let (version, _, _) = cargo_package_version_with_root(
        &parsed,
        &manifest,
        &layer.root,
        &root_manifest,
        root_parsed.as_deref(),
    )?;
    Some((name, version))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn packages(entries: &[(&str, &str)]) -> Option<ParsedLockfile> {
        Some(ParsedLockfile::CargoPackages(
            entries
                .iter()
                .map(|(name, version)| ((*name).to_owned(), (*version).to_owned()))
                .collect(),
        ))
    }

    fn membership_of(content: &str) -> Result<ParsedLockfile, String> {
        let lock = CargoLock::parse(content)?;
        lock.membership.map(|parsed| (*parsed).clone())
    }

    /// Every shipped `Cargo.lock` fixture, with the parse the accepted-version
    /// matrix specifies. `None` is `parse_failed`.
    #[test]
    fn every_shipped_cargo_fixture_parses_as_the_matrix_specifies() {
        let workspace = [
            ("alpha", "0.3.0"),
            ("fixture-root", "0.1.0"),
            ("hidden-tool", "0.1.0"),
            ("itoa", "0.1.0"),
            ("local-lib", "0.1.0"),
        ];
        let cases: [(&str, &str, Option<ParsedLockfile>); 8] = [
            (
                "cargo-1.98.1/workspace",
                include_str!("../../../../tests/fixtures/lockfiles/cargo-1.98.1/workspace/Cargo.lock"),
                packages(&workspace),
            ),
            (
                "cargo-1.77.2/workspace-v3",
                include_str!("../../../../tests/fixtures/lockfiles/cargo-1.77.2/workspace-v3/Cargo.lock"),
                packages(&workspace),
            ),
            (
                "cargo-1.52.0/workspace-v2",
                include_str!("../../../../tests/fixtures/lockfiles/cargo-1.52.0/workspace-v2/Cargo.lock"),
                packages(&workspace),
            ),
            (
                "cargo-1.98.1/workspace-edited-stale-extra",
                include_str!("../../../../tests/fixtures/lockfiles/cargo-1.98.1/workspace-edited-stale-extra/Cargo.lock"),
                packages(&[
                    ("alpha", "0.3.0"),
                    ("fixture-root", "0.1.0"),
                    ("gamma", "0.1.0"),
                    ("hidden-tool", "0.1.0"),
                    ("itoa", "0.1.0"),
                    ("local-lib", "0.1.0"),
                ]),
            ),
            (
                "cargo-1.98.1/workspace-edited-missing",
                include_str!("../../../../tests/fixtures/lockfiles/cargo-1.98.1/workspace-edited-missing/Cargo.lock"),
                packages(&[
                    ("alpha", "0.3.0"),
                    ("fixture-root", "0.1.0"),
                    ("hidden-tool", "0.1.0"),
                    ("local-lib", "0.1.0"),
                ]),
            ),
            (
                "cargo-1.98.1/workspace-edited-malformed-trailing",
                include_str!("../../../../tests/fixtures/lockfiles/cargo-1.98.1/workspace-edited-malformed-trailing/Cargo.lock"),
                None,
            ),
            (
                "cargo-1.98.1/workspace-edited-unknown-version",
                include_str!("../../../../tests/fixtures/lockfiles/cargo-1.98.1/workspace-edited-unknown-version/Cargo.lock"),
                Some(ParsedLockfile::UnsupportedVersion),
            ),
            (
                "cargo-1.98.1/workspace-edited-missing-required-field",
                include_str!("../../../../tests/fixtures/lockfiles/cargo-1.98.1/workspace-edited-missing-required-field/Cargo.lock"),
                None,
            ),
        ];
        for (fixture, content, expected) in cases {
            let parsed = membership_of(content).ok().map(|parsed| match parsed {
                ParsedLockfile::CargoPackages(mut entries) => {
                    entries.sort();
                    ParsedLockfile::CargoPackages(entries)
                }
                other => other,
            });
            assert_eq!(parsed, expected, "{fixture}");
        }
    }

    #[test]
    fn a_registry_entry_sharing_a_member_name_is_not_source_less() {
        let content = include_str!(
            "../../../../tests/fixtures/lockfiles/cargo-1.98.1/workspace-edited-missing/Cargo.lock"
        );
        let lock = CargoLock::parse(content).expect("valid TOML");
        let ParsedLockfile::CargoPackages(entries) = &*lock.membership.expect("accepted") else {
            panic!("an accepted lockfile yields packages");
        };
        assert!(entries.iter().all(|(name, _)| name != "itoa"), "{entries:?}");
        // The version index still resolves the registry crate.
        assert_eq!(lock.versions.resolve("itoa"), Some("1.0.18".to_owned()));
    }

    #[test]
    fn classifies_versions_by_number_and_signature() {
        let body = "[[package]]\nname = \"a\"\nversion = \"0.1.0\"\n";
        for (header, expected) in [
            ("version = 3\n", packages(&[("a", "0.1.0")])),
            ("version = 4\n", packages(&[("a", "0.1.0")])),
            ("", packages(&[("a", "0.1.0")])),
            ("version = 5\n", Some(ParsedLockfile::UnsupportedVersion)),
            ("version = 1\n", Some(ParsedLockfile::UnsupportedVersion)),
            ("version = \"4\"\n", None),
        ] {
            assert_eq!(membership_of(&format!("{header}{body}")).ok(), expected, "{header:?}");
        }
        // v1: no `version` key and a `[metadata]` checksum table.
        let v1 = format!("{body}\n[metadata]\n\"checksum a 0.1.0\" = \"abc\"\n");
        assert_eq!(membership_of(&v1), Ok(ParsedLockfile::UnsupportedVersion));
    }

    #[test]
    fn rejects_accepted_lockfiles_without_complete_package_records() {
        for (label, content) in [
            ("no package array", "version = 4\n"),
            ("package is a table", "version = 4\n[package]\nname = \"a\"\nversion = \"0.1.0\"\n"),
            ("entry without a name", "version = 4\n[[package]]\nversion = \"0.1.0\"\n"),
            ("entry without a version", "version = 4\n[[package]]\nname = \"a\"\n"),
            ("non-table entry", "version = 4\npackage = [\"a\"]\n"),
            ("integer name", "version = 4\npackage = [{ name = 1, version = \"0.1.0\" }]\n"),
        ] {
            let lock = CargoLock::parse(content).expect("valid TOML");
            assert!(lock.membership.is_err(), "{label}");
        }
        assert!(CargoLock::parse("[[package]\n").is_err());
    }

    /// The shared parse keeps `CargoLockVersions::resolve` byte for byte:
    /// the version index built from the corroboration document equals the
    /// generic `toml::Value` reference on every shipped fixture and on the
    /// workspace's own lockfile.
    #[test]
    fn version_index_is_unchanged_by_the_shared_parse() {
        for content in [
            include_str!("../../../../tests/fixtures/lockfiles/cargo-1.98.1/workspace/Cargo.lock"),
            include_str!("../../../../tests/fixtures/lockfiles/cargo-1.77.2/workspace-v3/Cargo.lock"),
            include_str!("../../../../tests/fixtures/lockfiles/cargo-1.52.0/workspace-v2/Cargo.lock"),
            include_str!("../../../../tests/fixtures/lockfiles/cargo-1.98.1/workspace-edited-missing-required-field/Cargo.lock"),
            include_str!("../../../../../../Cargo.lock"),
        ] {
            let lock = CargoLock::parse(content).expect("valid TOML");
            let reference = CargoLockVersions::parse_reference(content).expect("valid TOML");
            assert!(lock.versions == reference);
            for name in ["alpha", "itoa", "serde", "sniff", "missing"] {
                assert_eq!(lock.versions.resolve(name), reference.resolve(name), "{name}");
            }
        }
    }
}
