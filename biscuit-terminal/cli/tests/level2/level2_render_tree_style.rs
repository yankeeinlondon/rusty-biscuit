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

/// The WezTerm pane a test attaches to: `just test-l2`'s shared pane in
/// serial mode, or one the test spawns. Every command clears the screen on
/// its own line (see [`run_cleared`]), so output from an earlier test in a
/// shared pane cannot reach a capture.
static SHARED_WEZTERM: SharedHarness<WezTermHarness> = SharedHarness::new();

/// The Kitty window a test attaches to; see [`SHARED_WEZTERM`].
static SHARED_KITTY: SharedHarness<KittyHarness> = SharedHarness::new();

/// The tmux session a test attaches to; see [`SHARED_WEZTERM`].
static SHARED_TMUX: SharedHarness<TmuxHarness> = SharedHarness::new();

/// Unique words in the quoted text so the rendered output row can be
/// isolated from the shell prompt and the command echo.
const QUOTE_TEXT: &str = "kingfisher azimuth quorum";

/// A word from [`QUOTE_TEXT`] unlikely to appear in a shell prompt.
const QUOTE_NEEDLE: &str = "kingfisher";

/// The left-border glyph the migrated `BlockQuote` `Style` declares
/// (`renderable::style::BorderSides::Sides { left: true, .. }`).
const BORDER_GLYPH: char = '│';

/// Resolves the archived test binary instead of relying on the terminal
/// shell's PATH, which does not include nextest's archive layout.
fn bt_command(args: &str) -> String {
    let bin = biscuit_test_harness::bin_exe!("bt");
    let escaped = bin.to_string_lossy().replace('\'', "'\\''");
    format!("'{escaped}' {args}")
}

/// Runs the shell `line` on a cleared screen and returns the settled frame:
/// the line's output and the prompt after it.
///
/// The `clear` shares the command line, so no fixed wait for it is needed and
/// a frame that settles before the output arrives cannot show a prompt. One
/// subshell runs both: on a loaded macOS host the interactive shell's job
/// control can print `child setpgid` errors for each job it forks.
fn run_cleared<H: TerminalHarness>(harness: &mut H, line: &str) -> CapturedFrame {
    biscuit_test_harness::send_command_settled(harness, &format!("(clear; {line})"), &[])
        .unwrap_or_else(|error| panic!("failed to run {line:?}: {error}"))
}

/// Sends a `bt quote` with color forced on and returns the captured frame.
fn capture_styled_quote<H: TerminalHarness>(harness: &mut H) -> CapturedFrame {
    capture_bt(harness, &format!("bt quote \"{QUOTE_TEXT}\""))
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
    assert_styled_border(harness);
}

#[test]
#[serial(level2_terminal)]
fn level2_block_quote_style_border_in_kitty() {
    require_level!(Level::L2, KittyHarness::available(), Backend::Kitty);

    let mut guard = SHARED_KITTY
        .get_or_init(|| KittyHarness::shared_or_spawn().expect("attach/spawn kitty"));
    let harness = guard.as_mut().expect("shared Kitty harness present");
    assert_styled_border(harness);
}

#[test]
#[serial(level2_terminal)]
fn level2_block_quote_style_border_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let mut guard = SHARED_TMUX
        .get_or_init(|| TmuxHarness::shared_or_spawn().expect("attach/spawn tmux"));
    let harness = guard.as_mut().expect("shared tmux harness present");
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
    let frame = capture_bt(harness, &format!("bt quote \"{STYLED_QUOTE_INPUT}\""));

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
    assert_styled_inline_content(harness);
}

#[test]
#[serial(level2_terminal)]
fn level2_block_quote_styled_inline_content_in_kitty() {
    require_level!(Level::L2, KittyHarness::available(), Backend::Kitty);

    let mut guard = SHARED_KITTY
        .get_or_init(|| KittyHarness::shared_or_spawn().expect("attach/spawn kitty"));
    let harness = guard.as_mut().expect("shared Kitty harness present");
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

/// Runs a `bt` command with color forced on and returns the captured frame
/// (see [`run_cleared`]).
fn capture_bt<H: TerminalHarness>(harness: &mut H, cmd: &str) -> CapturedFrame {
    let args = cmd
        .strip_prefix("bt ")
        .expect("capture_bt command must start with bt");
    run_cleared(harness, &format!("FORCE_COLOR='1' {}", bt_command(args)))
}

/// Returns the raw (escape-bearing) capture line whose plain text contains
/// `needle`, skipping the command-echo line that carries the literal
/// invocation `echo_marker`.
fn output_row(frame: &CapturedFrame, echo_marker: &str, needle: &str) -> Option<String> {
    let raw_lines: Vec<&str> = frame.raw.lines().collect();
    let plain_lines: Vec<&str> = frame.plain.lines().collect();
    for i in (0..plain_lines.len()).rev() {
        let plain = plain_lines[i];
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
    // never the square set. Scope the negative check to the rounded block's
    // own border rows — the line carrying `╭`/`╮` (top) and the line carrying
    // `╰`/`╯` (bottom) — so it judges this block's corners and nothing else
    // the frame shows.
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
    let cmd = format!(
        "env -u NO_COLOR -u FORCE_COLOR -u CLICOLOR_FORCE {}",
        bt_command("table --example")
    );
    let frame = run_cleared(harness, &cmd);

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

/// The last plain capture row carrying `needle` (skipping the command echo
/// `echo_marker`), paired with its leading-space count.
fn row_leading_spaces(frame: &CapturedFrame, echo_marker: &str, needle: &str) -> Option<usize> {
    frame
        .plain
        .lines()
        .rev()
        .find(|plain| !plain.contains(echo_marker) && plain.contains(needle))
        .map(|line| line.len() - line.trim_start().len())
}

/// The leading-space count and trimmed visible width of the last plain
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
        .rev()
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
        .rev()
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
/// no padding renders the content adjacent to the drawn edge (`│snug`).
fn assert_block_border_snug<H: TerminalHarness>(harness: &mut H) {
    let snug = capture_bt(harness, "bt block \"snug\" --border all");
    assert!(
        snug.plain.contains("│snug"),
        "a no-padding border must sit adjacent to its content.\nplain:\n{}",
        snug.plain,
    );
}

/// Asserts `--padding 1` restores the one-cell gap inside a border
/// (`│ roomy`).
fn assert_block_border_roomy<H: TerminalHarness>(harness: &mut H) {
    let roomy = capture_bt(harness, "bt block \"roomy\" --border all --padding 1");
    assert!(
        roomy.plain.contains("│ roomy"),
        "--padding 1 must restore the one-cell border gap.\nplain:\n{}",
        roomy.plain,
    );
}

/// Asserts a box `box_width` cells wide, aligned with `--align <align>`, leads
/// by the column the alignment dictates: left sits flush, center leads by
/// `slack / 2`, and right leads by the whole slack, where the slack is the
/// pane's live column count less `box_width`.
///
/// Pinning the absolute lead, not just the `left < center < right` ordering,
/// catches a one-third / two-thirds placement bug the ordering would accept.
fn assert_aligned_lead<H: PaneWidth>(harness: &H, what: &str, align: &str, lead: usize, box_width: usize) {
    if align == "left" {
        assert!(lead <= 1, "{what} left placement should be flush left, got {lead} lead");
        return;
    }
    let available = harness.pane_width();
    let slack = available as usize - box_width;
    let (expected, share) = match align {
        "center" => (slack / 2, "half the slack"),
        "right" => (slack, "the whole slack"),
        other => panic!("no alignment named {other:?}"),
    };
    assert_eq!(
        lead, expected,
        "{what} {align} must lead by {share} ({expected} for a {box_width}-wide box in a \
         {available}-wide pane)",
    );
}

/// Asserts a sub-available `Fixed(20)` box is placed at the exact column
/// `--align <align>` dictates within the pane. With no margins the box may use
/// the full pane width as its alignment area, so the slack is `available − 20`
/// (see [`assert_aligned_lead`]).
///
/// Each alignment uses a distinct needle, so a row can only match its own
/// command's output.
fn assert_block_alignment_placement<H: TerminalHarness + PaneWidth>(harness: &mut H, align: &str) {
    let needle = format!("{align}mark");
    let frame = capture_bt(harness, &format!("bt block \"{needle}\" --width 20 --align {align}"));
    let lead = row_leading_spaces(&frame, "bt block", &needle)
        .unwrap_or_else(|| panic!("{align}-aligned row missing.\nplain:\n{}", frame.plain));
    // box = content(20) + no padding + no border.
    assert_aligned_lead(harness, "Fixed(20)", align, lead, 20);
}

/// Asserts the drawn box of the bordered block `needle` rendered with
/// `width_flag` is `expected` cells wide. Together the width-mode cases pin
/// that the drawn box follows the resolved content box: a `Fixed(20)` box
/// draws 22 cells (20 content + 2 edges), `FitContent` hugs its content
/// (`fitmark` = 7 → 9), and `Auto` fills the whole pane. This is the
/// real-terminal analogue of the `bt block hi --width 20 --border all`
/// reproduction the review flagged — the prior code drew the box at the text
/// width, not the resolved width.
fn assert_block_width_box<H: TerminalHarness>(
    harness: &mut H,
    needle: &str,
    width_flag: &str,
    expected: usize,
    why: &str,
) {
    let frame = capture_bt(harness, &format!("bt block \"{needle}\" {width_flag} --border all"));
    assert_eq!(box_width_for(&frame, needle), expected, "{why}.\nplain:\n{}", frame.plain);
}

/// Asserts an `Auto` box fills the whole pane. With no margin or padding the
/// `Auto` content box is `available − 2` (the two border edges), so the drawn
/// box is exactly the pane's live column count — pinned exactly rather than
/// merely "wider than the fixed box".
fn assert_block_auto_width_box<H: TerminalHarness + PaneWidth>(harness: &mut H) {
    let frame = capture_bt(harness, "bt block \"automark\" --width auto --border all");
    let available = harness.pane_width();
    assert_eq!(
        box_width_for(&frame, "automark"),
        available as usize,
        "Auto must fill the whole {available}-wide pane (box == pane width).\nplain:\n{}",
        frame.plain,
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

/// Asserts a `FitContent` box is placed at the exact `--align <align>` column.
/// A borderless, unpadded fit box hugs its content, so its drawn width is the
/// measured content width (the words differ in length per needle) and the
/// slack is `available − width` (see [`assert_aligned_lead`]).
fn assert_block_fit_alignment<H: TerminalHarness + PaneWidth>(harness: &mut H, align: &str) {
    let needle = format!("fit{align}ward");
    let frame = capture_bt(harness, &format!("bt block \"{needle}\" --width fit --align {align}"));
    let (lead, width) = row_lead_and_width(&frame, "bt block", &needle)
        .unwrap_or_else(|| panic!("fit {align} row missing.\nplain:\n{}", frame.plain));
    assert_aligned_lead(harness, "FitContent", align, lead, width);
}

/// Asserts an `Auto` box capped by `--max-width 20` is placed at the exact
/// `--align <align>` column. The cap fixes the content box at 20 cells; with
/// no margin or border the box is 20 wide and the slack is `available − 20`
/// (see [`assert_aligned_lead`]).
fn assert_block_capped_alignment<H: TerminalHarness + PaneWidth>(harness: &mut H, align: &str) {
    let needle = format!("cap{align}ward");
    let frame = capture_bt(harness, &format!("bt block \"{needle}\" --max-width 20 --align {align}"));
    let lead = row_leading_spaces(&frame, "bt block", &needle)
        .unwrap_or_else(|| panic!("capped {align} row missing.\nplain:\n{}", frame.plain));
    assert_aligned_lead(harness, "capped-Auto", align, lead, 20);
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
        .rev()
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

// ---------------------------------------------------------------------------
// tmux — relays glyphs and text but carries no styling protocol of its own
// ---------------------------------------------------------------------------
//
// tmux faithfully relays text glyphs. These assert the border glyphs and
// visible text survive and that no raw escape garbage leaks into the
// displayed cells. The painted background band is verified only on
// styling-capable terminals (WezTerm / Kitty) since tmux's plain capture
// carries no SGR.

/// A bordered block relays its box-drawing glyphs and text through tmux.
fn assert_tmux_block_border<H: TerminalHarness>(harness: &mut H) {
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
}

/// A rounded border relays its light-arc corner glyphs through tmux.
fn assert_tmux_block_rounded_border<H: TerminalHarness>(harness: &mut H) {
    let frame = capture_bt(
        harness,
        "bt block \"rounded notice\" --border all --border-radius 1",
    );
    assert!(
        frame.plain.contains('╭') && frame.plain.contains('╯'),
        "expected light-arc corner glyphs in the tmux capture.\nplain:\n{}",
        frame.plain,
    );
}

/// The progress bar's text relays through tmux.
fn assert_tmux_progress<H: TerminalHarness>(harness: &mut H) {
    let frame = capture_bt(harness, "bt progress 50 --label Loading");
    assert!(
        frame.plain.contains("50%") && frame.plain.contains("Loading"),
        "expected the progress bar text in tmux. plain:\n{}",
        frame.plain,
    );
}

/// A striped table's cell text relays through tmux.
fn assert_tmux_table_striped<H: TerminalHarness>(harness: &mut H) {
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
}

/// A table with typed header / body slot styling relays its text through tmux.
fn assert_tmux_table_styled<H: TerminalHarness>(harness: &mut H) {
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
}

// ---------------------------------------------------------------------------
// One test per scenario per terminal
// ---------------------------------------------------------------------------
//
// Each scenario is its own `#[test]` so nextest spreads them over the
// runner's cores; one test running every scenario took about 5 s in WezTerm.

/// The pane's live column count, which the alignment and `Auto` width
/// scenarios pin their expected geometry to.
trait PaneWidth {
    fn pane_width(&self) -> u32;
}

impl PaneWidth for WezTermHarness {
    fn pane_width(&self) -> u32 {
        self.pane_size().expect("WezTerm pane size").cols
    }
}

impl PaneWidth for KittyHarness {
    fn pane_width(&self) -> u32 {
        self.pane_cols().expect("kitty pane cols")
    }
}

impl PaneWidth for TmuxHarness {
    fn pane_width(&self) -> u32 {
        self.pane_cols().expect("tmux pane cols")
    }
}

/// A named scenario run against one terminal.
type Case<H> = (&'static str, fn(&mut H));

/// The scenarios a styling-capable terminal (WezTerm, Kitty) checks. The
/// names are the test names; `every_case_has_a_test_per_terminal` holds each
/// terminal's test list to this table.
fn styled_cases<H: TerminalHarness + PaneWidth>() -> [Case<H>; 27] {
    [
        ("block_fg", |h| assert_block_fg(h)),
        ("block_bg", |h| assert_block_bg(h)),
        ("block_bold", |h| assert_block_bold(h)),
        ("block_border", |h| assert_block_border(h)),
        ("block_rounded_border", |h| assert_block_rounded_border(h)),
        ("progress_slot_colors", |h| assert_progress_slot_colors(h)),
        ("table_striped", |h| assert_table_striped(h)),
        ("table_example_striped_without_force_color", |h| {
            assert_table_example_striped_without_force_color(h)
        }),
        ("table_styled", |h| assert_table_styled(h)),
        ("block_painted_padding", |h| assert_block_painted_padding(h)),
        ("block_border_snug", |h| assert_block_border_snug(h)),
        ("block_border_roomy", |h| assert_block_border_roomy(h)),
        ("block_align_left", |h| assert_block_alignment_placement(h, "left")),
        ("block_align_center", |h| assert_block_alignment_placement(h, "center")),
        ("block_align_right", |h| assert_block_alignment_placement(h, "right")),
        ("block_fixed_width_box", |h| {
            assert_block_width_box(h, "fixmark", "--width 20", 22, "Fixed(20) must draw a 22-cell box (20 content + 2 edges)")
        }),
        ("block_fit_width_box", |h| {
            assert_block_width_box(h, "fitmark", "--width fit", 9, "FitContent must hug its 7-cell content (box 9)")
        }),
        ("block_auto_width_box", |h| assert_block_auto_width_box(h)),
        ("block_max_width_box", |h| assert_block_max_width_box(h)),
        ("block_box_order_clamp", |h| assert_block_box_order_clamp(h)),
        ("block_fit_align_left", |h| assert_block_fit_alignment(h, "left")),
        ("block_fit_align_center", |h| assert_block_fit_alignment(h, "center")),
        ("block_fit_align_right", |h| assert_block_fit_alignment(h, "right")),
        ("block_capped_align_left", |h| assert_block_capped_alignment(h, "left")),
        ("block_capped_align_center", |h| assert_block_capped_alignment(h, "center")),
        ("block_capped_align_right", |h| assert_block_capped_alignment(h, "right")),
        ("block_painted_padding_and_transparent_margin", |h| {
            assert_block_painted_padding_and_transparent_margin(h)
        }),
    ]
}

/// The scenarios tmux checks: glyph and text relay, and the box-model
/// geometry, which needs no styling protocol.
fn tmux_cases<H: TerminalHarness + PaneWidth>() -> [Case<H>; 21] {
    [
        ("block_border", |h| assert_tmux_block_border(h)),
        ("block_rounded_border", |h| assert_tmux_block_rounded_border(h)),
        ("progress", |h| assert_tmux_progress(h)),
        ("table_striped", |h| assert_tmux_table_striped(h)),
        ("table_styled", |h| assert_tmux_table_styled(h)),
        ("block_border_snug", |h| assert_block_border_snug(h)),
        ("block_border_roomy", |h| assert_block_border_roomy(h)),
        ("block_align_left", |h| assert_block_alignment_placement(h, "left")),
        ("block_align_center", |h| assert_block_alignment_placement(h, "center")),
        ("block_align_right", |h| assert_block_alignment_placement(h, "right")),
        ("block_fixed_width_box", |h| {
            assert_block_width_box(h, "fixmark", "--width 20", 22, "Fixed(20) must draw a 22-cell box (20 content + 2 edges)")
        }),
        ("block_fit_width_box", |h| {
            assert_block_width_box(h, "fitmark", "--width fit", 9, "FitContent must hug its 7-cell content (box 9)")
        }),
        ("block_auto_width_box", |h| assert_block_auto_width_box(h)),
        ("block_max_width_box", |h| assert_block_max_width_box(h)),
        ("block_box_order_clamp", |h| assert_block_box_order_clamp(h)),
        ("block_fit_align_left", |h| assert_block_fit_alignment(h, "left")),
        ("block_fit_align_center", |h| assert_block_fit_alignment(h, "center")),
        ("block_fit_align_right", |h| assert_block_fit_alignment(h, "right")),
        ("block_capped_align_left", |h| assert_block_capped_alignment(h, "left")),
        ("block_capped_align_center", |h| assert_block_capped_alignment(h, "center")),
        ("block_capped_align_right", |h| assert_block_capped_alignment(h, "right")),
    ]
}

/// Runs the case called `name` from `cases`.
fn run_case<H>(harness: &mut H, cases: &[Case<H>], name: &str) {
    let (_, case) = cases
        .iter()
        .find(|(case, _)| *case == name)
        .unwrap_or_else(|| panic!("no render-tree style case named {name:?}"));
    case(harness);
}

fn styled_case_in_wezterm(name: &str) {
    let mut guard = SHARED_WEZTERM
        .get_or_init(|| WezTermHarness::shared_or_spawn().expect("attach/spawn WezTerm"));
    let harness = guard.as_mut().expect("shared WezTerm harness present");
    run_case(harness, &styled_cases(), name);
}

fn styled_case_in_kitty(name: &str) {
    let mut guard =
        SHARED_KITTY.get_or_init(|| KittyHarness::shared_or_spawn().expect("attach/spawn kitty"));
    let harness = guard.as_mut().expect("shared Kitty harness present");
    run_case(harness, &styled_cases(), name);
}

fn tmux_case(name: &str) {
    let mut guard =
        SHARED_TMUX.get_or_init(|| TmuxHarness::shared_or_spawn().expect("attach/spawn tmux"));
    let harness = guard.as_mut().expect("shared tmux harness present");
    run_case(harness, &tmux_cases(), name);
}

/// Generates module `$module` with one `#[test]` per listed case, gated on
/// `$harness` and run by `$run`, and a `CASES` list of the names for the
/// guard test.
macro_rules! case_tests {
    ($module:ident, $harness:ty, $backend:expr, $run:ident, [$($case:ident),* $(,)?]) => {
        mod $module {
            use super::*;

            pub(super) const CASES: &[&str] = &[$(stringify!($case)),*];

            $(
                #[test]
                #[serial(level2_terminal)]
                fn $case() {
                    require_level!(Level::L2, <$harness>::available(), $backend);
                    $run(stringify!($case));
                }
            )*
        }
    };
}

case_tests!(
    level2_render_tree_style_in_wezterm,
    WezTermHarness,
    Backend::WezTerm,
    styled_case_in_wezterm,
    [
        block_fg,
        block_bg,
        block_bold,
        block_border,
        block_rounded_border,
        progress_slot_colors,
        table_striped,
        table_example_striped_without_force_color,
        table_styled,
        block_painted_padding,
        block_border_snug,
        block_border_roomy,
        block_align_left,
        block_align_center,
        block_align_right,
        block_fixed_width_box,
        block_fit_width_box,
        block_auto_width_box,
        block_max_width_box,
        block_box_order_clamp,
        block_fit_align_left,
        block_fit_align_center,
        block_fit_align_right,
        block_capped_align_left,
        block_capped_align_center,
        block_capped_align_right,
        block_painted_padding_and_transparent_margin,
    ]
);

case_tests!(
    level2_render_tree_style_in_kitty,
    KittyHarness,
    Backend::Kitty,
    styled_case_in_kitty,
    [
        block_fg,
        block_bg,
        block_bold,
        block_border,
        block_rounded_border,
        progress_slot_colors,
        table_striped,
        table_example_striped_without_force_color,
        table_styled,
        block_painted_padding,
        block_border_snug,
        block_border_roomy,
        block_align_left,
        block_align_center,
        block_align_right,
        block_fixed_width_box,
        block_fit_width_box,
        block_auto_width_box,
        block_max_width_box,
        block_box_order_clamp,
        block_fit_align_left,
        block_fit_align_center,
        block_fit_align_right,
        block_capped_align_left,
        block_capped_align_center,
        block_capped_align_right,
        block_painted_padding_and_transparent_margin,
    ]
);

case_tests!(
    level2_render_tree_style_in_tmux,
    TmuxHarness,
    Backend::Tmux,
    tmux_case,
    [
        block_border,
        block_rounded_border,
        progress,
        table_striped,
        table_styled,
        block_border_snug,
        block_border_roomy,
        block_align_left,
        block_align_center,
        block_align_right,
        block_fixed_width_box,
        block_fit_width_box,
        block_auto_width_box,
        block_max_width_box,
        block_box_order_clamp,
        block_fit_align_left,
        block_fit_align_center,
        block_fit_align_right,
        block_capped_align_left,
        block_capped_align_center,
        block_capped_align_right,
    ]
);

/// A case added to a table without a test in every terminal's list would
/// never run.
#[test]
fn every_case_has_a_test_per_terminal() {
    let names = |cases: &[Case<TmuxHarness>]| cases.iter().map(|(name, _)| *name).collect::<Vec<_>>();
    let styled = names(&styled_cases());
    assert_eq!(styled, level2_render_tree_style_in_wezterm::CASES);
    assert_eq!(styled, level2_render_tree_style_in_kitty::CASES);
    assert_eq!(names(&tmux_cases()), level2_render_tree_style_in_tmux::CASES);
}
