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
use test_toolkit::{Backend, Level, require_level};

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
    require_level!(Level::L2, WezTermHarness::available(), Backend::WezTerm);

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
    require_level!(Level::L2, KittyHarness::available(), Backend::Kitty);

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
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

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
    require_level!(Level::L2, WezTermHarness::available(), Backend::WezTerm);

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
    require_level!(Level::L2, KittyHarness::available(), Backend::Kitty);

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

/// Asserts `bt table --example` enables row striping without relying on
/// FORCE_COLOR. This mirrors the public example path printed in the command
/// trailer and exercises ordinary biscuit-terminal color detection.
fn assert_table_example_striped_without_force_color<H: TerminalHarness>(harness: &mut H) {
    harness
        .send_command_with_env(
            "env -u NO_COLOR -u FORCE_COLOR -u CLICOLOR_FORCE bt table --example",
            &[],
        )
        .expect("send_command_with_env failed");
    let frame = biscuit_test_harness::capture_settled(harness).expect("capture failed");

    let raw_lines: Vec<&str> = frame.raw.lines().collect();
    let row = frame
        .plain
        .lines()
        .enumerate()
        .find(|(_, plain)| plain.contains('│') && plain.contains("Renderer"))
        .and_then(|(i, _)| raw_lines.get(i).map(|r| (*r).to_string()))
        .unwrap_or_else(|| {
            panic!(
                "could not locate striped `bt table --example` row.\nplain:\n{}",
                frame.plain
            )
        });
    assert!(
        row.contains("\x1b[48;2;") || row.contains("\x1b[48:2:") || row.contains("\x1b[48;5;"),
        "expected `bt table --example` to paint the alternate row using detected terminal color support, without FORCE_COLOR. row: {row:?}\nplain:\n{}\nraw:\n{}",
        frame.plain,
        frame.raw,
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

/// The leading-space count and trimmed visible width of the first plain
/// capture row carrying `needle` (skipping the command echo `echo_marker`).
///
/// For a borderless, unpadded `FitContent` box the trimmed width *is* the drawn
/// box width, so the pair pins both the box geometry and its placement.
fn row_lead_and_width(
    frame: &CapturedFrame,
    echo_marker: &str,
    needle: &str,
) -> Option<(usize, usize)> {
    frame
        .plain
        .lines()
        .find(|plain| !plain.contains(echo_marker) && plain.contains(needle))
        .map(|line| {
            let lead = line.len() - line.trim_start().len();
            let width = line.trim().chars().count();
            (lead, width)
        })
}

/// Whether a capture string carries a background SGR escape (basic blue,
/// truecolor semicolon, or truecolor colon form).
fn has_background(s: &str) -> bool {
    s.contains("\x1b[44m") || s.contains("\x1b[48;2;") || s.contains("\x1b[48:2:")
}

/// The visible width of a fully-boxed content row: the cell span from the
/// leftmost vertical border glyph to the rightmost, inclusive.
///
/// Trailing capture padding sits *outside* the right edge glyph, so the
/// `first..=last` span is the drawn box width regardless of how a terminal pads
/// the captured row. Box rows in these tests carry only ASCII content and box
/// glyphs, so the char span equals the visible-cell width.
fn boxed_row_width(line: &str) -> Option<usize> {
    let chars: Vec<char> = line.chars().collect();
    let first = chars.iter().position(|&c| c == '│')?;
    let last = chars.iter().rposition(|&c| c == '│')?;
    (last > first).then_some(last - first + 1)
}

/// The drawn box width of the bordered block whose content row carries
/// `needle`, read from the displayed plain cells. Terminal-agnostic — the box
/// glyphs relay even through tmux, which carries no styling protocol.
fn box_width_for(frame: &CapturedFrame, needle: &str) -> usize {
    frame
        .plain
        .lines()
        .find(|l| !l.contains("bt block") && l.contains('│') && l.contains(needle))
        .and_then(boxed_row_width)
        .unwrap_or_else(|| {
            panic!("no bordered content row for {needle:?}.\nplain:\n{}", frame.plain)
        })
}

/// Parses a raw capture row into `(transparent_margin, left_padding,
/// right_padding)`: the leading space cells *before* any background SGR opens,
/// and the painted space cells on each side of the content inside the
/// background run.
///
/// A `48;…` / basic background code opens the run, a `0`/`49` reset closes it.
/// SGR forms vary by terminal (semicolon vs ITU colon), so the parameter list
/// is parsed rather than matched byte-for-byte (per the L2 capture rule).
///
/// Both sides are measured because the caller draws a right border glyph after
/// the padding (`--border all`): the right padding cells are no longer trailing,
/// so the terminal's erase-to-end-of-line cannot collapse them. The painted run
/// is collected as the visible cells emitted while the background is open; its
/// leading and trailing space counts are the left and right padding.
fn painted_box_padding(row: &str) -> (usize, usize, usize) {
    let chars: Vec<char> = row.chars().collect();
    let mut idx = 0;
    let mut bg_open = false;
    let mut seen_bg = false;
    let mut margin = 0usize;
    let mut margin_done = false;
    let mut painted = String::new();
    while idx < chars.len() {
        if chars[idx] == '\x1b' && chars.get(idx + 1) == Some(&'[') {
            let start = idx + 2;
            let mut j = start;
            while j < chars.len() && !chars[j].is_ascii_alphabetic() {
                j += 1;
            }
            if chars.get(j) == Some(&'m') {
                let params: String = chars[start..j].iter().collect();
                if let Some(on) = sgr_opens_background(&params) {
                    bg_open = on;
                    if on {
                        seen_bg = true;
                    }
                }
            }
            idx = j + 1;
            continue;
        }
        let c = chars[idx];
        if !seen_bg {
            // Leading transparent margin: spaces before the background opens.
            // A drawn left border glyph (non-space) ends the margin run.
            if c == ' ' && !margin_done {
                margin += 1;
            } else {
                margin_done = true;
            }
        } else if bg_open {
            // Visible cells inside the painted background run.
            painted.push(c);
        }
        idx += 1;
    }
    let left_pad = painted.len() - painted.trim_start().len();
    let right_pad = painted.len() - painted.trim_end().len();
    (margin, left_pad, right_pad)
}

/// Interprets an SGR parameter string: `Some(true)` if it opens a background,
/// `Some(false)` if it resets one, `None` if it is irrelevant.
fn sgr_opens_background(params: &str) -> Option<bool> {
    let parts: Vec<&str> = params.split([';', ':']).collect();
    if params.is_empty() || parts.first() == Some(&"0") {
        return Some(false);
    }
    for part in &parts {
        match *part {
            "48" => return Some(true),
            "49" => return Some(false),
            _ => {
                if let Ok(code) = part.parse::<u16>()
                    && ((40..=47).contains(&code) || (100..=107).contains(&code))
                {
                    return Some(true);
                }
            }
        }
    }
    None
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

/// Asserts a sub-available `Fixed(20)` box is placed at the exact column
/// `--align` dictates within the pane. With no margins the box may use the full
/// pane width as its alignment area, so the slack is `available − 20`: left sits
/// flush, center leads by `slack / 2`, and right leads by the whole slack.
///
/// `available` is the live pane column count from the harness geometry — pinning
/// the absolute lead, not just the `left < center < right` ordering (which a
/// one-third / two-thirds placement bug would also satisfy).
///
/// Each alignment uses a distinct needle because the captures accumulate in the
/// shared pane; a shared needle would always match the first (left) row.
fn assert_block_alignment_placement<H: TerminalHarness>(harness: &mut H, available: u32) {
    // box = content(20) + no padding + no border; no margins, so the alignment
    // area is the full pane and the slack is `available − 20`.
    let slack = (available - 20) as usize;
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
    assert_eq!(
        center_lead,
        slack / 2,
        "center must lead by half the slack ({} of {available}-wide pane)",
        slack / 2,
    );
    assert_eq!(
        right_lead, slack,
        "right must lead by the whole slack ({slack} of {available}-wide pane)",
    );
}

/// Asserts the drawn box follows the resolved content box across width modes:
/// a `Fixed(20)` box draws 22 cells (20 content + 2 edges), `FitContent` hugs
/// its content (`fitmark` = 7 → 9), and `Auto` fills the whole pane. With no
/// margin or padding the `Auto` content box is `available − 2` (the two border
/// edges), so the drawn box is exactly `available`. This is the real-terminal
/// analogue of the `bt block hi --width 20 --border all` reproduction the review
/// flagged — the prior code drew the box at the text width, not the resolved
/// width.
///
/// `available` is the live pane column count from the harness geometry, so the
/// `Auto` fill is pinned exactly rather than merely "wider than the fixed box".
fn assert_block_width_mode_box<H: TerminalHarness>(harness: &mut H, available: u32) {
    let fixed = capture_bt(harness, "bt block \"fixmark\" --width 20 --border all");
    assert_eq!(
        box_width_for(&fixed, "fixmark"),
        22,
        "Fixed(20) must draw a 22-cell box (20 content + 2 edges).\nplain:\n{}",
        fixed.plain,
    );

    let fit = capture_bt(harness, "bt block \"fitmark\" --width fit --border all");
    assert_eq!(
        box_width_for(&fit, "fitmark"),
        9,
        "FitContent must hug its 7-cell content (box 9).\nplain:\n{}",
        fit.plain,
    );

    let auto = capture_bt(harness, "bt block \"automark\" --width auto --border all");
    assert_eq!(
        box_width_for(&auto, "automark"),
        available as usize,
        "Auto must fill the whole {available}-wide pane (box == pane width).\nplain:\n{}",
        auto.plain,
    );
}

/// Asserts `--max-width 20` caps the content box: a short-content block draws a
/// 22-cell box (20 content + 2 edges), the same geometry as `Fixed(20)`.
fn assert_block_max_width_box<H: TerminalHarness>(harness: &mut H) {
    let frame = capture_bt(harness, "bt block \"mwmark\" --max-width 20 --border all");
    assert_eq!(
        box_width_for(&frame, "mwmark"),
        22,
        "max-width 20 must cap the content box to 20 (box 22).\nplain:\n{}",
        frame.plain,
    );
}

/// Asserts the CSS box-order clamp is terminal-observable: an oversized
/// `Fixed(200)` box clamps to the available content area, and adding a 10-cell
/// horizontal margin shrinks the drawn box by exactly `2 × 10` cells — margin
/// participates in `margin + border + padding + content ≤ available`. The
/// 2-cell padding shrinks the content but is repainted into the box, so it does
/// not change the box width; the difference is the margins alone.
fn assert_block_box_order_clamp<H: TerminalHarness>(harness: &mut H) {
    let base = capture_bt(harness, "bt block \"clampbase\" --width 200 --border all");
    let inset = capture_bt(
        harness,
        "bt block \"clampinset\" --width 200 --margin 10 --padding 2 --border all",
    );
    let base_w = box_width_for(&base, "clampbase");
    let inset_w = box_width_for(&inset, "clampinset");
    assert!(
        base_w > inset_w,
        "margin must shrink the clamped box ({base_w} vs {inset_w})",
    );
    assert_eq!(
        base_w - inset_w,
        20,
        "the 10-cell margins reduce the clamped box by exactly 2×10 cells \
         ({base_w} vs {inset_w})",
    );
}

/// Asserts a `FitContent` box is placed at the exact `--align` column. A
/// borderless, unpadded fit box hugs its content, so its drawn width is the
/// measured content width and the alignment slack is `available − width`: left
/// sits flush, center leads by `slack / 2`, and right leads by the whole slack.
///
/// The box width is measured from each capture (the content words differ in
/// length per needle), and `available` is the live pane column count — together
/// pinning the absolute lead, not just the `left < center < right` ordering.
fn assert_block_fit_alignment<H: TerminalHarness>(harness: &mut H, available: u32) {
    let left = capture_bt(harness, "bt block \"fitleftward\" --width fit --align left");
    let (left_lead, _) = row_lead_and_width(&left, "bt block", "fitleftward")
        .unwrap_or_else(|| panic!("fit left row missing.\nplain:\n{}", left.plain));
    let center = capture_bt(harness, "bt block \"fitcenterward\" --width fit --align center");
    let (center_lead, center_w) = row_lead_and_width(&center, "bt block", "fitcenterward")
        .unwrap_or_else(|| panic!("fit center row missing.\nplain:\n{}", center.plain));
    let right = capture_bt(harness, "bt block \"fitrightward\" --width fit --align right");
    let (right_lead, right_w) = row_lead_and_width(&right, "bt block", "fitrightward")
        .unwrap_or_else(|| panic!("fit right row missing.\nplain:\n{}", right.plain));

    let center_slack = available as usize - center_w;
    let right_slack = available as usize - right_w;
    assert!(
        left_lead <= 1,
        "FitContent left placement should be flush left, got {left_lead}"
    );
    assert_eq!(
        center_lead,
        center_slack / 2,
        "FitContent center must lead by half the slack ({} for a {center_w}-wide box)",
        center_slack / 2,
    );
    assert_eq!(
        right_lead, right_slack,
        "FitContent right must lead by the whole slack ({right_slack} for a {right_w}-wide box)",
    );
}

/// Asserts an `Auto` box capped by `--max-width 20` is placed at the exact
/// `--align` column. The cap fixes the content box at 20 cells; with no margin
/// or border the box is 20 wide and the alignment slack is `available − 20`:
/// left sits flush, center leads by `slack / 2`, and right leads by the whole
/// slack.
///
/// `available` is the live pane column count, pinning the absolute lead rather
/// than only the `left < center < right` ordering.
fn assert_block_capped_alignment<H: TerminalHarness>(harness: &mut H, available: u32) {
    // Auto capped to `max-width 20`: box = 20 content + no padding + no border.
    let slack = (available - 20) as usize;
    let left = capture_bt(harness, "bt block \"capleftward\" --max-width 20 --align left");
    let left_lead = row_leading_spaces(&left, "bt block", "capleftward")
        .unwrap_or_else(|| panic!("capped left row missing.\nplain:\n{}", left.plain));
    let center = capture_bt(harness, "bt block \"capcenterward\" --max-width 20 --align center");
    let center_lead = row_leading_spaces(&center, "bt block", "capcenterward")
        .unwrap_or_else(|| panic!("capped center row missing.\nplain:\n{}", center.plain));
    let right = capture_bt(harness, "bt block \"caprightward\" --max-width 20 --align right");
    let right_lead = row_leading_spaces(&right, "bt block", "caprightward")
        .unwrap_or_else(|| panic!("capped right row missing.\nplain:\n{}", right.plain));
    assert!(
        left_lead <= 1,
        "capped-Auto left placement should be flush left, got {left_lead}"
    );
    assert_eq!(
        center_lead,
        slack / 2,
        "capped-Auto center must lead by half the slack ({} of {available}-wide pane)",
        slack / 2,
    );
    assert_eq!(
        right_lead, slack,
        "capped-Auto right must lead by the whole slack ({slack} of {available}-wide pane)",
    );
}

/// Asserts acceptance criterion 1 in a styling-capable terminal: a
/// content-hugging block with `--width fit --padding 2 --margin 3 --bg blue
/// --border all` keeps its three margin cells transparent (outside the
/// background SGR run) and paints exactly two cells of padding on **each** side
/// of the content.
///
/// `--width fit` is required: an `Auto` box fills the available width, so its
/// background band correctly extends past the padding. FitContent hugs the
/// content, so the painted band is exactly `2 + len(content) + 2`.
///
/// `--border all` draws a right edge glyph after the padding so the right
/// padding cells are no longer trailing — a terminal's erase-to-end-of-line
/// would otherwise collapse them and hide them from the capture. With the
/// border in place both painted sides are byte-observable, and the transparent
/// margin still sits outside the painted box.
fn assert_block_painted_padding_and_transparent_margin<H: TerminalHarness>(harness: &mut H) {
    let frame = capture_bt(
        harness,
        "bt block \"padword\" --width fit --bg blue --padding 2 --margin 3 --border all",
    );
    // The painted band reached the terminal at all.
    assert!(
        has_background(&frame.raw),
        "expected a painted background band in the capture.\nraw:\n{}",
        frame.raw,
    );
    // Locate the painted content row directly in the raw capture by its text, so
    // the multi-row painted band (vertical padding) cannot desync a plain/raw
    // index mapping.
    let row = frame
        .raw
        .lines()
        .find(|l| !l.contains("bt block") && l.contains("padword"))
        .unwrap_or_else(|| panic!("no painted content row.\nraw:\n{}", frame.raw))
        .to_string();
    let (margin, left_pad, right_pad) = painted_box_padding(&row);
    assert_eq!(
        margin, 3,
        "the 3 margin cells must stay transparent (outside the background run): {row:?}",
    );
    assert_eq!(
        left_pad, 2,
        "padding must paint exactly two cells before the content: {row:?}",
    );
    assert_eq!(
        right_pad, 2,
        "padding must paint exactly two cells after the content: {row:?}",
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_render_tree_style_in_wezterm() {
    require_level!(Level::L2, WezTermHarness::available(), Backend::WezTerm);

    let mut guard = SHARED_WEZTERM
        .get_or_init(|| WezTermHarness::shared_or_spawn().expect("attach/spawn WezTerm"));
    let harness = guard.as_mut().expect("shared WezTerm harness present");
    harness.send_text(b"clear\n").expect("send_text failed");
    harness.settle();
    let available = harness.pane_size().expect("WezTerm pane size").cols;
    assert_block_fg(harness);
    assert_block_bg(harness);
    assert_block_bold(harness);
    assert_block_border(harness);
    assert_block_rounded_border(harness);
    assert_progress_slot_colors(harness);
    assert_table_striped(harness);
    assert_table_example_striped_without_force_color(harness);
    assert_table_styled(harness);
    assert_block_painted_padding(harness);
    assert_block_border_adjacency(harness);
    assert_block_alignment_placement(harness, available);
    assert_block_width_mode_box(harness, available);
    assert_block_max_width_box(harness);
    assert_block_box_order_clamp(harness);
    assert_block_fit_alignment(harness, available);
    assert_block_capped_alignment(harness, available);
    assert_block_painted_padding_and_transparent_margin(harness);
}

#[test]
#[serial(level2_terminal)]
fn level2_render_tree_style_in_kitty() {
    require_level!(Level::L2, KittyHarness::available(), Backend::Kitty);

    let mut guard = SHARED_KITTY
        .get_or_init(|| KittyHarness::shared_or_spawn().expect("attach/spawn kitty"));
    let harness = guard.as_mut().expect("shared Kitty harness present");
    harness.send_text(b"clear\n").expect("send_text failed");
    harness.settle();
    let available = harness.pane_cols().expect("kitty pane cols");
    assert_block_fg(harness);
    assert_block_bg(harness);
    assert_block_bold(harness);
    assert_block_border(harness);
    assert_block_rounded_border(harness);
    assert_progress_slot_colors(harness);
    assert_table_striped(harness);
    assert_table_example_striped_without_force_color(harness);
    assert_table_styled(harness);
    assert_block_painted_padding(harness);
    assert_block_border_adjacency(harness);
    assert_block_alignment_placement(harness, available);
    assert_block_width_mode_box(harness, available);
    assert_block_max_width_box(harness);
    assert_block_box_order_clamp(harness);
    assert_block_fit_alignment(harness, available);
    assert_block_capped_alignment(harness, available);
    assert_block_painted_padding_and_transparent_margin(harness);
}

#[test]
#[serial(level2_terminal)]
fn level2_render_tree_style_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let mut guard = SHARED_TMUX
        .get_or_init(|| TmuxHarness::shared_or_spawn().expect("attach/spawn tmux"));
    let harness = guard.as_mut().expect("shared tmux harness present");
    harness.send_text(b"clear\n").expect("send_text failed");
    harness.settle();
    let available = harness.pane_cols().expect("tmux pane cols");

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
    // owned by padding, the drawn box follows the resolved width across modes,
    // the box-order clamp holds, and `--align` places sub-available boxes. The
    // painted background band is verified only on styling-capable terminals
    // (WezTerm / Kitty) since tmux's plain capture carries no SGR.
    assert_block_border_adjacency(harness);
    assert_block_alignment_placement(harness, available);
    assert_block_width_mode_box(harness, available);
    assert_block_max_width_box(harness);
    assert_block_box_order_clamp(harness);
    assert_block_fit_alignment(harness, available);
    assert_block_capped_alignment(harness, available);
}
