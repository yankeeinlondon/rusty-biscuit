//! `wt list` performance with the network down, with a PR request that hits
//! its deadline, and with a stale stored answer whose refresh is blocked,
//! against the ratified targets in `worktree/docs/performance-testing.md`
//! (warm `list gather` 120 ms, cold 300 ms, full non-image `wt list` 1 s).
//! Every request goes to a local proxy stub, so nothing leaves the host.

mod perf_support;

use std::process::Stdio;
use std::time::{Duration, Instant};

use perf_support::{MixedFixture, ProxyStub, RemoveOnDrop, list_gather_from_perf, stage_from_perf};
use serial_test::serial;
use worktree::pull_requests::{CachedPrs, origin_url, select_cached, unix_now};

const WARM_LIST_GATHER_BOUND: Duration = Duration::from_millis(120);
const COLD_LIST_GATHER_BOUND: Duration = Duration::from_millis(300);
const FULL_COMMAND_BOUND: Duration = Duration::from_millis(1000);
const PR_DEADLINE: Duration = Duration::from_millis(300);

/// `(list gather, pr gather)` from one `wt list --perf`.
fn stages(fixture: &MixedFixture, proxy: &ProxyStub) -> (Duration, Duration) {
    let output = fixture
        .wt_command_via(proxy)
        .args(["list", "--perf"])
        .output()
        .expect("wt list --perf should run");
    assert!(output.status.success(), "wt list --perf failed");
    let stderr = String::from_utf8_lossy(&output.stderr);
    (
        list_gather_from_perf(&stderr).expect("list gather stage"),
        stage_from_perf(&stderr, "pr gather").expect("pr gather stage"),
    )
}

fn best_full_command(fixture: &MixedFixture, proxy: &ProxyStub) -> Duration {
    (0..5)
        .map(|_| {
            let t0 = Instant::now();
            let status = fixture
                .wt_command_via(proxy)
                .arg("list")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .expect("wt list should run");
            assert!(status.success());
            t0.elapsed()
        })
        .min()
        .expect("at least one timed run")
}

#[test]
#[serial]
fn perf_list_meets_sla_with_the_network_down() {
    let fixture = MixedFixture::new().with_github_origin();
    fixture.warm_untracked_cache();
    let proxy = ProxyStub::refusing();

    let cold = (0..5)
        .map(|_| {
            fixture.clear_worktree_cache();
            stages(&fixture, &proxy).0
        })
        .min()
        .unwrap();
    let warm = (0..5).map(|_| stages(&fixture, &proxy).0).min().unwrap();
    let full = best_full_command(&fixture, &proxy);

    eprintln!("network down: cold list gather {cold:.2?}, warm {warm:.2?}, full {full:.2?}");
    assert!(cold < COLD_LIST_GATHER_BOUND, "cold list gather {cold:.2?}");
    assert!(warm < WARM_LIST_GATHER_BOUND, "warm list gather {warm:.2?}");
    assert!(full < FULL_COMMAND_BOUND, "full wt list {full:.2?}");
}

#[test]
#[serial]
fn perf_list_meets_sla_when_the_pr_request_hits_its_deadline() {
    let fixture = MixedFixture::new().with_github_origin();
    fixture.warm_untracked_cache();
    let proxy = ProxyStub::hanging();

    let runs: Vec<(Duration, Duration)> = (0..5).map(|_| stages(&fixture, &proxy)).collect();
    let warm = runs.iter().map(|(list, _)| *list).min().unwrap();
    let full = best_full_command(&fixture, &proxy);

    eprintln!("stalled PR request: warm list gather {warm:.2?}, full {full:.2?}, pr gather {:?}", runs.iter().map(|r| r.1).collect::<Vec<_>>());
    assert!(proxy.connections() >= runs.len(), "every run made the request");
    for (_, pr) in &runs {
        assert!(*pr >= PR_DEADLINE, "the request should have waited for its deadline, got {pr:?}");
    }
    assert!(warm < WARM_LIST_GATHER_BOUND, "warm list gather {warm:.2?}");
    assert!(full < FULL_COMMAND_BOUND, "full wt list {full:.2?}");
}

/// Best of five full `wt list` runs, reseeding the store `age` old before
/// each so a worker's success can never change what a sample measures.
fn best_full_command_with_store(fixture: &MixedFixture, proxy: &ProxyStub, age: Duration) -> Duration {
    (0..5)
        .map(|_| {
            fixture.seed_pr_store(age, 99, "divergent-0");
            let t0 = Instant::now();
            let status = fixture
                .wt_command_via(proxy)
                .arg("list")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .expect("wt list should run");
            assert!(status.success());
            t0.elapsed()
        })
        .min()
        .expect("at least one timed run")
}

/// `pr gather` from one `wt list --perf` with the store reseeded `age` old,
/// and its rendered output.
fn pr_gather_with_store(fixture: &MixedFixture, proxy: &ProxyStub, age: Duration) -> (Duration, String) {
    fixture.seed_pr_store(age, 99, "divergent-0");
    let output = fixture
        .wt_command_via(proxy)
        .args(["list", "--perf"])
        .env("NO_COLOR", "1")
        .output()
        .expect("wt list --perf should run");
    assert!(output.status.success(), "wt list --perf failed");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (stage_from_perf(&stderr, "pr gather").expect("pr gather stage"), stderr)
}

/// A stale matching answer renders at once while its refresh is blocked on a
/// hanging request. Timing alone cannot show that nothing waits (1 s would
/// hide a 300 ms wait), so `pr gather` must also stay under the request
/// deadline; `list_prs::a_detached_workers_answer_replaces_the_stale_one_on_the_next_list`
/// proves deterministically that the parent never joins its worker.
#[test]
#[serial]
fn perf_list_meets_sla_with_a_stale_answer_and_a_blocked_refresh() {
    const STALE: Duration = Duration::from_secs(12 * 60 + 5);
    let fixture = MixedFixture::new().with_github_origin();
    let _cleanup = RemoveOnDrop(fixture.pr_store());
    fixture.warm_untracked_cache();
    let proxy = ProxyStub::hanging();
    let origin = origin_url(fixture.main()).expect("origin");

    // Warm the comparison cache with a fresh answer, which starts no worker.
    let _ = pr_gather_with_store(&fixture, &proxy, Duration::ZERO);
    let fresh_full = best_full_command_with_store(&fixture, &proxy, Duration::ZERO);
    let fresh_pr = (0..5).map(|_| pr_gather_with_store(&fixture, &proxy, Duration::ZERO).0).min().unwrap();
    assert_eq!(proxy.connections(), 0, "a fresh answer makes no request");

    let stale_full = best_full_command_with_store(&fixture, &proxy, STALE);
    let mut stale_pr = Vec::new();
    for _ in 0..5 {
        let (pr, stderr) = pr_gather_with_store(&fixture, &proxy, STALE);
        assert!(stderr.contains("PR #99"), "the stale badge is shown:\n{stderr}");
        assert!(stderr.contains("PRs as of 12 min ago"), "with its age:\n{stderr}");
        stale_pr.push(pr);
    }
    // Every sample above read a stale answer: the blocked worker never stored one.
    assert!(
        matches!(select_cached(&fixture.pr_store(), Some(&origin), unix_now()), CachedPrs::Stale(_)),
        "the store stayed stale"
    );
    assert!(proxy.wait_for_connections(1, Duration::from_secs(20)), "a worker made the blocked request");

    eprintln!(
        "fresh answer: full {fresh_full:.2?}, pr gather {fresh_pr:.2?}; \
         stale answer, blocked refresh: full {stale_full:.2?}, pr gather {stale_pr:.2?}"
    );
    assert!(
        fixture.wait_until_unlocked(Duration::from_secs(20), || proxy.close_held()),
        "the worker released its lock"
    );
    for pr in &stale_pr {
        assert!(*pr < PR_DEADLINE, "a stale answer waits for no request, got {pr:?}");
    }
    assert!(stale_full < FULL_COMMAND_BOUND, "full wt list with a stale answer {stale_full:.2?}");
}
