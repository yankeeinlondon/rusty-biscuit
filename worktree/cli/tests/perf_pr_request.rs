//! `wt list` performance with the network down, with a stale stored answer
//! whose refresh fails, with a PR request held by the provider, with a
//! live-head check or a fetch held by `origin`, and with `-r` and `--ff`
//! against a held `origin`, against the targets in
//! `worktree/docs/performance-testing.md` (warm `list gather` 120 ms, cold
//! 300 ms, full non-image `wt list` 1 s when the worker answers at once, 3 s
//! plus that when it stalls, and the worker's 10 s check and 60 s fetch
//! deadlines for `-r` and `--ff`). Every request goes to a local stand-in, so
//! nothing leaves the host.

mod perf_support;
mod remote_fixture;

use std::process::Stdio;
use std::time::{Duration, Instant};

use perf_support::{
    FakeGitea, GiteaReply, HoldingOrigin, MixedFixture, ProxyStub, RemoveOnDrop, list_gather_from_perf,
    refresh_workers, stage_from_perf, wait_for_refresh_workers,
};
use remote_fixture::{Fixture, UploadPackGate, assert_no_spinner};
use serial_test::serial;
use worktree::pull_requests::{CachedPrs, origin_url, select_cached, unix_now};
use worktree::remote_head::REMOTE_HEAD_REFRESH_DEADLINE;
use worktree::remote_update::FETCH_DEADLINE;

const WARM_LIST_GATHER_BOUND: Duration = Duration::from_millis(120);
const COLD_LIST_GATHER_BOUND: Duration = Duration::from_millis(300);
const FULL_COMMAND_BOUND: Duration = Duration::from_millis(1000);
/// What a stage may take beyond the work it waits for: `pr gather` (the PR
/// store reads around the wait, no request) and `remote wait` past its 3 s.
const STAGE_SLACK: Duration = Duration::from_millis(300);
/// Ordinary listing's wait for a stalled worker (spec §3).
const REMOTE_WAIT: Duration = Duration::from_secs(3);
/// What a forced listing may add to the worker's own deadline: launching it,
/// killing the held transport, publishing, and the local gather and render.
const FORCED_SLACK: Duration = Duration::from_secs(3);

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

/// A live-head answer checked just now, so the caption has an answer to date.
fn seed_fresh_head(fixture: &MixedFixture) {
    fixture.seed_remote_head_store(Duration::ZERO, Some("0123456789abcdef0123456789abcdef01234567"));
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
    let _cleanup = RemoveOnDrop(fixture.pr_store());
    seed_fresh_head(&fixture);
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

/// Every listing's worker asks for open PRs, fresh answer or stale; when that
/// request fails at once, the listing still meets the 1 s bound and shows the
/// stale answer as one it couldn't refresh. `pr gather` (the store reads
/// around the wait) must stay under [`STAGE_SLACK`] too, since the 1 s bound
/// alone would hide a reintroduced foreground request.
#[test]
#[serial]
fn perf_list_meets_sla_with_a_stale_answer_and_a_failing_refresh() {
    const STALE: Duration = Duration::from_secs(12 * 60 + 5);
    let fixture = MixedFixture::new().with_github_origin();
    let _cleanup = RemoveOnDrop(fixture.pr_store());
    seed_fresh_head(&fixture);
    fixture.warm_untracked_cache();
    // Every worker request is counted and fails at once.
    let proxy = ProxyStub::closing_after(Duration::ZERO);
    let origin = origin_url(fixture.main()).expect("origin");

    // Warm the comparison cache with a fresh answer. Its workers ask for
    // open PRs as well as for the live head: one request per half.
    let _ = pr_gather_with_store(&fixture, &proxy, Duration::ZERO);
    let fresh_full = best_full_command_with_store(&fixture, &proxy, Duration::ZERO);
    let fresh_pr = (0..5).map(|_| pr_gather_with_store(&fixture, &proxy, Duration::ZERO).0).min().unwrap();
    assert!(fixture.wait_until_unlocked(Duration::from_secs(20), || ()), "the fresh runs' workers exited");
    assert_eq!(proxy.connections(), 2 * 11, "a fresh answer is asked for again, beside the live-head check");

    let stale_full = best_full_command_with_store(&fixture, &proxy, STALE);
    let mut stale_pr = Vec::new();
    for _ in 0..5 {
        let (pr, stderr) = pr_gather_with_store(&fixture, &proxy, STALE);
        assert!(stderr.contains("PR #99"), "the stale badge is shown:\n{stderr}");
        assert!(stderr.contains("PRs as of 12 min ago (couldn't refresh)"), "with its age:\n{stderr}");
        assert!(!stderr.contains("running this command again"), "the failure ended the wait:\n{stderr}");
        stale_pr.push(pr);
    }
    assert!(fixture.wait_until_unlocked(Duration::from_secs(20), || ()), "the stale runs' workers exited");
    // Every sample above read a stale answer: the failing worker never stored one.
    assert!(
        matches!(select_cached(&fixture.pr_store(), Some(&origin), unix_now()), CachedPrs::Stale(_)),
        "the store stayed stale"
    );
    assert_eq!(proxy.connections(), 2 * (11 + 10), "the stale runs' workers made one request per half");

    eprintln!(
        "fresh answer: full {fresh_full:.2?}, pr gather {fresh_pr:.2?}; \
         stale answer, failing refresh: full {stale_full:.2?}, pr gather {stale_pr:.2?}"
    );
    for pr in &stale_pr {
        assert!(*pr < STAGE_SLACK, "no PR request in the foreground, got {pr:?}");
    }
    assert!(stale_full < FULL_COMMAND_BOUND, "full wt list with a stale answer {stale_full:.2?}");
}

/// A PR request that the provider holds costs a listing its 3 s wait and no
/// more, like a held live-head check: the worker's head half ends at once
/// (the branch-head API answers 404 and git's fallback is refused), the
/// `remote wait` stage ends at the wait, and the command returns within that
/// plus the full-command bound while the PR request is still held.
#[test]
#[serial]
fn perf_a_held_pr_request_costs_the_listing_only_its_wait() {
    let fixture = MixedFixture::new().with_gitea_origin();
    let _cleanup = RemoveOnDrop(fixture.pr_store());
    fixture.seed_pr_store(Duration::from_secs(12 * 60 + 5), 99, "divergent-0");
    seed_fresh_head(&fixture);
    fixture.warm_untracked_cache();
    let gitea = FakeGitea::new(GiteaReply::Status(503));
    gitea.hold();

    // The second listing's worker finds the first one's PR request holding
    // the lock, and waits within the same budget.
    let mut samples = Vec::new();
    for _ in 0..2 {
        let t0 = Instant::now();
        let output = fixture
            .wt_command_via_gitea(&gitea)
            .args(["list", "--perf"])
            .env("NO_COLOR", "1")
            .output()
            .expect("wt list --perf should run");
        let full = t0.elapsed();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        assert!(output.status.success(), "wt list --perf failed:\n{stderr}");
        let text = stderr.split_whitespace().collect::<Vec<_>>().join(" ");
        assert!(text.contains("- PRs as of 12 min ago - running this command again"), "pending, then the hint:\n{stderr}");
        samples.push((stage_from_perf(&stderr, "remote wait").expect("remote wait stage"), full));
    }
    let held = gitea.waiting() == 1;
    gitea.release(GiteaReply::Status(503));
    let left = wait_for_refresh_workers(fixture.main(), 0, Duration::from_secs(20));

    eprintln!("held PR request: (remote wait, full) {samples:.2?}");
    assert!(held, "the PR request was still held after both listings returned");
    assert_eq!(gitea.requests(), 1, "one PR request at a time");
    assert!(left.is_empty(), "every worker exited: {left:?}");
    for (wait, full) in &samples {
        assert!(*wait >= REMOTE_WAIT && *wait < REMOTE_WAIT + STAGE_SLACK, "remote wait {wait:?}");
        assert!(*full < REMOTE_WAIT + FULL_COMMAND_BOUND, "full wt list {full:?}");
    }
}

/// A live-head check that `origin` holds costs a listing its 3 s wait and no
/// more: the `remote wait` stage ends at the wait, and the command returns
/// within that plus the full-command bound while the worker is still held.
/// The deterministic no-join proof is
/// `list_prs::a_held_live_head_check_holds_the_listing_only_until_its_deadline`.
#[test]
#[serial]
fn perf_a_held_live_head_check_costs_the_listing_only_its_wait() {
    let origin = HoldingOrigin::new();
    let fixture = MixedFixture::new().with_origin(&origin.url());
    let _cleanup = RemoveOnDrop(fixture.pr_store());
    fixture.seed_empty_pr_store(Duration::ZERO);
    fixture.warm_untracked_cache();

    fixture.seed_remote_head_store(Duration::from_secs(12 * 60 + 5), Some("0123456789abcdef0123456789abcdef01234567"));

    // The second listing adopts the first one's held attempt.
    let mut samples = Vec::new();
    for _ in 0..2 {
        let t0 = Instant::now();
        let output = fixture
            .wt_command_direct()
            .args(["list", "--perf"])
            .output()
            .expect("wt list --perf should run");
        let full = t0.elapsed();
        assert!(output.status.success(), "wt list --perf failed");
        let stderr = String::from_utf8_lossy(&output.stderr);
        samples.push((stage_from_perf(&stderr, "remote wait").expect("remote wait stage"), full));
    }
    let blocked = origin.wait_for_requests(1, Duration::from_secs(20));
    let held = fixture.head_lock_held();
    let released = fixture.wait_until_unlocked(Duration::from_secs(20), || origin.close_held());

    eprintln!("held live-head check: (remote wait, full) {samples:.2?}");
    assert!(blocked, "a worker made the held request");
    assert!(held, "the worker was still held after both listings returned");
    assert!(released, "the worker exited and released its locks");
    for (wait, full) in &samples {
        assert!(*wait >= REMOTE_WAIT && *wait < REMOTE_WAIT + STAGE_SLACK, "remote wait {wait:?}");
        assert!(*full < REMOTE_WAIT + FULL_COMMAND_BOUND, "full wt list {full:?}");
    }
}

/// `wt <args> --perf` from the fixture's main checkout: its elapsed time, its
/// `remote wait` stage, and its stderr with whitespace collapsed.
fn timed_listing(fixture: &Fixture, args: &[&str]) -> (Duration, Duration, String) {
    let t0 = Instant::now();
    let output = fixture.wt(&fixture.main).args(args).arg("--perf").output().expect("wt runs");
    let elapsed = t0.elapsed();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "wt {args:?} failed:\n{stderr}");
    assert_no_spinner(&stderr);
    let wait = stage_from_perf(&stderr, "remote wait").expect("remote wait stage");
    (elapsed, wait, stderr.split_whitespace().collect::<Vec<_>>().join(" "))
}

/// A fetch that `origin` holds costs a listing its 3 s wait and no more: the
/// check answered, so the row is "still pulling", and `.output()` returns
/// while the worker is still held in the fetch.
#[test]
#[serial]
fn perf_a_held_fetch_costs_the_listing_only_its_wait() {
    let fixture = Fixture::new();
    fixture.commit_and_push("second");
    let gate = UploadPackGate::install(&fixture, 1);

    let (full, wait, stderr) = timed_listing(&fixture, &["list"]);
    let still_held = gate.runs() == 2 && refresh_workers(&fixture.main).len() == 1;

    eprintln!("held fetch: remote wait {wait:.2?}, full {full:.2?}");
    assert!(stderr.contains("pulling remote updates in the background"), "still pulling:\n{stderr}");
    assert!(still_held, "the worker was still held in its fetch after the listing returned");
    assert!(wait >= REMOTE_WAIT && wait < REMOTE_WAIT + STAGE_SLACK, "remote wait {wait:?}");
    assert!(full < REMOTE_WAIT + FULL_COMMAND_BOUND, "full wt list {full:?}");
}

/// `-r` against an `origin` that holds the check waits for the worker, which
/// gives up at its 10 s check deadline; the listing then reports the failure.
#[test]
#[serial]
fn perf_refresh_against_a_held_check_reports_within_the_check_deadline() {
    let origin = HoldingOrigin::new();
    let fixture = MixedFixture::new().with_origin(&origin.url());
    let _cleanup = RemoveOnDrop(fixture.pr_store());
    fixture.seed_empty_pr_store(Duration::ZERO);
    fixture.warm_untracked_cache();

    let t0 = Instant::now();
    let output = fixture.wt_command_direct().args(["list", "-r"]).env("NO_COLOR", "1").output().expect("wt list -r runs");
    let full = t0.elapsed();
    let stderr = String::from_utf8_lossy(&output.stderr).split_whitespace().collect::<Vec<_>>().join(" ");
    let released = fixture.wait_until_unlocked(Duration::from_secs(20), || origin.close_held());

    eprintln!("-r, held check: full {full:.2?}");
    assert!(output.status.success(), "wt list -r failed:\n{stderr}");
    assert!(origin.wait_for_requests(1, Duration::ZERO), "the worker's check reached origin");
    assert!(
        stderr.contains(&format!("origin didn't answer within {} s", REMOTE_HEAD_REFRESH_DEADLINE.as_secs())),
        "the check's deadline is reported:\n{stderr}"
    );
    assert!(released, "the worker exited and released its locks");
    assert!(
        full >= REMOTE_HEAD_REFRESH_DEADLINE && full < REMOTE_HEAD_REFRESH_DEADLINE + FORCED_SLACK,
        "wt list -r {full:?}"
    );
}

/// `--ff` against an `origin` that holds the fetch waits for the worker, which
/// gives up at its 60 s fetch deadline; the listing reports the failed fetch
/// and moves nothing, since the local tracking ref did not change.
#[test]
#[serial]
fn perf_fast_forward_against_a_held_fetch_reports_within_the_fetch_deadline() {
    let fixture = Fixture::new();
    let before = fixture.git(&fixture.main, &["rev-parse", "main"]);
    fixture.commit_and_push("second");
    let gate = UploadPackGate::install(&fixture, 1);

    let (full, wait, stderr) = timed_listing(&fixture, &["--ff"]);

    eprintln!("--ff, held fetch: remote wait {wait:.2?}, full {full:.2?}");
    assert_eq!(gate.runs(), 2, "one check and one fetch");
    assert!(
        stderr.contains(&format!("fetch didn't finish within {} s", FETCH_DEADLINE.as_secs())),
        "the fetch's deadline is reported:\n{stderr}"
    );
    assert_eq!(fixture.git(&fixture.main, &["rev-parse", "main"]), before, "main did not move");
    assert!(full >= FETCH_DEADLINE && full < FETCH_DEADLINE + FORCED_SLACK, "wt --ff {full:?}");
}
