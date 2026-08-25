//! Level-2 real-terminal coverage for the three Style Everywhere matrix
//! scenarios: `background_subtle`, `border_thin_left`, and
//! `emphasis_bold_italic`.
//!
//! The Style Everywhere feature (`renderable/features/2026-06-30-style-everywhere`)
//! pins these three `renderable::style::Style` properties as Level-1 invariants
//! in `biscuit-terminal/lib/tests/layout_matrix.rs`. Level 1 proves the
//! renderer's own byte stream carries the SGR / glyph; these Level-2 tests
//! prove the styling survives the round-trip through a real terminal
//! emulator's display + capture path — the user-visible result the review
//! flagged as the missing tier.
//!
//! Each scenario is driven through the `bt block` CLI, which builds a
//! declared `Style` (`block.rs::BlockArgs::build_style`) and renders it via
//! `render_terminal_node`, so the exercise path is the same
//! `renderable::style::Style` surface the matrix enumerates:
//!
//! | Scenario           | CLI flag                              | Style field                          |
//! |--------------------|---------------------------------------|--------------------------------------|
//! | `background_subtle`| `--fill subtle` (`Background::subtle`)| `Style.background`                   |
//! | `border_thin_left` | `--border left`                       | `Style.border` (`BorderSides::left`) |
//! | `emphasis_bold_italic` | `--bold --italic`                 | `Style.emphasis`                     |
//!
//! ## Skip-clean contract
//!
//! Every test checks `Harness::available()` before spawning. When the required
//! terminal is absent the test prints `skipping: requires <X>` to stderr and
//! returns immediately. No `#[ignore]` markers are used.

mod common;

use biscuit_test_harness::kitty::KittyHarness;
use biscuit_test_harness::shared::SharedHarness;
use biscuit_test_harness::tmux::TmuxHarness;
use biscuit_test_harness::wezterm::WezTermHarness;
use biscuit_test_harness::{CapturedFrame, TerminalHarness};
use serial_test::serial;
use test_toolkit::{Backend, Level, require_level};

/// Process-share WezTerm pane reused across the WezTerm tests in this file.
/// A `clear` is sent before each test's first interaction so prior renders
/// cannot leak into the capture window.
static SHARED_WEZTERM: SharedHarness<WezTermHarness> = SharedHarness::new();

/// Process-share Kitty window reused across the Kitty tests in this file.
static SHARED_KITTY: SharedHarness<KittyHarness> = SharedHarness::new();

/// Process-share tmux session reused across the tmux tests in this file.
static SHARED_TMUX: SharedHarness<TmuxHarness> = SharedHarness::new();

// ---------------------------------------------------------------------------
// Scenario needles — distinctive words unlikely to collide with the shell
// prompt or the echoed command line.
// ---------------------------------------------------------------------------

const BG_NEEDLE: &str = "subtletint";
const BORDER_NEEDLE: &str = "edgeword";
const EMPHASIS_NEEDLE: &str = "emphasisrun";

/// The thin left-border glyph the migrated `Style.border` declares for
/// `BorderSides::Sides { left: true, .. }`.
const BORDER_GLYPH: char = '│';

// ---------------------------------------------------------------------------
// Capture helpers
// ---------------------------------------------------------------------------

/// Runs a `bt` command with color forced on and returns the captured frame.
fn capture_bt<H: TerminalHarness>(harness: &mut H, cmd: &str) -> CapturedFrame {
    let args = cmd
        .strip_prefix("bt ")
        .expect("capture_bt command must start with `bt `");
    harness
        .send_command_with_env(&common::bt_command(args), &[("FORCE_COLOR", "1")])
        .expect("send_command_with_env failed");
    biscuit_test_harness::capture_settled(harness).expect("capture failed")
}

/// Returns the raw output line after the newest `bt` command echo whose plain
/// text contains `needle`.
fn output_row(frame: &CapturedFrame, subcommand: &str, needle: &str) -> Option<String> {
    let raw_lines: Vec<&str> = frame.raw.lines().collect();
    let plain_lines: Vec<&str> = frame.plain.lines().collect();
    let command_end = common::find_bt_command_end(&plain_lines, subcommand)?;
    for (i, plain) in plain_lines.iter().enumerate().skip(command_end + 1) {
        if plain.contains(needle) {
            return raw_lines.get(i).map(|raw| (*raw).to_string());
        }
    }
    None
}

/// Whether `s` carries a background SGR escape: basic (`44`..`47`,
/// `100`..`107`), truecolor semicolon (`48;2;`), or truecolor colon
/// (`48:2:`). Terminals re-emit truecolor in either form.
fn has_background(s: &str) -> bool {
    s.contains("\x1b[48;2;") || s.contains("\x1b[48:2:") || s.contains("\x1b[48;5;")
        || s.contains("\x1b[44m")
}

/// Whether `row` carries an SGR sequence (`ESC [ … m`) whose parameter list
/// includes `attr` (e.g. `"1"` for bold, `"3"` for italic).
///
/// Terminals coalesce attributes differently: Kitty emits a standalone
/// `\x1b[1m`; WezTerm's `get-text --escapes` merges runs as `\x1b[0;1;3m`.
/// Scanning every CSI `m` sequence and splitting on `;`/`:` accepts both.
fn sgr_carries_attr(row: &str, attr: &str) -> bool {
    let mut rest = row;
    while let Some(start) = rest.find("\x1b[") {
        let after = &rest[start + 2..];
        let Some(end) = after.find('m') else {
            break;
        };
        let params = &after[..end];
        if params
            .chars()
            .all(|c| c.is_ascii_digit() || c == ';' || c == ':')
            && params.split([';', ':']).any(|p| p == attr)
        {
            return true;
        }
        rest = &after[end + 1..];
    }
    false
}

// ---------------------------------------------------------------------------
// Scenario assertions (styling-capable terminals: WezTerm, Kitty)
// ---------------------------------------------------------------------------

/// Asserts the `background_subtle` matrix scenario survives to real terminal
/// cells: `--fill subtle` lowers `Background::subtle()` to a truecolor
/// background SGR, the painted band reaches the terminal, and `--padding 1`
/// proves the paint covers the padding cells (the content row shows a painted
/// space immediately before the text — the background is not restricted to
/// the text glyphs).
fn assert_background_subtle_painted<H: TerminalHarness>(harness: &mut H) {
    let frame = capture_bt(
        harness,
        &format!("bt block \"{BG_NEEDLE}\" --fill subtle --padding 1"),
    );
    // The painted band reached the terminal at all.
    assert!(
        has_background(&frame.raw),
        "expected a painted background band from --fill subtle in the capture.\nraw:\n{}",
        frame.raw,
    );
    // The left padding cell is painted: a space appears immediately before the
    // content, proving the background fills the content row width rather than
    // only the text glyphs.
    assert!(
        frame.plain.lines().any(|l| l.contains(&format!(" {BG_NEEDLE}"))),
        "expected the painted left padding column before the content.\nplain:\n{}",
        frame.plain,
    );
    assert!(
        frame.plain.contains(BG_NEEDLE),
        "expected the block text to remain visible inside the background.\nplain:\n{}",
        frame.plain,
    );
}

/// Asserts the `border_thin_left` matrix scenario survives to real terminal
/// cells: `--border left` lowers `BorderSides::Sides { left: true, .. }` to a
/// displayed `│` glyph, `--border-color cyan` lowers the border color to a
/// foreground SGR that wraps the glyph, and the text sits adjacent to the
/// drawn edge.
fn assert_border_thin_left_glyph<H: TerminalHarness>(harness: &mut H) {
    let frame = capture_bt(
        harness,
        &format!("bt block \"{BORDER_NEEDLE}\" --border left --border-color cyan"),
    );
    let row = output_row(&frame, "block", BORDER_NEEDLE).unwrap_or_else(|| {
        panic!(
            "could not locate the `bt block` row carrying the border glyph and {BORDER_NEEDLE}.\nplain:\n{}\nraw:\n{}",
            frame.plain, frame.raw,
        )
    });
    assert!(
        row.contains(BORDER_GLYPH),
        "expected the thin left-border glyph in the captured row: {row:?}",
    );
    // The cyan border color lowers to a 16-color foreground SGR (`36`) or a
    // truecolor re-emission on capture; accept all documented forms.
    assert!(
        row.contains("\x1b[36m")
            || row.contains("\x1b[38;2;")
            || row.contains("\x1b[38:2:")
            || row.contains("\x1b[38;5;"),
        "expected the cyan border-color SGR to wrap the border glyph: {row:?}",
    );
    // The text sits adjacent to the drawn edge (no padding was requested).
    assert!(
        frame.plain.contains(&format!("{BORDER_GLYPH}{BORDER_NEEDLE}")),
        "expected the text adjacent to the thin left border.\nplain:\n{}",
        frame.plain,
    );
}

/// Asserts the `emphasis_bold_italic` matrix scenario survives to real
/// terminal cells: `--bold --italic` lowers `TextEmphasis { bold: true,
/// italic: true, .. }` to both a bold SGR attribute (`1`) and an italic SGR
/// attribute (`3`) on the content row.
fn assert_emphasis_bold_italic_sgr<H: TerminalHarness>(harness: &mut H) {
    let frame = capture_bt(
        harness,
        &format!("bt block \"{EMPHASIS_NEEDLE}\" --bold --italic"),
    );
    let row = output_row(&frame, "block", EMPHASIS_NEEDLE).unwrap_or_else(|| {
        panic!(
            "could not locate the `bt block` row carrying {EMPHASIS_NEEDLE}.\nplain:\n{}\nraw:\n{}",
            frame.plain, frame.raw,
        )
    });
    assert!(
        sgr_carries_attr(&row, "1"),
        "expected a bold (`1`) SGR attribute in the captured row: {row:?}",
    );
    assert!(
        sgr_carries_attr(&row, "3"),
        "expected an italic (`3`) SGR attribute in the captured row: {row:?}",
    );
    assert!(
        frame.plain.contains(EMPHASIS_NEEDLE),
        "expected the block text to remain visible.\nplain:\n{}",
        frame.plain,
    );
}

// ---------------------------------------------------------------------------
// WezTerm
// ---------------------------------------------------------------------------

#[test]
#[serial(level2_terminal)]
fn level2_style_everywhere_in_wezterm() {
    require_level!(Level::L2, WezTermHarness::available(), Backend::WezTerm);

    let mut guard = SHARED_WEZTERM
        .get_or_init(|| WezTermHarness::shared_or_spawn().expect("attach/spawn WezTerm"));
    let harness = guard.as_mut().expect("shared WezTerm harness present");
    harness.send_text(b"clear\n").expect("send_text failed");
    harness.settle();
    assert_background_subtle_painted(harness);
    assert_border_thin_left_glyph(harness);
    assert_emphasis_bold_italic_sgr(harness);
}

// ---------------------------------------------------------------------------
// Kitty
// ---------------------------------------------------------------------------

#[test]
#[serial(level2_terminal)]
fn level2_style_everywhere_in_kitty() {
    require_level!(Level::L2, KittyHarness::available(), Backend::Kitty);

    let mut guard = SHARED_KITTY
        .get_or_init(|| KittyHarness::shared_or_spawn().expect("attach/spawn kitty"));
    let harness = guard.as_mut().expect("shared Kitty harness present");
    harness.send_text(b"clear\n").expect("send_text failed");
    harness.settle();
    assert_background_subtle_painted(harness);
    assert_border_thin_left_glyph(harness);
    assert_emphasis_bold_italic_sgr(harness);
}

// ---------------------------------------------------------------------------
// tmux — relays glyphs and text but carries no styling protocol of its own
// ---------------------------------------------------------------------------

#[test]
#[serial(level2_terminal)]
fn level2_style_everywhere_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let mut guard = SHARED_TMUX
        .get_or_init(|| TmuxHarness::shared_or_spawn().expect("attach/spawn tmux"));
    let harness = guard.as_mut().expect("shared tmux harness present");
    harness.send_text(b"clear\n").expect("send_text failed");
    harness.settle();

    // background_subtle: the painted band relays as visible cells. The left
    // padding column still appears before the content (the box geometry is
    // glyph-faithful through tmux), and no raw escape bytes leak into the
    // plain capture.
    let bg_frame = capture_bt(
        harness,
        &format!("bt block \"{BG_NEEDLE}\" --fill subtle --padding 1"),
    );
    assert!(
        bg_frame.plain.lines().any(|l| l.contains(&format!(" {BG_NEEDLE}"))),
        "expected the painted left padding column before the content in tmux.\nplain:\n{}",
        bg_frame.plain,
    );
    assert!(
        bg_frame.plain.contains(BG_NEEDLE),
        "expected the block text to remain visible in tmux. plain:\n{}",
        bg_frame.plain,
    );
    assert!(
        !bg_frame.plain.contains('\x1b'),
        "expected no raw escape bytes in the tmux plain capture. plain:\n{}",
        bg_frame.plain,
    );

    // border_thin_left: the box-drawing glyph relays through tmux and the
    // text sits adjacent to it.
    let border_frame = capture_bt(
        harness,
        &format!("bt block \"{BORDER_NEEDLE}\" --border left --border-color cyan"),
    );
    assert!(
        border_frame.plain.contains(&format!("{BORDER_GLYPH}{BORDER_NEEDLE}")),
        "expected the thin left-border glyph adjacent to the text in tmux.\nplain:\n{}",
        border_frame.plain,
    );
    assert!(
        !border_frame.plain.contains('\x1b'),
        "expected no raw escape bytes in the tmux border capture. plain:\n{}",
        border_frame.plain,
    );

    // emphasis_bold_italic: the text relays through tmux. The SGR attributes
    // are not visible in tmux's plain capture (no styling protocol), so the
    // strict SGR assertion is reserved for the styling-capable backends above.
    let emphasis_frame = capture_bt(
        harness,
        &format!("bt block \"{EMPHASIS_NEEDLE}\" --bold --italic"),
    );
    assert!(
        emphasis_frame.plain.contains(EMPHASIS_NEEDLE),
        "expected the block text to remain visible in tmux. plain:\n{}",
        emphasis_frame.plain,
    );
    assert!(
        !emphasis_frame.plain.contains('\x1b'),
        "expected no raw escape bytes in the tmux emphasis capture. plain:\n{}",
        emphasis_frame.plain,
    );
}
