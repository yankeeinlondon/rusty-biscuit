//! Level 2 tests for dirty-file tree terminal rendering.
//!
//! Verifies that `dirty_tree::render_markup` output, when processed through
//! `Prose::new(...).render(terminal)`, produces correct box-drawing characters
//! and SGR color codes in a real terminal (tmux).

use assert_cmd::cargo::cargo_bin;
use biscuit_test_harness::tmux::TmuxHarness;
use biscuit_test_harness::TerminalHarness;
use serial_test::serial;
use std::fs;
use std::process::Command;
use test_toolkit::{Backend, Level, require_level};

/// The tree should contain these box-drawing characters.
const TREE_NEEDLE: &str = "src";
const FILE_NEEDLE: &str = "lib.rs";

/// Returns a temporary git repository with a dirty worktree that contains
/// source files.
fn temp_repo_with_dirty_worktree() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("create temp dir");
    let path = dir.path();

    run_git(path, &["init"]);
    run_git(path, &["config", "user.email", "test@example.com"]);
    run_git(path, &["config", "user.name", "Test User"]);
    run_git(path, &["config", "commit.gpgsign", "false"]);

    fs::write(path.join("README.md"), "# test\n").unwrap();
    run_git(path, &["add", "README.md"]);
    run_git(path, &["commit", "-m", "initial"]);

    // Create a branch and worktree, then dirty it with a source file.
    run_git(path, &["branch", "feat-dirty"]);
    let wt_path = path.join("wt-dirty");
    run_git(path, &[
        "worktree", "add", wt_path.to_str().unwrap(), "feat-dirty",
    ]);
    // Place the dirty source file at the root so git status --porcelain
    // reports it directly (nested untracked dirs are collapsed to the dir).
    fs::write(wt_path.join("lib.rs"), "pub fn x() {}\n").unwrap();

    dir
}

fn run_git(repo: &std::path::Path, args: &[&str]) {
    let status = Command::new("git")
        .current_dir(repo)
        .args(args)
        .status()
        .expect("git should be installed");
    assert!(status.success(), "git {:?} failed in {:?}", args, repo);
}

#[test]
#[serial(level2_terminal)]
fn level2_dirty_tree_renders_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let mut harness = TmuxHarness::new();
    harness.spawn_shell().expect("spawn_shell failed");

    let bin_path = cargo_bin!("render_dirty_tree").display().to_string();
    harness
        .send_command_with_env(
            &bin_path,
            &[("FORCE_COLOR", "1")],
        )
        .expect("send_command_with_env failed");

    let _ = biscuit_test_harness::wait_for_prompt(&mut harness);
    std::thread::sleep(std::time::Duration::from_millis(200));

    let frame = harness.capture().expect("capture failed");

    // The plain text must contain the tree structure.
    assert!(
        frame.plain.contains(TREE_NEEDLE),
        "expected '{}' in captured pane.\nplain:\n{}",
        TREE_NEEDLE,
        frame.plain,
    );
    assert!(
        frame.plain.contains(FILE_NEEDLE),
        "expected '{}' in captured pane.\nplain:\n{}",
        FILE_NEEDLE,
        frame.plain,
    );

    // With FORCE_COLOR=1 and a color-capable tmux, the raw capture should
    // contain SGR escape sequences (the source file is colored red).
    assert!(
        frame.raw.contains("\x1b["),
        "expected SGR escapes in raw capture.\nraw:\n{}",
        frame.raw,
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_remove_dirty_worktree_shows_tree_and_prompt() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let repo = temp_repo_with_dirty_worktree();
    let repo_path = repo.path().display().to_string();

    let mut harness = TmuxHarness::new();
    harness.spawn_shell().expect("spawn_shell failed");

    // Navigate to the repo.
    harness
        .send_text(format!("cd {}\n", repo_path).as_bytes())
        .expect("send cd failed");
    std::thread::sleep(std::time::Duration::from_millis(200));

    // Run wt remove on the dirty worktree (no force = prompts).
    let wt_path = cargo_bin!("wt").display().to_string();
    harness
        .send_command_with_env(&format!("{} remove feat-dirty", wt_path), &[])
        .expect("send_command_with_env failed");

    // Wait for the interactive prompt to render.
    std::thread::sleep(std::time::Duration::from_millis(800));

    // Cancel the removal.
    harness.send_text(b"n\n").expect("send n failed");

    // Wait for the shell prompt to return.
    let _ = biscuit_test_harness::wait_for_prompt(&mut harness);
    std::thread::sleep(std::time::Duration::from_millis(200));

    let frame = harness.capture().expect("capture failed");

    // Verify the dirty tree is displayed.
    assert!(
        frame.plain.contains("lib.rs"),
        "expected 'lib.rs' in captured pane.\nplain:\n{}",
        frame.plain,
    );
    assert!(
        frame.plain.contains("source code files"),
        "expected 'source code files' warning in captured pane.\nplain:\n{}",
        frame.plain,
    );

    // The raw capture should contain SGR escapes for colored tree rendering.
    assert!(
        frame.raw.contains("\x1b["),
        "expected SGR escapes in raw capture.\nraw:\n{}",
        frame.raw,
    );
}
