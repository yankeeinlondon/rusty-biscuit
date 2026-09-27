//! Lockfile provenance fixture matrix (`2026-09-21-lockfile-provenance-cost`,
//! AC3, extended by `2026-09-26-lockfile-corroboration`): each lockfile
//! authority (Cargo, pnpm, uv) crossed with each lockfile state, detected
//! through the public API under a corroborating request and a declining one,
//! with hand-written expected results and fresh work counters. Each cell also
//! asserts the complete serialized `RepoInfo` against a hand-written document.
//!
//! `2026-09-26-lockfile-corroboration` changed three expectations on purpose:
//! every layer now carries a `lockfile` object; Cargo reports subset evidence
//! (`members_present`/`members_missing`) and never upgrades provenance; and
//! the uv lockfile uses uv's real `[manifest]` layout with the root excluded
//! from comparison. An absent lockfile is now probed and never opened.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use sniff::filesystem::repo::{LockfileObservation, detect_repo_with_request};
use sniff::filesystem::{
    MonorepoLayer, MonorepoStandard, PackageEcosystem, PackageProvenance, RepoInfo, detect_repo,
    detect_repo_structure,
};
use sniff::performance::{PerformanceCollector, counters, with_current_collector};
use sniff::request::RepoRequest;
use tempfile::TempDir;

#[derive(Debug, Clone, Copy)]
enum Authority {
    Cargo,
    Pnpm,
    Uv,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LockState {
    Matching,
    ExtraMember,
    MissingMember,
    Absent,
    Unparseable,
}

/// The package identity every request tier reports.
#[derive(Debug, PartialEq, Eq)]
struct PackageIdentity {
    relative: String,
    path: PathBuf,
    name: String,
    package_area: String,
    standard: MonorepoStandard,
    ecosystem: PackageEcosystem,
    provenance: PackageProvenance,
}

struct Fixture {
    _dir: TempDir,
    root: PathBuf,
}

fn write(root: &Path, relative: &str, content: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().expect("fixture file has a parent")).expect("create parent");
    fs::write(path, content).expect("write fixture file");
}

fn build_fixture(authority: Authority, state: LockState) -> Fixture {
    let dir = TempDir::new().expect("tempdir");
    let root = dir.path().to_path_buf();
    let lockfile = match authority {
        Authority::Cargo => {
            write(
                &root,
                "Cargo.toml",
                "[workspace]\nmembers = [\"crates/*\"]\nresolver = \"2\"\n",
            );
            for name in ["alpha", "beta"] {
                write(
                    &root,
                    &format!("crates/{name}/Cargo.toml"),
                    &format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\n"),
                );
            }
            let entry =
                |name: &str| format!("[[package]]\nname = \"{name}\"\nversion = \"0.1.0\"\n\n");
            let body = match state {
                LockState::Matching => Some(entry("alpha") + &entry("beta")),
                LockState::ExtraMember => Some(entry("alpha") + &entry("beta") + &entry("gamma")),
                LockState::MissingMember => Some(entry("alpha")),
                LockState::Absent => None,
                LockState::Unparseable => Some("[[package]\nname = \n".to_owned()),
            };
            body.map(|body| ("Cargo.lock", format!("version = 3\n\n{body}")))
        }
        Authority::Pnpm => {
            write(&root, "package.json", r#"{"name":"root","private":true}"#);
            write(
                &root,
                "pnpm-workspace.yaml",
                "packages:\n  - \"packages/*\"\n",
            );
            for name in ["web", "ui"] {
                write(
                    &root,
                    &format!("packages/{name}/package.json"),
                    &format!(r#"{{"name":"{name}","version":"1.0.0"}}"#),
                );
            }
            let importers = match state {
                LockState::Matching => Some("  .: {}\n  packages/web: {}\n  packages/ui: {}\n"),
                LockState::ExtraMember => {
                    Some("  .: {}\n  packages/web: {}\n  packages/ui: {}\n  packages/gone: {}\n")
                }
                LockState::MissingMember => Some("  .: {}\n  packages/web: {}\n"),
                LockState::Absent => None,
                LockState::Unparseable => Some("  packages/web: {\n"),
            };
            importers.map(|importers| {
                (
                    "pnpm-lock.yaml",
                    format!("lockfileVersion: '9.0'\nimporters:\n{importers}"),
                )
            })
        }
        Authority::Uv => {
            write(
                &root,
                "pyproject.toml",
                "[project]\nname = \"root-py\"\nversion = \"0.1.0\"\n\n\
                 [tool.uv.workspace]\nmembers = [\"py/*\"]\n",
            );
            for name in ["lib-a", "lib-b"] {
                write(
                    &root,
                    &format!("py/{name}/pyproject.toml"),
                    &format!("[project]\nname = \"{name}\"\nversion = \"0.1.0\"\n"),
                );
            }
            let entry = |name: &str, source: &str| {
                format!("[[package]]\nname = \"{name}\"\nversion = \"0.1.0\"\nsource = {source}\n\n")
            };
            let root = entry("root-py", r#"{ virtual = "." }"#);
            let lib = |name: &str| entry(name, &format!(r#"{{ editable = "py/{name}" }}"#));
            let lock = |names: &str, packages: String| {
                format!(
                    "version = 1\nrevision = 3\n\n[manifest]\nmembers = [{names}]\n\n{packages}"
                )
            };
            let body = match state {
                LockState::Matching => Some(lock(
                    r#""lib-a", "lib-b", "root-py""#,
                    lib("lib-a") + &lib("lib-b") + &root,
                )),
                LockState::ExtraMember => Some(lock(
                    r#""lib-a", "lib-b", "lib-gone", "root-py""#,
                    lib("lib-a") + &lib("lib-b") + &lib("lib-gone") + &root,
                )),
                LockState::MissingMember => {
                    Some(lock(r#""lib-a", "root-py""#, lib("lib-a") + &root))
                }
                LockState::Absent => None,
                LockState::Unparseable => Some("version = 1\nrevision = 3\n[manifest\n".to_owned()),
            };
            body.map(|body| ("uv.lock", body))
        }
    };
    if let Some((name, content)) = lockfile {
        write(&root, name, &content);
    }
    Fixture { _dir: dir, root }
}

/// A hand-written `lockfile` object in its wire spelling. `paths` is the
/// authority's lockfile unless the state is [`LockState::Absent`].
#[derive(Debug, Clone, Copy)]
struct Expected {
    status: &'static str,
    reason: Option<&'static str>,
    extra: &'static [&'static str],
    missing: &'static [&'static str],
}

const fn expected(
    status: &'static str,
    reason: Option<&'static str>,
    extra: &'static [&'static str],
    missing: &'static [&'static str],
) -> Expected {
    Expected {
        status,
        reason,
        extra,
        missing,
    }
}

/// Hand-written expected layer answer for one matrix cell under a request that
/// corroborates: `(layer provenance, lockfile object)`.
///
/// pnpm and uv compare exact member sets. Cargo can only check that each
/// member has a source-less entry with its name and version (ruling R1), so a
/// stale extra entry is invisible to it and it never upgrades provenance.
const CORROBORATED: &[(Authority, LockState, PackageProvenance, Expected)] = {
    use Authority::{Cargo, Pnpm, Uv};
    use LockState::{Absent, ExtraMember, Matching, MissingMember, Unparseable};
    use PackageProvenance::{Globbed, Lockfile};
    const SUBSET: Option<&str> = Some("subset_only");
    const PARSE_FAILED: Option<&str> = Some("parse_failed");
    &[
        (Cargo, Matching, Globbed, expected("members_present", SUBSET, &[], &[])),
        (Cargo, ExtraMember, Globbed, expected("members_present", SUBSET, &[], &[])),
        (
            Cargo,
            MissingMember,
            Globbed,
            expected("members_missing", SUBSET, &[], &["crates/beta"]),
        ),
        (Cargo, Absent, Globbed, expected("absent", None, &[], &[])),
        (Cargo, Unparseable, Globbed, expected("unreadable", PARSE_FAILED, &[], &[])),
        (Pnpm, Matching, Lockfile, expected("match", None, &[], &[])),
        (
            Pnpm,
            ExtraMember,
            Globbed,
            expected("mismatch", None, &["packages/gone"], &[]),
        ),
        (
            Pnpm,
            MissingMember,
            Globbed,
            expected("mismatch", None, &[], &["packages/ui"]),
        ),
        (Pnpm, Absent, Globbed, expected("absent", None, &[], &[])),
        (Pnpm, Unparseable, Globbed, expected("unreadable", PARSE_FAILED, &[], &[])),
        (Uv, Matching, Lockfile, expected("match", None, &[], &[])),
        (
            Uv,
            ExtraMember,
            Globbed,
            expected("mismatch", None, &["py/lib-gone"], &[]),
        ),
        (
            Uv,
            MissingMember,
            Globbed,
            expected("mismatch", None, &[], &["py/lib-b"]),
        ),
        (Uv, Absent, Globbed, expected("absent", None, &[], &[])),
        (Uv, Unparseable, Globbed, expected("unreadable", PARSE_FAILED, &[], &[])),
    ]
};

/// The answer a request declining corroboration reports: a present lockfile
/// is probed but not read.
fn declined(state: LockState) -> Expected {
    if state == LockState::Absent {
        expected("absent", None, &[], &[])
    } else {
        expected("not_requested", Some("request_disabled"), &[], &[])
    }
}

/// `expected` as the serialized `lockfile` object for `authority` in `state`.
fn lockfile_json(authority: Authority, state: LockState, expected: Expected) -> serde_json::Value {
    let paths: Vec<&str> = if state == LockState::Absent {
        Vec::new()
    } else {
        vec![shape(authority).lockfile]
    };
    serde_json::json!({
        "status": expected.status,
        "paths": paths,
        "reason": expected.reason,
        "extra": expected.extra,
        "missing": expected.missing,
    })
}

/// `expected` as the typed observation, parsed from its hand-written wire form.
fn lockfile_observation(
    authority: Authority,
    state: LockState,
    expected: Expected,
) -> LockfileObservation {
    serde_json::from_value(lockfile_json(authority, state, expected))
        .expect("the expected lockfile object uses the wire vocabulary")
}

/// The layer every request reports for `authority`, with `provenance` and
/// `lockfile` supplied by the caller. Layer packages are sorted and use `/`
/// separators.
fn expected_layer(
    authority: Authority,
    root: &Path,
    provenance: PackageProvenance,
    lockfile: LockfileObservation,
) -> MonorepoLayer {
    let (standard, root_is_package, packages): (_, _, &[&str]) = match authority {
        Authority::Cargo => (
            MonorepoStandard::CargoWorkspace,
            false,
            &["crates/alpha", "crates/beta"],
        ),
        Authority::Pnpm => (
            MonorepoStandard::PnpmWorkspaces,
            false,
            &["packages/ui", "packages/web"],
        ),
        Authority::Uv => (
            MonorepoStandard::UvWorkspace,
            true,
            &["", "py/lib-a", "py/lib-b"],
        ),
    };
    MonorepoLayer {
        root: root.to_path_buf(),
        authority: standard,
        orchestrators: Vec::new(),
        provenance,
        lockfile,
        root_is_package,
        packages: packages.iter().map(|p| (*p).to_owned()).collect(),
    }
}

/// The complete package catalog for `authority`, sorted by relative path, with
/// every package carrying `provenance`.
fn expected_packages(
    authority: Authority,
    root: &Path,
    provenance: PackageProvenance,
) -> Vec<PackageIdentity> {
    let (standard, ecosystem, rows): (_, _, &[(&str, &str, &str)]) = match authority {
        Authority::Cargo => (
            MonorepoStandard::CargoWorkspace,
            PackageEcosystem::Cargo,
            &[
                ("crates/alpha", "alpha", "crates"),
                ("crates/beta", "beta", "crates"),
            ],
        ),
        Authority::Pnpm => (
            MonorepoStandard::PnpmWorkspaces,
            PackageEcosystem::Node,
            &[
                ("packages/ui", "ui", "packages"),
                ("packages/web", "web", "packages"),
            ],
        ),
        Authority::Uv => (
            MonorepoStandard::UvWorkspace,
            PackageEcosystem::Python,
            &[
                ("", "root-py", ""),
                ("py/lib-a", "lib-a", "py"),
                ("py/lib-b", "lib-b", "py"),
            ],
        ),
    };
    rows.iter()
        .map(|(relative, name, package_area)| PackageIdentity {
            relative: (*relative).to_owned(),
            path: relative
                .split('/')
                .filter(|s| !s.is_empty())
                .fold(root.to_path_buf(), |p, s| p.join(s)),
            name: (*name).to_owned(),
            package_area: (*package_area).to_owned(),
            standard,
            ecosystem,
            provenance,
        })
        .collect()
}

/// The layer with its package list in the same normalized, sorted form
/// [`expected_layer`] uses.
fn normalized_layer(repo: &RepoInfo) -> MonorepoLayer {
    assert_eq!(repo.monorepo_layers.len(), 1, "{:?}", repo.monorepo_layers);
    let mut layer = repo.monorepo_layers[0].clone();
    for package in &mut layer.packages {
        *package = package.replace('\\', "/");
    }
    layer.packages.sort();
    layer
}

fn package_identities(repo: &RepoInfo) -> Vec<PackageIdentity> {
    let mut identities: Vec<_> = repo
        .packages
        .as_deref()
        .expect("a monorepo reports its package catalog")
        .iter()
        .map(|package| PackageIdentity {
            relative: package.relative.replace('\\', "/"),
            path: package.path.clone(),
            name: package.name.clone(),
            package_area: package.package_area.replace('\\', "/"),
            standard: package.standard,
            ecosystem: package.ecosystem,
            provenance: package.provenance,
        })
        .collect();
    identities.sort_by(|a, b| a.relative.cmp(&b.relative));
    identities
}

fn assert_repo(
    repo: &RepoInfo,
    authority: Authority,
    state: LockState,
    root: &Path,
    provenance: PackageProvenance,
    lockfile: Expected,
    context: &str,
) {
    let lockfile = lockfile_observation(authority, state, lockfile);
    assert!(repo.is_monorepo, "{context}");
    assert_eq!(repo.root, root, "{context}");
    let standards: Vec<_> = repo
        .monorepo_standards
        .iter()
        .map(|s| (s.standard, s.root.clone()))
        .collect();
    assert_eq!(
        standards,
        vec![(
            expected_layer(authority, root, provenance, lockfile.clone()).authority,
            root.to_path_buf()
        )],
        "{context}"
    );
    assert_eq!(
        normalized_layer(repo),
        expected_layer(authority, root, provenance, lockfile),
        "{context}"
    );
    // Packages inherit the layer's provenance.
    assert_eq!(
        package_identities(repo),
        expected_packages(authority, root, provenance),
        "{context}"
    );
}

/// Run `detect` under a fresh collector and return its result with the counters.
fn measured<T>(detect: impl FnOnce() -> T) -> (T, BTreeMap<String, u64>) {
    let collector = PerformanceCollector::new_shared();
    let result = with_current_collector(Some(Arc::clone(&collector)), detect);
    (result, collector.snapshot(Duration::ZERO).counters)
}

fn counter(counts: &BTreeMap<String, u64>, name: &str) -> u64 {
    counts.get(name).copied().unwrap_or(0)
}

/// Which package fields the request fills beyond identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tier {
    /// `RepoRequest::structure()` and `detect_repo_structure`.
    Structure,
    /// `detect_repo`.
    Full,
}

/// Stands in for the fixture's temporary root in normalized JSON.
const ROOT: &str = "<root>";
/// Stands in for `DetectedStandard::binary`, a lookup on the host's `PATH`
/// that no fixture file controls.
const HOST_BINARY: &str = "<host PATH lookup>";

/// Fixture facts the complete expected JSON is built from, per authority.
struct Shape {
    tool: &'static str,
    standard: &'static str,
    ecosystem: &'static str,
    marker: &'static str,
    lockfile: &'static str,
    root_is_package: bool,
    /// Layer member order as reported: members first, then a root package.
    layer_packages: &'static [&'static str],
    package_manager: &'static str,
    test_runner: &'static str,
    /// Sorted by `relative`.
    packages: &'static [PackageRow],
}

/// `(relative, name, package_area, version, manifest file name)`.
type PackageRow = (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
);

fn shape(authority: Authority) -> Shape {
    match authority {
        Authority::Cargo => Shape {
            tool: "cargo",
            standard: "cargo-workspace",
            ecosystem: "cargo",
            marker: "Cargo.toml",
            lockfile: "Cargo.lock",
            root_is_package: false,
            layer_packages: &["crates/alpha", "crates/beta"],
            package_manager: "cargo",
            test_runner: "CargoTest",
            packages: &[
                ("crates/alpha", "alpha", "crates", "0.1.0", "Cargo.toml"),
                ("crates/beta", "beta", "crates", "0.1.0", "Cargo.toml"),
            ],
        },
        Authority::Pnpm => Shape {
            tool: "pnpm",
            standard: "pnpm-workspaces",
            ecosystem: "node",
            marker: "pnpm-workspace.yaml",
            lockfile: "pnpm-lock.yaml",
            root_is_package: false,
            layer_packages: &["packages/ui", "packages/web"],
            package_manager: "npm",
            test_runner: "NodeTest",
            packages: &[
                ("packages/ui", "ui", "packages", "1.0.0", "package.json"),
                ("packages/web", "web", "packages", "1.0.0", "package.json"),
            ],
        },
        Authority::Uv => Shape {
            tool: "uv",
            standard: "uv-workspace",
            ecosystem: "python",
            marker: "pyproject.toml",
            lockfile: "uv.lock",
            root_is_package: true,
            layer_packages: &["py/lib-a", "py/lib-b", ""],
            package_manager: "pip",
            test_runner: "Unittest",
            packages: &[
                ("", "root-py", "", "0.1.0", "pyproject.toml"),
                ("py/lib-a", "lib-a", "py", "0.1.0", "pyproject.toml"),
                ("py/lib-b", "lib-b", "py", "0.1.0", "pyproject.toml"),
            ],
        },
    }
}

/// `rest` under `base` with `/`, where an empty `base` or `rest` adds no
/// separator.
fn under(base: &str, rest: &str) -> String {
    match (base.is_empty(), rest.is_empty()) {
        (true, _) => rest.to_owned(),
        (_, true) => base.to_owned(),
        _ => format!("{base}/{rest}"),
    }
}

/// The complete serialized `RepoInfo` for one matrix cell, after
/// [`normalized_json`]. Every top-level, standard, layer, and package field is
/// spelled out; nothing is copied from detection output.
fn expected_json(
    authority: Authority,
    state: LockState,
    tier: Tier,
    provenance: PackageProvenance,
    lockfile: Expected,
) -> serde_json::Value {
    use serde_json::{Value, json};

    let shape = shape(authority);
    let provenance = match provenance {
        PackageProvenance::Lockfile => "lockfile",
        PackageProvenance::Globbed => "globbed",
        other => panic!("the matrix never reports {other:?}"),
    };
    let packages: Vec<Value> = shape
        .packages
        .iter()
        .map(|&(relative, name, package_area, version, manifest)| {
            let mut package = json!({
                "path": under(ROOT, relative),
                "relative": relative,
                "package_area": package_area,
                "name": name,
                "ecosystem": shape.ecosystem,
                "standard": shape.standard,
                "provenance": provenance,
                "primary_language": null,
                "package_managers": [],
                "version": version,
            });
            if tier == Tier::Full {
                // The root package's tree holds the lockfile; a member's does
                // not.
                let mut files = vec![under(relative, manifest)];
                if relative.is_empty() && state != LockState::Absent {
                    files.push(shape.lockfile.to_owned());
                }
                // Pre-existing behavior pinned as the pre-change contract, not
                // endorsed: `configuration` prefixes the package's relative
                // path onto an already repository-relative file path.
                let configuration: Vec<String> =
                    files.iter().map(|file| under(relative, file)).collect();
                let fields = package.as_object_mut().expect("package is an object");
                fields.insert(
                    "file_associations".into(),
                    json!([{
                        "association": "configuration",
                        "file_count": files.len(),
                        "percentage": 100.0,
                        "files": files,
                    }]),
                );
                fields.insert("configuration".into(), json!(configuration));
                fields.insert("package_managers".into(), json!([shape.package_manager]));
                fields.insert(
                    "test_runners".into(),
                    json!([{
                        "runner": shape.test_runner,
                        "source": { "kind": "ecosystem_default" },
                    }]),
                );
                if relative.is_empty() {
                    let nested: Vec<&str> = shape.packages[1..]
                        .iter()
                        .map(|&(_, nested_name, ..)| nested_name)
                        .collect();
                    fields.insert("nested_packages".into(), json!(nested));
                }
            }
            package
        })
        .collect();

    let layer = json!({
        "root": ROOT,
        "authority": shape.standard,
        "orchestrators": [],
        "provenance": provenance,
        "lockfile": lockfile_json(authority, state, lockfile),
        "root_is_package": shape.root_is_package,
        "packages": shape.layer_packages,
    });

    json!({
        "is_monorepo": true,
        "root": ROOT,
        "packages": packages,
        "monorepo_standards": [{
            "standard": shape.standard,
            "root": ROOT,
            "matched_markers": [format!("{ROOT}/{}", shape.marker)],
            "binary": HOST_BINARY,
            "confidence": "marker-confirmed",
        }],
        "monorepo_layers": [layer],
        "standalone_lockfiles": [],
    })
}

/// `repo` serialized, with the temporary root replaced by [`ROOT`], `\`
/// separators turned into `/`, and each standard's host `PATH` lookup
/// replaced by [`HOST_BINARY`] once any binary it found is checked to be the
/// authority's tool.
fn normalized_json(repo: &RepoInfo, root: &Path, authority: Authority) -> serde_json::Value {
    fn normalize(value: &mut serde_json::Value, root: &str) {
        match value {
            serde_json::Value::String(text) => {
                *text = text.replace(root, ROOT).replace('\\', "/");
            }
            serde_json::Value::Array(items) => {
                items.iter_mut().for_each(|item| normalize(item, root));
            }
            serde_json::Value::Object(fields) => {
                fields.values_mut().for_each(|item| normalize(item, root));
            }
            _ => {}
        }
    }

    let mut value = serde_json::to_value(repo).expect("RepoInfo serializes");
    let standards = value["monorepo_standards"]
        .as_array_mut()
        .expect("monorepo_standards array");
    for standard in standards {
        let binary = standard["binary"].take();
        if !binary.is_null() {
            assert_eq!(binary["name"], shape(authority).tool, "{binary}");
        }
        standard["binary"] = serde_json::Value::from(HOST_BINARY);
    }
    normalize(&mut value, root.to_str().expect("temporary root is UTF-8"));
    value
}

#[allow(clippy::too_many_arguments)]
fn assert_complete_json(
    repo: &RepoInfo,
    root: &Path,
    authority: Authority,
    state: LockState,
    tier: Tier,
    provenance: PackageProvenance,
    lockfile: Expected,
    context: &str,
) {
    let actual = normalized_json(repo, root, authority);
    let expected = expected_json(authority, state, tier, provenance, lockfile);
    assert!(
        actual == expected,
        "{context}\n--- expected\n{}\n--- actual\n{}",
        serde_json::to_string_pretty(&expected).expect("pretty expected"),
        serde_json::to_string_pretty(&actual).expect("pretty actual"),
    );
}

#[test]
fn corroborating_requests_report_hand_written_provenance_for_every_lockfile_state() {
    for &(authority, state, provenance, lockfile) in CORROBORATED {
        let fixture = build_fixture(authority, state);
        let root = fixture.root.as_path();
        // An absent lockfile is probed and never opened.
        let expected_work = u64::from(state != LockState::Absent);

        let request = RepoRequest::structure().with_lockfile_provenance(true);
        let (repo, counts) = measured(|| detect_repo_with_request(root, &request));
        let repo = repo
            .expect("detection succeeds")
            .expect("fixture is a workspace");
        let context = format!("{authority:?} × {state:?}, opted-in structure: {counts:?}");
        assert_repo(&repo, authority, state, root, provenance, lockfile, &context);
        assert_complete_json(
            &repo,
            root,
            authority,
            state,
            Tier::Structure,
            provenance,
            lockfile,
            &context,
        );
        assert_eq!(
            counter(&counts, counters::REPO_LOCKFILE_READS),
            expected_work,
            "{context}"
        );
        assert_eq!(
            counter(&counts, counters::REPO_LOCKFILE_PARSES),
            expected_work,
            "{context}"
        );

        let (repo, counts) = measured(|| detect_repo(root));
        let repo = repo
            .expect("detection succeeds")
            .expect("fixture is a workspace");
        let context = format!("{authority:?} × {state:?}, full: {counts:?}");
        assert_repo(&repo, authority, state, root, provenance, lockfile, &context);
        assert_complete_json(
            &repo,
            root,
            authority,
            state,
            Tier::Full,
            provenance,
            lockfile,
            &context,
        );
        // Cargo's dependency enrichment shares the corroboration parse.
        assert_eq!(
            counter(&counts, counters::REPO_LOCKFILE_READS),
            expected_work,
            "{context}"
        );
        assert_eq!(
            counter(&counts, counters::REPO_LOCKFILE_PARSES),
            expected_work,
            "{context}"
        );
    }
}

#[test]
fn declining_requests_report_manifest_provenance_and_read_no_lockfile() {
    for &(authority, state, _, _) in CORROBORATED {
        let fixture = build_fixture(authority, state);
        let root = fixture.root.as_path();

        for (label, detect) in [
            (
                "structure request",
                (|root: &Path| {
                    detect_repo_with_request(root, &RepoRequest::structure())
                }) as fn(&Path) -> sniff::Result<Option<RepoInfo>>,
            ),
            ("detect_repo_structure", detect_repo_structure),
        ] {
            let (repo, counts) = measured(|| detect(root));
            let repo = repo
                .expect("detection succeeds")
                .expect("fixture is a workspace");
            let context = format!("{authority:?} × {state:?}, {label}: {counts:?}");
            assert_repo(
                &repo,
                authority,
                state,
                root,
                PackageProvenance::Globbed,
                declined(state),
                &context,
            );
            assert_complete_json(
                &repo,
                root,
                authority,
                state,
                Tier::Structure,
                PackageProvenance::Globbed,
                declined(state),
                &context,
            );
            assert_eq!(
                counter(&counts, counters::REPO_LOCKFILE_READS),
                0,
                "{context}"
            );
            assert_eq!(
                counter(&counts, counters::REPO_LOCKFILE_PARSES),
                0,
                "{context}"
            );
            assert!(
                counter(&counts, counters::REPO_LOCKFILE_PROBES) >= 1,
                "the layer's lockfile is still probed: {context}"
            );
        }
    }
}

#[test]
fn an_extra_lockfile_member_is_a_mismatch_for_pnpm_and_uv_but_invisible_to_cargo() {
    let lockfile = |authority| {
        let fixture = build_fixture(authority, LockState::ExtraMember);
        let request = RepoRequest::structure().with_lockfile_provenance(true);
        let repo = detect_repo_with_request(&fixture.root, &request)
            .expect("detection succeeds")
            .expect("fixture is a workspace");
        let layer = &repo.monorepo_layers[0];
        (serde_json::to_value(layer.lockfile.status).expect("status"), layer.lockfile.extra.clone())
    };

    assert_eq!(
        lockfile(Authority::Cargo),
        (serde_json::json!("members_present"), Vec::<String>::new())
    );
    assert_eq!(
        lockfile(Authority::Pnpm),
        (serde_json::json!("mismatch"), vec!["packages/gone".to_owned()])
    );
    assert_eq!(
        lockfile(Authority::Uv),
        (serde_json::json!("mismatch"), vec!["py/lib-gone".to_owned()])
    );
}

// ---------------------------------------------------------------------------
// Every other authority (`2026-09-26-lockfile-corroboration` AC1)
// ---------------------------------------------------------------------------
//
// The matrix above crosses Cargo, pnpm, and uv with every lockfile state at
// both request tiers. The cases below extend it to every other authority and
// every status each one can reach, plus orchestrator-only roots, asserting the
// complete serialized `RepoInfo` of a corroborating (or, where stated,
// declining) structure request. Node and Rush cases start from the real-tool
// fixtures under `tests/fixtures/lockfiles/`; the build-graph cases are
// hand-written because their tools write no lockfile. `unknown_standard`,
// `metadata_failed`, `ambiguous_membership`, `incomplete_manifest_discovery`,
// and `invalid_member_path` are pinned by the library's `lockfile` unit tests:
// `Unknown` never owns a layer (the orchestrator-only cases below show its
// standard entry and the absent layer list), and the metadata seam is
// crate-private (ruling R9).

fn lockfile_fixture_root() -> PathBuf {
    biscuit_test_harness::manifest_dir!().join("tests/fixtures/lockfiles")
}

fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).expect("create fixture copy directory");
    for entry in fs::read_dir(from).expect("read fixture directory") {
        let entry = entry.expect("fixture entry");
        let target = to.join(entry.file_name());
        if entry.file_type().expect("fixture entry type").is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), &target).expect("copy fixture file");
        }
    }
}

/// A disposable copy of one real-tool fixture, or an empty root when
/// `fixture` is `None`.
fn workspace(fixture: Option<&str>) -> Fixture {
    let dir = TempDir::new().expect("tempdir");
    let root = dir.path().join("repo");
    match fixture {
        Some(fixture) => copy_tree(&lockfile_fixture_root().join(fixture), &root),
        None => fs::create_dir_all(&root).expect("create root"),
    }
    Fixture { _dir: dir, root }
}

fn replace_in(root: &Path, relative: &str, from: &str, to: &str) {
    let path = root.join(relative);
    let content = fs::read_to_string(&path).expect("read fixture file");
    assert!(content.contains(from), "{relative} lacks {from:?}");
    fs::write(&path, content.replacen(from, to, 1)).expect("write fixture file");
}

/// One hand-written package entry of a structure-tier result.
fn package_json(
    relative: &str,
    name: &str,
    package_area: &str,
    ecosystem: &str,
    standard: &str,
    provenance: &str,
    version: Option<&str>,
) -> serde_json::Value {
    let mut package = serde_json::json!({
        "path": under(ROOT, relative),
        "relative": relative,
        "package_area": package_area,
        "name": name,
        "ecosystem": ecosystem,
        "standard": standard,
        "provenance": provenance,
        "primary_language": null,
        "package_managers": [],
    });
    if let Some(version) = version {
        package["version"] = serde_json::Value::from(version);
    }
    package
}

/// The three members every Node and Rush real-tool fixture declares, sorted
/// by relative path. `local-lib` is a local dependency, never a member.
fn fixture_node_packages(standard: &str, provenance: &str, version: &str) -> Vec<serde_json::Value> {
    [
        (".tools/hidden", "hidden-tool", ".tools"),
        ("packages/alpha", "alpha", "packages"),
        ("packages/beta", "@fixture/beta", "packages"),
    ]
    .iter()
    .map(|&(relative, name, area)| {
        package_json(relative, name, area, "node", standard, provenance, Some(version))
    })
    .collect()
}

fn lock_json(
    status: &str,
    paths: &[&str],
    reason: Option<&str>,
    extra: &[&str],
    missing: &[&str],
) -> serde_json::Value {
    serde_json::json!({
        "status": status,
        "paths": paths,
        "reason": reason,
        "extra": extra,
        "missing": missing,
    })
}

fn layer_json(
    authority: &str,
    provenance: &str,
    lockfile: serde_json::Value,
    packages: &[&str],
) -> serde_json::Value {
    serde_json::json!({
        "root": ROOT,
        "authority": authority,
        "orchestrators": [],
        "provenance": provenance,
        "lockfile": lockfile,
        "root_is_package": false,
        "packages": packages,
    })
}

/// A detected standard; `marker` is the matched marker file, if any.
fn standard_json(standard: &str, marker: Option<&str>, confidence: &str) -> serde_json::Value {
    let markers: Vec<String> = marker.iter().map(|marker| under(ROOT, marker)).collect();
    serde_json::json!({
        "standard": standard,
        "root": ROOT,
        "matched_markers": markers,
        "binary": HOST_BINARY,
        "confidence": confidence,
    })
}

/// A complete structure-tier `RepoInfo`. `layers` is `None` when the result
/// has no layer, which omits the key.
fn repo_json(
    is_monorepo: bool,
    packages: Vec<serde_json::Value>,
    standards: Vec<serde_json::Value>,
    layers: Option<Vec<serde_json::Value>>,
) -> serde_json::Value {
    let mut repo = serde_json::json!({
        "is_monorepo": is_monorepo,
        "root": ROOT,
        "packages": packages,
        "monorepo_standards": standards,
    });
    if let Some(layers) = layers {
        repo["monorepo_layers"] = serde_json::Value::from(layers);
    }
    repo["standalone_lockfiles"] = serde_json::json!([]);
    repo
}

/// A single-layer monorepo result.
fn one_layer_json(
    standard: serde_json::Value,
    layer: serde_json::Value,
    packages: Vec<serde_json::Value>,
) -> serde_json::Value {
    repo_json(true, packages, vec![standard], Some(vec![layer]))
}

/// `repo` serialized with the temporary root replaced by [`ROOT`], `\`
/// separators turned into `/`, and each standard's host `PATH` lookup
/// replaced by [`HOST_BINARY`].
fn normalized_any(repo: &RepoInfo, root: &Path) -> serde_json::Value {
    fn normalize(value: &mut serde_json::Value, root: &str) {
        match value {
            serde_json::Value::String(text) => {
                *text = text.replace(root, ROOT).replace('\\', "/");
            }
            serde_json::Value::Array(items) => items.iter_mut().for_each(|item| normalize(item, root)),
            serde_json::Value::Object(fields) => {
                fields.values_mut().for_each(|item| normalize(item, root));
            }
            _ => {}
        }
    }

    let mut value = serde_json::to_value(repo).expect("RepoInfo serializes");
    for standard in value["monorepo_standards"]
        .as_array_mut()
        .expect("monorepo_standards array")
    {
        let binary = standard["binary"].take();
        assert!(binary.is_null() || binary["name"].is_string(), "{binary}");
        standard["binary"] = serde_json::Value::from(HOST_BINARY);
    }
    normalize(&mut value, root.to_str().expect("temporary root is UTF-8"));
    // A leaf-marker layer lists members in directory-walk order, which differs
    // by filesystem (ext4 versus APFS and NTFS); every other order is pinned.
    if let Some(layers) = value
        .get_mut("monorepo_layers")
        .and_then(serde_json::Value::as_array_mut)
    {
        for layer in layers.iter_mut().filter(|layer| layer["provenance"] == "leaf-markers") {
            let packages = layer["packages"].as_array_mut().expect("layer packages");
            packages.sort_by(|a, b| a.as_str().cmp(&b.as_str()));
        }
    }
    value
}

/// One every-authority case: the tree, whether the request corroborates, the
/// complete expected result, and the lockfile reads and parses it costs.
struct AuthorityCase {
    label: &'static str,
    fixture: Fixture,
    corroborate: bool,
    expected: serde_json::Value,
    reads: u64,
    parses: u64,
}

fn authority_cases() -> Vec<AuthorityCase> {
    let npm_standard = || standard_json("npm-workspaces", Some("package.json"), "marker-confirmed");
    let node_members: &[&str] = &[".tools/hidden", "packages/alpha", "packages/beta"];
    let npm_case = |label, fixture: Fixture, corroborate, provenance, lockfile, reads, parses| {
        AuthorityCase {
            label,
            fixture,
            corroborate,
            expected: one_layer_json(
                npm_standard(),
                layer_json("npm-workspaces", provenance, lockfile, node_members),
                fixture_node_packages("npm-workspaces", provenance, "0.0.0"),
            ),
            reads,
            parses,
        }
    };
    const NPM_LOCK: &[&str] = &["package-lock.json"];

    let without_npm_lockfile = |replacement: fn(&Path)| {
        let fixture = workspace(Some("npm-11.6.4/workspace"));
        let lockfile = fixture.root.join("package-lock.json");
        fs::remove_file(&lockfile).expect("remove package-lock.json");
        replacement(&lockfile);
        fixture
    };

    let rush_lock: &[&str] = &["common/config/rush/pnpm-lock.yaml"];
    let rush_members: &[&str] = &["packages/alpha", "packages/beta", ".tools/hidden"];
    let rush_subspaces = workspace(Some("rush-5.179.0/pnpm-workspace"));
    replace_in(
        &rush_subspaces.root,
        "common/config/rush/subspaces.json",
        "\"subspacesEnabled\": false",
        "\"subspacesEnabled\": true",
    );

    let go_absent = workspace(Some("go-1.27.1/workspace"));
    fs::remove_file(go_absent.root.join("go.work.sum")).expect("remove go.work.sum");
    let go_packages = |provenance| {
        ["alpha", "beta"]
            .iter()
            .map(|name| {
                package_json(
                    &format!("packages/{name}"),
                    &format!("example.com/fixture/{name}"),
                    "packages",
                    "go",
                    "go-workspace",
                    provenance,
                    None,
                )
            })
            .collect::<Vec<_>>()
    };
    let go_case = |label, fixture, lockfile| AuthorityCase {
        label,
        fixture,
        corroborate: true,
        expected: one_layer_json(
            standard_json("go-workspace", Some("go.work"), "marker-confirmed"),
            layer_json("go-workspace", "explicit", lockfile, &["packages/alpha", "packages/beta"]),
            go_packages("explicit"),
        ),
        reads: 0,
        parses: 0,
    };

    let bazel = workspace(Some("bazel-8.4.2/bzlmod"));
    for package in ["app", "lib"] {
        write(&bazel.root, &format!("{package}/BUILD.bazel"), "");
    }

    // Build graphs with no lockfile source: `(label, standard, marker,
    // provenance, members as (relative, name, package_area), files)`.
    type BuildGraph = (
        &'static str,
        &'static str,
        Option<&'static str>,
        &'static str,
        [(&'static str, &'static str, &'static str); 2],
        &'static [(&'static str, &'static str)],
    );
    const POM: &str = "<project><modelVersion>4.0.0</modelVersion><groupId>g</groupId>";
    let build_graphs: [BuildGraph; 4] = [
        (
            "Maven multi-module",
            "maven-multi-module",
            Some("pom.xml"),
            "explicit",
            [("alpha", "alpha", ""), ("beta", "beta", "")],
            &[
                (
                    "pom.xml",
                    "<project><modelVersion>4.0.0</modelVersion><groupId>g</groupId>\
                     <artifactId>root</artifactId><version>1.0</version><packaging>pom</packaging>\
                     <modules><module>alpha</module><module>beta</module></modules></project>",
                ),
                ("alpha/pom.xml", "ALPHA"),
                ("beta/pom.xml", "BETA"),
            ],
        ),
        (
            ".NET solution",
            "dot-net-solution",
            // `*.sln` is a pattern, not a file name, so no marker is listed.
            None,
            "explicit",
            [("src/Alpha", "src/Alpha", "src"), ("src/Beta", "src/Beta", "src")],
            &[
                (
                    "App.sln",
                    "Microsoft Visual Studio Solution File, Format Version 12.00\n\
                     Project(\"{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}\") = \"Alpha\", \
                     \"src\\Alpha\\Alpha.csproj\", \"{11111111-1111-1111-1111-111111111111}\"\n\
                     EndProject\n\
                     Project(\"{FAE04EC0-301F-11D3-BF4B-00C04F79EFBC}\") = \"Beta\", \
                     \"src\\Beta\\Beta.csproj\", \"{22222222-2222-2222-2222-222222222222}\"\n\
                     EndProject\n",
                ),
                ("src/Alpha/Alpha.csproj", "<Project Sdk=\"Microsoft.NET.Sdk\"></Project>"),
                ("src/Beta/Beta.csproj", "<Project Sdk=\"Microsoft.NET.Sdk\"></Project>"),
            ],
        ),
        (
            "Pants",
            "pants",
            Some("pants.toml"),
            "leaf-markers",
            [("src/app", "src/app", "src"), ("src/lib", "src/lib", "src")],
            &[
                ("pants.toml", "[GLOBAL]\npants_version = \"2.20.0\"\n"),
                ("src/app/BUILD.pants", ""),
                ("src/lib/BUILD.pants", ""),
            ],
        ),
        (
            "Buck2",
            "buck2",
            Some(".buckconfig"),
            "leaf-markers",
            [("app", "app", ""), ("lib", "lib", "")],
            &[(".buckconfig", "[cells]\nroot = .\n"), ("app/BUCK", ""), ("lib/BUCK", "")],
        ),
    ];

    let mut cases = vec![
        npm_case(
            "npm match",
            workspace(Some("npm-11.6.4/workspace")),
            true,
            "lockfile",
            lock_json("match", NPM_LOCK, None, &[], &[]),
            1,
            1,
        ),
        npm_case(
            "npm stale extra member",
            workspace(Some("npm-11.6.4/workspace-edited-stale-extra")),
            true,
            "globbed",
            lock_json("mismatch", NPM_LOCK, None, &["packages/gamma"], &[]),
            1,
            1,
        ),
        npm_case(
            "npm missing member",
            workspace(Some("npm-11.6.4/workspace-edited-missing")),
            true,
            "globbed",
            lock_json("mismatch", NPM_LOCK, None, &[], &["packages/beta"]),
            1,
            1,
        ),
        npm_case(
            "npm malformed trailing content",
            workspace(Some("npm-11.6.4/workspace-edited-malformed-trailing")),
            true,
            "globbed",
            lock_json("unreadable", NPM_LOCK, Some("parse_failed"), &[], &[]),
            1,
            1,
        ),
        npm_case(
            "npm unknown version",
            workspace(Some("npm-11.6.4/workspace-edited-unknown-version")),
            true,
            "globbed",
            lock_json("unverifiable", NPM_LOCK, Some("unsupported_version"), &[], &[]),
            1,
            1,
        ),
        npm_case(
            "npm directory in place of the lockfile",
            without_npm_lockfile(|lockfile| fs::create_dir(lockfile).expect("create directory")),
            true,
            "globbed",
            lock_json("unreadable", NPM_LOCK, Some("read_failed"), &[], &[]),
            1,
            0,
        ),
        npm_case(
            "npm absent",
            without_npm_lockfile(|_| {}),
            true,
            "globbed",
            lock_json("absent", &[], None, &[], &[]),
            0,
            0,
        ),
        npm_case(
            "npm declined",
            workspace(Some("npm-11.6.4/workspace")),
            false,
            "globbed",
            lock_json("not_requested", NPM_LOCK, Some("request_disabled"), &[], &[]),
            0,
            0,
        ),
        AuthorityCase {
            label: "Yarn match beside an overlapping lockfile-less npm layer",
            fixture: workspace(Some("yarn-4.18.1/workspace")),
            corroborate: true,
            expected: repo_json(
                true,
                fixture_node_packages("yarn-workspaces", "lockfile", "1.0.0"),
                vec![
                    standard_json("yarn-workspaces", Some("package.json"), "marker-confirmed"),
                    npm_standard(),
                ],
                Some(vec![
                    layer_json(
                        "yarn-workspaces",
                        "lockfile",
                        lock_json("match", &["yarn.lock"], None, &[], &[]),
                        node_members,
                    ),
                    layer_json(
                        "npm-workspaces",
                        "globbed",
                        lock_json("absent", &[], None, &[], &[]),
                        node_members,
                    ),
                ]),
            ),
            reads: 1,
            parses: 1,
        },
        AuthorityCase {
            label: "Bun text lockfile match",
            fixture: workspace(Some("bun-1.3.3/workspace")),
            corroborate: true,
            expected: one_layer_json(
                standard_json("bun-workspaces", Some("package.json"), "marker-confirmed"),
                layer_json(
                    "bun-workspaces",
                    "lockfile",
                    lock_json("match", &["bun.lock"], None, &[], &[]),
                    node_members,
                ),
                fixture_node_packages("bun-workspaces", "lockfile", "1.0.0"),
            ),
            reads: 1,
            parses: 1,
        },
        AuthorityCase {
            label: "Bun binary lockfile",
            fixture: workspace(Some("bun-1.3.3/workspace-binary")),
            corroborate: true,
            expected: one_layer_json(
                standard_json("bun-workspaces", Some("package.json"), "marker-confirmed"),
                layer_json(
                    "bun-workspaces",
                    "globbed",
                    lock_json("unverifiable", &["bun.lockb"], Some("no_membership_data"), &[], &[]),
                    node_members,
                ),
                fixture_node_packages("bun-workspaces", "globbed", "1.0.0"),
            ),
            reads: 0,
            parses: 0,
        },
        AuthorityCase {
            label: "Rush pnpm workspace match",
            fixture: workspace(Some("rush-5.179.0/pnpm-workspace")),
            corroborate: true,
            expected: one_layer_json(
                standard_json("rush-stack", Some("rush.json"), "marker-confirmed"),
                layer_json(
                    "rush-stack",
                    "lockfile",
                    lock_json("match", rush_lock, None, &[], &[]),
                    rush_members,
                ),
                fixture_node_packages("rush-stack", "lockfile", "0.0.0"),
            ),
            reads: 1,
            parses: 1,
        },
        AuthorityCase {
            label: "Rush with subspaces enabled",
            fixture: rush_subspaces,
            corroborate: true,
            expected: one_layer_json(
                standard_json("rush-stack", Some("rush.json"), "marker-confirmed"),
                layer_json(
                    "rush-stack",
                    "explicit",
                    lock_json("unverifiable", rush_lock, Some("unsupported_layout"), &[], &[]),
                    rush_members,
                ),
                fixture_node_packages("rush-stack", "explicit", "0.0.0"),
            ),
            reads: 0,
            parses: 0,
        },
        go_case(
            "Go workspace sum",
            workspace(Some("go-1.27.1/workspace")),
            lock_json("unverifiable", &["go.work.sum"], Some("no_membership_data"), &[], &[]),
        ),
        go_case(
            "Go workspace without a sum",
            go_absent,
            lock_json("absent", &[], None, &[], &[]),
        ),
        AuthorityCase {
            label: "Gradle root lockfile",
            fixture: workspace(Some("gradle-8.14.5/root-lockfile")),
            corroborate: true,
            expected: one_layer_json(
                standard_json("gradle-multi-project", Some("settings.gradle"), "marker-confirmed"),
                layer_json(
                    "gradle-multi-project",
                    "explicit",
                    lock_json("unverifiable", &["gradle.lockfile"], Some("no_membership_data"), &[], &[]),
                    &["app", "lib"],
                ),
                ["app", "lib"]
                    .iter()
                    .map(|name| {
                        package_json(name, name, "", "unknown", "gradle-multi-project", "explicit", None)
                    })
                    .collect(),
            ),
            reads: 0,
            parses: 0,
        },
        AuthorityCase {
            label: "Bazel module lockfile",
            fixture: bazel,
            corroborate: true,
            expected: one_layer_json(
                standard_json("bazel", Some("MODULE.bazel"), "marker-confirmed"),
                layer_json(
                    "bazel",
                    "leaf-markers",
                    lock_json("unverifiable", &["MODULE.bazel.lock"], Some("no_membership_data"), &[], &[]),
                    &["app", "lib"],
                ),
                ["app", "lib"]
                    .iter()
                    .map(|name| package_json(name, name, "", "unknown", "bazel", "leaf-markers", None))
                    .collect(),
            ),
            reads: 0,
            parses: 0,
        },
    ];

    for (label, standard, marker, provenance, members, files) in build_graphs {
        let fixture = workspace(None);
        for &(relative, content) in files {
            let content = match content {
                "ALPHA" | "BETA" => format!(
                    "{POM}<artifactId>{}</artifactId><version>1.0</version></project>",
                    content.to_lowercase()
                ),
                other => other.to_owned(),
            };
            write(&fixture.root, relative, &content);
        }
        let relatives: Vec<&str> = members.iter().map(|&(relative, ..)| relative).collect();
        cases.push(AuthorityCase {
            label,
            fixture,
            corroborate: true,
            expected: one_layer_json(
                standard_json(standard, marker, "marker-confirmed"),
                layer_json(
                    standard,
                    provenance,
                    lock_json("not_applicable", &[], Some("no_lockfile_source"), &[], &[]),
                    &relatives,
                ),
                members
                    .iter()
                    .map(|&(relative, name, area)| {
                        package_json(relative, name, area, "unknown", standard, provenance, None)
                    })
                    .collect(),
            ),
            reads: 0,
            parses: 0,
        });
    }

    // Orchestrator-only roots define no membership: no layer is synthesized
    // and the downgrade shows as an inferred `unknown` standard. The root's
    // `package-lock.json` and `package.json` belong to no layer.
    for (label, standard, marker, content) in [
        ("Nx only", "nx", "nx.json", "{}"),
        ("Turborepo only", "turborepo", "turbo.json", "{}"),
        ("Lerna only", "lerna", "lerna.json", "{\"version\":\"0.0.0\"}"),
    ] {
        let fixture = workspace(None);
        write(&fixture.root, marker, content);
        write(&fixture.root, "package.json", r#"{"name":"solo","version":"1.0.0"}"#);
        write(&fixture.root, "package-lock.json", "{\"lockfileVersion\": 3, \"packages\": {}}");
        cases.push(AuthorityCase {
            label,
            fixture,
            corroborate: true,
            expected: repo_json(
                false,
                Vec::new(),
                vec![
                    standard_json(standard, Some(marker), "inferred"),
                    standard_json("unknown", None, "inferred"),
                ],
                None,
            ),
            reads: 0,
            parses: 0,
        });
    }
    cases
}

#[test]
fn every_other_authority_reports_its_complete_repository_result() {
    let cases = authority_cases();
    assert_eq!(cases.len(), 24);
    for case in cases {
        let root = case.fixture.root.as_path();
        let request = RepoRequest::structure().with_lockfile_provenance(case.corroborate);
        let (repo, counts) = measured(|| detect_repo_with_request(root, &request));
        let repo = repo
            .expect("detection succeeds")
            .unwrap_or_else(|| panic!("{}: no repository", case.label));
        let actual = normalized_any(&repo, root);
        assert!(
            actual == case.expected,
            "{}\n--- expected\n{}\n--- actual\n{}",
            case.label,
            serde_json::to_string_pretty(&case.expected).expect("pretty expected"),
            serde_json::to_string_pretty(&actual).expect("pretty actual"),
        );
        // The typed result round-trips through its own wire form.
        let reparsed: RepoInfo = serde_json::from_value(serde_json::to_value(&repo).expect("serialize"))
            .expect("RepoInfo deserializes");
        assert_eq!(normalized_any(&reparsed, root), actual, "{}", case.label);
        assert_eq!(
            (
                counter(&counts, counters::REPO_LOCKFILE_READS),
                counter(&counts, counters::REPO_LOCKFILE_PARSES)
            ),
            (case.reads, case.parses),
            "{}: {counts:?}",
            case.label
        );
    }
}
