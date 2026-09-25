//! Repo package-boundary refresh Criterion benches.
//!
//! These benches isolate the cost of `refresh_package_boundaries` as the
//! number of Cargo packages in a monorepo grows. The fixtures are
//! intentionally simple (pure Cargo workspace) so the measured cost
//! reflects package-boundary assignment rather than multi-ecosystem
//! discovery overhead.
//!
//! Each [`BenchmarkId`](criterion::BenchmarkId) is parameterized by
//! package count, so Criterion produces a single comparable line across
//! counts.
//!
//! The `nested_marker_walk_corpus` group times the nested-marker fallback walk
//! against a caller-named checkout. It is manual evidence for
//! `2026-09-20-repo-perf`, not a portable timing gate: it registers only when
//! `SNIFF_BENCH_NESTED_CORPUS` names a directory and `bench-internals` is on.

use criterion::{BenchmarkId, Criterion, Throughput, black_box};
use sniff::filesystem::file_types::scan_file_inventory;
use sniff::filesystem::repo::detect_repo_structure;
use sniff::filesystem::repo::detection::refresh_package_boundaries;

use crate::support::{fixtures, util};

/// Package counts exercised by the scaling benchmarks.
///
/// `100` is the default upper bound so a normal `cargo bench -p sniff` run
/// stays under a couple of minutes even on slower hardware. Set
/// `SNIFF_BENCH_DEEP_REPO=1` to also include the `500` row.
const BASE_PKG_COUNTS: &[usize] = &[10, 100];
const HEAVY_PKG_COUNTS: &[usize] = &[10, 100, 500];

fn pkg_counts() -> &'static [usize] {
    if std::env::var_os("SNIFF_BENCH_DEEP_REPO").is_some() {
        HEAVY_PKG_COUNTS
    } else {
        BASE_PKG_COUNTS
    }
}

pub fn register(c: &mut Criterion) {
    let mut group = util::configure_slow_group(c, "repo_package_boundaries");

    for &count in pkg_counts() {
        let fixture = fixtures::cargo_monorepo(count);

        // Prepare structure and inventory outside the timed loop so
        // only `refresh_package_boundaries` is measured.
        let mut repo_info = detect_repo_structure(fixture.path()).unwrap().unwrap();
        let inventory = scan_file_inventory(fixture.path()).unwrap();
        let mut packages = repo_info.packages.take().unwrap();

        group.throughput(Throughput::Elements(count as u64));
        group.bench_with_input(
            BenchmarkId::new("package_boundary_refresh", count),
            &count,
            |b, _| {
                b.iter(|| {
                    refresh_package_boundaries(
                        black_box(&mut packages),
                        black_box(Some(&inventory)),
                    );
                    black_box(&packages);
                });
            },
        );
    }

    group.finish();

    #[cfg(feature = "bench-internals")]
    register_nested_marker_corpus(c);
}

#[cfg(feature = "bench-internals")]
fn register_nested_marker_corpus(c: &mut Criterion) {
    use criterion::SamplingMode;
    use sniff::filesystem::repo::nested_benchmark;
    use std::path::PathBuf;
    use std::time::Duration;

    let Some(corpus) = std::env::var_os("SNIFF_BENCH_NESTED_CORPUS").map(PathBuf::from) else {
        return;
    };
    // Recorded beside the timings so a sample can be tied to the tree it walked.
    let facts = nested_benchmark::corpus(&corpus);
    let candidates = nested_benchmark::serial_reference_walk(&corpus).len();
    eprintln!(
        "nested_marker_walk_corpus: root={} walked_entries={} marker_entries={} candidates={candidates}",
        corpus.display(),
        facts.walked_entries,
        facts.marker_entries,
    );

    let mut group = c.benchmark_group("nested_marker_walk_corpus");
    // Flat sampling keeps every one of the 20 samples at the same iteration
    // count, so `sample.json` holds directly comparable per-sample times.
    group
        .sampling_mode(SamplingMode::Flat)
        .sample_size(20)
        .warm_up_time(Duration::from_secs(3))
        .measurement_time(Duration::from_secs(10));

    group.bench_function("serial_reference", |b| {
        b.iter(|| black_box(nested_benchmark::serial_reference_walk(black_box(&corpus))));
    });
    group.bench_function("production", |b| {
        b.iter(|| black_box(nested_benchmark::production_walk(black_box(&corpus))));
    });
    group.bench_function("detect_repo_structure", |b| {
        b.iter(|| black_box(detect_repo_structure(black_box(&corpus)).unwrap()));
    });
    group.finish();
}
