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

use biscuit_test_harness::kitty::KittyHarness;
use biscuit_test_harness::shared::SharedHarness;
use biscuit_test_harness::wezterm::WezTermHarness;
use biscuit_test_harness::{CapturedFrame, TerminalHarness};
use common::send_bt_command;
use serial_test::serial;
use test_toolkit::{Level, require_level};
use unicode_width::UnicodeWidthStr;

/// Process-shared WezTerm pane reused across the WezTerm prose tests.
/// A `clear` is sent before each test's first interaction so prior
/// renders cannot leak into the capture window.
static SHARED_WEZTERM: SharedHarness<WezTermHarness> = SharedHarness::new();

/// Process-shared Kitty window reused across the Kitty prose tests.
static SHARED_KITTY: SharedHarness<KittyHarness> = SharedHarness::new();

// ------------------------------------------------------------------
// WezTerm — SGR, OSC8, NO_COLOR
// ------------------------------------------------------------------

#[test]
#[serial(level2_terminal)]
fn level2_prose_emits_sgr_in_real_terminal() {
    require_level!(
        Level::L2,
        WezTermHarness::available(),
        "WezTerm CLI (set WEZTERM_UNIX_SOCKET)",
    );

    // `WezTermHarness::new()` spawns into a dedicated background
    // workspace, so the pane never grabs focus or steals the desktop.
    let mut guard = SHARED_WEZTERM
        .get_or_init(|| WezTermHarness::shared_or_spawn().expect("attach/spawn WezTerm"));
    let harness = guard.as_mut().expect("shared WezTerm harness present");
    harness.send_text(b"clear\n").expect("send_text failed");
    harness.settle();

    // Scope FORCE_COLOR=1 to the spawned `bt` and use the CLI
    // `--force-color` flag so styling is decoupled from both the
    // spawned shell's environment AND from `bt`'s TTY detection.
    harness
        .send_command_with_env(
            "bt prose --force-color \"<red>x</red>\"",
            &[("FORCE_COLOR", "1")],
        )
        .expect("send_command_with_env failed");
    // Wait for the shell prompt to return so the bt output has been
    // committed to cells before we capture.
    let _ = biscuit_test_harness::wait_for_prompt(harness);

    let frame_a = harness.capture().expect("capture A failed");
    // Capture again 200 ms later to defend against the rare WezTerm
    // get-text race where cell SGR has not been re-emitted into the
    // dump on the first capture.
    std::thread::sleep(std::time::Duration::from_millis(200));
    let frame_b = harness.capture().expect("capture B failed");

    // Strict Level 2: the proof is the real terminal's own `get-text
    // --escapes` capture path. We isolate the rendered `x` output line
    // (so a colored shell prompt cannot satisfy the assertion) and
    // require SGR red in the cells WezTerm actually displayed.
    assert!(
        output_line_has_sgr_red(&frame_a) || output_line_has_sgr_red(&frame_b),
        "expected SGR red in the captured `x` output line.\n\
         capture A raw:\n{}\n\
         capture B raw:\n{}",
        frame_a.raw,
        frame_b.raw,
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_prose_osc8_link_renders() {
    require_level!(
        Level::L2,
        WezTermHarness::available(),
        "WezTerm CLI (set WEZTERM_UNIX_SOCKET)",
    );

    let mut guard = SHARED_WEZTERM
        .get_or_init(|| WezTermHarness::shared_or_spawn().expect("attach/spawn WezTerm"));
    let harness = guard.as_mut().expect("shared WezTerm harness present");
    harness.send_text(b"clear\n").expect("send_text failed");
    harness.settle();

    send_bt_command(
        harness,
        "prose \"<a href=https://example.com>link</a>\"",
    );

    let frame = harness.capture().expect("capture failed");
    assert_osc8_link_present(&frame, "https://example.com", "link");
}

#[test]
#[serial(level2_terminal)]
fn level2_no_color_strips_sgr_in_real_terminal() {
    require_level!(
        Level::L2,
        WezTermHarness::available(),
        "WezTerm CLI (set WEZTERM_UNIX_SOCKET)",
    );

    let mut guard = SHARED_WEZTERM
        .get_or_init(|| WezTermHarness::shared_or_spawn().expect("attach/spawn WezTerm"));
    let harness = guard.as_mut().expect("shared WezTerm harness present");
    harness.send_text(b"clear\n").expect("send_text failed");
    harness.settle();

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
    require_level!(
        Level::L2,
        KittyHarness::available(),
        "Kitty remote control (set KITTY_LISTEN_ON)",
    );

    // `KittyHarness::new()` passes `--keep-focus` so the spawned OS
    // window never steals focus from the developer's session.
    let mut guard = SHARED_KITTY
        .get_or_init(|| KittyHarness::shared_or_spawn().expect("attach/spawn kitty"));
    let harness = guard.as_mut().expect("shared Kitty harness present");
    harness.send_text(b"clear\n").expect("send_text failed");
    harness.settle();

    // Scope FORCE_COLOR=1 and use the CLI `--force-color` flag for
    // symmetry with the WezTerm test.
    harness
        .send_command_with_env(
            "bt prose --force-color \"<red>x</red>\"",
            &[("FORCE_COLOR", "1")],
        )
        .expect("send_command_with_env failed");
    let _ = biscuit_test_harness::wait_for_prompt(harness);
    let frame_a = harness.capture().expect("capture A failed");
    std::thread::sleep(std::time::Duration::from_millis(200));
    let frame_b = harness.capture().expect("capture B failed");

    // Strict Level 2: assert SGR red in the cells Kitty actually
    // displayed, isolated to the rendered `x` output line.
    assert!(
        output_line_has_sgr_red(&frame_a) || output_line_has_sgr_red(&frame_b),
        "expected SGR red in the captured `x` output line.\n\
         capture A raw:\n{}\n\
         capture B raw:\n{}",
        frame_a.raw,
        frame_b.raw,
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_prose_osc8_link_renders_in_kitty() {
    require_level!(
        Level::L2,
        KittyHarness::available(),
        "Kitty remote control (set KITTY_LISTEN_ON)",
    );

    let mut guard = SHARED_KITTY
        .get_or_init(|| KittyHarness::shared_or_spawn().expect("attach/spawn kitty"));
    let harness = guard.as_mut().expect("shared Kitty harness present");
    harness.send_text(b"clear\n").expect("send_text failed");
    harness.settle();

    send_bt_command(
        harness,
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
    require_level!(
        Level::L2,
        WezTermHarness::available(),
        "WezTerm CLI (set WEZTERM_UNIX_SOCKET)",
    );

    let mut guard = SHARED_WEZTERM
        .get_or_init(|| WezTermHarness::shared_or_spawn().expect("attach/spawn WezTerm"));
    let harness = guard.as_mut().expect("shared WezTerm harness present");
    harness.send_text(b"clear\n").expect("send_text failed");
    harness.settle();

    send_bt_command(harness, "padleft 30 \"x\"");

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
    require_level!(
        Level::L2,
        WezTermHarness::available(),
        "WezTerm CLI (set WEZTERM_UNIX_SOCKET)",
    );

    let mut guard = SHARED_WEZTERM
        .get_or_init(|| WezTermHarness::shared_or_spawn().expect("attach/spawn WezTerm"));
    let harness = guard.as_mut().expect("shared WezTerm harness present");
    harness.send_text(b"clear\n").expect("send_text failed");
    harness.settle();

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
    send_bt_command(harness, &format!("prose \"{long_word}\""));

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
// Rich styling — nested emphasis, fg/bg RGB color, strikethrough,
// underline, code-block indentation through the IR-backed renderer
// ------------------------------------------------------------------

/// Rich `bt prose` input exercising nested bold/italic, strikethrough,
/// underline, foreground and background RGB color, and a fenced code
/// block — the styling the IR-backed terminal renderer must preserve.
///
/// The visible output collapses (whitespace removed) to `bisufgq`, which
/// [`find_bt_output_line`] uses to isolate the rendered row from the
/// shell prompt and the command echo.
const RICH_PROSE_INPUT: &str = "<b><i>bi</i></b> <~>s</~> <u>u</u> \
     <rgb 4,5,6>f</rgb> <bg-rgb 1,2,3>g</bg-rgb> \
     <code-block lang=rust>q</code-block>";

/// SGR effects [`RICH_PROSE_INPUT`] must produce, decoded from the real
/// terminal's `get-text` capture (not the renderer's own byte stream).
const RICH_EXPECTED_SGR: &[Sgr] = &[
    Sgr::Bold,
    Sgr::Italic,
    Sgr::Strikethrough,
    Sgr::Underline,
    Sgr::FgRgb(4, 5, 6),
    Sgr::BgRgb(1, 2, 3),
    Sgr::Dim,
];

/// Runs [`RICH_PROSE_INPUT`] through `bt prose` in the given real
/// terminal and asserts every [`RICH_EXPECTED_SGR`] effect is present in
/// the styled cells the emulator actually displayed.
///
/// This is a strict Level 2 check: the proof is the terminal's own
/// `get-text` capture path (`wezterm cli get-text --escapes` /
/// `kitty @ get-text --ansi`), decoded by [`decode_sgr_effects`]. The
/// assertion fails when the capture does not contain the rendered style
/// evidence — the renderer's own byte stream is never consulted.
fn assert_rich_prose_sgr<H: TerminalHarness>(harness: &mut H) {
    harness
        .send_command_with_env(
            &format!("bt prose --force-color \"{RICH_PROSE_INPUT}\""),
            &[("FORCE_COLOR", "1")],
        )
        .expect("send_command_with_env failed");
    let _ = biscuit_test_harness::wait_for_prompt(harness);
    std::thread::sleep(std::time::Duration::from_millis(200));
    let frame = harness.capture().expect("capture failed");

    // Isolate the rendered output row so SGR carried by a colored shell
    // prompt (bold/italic/dim in a starship theme, etc.) cannot satisfy
    // the assertion.
    let output_line = find_bt_output_line(&frame, "bisufgq").unwrap_or_else(|| {
        panic!(
            "could not locate the rich-prose output row (compact `bisufgq`).\nraw:\n{}",
            frame.raw
        )
    });
    let effects = decode_sgr_effects(output_line);

    for expected in RICH_EXPECTED_SGR {
        assert!(
            effects.contains(expected),
            "expected {expected:?} in the captured output row's SGR.\n\
             decoded effects: {effects:?}\noutput row:\n{output_line:?}\nraw:\n{}",
            frame.raw,
        );
    }
}

#[test]
#[serial(level2_terminal)]
fn level2_prose_rich_styling_emits_sgr_in_wezterm() {
    require_level!(
        Level::L2,
        WezTermHarness::available(),
        "WezTerm CLI (set WEZTERM_UNIX_SOCKET)",
    );

    let mut guard = SHARED_WEZTERM
        .get_or_init(|| WezTermHarness::shared_or_spawn().expect("attach/spawn WezTerm"));
    let harness = guard.as_mut().expect("shared WezTerm harness present");
    harness.send_text(b"clear\n").expect("send_text failed");
    harness.settle();
    assert_rich_prose_sgr(harness);
}

#[test]
#[serial(level2_terminal)]
fn level2_prose_rich_styling_emits_sgr_in_kitty() {
    require_level!(
        Level::L2,
        KittyHarness::available(),
        "Kitty remote control (set KITTY_LISTEN_ON)",
    );

    let mut guard = SHARED_KITTY
        .get_or_init(|| KittyHarness::shared_or_spawn().expect("attach/spawn kitty"));
    let harness = guard.as_mut().expect("shared Kitty harness present");
    harness.send_text(b"clear\n").expect("send_text failed");
    harness.settle();
    assert_rich_prose_sgr(harness);
}

// ------------------------------------------------------------------
// Nested code block — enclosing style restoration (review-5)
// ------------------------------------------------------------------

/// `bt prose` input where a fenced code block sits inside an active
/// `<red>` span with sibling text on both sides.
///
/// The compact visible form is `beforecodeafter`, which
/// [`find_bt_output_line`] uses to isolate the rendered row.
const NESTED_CODE_BLOCK_INPUT: &str =
    "<red>before <code-block lang=rust>code</code-block> after</red>";

/// Runs [`NESTED_CODE_BLOCK_INPUT`] through `bt prose` in the given real
/// terminal and asserts the enclosing red style is restored after the
/// code block — i.e. the trailing `after` text is still red.
///
/// Regression for review-5: the code block's hard `\x1b[0m` reset
/// previously cleared the enclosing span, leaving `after` unstyled. The
/// proof is the terminal's own `get-text` capture: the SGR run between
/// the dim `code` cells and the `after` cells must re-select red.
fn assert_code_block_restores_parent_style<H: TerminalHarness>(harness: &mut H) {
    harness
        .send_command_with_env(
            &format!("bt prose --force-color \"{NESTED_CODE_BLOCK_INPUT}\""),
            &[("FORCE_COLOR", "1")],
        )
        .expect("send_command_with_env failed");
    let _ = biscuit_test_harness::wait_for_prompt(harness);
    std::thread::sleep(std::time::Duration::from_millis(200));
    let frame = harness.capture().expect("capture failed");

    let row = find_bt_output_line(&frame, "beforecodeafter").unwrap_or_else(|| {
        panic!(
            "could not locate the nested code-block output row (compact \
             `beforecodeafter`).\nraw:\n{}",
            frame.raw
        )
    });
    let code_pos = row.find("code").expect("`code` text present in row");
    let after_pos = row.find("after").expect("`after` text present in row");
    assert!(
        code_pos < after_pos,
        "expected `code` before `after` in the output row: {row:?}",
    );
    let between = &row[code_pos + "code".len()..after_pos];
    assert!(
        segment_selects_red(between),
        "expected the enclosing red style to be restored before the \
         post-code-block `after` text.\nbetween: {between:?}\nrow: {row:?}",
    );
}

/// Whether `segment` contains an SGR sequence that selects basic red
/// (`31`) or bright red (`91`) foreground.
fn segment_selects_red(segment: &str) -> bool {
    let bytes = segment.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == 0x1b && bytes.get(i + 1) == Some(&b'[') {
            let start = i + 2;
            let mut j = start;
            while j < bytes.len()
                && (bytes[j].is_ascii_digit() || bytes[j] == b';' || bytes[j] == b':')
            {
                j += 1;
            }
            if bytes.get(j) == Some(&b'm') {
                let selects_red = segment[start..j]
                    .split([';', ':'])
                    .any(|p| p == "31" || p == "91");
                if selects_red {
                    return true;
                }
                i = j + 1;
                continue;
            }
        }
        i += 1;
    }
    false
}

#[test]
#[serial(level2_terminal)]
fn level2_prose_code_block_restores_parent_style_in_wezterm() {
    require_level!(
        Level::L2,
        WezTermHarness::available(),
        "WezTerm CLI (set WEZTERM_UNIX_SOCKET)",
    );

    let mut guard = SHARED_WEZTERM
        .get_or_init(|| WezTermHarness::shared_or_spawn().expect("attach/spawn WezTerm"));
    let harness = guard.as_mut().expect("shared WezTerm harness present");
    harness.send_text(b"clear\n").expect("send_text failed");
    harness.settle();
    assert_code_block_restores_parent_style(harness);
}

#[test]
#[serial(level2_terminal)]
fn level2_prose_code_block_restores_parent_style_in_kitty() {
    require_level!(
        Level::L2,
        KittyHarness::available(),
        "Kitty remote control (set KITTY_LISTEN_ON)",
    );

    let mut guard = SHARED_KITTY
        .get_or_init(|| KittyHarness::shared_or_spawn().expect("attach/spawn kitty"));
    let harness = guard.as_mut().expect("shared Kitty harness present");
    harness.send_text(b"clear\n").expect("send_text failed");
    harness.settle();
    assert_code_block_restores_parent_style(harness);
}

#[test]
#[serial(level2_terminal)]
fn level2_prose_nested_emphasis_visible_text_in_wezterm() {
    require_level!(
        Level::L2,
        WezTermHarness::available(),
        "WezTerm CLI (set WEZTERM_UNIX_SOCKET)",
    );

    let mut guard = SHARED_WEZTERM
        .get_or_init(|| WezTermHarness::shared_or_spawn().expect("attach/spawn WezTerm"));
    let harness = guard.as_mut().expect("shared WezTerm harness present");
    harness.send_text(b"clear\n").expect("send_text failed");
    harness.settle();

    send_bt_command(harness, "prose \"<b><i>nested</i></b> tail\"");
    let _ = biscuit_test_harness::wait_for_prompt(harness);

    let frame = harness.capture().expect("capture failed");
    // The styled and trailing text must remain visible with no escape
    // garbage leaking into the captured plain cells.
    assert!(
        frame.plain.contains("nested") && frame.plain.contains("tail"),
        "expected visible 'nested' and 'tail' text. plain:\n{}",
        frame.plain
    );
    assert!(
        !frame.plain.contains("\x1b") && !frame.plain.contains("[1m"),
        "expected no raw escape text in the visible capture. plain:\n{}",
        frame.plain
    );
}

// ------------------------------------------------------------------
// SGR decoding — strict Level 2 capture verification
// ------------------------------------------------------------------

/// A single SGR effect decoded from a CSI `m` sequence in a terminal
/// `get-text` capture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Sgr {
    Bold,
    Dim,
    Italic,
    Underline,
    Strikethrough,
    FgRgb(i64, i64, i64),
    BgRgb(i64, i64, i64),
}

/// Decode every [`Sgr`] effect present in `raw`, across all CSI `m`
/// sequences.
///
/// Terminal `get-text` capture paths re-emit cell attributes in their
/// own normalized form rather than echoing the renderer's bytes: WezTerm
/// coalesces attributes (`\x1b[0;1;4m`) and emits truecolor in the
/// colon form (`\x1b[38:2::r:g:bm`); Kitty uses the semicolon form.
/// Both separators are accepted and empty sub-parameters (the colon-form
/// colorspace slot) are dropped before decoding.
fn decode_sgr_effects(raw: &str) -> Vec<Sgr> {
    let mut effects = Vec::new();
    let bytes = raw.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == 0x1b && bytes.get(i + 1) == Some(&b'[') {
            let start = i + 2;
            let mut j = start;
            while j < bytes.len()
                && (bytes[j].is_ascii_digit() || bytes[j] == b';' || bytes[j] == b':')
            {
                j += 1;
            }
            if bytes.get(j) == Some(&b'm') {
                let params: Vec<i64> = raw[start..j]
                    .split([';', ':'])
                    .filter(|s| !s.is_empty())
                    .filter_map(|s| s.parse().ok())
                    .collect();
                decode_sgr_params(&params, &mut effects);
                i = j + 1;
                continue;
            }
        }
        i += 1;
    }
    effects
}

/// Decode one SGR sequence's parameter list into [`Sgr`] effects.
///
/// `38`/`48` (set foreground/background) consume the following
/// parameters: `2;r;g;b` for truecolor and `5;n` for indexed color.
fn decode_sgr_params(params: &[i64], out: &mut Vec<Sgr>) {
    let mut i = 0;
    while i < params.len() {
        match params[i] {
            1 => out.push(Sgr::Bold),
            2 => out.push(Sgr::Dim),
            3 => out.push(Sgr::Italic),
            4 => out.push(Sgr::Underline),
            9 => out.push(Sgr::Strikethrough),
            38 | 48 => {
                let is_fg = params[i] == 38;
                match params.get(i + 1) {
                    Some(&2) => {
                        if let (Some(&r), Some(&g), Some(&b)) =
                            (params.get(i + 2), params.get(i + 3), params.get(i + 4))
                        {
                            out.push(if is_fg {
                                Sgr::FgRgb(r, g, b)
                            } else {
                                Sgr::BgRgb(r, g, b)
                            });
                        }
                        i += 5;
                        continue;
                    }
                    Some(&5) => {
                        i += 3;
                        continue;
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        i += 1;
    }
}

/// Return the raw (escape-bearing) capture line for the `bt prose`
/// output row whose visible text, with all whitespace removed, equals
/// `compact_plain`.
///
/// The search starts after the command-echo line so the echoed input
/// (which contains the literal markup, not rendered cells) is skipped.
/// Isolating the output row keeps a colored shell prompt from
/// satisfying SGR assertions.
fn find_bt_output_line<'a>(frame: &'a CapturedFrame, compact_plain: &str) -> Option<&'a str> {
    let raw_lines: Vec<&str> = frame.raw.lines().collect();
    let plain_lines: Vec<&str> = frame.plain.lines().collect();
    let cmd_idx = plain_lines.iter().position(|l| l.contains("bt prose"))?;
    for (i, plain) in plain_lines.iter().enumerate().skip(cmd_idx + 1) {
        let compact: String = plain.chars().filter(|c| !c.is_whitespace()).collect();
        if compact == compact_plain {
            return raw_lines.get(i).copied();
        }
    }
    None
}

/// Whether the isolated `<red>x</red>` output row carries SGR red.
///
/// The renderer emits `\x1b[31m`; both WezTerm and Kitty re-emit basic
/// colors faithfully, so a standalone `31`/`91` SGR parameter on the
/// row is the strict capture proof.
fn output_line_has_sgr_red(frame: &CapturedFrame) -> bool {
    find_bt_output_line(frame, "x")
        .map(|line| line.contains("\x1b[31m") || line.contains("\x1b[91m"))
        .unwrap_or(false)
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
