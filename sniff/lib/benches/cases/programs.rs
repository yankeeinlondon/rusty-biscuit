//! Program detection Criterion benches.
//!
//! Phase 4 baseline benchmarks for program lookup behavior. These exist
//! alongside the broader `inventory::programs_detect` bench (which
//! exercises the full 8-category fan-out) and zoom in on:
//!
//! - the eager vs. lazy `ExecutableIndex` build paths (the optimization
//!   restored in Phase 3.3)
//! - bulk lookup speed when many program names are resolved against a
//!   shared index (`ProgramsInfo::detect`'s hot path)
//! - the upper-bound fan-out cost via `ProgramsInfo::detect` itself,
//!   re-registered here under `programs/` so the high-level identifier
//!   sits next to the lower-level building blocks
//!
//! PATH-length scaling is intentionally measured by `hyperfine` against
//! the compiled CLI (see `sniff/lib/README.md` for the documented
//! invocation): mutating `PATH` from inside Criterion's test harness
//! would race with other benches that read environment state.

use criterion::{Criterion, black_box};
use sniff::ProgramsInfo;
use sniff::programs::find_program::{ExecutableIndex, find_programs_with_source_from_index};

use crate::support::util;

/// Representative bulk-lookup workload — a mix of programs that almost
/// always exist on developer machines and a few that may not. The mix
/// keeps the eager and lazy paths comparable: the eager index resolves
/// every name from its in-memory map, while the lazy path falls back to
/// `which` on each miss.
#[cfg(unix)]
const BULK_LOOKUPS: &[&str] = &[
    "ls", "cat", "sh", "env", "git", "cargo", "rustc", "node", "python3", "make", "uv", "go",
    "ruby", "perl", "awk", "sed", "grep", "find", "tar", "gzip", "curl", "wget", "ssh", "rsync",
];

#[cfg(windows)]
const BULK_LOOKUPS: &[&str] = &[
    "cmd",
    "powershell",
    "explorer",
    "git",
    "cargo",
    "rustc",
    "node",
    "python",
    "where",
    "ping",
];

pub fn register(c: &mut Criterion) {
    // ---------- ExecutableIndex build modes ----------
    let mut build_group = util::configure_group(c, "programs");

    build_group.bench_function("index_build_lazy", |b| {
        b.iter(|| {
            let index = ExecutableIndex::build();
            black_box(index);
        });
    });

    build_group.bench_function("index_build_eager_path", |b| {
        b.iter(|| {
            let index = ExecutableIndex::build_eager_path();
            black_box(index);
        });
    });

    build_group.finish();

    // ---------- bulk lookup against pre-built indexes ----------
    let mut lookup_group = util::configure_group(c, "programs_bulk_lookup");

    let lazy_index = ExecutableIndex::build();
    lookup_group.bench_function("lookup_lazy", |b| {
        b.iter(|| {
            let results = find_programs_with_source_from_index(
                black_box(&lazy_index),
                black_box(BULK_LOOKUPS),
            );
            black_box(results);
        });
    });

    let eager_index = ExecutableIndex::build_eager_path();
    lookup_group.bench_function("lookup_eager_path", |b| {
        b.iter(|| {
            let results = find_programs_with_source_from_index(
                black_box(&eager_index),
                black_box(BULK_LOOKUPS),
            );
            black_box(results);
        });
    });

    lookup_group.finish();

    // ---------- end-to-end fan-out ----------
    let mut fanout_group = util::configure_slow_group(c, "programs_fanout");

    fanout_group.bench_function("programs_info_detect", |b| {
        b.iter(|| {
            let programs = ProgramsInfo::detect();
            black_box(programs);
        });
    });

    fanout_group.finish();
}
