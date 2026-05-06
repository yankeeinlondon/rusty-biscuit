//! Repo package-boundary refresh Criterion benches.
//!
//! These benches isolate the cost of `refresh_package_boundaries` as the
//! number of Cargo packages in a monorepo grows. The fixtures are
//! intentionally simple (pure Cargo workspace) so the measured cost
//! reflects package-boundary assignment rather than multi-ecosystem
//! discovery overhead.
//!
//! Each [`BenchmarkId`](criterion::BenchmarkId) is parameterized by
//! package count so Criterion produces a single comparable line across
//! counts.

use criterion::{BenchmarkId, Criterion, black_box};
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

        group.bench_with_input(
            BenchmarkId::new("refresh_boundaries", count),
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
}
