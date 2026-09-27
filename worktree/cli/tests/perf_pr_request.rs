//! `wt list` performance with the network down and with a PR request that
//! hits its deadline, against the ratified targets in
//! `worktree/docs/performance-testing.md` (warm `list gather` 120 ms, cold
//! 300 ms, full non-image `wt list` 1 s). Every request goes to a local proxy
//! stub, so nothing leaves the host.

mod perf_support;

use std::process::Stdio;
use std::time::{Duration, Instant};

use perf_support::{MixedFixture, ProxyStub, list_gather_from_perf, stage_from_perf};
use serial_test::serial;

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
