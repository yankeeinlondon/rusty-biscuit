//! Git dirty-file scaling Criterion benches.
//!
//! These benches isolate the cost of producing per-file change stats and
//! deep-mode unified diffs as the number of modified working-tree files
//! grows. The fixtures are deliberately minimal (one tracked source file
//! per dirty entry) so the measured cost reflects diff aggregation rather
//! than fixture size or per-package discovery.
//!
//! Each [`BenchmarkId`](criterion::BenchmarkId) is parameterized by dirty
//! file count, so Criterion produces a single comparable line per request
//! preset across counts. The `1000` row is gated behind an opt-in env
//! variable so day-to-day runs stay fast on developer hardware while CI
//! and dedicated profiling can still exercise the high-N case.

use criterion::{BenchmarkId, Criterion, Throughput, black_box};
use sniff::filesystem::detect_git_with_request;
use sniff::request::GitRequest;

use crate::support::{fixtures, util};

/// Dirty-file counts exercised by the scaling benchmarks.
///
/// `100` is the default upper bound so a normal `cargo bench -p sniff` run
/// stays under a couple of minutes even on slower hardware. Set
/// `SNIFF_BENCH_DEEP_DIRTY=1` to also include the `1000` row.
const BASE_DIRTY_COUNTS: &[usize] = &[10, 100];
const HEAVY_DIRTY_COUNTS: &[usize] = &[10, 100, 1000];

fn dirty_counts() -> &'static [usize] {
    if std::env::var_os("SNIFF_BENCH_DEEP_DIRTY").is_some() {
        HEAVY_DIRTY_COUNTS
    } else {
        BASE_DIRTY_COUNTS
    }
}

pub fn register(c: &mut Criterion) {
    let mut group = util::configure_slow_group(c, "git_dirty_scaling");

    for &count in dirty_counts() {
        let fixture = fixtures::git_repo_with_dirty_files(count);

        let full_req = GitRequest::full();
        group.throughput(Throughput::Elements(count as u64));
        group.bench_with_input(
            BenchmarkId::new("git_full_with_file_stats", count),
            &count,
            |b, _| {
                b.iter(|| {
                    let info =
                        detect_git_with_request(black_box(fixture.path()), black_box(&full_req))
                            .unwrap();
                    black_box(info);
                });
            },
        );

        // `deep()` enables full unified diff emission, which is the
        // worst-case path through the batched diff aggregator.
        let deep_req = GitRequest::deep();
        group.bench_with_input(
            BenchmarkId::new("git_deep_with_unified_diffs", count),
            &count,
            |b, _| {
                b.iter(|| {
                    let info =
                        detect_git_with_request(black_box(fixture.path()), black_box(&deep_req))
                            .unwrap();
                    black_box(info);
                });
            },
        );
    }

    group.finish();

    // ---------- deep-git remote containment ----------
    let mut deep_group = util::configure_slow_group(c, "git_deep_remote");

    // Vary the number of fake remote-tracking branches to stress the
    // ancestry-walk optimization in `populate_recent_commit_remotes`.
    for &remote_count in &[1, 5, 10, 25] {
        let fixture = fixtures::git_repo_with_fake_remotes(20, remote_count);
        let deep_req = GitRequest::deep().commit_count(10);

        deep_group.throughput(Throughput::Elements(remote_count as u64));
        deep_group.bench_with_input(
            BenchmarkId::new("git_deep_remote_containment_check", remote_count),
            &remote_count,
            |b, _| {
                b.iter(|| {
                    let info =
                        detect_git_with_request(black_box(fixture.path()), black_box(&deep_req))
                            .unwrap();
                    black_box(info);
                });
            },
        );
    }

    deep_group.finish();
}
