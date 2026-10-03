//! Level 2 Ctrl+C through kitty's own key encoder while the exit drain waits
//! on an outbound message, for `compose`, `inline-compose`, `sequence`, and
//! the provider wrapper. No window is raised or focused.
//!
//! Each press is `kitty @ send-key ctrl+c`: kitty encodes the key for the
//! pane's current keyboard mode, as it does a physical press once the press
//! reaches kitty, and the pane's line discipline turns the byte into `SIGINT`
//! for the foreground process group. This is terminal-level evidence. The OS
//! input layer ahead of kitty is not involved, which is why the file is Level
//! 2; the OS keypress is `level3_drain_ctrl_c.rs`. The scenario lives in
//! `common::drain_interrupt`, shared with the tmux file
//! (`level2_drain_ctrl_c_tmux.rs`) and the L1 control-event tests.
//!
//! ## Backends
//!
//! macOS launches a private `KittyInstance` with `open -g`, which shows a
//! window but never takes focus. Elsewhere the tests need a host kitty session
//! with remote control (`KITTY_LISTEN_ON`), and open their window with
//! `--keep-focus`. Skip-clean without either. Run via `just test-l2`.

use crate::common::drain_interrupt::DrainCommand;
use crate::common::drain_interrupt::pane::assert_second_press_during_the_drain_exits_130;

use biscuit_test_harness::TerminalHarness;
use biscuit_test_harness::kitty::{KittyHarness, KittyInstance};
use test_toolkit::{Backend, Level, require_level};

fn kitty_available() -> bool {
    KittyInstance::can_launch() || KittyHarness::available()
}

fn second_press_through_kitty_exits_130(command: DrainCommand) {
    // Declared before the harness so it is dropped after it: the instance
    // owns the window the harness drives.
    let instance = KittyInstance::can_launch()
        .then(|| KittyInstance::launch(160, 50).expect("launch a private kitty"));
    let mut harness = match &instance {
        Some(instance) => instance.harness(),
        None => {
            let mut harness = KittyHarness::new();
            harness.spawn_shell().expect("spawn a kitty window");
            harness
        }
    };
    assert_second_press_during_the_drain_exits_130(&mut harness, command, |harness| {
        harness.send_key("ctrl+c").expect("kitty @ send-key ctrl+c");
    });
}

#[test]
fn level2_second_kitty_ctrl_c_during_the_compose_drain_exits_130() {
    require_level!(Level::L2, kitty_available(), Backend::Kitty);
    second_press_through_kitty_exits_130(DrainCommand::Compose);
}

#[test]
fn level2_second_kitty_ctrl_c_during_the_inline_compose_drain_exits_130() {
    require_level!(Level::L2, kitty_available(), Backend::Kitty);
    second_press_through_kitty_exits_130(DrainCommand::InlineCompose);
}

#[test]
fn level2_second_kitty_ctrl_c_during_the_sequence_drain_exits_130() {
    require_level!(Level::L2, kitty_available(), Backend::Kitty);
    second_press_through_kitty_exits_130(DrainCommand::Sequence);
}

#[test]
fn level2_second_kitty_ctrl_c_during_the_wrapper_drain_exits_130() {
    require_level!(Level::L2, kitty_available(), Backend::Kitty);
    second_press_through_kitty_exits_130(DrainCommand::Wrapper);
}
