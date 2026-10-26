//! Crate-private scenarios of the `2026-10-25-pipeline-work-reduction`
//! measurement spike.
//!
//! The public scenarios live in `benches/pipeline_work_alloc.rs`. The ones
//! here reach code a bench target cannot see, so they run as ignored unit
//! tests of this crate, whose test binary declares the counting allocator
//! (`lib.rs`). Counters and the allocator are process-wide, so run them on one
//! thread:
//!
//! ```text
//! cargo test -p claudine --lib --release --features work-counters -- \
//!     --ignored --nocapture --test-threads=1 pipeline_spike
//! ```
//!
//! Each scenario prints `SPIKE` rows in the bench target's format.

use std::hint::black_box;
use std::time::{Duration, Instant};

use test_toolkit::alloc_count;

/// One scenario's cost: the median wall time over the runs, and the
/// allocation and counter deltas of the first (cold) and last (warm) run.
pub(crate) struct Measured {
    pub median: Duration,
    pub cold: alloc_count::AllocationSnapshot,
    pub warm: alloc_count::AllocationSnapshot,
    pub claudine: crate::work_counters::WorkSnapshot,
    pub darkmatter: darkmatter::work_counters::WorkSnapshot,
}

/// Run `scenario` `runs` times and measure each run.
pub(crate) fn measure<T>(runs: usize, mut scenario: impl FnMut() -> T) -> Measured {
    let mut times = Vec::with_capacity(runs);
    let mut cold = None;
    let mut warm = alloc_count::AllocationSnapshot::default();
    let mut claudine = crate::work_counters::WorkSnapshot::default();
    let mut darkmatter = darkmatter::work_counters::WorkSnapshot::default();
    for _ in 0..runs.max(1) {
        alloc_count::reset_peak();
        let allocations = alloc_count::snapshot();
        let claudine_before = crate::work_counters::snapshot();
        let darkmatter_before = darkmatter::work_counters::snapshot();
        let started = Instant::now();
        let output = scenario();
        let elapsed = started.elapsed();
        warm = alloc_count::snapshot().since(allocations);
        claudine = crate::work_counters::snapshot().since(&claudine_before);
        darkmatter = darkmatter::work_counters::snapshot().since(&darkmatter_before);
        black_box(output);
        cold.get_or_insert(warm);
        times.push(elapsed);
    }
    times.sort();
    Measured {
        median: times[times.len() / 2],
        cold: cold.unwrap_or_default(),
        warm,
        claudine,
        darkmatter,
    }
}

/// Print one machine-readable row: `SPIKE item=… size=… time_ns=… …`.
pub(crate) fn report(item: &str, size: &str, measured: &Measured) {
    let mut work: Vec<String> = measured
        .claudine
        .nonzero()
        .into_iter()
        .map(|(work, count)| format!("{work:?}:{count}"))
        .collect();
    work.extend(
        measured
            .darkmatter
            .nonzero()
            .into_iter()
            .map(|(work, count)| format!("dm.{work:?}:{count}")),
    );
    println!(
        "SPIKE item={item} size={size} time_ns={} allocs={} bytes={} peak={} cold_allocs={} cold_bytes={} cold_peak={} work={}",
        measured.median.as_nanos(),
        measured.warm.allocations,
        measured.warm.bytes_allocated,
        measured.warm.peak_live_bytes,
        measured.cold.allocations,
        measured.cold.bytes_allocated,
        measured.cold.peak_live_bytes,
        if work.is_empty() {
            "-".to_string()
        } else {
            work.join(",")
        },
    );
}

/// A string of `bytes` ASCII characters.
pub(crate) fn text_of(bytes: usize) -> String {
    "x".repeat(bytes)
}

/// A frontmatter-like mapping of about `bytes` bytes: string properties of
/// 1 KB each.
pub(crate) fn frontmatter_of(bytes: usize) -> serde_json::Map<String, serde_json::Value> {
    (0..bytes.div_ceil(1024))
        .map(|index| {
            (
                format!("field_{index}"),
                serde_json::Value::String(text_of(1024)),
            )
        })
        .collect()
}

// The scenarios live in a test path so the source-scan guards treat them as
// test code.
mod tests;
