//! Level 2 tests for `wt list` and `wt list -v` terminal rendering.
//!
//! Verifies that the status table and verbose commit section render correctly
//! in a real terminal (tmux), both on a plain terminal and when the
//! image-capable graph path is exercised. The Kitty-backed test runs the full
//! Mermaid -> SVG -> PNG -> Kitty graphics pipeline in a real image-capable
//! terminal and asserts the inline image protocol bytes are emitted while the
//! status table survives.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use assert_cmd::cargo::cargo_bin;
use biscuit_test_harness::kitty::KittyHarness;
use biscuit_test_harness::shared::SharedHarness;
use biscuit_test_harness::tmux::TmuxHarness;
use biscuit_test_harness::CapturedFrame;
use biscuit_test_harness::TerminalHarness;
use serial_test::serial;
use test_toolkit::{Backend, Level, require_level};

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
    let bin = cargo_bin!("wt").display().to_string();
    let cmd = format!("env -u TERM_PROGRAM -u KITTY_WINDOW_ID FORCE_COLOR=1 {bin} list -v\n");
    harness.send_text(cmd.as_bytes()).expect("send_text failed");

    let _ = biscuit_test_harness::wait_for_prompt(&mut harness);
    std::thread::sleep(std::time::Duration::from_millis(200));

    let frame = capture_with_scrollback(&harness);

    assert!(
        frame.plain.contains("Worktree") || frame.plain.contains("Branch"),
        "expected table headers in captured pane.\nplain:\n{}",
        frame.plain,
    );
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
    assert!(
        frame.raw.contains("\x1b["),
        "expected SGR escapes in raw capture.\nraw:\n{}",
        frame.raw,
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
    let bin = cargo_bin!("wt").display().to_string();
    harness
        .send_command_with_env(
            &format!("{bin} list -v"),
            &[("TERM_PROGRAM", "ghostty"), ("FORCE_COLOR", "1")],
        )
        .expect("send_command_with_env failed");

    // Graph gather + Mermaid render/fallback may add latency.
    let _ = biscuit_test_harness::wait_for_prompt(&mut harness);
    std::thread::sleep(std::time::Duration::from_millis(1000));

    let frame = capture_with_scrollback(&harness);

    // Status table must survive the graph path.
    assert!(
        frame.plain.contains("Worktree") || frame.plain.contains("Branch"),
        "expected table headers even with graph path active.\nplain:\n{}",
        frame.plain,
    );

    // Verbose section: the feature branch name must still appear.
    assert!(
        frame.plain.contains("feature-test"),
        "expected 'feature-test' in verbose section with graph path active.\nplain:\n{}",
        frame.plain,
    );

    assert!(
        frame.raw.contains("\x1b["),
        "expected SGR escapes in raw capture.\nraw:\n{}",
        frame.raw,
    );

    drop(repo);
}

/// Process-shared Kitty window reused across the Kitty-backed tests so the
/// 2-3 s spawn cost is paid once per test process.
static SHARED_KITTY: SharedHarness<KittyHarness> = SharedHarness::new();

/// `wt list` on a real image-capable terminal (Kitty) must emit the graph as
/// an inline image via the Kitty graphics protocol and must not suppress the
/// status table.
///
/// This is the strongest verification of the image-terminal graph path: unlike
/// the tmux spoofed-env test, it runs the full Mermaid -> SVG -> PNG -> Kitty
/// graphics pipeline inside a terminal that can actually display images, and
/// asserts the APC graphics bytes (`\x1b_G`) appear in the captured pane while
/// the status-table text survives alongside the image.
#[test]
#[serial(level2_terminal)]
fn level2_graph_emits_image_protocol_bytes_in_kitty() {
    require_level!(Level::L2, KittyHarness::available(), Backend::Kitty);

    let (repo, wt_path) = temp_repo_with_feature_worktree();
    let wt_display = wt_path.display().to_string();

    let mut guard = SHARED_KITTY
        .get_or_init(|| KittyHarness::shared_or_spawn().expect("attach/spawn kitty"));
    let harness = guard.as_mut().expect("shared Kitty harness present");

    harness.send_text(b"clear\n").expect("clear failed");
    harness.settle();

    harness
        .send_text(format!("cd {wt_display}\n").as_bytes())
        .expect("cd failed");
    harness.settle();

    let bin = cargo_bin!("wt").display().to_string();
    harness
        .send_text(format!("{bin} list\n").as_bytes())
        .expect("send_text failed");

    // Allow the Mermaid rasterization pipeline (SVG -> PNG) to complete.
    let _ = biscuit_test_harness::wait_for_prompt(harness);
    std::thread::sleep(std::time::Duration::from_millis(1000));

    let frame = harness.capture().expect("capture failed");

    // An inline graph image was emitted via the Kitty graphics protocol.
    assert!(
        frame.raw.contains("\x1b_G"),
        "expected Kitty graphics protocol bytes for the rendered graph.\nraw:\n{}",
        frame.raw,
    );

    // The status table was not suppressed by the image-rendering path.
    assert!(
        frame.plain.contains("Worktree") || frame.plain.contains("Branch"),
        "expected table headers alongside the graph image.\nplain:\n{}",
        frame.plain,
    );

    drop(repo);
}
