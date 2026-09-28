//! Full-command performance SLA test for `wt list`.
//!
//! Spawns the built `wt` binary and asserts the 1-second bound on "local
//! gather plus render" (see `worktree/docs/performance-testing.md`) on the
//! non-image fast path, with a worker that answers at once. Rasterization never
//! runs on the non-image path, so it is excluded by construction. This
//! complements the in-process `perf_subprocess_counts_meet_sla` unit test,
//! which covers the worktree-owned data-gather pieces. The bound with a
//! stalled worker (3 s plus this one) is `perf_pr_request.rs`'s.
//!
//! Cache-path SLA coverage lives in the sibling `cache_warm_path.rs` and
//! `cache_cold_path.rs` integration tests. Their ratified targets are documented
//! in `worktree/docs/performance-testing.md` under "Ratified SLA Targets".

mod perf_support;

use std::time::{Duration, Instant};

use serial_test::serial;

use perf_support::MixedFixture;

/// Full non-image `wt list` must meet the 1-second SLA on a warm cache when
/// its worker answers at once: a local bare `origin` in sync with the
/// tracking ref, so every run launches the worker, waits for its check, and
/// fetches nothing.
///
/// A warm-up run primes the untracked/object caches, then five timed runs
/// measure steady-state behavior. The SLA is checked against the best (minimum)
/// run so transient load spikes from parallel test execution do not produce
/// false failures. Every run must report the check it waited for, so a run
/// that skipped the worker cannot pass.
///
/// For a contention-free measurement, run the perf tests serially via
/// `just test-perf`. Run with `--nocapture` to see each measured timing.
#[test]
#[serial]
fn perf_full_command_non_image_meets_sla() {
    let fixture = MixedFixture::new().with_local_origin();
    fixture.warm_untracked_cache();

    let run = || {
        let output = fixture
            .wt_command_direct()
            .arg("list")
            .env("NO_COLOR", "1")
            .output()
            .expect("wt list should run");
        let stderr = String::from_utf8_lossy(&output.stderr).split_whitespace().collect::<Vec<_>>().join(" ");
        assert!(output.status.success(), "wt list should succeed:\n{stderr}");
        assert!(
            stderr.contains("main is in sync with origin/main") && !stderr.contains("main is in sync with origin/main ("),
            "the listing waited for its worker's check:\n{stderr}"
        );
    };

    run();

    let best = (0..5)
        .map(|_| {
            let t0 = Instant::now();
            run();
            t0.elapsed()
        })
        .min()
        .expect("at least one timed run");
    assert!(
        fixture.wait_until_unlocked(Duration::from_secs(20), || ()),
        "every worker exited and released its locks"
    );

    eprintln!("full non-image `wt list`, worker answering at once (best of 5): {best:.2?}");
    assert!(
        best < Duration::from_millis(1000),
        "non-image `wt list` best-of-5 took {best:.2?}, exceeding the 1-second SLA"
    );
}
