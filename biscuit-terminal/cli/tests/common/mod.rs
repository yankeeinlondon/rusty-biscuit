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

use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

#[allow(unused_imports)]
pub use biscuit_test_harness::{CapturedFrame, TerminalHarness, skip_with_reason};

pub mod output_region;
pub mod pane_geometry;

pub use output_region::{OutputRow, marked_command, rows_between_markers};

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
///
/// Matching ignores whitespace: when the wrap falls on the space before the
/// subcommand, that space is indistinguishable from the row's blank padding.
pub fn find_bt_command_end(lines: &[&str], subcommand: &str) -> Option<usize> {
    let compact = |text: &str| text.chars().filter(|c| !c.is_whitespace()).collect::<String>();
    let subcommand = compact(subcommand);
    let markers = [
        format!("bt{subcommand}"),
        format!("bt'{subcommand}"),
        format!("bt.exe'{subcommand}"),
    ];

    for end in (0..lines.len()).rev() {
        let start = end.saturating_sub(3);
        let prefix = lines[start..end].iter().map(|line| compact(line)).collect::<String>();
        let prefix_len = prefix.len();
        let joined = format!("{prefix}{}", compact(lines[end]));
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

/// Types `line` into a cleared pane, run between unique output markers, and
/// returns the settled frame with the rows the command displayed. See
/// [`rows_between_markers`].
///
/// Panics with the last frame when the end marker does not appear within
/// 20 seconds.
fn run_line_rows(harness: &mut impl TerminalHarness, line: &str) -> (CapturedFrame, Vec<OutputRow>) {
    static NEXT_ID: AtomicUsize = AtomicUsize::new(0);
    // The pid keeps ids unique across the test processes sharing one pane.
    let id = format!("{}x{}", std::process::id(), NEXT_ID.fetch_add(1, Ordering::Relaxed));
    harness.send_text(b"clear\n").expect("send_text failed");
    harness.settle();
    harness
        .send_text(format!("{}\n", marked_command(line, &id)).as_bytes())
        .expect("send_text failed");
    let deadline = Instant::now() + Duration::from_secs(20);
    loop {
        let frame = biscuit_test_harness::capture_settled(harness).expect("capture failed");
        if let Some(rows) = rows_between_markers(&frame.raw, &frame.plain, &id) {
            return (frame, rows);
        }
        assert!(
            Instant::now() < deadline,
            "`{line}` did not display its output markers.\nplain:\n{}",
            frame.plain
        );
        std::thread::sleep(Duration::from_millis(100));
    }
}

/// The command's output rows as a frame, for checks written against a whole
/// frame: neither the echo nor a prompt can satisfy them.
pub fn output_frame(rows: &[OutputRow]) -> CapturedFrame {
    let join = |pick: fn(&OutputRow) -> &str| rows.iter().map(pick).collect::<Vec<_>>().join("\n");
    CapturedFrame { raw: join(|row| &row.raw), plain: join(|row| &row.plain) }
}

/// Runs `bt <args>` with `env` scoped to the command (`KEY='value'` prefixes,
/// as [`TerminalHarness::send_command_with_env`] types them) and returns the
/// rows it displayed once the pane settles.
pub fn run_bt_rows(
    harness: &mut impl TerminalHarness,
    args: &str,
    env: &[(&str, &str)],
) -> (CapturedFrame, Vec<OutputRow>) {
    let prefix: String = env
        .iter()
        .map(|(key, value)| format!("{key}='{}' ", value.replace('\'', "'\\''")))
        .collect();
    run_line_rows(harness, &format!("{prefix}{}", bt_command(args)))
}

/// Runs `bt <args>` like [`run_bt_rows`] and returns only its output rows,
/// as an [`output_frame`].
pub fn run_bt_output(harness: &mut impl TerminalHarness, args: &str, env: &[(&str, &str)]) -> CapturedFrame {
    output_frame(&run_bt_rows(harness, args, env).1)
}

/// Shows `bytes` in the pane with `cat` and returns the rows the terminal
/// displayed.
///
/// For a component the `bt` CLI has no flag for: the test renders it
/// in-process and this proves what a real terminal makes of those bytes.
pub fn display_bytes_rows(
    harness: &mut impl TerminalHarness,
    bytes: &[u8],
) -> (CapturedFrame, Vec<OutputRow>) {
    use std::io::Write as _;

    let mut file = tempfile::Builder::new()
        .prefix("l2show")
        .tempfile()
        .expect("create the display file");
    file.write_all(bytes).expect("write the display file");
    // A component's output carries no final newline; without one the end
    // marker would share its last row.
    if !bytes.ends_with(b"\n") {
        file.write_all(b"\n").expect("write the display file");
    }
    file.flush().expect("flush the display file");
    let path = file.path().to_string_lossy().replace('\'', "'\\''");
    run_line_rows(harness, &format!("cat '{path}'"))
}

/// A truecolor, hyperlink-capable terminal 60 columns wide, for rendering a
/// component in-process before [`display_bytes_rows`] shows its bytes.
pub fn styled_terminal() -> biscuit_terminal::terminal::Terminal {
    biscuit_terminal::terminal::Terminal::builder()
        .color_depth(biscuit_terminal::discovery::detection::ColorDepth::TrueColor)
        .osc_link_support(true)
        .width(60)
        .build()
}

/// The maximal runs of visible cells whose state is on, in display order.
///
/// `trim` drops whitespace at each run's edges: an attribute that changes
/// only glyph color or intensity is invisible on a blank cell, so a terminal
/// may report it there or not.
pub fn active_runs(cells: &[(char, bool)], trim: bool) -> Vec<String> {
    let mut runs = Vec::new();
    let mut current = String::new();
    for (ch, on) in cells {
        if *on {
            current.push(*ch);
        } else if !current.is_empty() {
            runs.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        runs.push(current);
    }
    if trim {
        runs = runs
            .into_iter()
            .map(|run| run.trim().to_string())
            .filter(|run| !run.is_empty())
            .collect();
    }
    runs
}

/// The OSC 8 destinations opened in a raw capture row, in order.
pub fn osc8_destinations(raw: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = raw;
    while let Some(start) = rest.find("\x1b]8;") {
        let body = &rest[start + 4..];
        let end = body
            .find(['\u{07}', '\x1b'])
            .unwrap_or(body.len());
        if let Some((_, uri)) = body[..end].split_once(';')
            && !uri.is_empty()
        {
            out.push(uri.to_string());
        }
        rest = &body[end..];
    }
    out
}

#[cfg(test)]
mod tests {
    use super::find_bt_command_end;

    #[test]
    fn finds_a_command_wrapped_at_the_space_before_the_subcommand() {
        let lines = [
            "bash-3.2$ '/Volumes/coding/wt/rusty-biscuit/fix-wt-ux/target/ci-local/debug/bt' ",
            "prose \"<hidden>x</hidden>\"",
            "<hidden>x</hidden>",
            "bash-3.2$",
        ];
        assert_eq!(find_bt_command_end(&lines, "prose"), Some(1));
    }

    #[test]
    fn finds_a_command_wrapped_inside_the_binary_name() {
        let lines = ["$ '/tmp/target/debug/b", "t' image --debug x.png", "out"];
        assert_eq!(find_bt_command_end(&lines, "image --debug"), Some(1));
    }
}
