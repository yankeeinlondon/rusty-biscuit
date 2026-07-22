//! Prints work counters for the specification's baseline cases.
//!
//! Wall-clock benchmarks are directional evidence on a shared host; work
//! counters are the acceptance evidence. This example is the repeatable way to
//! produce them: it runs each baseline case under a scoped
//! `PerformanceCollector` and prints every counter the request recorded, so a
//! later phase can claim "fewer entries visited / opens / parses / status
//! walks" against a comparable number rather than against a timing that moved
//! because the host was busy.
//!
//! Run from the `sniff/` package area:
//!
//! ```text
//! cargo run -p sniff --release --example work_counts
//! ```
//!
//! Fixtures are the same synthetic trees the Criterion benches use, so a
//! counter here and a timing there describe the same workload.

// The bench fixture builders are deliberately `#[path]`-includable so benches,
// profiling examples, and tests materialize byte-identical trees.
#[path = "../benches/support/builder.rs"]
mod builder;
#[path = "../benches/support/fixtures.rs"]
mod fixtures;

use std::sync::Arc;
use std::time::{Duration, Instant};

use sniff::filesystem::{detect_filesystem_with_request, detect_git_with_request, detect_repo};
use sniff::filesystem::repo::detect_repo_structure;
use sniff::performance::{PerformanceCollector, PerformanceReport, with_current_collector};
use sniff::request::{FilesystemRequest, GitRequest, RepoRequest};

/// Dirty files in the rich-status baseline case, matching the spec's table.
const DIRTY_FILES: usize = 100;

fn main() {
    println!("# Sniff work-count baseline\n");
    println!("Fixtures: bench `large_monorepo`, `huge_monorepo` (375 packages), ");
    println!("`git_repo_with_dirty_files({DIRTY_FILES})`.\n");

    let large = fixtures::large_monorepo();
    let huge = fixtures::huge_monorepo();
    let dirty = fixtures::git_repo_with_dirty_files(DIRTY_FILES);

    report("staged_filesystem_summary_git_plus_repo", || {
        let request = FilesystemRequest::new()
            .git(GitRequest::summary())
            .repo(RepoRequest::structure())
            .without_docs()
            .without_formatting()
            .without_file_inventory();
        detect_filesystem_with_request(large.path(), &request).map(drop)
    });

    report("staged_filesystem_full_all_stages", || {
        detect_filesystem_with_request(large.path(), &FilesystemRequest::new()).map(drop)
    });

    report("repo_structure_huge_375_packages", || {
        detect_repo_structure(huge.path()).map(drop)
    });

    report("repo_full_huge_375_packages", || {
        detect_repo(huge.path()).map(drop)
    });

    report("git_status_file_changes_100_dirty", || {
        detect_git_with_request(dirty.path(), &GitRequest::full()).map(drop)
    });

    report("git_status_unified_diffs_100_dirty", || {
        detect_git_with_request(dirty.path(), &GitRequest::deep()).map(drop)
    });
}

/// Runs `case` under a private collector and prints what it cost.
///
/// The elapsed time is printed for orientation only — it is one unmeasured
/// sample on whatever host ran it, and must not be compared across machines or
/// across runs the way the counters can be.
fn report(name: &str, case: impl FnOnce() -> sniff::Result<()>) {
    let collector = PerformanceCollector::new_shared();
    let started = Instant::now();
    let outcome = with_current_collector(Some(Arc::clone(&collector)), case);
    let elapsed = started.elapsed();

    println!("## {name}\n");
    if let Err(error) = outcome {
        println!("FAILED: {error}\n");
        return;
    }

    print_report(&collector.snapshot(elapsed), elapsed);
}

fn print_report(report: &PerformanceReport, elapsed: Duration) {
    println!("elapsed: {:.1} ms (directional only)\n", elapsed.as_secs_f64() * 1000.0);
    if report.counters.is_empty() {
        println!("no counters recorded\n");
        return;
    }
    println!("| counter | value |");
    println!("|---|---:|");
    for (name, value) in &report.counters {
        println!("| `{name}` | {value} |");
    }
    println!();
}
