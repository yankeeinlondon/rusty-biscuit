//! Level-2 tests for prose styling, inline code, OSC8 hyperlinks, NO_COLOR,
//! line-break geometry, and layout.
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

use crate::common;

use biscuit_test_harness::kitty::KittyHarness;
use biscuit_test_harness::shared::SharedHarness;
use biscuit_test_harness::tmux::TmuxHarness;
use biscuit_test_harness::wezterm::WezTermHarness;
use biscuit_test_harness::{CapturedFrame, TerminalHarness};
use common::{OutputRow, active_runs, osc8_destinations, run_bt_output, run_bt_rows, send_bt_command};
use crate::prose_cells::{attr_run_cells, fg_red_run_cells, osc8_run_cells};
use biscuit_terminal::components::prose::{LineBreaks, Prose};
use biscuit_terminal::components::renderable::TerminalRenderable;
use serial_test::serial;
use test_toolkit::{Backend, Level, require_level};
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
    require_level!(Level::L2, WezTermHarness::available(), Backend::WezTerm);

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
    // The output frame comes from a settled capture (two identical dumps),
    // which also covers the WezTerm get-text race where cell SGR is not yet
    // re-emitted on the first dump.
    let frame = run_bt_output(harness, "prose --force-color \"<red>x</red>\"", FORCED);

    // Strict Level 2: the proof is the real terminal's own `get-text
    // --escapes` capture path. We isolate the rendered `x` output line
    // (so a colored shell prompt cannot satisfy the assertion) and
    // require SGR red in the cells WezTerm actually displayed.
    assert!(
        output_line_has_sgr_red(&frame),
        "expected SGR red in the captured `x` output line.\nraw:\n{}",
        frame.raw,
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_prose_osc8_link_renders() {
    require_level!(Level::L2, WezTermHarness::available(), Backend::WezTerm);

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
    require_level!(Level::L2, WezTermHarness::available(), Backend::WezTerm);

    let mut guard = SHARED_WEZTERM
        .get_or_init(|| WezTermHarness::shared_or_spawn().expect("attach/spawn WezTerm"));
    let harness = guard.as_mut().expect("shared WezTerm harness present");
    harness.send_text(b"clear\n").expect("send_text failed");
    harness.settle();

    // Scope NO_COLOR=1 to a single command via the harness helper
    // (portable inline-env syntax; works regardless of the developer's
    // login shell).
    let frame = run_bt_output(harness, "prose \"<red>x</red>\"", &[("NO_COLOR", "1")]);
    assert_no_sgr_red(&frame);
}

// ------------------------------------------------------------------
// Kitty — SGR, OSC8
// ------------------------------------------------------------------

#[test]
#[serial(level2_terminal)]
fn level2_prose_emits_sgr_in_kitty() {
    require_level!(Level::L2, KittyHarness::available(), Backend::Kitty);

    // `KittyHarness::new()` passes `--keep-focus` so the spawned OS
    // window never steals focus from the developer's session.
    let mut guard = SHARED_KITTY
        .get_or_init(|| KittyHarness::shared_or_spawn().expect("attach/spawn kitty"));
    let harness = guard.as_mut().expect("shared Kitty harness present");
    harness.send_text(b"clear\n").expect("send_text failed");
    harness.settle();

    // Scope FORCE_COLOR=1 and use the CLI `--force-color` flag for
    // symmetry with the WezTerm test.
    let frame = run_bt_output(harness, "prose --force-color \"<red>x</red>\"", FORCED);

    // Strict Level 2: assert SGR red in the cells Kitty actually
    // displayed, isolated to the rendered `x` output line.
    assert!(
        output_line_has_sgr_red(&frame),
        "expected SGR red in the captured `x` output line.\nraw:\n{}",
        frame.raw,
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_prose_osc8_link_renders_in_kitty() {
    require_level!(Level::L2, KittyHarness::available(), Backend::Kitty);

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
    require_level!(Level::L2, WezTermHarness::available(), Backend::WezTerm);

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
    require_level!(Level::L2, WezTermHarness::available(), Backend::WezTerm);

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
    let frame = run_bt_output(harness, &format!("prose \"{long_word}\""), &[]);
    let plain = &frame.plain;

    // Locate the *output* lines — rows whose trimmed content is
    // composed entirely of 'a' characters (and optionally a single
    // trailing hyphen, which textwrap may insert as a soft-break marker
    // when forced to break inside a word).
    let is_wrap_row = |line: &str| -> bool {
        let trimmed = line.trim_end();
        if trimmed.is_empty() {
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
/// The fenced code block (`q`) renders on its own indented rows, so the
/// styled inline run and the dim code occupy separate lines;
/// [`rendered_region_effects`] decodes SGR across the whole rendered region
/// (excluding the shell prompt) rather than a single row.
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
    let frame = run_bt_output(harness, &format!("prose --force-color \"{RICH_PROSE_INPUT}\""), FORCED);

    // Decode SGR across the whole rendered region (the styled inline run and
    // the dim fenced code block land on separate rows). The trailing prompt —
    // which may carry its own theme SGR — is excluded.
    let effects = rendered_region_effects(&frame);

    for expected in RICH_EXPECTED_SGR {
        assert!(
            effects.contains(expected),
            "expected {expected:?} in the rendered region's SGR.\n\
             decoded effects: {effects:?}\nraw:\n{}",
            frame.raw,
        );
    }
}

#[test]
#[serial(level2_terminal)]
fn level2_prose_rich_styling_emits_sgr_in_wezterm() {
    require_level!(Level::L2, WezTermHarness::available(), Backend::WezTerm);

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
    require_level!(Level::L2, KittyHarness::available(), Backend::Kitty);

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
/// The code block is block-level, so it renders as its own dim, indented
/// block on separate rows; `before` and `after` therefore land on their
/// own lines, each isolatable by [`find_bt_output_line`].
const NESTED_CODE_BLOCK_INPUT: &str =
    "<red>before <code-block lang=rust>code</code-block> after</red>";

/// Runs [`NESTED_CODE_BLOCK_INPUT`] through `bt prose` in the given real
/// terminal and asserts the enclosing red style wraps the inline text on
/// **both** sides of the code block — i.e. `before` and `after` are each
/// still red.
///
/// Regression for the prose-tree cutover: a fenced code block nested in a
/// styled span is split around the block child, so the block's own reset
/// cannot clear the enclosing span. The proof is the terminal's own
/// `get-text` capture: the `before` and `after` rows must each select red.
fn assert_code_block_restores_parent_style<H: TerminalHarness>(harness: &mut H) {
    let frame = run_bt_output(harness, &format!("prose --force-color \"{NESTED_CODE_BLOCK_INPUT}\""), FORCED);

    let before_row = find_bt_output_line(&frame, "before").unwrap_or_else(|| {
        panic!(
            "could not locate the `before` output row.\nraw:\n{}",
            frame.raw
        )
    });
    assert!(
        segment_selects_red(before_row),
        "expected the `before` text to be red.\nrow: {before_row:?}",
    );

    let after_row = find_bt_output_line(&frame, "after").unwrap_or_else(|| {
        panic!(
            "could not locate the `after` output row.\nraw:\n{}",
            frame.raw
        )
    });
    assert!(
        segment_selects_red(after_row),
        "expected the enclosing red style to be restored for the \
         post-code-block `after` text.\nrow: {after_row:?}",
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
    require_level!(Level::L2, WezTermHarness::available(), Backend::WezTerm);

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
    require_level!(Level::L2, KittyHarness::available(), Backend::Kitty);

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
    require_level!(Level::L2, WezTermHarness::available(), Backend::WezTerm);

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
// Inverse (SGR 7) — shared TextEmphasis::inverse through the tree
// ------------------------------------------------------------------

/// Runs `<inverse>` through `bt prose` and asserts the reverse-video SGR
/// (`7`) is present in the cells the real terminal displayed.
///
/// `<inverse>` / `<reverse>` lower to `TextEmphasis::inverse`, which the
/// shared terminal tree renderer emits as SGR 7. This is the strict
/// Level 2 proof for the new inverse capability: the evidence is the
/// terminal's own `get-text` capture decoded by [`decode_sgr_effects`],
/// which accepts both the semicolon and colon SGR sub-parameter forms.
fn assert_prose_inverse_sgr<H: TerminalHarness>(harness: &mut H) {
    let frame = run_bt_output(harness, "prose --force-color \"<inverse>x</inverse>\"", FORCED);

    let row = find_bt_output_line(&frame, "x").unwrap_or_else(|| {
        panic!(
            "could not locate the inverse output row (compact `x`).\nraw:\n{}",
            frame.raw
        )
    });
    let effects = decode_sgr_effects(row);
    assert!(
        effects.contains(&Sgr::Inverse),
        "expected SGR 7 (inverse) in the captured output row.\n\
         decoded effects: {effects:?}\noutput row:\n{row:?}\nraw:\n{}",
        frame.raw,
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_prose_inverse_emits_sgr_in_wezterm() {
    require_level!(Level::L2, WezTermHarness::available(), Backend::WezTerm);

    let mut guard = SHARED_WEZTERM
        .get_or_init(|| WezTermHarness::shared_or_spawn().expect("attach/spawn WezTerm"));
    let harness = guard.as_mut().expect("shared WezTerm harness present");
    harness.send_text(b"clear\n").expect("send_text failed");
    harness.settle();
    assert_prose_inverse_sgr(harness);
}

#[test]
#[serial(level2_terminal)]
fn level2_prose_inverse_emits_sgr_in_kitty() {
    require_level!(Level::L2, KittyHarness::available(), Backend::Kitty);

    let mut guard = SHARED_KITTY
        .get_or_init(|| KittyHarness::shared_or_spawn().expect("attach/spawn kitty"));
    let harness = guard.as_mut().expect("shared Kitty harness present");
    harness.send_text(b"clear\n").expect("send_text failed");
    harness.settle();
    assert_prose_inverse_sgr(harness);
}

// ------------------------------------------------------------------
// `<hidden>` — REMOVED: renders as inert literal text
// ------------------------------------------------------------------

#[test]
#[serial(level2_terminal)]
fn level2_prose_hidden_renders_as_literal_text_in_wezterm() {
    require_level!(Level::L2, WezTermHarness::available(), Backend::WezTerm);

    let mut guard = SHARED_WEZTERM
        .get_or_init(|| WezTermHarness::shared_or_spawn().expect("attach/spawn WezTerm"));
    let harness = guard.as_mut().expect("shared WezTerm harness present");
    harness.send_text(b"clear\n").expect("send_text failed");
    harness.settle();

    let frame = run_bt_output(harness, "prose \"<hidden>x</hidden>\"", &[]);

    // `<hidden>` is no longer recognized: the whole tag passes through as
    // visible literal text. The isolated output row's compact plain form is
    // exactly the literal markup.
    let row = find_bt_output_line(&frame, "<hidden>x</hidden>").unwrap_or_else(|| {
        panic!(
            "expected `<hidden>x</hidden>` to render as literal text on its \
             own row.\nplain:\n{}\nraw:\n{}",
            frame.plain, frame.raw
        )
    });
    // The renderer must not emit the SGR 8 conceal sequence the old bespoke
    // path used for `<hidden>`.
    assert!(
        !row.contains("\x1b[8m"),
        "expected no SGR 8 (conceal) for the removed `<hidden>` tag.\nrow:\n{row:?}",
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
    Inverse,
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
            7 => out.push(Sgr::Inverse),
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

/// The raw row of `frame`, a command's output frame, whose whitespace-free
/// visible text is exactly `compact_plain`.
fn find_bt_output_line<'a>(frame: &'a CapturedFrame, compact_plain: &str) -> Option<&'a str> {
    frame.plain.lines().zip(frame.raw.lines()).find_map(|(plain, raw)| {
        let compact: String = plain.chars().filter(|c| !c.is_whitespace()).collect();
        (compact == compact_plain).then_some(raw)
    })
}

/// Decode every [`Sgr`] effect in `frame`, a command's output frame.
///
/// Block-level content (a fenced code block) renders on its own indented
/// rows, so a styled run and its sibling code block occupy different lines;
/// a single-row decode cannot see them both. The output frame holds neither
/// the echo nor the prompt, so a themed prompt contributes no effects.
fn rendered_region_effects(frame: &CapturedFrame) -> Vec<Sgr> {
    frame.raw.lines().flat_map(decode_sgr_effects).collect()
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

/// Asserts that `frame`, a command's output frame, contains no SGR red
/// sequences (`\x1b[31m`, `\x1b[91m`). The output frame excludes the prompt,
/// so prompt styling cannot affect the assertion.
fn assert_no_sgr_red(frame: &CapturedFrame) {
    assert!(!frame.plain.trim().is_empty(), "expected `bt` output rows to check");
    for (i, (plain, raw_line)) in frame.plain.lines().zip(frame.raw.lines()).enumerate() {
        assert!(
            !raw_line.contains("\x1b[31m") && !raw_line.contains("\x1b[91m"),
            "expected NO red SGR with NO_COLOR=1 in bt output row {i}.\nplain: {plain}\nraw:   {raw_line}",
        );
    }
}

// ------------------------------------------------------------------
// Inline code, link extent, unstyled fallback, and break geometry
// ------------------------------------------------------------------

/// Process-shared tmux session for the inline-code and geometry tests. tmux is
/// the backend CI hosts; its `capture-pane -e` re-serializes each cell's SGR
/// and OSC 8 state, so the style and link assertions hold there too.
static SHARED_TMUX: SharedHarness<TmuxHarness> = SharedHarness::new();

/// Color forced on for the command, so styling does not depend on detection.
const FORCED: &[(&str, &str)] = &[("FORCE_COLOR", "1")];

/// The output's only row, after asserting the command displayed exactly one.
fn only_row<'a>(command: &str, frame: &CapturedFrame, rows: &'a [OutputRow]) -> &'a OutputRow {
    assert_eq!(
        rows.len(),
        1,
        "`bt {command}` must display one row, got {:?}.\nplain:\n{}",
        rows.iter().map(|row| &row.plain).collect::<Vec<_>>(),
        frame.plain,
    );
    &rows[0]
}

fn dim_runs(row: &OutputRow) -> Vec<String> {
    active_runs(&attr_run_cells(&row.raw, "2"), true)
}

fn red_runs(row: &OutputRow) -> Vec<String> {
    active_runs(&fg_red_run_cells(&row.raw), true)
}

/// Link runs are not trimmed: a clickable blank cell beside the label is a
/// visible defect, unlike an invisible attribute on a blank.
fn link_runs(row: &OutputRow) -> Vec<String> {
    active_runs(&osc8_run_cells(&row.raw), false)
}

/// Inline code in the cells the terminal displays: the backtick delimiters
/// are gone and the code is dim; inside a colored run the code keeps the color
/// and the color resumes after it while the dim does not; inside a link the
/// code is dim and the hyperlink covers exactly the label; a relative link
/// destination reaches the terminal as a `file://` URL; and with color off the
/// visible text keeps a backtick fence instead.
fn assert_inline_code_display<H: TerminalHarness>(harness: &mut H) {
    let command = "prose --force-color 'See `md hash` here'";
    let (frame, rows) = run_bt_rows(harness, command, FORCED);
    let row = only_row(command, &frame, &rows);
    assert_eq!(row.plain, "See md hash here", "delimiters must not display: {row:?}");
    assert_eq!(dim_runs(row), ["md hash"], "only the code is dim: {row:?}");

    let command = "prose --force-color '<red>run `md hash` now</red>'";
    let (frame, rows) = run_bt_rows(harness, command, FORCED);
    let row = only_row(command, &frame, &rows);
    assert_eq!(row.plain, "run md hash now", "{row:?}");
    assert_eq!(red_runs(row), ["run md hash now"], "red spans the code and resumes after it: {row:?}");
    assert_eq!(dim_runs(row), ["md hash"], "the dim ends with the code: {row:?}");

    let command = "prose --force-color 'see [`inline-block`](https://x.io/a) done'";
    let (frame, rows) = run_bt_rows(harness, command, FORCED);
    let row = only_row(command, &frame, &rows);
    assert_eq!(row.plain, "see inline-block done", "{row:?}");
    assert_eq!(dim_runs(row), ["inline-block"], "{row:?}");
    assert_eq!(link_runs(row), ["inline-block"], "the link covers exactly its code label: {row:?}");
    assert_eq!(osc8_destinations(&row.raw), ["https://x.io/a"], "{row:?}");

    let command = "prose --force-color 'read [plan](plan.md) first'";
    let (frame, rows) = run_bt_rows(harness, command, FORCED);
    let row = only_row(command, &frame, &rows);
    assert_eq!(row.plain, "read plan first", "{row:?}");
    assert_eq!(link_runs(row), ["plan"], "{row:?}");
    let destinations = osc8_destinations(&row.raw);
    assert!(
        destinations.len() == 1
            && destinations[0].starts_with("file://")
            && destinations[0].ends_with("/plan.md"),
        "a relative destination must reach the terminal as a file URL: {destinations:?}"
    );

    // Unstyled fallback, through `--plain` and through `NO_COLOR` (which wins
    // over the forced color).
    for (command, env) in [
        ("prose --plain 'See `md hash` here'", FORCED),
        ("prose 'See `md hash` here'", &[("FORCE_COLOR", "1"), ("NO_COLOR", "1")][..]),
        ("prose --plain '<red>run `md hash` now</red>'", FORCED),
        ("prose '<red>run `md hash` now</red>'", &[("FORCE_COLOR", "1"), ("NO_COLOR", "1")][..]),
    ] {
        let (frame, rows) = run_bt_rows(harness, command, env);
        let row = only_row(command, &frame, &rows);
        assert!(
            row.plain == "See `md hash` here" || row.plain == "run `md hash` now",
            "the colorless fallback must show a backtick fence ({command}, {env:?}): {row:?}"
        );
        assert!(
            dim_runs(row).is_empty() && red_runs(row).is_empty(),
            "the colorless fallback must not be styled ({command}, {env:?}): {row:?}"
        );
    }
}

/// Paragraph and line-break geometry as displayed: a single newline reflows as
/// one space, a blank line puts a blank row between paragraphs, a backslash
/// before the newline (or any newline in `Hard` mode) breaks the row,
/// whitespace around a soft break inside a wrapper collapses to exactly one
/// space, and a link around an explicit code block displays linked paragraphs
/// around the dim code.
///
/// The commands build their newlines with `printf` so every POSIX shell the
/// pane may run types them identically.
fn assert_break_geometry_display<H: TerminalHarness>(harness: &mut H) {
    let plain_rows = |rows: &[OutputRow]| rows.iter().map(|row| row.plain.clone()).collect::<Vec<_>>();

    let (_, rows) = run_bt_rows(harness, r#"prose --force-color "$(printf 'a\nb')""#, FORCED);
    assert_eq!(plain_rows(&rows), ["a b"], "a soft break reflows as one space");

    let (_, rows) = run_bt_rows(harness, r#"prose --force-color "$(printf 'a\n\nb')""#, FORCED);
    assert_eq!(plain_rows(&rows), ["a", "", "b"], "a blank line separates paragraphs");

    let (_, rows) = run_bt_rows(harness, r#"prose --force-color "$(printf 'a\\\nb')""#, FORCED);
    assert_eq!(plain_rows(&rows), ["a", "b"], "a backslash before the newline breaks the row");

    // `bt prose` has no line-break flag, so `Hard` mode is rendered in-process
    // and its bytes shown in the pane.
    let hard = Prose::new("a\nb")
        .with_line_breaks(LineBreaks::Hard)
        .render(&common::styled_terminal());
    let (_, rows) = common::display_bytes_rows(harness, hard.as_bytes());
    assert_eq!(plain_rows(&rows), ["a", "b"], "in `Hard` mode a newline breaks the row");

    let (frame, rows) = run_bt_rows(
        harness,
        r#"prose --force-color "$(printf 'a <b> \nb</b>')""#,
        FORCED,
    );
    let row = only_row("prose 'a <b> \\nb</b>'", &frame, &rows);
    assert_eq!(row.plain, "a b", "soft-break whitespace across a wrapper is one space: {row:?}");
    assert_eq!(
        active_runs(&attr_run_cells(&row.raw, "1"), true),
        ["b"],
        "the bold keeps its extent: {row:?}"
    );

    let command = "prose --force-color '[a<code-block>x</code-block>b](https://e.io)'";
    let (frame, rows) = run_bt_rows(harness, command, FORCED);
    let shown: Vec<&OutputRow> = rows.iter().filter(|row| !row.plain.trim().is_empty()).collect();
    let first = shown.first().unwrap_or_else(|| panic!("no output.\nplain:\n{}", frame.plain));
    let last = shown.last().expect("output rows");
    assert_eq!(first.plain, "a", "{:?}", plain_rows(&rows));
    assert_eq!(last.plain, "b", "{:?}", plain_rows(&rows));
    for row in [first, last] {
        assert_eq!(link_runs(row), [row.plain.as_str()], "{row:?}");
        assert_eq!(osc8_destinations(&row.raw), ["https://e.io"], "{row:?}");
    }
    let code = shown
        .iter()
        .find(|row| row.plain.trim() == "x")
        .unwrap_or_else(|| panic!("the code row is missing: {:?}", plain_rows(&rows)));
    assert_eq!(dim_runs(code), ["x"], "{code:?}");
    assert!(link_runs(code).is_empty(), "the code block is not linked: {code:?}");
}

#[test]
#[serial(level2_terminal)]
fn level2_prose_inline_code_and_breaks_in_wezterm() {
    require_level!(Level::L2, WezTermHarness::available(), Backend::WezTerm);
    let mut guard = SHARED_WEZTERM
        .get_or_init(|| WezTermHarness::shared_or_spawn().expect("attach/spawn WezTerm"));
    let harness = guard.as_mut().expect("shared WezTerm harness present");
    assert_inline_code_display(harness);
    assert_break_geometry_display(harness);
}

#[test]
#[serial(level2_terminal)]
fn level2_prose_inline_code_and_breaks_in_kitty() {
    require_level!(Level::L2, KittyHarness::available(), Backend::Kitty);
    let mut guard =
        SHARED_KITTY.get_or_init(|| KittyHarness::shared_or_spawn().expect("attach/spawn kitty"));
    let harness = guard.as_mut().expect("shared Kitty harness present");
    assert_inline_code_display(harness);
    assert_break_geometry_display(harness);
}

#[test]
#[serial(level2_terminal)]
fn level2_prose_inline_code_and_breaks_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    let mut guard =
        SHARED_TMUX.get_or_init(|| TmuxHarness::shared_or_spawn().expect("attach/spawn tmux"));
    let harness = guard.as_mut().expect("shared tmux harness present");
    assert_inline_code_display(harness);
    assert_break_geometry_display(harness);
}
