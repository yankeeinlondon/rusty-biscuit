//! Level 2 tests for `wt list` and `wt list -v` terminal rendering.
//!
//! Verifies that the status table and verbose commit section render correctly
//! in a real terminal (tmux), both on a plain terminal and when the
//! image-capable graph path is exercised. The design test checks the table's
//! colors and emphasis cell by cell in a styled tmux capture. The graph as an
//! image-capable terminal draws it is tested in `level2_graph_in_kitty.rs`.

mod styled_capture;

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use assert_cmd::cargo::cargo_bin;
use biscuit_test_harness::tmux::TmuxHarness;
use biscuit_test_harness::CapturedFrame;
use biscuit_test_harness::TerminalHarness;
use serial_test::serial;
use styled_capture::{Color, StyledScreen};
use test_toolkit::{Backend, Level, require_level};
use worktree::fork_origin::{ForkOrigin, ForkOriginStore, fork_origin_path};
use worktree::pull_requests::pr_store_path;

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
/// Dirty dots and the conflict connector (`<orange>`, `<yellow>`, `<red>`).
const ORANGE: Color = Color::Rgb(255, 165, 0);
const YELLOW: Color = Color::Indexed(3);
const RED: Color = Color::Indexed(1);
/// A clean child's connector (`<gray-500>`).
const GRAY: Color = Color::Rgb(106, 114, 130);
/// Badge backgrounds: local branch, remote branch, and PR.
const LOCAL_BADGE: Color = Color::Rgb(25, 60, 184);
const REMOTE_BADGE: Color = Color::Rgb(93, 14, 192);
const PR_BADGE: Color = Color::Rgb(0, 96, 69);
const BADGE_TEXT: Color = Color::Indexed(7);

/// A repository exercising every styled element of the table:
///
/// - `main-repo` on `main`, clean (dim ring), with `origin/main` one commit
///   ahead, so the caption and target header carry badges of both kinds.
/// - `wt-clash` (`clash`) conflicts with `main` (red connector).
/// - `wt-docs` (`docs-work`) holds an uncommitted Markdown file (yellow dot).
/// - `wt-feature` (`feature-test`) holds an uncommitted Rust file (orange
///   dot), has an open PR in a fresh store, and is the current worktree.
///
/// All three branches have fork-origin records naming `main`, so they hang
/// from it in the Branch column. The stores live under `home`, which the pane
/// passes to `wt` as `HOME` and `XDG_CACHE_HOME`'s parent.
struct DesignFixture {
    _parent: tempfile::TempDir,
    home: PathBuf,
    feature: PathBuf,
}

impl DesignFixture {
    fn new() -> Self {
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
        run_git(&main, &["remote", "add", "origin", "https://github.com/owner/repo.git"]);

        commit(&sibling("wt-clash"), "clash\n", "clash with main");
        fs::write(sibling("wt-feature").join("feature.txt"), "feature work\n").unwrap();
        run_git(&sibling("wt-feature"), &["add", "."]);
        run_git(&sibling("wt-feature"), &["commit", "-m", "add feature work"]);
        fs::write(sibling("wt-feature").join("lib.rs"), "fn uncommitted() {}\n").unwrap();
        fs::write(sibling("wt-docs").join("notes.md"), "uncommitted notes\n").unwrap();

        let fixture = Self {
            home,
            feature: sibling("wt-feature"),
            _parent: parent,
        };
        fixture.seed_stores(&main);
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

    fn seed_stores(&self, main: &std::path::Path) {
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
        store
            .save_atomic(&self.store_path(fork_origin_path(main).expect("fork-origin path")))
            .expect("write fork-origin store");

        // Fetched just now, so `wt` makes no request.
        let now = worktree::pull_requests::unix_now();
        let prs = serde_json::json!({
            "format_version": 1,
            "fetched_at": now,
            "source_repo": "owner/repo",
            "pull_requests": [{
                "number": 99,
                "url": "https://github.com/owner/repo/pull/99",
                "source_repo": "owner/repo",
                "source_branch": "feature-test",
                "target_branch": "main",
            }],
        });
        fs::write(
            self.store_path(pr_store_path(main).expect("PR store path")),
            serde_json::to_vec(&prs).unwrap(),
        )
        .expect("write PR store");
    }

    /// Runs `wt list` in the feature worktree with `COLORFGBG` set, and
    /// returns the pane once the legend has printed.
    ///
    /// Each call gets its own freshly spawned tmux session: `clear` does not
    /// reliably empty a tmux pane, so a reused pane can still show an earlier
    /// run's legend and satisfy this run's wait.
    fn list_in(&self, colorfgbg: &str) -> StyledScreen {
        let mut harness = TmuxHarness::new();
        harness.spawn_shell().expect("spawn_shell failed");
        let harness = &mut harness;
        let bin = cargo_bin("wt").display().to_string();
        let home = self.home.display().to_string();
        let cache = self.home.join("cache").display().to_string();
        harness
            .send_text(format!("cd '{}'\n", self.feature.display()).as_bytes())
            .expect("send cd failed");
        // A refused proxy keeps any request off the network; the fresh store
        // means none is made.
        harness
            .send_command_with_env(
                &format!("env -u TERM_PROGRAM -u KITTY_WINDOW_ID -u GH_TOKEN -u GITHUB_TOKEN {bin} list"),
                &[
                    ("HOME", &home),
                    ("XDG_CACHE_HOME", &cache),
                    ("HTTPS_PROXY", "http://127.0.0.1:9"),
                    ("FORCE_COLOR", "1"),
                    ("COLORFGBG", colorfgbg),
                ],
            )
            .expect("send wt list failed");

        StyledScreen::parse(&wait_for_pane(harness, |plain| plain.contains("parent deleted")).raw)
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

    // Caption: the local and remote badges around a yellow count.
    let caption = screen.row_with(&["main", "is", "behind", "origin/main"]);
    assert_eq!(screen.text(caption).trim(), "main  is 1 commit behind  origin/main", "{plain}");
    screen.assert_span(caption, " main ", "a local badge", |s| s.bg_is(LOCAL_BADGE) && s.fg_is(BADGE_TEXT));
    screen.assert_span(caption, " origin/main ", "a remote badge", |s| {
        s.bg_is(REMOTE_BADGE) && s.fg_is(BADGE_TEXT)
    });
    screen.assert_span(caption, "1 commit", "yellow", |s| s.fg_is(YELLOW));

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

    // Worktree column: dim ring, yellow and orange dots, bold current name.
    screen.assert_span(base, "○", "dim", |s| s.dim);
    screen.assert_span(base, "base repo", "dim italic", |s| s.dim && s.italic);
    screen.assert_span(docs, "●", "yellow", |s| s.fg_is(YELLOW));
    screen.assert_span(feature, "●", "orange", |s| s.fg_is(ORANGE));
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

    // Legend: both dots, and the red conflict connector.
    let legend = screen.row_with(&["Worktree", "uncommitted source files"]);
    screen.assert_span(legend, "○", "dim", |s| s.dim);
    let dots: Vec<_> = screen.rows[legend].iter().filter(|c| c.ch == '●').collect();
    assert!(dots.len() == 2 && dots[0].style.fg_is(YELLOW) && dots[1].style.fg_is(ORANGE), "{dots:?}");

    // A light background switches the highlight to its light variant.
    let light = fixture.list_in("0;15");
    let feature = light.row_with(&["● wt-feature", "│"]);
    light.assert_span(feature, "wt-feature", "on the light highlight", |s| {
        s.bold && s.bg_is(LIGHT_ROW_HIGHLIGHT)
    });
}
