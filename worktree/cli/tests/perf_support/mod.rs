//! Shared fixtures and `--perf` parsing for the cache SLA and PR integration
//! tests, plus the local network stand-ins ([`ProxyStub`], [`FakeGitea`],
//! [`HoldingOrigin`]) and refresh-worker helpers the PR tests use.
//!
//! A `wt list` against a fixture with an `origin` leaves a detached
//! `wt internal-refresh` running, which nothing in the test owns. Teardown
//! is guaranteed on every exit path, an assertion failure included:
//! [`MixedFixture`]'s own `Drop` waits for its workers and both refresh
//! locks (killing a worker still running at [`TEARDOWN_WAIT`]) before it
//! removes the stores and its temporary directories, and every stand-in
//! ends its held requests when dropped. A stand-in declared before the
//! fixture, such as a [`HoldingOrigin`] whose URL the fixture needs, drops
//! after it, so pair it with a [`WorkerReaper`] declared after the fixture.
//! A directly spawned `wt` belongs in a [`KillOnDrop`].
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
use std::process::{Child, Command, ExitStatus, Output, Stdio};
use std::io::{Read, Write};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use assert_cmd::cargo::cargo_bin;
use worktree::pull_requests::{
    FetchedPrs, OpenPrSource, PrRequestError, RefreshOutcome, SniffOpenPrSource, pr_lock_path, refresh, unix_now,
};
use worktree::remote_head::{PrFailure, refresh_lock_held, remote_head_lock_path, remote_head_store_path};

pub mod graph;

/// Branches of each divergence shape in the mixed fixture. The total worktree
/// count is `1 (main) + DIVERGENT + FAST_FORWARD + BEHIND`.
pub const DIVERGENT_BRANCHES: usize = 4;
pub const FAST_FORWARD_BRANCHES: usize = 3;
pub const BEHIND_BRANCHES: usize = 3;

/// A throwaway repo with `main` plus a mix of divergent, fast-forward, and
/// behind-only linked worktrees, isolated from the user's real cache via a
/// temp `HOME` / `XDG_CACHE_HOME`.
///
/// Dropping it reaps its refresh workers ([`reap_workers`]) and then removes
/// the PR and live-head stores and their locks, which on Windows live in the
/// real user cache (see [`MixedFixture::pr_store`]).
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

    /// Points `origin` at a bare copy of the repository beside `HOME` and
    /// fetches it, so the worker's check answers at once from local Git and
    /// finds nothing to fetch. Pair it with [`MixedFixture::wt_command_direct`].
    pub fn with_local_origin(self) -> Self {
        let bare = self.home.path().join("origin.git");
        run_git(&self.main, &["clone", "--bare", "--quiet", ".", bare.to_str().unwrap()]);
        run_git(&self.main, &["remote", "add", "origin", bare.to_str().unwrap()]);
        run_git(&self.main, &["fetch", "--quiet", "origin"]);
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

    /// Makes [`FakeGitea::ORIGIN`] a real repository that `gitea` serves to
    /// git through `git http-backend`: a bare `o/r.git` under `HOME` whose
    /// `main` is one commit past this checkout's `main`, which the tracking
    /// ref names too, so the worker's check finds a new tip and fetches it.
    /// Pair it with [`MixedFixture::with_gitea_origin`] and
    /// [`MixedFixture::wt_command_via_gitea_git`]. Returns the commit only
    /// `origin` has.
    pub fn serve_gitea_origin_one_commit_ahead(&self, gitea: &FakeGitea) -> String {
        let root = self.home.path().join("gitea");
        let bare = root.join("o").join("r.git");
        fs::create_dir_all(&bare).expect("create the served repository");
        run_git(&bare, &["init", "--bare", "--quiet", "-b", "main"]);
        run_git(&self.main, &["push", "--quiet", bare.to_str().unwrap(), "main:refs/heads/main"]);
        run_git(&self.main, &["update-ref", "refs/remotes/origin/main", "main"]);
        let output = Command::new("git")
            .current_dir(&bare)
            .args(["-c", "user.name=Someone Else", "-c", "user.email=else@example.com"])
            .args(["commit-tree", "main^{tree}", "-p", "main", "-m", "pushed by someone else"])
            .output()
            .expect("git commit-tree");
        assert!(output.status.success(), "commit-tree: {output:?}");
        let pushed = String::from_utf8(output.stdout).expect("utf-8").trim().to_string();
        run_git(&bare, &["update-ref", "refs/heads/main", &pushed]);
        gitea.serve_repositories(&root);
        pushed
    }

    /// [`MixedFixture::wt_command_via_gitea`] whose git also reaches `gitea`
    /// (`http.proxy`), with no user or system git configuration, for a
    /// fixture built with [`MixedFixture::serve_gitea_origin_one_commit_ahead`].
    pub fn wt_command_via_gitea_git(&self, gitea: &FakeGitea) -> Command {
        let global = self.home.path().join("empty.gitconfig");
        fs::write(&global, "").expect("write an empty global git config");
        let mut command = self.wt_command_via_gitea(gitea);
        command
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", global)
            .env("GIT_CONFIG_KEY_0", "http.proxy")
            .env("GIT_CONFIG_VALUE_0", gitea.url())
            .env("GIT_CONFIG_KEY_1", "protocol.http.allow")
            .env("GIT_CONFIG_VALUE_1", "always");
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

    /// The `HOME` every `wt` command runs with.
    pub fn home(&self) -> &Path {
        self.home.path()
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
    /// A fresh one gives the caption an answer to date; `wt list` still
    /// launches its worker, and the worker's check still runs.
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
            "format_version": worktree::pull_requests::PR_STORE_FORMAT_VERSION,
            "origin_digest": worktree::pull_requests::origin_digest(&origin),
            "fetched_at": now - age.as_secs(),
            "publication": worktree::remote_head::new_attempt_id().expect("a publication id"),
            "source_repo": source_repo,
            "pull_requests": [{
                "number": number,
                "url": format!("https://example.invalid/{source_repo}/pull/{number}"),
                "source_repo": source_repo,
                "source_branch": branch,
                "target_branch": "main",
            }],
            // A seeded answer: nothing is known of how it was asked.
            "credentials": { "state": "unknown" },
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
            "format_version": worktree::pull_requests::PR_STORE_FORMAT_VERSION,
            "origin_digest": worktree::pull_requests::origin_digest(&origin),
            "fetched_at": unix_now() - age.as_secs(),
            "publication": worktree::remote_head::new_attempt_id().expect("a publication id"),
            "source_repo": source_repo,
            "pull_requests": [],
            // A seeded answer: nothing is known of how it was asked.
            "credentials": { "state": "unknown" },
        });
        fs::write(&store, serde_json::to_vec(&json).expect("serialize")).expect("write store");
    }

    /// Runs a refresh the way a competing worker would, with a source that
    /// must never be asked: `Contended` while a worker holds the lock.
    pub fn probe_refresh(&self) -> RefreshOutcome {
        refresh(&self.pr_store(), &self.main, unix_now, |_| Box::new(NoRequest))
    }

    /// Whether a worker holds the live-head lock.
    pub fn head_lock_held(&self) -> bool {
        refresh_lock_held(&self.remote_head_store())
    }

    /// Waits up to `limit` until no worker for the fixture runs and neither
    /// refresh lock is held, calling `nudge` before each probe (to unblock the
    /// worker's request). A free PR lock alone is no proof: the worker's
    /// live-head half may still be running.
    pub fn wait_until_unlocked(&self, limit: Duration, nudge: impl FnMut()) -> bool {
        wait_for_workers(&self.worker_paths(), limit, nudge)
    }

    /// The repository and stores a worker for this fixture uses, computed
    /// without panicking so `Drop` can use it while unwinding.
    fn worker_paths(&self) -> WorkerPaths {
        let real_pr = worktree::pull_requests::pr_store_path(&self.main).ok();
        let real_head = remote_head_store_path(&self.main).ok();
        let (home, xdg) = (self.home.path(), self.xdg_cache.path());
        WorkerPaths {
            main: self.main.clone(),
            pr_store: real_pr.map(|real| isolated_cache_file(home, xdg, &real)),
            head_store: real_head.map(|real| isolated_cache_file(home, xdg, &real)),
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

impl Drop for MixedFixture {
    fn drop(&mut self) {
        let paths = self.worker_paths();
        let finished = reap_workers(&paths, || {});
        paths.remove_stores();
        if !std::thread::panicking() {
            assert!(finished, "a refresh worker outlived the test or kept a lock; release its request first");
        }
    }
}

/// How long teardown waits for a fixture's refresh workers before it kills
/// the ones still running.
pub const TEARDOWN_WAIT: Duration = Duration::from_secs(20);

/// A repository's main checkout and the stores (as the spawned `wt` resolves
/// them) whose locks its refresh workers hold. A store is `None` where the
/// library could not name it.
#[derive(Debug, Clone)]
pub struct WorkerPaths {
    pub main: PathBuf,
    pub pr_store: Option<PathBuf>,
    pub head_store: Option<PathBuf>,
}

impl WorkerPaths {
    /// Whether no worker for `main` runs and neither lock is held. Probing
    /// the PR lock takes it for an instant, so probe only at teardown or once
    /// the worker under test is known to be running.
    fn quiet(&self) -> bool {
        let pr_free = self.pr_store.as_deref().is_none_or(|store| {
            refresh(store, &self.main, unix_now, |_| Box::new(NoRequest)) != RefreshOutcome::Contended
        });
        let head_free = self.head_store.as_deref().is_none_or(|store| !refresh_lock_held(store));
        pr_free && head_free && try_refresh_workers(&self.main).is_some_and(|workers| workers.is_empty())
    }

    /// Removes both stores and their lock sidecars.
    pub fn remove_stores(&self) {
        if let Some(store) = &self.pr_store {
            let _ = fs::remove_file(pr_lock_path(store));
            let _ = fs::remove_file(store);
        }
        if let Some(store) = &self.head_store {
            let _ = fs::remove_file(remote_head_lock_path(store));
            let _ = fs::remove_file(store);
        }
    }
}

/// Waits up to `limit` until no worker for `paths.main` runs and neither
/// lock is held, calling `nudge` before each probe.
pub fn wait_for_workers(paths: &WorkerPaths, limit: Duration, mut nudge: impl FnMut()) -> bool {
    let deadline = Instant::now() + limit;
    loop {
        nudge();
        if paths.quiet() {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// Teardown for a fixture's detached workers: [`wait_for_workers`] for
/// [`TEARDOWN_WAIT`], then kills any worker still running and waits briefly
/// for it to go, so teardown is bounded and no worker outlives the
/// directories it works in. Returns whether the workers ended on their own.
/// Never panics, so it is safe in `Drop` while a failed assertion unwinds.
pub fn reap_workers(paths: &WorkerPaths, nudge: impl FnMut()) -> bool {
    if wait_for_workers(paths, TEARDOWN_WAIT, nudge) {
        return true;
    }
    kill_refresh_workers(&paths.main);
    let deadline = Instant::now() + Duration::from_secs(5);
    while try_refresh_workers(&paths.main).is_some_and(|workers| !workers.is_empty()) && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    false
}

/// A stand-in that holds requests until the test lets them go.
pub trait HeldRequests {
    /// Ends every request held so far and every later one, at once.
    fn release_held(&self);
}

/// Nothing held: for a [`WorkerReaper`] whose fixture has no stand-in.
impl HeldRequests for () {
    fn release_held(&self) {}
}

/// On drop, ends every request `held` holds and reaps the repository's
/// refresh workers ([`reap_workers`]), so none outlives the test, even when an
/// assertion failed first. Declare it after the fixture and the stand-in,
/// so it drops before both. When the test is not already failing, a worker
/// that had to be killed fails it.
pub struct WorkerReaper<'a> {
    paths: WorkerPaths,
    held: &'a dyn HeldRequests,
}

impl<'a> WorkerReaper<'a> {
    pub fn new(fixture: &MixedFixture, held: &'a dyn HeldRequests) -> Self {
        Self::for_repository(fixture.worker_paths(), held)
    }

    pub fn for_repository(paths: WorkerPaths, held: &'a dyn HeldRequests) -> Self {
        Self { paths, held }
    }
}

impl Drop for WorkerReaper<'_> {
    fn drop(&mut self) {
        let finished = reap_workers(&self.paths, || self.held.release_held());
        if !std::thread::panicking() {
            assert!(finished, "a refresh worker outlived the test or kept a lock");
        }
    }
}

/// A test-owned `wt` child, killed and reaped on drop unless already waited
/// for, so a failed assertion never leaves it running.
pub struct KillOnDrop(Option<Child>);

impl KillOnDrop {
    /// Spawns `command` with null stdio.
    pub fn spawn(mut command: Command) -> Self {
        Self::spawn_with(command.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()))
    }

    /// Spawns `command` with the stdio it was given.
    pub fn spawn_with(command: &mut Command) -> Self {
        Self(Some(command.spawn().expect("spawn wt")))
    }

    pub fn child(&mut self) -> &mut Child {
        self.0.as_mut().expect("the child is owned until waited for")
    }

    /// Waits up to [`TEARDOWN_WAIT`] for the child to exit.
    pub fn wait(&mut self) -> ExitStatus {
        let deadline = Instant::now() + TEARDOWN_WAIT;
        loop {
            if let Some(status) = self.child().try_wait().expect("poll child") {
                return status;
            }
            assert!(Instant::now() < deadline, "wt did not exit");
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    /// [`Child::wait_with_output`]; the child is then no longer owned here.
    pub fn wait_with_output(mut self) -> Output {
        self.0.take().expect("the child is owned until waited for").wait_with_output().expect("wait for wt")
    }
}

impl Drop for KillOnDrop {
    fn drop(&mut self) {
        if let Some(child) = self.0.as_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }
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

/// A PR source that must never be asked.
pub struct NoRequest;

impl OpenPrSource for NoRequest {
    fn source_repo(&self) -> Option<String> {
        None
    }
    fn fetch(&self) -> Result<FetchedPrs, PrRequestError> {
        Err(PrFailure::Other.into())
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
    try_refresh_workers(main).expect("canonical main checkout")
}

/// [`refresh_workers`], or `None` when `main` cannot be canonicalized.
fn try_refresh_workers(main: &Path) -> Option<Vec<RefreshWorker>> {
    let main = fs::canonicalize(main).ok()?;
    let system = worker_processes();
    let workers = system
        .processes()
        .iter()
        // On Linux every thread is listed too, with its process's argv; the
        // worker runs two, so count processes only.
        .filter(|(_, process)| process.thread_kind().is_none())
        .filter(|(_, process)| is_worker_for(process, &main))
        .map(|(pid, process)| RefreshWorker {
            pid: pid.as_u32(),
            cwd: process.cwd().map(Path::to_path_buf),
        })
        .collect();
    Some(workers)
}

fn worker_processes() -> sysinfo::System {
    use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

    let mut system = System::new();
    system.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::nothing().with_cmd(UpdateKind::Always).with_cwd(UpdateKind::Always),
    );
    system
}

fn is_worker_for(process: &sysinfo::Process, canonical_main: &Path) -> bool {
    let cmd = process.cmd();
    let position = cmd.iter().position(|arg| arg == "internal-refresh");
    position
        .and_then(|at| cmd.get(at + 1))
        .and_then(|repo| fs::canonicalize(repo).ok())
        .is_some_and(|repo| repo == canonical_main)
}

/// The panic message a teardown test raises in place of a failed assertion.
pub const MANUFACTURED_FAILURE: &str = "manufactured assertion failure (expected by a teardown test)";

/// `unwound` is the unwind of [`MANUFACTURED_FAILURE`]: teardown neither
/// replaced the failure nor aborted, and nothing failed before it.
pub fn assert_manufactured_failure(unwound: std::thread::Result<()>) {
    let payload = unwound.expect_err("the test body panicked");
    let message = payload
        .downcast_ref::<String>()
        .map(String::as_str)
        .or_else(|| payload.downcast_ref::<&str>().copied())
        .unwrap_or("<not a string>");
    assert_eq!(message, MANUFACTURED_FAILURE, "the original failure reached the caller unchanged");
}

/// Whether process `pid` is still running (a zombie is not).
pub fn process_running(pid: u32) -> bool {
    use sysinfo::{Pid, ProcessRefreshKind, ProcessStatus, ProcessesToUpdate, System};

    let pid = Pid::from_u32(pid);
    let mut system = System::new();
    system.refresh_processes_specifics(ProcessesToUpdate::Some(&[pid]), true, ProcessRefreshKind::nothing());
    system.process(pid).is_some_and(|process| process.status() != ProcessStatus::Zombie)
}

/// Kills every refresh worker for `main`. They are detached, so nothing here
/// can reap them; [`reap_workers`] waits for them to disappear instead.
fn kill_refresh_workers(main: &Path) {
    let Ok(main) = fs::canonicalize(main) else {
        return;
    };
    let system = worker_processes();
    for process in system.processes().values() {
        if process.thread_kind().is_none() && is_worker_for(process, &main) {
            let _ = process.kill();
        }
    }
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

/// One row of a rendered `--perf` report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PerfRow {
    /// 0 for the `Performance` root, 1 for a top-level stage or group, 2 for
    /// a group's child.
    pub depth: usize,
    pub label: String,
    pub duration: Duration,
}

/// Every report row in rendered `--perf` output, in order. Lines that are not
/// report rows (the listing above the report) are skipped.
pub fn perf_rows(stderr: &str) -> Vec<PerfRow> {
    strip_ansi(stderr).lines().filter_map(perf_row).collect()
}

/// A report row: the quote border, then tree connectors (three columns per
/// level), the label, and the duration as the first token that parses as one.
fn perf_row(line: &str) -> Option<PerfRow> {
    let body = line.trim_start().strip_prefix('▌')?.trim_start_matches(' ');
    let connectors = body.chars().take_while(|c| matches!(c, '│' | '├' | '└' | '─' | ' ')).count();
    let rest: String = body.chars().skip(connectors).collect();
    let tokens: Vec<&str> = rest.split_whitespace().collect();
    let at = tokens.iter().position(|token| parse_perf_duration(token).is_some())?;
    (at > 0).then(|| PerfRow {
        depth: connectors.div_ceil(3),
        label: tokens[..at].join(" "),
        duration: parse_perf_duration(tokens[at]).expect("position found a duration"),
    })
}

/// The duration of the one row labeled exactly `stage`, at any depth.
///
/// Labels are matched whole, so a group such as `remote wait ‖ local gather`
/// is never read as its `remote wait` child. Panics when two rows share the
/// label, rather than picking one.
pub fn stage_from_perf(stderr: &str, stage: &str) -> Option<Duration> {
    let rows = perf_rows(stderr);
    let mut matching = rows.iter().filter(|row| row.label == stage);
    let found = matching.next()?;
    assert!(matching.next().is_none(), "two `{stage}` rows in the --perf report:\n{rows:#?}");
    Some(found.duration)
}

/// A local stand-in for an HTTPS proxy, so a PR request never leaves the host.
/// Dropping it closes every held connection and every later one.
pub struct ProxyStub {
    port: u16,
    connections: Arc<AtomicUsize>,
    held: Arc<Mutex<Vec<TcpStream>>>,
    released: Arc<AtomicBool>,
}

impl ProxyStub {
    /// Accepts every connection and never answers, so each request runs into
    /// its deadline. Connections are held open until
    /// [`ProxyStub::close_held`] or the stub is dropped.
    pub fn hanging() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind proxy stub");
        let port = listener.local_addr().expect("proxy stub address").port();
        let connections = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&connections);
        let held: Arc<Mutex<Vec<TcpStream>>> = Arc::default();
        let released: Arc<AtomicBool> = Arc::default();
        let (holder, gone) = (Arc::clone(&held), Arc::clone(&released));
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                hold_unless_released(&holder, &gone, stream);
                counter.fetch_add(1, Ordering::SeqCst);
            }
        });
        Self { port, connections, held, released }
    }

    /// Closes every connection held so far, so a request blocked on one
    /// fails at once instead of waiting for its deadline.
    pub fn close_held(&self) {
        close_all(&self.held);
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

    /// Accepts and counts every connection, and closes each one `hold`
    /// after accepting it: a request with a shorter deadline runs into its
    /// deadline, while a worker's requests fail soon after instead of holding
    /// the listing's whole wait.
    pub fn closing_after(hold: Duration) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind proxy stub");
        let port = listener.local_addr().expect("proxy stub address").port();
        let connections = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&connections);
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                counter.fetch_add(1, Ordering::SeqCst);
                std::thread::spawn(move || {
                    std::thread::sleep(hold);
                    let _ = stream.shutdown(std::net::Shutdown::Both);
                });
            }
        });
        Self { port, connections, held: Arc::default(), released: Arc::default() }
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
            released: Arc::default(),
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

impl HeldRequests for ProxyStub {
    fn release_held(&self) {
        self.close_held();
    }
}

impl Drop for ProxyStub {
    fn drop(&mut self) {
        release_all(&self.held, &self.released);
    }
}

/// Holds `stream` in `held`, or closes it at once once `released` is set.
/// Both sides decide under `held`'s lock, so no connection is held after
/// [`release_all`].
fn hold_unless_released(held: &Mutex<Vec<TcpStream>>, released: &AtomicBool, stream: TcpStream) {
    let mut held = held.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    if released.load(Ordering::SeqCst) {
        let _ = stream.shutdown(std::net::Shutdown::Both);
    } else {
        held.push(stream);
    }
}

fn close_all(held: &Mutex<Vec<TcpStream>>) {
    for stream in held.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).drain(..) {
        let _ = stream.shutdown(std::net::Shutdown::Both);
    }
}

/// Closes every held connection and makes every later one close at once.
fn release_all(held: &Mutex<Vec<TcpStream>>, released: &AtomicBool) {
    released.store(true, Ordering::SeqCst);
    close_all(held);
}

/// A loopback `origin` for git's smart-HTTP transport that accepts every
/// connection, records its request line, and never answers, so a live-head
/// `ls-remote` stays blocked until [`HoldingOrigin::close_held`] or its
/// deadline. Git reaches it directly: the caller clears the proxy variables
/// and the user's git configuration. Dropping it closes every held
/// connection and every later one; it is usually declared before the
/// fixture that names its URL, so pair it with a [`WorkerReaper`].
pub struct HoldingOrigin {
    port: u16,
    requests: Arc<Mutex<Vec<String>>>,
    held: Arc<Mutex<Vec<TcpStream>>>,
    released: Arc<AtomicBool>,
}

impl HoldingOrigin {
    pub fn new() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind holding origin");
        let port = listener.local_addr().expect("holding origin address").port();
        let requests: Arc<Mutex<Vec<String>>> = Arc::default();
        let held: Arc<Mutex<Vec<TcpStream>>> = Arc::default();
        let released: Arc<AtomicBool> = Arc::default();
        let (recorder, holder, gone) = (Arc::clone(&requests), Arc::clone(&held), Arc::clone(&released));
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
                hold_unless_released(&holder, &gone, stream);
                recorder.lock().expect("recorded requests").push(line);
            }
        });
        Self { port, requests, held, released }
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
        close_all(&self.held);
    }
}

impl Default for HoldingOrigin {
    fn default() -> Self {
        Self::new()
    }
}

impl HeldRequests for HoldingOrigin {
    fn release_held(&self) {
        self.close_held();
    }
}

impl Drop for HoldingOrigin {
    fn drop(&mut self) {
        release_all(&self.held, &self.released);
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

/// Which git smart-HTTP requests [`FakeGitea::hold_git`] keeps waiting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitHold {
    None,
    /// Every git request, which holds an `ls-remote` at its first request.
    All,
    /// Only a fetch's pack request (a body naming `command=fetch` or a
    /// `want`), so an `ls-remote` passes and the fetch after it waits.
    Fetch,
}

#[derive(Debug)]
struct GiteaState {
    requests: usize,
    branch_requests: usize,
    branch_status: u16,
    /// With status 200, the head a branch-head request answers.
    branch_head: Option<String>,
    git_requests: usize,
    git_waiting: usize,
    git_hold: GitHold,
    git_root: Option<PathBuf>,
    waiting: usize,
    held: bool,
    branches_held: bool,
    reply: GiteaReply,
}

/// A local plain-HTTP Gitea stand-in. `wt` reaches it as
/// [`FakeGitea::ORIGIN`] through `HTTP_PROXY` (sniff maps a `gitea.` host to
/// the Gitea API), so no request leaves the host and no TLS is involved.
///
/// While [`FakeGitea::hold`] is in effect every PR request waits unanswered,
/// which is how a test blocks a detached worker mid-request with its lock
/// held. Dropping the server answers every waiting and later request with
/// 503 ([`HeldRequests::release_held`] does the same without dropping it).
///
/// The worker's live-head half asks for the default branch's head
/// (`/branches/`); that request is answered 404 (or the status given to
/// [`FakeGitea::answer_branch_heads_with`]) at once (or, after
/// [`FakeGitea::hold_branch_heads`], once released) and counted apart
/// ([`FakeGitea::branch_requests`]), so it never counts as a PR request. Its
/// `ls-remote` fallback is refused by the fixture's git config, unless the
/// test lets git through this server ([`FakeGitea::serve_repositories`]),
/// which then answers git's smart-HTTP requests itself.
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
            Mutex::new(GiteaState {
                requests: 0,
                branch_requests: 0,
                branch_status: 404,
                branch_head: None,
                git_requests: 0,
                git_waiting: 0,
                git_hold: GitHold::None,
                git_root: None,
                waiting: 0,
                held: false,
                branches_held: false,
                reply,
            }),
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

    /// Makes every branch-head request from now on wait until
    /// [`FakeGitea::release`], holding the worker's check without a connect
    /// timeout racing it (the request reaches this plain-HTTP server at once).
    pub fn hold_branch_heads(&self) {
        self.state().branches_held = true;
    }

    /// Answers branch-head requests with `status` instead of 404 (429 is a
    /// rate limit).
    pub fn answer_branch_heads_with(&self, status: u16) {
        self.state().branch_status = status;
    }

    /// Answers branch-head requests 200 with `sha` as the head, the way
    /// Gitea's branch API does: a successful API check.
    pub fn answer_branch_heads_at(&self, sha: &str) {
        let mut state = self.state();
        state.branch_status = 200;
        state.branch_head = Some(sha.to_string());
    }

    /// Serves git's smart-HTTP requests for `/<namespace>/<repo>.git` from
    /// the bare repositories under `root` through `git http-backend`, for a
    /// git whose `http.proxy` is [`FakeGitea::url`].
    pub fn serve_repositories(&self, root: &Path) {
        self.state().git_root = Some(root.to_path_buf());
    }

    /// Makes the git requests `hold` names wait from now on, until another
    /// `hold_git` or [`FakeGitea::release`] lets them through.
    pub fn hold_git(&self, hold: GitHold) {
        self.state().git_hold = hold;
        self.shared.1.notify_all();
    }

    /// Git requests received so far, answered or not.
    pub fn git_requests(&self) -> usize {
        self.state().git_requests
    }

    /// Waits up to `limit` until `count` git requests are held unanswered.
    pub fn wait_for_git_waiting(&self, count: usize, limit: Duration) -> bool {
        let deadline = Instant::now() + limit;
        let mut state = self.state();
        while state.git_waiting < count {
            let Some(left) = deadline.checked_duration_since(Instant::now()) else {
                return false;
            };
            state = self.shared.1.wait_timeout(state, left).expect("fake gitea state").0;
        }
        true
    }

    /// Runs `action` for every later request after it is received and before
    /// it is answered, while the requester waits.
    ///
    /// The action happens exactly between the worker's request and its
    /// answer, while the test thread is blocked in the listing; `action` runs
    /// on the server thread for that reason.
    pub fn before_reply(&self, action: impl Fn() + Send + 'static) {
        *self.before_reply.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(Box::new(action));
    }

    /// Answers waiting and later requests with `reply`.
    pub fn release(&self, reply: GiteaReply) {
        let mut state = self.state();
        state.held = false;
        state.branches_held = false;
        state.git_hold = GitHold::None;
        state.reply = reply;
        self.shared.1.notify_all();
    }

    /// PR requests received so far, answered or not.
    pub fn requests(&self) -> usize {
        self.state().requests
    }

    /// Branch-head requests received so far, answered or not.
    pub fn branch_requests(&self) -> usize {
        self.state().branch_requests
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

impl HeldRequests for FakeGitea {
    fn release_held(&self) {
        self.release(GiteaReply::Status(503));
    }
}

impl Drop for FakeGitea {
    fn drop(&mut self) {
        self.release_held();
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
    let request_line = head.split(|byte| *byte == b'\r').next().unwrap_or_default();
    let line = String::from_utf8_lossy(request_line).into_owned();
    if line.contains("/info/refs") || line.contains("/git-upload-pack ") {
        serve_git(stream, &line, head, shared);
        return;
    }
    if request_line.windows(10).any(|window| window == b"/branches/") {
        let (lock, changed) = shared;
        let mut state = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        state.branch_requests += 1;
        changed.notify_all();
        let deadline = Instant::now() + GITEA_HOLD_LIMIT;
        while state.branches_held {
            let Some(left) = deadline.checked_duration_since(Instant::now()) else {
                break;
            };
            state = changed.wait_timeout(state, left).unwrap_or_else(|poisoned| poisoned.into_inner()).0;
        }
        let status = state.branch_status;
        let body = match (&state.branch_head, status) {
            (Some(sha), 200) => serde_json::json!({ "name": "main", "commit": { "id": sha } }).to_string(),
            _ => r#"{"message":"branch not found"}"#.to_string(),
        };
        drop(state);
        let response = format!(
            "HTTP/1.1 {status} Fake\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
            body.len()
        );
        let _ = stream.write_all(response.as_bytes());
        return;
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

/// Answers one git smart-HTTP request through `git http-backend`, after
/// holding it while [`GitHold`] says so. `head` is everything read so far,
/// which may already hold the start of the body.
fn serve_git(mut stream: TcpStream, line: &str, mut head: Vec<u8>, shared: &(Mutex<GiteaState>, Condvar)) {
    let header_end = head.windows(4).position(|window| window == b"\r\n\r\n").map_or(head.len(), |at| at + 4);
    let mut body = head.split_off(header_end);
    let headers = String::from_utf8_lossy(&head).into_owned();
    let header = |name: &str| {
        headers.lines().find_map(|line| {
            let (key, value) = line.split_once(':')?;
            key.trim().eq_ignore_ascii_case(name).then(|| value.trim().to_string())
        })
    };
    let length: usize = header("content-length").and_then(|value| value.parse().ok()).unwrap_or(0);
    let mut buffer = [0_u8; 8192];
    while body.len() < length {
        match stream.read(&mut buffer) {
            Ok(0) | Err(_) => return,
            Ok(read) => body.extend_from_slice(&buffer[..read]),
        }
    }
    let fetch = body.windows(13).any(|window| window == b"command=fetch") || body.windows(5).any(|w| w == b"want ");

    let root = {
        let (lock, changed) = shared;
        let mut state = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        state.git_requests += 1;
        state.git_waiting += 1;
        changed.notify_all();
        let deadline = Instant::now() + GITEA_HOLD_LIMIT;
        while state.git_hold == GitHold::All || (state.git_hold == GitHold::Fetch && fetch) {
            let Some(left) = deadline.checked_duration_since(Instant::now()) else {
                break;
            };
            state = changed.wait_timeout(state, left).unwrap_or_else(|poisoned| poisoned.into_inner()).0;
        }
        state.git_waiting -= 1;
        changed.notify_all();
        state.git_root.clone()
    };
    let Some(root) = root else {
        let _ = stream.write_all(b"HTTP/1.1 503 Fake\r\ncontent-length: 0\r\nconnection: close\r\n\r\n");
        return;
    };

    // Through a proxy the request target is absolute: `GET http://host/path?query HTTP/1.1`.
    let mut words = line.split(' ');
    let method = words.next().unwrap_or_default();
    let target = words.next().unwrap_or_default();
    let path = target.split_once("://").map_or(target, |(_, rest)| rest.find('/').map_or("/", |at| &rest[at..]));
    let (path, query) = path.split_once('?').unwrap_or((path, ""));
    let mut backend = Command::new("git");
    backend
        .arg("http-backend")
        .env("GIT_PROJECT_ROOT", &root)
        .env("GIT_HTTP_EXPORT_ALL", "1")
        .env("PATH_INFO", path)
        .env("QUERY_STRING", query)
        .env("REQUEST_METHOD", method)
        .env("CONTENT_LENGTH", body.len().to_string())
        .env("REMOTE_ADDR", "127.0.0.1")
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null());
    for (name, variable) in
        [("content-type", "CONTENT_TYPE"), ("content-encoding", "HTTP_CONTENT_ENCODING"), ("git-protocol", "GIT_PROTOCOL")]
    {
        if let Some(value) = header(name) {
            backend.env(variable, value);
        }
    }
    let Ok(mut child) = backend.spawn() else {
        return;
    };
    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(&body);
    }
    let Ok(output) = child.wait_with_output() else {
        return;
    };
    // CGI output: headers (with an optional `Status:`), a blank line, the body.
    let cgi = output.stdout;
    let split = cgi.windows(4).position(|window| window == b"\r\n\r\n").map_or(cgi.len(), |at| at + 4);
    let (cgi_head, cgi_body) = cgi.split_at(split);
    let cgi_head = String::from_utf8_lossy(cgi_head);
    let mut status = "200 OK".to_string();
    let mut response = String::new();
    for line in cgi_head.lines().filter(|line| !line.is_empty()) {
        match line.split_once(':') {
            Some((key, value)) if key.eq_ignore_ascii_case("status") => status = value.trim().to_string(),
            _ => response.push_str(&format!("{line}\r\n")),
        }
    }
    let response =
        format!("HTTP/1.1 {status}\r\n{response}content-length: {}\r\nconnection: close\r\n\r\n", cgi_body.len());
    let _ = stream.write_all(response.as_bytes());
    let _ = stream.write_all(cgi_body);
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
