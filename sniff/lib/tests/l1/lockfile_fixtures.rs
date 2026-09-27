//! Real-tool lockfile fixtures through the public detection API
//! (`2026-09-26-lockfile-corroboration`, AC2).
//!
//! Each fixture under `tests/fixtures/lockfiles/<tool>-<version>/<case>/` was
//! written by the recorded tool (see its `PROVENANCE.md`). It is copied into a
//! temporary directory before detection, because in place this monorepo's
//! `.git` and workspaces would take over. Expected results are the
//! "Expected results" table of the feature's `accepted-versions.md`.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use sniff::filesystem::repo::{
    LockfileObservation, LockfileReason, LockfileStatus, StandaloneLockfileObservation,
    StandaloneLockfileTool, detect_repo_with_request, detect_repo_with_request_or_root_package,
};
use sniff::filesystem::{MonorepoStandard, PackageProvenance, RepoInfo};
use sniff::performance::{PerformanceCollector, counters, with_current_collector};
use sniff::request::RepoRequest;
use tempfile::TempDir;

fn fixture_root() -> PathBuf {
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

/// A disposable copy of one fixture directory.
fn copied(fixture: &str) -> (TempDir, PathBuf) {
    let source = fixture_root().join(fixture);
    assert!(source.is_dir(), "missing fixture {}", source.display());
    let dir = TempDir::new().expect("tempdir");
    let root = dir.path().join("repo");
    copy_tree(&source, &root);
    (dir, root)
}

fn measured(
    root: &Path,
    request: &RepoRequest,
) -> (RepoInfo, BTreeMap<String, u64>) {
    let collector = PerformanceCollector::new_shared();
    let repo = with_current_collector(Some(Arc::clone(&collector)), || {
        detect_repo_with_request(root, request)
    });
    let repo = repo
        .expect("detection succeeds")
        .expect("the fixture is a workspace");
    (repo, collector.snapshot(Duration::ZERO).counters)
}

fn counter(counts: &BTreeMap<String, u64>, name: &str) -> u64 {
    counts.get(name).copied().unwrap_or(0)
}

/// One expected fixture result: `(fixture, authority, lockfile, status,
/// reason, extra, missing)`.
type Row = (
    &'static str,
    MonorepoStandard,
    &'static str,
    LockfileStatus,
    Option<LockfileReason>,
    &'static [&'static str],
    &'static [&'static str],
);

fn observation(
    lockfile: &str,
    status: LockfileStatus,
    reason: Option<LockfileReason>,
    extra: &[&str],
    missing: &[&str],
) -> LockfileObservation {
    let owned = |items: &[&str]| items.iter().map(|item| (*item).to_owned()).collect();
    LockfileObservation {
        status,
        paths: vec![lockfile.to_owned()],
        reason,
        extra: owned(extra),
        missing: owned(missing),
    }
}

/// Assert each row's layer under a corroborating request, then that a
/// declining request reports it as `not_requested` without reading it.
fn assert_rows(rows: &[Row]) {
    for &(fixture, authority, lockfile, status, reason, extra, missing) in rows {
        let (_dir, root) = copied(fixture);

        let (repo, counts) = measured(&root, &RepoRequest::full());
        let layers: Vec<_> = repo
            .monorepo_layers
            .iter()
            .filter(|layer| layer.authority == authority)
            .collect();
        assert_eq!(layers.len(), 1, "{fixture}: {:?}", repo.monorepo_layers);
        let layer = layers[0];
        assert_eq!(
            layer.lockfile,
            observation(lockfile, status, reason, extra, missing),
            "{fixture}"
        );
        assert_eq!(counter(&counts, counters::REPO_LOCKFILE_READS), 1, "{fixture}");
        assert_eq!(counter(&counts, counters::REPO_LOCKFILE_PARSES), 1, "{fixture}");

        // Only an exact match upgrades the layer and the packages it owns.
        let upgraded = status == LockfileStatus::Match;
        assert_eq!(
            layer.provenance == PackageProvenance::Lockfile,
            upgraded,
            "{fixture}: {layer:?}"
        );
        for package in repo.packages.as_deref().expect("packages") {
            if package.provenance == PackageProvenance::Lockfile {
                assert!(upgraded, "{fixture}: {package:?}");
                assert_eq!(package.standard, authority, "{fixture}: {package:?}");
            }
        }

        let (repo, counts) = measured(&root, &RepoRequest::structure());
        let layer = repo
            .monorepo_layers
            .iter()
            .find(|layer| layer.authority == authority)
            .unwrap_or_else(|| panic!("{fixture}: no declined layer"));
        assert_eq!(
            layer.lockfile,
            observation(
                lockfile,
                LockfileStatus::NotRequested,
                Some(LockfileReason::RequestDisabled),
                &[],
                &[]
            ),
            "{fixture} declined"
        );
        assert_ne!(layer.provenance, PackageProvenance::Lockfile, "{fixture}");
        assert_eq!(counter(&counts, counters::REPO_LOCKFILE_READS), 0, "{fixture}");
        assert_eq!(counter(&counts, counters::REPO_LOCKFILE_PARSES), 0, "{fixture}");
    }
}

#[test]
fn pnpm_tool_fixtures_report_the_accepted_version_matrix() {
    use LockfileStatus::{Match, Mismatch, Unreadable, Unverifiable};
    const PNPM: MonorepoStandard = MonorepoStandard::PnpmWorkspaces;
    const LOCK: &str = "pnpm-lock.yaml";
    let parse_failed = Some(LockfileReason::ParseFailed);
    assert_rows(&[
        ("pnpm-8.15.9/workspace", PNPM, LOCK, Match, None, &[], &[]),
        ("pnpm-9.15.9/workspace", PNPM, LOCK, Match, None, &[], &[]),
        ("pnpm-10.32.1/workspace", PNPM, LOCK, Match, None, &[], &[]),
        (
            "pnpm-10.32.1/workspace-edited-stale-extra",
            PNPM,
            LOCK,
            Mismatch,
            None,
            &["packages/gamma"],
            &[],
        ),
        (
            "pnpm-10.32.1/workspace-edited-missing",
            PNPM,
            LOCK,
            Mismatch,
            None,
            &[],
            &["packages/beta"],
        ),
        (
            "pnpm-10.32.1/workspace-edited-malformed-trailing",
            PNPM,
            LOCK,
            Unreadable,
            parse_failed,
            &[],
            &[],
        ),
        (
            "pnpm-10.32.1/workspace-edited-duplicate-key",
            PNPM,
            LOCK,
            Unreadable,
            parse_failed,
            &[],
            &[],
        ),
        (
            "pnpm-10.32.1/workspace-edited-unknown-version",
            PNPM,
            LOCK,
            Unverifiable,
            Some(LockfileReason::UnsupportedVersion),
            &[],
            &[],
        ),
        (
            "pnpm-10.32.1/workspace-edited-missing-required-field",
            PNPM,
            LOCK,
            Unreadable,
            parse_failed,
            &[],
            &[],
        ),
    ]);
}

#[test]
fn uv_tool_fixtures_report_the_accepted_version_matrix() {
    use LockfileStatus::{Match, Mismatch, Unreadable, Unverifiable};
    const UV: MonorepoStandard = MonorepoStandard::UvWorkspace;
    const LOCK: &str = "uv.lock";
    assert_rows(&[
        ("uv-0.9.5/workspace", UV, LOCK, Match, None, &[], &[]),
        ("uv-0.9.5/virtual-root", UV, LOCK, Match, None, &[], &[]),
        (
            "uv-0.9.5/workspace-edited-stale-extra",
            UV,
            LOCK,
            Mismatch,
            None,
            &["packages/gamma"],
            &[],
        ),
        (
            "uv-0.9.5/workspace-edited-missing",
            UV,
            LOCK,
            Mismatch,
            None,
            &[],
            &["packages/beta"],
        ),
        (
            "uv-0.9.5/workspace-edited-malformed-trailing",
            UV,
            LOCK,
            Unreadable,
            Some(LockfileReason::ParseFailed),
            &[],
            &[],
        ),
        (
            "uv-0.9.5/workspace-edited-unknown-version",
            UV,
            LOCK,
            Unverifiable,
            Some(LockfileReason::UnsupportedVersion),
            &[],
            &[],
        ),
        // An absent `[manifest]` means the root alone, so every current member
        // is missing rather than the document failing to parse.
        (
            "uv-0.9.5/workspace-edited-missing-required-field",
            UV,
            LOCK,
            Mismatch,
            None,
            &[],
            &[".tools/hidden", "packages/alpha", "packages/beta"],
        ),
    ]);
}

#[test]
fn cargo_tool_fixtures_report_subset_evidence() {
    use LockfileStatus::{MembersMissing, MembersPresent, Unreadable, Unverifiable};
    const CARGO: MonorepoStandard = MonorepoStandard::CargoWorkspace;
    const LOCK: &str = "Cargo.lock";
    let subset = Some(LockfileReason::SubsetOnly);
    let parse_failed = Some(LockfileReason::ParseFailed);
    assert_rows(&[
        ("cargo-1.98.1/workspace", CARGO, LOCK, MembersPresent, subset, &[], &[]),
        ("cargo-1.77.2/workspace-v3", CARGO, LOCK, MembersPresent, subset, &[], &[]),
        ("cargo-1.52.0/workspace-v2", CARGO, LOCK, MembersPresent, subset, &[], &[]),
        // Cargo cannot recover a stale extra member.
        (
            "cargo-1.98.1/workspace-edited-stale-extra",
            CARGO,
            LOCK,
            MembersPresent,
            subset,
            &[],
            &[],
        ),
        // The registry `itoa` 1.0.18 does not satisfy the member `itoa` 0.1.0.
        (
            "cargo-1.98.1/workspace-edited-missing",
            CARGO,
            LOCK,
            MembersMissing,
            subset,
            &[],
            &["crates/beta"],
        ),
        (
            "cargo-1.98.1/workspace-edited-malformed-trailing",
            CARGO,
            LOCK,
            Unreadable,
            parse_failed,
            &[],
            &[],
        ),
        (
            "cargo-1.98.1/workspace-edited-unknown-version",
            CARGO,
            LOCK,
            Unverifiable,
            Some(LockfileReason::UnsupportedVersion),
            &[],
            &[],
        ),
        (
            "cargo-1.98.1/workspace-edited-missing-required-field",
            CARGO,
            LOCK,
            Unreadable,
            parse_failed,
            &[],
            &[],
        ),
    ]);
}

/// A uv workspace whose only member is its root declares no members, so the
/// uv detector reports no layer and no observation is made. The lockfile's
/// root-only shape (no `[manifest]`) is proven by the parser's fixture corpus.
#[test]
fn a_root_only_uv_workspace_has_no_layer_to_corroborate() {
    let (_dir, root) = copied("uv-0.9.5/root-only-workspace");

    let repo = detect_repo_with_request(&root, &RepoRequest::full()).expect("detection succeeds");

    assert!(
        repo.as_ref().is_none_or(|repo| repo
            .monorepo_layers
            .iter()
            .all(|layer| layer.authority != MonorepoStandard::UvWorkspace)),
        "{repo:?}"
    );
}

/// The serialized layer always carries all five fields, with empty lists and
/// a `null` reason spelled out.
#[test]
fn the_serialized_layer_carries_the_complete_lockfile_object() {
    let (_dir, root) = copied("pnpm-10.32.1/workspace-edited-missing");

    let (repo, _) = measured(&root, &RepoRequest::full());

    let json = serde_json::to_value(&repo).expect("RepoInfo serializes");
    let layer = &json["monorepo_layers"][0];
    assert!(layer.get("lockfile_match").is_none(), "{layer}");
    assert_eq!(
        layer["lockfile"],
        serde_json::json!({
            "status": "mismatch",
            "paths": ["pnpm-lock.yaml"],
            "reason": null,
            "extra": [],
            "missing": ["packages/beta"],
        })
    );
}

#[test]
fn npm_tool_fixtures_report_the_accepted_version_matrix() {
    use LockfileStatus::{Match, Mismatch, Unreadable, Unverifiable};
    const NPM: MonorepoStandard = MonorepoStandard::NpmWorkspaces;
    const LOCK: &str = "package-lock.json";
    let parse_failed = Some(LockfileReason::ParseFailed);
    assert_rows(&[
        ("npm-11.6.4/workspace", NPM, LOCK, Match, None, &[], &[]),
        ("npm-11.6.4/workspace-lockfile-v2", NPM, LOCK, Match, None, &[], &[]),
        ("npm-11.6.4/shrinkwrap", NPM, "npm-shrinkwrap.json", Match, None, &[], &[]),
        (
            "npm-11.6.4/workspace-edited-stale-extra",
            NPM,
            LOCK,
            Mismatch,
            None,
            &["packages/gamma"],
            &[],
        ),
        (
            "npm-11.6.4/workspace-edited-missing",
            NPM,
            LOCK,
            Mismatch,
            None,
            &[],
            &["packages/beta"],
        ),
        (
            "npm-11.6.4/workspace-edited-malformed-trailing",
            NPM,
            LOCK,
            Unreadable,
            parse_failed,
            &[],
            &[],
        ),
        (
            "npm-11.6.4/workspace-edited-duplicate-key",
            NPM,
            LOCK,
            Unreadable,
            parse_failed,
            &[],
            &[],
        ),
        (
            "npm-11.6.4/workspace-edited-unknown-version",
            NPM,
            LOCK,
            Unverifiable,
            Some(LockfileReason::UnsupportedVersion),
            &[],
            &[],
        ),
        (
            "npm-11.6.4/workspace-edited-missing-required-field",
            NPM,
            LOCK,
            Unreadable,
            parse_failed,
            &[],
            &[],
        ),
    ]);
}

/// npm 6 wrote no workspaces, so its real v1 lockfile is placed beside the
/// npm 11 workspace manifests: the version alone makes it unverifiable.
#[test]
fn an_npm_v1_lockfile_is_an_unsupported_version_through_detection() {
    let (_dir, root) = copied("npm-11.6.4/workspace");
    fs::copy(
        fixture_root().join("npm-6.14.18/single-project/package-lock.json"),
        root.join("package-lock.json"),
    )
    .expect("replace the lockfile");

    let (repo, counts) = measured(&root, &RepoRequest::full());

    let layer = &repo.monorepo_layers[0];
    assert_eq!(layer.authority, MonorepoStandard::NpmWorkspaces);
    assert_eq!(
        layer.lockfile,
        observation(
            "package-lock.json",
            LockfileStatus::Unverifiable,
            Some(LockfileReason::UnsupportedVersion),
            &[],
            &[]
        )
    );
    assert_ne!(layer.provenance, PackageProvenance::Lockfile);
    assert_eq!(counter(&counts, counters::REPO_LOCKFILE_PARSES), 1);
}

/// `npm-shrinkwrap.json` wins over `package-lock.json` (ruling R6): only the
/// winner is listed, read, and parsed, even when the loser would disagree.
#[test]
fn npm_shrinkwrap_takes_precedence_over_package_lock() {
    let (_dir, root) = copied("npm-11.6.4/shrinkwrap");
    fs::copy(
        fixture_root().join("npm-11.6.4/workspace-edited-missing/package-lock.json"),
        root.join("package-lock.json"),
    )
    .expect("add a disagreeing package-lock.json");

    let (repo, counts) = measured(&root, &RepoRequest::full());

    assert_eq!(
        repo.monorepo_layers[0].lockfile,
        observation("npm-shrinkwrap.json", LockfileStatus::Match, None, &[], &[])
    );
    assert_eq!(counter(&counts, counters::REPO_LOCKFILE_READS), 1);
}

#[test]
fn yarn_tool_fixtures_report_the_accepted_version_matrix() {
    use LockfileStatus::{Match, Mismatch, Unreadable, Unverifiable};
    const YARN: MonorepoStandard = MonorepoStandard::YarnWorkspaces;
    const LOCK: &str = "yarn.lock";
    let parse_failed = Some(LockfileReason::ParseFailed);
    let unsupported = Some(LockfileReason::UnsupportedVersion);
    assert_rows(&[
        ("yarn-3.8.7/workspace", YARN, LOCK, Match, None, &[], &[]),
        ("yarn-4.0.2/workspace", YARN, LOCK, Match, None, &[], &[]),
        ("yarn-4.18.1/workspace", YARN, LOCK, Match, None, &[], &[]),
        ("yarn-1.22.22/workspace", YARN, LOCK, Unverifiable, unsupported, &[], &[]),
        (
            "yarn-4.18.1/workspace-edited-stale-extra",
            YARN,
            LOCK,
            Mismatch,
            None,
            &["packages/gamma"],
            &[],
        ),
        (
            "yarn-4.18.1/workspace-edited-missing",
            YARN,
            LOCK,
            Mismatch,
            None,
            &[],
            &["packages/beta"],
        ),
        (
            "yarn-4.18.1/workspace-edited-malformed-trailing",
            YARN,
            LOCK,
            Unreadable,
            parse_failed,
            &[],
            &[],
        ),
        (
            "yarn-4.18.1/workspace-edited-duplicate-key",
            YARN,
            LOCK,
            Unreadable,
            parse_failed,
            &[],
            &[],
        ),
        (
            "yarn-4.18.1/workspace-edited-unknown-version",
            YARN,
            LOCK,
            Unverifiable,
            unsupported,
            &[],
            &[],
        ),
        (
            "yarn-4.18.1/workspace-edited-missing-required-field",
            YARN,
            LOCK,
            Unreadable,
            parse_failed,
            &[],
            &[],
        ),
    ]);
}

#[test]
fn bun_tool_fixtures_report_the_accepted_version_matrix() {
    use LockfileStatus::{Match, Mismatch, Unreadable, Unverifiable};
    const BUN: MonorepoStandard = MonorepoStandard::BunWorkspaces;
    const LOCK: &str = "bun.lock";
    let parse_failed = Some(LockfileReason::ParseFailed);
    assert_rows(&[
        ("bun-1.2.0/workspace", BUN, LOCK, Match, None, &[], &[]),
        ("bun-1.3.3/workspace", BUN, LOCK, Match, None, &[], &[]),
        (
            "bun-1.3.3/workspace-edited-comments-trailing-commas",
            BUN,
            LOCK,
            Match,
            None,
            &[],
            &[],
        ),
        // Ruling R6: `bun.lock` wins, and `bun.lockb` is not listed.
        ("bun-1.3.3/precedence-both", BUN, LOCK, Match, None, &[], &[]),
        (
            "bun-1.3.3/workspace-edited-stale-extra",
            BUN,
            LOCK,
            Mismatch,
            None,
            &["packages/gamma"],
            &[],
        ),
        (
            "bun-1.3.3/workspace-edited-missing",
            BUN,
            LOCK,
            Mismatch,
            None,
            &[],
            &["packages/beta"],
        ),
        (
            "bun-1.3.3/workspace-edited-malformed-trailing",
            BUN,
            LOCK,
            Unreadable,
            parse_failed,
            &[],
            &[],
        ),
        (
            "bun-1.3.3/workspace-edited-duplicate-key",
            BUN,
            LOCK,
            Unreadable,
            parse_failed,
            &[],
            &[],
        ),
        (
            "bun-1.3.3/workspace-edited-unknown-version",
            BUN,
            LOCK,
            Unverifiable,
            Some(LockfileReason::UnsupportedVersion),
            &[],
            &[],
        ),
        (
            "bun-1.3.3/workspace-edited-missing-required-field",
            BUN,
            LOCK,
            Unreadable,
            parse_failed,
            &[],
            &[],
        ),
    ]);
}

/// Binary `bun.lockb` is reported from metadata alone: never read or parsed,
/// and Bun is never invoked.
#[test]
fn a_binary_bun_lockfile_is_unverifiable_without_a_read() {
    let (_dir, root) = copied("bun-1.3.3/workspace-binary");

    for (request, status, reason) in [
        (
            RepoRequest::full(),
            LockfileStatus::Unverifiable,
            LockfileReason::NoMembershipData,
        ),
        (
            RepoRequest::structure(),
            LockfileStatus::NotRequested,
            LockfileReason::RequestDisabled,
        ),
    ] {
        let (repo, counts) = measured(&root, &request);
        let layer = repo
            .monorepo_layers
            .iter()
            .find(|layer| layer.authority == MonorepoStandard::BunWorkspaces)
            .expect("a Bun layer");
        assert_eq!(
            layer.lockfile,
            observation("bun.lockb", status, Some(reason), &[], &[])
        );
        assert_eq!(counter(&counts, counters::REPO_LOCKFILE_READS), 0);
        assert_eq!(counter(&counts, counters::REPO_LOCKFILE_PARSES), 0);
    }
}

/// Regression: real `rush.json` files carry comments, and the former strict
/// JSON parse found no Rush layer at all. The ordinary pnpm workspace layout
/// compares importer keys resolved against `common/temp`.
#[test]
fn rush_tool_fixture_reports_the_accepted_version_matrix() {
    assert_rows(&[(
        "rush-5.179.0/pnpm-workspace",
        MonorepoStandard::RushStack,
        "common/config/rush/pnpm-lock.yaml",
        LockfileStatus::Match,
        None,
        &[],
        &[],
    )]);
}

fn edit(root: &Path, relative: &str, from: &str, to: &str) {
    let path = root.join(relative);
    let content = fs::read_to_string(&path).expect("read fixture file");
    assert!(content.contains(from), "{relative} lacks {from:?}");
    fs::write(&path, content.replacen(from, to, 1)).expect("write fixture file");
}

fn rush_layer_lockfile(root: &Path) -> LockfileObservation {
    let (repo, _) = measured(root, &RepoRequest::full());
    repo.monorepo_layers
        .into_iter()
        .find(|layer| layer.authority == MonorepoStandard::RushStack)
        .expect("a Rush layer")
        .lockfile
}

/// Edits of the real Rush fixture, one setting at a time: a dropped importer
/// is a missing member, and each layout ruling R3 excludes is unverifiable.
#[test]
fn rush_tool_fixture_variants_follow_ruling_r3() {
    const LOCK: &str = "common/config/rush/pnpm-lock.yaml";

    let (_dir, root) = copied("rush-5.179.0/pnpm-workspace");
    edit(&root, LOCK, "  ../../packages/beta:", "  ../../packages/renamed:");
    assert_eq!(
        rush_layer_lockfile(&root),
        observation(
            LOCK,
            LockfileStatus::Mismatch,
            None,
            &["packages/renamed"],
            &["packages/beta"]
        )
    );

    let unsupported = observation(
        LOCK,
        LockfileStatus::Unverifiable,
        Some(LockfileReason::UnsupportedLayout),
        &[],
        &[],
    );
    for (file, from, to) in [
        (
            "common/config/rush/subspaces.json",
            "\"subspacesEnabled\": false",
            "\"subspacesEnabled\": true",
        ),
        (
            "common/config/rush/pnpm-config.json",
            "\"useWorkspaces\": true",
            "\"useWorkspaces\": false",
        ),
    ] {
        let (_dir, root) = copied("rush-5.179.0/pnpm-workspace");
        edit(&root, file, from, to);
        assert_eq!(rush_layer_lockfile(&root), unsupported, "{file}: {to}");
    }

    let (_dir, root) = copied("rush-5.179.0/pnpm-workspace");
    fs::create_dir_all(root.join("common/config/rush/variants/v1")).expect("variant");
    assert_eq!(rush_layer_lockfile(&root), unsupported, "variants");
}

fn layer_lockfile(repo: &RepoInfo, authority: MonorepoStandard, fixture: &str) -> LockfileObservation {
    let layers: Vec<_> = repo
        .monorepo_layers
        .iter()
        .filter(|layer| layer.authority == authority)
        .collect();
    assert_eq!(layers.len(), 1, "{fixture}: {:?}", repo.monorepo_layers);
    layers[0].lockfile.clone()
}

/// Metadata-only sources on real-tool fixtures: a present file is
/// `unverifiable` + `no_membership_data` (or `not_requested` when declined),
/// and nothing is ever read or parsed. Subproject-only Gradle locks are
/// never searched, so those fixtures are `absent`.
#[test]
fn fallback_tool_fixtures_are_reported_from_metadata_alone() {
    let legacy: Vec<String> = [
        "annotationProcessor",
        "archives",
        "compile",
        "compileClasspath",
        "compileOnly",
        "default",
        "runtime",
        "runtimeClasspath",
        "testAnnotationProcessor",
        "testCompile",
        "testCompileClasspath",
        "testCompileOnly",
        "testRuntime",
        "testRuntimeClasspath",
    ]
    .iter()
    .map(|name| format!("gradle/dependency-locks/{name}.lockfile"))
    .collect();
    let legacy: Vec<&str> = legacy.iter().map(String::as_str).collect();
    let cases: [(&str, MonorepoStandard, &[&str]); 6] = [
        ("go-1.27.1/workspace", MonorepoStandard::GoWorkspace, &["go.work.sum"]),
        (
            "gradle-8.14.5/root-lockfile",
            MonorepoStandard::GradleMultiProject,
            &["gradle.lockfile"],
        ),
        (
            "gradle-5.6.4/root-legacy-lock-dir",
            MonorepoStandard::GradleMultiProject,
            &legacy,
        ),
        ("gradle-8.14.5/multi-project", MonorepoStandard::GradleMultiProject, &[]),
        ("gradle-5.6.4/legacy-lock-dir", MonorepoStandard::GradleMultiProject, &[]),
        ("bazel-8.4.2/bzlmod", MonorepoStandard::Bazel, &["MODULE.bazel.lock"]),
    ];
    assert_eq!(legacy.len(), 14);
    for (fixture, authority, paths) in cases {
        let (_dir, root) = copied(fixture);
        if authority == MonorepoStandard::Bazel {
            // The Bazel detector needs leaf packages; the fixture ships only
            // the module files Bazel itself wrote.
            for package in ["app", "lib"] {
                fs::create_dir_all(root.join(package)).expect("create package");
                fs::write(root.join(package).join("BUILD.bazel"), "").expect("write BUILD");
            }
        }
        let owned = |items: &[&str]| items.iter().map(|item| (*item).to_owned()).collect();
        for (request, status, reason) in [
            (
                RepoRequest::full(),
                LockfileStatus::Unverifiable,
                LockfileReason::NoMembershipData,
            ),
            (
                RepoRequest::structure(),
                LockfileStatus::NotRequested,
                LockfileReason::RequestDisabled,
            ),
        ] {
            let (repo, counts) = measured(&root, &request);
            let expected = if paths.is_empty() {
                LockfileObservation {
                    status: LockfileStatus::Absent,
                    paths: Vec::new(),
                    reason: None,
                    extra: Vec::new(),
                    missing: Vec::new(),
                }
            } else {
                LockfileObservation {
                    status,
                    paths: owned(paths),
                    reason: Some(reason),
                    extra: Vec::new(),
                    missing: Vec::new(),
                }
            };
            assert_eq!(layer_lockfile(&repo, authority, fixture), expected, "{fixture}");
            assert_eq!(counter(&counts, counters::REPO_LOCKFILE_READS), 0, "{fixture}");
            assert_eq!(counter(&counts, counters::REPO_LOCKFILE_PARSES), 0, "{fixture}");
        }
    }
}

fn standalone(
    root: &str,
    tool: StandaloneLockfileTool,
    lockfile: &str,
    status: LockfileStatus,
    reason: LockfileReason,
) -> StandaloneLockfileObservation {
    StandaloneLockfileObservation {
        root: root.to_owned(),
        tool,
        observation: LockfileObservation {
            status,
            paths: vec![lockfile.to_owned()],
            reason: Some(reason),
            extra: Vec::new(),
            missing: Vec::new(),
        },
    }
}

/// Ruling R2 on real-tool fixtures: a standalone Poetry, PDM, or Composer
/// project is a root package with no layer, and its lockfile is reported at
/// the repository level from metadata alone. Composer's fixture gains a
/// `package.json`, because Sniff recognizes no PHP-only package.
#[test]
fn standalone_tool_fixtures_are_repository_level_observations() {
    for (fixture, tool, lockfile) in [
        ("poetry-2.5.1/single-project", StandaloneLockfileTool::Poetry, "poetry.lock"),
        ("pdm-2.29.2/single-project", StandaloneLockfileTool::Pdm, "pdm.lock"),
        ("composer-2.10.3/single-project", StandaloneLockfileTool::Composer, "composer.lock"),
    ] {
        let (_dir, root) = copied(fixture);
        if tool == StandaloneLockfileTool::Composer {
            fs::write(root.join("package.json"), r#"{"name": "front-end"}"#)
                .expect("write package.json");
        }
        for (request, status, reason) in [
            (
                RepoRequest::full(),
                LockfileStatus::Unverifiable,
                LockfileReason::NoMembershipData,
            ),
            (
                RepoRequest::structure(),
                LockfileStatus::NotRequested,
                LockfileReason::RequestDisabled,
            ),
        ] {
            let collector = PerformanceCollector::new_shared();
            let repo = with_current_collector(Some(Arc::clone(&collector)), || {
                detect_repo_with_request_or_root_package(&root, &request)
            })
            .expect("detection succeeds")
            .expect("a root package");
            let counts = collector.snapshot(Duration::ZERO).counters;

            assert!(repo.monorepo_layers.is_empty(), "{fixture}");
            assert_eq!(
                repo.standalone_lockfiles,
                [standalone("", tool, lockfile, status, reason)],
                "{fixture}"
            );
            assert_eq!(counter(&counts, counters::REPO_LOCKFILE_READS), 0, "{fixture}");

            let json = serde_json::to_value(&repo).expect("RepoInfo serializes");
            assert_eq!(
                json["standalone_lockfiles"][0]["tool"],
                serde_json::to_value(tool).expect("tool serializes"),
                "{fixture}"
            );
        }
    }
}

/// A package root inside a workspace is probed too, and a lockfile that is
/// neither at the root nor at a package root is never looked for.
#[test]
fn standalone_lockfiles_at_workspace_package_roots_are_reported() {
    let (_dir, root) = copied("pnpm-10.32.1/workspace");
    fs::copy(
        fixture_root().join("poetry-2.5.1/single-project/poetry.lock"),
        root.join("packages/alpha/poetry.lock"),
    )
    .expect("add a package-level poetry.lock");
    fs::create_dir_all(root.join("vendor/lib")).expect("vendor directory");
    fs::write(root.join("vendor/lib/composer.lock"), "{}").expect("unprobed lockfile");

    let (repo, _) = measured(&root, &RepoRequest::full());

    assert_eq!(
        repo.standalone_lockfiles,
        [standalone(
            "packages/alpha",
            StandaloneLockfileTool::Poetry,
            "poetry.lock",
            LockfileStatus::Unverifiable,
            LockfileReason::NoMembershipData,
        )]
    );
    // The pnpm layer is unaffected.
    assert_eq!(repo.monorepo_layers[0].lockfile.status, LockfileStatus::Match);
}

/// The library always serializes the list, `[]` when empty.
#[test]
fn an_empty_standalone_list_is_still_serialized() {
    let (_dir, root) = copied("pnpm-10.32.1/workspace");

    let (repo, _) = measured(&root, &RepoRequest::structure());

    let json = serde_json::to_value(&repo).expect("RepoInfo serializes");
    assert_eq!(json["standalone_lockfiles"], serde_json::json!([]));
}
