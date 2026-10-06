//! Criterion benchmark for the shared local half of `wt list`:
//! `worktree::list::gather` with no worker, no graph, no verbose details, no
//! `--ff`, and no `--ignore-api` (so no API preference write).
//!
//! It measures the parse step, dirtiness, the branch comparisons, and the
//! commit, plus a read of the stored remote answers. It is not a
//! full-command benchmark: it launches no refresh worker, waits for nothing,
//! makes no network request, and renders nothing.
//!
//! The benchmark runs against the ambient `rusty-biscuit` checkout. Baseline
//! numbers drift as the repository grows and as worktree metadata changes.
//! The shared `_bench_preflight` gating and host-derived `_bench_id` baseline
//! name mitigate cross-host comparison noise; always compare against a baseline
//! captured on the same machine and in a similar host state.

use criterion::{BatchSize, Criterion, Throughput, criterion_group, criterion_main};
use std::hint::black_box;
use worktree::list::{ListOptions, gather};

/// One listing of the ambient repository, as described in the module docs.
///
/// A single listing is made outside the measured loop to verify the ambient
/// directory is a usable git worktree. If it fails (for example, the
/// benchmark is run from a bare directory or the linked worktree metadata
/// references paths outside the current working directory), the entire group
/// is skipped.
fn bench_list_status(c: &mut Criterion) {
    let Ok(repo) = std::env::current_dir() else {
        return;
    };
    let options = ListOptions::default();
    if gather(&repo, &options, &mut |_| {}).is_err() {
        return;
    }

    let mut group = c.benchmark_group("list_status");
    group.throughput(Throughput::Elements(1));
    group.bench_function("warm", |b| {
        b.iter_batched_ref(
            || (),
            |_| black_box(gather(&repo, &options, &mut |_| {}).expect("gather failed")),
            BatchSize::SmallInput,
        )
    });
    group.finish();
}

criterion_group!(benches, bench_list_status);
criterion_main!(benches);
