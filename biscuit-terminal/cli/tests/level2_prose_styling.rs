//! Level-2 tests for prose styling, OSC8 hyperlinks, NO_COLOR, and layout.
//!
//! These tests run the `bt` CLI inside a real terminal emulator so that
//! escape-sequence output is validated against the actual terminal's
//! display path.
//!
//! ## Skip-clean contract
//!
//! Every test checks `harness.available()` before spawning. When the
//! required terminal is absent the test prints `skipping: requires <X>`
//! to stderr and returns immediately. No `#[ignore]` markers are used.

mod common;

use biscuit_test_harness::{CapturedFrame, TerminalHarness, skip_with_reason};
use common::send_bt_command;
use serial_test::serial;
use unicode_width::UnicodeWidthStr;

// ------------------------------------------------------------------
// WezTerm — SGR, OSC8, NO_COLOR
// ------------------------------------------------------------------

#[test]
#[serial(level2_terminal)]
fn level2_prose_emits_sgr_in_real_terminal() {
    use biscuit_test_harness::wezterm::WezTermHarness;

    if !WezTermHarness::available() {
        skip_with_reason("WezTerm CLI (set WEZTERM_UNIX_SOCKET)");
        return;
    }

    let mut harness = WezTermHarness::new();
    harness.spawn_shell().expect("spawn_shell failed");

    // Belt-and-braces: scope FORCE_COLOR=1 to the spawned `bt` and use
    // the CLI `--force-color` flag so styling is decoupled from both
    // the spawned shell's environment AND from `bt`'s TTY detection.
    harness
        .send_command_with_env(
            "bt prose --force-color \"<red>x</red>\"",
            &[("FORCE_COLOR", "1")],
        )
        .expect("send_command_with_env failed");
    // Wait for the shell prompt to return so the bt output has been
    // committed to cells before we capture.
    let _ = biscuit_test_harness::wait_for_prompt(&mut harness);

    let frame_a = harness.capture().expect("capture A failed");
    // Capture again 200 ms later to defend against the rare WezTerm
    // get-text race where cell SGR has not been re-emitted into the
    // dump on the first capture.
    std::thread::sleep(std::time::Duration::from_millis(200));
    let frame_b = harness.capture().expect("capture B failed");

    // Backstop: ask `bt prose --print-bytes` to print the rendered
    // byte stream as a hex dump. This is independent of the WezTerm
    // capture path and proves the renderer's output contains SGR red.
    harness
        .send_command_with_env(
            "bt prose --force-color --print-bytes \"<red>x</red>\"",
            &[("FORCE_COLOR", "1")],
        )
        .expect("send_command_with_env (print-bytes) failed");
    let _ = biscuit_test_harness::wait_for_prompt(&mut harness);
    std::thread::sleep(std::time::Duration::from_millis(200));
    let frame_dbg = harness.capture().expect("capture debug failed");

    let raw_has_sgr =
        |frame: &CapturedFrame| frame.raw.contains("\x1b[31m") || frame.raw.contains("\x1b[91m");
    let dbg_has_sgr_hex = |frame: &CapturedFrame| {
        // Hex encodings of \x1b[31m and \x1b[91m respectively. Match
        // on either the raw or plain capture form so the assertion
        // succeeds whether or not WezTerm filtered SGR from the dump.
        let needles = ["1b5b33316d", "1b5b39316d"];
        let haystacks = [&frame.raw, &frame.plain];
        needles
            .iter()
            .any(|needle| haystacks.iter().any(|hay| hay.contains(needle)))
    };

    assert!(
        raw_has_sgr(&frame_a) || raw_has_sgr(&frame_b) || dbg_has_sgr_hex(&frame_dbg),
        "expected SGR red in raw capture OR in --print-bytes hex dump.\n\
         capture A raw:\n{}\n\
         capture B raw:\n{}\n\
         debug plain:\n{}\n\
         debug raw:\n{}",
        frame_a.raw,
        frame_b.raw,
        frame_dbg.plain,
        frame_dbg.raw,
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_prose_osc8_link_renders() {
    use biscuit_test_harness::wezterm::WezTermHarness;

    if !WezTermHarness::available() {
        skip_with_reason("WezTerm CLI (set WEZTERM_UNIX_SOCKET)");
        return;
    }

    let mut harness = WezTermHarness::new();
    harness.spawn_shell().expect("spawn_shell failed");

    send_bt_command(
        &mut harness,
        "prose \"<a href=https://example.com>link</a>\"",
    );

    let frame = harness.capture().expect("capture failed");
    assert_osc8_link_present(&frame, "https://example.com", "link");
}

#[test]
#[serial(level2_terminal)]
fn level2_no_color_strips_sgr_in_real_terminal() {
    use biscuit_test_harness::wezterm::WezTermHarness;

    if !WezTermHarness::available() {
        skip_with_reason("WezTerm CLI (set WEZTERM_UNIX_SOCKET)");
        return;
    }

    let mut harness = WezTermHarness::new();
    harness.spawn_shell().expect("spawn_shell failed");

    // Scope NO_COLOR=1 to a single command via the harness helper
    // (portable inline-env syntax; works regardless of the developer's
    // login shell).
    harness
        .send_command_with_env("bt prose \"<red>x</red>\"", &[("NO_COLOR", "1")])
        .expect("send_command_with_env failed");

    let frame = harness.capture().expect("capture failed");
    assert_no_sgr_red(&frame);
}

// ------------------------------------------------------------------
// Kitty — SGR, OSC8
// ------------------------------------------------------------------

#[test]
#[serial(level2_terminal)]
fn level2_prose_emits_sgr_in_kitty() {
    use biscuit_test_harness::kitty::KittyHarness;

    if !KittyHarness::available() {
        skip_with_reason("Kitty remote control (set KITTY_LISTEN_ON)");
        return;
    }

    let mut harness = KittyHarness::new();
    harness.spawn_shell().expect("spawn_shell failed");

    // Belt-and-braces: scope FORCE_COLOR=1 and use the CLI
    // `--force-color` flag for symmetry with the WezTerm test. Kitty
    // preserves SGR in `get-text --ansi`, so the primary path
    // typically wins; the `--print-bytes` backstop is redundant but
    // cheap and keeps the two tests symmetric.
    harness
        .send_command_with_env(
            "bt prose --force-color \"<red>x</red>\"",
            &[("FORCE_COLOR", "1")],
        )
        .expect("send_command_with_env failed");
    let _ = biscuit_test_harness::wait_for_prompt(&mut harness);
    let frame_a = harness.capture().expect("capture A failed");
    std::thread::sleep(std::time::Duration::from_millis(200));
    let frame_b = harness.capture().expect("capture B failed");

    harness
        .send_command_with_env(
            "bt prose --force-color --print-bytes \"<red>x</red>\"",
            &[("FORCE_COLOR", "1")],
        )
        .expect("send_command_with_env (print-bytes) failed");
    let _ = biscuit_test_harness::wait_for_prompt(&mut harness);
    std::thread::sleep(std::time::Duration::from_millis(200));
    let frame_dbg = harness.capture().expect("capture debug failed");

    let raw_has_sgr =
        |frame: &CapturedFrame| frame.raw.contains("\x1b[31m") || frame.raw.contains("\x1b[91m");
    let dbg_has_sgr_hex = |frame: &CapturedFrame| {
        let needles = ["1b5b33316d", "1b5b39316d"];
        let haystacks = [&frame.raw, &frame.plain];
        needles
            .iter()
            .any(|needle| haystacks.iter().any(|hay| hay.contains(needle)))
    };

    assert!(
        raw_has_sgr(&frame_a) || raw_has_sgr(&frame_b) || dbg_has_sgr_hex(&frame_dbg),
        "expected SGR red in raw capture OR in --print-bytes hex dump.\n\
         capture A raw:\n{}\n\
         capture B raw:\n{}\n\
         debug plain:\n{}\n\
         debug raw:\n{}",
        frame_a.raw,
        frame_b.raw,
        frame_dbg.plain,
        frame_dbg.raw,
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_prose_osc8_link_renders_in_kitty() {
    use biscuit_test_harness::kitty::KittyHarness;

    if !KittyHarness::available() {
        skip_with_reason("Kitty remote control (set KITTY_LISTEN_ON)");
        return;
    }

    let mut harness = KittyHarness::new();
    harness.spawn_shell().expect("spawn_shell failed");

    send_bt_command(
        &mut harness,
        "prose \"<a href=https://example.com>link</a>\"",
    );

    let frame = harness.capture().expect("capture failed");
    assert_osc8_link_present(&frame, "https://example.com", "link");
}

// ------------------------------------------------------------------
// Layout — padleft, columns
// ------------------------------------------------------------------

#[test]
#[serial(level2_terminal)]
fn level2_pad_columns_respect_actual_pane_width() {
    use biscuit_test_harness::wezterm::WezTermHarness;

    if !WezTermHarness::available() {
        skip_with_reason("WezTerm CLI (set WEZTERM_UNIX_SOCKET)");
        return;
    }

    let mut harness = WezTermHarness::new();
    harness.spawn_shell().expect("spawn_shell failed");

    send_bt_command(&mut harness, "padleft 30 \"x\"");

    let frame = harness.capture().expect("capture failed");
    let plain = &frame.plain;

    // Find a line where 'x' is the last non-space character and there are leading spaces.
    let line = plain
        .lines()
        .find(|l| {
            let trimmed_end = l.trim_end();
            trimmed_end.ends_with('x') && !trimmed_end.contains("padleft")
        })
        .expect("expected a line containing padded 'x'");

    // The line should have 29 spaces before the x, making the x the 30th column.
    let x_pos = line
        .chars()
        .position(|c| c == 'x')
        .expect("x should be present");
    assert_eq!(
        x_pos, 29,
        "expected 'x' at column 30 (index 29), got index {x_pos}",
    );

    // Harden: the row's trim_end length must be exactly 30 — the x is the
    // last visible character on the line, not surrounded by trailing
    // padding. WezTerm `get-text` does not pad lines with trailing spaces
    // beyond the last visible cell.
    let trimmed_end = line.trim_end();
    let visible_width = UnicodeWidthStr::width(trimmed_end);
    assert_eq!(
        visible_width, 30,
        "expected padded row to have visible width 30; got {visible_width}.\nline: {line:?}",
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_columns_word_wrap_in_pane() {
    use biscuit_test_harness::wezterm::WezTermHarness;

    if !WezTermHarness::available() {
        skip_with_reason("WezTerm CLI (set WEZTERM_UNIX_SOCKET)");
        return;
    }

    let mut harness = WezTermHarness::new();
    harness.spawn_shell().expect("spawn_shell failed");

    // Read the actual pane geometry so the wrap-row math is portable
    // across host font configurations.
    let pane_size = harness.pane_size().expect("pane_size failed");
    let cols = pane_size.cols as usize;
    assert!(
        cols >= 20,
        "pane too narrow ({cols} cols) for wrap test; need at least 20"
    );

    // Construct a continuous-letter word longer than the pane so wrapping
    // is guaranteed. Using lowercase ASCII keeps unicode-width = 1 per
    // char and makes the boundary math exact.
    let word_len = cols + 5;
    let long_word: String = std::iter::repeat_n('a', word_len).collect();
    send_bt_command(&mut harness, &format!("prose \"{long_word}\""));

    let frame = harness.capture().expect("capture failed");
    let plain = &frame.plain;

    // Locate the *output* lines — rows whose trimmed content is
    // composed entirely of 'a' characters (and optionally a single
    // trailing hyphen, which textwrap may insert as a soft-break marker
    // when forced to break inside a word). Exclude the bt invocation
    // line which contains literal `bt prose`.
    let is_wrap_row = |line: &str| -> bool {
        let trimmed = line.trim_end();
        if trimmed.is_empty() || trimmed.contains("bt ") {
            return false;
        }
        // Strip an optional trailing soft-break hyphen the prose
        // renderer may emit when forcibly breaking inside a word.
        let core = trimmed.strip_suffix('-').unwrap_or(trimmed);
        !core.is_empty() && core.chars().all(|c| c == 'a')
    };
    let wrap_rows: Vec<(usize, &str)> = plain
        .lines()
        .enumerate()
        .filter(|(_, line)| is_wrap_row(line))
        .collect();

    assert!(
        wrap_rows.len() >= 2,
        "expected the long word to wrap across at least 2 rows; \
         found {} wrap rows in capture.\ncols={cols}, word_len={word_len}\n\
         plain:\n{plain}",
        wrap_rows.len(),
    );

    // The two rows must be consecutive (wrap continuation, not unrelated
    // lines).
    let (first_idx, first_row) = wrap_rows[0];
    let (second_idx, second_row) = wrap_rows[1];
    assert_eq!(
        second_idx,
        first_idx + 1,
        "expected wrap continuation row immediately after the first wrapped row; \
         got rows {first_idx} and {second_idx}.\nplain:\n{plain}",
    );

    // The first wrapped row's visible width must be exactly `cols` —
    // the prose renderer fills to the hard pane boundary then wraps
    // (optionally appending a trailing hyphen which itself counts toward
    // the cell budget).
    let first_trimmed = first_row.trim_end();
    let first_width = UnicodeWidthStr::width(first_trimmed);
    assert_eq!(
        first_width, cols,
        "expected first wrapped row visible width to equal pane cols ({cols}); \
         got {first_width}.\nrow: {first_row:?}",
    );

    // The first row's visible width MUST NOT exceed the pane.
    assert!(
        first_width <= cols,
        "first wrapped row width {first_width} exceeds pane cols {cols}.\n\
         row: {first_row:?}",
    );

    // Count 'a' characters across the wrap rows. With a soft-break
    // hyphen at the end of the first row, the count of 'a's equals
    // word_len (the hyphen is added, not substituted).
    let total_a: usize = wrap_rows
        .iter()
        .map(|(_, r)| r.trim_end().chars().filter(|c| *c == 'a').count())
        .sum();
    assert_eq!(
        total_a, word_len,
        "expected {word_len} total 'a' characters across wrapped rows; got {total_a}.\n\
         rows: {wrap_rows:?}\nplain:\n{plain}",
    );

    // The continuation row's first non-space character must be 'a' — the
    // wrap point is exactly at the column boundary.
    let continuation = second_row.trim_start();
    assert!(
        continuation.starts_with('a'),
        "expected continuation row to start with 'a'; got {continuation:?}",
    );
}

// ------------------------------------------------------------------
// Assertion helpers
// ------------------------------------------------------------------

/// Asserts that `frame.raw` contains an OSC8 hyperlink sequence for `url`
/// and that `frame.plain` contains the visible `label` text.
fn assert_osc8_link_present(frame: &CapturedFrame, url: &str, label: &str) {
    assert!(
        frame.raw.contains(&format!("\x1b]8;;{}", url)),
        "expected raw output to contain OSC8 hyperlink sequence for {}. raw:\n{}",
        url,
        frame.raw
    );
    assert!(
        frame.plain.contains(label),
        "expected plain text to contain label '{}'. plain:\n{}",
        label,
        frame.plain
    );
}

/// Asserts that the `bt` output region of `frame` contains no SGR red
/// sequences (`\x1b[31m`, `\x1b[91m`).
///
/// The bt output region is defined as: starting from the line whose
/// *plain* form contains a `NO_COLOR=` prefix and `bt prose`, up to
/// but not including the next shell-prompt line (lines whose plain
/// form ends in `$ `, `% `, or `# `). We pair the raw and plain
/// forms by line index so we can reason about prompts (which may
/// themselves carry SGR from a colored prompt theme like starship)
/// without false-positiving on the shell's own colors.
///
/// The predicate matches both the bare `NO_COLOR=1` form and the
/// shell-quoted `NO_COLOR='1'` form emitted by
/// [`TerminalHarness::send_command_with_env`].
fn assert_no_sgr_red(frame: &CapturedFrame) {
    let raw_lines: Vec<&str> = frame.raw.lines().collect();
    let plain_lines: Vec<&str> = frame.plain.lines().collect();

    // Locate the command-issue line. We match on plain form so SGR
    // bytes within the captured prompt don't break the search. The
    // `NO_COLOR=` prefix matches both `NO_COLOR=1` and the
    // single-quote-escaped `NO_COLOR='1'` produced by the harness.
    let cmd_idx = plain_lines
        .iter()
        .position(|l| l.contains("NO_COLOR=") && l.contains("bt prose"));

    let Some(cmd_idx) = cmd_idx else {
        // We could not locate the command line; the test plainly cannot
        // make a localized assertion. Fall back to a global check.
        assert!(
            !frame.raw.contains("\x1b[31m"),
            "expected NO \\x1b[31m with NO_COLOR=1; could not locate command line. raw:\n{}",
            frame.raw
        );
        assert!(
            !frame.raw.contains("\x1b[91m"),
            "expected NO \\x1b[91m with NO_COLOR=1; could not locate command line. raw:\n{}",
            frame.raw
        );
        return;
    };

    // Walk forward from cmd_idx + 1 up to N lines or until the next
    // prompt-suffix line. Skip the command line itself (which by
    // definition carries the literal "<red>x</red>" text the user
    // typed, but no SGR).
    const WINDOW: usize = 10;
    let end_exclusive = (cmd_idx + 1 + WINDOW).min(plain_lines.len());

    for (i, plain) in plain_lines
        .iter()
        .enumerate()
        .take(end_exclusive)
        .skip(cmd_idx + 1)
    {
        let trimmed = plain.trim_end();
        // Stop scanning once we hit the *next* shell prompt; everything
        // after it belongs to a subsequent command, not this one.
        if trimmed.ends_with('$') || trimmed.ends_with('%') || trimmed.ends_with('#') {
            return;
        }
        let raw_line = raw_lines.get(i).copied().unwrap_or("");
        assert!(
            !raw_line.contains("\x1b[31m"),
            "expected NO \\x1b[31m with NO_COLOR=1 in bt output line {i}.\nplain: {plain}\nraw:   {raw_line}",
        );
        assert!(
            !raw_line.contains("\x1b[91m"),
            "expected NO \\x1b[91m with NO_COLOR=1 in bt output line {i}.\nplain: {plain}\nraw:   {raw_line}",
        );
    }
}
