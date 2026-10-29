//! Criterion timings for the `pipeline_work` scenarios
//! (`2026-10-25-pipeline-work-reduction`): anchors A1 and A2 and every
//! Darkmatter item case, with the system allocator. Counts and allocations
//! come from `pipeline_work_alloc`; this bench only times.
//!
//! A cold case empties the process caches before each iteration, outside
//! the measured work.

#[path = "pipeline_work_support/mod.rs"]
mod support;

use std::time::Duration;

use criterion::{BatchSize, Criterion, criterion_group, criterion_main};

fn bench_pipeline_work(c: &mut Criterion) {
    let mut group = c.benchmark_group("pipeline_work");
    group.sample_size(10);
    group.warm_up_time(Duration::from_millis(300));
    group.measurement_time(Duration::from_secs(2));
    for case in support::cases() {
        let name = case.name();
        // Criterion's own filter, applied before the (sometimes heavy) setup.
        if !criterion_selects(&name) {
            continue;
        }
        let cold = case.cold;
        let mut run = case.prepare();
        group.bench_function(name, |b| {
            if cold {
                // `PerIteration`: each reset runs just before its own run.
                b.iter_batched(support::reset_caches, |()| run(), BatchSize::PerIteration);
            } else {
                b.iter(&mut run);
            }
        });
    }
    group.finish();
}

/// Criterion options whose value is the next argument, so it is never
/// mistaken for the filter.
const VALUED_OPTIONS: [&str; 14] = [
    "--save-baseline",
    "--baseline",
    "--baseline-lenient",
    "--load-baseline",
    "--sample-size",
    "--warm-up-time",
    "--measurement-time",
    "--nresamples",
    "--noise-threshold",
    "--confidence-level",
    "--significance-level",
    "--profile-time",
    "--output-format",
    "--color",
];

/// Whether the command line's Criterion filter (its first positional
/// argument) could select `pipeline_work/<name>`.
fn criterion_selects(name: &str) -> bool {
    let mut arguments = std::env::args().skip(1);
    while let Some(argument) = arguments.next() {
        if VALUED_OPTIONS.contains(&argument.as_str()) {
            arguments.next();
        } else if !argument.starts_with('-') {
            return format!("pipeline_work/{name}").contains(&argument);
        }
    }
    true
}

criterion_group!(benches, bench_pipeline_work);
criterion_main!(benches);
