//! Cold-cache performance SLA guard for `wt list`.
//!
//! The SHA-pair cache is deleted before each timed run, so every run is a full
//! cache miss: one `rev-list` plus one speculative `merge-tree` per non-main
//! branch. The fixture is deliberately mixed (divergent, fast-forward, and
//! behind-only branches) so this bounds the cold-path tradeoff the cache cannot
//! help — in particular the speculative `merge-tree` now issued for
//! fast-forward and behind-only branches. Measuring the `list gather` stage
//! (not just full-command wall-clock) makes this a readiness gate for the
//! spec's target metric. The ratified bound is documented in
//! `worktree/docs/performance-testing.md` under "Ratified SLA Targets".

mod perf_support;

use std::time::Duration;

use perf_support::MixedFixture;
use serial_test::serial;

/// Best-of-5 cold `list gather` must stay under this bound on the mixed
/// fixture. This is the speculative-`merge-tree` budget: every non-main branch
/// runs `rev-list` + `merge-tree` concurrently, including the fast-forward and
/// behind-only branches that previously skipped `merge-tree`.
const COLD_LIST_GATHER_BOUND: Duration = Duration::from_millis(300);

#[test]
#[serial]
fn perf_cache_cold_list_gather_meets_sla() {
    let fixture = MixedFixture::new();
    fixture.warm_untracked_cache();

    let cold = (0..5)
        .map(|_| {
            fixture.clear_worktree_cache();
            fixture.list_gather_duration()
        })
        .min()
        .expect("at least one timed run");

    eprintln!("cold `list gather` (best of 5): {cold:.2?}");
    assert!(
        cold < COLD_LIST_GATHER_BOUND,
        "cold `list gather` best-of-5 was {cold:.2?}, exceeding the {COLD_LIST_GATHER_BOUND:.2?} SLA"
    );
}
