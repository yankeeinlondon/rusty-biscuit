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
    LockfileObservation, LockfileReason, LockfileStatus, detect_repo_with_request,
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
