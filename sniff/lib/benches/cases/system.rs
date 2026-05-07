//! System-level Criterion benches.
//!
//! Answers the design's top-level question: what is the total cost of
//! each detection tier when driven through `detect_with_plan`?

use criterion::{Criterion, black_box};
use sniff::detect_with_plan;

use crate::support::{fixtures, plans, util};

pub fn register(c: &mut Criterion) {
    let fixture = fixtures::small_git_repo();

    let mut group = util::configure_group(c, "system");

    group.bench_function("minimal_plan_os_hardware_only", |b| {
        b.iter(|| {
            let plan = plans::minimal_plan();
            detect_with_plan(black_box(plan)).unwrap();
        });
    });

    group.bench_function("summary_plan_with_git_repo", |b| {
        b.iter(|| {
            let plan = plans::summary_plan(fixture.path().to_path_buf());
            detect_with_plan(black_box(plan)).unwrap();
        });
    });

    group.finish();

    let mut slow_group = util::configure_slow_group(c, "system_full");
    slow_group.bench_function("full_plan_monorepo_all_domains", |b| {
        b.iter(|| {
            let plan = plans::full_plan(fixture.path().to_path_buf());
            detect_with_plan(black_box(plan)).unwrap();
        });
    });
    slow_group.finish();
}
