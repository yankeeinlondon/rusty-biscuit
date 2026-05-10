//! Programs + services inventory Criterion benches.
//!
//! The programs path fans out across 8 categories using Rayon and a
//! shared `ExecutableIndex`; the services path walks whichever init
//! system the host exposes. Both are benchmarked end-to-end so the
//! design's parallelism target has a before/after signal.

use criterion::{Criterion, black_box};
use sniff::ProgramsInfo;
use sniff::executable_index::ExecutableIndex;
use sniff::services::detect_services;

use crate::support::util;

pub fn register(c: &mut Criterion) {
    let mut group = util::configure_slow_group(c, "inventory");

    group.bench_function("programs_detect_all_8_categories", |b| {
        b.iter(|| {
            let programs = ProgramsInfo::detect();
            black_box(programs);
        });
    });

    group.bench_function("services_detect_init_system", |b| {
        b.iter(|| {
            let services = detect_services();
            black_box(services);
        });
    });

    group.finish();

    // ExecutableIndex benchmarks — separate fast group
    let mut idx_group = util::configure_group(c, "executable_index");

    idx_group.bench_function("executable_index_build_lazy", |b| {
        b.iter(|| {
            let index = ExecutableIndex::build();
            black_box(index);
        });
    });

    // Benchmark lookup of common system binaries
    #[cfg(unix)]
    let lookups = &["ls", "cat", "sh", "env", "git"];
    #[cfg(windows)]
    let lookups = &["cmd", "powershell", "explorer"];

    let index = ExecutableIndex::build();
    idx_group.bench_function("executable_index_lookup_5_names", |b| {
        b.iter(|| {
            for prog in lookups {
                let result = index.find_with_source(prog);
                black_box(result);
            }
        });
    });

    idx_group.finish();
}
