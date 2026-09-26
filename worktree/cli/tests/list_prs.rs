//! `wt list`'s PR badges through the real binary, with every request sent to
//! a local stand-in: a fresh store makes no request, a stale store shows its
//! badges with their age at once and leaves the request to a detached
//! `wt internal-refresh-prs`, and a refused connection on a miss shows the
//! table without badges and stores nothing.
//!
//! The worker's lifecycle runs against [`FakeGitea`], which holds a request
//! until the test releases it: the parent returns while its worker is
//! blocked, concurrent workers make one request, a killed worker releases its
//! lock, and a failed or misbound answer is never stored. A foreground
//! request during which `origin` changes shows no badges at all.
//!
//! Timing bounds live in `perf_pr_request.rs`; these tests check behavior.

mod perf_support;

use std::fs;
use std::process::{Child, Command, ExitStatus, Output, Stdio};
use std::time::{Duration, Instant};

use perf_support::{
    FakeGitea, GiteaReply, MixedFixture, ProxyStub, RemoveOnDrop, refresh_workers,
    wait_for_refresh_workers,
};
use serial_test::serial;
use worktree::pull_requests::{RefreshOutcome, pr_lock_path};

/// How long a test waits for the detached worker to act before failing.
const WORKER_WAIT: Duration = Duration::from_secs(20);

fn list(fixture: &MixedFixture, proxy: &ProxyStub) -> (Output, String) {
    list_with(fixture.wt_command_via(proxy))
}

fn list_with(mut command: Command) -> (Output, String) {
    let output = command
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

/// Ends a worker blocked on `proxy` and waits until it released its lock.
fn finish_worker(fixture: &MixedFixture, proxy: &ProxyStub) {
    assert!(
        fixture.wait_until_unlocked(WORKER_WAIT, || proxy.close_held()),
        "the worker never released its lock"
    );
}

/// On drop, answers every request still held by `gitea` and waits until no
/// worker for the fixture runs, so none outlives the fixture or its store.
/// Declare it after the fixture, the server, and [`RemoveOnDrop`].
struct Reaper<'a> {
    fixture: &'a MixedFixture,
    gitea: &'a FakeGitea,
}

impl Drop for Reaper<'_> {
    fn drop(&mut self) {
        self.gitea.release(GiteaReply::Status(503));
        let left = wait_for_refresh_workers(self.fixture.main(), 0, WORKER_WAIT);
        if !std::thread::panicking() {
            assert!(left.is_empty(), "workers outlived the test: {left:?}");
        }
    }
}

/// A test-owned child that is killed and reaped if the test fails first.
struct KillOnDrop(Child);

impl KillOnDrop {
    fn spawn(mut command: Command) -> Self {
        let child = command
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn wt");
        Self(child)
    }

    /// Waits up to [`WORKER_WAIT`] for the child to exit.
    fn wait(&mut self) -> ExitStatus {
        let deadline = Instant::now() + WORKER_WAIT;
        loop {
            if let Some(status) = self.0.try_wait().expect("poll child") {
                return status;
            }
            assert!(Instant::now() < deadline, "wt did not exit");
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}

impl Drop for KillOnDrop {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// A fixture whose stored answer (PR #99 on `divergent-0`) is 12 minutes old,
/// bound to [`FakeGitea::ORIGIN`].
fn stale_gitea_fixture() -> (MixedFixture, RemoveOnDrop, Vec<u8>) {
    let fixture = MixedFixture::new().with_gitea_origin();
    let cleanup = RemoveOnDrop(fixture.pr_store());
    fixture.seed_pr_store(Duration::from_secs(12 * 60 + 5), 99, "divergent-0");
    let stored = fs::read(fixture.pr_store()).expect("seeded store");
    (fixture, cleanup, stored)
}

fn stored(fixture: &MixedFixture) -> Vec<u8> {
    fs::read(fixture.pr_store()).expect("store")
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

    // The worker's request reaches the proxy after `wt list` returned, and
    // it holds the refresh lock while that request is blocked.
    assert!(proxy.wait_for_connections(1, WORKER_WAIT), "the worker never made its request");
    assert_eq!(fixture.probe_refresh(), RefreshOutcome::Contended, "the blocked worker holds the lock");
    finish_worker(&fixture, &proxy);
    // A foreground request would be a second connection. Its duration is
    // `perf_pr_request.rs`'s to bound: parallel L1 load on Windows pushes
    // even a no-request `pr gather` past the 300 ms deadline.
    assert_eq!(proxy.connections(), 1, "one worker, one request, none in the foreground");
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
    let stored_at = Instant::now();
    fixture.seed_pr_store(Duration::from_secs(5 * 60), 99, "divergent-0");
    let stored = fs::read(fixture.pr_store()).expect("seeded store");
    let (_, stderr) = list(&fixture, &proxy);
    assert!(row(&stderr, "divergent-0").contains("PR #99"), "{stderr}");
    assert!(stderr.contains("PRs as of 5 min ago"), "{stderr}");
    // The worker it started fails against the refused port and leaves the
    // answer untouched; wait for its lock so it cannot outlive the fixture.
    let deadline = stored_at + WORKER_WAIT;
    while !pr_lock_path(&fixture.pr_store()).exists() || fixture.probe_refresh() == RefreshOutcome::Contended {
        assert!(Instant::now() < deadline, "the worker never ran");
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(fs::read(fixture.pr_store()).expect("store"), stored);
}

#[test]
#[serial]
fn a_detached_workers_answer_replaces_the_stale_one_on_the_next_list() {
    let (fixture, _cleanup, seeded) = stale_gitea_fixture();
    let gitea = FakeGitea::new(GiteaReply::Status(500));
    gitea.hold();
    let _reaper = Reaper { fixture: &fixture, gitea: &gitea };

    // From a linked worktree, as a user standing in one would run it.
    let linked = &fixture.worktrees()[0];
    let mut command = fixture.wt_command_via_gitea(&gitea);
    command.current_dir(linked);
    let (_, stderr) = list_with(command);

    assert!(row(&stderr, "divergent-0").contains("PR #99"), "{stderr}");
    assert!(stderr.contains("PRs as of 12 min ago"), "{stderr}");

    // `wt list` returned with its output captured while its worker's request
    // is still unanswered and the worker holds the lock: it never joined it.
    assert!(gitea.wait_for_waiting(1, WORKER_WAIT), "the worker never made its request");
    assert_eq!(fixture.probe_refresh(), RefreshOutcome::Contended);
    let workers = refresh_workers(fixture.main());
    assert_eq!(workers.len(), 1, "{workers:?}");
    let cwd = workers[0].cwd.as_deref().expect("the worker's working directory");
    assert_eq!(
        fs::canonicalize(cwd).expect("worker cwd"),
        fs::canonicalize(fixture.main()).expect("main"),
        "the worker runs from the main checkout"
    );
    // Nothing holds the linked worktree (Windows refuses to rename a
    // directory that is some process's working directory).
    let moved = linked.with_extension("moved");
    fs::rename(linked, &moved).expect("the linked worktree is not held");
    fs::rename(&moved, linked).expect("move the linked worktree back");

    gitea.release(GiteaReply::Open(vec![(7, "divergent-1")]));
    assert!(wait_for_refresh_workers(fixture.main(), 0, WORKER_WAIT).is_empty(), "the worker finished");
    assert_ne!(stored(&fixture), seeded, "the worker stored its answer");
    assert_eq!(gitea.requests(), 1);

    let (_, stderr) = list_with(fixture.wt_command_via_gitea(&gitea));
    assert!(row(&stderr, "divergent-1").contains("PR #7"), "{stderr}");
    assert!(!stderr.contains("PR #99"), "{stderr}");
    assert!(!stderr.contains("PRs as of"), "a fresh answer has no age line:\n{stderr}");
    assert_eq!(gitea.requests(), 1, "a fresh answer makes no request");
    assert!(refresh_workers(fixture.main()).is_empty(), "and starts no worker");
}

#[test]
#[serial]
fn concurrent_lists_and_workers_make_one_request_and_a_fresh_answer_stops_the_next() {
    let (fixture, _cleanup, seeded) = stale_gitea_fixture();
    let gitea = FakeGitea::new(GiteaReply::Status(500));
    gitea.hold();
    let _reaper = Reaper { fixture: &fixture, gitea: &gitea };

    let mut lists: Vec<KillOnDrop> = (0..4)
        .map(|_| {
            let mut command = fixture.wt_command_via_gitea(&gitea);
            command.arg("list");
            KillOnDrop::spawn(command)
        })
        .collect();
    for list in &mut lists {
        assert!(list.wait().success());
    }
    assert!(gitea.wait_for_waiting(1, WORKER_WAIT), "one worker made the request");

    // More workers while that request is blocked: each finds the lock held
    // and exits without asking.
    for _ in 0..2 {
        assert!(KillOnDrop::spawn(fixture.refresh_worker_via_gitea(&gitea)).wait().success());
    }
    let workers = wait_for_refresh_workers(fixture.main(), 1, WORKER_WAIT);
    assert_eq!(workers.len(), 1, "every competitor exited: {workers:?}");
    assert_eq!(gitea.requests(), 1, "four lists and two workers, one request");
    assert_eq!(stored(&fixture), seeded);

    gitea.release(GiteaReply::Open(vec![(7, "divergent-1")]));
    assert!(wait_for_refresh_workers(fixture.main(), 0, WORKER_WAIT).is_empty());
    let refreshed = stored(&fixture);
    assert_ne!(refreshed, seeded);

    // A worker started after that success rechecks under the lock and finds
    // the answer fresh.
    assert!(KillOnDrop::spawn(fixture.refresh_worker_via_gitea(&gitea)).wait().success());
    assert_eq!(gitea.requests(), 1, "a fresh answer is not requested again");
    assert_eq!(stored(&fixture), refreshed);
}

#[test]
#[serial]
fn a_killed_worker_releases_its_lock_and_a_later_worker_refreshes() {
    let (fixture, _cleanup, seeded) = stale_gitea_fixture();
    let gitea = FakeGitea::new(GiteaReply::Status(500));
    gitea.hold();
    let _reaper = Reaper { fixture: &fixture, gitea: &gitea };

    let mut worker = KillOnDrop::spawn(fixture.refresh_worker_via_gitea(&gitea));
    assert!(gitea.wait_for_waiting(1, WORKER_WAIT), "the worker made its request");
    assert_eq!(fixture.probe_refresh(), RefreshOutcome::Contended);

    worker.0.kill().expect("kill the worker");
    worker.0.wait().expect("reap the worker");
    assert!(fixture.wait_until_unlocked(WORKER_WAIT, || {}), "the OS released the dead worker's lock");
    assert_eq!(stored(&fixture), seeded, "a crash stores nothing");
    assert!(pr_lock_path(&fixture.pr_store()).exists(), "the sidecar stays");

    gitea.release(GiteaReply::Open(vec![(7, "divergent-1")]));
    assert!(KillOnDrop::spawn(fixture.refresh_worker_via_gitea(&gitea)).wait().success());
    assert_eq!(gitea.requests(), 2);
    let (_, stderr) = list_with(fixture.wt_command_via_gitea(&gitea));
    assert!(row(&stderr, "divergent-1").contains("PR #7"), "{stderr}");
    assert!(!stderr.contains("PRs as of"), "{stderr}");
}

#[test]
#[serial]
fn a_failed_or_unauthorized_refresh_keeps_the_stored_answer() {
    let (fixture, _cleanup, seeded) = stale_gitea_fixture();

    for status in [500, 401] {
        let gitea = FakeGitea::new(GiteaReply::Status(status));
        let _reaper = Reaper { fixture: &fixture, gitea: &gitea };

        assert!(KillOnDrop::spawn(fixture.refresh_worker_via_gitea(&gitea)).wait().success());

        assert!(gitea.requests() >= 1, "HTTP {status}: the worker asked");
        assert_eq!(stored(&fixture), seeded, "HTTP {status} is never stored, not even as no PRs");
        let (_, stderr) = list_with(fixture.wt_command_via_gitea(&gitea));
        assert!(row(&stderr, "divergent-0").contains("PR #99"), "HTTP {status}:\n{stderr}");
        assert!(stderr.contains("PRs as of 12 min ago"), "HTTP {status}:\n{stderr}");
    }
}

#[test]
#[serial]
fn an_origin_change_during_a_workers_request_discards_its_answer() {
    let (fixture, _cleanup, seeded) = stale_gitea_fixture();
    let gitea = FakeGitea::new(GiteaReply::Status(500));
    gitea.hold();
    let _reaper = Reaper { fixture: &fixture, gitea: &gitea };

    let mut worker = KillOnDrop::spawn(fixture.refresh_worker_via_gitea(&gitea));
    assert!(gitea.wait_for_waiting(1, WORKER_WAIT), "the worker made its request");
    let status = Command::new("git")
        .current_dir(fixture.main())
        .args(["remote", "set-url", "origin", "http://gitea.test/o/other.git"])
        .status()
        .expect("git");
    assert!(status.success());
    gitea.release(GiteaReply::Open(vec![(7, "divergent-1")]));

    assert!(worker.wait().success());
    assert_eq!(gitea.requests(), 1);
    assert_eq!(stored(&fixture), seeded, "an answer for the old origin is discarded");

    // The stored answer belongs to the old origin, so the next list shows
    // none of it (its foreground request for the new origin is a miss's,
    // covered elsewhere).
    let (_, stderr) = list_with(fixture.wt_command_via_gitea(&gitea));
    assert!(!stderr.contains("PR #99"), "{stderr}");
    assert!(!stderr.contains("PRs as of"), "{stderr}");
}

#[test]
#[serial]
fn an_origin_change_during_a_foreground_request_shows_no_badges_from_the_old_origin() {
    let fixture = MixedFixture::new().with_gitea_origin();
    let _cleanup = RemoveOnDrop(fixture.pr_store());
    let gitea = FakeGitea::new(GiteaReply::Open(vec![(7, "divergent-1")]));

    // Control: with no stored answer the foreground request's answer is
    // shown, so the absence below is the discarded answer, not a timeout.
    let (_, stderr) = list_with(fixture.wt_command_via_gitea(&gitea));
    assert!(row(&stderr, "divergent-1").contains("PR #7"), "{stderr}");
    fs::remove_file(fixture.pr_store()).expect("forget the stored answer");

    let main = fixture.main().to_path_buf();
    gitea.before_reply(move || {
        let status = Command::new("git")
            .current_dir(&main)
            .args(["remote", "set-url", "origin", "http://gitea.example.invalid/o/other.git"])
            .status()
            .expect("git");
        assert!(status.success());
    });
    let (_, stderr) = list_with(fixture.wt_command_via_gitea(&gitea));

    assert_eq!(gitea.requests(), 2, "the second list made its request in the foreground");
    assert!(row(&stderr, "divergent-1").contains("divergent-1"));
    assert!(!stderr.contains("PR #"), "badges from the previous origin:\n{stderr}");
    assert!(!stderr.contains("PRs as of"), "{stderr}");
    assert!(!fixture.pr_store().exists(), "an answer for the old origin is not stored");
}
