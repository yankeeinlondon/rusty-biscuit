//! Level 2 Ctrl+C through a tmux pane while the exit drain waits on an
//! outbound message, for `compose`, `inline-compose`, `sequence`, and the
//! provider wrapper.
//!
//! `tmux send-keys C-c` encodes the key through tmux's key layer and writes
//! ETX into the pane; the pane's line discipline turns that into `SIGINT` for
//! the whole foreground process group. The L1
//! `lifecycle_message_drain_interrupt.rs` tests signal the process alone.
//! Headless, so it runs wherever tmux does, including CI; the kitty-encoded
//! press is `level2_drain_ctrl_c_kitty.rs` and the OS key press is
//! `level3_drain_ctrl_c.rs`. The scenario lives in `common::drain_interrupt`.
//! Run via `just test-l2`.

use crate::common::drain_interrupt::DrainCommand;
use crate::common::drain_interrupt::pane::assert_second_press_during_the_drain_exits_130;

use biscuit_test_harness::TerminalHarness;
use biscuit_test_harness::tmux::TmuxHarness;
use test_toolkit::{Backend, Level, require_level};

fn second_press_through_tmux_exits_130(command: DrainCommand) {
    let mut harness = TmuxHarness::new();
    harness.spawn_shell().expect("spawn tmux shell");
    harness.resize(160, 60).expect("resize tmux pane");
    assert_second_press_during_the_drain_exits_130(&mut harness, command, |harness| {
        harness.send_key("C-c").expect("send C-c");
    });
}

#[test]
fn level2_second_ctrl_c_during_the_compose_drain_exits_130() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    second_press_through_tmux_exits_130(DrainCommand::Compose);
}

#[test]
fn level2_second_ctrl_c_during_the_inline_compose_drain_exits_130() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    second_press_through_tmux_exits_130(DrainCommand::InlineCompose);
}

#[test]
fn level2_second_ctrl_c_during_the_sequence_drain_exits_130() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    second_press_through_tmux_exits_130(DrainCommand::Sequence);
}

#[test]
fn level2_second_ctrl_c_during_the_wrapper_drain_exits_130() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    second_press_through_tmux_exits_130(DrainCommand::Wrapper);
}
