//! Level 2 tests for `wt list` and `wt list -v` terminal rendering.
//!
//! Verifies that the status table and verbose commit section render correctly
//! in a real terminal (tmux), both on a plain terminal and when the
//! image-capable graph path is exercised. The design test checks the table's
//! colors and emphasis cell by cell in a styled tmux capture. The stale scene
//! holds the worker's PR request past the 3 s wait and checks the dim age
//! item and hint beneath the legend, the cleared spinner, and a caption that
//! is no longer "still checking"; the failed-refresh scene checks the dim
//! `(couldn't refresh)` item. Two scenes resize the
//! pane to exactly 99 and 100 columns to prove the counts' width gate on the
//! terminal's own width, including a `-> parent` cell against a non-default
//! parent. Two scenes point `origin` at a local Gitea stand-in: one proves the
//! dim credentials warning beneath the caption, the other holds the check
//! past the 3 s wait to prove the spinner is drawn and then cleared before
//! the caption, and that the dim refresh hint follows the legend. Two more
//! let the pane's git reach a bare repository through the stand-in and hold
//! `wt list -r`'s worker in each spinner phase: the no-key fallback turning
//! into the fetch on the same line, and the rate-limited fallback, each
//! captured as one spinner line before it clears ahead of the caption. A
//! third holds the PR lock while `wt list -r` runs, then releases it with
//! nothing published, and proves that once the retry's head finishes after
//! showing the fallback, the spinner is back to one `updating` line. The
//! caption suffix's dim italic is checked in the design test. The graph as an
//! image-capable terminal draws it is tested in
//! `level2_graph_in_kitty.rs`.

mod perf_support;
mod styled_capture;

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use assert_cmd::cargo::cargo_bin;
use biscuit_test_harness::tmux::TmuxHarness;
use biscuit_test_harness::CapturedFrame;
use biscuit_test_harness::TerminalHarness;
use perf_support::{FakeGitea, GitHold, GiteaReply, NoRequest, WorkerPaths, reap_workers, wait_for_refresh_workers};
use serial_test::serial;
use styled_capture::{Color, StyledScreen};
use test_toolkit::{Backend, Level, require_level};
use worktree::fork_origin::{ForkOrigin, ForkOriginStore, fork_origin_path};
use worktree::pull_requests::{RefreshOutcome, pr_lock_path, pr_store_path, refresh, unix_now};
use worktree::remote_head::{refresh_lock_held, remote_head_store_path};

fn run_git(repo: &std::path::Path, args: &[&str]) {
    let status = Command::new("git")
        .current_dir(repo)
        .args(args)
        .status()
        .expect("git should be installed");
    assert!(status.success(), "git {:?} failed in {:?}", args, repo);
}

/// Create a temporary git repo with a main worktree and a linked feature
/// worktree as siblings (not nested), so `is_current` detection is
/// unambiguous. The feature worktree has at least one commit so `wt list -v`
/// has meaningful verbose data to render.
///
/// Main receives two commits before the worktree branches, so the merge-base
/// has an ancestor. A Mermaid gitGraph needs at least one `commit` on the
/// default branch before a `branch` directive; without it the diagram fails to
/// rasterize and no image bytes are emitted.
fn temp_repo_with_feature_worktree() -> (tempfile::TempDir, PathBuf) {
    let parent = tempfile::tempdir().expect("create parent temp dir");
    let repo_path = parent.path().join("main-repo");
    let wt_path = parent.path().join("wt-feature");

    fs::create_dir(&repo_path).unwrap();

    run_git(&repo_path, &["init", "-b", "main"]);
    run_git(&repo_path, &["config", "user.email", "test@example.com"]);
    run_git(&repo_path, &["config", "user.name", "Test User"]);
    run_git(&repo_path, &["config", "commit.gpgsign", "false"]);
    // Suppress background/detached git work so nextest leak detection
    // sees no lingering child processes after the test returns.
    run_git(&repo_path, &["config", "gc.auto", "0"]);
    run_git(&repo_path, &["config", "core.fsmonitor", "false"]);
    run_git(&repo_path, &["config", "core.commitGraph", "false"]);

    fs::write(repo_path.join("file.txt"), "1\n").unwrap();
    run_git(&repo_path, &["add", "."]);
    run_git(&repo_path, &["commit", "-m", "initial commit"]);

    fs::write(repo_path.join("file.txt"), "2\n").unwrap();
    run_git(&repo_path, &["add", "."]);
    run_git(&repo_path, &["commit", "-m", "second commit on main"]);

    run_git(
        &repo_path,
        &[
            "worktree",
            "add",
            wt_path.to_str().unwrap(),
            "-b",
            "feature-test",
        ],
    );

    // Advance main past the branch point so the graph has post-divergence
    // commits on the default branch.
    fs::write(repo_path.join("file.txt"), "3\n").unwrap();
    run_git(&repo_path, &["add", "."]);
    run_git(&repo_path, &["commit", "-m", "third commit on main"]);

    fs::write(wt_path.join("feature.txt"), "feature work\n").unwrap();
    run_git(&wt_path, &["add", "."]);
    run_git(&wt_path, &["commit", "-m", "add feature work"]);

    (parent, wt_path)
}

/// The redesigned table in a real terminal: the target and parent headers,
/// the base-repo row, the feature row with its dirty dot and merge answer,
/// and both legend lines.
fn assert_redesigned_table(frame: &CapturedFrame) {
    let plain = &frame.plain;
    for expected in [
        "-> parent",
        "base repo",
        "wt-feature",
        "merges cleanly into parent",
        "uncommitted source files",
    ] {
        assert!(plain.contains(expected), "expected {expected:?} in the table.\nplain:\n{plain}");
    }
    let feature_row = plain
        .lines()
        .find(|line| line.contains("wt-feature") && line.contains('│'))
        .unwrap_or_else(|| panic!("no feature row.\nplain:\n{plain}"));
    assert!(feature_row.contains("○ wt-feature"), "clean dot: {feature_row}");
    assert!(feature_row.contains("clean"), "merge answer: {feature_row}");

    // The current (feature) row: dim clean ring, bold name, highlighted row.
    let screen = StyledScreen::parse(&frame.raw);
    let row = screen.row_with(&["○ wt-feature", "│"]);
    screen.assert_span(row, "○", "dim", |s| s.dim);
    screen.assert_span(row, "wt-feature", "bold on the dark highlight", |s| {
        s.bold && s.bg_is(DARK_ROW_HIGHLIGHT)
    });
}

/// Capture the pane including scrollback history, because the table + graph
/// image + verbose section may exceed the visible pane height.
fn capture_with_scrollback(harness: &TmuxHarness) -> CapturedFrame {
    let session = harness.session_name().to_string();
    let output = Command::new("tmux")
        .args([
            "capture-pane", "-t", &session, "-p", "-e", "-S", "-200", "-E", "-",
        ])
        .output()
        .expect("tmux capture-pane should succeed");
    let raw = String::from_utf8_lossy(&output.stdout).into_owned();
    CapturedFrame::from_raw(raw)
}

/// `wt list -v` on a plain (non-image) terminal must render the status table
/// headers, verbose commit section, and SGR color codes.
#[test]
#[serial(level2_terminal)]
fn level2_list_verbose_renders_table_and_verbose_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let (repo, wt_path) = temp_repo_with_feature_worktree();
    let wt_display = wt_path.display().to_string();

    let mut harness = TmuxHarness::new();
    harness.spawn_shell().expect("spawn_shell failed");

    harness
        .send_text(format!("cd {wt_display}\n").as_bytes())
        .expect("send cd failed");
    std::thread::sleep(std::time::Duration::from_millis(200));

    // Use `env -u` to ensure no image-capable env vars leak from the parent
    // terminal, so the non-image verbose path is taken.
    let bin = cargo_bin("wt").display().to_string();
    let cmd = format!("env -u TERM_PROGRAM -u KITTY_WINDOW_ID FORCE_COLOR=1 COLORFGBG='15;0' {bin} list -v\n");
    harness.send_text(cmd.as_bytes()).expect("send_text failed");

    let _ = biscuit_test_harness::wait_for_prompt(&mut harness);
    std::thread::sleep(std::time::Duration::from_millis(200));

    let frame = capture_with_scrollback(&harness);

    assert_redesigned_table(&frame);
    assert!(
        frame.plain.contains("feature-test"),
        "expected 'feature-test' branch name in verbose output.\nplain:\n{}",
        frame.plain,
    );
    assert!(
        frame.plain.contains("add feature"),
        "expected verbose commit message in captured pane.\nplain:\n{}",
        frame.plain,
    );
    drop(repo);
}

/// `wt list -v` on an image-detected terminal must still render the status
/// table and verbose commit section. The graph gather path runs because
/// TERM_PROGRAM reports an image-capable emulator, but it must not suppress
/// the table or verbose text.
#[test]
#[serial(level2_terminal)]
fn level2_list_verbose_renders_with_graph_path_active() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let (repo, wt_path) = temp_repo_with_feature_worktree();
    let wt_display = wt_path.display().to_string();

    let mut harness = TmuxHarness::new();
    harness.spawn_shell().expect("spawn_shell failed");

    harness
        .send_text(format!("cd {wt_display}\n").as_bytes())
        .expect("send cd failed");
    std::thread::sleep(std::time::Duration::from_millis(200));

    // Force image-capable detection so the graph gather path executes.
    // tmux cannot display Kitty graphics, so Mermaid rendering falls back
    // silently — but the gather path still runs, and the table and verbose
    // section must remain visible.
    let bin = cargo_bin("wt").display().to_string();
    harness
        .send_command_with_env(
            &format!("{bin} list -v"),
            &[("TERM_PROGRAM", "ghostty"), ("FORCE_COLOR", "1"), ("COLORFGBG", "15;0")],
        )
        .expect("send_command_with_env failed");

    // Graph gather + Mermaid render/fallback may add latency.
    let _ = biscuit_test_harness::wait_for_prompt(&mut harness);
    std::thread::sleep(std::time::Duration::from_millis(1000));

    let frame = capture_with_scrollback(&harness);

    // Status table must survive the graph path.
    assert_redesigned_table(&frame);

    // Verbose section: the feature branch name must still appear.
    assert!(
        frame.plain.contains("feature-test"),
        "expected 'feature-test' in verbose section with graph path active.\nplain:\n{}",
        frame.plain,
    );

    drop(repo);
}

/// Row emphasis for a dark and a light background, as `styles_follow_the_design`
/// in `list_table.rs` expects. `COLORFGBG` picks the mode: inside tmux it is
/// the only signal `wt` reads before falling back to the host's appearance.
const DARK_ROW_HIGHLIGHT: Color = Color::Rgb(38, 42, 54);
const LIGHT_ROW_HIGHLIGHT: Color = Color::Rgb(234, 238, 246);
/// Dirty dots, the conflict connector, and the counts (`<yellow>`, `<red>`,
/// `<green>`). Source dots, `conflicts`, and `-N` share the basic red.
const YELLOW: Color = Color::Indexed(3);
const RED: Color = Color::Indexed(1);
const GREEN: Color = Color::Indexed(2);
/// A clean child's connector (`<gray-500>`).
const GRAY: Color = Color::Rgb(106, 114, 130);
/// Badge backgrounds: local branch, remote branch, and PR.
const LOCAL_BADGE: Color = Color::Rgb(25, 60, 184);
const REMOTE_BADGE: Color = Color::Rgb(93, 14, 192);
const PR_BADGE: Color = Color::Rgb(0, 96, 69);
const BADGE_TEXT: Color = Color::Indexed(7);

const GITHUB_ORIGIN: &str = "https://github.com/owner/repo.git";

/// A repository exercising every styled element of the table:
///
/// - `main-repo` on `main`, clean (dim ring), with `origin/main` one commit
///   ahead, so the caption and target header carry badges of both kinds.
/// - `wt-clash` (`clash`) conflicts with `main` (red connector).
/// - `wt-docs` (`docs-work`) holds an uncommitted Markdown file (yellow dot).
/// - `wt-feature` (`feature-test`) holds an uncommitted Rust file (red
///   dot), has an open PR in the stored answer, and is the current worktree.
///   Against `origin/main` it is one commit ahead and two behind, so its
///   target cell carries both counts; `wt-docs` is only behind.
///
/// All three branches have fork-origin records naming `main`, so they hang
/// from it in the Branch column. The stores live under `home`, which the pane
/// passes to `wt` as `HOME` and `XDG_CACHE_HOME`'s parent.
///
/// Every listing in a pane leaves a detached worker. Dropping the fixture
/// reaps it ([`reap_workers`]) before the temporary directory goes, so declare
/// a [`FakeGitea`] after the fixture: it drops first and ends its held
/// requests.
struct DesignFixture {
    _parent: tempfile::TempDir,
    home: PathBuf,
    main: PathBuf,
    feature: PathBuf,
    /// Set by [`Self::with_gitea_repository`]: the root holding `o/r.git`,
    /// which the pane's git reaches through the proxy.
    git_root: Option<PathBuf>,
}

impl DesignFixture {
    /// A fixture whose stored PR answer was fetched just now.
    fn new() -> Self {
        Self::with_pr_age(Duration::ZERO)
    }

    /// A fixture whose stored PR answer, bound to the current `origin`, was
    /// fetched `pr_age` ago.
    fn with_pr_age(pr_age: Duration) -> Self {
        Self::build(pr_age, false, GITHUB_ORIGIN)
    }

    /// [`Self::new`] with `origin` at [`FakeGitea::ORIGIN`]; pair it with a
    /// pane whose proxy is a [`FakeGitea`]. The listing's worker asks Gitea
    /// for open PRs, so the stored answer's badge shows only until Gitea
    /// answers.
    fn with_gitea_origin() -> Self {
        Self::with_gitea_pr_age(Duration::ZERO)
    }

    /// [`Self::with_gitea_origin`] whose stored PR answer was fetched
    /// `pr_age` ago.
    fn with_gitea_pr_age(pr_age: Duration) -> Self {
        Self::build(pr_age, false, FakeGitea::ORIGIN)
    }

    /// [`Self::with_gitea_origin`] plus a bare `o/r.git` whose `main` is one
    /// commit past the listed clone's `origin/main`, so a check answered
    /// through git differs from the tracking ref and the worker fetches it.
    /// The commit exists only in `o/r.git`, or the fetch would send no pack.
    /// Pair it with [`FakeGitea::serve_repositories`] on [`Self::git_root`];
    /// the pane then sends git's HTTP requests to the proxy instead of
    /// refusing them.
    fn with_gitea_repository() -> Self {
        let mut fixture = Self::with_gitea_origin();
        let root = fixture.home.join("gitea");
        let bare = root.join("o").join("r.git");
        fs::create_dir_all(&bare).unwrap();
        run_git(&bare, &["init", "--bare", "-b", "main"]);
        run_git(&fixture.main, &["push", "-q", bare.to_str().unwrap(), "refs/remotes/origin/main:refs/heads/main"]);
        let output = Command::new("git")
            .current_dir(&bare)
            .args(["-c", "user.name=Someone Else", "-c", "user.email=else@example.com"])
            .args(["commit-tree", "main^{tree}", "-p", "main", "-m", "pushed by someone else"])
            .output()
            .unwrap();
        assert!(output.status.success(), "commit-tree: {output:?}");
        let pushed = String::from_utf8(output.stdout).unwrap().trim().to_string();
        run_git(&bare, &["update-ref", "refs/heads/main", &pushed]);
        fixture.git_root = Some(root);
        fixture
    }

    fn git_root(&self) -> &std::path::Path {
        self.git_root.as_deref().expect("a fixture built with_gitea_repository")
    }

    /// [`Self::new`] plus `wt-child-work` (`child/long-descriptive-name`),
    /// forked from `feature-test` and recorded with it as parent. It is one
    /// commit ahead of `feature-test` and one behind, and its open PR #105
    /// targets `feature-test`, so its `-> parent` cell carries counts and a
    /// badge. The long branch name makes the table wider than 100 columns
    /// once the counts show, so that cell wraps at exactly 100.
    fn with_child() -> Self {
        Self::build(Duration::ZERO, true, GITHUB_ORIGIN)
    }

    fn build(pr_age: Duration, with_child: bool, origin: &str) -> Self {
        let parent = tempfile::tempdir().expect("create parent temp dir");
        let home = parent.path().join("home");
        let main = parent.path().join("main-repo");
        fs::create_dir_all(&home).unwrap();
        fs::create_dir(&main).unwrap();

        run_git(&main, &["init", "-b", "main"]);
        for (key, value) in [
            ("user.email", "test@example.com"),
            ("user.name", "Test User"),
            ("commit.gpgsign", "false"),
            ("gc.auto", "0"),
            ("core.fsmonitor", "false"),
            ("core.commitGraph", "false"),
        ] {
            run_git(&main, &["config", key, value]);
        }
        let commit = |dir: &std::path::Path, contents: &str, message: &str| {
            fs::write(dir.join("file.txt"), contents).unwrap();
            run_git(dir, &["add", "."]);
            run_git(dir, &["commit", "-m", message]);
        };
        commit(&main, "1\n", "initial commit");
        commit(&main, "2\n", "second commit on main");

        let sibling = |name: &str| parent.path().join(name);
        for (dir, branch) in [("wt-clash", "clash"), ("wt-docs", "docs-work"), ("wt-feature", "feature-test")] {
            run_git(&main, &["worktree", "add", sibling(dir).to_str().unwrap(), "-b", branch]);
        }
        commit(&main, "3\n", "third commit on main");
        commit(&main, "4\n", "fourth commit, pushed only");
        run_git(&main, &["update-ref", "refs/remotes/origin/main", "HEAD"]);
        run_git(&main, &["reset", "--hard", "HEAD~1"]);
        run_git(&main, &["remote", "add", "origin", origin]);

        commit(&sibling("wt-clash"), "clash\n", "clash with main");
        fs::write(sibling("wt-feature").join("feature.txt"), "feature work\n").unwrap();
        run_git(&sibling("wt-feature"), &["add", "."]);
        run_git(&sibling("wt-feature"), &["commit", "-m", "add feature work"]);
        if with_child {
            let child = sibling("wt-child-work");
            run_git(&main, &["worktree", "add", child.to_str().unwrap(), "-b", "child/long-descriptive-name", "feature-test"]);
            fs::write(child.join("child.txt"), "child work\n").unwrap();
            run_git(&child, &["add", "child.txt"]);
            run_git(&child, &["commit", "-m", "add child work"]);
            fs::write(sibling("wt-feature").join("more.txt"), "more feature work\n").unwrap();
            run_git(&sibling("wt-feature"), &["add", "more.txt"]);
            run_git(&sibling("wt-feature"), &["commit", "-m", "more feature work"]);
        }
        fs::write(sibling("wt-feature").join("lib.rs"), "fn uncommitted() {}\n").unwrap();
        fs::write(sibling("wt-docs").join("notes.md"), "uncommitted notes\n").unwrap();

        let fixture = Self {
            home,
            feature: sibling("wt-feature"),
            main,
            git_root: None,
            _parent: parent,
        };
        fixture.seed_stores(pr_age, with_child);
        fixture
    }

    /// The user cache directory `wt` resolves in the pane.
    fn cache_dir(&self) -> PathBuf {
        if cfg!(target_os = "macos") {
            self.home.join("Library").join("Caches")
        } else {
            self.home.join("cache")
        }
    }

    /// Store files are named by repository hash; take the name from the
    /// library and place it in the pane's cache directory.
    fn store_path(&self, real: PathBuf) -> PathBuf {
        let path = self.cache_dir().join("worktree").join(real.file_name().expect("store file name"));
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        path
    }

    fn pr_store(&self) -> PathBuf {
        self.store_path(pr_store_path(&self.main).expect("PR store path"))
    }

    fn seed_stores(&self, pr_age: Duration, with_child: bool) {
        let main = self.main.as_path();
        let base_sha = String::from_utf8(
            Command::new("git").current_dir(main).args(["rev-parse", "HEAD~1"]).output().unwrap().stdout,
        )
        .unwrap()
        .trim()
        .to_string();
        let mut store = ForkOriginStore::default();
        for (created_at, branch) in [(1, "clash"), (2, "docs-work"), (3, "feature-test")] {
            let origin = ForkOrigin { base_branch: "main".to_string(), base_sha: base_sha.clone(), created_at };
            store.insert(branch, origin);
        }
        if with_child {
            let feature_sha = String::from_utf8(
                Command::new("git").current_dir(main).args(["rev-parse", "feature-test~1"]).output().unwrap().stdout,
            )
            .unwrap()
            .trim()
            .to_string();
            let origin = ForkOrigin { base_branch: "feature-test".to_string(), base_sha: feature_sha, created_at: 4 };
            store.insert("child/long-descriptive-name", origin);
        }
        store
            .save_atomic(&self.store_path(fork_origin_path(main).expect("fork-origin path")))
            .expect("write fork-origin store");

        // Bound to this origin and naming its repository.
        let fetched_at = unix_now() - pr_age.as_secs();
        let origin = worktree::pull_requests::origin_url(main).expect("the fixture has an origin");
        let source_repo = if origin == FakeGitea::ORIGIN { "o/r" } else { "owner/repo" };
        let pr = |number: u32, source: &str, target: &str| {
            serde_json::json!({
                "number": number,
                "url": format!("https://example.invalid/{source_repo}/pull/{number}"),
                "source_repo": source_repo,
                "source_branch": source,
                "target_branch": target,
            })
        };
        let mut pull_requests = vec![pr(99, "feature-test", "main")];
        if with_child {
            pull_requests.push(pr(105, "child/long-descriptive-name", "feature-test"));
        }
        let prs = serde_json::json!({
            "format_version": worktree::pull_requests::PR_STORE_FORMAT_VERSION,
            "origin_digest": worktree::pull_requests::origin_digest(&origin),
            "fetched_at": fetched_at,
            "publication": worktree::remote_head::new_attempt_id().expect("a publication id"),
            "source_repo": source_repo,
            "pull_requests": pull_requests,
        });
        fs::write(self.pr_store(), serde_json::to_vec(&prs).unwrap()).expect("write PR store");

        // A live head matching `origin/main`, checked just now, so `wt` starts
        // no worker for the live head alone.
        let tracking_tip = String::from_utf8(
            Command::new("git").current_dir(main).args(["rev-parse", "origin/main"]).output().unwrap().stdout,
        )
        .unwrap()
        .trim()
        .to_string();
        let head = serde_json::json!({
            "format_version": 1,
            "origin_digest": worktree::pull_requests::origin_digest(&origin),
            "branch": "main",
            "sha": tracking_tip,
            "checked_at": unix_now(),
        });
        let head_store = self.store_path(remote_head_store_path(main).expect("remote-head store path"));
        fs::write(head_store, serde_json::to_vec(&head).unwrap()).expect("write remote-head store");
    }

    /// Runs `wt list` in the feature worktree with `COLORFGBG` set, and
    /// returns the pane once the legend has printed.
    ///
    /// Each call gets its own freshly spawned tmux session: `clear` does not
    /// reliably empty a tmux pane, so a reused pane can still show an earlier
    /// run's legend and satisfy this run's wait.
    fn list_in(&self, colorfgbg: &str) -> StyledScreen {
        // A refused proxy keeps every request off the network, and fails
        // the worker's PR request at once.
        self.list_until("list", colorfgbg, "http://127.0.0.1:9", "parent deleted")
    }

    /// [`Self::list_in`] running `wt {args}` with every request sent to
    /// `proxy`, returning the pane once it shows `ready`.
    fn list_until(&self, args: &str, colorfgbg: &str, proxy: &str, ready: &str) -> StyledScreen {
        self.list_in_pane(None, args, colorfgbg, proxy, ready)
    }

    /// `wt list` in a pane resized to exactly `cols` columns.
    fn list_at(&self, cols: u32) -> StyledScreen {
        self.list_in_pane(Some(cols), "list", "15;0", "http://127.0.0.1:9", "parent deleted")
    }

    /// Runs `wt {args}` in a fresh pane, `cols` wide when given. Without
    /// `cols` the pane keeps its spawn width, which must be at least 100
    /// because the counts in the comparison cells show only from there.
    fn list_in_pane(&self, cols: Option<u32>, args: &str, colorfgbg: &str, proxy: &str, ready: &str) -> StyledScreen {
        let mut harness = self.start_in_pane(cols, args, colorfgbg, proxy);
        StyledScreen::parse(&wait_for_pane(&mut harness, |plain| plain.contains(ready)).raw)
    }

    /// Starts `wt {args}` in a fresh pane, `cols` wide when given, with every
    /// HTTP and HTTPS request sent to `proxy`, and returns the pane without
    /// waiting for it.
    fn start_in_pane(&self, cols: Option<u32>, args: &str, colorfgbg: &str, proxy: &str) -> TmuxHarness {
        let mut harness = TmuxHarness::new();
        harness.spawn_shell().expect("spawn_shell failed");
        match cols {
            Some(cols) => {
                harness.resize(cols, 40).expect("resize the pane");
                assert_eq!(harness.pane_cols().expect("pane width"), cols, "the pane took the requested width");
            }
            None => {
                let cols = harness.pane_cols().expect("pane width");
                assert!(cols >= 100, "the pane is {cols} columns; the counts need at least 100");
            }
        }
        let bin = cargo_bin("wt").display().to_string();
        let home = self.home.display().to_string();
        let cache = self.home.join("cache").display().to_string();
        // No user or system git config (no `insteadOf` can send the worker's
        // `ls-remote` fallback to the real host), and git's HTTP transports
        // refused, since git honors `HTTPS_PROXY` too; with a served
        // repository, git's HTTP goes to the proxy instead.
        let empty_config = self.home.join("empty.gitconfig");
        fs::write(&empty_config, "").expect("write an empty global git config");
        let empty_config = empty_config.display().to_string();
        let git_http: [(&str, &str); 4] = match self.git_root {
            Some(_) => [
                ("GIT_CONFIG_KEY_0", "http.proxy"),
                ("GIT_CONFIG_VALUE_0", proxy),
                ("GIT_CONFIG_KEY_1", "protocol.http.allow"),
                ("GIT_CONFIG_VALUE_1", "always"),
            ],
            None => [
                ("GIT_CONFIG_KEY_0", "protocol.http.allow"),
                ("GIT_CONFIG_VALUE_0", "never"),
                ("GIT_CONFIG_KEY_1", "protocol.https.allow"),
                ("GIT_CONFIG_VALUE_1", "never"),
            ],
        };
        let mut env = vec![
            ("HOME", home.as_str()),
            ("XDG_CACHE_HOME", cache.as_str()),
            ("HTTPS_PROXY", proxy),
            ("HTTP_PROXY", proxy),
            ("FORCE_COLOR", "1"),
            ("COLORFGBG", colorfgbg),
            ("GIT_CONFIG_NOSYSTEM", "1"),
            ("GIT_CONFIG_GLOBAL", empty_config.as_str()),
            ("GIT_CONFIG_COUNT", "2"),
        ];
        env.extend(git_http);
        harness
            .send_text(format!("cd '{}'\n", self.feature.display()).as_bytes())
            .expect("send cd failed");
        harness
            .send_command_with_env(
                &format!(
                    "env -u TERM_PROGRAM -u KITTY_WINDOW_ID -u GH_TOKEN -u GITHUB_TOKEN -u GITEA_TOKEN \
                     -u FORGEJO_TOKEN -u CODEBERG_TOKEN {bin} {args}"
                ),
                &env,
            )
            .expect("send wt list failed");
        harness
    }
}

impl Drop for DesignFixture {
    fn drop(&mut self) {
        let in_cache = |real: PathBuf| real.file_name().map(|name| self.cache_dir().join("worktree").join(name));
        let paths = WorkerPaths {
            main: self.main.clone(),
            pr_store: pr_store_path(&self.main).ok().and_then(in_cache),
            head_store: remote_head_store_path(&self.main).ok().and_then(in_cache),
        };
        let finished = reap_workers(&paths, || {});
        if !std::thread::panicking() {
            assert!(finished, "a refresh worker outlived the test or kept a lock");
        }
    }
}

/// Polls the pane until `ready` holds for its visible text (15 s cap).
fn wait_for_pane(harness: &mut TmuxHarness, ready: impl Fn(&str) -> bool) -> CapturedFrame {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    loop {
        let frame = harness.capture().expect("capture failed");
        if ready(&frame.plain) {
            return frame;
        }
        assert!(std::time::Instant::now() < deadline, "pane never became ready.\nplain:\n{}", frame.plain);
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

/// `wt list`'s colors and emphasis in a real terminal: dirty dots, badge
/// backgrounds, tree connectors, and the current row's highlight, each
/// checked on the cells of its own glyph or word, together with the visible
/// row layout.
#[test]
#[serial(level2_terminal)]
fn level2_list_styles_follow_the_design_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let fixture = DesignFixture::new();

    let screen = fixture.list_in("15;0");
    let plain = screen.plain();

    // Caption: the local and remote badges around a yellow count. The pane
    // cannot reach origin, so the suffix dates the seeded answer. The `(`
    // tells the caption from the `--ff` suggestion, which a failed check
    // still prints below the graph with the same words.
    let caption = screen.row_with(&["main", "is", "behind", "origin/main", "("]);
    assert!(screen.text(caption).trim().starts_with("main  is 1 commit behind  origin/main  ("), "{plain}");
    // The suffix is part of the same sentence, word-wrapped to the pane.
    let unwrapped = plain.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(
        unwrapped.contains("origin/main (couldn't check origin; last checked with origin less than 1 min ago)"),
        "{plain}"
    );
    screen.assert_span(caption, " main ", "a local badge", |s| s.bg_is(LOCAL_BADGE) && s.fg_is(BADGE_TEXT));
    screen.assert_span(caption, " origin/main ", "a remote badge", |s| {
        s.bg_is(REMOTE_BADGE) && s.fg_is(BADGE_TEXT)
    });
    screen.assert_span(caption, "1 commit", "yellow", |s| s.fg_is(YELLOW));
    // Everything after the comparison is dim italic, and only that.
    screen.assert_span(caption, "behind", "neither dim nor italic", |s| !s.dim && !s.italic);
    screen.assert_span(caption, "(couldn't", "dim italic", |s| s.dim && s.italic);
    screen.assert_span(screen.row_with(&["ago)"]), "ago)", "dim italic", |s| s.dim && s.italic);

    // Header: the target is the remote badge.
    let header = screen.row_with(&["Worktree", "Branch", "->", "-> parent"]);
    screen.assert_span(header, " origin/main ", "a remote badge", |s| s.bg_is(REMOTE_BADGE));

    // Rows in tree order, with their visible glyphs.
    let base = screen.row_with(&["○ base repo", "│"]);
    let expected = [
        ("base repo", "○ base repo", " main "),
        ("wt-clash", "○ wt-clash", "├─ clash"),
        ("wt-docs", "● wt-docs", "├─ docs-work"),
        ("wt-feature", "● wt-feature", "└─ feature-test"),
    ];
    for (offset, (name, worktree_cell, branch_cell)) in expected.iter().enumerate() {
        let text = screen.text(base + offset);
        let cells: Vec<&str> = text.split('│').map(str::trim).collect();
        assert!(
            cells.len() >= 5 && cells[1] == *worktree_cell && cells[2].starts_with(branch_cell.trim()),
            "row {name}: {text:?}\n{plain}"
        );
    }
    let [clash, docs, feature] = [base + 1, base + 2, base + 3];

    // Worktree column: dim ring, yellow and red dots, bold current name.
    screen.assert_span(base, "○", "dim", |s| s.dim);
    screen.assert_span(base, "base repo", "dim italic", |s| s.dim && s.italic);
    screen.assert_span(docs, "●", "yellow", |s| s.fg_is(YELLOW));
    screen.assert_span(feature, "●", "red", |s| s.fg_is(RED));
    screen.assert_span(feature, "wt-feature", "bold", |s| s.bold);
    screen.assert_span(docs, "wt-docs", "not bold", |s| !s.bold);

    // Branch column: default branch badge, red conflict connector and cell,
    // gray clean connectors.
    screen.assert_span(base, " main ", "a local badge", |s| s.bg_is(LOCAL_BADGE));
    screen.assert_span(clash, "├─", "red", |s| s.fg_is(RED));
    screen.assert_span(clash, "conflicts", "red", |s| s.fg_is(RED));
    screen.assert_span(docs, "├─", "gray", |s| s.fg_is(GRAY));
    screen.assert_span(feature, "└─", "gray", |s| s.fg_is(GRAY));

    // The PR badge.
    screen.assert_span(feature, " PR #99 ", "a PR badge", |s| s.bg_is(PR_BADGE) && s.fg_is(BADGE_TEXT));

    // Counts: `+N` dim green and `-N` dim red after the state word, and before
    // the PR badge.
    let feature_text = screen.text(feature);
    let cells: Vec<&str> = feature_text.split('│').map(str::trim).collect();
    assert_eq!(cells[3], "clean +1 -2  PR #99", "{feature_text:?}\n{plain}");
    screen.assert_span(feature, "+1", "dim green", |s| s.dim && s.fg_is(GREEN));
    screen.assert_span(feature, "-2", "dim red", |s| s.dim && s.fg_is(RED));
    // Behind only: the zero side is omitted.
    screen.assert_span(docs, "clean -2 ", "", |_| true);
    assert!(!screen.text(docs).contains('+'), "{:?}", screen.text(docs));
    screen.assert_span(docs, "-2", "dim red", |s| s.dim && s.fg_is(RED));
    screen.assert_span(clash, "conflicts +1 -2", "", |_| true);
    screen.assert_span(clash, "+1", "dim green", |s| s.dim && s.fg_is(GREEN));
    screen.assert_span(clash, "-2", "dim red", |s| s.dim && s.fg_is(RED));
    screen.assert_span(clash, "conflicts", "red, not dim", |s| s.fg_is(RED) && !s.dim);

    // Row emphasis: every cell of the current row between the borders, and
    // no other row.
    let highlighted = |row: usize| {
        let cells = &screen.rows[row];
        let first = cells.iter().position(|c| c.ch == '│').unwrap();
        let last = cells.iter().rposition(|c| c.ch == '│').unwrap();
        cells[first + 1..last]
            .iter()
            .filter(|c| c.ch != '│')
            .all(|c| c.style.bg_is(DARK_ROW_HIGHLIGHT) || c.style.bg_is(PR_BADGE))
    };
    assert!(highlighted(feature), "the current row is highlighted: {:?}", screen.rows[feature]);
    for row in [base, clash, docs] {
        assert!(
            screen.rows[row].iter().all(|c| !c.style.bg_is(DARK_ROW_HIGHLIGHT)),
            "only the current row is highlighted: {:?}",
            screen.text(row)
        );
    }

    // Legend: the dim ring, both dots, and the Branch connectors.
    let legend = screen.row_with(&["Worktree", "uncommitted source files"]);
    screen.assert_span(legend, "○", "dim", |s| s.dim);
    // The worker's PR request was refused, so this run dates the stored
    // answer it shows; the item is the PR status, not part of the legend.
    let item = screen.row_with(&["PRs as of"]);
    assert_eq!(screen.text(item).trim(), "- PRs as of less than 1 min ago (couldn't refresh)", "{plain}");
    assert_eq!(item, screen.row_with(&["Branch", "parent deleted"]) + 1, "{plain}");
    let dots: Vec<_> = screen.rows[legend].iter().filter(|c| c.ch == '●').collect();
    assert!(dots.len() == 2 && dots[0].style.fg_is(YELLOW) && dots[1].style.fg_is(RED), "{dots:?}");
    let branch_legend = screen.row_with(&["Branch", "conflicts with parent"]);
    let connectors: Vec<_> = screen.rows[branch_legend].iter().filter(|c| matches!(c.ch, '└' | '├')).collect();
    assert!(
        connectors.len() == 3
            && connectors.iter().all(|c| c.ch == '└')
            && connectors[0].style.fg_is(GRAY)
            && connectors[1].style.fg_is(RED)
            && connectors[2].style.dim,
        "{connectors:?}"
    );

    // A light background switches the highlight to its light variant.
    let light = fixture.list_in("0;15");
    let feature = light.row_with(&["● wt-feature", "│"]);
    light.assert_span(feature, "wt-feature", "on the light highlight", |s| {
        s.bold && s.bg_is(LIGHT_ROW_HIGHLIGHT)
    });
}

/// `-w` sizes only the graph: a narrow `-w` in a pane of at least 100 columns
/// keeps the counts, because the gate reads the terminal's own width.
#[test]
#[serial(level2_terminal)]
fn level2_list_width_flag_leaves_the_counts_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let fixture = DesignFixture::new();
    let screen = fixture.list_until("list -w 40", "15;0", "http://127.0.0.1:9", "parent deleted");
    let plain = screen.plain();

    let feature = screen.row_with(&["● wt-feature", "│"]);
    let feature_text = screen.text(feature);
    let cells: Vec<&str> = feature_text.split('│').map(str::trim).collect();
    assert_eq!(cells[3], "clean +1 -2  PR #99", "{feature_text:?}\n{plain}");
    screen.assert_span(feature, "+1", "dim green", |s| s.dim && s.fg_is(GREEN));
}

/// A stale stored answer in a real terminal while the worker's PR request is
/// still held at the 3 s wait: its badge still shows, the dim age item is the
/// first status item (it follows the legend when, as in tmux, there is no
/// graph), and the dim refresh hint follows it. Only the PR half was left, so
/// the spinner said `updating`, is gone, and the caption says the check
/// ended rather than "still checking". Released, the request fails and the
/// worker stores nothing.
#[test]
#[serial(level2_terminal)]
fn level2_list_stale_pr_answer_shows_a_dim_age_line_in_tmux() {
    use biscuit_terminal::components::spinner::FRAMES;

    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let fixture = DesignFixture::with_gitea_pr_age(Duration::from_secs(12 * 60 + 5));
    let seeded = fs::read(fixture.pr_store()).expect("seeded store");
    // Plain HTTP reaches the stand-in at once, so no connect timeout races
    // the listing's 3 s wait.
    let gitea = FakeGitea::new(GiteaReply::Status(503));
    gitea.hold();

    let mut harness = fixture.start_in_pane(None, "list", "15;0", &gitea.url());
    let during = wait_for_pane(&mut harness, |plain| plain.contains("updating"));
    assert!(FRAMES.iter().any(|glyph| during.plain.contains(glyph)), "the spinner drew a frame:\n{}", during.plain);
    let screen =
        StyledScreen::parse(&wait_for_pane(&mut harness, |plain| unwrapped(plain).contains("force refresh immediately")).raw);
    let plain = screen.plain();

    let feature = screen.row_with(&["● wt-feature", "│"]);
    screen.assert_span(feature, " PR #99 ", "a PR badge", |s| s.bg_is(PR_BADGE));

    let legend_end = screen.row_with(&["Branch", "parent deleted"]);
    let age = screen.row_with(&["PRs as of"]);
    assert_eq!(age, legend_end + 1, "the age line follows the legend:\n{plain}");
    assert_eq!(screen.text(age).trim(), "- PRs as of 12 min ago", "{plain}");
    screen.assert_span(age, "PRs as of 12 min ago", "dim", |s| s.dim);
    let hint = screen.row_with(&["running this command again"]);
    assert_eq!(hint, age + 1, "the hint follows the PR item:\n{plain}");
    screen.assert_span(hint, "running this command again", "dim", |s| s.dim);

    for glyph in FRAMES {
        assert!(!plain.contains(glyph), "a spinner frame is left on the pane:\n{plain}");
    }
    assert!(!plain.contains("updating"), "the spinner's text is left on the pane:\n{plain}");
    let sentences = unwrapped(&plain);
    assert!(sentences.contains("(couldn't check origin;"), "the head half had ended:\n{plain}");
    assert!(!sentences.contains("still checking"), "{plain}");

    // The request is still held after the listing returned; fail it and
    // wait until the worker released both locks and exited.
    assert_eq!(gitea.waiting(), 1, "the PR request is still held:\n{plain}");
    gitea.release(GiteaReply::Status(503));
    assert_worker_gone(&fixture);
    assert_ne!(probe_refresh(&fixture), RefreshOutcome::Contended, "the PR lock is free");
    assert_eq!(gitea.requests(), 1, "one PR request, from the worker");
    assert_eq!(fs::read(fixture.pr_store()).expect("store"), seeded, "a failed refresh is never stored");
}

/// A failed assertion in a scene whose PR request is still held after the
/// listing returned: the pane, then the server (ending the request), then
/// the fixture (reaping the worker) drop, so no worker outlives the
/// fixture's directory, and the failure reaches the harness unchanged.
#[test]
#[serial(level2_terminal)]
fn level2_a_failed_assertion_in_a_held_pr_scene_still_reaps_the_worker() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let mut workers = Vec::new();
    let mut directory = None;
    let unwound = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let fixture = DesignFixture::with_gitea_pr_age(Duration::from_secs(12 * 60 + 5));
        let gitea = FakeGitea::new(GiteaReply::Status(503));
        gitea.hold();
        let mut harness = fixture.start_in_pane(None, "list", "15;0", &gitea.url());
        wait_for_pane(&mut harness, |plain| unwrapped(plain).contains("force refresh immediately"));
        assert_eq!(gitea.waiting(), 1, "the PR request is still held");
        workers = perf_support::refresh_workers(&fixture.main).iter().map(|worker| worker.pid).collect();
        directory = Some(fixture.main.clone());
        // Answered only after this stall: the worker outlives the release,
        // so only a teardown that waits for it can pass.
        gitea.before_reply(|| std::thread::sleep(Duration::from_secs(2)));
        panic!("{}", perf_support::MANUFACTURED_FAILURE);
    }));

    perf_support::assert_manufactured_failure(unwound);
    assert_eq!(workers.len(), 1, "the listing returned with its worker held");
    for pid in workers {
        assert!(!perf_support::process_running(pid), "worker {pid} outlived the fixture");
    }
    let directory = directory.expect("the fixture was built");
    assert!(!directory.exists(), "{directory:?} outlived the fixture");
}

/// A PR request that fails inside the wait leaves the stored badge and dates
/// it with one dim `(couldn't refresh)` item beneath the legend, and no hint:
/// both halves ended.
#[test]
#[serial(level2_terminal)]
fn level2_list_failed_pr_refresh_shows_a_dim_couldnt_refresh_item_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let fixture = DesignFixture::with_gitea_pr_age(Duration::from_secs(12 * 60 + 5));
    let seeded = fs::read(fixture.pr_store()).expect("seeded store");
    let gitea = FakeGitea::new(GiteaReply::Status(500));

    let screen = fixture.list_until("list", "15;0", &gitea.url(), "couldn't refresh");
    let plain = screen.plain();

    let feature = screen.row_with(&["● wt-feature", "│"]);
    screen.assert_span(feature, " PR #99 ", "a PR badge", |s| s.bg_is(PR_BADGE));
    let item = screen.row_with(&["PRs as of"]);
    assert_eq!(item, screen.row_with(&["Branch", "parent deleted"]) + 1, "the item follows the legend:\n{plain}");
    assert_eq!(screen.text(item).trim(), "- PRs as of 12 min ago (couldn't refresh)", "{plain}");
    screen.assert_span(item, "PRs as of 12 min ago (couldn't refresh)", "dim", |s| s.dim);
    assert!(!unwrapped(&plain).contains("running this command again"), "both halves ended:\n{plain}");

    assert_worker_gone(&fixture);
    assert_eq!(gitea.requests(), 1);
    assert_eq!(fs::read(fixture.pr_store()).expect("store"), seeded, "a failed refresh is never stored");
}

/// A refresh that makes no request: `Contended` while a worker holds the lock.
fn probe_refresh(fixture: &DesignFixture) -> RefreshOutcome {
    refresh(&fixture.pr_store(), &fixture.main, unix_now, |_| Box::new(NoRequest))
}

/// Whether a worker holds the live-head lock.
fn head_lock_held(fixture: &DesignFixture) -> bool {
    refresh_lock_held(&fixture.store_path(remote_head_store_path(&fixture.main).expect("remote-head store path")))
}


/// The child row of [`DesignFixture::with_child`] and its cells split on the
/// column rules; the Branch cell's guide is `└─`, so no extra `│` appears.
fn child_row(screen: &StyledScreen) -> (usize, Vec<String>) {
    let row = screen.row_with(&["○ wt-child-work", "│"]);
    let cells = screen.text(row).split('│').map(|cell| cell.trim().to_string()).collect();
    (row, cells)
}

/// A real 99-column pane keeps state words and PR badges in both comparison
/// cells but shows no counts, and nothing in the table is dim green.
#[test]
#[serial(level2_terminal)]
fn level2_list_hides_the_counts_in_a_99_column_pane() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let fixture = DesignFixture::with_child();
    let screen = fixture.list_at(99);
    let plain = screen.plain();

    let feature = screen.row_with(&["● wt-feature", "│"]);
    let feature_text = screen.text(feature);
    let feature_cells: Vec<&str> = feature_text.split('│').map(str::trim).collect();
    assert_eq!(feature_cells[3..5], ["clean  PR #99", "—"], "{feature_text:?}\n{plain}");
    screen.assert_span(feature, " PR #99 ", "a PR badge", |s| s.bg_is(PR_BADGE) && s.fg_is(BADGE_TEXT));

    let (child, cells) = child_row(&screen);
    assert_eq!(cells[3..5], ["clean", "clean  PR #105"], "{cells:?}\n{plain}");
    screen.assert_span(child, " PR #105 ", "a PR badge", |s| s.bg_is(PR_BADGE) && s.fg_is(BADGE_TEXT));
    let parent_rule = screen.text(child).rfind('│').unwrap();
    assert!(screen.text(child).find("PR #105").unwrap() < parent_rule, "the badge sits in -> parent\n{plain}");
    // No row wraps: the row after the child's is the table's bottom border.
    assert!(screen.text(child + 1).trim_start().starts_with('└'), "{plain}");

    let top = screen.row_with(&["Worktree", "Branch", "->", "-> parent"]);
    for row in top..=child {
        let text = screen.text(row);
        for token in ["+1", "+2", "-1", "-2"] {
            assert!(!text.contains(token), "no {token} at 99 columns: {text:?}\n{plain}");
        }
        assert!(screen.rows[row].iter().all(|c| !c.style.fg_is(GREEN)), "no green at 99 columns: {text:?}");
    }
}

/// A real 100-column pane shows the counts in both comparison cells, dim
/// green and dim red, before the PR badge. The child's `-> parent` cell
/// compares against its recorded parent `feature-test`, and at exactly 100
/// columns its badge wraps: `PR` stays on the row and `#105` continues on
/// the next line of the same cell, both on the badge background.
#[test]
#[serial(level2_terminal)]
fn level2_list_shows_target_and_parent_counts_in_a_100_column_pane() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let fixture = DesignFixture::with_child();
    let screen = fixture.list_at(100);
    let plain = screen.plain();

    let feature = screen.row_with(&["● wt-feature", "│"]);
    let feature_text = screen.text(feature);
    let feature_cells: Vec<&str> = feature_text.split('│').map(str::trim).collect();
    assert_eq!(feature_cells[3..5], ["clean +2 -2  PR #99", "—"], "{feature_text:?}\n{plain}");
    screen.assert_span(feature, "+2", "dim green", |s| s.dim && s.fg_is(GREEN));
    screen.assert_span(feature, "-2", "dim red", |s| s.dim && s.fg_is(RED));
    screen.assert_span(feature, " PR #99 ", "a PR badge", |s| s.bg_is(PR_BADGE) && s.fg_is(BADGE_TEXT));

    let (child, cells) = child_row(&screen);
    assert_eq!(cells[3..5], ["clean +2 -2", "clean +1 -1  PR"], "{cells:?}\n{plain}");
    // The first `+1` and `-1` on the row are the parent cell's.
    screen.assert_span(child, "+1", "dim green", |s| s.dim && s.fg_is(GREEN));
    screen.assert_span(child, "-1", "dim red", |s| s.dim && s.fg_is(RED));
    screen.assert_span(child, "+2", "dim green", |s| s.dim && s.fg_is(GREEN));
    screen.assert_span(child, " PR", "a PR badge", |s| s.bg_is(PR_BADGE) && s.fg_is(BADGE_TEXT));
    let text = screen.text(child);
    assert!(text.find("-1").unwrap() < text.find(" PR").unwrap(), "counts precede the badge: {text:?}");

    // The wrapped line: only the parent cell has content, and it is the rest
    // of the badge.
    let wrapped = screen.text(child + 1);
    let wrapped_cells: Vec<&str> = wrapped.split('│').map(str::trim).collect();
    assert_eq!(wrapped_cells[1..5], ["", "", "", "#105"], "{wrapped:?}\n{plain}");
    screen.assert_span(child + 1, "#105", "a PR badge", |s| s.bg_is(PR_BADGE) && s.fg_is(BADGE_TEXT));
    assert!(screen.text(child + 2).trim_start().starts_with('└'), "{plain}");
}

/// The whole sentence of a pane, with the word wrap undone.
fn unwrapped(plain: &str) -> String {
    plain.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Waits up to 20 s until no worker runs for the fixture and its live-head
/// lock is free.
fn assert_worker_gone(fixture: &DesignFixture) {
    let left = wait_for_refresh_workers(&fixture.main, 0, Duration::from_secs(20));
    assert!(left.is_empty(), "the worker outlived the test: {left:?}");
    assert!(!head_lock_held(fixture), "the live-head lock is free");
}

/// A confirmed credentials condition this run observed prints one dim line
/// directly beneath the caption, not italic like the caption's suffix. The
/// worker's branch-head request gets a 404 without a key, and git's fallback
/// is refused, so Gitea "did not show this repository".
#[test]
#[serial(level2_terminal)]
fn level2_list_credentials_warning_is_a_dim_line_beneath_the_caption_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let fixture = DesignFixture::with_gitea_origin();
    let gitea = FakeGitea::new(GiteaReply::Open(Vec::new()));

    let screen = fixture.list_until("list", "15;0", &gitea.url(), "parent deleted");
    let plain = screen.plain();
    let sentences = unwrapped(&plain);

    assert!(
        sentences.contains("origin/main (couldn't check origin; last checked with origin less than 1 min ago)"),
        "{plain}"
    );
    assert!(
        sentences.contains(
            "Gitea did not show this repository, and Git could not check it. If it is private, set GITEA_TOKEN"
        ),
        "{plain}"
    );
    let suffix_end = screen.row_with(&["ago)"]);
    let warning = screen.row_with(&["did not show this repository"]);
    assert_eq!(warning, suffix_end + 1, "the warning follows the caption:\n{plain}");
    screen.assert_span(warning, "did not show this repository", "dim, not italic", |s| s.dim && !s.italic);
    assert!(warning < screen.row_with(&["Worktree", "Branch"]), "the warning precedes the table:\n{plain}");
    assert!(!sentences.contains("running this command again"), "the worker finished, so no hint:\n{plain}");
    assert!(!plain.contains("GITEA_TOKEN="), "a variable is named, never assigned:\n{plain}");

    assert!(!sentences.contains("PRs as of") && !sentences.contains("couldn't get"), "the PRs answered:\n{plain}");

    assert_worker_gone(&fixture);
    assert!(gitea.branch_requests() >= 1, "the worker asked Gitea for the branch head");
    assert_eq!(gitea.requests(), 1, "and, like every listing, for open PRs");
}

/// With `origin`'s answer held past the 3 s wait, the spinner draws on the
/// pane while `wt list` waits, and its line is cleared before the caption:
/// the finished pane has no spinner glyph or text, the caption starts its
/// own row, and the dim refresh hint follows the legend.
#[test]
#[serial(level2_terminal)]
fn level2_list_clears_the_spinner_before_the_caption_and_shows_a_dim_hint_in_tmux() {
    use biscuit_terminal::components::spinner::FRAMES;

    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let fixture = DesignFixture::with_gitea_origin();
    let gitea = FakeGitea::new(GiteaReply::Open(Vec::new()));
    gitea.hold_branch_heads();

    let mut harness = fixture.start_in_pane(None, "list", "15;0", &gitea.url());
    let during = wait_for_pane(&mut harness, |plain| plain.contains("updating"));
    assert!(FRAMES.iter().any(|glyph| during.plain.contains(glyph)), "the spinner drew a frame:\n{}", during.plain);
    let screen = StyledScreen::parse(&wait_for_pane(&mut harness, |plain| plain.contains("parent deleted")).raw);
    let plain = screen.plain();
    let sentences = unwrapped(&plain);

    for glyph in FRAMES {
        assert!(!plain.contains(glyph), "a spinner frame is left on the pane:\n{plain}");
    }
    assert!(!plain.contains("updating"), "the spinner's text is left on the pane:\n{plain}");
    let caption = screen.row_with(&["main", "is", "behind", "origin/main"]);
    assert!(screen.text(caption).trim().starts_with("main  is 1 commit behind"), "the caption starts its row:\n{plain}");
    assert!(
        sentences.contains("(origin hasn't answered yet; still checking in the background;"),
        "the held check is still running:\n{plain}"
    );

    let hint = screen.row_with(&["running this command again"]);
    assert!(hint > screen.row_with(&["Branch", "parent deleted"]), "the hint follows the legend:\n{plain}");
    screen.assert_span(hint, "running this command again", "dim, not italic", |s| s.dim && !s.italic);
    assert!(sentences.contains("use the --refresh / -r flags to force refresh immediately"), "{plain}");

    gitea.release(GiteaReply::Open(Vec::new()));
    assert_worker_gone(&fixture);
    assert_eq!(gitea.requests(), 1, "like every listing, it asked for open PRs");
}

/// The spinner's texts (spec §3 step 3).
const SPINNER_TEXTS: [&str; 4] = [
    "updating",
    "no API key, using fallback method",
    "rate limited, using fallback method",
    "pulling remote updates",
];

/// The pane holds exactly one spinner line, it reads `<frame> {text}` with
/// nothing after it, and no other spinner text is anywhere on the pane.
fn assert_one_spinner_line(plain: &str, text: &str) {
    use biscuit_terminal::components::spinner::FRAMES;

    let lines: Vec<&str> =
        plain.lines().filter(|line| FRAMES.iter().any(|glyph| line.contains(glyph))).collect();
    assert_eq!(lines.len(), 1, "one spinner line:\n{plain}");
    let line = lines[0].trim_end();
    assert!(
        FRAMES.iter().any(|glyph| line == format!("{glyph} {text}")),
        "the spinner line is exactly the frame and {text:?}: {line:?}\n{plain}"
    );
    for other in SPINNER_TEXTS.iter().filter(|other| **other != text) {
        assert!(!plain.contains(other), "{other:?} is left on the pane:\n{plain}");
    }
}

/// The finished pane has no spinner frame and no spinner text other than the
/// caption's own `pulling remote updates` row, and its caption starts a row
/// with `expected`.
fn assert_spinner_cleared_before_caption(screen: &StyledScreen, expected: &str) {
    use biscuit_terminal::components::spinner::FRAMES;

    let plain = screen.plain();
    for glyph in FRAMES {
        assert!(!plain.contains(glyph), "a spinner frame is left on the pane:\n{plain}");
    }
    for text in &SPINNER_TEXTS[..3] {
        assert!(!plain.contains(text), "the spinner's {text:?} is left on the pane:\n{plain}");
    }
    let caption = (0..screen.rows.len())
        .find(|&row| screen.text(row).trim().starts_with(expected))
        .unwrap_or_else(|| panic!("no row starts with {expected:?}:\n{plain}"));
    assert!(caption < screen.row_with(&["Worktree", "Branch"]), "the caption precedes the table:\n{plain}");
}

/// With `origin`'s API answering 404 without a key and git served through the
/// proxy, the spinner shows the fallback while `ls-remote` is held, then,
/// once the check answers a new tip and the fetch is held, replaces it on
/// the same line with the shorter fetch text and no remnant of the longer
/// one. Released, the spinner line is gone and the caption describes the
/// fetched state.
#[test]
#[serial(level2_terminal)]
fn level2_list_spinner_moves_from_the_fallback_to_the_fetch_on_one_line_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let fixture = DesignFixture::with_gitea_repository();
    let gitea = FakeGitea::new(GiteaReply::Open(Vec::new()));
    gitea.serve_repositories(fixture.git_root());
    gitea.hold_git(GitHold::All);

    // `-r` waits for the worker, so the held phases stay on screen.
    let mut harness = fixture.start_in_pane(None, "list -r", "15;0", &gitea.url());
    assert!(gitea.wait_for_git_waiting(1, Duration::from_secs(15)), "the fallback's ls-remote reached git");
    let fallback = wait_for_pane(&mut harness, |plain| plain.contains("no API key, using fallback method"));
    assert_one_spinner_line(&fallback.plain, "no API key, using fallback method");
    assert!(gitea.branch_requests() >= 1, "the API was asked first");

    gitea.hold_git(GitHold::Fetch);
    let fetching = wait_for_pane(&mut harness, |plain| plain.contains("pulling remote updates"));
    assert_one_spinner_line(&fetching.plain, "pulling remote updates");
    assert!(gitea.wait_for_git_waiting(1, Duration::from_secs(15)), "the fetch is held");

    gitea.release(GiteaReply::Open(Vec::new()));
    let screen = StyledScreen::parse(&wait_for_pane(&mut harness, |plain| plain.contains("parent deleted")).raw);
    // The fetched commit puts `main` two behind, where it was one before.
    assert_spinner_cleared_before_caption(&screen, "main  is 2 commits behind");

    assert_worker_gone(&fixture);
}

/// A rate-limited API shows its own fallback text on one spinner line while
/// `ls-remote` is held, and the line is gone before the caption.
#[test]
#[serial(level2_terminal)]
fn level2_list_spinner_shows_the_rate_limited_fallback_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let fixture = DesignFixture::with_gitea_repository();
    let gitea = FakeGitea::new(GiteaReply::Open(Vec::new()));
    gitea.answer_branch_heads_with(429);
    gitea.serve_repositories(fixture.git_root());
    gitea.hold_git(GitHold::All);

    let mut harness = fixture.start_in_pane(None, "list -r", "15;0", &gitea.url());
    assert!(gitea.wait_for_git_waiting(1, Duration::from_secs(15)), "the fallback's ls-remote reached git");
    let fallback = wait_for_pane(&mut harness, |plain| plain.contains("rate limited, using fallback method"));
    assert_one_spinner_line(&fallback.plain, "rate limited, using fallback method");

    gitea.release(GiteaReply::Open(Vec::new()));
    let screen = StyledScreen::parse(&wait_for_pane(&mut harness, |plain| plain.contains("parent deleted")).raw);
    assert_spinner_cleared_before_caption(&screen, "main  is 2 commits behind");

    assert_worker_gone(&fixture);
}

/// `wt list -r`'s PR retry, in a real pane. The test holds the PR lock, as
/// a worker whose request is in flight would, so the listing's first worker
/// finds the PRs contended while its head shows the no-key fallback (its
/// `ls-remote` held), then fetches `origin`'s new commit, and the spinner
/// says `updating` while the wait follows the lock. The lock is then released
/// with nothing published, as when its holder fails, so the listing
/// relaunches; the replacement's head shows the fallback again while its
/// `ls-remote` is held, and once that head finishes with the replacement's
/// PR request still held at the stand-in, the spinner must go back to one
/// `updating` line rather than keep the finished fallback. Released, the
/// spinner is gone before the caption.
#[test]
#[serial(level2_terminal)]
fn level2_list_spinner_returns_to_updating_after_a_pr_retrys_head_finishes_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let fixture = DesignFixture::with_gitea_repository();
    let gitea = FakeGitea::new(GiteaReply::Open(Vec::new()));
    gitea.serve_repositories(fixture.git_root());
    gitea.hold();
    gitea.hold_git(GitHold::All);
    // `std`'s file lock is the same `flock` the worker's sidecar lock takes.
    let holder = fs::File::create(pr_lock_path(&fixture.pr_store())).expect("create the PR lock sidecar");
    holder.try_lock().expect("the test holds the PR lock");

    let mut harness = fixture.start_in_pane(None, "list -r", "15;0", &gitea.url());
    assert!(gitea.wait_for_git_waiting(1, Duration::from_secs(15)), "the first head's ls-remote reached git");
    let first = wait_for_pane(&mut harness, |plain| plain.contains("no API key, using fallback method"));
    assert_one_spinner_line(&first.plain, "no API key, using fallback method");

    // The first head fetches and finishes; the PRs are still contended.
    gitea.hold_git(GitHold::None);
    let waiting = wait_for_pane(&mut harness, |plain| plain.contains("updating"));
    assert_one_spinner_line(&waiting.plain, "updating");
    assert_eq!(gitea.requests(), 0, "the contended worker asked for no PRs");

    // The holder gives up with nothing published: the listing retries both
    // halves.
    gitea.hold_git(GitHold::All);
    drop(holder);
    assert!(gitea.wait_for_git_waiting(1, Duration::from_secs(15)), "the replacement's ls-remote reached git");
    let replacement = wait_for_pane(&mut harness, |plain| plain.contains("no API key, using fallback method"));
    assert_one_spinner_line(&replacement.plain, "no API key, using fallback method");

    // The replacement's head finishes; its PR request is still held.
    gitea.hold_git(GitHold::None);
    let pr_only = wait_for_pane(&mut harness, |plain| plain.contains("updating"));
    assert_one_spinner_line(&pr_only.plain, "updating");
    assert_eq!(gitea.requests(), 1, "the replacement asked for PRs");

    gitea.release(GiteaReply::Open(Vec::new()));
    let screen = StyledScreen::parse(&wait_for_pane(&mut harness, |plain| plain.contains("parent deleted")).raw);
    // The first head's fetch put `main` two behind; the replacement found it
    // current.
    assert_spinner_cleared_before_caption(&screen, "main  is 2 commits behind");

    assert_worker_gone(&fixture);
}
