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
        MonorepoStandard::Bazel,
        MonorepoStandard::RushStack,
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

/// Formats whose parser lands in a later phase can never yield `match` or
/// `mismatch` in the meantime.
#[test]
fn stubbed_formats_are_unverifiable_after_one_read() {
    for (authority, lockfile) in [
        (MonorepoStandard::NpmWorkspaces, "package-lock.json"),
        (MonorepoStandard::YarnWorkspaces, "yarn.lock"),
        (MonorepoStandard::BunWorkspaces, "bun.lock"),
    ] {
        let dir = TempDir::new().expect("tempdir");
        touch(dir.path(), lockfile);

        let (observation, counts) = observe(dir.path(), authority, true);

        assert_eq!(
            observation,
            expected(
                LockfileStatus::Unverifiable,
                &[lockfile],
                Some(LockfileReason::UnsupportedVersion)
            ),
            "{authority:?}"
        );
        assert_eq!(counts.get(counters::REPO_LOCKFILE_READS), 1, "{authority:?}");
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
