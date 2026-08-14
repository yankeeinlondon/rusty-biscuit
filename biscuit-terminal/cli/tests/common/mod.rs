// Level-2 real-terminal test helpers for biscuit-terminal CLI.
//
// These tests exercise the `bt` binary inside a real terminal emulator
// (WezTerm, Kitty, or tmux) so that escape-sequence output, glyph
// widths, and scroll behaviour are validated against the actual
// terminal's display path.
//
// ## Skip-clean contract
//
// Every test checks `harness.available()` before spawning. When the
// required terminal is absent the test prints `skipping: requires <X>`
// to stderr and returns immediately. No `#[ignore]` markers are used.
// This keeps CI green on GitHub-hosted runners that lack WezTerm or
// Kitty.

#![allow(dead_code)]

use std::time::{Duration, Instant};

#[allow(unused_imports)]
pub use biscuit_test_harness::{CapturedFrame, TerminalHarness, skip_with_reason};

pub mod pane_geometry;

/// Absolute path to the Cargo-built `bt` binary, shaped for a POSIX shell.
///
/// Typing a bare `bt` into the pane resolves through the login shell's
/// `PATH`, which lacks the built binary on CI runners (`bt: command not
/// found`) and, worse, silently exercises a stale host-installed `bt` on
/// developer machines while the code under review still fails. Resolving via
/// [`biscuit_test_harness::bin_exe!`] also works when the suite runs from a
/// relocated nextest archive (no local `target/` build).
///
/// Only the parent directory is single-quoted (`'/…/debug/'bt`): the trailing
/// `bt` stays bare so the pane's command echo still ends in `bt <args>`, which
/// several tests anchor on (e.g. `find_row_of(plain, "bt image --debug")` and
/// the `contains("bt ")` echo filters). Full quoting would rewrite the echo to
/// `…/bt' <args>` and silently break those anchors.
pub fn bt_shell_path() -> &'static str {
    static BIN: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    BIN.get_or_init(|| {
        let path = biscuit_test_harness::bin_exe!("bt");
        let parent = path
            .parent()
            .expect("built bt binary has a parent directory")
            .display()
            .to_string();
        let name = path
            .file_name()
            .expect("built bt binary has a file name")
            .to_string_lossy()
            .into_owned();
        format!("'{}/'{name}", parent.replace('\'', "'\\''"))
    })
}

/// Sends a `bt` command to the harness and waits for the terminal to
/// settle.
///
/// `args` is the full argument string after `bt` — e.g. `"prose \"<red>x</red>\""`.
/// The command runs the freshly built binary via [`bt_shell_path`].
pub fn send_bt_command(harness: &mut impl TerminalHarness, args: &str) {
    let cmd = format!("{} {}\n", bt_shell_path(), args);
    harness.send_text(cmd.as_bytes()).expect("send_text failed");
    harness.settle();
}

/// Polls [`TerminalHarness::capture`] until `predicate` accepts a frame
/// or `timeout` elapses, returning the most recent frame either way.
///
/// Real-terminal renders (diagrams, images) finish anywhere from a few
/// tens of milliseconds to ~1 s on a cold cache. A blind
/// `sleep(worst_case)` before `capture()` pays the worst case on every
/// run; polling lets the fast path return as soon as the expected
/// evidence appears while still bounding the slow / failure path.
///
/// The caller still asserts on the returned frame — `predicate` only
/// decides *when to stop waiting*, never *whether the test passes*. A
/// timed-out poll returns the last frame so the caller's assertion
/// produces its normal diagnostic.
pub fn capture_until(
    harness: &mut impl TerminalHarness,
    timeout: Duration,
    predicate: impl Fn(&CapturedFrame) -> bool,
) -> CapturedFrame {
    let deadline = Instant::now() + timeout;
    let mut last = harness.capture().expect("capture failed");
    if predicate(&last) {
        return last;
    }
    while Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(100));
        if let Ok(frame) = harness.capture() {
            let satisfied = predicate(&frame);
            last = frame;
            if satisfied {
                break;
            }
        }
    }
    last
}
