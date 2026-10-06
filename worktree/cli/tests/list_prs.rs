//! `wt list`'s PR badges through the real binary, with every request sent to
//! a local stand-in: every listing's detached `wt internal-refresh` asks for
//! open PRs whatever the stored answer's age, and the listing waits for that
//! answer within its 3 s budget. An answer that arrives in time is shown with
//! no status item (an empty one clears the badges); a failed or refused
//! request stores nothing and shows the stored badges dated `(couldn't
//! refresh)`, or `couldn't get open PRs` with none stored; a request still
//! held at 3 s ends the wait with the refresh hint. No origin, a local-path,
//! or an unsupported origin shows no PR badges or item (an ignored one is
//! `list_flags.rs`'s, which covers `--ignore-api`). A
//! rejected key on the PR request names the key in an ordinary listing, and
//! a PR failure never blocks `--ff`.
//!
//! Git's own HTTP transport is refused (see `perf_support`), so the worker's
//! live-head half fails fast once its provider request does. That request
//! asks for the branch head: [`FakeGitea`] answers it apart from its PR
//! requests, and through [`ProxyStub`] it is one more connection.
//!
//! The worker's lifecycle runs against [`FakeGitea`], which holds a request
//! until the test releases it: the parent returns while its worker is
//! blocked, concurrent workers make one request, a killed worker releases its
//! lock, and a failed or misbound answer is never stored. A listing during
//! whose wait `origin` is replaced or removed shows no badges and no notice
//! about the old origin's requests, the head check's included.
//!
//! The live head is never asked in the foreground: `wt list` waits at most
//! 3 s for its worker, and returns while the worker's `ls-remote` is still
//! held by a loopback `origin`.
//!
//! A listing whose worker is held past its wait cuts that wait to
//! [`HELD_WAIT`] (debug builds only), since the held request outlasts any
//! budget. `a_held_pr_request_with_nothing_stored_ends_at_the_budget_with_only_the_hint`
//! keeps the real 3 s wait.
//!
//! Timing bounds live in `perf_pr_request.rs`; these tests check behavior.

mod perf_support;

use std::fs;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use perf_support::{
    FakeGitea, GiteaReply, HoldingOrigin, KillOnDrop, MANUFACTURED_FAILURE, MixedFixture, ProxyStub, WorkerReaper,
    assert_manufactured_failure, process_running, refresh_workers, wait_for_refresh_workers,
};
use serial_test::serial;
use worktree::pull_requests::{CachedPrs, RefreshOutcome, pr_lock_path, select_cached, unix_now};
use worktree::remote_head::{AnswerSource, CheckFailure, Outcome, Phase, read_store, remote_head_lock_path};
use worktree_cli::env::TEST_WAIT_BUDGET_VAR;

/// How long a test waits for the detached worker to act before failing.
const WORKER_WAIT: Duration = Duration::from_secs(20);

/// The wait for a listing whose worker is held before anything the listing
/// asserts could finish.
const HELD_WAIT: Duration = Duration::from_millis(300);

/// `command` with its listing's wait cut to [`HELD_WAIT`].
fn held(mut command: Command) -> Command {
    command.env(TEST_WAIT_BUDGET_VAR, HELD_WAIT.as_millis().to_string());
    command
}

fn list(fixture: &MixedFixture, proxy: &ProxyStub) -> (Output, String) {
    list_with(fixture.wt_command_via(proxy))
}

fn list_with(mut command: Command) -> (Output, String) {
    let output = command
        .arg("list")
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

/// Ends a worker blocked on `proxy` and asserts that it exited and released
/// both locks. When an assertion fails first, dropping the proxy and then the
/// fixture does the same.
fn finish_worker(fixture: &MixedFixture, proxy: &ProxyStub) {
    assert!(
        fixture.wait_until_unlocked(WORKER_WAIT, || proxy.close_held()),
        "the worker never exited or released its locks"
    );
}

/// Closes `proxy`'s held connections once `count` have arrived, so a
/// listing's worker fails its requests at once instead of holding the
/// listing for its whole wait. Run the listing inside `scope`.
fn close_after<'scope>(scope: &'scope std::thread::Scope<'scope, '_>, proxy: &'scope ProxyStub, count: usize) {
    scope.spawn(move || {
        if proxy.wait_for_connections(count, WORKER_WAIT) {
            proxy.close_held();
        }
    });
}

/// A live-head answer checked just now; the listing's check still runs, but
/// its caption has an earlier answer to date.
fn seed_fresh_head(fixture: &MixedFixture) {
    fixture.seed_remote_head_store(Duration::ZERO, Some(HEAD_SHA));
}

const HEAD_SHA: &str = "0123456789abcdef0123456789abcdef01234567";

/// A fixture whose stored answer (PR #99 on `divergent-0`) is 12 minutes old,
/// bound to [`FakeGitea::ORIGIN`].
fn stale_gitea_fixture() -> (MixedFixture, Vec<u8>) {
    let fixture = MixedFixture::new().with_gitea_origin();
    fixture.seed_pr_store(Duration::from_secs(12 * 60 + 5), 99, "divergent-0");
    seed_fresh_head(&fixture);
    let stored = fs::read(fixture.pr_store()).expect("seeded store");
    (fixture, stored)
}

fn stored(fixture: &MixedFixture) -> Vec<u8> {
    fs::read(fixture.pr_store()).expect("store")
}

/// Whitespace collapsed, so wrapped lines read as one.
fn collapsed(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// No PR status item and no refresh hint.
fn assert_no_pr_item_or_hint(stderr: &str) {
    let text = collapsed(stderr);
    for item in ["PRs as of", "couldn't refresh", "couldn't get open PRs", "running this command again"] {
        assert!(!text.contains(item), "{item:?} is shown:\n{stderr}");
    }
}

#[test]
#[serial]
fn a_fresh_pr_store_is_asked_again_and_a_failed_request_keeps_its_badges() {
    let fixture = MixedFixture::new().with_github_origin();
    fixture.seed_pr_store(Duration::from_secs(10), 99, "divergent-0");
    seed_fresh_head(&fixture);
    let proxy = ProxyStub::hanging();

    // One request per half, the PR half's included; closing both ends the
    // worker at once.
    let (_, stderr) = std::thread::scope(|scope| {
        close_after(scope, &proxy, 2);
        list(&fixture, &proxy)
    });
    finish_worker(&fixture, &proxy);

    assert_eq!(proxy.connections(), 2, "a store younger than 60 s is asked for again");
    assert!(row(&stderr, "divergent-0").contains("PR #99"), "{stderr}");
    assert!(stderr.contains("PRs as of less than 1 min ago (couldn't refresh)"), "{stderr}");
    assert!(!stderr.contains("running this command again"), "the wait ended with both halves:\n{stderr}");
}

#[test]
#[serial]
fn a_stale_store_shows_its_badges_and_its_workers_failed_request_stores_nothing() {
    let fixture = MixedFixture::new().with_github_origin();
    fixture.seed_pr_store(Duration::from_secs(12 * 60 + 5), 99, "divergent-0");
    seed_fresh_head(&fixture);
    let stored = fs::read(fixture.pr_store()).expect("seeded store");
    let proxy = ProxyStub::hanging();

    // The worker makes one request per half: its PR half's and its live-head
    // half's branch-head request, which the proxy cannot tell apart. Held,
    // both would end at sniff's 3 s connect timeout, racing the listing's own
    // 3 s wait; `a_detached_workers_answer_replaces_the_stale_one_on_the_next_list`
    // holds a PR request past the listing instead.
    let (_, stderr) = std::thread::scope(|scope| {
        close_after(scope, &proxy, 2);
        list(&fixture, &proxy)
    });
    finish_worker(&fixture, &proxy);

    assert!(row(&stderr, "divergent-0").contains("PR #99"), "{stderr}");
    assert!(stderr.contains("PRs as of 12 min ago"), "{stderr}");
    // A foreground request would be a third connection. Its duration is
    // `perf_pr_request.rs`'s to bound: parallel L1 load on Windows pushes
    // even no-request PR store reads past that file's 300 ms stage bound.
    assert_eq!(proxy.connections(), 2, "one worker, one request per half, none in the foreground");
    assert_eq!(fs::read(fixture.pr_store()).expect("store"), stored, "a failed refresh is never stored");
}

#[test]
#[serial]
fn a_changed_origin_hides_the_stored_badges_and_its_worker_stores_nothing() {
    let fixture = MixedFixture::new().with_github_origin();
    fixture.seed_pr_store(Duration::from_secs(12 * 60 + 5), 99, "divergent-0");
    let stored = fs::read(fixture.pr_store()).expect("seeded store");
    let status = std::process::Command::new("git")
        .current_dir(fixture.main())
        .args(["remote", "set-url", "origin", "https://github.com/someone-else/repo.git"])
        .status()
        .expect("git");
    assert!(status.success());
    let proxy = ProxyStub::refusing();

    let (_, stderr) = list(&fixture, &proxy);
    finish_worker(&fixture, &proxy);

    assert!(!stderr.contains("PR #"), "badges from another origin:\n{stderr}");
    assert!(!stderr.contains("PRs as of"), "{stderr}");
    // Whether the refused check ends inside the 3 s wait depends on the
    // host (native Windows spends about 2 s on each refused connection), but
    // either row dates nothing: no answer for this origin was ever stored.
    let text = stderr.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(text.contains("; never checked with origin)"), "{stderr}");
    assert_eq!(fs::read(fixture.pr_store()).expect("store"), stored, "the refused requests stored nothing");
}

#[test]
#[serial]
fn the_worker_command_prints_nothing_and_ignores_anything_but_a_main_checkout() {
    let fixture = MixedFixture::new().with_github_origin();
    fixture.seed_pr_store(Duration::from_secs(12 * 60 + 5), 99, "divergent-0");
    fixture.seed_remote_head_store(Duration::from_secs(12 * 60 + 5), Some(HEAD_SHA));
    let stored = fs::read(fixture.pr_store()).expect("seeded store");
    let stored_head = read_store(&fixture.remote_head_store()).answer.expect("seeded head");
    let head_lock = remote_head_lock_path(&fixture.remote_head_store());
    let proxy = ProxyStub::refusing();
    let run_worker = |repo: &std::path::Path| {
        let output = fixture
            .wt_command_via(&proxy)
            .args(["internal-refresh".as_ref(), repo.as_os_str()])
            .output()
            .expect("the worker runs");
        assert!(output.status.success(), "{output:?}");
        assert!(output.stdout.is_empty() && output.stderr.is_empty(), "{output:?}");
    };

    let subdirectory = fixture.main().join("nested");
    fs::create_dir(&subdirectory).expect("create a subdirectory");
    let missing = fixture.main().join("no-such-directory");
    for ignored in [fixture.worktrees()[0].as_path(), subdirectory.as_path(), missing.as_path()] {
        run_worker(ignored);
        assert!(!pr_lock_path(&fixture.pr_store()).exists(), "{ignored:?} is not a main checkout");
        assert!(!head_lock.exists(), "{ignored:?} is not a main checkout");
    }

    run_worker(fixture.main());
    assert!(pr_lock_path(&fixture.pr_store()).exists(), "the main checkout's worker took its PR lock");
    assert!(head_lock.exists(), "and its live-head lock");
    assert_eq!(fs::read(fixture.pr_store()).expect("store"), stored, "its failed PR refresh stored nothing");
    let head = read_store(&fixture.remote_head_store());
    assert_eq!(head.answer, Some(stored_head), "nor its failed live-head check replace the answer");
    let attempt = head.attempt.expect("the attempt is recorded");
    assert_eq!(attempt.outcome, Some(Outcome::CheckFailed { reason: CheckFailure::Other }), "{attempt:?}");
}

#[test]
fn the_worker_command_is_hidden_from_help_and_completion() {
    let help = assert_cmd::Command::cargo_bin("wt").unwrap().arg("--help").output().expect("help");
    assert!(help.status.success());
    assert!(!String::from_utf8_lossy(&help.stdout).contains("internal-refresh"));

    let completion = assert_cmd::Command::cargo_bin("wt")
        .unwrap()
        .env("COMPLETE", "fish")
        .args(["--", "wt", ""])
        .output()
        .expect("completion");
    assert!(completion.status.success());
    let offered = String::from_utf8_lossy(&completion.stdout);
    assert!(offered.contains("list"), "completion offers subcommands: {offered}");
    assert!(!offered.contains("internal-refresh"), "{offered}");
}

#[test]
#[serial]
fn with_the_network_down_the_table_shows_without_badges_and_nothing_is_stored() {
    let fixture = MixedFixture::new().with_github_origin();
    seed_fresh_head(&fixture);
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
    while !pr_lock_path(&fixture.pr_store()).exists() {
        assert!(Instant::now() < deadline, "the worker never ran");
        std::thread::sleep(Duration::from_millis(20));
    }
    assert!(fixture.wait_until_unlocked(WORKER_WAIT, || {}), "the worker never finished");
    assert_eq!(fs::read(fixture.pr_store()).expect("store"), stored);
}

#[test]
#[serial]
fn a_detached_workers_answer_replaces_the_stale_one_on_the_next_list() {
    let (fixture, seeded) = stale_gitea_fixture();
    let gitea = FakeGitea::new(GiteaReply::Status(500));
    gitea.hold();
    let _reaper = WorkerReaper::new(&fixture, &gitea);

    // From a linked worktree, as a user standing in one would run it.
    let linked = &fixture.worktrees()[0];
    let mut command = held(fixture.wt_command_via_gitea(&gitea));
    command.current_dir(linked);
    let (_, stderr) = list_with(command);

    assert!(row(&stderr, "divergent-0").contains("PR #99"), "{stderr}");
    let text = stderr.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(text.contains("- PRs as of 12 min ago - running this command again"), "pending, then the hint: {stderr}");

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
    assert_eq!(gitea.branch_requests(), 1, "its live-head half asked the provider for the branch head");

    let (_, stderr) = list_with(fixture.wt_command_via_gitea(&gitea));
    assert!(row(&stderr, "divergent-1").contains("PR #7"), "{stderr}");
    assert!(!stderr.contains("PR #99"), "{stderr}");
    assert!(!stderr.contains("PRs as of"), "this run's published answer has no PR item:\n{stderr}");
    assert!(wait_for_refresh_workers(fixture.main(), 0, WORKER_WAIT).is_empty(), "its worker finished");
    assert_eq!(gitea.requests(), 2, "the next listing asks again");
}

#[test]
#[serial]
fn concurrent_lists_and_workers_make_one_request_and_the_next_worker_asks_again() {
    let (fixture, seeded) = stale_gitea_fixture();
    let gitea = FakeGitea::new(GiteaReply::Status(500));
    gitea.hold();
    let _reaper = WorkerReaper::new(&fixture, &gitea);

    let mut lists: Vec<KillOnDrop> = (0..4)
        .map(|_| {
            let mut command = held(fixture.wt_command_via_gitea(&gitea));
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

    // A worker started after that success asks again, and publishes anew.
    assert!(KillOnDrop::spawn(fixture.refresh_worker_via_gitea(&gitea)).wait().success());
    assert_eq!(gitea.requests(), 2, "a fresh answer is requested again");
    assert_ne!(stored(&fixture), refreshed, "with a new publication id");
}

#[test]
#[serial]
fn a_killed_worker_releases_its_lock_and_a_later_worker_refreshes() {
    let (fixture, seeded) = stale_gitea_fixture();
    let gitea = FakeGitea::new(GiteaReply::Status(500));
    gitea.hold();
    let _reaper = WorkerReaper::new(&fixture, &gitea);

    let mut worker = KillOnDrop::spawn(fixture.refresh_worker_via_gitea(&gitea));
    assert!(gitea.wait_for_waiting(1, WORKER_WAIT), "the worker made its request");
    assert_eq!(fixture.probe_refresh(), RefreshOutcome::Contended);

    worker.child().kill().expect("kill the worker");
    worker.child().wait().expect("reap the worker");
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
    let (fixture, seeded) = stale_gitea_fixture();

    for status in [500, 401] {
        let gitea = FakeGitea::new(GiteaReply::Status(status));
        let _reaper = WorkerReaper::new(&fixture, &gitea);

        assert!(KillOnDrop::spawn(fixture.refresh_worker_via_gitea(&gitea)).wait().success());

        assert!(gitea.requests() >= 1, "HTTP {status}: the worker asked");
        assert_eq!(stored(&fixture), seeded, "HTTP {status} is never stored, not even as no PRs");
        let (_, stderr) = list_with(fixture.wt_command_via_gitea(&gitea));
        assert!(row(&stderr, "divergent-0").contains("PR #99"), "HTTP {status}:\n{stderr}");
        // That listing's own worker failed the same way, inside its wait.
        assert!(stderr.contains("PRs as of 12 min ago (couldn't refresh)"), "HTTP {status}:\n{stderr}");
        assert!(!collapsed(&stderr).contains("running this command again"), "HTTP {status}:\n{stderr}");
        // Without a key, the PR half's 401 names no condition a key would
        // fix (the head half's 404 still asks for one).
        assert!(!stderr.contains("didn't accept"), "HTTP {status}:\n{stderr}");
        assert_eq!(stored(&fixture), seeded, "HTTP {status}: still nothing stored");
    }
}

#[test]
#[serial]
fn an_origin_change_during_a_workers_request_discards_its_answer() {
    let (fixture, seeded) = stale_gitea_fixture();
    let gitea = FakeGitea::new(GiteaReply::Status(500));
    gitea.hold();
    let _reaper = WorkerReaper::new(&fixture, &gitea);

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
    // none of it.
    let (_, stderr) = list_with(fixture.wt_command_via_gitea(&gitea));
    assert!(!stderr.contains("PR #99"), "{stderr}");
    assert!(!stderr.contains("PRs as of"), "{stderr}");
}

#[test]
#[serial]
fn an_origin_change_during_the_wait_shows_no_badges_from_the_old_origin() {
    let fixture = MixedFixture::new().with_gitea_origin();
    seed_fresh_head(&fixture);
    let gitea = FakeGitea::new(GiteaReply::Open(vec![(7, "divergent-1")]));
    let _reaper = WorkerReaper::new(&fixture, &gitea);

    // Control: with no stored answer, the worker's answer arrives within the
    // wait and is shown, so the absence below is the discarded answer.
    let (_, stderr) = list_with(fixture.wt_command_via_gitea(&gitea));
    assert!(row(&stderr, "divergent-1").contains("PR #7"), "{stderr}");
    assert!(!stderr.contains("couldn't refresh") && !stderr.contains("couldn't get open PRs"), "{stderr}");
    assert!(wait_for_refresh_workers(fixture.main(), 0, WORKER_WAIT).is_empty(), "the worker finished");
    fs::remove_file(fixture.pr_store()).expect("forget the stored answer");

    const NEW_ORIGIN: &str = "http://gitea.example.invalid/o/other.git";
    let main = fixture.main().to_path_buf();
    gitea.before_reply(move || {
        let status = Command::new("git")
            .current_dir(&main)
            .args(["remote", "set-url", "origin", NEW_ORIGIN])
            .status()
            .expect("git");
        assert!(status.success());
    });
    let (_, stderr) = list_with(fixture.wt_command_via_gitea(&gitea));
    assert!(wait_for_refresh_workers(fixture.main(), 0, WORKER_WAIT).is_empty(), "the worker finished");

    assert_eq!(gitea.requests(), 2, "one per listing; no second refresh for the new origin");
    assert!(row(&stderr, "divergent-1").contains("divergent-1"));
    assert!(!stderr.contains("PR #"), "badges from the previous origin:\n{stderr}");
    assert!(!stderr.contains("PRs as of"), "{stderr}");
    assert!(
        !stderr.contains("couldn't refresh") && !stderr.contains("couldn't get open PRs"),
        "nothing said about the old origin's refresh:\n{stderr}"
    );
    for origin in [FakeGitea::ORIGIN, NEW_ORIGIN] {
        assert_eq!(select_cached(&fixture.pr_store(), Some(origin), unix_now()), CachedPrs::Miss, "{origin}");
    }
}

#[test]
#[serial]
fn a_held_live_head_check_holds_the_listing_only_until_its_deadline() {
    let origin = HoldingOrigin::new();
    let fixture = MixedFixture::new().with_origin(&origin.url());
    let _reaper = WorkerReaper::new(&fixture, &origin);
    fixture.seed_empty_pr_store(Duration::ZERO);
    let status = Command::new("git")
        .current_dir(fixture.main())
        .args(["update-ref", "refs/remotes/origin/main", "main"])
        .status()
        .expect("git");
    assert!(status.success());

    // `.output()` returns only once every holder of stdout and stderr has
    // exited, so it returning while `origin` still holds the worker's
    // request proves `wt list` did not join it. The wait's budget is proven
    // with a scripted clock (`list::wait::tests::the_budget_ends_the_wait_
    // with_the_last_phase_seen`) and its real bound by `perf_pr_request.rs`.
    let (_, stderr) = list_with(held(fixture.wt_command_direct()));

    assert!(origin.wait_for_requests(1, WORKER_WAIT), "the worker never asked origin");
    assert!(fixture.head_lock_held(), "the request is still held");
    assert_eq!(refresh_workers(fixture.main()).len(), 1, "one worker");
    let caption = stderr.split_whitespace().collect::<Vec<_>>().join(" ");
    // No stored answer: the tracking ref's reflog dates it instead.
    assert!(
        caption.contains("origin hasn't answered yet; still checking in the background; tracking ref last changed less than 1 min ago"),
        "{caption}"
    );
    assert!(caption.contains("running this command again"), "{caption}");

    let requests = origin.requests();
    assert_eq!(requests.len(), 1, "one live request and no PR request: {requests:?}");
    assert!(
        requests[0].starts_with("GET /r.git/info/refs?service=git-upload-pack"),
        "the only request is git's: {requests:?}"
    );
    let head = read_store(&fixture.remote_head_store());
    assert_eq!(head.answer, None, "no answer stored yet");
    let attempt = head.attempt.expect("the attempt is recorded before its request");
    assert_eq!((attempt.phase, attempt.outcome), (Phase::Checking, None), "still checking");
}

#[test]
#[serial]
fn two_sequential_listings_each_make_one_pr_query_and_show_the_answer_they_waited_for() {
    let fixture = MixedFixture::new().with_gitea_origin();
    fixture.seed_pr_store(Duration::from_secs(10), 99, "divergent-0");
    seed_fresh_head(&fixture);
    let gitea = FakeGitea::new(GiteaReply::Status(503));
    let _reaper = WorkerReaper::new(&fixture, &gitea);

    // A one-page answer is one request; the second listing's store is the
    // first one's answer, seconds old.
    for (listing, (number, branch)) in [(7, "divergent-1"), (8, "divergent-2")].into_iter().enumerate() {
        gitea.release(GiteaReply::Open(vec![(number, branch)]));
        let (_, stderr) = list_with(fixture.wt_command_via_gitea(&gitea));
        assert!(wait_for_refresh_workers(fixture.main(), 0, WORKER_WAIT).is_empty(), "the worker finished");

        assert_eq!(gitea.requests(), listing + 1, "one PR query per listing, however fresh the store");
        assert!(row(&stderr, branch).contains(&format!("PR #{number}")), "this run's answer:\n{stderr}");
        assert!(!stderr.contains("PR #99"), "{stderr}");
        assert_no_pr_item_or_hint(&stderr);
    }
}

#[test]
#[serial]
fn a_failed_refresh_with_nothing_stored_says_it_couldnt_get_open_prs() {
    let fixture = MixedFixture::new().with_gitea_origin();
    seed_fresh_head(&fixture);
    let gitea = FakeGitea::new(GiteaReply::Status(500));
    let _reaper = WorkerReaper::new(&fixture, &gitea);

    let (_, stderr) = list_with(fixture.wt_command_via_gitea(&gitea));
    assert!(wait_for_refresh_workers(fixture.main(), 0, WORKER_WAIT).is_empty(), "the worker finished");

    let text = collapsed(&stderr);
    assert_eq!(gitea.requests(), 1);
    assert!(!stderr.contains("PR #"), "{stderr}");
    assert!(text.contains("- couldn't get open PRs"), "{stderr}");
    assert!(!text.contains("PRs as of") && !text.contains("running this command again"), "{stderr}");
    assert!(!fixture.pr_store().exists(), "a failure is never stored as no PRs");
}

#[test]
#[serial]
fn a_held_pr_request_with_nothing_stored_ends_at_the_budget_with_only_the_hint() {
    let fixture = MixedFixture::new().with_gitea_origin();
    seed_fresh_head(&fixture);
    let gitea = FakeGitea::new(GiteaReply::Status(503));
    gitea.hold();
    let _reaper = WorkerReaper::new(&fixture, &gitea);

    // The budget's expiry is proven with a scripted clock (`list::wait::
    // tests::the_budget_ends_the_wait_with_the_head_finished_and_the_pr_half_
    // running`) and its real bound by `perf_pr_request.rs`. This listing
    // keeps the real 3 s wait: the head half must finish inside it, and one
    // functional test should run the budget a release build uses.
    let (_, stderr) = list_with(fixture.wt_command_via_gitea(&gitea));

    assert!(gitea.wait_for_waiting(1, Duration::ZERO), "the PR request is still held");
    let text = collapsed(&stderr);
    assert!(text.contains("running this command again"), "the wait timed out:\n{stderr}");
    for item in ["PRs as of", "couldn't refresh", "couldn't get open PRs"] {
        assert!(!text.contains(item), "nothing stored, nothing failed:\n{stderr}");
    }
    assert!(!text.contains("still checking"), "the head half had finished:\n{stderr}");
    assert!(!stderr.contains("PR #"), "{stderr}");
}

#[test]
#[serial]
fn an_empty_answer_clears_the_stored_badges_and_shows_no_item() {
    let (fixture, _) = stale_gitea_fixture();
    let gitea = FakeGitea::new(GiteaReply::Open(Vec::new()));
    let _reaper = WorkerReaper::new(&fixture, &gitea);

    let (_, stderr) = list_with(fixture.wt_command_via_gitea(&gitea));
    assert!(wait_for_refresh_workers(fixture.main(), 0, WORKER_WAIT).is_empty(), "the worker finished");

    assert_eq!(gitea.requests(), 1);
    assert!(!stderr.contains("PR #"), "the empty answer replaced PR #99:\n{stderr}");
    assert_no_pr_item_or_hint(&stderr);
    match select_cached(&fixture.pr_store(), Some(FakeGitea::ORIGIN), unix_now()) {
        CachedPrs::Fresh(listing) => assert!(listing.pull_requests.is_empty(), "{listing:?}"),
        other => panic!("the empty answer is stored as an answer: {other:?}"),
    }
}

#[test]
#[serial]
fn without_an_origin_nothing_is_asked_and_no_pr_item_or_hint_shows() {
    let fixture = MixedFixture::new();
    let proxy = ProxyStub::closing_after(Duration::ZERO);

    let (_, stderr) = list(&fixture, &proxy);

    assert!(row(&stderr, "divergent-0").contains("divergent-0"));
    assert!(refresh_workers(fixture.main()).is_empty(), "no worker was launched");
    assert_eq!(proxy.connections(), 0, "no request");
    assert!(!stderr.contains("PR #"), "{stderr}");
    assert_no_pr_item_or_hint(&stderr);
}

#[test]
#[serial]
fn local_path_and_unsupported_origins_keep_the_head_check_and_show_no_pr_badges_or_item() {
    // A host no provider recognizes: its stored answer (left from when it
    // was supported, say) is not shown, and the head check still runs.
    let unsupported = MixedFixture::new().with_origin("https://git.example.invalid/o/r.git");
    unsupported.seed_pr_store(Duration::from_secs(12 * 60 + 5), 99, "divergent-0");
    let seeded = stored(&unsupported);
    let proxy = ProxyStub::closing_after(Duration::ZERO);

    let (_, stderr) = list(&unsupported, &proxy);
    finish_worker(&unsupported, &proxy);

    assert!(!stderr.contains("PR #"), "an unsupported origin shows no badges:\n{stderr}");
    assert_no_pr_item_or_hint(&stderr);
    assert_eq!(proxy.connections(), 0, "no provider request from either half");
    assert_eq!(stored(&unsupported), seeded, "nothing stored");
    let attempt = read_store(&unsupported.remote_head_store()).attempt.expect("the head check ran");
    // Git's HTTPS transport is refused here, so the check fails; it ran.
    assert_eq!(attempt.outcome, Some(Outcome::CheckFailed { reason: CheckFailure::Other }), "{attempt:?}");

    // A local path: the head check answers through Git.
    let local = MixedFixture::new().with_local_origin();
    let (_, stderr) = list_with(local.wt_command_direct());
    assert!(local.wait_until_unlocked(WORKER_WAIT, || {}), "the worker finished");

    assert!(!stderr.contains("PR #"), "{stderr}");
    assert_no_pr_item_or_hint(&stderr);
    assert!(!local.pr_store().exists(), "nothing stored for a local path");
    let answer = read_store(&local.remote_head_store()).answer.expect("the head check answered");
    assert_eq!(answer.source, AnswerSource::Git, "{answer:?}");
}

/// A key the provider rejects on the PR request is named in an ordinary
/// listing, never its value, and the PR item does not repeat the reason. The
/// head half's 404 with a key is ambiguous, so it asserts nothing. When
/// `origin` changes during the wait, the old origin's rejection is not shown.
#[test]
#[serial]
fn a_rejected_key_on_the_pr_request_shows_the_credentials_line_in_an_ordinary_listing() {
    const SECRET: &str = "secret-token-value";
    let (fixture, seeded) = stale_gitea_fixture();
    let gitea = FakeGitea::new(GiteaReply::Status(401));
    let _reaper = WorkerReaper::new(&fixture, &gitea);
    let listing = || {
        let mut command = fixture.wt_command_via_gitea(&gitea);
        command.env("GITEA_TOKEN", SECRET);
        let (_, stderr) = list_with(command);
        assert!(wait_for_refresh_workers(fixture.main(), 0, WORKER_WAIT).is_empty(), "the worker finished");
        stderr
    };

    let stderr = listing();
    let text = collapsed(&stderr);
    assert!(
        text.contains("Gitea didn't accept GITEA_TOKEN; it may be invalid, expired, or revoked."),
        "{stderr}"
    );
    assert!(!stderr.contains(SECRET), "the key's value is never printed:\n{stderr}");
    let item = stderr.lines().find(|line| line.contains("PRs as of")).expect("the PR item");
    assert_eq!(item.trim(), "- PRs as of 12 min ago (couldn't refresh)", "the item gives no reason:\n{stderr}");
    assert!(!text.contains("did not show this repository"), "an ambiguous 404 asserts nothing:\n{stderr}");
    assert_eq!(stored(&fixture), seeded);

    let main = fixture.main().to_path_buf();
    gitea.before_reply(move || {
        let status = Command::new("git")
            .current_dir(&main)
            .args(["remote", "set-url", "origin", "http://gitea.example.invalid/o/other.git"])
            .status()
            .expect("git");
        assert!(status.success());
    });
    let stderr = listing();
    assert_eq!(gitea.requests(), 2);
    assert!(!stderr.contains("didn't accept"), "the old origin's rejection is not shown:\n{stderr}");
    assert!(!stderr.contains("PR #"), "{stderr}");
    assert_no_pr_item_or_hint(&stderr);
}

/// A confirmed condition from the head half outranks the PR half's.
#[test]
#[serial]
fn a_rejected_key_on_the_head_check_outranks_the_pr_requests_rate_limit() {
    let (fixture, _) = stale_gitea_fixture();
    let gitea = FakeGitea::new(GiteaReply::Status(429));
    gitea.answer_branch_heads_with(401);
    let _reaper = WorkerReaper::new(&fixture, &gitea);

    let mut command = fixture.wt_command_via_gitea(&gitea);
    command.env("GITEA_TOKEN", "secret-token-value");
    let (_, stderr) = list_with(command);
    assert!(wait_for_refresh_workers(fixture.main(), 0, WORKER_WAIT).is_empty(), "the worker finished");

    let text = collapsed(&stderr);
    assert!(text.contains("Gitea didn't accept GITEA_TOKEN"), "{stderr}");
    assert!(!text.contains("rate limited"), "one line, the head half's:\n{stderr}");
    assert!(text.contains("PRs as of 12 min ago (couldn't refresh)"), "{stderr}");
}

/// `--ff` keeps its forced wait and its local rules when the PR half fails:
/// the fetched tip is fast-forwarded to, and the failure is the PR item.
#[test]
#[serial]
fn a_pr_failure_never_blocks_a_permitted_fast_forward() {
    let fixture = MixedFixture::new().with_gitea_origin();
    let gitea = FakeGitea::new(GiteaReply::Status(500));
    let _reaper = WorkerReaper::new(&fixture, &gitea);
    let pushed = fixture.serve_gitea_origin_one_commit_ahead(&gitea);

    let output = fixture.wt_command_via_gitea_git(&gitea).arg("--ff").env("NO_COLOR", "1").output().expect("wt --ff");
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert!(output.status.success(), "{stderr}");
    assert!(wait_for_refresh_workers(fixture.main(), 0, WORKER_WAIT).is_empty(), "the worker finished");

    let main = Command::new("git").current_dir(fixture.main()).args(["rev-parse", "main"]).output().expect("git");
    assert_eq!(String::from_utf8_lossy(&main.stdout).trim(), pushed, "main moved to the fetched tip:\n{stderr}");
    let text = collapsed(&stderr);
    assert!(text.contains("main is in sync with origin/main (updated from origin just now)"), "{stderr}");
    assert!(text.contains("- couldn't get open PRs"), "{stderr}");
    assert!(!text.contains("running this command again"), "{stderr}");
    assert_eq!(gitea.requests(), 1);
}

// Teardown guards: a failed assertion while a worker is held still ends the
// worker and frees both locks before the fixture's directories go, and the
// failure reaches the test harness unchanged.

/// The fixture's own `Drop`, after the server declared after it ends the
/// held PR request.
#[test]
#[serial]
fn a_failed_assertion_while_a_pr_request_is_held_still_reaps_the_worker_before_the_fixture_goes() {
    let mut workers = Vec::new();
    let mut leftovers = Vec::new();
    let unwound = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let (fixture, _) = stale_gitea_fixture();
        let gitea = FakeGitea::new(GiteaReply::Status(503));
        gitea.hold();
        list_with(held(fixture.wt_command_via_gitea(&gitea)));
        assert!(gitea.wait_for_waiting(1, WORKER_WAIT), "the worker's PR request is held");
        workers = refresh_workers(fixture.main()).iter().map(|worker| worker.pid).collect();
        let (pr_store, head_store) = (fixture.pr_store(), fixture.remote_head_store());
        leftovers = vec![fixture.main().to_path_buf(), pr_lock_path(&pr_store), remote_head_lock_path(&head_store)];
        leftovers.extend([pr_store, head_store]);
        // Dropping `gitea` releases the request, but it is answered only once
        // the fixture's teardown is waiting for the worker, so the worker is
        // still running when teardown starts: only a teardown that waits for
        // it can pass. One that does not never answers it, and the worker
        // outlives the fixture.
        let waiting = Arc::new((Mutex::new(false), Condvar::new()));
        fixture.on_teardown_wait({
            let waiting = Arc::clone(&waiting);
            move || {
                *waiting.0.lock().expect("teardown flag") = true;
                waiting.1.notify_all();
            }
        });
        gitea.before_reply(move || {
            let flag = waiting.0.lock().expect("teardown flag");
            let _ = waiting.1.wait_timeout_while(flag, WORKER_WAIT, |waiting| !*waiting);
        });
        panic!("{MANUFACTURED_FAILURE}");
    }));

    assert_manufactured_failure(unwound);
    assert_eq!(workers.len(), 1, "the listing returned with its worker held");
    for pid in workers {
        assert!(!process_running(pid), "worker {pid} outlived the fixture");
    }
    for path in leftovers {
        assert!(!path.exists(), "{path:?} outlived the fixture");
    }
}

/// [`WorkerReaper`] for a stand-in declared before the fixture: the held
/// live-head check is closed and the worker gone, with the fixture still
/// alive to show both locks free.
#[test]
#[serial]
fn a_failed_assertion_while_a_live_head_check_is_held_still_frees_both_locks() {
    let origin = HoldingOrigin::new();
    let fixture = MixedFixture::new().with_origin(&origin.url());
    fixture.seed_empty_pr_store(Duration::ZERO);
    let mut workers = Vec::new();
    let unwound = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _reaper = WorkerReaper::new(&fixture, &origin);
        list_with(held(fixture.wt_command_direct()));
        assert!(origin.wait_for_requests(1, WORKER_WAIT), "the worker's check reached origin");
        assert!(fixture.head_lock_held(), "the check is held with its lock");
        workers = refresh_workers(fixture.main()).iter().map(|worker| worker.pid).collect();
        panic!("{MANUFACTURED_FAILURE}");
    }));

    assert_manufactured_failure(unwound);
    assert_eq!(workers.len(), 1, "the listing returned with its worker held");
    assert!(refresh_workers(fixture.main()).is_empty(), "the worker was reaped");
    assert!(!fixture.head_lock_held(), "the live-head lock is free");
    assert_ne!(fixture.probe_refresh(), RefreshOutcome::Contended, "the PR lock is free");
}

/// [`KillOnDrop`] and [`WorkerReaper`] together, as in a test that starts a
/// worker itself: the child is killed and reaped, and the held request is
/// ended, before the fixture goes.
#[test]
#[serial]
fn a_failed_assertion_kills_a_test_owned_worker_and_frees_its_lock() {
    let (fixture, _) = stale_gitea_fixture();
    let gitea = FakeGitea::new(GiteaReply::Status(503));
    gitea.hold();
    let mut holder = None;
    let unwound = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _reaper = WorkerReaper::new(&fixture, &gitea);
        let mut worker = KillOnDrop::spawn(fixture.refresh_worker_via_gitea(&gitea));
        assert!(gitea.wait_for_waiting(1, WORKER_WAIT), "the worker's PR request is held");
        holder = Some(worker.child().id());
        panic!("{MANUFACTURED_FAILURE}");
    }));

    assert_manufactured_failure(unwound);
    let holder = holder.expect("the worker started");
    assert!(!process_running(holder), "the test-owned worker was killed and reaped");
    assert!(refresh_workers(fixture.main()).is_empty(), "no worker is left");
    assert_ne!(fixture.probe_refresh(), RefreshOutcome::Contended, "the PR lock is free");
    assert!(!fixture.head_lock_held(), "the live-head lock is free");
}


// The keyless notice through the shipped binary: one dim line beneath the
// caption whenever this listing saw an API answer sent without a key, in
// either half, and a generic failure in the other half never hides it.

const GITEA_KEYLESS: &str = "Gitea answered without an API key; set GITEA_TOKEN or FORGEJO_TOKEN or CODEBERG_TOKEN to authenticate API requests.";

/// A listing through `command` whose worker is waited for, so the stores it
/// leaves are final.
fn keyless_listing(fixture: &MixedFixture, command: Command) -> String {
    let (_, stderr) = list_with(command);
    assert!(wait_for_refresh_workers(fixture.main(), 0, WORKER_WAIT).is_empty(), "the worker finished");
    stderr
}

fn keyless_lines(stderr: &str) -> usize {
    collapsed(stderr).matches("answered without an API key").count()
}

#[test]
#[serial]
fn an_anonymous_pr_answer_with_a_failed_head_check_shows_the_keyless_notice() {
    let fixture = MixedFixture::new().with_gitea_origin();
    let gitea = FakeGitea::new(GiteaReply::Open(Vec::new()));
    gitea.answer_branch_heads_with(500);
    let _reaper = WorkerReaper::new(&fixture, &gitea);

    let stderr = keyless_listing(&fixture, fixture.wt_command_via_gitea(&gitea));

    assert_eq!((gitea.requests(), gitea.branch_requests()), (1, 1), "no extra request");
    assert!(collapsed(&stderr).contains(GITEA_KEYLESS), "{stderr}");
    assert_eq!(keyless_lines(&stderr), 1, "{stderr}");
}

#[test]
#[serial]
fn an_anonymous_head_answer_with_a_failed_pr_request_shows_the_keyless_notice_through_the_fetch() {
    let fixture = MixedFixture::new().with_gitea_origin();
    let gitea = FakeGitea::new(GiteaReply::Status(500));
    let _reaper = WorkerReaper::new(&fixture, &gitea);
    let pushed = fixture.serve_gitea_origin_one_commit_ahead(&gitea);
    gitea.answer_branch_heads_at(&pushed);

    let stderr = keyless_listing(&fixture, fixture.wt_command_via_gitea_git(&gitea));

    assert_eq!((gitea.requests(), gitea.branch_requests()), (1, 1), "no extra request");
    assert!(collapsed(&stderr).contains(GITEA_KEYLESS), "{stderr}");
    assert_eq!(keyless_lines(&stderr), 1, "{stderr}");
    let attempt = read_store(&fixture.remote_head_store()).attempt.expect("the attempt");
    assert_eq!(attempt.phase, Phase::Fetching, "the API answer moved on to the fetch");
    assert_eq!(attempt.credentials, worktree::remote_head::CredentialEvidence::Anonymous);
}

#[test]
#[serial]
fn two_anonymous_answers_show_exactly_one_keyless_line_and_a_key_shows_none() {
    const SECRET: &str = "gitea-secret-token-value";
    let fixture = MixedFixture::new().with_gitea_origin();
    let gitea = FakeGitea::new(GiteaReply::Open(Vec::new()));
    let _reaper = WorkerReaper::new(&fixture, &gitea);
    let pushed = fixture.serve_gitea_origin_one_commit_ahead(&gitea);
    gitea.answer_branch_heads_at(&pushed);

    let stderr = keyless_listing(&fixture, fixture.wt_command_via_gitea_git(&gitea));

    assert_eq!((gitea.requests(), gitea.branch_requests()), (1, 1), "no extra request");
    assert_eq!(keyless_lines(&stderr), 1, "{stderr}");
    let lines: Vec<&str> = stderr.lines().collect();
    let legend = lines.iter().position(|line| line.contains("parent deleted")).expect("the legend");
    let notice = lines.iter().position(|line| line.contains("Gitea answered without")).expect("the notice");
    assert!(notice > legend && lines[notice].trim_start().starts_with("- Gitea answered without"), "a closing note:\n{stderr}");

    // The same answers sent with a key: no notice, and the key's value is
    // in neither store nor the output.
    let mut command = fixture.wt_command_via_gitea_git(&gitea);
    command.env("GITEA_TOKEN", SECRET);
    let stderr = keyless_listing(&fixture, command);
    assert_eq!(keyless_lines(&stderr), 0, "{stderr}");
    assert!(!stderr.contains(SECRET), "{stderr}");
    for store in [fixture.pr_store(), fixture.remote_head_store()] {
        let text = fs::read_to_string(&store).expect("store");
        assert!(!text.contains(SECRET), "{}: {text}", store.display());
        assert!(text.contains("GITEA_TOKEN"), "the variable's name is recorded: {text}");
    }
}

/// A keyless head check that fell back to an answering `ls-remote` keeps its
/// closing notice, and the anonymous PR answer adds the keyless line.
#[test]
#[serial]
fn the_keyless_notice_coexists_with_the_closing_fallback_notice() {
    let fixture = MixedFixture::new().with_gitea_origin();
    let gitea = FakeGitea::new(GiteaReply::Open(Vec::new()));
    gitea.answer_branch_heads_with(401);
    let _reaper = WorkerReaper::new(&fixture, &gitea);
    fixture.serve_gitea_origin_one_commit_ahead(&gitea);

    let stderr = keyless_listing(&fixture, fixture.wt_command_via_gitea_git(&gitea));

    let text = collapsed(&stderr);
    assert!(text.contains(GITEA_KEYLESS), "{stderr}");
    assert!(text.contains("Git checked origin using `ls-remote`"), "{stderr}");
    assert_eq!(keyless_lines(&stderr), 1, "{stderr}");
}

/// Anonymous evidence an earlier listing stored is never this listing's: a
/// later listing whose own requests fail shows no notice while the store
/// still holds the earlier anonymous publication, and an ignored repository
/// shows none either.
#[test]
#[serial]
fn stored_anonymous_evidence_from_an_earlier_listing_never_gives_the_notice() {
    let fixture = MixedFixture::new().with_gitea_origin();
    let gitea = FakeGitea::new(GiteaReply::Open(Vec::new()));
    let _reaper = WorkerReaper::new(&fixture, &gitea);
    let pushed = fixture.serve_gitea_origin_one_commit_ahead(&gitea);
    gitea.answer_branch_heads_at(&pushed);

    // Control: this listing's own anonymous answers give the notice.
    let stderr = keyless_listing(&fixture, fixture.wt_command_via_gitea_git(&gitea));
    assert_eq!(keyless_lines(&stderr), 1, "{stderr}");

    // Both halves fail now; only the earlier listing's evidence is anonymous.
    gitea.release(GiteaReply::Status(500));
    gitea.answer_branch_heads_with(500);
    let requests = (gitea.requests(), gitea.branch_requests());
    let stderr = keyless_listing(&fixture, fixture.wt_command_via_gitea_git(&gitea));
    assert_eq!(keyless_lines(&stderr), 0, "{stderr}");
    assert!(
        gitea.requests() > requests.0 && gitea.branch_requests() > requests.1,
        "this listing asked both halves again: {requests:?}"
    );
    let stored: serde_json::Value =
        serde_json::from_slice(&fs::read(fixture.pr_store()).expect("PR store")).expect("json");
    assert_eq!(stored["credentials"]["state"], "anonymous", "the earlier publication is still stored: {stored}");

    #[cfg(unix)]
    {
        // Ignored: nothing is asked and the stored evidence stays unread.
        gitea.release(GiteaReply::Open(Vec::new()));
        gitea.answer_branch_heads_at(&pushed);
        let requests = (gitea.requests(), gitea.branch_requests());
        let mut command = fixture.wt_command_via_gitea_git(&gitea);
        command.arg("--ignore-api");
        let stderr = keyless_listing(&fixture, command);
        assert_eq!(keyless_lines(&stderr), 0, "{stderr}");
        assert_eq!((gitea.requests(), gitea.branch_requests()), requests, "no API request");
    }
}

// A changed `origin` drops every notice about the old one's requests, the
// head check's included: the PR reply is held until this listing's own head
// attempt has finished, and only then is `origin` replaced or removed, so the
// head evidence the listing reads is complete and current-attempt.

/// What happens to `origin` at [`change_origin_at_checkpoint`].
#[derive(Clone, Copy, Debug)]
enum OriginChange {
    Unchanged,
    Replaced,
    Removed,
}

const REPLACEMENT_ORIGIN: &str = "http://gitea.example.invalid/o/other.git";

/// How long the checkpoint waits for the head outcome: the listing's own
/// wait, after which nothing the worker does can reach this listing.
const CHECKPOINT_WAIT: Duration = Duration::from_secs(3);

/// Holds `gitea`'s next PR reply until the head attempt of the listing about
/// to run (an attempt ID other than the one stored now) has an outcome, then
/// applies `change` before the reply goes out. The flag reports whether the
/// checkpoint was reached; a checkpoint missed leaves `origin` unchanged.
fn change_origin_at_checkpoint(fixture: &MixedFixture, gitea: &FakeGitea, change: OriginChange) -> Arc<AtomicBool> {
    let head_store = fixture.remote_head_store();
    let previous = read_store(&head_store).attempt.map(|attempt| attempt.id);
    let main = fixture.main().to_path_buf();
    let reached = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&reached);
    gitea.before_reply(move || {
        let deadline = Instant::now() + CHECKPOINT_WAIT;
        let finished = || {
            read_store(&head_store)
                .attempt
                .is_some_and(|attempt| Some(&attempt.id) != previous.as_ref() && attempt.outcome.is_some())
        };
        while !finished() {
            if Instant::now() >= deadline {
                return;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let args: &[&str] = match change {
            OriginChange::Unchanged => &[],
            OriginChange::Replaced => &["remote", "set-url", "origin", REPLACEMENT_ORIGIN],
            OriginChange::Removed => &["remote", "remove", "origin"],
        };
        if !args.is_empty() {
            let status = Command::new("git").current_dir(&main).args(args).status().expect("git");
            assert!(status.success(), "{change:?}");
        }
        flag.store(true, Ordering::SeqCst);
    });
    reached
}

/// One listing through `command` with `change` applied at the checkpoint;
/// its worker is waited for. Fails unless the checkpoint was reached inside
/// the listing's wait.
fn listing_with_origin_change(fixture: &MixedFixture, gitea: &FakeGitea, command: Command, change: OriginChange) -> String {
    let reached = change_origin_at_checkpoint(fixture, gitea, change);
    let stderr = keyless_listing(fixture, command);
    gitea.before_reply(|| {});
    assert!(reached.load(Ordering::SeqCst), "{change:?}: the head attempt never finished:\n{stderr}");
    assert!(!collapsed(&stderr).contains("running this command again"), "{change:?}: the wait saw both halves:\n{stderr}");
    stderr
}

#[test]
#[serial]
fn a_changed_origin_drops_the_old_head_checks_credentials_warning() {
    const WARNING: &str = "Gitea didn't accept GITEA_TOKEN";
    let fixture = MixedFixture::new().with_gitea_origin();
    let gitea = FakeGitea::new(GiteaReply::Open(Vec::new()));
    gitea.answer_branch_heads_with(401);
    let _reaper = WorkerReaper::new(&fixture, &gitea);
    let command = || {
        let mut command = fixture.wt_command_via_gitea(&gitea);
        command.env("GITEA_TOKEN", "secret-token-value");
        command
    };

    // Control: the same checkpoint with `origin` unchanged shows the warning.
    let stderr = listing_with_origin_change(&fixture, &gitea, command(), OriginChange::Unchanged);
    assert!(collapsed(&stderr).contains(WARNING), "{stderr}");

    for change in [OriginChange::Replaced, OriginChange::Removed] {
        let branch_requests = gitea.branch_requests();
        let stderr = listing_with_origin_change(&fixture, &gitea, command(), change);
        assert!(gitea.branch_requests() > branch_requests, "{change:?}: this listing checked the head again");
        let text = collapsed(&stderr);
        assert!(!text.contains(WARNING), "{change:?}: the old origin's warning is shown:\n{stderr}");
        assert!(!text.contains("rate limited") && !text.contains("answered without"), "{change:?}:\n{stderr}");
        if let OriginChange::Replaced = change {
            run_git_in(fixture.main(), &["remote", "set-url", "origin", FakeGitea::ORIGIN]);
        }
    }
}

#[test]
#[serial]
fn a_changed_origin_drops_the_old_head_checks_fallback_notice_and_caption() {
    const FALLBACK: &str = "Git checked origin using `ls-remote`";
    let fixture = MixedFixture::new().with_gitea_origin();
    let gitea = FakeGitea::new(GiteaReply::Open(Vec::new()));
    gitea.answer_branch_heads_with(401);
    let _reaper = WorkerReaper::new(&fixture, &gitea);
    fixture.serve_gitea_origin_one_commit_ahead(&gitea);

    // Control: an anonymous 401 whose `ls-remote` answered gives the closing
    // notice, and the caption reports the fetch it led to.
    let stderr =
        listing_with_origin_change(&fixture, &gitea, fixture.wt_command_via_gitea_git(&gitea), OriginChange::Unchanged);
    let text = collapsed(&stderr);
    assert!(text.contains(FALLBACK), "{stderr}");
    assert!(text.contains("(updated from origin just now)"), "{stderr}");

    for change in [OriginChange::Replaced, OriginChange::Removed] {
        let git_requests = gitea.git_requests();
        let stderr = listing_with_origin_change(&fixture, &gitea, fixture.wt_command_via_gitea_git(&gitea), change);
        assert!(gitea.git_requests() > git_requests, "{change:?}: this listing's ls-remote answered again");
        let text = collapsed(&stderr);
        assert!(!text.contains(FALLBACK), "{change:?}: the old origin's fallback notice is shown:\n{stderr}");
        assert!(!text.contains("answered without"), "{change:?}:\n{stderr}");
        assert!(!text.contains("origin just now"), "{change:?}: the caption reports the old check:\n{stderr}");
        assert!(text.contains("couldn't check origin;"), "{change:?}: the caption reads as a check not made:\n{stderr}");
        if let OriginChange::Replaced = change {
            run_git_in(fixture.main(), &["remote", "set-url", "origin", FakeGitea::ORIGIN]);
        }
    }
}

fn run_git_in(repo: &std::path::Path, args: &[&str]) {
    let status = Command::new("git").current_dir(repo).args(args).status().expect("git");
    assert!(status.success(), "git {args:?}");
}
