//! Programs + services inventory Criterion benches.
//!
//! The programs path fans out across 8 categories using Rayon and a
//! shared `ExecutableIndex`; the services path walks whichever init
//! system the host exposes. Both are benchmarked end-to-end so the
//! design's parallelism target has a before/after signal.

use criterion::{Criterion, black_box};
use sniff::ProgramsInfo;
use sniff::services::detect_services;

use crate::support::util;

pub fn register(c: &mut Criterion) {
    let mut group = util::configure_slow_group(c, "inventory");

    group.bench_function("programs_detect", |b| {
        b.iter(|| {
            let programs = ProgramsInfo::detect();
            black_box(programs);
        });
    });

    group.bench_function("services_detect", |b| {
        b.iter(|| {
            let services = detect_services();
            black_box(services);
        });
    });

    group.finish();
}
