//! Contract checks for the parameterized Criterion fixture families.

#[path = "../benches/support/builder.rs"]
mod builder;
#[path = "../benches/support/fixtures.rs"]
mod fixtures;

use std::sync::Arc;
use std::time::Duration;

use sniff::filesystem::detect_filesystem_with_request;
use sniff::filesystem::repo::detect_repo_structure;
use sniff::performance::{PerformanceCollector, counters, with_current_collector};
use sniff::request::FilesystemRequest;

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

    assert_eq!(counts.get(counters::FS_WALK_STARTS).copied().unwrap_or(0), 0);
    assert_eq!(counts.get(counters::FS_WALK_ENTRIES).copied().unwrap_or(0), 0);
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
