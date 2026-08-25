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

/// Builds a shell-safe command for the nextest-provided bt binary.
pub fn bt_command(args: &str) -> String {
    let bin = biscuit_test_harness::bin_exe!("bt");
    let escaped = bin.to_string_lossy().replace('\'', "'\\''");
    format!("'{escaped}' {args}")
}

/// Finds the newest command-echo row where a `bt` subcommand marker finishes.
///
/// Real terminals can wrap the archived binary's absolute path at any byte,
/// including inside the `bt` filename. Joining a bounded number of preceding
/// rows keeps command-region assertions stable without selecting stale
/// scrollback from an earlier test.
pub fn find_bt_command_end(lines: &[&str], subcommand: &str) -> Option<usize> {
    let markers = [
        format!("bt {subcommand}"),
        format!("bt' {subcommand}"),
        format!("bt.exe' {subcommand}"),
    ];

    for end in (0..lines.len()).rev() {
        let start = end.saturating_sub(3);
        let prefix = lines[start..end]
            .iter()
            .map(|line| line.trim())
            .collect::<String>();
        let prefix_len = prefix.len();
        let joined = format!("{prefix}{}", lines[end].trim());
        if markers.iter().any(|marker| {
            joined
                .match_indices(marker)
                .any(|(index, value)| index + value.len() > prefix_len)
        }) {
            return Some(end);
        }
    }
    None
}

/// Sends a `bt` command to the harness and waits for the terminal to
/// settle.
///
/// `args` is the full argument string after `bt` — e.g. `"prose \"<red>x</red>\""`.
/// The binary path comes from nextest rather than the spawned login shell's
/// `PATH`, which keeps clean and archived test runs equivalent.
pub fn send_bt_command(harness: &mut impl TerminalHarness, args: &str) {
    let cmd = format!("{}\n", bt_command(args));
    harness.send_text(cmd.as_bytes()).expect("send_text failed");
    biscuit_test_harness::capture_settled(harness).expect("bt command did not settle");
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
