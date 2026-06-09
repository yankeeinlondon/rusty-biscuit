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

/// Applies one SGR parameter list to the running bold flag. An empty list
/// (`ESC[m`) is a full reset. `1` sets bold; `0`/`22` clear it.
///
/// SGR has two levels of structure: `;` separates parameters, `:` separates
/// *subparameters within one self-contained parameter*. Extended colors take
/// both forms — the ITU colon form `38:2::r:g:b` is one `;`-parameter whose
/// channels never bleed into the next, while the legacy semicolon form
/// `38;2;r;g;b` spreads channels across several `;`-parameters. Either way the
/// channel values (one of which may equal `1`) must not be read as the bold
/// attribute, so colon groups are inspected only at their leading subparameter
/// and a semicolon `38`/`48` introducer consumes its following channel
/// parameters.
fn apply_sgr_bold(params: &str, bold: &mut bool) {
    if params.is_empty() {
        *bold = false;
        return;
    }
    let groups: Vec<&str> = params.split(';').collect();
    let mut k = 0;
    while k < groups.len() {
        let group = groups[k];
        if group.contains(':') {
            // Self-contained ITU subparameter group (e.g. `38:2::r:g:b`): only
            // its leading subparameter is an attribute code; the channels stay
            // inside this group and cannot toggle bold.
            match group.split(':').next() {
                Some("1") => *bold = true,
                Some("0") | Some("22") => *bold = false,
                _ => {}
            }
        } else {
            match group {
                "0" => *bold = false,
                "1" => *bold = true,
                "22" => *bold = false,
                // Legacy semicolon extended color: skip its channel parameters
                // so a channel equal to `1` is not misread as bold.
                "38" | "48" => {
                    k += match groups.get(k + 1) {
                        Some(&"2") => 4, // 2;r;g;b
                        Some(&"5") => 2, // 5;n
                        _ => 0,
                    };
                }
                _ => {}
            }
        }
        k += 1;
    }
}

/// Reconstructs the running bold state cell-by-cell across a captured raw row.
///
/// Walks `row`, applying each SGR sequence (`ESC [ … m`) to a running bold flag
/// and pairing every *visible* character with the flag's value at that point.
/// Non-SGR escape sequences — the cursor moves the bespoke path emits, OSC-8
/// link wrappers — are consumed without producing a cell. This is the basis for
/// asserting that styling covers the content cells but stops before the trailing
/// padding and border, rather than merely proving bold appears *somewhere* on
/// the row.
fn bold_run_cells(row: &str) -> Vec<(char, bool)> {
    let chars: Vec<char> = row.chars().collect();
    let mut cells = Vec::new();
    let mut bold = false;
    let mut i = 0;
    while i < chars.len() {
        if chars[i] != '\x1b' {
            cells.push((chars[i], bold));
            i += 1;
            continue;
        }
        match chars.get(i + 1) {
            // CSI: ESC [ params <final byte 0x40..=0x7E>. Only `m` is an SGR.
            Some('[') => {
                let mut j = i + 2;
                while j < chars.len() && !('\u{40}'..='\u{7e}').contains(&chars[j]) {
                    j += 1;
                }
                if j >= chars.len() {
                    break; // malformed, unterminated CSI
                }
                if chars[j] == 'm' {
                    let params: String = chars[i + 2..j].iter().collect();
                    apply_sgr_bold(&params, &mut bold);
                }
                i = j + 1;
            }
            // OSC: ESC ] … (BEL | ST). Consumed without emitting cells.
            Some(']') => {
                let mut j = i + 2;
                while j < chars.len() {
                    if chars[j] == '\u{07}' {
                        j += 1;
                        break;
                    }
                    if chars[j] == '\x1b' && chars.get(j + 1) == Some(&'\\') {
                        j += 2;
                        break;
                    }
                    j += 1;
                }
                i = j;
            }
            // Two-byte escape (e.g. ST `ESC \`) or a trailing ESC.
            _ => i += 2,
        }
    }
    cells
}

/// Asserts the bold run inside captured raw `row` covers every cell of `needle`
/// yet is off across the trailing padding and the trailing border glyph — the
/// styled run must not bleed past its content into the cell padding, the box
/// border, or (the border being the row's last visible cell) the next row.
///
/// Reconstructs per-cell bold state from the row's SGR transitions, so it
/// accepts whatever reset encoding the terminal used (`ESC[0m`, `ESC[22m`, or an
/// elided/coalesced form) rather than requiring one literal sequence.
fn assert_bold_contained(row: &str, needle: &str) {
    let cells = bold_run_cells(row);
    let visible: String = cells.iter().map(|(c, _)| *c).collect();
    let byte_start = visible.find(needle).unwrap_or_else(|| {
        panic!("content {needle:?} not found in the visible cells of row: {row:?}")
    });
    let content_start = visible[..byte_start].chars().count();
    let content_end = content_start + needle.chars().count();

    for (ch, is_bold) in &cells[content_start..content_end] {
        assert!(
            *is_bold,
            "content cell {ch:?} of {needle:?} must be bold: {row:?}",
        );
    }

    let last_border = cells
        .iter()
        .rposition(|(c, _)| *c == '│')
        .unwrap_or_else(|| panic!("row carries no trailing border glyph: {row:?}"));
    assert!(
        last_border >= content_end,
        "the trailing border must sit after the content {needle:?}: {row:?}",
    );
    for (ch, is_bold) in &cells[content_end..=last_border] {
        assert!(
            !*is_bold,
            "trailing padding/border cell {ch:?} after {needle:?} must not be bold \
             (styling bled past the content): {row:?}",
        );
    }
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
/// (checked on `frame.plain`) and keep the bold run contained to its content
/// cells — off across the trailing padding and border (checked on the raw row),
/// proving wrap geometry and per-line style containment in a real terminal.
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
        assert_bold_contained(&row, word);
    }
}

/// Asserts an explicit hard line break inside one styled run renders as two
/// bordered, bold visual lines. `<b>line one\nline two</b>` (CLI `\n` becomes a
/// newline) must keep each line inside the box with its bold run contained to
/// the content cells — off across the trailing padding and border — proving a
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
        assert_bold_contained(&row, phrase);
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
    // The bold run must be contained to its content cells — off across the
    // trailing padding and the row's trailing border — proving it cannot bleed
    // into the padding, the border glyph, or the next row. Reconstructing the
    // per-cell bold state accepts whatever bold-off encoding the terminal used
    // (the WezTerm harness may re-emit `ESC[0m` as `ESC[22m` or elide it).
    assert_bold_contained(&bold_row, "alphaword");
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

/// L1 unit coverage for the [`assert_bold_contained`] SGR-state reconstruction.
///
/// The real-terminal (L2) tests cannot run without a usable controlling TTY, so
/// these prove the *assertion itself* has no false-positive window — bold
/// bleeding into the padding or border, or a bold border, must be rejected —
/// against synthetic rows that mirror the SGR forms WezTerm/Kitty re-emit on
/// capture (semicolon and ITU colon, with the documented bold-off encodings).
#[cfg(test)]
mod bold_containment {
    use super::assert_bold_contained;

    /// Runs `assert_bold_contained` and reports whether it accepted the row,
    /// swallowing the panic so negative cases can assert rejection.
    fn accepts(row: &str, needle: &str) -> bool {
        std::panic::catch_unwind(|| assert_bold_contained(row, needle)).is_ok()
    }

    #[test]
    fn accepts_bold_reset_before_padding_and_border() {
        // `ESC[1m`content`ESC[0m`, padding, then the trailing border.
        assert!(accepts("│ \x1b[1malpha\x1b[0m    │", "alpha"));
    }

    #[test]
    fn accepts_explicit_bold_off_encoding() {
        // WezTerm may re-emit the reset as an explicit bold-off (`ESC[22m`).
        assert!(accepts("│ \x1b[1mline one\x1b[22m  │", "line one"));
    }

    #[test]
    fn accepts_itu_colon_color_plus_bold() {
        // Bold + red maroon in ITU colon form on the content, reset before the
        // border. A `0` color channel must not be read as a bold-off, and the
        // `38:2:…` channels must not be read as bold.
        assert!(accepts(
            "│ \x1b[1;38:2::128:0:0mAlice\x1b[0m   │",
            "Alice"
        ));
    }

    #[test]
    fn rejects_bold_bleeding_into_padding_and_border() {
        // No reset after the content: bold runs through the padding and border.
        assert!(!accepts("│ \x1b[1malpha    │", "alpha"));
    }

    #[test]
    fn rejects_bold_border_after_reset() {
        // Reset after the content, but a fresh bold re-opens over the trailing
        // border — the exact false positive the old reset-ordering check missed.
        assert!(!accepts("│ \x1b[1malpha\x1b[0m   \x1b[1m│", "alpha"));
    }

    #[test]
    fn rejects_non_bold_content() {
        // The content carries no bold at all — must not be accepted.
        assert!(!accepts("│ alpha\x1b[0m    │", "alpha"));
    }
}
