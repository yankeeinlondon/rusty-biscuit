//! Warm-cache performance SLA guard for `wt list`.
//!
//! Builds the shared mixed multi-worktree fixture (several divergent branches
//! plus fast-forward and behind-only ones), primes the SHA-pair cache, then
//! measures the `list gather` stage from `wt list --perf`. The warm cache must
//! collapse `list gather` below the live recompute cost — asserting on the
//! stage (not just full-command wall-clock) is what makes this a readiness gate
//! for the spec's target metric rather than a smoke test. The ratified bound is
//! documented in `worktree/docs/performance-testing.md` under
//! "Ratified SLA Targets".

mod perf_support;

use std::time::Duration;

use perf_support::MixedFixture;
use serial_test::serial;

/// Best-of-5 warm `list gather` must stay under this bound. The dominant warm
/// cost is the live dirty walk (one `git status` per worktree); the cached
/// `rev-list` + `merge-tree` results are skipped entirely.
const WARM_LIST_GATHER_BOUND: Duration = Duration::from_millis(120);

#[test]
#[serial]
fn perf_cache_warm_list_gather_meets_sla() {
    let fixture = MixedFixture::new();
    fixture.warm_untracked_cache();

    // One cold run populates the SHA-pair cache for every divergent branch.
    fixture.clear_worktree_cache();
    let cold = fixture.list_gather_duration();

    let warm = (0..5)
        .map(|_| fixture.list_gather_duration())
        .min()
        .expect("at least one timed run");

    eprintln!("warm `list gather` (best of 5): {warm:.2?}; cold reference: {cold:.2?}");
    assert!(
        warm < WARM_LIST_GATHER_BOUND,
        "warm `list gather` best-of-5 was {warm:.2?}, exceeding the {WARM_LIST_GATHER_BOUND:.2?} SLA"
    );
    // Collapse check: prove the cache actually helps on this workload rather
    // than the host merely being fast. If caching regressed, the warm run would
    // recompute every branch and this would fail.
    assert!(
        warm < cold,
        "warm `list gather` ({warm:.2?}) should be below the cold reference ({cold:.2?}); cache did not collapse the divergent-branch recompute"
    );
}
