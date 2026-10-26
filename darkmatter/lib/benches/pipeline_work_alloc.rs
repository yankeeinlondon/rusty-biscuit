//! Work counts, allocations, and wall time for every `pipeline_work`
//! scenario (`2026-10-25-pipeline-work-reduction`, phase 1).
//!
//! Runs each [`support::Case`] once for its counts and allocations, then a
//! few more times for a median wall time, and prints one tab-separated row
//! per case:
//!
//! ```text
//! case  runs  work  allocations  bytes_allocated  peak_live_bytes  median_ns
//! ```
//!
//! `work` lists every nonzero [`Work`] counter as `Kind=count`, comma
//! separated. Counts, allocations, and bytes are deterministic; the wall time
//! includes the counting overhead, so compare timings from `pipeline_work`
//! (the Criterion bench, system allocator) instead.
//!
//! Arguments that do not start with `-` filter cases by substring:
//! `cargo bench -p darkmatter --features work-counters --bench pipeline_work_alloc -- D5`.

#[path = "pipeline_work_support/mod.rs"]
mod support;

use std::time::Instant;

use darkmatter::work_counters::{self, Work};
use test_toolkit::alloc_count::{self, CountingAllocator};

#[global_allocator]
static ALLOCATOR: CountingAllocator = CountingAllocator;

/// Timed runs after the counted one; the median is reported.
const TIMED_RUNS: usize = 5;

fn main() {
    let filters: Vec<String> = std::env::args().skip(1).filter(|argument| !argument.starts_with('-')).collect();
    println!("case\truns\twork\tallocations\tbytes_allocated\tpeak_live_bytes\tmedian_ns");
    for case in support::cases() {
        let name = case.name();
        if !filters.is_empty() && !filters.iter().any(|filter| name.contains(filter.as_str())) {
            continue;
        }
        let cold = case.cold;
        let mut run = case.prepare();

        if cold {
            support::reset_caches();
        } else {
            // A warm case is counted after one run has filled the caches.
            run();
        }
        let work_before = work_counters::snapshot();
        alloc_count::reset_peak();
        let alloc_before = alloc_count::snapshot();
        run();
        let allocated = alloc_count::snapshot().since(alloc_before);
        let work = work_counters::snapshot().since(&work_before);

        let mut nanos: Vec<u128> = (0..TIMED_RUNS)
            .map(|_| {
                if cold {
                    support::reset_caches();
                }
                let start = Instant::now();
                run();
                start.elapsed().as_nanos()
            })
            .collect();
        nanos.sort_unstable();

        let work: Vec<String> = work.nonzero().into_iter().map(|(kind, count): (Work, u64)| format!("{kind:?}={count}")).collect();
        println!(
            "{name}\t{}\t{}\t{}\t{}\t{}\t{}",
            TIMED_RUNS + 1,
            if work.is_empty() { "-".to_string() } else { work.join(",") },
            allocated.allocations,
            allocated.bytes_allocated,
            allocated.peak_live_bytes,
            nanos[TIMED_RUNS / 2],
        );
    }
}
