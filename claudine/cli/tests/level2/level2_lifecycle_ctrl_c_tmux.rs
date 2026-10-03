//! Level 2 Ctrl+C through a tmux pane during the post-provider windows the
//! 2026-09-23-opencode-failure-lifecycle fix changed: the orphan teardown
//! (F1) and terminal lifecycle work (F2).
//!
//! `tmux send-keys C-c` writes ETX into the pane; its line discipline turns
//! that into `SIGINT` for the whole foreground process group, the delivery a
//! keypress gets. The terminal emulator's input encoder does not participate,
//! so this is not OS keyboard verification — that is
//! `level3_lifecycle_ctrl_c.rs`. What this tier adds over the L1
//! `wrap_sigint.rs` tests is foreground-group delivery and the notices as the
//! terminal drew them, including the grace and force-exit notices.
//!
//! The scenarios live in `common::terminal_interrupt`, shared with L3.
//! Skip-clean on a host without tmux; run via `just test-l2`.

use crate::common::terminal_interrupt::{
    assert_later_press_does_not_postpone_the_grace,
    assert_press_during_orphan_teardown_runs_the_lifecycle,
    assert_repeat_press_during_orphan_teardown_defers,
    assert_repeat_press_force_exits_long_lifecycle, assert_repeat_press_lets_short_lifecycle_finish,
    assert_single_press_lets_lifecycle_outlast_the_grace,
};

use biscuit_test_harness::TerminalHarness;
use biscuit_test_harness::tmux::TmuxHarness;
use test_toolkit::{Backend, Level, require_level};

/// An owned session, killed on drop (on panic too), tall enough that the run's
/// output never scrolls out of the capture.
fn pane() -> TmuxHarness {
    let mut harness = TmuxHarness::new();
    harness.spawn_shell().expect("spawn tmux shell");
    harness.resize(160, 60).expect("resize tmux pane");
    harness
}

fn press_ctrl_c(harness: &mut TmuxHarness) {
    harness.send_key("C-c").expect("send C-c");
}

#[test]
fn level2_ctrl_c_during_orphan_teardown_runs_failure_and_finalize() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    assert_press_during_orphan_teardown_runs_the_lifecycle(&mut pane(), press_ctrl_c);
}

#[test]
fn level2_repeat_ctrl_c_lets_a_short_terminal_lifecycle_finish() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    assert_repeat_press_lets_short_lifecycle_finish(&mut pane(), press_ctrl_c);
}

#[test]
fn level2_repeat_ctrl_c_force_exits_a_long_terminal_lifecycle() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    assert_repeat_press_force_exits_long_lifecycle(&mut pane(), press_ctrl_c);
}

#[test]
fn level2_repeat_ctrl_c_during_orphan_teardown_is_deferred() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    assert_repeat_press_during_orphan_teardown_defers(&mut pane(), press_ctrl_c);
}

#[test]
fn level2_single_ctrl_c_lets_a_terminal_lifecycle_outlast_the_grace() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    assert_single_press_lets_lifecycle_outlast_the_grace(&mut pane(), press_ctrl_c);
}

#[test]
fn level2_third_ctrl_c_does_not_postpone_the_grace() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    assert_later_press_does_not_postpone_the_grace(&mut pane(), press_ctrl_c);
}
