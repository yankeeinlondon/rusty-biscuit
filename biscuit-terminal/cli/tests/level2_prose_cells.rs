//! Level-2 tests for `StyledProse` table cells in a real terminal emulator.
//!
//! Styled Prose cells and the cursor-alignment bespoke path are driven through
//! the `bt table` CLI (`--prose-row`, `--cursor-align`) inside a real WezTerm /
//! Kitty / tmux pane, verifying that capability-resolved styling (bold, color)
//! survives to the cells the emulator displays, that the box geometry stays
//! intact, and that a styled run never bleeds into a border. The byte/SGR
//! behavior of each layer is pinned by Level-1 tests in `prose_cells_parity`;
//! these confirm the user-visible result on the screen.
//!
//! ## Skip-clean contract
//!
//! Every test checks `Harness::available()` before spawning. When the required
//! terminal is absent the test skips via `require_level!`. No `#[ignore]`.

use biscuit_test_harness::kitty::KittyHarness;
use biscuit_test_harness::shared::SharedHarness;
use biscuit_test_harness::tmux::TmuxHarness;
use biscuit_test_harness::wezterm::WezTermHarness;
use biscuit_test_harness::{CapturedFrame, TerminalHarness};
use serial_test::serial;
use test_toolkit::{Level, require_level};

static SHARED_WEZTERM: SharedHarness<WezTermHarness> = SharedHarness::new();
static SHARED_KITTY: SharedHarness<KittyHarness> = SharedHarness::new();
static SHARED_TMUX: SharedHarness<TmuxHarness> = SharedHarness::new();

/// Runs a `bt` command with color forced on and returns the captured frame.
fn capture_bt<H: TerminalHarness>(harness: &mut H, cmd: &str) -> CapturedFrame {
    harness
        .send_command_with_env(cmd, &[("FORCE_COLOR", "1")])
        .expect("send_command_with_env failed");
    biscuit_test_harness::capture_settled(harness).expect("capture failed")
}

/// The raw (escape-bearing) capture line for a rendered table cell: the row
/// whose plain text carries both a box-drawing border glyph and `needle`.
///
/// The command echo never contains a border glyph, so requiring `│` reliably
/// distinguishes the rendered row from the (possibly line-wrapped) echoed
/// command, which also carries the cell's visible text.
fn cell_row(frame: &CapturedFrame, needle: &str) -> Option<String> {
    let raw_lines: Vec<&str> = frame.raw.lines().collect();
    for (i, plain) in frame.plain.lines().enumerate() {
        if plain.contains('│') && plain.contains(needle) {
            return raw_lines.get(i).map(|raw| (*raw).to_string());
        }
    }
    None
}

/// The plain (escape-stripped) capture line for a rendered cell: the row whose
/// plain text carries both a box-drawing border glyph and `needle`. Used for
/// border-geometry assertions that must see the visible grid, not escapes.
fn plain_cell_row(frame: &CapturedFrame, needle: &str) -> Option<String> {
    frame
        .plain
        .lines()
        .find(|plain| plain.contains('│') && plain.contains(needle))
        .map(str::to_string)
}

/// Asserts the cell line carrying `needle` sits inside the box: its visible
/// content opens and closes with a vertical border glyph (trailing pane padding
/// spaces are ignored).
fn assert_bordered_line(frame: &CapturedFrame, needle: &str) {
    let plain = plain_cell_row(frame, needle)
        .unwrap_or_else(|| panic!("missing cell line {needle:?}.\nplain:\n{}", frame.plain));
    assert!(
        plain.trim_start().starts_with('│') && plain.trim_end().ends_with('│'),
        "cell line {needle:?} must stay inside the box border: {plain:?}"
    );
}

/// Whether `row` carries an SGR sequence (`ESC [ … m`) whose parameter list
/// includes the bold attribute `1`. SGR forms vary by terminal (semicolon vs
/// ITU colon, standalone vs coalesced), so the parameter list is parsed rather
/// than matched byte-for-byte.
fn sgr_carries_bold(row: &str) -> bool {
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
            && params.split([';', ':']).any(|p| p == "1")
        {
            return true;
        }
        rest = &after[end + 1..];
    }
    false
}

/// Whether `row` carries a red foreground SGR. A named basic color lowers to a
/// 16-color SGR; some terminals re-emit it as truecolor on capture.
fn carries_red_fg(row: &str) -> bool {
    row.contains("\x1b[31m")
        || row.contains("\x1b[91m")
        || row.contains("\x1b[38;2;")
        || row.contains("\x1b[38:2:")
        || row.contains("\x1b[38;5;")
}

/// Asserts a styled Prose cell's capability-resolved SGR survives to the real
/// terminal and that the box geometry stays intact: the bold cell carries a
/// bold attribute, the colored cell a red foreground, and the visible borders
/// and text remain in the displayed plain cells.
fn assert_prose_cell_styled<H: TerminalHarness>(harness: &mut H) {
    let frame = capture_bt(
        harness,
        "bt table --columns \"Status,Owner\" --prose-row \"<b>active</b>,<red>Alice</red>\"",
    );
    let row = cell_row(&frame, "active").unwrap_or_else(|| {
        panic!(
            "could not locate the styled Prose-cell data row.\nplain:\n{}\nraw:\n{}",
            frame.plain, frame.raw,
        )
    });
    assert!(
        sgr_carries_bold(&row),
        "expected the <b> Prose cell to carry a bold SGR attribute: {row:?}"
    );
    assert!(
        carries_red_fg(&row),
        "expected the <red> Prose cell to carry a red foreground SGR: {row:?}"
    );
    // Geometry: the box borders and both cell texts survive to the plain cells.
    assert!(
        frame.plain.contains('│') && frame.plain.contains("active") && frame.plain.contains("Alice"),
        "expected bordered, visible Prose-cell content.\nplain:\n{}",
        frame.plain,
    );
}

/// Asserts a word-wrapped styled Prose cell keeps every wrapped visual line
/// bordered and bold. A single bold run `<b>alpha bravo charlie</b>` wraps into
/// three visual lines under a narrow column; each must stay inside the box
/// (checked on `frame.plain`) and retain the bold attribute (checked on the raw
/// row), proving wrap geometry and per-line style containment in a real
/// terminal.
fn assert_prose_cell_wraps_styled<H: TerminalHarness>(harness: &mut H) {
    let frame = capture_bt(
        harness,
        "bt table --columns \"Notes\" --prose-row \"<b>alpha bravo charlie</b>\" --col-width 9",
    );
    for word in ["alpha", "bravo", "charlie"] {
        assert_bordered_line(&frame, word);
        let row = cell_row(&frame, word).unwrap_or_else(|| {
            panic!(
                "missing raw wrapped line {word:?}.\nplain:\n{}",
                frame.plain
            )
        });
        assert!(
            sgr_carries_bold(&row),
            "wrapped line {word:?} must keep its bold attribute: {row:?}"
        );
    }
}

/// Asserts an explicit hard line break inside one styled run renders as two
/// bordered, bold visual lines. `<b>line one\nline two</b>` (CLI `\n` becomes a
/// newline) must keep each line inside the box and carrying bold, proving a
/// styled run that crosses a newline does not bleed into the border or the next
/// visual line.
fn assert_prose_cell_multiline_styled<H: TerminalHarness>(harness: &mut H) {
    let frame = capture_bt(
        harness,
        "bt table --columns \"Msg\" --prose-row \"<b>line one\\nline two</b>\"",
    );
    for phrase in ["line one", "line two"] {
        assert_bordered_line(&frame, phrase);
        let row = cell_row(&frame, phrase).unwrap_or_else(|| {
            panic!("missing raw multiline {phrase:?}.\nplain:\n{}", frame.plain)
        });
        assert!(
            sgr_carries_bold(&row),
            "multiline {phrase:?} must keep its bold attribute: {row:?}"
        );
    }
}

/// Asserts two styled Prose rows render independently: the bold row carries a
/// bold attribute, the red row a red foreground, and the bold run is turned off
/// before its row's border so styling does not bleed across rows.
fn assert_prose_rows_independently_styled<H: TerminalHarness>(harness: &mut H) {
    let frame = capture_bt(
        harness,
        "bt table --columns \"Cell\" --prose-row \"<b>alphaword</b>\" --prose-row \"<red>bravoword</red>\"",
    );
    let bold_row = cell_row(&frame, "alphaword")
        .unwrap_or_else(|| panic!("missing bold Prose row.\nplain:\n{}", frame.plain));
    let red_row = cell_row(&frame, "bravoword")
        .unwrap_or_else(|| panic!("missing red Prose row.\nplain:\n{}", frame.plain));
    assert!(
        sgr_carries_bold(&bold_row),
        "the first Prose row must carry bold: {bold_row:?}"
    );
    assert!(
        carries_red_fg(&red_row),
        "the second Prose row must carry red: {red_row:?}"
    );
    // The bold run must turn off after its text and before the row's trailing
    // border, and must not re-open afterward — proving it cannot bleed into the
    // padding, the border glyph, or the next row. The WezTerm harness warns a
    // literal `ESC[0m` may be re-emitted as an explicit bold-off (`ESC[22m`) or
    // elided across captures, so this asserts ordering and accepts either
    // documented bold-off form rather than depending on one capture byte.
    let text_end = bold_row
        .find("alphaword")
        .expect("the bold cell text must appear in its raw row")
        + "alphaword".len();
    let tail = &bold_row[text_end..];
    let reset_at = tail
        .find("\x1b[0m")
        .or_else(|| tail.find("\x1b[22m"))
        .unwrap_or_else(|| {
            panic!("the bold run must turn off (ESC[0m or ESC[22m) after its text: {bold_row:?}")
        });
    assert!(
        !sgr_carries_bold(&tail[reset_at..]),
        "no bold may re-open after the row's reset, before the border: {bold_row:?}"
    );
}

/// Asserts the cursor-alignment bespoke path renders a styled Prose cell with
/// styling and geometry intact in a real terminal.
///
/// The cursor-positioning control sequences the bespoke path emits (`ESC [ N G`)
/// are *consumed* by the terminal — they move the cursor and are not retained in
/// the captured cell grid — so they cannot be asserted from a capture. What is
/// observable, and what the spec requires (the cursor path "preserve[s] visible
/// content and styling"), is the result: the bold styling survives onto the
/// cells, the text is visible, and the box border is intact.
fn assert_prose_cursor_align<H: TerminalHarness>(harness: &mut H) {
    let frame = capture_bt(
        harness,
        "bt table --columns \"Msg\" --prose-row \"<b>cursorword</b>\" --cursor-align",
    );
    let row = cell_row(&frame, "cursorword")
        .unwrap_or_else(|| panic!("missing cursor-aligned Prose row.\nplain:\n{}", frame.plain));
    assert!(
        sgr_carries_bold(&row),
        "the cursor-aligned Prose cell must carry bold: {row:?}"
    );
    assert!(
        row.contains('│'),
        "the cursor-aligned Prose cell must keep its box border: {row:?}"
    );
    assert!(
        frame.plain.contains("cursorword"),
        "the cursor-aligned cell text must remain visible.\nplain:\n{}",
        frame.plain,
    );
}

#[test]
#[serial(level2_terminal)]
fn level2_prose_cells_in_wezterm() {
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
    assert_prose_cell_styled(harness);
    assert_prose_cell_wraps_styled(harness);
    assert_prose_cell_multiline_styled(harness);
    assert_prose_rows_independently_styled(harness);
    assert_prose_cursor_align(harness);
}

#[test]
#[serial(level2_terminal)]
fn level2_prose_cells_in_kitty() {
    require_level!(
        Level::L2,
        KittyHarness::available(),
        "Kitty remote control (set KITTY_LISTEN_ON)",
    );
    let mut guard =
        SHARED_KITTY.get_or_init(|| KittyHarness::shared_or_spawn().expect("attach/spawn kitty"));
    let harness = guard.as_mut().expect("shared Kitty harness present");
    harness.send_text(b"clear\n").expect("send_text failed");
    harness.settle();
    assert_prose_cell_styled(harness);
    assert_prose_cell_wraps_styled(harness);
    assert_prose_cell_multiline_styled(harness);
    assert_prose_rows_independently_styled(harness);
    assert_prose_cursor_align(harness);
}

#[test]
#[serial(level2_terminal)]
fn level2_prose_cells_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), "tmux");
    let mut guard =
        SHARED_TMUX.get_or_init(|| TmuxHarness::shared_or_spawn().expect("attach/spawn tmux"));
    let harness = guard.as_mut().expect("shared tmux harness present");
    harness.send_text(b"clear\n").expect("send_text failed");
    harness.settle();

    // tmux carries no styling protocol of its own, but it faithfully relays text
    // glyphs and box-drawing borders. Assert the Prose-cell text and borders
    // survive and that no raw escape garbage leaks into the displayed cells.
    let frame = capture_bt(
        harness,
        "bt table --columns \"Status,Owner\" --prose-row \"<b>active</b>,<red>Alice</red>\"",
    );
    assert!(
        frame.plain.contains('│') && frame.plain.contains("active") && frame.plain.contains("Alice"),
        "expected bordered, visible Prose-cell content in tmux.\nplain:\n{}",
        frame.plain,
    );
    assert!(
        !frame.plain.contains('\x1b'),
        "expected no raw escape bytes in the tmux plain capture.\nplain:\n{}",
        frame.plain,
    );

    // The cursor-alignment path also relays its text faithfully through tmux.
    let frame = capture_bt(
        harness,
        "bt table --columns \"Msg\" --prose-row \"<b>cursorword</b>\" --cursor-align",
    );
    assert!(
        frame.plain.contains("cursorword"),
        "expected the cursor-aligned Prose cell text in tmux.\nplain:\n{}",
        frame.plain,
    );

    // tmux carries no styling, but it must still wrap a long styled cell onto
    // multiple bordered visual lines, and honor a hard line break inside a
    // styled run. Assert the wrap/newline geometry survives with intact borders.
    let frame = capture_bt(
        harness,
        "bt table --columns \"Notes\" --prose-row \"<b>alpha bravo charlie</b>\" --col-width 9",
    );
    for word in ["alpha", "bravo", "charlie"] {
        assert_bordered_line(&frame, word);
    }
    let frame = capture_bt(
        harness,
        "bt table --columns \"Msg\" --prose-row \"<b>line one\\nline two</b>\"",
    );
    for phrase in ["line one", "line two"] {
        assert_bordered_line(&frame, phrase);
    }
    assert!(
        !frame.plain.contains('\x1b'),
        "expected no raw escape bytes in the tmux multiline plain capture.\nplain:\n{}",
        frame.plain,
    );
}
