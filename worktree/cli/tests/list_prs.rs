//! `wt list`'s PR badges through the real binary, with every request sent to
//! a local proxy stub: a fresh store makes no request, a stale store shows its
//! badges with their age at once and leaves the request to a detached
//! `wt internal-refresh-prs`, and a refused connection on a miss shows the
//! table without badges and stores nothing.
//!
//! Timing bounds live in `perf_pr_request.rs`; these tests check behavior.

mod perf_support;

use std::fs;
use std::process::Output;
use std::time::Duration;

use perf_support::{MixedFixture, ProxyStub, stage_from_perf};
use serial_test::serial;
use worktree::pull_requests::{OpenPrSource, OpenPullRequest, RefreshOutcome, pr_lock_path, refresh, unix_now};

/// How long a test waits for the detached worker to act before failing.
const WORKER_WAIT: Duration = Duration::from_secs(20);

fn list(fixture: &MixedFixture, proxy: &ProxyStub) -> (Output, String) {
    let output = fixture
        .wt_command_via(proxy)
        .args(["list", "--perf"])
        .env("NO_COLOR", "1")
        .output()
        .expect("wt list should run");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(output.status.success(), "wt list failed:\n{stderr}");
    (output, stderr)
}

fn row<'a>(stderr: &'a str, needle: &str) -> &'a str {
    stderr
        .lines()
        .find(|line| line.contains(needle) && line.contains('│'))
        .unwrap_or_else(|| panic!("no table row with {needle:?}:\n{stderr}"))
}

/// Removes a store and its lock written into the real user cache (Windows
/// only; see `MixedFixture::pr_store`).
struct RemoveOnDrop(std::path::PathBuf);

impl Drop for RemoveOnDrop {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
        let _ = fs::remove_file(pr_lock_path(&self.0));
    }
}

/// A source that must never be asked: the test's own `refresh` runs only to
/// probe the worker's lock.
struct NoRequest;

impl OpenPrSource for NoRequest {
    fn source_repo(&self) -> Option<String> {
        None
    }
    fn fetch(&self) -> Result<Vec<OpenPullRequest>, String> {
        Err("the probe makes no request".into())
    }
}

/// Probes the refresh lock the way a competing worker would.
fn probe(fixture: &MixedFixture) -> RefreshOutcome {
    refresh(&fixture.pr_store(), fixture.main(), unix_now, |_| Box::new(NoRequest))
}

/// Ends a worker blocked on `proxy` and waits until it released its lock.
fn finish_worker(fixture: &MixedFixture, proxy: &ProxyStub) {
    let deadline = std::time::Instant::now() + WORKER_WAIT;
    loop {
        proxy.close_held();
        if probe(fixture) != RefreshOutcome::Contended {
            return;
        }
        assert!(std::time::Instant::now() < deadline, "the worker never released its lock");
        std::thread::sleep(Duration::from_millis(20));
    }
}

#[test]
#[serial]
fn a_fresh_pr_store_makes_no_request_and_shows_its_badges() {
    let fixture = MixedFixture::new().with_github_origin();
    let _cleanup = RemoveOnDrop(fixture.pr_store());
    fixture.seed_pr_store(Duration::from_secs(10), 99, "divergent-0");
    let proxy = ProxyStub::hanging();

    let (_, stderr) = list(&fixture, &proxy);

    assert_eq!(proxy.connections(), 0, "a store younger than 60 s skips the request");
    assert!(row(&stderr, "divergent-0").contains("PR #99"), "{stderr}");
    assert!(!stderr.contains("PRs as of"), "fresh results need no age line:\n{stderr}");
}

#[test]
#[serial]
fn a_stale_store_shows_its_badges_at_once_and_a_detached_worker_makes_the_request() {
    let fixture = MixedFixture::new().with_github_origin();
    let _cleanup = RemoveOnDrop(fixture.pr_store());
    fixture.seed_pr_store(Duration::from_secs(12 * 60 + 5), 99, "divergent-0");
    let stored = fs::read(fixture.pr_store()).expect("seeded store");
    let proxy = ProxyStub::hanging();

    // `.output()` captures stdout and stderr, so it returns only once every
    // holder of those pipes has exited: the worker must hold none of them.
    let (_, stderr) = list(&fixture, &proxy);

    assert!(row(&stderr, "divergent-0").contains("PR #99"), "{stderr}");
    assert!(stderr.contains("PRs as of 12 min ago"), "{stderr}");
    let waited = stage_from_perf(&stderr, "pr gather").expect("pr gather stage");
    assert!(waited < Duration::from_millis(300), "no request in the foreground, got {waited:?}");

    // The worker's request reaches the proxy after `wt list` returned, and
    // it holds the refresh lock while that request is blocked.
    assert!(proxy.wait_for_connections(1, WORKER_WAIT), "the worker never made its request");
    assert_eq!(probe(&fixture), RefreshOutcome::Contended, "the blocked worker holds the lock");
    finish_worker(&fixture, &proxy);
    assert_eq!(proxy.connections(), 1, "one worker, one request");
    assert_eq!(fs::read(fixture.pr_store()).expect("store"), stored, "a failed refresh is never stored");

    // The failed refresh left the answer stale, so the next list shows it
    // again and tries again.
    let (_, stderr) = list(&fixture, &proxy);
    assert!(row(&stderr, "divergent-0").contains("PR #99"), "{stderr}");
    assert!(proxy.wait_for_connections(2, WORKER_WAIT), "the next stale list refreshes again");
    finish_worker(&fixture, &proxy);
}

#[test]
#[serial]
fn a_changed_origin_hides_the_stored_badges_and_starts_no_worker() {
    let fixture = MixedFixture::new().with_github_origin();
    let _cleanup = RemoveOnDrop(fixture.pr_store());
    fixture.seed_pr_store(Duration::from_secs(12 * 60 + 5), 99, "divergent-0");
    let status = std::process::Command::new("git")
        .current_dir(fixture.main())
        .args(["remote", "set-url", "origin", "https://github.com/someone-else/repo.git"])
        .status()
        .expect("git");
    assert!(status.success());
    let proxy = ProxyStub::refusing();

    let (_, stderr) = list(&fixture, &proxy);

    assert!(!stderr.contains("PR #"), "badges from another origin:\n{stderr}");
    assert!(!stderr.contains("PRs as of"), "{stderr}");
    assert!(!pr_lock_path(&fixture.pr_store()).exists(), "a miss starts no worker");
}

#[test]
#[serial]
fn the_worker_command_prints_nothing_and_ignores_a_linked_worktree() {
    let fixture = MixedFixture::new().with_github_origin();
    let _cleanup = RemoveOnDrop(fixture.pr_store());
    fixture.seed_pr_store(Duration::from_secs(12 * 60 + 5), 99, "divergent-0");
    let stored = fs::read(fixture.pr_store()).expect("seeded store");
    let proxy = ProxyStub::refusing();
    let run_worker = |repo: &std::path::Path| {
        let output = fixture
            .wt_command_via(&proxy)
            .args(["internal-refresh-prs".as_ref(), repo.as_os_str()])
            .output()
            .expect("the worker runs");
        assert!(output.status.success(), "{output:?}");
        assert!(output.stdout.is_empty() && output.stderr.is_empty(), "{output:?}");
    };

    run_worker(&fixture.worktrees()[0]);
    assert!(!pr_lock_path(&fixture.pr_store()).exists(), "a linked worktree is not a main checkout");

    run_worker(fixture.main());
    assert!(pr_lock_path(&fixture.pr_store()).exists(), "the main checkout's worker took its lock");
    assert_eq!(fs::read(fixture.pr_store()).expect("store"), stored, "its failed refresh stored nothing");
}

#[test]
fn the_worker_command_is_hidden_from_help_and_completion() {
    let help = assert_cmd::Command::cargo_bin("wt").unwrap().arg("--help").output().expect("help");
    assert!(help.status.success());
    assert!(!String::from_utf8_lossy(&help.stdout).contains("internal-refresh-prs"));

    let completion = assert_cmd::Command::cargo_bin("wt")
        .unwrap()
        .env("COMPLETE", "fish")
        .args(["--", "wt", ""])
        .output()
        .expect("completion");
    assert!(completion.status.success());
    let offered = String::from_utf8_lossy(&completion.stdout);
    assert!(offered.contains("list"), "completion offers subcommands: {offered}");
    assert!(!offered.contains("internal-refresh-prs"), "{offered}");
}

#[test]
#[serial]
fn with_the_network_down_the_table_shows_without_badges_and_nothing_is_stored() {
    let fixture = MixedFixture::new().with_github_origin();
    let _cleanup = RemoveOnDrop(fixture.pr_store());
    let proxy = ProxyStub::refusing();

    let (_, stderr) = list(&fixture, &proxy);

    assert!(row(&stderr, "divergent-0").contains("divergent-0"));
    assert!(!stderr.contains("PR #"), "{stderr}");
    assert!(!stderr.contains("PRs as of"), "{stderr}");
    assert!(!fixture.pr_store().exists(), "an unavailable answer is not an empty list");

    // With results stored earlier, the same outage shows them with their age.
    let stored_at = std::time::Instant::now();
    fixture.seed_pr_store(Duration::from_secs(5 * 60), 99, "divergent-0");
    let stored = fs::read(fixture.pr_store()).expect("seeded store");
    let (_, stderr) = list(&fixture, &proxy);
    assert!(row(&stderr, "divergent-0").contains("PR #99"), "{stderr}");
    assert!(stderr.contains("PRs as of 5 min ago"), "{stderr}");
    // The worker it started fails against the refused port and leaves the
    // answer untouched; wait for its lock so it cannot outlive the fixture.
    let deadline = stored_at + WORKER_WAIT;
    while !pr_lock_path(&fixture.pr_store()).exists() || probe(&fixture) == RefreshOutcome::Contended {
        assert!(std::time::Instant::now() < deadline, "the worker never ran");
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(fs::read(fixture.pr_store()).expect("store"), stored);
}
