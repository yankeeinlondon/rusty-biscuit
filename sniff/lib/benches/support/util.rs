//! Shared Criterion configuration helpers.

use criterion::{Criterion, measurement::WallTime};
use std::time::Duration;

/// Configure a Criterion group for a moderately expensive benchmark.
///
/// Detection functions in `sniff` typically run in the 10ms–1s range,
/// so we trade a smaller sample size for shorter wall-clock runtimes
/// without losing statistical usefulness.
pub fn configure_group<'a>(
    c: &'a mut Criterion,
    name: &str,
) -> criterion::BenchmarkGroup<'a, WallTime> {
    let mut group = c.benchmark_group(name);
    group
        .sample_size(20)
        .warm_up_time(Duration::from_secs(2))
        .measurement_time(Duration::from_secs(10));
    group
}

/// Configure a group for very slow benchmarks (full detection, large monorepos).
pub fn configure_slow_group<'a>(
    c: &'a mut Criterion,
    name: &str,
) -> criterion::BenchmarkGroup<'a, WallTime> {
    let mut group = c.benchmark_group(name);
    group
        .sample_size(10)
        .warm_up_time(Duration::from_secs(2))
        .measurement_time(Duration::from_secs(15));
    group
}
