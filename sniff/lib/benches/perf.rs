//! Criterion benchmark entry point for the `sniff` library.
//!
//! Bench groups are registered from the `cases` submodules so each
//! domain can be iterated independently while sharing fixture and
//! plan helpers from `support`.

use criterion::{Criterion, criterion_group, criterion_main};

mod support {
    pub mod fixtures;
    pub mod plans;
    pub mod util;
}

mod cases {
    pub mod filesystem;
    pub mod hardware;
    pub mod inventory;
    pub mod system;
}

fn register_all(c: &mut Criterion) {
    cases::system::register(c);
    cases::hardware::register(c);
    cases::filesystem::register(c);
    cases::inventory::register(c);
}

criterion_group!(perf, register_all);
criterion_main!(perf);
