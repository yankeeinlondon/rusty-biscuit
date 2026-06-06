//! Level-2 tests for the migrated `BlockQuote` component's declared `Style`.
//!
//! `BlockQuote`'s former bespoke `text_color` / `bg_color` / `left_block_color`
//! fields were migrated to a declared `renderable::style::Style` (Spec B). The
//! `bt quote` command renders a `BlockQuote`, so these tests drive the migrated
//! component through the `bt` CLI inside a real terminal emulator and verify
//! the declared left border — glyph and color — survives to the cells the
//! emulator actually displays.
//!
//! The lower-level byte/SGR behavior of the `Style` primitive is guarded by
//! Level-1 tests in `render_tree::style` and `render_tree::render`; these
//! Level-2 tests confirm the user-visible result in a real terminal.
//!
//! ## Skip-clean contract
//!
//! Every test checks `Harness::available()` before spawning. When the required
//! terminal is absent the test prints `skipping: requires <X>` to stderr and
//! returns immediately. No `#[ignore]` markers are used.

use biscuit_test_harness::kitty::KittyHarness;
use biscuit_test_harness::shared::SharedHarness;
use biscuit_test_harness::tmux::TmuxHarness;
use biscuit_test_harness::wezterm::WezTermHarness;
use biscuit_test_harness::{CapturedFrame, TerminalHarness};
use serial_test::serial;
use test_toolkit::{Level, require_level};

/// Process-shared WezTerm pane reused across the WezTerm tests in this
/// file. A `clear` is sent before each test's first interaction so
/// prior renders cannot leak into the capture window.
static SHARED_WEZTERM: SharedHarness<WezTermHarness> = SharedHarness::new();

/// Process-shared Kitty window reused across the Kitty tests in this
/// file.
static SHARED_KITTY: SharedHarness<KittyHarness> = SharedHarness::new();

/// Process-shared tmux session reused across the tmux tests in this
/// file.
static SHARED_TMUX: SharedHarness<TmuxHarness> = SharedHarness::new();

/// Unique words in the quoted text so the rendered output row can be
/// isolated from the shell prompt and the command echo.
const QUOTE_TEXT: &str = "kingfisher azimuth quorum";

/// A word from [`QUOTE_TEXT`] unlikely to appear in a shell prompt.
const QUOTE_NEEDLE: &str = "kingfisher";

/// The left-border glyph the migrated `BlockQuote` `Style` declares
/// (`renderable::style::BorderSides::Sides { left: true, .. }`).
const BORDER_GLYPH: char = '│';

/// Sends a `bt quote` with color forced on and returns the captured frame.
fn capture_styled_quote<H: TerminalHarness>(harness: &mut H) -> CapturedFrame {
    harness
        .send_command_with_env(
            &format!("bt quote \"{QUOTE_TEXT}\""),
            &[("FORCE_COLOR", "1")],
        )
        .expect("send_command_with_env failed");
    biscuit_test_harness::capture_settled(harness).expect("capture failed")
}

/// Returns the raw (escape-bearing) capture line for the rendered block
/// quote: the row whose plain text carries both the border glyph and the
/// quoted content, skipping the command-echo line.
fn quote_output_row(frame: &CapturedFrame) -> Option<String> {
    let raw_lines: Vec<&str> = frame.raw.lines().collect();
    for (i, plain) in frame.plain.lines().enumerate() {
        // The command-echo line carries the literal `bt quote` invocation.
        if plain.contains("bt quote") {
            continue;
        }
        if plain.contains(BORDER_GLYPH) && plain.contains(QUOTE_NEEDLE) {
            return raw_lines.get(i).map(|raw| (*raw).to_string());
        }
    }
    None
}

/// Asserts the migrated `BlockQuote` border survives to the real terminal:
/// the border glyph is displayed and it carries a truecolor SGR escape.
fn assert_styled_border<H: TerminalHarness>(harness: &mut H) {
    let frame = capture_styled_quote(harness);

    let row = quote_output_row(&frame).unwrap_or_else(|| {
        panic!(
            "could not locate the `bt quote` output row carrying the \
             border glyph and {QUOTE_NEEDLE:?}.\nplain:\n{}\nraw:\n{}",
            frame.plain, frame.raw,
        )
    });

    assert!(
        row.contains(BORDER_GLYPH),
        "expected the migrated BlockQuote left border glyph in the \
         captured row: {row:?}",
    );
    // The declared `Style.border` color (Tailwind Gray500) lowers to a
    // truecolor foreground SGR escape on a color-capable terminal. WezTerm's
    // `get-text` capture re-emits truecolor in the colon form
    // (`\x1b[38:2::r:g:bm`); Kitty uses the semicolon form — accept both.
    assert!(
        row.contains("\x1b[38;2;") || row.contains("\x1b[38:2:"),
        "expected the declared border color to lower to a truecolor SGR \
         escape in the captured row: {row:?}",
    );
    // The quoted text must remain visible alongside the border.
    assert!(
        frame.plain.contains(QUOTE_NEEDLE),
        "expected the quoted text to remain visible. plain:\n{}",
        frame.plain,
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_block_quote_style_border_in_wezterm() {
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
    assert_styled_border(harness);
}

#[test]
#[serial(level2_terminal)]
fn level2_block_quote_style_border_in_kitty() {
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
    assert_styled_border(harness);
}

#[test]
#[serial(level2_terminal)]
fn level2_block_quote_style_border_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let mut guard = SHARED_TMUX
        .get_or_init(|| TmuxHarness::shared_or_spawn().expect("attach/spawn tmux"));
    let harness = guard.as_mut().expect("shared tmux harness present");
    harness.send_text(b"clear\n").expect("send_text failed");
    harness.settle();
    let frame = capture_styled_quote(harness);

    // tmux faithfully relays text glyphs: the declared left border must be
    // visible in the displayed cells even though tmux carries no image or
    // styling protocol of its own.
    let row = quote_output_row(&frame).unwrap_or_else(|| {
        panic!(
            "could not locate the `bt quote` output row in tmux.\n\
             plain:\n{}\nraw:\n{}",
            frame.plain, frame.raw,
        )
    });
    assert!(
        row.contains(BORDER_GLYPH),
        "expected the migrated BlockQuote left border glyph in the tmux \
         capture: {row:?}",
    );
    assert!(
        frame.plain.contains(QUOTE_NEEDLE),
        "expected the quoted text to remain visible in tmux. plain:\n{}",
        frame.plain,
    );
    // No raw escape garbage should leak into the visible plain cells.
    assert!(
        !frame.plain.contains('\x1b'),
        "expected no raw escape bytes in the tmux plain capture. plain:\n{}",
        frame.plain,
    );
}

// ---------------------------------------------------------------------------
// Styled inline content inside the migrated BlockQuote
// ---------------------------------------------------------------------------

/// `bt quote` content carrying an inline color span followed by trailing
/// plain text. The compact visible form is `alphabravo`.
const STYLED_QUOTE_INPUT: &str = "<red>alpha</red> bravo";

/// Drives `bt quote` with [`STYLED_QUOTE_INPUT`] and asserts the styled
/// inline span (`alpha`, red) and the trailing plain text (`bravo`) both
/// render inside the bordered quote in the given real terminal.
///
/// This is the Level-2 end-to-end check for styled inline content flowing
/// through the migrated `BlockQuote` component: the renderer applies the
/// span color and then continues the trailing run, all within the declared
/// left border.
fn assert_styled_inline_content<H: TerminalHarness>(harness: &mut H) {
    harness
        .send_command_with_env(
            &format!("bt quote \"{STYLED_QUOTE_INPUT}\""),
            &[("FORCE_COLOR", "1")],
        )
        .expect("send_command_with_env failed");
    let _ = biscuit_test_harness::wait_for_prompt(harness);
    std::thread::sleep(std::time::Duration::from_millis(200));
    let frame = harness.capture().expect("capture failed");

    // Isolate the rendered quote row by its compact visible text.
    let raw_lines: Vec<&str> = frame.raw.lines().collect();
    let row = frame
        .plain
        .lines()
        .enumerate()
        .find(|(_, plain)| {
            !plain.contains("bt quote") && plain.contains(BORDER_GLYPH) && plain.contains("alpha")
        })
        .and_then(|(i, _)| raw_lines.get(i).copied())
        .unwrap_or_else(|| {
            panic!(
                "could not locate the styled `bt quote` output row.\n\
                 plain:\n{}\nraw:\n{}",
                frame.plain, frame.raw,
            )
        });

    assert!(
        row.contains(BORDER_GLYPH),
        "expected the BlockQuote border around styled content: {row:?}"
    );
    assert!(
        row.contains("\x1b[31m") || row.contains("\x1b[91m"),
        "expected the inline red span to render as SGR: {row:?}"
    );
    assert!(
        frame.plain.contains("alpha") && frame.plain.contains("bravo"),
        "expected both the styled span and the trailing text to remain \
         visible. plain:\n{}",
        frame.plain,
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_block_quote_styled_inline_content_in_wezterm() {
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
    assert_styled_inline_content(harness);
}

#[test]
#[serial(level2_terminal)]
fn level2_block_quote_styled_inline_content_in_kitty() {
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
    assert_styled_inline_content(harness);
}

// ---------------------------------------------------------------------------
// Render-tree `Style` reachable via `bt block` / `bt progress` / `bt table`
// ---------------------------------------------------------------------------
//
// These commands render through `render_terminal_node`, so they exercise the
// render-tree `Style` primitive (generic foreground/background/emphasis/fill/
// border), `Progress` slot colors, and `Table` striping in a real terminal.
// The byte/SGR behavior of each layer is guarded by Level-1 tests in
// `render_tree::style`; these confirm the styling survives to the cells the
// emulator actually displays.

/// Runs a `bt` command with color forced on and returns the captured frame.
fn capture_bt<H: TerminalHarness>(harness: &mut H, cmd: &str) -> CapturedFrame {
    harness
        .send_command_with_env(cmd, &[("FORCE_COLOR", "1")])
        .expect("send_command_with_env failed");
    biscuit_test_harness::capture_settled(harness).expect("capture failed")
}

/// Returns the raw (escape-bearing) capture line whose plain text contains
/// `needle`, skipping the command-echo line that carries the literal
/// invocation `echo_marker`.
fn output_row(frame: &CapturedFrame, echo_marker: &str, needle: &str) -> Option<String> {
    let raw_lines: Vec<&str> = frame.raw.lines().collect();
    for (i, plain) in frame.plain.lines().enumerate() {
        if plain.contains(echo_marker) {
            continue;
        }
        if plain.contains(needle) {
            return raw_lines.get(i).map(|raw| (*raw).to_string());
        }
    }
    None
}

/// Asserts a `bt block --fg red` foreground SGR survives to the captured row.
fn assert_block_fg<H: TerminalHarness>(harness: &mut H) {
    let frame = capture_bt(harness, "bt block \"crimson glyphwork\" --fg red");
    let row = output_row(&frame, "bt block", "crimson glyphwork")
        .unwrap_or_else(|| panic!("could not locate `bt block` row.\nplain:\n{}", frame.plain));
    // A named basic color lowers to a 16-color SGR; some terminals re-emit it
    // as truecolor on capture. Accept the documented forms.
    assert!(
        row.contains("\x1b[31m")
            || row.contains("\x1b[91m")
            || row.contains("\x1b[38;2;")
            || row.contains("\x1b[38:2:"),
        "expected a foreground SGR escape in the captured row: {row:?}",
    );
    assert!(
        frame.plain.contains("crimson glyphwork"),
        "expected the block text to remain visible. plain:\n{}",
        frame.plain,
    );
}

/// Asserts a `bt block --bg blue` background SGR survives to the captured row.
fn assert_block_bg<H: TerminalHarness>(harness: &mut H) {
    let frame = capture_bt(harness, "bt block \"azureground marker\" --bg blue");
    let row = output_row(&frame, "bt block", "azureground marker")
        .unwrap_or_else(|| panic!("could not locate `bt block` row.\nplain:\n{}", frame.plain));
    assert!(
        row.contains("\x1b[44m") || row.contains("\x1b[48;2;") || row.contains("\x1b[48:2:"),
        "expected a background SGR escape in the captured row: {row:?}",
    );
}

/// Asserts a `bt block --bold` weight SGR survives to the captured row.
///
/// Kitty emits the bold attribute as a standalone `\x1b[1m`; WezTerm's
/// `get-text --escapes` coalesces it into a combined run such as
/// `\x1b[0;1m`. Accept either: the bold attribute (`1`) carried by any SGR
/// sequence in the row.
fn assert_block_bold<H: TerminalHarness>(harness: &mut H) {
    let frame = capture_bt(harness, "bt block \"weighty pronouncement\" --bold");
    let row = output_row(&frame, "bt block", "weighty pronouncement")
        .unwrap_or_else(|| panic!("could not locate `bt block` row.\nplain:\n{}", frame.plain));
    assert!(
        sgr_carries_bold(&row),
        "expected a bold (`1`) SGR attribute in the captured row: {row:?}",
    );
}

/// Returns `true` when `row` contains an SGR sequence (`ESC [ … m`) whose
/// semicolon-separated parameter list includes the bold attribute `1`.
fn sgr_carries_bold(row: &str) -> bool {
    let mut rest = row;
    while let Some(start) = rest.find("\x1b[") {
        let after = &rest[start + 2..];
        let Some(end) = after.find('m') else {
            break;
        };
        let params = &after[..end];
        // SGR parameters are digits separated by `;` (or `:`).
        if params
            .chars()
            .all(|c| c.is_ascii_digit() || c == ';' || c == ':')
            && params.split([';', ':']).any(|p| p == "1")
        {
            return true;
        }
        rest = &after[end + 1..];
    }
    false
}

/// Asserts a `bt block --border all` row carries box-drawing border glyphs.
fn assert_block_border<H: TerminalHarness>(harness: &mut H) {
    let frame = capture_bt(harness, "bt block \"bordered notice\" --border all");
    assert!(
        frame.plain.contains('┌') && frame.plain.contains('│') && frame.plain.contains('└'),
        "expected box-drawing border glyphs in the capture.\nplain:\n{}",
        frame.plain,
    );
    assert!(
        frame.plain.contains("bordered notice"),
        "expected the block text inside the border. plain:\n{}",
        frame.plain,
    );
}

/// Asserts a `bt block --border all --border-radius 1` row carries the
/// light-arc (rounded) corner glyphs rather than the square set.
fn assert_block_rounded_border<H: TerminalHarness>(harness: &mut H) {
    let frame = capture_bt(
        harness,
        "bt block \"rounded notice\" --border all --border-radius 1",
    );
    assert!(
        frame.plain.contains('╭')
            && frame.plain.contains('╮')
            && frame.plain.contains('╰')
            && frame.plain.contains('╯'),
        "expected light-arc corner glyphs from --border-radius in the \
         capture.\nplain:\n{}",
        frame.plain,
    );
    // `--border-radius` must select the rounded corner set *for this block*,
    // never the square set. A whole-frame `!contains('┌')` check is invalid:
    // an earlier `bt block --border all` leaves a square-bordered block in
    // scrollback. Scope the negative check to the rounded block's own border
    // rows — the line carrying `╭`/`╮` (top) and the line carrying `╰`/`╯`
    // (bottom) — which sit on different rows from any earlier square block.
    let top_border = frame
        .plain
        .lines()
        .find(|line| line.contains('╭'))
        .unwrap_or_else(|| {
            panic!(
                "could not locate the rounded block's top border row.\n\
                 plain:\n{}",
                frame.plain,
            )
        });
    assert!(
        !top_border.contains('┌') && !top_border.contains('┐'),
        "expected the rounded block's top border row to use arc corners \
         exclusively, found a square corner glyph: {top_border:?}",
    );
    let bottom_border = frame
        .plain
        .lines()
        .find(|line| line.contains('╰'))
        .unwrap_or_else(|| {
            panic!(
                "could not locate the rounded block's bottom border row.\n\
                 plain:\n{}",
                frame.plain,
            )
        });
    assert!(
        !bottom_border.contains('└') && !bottom_border.contains('┘'),
        "expected the rounded block's bottom border row to use arc corners \
         exclusively, found a square corner glyph: {bottom_border:?}",
    );
    assert!(
        frame.plain.contains("rounded notice"),
        "expected the block text inside the rounded border. plain:\n{}",
        frame.plain,
    );
}

/// Asserts `bt progress` slot colors survive: the filled track is green and
/// the brackets are cyan, with the `50%` text visible.
fn assert_progress_slot_colors<H: TerminalHarness>(harness: &mut H) {
    let frame = capture_bt(
        harness,
        "bt progress 50 --fill-color green --bracket-color cyan",
    );
    let row = output_row(&frame, "bt progress", "50%").unwrap_or_else(|| {
        panic!(
            "could not locate `bt progress` row.\nplain:\n{}",
            frame.plain
        )
    });
    // Green filled track (fg 32) and cyan brackets (fg 36); truecolor
    // re-emission is accepted for terminals that promote basic colors.
    assert!(
        row.contains("\x1b[32m") || row.contains("\x1b[38;2;") || row.contains("\x1b[38:2:"),
        "expected the green filled-track SGR in the captured row: {row:?}",
    );
    assert!(
        row.contains("\x1b[36m") || row.contains("\x1b[38;2;") || row.contains("\x1b[38:2:"),
        "expected the cyan bracket SGR in the captured row: {row:?}",
    );
    assert!(
        frame.plain.contains("50%"),
        "expected the progress percentage to remain visible. plain:\n{}",
        frame.plain,
    );
}

/// Asserts a striped `bt table` carries a background SGR on a striped row and
/// keeps the cell text visible.
fn assert_table_striped<H: TerminalHarness>(harness: &mut H) {
    let frame = capture_bt(
        harness,
        "bt table --columns \"Name,Score\" --row \"Annwyl,90\" --row \"Bertrand,75\" --striped",
    );
    // The second data row is the striped one. Locate the rendered table row
    // by the border glyph plus the cell text — the command echo carries
    // `Bertrand` too but never the box-drawing vertical edge.
    let raw_lines: Vec<&str> = frame.raw.lines().collect();
    let row = frame
        .plain
        .lines()
        .enumerate()
        .find(|(_, plain)| plain.contains('│') && plain.contains("Bertrand"))
        .and_then(|(i, _)| raw_lines.get(i).map(|r| (*r).to_string()))
        .unwrap_or_else(|| {
            panic!(
                "could not locate striped `bt table` row.\nplain:\n{}",
                frame.plain
            )
        });
    assert!(
        row.contains("\x1b[48;2;") || row.contains("\x1b[48:2:") || row.contains("\x1b[48;5;"),
        "expected a striped-row background SGR in the captured row: {row:?}",
    );
    assert!(
        frame.plain.contains("Annwyl") && frame.plain.contains("Bertrand"),
        "expected the table cell text to remain visible. plain:\n{}",
        frame.plain,
    );
}

/// Asserts a `bt table` with typed header / body slot styling renders those
/// slots visibly in a real terminal: the header row carries a bold SGR
/// attribute and a data row carries a foreground-color SGR.
fn assert_table_styled<H: TerminalHarness>(harness: &mut H) {
    let frame = capture_bt(
        harness,
        "bt table --columns \"Pipeline,Verdict\" --row \"Quokka,Affirmed\" \
         --bold-header --body-color red",
    );
    // The header row is the one that carries the column name but no data.
    let header = frame
        .plain
        .lines()
        .enumerate()
        .find(|(_, plain)| {
            !plain.contains("bt table") && plain.contains('│') && plain.contains("Pipeline")
        })
        .and_then(|(i, _)| frame.raw.lines().nth(i).map(str::to_string))
        .unwrap_or_else(|| {
            panic!(
                "could not locate styled `bt table` header row.\nplain:\n{}",
                frame.plain
            )
        });
    assert!(
        sgr_carries_bold(&header),
        "expected a bold SGR attribute in the styled table header: {header:?}",
    );

    // The data row carries the body slot's red foreground color.
    let body = frame
        .plain
        .lines()
        .enumerate()
        .find(|(_, plain)| {
            !plain.contains("bt table") && plain.contains('│') && plain.contains("Quokka")
        })
        .and_then(|(i, _)| frame.raw.lines().nth(i).map(str::to_string))
        .unwrap_or_else(|| {
            panic!(
                "could not locate styled `bt table` body row.\nplain:\n{}",
                frame.plain
            )
        });
    assert!(
        body.contains("\x1b[31m")
            || body.contains("\x1b[91m")
            || body.contains("\x1b[38;2;")
            || body.contains("\x1b[38:2:")
            || body.contains("\x1b[38;5;"),
        "expected a body foreground-color SGR in the styled table row: {body:?}",
    );
    assert!(
        frame.plain.contains("Pipeline") && frame.plain.contains("Quokka"),
        "expected the styled table text to remain visible. plain:\n{}",
        frame.plain,
    );
}

// ---------------------------------------------------------------------------
// Render-tree box model: padding, width modes, alignment, border adjacency
// ---------------------------------------------------------------------------
//
// `bt block` exposes the `Layout` box model (`--padding`, `--margin`, `--width`,
// `--align`) so the renderer-folds box behavior is observable in a real
// terminal. The cell geometry and SGR are pinned by Level-1 tests in
// `render_tree::render`; these confirm the user-visible result on the screen.

/// The first plain capture row carrying `needle` (skipping the command echo
/// `echo_marker`), paired with its leading-space count.
fn row_leading_spaces(frame: &CapturedFrame, echo_marker: &str, needle: &str) -> Option<usize> {
    frame
        .plain
        .lines()
        .find(|plain| !plain.contains(echo_marker) && plain.contains(needle))
        .map(|line| line.len() - line.trim_start().len())
}

/// Whether a capture string carries a background SGR escape (basic blue,
/// truecolor semicolon, or truecolor colon form).
fn has_background(s: &str) -> bool {
    s.contains("\x1b[44m") || s.contains("\x1b[48;2;") || s.contains("\x1b[48:2:")
}

/// Asserts `bt block --bg ... --padding 1` paints the padding cells: a
/// background band reaches the terminal and the padding reserves a cell before
/// the content. Needs a styling-capable terminal (WezTerm / Kitty). A per-row
/// index check is avoided because a multi-row painted band desynchronizes the
/// `plain`/`raw` line mapping in some terminals' capture.
fn assert_block_painted_padding<H: TerminalHarness>(harness: &mut H) {
    let frame = capture_bt(harness, "bt block \"padglyph\" --bg blue --padding 1");
    // The painted band reached the terminal (the background SGR survives).
    assert!(
        has_background(&frame.raw),
        "expected a painted background band in the capture.\nraw:\n{}",
        frame.raw,
    );
    // Padding reserves a cell: the content row shows a space immediately before
    // the text (the left padding column), widening the box beyond the content.
    assert!(
        frame.plain.lines().any(|l| l.contains(" padglyph")),
        "expected the left padding column before the content.\nplain:\n{}",
        frame.plain,
    );
}

/// Asserts border interior spacing is owned by `padding`: a bordered block with
/// no padding renders the content adjacent to the drawn edge (`│snug`), while
/// adding `--padding 1` restores the one-cell gap (`│ roomy`).
fn assert_block_border_adjacency<H: TerminalHarness>(harness: &mut H) {
    let snug = capture_bt(harness, "bt block \"snug\" --border all");
    assert!(
        snug.plain.contains("│snug"),
        "a no-padding border must sit adjacent to its content.\nplain:\n{}",
        snug.plain,
    );

    let roomy = capture_bt(harness, "bt block \"roomy\" --border all --padding 1");
    assert!(
        roomy.plain.contains("│ roomy"),
        "--padding 1 must restore the one-cell border gap.\nplain:\n{}",
        roomy.plain,
    );
}

/// Asserts a sub-available `Fixed` box is placed within the terminal width by
/// `--align`: right pushes the box further than center, which pushes further
/// than left (which stays flush). Geometry only — a tmux text capture suffices.
///
/// Each alignment uses a distinct needle because the captures accumulate in the
/// shared pane; a shared needle would always match the first (left) row.
fn assert_block_alignment_placement<H: TerminalHarness>(harness: &mut H) {
    let left = capture_bt(harness, "bt block \"leftmark\" --width 20 --align left");
    let left_lead = row_leading_spaces(&left, "bt block", "leftmark")
        .unwrap_or_else(|| panic!("left-aligned row missing.\nplain:\n{}", left.plain));
    let center = capture_bt(harness, "bt block \"centermark\" --width 20 --align center");
    let center_lead = row_leading_spaces(&center, "bt block", "centermark")
        .unwrap_or_else(|| panic!("center-aligned row missing.\nplain:\n{}", center.plain));
    let right = capture_bt(harness, "bt block \"rightmark\" --width 20 --align right");
    let right_lead = row_leading_spaces(&right, "bt block", "rightmark")
        .unwrap_or_else(|| panic!("right-aligned row missing.\nplain:\n{}", right.plain));

    assert!(
        left_lead <= 1,
        "left placement should be flush left, got {left_lead} lead"
    );
    assert!(
        center_lead > left_lead,
        "center placement should indent past left ({center_lead} vs {left_lead})"
    );
    assert!(
        right_lead > center_lead,
        "right placement should indent past center ({right_lead} vs {center_lead})"
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_render_tree_style_in_wezterm() {
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
    assert_block_fg(harness);
    assert_block_bg(harness);
    assert_block_bold(harness);
    assert_block_border(harness);
    assert_block_rounded_border(harness);
    assert_progress_slot_colors(harness);
    assert_table_striped(harness);
    assert_table_styled(harness);
    assert_block_painted_padding(harness);
    assert_block_border_adjacency(harness);
    assert_block_alignment_placement(harness);
}

#[test]
#[serial(level2_terminal)]
fn level2_render_tree_style_in_kitty() {
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
    assert_block_fg(harness);
    assert_block_bg(harness);
    assert_block_bold(harness);
    assert_block_border(harness);
    assert_block_rounded_border(harness);
    assert_progress_slot_colors(harness);
    assert_table_striped(harness);
    assert_table_styled(harness);
    assert_block_painted_padding(harness);
    assert_block_border_adjacency(harness);
    assert_block_alignment_placement(harness);
}

#[test]
#[serial(level2_terminal)]
fn level2_render_tree_style_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");

    let mut guard = SHARED_TMUX
        .get_or_init(|| TmuxHarness::shared_or_spawn().expect("attach/spawn tmux"));
    let harness = guard.as_mut().expect("shared tmux harness present");
    harness.send_text(b"clear\n").expect("send_text failed");
    harness.settle();

    // tmux carries no styling protocol of its own, but it faithfully relays
    // text glyphs. Assert the border glyphs and visible text survive and that
    // no raw escape garbage leaks into the displayed cells.
    let frame = capture_bt(harness, "bt block \"bordered notice\" --border all");
    assert!(
        frame.plain.contains('┌') && frame.plain.contains('│') && frame.plain.contains('└'),
        "expected box-drawing border glyphs in the tmux capture.\nplain:\n{}",
        frame.plain,
    );
    assert!(
        frame.plain.contains("bordered notice"),
        "expected the block text in tmux. plain:\n{}",
        frame.plain,
    );
    assert!(
        !frame.plain.contains('\x1b'),
        "expected no raw escape bytes in the tmux plain capture. plain:\n{}",
        frame.plain,
    );

    // A rounded border must relay its light-arc corner glyphs through tmux.
    let frame = capture_bt(
        harness,
        "bt block \"rounded notice\" --border all --border-radius 1",
    );
    assert!(
        frame.plain.contains('╭') && frame.plain.contains('╯'),
        "expected light-arc corner glyphs in the tmux capture.\nplain:\n{}",
        frame.plain,
    );

    // The progress bar and striped table glyphs must also relay through tmux.
    let frame = capture_bt(harness,"bt progress 50 --label Loading");
    assert!(
        frame.plain.contains("50%") && frame.plain.contains("Loading"),
        "expected the progress bar text in tmux. plain:\n{}",
        frame.plain,
    );

    let frame = capture_bt(
        harness,
        "bt table --columns \"Name,Score\" --row \"Annwyl,90\" --row \"Bertrand,75\" --striped",
    );
    assert!(
        frame.plain.contains("Annwyl") && frame.plain.contains("Bertrand"),
        "expected the table cell text in tmux. plain:\n{}",
        frame.plain,
    );
    assert!(
        !frame.plain.contains('\x1b'),
        "expected no raw escape bytes in the tmux table capture. plain:\n{}",
        frame.plain,
    );

    // A table with typed header / body slot styling relays its text faithfully
    // through tmux with no escape garbage in the displayed cells.
    let frame = capture_bt(
        harness,
        "bt table --columns \"Pipeline,Verdict\" --row \"Quokka,Affirmed\" \
         --bold-header --body-color red",
    );
    assert!(
        frame.plain.contains("Pipeline") && frame.plain.contains("Quokka"),
        "expected the styled table text in tmux. plain:\n{}",
        frame.plain,
    );
    assert!(
        !frame.plain.contains('\x1b'),
        "expected no raw escape bytes in the tmux styled-table capture. plain:\n{}",
        frame.plain,
    );

    // Box-model geometry relays faithfully through tmux: border interior gap is
    // owned by padding, and `--align` places a sub-available box.
    assert_block_border_adjacency(harness);
    assert_block_alignment_placement(harness);
}
