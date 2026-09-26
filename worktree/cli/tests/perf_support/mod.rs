//! Shared fixtures and `--perf` parsing for the cache SLA integration tests.
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
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use assert_cmd::cargo::cargo_bin;

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

    /// [`MixedFixture::wt_command`] with every HTTPS request sent through
    /// `proxy`, and no provider token.
    pub fn wt_command_via(&self, proxy: &ProxyStub) -> Command {
        let mut command = self.wt_command();
        for name in ["ALL_PROXY", "all_proxy", "NO_PROXY", "no_proxy", "GH_TOKEN", "GITHUB_TOKEN"] {
            command.env_remove(name);
        }
        command
            .env("HTTPS_PROXY", proxy.url())
            .env("https_proxy", proxy.url())
            .env_remove("TERM_PROGRAM")
            .env_remove("KITTY_WINDOW_ID");
        command
    }

    pub fn main(&self) -> &Path {
        &self.main
    }

    /// The PR store the spawned `wt` reads and writes. The cache directory
    /// follows `HOME` (macOS) or `XDG_CACHE_HOME` (Linux); on Windows it does
    /// not, so the real per-user path (keyed by this temporary repository) is
    /// used there.
    pub fn pr_store(&self) -> PathBuf {
        let real = worktree::pull_requests::pr_store_path(&self.main).expect("PR store path");
        if cfg!(windows) {
            return real;
        }
        let root = if cfg!(target_os = "macos") {
            self.home.path().join("Library").join("Caches")
        } else {
            self.xdg_cache.path().to_path_buf()
        };
        root.join("worktree").join(real.file_name().expect("store file name"))
    }

    /// Writes a PR store fetched `age` ago holding one open PR from
    /// `branch` into `main`.
    pub fn seed_pr_store(&self, age: Duration, number: u64, branch: &str) {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_secs();
        let store = self.pr_store();
        fs::create_dir_all(store.parent().expect("store dir")).expect("create store dir");
        let json = serde_json::json!({
            "format_version": 1,
            "fetched_at": now - age.as_secs(),
            "source_repo": "owner/repo",
            "pull_requests": [{
                "number": number,
                "url": format!("https://github.com/owner/repo/pull/{number}"),
                "source_repo": "owner/repo",
                "source_branch": branch,
                "target_branch": "main",
            }],
        });
        fs::write(&store, serde_json::to_vec(&json).expect("serialize")).expect("write store");
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
}

impl ProxyStub {
    /// Accepts every connection and never answers, so each request runs into
    /// its deadline. Connections are held open until the test process ends.
    pub fn hanging() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind proxy stub");
        let port = listener.local_addr().expect("proxy stub address").port();
        let connections = Arc::new(AtomicUsize::new(0));
        let counter = Arc::clone(&connections);
        let held: Arc<Mutex<Vec<TcpStream>>> = Arc::default();
        std::thread::spawn(move || {
            for stream in listener.incoming().flatten() {
                counter.fetch_add(1, Ordering::SeqCst);
                held.lock().expect("held connections").push(stream);
            }
        });
        Self { port, connections }
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
