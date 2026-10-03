//! Level 3 OS keyboard Ctrl+C while the exit drain waits on an outbound
//! message, for `compose`, `inline-compose`, `sequence`, and the provider
//! wrapper (Linux).
//!
//! Each press is an XTEST key event on a private X display
//! (`biscuit_test_harness::xvfb::XvfbKitty`): the test starts its own `Xvfb`,
//! runs kitty on it, gives kitty input focus on that display, and presses
//! Ctrl+C there. The event enters the X server where a physical keyboard's
//! events enter, kitty encodes it, and the pane's line discipline turns it into
//! `SIGINT` for the foreground process group. The host user's display and
//! focus are never touched, so these tests may run unattended. The scenario
//! lives in `common::drain_interrupt`, shared with the terminal-level files
//! (`level2_drain_ctrl_c_tmux.rs`, `level2_drain_ctrl_c_kitty.rs`) and the L1
//! control-event tests.
//!
//! ## Other operating systems
//!
//! macOS has no second, invisible desktop a process can create: key events go
//! to the key window of the one login session, so a real keypress would take
//! the user's focus. Windows `SendInput` reaches only the input desktop, which
//! is the user's; a desktop made with `CreateDesktop` receives no input until
//! `SwitchDesktop` shows it to the user. Neither has an isolated equivalent,
//! so this module is Linux-only. The Windows typed press is the L1
//! `lifecycle_message_drain_console_windows.rs`, which writes ETX into a
//! windowless console.
//!
//! Skip-clean without `Xvfb` and `kitty`, and unless `RUN_LEVEL3=1`.

use crate::common::drain_interrupt::DrainCommand;
use crate::common::drain_interrupt::pane::assert_second_press_during_the_drain_exits_130;

use biscuit_test_harness::xvfb::XvfbKitty;
use test_toolkit::{Level, require_level};

const REQUIREMENT: &str = "Xvfb + kitty (private X display)";

fn second_os_press_exits_130(command: DrainCommand) {
    let instance = XvfbKitty::launch(160, 50).expect("launch kitty on a private X display");
    let mut harness = instance.harness();
    assert_second_press_during_the_drain_exits_130(&mut harness, command, |_| {
        instance.press_ctrl('c').expect("XTEST Ctrl+C on the private display");
    });
}

#[test]
fn level3_second_os_ctrl_c_during_the_compose_drain_exits_130() {
    require_level!(Level::L3, XvfbKitty::can_launch(), REQUIREMENT);
    second_os_press_exits_130(DrainCommand::Compose);
}

#[test]
fn level3_second_os_ctrl_c_during_the_inline_compose_drain_exits_130() {
    require_level!(Level::L3, XvfbKitty::can_launch(), REQUIREMENT);
    second_os_press_exits_130(DrainCommand::InlineCompose);
}

#[test]
fn level3_second_os_ctrl_c_during_the_sequence_drain_exits_130() {
    require_level!(Level::L3, XvfbKitty::can_launch(), REQUIREMENT);
    second_os_press_exits_130(DrainCommand::Sequence);
}

#[test]
fn level3_second_os_ctrl_c_during_the_wrapper_drain_exits_130() {
    require_level!(Level::L3, XvfbKitty::can_launch(), REQUIREMENT);
    second_os_press_exits_130(DrainCommand::Wrapper);
}
