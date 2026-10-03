//! Level 3 OS-keyboard Ctrl+C during the post-provider windows the
//! 2026-09-23-opencode-failure-lifecycle fix changed: the orphan teardown
//! (F1) and terminal lifecycle work (F2).
//!
//! Each press is a synthesized macOS keyboard chord into a focused WezTerm
//! window, so WezTerm's input encoder and the tty's foreground-group delivery
//! both participate, as for a physical keypress. `level3_wrap_ctrl_c.rs`
//! covers one chord while the provider is still running; these enter the
//! teardown and the terminal lifecycle scope and press as many times as the
//! contract counts: once (no deadline), twice (grace or deferral), or three
//! times (a later press does not extend the grace). The repeat press during
//! teardown relies on the binary's `terminal-tests` teardown-hold seam. The
//! scenarios live in `common::terminal_interrupt`, shared with the tmux tier
//! (`level2_lifecycle_ctrl_c_tmux.rs`); precise timing and rung state stay in
//! the L1 `wrap_sigint.rs` tests and the `kill_process_group` unit test.
//!
//! macOS only, like `level3_wrap_ctrl_c.rs`: this area's L3 wrapper tests
//! inject through `cliclick`.
//!
//! These tests raise a GUI terminal and type into whatever holds focus. They
//! skip unless `RUN_LEVEL3=1`, and `just test-l3` refuses to start unattended
//! (no TTY and no `BISCUIT_L3_TAKE_FOCUS=1`).

use crate::common::terminal_interrupt::{
    assert_later_press_does_not_postpone_the_grace,
    assert_press_during_orphan_teardown_runs_the_lifecycle,
    assert_repeat_press_during_orphan_teardown_defers,
    assert_repeat_press_force_exits_long_lifecycle, assert_repeat_press_lets_short_lifecycle_finish,
    assert_single_press_lets_lifecycle_outlast_the_grace,
};

use biscuit_test_harness::wezterm::WezTermHarness;
use biscuit_test_harness::{SpawnVisibility, TerminalHarness, cliclick};
use serial_test::serial;
use test_toolkit::{Level, require_level};

/// A foreground WezTerm pane, killed on drop. WezTerm retitles the OS window
/// after the foreground program, so `claudine` is registered for the raise to
/// find it while the compose runs.
fn pane() -> WezTermHarness {
    let mut harness = WezTermHarness::new()
        .with_spawn_visibility(SpawnVisibility::Foreground)
        .with_expected_window_title("claudine");
    harness.spawn_shell().expect("spawn WezTerm shell pane");
    harness
}

/// A press closure that raises the window once, then clicks into it and sends
/// the Ctrl+C chord on every call.
fn keyboard_ctrl_c() -> impl FnMut(&mut WezTermHarness) {
    let mut coords = None;
    move |harness| {
        let (x, y) = *coords.get_or_insert_with(|| {
            harness
                .focus_spawned_pane()
                .expect("focus WezTerm pane (Accessibility grant + window-title match)")
                .expect("AXRaise yielded no window coordinates")
        });
        cliclick::click_then_ctrl_chord(x, y, "c").expect("inject OS Ctrl+C chord");
    }
}

fn keyboard_available() -> bool {
    WezTermHarness::available() && cliclick::available()
}

#[test]
#[serial(level3_keyboard)]
fn level3_ctrl_c_during_orphan_teardown_runs_failure_and_finalize() {
    require_level!(Level::L3, keyboard_available(), "WezTerm + cliclick");
    assert_press_during_orphan_teardown_runs_the_lifecycle(&mut pane(), keyboard_ctrl_c());
}

#[test]
#[serial(level3_keyboard)]
fn level3_repeat_ctrl_c_lets_a_short_terminal_lifecycle_finish() {
    require_level!(Level::L3, keyboard_available(), "WezTerm + cliclick");
    assert_repeat_press_lets_short_lifecycle_finish(&mut pane(), keyboard_ctrl_c());
}

#[test]
#[serial(level3_keyboard)]
fn level3_repeat_ctrl_c_force_exits_a_long_terminal_lifecycle() {
    require_level!(Level::L3, keyboard_available(), "WezTerm + cliclick");
    assert_repeat_press_force_exits_long_lifecycle(&mut pane(), keyboard_ctrl_c());
}

#[test]
#[serial(level3_keyboard)]
fn level3_repeat_ctrl_c_during_orphan_teardown_is_deferred() {
    require_level!(Level::L3, keyboard_available(), "WezTerm + cliclick");
    assert_repeat_press_during_orphan_teardown_defers(&mut pane(), keyboard_ctrl_c());
}

#[test]
#[serial(level3_keyboard)]
fn level3_single_ctrl_c_lets_a_terminal_lifecycle_outlast_the_grace() {
    require_level!(Level::L3, keyboard_available(), "WezTerm + cliclick");
    assert_single_press_lets_lifecycle_outlast_the_grace(&mut pane(), keyboard_ctrl_c());
}

#[test]
#[serial(level3_keyboard)]
fn level3_third_ctrl_c_does_not_postpone_the_grace() {
    require_level!(Level::L3, keyboard_available(), "WezTerm + cliclick");
    assert_later_press_does_not_postpone_the_grace(&mut pane(), keyboard_ctrl_c());
}
