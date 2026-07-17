//! Criterion benchmark entry point for the `sniff` library.
//!
//! Bench groups are registered from the `cases` submodules so each
//! domain can be iterated independently while sharing fixture and
//! plan helpers from `support`.

use criterion::{Criterion, criterion_group, criterion_main};

mod support {
    pub mod bench_ids;
    pub mod builder;
    pub mod fixtures;
    pub mod network_fixture;
    pub mod plans;
    #[cfg(feature = "remote")]
    pub mod remote_report_fixture;
    pub mod util;
}

mod cases {
    pub mod filesystem;
    pub mod git;
    pub mod git_ops;
    pub mod hardware;
    pub mod inventory;
    pub mod network;
    pub mod programs;
    pub mod repo;
    pub mod system;
    pub mod workload_matrix;
}

fn register_all(c: &mut Criterion) {
    // Spin up the wiremock-backed WAN IP fixture before any plan-based
    // group runs; `full_plan()` now exercises the real WAN IP path and
    // depends on the env var set here.
    support::network_fixture::ensure_ready();

    cases::system::register(c);
    cases::hardware::register(c);
    cases::filesystem::register(c);
    cases::git::register(c);
    cases::git_ops::register(c);
    cases::inventory::register(c);
    cases::programs::register(c);
    cases::repo::register(c);
    cases::network::register(c);
    cases::workload_matrix::register(c);
}

criterion_group!(perf, register_all);
criterion_main!(perf);
