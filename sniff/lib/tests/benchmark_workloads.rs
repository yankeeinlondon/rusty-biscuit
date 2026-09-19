//! Contract checks for the parameterized Criterion fixture families.

#[path = "../benches/support/builder.rs"]
mod builder;
#[path = "../benches/support/fixtures.rs"]
mod fixtures;

use std::sync::Arc;
use std::time::Duration;

use sniff::filesystem::repo::detect_repo_structure;
use sniff::filesystem::{
    FilesystemObservation, detect_filesystem_with_observation, detect_filesystem_with_request,
};
use sniff::performance::{PerformanceCollector, counters, with_current_collector};
use sniff::request::{FilesystemRequest, GitRequest};

#[test]
fn formatting_workload_keeps_descendant_work_at_zero() {
    let fixture = fixtures::deep_wide_formatting_tree(4, 4);
    let request = FilesystemRequest::new()
        .without_git()
        .without_repo()
        .without_docs()
        .without_file_inventory();
    let collector = PerformanceCollector::new_shared();

    with_current_collector(Some(Arc::clone(&collector)), || {
        detect_filesystem_with_request(fixture.path(), &request).unwrap()
    });
    let counts = collector.snapshot(Duration::ZERO).counters;

    assert_eq!(
        counts.get(counters::FS_WALK_STARTS).copied().unwrap_or(0),
        0
    );
    assert_eq!(
        counts.get(counters::FS_WALK_ENTRIES).copied().unwrap_or(0),
        0
    );
    assert_eq!(
        counts.get(counters::GIT_DISCOVERIES).copied().unwrap_or(0),
        0
    );
    assert_eq!(
        counts.get(counters::GIT_STATUS_WALKS).copied().unwrap_or(0),
        0
    );
}

#[test]
fn seeded_git_execution_and_projection_do_not_rediscover_the_repository() {
    let fixture = fixtures::git_repo_with_dirty_files(4);

    let acquisition = PerformanceCollector::new_shared();
    let observation = with_current_collector(Some(Arc::clone(&acquisition)), || {
        FilesystemObservation::discover(fixture.path())
    });
    let acquisition_counts = acquisition.snapshot(Duration::ZERO).counters;
    assert_eq!(
        acquisition_counts
            .get(counters::GIT_DISCOVERIES)
            .copied()
            .unwrap_or(0),
        1
    );

    let execution = PerformanceCollector::new_shared();
    let (filesystem, changes) = with_current_collector(Some(Arc::clone(&execution)), || {
        let filesystem = detect_filesystem_with_observation(
            fixture.path(),
            &FilesystemRequest::new()
                .git(GitRequest::full())
                .without_repo()
                .without_docs()
                .without_formatting()
                .without_file_inventory(),
            &observation,
        )
        .expect("seeded filesystem detection");
        let changes = observation
            .detect_file_changes()
            .expect("seeded file-change projection")
            .expect("fixture repository observation");
        (filesystem, changes)
    });

    let git = filesystem.git.expect("Git was requested");
    assert!(git.status.is_some_and(|status| status.is_dirty));
    assert_eq!(changes.len(), 4);
    assert!(changes.iter().all(|change| change.path.starts_with("src")));

    let execution_counts = execution.snapshot(Duration::ZERO).counters;
    assert_eq!(
        execution_counts
            .get(counters::GIT_DISCOVERIES)
            .copied()
            .unwrap_or(0),
        0,
        "seeded execution and projection must reuse the observed handle"
    );
    assert_eq!(
        execution_counts
            .get(counters::GIT_OPENS)
            .copied()
            .unwrap_or(0),
        0,
        "reusing an observed handle must not reopen the repository"
    );
    assert_eq!(
        execution_counts
            .get(counters::GIT_STATUS_WALKS)
            .copied()
            .unwrap_or(0),
        2,
        "full detection and file-change projection each perform one requested status walk"
    );
}

#[test]
fn mixed_structure_workload_preserves_its_requested_cardinality() {
    let fixture = fixtures::mixed_monorepo(100);
    let repo = detect_repo_structure(fixture.path())
        .unwrap()
        .expect("mixed fixture should be a monorepo");
    assert_eq!(repo.packages.expect("package catalog").len(), 100);
}

#[test]
fn sized_dirty_workload_writes_exact_payload_sizes() {
    let fixture = fixtures::git_repo_with_dirty_files_of_size(100, 1_024);
    for i in 0..100 {
        let metadata = std::fs::metadata(fixture.path().join(format!("src/m{i:04}.rs"))).unwrap();
        assert_eq!(metadata.len(), 1_024);
    }
}
