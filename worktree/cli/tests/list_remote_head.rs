//! The caption's remote observation against real git: a local bare `origin`,
//! a `pusher` clone standing in for everyone else, and the listed clone with
//! one linked worktree.
//!
//! The live head is recorded by `wt internal-refresh <main>`, run here as a
//! direct child and waited for, so each step's store is known before `wt
//! list` reads it. The worker fetches `origin/main` when its check finds it
//! differs. `wt list` itself never asks `origin`: a manual fetch changes the
//! comparison and leaves the stored observation (and its age) as it was.

mod perf_support;

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::Duration;

use assert_cmd::cargo::cargo_bin;
use perf_support::{isolated_cache_file, refresh_workers, wait_for_refresh_workers};
use serial_test::serial;
use worktree::pull_requests::{pr_lock_path, pr_store_path};
use worktree::remote_head::{refresh_receipt_path, remote_head_lock_path, remote_head_store_path};

/// How long a test waits for a detached worker before failing.
const WORKER_WAIT: Duration = Duration::from_secs(20);

struct Fixture {
    root: tempfile::TempDir,
    main: PathBuf,
    linked: PathBuf,
    pusher: PathBuf,
    bare: PathBuf,
}

impl Fixture {
    /// `origin` holds one commit on `main`, which the listed clone has
    /// fetched: its `main` is in sync with `origin/main`.
    fn new() -> Self {
        let root = tempfile::tempdir().expect("create temp dir");
        for dir in ["home", "cache"] {
            fs::create_dir(root.path().join(dir)).expect("create dir");
        }
        fs::write(root.path().join("empty.gitconfig"), "").expect("write empty git config");
        let bare = root.path().join("origin.git");
        let main = root.path().join("main");
        let pusher = root.path().join("pusher");
        let linked = root.path().join("main-feature");
        let fixture = Self { main, linked, pusher, bare, root };

        fixture.git(fixture.root.path(), &["init", "--bare", "-b", "main", "origin.git"]);
        fixture.git(fixture.root.path(), &["init", "-b", "main", "pusher"]);
        fixture.git(&fixture.pusher, &["remote", "add", "origin", fixture.bare.to_str().unwrap()]);
        fixture.commit_and_push("first");
        fixture.git(fixture.root.path(), &["clone", fixture.bare.to_str().unwrap(), "main"]);
        fixture.git(&fixture.main, &["worktree", "add", "-b", "feature", fixture.linked.to_str().unwrap()]);
        fixture
    }

    /// Git with no user or system configuration and a fixed identity.
    fn git(&self, dir: &Path, args: &[&str]) -> String {
        let output = self.isolated(Command::new("git")).current_dir(dir).args(args).output().expect("git runs");
        assert!(output.status.success(), "git {args:?} in {dir:?}: {output:?}");
        String::from_utf8(output.stdout).expect("utf-8").trim().to_string()
    }

    fn isolated(&self, mut command: Command) -> Command {
        command
            .env("HOME", self.root.path().join("home"))
            .env("XDG_CACHE_HOME", self.root.path().join("cache"))
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", self.root.path().join("empty.gitconfig"))
            .env("GIT_AUTHOR_NAME", "Test User")
            .env("GIT_AUTHOR_EMAIL", "test@example.com")
            .env("GIT_COMMITTER_NAME", "Test User")
            .env("GIT_COMMITTER_EMAIL", "test@example.com")
            .env_remove("GIT_DIR")
            .env_remove("GIT_WORK_TREE");
        command
    }

    /// Another clone advances `origin`'s `main` by one commit; returns it.
    fn commit_and_push(&self, message: &str) -> String {
        fs::write(self.pusher.join("file.txt"), format!("{message}\n")).expect("write");
        self.git(&self.pusher, &["add", "."]);
        self.git(&self.pusher, &["commit", "-m", message]);
        self.git(&self.pusher, &["push", "origin", "main"]);
        self.git(&self.pusher, &["rev-parse", "HEAD"])
    }

    fn wt(&self, dir: &Path) -> Command {
        let mut command = self.isolated(Command::new(cargo_bin("wt")));
        command.current_dir(dir).env("NO_COLOR", "1").env_remove("WT_SHELL_WRAPPER");
        command
    }

    /// `wt list` from `dir`, with its stderr's whitespace collapsed (the
    /// caption word-wraps) and the padding that follows a plain branch badge
    /// dropped before punctuation.
    fn list_from(&self, dir: &Path) -> String {
        let output = self.wt(dir).arg("list").output().expect("wt list runs");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(output.status.success(), "wt list failed:\n{stderr}");
        stderr.split_whitespace().collect::<Vec<_>>().join(" ").replace(" .", ".").replace(" ;", ";")
    }

    fn list(&self) -> String {
        self.list_from(&self.main)
    }

    /// `wt internal-refresh <main>` as a direct child, waited for.
    fn refresh(&self) {
        self.run_worker(&[]);
    }

    /// `wt internal-refresh <main> <args>` as a direct child, waited for.
    fn run_worker(&self, args: &[&str]) {
        let output: Output = self
            .wt(&self.main)
            .arg("internal-refresh")
            .arg(&self.main)
            .args(args)
            .output()
            .expect("the worker runs");
        assert!(output.status.success() && output.stdout.is_empty() && output.stderr.is_empty(), "{output:?}");
    }

    fn head_store(&self) -> PathBuf {
        self.cache_file(remote_head_store_path(&self.main).expect("remote-head store path"))
    }

    fn cache_file(&self, real: PathBuf) -> PathBuf {
        isolated_cache_file(&self.root.path().join("home"), &self.root.path().join("cache"), &real)
    }

    fn stored_document(&self) -> serde_json::Value {
        serde_json::from_slice(&fs::read(self.head_store()).expect("a stored live head")).expect("json")
    }

    /// The store's `answer` half.
    fn stored_head(&self) -> serde_json::Value {
        self.stored_document()["answer"].clone()
    }

    /// Moves the stored answer's `checked_at` back by `seconds`, as if it had
    /// been recorded that long ago; a refresh only asks once it is stale.
    fn age_head(&self, seconds: u64) {
        let mut stored = self.stored_document();
        let checked_at = stored["answer"]["checked_at"].as_u64().expect("checked_at");
        stored["answer"]["checked_at"] = (checked_at - seconds).into();
        fs::write(self.head_store(), serde_json::to_vec(&stored).expect("json")).expect("write store");
    }
}

impl Drop for Fixture {
    /// On Windows the stores live in the real user cache, keyed by this
    /// temporary repository; remove them with it.
    fn drop(&mut self) {
        let _ = wait_for_refresh_workers(&self.main, 0, WORKER_WAIT);
        let pr_store = self.cache_file(pr_store_path(&self.main).expect("PR store path"));
        let head_store = self.head_store();
        for path in [pr_lock_path(&pr_store), pr_store, remote_head_lock_path(&head_store), head_store] {
            let _ = fs::remove_file(path);
        }
    }
}

#[test]
#[serial]
fn a_push_elsewhere_is_fetched_by_the_worker_and_then_reads_as_behind_and_matched() {
    let fixture = Fixture::new();
    let local = fixture.git(&fixture.main, &["rev-parse", "main"]);
    let pushed = fixture.commit_and_push("second");

    fixture.refresh();

    // The check found the variance and fetched exactly the tracking ref.
    assert_eq!(fixture.git(&fixture.main, &["rev-parse", "origin/main"]), pushed);
    assert_eq!(fixture.git(&fixture.main, &["rev-parse", "main"]), local, "the local branch did not move");
    assert!(!fixture.main.join(".git").join("FETCH_HEAD").exists(), "no FETCH_HEAD");
    assert_eq!(fixture.stored_head()["sha"], pushed, "the fetched tip is the answer");
    assert_eq!(fixture.stored_head()["source"], "fetch");
    assert_eq!(fixture.stored_document()["attempt"]["outcome"]["kind"], "fetched");

    // The answer is fresh, so this list starts no worker: it only reads.
    let stored = fs::read(fixture.head_store()).expect("store");
    let caption = fixture.list();
    assert!(caption.contains("main is 1 commit behind local tracking ref origin/main."), "{caption}");
    assert!(caption.contains("origin/main matched the remote when checked less than 1 min ago."), "{caption}");
    assert_eq!(fs::read(fixture.head_store()).expect("store"), stored, "listing asked origin nothing");
    assert!(refresh_workers(&fixture.main).is_empty(), "a fresh answer starts no worker");
}

#[test]
#[serial]
fn an_in_sync_check_fetches_nothing() {
    let fixture = Fixture::new();
    let refs = fixture.git(&fixture.main, &["for-each-ref"]);

    fixture.refresh();

    assert_eq!(fixture.git(&fixture.main, &["for-each-ref"]), refs, "no ref moved");
    assert_eq!(fixture.stored_head()["source"], "git", "a local origin is checked by ls-remote");
    let attempt = &fixture.stored_document()["attempt"];
    assert_eq!(attempt["outcome"]["kind"], "in-sync");
    assert_eq!(attempt["phase"]["kind"], "checking", "a local origin is no fallback");
}

#[test]
#[serial]
fn a_forced_worker_records_the_given_attempt_and_a_receipt_for_both_halves() {
    const ID: &str = "00112233445566778899aabbccddeeff";
    let fixture = Fixture::new();
    let pushed = fixture.commit_and_push("second");
    let receipt_path = fixture.cache_file(refresh_receipt_path(&fixture.main).expect("receipt path"));

    // Unforced: the attempt runs under the given id, and no receipt is written.
    fixture.run_worker(&["--attempt", ID]);
    assert_eq!(fixture.stored_document()["attempt"]["id"], ID);
    assert!(!receipt_path.exists(), "only a forced run writes a receipt");

    fixture.run_worker(&["--attempt", ID, "--force"]);
    let receipt: serde_json::Value =
        serde_json::from_slice(&fs::read(&receipt_path).expect("a receipt")).expect("json");
    assert_eq!(receipt["attempt_id"], ID);
    assert_eq!(receipt["branch"], "main");
    assert_eq!(receipt["head"], "ok", "{receipt}");
    // A local origin is no provider, so the PR half fails as `other`.
    assert_eq!(receipt["prs"], serde_json::json!({ "kind": "failed", "failure": { "kind": "other" } }), "{receipt}");
    assert_eq!(fixture.git(&fixture.main, &["rev-parse", "origin/main"]), pushed);
    let _ = fs::remove_file(receipt_path);
}

#[test]
#[serial]
fn a_fetch_newer_than_the_observation_is_a_difference_never_a_move() {
    let fixture = Fixture::new();
    fixture.refresh();
    let observed = fixture.stored_head()["sha"].clone();
    assert_eq!(observed, fixture.git(&fixture.main, &["rev-parse", "origin/main"]));

    fixture.commit_and_push("second");
    fixture.git(&fixture.main, &["fetch", "origin"]);
    let caption = fixture.list();

    assert!(caption.contains("main is 1 commit behind local tracking ref origin/main."), "{caption}");
    assert!(caption.contains("origin/main differs from the remote head observed"), "{caption}");
    for claim in ["moved", "advanced"] {
        assert!(!caption.contains(claim), "the check predates the fetch; nothing {claim}: {caption}");
    }
    assert_eq!(fixture.stored_head()["sha"], observed, "the older observation is kept as it was");
}

#[test]
#[serial]
fn a_deleted_then_recreated_remote_branch_is_reported_absent_then_present() {
    let fixture = Fixture::new();
    let tip = fixture.git(&fixture.bare, &["rev-parse", "main"]);

    fixture.git(&fixture.bare, &["update-ref", "-d", "refs/heads/main"]);
    fixture.refresh();
    assert_eq!(fixture.stored_head()["sha"], serde_json::Value::Null, "a verified absence");
    let caption = fixture.list();
    assert!(caption.contains("main is in sync with local tracking ref origin/main."), "{caption}");
    assert!(caption.contains("main was absent on origin when checked less than 1 min ago."), "{caption}");

    fixture.git(&fixture.main, &["fetch", "--prune", "origin"]);
    let caption = fixture.list();
    assert!(
        caption.contains("No local tracking ref origin/main; the remote branch was absent when checked"),
        "{caption}"
    );
    assert!(!caption.contains("in sync with"), "no tracking ref, no comparison: {caption}");

    // Recreated on origin: the worker sees it and fetches the tracking ref
    // back.
    fixture.git(&fixture.bare, &["update-ref", "refs/heads/main", &tip]);
    fixture.age_head(2 * 60);
    fixture.refresh();
    assert_eq!(fixture.stored_head()["sha"], tip.as_str(), "present again");
    assert_eq!(fixture.git(&fixture.main, &["rev-parse", "origin/main"]), tip, "fetched back");
    let caption = fixture.list();
    assert!(caption.contains("main is in sync with local tracking ref origin/main."), "{caption}");
    assert!(caption.contains("origin/main matched the remote when checked"), "{caption}");
}

#[test]
#[serial]
fn the_main_checkout_and_a_linked_worktree_share_one_live_head_store() {
    let fixture = Fixture::new();

    // From the linked worktree, `wt list` launches the worker for the main
    // checkout, which records into the main checkout's store.
    let caption = fixture.list_from(&fixture.linked);
    assert!(caption.contains("Remote state has not been verified."), "{caption}");
    assert!(wait_for_refresh_workers(&fixture.main, 0, WORKER_WAIT).is_empty(), "the worker finished");
    let tip = fixture.git(&fixture.main, &["rev-parse", "origin/main"]);
    assert_eq!(fixture.stored_head()["sha"], tip.as_str(), "stored at the main checkout's path");

    // Both read that one answer.
    for dir in [&fixture.main, &fixture.linked] {
        let caption = fixture.list_from(dir);
        assert!(caption.contains("origin/main matched the remote when checked"), "{dir:?}: {caption}");
    }
    assert!(refresh_workers(&fixture.main).is_empty(), "a fresh answer starts no worker");
}

#[test]
#[serial]
fn without_an_origin_leftover_tracking_refs_show_no_caption_and_start_no_worker() {
    let fixture = Fixture::new();
    let tip = fixture.git(&fixture.main, &["rev-parse", "origin/main"]);
    fixture.git(&fixture.main, &["remote", "remove", "origin"]);
    fixture.git(&fixture.main, &["update-ref", "refs/remotes/origin/main", &tip]);

    let caption = fixture.list();

    for text in ["tracking ref", "Remote state", "origin/main", "in sync"] {
        assert!(!caption.contains(text), "{text:?} shown without an origin: {caption}");
    }
    assert!(refresh_workers(&fixture.main).is_empty(), "no worker");
    assert!(!remote_head_lock_path(&fixture.head_store()).exists(), "no worker ever took the live-head lock");
    assert!(!fixture.head_store().exists());
}
