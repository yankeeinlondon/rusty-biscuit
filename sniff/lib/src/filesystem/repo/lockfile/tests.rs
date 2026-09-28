//! Engine tests: [`observe_layer_lockfile`] against constructed layers, so the
//! selection and precedence rules are pinned independently of any detector.

use std::fs;
use std::path::Path;

use tempfile::TempDir;

use super::*;
use crate::filesystem::repo::standard::{MonorepoStandard, PackageProvenance};
use crate::performance::counters;
use crate::performance::testing;

fn layer(root: &Path, authority: MonorepoStandard) -> MonorepoLayer {
    MonorepoLayer {
        root: root.to_path_buf(),
        authority,
        orchestrators: Vec::new(),
        provenance: PackageProvenance::Globbed,
        lockfile: LockfileObservation::not_applicable(LockfileReason::NoLockfileSource),
        root_is_package: false,
        packages: Vec::new(),
    }
}

fn observe(
    root: &Path,
    authority: MonorepoStandard,
    wants: bool,
) -> (LockfileObservation, testing::WorkCounts) {
    let store = ManifestStore::default();
    let request = RepoRequest::structure().with_lockfile_provenance(wants);
    testing::measure(|| observe_layer_lockfile(&layer(root, authority), Some(&[]), &request, &store))
}

fn expected(
    status: LockfileStatus,
    paths: &[&str],
    reason: Option<LockfileReason>,
) -> LockfileObservation {
    LockfileObservation {
        status,
        paths: paths.iter().map(|path| (*path).to_owned()).collect(),
        reason,
        extra: Vec::new(),
        missing: Vec::new(),
    }
}

fn touch(root: &Path, relative: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().expect("parent")).expect("create parent");
    fs::write(path, "").expect("write file");
}

#[test]
fn standards_without_a_lockfile_source_are_not_applicable_without_probing() {
    let dir = TempDir::new().expect("tempdir");
    for (authority, reason) in [
        (MonorepoStandard::MavenMultiModule, LockfileReason::NoLockfileSource),
        (MonorepoStandard::DotNetSolution, LockfileReason::NoLockfileSource),
        (MonorepoStandard::Pants, LockfileReason::NoLockfileSource),
        (MonorepoStandard::Buck2, LockfileReason::NoLockfileSource),
        (MonorepoStandard::Unknown, LockfileReason::UnknownStandard),
    ] {
        for wants in [true, false] {
            let (observation, counts) = observe(dir.path(), authority, wants);
            assert_eq!(
                observation,
                expected(LockfileStatus::NotApplicable, &[], Some(reason)),
                "{authority:?}"
            );
            assert_eq!(counts.get(counters::REPO_LOCKFILE_PROBES), 0, "{authority:?}");
        }
    }
}

#[test]
fn an_absent_candidate_is_absent_whatever_the_request() {
    let dir = TempDir::new().expect("tempdir");
    for authority in [
        MonorepoStandard::PnpmWorkspaces,
        MonorepoStandard::NpmWorkspaces,
        MonorepoStandard::BunWorkspaces,
        MonorepoStandard::YarnWorkspaces,
        MonorepoStandard::UvWorkspace,
        MonorepoStandard::CargoWorkspace,
        MonorepoStandard::GoWorkspace,
        MonorepoStandard::GradleMultiProject,
    ] {
        for wants in [true, false] {
            let (observation, counts) = observe(dir.path(), authority, wants);
            assert_eq!(
                observation,
                expected(LockfileStatus::Absent, &[], None),
                "{authority:?}"
            );
            assert_eq!(counts.get(counters::REPO_LOCKFILE_READS), 0, "{authority:?}");
        }
    }
}

#[test]
fn a_present_candidate_is_not_requested_without_a_read_when_declined() {
    let dir = TempDir::new().expect("tempdir");
    for (authority, lockfile) in [
        (MonorepoStandard::PnpmWorkspaces, "pnpm-lock.yaml"),
        (MonorepoStandard::NpmWorkspaces, "package-lock.json"),
        (MonorepoStandard::YarnWorkspaces, "yarn.lock"),
        (MonorepoStandard::BunWorkspaces, "bun.lockb"),
        (MonorepoStandard::UvWorkspace, "uv.lock"),
        (MonorepoStandard::CargoWorkspace, "Cargo.lock"),
        (MonorepoStandard::GoWorkspace, "go.work.sum"),
        (MonorepoStandard::GradleMultiProject, "gradle.lockfile"),
        (MonorepoStandard::Bazel, "MODULE.bazel.lock"),
    ] {
        touch(dir.path(), lockfile);
        touch(dir.path(), "MODULE.bazel");
        let (observation, counts) = observe(dir.path(), authority, false);
        assert_eq!(
            observation,
            expected(
                LockfileStatus::NotRequested,
                &[lockfile],
                Some(LockfileReason::RequestDisabled)
            ),
            "{authority:?}"
        );
        assert_eq!(counts.get(counters::REPO_LOCKFILE_READS), 0, "{authority:?}");
        assert_eq!(counts.get(counters::REPO_LOCKFILE_PARSES), 0, "{authority:?}");
    }
}

#[test]
fn fallback_sources_are_unverifiable_from_metadata_alone() {
    let dir = TempDir::new().expect("tempdir");
    for (authority, lockfile) in [
        (MonorepoStandard::GoWorkspace, "go.work.sum"),
        (MonorepoStandard::GradleMultiProject, "gradle.lockfile"),
        (MonorepoStandard::Bazel, "MODULE.bazel.lock"),
    ] {
        touch(dir.path(), lockfile);
        touch(dir.path(), "MODULE.bazel");
        let (observation, counts) = observe(dir.path(), authority, true);
        assert_eq!(
            observation,
            expected(
                LockfileStatus::Unverifiable,
                &[lockfile],
                Some(LockfileReason::NoMembershipData)
            ),
            "{authority:?}"
        );
        assert_eq!(counts.get(counters::REPO_LOCKFILE_READS), 0, "{authority:?}");
    }
}

#[test]
fn the_binary_bun_lockfile_is_never_read() {
    let dir = TempDir::new().expect("tempdir");
    touch(dir.path(), "bun.lockb");

    let (observation, counts) = observe(dir.path(), MonorepoStandard::BunWorkspaces, true);

    assert_eq!(
        observation,
        expected(
            LockfileStatus::Unverifiable,
            &["bun.lockb"],
            Some(LockfileReason::NoMembershipData)
        )
    );
    assert_eq!(counts.get(counters::REPO_LOCKFILE_READS), 0);
}

/// R6: the higher-priority file wins and alone appears in `paths`; the
/// lower-priority file is not even probed.
#[test]
fn only_the_first_present_candidate_is_selected() {
    for (authority, winner, loser) in [
        (
            MonorepoStandard::NpmWorkspaces,
            "npm-shrinkwrap.json",
            "package-lock.json",
        ),
        (MonorepoStandard::BunWorkspaces, "bun.lock", "bun.lockb"),
    ] {
        let dir = TempDir::new().expect("tempdir");
        touch(dir.path(), winner);
        touch(dir.path(), loser);

        let (observation, counts) = observe(dir.path(), authority, false);

        assert_eq!(observation.paths, [winner], "{authority:?}");
        assert_eq!(counts.get(counters::REPO_LOCKFILE_PROBES), 1, "{authority:?}");
    }
}

/// A metadata failure on a higher-priority candidate is `unreadable`: it is
/// not skipped in favor of a lower-priority file that exists.
#[test]
fn a_metadata_failure_is_not_skipped_for_a_lower_priority_candidate() {
    let dir = TempDir::new().expect("tempdir");
    touch(dir.path(), "package-lock.json");
    let _failure = test_seam::fail_metadata(
        &dir.path().join("npm-shrinkwrap.json"),
        std::io::ErrorKind::PermissionDenied,
    );

    let (observation, counts) = observe(dir.path(), MonorepoStandard::NpmWorkspaces, true);

    assert_eq!(
        observation,
        expected(
            LockfileStatus::Unreadable,
            &[],
            Some(LockfileReason::MetadataFailed)
        )
    );
    assert_eq!(counts.get(counters::REPO_LOCKFILE_READS), 0);
}

/// A metadata failure outranks a disabled request.
#[test]
fn a_metadata_failure_is_unreadable_even_when_declined() {
    let dir = TempDir::new().expect("tempdir");
    for (authority, lockfile) in [
        (MonorepoStandard::PnpmWorkspaces, "pnpm-lock.yaml"),
        (MonorepoStandard::GoWorkspace, "go.work.sum"),
        (MonorepoStandard::RushStack, "common/config/rush/pnpm-lock.yaml"),
    ] {
        fs::write(dir.path().join("rush.json"), RUSH_PNPM).expect("write rush.json");
        let _failure = test_seam::fail_metadata(
            &dir.path().join(lockfile),
            std::io::ErrorKind::PermissionDenied,
        );
        let (observation, _) = observe(dir.path(), authority, false);
        assert_eq!(
            observation,
            expected(
                LockfileStatus::Unreadable,
                &[],
                Some(LockfileReason::MetadataFailed)
            ),
            "{authority:?}"
        );
    }
}

/// An empty file is a parse failure for every text format, after exactly one
/// read and one parse: never an empty member set.
#[test]
fn an_empty_lockfile_is_a_parse_failure_after_one_read() {
    for (authority, lockfile) in [
        (MonorepoStandard::PnpmWorkspaces, "pnpm-lock.yaml"),
        (MonorepoStandard::NpmWorkspaces, "package-lock.json"),
        (MonorepoStandard::YarnWorkspaces, "yarn.lock"),
        (MonorepoStandard::BunWorkspaces, "bun.lock"),
        (MonorepoStandard::UvWorkspace, "uv.lock"),
        (MonorepoStandard::CargoWorkspace, "Cargo.lock"),
    ] {
        let dir = TempDir::new().expect("tempdir");
        touch(dir.path(), lockfile);

        let (observation, counts) = observe(dir.path(), authority, true);

        assert_eq!(
            observation,
            expected(
                LockfileStatus::Unreadable,
                &[lockfile],
                Some(LockfileReason::ParseFailed)
            ),
            "{authority:?}"
        );
        assert_eq!(counts.get(counters::REPO_LOCKFILE_READS), 1, "{authority:?}");
        assert_eq!(counts.get(counters::REPO_LOCKFILE_PARSES), 1, "{authority:?}");
    }
}

#[test]
fn a_layer_without_its_detector_outcome_is_incomplete() {
    let dir = TempDir::new().expect("tempdir");
    fs::write(
        dir.path().join("pnpm-lock.yaml"),
        "lockfileVersion: '9.0'\nimporters:\n  .: {}\n",
    )
    .expect("write lockfile");
    let store = ManifestStore::default();
    let request = RepoRequest::full();

    let observation = observe_layer_lockfile(
        &layer(dir.path(), MonorepoStandard::PnpmWorkspaces),
        None,
        &request,
        &store,
    );

    assert_eq!(
        observation,
        expected(
            LockfileStatus::Unverifiable,
            &["pnpm-lock.yaml"],
            Some(LockfileReason::IncompleteManifestDiscovery)
        )
    );
}

/// Ruling R10: a member whose manifest fails to parse, or is missing, is not
/// a resolved package, so the lockfile recording exactly the discovered
/// members is still not a `match`. The public-API uv case is in the L1
/// `lockfile_isolation` suite.
#[test]
fn a_member_without_a_parseable_manifest_is_incomplete() {
    let cases = [
        (
            MonorepoStandard::PnpmWorkspaces,
            "pnpm-lock.yaml",
            "lockfileVersion: '9.0'\nimporters:\n  .: {}\n  packages/a: {}\n  packages/b: {}\n",
        ),
        (
            MonorepoStandard::NpmWorkspaces,
            "package-lock.json",
            r#"{"lockfileVersion":3,"packages":{"":{"workspaces":["packages/*"]},
                "node_modules/a":{"resolved":"packages/a","link":true},
                "node_modules/b":{"resolved":"packages/b","link":true},
                "packages/a":{"name":"a"},"packages/b":{"name":"b"}}}"#,
        ),
    ];
    for (authority, lockfile, content) in cases {
        for broken in [Some("{ not json"), None] {
            let dir = TempDir::new().expect("tempdir");
            let root = dir.path();
            write(root, lockfile, content);
            write(root, "packages/a/package.json", r#"{"name":"a"}"#);
            fs::create_dir_all(root.join("packages/b")).expect("create member");
            if let Some(broken) = broken {
                write(root, "packages/b/package.json", broken);
            }
            let owned: Vec<PackageSeed> = ["a", "b"]
                .iter()
                .map(|name| {
                    PackageSeed::new(
                        &root.join("packages").join(name),
                        root,
                        authority,
                        PackageProvenance::Globbed,
                    )
                })
                .collect();
            let store = ManifestStore::default();

            let observation = observe_layer_lockfile(
                &layer(root, authority),
                Some(&owned),
                &RepoRequest::full(),
                &store,
            );

            assert_eq!(
                observation,
                expected(
                    LockfileStatus::Unverifiable,
                    &[lockfile],
                    Some(LockfileReason::IncompleteManifestDiscovery)
                ),
                "{authority:?}, broken manifest {broken:?}"
            );

            // The control: with both manifests valid the same lockfile matches.
            write(root, "packages/b/package.json", r#"{"name":"b"}"#);
            let store = ManifestStore::default();
            let observation = observe_layer_lockfile(
                &layer(root, authority),
                Some(&owned),
                &RepoRequest::full(),
                &store,
            );
            assert_eq!(
                observation,
                expected(LockfileStatus::Match, &[lockfile], None),
                "{authority:?}"
            );
        }
    }
}

/// A lockfile path that is absolute is `invalid_member_path`, never a
/// silently dropped entry.
#[test]
fn an_absolute_member_path_is_unverifiable() {
    let dir = TempDir::new().expect("tempdir");
    fs::write(
        dir.path().join("pnpm-lock.yaml"),
        "lockfileVersion: '9.0'\nimporters:\n  .: {}\n  /etc/app: {}\n",
    )
    .expect("write lockfile");

    let (observation, _) = observe(dir.path(), MonorepoStandard::PnpmWorkspaces, true);

    assert_eq!(
        observation,
        expected(
            LockfileStatus::Unverifiable,
            &["pnpm-lock.yaml"],
            Some(LockfileReason::InvalidMemberPath)
        )
    );
}

/// Both member sets empty and established from complete records is a match.
#[test]
fn a_root_only_lockfile_matches_an_empty_member_set() {
    let dir = TempDir::new().expect("tempdir");
    fs::write(
        dir.path().join("pnpm-lock.yaml"),
        "lockfileVersion: '9.0'\nimporters:\n  .: {}\n",
    )
    .expect("write lockfile");

    let (observation, _) = observe(dir.path(), MonorepoStandard::PnpmWorkspaces, true);

    assert_eq!(
        observation,
        expected(LockfileStatus::Match, &["pnpm-lock.yaml"], None)
    );
}

/// Two layers whose roots name one directory through different spellings
/// share the request's lockfile outcome: one probe, one read, and one parse,
/// for a success and for a cached parse failure alike.
#[test]
fn layers_sharing_one_lockfile_read_and_parse_it_once() {
    for (content, status, reason) in [
        (
            "lockfileVersion: '9.0'\nimporters:\n  .: {}\n",
            LockfileStatus::Match,
            None,
        ),
        (
            "lockfileVersion: '9.0'\nimporters: [\n",
            LockfileStatus::Unreadable,
            Some(LockfileReason::ParseFailed),
        ),
    ] {
        let dir = TempDir::new().expect("tempdir");
        fs::create_dir(dir.path().join("sub")).expect("create sub");
        fs::write(dir.path().join("pnpm-lock.yaml"), content).expect("write lockfile");
        let respelled = dir.path().join("sub").join("..");
        let store = ManifestStore::default();
        let request = RepoRequest::full();

        let (observations, counts) = testing::measure(|| {
            [dir.path(), respelled.as_path()].map(|root| {
                observe_layer_lockfile(
                    &layer(root, MonorepoStandard::PnpmWorkspaces),
                    Some(&[]),
                    &request,
                    &store,
                )
            })
        });

        let expected = expected(status, &["pnpm-lock.yaml"], reason);
        assert_eq!(observations, [expected.clone(), expected], "{content}");
        assert_eq!(counts.get(counters::REPO_LOCKFILE_PROBES), 1, "{content}");
        assert_eq!(counts.get(counters::REPO_LOCKFILE_READS), 1, "{content}");
        assert_eq!(counts.get(counters::REPO_LOCKFILE_PARSES), 1, "{content}");
    }
}

#[test]
fn the_wire_names_are_the_frozen_snake_case_vocabulary() {
    let statuses = [
        (LockfileStatus::Match, "match"),
        (LockfileStatus::Mismatch, "mismatch"),
        (LockfileStatus::MembersPresent, "members_present"),
        (LockfileStatus::MembersMissing, "members_missing"),
        (LockfileStatus::Unverifiable, "unverifiable"),
        (LockfileStatus::Unreadable, "unreadable"),
        (LockfileStatus::Absent, "absent"),
        (LockfileStatus::NotApplicable, "not_applicable"),
        (LockfileStatus::NotRequested, "not_requested"),
    ];
    for (status, name) in statuses {
        assert_eq!(serde_json::to_value(status).expect("serializes"), name);
    }
    let reasons = [
        (LockfileReason::RequestDisabled, "request_disabled"),
        (LockfileReason::NoLockfileSource, "no_lockfile_source"),
        (LockfileReason::UnknownStandard, "unknown_standard"),
        (LockfileReason::UnsupportedVersion, "unsupported_version"),
        (LockfileReason::UnsupportedLayout, "unsupported_layout"),
        (LockfileReason::NoMembershipData, "no_membership_data"),
        (LockfileReason::AmbiguousMembership, "ambiguous_membership"),
        (
            LockfileReason::IncompleteManifestDiscovery,
            "incomplete_manifest_discovery",
        ),
        (LockfileReason::InvalidMemberPath, "invalid_member_path"),
        (LockfileReason::MetadataFailed, "metadata_failed"),
        (LockfileReason::ReadFailed, "read_failed"),
        (LockfileReason::ParseFailed, "parse_failed"),
        (LockfileReason::SubsetOnly, "subset_only"),
    ];
    for (reason, name) in reasons {
        assert_eq!(serde_json::to_value(reason).expect("serializes"), name);
    }
}

/// Every field is serialized, including empty lists and a `null` reason, and
/// the JSON round-trips repeatedly without change.
#[test]
fn the_observation_serializes_every_field_and_round_trips() {
    let observation = LockfileObservation {
        status: LockfileStatus::Mismatch,
        paths: vec!["pnpm-lock.yaml".to_owned()],
        reason: None,
        extra: Vec::new(),
        missing: vec!["packages/ui".to_owned()],
    };
    let json = serde_json::to_value(&observation).expect("serializes");
    assert_eq!(
        json,
        serde_json::json!({
            "status": "mismatch",
            "paths": ["pnpm-lock.yaml"],
            "reason": null,
            "extra": [],
            "missing": ["packages/ui"],
        })
    );
    let mut round_tripped = observation.clone();
    for _ in 0..2 {
        let text = serde_json::to_string(&round_tripped).expect("serializes");
        round_tripped = serde_json::from_str(&text).expect("deserializes");
    }
    assert_eq!(round_tripped, observation);
}

const RUSH_PNPM: &str = "// comment\n{ \"pnpmVersion\": \"9.15.9\", \"projects\": [], }";
const RUSH_LOCK: &str = "common/config/rush/pnpm-lock.yaml";
const PNPM_CONFIG: &str = "common/config/rush/pnpm-config.json";
const SUBSPACES: &str = "common/config/rush/subspaces.json";

fn write(root: &Path, relative: &str, content: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().expect("parent")).expect("create parent");
    fs::write(path, content).expect("write file");
}

/// The ordinary layout: pnpm, `useWorkspaces`, subspaces off, no variants.
fn rush_pnpm_workspace(importers: &str) -> TempDir {
    let dir = TempDir::new().expect("tempdir");
    write(dir.path(), "rush.json", RUSH_PNPM);
    write(dir.path(), PNPM_CONFIG, "{ /* c */ \"useWorkspaces\": true, }");
    write(dir.path(), SUBSPACES, "{ \"subspacesEnabled\": false }");
    write(
        dir.path(),
        RUSH_LOCK,
        &format!("lockfileVersion: '9.0'\nimporters:\n{importers}"),
    );
    dir
}

/// Importer keys resolve against `common/temp`, and its synthetic `.`
/// project is not a member.
#[test]
fn rush_importers_resolve_against_common_temp() {
    let dir = rush_pnpm_workspace("  .: {}\n  ../../apps/web: {}\n");

    let (observation, counts) = observe(dir.path(), MonorepoStandard::RushStack, true);

    assert_eq!(observation.status, LockfileStatus::Mismatch);
    assert_eq!(observation.paths, [RUSH_LOCK]);
    assert_eq!(observation.extra, ["apps/web"]);
    assert_eq!(counts.get(counters::REPO_LOCKFILE_READS), 1);
    // rush.json, pnpm-config.json, and subspaces.json.
    assert_eq!(counts.get(counters::REPO_MANIFEST_PARSES), 3);

    let dir = rush_pnpm_workspace("  .: {}\n");
    let (observation, _) = observe(dir.path(), MonorepoStandard::RushStack, true);
    assert_eq!(observation, expected(LockfileStatus::Match, &[RUSH_LOCK], None));
}

/// Only the exact `.` importer is synthetic: an absolute importer beside it
/// is an invalid member path, never dropped.
#[test]
fn an_absolute_rush_importer_is_an_invalid_member_path() {
    let dir = rush_pnpm_workspace("  .: {}\n  /opt/elsewhere/pkg: {}\n");

    let (observation, counts) = observe(dir.path(), MonorepoStandard::RushStack, true);

    assert_eq!(
        observation,
        expected(
            LockfileStatus::Unverifiable,
            &[RUSH_LOCK],
            Some(LockfileReason::InvalidMemberPath)
        )
    );
    assert_eq!(counts.get(counters::REPO_LOCKFILE_READS), 1);
}

/// A declining request reads no Rush configuration beyond the `rush.json`
/// the detector already caches, and no lockfile.
#[test]
fn a_declined_rush_request_reads_no_configuration() {
    let dir = rush_pnpm_workspace("  .: {}\n");

    let (observation, counts) = observe(dir.path(), MonorepoStandard::RushStack, false);

    assert_eq!(
        observation,
        expected(
            LockfileStatus::NotRequested,
            &[RUSH_LOCK],
            Some(LockfileReason::RequestDisabled)
        )
    );
    assert_eq!(counts.get(counters::REPO_MANIFEST_PARSES), 1);
    assert_eq!(counts.get(counters::REPO_LOCKFILE_READS), 0);

    fs::remove_file(dir.path().join(RUSH_LOCK)).expect("remove lockfile");
    let (observation, _) = observe(dir.path(), MonorepoStandard::RushStack, false);
    assert_eq!(observation, expected(LockfileStatus::Absent, &[], None));
}

/// Every layout other than the ordinary pnpm workspace is unverifiable,
/// with the lockfile listed when it exists, and its lockfile is never read.
#[test]
fn unsupported_rush_layouts_are_unverifiable_without_reading_the_lockfile() {
    type Edit = fn(&Path);
    let cases: [(&str, Edit); 8] = [
        ("subspaces enabled", |root| {
            write(root, SUBSPACES, "{ \"subspacesEnabled\": true }");
        }),
        ("unreadable subspaces.json", |root| {
            write(root, SUBSPACES, "{ subspacesEnabled: true }");
        }),
        ("useWorkspaces false", |root| {
            write(root, PNPM_CONFIG, "{ \"useWorkspaces\": false }");
        }),
        ("useWorkspaces omitted (Rush defaults to false)", |root| {
            write(root, PNPM_CONFIG, "{}");
        }),
        ("no pnpm-config.json and no legacy option", |root| {
            fs::remove_file(root.join(PNPM_CONFIG)).expect("remove pnpm-config.json");
        }),
        ("unreadable pnpm-config.json", |root| {
            fs::create_dir_all(root.join(PNPM_CONFIG)).ok();
        }),
        ("variants directory", |root| {
            fs::create_dir_all(root.join("common/config/rush/variants/v1")).expect("variant");
        }),
        ("variants in rush.json", |root| {
            write(
                root,
                "rush.json",
                "{ \"pnpmVersion\": \"9.15.9\", \"variants\": [{ \"variantName\": \"v1\" }] }",
            );
        }),
    ];
    for (label, edit) in cases {
        let dir = rush_pnpm_workspace("  .: {}\n");
        if label == "unreadable pnpm-config.json" {
            fs::remove_file(dir.path().join(PNPM_CONFIG)).expect("remove pnpm-config.json");
        }
        edit(dir.path());

        let (observation, counts) = observe(dir.path(), MonorepoStandard::RushStack, true);

        assert_eq!(
            observation,
            expected(
                LockfileStatus::Unverifiable,
                &[RUSH_LOCK],
                Some(LockfileReason::UnsupportedLayout)
            ),
            "{label}"
        );
        assert_eq!(counts.get(counters::REPO_LOCKFILE_READS), 0, "{label}");
    }
}

/// An unsupported layout is reported as such even without its lockfile,
/// rather than guessed absent.
#[test]
fn an_unsupported_rush_layout_without_a_lockfile_is_not_absent() {
    let dir = rush_pnpm_workspace("  .: {}\n");
    fs::remove_file(dir.path().join(RUSH_LOCK)).expect("remove lockfile");
    write(dir.path(), SUBSPACES, "{ \"subspacesEnabled\": true }");

    let (observation, _) = observe(dir.path(), MonorepoStandard::RushStack, true);

    assert_eq!(
        observation,
        expected(
            LockfileStatus::Unverifiable,
            &[],
            Some(LockfileReason::UnsupportedLayout)
        )
    );

    // A supported layout with no lockfile is absent.
    write(dir.path(), SUBSPACES, "{ \"subspacesEnabled\": false }");
    let (observation, _) = observe(dir.path(), MonorepoStandard::RushStack, true);
    assert_eq!(observation, expected(LockfileStatus::Absent, &[], None));
}

/// Without `pnpm-config.json`, the legacy `rush.json` `pnpmOptions` decides.
#[test]
fn the_legacy_rush_pnpm_option_enables_the_workspace_layout() {
    let dir = rush_pnpm_workspace("  .: {}\n");
    fs::remove_file(dir.path().join(PNPM_CONFIG)).expect("remove pnpm-config.json");
    write(
        dir.path(),
        "rush.json",
        "{ \"pnpmVersion\": \"9.15.9\", \"pnpmOptions\": { \"useWorkspaces\": true } }",
    );

    let (observation, _) = observe(dir.path(), MonorepoStandard::RushStack, true);

    assert_eq!(observation, expected(LockfileStatus::Match, &[RUSH_LOCK], None));
}

/// npm and Yarn managers, and a `rush.json` naming no single manager, are
/// never compared.
#[test]
fn rush_managers_other_than_pnpm_are_unsupported_layouts() {
    for (rush_json, lockfile) in [
        (
            "{ \"npmVersion\": \"6.14.18\" }",
            Some("common/config/rush/npm-shrinkwrap.json"),
        ),
        ("{ \"yarnVersion\": \"1.22.22\" }", Some("common/config/rush/yarn.lock")),
        ("{ \"yarnVersion\": \"1.22.22\" }", None),
        ("{ \"rushVersion\": \"5.179.0\" }", None),
        ("{ \"pnpmVersion\": \"9.15.9\", \"npmVersion\": \"6.14.18\" }", None),
    ] {
        let dir = TempDir::new().expect("tempdir");
        write(dir.path(), "rush.json", rush_json);
        if let Some(lockfile) = lockfile {
            touch(dir.path(), lockfile);
        }
        let paths: Vec<&str> = lockfile.into_iter().collect();

        let (observation, counts) = observe(dir.path(), MonorepoStandard::RushStack, true);
        assert_eq!(
            observation,
            expected(
                LockfileStatus::Unverifiable,
                &paths,
                Some(LockfileReason::UnsupportedLayout)
            ),
            "{rush_json}"
        );
        assert_eq!(counts.get(counters::REPO_LOCKFILE_READS), 0, "{rush_json}");

        let (declined, _) = observe(dir.path(), MonorepoStandard::RushStack, false);
        let expected_declined = if lockfile.is_some() {
            expected(
                LockfileStatus::NotRequested,
                &paths,
                Some(LockfileReason::RequestDisabled),
            )
        } else {
            expected(
                LockfileStatus::Unverifiable,
                &[],
                Some(LockfileReason::UnsupportedLayout),
            )
        };
        assert_eq!(declined, expected_declined, "{rush_json} declined");
    }
}

/// Bazel's module lockfile applies only under Bzlmod: without `MODULE.bazel`
/// there is no lockfile source, even when a stray `MODULE.bazel.lock` exists.
#[test]
fn a_bazel_root_without_module_bazel_has_no_lockfile_source() {
    let dir = TempDir::new().expect("tempdir");
    touch(dir.path(), "WORKSPACE");
    touch(dir.path(), "MODULE.bazel.lock");
    for wants in [true, false] {
        let (observation, counts) = observe(dir.path(), MonorepoStandard::Bazel, wants);
        assert_eq!(
            observation,
            expected(
                LockfileStatus::NotApplicable,
                &[],
                Some(LockfileReason::NoLockfileSource)
            )
        );
        assert_eq!(counts.get(counters::REPO_LOCKFILE_PROBES), 0);
    }

    touch(dir.path(), "MODULE.bazel");
    fs::remove_file(dir.path().join("MODULE.bazel.lock")).expect("remove lockfile");
    let (observation, _) = observe(dir.path(), MonorepoStandard::Bazel, true);
    assert_eq!(observation, expected(LockfileStatus::Absent, &[], None));
}

/// The legacy Gradle group lists each direct `*.lockfile` child of the root
/// `gradle/dependency-locks/`, never a nested or non-lockfile entry, beside
/// any root `gradle.lockfile`.
#[test]
fn legacy_gradle_locks_are_listed_one_level_deep() {
    let dir = TempDir::new().expect("tempdir");
    for file in [
        "gradle/dependency-locks/compileClasspath.lockfile",
        "gradle/dependency-locks/runtimeClasspath.lockfile",
        "gradle/dependency-locks/README.md",
        "gradle/dependency-locks/nested/deep.lockfile",
        "app/gradle/dependency-locks/compileClasspath.lockfile",
    ] {
        touch(dir.path(), file);
    }
    fs::create_dir_all(dir.path().join("gradle/dependency-locks/dir.lockfile"))
        .expect("directory named like a lockfile");
    let legacy = [
        "gradle/dependency-locks/compileClasspath.lockfile",
        "gradle/dependency-locks/runtimeClasspath.lockfile",
    ];

    let (observation, counts) = observe(dir.path(), MonorepoStandard::GradleMultiProject, true);
    assert_eq!(
        observation,
        expected(
            LockfileStatus::Unverifiable,
            &legacy,
            Some(LockfileReason::NoMembershipData)
        )
    );
    assert_eq!(counts.get(counters::REPO_LOCKFILE_READS), 0);
    assert_eq!(counts.get(counters::FS_READ_DIRS), 1);

    let (declined, _) = observe(dir.path(), MonorepoStandard::GradleMultiProject, false);
    assert_eq!(
        declined,
        expected(
            LockfileStatus::NotRequested,
            &legacy,
            Some(LockfileReason::RequestDisabled)
        )
    );

    touch(dir.path(), "gradle.lockfile");
    let (observation, _) = observe(dir.path(), MonorepoStandard::GradleMultiProject, true);
    assert_eq!(
        observation.paths,
        [
            "gradle.lockfile",
            "gradle/dependency-locks/compileClasspath.lockfile",
            "gradle/dependency-locks/runtimeClasspath.lockfile",
        ]
    );
}

/// An empty legacy lock directory, or a file in its place, holds no
/// lockfiles.
#[test]
fn an_empty_or_misplaced_legacy_gradle_lock_directory_is_absent() {
    let dir = TempDir::new().expect("tempdir");
    fs::create_dir_all(dir.path().join("gradle/dependency-locks")).expect("empty lock dir");
    let (observation, _) = observe(dir.path(), MonorepoStandard::GradleMultiProject, true);
    assert_eq!(observation, expected(LockfileStatus::Absent, &[], None));

    let dir = TempDir::new().expect("tempdir");
    touch(dir.path(), "gradle/dependency-locks");
    let (observation, _) = observe(dir.path(), MonorepoStandard::GradleMultiProject, true);
    assert_eq!(observation, expected(LockfileStatus::Absent, &[], None));
}

#[test]
fn a_metadata_failure_on_the_legacy_gradle_directory_is_unreadable() {
    let dir = TempDir::new().expect("tempdir");
    touch(dir.path(), "gradle.lockfile");
    let _failure = test_seam::fail_metadata(
        &dir.path().join("gradle/dependency-locks"),
        std::io::ErrorKind::PermissionDenied,
    );

    let (observation, _) = observe(dir.path(), MonorepoStandard::GradleMultiProject, false);

    assert_eq!(
        observation,
        expected(
            LockfileStatus::Unreadable,
            &[],
            Some(LockfileReason::MetadataFailed)
        )
    );
}
