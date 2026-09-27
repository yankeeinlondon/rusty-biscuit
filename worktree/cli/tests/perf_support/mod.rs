//! Shared fixtures and `--perf` parsing for the cache SLA and PR integration
//! tests, plus the local network stand-ins ([`ProxyStub`], [`FakeGitea`]) and
//! refresh-worker helpers the PR tests use.
//!
//! Both `cache_warm_path.rs` and `cache_cold_path.rs` build the same mixed
//! multi-worktree repo and assert on the `list gather` stage timing parsed
//! from `wt list --perf` — the stage the spec targets (it dominates a cold
//! `wt list`). Asserting full-command wall-clock alone could pass while
//! `list gather` regresses, so these helpers measure the stage directly.
//!
//! The fixture is *mixed* on purpose: several divergent branches (the warm
//! cache collapses their `rev-list` + `merge-tree` cost) plus fast-forward and
//! behind-only branches (which exercise the cold-path speculative `merge-tree`
//! the cache cannot help). A representative mix is what proves the cache win on
//! the divergent shape and bounds the speculative-`merge-tree` tradeoff on the
//! non-divergent shapes.

#![allow(dead_code)]

use std::fs;
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::io::{Read, Write};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use assert_cmd::cargo::cargo_bin;
use worktree::live_remote::RemoteHeads;
use worktree::pull_requests::{
    OpenPrSource, OpenPullRequest, RefreshOutcome, SniffOpenPrSource, pr_lock_path, refresh, unix_now,
};
use worktree::remote_head::{PrFailure, refresh_remote_head, remote_head_lock_path, remote_head_store_path};

/// Branches of each divergence shape in the mixed fixture. The total worktree
/// count is `1 (main) + DIVERGENT + FAST_FORWARD + BEHIND`.
pub const DIVERGENT_BRANCHES: usize = 4;
pub const FAST_FORWARD_BRANCHES: usize = 3;
pub const BEHIND_BRANCHES: usize = 3;

/// A throwaway repo with `main` plus a mix of divergent, fast-forward, and
/// behind-only linked worktrees, isolated from the user's real cache via a
/// temp `HOME` / `XDG_CACHE_HOME`.
pub struct MixedFixture {
    _repo: tempfile::TempDir,
    home: tempfile::TempDir,
    xdg_cache: tempfile::TempDir,
    main: PathBuf,
    worktrees: Vec<PathBuf>,
}

impl MixedFixture {
    pub fn new() -> Self {
        let repo = tempfile::tempdir().expect("create temp repo");
        let home = tempfile::tempdir().expect("create temp home");
        let xdg_cache = tempfile::tempdir().expect("create temp xdg cache");
        let main = repo.path().to_path_buf();
        let parent = main.parent().expect("temp repo has parent").to_path_buf();
        let repo_name = main
            .file_name()
            .expect("repo has name")
            .to_string_lossy()
            .into_owned();

        run_git(&main, &["init", "-b", "main"]);
        run_git(&main, &["config", "user.email", "test@example.com"]);
        run_git(&main, &["config", "user.name", "Test User"]);
        run_git(&main, &["config", "commit.gpgsign", "false"]);
        run_git(&main, &["config", "gc.auto", "0"]);
        run_git(&main, &["config", "core.untrackedCache", "true"]);

        commit(&main, "base.txt", "base", "base");

        // Behind-only branches: forked at base and never advanced.
        let mut branches = Vec::new();
        for i in 0..BEHIND_BRANCHES {
            let name = format!("behind-{i}");
            run_git(&main, &["branch", &name]);
            branches.push(name);
        }

        // Divergent branches: one commit on the branch (ahead); `main` advances
        // below (behind).
        for i in 0..DIVERGENT_BRANCHES {
            let name = format!("divergent-{i}");
            run_git(&main, &["checkout", "-b", &name, "main"]);
            commit(&main, &format!("{name}.txt"), "x", "divergent commit");
            run_git(&main, &["checkout", "main"]);
            branches.push(name);
        }

        // Advance `main`: behind-* are now behind by one; divergent-* diverge.
        commit(&main, "main-advance.txt", "advance", "advance main");

        // Fast-forward branches: forked at the advanced `main` tip with one
        // commit (ahead, not behind). `main` does not move afterward.
        for i in 0..FAST_FORWARD_BRANCHES {
            let name = format!("fast-forward-{i}");
            run_git(&main, &["checkout", "-b", &name, "main"]);
            commit(&main, &format!("{name}.txt"), "x", "fast-forward commit");
            run_git(&main, &["checkout", "main"]);
            branches.push(name);
        }

        let mut worktrees = Vec::new();
        for name in &branches {
            let path = parent.join(format!("{repo_name}-{name}"));
            run_git(&main, &["worktree", "add", path.to_str().unwrap(), name]);
            worktrees.push(path);
        }

        Self {
            _repo: repo,
            home,
            xdg_cache,
            main,
            worktrees,
        }
    }

    /// Prime git's untracked-files cache in every checkout so the live dirty
    /// walk is steady-state and does not skew the `list gather` measurement.
    pub fn warm_untracked_cache(&self) {
        let args = &["-c", "core.untrackedCache=true", "status", "--porcelain"];
        run_git(&self.main, args);
        for worktree in &self.worktrees {
            run_git(worktree, args);
        }
    }

    /// Construct `wt` with the fixture's repository and isolated cache roots.
    pub fn wt_command(&self) -> Command {
        let mut command = Command::new(cargo_bin("wt"));
        command
            .current_dir(&self.main)
            .env("HOME", self.home.path())
            .env("XDG_CACHE_HOME", self.xdg_cache.path());
        command
    }

    /// Points `origin` at a GitHub URL, so `wt list` asks GitHub for open PRs.
    /// Pair it with [`MixedFixture::wt_command_via`] so no request leaves the
    /// host.
    pub fn with_github_origin(self) -> Self {
        run_git(&self.main, &["remote", "add", "origin", "https://github.com/owner/repo.git"]);
        self
    }

    /// Points `origin` at [`FakeGitea::ORIGIN`]. Pair it with
    /// [`MixedFixture::wt_command_via_gitea`].
    pub fn with_gitea_origin(self) -> Self {
        run_git(&self.main, &["remote", "add", "origin", FakeGitea::ORIGIN]);
        self
    }

    /// Points `origin` at `url`, such as a [`HoldingOrigin`]. Pair it with
    /// [`MixedFixture::wt_command_direct`].
    pub fn with_origin(self, url: &str) -> Self {
        run_git(&self.main, &["remote", "add", "origin", url]);
        self
    }

    /// [`MixedFixture::wt_command`] whose git reaches a loopback `origin`
    /// directly: no proxy variable, no provider token, and none of the
    /// user's or the system's git configuration (so no `insteadOf` or
    /// `http.proxy` applies).
    pub fn wt_command_direct(&self) -> Command {
        let global = self.home.path().join("empty.gitconfig");
        fs::write(&global, "").expect("write an empty global git config");
        let mut command = self.wt_command_without_network();
        for name in ["GIT_CONFIG_COUNT", "GIT_CONFIG_KEY_0", "GIT_CONFIG_VALUE_0", "GIT_CONFIG_KEY_1", "GIT_CONFIG_VALUE_1"] {
            command.env_remove(name);
        }
        command.env("GIT_CONFIG_NOSYSTEM", "1").env("GIT_CONFIG_GLOBAL", global);
        command
    }

    /// [`MixedFixture::wt_command`] with every HTTPS request sent through
    /// `proxy`, and no provider token.
    pub fn wt_command_via(&self, proxy: &ProxyStub) -> Command {
        let mut command = self.wt_command_without_network();
        command.env("HTTPS_PROXY", proxy.url()).env("https_proxy", proxy.url());
        command
    }

    /// [`MixedFixture::wt_command`] with every plain-HTTP request sent to
    /// `gitea`, and no provider token.
    pub fn wt_command_via_gitea(&self, gitea: &FakeGitea) -> Command {
        let mut command = self.wt_command_without_network();
        command.env("HTTP_PROXY", gitea.url()).env("http_proxy", gitea.url());
        command
    }

    /// `wt internal-refresh <main>`, as `wt list` starts it, but owned by
    /// the test: it can be waited for or killed.
    pub fn refresh_worker_via_gitea(&self, gitea: &FakeGitea) -> Command {
        let mut command = self.wt_command_via_gitea(gitea);
        command.arg("internal-refresh").arg(&self.main);
        command
    }

    /// `wt` with no proxy, no provider token, and no image terminal; the
    /// caller adds the one proxy its requests go to.
    ///
    /// Git's own HTTP transports are refused (`protocol.http(s).allow=never`
    /// through `GIT_CONFIG_*`), because git honors the same proxy variables:
    /// otherwise the worker's live-head `ls-remote` would reach the PR
    /// stand-ins and be counted as a PR request. A live-head test that needs
    /// git's transport removes `GIT_CONFIG_COUNT`.
    fn wt_command_without_network(&self) -> Command {
        let mut command = self.wt_command();
        command
            .env("GIT_CONFIG_COUNT", "2")
            .env("GIT_CONFIG_KEY_0", "protocol.http.allow")
            .env("GIT_CONFIG_VALUE_0", "never")
            .env("GIT_CONFIG_KEY_1", "protocol.https.allow")
            .env("GIT_CONFIG_VALUE_1", "never");
        for name in [
            "ALL_PROXY", "all_proxy", "NO_PROXY", "no_proxy", "HTTPS_PROXY", "https_proxy", "HTTP_PROXY",
            "http_proxy", "GH_TOKEN", "GITHUB_TOKEN", "GITEA_TOKEN", "FORGEJO_TOKEN", "CODEBERG_TOKEN",
        ] {
            command.env_remove(name);
        }
        for (name, _) in std::env::vars_os() {
            let name = name.to_string_lossy();
            if name.starts_with("SNIFF_") && name.ends_with("_TOKEN") {
                command.env_remove(name.as_ref());
            }
        }
        command.env_remove("TERM_PROGRAM").env_remove("KITTY_WINDOW_ID");
        command
    }

    pub fn main(&self) -> &Path {
        &self.main
    }

    /// The linked worktrees, one per branch.
    pub fn worktrees(&self) -> &[PathBuf] {
        &self.worktrees
    }

    /// The PR store the spawned `wt` reads and writes. The cache directory
    /// follows `HOME` (macOS) or `XDG_CACHE_HOME` (Linux); on Windows it does
    /// not, so the real per-user path (keyed by this temporary repository) is
    /// used there.
    pub fn pr_store(&self) -> PathBuf {
        let real = worktree::pull_requests::pr_store_path(&self.main).expect("PR store path");
        isolated_cache_file(self.home.path(), self.xdg_cache.path(), &real)
    }

    /// The live-head store the spawned `wt` reads and the worker writes; see
    /// [`MixedFixture::pr_store`] for where it lives.
    pub fn remote_head_store(&self) -> PathBuf {
        let real = remote_head_store_path(&self.main).expect("remote-head store path");
        self.pr_store().with_file_name(real.file_name().expect("store file name"))
    }

    /// Writes a live head of `main` for the current `origin`, checked `age`
    /// ago: `sha` is its object ID, or `None` for a verified absence.
    ///
    /// A fresh one isolates a test from the worker's live-head half: `wt list`
    /// launches no worker for it, and a worker skips its request.
    pub fn seed_remote_head_store(&self, age: Duration, sha: Option<&str>) {
        let origin = worktree::pull_requests::origin_url(&self.main).expect("the fixture has an origin");
        let store = self.remote_head_store();
        fs::create_dir_all(store.parent().expect("store dir")).expect("create store dir");
        let json = serde_json::json!({
            "format_version": 1,
            "origin_digest": worktree::pull_requests::origin_digest(&origin),
            "branch": "main",
            "sha": sha,
            "checked_at": unix_now() - age.as_secs(),
        });
        fs::write(&store, serde_json::to_vec(&json).expect("serialize")).expect("write store");
    }

    /// Writes a PR store for the current `origin`, fetched `age` ago,
    /// holding one open PR from `branch` into `main`.
    pub fn seed_pr_store(&self, age: Duration, number: u64, branch: &str) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_secs();
        let origin = worktree::pull_requests::origin_url(&self.main).expect("the fixture has an origin");
        let source_repo = SniffOpenPrSource { remote_url: origin.clone(), deadline: Duration::ZERO }
            .source_repo()
            .expect("the origin names a repository");
        let store = self.pr_store();
        fs::create_dir_all(store.parent().expect("store dir")).expect("create store dir");
        let json = serde_json::json!({
            "format_version": 2,
            "origin_digest": worktree::pull_requests::origin_digest(&origin),
            "fetched_at": now - age.as_secs(),
            "source_repo": source_repo,
            "pull_requests": [{
                "number": number,
                "url": format!("https://example.invalid/{source_repo}/pull/{number}"),
                "source_repo": source_repo,
                "source_branch": branch,
                "target_branch": "main",
            }],
        });
        fs::write(&store, serde_json::to_vec(&json).expect("serialize")).expect("write store");
    }

    /// Writes a PR store for the current `origin`, fetched `age` ago, holding
    /// no open PR: an answer for any origin, including one no provider
    /// recognizes.
    pub fn seed_empty_pr_store(&self, age: Duration) {
        let origin = worktree::pull_requests::origin_url(&self.main).expect("the fixture has an origin");
        let source_repo = SniffOpenPrSource { remote_url: origin.clone(), deadline: Duration::ZERO }.source_repo();
        let store = self.pr_store();
        fs::create_dir_all(store.parent().expect("store dir")).expect("create store dir");
        let json = serde_json::json!({
            "format_version": 2,
            "origin_digest": worktree::pull_requests::origin_digest(&origin),
            "fetched_at": unix_now() - age.as_secs(),
            "source_repo": source_repo,
            "pull_requests": [],
        });
        fs::write(&store, serde_json::to_vec(&json).expect("serialize")).expect("write store");
    }

    /// Runs a refresh the way a competing worker would, with a source that
    /// must never be asked: `Contended` while a worker holds the lock.
    pub fn probe_refresh(&self) -> RefreshOutcome {
        refresh(&self.pr_store(), &self.main, unix_now, |_| Box::new(NoRequest))
    }

    /// [`MixedFixture::probe_refresh`] for the live-head lock.
    pub fn probe_head_refresh(&self) -> RefreshOutcome {
        refresh_remote_head(&self.remote_head_store(), &self.main, unix_now, &NoRequest)
    }

    /// Waits up to `limit` until no worker for the fixture runs and neither
    /// refresh lock is held, calling `nudge` before each probe (to unblock the
    /// worker's request). A free PR lock alone is no proof: the worker's
    /// live-head half may still be running.
    pub fn wait_until_unlocked(&self, limit: Duration, mut nudge: impl FnMut()) -> bool {
        let deadline = Instant::now() + limit;
        loop {
            nudge();
            if self.probe_refresh() != RefreshOutcome::Contended
                && self.probe_head_refresh() != RefreshOutcome::Contended
                && refresh_workers(&self.main).is_empty()
            {
                return true;
            }
            if Instant::now() >= deadline {
                return false;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    /// Delete the worktree SHA-pair cache so the next run is a guaranteed miss.
    pub fn clear_worktree_cache(&self) {
        let _ = fs::remove_dir_all(self.home.path().join("Library").join("Caches").join("worktree"));
        let _ = fs::remove_dir_all(self.home.path().join(".cache").join("worktree"));
        let _ = fs::remove_dir_all(self.xdg_cache.path().join("worktree"));
    }

    /// Run `wt list --perf` and return the parsed `list gather` stage duration.
    pub fn list_gather_duration(&self) -> Duration {
        let output = self
            .wt_command()
            .args(["list", "--perf"])
            .env_remove("TERM_PROGRAM")
            .env_remove("KITTY_WINDOW_ID")
            .output()
            .expect("wt list --perf should run");
        assert!(
            output.status.success(),
            "wt list --perf failed: status={:?}",
            output.status
        );
        let stderr = String::from_utf8_lossy(&output.stderr);
        list_gather_from_perf(&stderr).unwrap_or_else(|| {
            panic!("could not find `list gather` stage in --perf output:\n{stderr}")
        })
    }
}

impl Default for MixedFixture {
    fn default() -> Self {
        Self::new()
    }
}

/// Where a spawned `wt` with `HOME=home` and `XDG_CACHE_HOME=xdg_cache` keeps
/// the cache file the library names `real`: under `home` on macOS, under
/// `xdg_cache` on Linux, and at `real` itself on Windows, whose user cache
/// ignores both variables.
pub fn isolated_cache_file(home: &Path, xdg_cache: &Path, real: &Path) -> PathBuf {
    if cfg!(windows) {
        return real.to_path_buf();
    }
    let root = if cfg!(target_os = "macos") {
        home.join("Library").join("Caches")
    } else {
        xdg_cache.to_path_buf()
    };
    root.join("worktree").join(real.file_name().expect("store file name"))
}

/// Removes a PR store, the live-head store beside it, and both lock sidecars,
/// which on Windows live in the real user cache (see
/// [`MixedFixture::pr_store`]).
pub struct RemoveOnDrop(pub PathBuf);

impl Drop for RemoveOnDrop {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
        let _ = fs::remove_file(pr_lock_path(&self.0));
        // `<hash>.prs.json` → `<hash>.remote-head.json`.
        if let Some(name) = self.0.file_name().and_then(|name| name.to_str()) {
            let head = self.0.with_file_name(name.replace(".prs.json", ".remote-head.json"));
            let _ = fs::remove_file(remote_head_lock_path(&head));
            let _ = fs::remove_file(head);
        }
    }
}

/// A source (PR or live head) that must never be asked.
pub struct NoRequest;

impl RemoteHeads for NoRequest {
    fn live_head(&self, _remote: &str, _branch: &str) -> Result<Option<String>, String> {
        Err("the probe makes no request".into())
    }
}

impl OpenPrSource for NoRequest {
    fn source_repo(&self) -> Option<String> {
        None
    }
    fn fetch(&self) -> Result<Vec<OpenPullRequest>, PrFailure> {
        Err(PrFailure::Other)
    }
}

/// A running `wt internal-refresh` for one repository.
#[derive(Debug)]
pub struct RefreshWorker {
    pub pid: u32,
    /// `None` where the OS would not say.
    pub cwd: Option<PathBuf>,
}

/// The running refresh workers for the repository whose main checkout is
/// `main`, whoever started them.
pub fn refresh_workers(main: &Path) -> Vec<RefreshWorker> {
    use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

    let main = fs::canonicalize(main).expect("canonical main checkout");
    let mut system = System::new();
    system.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::nothing().with_cmd(UpdateKind::Always).with_cwd(UpdateKind::Always),
    );
    system
        .processes()
        .iter()
        // On Linux every thread is listed too, with its process's argv; the
        // worker runs two, so count processes only.
        .filter(|(_, process)| process.thread_kind().is_none())
        .filter(|(_, process)| {
            let cmd = process.cmd();
            let position = cmd.iter().position(|arg| arg == "internal-refresh");
            position
                .and_then(|at| cmd.get(at + 1))
                .and_then(|repo| fs::canonicalize(repo).ok())
                .is_some_and(|repo| repo == main)
        })
        .map(|(pid, process)| RefreshWorker {
            pid: pid.as_u32(),
            cwd: process.cwd().map(Path::to_path_buf),
        })
        .collect()
}

/// Waits up to `limit` until exactly `count` refresh workers run for `main`.
pub fn wait_for_refresh_workers(main: &Path, count: usize, limit: Duration) -> Vec<RefreshWorker> {
    let deadline = Instant::now() + limit;
    loop {
        let workers = refresh_workers(main);
        if workers.len() == count || Instant::now() >= deadline {
            return workers;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn commit(repo: &Path, file: &str, contents: &str, message: &str) {
    fs::write(repo.join(file), format!("{contents}\n")).expect("write commit file");
    run_git(repo, &["add", "."]);
    run_git(repo, &["commit", "-m", message]);
}

fn run_git(repo: &Path, args: &[&str]) {
    let status = Command::new("git")
        .current_dir(repo)
        .args(args)
        .status()
        .expect("git should be installed");
    assert!(status.success(), "git {args:?} failed in {repo:?}");
}

/// Extract the `list gather` stage duration from rendered `--perf` output.
pub fn list_gather_from_perf(stderr: &str) -> Option<Duration> {
    stage_from_perf(stderr, "list gather")
}

/// Extract a stage's duration from rendered `--perf` output.
pub fn stage_from_perf(stderr: &str, stage: &str) -> Option<Duration> {
    let clean = strip_ansi(stderr);
    let line = clean.lines().find(|line| line.contains(stage))?;
    line.split_whitespace().find_map(parse_perf_duration)
}

/// A local stand-in for an HTTPS proxy, so a PR request never leaves the host.
pub struct ProxyStub {
    port: u16,
    connections: Arc<AtomicUsize>,
    held: Arc<Mutex<Vec<TcpStream>>>,
}

impl ProxyStub {
    /// Accepts every connection and never answers, so each request runs into
    /// its deadline. Connections are held open until
    /// [`ProxyStub::close_held`] or the test process ends.
    pub fn hanging() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind proxy stub");
        let port = listener.local_addr().expect("proxy stub address").port();
        let connections = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&connections);
        let held: Arc<Mutex<Vec<TcpStream>>> = Arc::default();
        let holder = Arc::clone(&held);
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                holder.lock().expect("held connections").push(stream);
                counter.fetch_add(1, Ordering::SeqCst);
            }
        });
        Self { port, connections, held }
    }

    /// Closes every connection held so far, so a request blocked on one
    /// fails at once instead of waiting for its deadline.
    pub fn close_held(&self) {
        for stream in self.held.lock().expect("held connections").drain(..) {
            let _ = stream.shutdown(std::net::Shutdown::Both);
        }
    }

    /// Waits up to `limit` for at least `count` accepted connections.
    pub fn wait_for_connections(&self, count: usize, limit: Duration) -> bool {
        let deadline = std::time::Instant::now() + limit;
        while self.connections() < count {
            if std::time::Instant::now() >= deadline {
                return false;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        true
    }

    /// A port with nothing listening: every connection is refused at once,
    /// as with the network down.
    pub fn refusing() -> Self {
        let port = TcpListener::bind("127.0.0.1:0")
            .expect("bind a free port")
            .local_addr()
            .expect("free port address")
            .port();
        Self {
            port,
            connections: Arc::default(),
            held: Arc::default(),
        }
    }

    pub fn url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    /// Connections accepted so far (always 0 for a refusing stub).
    pub fn connections(&self) -> usize {
        self.connections.load(Ordering::SeqCst)
    }
}

/// A loopback `origin` for git's smart-HTTP transport that accepts every
/// connection, records its request line, and never answers, so a live-head
/// `ls-remote` stays blocked until [`HoldingOrigin::close_held`] or its
/// deadline. Git reaches it directly: the caller clears the proxy variables
/// and the user's git configuration.
pub struct HoldingOrigin {
    port: u16,
    requests: Arc<Mutex<Vec<String>>>,
    held: Arc<Mutex<Vec<TcpStream>>>,
}

impl HoldingOrigin {
    pub fn new() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind holding origin");
        let port = listener.local_addr().expect("holding origin address").port();
        let requests: Arc<Mutex<Vec<String>>> = Arc::default();
        let held: Arc<Mutex<Vec<TcpStream>>> = Arc::default();
        let (recorder, holder) = (Arc::clone(&requests), Arc::clone(&held));
        std::thread::spawn(move || {
            for mut stream in listener.incoming().flatten() {
                let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
                let mut head = Vec::new();
                let mut buffer = [0_u8; 1024];
                while !head.windows(4).any(|window| window == b"\r\n\r\n") {
                    match stream.read(&mut buffer) {
                        Ok(0) | Err(_) => break,
                        Ok(read) => head.extend_from_slice(&buffer[..read]),
                    }
                }
                let line = String::from_utf8_lossy(&head).lines().next().unwrap_or_default().to_string();
                holder.lock().expect("held connections").push(stream);
                recorder.lock().expect("recorded requests").push(line);
            }
        });
        Self { port, requests, held }
    }

    /// The URL to set as `origin`.
    pub fn url(&self) -> String {
        format!("http://127.0.0.1:{}/r.git", self.port)
    }

    /// The request line of every connection so far.
    pub fn requests(&self) -> Vec<String> {
        self.requests.lock().expect("recorded requests").clone()
    }

    /// Waits up to `limit` for at least `count` recorded requests.
    pub fn wait_for_requests(&self, count: usize, limit: Duration) -> bool {
        let deadline = Instant::now() + limit;
        while self.requests().len() < count {
            if Instant::now() >= deadline {
                return false;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        true
    }

    /// Closes every held connection, so a blocked `ls-remote` fails at once.
    pub fn close_held(&self) {
        for stream in self.held.lock().expect("held connections").drain(..) {
            let _ = stream.shutdown(std::net::Shutdown::Both);
        }
    }
}

impl Default for HoldingOrigin {
    fn default() -> Self {
        Self::new()
    }
}

/// What [`FakeGitea`] answers a PR-list request with.
#[derive(Debug, Clone)]
pub enum GiteaReply {
    /// Open PRs `(number, source branch)` from `o/r` into `main`.
    Open(Vec<(u64, &'static str)>),
    /// An HTTP error status, such as 401 or 500.
    Status(u16),
}

#[derive(Debug)]
struct GiteaState {
    requests: usize,
    waiting: usize,
    held: bool,
    reply: GiteaReply,
}

/// A local plain-HTTP Gitea stand-in. `wt` reaches it as
/// [`FakeGitea::ORIGIN`] through `HTTP_PROXY` (sniff maps a `gitea.` host to
/// the Gitea API), so no request leaves the host and no TLS is involved.
///
/// While [`FakeGitea::hold`] is in effect every request waits unanswered,
/// which is how a test blocks a detached worker mid-request with its lock
/// held. Dropping the server answers every waiting request with 503.
pub struct FakeGitea {
    port: u16,
    shared: Arc<(Mutex<GiteaState>, Condvar)>,
    before_reply: BeforeReply,
}

type BeforeReply = Arc<Mutex<Option<Box<dyn Fn() + Send>>>>;

/// How long a held request waits before the server gives up on the test.
const GITEA_HOLD_LIMIT: Duration = Duration::from_secs(60);

impl FakeGitea {
    pub const ORIGIN: &'static str = "http://gitea.test/o/r.git";

    pub fn new(reply: GiteaReply) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind fake gitea");
        let port = listener.local_addr().expect("fake gitea address").port();
        let shared = Arc::new((
            Mutex::new(GiteaState { requests: 0, waiting: 0, held: false, reply }),
            Condvar::new(),
        ));
        let before_reply: BeforeReply = Arc::new(Mutex::new(None));
        let server = Arc::clone(&shared);
        let server_before_reply = Arc::clone(&before_reply);
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                let shared = Arc::clone(&server);
                let before_reply = Arc::clone(&server_before_reply);
                std::thread::spawn(move || serve(stream, &shared, &before_reply));
            }
        });
        Self { port, shared, before_reply }
    }

    pub fn url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    /// Makes every request from now on wait until [`FakeGitea::release`].
    pub fn hold(&self) {
        self.state().held = true;
    }

    /// Runs `action` for every later request after it is received and before
    /// it is answered, while the requester waits.
    ///
    /// A foreground `wt list` request gives up after its 300 ms deadline, too
    /// short for the test thread to observe a held request, act, and release
    /// it; `action` runs on the server thread instead.
    pub fn before_reply(&self, action: impl Fn() + Send + 'static) {
        *self.before_reply.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(Box::new(action));
    }

    /// Answers waiting and later requests with `reply`.
    pub fn release(&self, reply: GiteaReply) {
        let mut state = self.state();
        state.held = false;
        state.reply = reply;
        self.shared.1.notify_all();
    }

    /// Requests received so far, answered or not.
    pub fn requests(&self) -> usize {
        self.state().requests
    }

    /// Requests received and not yet answered.
    pub fn waiting(&self) -> usize {
        self.state().waiting
    }

    /// Waits up to `limit` until `count` requests are waiting unanswered.
    pub fn wait_for_waiting(&self, count: usize, limit: Duration) -> bool {
        let deadline = Instant::now() + limit;
        let mut state = self.state();
        while state.waiting < count {
            let Some(left) = deadline.checked_duration_since(Instant::now()) else {
                return false;
            };
            state = self.shared.1.wait_timeout(state, left).expect("fake gitea state").0;
        }
        true
    }

    fn state(&self) -> std::sync::MutexGuard<'_, GiteaState> {
        self.shared.0.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

impl Drop for FakeGitea {
    fn drop(&mut self) {
        self.release(GiteaReply::Status(503));
    }
}

fn serve(mut stream: TcpStream, shared: &(Mutex<GiteaState>, Condvar), before_reply: &Mutex<Option<Box<dyn Fn() + Send>>>) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
    let mut head = Vec::new();
    let mut buffer = [0_u8; 1024];
    while !head.windows(4).any(|window| window == b"\r\n\r\n") {
        match stream.read(&mut buffer) {
            Ok(0) | Err(_) => return,
            Ok(read) => head.extend_from_slice(&buffer[..read]),
        }
    }
    let reply = {
        let (lock, changed) = shared;
        let mut state = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        state.requests += 1;
        state.waiting += 1;
        changed.notify_all();
        let deadline = Instant::now() + GITEA_HOLD_LIMIT;
        while state.held {
            let Some(left) = deadline.checked_duration_since(Instant::now()) else {
                break;
            };
            state = changed.wait_timeout(state, left).unwrap_or_else(|poisoned| poisoned.into_inner()).0;
        }
        state.waiting -= 1;
        changed.notify_all();
        state.reply.clone()
    };
    if let Some(action) = before_reply.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).as_ref() {
        action();
    }
    let (status, body) = match reply {
        GiteaReply::Open(prs) => (200, gitea_pulls_json(&prs)),
        GiteaReply::Status(status) => (status, r#"{"message":"fake gitea error"}"#.to_string()),
    };
    let response = format!(
        "HTTP/1.1 {status} Fake\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    );
    // The requester may be gone (killed, or past its deadline).
    let _ = stream.write_all(response.as_bytes());
}

fn gitea_pulls_json(prs: &[(u64, &str)]) -> String {
    let pulls: Vec<serde_json::Value> = prs
        .iter()
        .map(|(number, branch)| {
            serde_json::json!({
                "number": number,
                "title": format!("PR {number}"),
                "state": "open",
                "html_url": format!("http://gitea.test/o/r/pulls/{number}"),
                "head": { "ref": branch, "sha": "0123456789abcdef0123456789abcdef01234567", "repo": { "full_name": "o/r" } },
                "base": { "ref": "main", "repo": { "full_name": "o/r" } },
            })
        })
        .collect();
    serde_json::Value::Array(pulls).to_string()
}

/// Parse a metrics-tree duration token such as `216.0ms`, `39.0µs`, or `1.2s`.
///
/// ## Returns
///
/// `None` for tokens that are not durations (e.g. the trailing `64%` share).
pub fn parse_perf_duration(token: &str) -> Option<Duration> {
    // `ms` / `µs` / `us` / `ns` must be tried before the bare `s` suffix, which
    // would otherwise strip the trailing `s` of `ms` and misparse the rest.
    let (number, divisor) = if let Some(rest) = token.strip_suffix("ms") {
        (rest, 1e3)
    } else if let Some(rest) = token.strip_suffix("µs") {
        (rest, 1e6)
    } else if let Some(rest) = token.strip_suffix("us") {
        (rest, 1e6)
    } else if let Some(rest) = token.strip_suffix("ns") {
        (rest, 1e9)
    } else {
        (token.strip_suffix('s')?, 1.0)
    };
    let value: f64 = number.parse().ok()?;
    Some(Duration::from_secs_f64(value / divisor))
}

/// Remove ANSI CSI/OSC escape sequences, preserving multibyte glyphs.
fn strip_ansi(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\u{1b}' {
            out.push(c);
            continue;
        }
        match chars.peek() {
            // CSI: ESC [ ... <final letter>
            Some('[') => {
                chars.next();
                while let Some(&next) = chars.peek() {
                    chars.next();
                    if next.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
            // OSC: ESC ] ... BEL
            Some(']') => {
                chars.next();
                while let Some(&next) = chars.peek() {
                    chars.next();
                    if next == '\u{07}' {
                        break;
                    }
                }
            }
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_known_duration_units() {
        assert_eq!(parse_perf_duration("216.0ms"), Some(Duration::from_micros(216_000)));
        assert_eq!(parse_perf_duration("39.0µs"), Some(Duration::from_nanos(39_000)));
        assert_eq!(parse_perf_duration("1.5s"), Some(Duration::from_millis(1500)));
        assert_eq!(parse_perf_duration("64%"), None);
        assert_eq!(parse_perf_duration("gather"), None);
    }

    #[test]
    fn finds_list_gather_line_in_rendered_perf() {
        let sample = "\u{1b}[33m▌\u{1b}[0m ├─ list gather   \u{1b}[1m216.0ms\u{1b}[0m   64%\n";
        assert_eq!(
            list_gather_from_perf(sample),
            Some(Duration::from_micros(216_000))
        );
    }
}
