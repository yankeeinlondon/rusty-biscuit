//! Full-command performance SLA test for `wt list`.
//!
//! Spawns the built `wt` binary and asserts the repo-wide 1-second SLA (see
//! `sniff/fixes/_completed/2026-04-21-performance/spec.md`) on the non-image
//! fast path. Rasterization never runs on the non-image path, so it is excluded
//! by construction. This complements the in-process `perf_subprocess_counts_meet_sla`
//! unit test, which covers the worktree-owned data-gather pieces.
//!
//! Cache-path SLA coverage lives in the sibling `cache_warm_path.rs` and
//! `cache_cold_path.rs` integration tests. Their ratified targets are documented
//! in `worktree/docs/performance-testing.md` under "Ratified SLA Targets".

mod perf_support;

use std::process::Stdio;
use std::time::{Duration, Instant};

use serial_test::serial;

use perf_support::MixedFixture;

/// Full non-image `wt list` must meet the repo-wide 1-second SLA on a warm
/// cache.
///
/// A warm-up run primes the untracked/object caches, then five timed runs
/// measure steady-state behavior. The SLA is checked against the best (minimum)
/// run so transient load spikes from parallel test execution do not produce
/// false failures.
///
/// The SLA bound is 1 second: a regression past the spec'd SLA fails this gate.
/// The best-of-5 minimum (rather than the mean) tolerates parallel-test
/// contention, since a transient load spike may slow individual runs without
/// indicating steady-state behavior. For a contention-free measurement, run the
/// perf tests serially via `just test-perf`. Run with `--nocapture` to see each
/// measured timing.
#[test]
#[serial]
fn perf_full_command_non_image_meets_sla() {
    let fixture = MixedFixture::new();
    fixture.warm_untracked_cache();

    let run = || {
        fixture
            .wt_command()
            .env_remove("TERM_PROGRAM")
            .env_remove("KITTY_WINDOW_ID")
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .expect("wt list should run")
            .success()
    };

    assert!(run(), "wt list should succeed (warm-up)");

    let best = (0..5)
        .map(|_| {
            let t0 = Instant::now();
            assert!(run(), "wt list should succeed");
            t0.elapsed()
        })
        .min()
        .expect("at least one timed run");

    eprintln!("full non-image `wt list` (best of 5): {best:.2?}");
    assert!(
        best < Duration::from_millis(1000),
        "non-image `wt list` best-of-5 took {best:.2?}, exceeding the 1-second SLA"
    );
}
