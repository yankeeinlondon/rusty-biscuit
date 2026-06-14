//! Criterion benchmarks for `worktree::list_worktrees`.
//!
//! The benchmark runs against the ambient `rusty-biscuit` checkout. Baseline
//! numbers drift as the repository grows and as worktree metadata changes.
//! The shared `_bench_preflight` gating and host-derived `_bench_id` baseline
//! name mitigate cross-host comparison noise; always compare against a baseline
//! captured on the same machine and in a similar host state.

use criterion::{BatchSize, Criterion, Throughput, criterion_group, criterion_main};
use std::hint::black_box;
use worktree::worktree::list_worktrees;

/// Benchmark the library-owned gather surface.
///
/// A single `list_worktrees()` call is made outside the measured loop to
/// verify the ambient directory is a usable git worktree. If that call fails
/// (for example, the benchmark is run from a bare directory or the linked
/// worktree metadata references paths outside the current working directory),
/// the entire group is skipped.
fn bench_list_status(c: &mut Criterion) {
    if list_worktrees().is_err() {
        return;
    }

    let mut group = c.benchmark_group("list_status");
    group.throughput(Throughput::Elements(1));
    group.bench_function("warm", |b| {
        b.iter_batched_ref(
            || (),
            |_| black_box(list_worktrees().expect("list_worktrees failed")),
            BatchSize::SmallInput,
        )
    });
    group.finish();
}

criterion_group!(benches, bench_list_status);
criterion_main!(benches);
