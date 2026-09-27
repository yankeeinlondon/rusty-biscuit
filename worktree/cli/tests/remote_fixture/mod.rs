//! A local bare `origin`, a `pusher` clone standing in for everyone else,
//! and the listed clone with one linked worktree, all with the user's and the
//! system's git configuration shut out; plus [`UploadPackGate`], which holds
//! `origin`'s `upload-pack` to keep the worker working. Shared by the
//! real-Git listing tests (`list_remote_head.rs`, `list_flags.rs`).

#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::time::{Duration, Instant};

use assert_cmd::cargo::cargo_bin;
use worktree::pull_requests::{pr_lock_path, pr_store_path};
use worktree::remote_head::{remote_head_lock_path, remote_head_store_path};

use crate::perf_support::{isolated_cache_file, wait_for_refresh_workers};

/// How long a test waits for a detached worker before failing.
pub const WORKER_WAIT: Duration = Duration::from_secs(20);

pub struct Fixture {
    pub root: tempfile::TempDir,
    pub main: PathBuf,
    pub linked: PathBuf,
    pub pusher: PathBuf,
    pub bare: PathBuf,
}

impl Fixture {
    /// `origin` holds one commit on `main`, which the listed clone has
    /// fetched: its `main` is in sync with `origin/main`.
    pub fn new() -> Self {
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
    pub fn git(&self, dir: &Path, args: &[&str]) -> String {
        let output = self.isolated(Command::new("git")).current_dir(dir).args(args).output().expect("git runs");
        assert!(output.status.success(), "git {args:?} in {dir:?}: {output:?}");
        String::from_utf8(output.stdout).expect("utf-8").trim().to_string()
    }

    pub fn isolated(&self, mut command: Command) -> Command {
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
    pub fn commit_and_push(&self, message: &str) -> String {
        fs::write(self.pusher.join("file.txt"), format!("{message}\n")).expect("write");
        self.git(&self.pusher, &["add", "."]);
        self.git(&self.pusher, &["commit", "-m", message]);
        self.git(&self.pusher, &["push", "origin", "main"]);
        self.git(&self.pusher, &["rev-parse", "HEAD"])
    }

    pub fn wt(&self, dir: &Path) -> Command {
        let mut command = self.isolated(Command::new(cargo_bin("wt")));
        command.current_dir(dir).env("NO_COLOR", "1").env_remove("WT_SHELL_WRAPPER");
        command
    }

    /// `wt list` from `dir`, with its stderr's whitespace collapsed (the
    /// caption word-wraps) and the padding that follows a plain branch badge
    /// dropped before punctuation.
    pub fn list_from(&self, dir: &Path) -> String {
        let output = self.wt(dir).arg("list").output().expect("wt list runs");
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(output.status.success(), "wt list failed:\n{stderr}");
        assert_no_spinner(&stderr);
        stderr.split_whitespace().collect::<Vec<_>>().join(" ").replace(" .", ".").replace(" ;", ";")
    }

    pub fn list(&self) -> String {
        self.list_from(&self.main)
    }

    /// `wt internal-refresh <main>` as a direct child, waited for.
    pub fn refresh(&self) {
        self.run_worker(&[]);
    }

    /// `wt internal-refresh <main> <args>` as a direct child, waited for.
    pub fn run_worker(&self, args: &[&str]) {
        let output: Output = self
            .wt(&self.main)
            .arg("internal-refresh")
            .arg(&self.main)
            .args(args)
            .output()
            .expect("the worker runs");
        assert!(output.status.success() && output.stdout.is_empty() && output.stderr.is_empty(), "{output:?}");
    }

    pub fn head_store(&self) -> PathBuf {
        self.cache_file(remote_head_store_path(&self.main).expect("remote-head store path"))
    }

    pub fn cache_file(&self, real: PathBuf) -> PathBuf {
        isolated_cache_file(&self.root.path().join("home"), &self.root.path().join("cache"), &real)
    }

    pub fn stored_document(&self) -> serde_json::Value {
        serde_json::from_slice(&fs::read(self.head_store()).expect("a stored live head")).expect("json")
    }

    /// The store's `answer` half.
    pub fn stored_head(&self) -> serde_json::Value {
        self.stored_document()["answer"].clone()
    }

    /// Moves the stored answer's `checked_at` back by `seconds`, as if it had
    /// been recorded that long ago; a refresh only asks once it is stale.
    pub fn age_head(&self, seconds: u64) {
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

/// The spinner writes only to a terminal, so captured stderr never has it.
pub fn assert_no_spinner(stderr: &str) {
    for glyph in biscuit_terminal::components::spinner::FRAMES {
        assert!(!stderr.contains(glyph), "a spinner frame reached captured stderr: {stderr}");
    }
    // "pulling remote updates" is also caption text, so only the spinner's
    // own texts and its line control are checked.
    for text in ["updating", "using fallback method", "\r", biscuit_terminal::components::spinner::CLEAR_LINE] {
        assert!(!stderr.contains(text), "spinner output reached captured stderr: {stderr:?}");
    }
}

/// Holds `origin`'s `upload-pack` from its `hold_from`-th run (0: the
/// check's `ls-remote`; 1: the fetch after it) until released, and counts
/// every run: `remote.origin.uploadpack` points at a script, which both
/// `ls-remote origin` and `fetch origin` run for a local `origin`.
pub struct UploadPackGate {
    dir: PathBuf,
}

impl UploadPackGate {
    pub fn install(fixture: &Fixture, hold_from: usize) -> Self {
        let dir = fixture.root.path().join("gate");
        fs::create_dir_all(&dir).expect("gate dir");
        let shell_dir = dir.to_string_lossy().replace('\\', "/");
        let script = format!(
            "#!/bin/sh\n\
             n=$(cat '{shell_dir}/count' 2>/dev/null || echo 0)\n\
             echo $((n + 1)) > '{shell_dir}/count'\n\
             if [ \"$n\" -ge {hold_from} ]; then\n\
             \x20 while [ ! -f '{shell_dir}/release' ]; do sleep 0.05; done\n\
             fi\n\
             exec git upload-pack \"$@\"\n"
        );
        fs::write(dir.join("upload-pack.sh"), script).expect("gate script");
        fixture.git(&fixture.main, &["config", "remote.origin.uploadpack", &format!("sh '{shell_dir}/upload-pack.sh'")]);
        Self { dir }
    }

    /// How many times `upload-pack` has started.
    pub fn runs(&self) -> usize {
        fs::read_to_string(self.dir.join("count")).ok().and_then(|n| n.trim().parse().ok()).unwrap_or(0)
    }

    pub fn wait_for_runs(&self, count: usize) {
        let deadline = Instant::now() + WORKER_WAIT;
        while self.runs() < count {
            assert!(Instant::now() < deadline, "upload-pack ran {} times, expected {count}", self.runs());
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    pub fn release(&self) {
        let _ = fs::write(self.dir.join("release"), "");
    }
}

impl Drop for UploadPackGate {
    /// Released before the fixture waits for its workers, even when an
    /// assertion failed.
    fn drop(&mut self) {
        self.release();
    }
}

