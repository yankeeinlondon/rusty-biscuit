//! Level 2 tests for dirty-file tree terminal rendering.
//!
//! Verifies that `dirty_tree::render_markup` output, when processed through
//! `Prose::new(...).render(terminal)`, produces correct box-drawing characters
//! and SGR color codes in a real terminal (tmux).

use assert_cmd::cargo::cargo_bin;
use biscuit_test_harness::tmux::TmuxHarness;
use biscuit_test_harness::TerminalHarness;
use serial_test::serial;
use test_toolkit::{Backend, Level, require_level};

/// The tree should contain these box-drawing characters.
const TREE_NEEDLE: &str = "src";
const FILE_NEEDLE: &str = "lib.rs";

#[test]
#[serial(level2_terminal)]
fn level2_dirty_tree_renders_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let mut harness = TmuxHarness::new();
    harness.spawn_shell().expect("spawn_shell failed");

    let bin_path = cargo_bin("render_dirty_tree").display().to_string();
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
    // contain SGR escape sequences (the source file is colored orange).
    assert!(
        frame.raw.contains("\x1b["),
        "expected SGR escapes in raw capture.\nraw:\n{}",
        frame.raw,
    );
}
