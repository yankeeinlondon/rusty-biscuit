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
use test_toolkit::{Backend, Level, require_level};

static SHARED_WEZTERM: SharedHarness<WezTermHarness> = SharedHarness::new();
static SHARED_KITTY: SharedHarness<KittyHarness> = SharedHarness::new();
static SHARED_TMUX: SharedHarness<TmuxHarness> = SharedHarness::new();

/// Runs a `bt` command with color forced on and returns the captured frame.
fn capture_bt<H: TerminalHarness>(harness: &mut H, cmd: &str) -> CapturedFrame {
    let bin = biscuit_test_harness::bin_exe!("bt");
    let escaped = bin.to_string_lossy().replace('\'', "'\\''");
    let args = cmd
        .strip_prefix("bt ")
        .expect("capture_bt command must start with `bt `");
    let cmd = format!("'{escaped}' {args}");
    harness
        .send_command_with_env(&cmd, &[("FORCE_COLOR", "1")])
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

/// Applies one SGR parameter list to the running flag for the intensity
/// attribute `on_code` (`"1"` bold, `"2"` dim). An empty list (`ESC[m`) is a
/// full reset; `on_code` sets the flag; `0` and `22` (normal intensity) clear
/// it.
///
/// SGR has two levels of structure: `;` separates parameters, `:` separates
/// *subparameters within one self-contained parameter*. Extended colors take
/// both forms — the ITU colon form `38:2::r:g:b` is one `;`-parameter whose
/// channels never bleed into the next, while the legacy semicolon form
/// `38;2;r;g;b` spreads channels across several `;`-parameters. Either way a
/// channel value (one of which may equal the `1`/`2` attribute code) must not
/// be read as the attribute, so colon groups are inspected only at their
/// leading subparameter and a semicolon `38`/`48` introducer consumes its
/// following channel parameters.
fn apply_sgr_attr(params: &str, on_code: &str, active: &mut bool) {
    if params.is_empty() {
        *active = false;
        return;
    }
    let groups: Vec<&str> = params.split(';').collect();
    let mut k = 0;
    while k < groups.len() {
        let group = groups[k];
        if group.contains(':') {
            // Self-contained ITU subparameter group (e.g. `38:2::r:g:b`): only
            // its leading subparameter is an attribute code; the channels stay
            // inside this group and cannot toggle the attribute.
            match group.split(':').next() {
                Some("0") | Some("22") => *active = false,
                Some(code) if code == on_code => *active = true,
                _ => {}
            }
        } else {
            match group {
                "0" | "22" => *active = false,
                // Legacy semicolon extended color: skip its channel parameters
                // so a channel equal to the attribute code is not misread as it.
                "38" | "48" => {
                    k += match groups.get(k + 1) {
                        Some(&"2") => 4, // 2;r;g;b
                        Some(&"5") => 2, // 5;n
                        _ => 0,
                    };
                }
                code if code == on_code => *active = true,
                _ => {}
            }
        }
        k += 1;
    }
}

/// Whether an RGB triple reads as a red foreground: the red channel clearly
/// dominates both others. Catches `<red>`'s maroon (128,0,0) — which some
/// terminals re-emit as a truecolor on capture — and pure red (255,0,0) while
/// rejecting the blue/green a neighboring cell (e.g. an OSC8 link's blue) might
/// re-serialize as.
fn is_red_rgb(r: u8, g: u8, b: u8) -> bool {
    r >= 96 && r > g.saturating_add(64) && r > b.saturating_add(64)
}

/// The 24-bit RGB an xterm 256-color index resolves to: the 16 ANSI colors, the
/// 6×6×6 color cube (16–231), then the grayscale ramp (232–255). Used to
/// classify a `38;5;n` / `38:5:n` foreground as red or not.
fn xterm_256_rgb(n: u8) -> (u8, u8, u8) {
    const ANSI_16: [(u8, u8, u8); 16] = [
        (0, 0, 0),
        (128, 0, 0),
        (0, 128, 0),
        (128, 128, 0),
        (0, 0, 128),
        (128, 0, 128),
        (0, 128, 128),
        (192, 192, 192),
        (128, 128, 128),
        (255, 0, 0),
        (0, 255, 0),
        (255, 255, 0),
        (0, 0, 255),
        (255, 0, 255),
        (0, 255, 255),
        (255, 255, 255),
    ];
    match n {
        0..=15 => ANSI_16[n as usize],
        16..=231 => {
            let c = n - 16;
            let level = |v: u8| if v == 0 { 0 } else { 55 + v * 40 };
            (level(c / 36), level((c % 36) / 6), level(c % 6))
        }
        232..=255 => {
            let v = 8 + (n - 232) * 10;
            (v, v, v)
        }
    }
}

/// Classifies a single ITU colon SGR subparameter group (e.g. `38:2::r:g:b`,
/// `38:5:n`, or a bare `31`) as setting the foreground to red (`Some(true)`),
/// to a non-red color / default (`Some(false)`), or as not a foreground code
/// (`None`, leaving the running state unchanged).
fn classify_fg_colon_group(parts: &[&str]) -> Option<bool> {
    match parts.first().copied() {
        Some("0") | Some("39") => Some(false),
        Some("31") | Some("91") => Some(true),
        Some("38") => match parts.get(1).copied() {
            // RGB channels are the trailing three numeric subparameters; an
            // optional empty colorspace-id subparameter may precede them.
            Some("2") => {
                let nums: Vec<u8> = parts[2..].iter().filter_map(|s| s.parse().ok()).collect();
                match nums.as_slice() {
                    [.., r, g, b] => Some(is_red_rgb(*r, *g, *b)),
                    _ => Some(false),
                }
            }
            Some("5") => Some(
                parts
                    .get(2)
                    .and_then(|s| s.parse::<u8>().ok())
                    .map(|n| {
                        let (r, g, b) = xterm_256_rgb(n);
                        is_red_rgb(r, g, b)
                    })
                    .unwrap_or(false),
            ),
            _ => Some(false),
        },
        // Any other explicit basic/bright foreground color is non-red.
        Some("30") | Some("32") | Some("33") | Some("34") | Some("35") | Some("36")
        | Some("37") | Some("90") | Some("92") | Some("93") | Some("94") | Some("95")
        | Some("96") | Some("97") => Some(false),
        _ => None,
    }
}

/// Applies one SGR parameter list to the running "foreground is red" flag.
///
/// An empty list (`ESC[m`) and the reset/default-foreground codes (`0`, `39`)
/// clear it; the basic red SGRs (`31`, `91`) set it; any other basic foreground
/// clears it. Extended foregrounds — `38;2;r;g;b` / `38:2::r:g:b` (truecolor)
/// and `38;5;n` / `38:5:n` (256-color) — set it iff the color reads as red
/// ([`is_red_rgb`]); some terminals re-emit `<red>` as a maroon truecolor on
/// capture, so the truecolor form must be classified rather than ignored.
///
/// Mirrors the two-level `;`/`:` SGR structure handled by [`apply_sgr_attr`]: a
/// colon group is self-contained and inspected whole, while a semicolon `38`
/// introducer consumes its following channel parameters so a channel value is
/// never read as its own color code.
fn apply_sgr_fg_red(params: &str, active: &mut bool) {
    if params.is_empty() {
        *active = false;
        return;
    }
    let groups: Vec<&str> = params.split(';').collect();
    let mut k = 0;
    while k < groups.len() {
        let group = groups[k];
        if group.contains(':') {
            let parts: Vec<&str> = group.split(':').collect();
            if let Some(state) = classify_fg_colon_group(&parts) {
                *active = state;
            }
        } else {
            match group {
                "0" | "39" => *active = false,
                "31" | "91" => *active = true,
                // Legacy semicolon extended foreground: classify from the channel
                // parameters and skip past them.
                "38" => {
                    match groups.get(k + 1).copied() {
                        Some("2") => {
                            *active = match (groups.get(k + 2), groups.get(k + 3), groups.get(k + 4))
                            {
                                (Some(r), Some(g), Some(b)) => {
                                    match (r.parse::<u8>(), g.parse::<u8>(), b.parse::<u8>()) {
                                        (Ok(r), Ok(g), Ok(b)) => is_red_rgb(r, g, b),
                                        _ => false,
                                    }
                                }
                                _ => false,
                            };
                            k += 4; // 2;r;g;b
                        }
                        Some("5") => {
                            *active = groups
                                .get(k + 2)
                                .and_then(|s| s.parse::<u8>().ok())
                                .map(|n| {
                                    let (r, g, b) = xterm_256_rgb(n);
                                    is_red_rgb(r, g, b)
                                })
                                .unwrap_or(false);
                            k += 2; // 5;n
                        }
                        _ => {}
                    }
                }
                "30" | "32" | "33" | "34" | "35" | "36" | "37" | "90" | "92" | "93" | "94"
                | "95" | "96" | "97" => *active = false,
                _ => {}
            }
        }
        k += 1;
    }
}

/// Reconstructs a running boolean SGR state cell-by-cell across a captured raw
/// row.
///
/// Walks `row`, handing the parameter list of each SGR sequence (`ESC [ … m`) to
/// `apply` — which updates a running flag — and pairing every *visible*
/// character with the flag's value at that point. Non-SGR escape sequences — the
/// cursor moves the bespoke path emits, OSC-8 link wrappers, and the
/// charset-designation sequences WezTerm brackets resets with (`ESC ( B`) — are
/// consumed without producing a cell. This is the basis for asserting that a
/// styling attribute covers the content cells but stops before the trailing
/// padding and border, rather than merely proving the attribute appears
/// *somewhere* on the row.
fn sgr_run_cells(row: &str, mut apply: impl FnMut(&str, &mut bool)) -> Vec<(char, bool)> {
    let chars: Vec<char> = row.chars().collect();
    let mut cells = Vec::new();
    let mut active = false;
    let mut i = 0;
    while i < chars.len() {
        if chars[i] != '\x1b' {
            cells.push((chars[i], active));
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
                    apply(&params, &mut active);
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
            Some(c) if is_nf_intermediate(c) => i = end_of_nf_escape(&chars, i),
            // Two-byte escape (e.g. ST `ESC \`) or a trailing ESC.
            _ => i += 2,
        }
    }
    cells
}

/// Reconstructs the running state of the intensity attribute `on_code`
/// (`"1"` bold, `"2"` dim) cell-by-cell across a captured raw row. See
/// [`sgr_run_cells`].
fn attr_run_cells(row: &str, on_code: &str) -> Vec<(char, bool)> {
    sgr_run_cells(row, |params, active| {
        apply_sgr_attr(params, on_code, active)
    })
}

/// Reconstructs whether the active SGR foreground is red, cell-by-cell across a
/// captured raw row. See [`sgr_run_cells`] and [`apply_sgr_fg_red`].
fn fg_red_run_cells(row: &str) -> Vec<(char, bool)> {
    sgr_run_cells(row, apply_sgr_fg_red)
}

/// Whether `c` is an `nF` escape intermediate byte (0x20..=0x2F) — the byte that
/// follows `ESC` in a charset-designation sequence such as `ESC ( B`.
fn is_nf_intermediate(c: &char) -> bool {
    ('\u{20}'..='\u{2f}').contains(c)
}

/// Index just past an `nF` escape sequence beginning at `esc` (`chars[esc] ==
/// ESC`, `chars[esc + 1]` an intermediate): consumes the intermediate bytes
/// (0x20..=0x2F) and the single final byte (0x30..=0x7E) — e.g. all of `ESC ( B`.
fn end_of_nf_escape(chars: &[char], esc: usize) -> usize {
    let mut j = esc + 1;
    while j < chars.len() && is_nf_intermediate(&chars[j]) {
        j += 1;
    }
    // chars[j] is the final byte; step past it (clamped at end-of-input).
    (j + 1).min(chars.len())
}

/// Asserts the intensity-attribute run (`on_code`: `"1"` bold, `"2"` dim,
/// named `attr` in messages) inside captured raw `row` covers every cell of
/// `needle` yet is off across the leading border and padding before the content
/// and the trailing padding and border glyph after it — the styled run must not
/// bleed past its content into the cell padding, either box border, a separator,
/// an earlier cell, or an adjacent row.
///
/// Reconstructs per-cell attribute state from the row's SGR transitions, so it
/// accepts whatever reset encoding the terminal used (`ESC[0m`, `ESC[22m`, or an
/// elided/coalesced form) rather than requiring one literal sequence.
fn assert_attr_contained(row: &str, needle: &str, on_code: &str, attr: &str) {
    assert_run_contained(&attr_run_cells(row, on_code), row, needle, attr);
}

/// Index of the cell's leading border glyph: the nearest `│` at or before the
/// content. The run/link state must be off from this glyph through the leading
/// padding, so styling cannot bleed backward onto the leading border, the
/// padding, a separator, or an earlier cell. Shared by the style-run and OSC8
/// containment assertions.
fn leading_border_index(
    cells: &[(char, bool)],
    row: &str,
    needle: &str,
    content_start: usize,
) -> usize {
    cells[..content_start]
        .iter()
        .rposition(|(c, _)| *c == '│')
        .unwrap_or_else(|| panic!("row carries no leading border glyph before {needle:?}: {row:?}"))
}

/// Asserts the reconstructed boolean run `cells` is active across every cell of
/// `needle` yet off across the leading border and padding *before* the content
/// and the trailing padding and border glyph *after* it, proving the run
/// (`label`: e.g. `bold`, `dim`, `red foreground`) does not bleed past its
/// content into the cell padding, the box borders, a separator, an earlier
/// cell, or an adjacent row.
fn assert_run_contained(cells: &[(char, bool)], row: &str, needle: &str, label: &str) {
    let visible: String = cells.iter().map(|(c, _)| *c).collect();
    let byte_start = visible.find(needle).unwrap_or_else(|| {
        panic!("content {needle:?} not found in the visible cells of row: {row:?}")
    });
    let content_start = visible[..byte_start].chars().count();
    let content_end = content_start + needle.chars().count();

    // Leading edge: the run must be off from the nearest preceding border glyph
    // through the cell's leading padding, so styling cannot bleed *backward*
    // onto the leading border, the leading padding, a separator, an earlier
    // cell, or (the border being a row's first visible cell) the prior row.
    let leading_border = leading_border_index(cells, row, needle, content_start);
    for (ch, is_active) in &cells[leading_border..content_start] {
        assert!(
            !*is_active,
            "leading border/padding cell {ch:?} before {needle:?} must not be {label} \
             (styling bled before the content): {row:?}",
        );
    }

    for (ch, is_active) in &cells[content_start..content_end] {
        assert!(
            *is_active,
            "content cell {ch:?} of {needle:?} must be {label}: {row:?}",
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
    for (ch, is_active) in &cells[content_end..=last_border] {
        assert!(
            !*is_active,
            "trailing padding/border cell {ch:?} after {needle:?} must not be {label} \
             (styling bled past the content): {row:?}",
        );
    }
}

/// Asserts a bold run is contained to `needle`'s content cells. See
/// [`assert_attr_contained`].
fn assert_bold_contained(row: &str, needle: &str) {
    assert_attr_contained(row, needle, "1", "bold");
}

/// Asserts a dim run is contained to `needle`'s content cells — active over the
/// content, off across the leading and trailing padding and borders. See
/// [`assert_attr_contained`].
fn assert_dim_contained(row: &str, needle: &str) {
    assert_attr_contained(row, needle, "2", "dim");
}

/// Asserts a red foreground is contained to `needle`'s content cells — active
/// over the content, off across the leading and trailing padding and borders —
/// so the red color does not bleed into the padding, either box border, a
/// separator, an earlier cell, or an adjacent row. See [`fg_red_run_cells`] and
/// [`assert_run_contained`].
fn assert_fg_red_contained(row: &str, needle: &str) {
    assert_run_contained(&fg_red_run_cells(row), row, needle, "red foreground");
}

/// Reconstructs the running OSC8-hyperlink open/closed state cell-by-cell across
/// a captured raw row.
///
/// Walks `row`, toggling a "link open" flag on each OSC8 sequence
/// (`ESC ] 8 ; params ; URI ST`, terminated by ST `ESC \` or BEL): a non-empty
/// URI opens a link, the empty-URI form `ESC ] 8 ; ; ST` closes it. Every
/// *visible* character is paired with the flag's value at that point; other
/// escapes (SGR, the cursor moves the bespoke path emits) are consumed without
/// producing a cell. This is the basis for asserting that an OSC8 link covers
/// only its label cells and closes before the trailing padding and border,
/// rather than merely proving the opener appears *somewhere* on the row.
///
/// The capture is the terminal's re-serialization of the final grid in display
/// order, so the link's `close` lands after the label regardless of how the
/// standard and cursor-alignment paths originally ordered their bytes.
fn osc8_run_cells(row: &str) -> Vec<(char, bool)> {
    let chars: Vec<char> = row.chars().collect();
    let mut cells = Vec::new();
    let mut link_open = false;
    let mut i = 0;
    while i < chars.len() {
        if chars[i] != '\x1b' {
            cells.push((chars[i], link_open));
            i += 1;
            continue;
        }
        match chars.get(i + 1) {
            // CSI: ESC [ params <final byte 0x40..=0x7E>. Consumed without cells.
            Some('[') => {
                let mut j = i + 2;
                while j < chars.len() && !('\u{40}'..='\u{7e}').contains(&chars[j]) {
                    j += 1;
                }
                if j >= chars.len() {
                    break; // malformed, unterminated CSI
                }
                i = j + 1;
            }
            // OSC: ESC ] body (BEL | ST). An OSC8 body toggles the link state.
            Some(']') => {
                let mut j = i + 2;
                let mut st_len = 0;
                while j < chars.len() {
                    if chars[j] == '\u{07}' {
                        st_len = 1;
                        break;
                    }
                    if chars[j] == '\x1b' && chars.get(j + 1) == Some(&'\\') {
                        st_len = 2;
                        break;
                    }
                    j += 1;
                }
                if j >= chars.len() {
                    break; // malformed, unterminated OSC
                }
                let body: String = chars[i + 2..j].iter().collect();
                // OSC8 hyperlink: `8;params;URI`. A non-empty URI opens the link;
                // the empty-URI close form (`8;;`) closes it. Other OSC bodies
                // (titles, etc.) leave the link state unchanged.
                if let Some(rest) = body.strip_prefix("8;") {
                    let uri = rest.split_once(';').map(|(_, uri)| uri).unwrap_or("");
                    link_open = !uri.is_empty();
                }
                i = j + st_len;
            }
            Some(c) if is_nf_intermediate(c) => i = end_of_nf_escape(&chars, i),
            // Two-byte escape (e.g. ST `ESC \`) or a trailing ESC.
            _ => i += 2,
        }
    }
    cells
}

/// Asserts the OSC8 hyperlink in captured raw `row` covers every cell of
/// `needle` (the link label) yet is closed across the leading border and padding
/// *before* the label and the trailing padding and border glyph *after* it — the
/// link must not leave the leading border, a separator, an earlier cell, or the
/// rest of this cell and its border clickable.
///
/// Reconstructs per-cell link state from the row's OSC8 transitions
/// ([`osc8_run_cells`]), so it accepts either ST terminator (`ESC \` or BEL) and
/// whatever ordering the terminal used when re-serializing the grid.
fn assert_link_contained(row: &str, needle: &str) {
    let cells = osc8_run_cells(row);
    let visible: String = cells.iter().map(|(c, _)| *c).collect();
    let byte_start = visible.find(needle).unwrap_or_else(|| {
        panic!("link label {needle:?} not found in the visible cells of row: {row:?}")
    });
    let content_start = visible[..byte_start].chars().count();
    let content_end = content_start + needle.chars().count();

    // Leading edge: the link must be closed from the nearest preceding border
    // glyph through the cell's leading padding, so an opener before the leading
    // border cannot leave the border, the padding, a separator, or an earlier
    // cell clickable.
    let leading_border = leading_border_index(&cells, row, needle, content_start);
    for (ch, is_open) in &cells[leading_border..content_start] {
        assert!(
            !*is_open,
            "leading border/padding cell {ch:?} before {needle:?} must not be inside the \
             OSC8 link (the link bled before its label): {row:?}",
        );
    }

    for (ch, is_open) in &cells[content_start..content_end] {
        assert!(
            *is_open,
            "label cell {ch:?} of {needle:?} must sit inside the OSC8 link: {row:?}",
        );
    }

    let last_border = cells
        .iter()
        .rposition(|(c, _)| *c == '│')
        .unwrap_or_else(|| panic!("row carries no trailing border glyph: {row:?}"));
    assert!(
        last_border >= content_end,
        "the trailing border must sit after the link label {needle:?}: {row:?}",
    );
    for (ch, is_open) in &cells[content_end..=last_border] {
        assert!(
            !*is_open,
            "trailing padding/border cell {ch:?} after {needle:?} must not be inside the \
             OSC8 link (the link bled past its label): {row:?}",
        );
    }
}

/// Asserts a styled Prose cell's capability-resolved SGR survives to the real
/// terminal with its styling *contained* and the box geometry intact: the bold
/// cell carries a bold attribute, the colored cell keeps its red foreground over
/// `Alice` yet off across the trailing padding and border, and the visible
/// borders and text remain in the displayed plain cells.
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
    // The red foreground must cover only `Alice` and turn off before the trailing
    // padding and border, proving the color does not bleed into the padding, the
    // box border, or the next row.
    assert_fg_red_contained(&row, "Alice");
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
    // Each run must be contained to its content cells — off across the trailing
    // padding and the row's trailing border — proving styling cannot bleed into
    // the padding, the border glyph, or the next row. Reconstructing the per-cell
    // state accepts whatever off-encoding the terminal used (the WezTerm harness
    // may re-emit `ESC[0m` as `ESC[22m` or elide it).
    assert_bold_contained(&bold_row, "alphaword");
    assert_fg_red_contained(&red_row, "bravoword");
}

/// Asserts the cursor-alignment bespoke path renders a styled Prose cell with
/// styling *contained* and geometry intact in a real terminal.
///
/// The cursor-positioning control sequences the bespoke path emits (`ESC [ N G`)
/// are *consumed* by the terminal — they move the cursor and are not retained in
/// the captured cell grid — so they cannot be asserted from a capture. What is
/// observable, and what the spec requires (the cursor path "preserve[s] visible
/// content and styling"), is the result: the bold run covers exactly the content
/// cells and turns off before the trailing padding and border (which also proves
/// the border is intact), and the text stays visible.
fn assert_prose_cursor_align<H: TerminalHarness>(harness: &mut H) {
    let frame = capture_bt(
        harness,
        "bt table --columns \"Msg\" --prose-row \"<b>cursorword</b>\" --cursor-align",
    );
    let row = cell_row(&frame, "cursorword")
        .unwrap_or_else(|| panic!("missing cursor-aligned Prose row.\nplain:\n{}", frame.plain));
    // The bold run must cover only `cursorword` and turn off before the trailing
    // padding and border — the cursor-alignment path must contain styling just as
    // the standard path does. This also proves a trailing border exists.
    assert_bold_contained(&row, "cursorword");
    assert!(
        frame.plain.contains("cursorword"),
        "the cursor-aligned cell text must remain visible.\nplain:\n{}",
        frame.plain,
    );
}

/// Asserts a `<dim>` Prose cell resolves dim styling onto the terminal cells and
/// that the dim run is contained to its content — off across the trailing
/// padding and border.
///
/// Dim (SGR 2) cannot be proved by a naive parameter scan: a `38;2;…` truecolor
/// SGR also carries a `2`. The per-cell SGR-state reconstruction in
/// [`assert_dim_contained`] is the proof — it both confirms dim over the content
/// and rejects any bleed into the padding or border.
fn assert_prose_cell_dim_styled<H: TerminalHarness>(harness: &mut H) {
    let frame = capture_bt(
        harness,
        "bt table --columns \"Status,Owner\" --prose-row \"<dim>inactive</dim>,<b>Alice</b>\"",
    );
    let row = cell_row(&frame, "inactive").unwrap_or_else(|| {
        panic!(
            "could not locate the dim Prose-cell data row.\nplain:\n{}\nraw:\n{}",
            frame.plain, frame.raw,
        )
    });
    assert_dim_contained(&row, "inactive");
    // Geometry: the box borders and both cell texts survive to the plain cells.
    assert!(
        frame.plain.contains('│')
            && frame.plain.contains("inactive")
            && frame.plain.contains("Alice"),
        "expected bordered, visible dim Prose-cell content.\nplain:\n{}",
        frame.plain,
    );
}

/// Asserts an `<a href=…>` Prose cell projects an OSC8 hyperlink to the real
/// terminal with the link *contained* to its label: the captured raw row retains
/// the OSC8 destination, the link is open across exactly the label cells, and it
/// closes before the trailing padding and border so the rest of the cell and the
/// border stay unclickable.
///
/// `cursor_align` selects the cursor-alignment bespoke path, which resolves
/// `StyledProse` through `Prose::render(term)` before planning — a different
/// implementation boundary from canonical tree reconstruction. Both paths must
/// project an equally contained link.
fn assert_prose_cell_link_styled<H: TerminalHarness>(harness: &mut H, cursor_align: bool) {
    let suffix = if cursor_align { " --cursor-align" } else { "" };
    let frame = capture_bt(
        harness,
        &format!(
            "bt table --columns \"Link\" \
             --prose-row '<a href=\"https://example.com\">go</a>'{suffix}"
        ),
    );
    let row = cell_row(&frame, "go").unwrap_or_else(|| {
        panic!(
            "could not locate the link Prose-cell data row (cursor_align={cursor_align}).\nplain:\n{}\nraw:\n{}",
            frame.plain, frame.raw,
        )
    });
    assert!(
        row.contains("\x1b]8;;https://example.com"),
        "the captured raw cell row must retain the OSC8 hyperlink destination \
         (cursor_align={cursor_align}): {row:?}"
    );
    // The link must cover only its label cells and close before the trailing
    // padding and border. This also proves a trailing border exists after the
    // label, so it is the geometry guard for the cursor-alignment path.
    assert_link_contained(&row, "go");
    if cursor_align {
        // The cursor-positioning control sequences are consumed by the terminal,
        // so the cursor path is verified through its result: the label stays
        // visible in the displayed grid.
        assert!(
            frame.plain.contains("go"),
            "the cursor-aligned link cell label must remain visible.\nplain:\n{}",
            frame.plain,
        );
    } else {
        // The standard path's visible label stays inside the cell box border.
        assert_bordered_line(&frame, "go");
    }
}

/// The inner cell segments of a rendered data row: the substrings between the
/// vertical border glyphs, in column order. The leading empty segment (before
/// the first border) and any trailing pane padding (after the last border) are
/// dropped, so each element is one cell's padded content.
fn data_row_cells(line: &str) -> Vec<&str> {
    let parts: Vec<&str> = line.split('│').collect();
    if parts.len() < 3 {
        return Vec::new();
    }
    parts[1..parts.len() - 1].to_vec()
}

/// Whether a padded cell segment is left- or right-aligned, inferred from the
/// asymmetry between its leading and trailing whitespace: a left-aligned cell
/// carries its fill on the right (more trailing space), a right-aligned cell on
/// the left. `None` when content fills the cell (equal slack) and neither edge
/// dominates — the mixed-row fixture sizes columns so every asserted cell has
/// slack.
fn left_or_right_aligned(segment: &str) -> Option<&'static str> {
    let lead = segment.chars().take_while(|c| c.is_whitespace()).count();
    let trail = segment.chars().rev().take_while(|c| c.is_whitespace()).count();
    match lead.cmp(&trail) {
        std::cmp::Ordering::Less => Some("left"),
        std::cmp::Ordering::Greater => Some("right"),
        std::cmp::Ordering::Equal => None,
    }
}

/// Asserts a mixed typed row renders with the geometry the typed columns
/// require: the styled Prose cell sits at the left edge of its cell and each
/// numeric cell (integer, float, currency) sits at the right edge, with the
/// typed values formatted. `cursor_align` selects the cursor-alignment bespoke
/// path; both paths must place the values identically.
///
/// The column headers are wider than their values so every asserted cell has
/// fill on one side, making the left/right placement observable in the captured
/// grid rather than inferred from a hint.
fn assert_mixed_row_alignment<H: TerminalHarness>(harness: &mut H, cursor_align: bool) {
    let suffix = if cursor_align { " --cursor-align" } else { "" };
    let frame = capture_bt(
        harness,
        &format!(
            "bt table --columns \"Status,Quantity,Ratio,Amount\" \
             --column-types \",int,float,usd\" \
             --mixed-row \"<b>ok</b>,5,3.5,9.99\"{suffix}"
        ),
    );
    let line = plain_cell_row(&frame, "$9.99").unwrap_or_else(|| {
        panic!(
            "could not locate the mixed typed-row data line (cursor_align={cursor_align}).\nplain:\n{}",
            frame.plain,
        )
    });
    let cells = data_row_cells(&line);
    assert!(
        cells.len() >= 4,
        "expected 4 rendered cells in the mixed row, got {cells:?} from {line:?}"
    );
    // The typed values were formatted into the rendered grid.
    assert!(
        line.contains("ok") && line.contains("3.50") && line.contains("$9.99"),
        "expected the typed values formatted in the data row: {line:?}"
    );
    // The styled Prose cell hugs the left edge.
    assert_eq!(
        left_or_right_aligned(cells[0]),
        Some("left"),
        "the StyledProse cell must be left-aligned (content at the left edge): {:?}",
        cells[0],
    );
    // Each numeric cell hugs the right edge.
    for (idx, name) in [(1usize, "integer"), (2, "float"), (3, "currency")] {
        assert_eq!(
            left_or_right_aligned(cells[idx]),
            Some("right"),
            "the {name} cell must be right-aligned (content at the right edge): {:?}",
            cells[idx],
        );
    }
}

#[test]
#[serial(level2_terminal)]
fn level2_prose_cells_in_wezterm() {
    require_level!(Level::L2, WezTermHarness::available(), Backend::WezTerm);
    let mut guard = SHARED_WEZTERM
        .get_or_init(|| WezTermHarness::shared_or_spawn().expect("attach/spawn WezTerm"));
    let harness = guard.as_mut().expect("shared WezTerm harness present");
    harness.send_text(b"clear\n").expect("send_text failed");
    harness.settle();
    assert_prose_cell_styled(harness);
    assert_prose_cell_dim_styled(harness);
    assert_prose_cell_link_styled(harness, false);
    assert_prose_cell_link_styled(harness, true);
    assert_prose_cell_wraps_styled(harness);
    assert_prose_cell_multiline_styled(harness);
    assert_prose_rows_independently_styled(harness);
    assert_prose_cursor_align(harness);
    assert_mixed_row_alignment(harness, false);
    assert_mixed_row_alignment(harness, true);
}

#[test]
#[serial(level2_terminal)]
fn level2_prose_cells_in_kitty() {
    require_level!(Level::L2, KittyHarness::available(), Backend::Kitty);
    let mut guard =
        SHARED_KITTY.get_or_init(|| KittyHarness::shared_or_spawn().expect("attach/spawn kitty"));
    let harness = guard.as_mut().expect("shared Kitty harness present");
    harness.send_text(b"clear\n").expect("send_text failed");
    harness.settle();
    assert_prose_cell_styled(harness);
    assert_prose_cell_dim_styled(harness);
    assert_prose_cell_link_styled(harness, false);
    assert_prose_cell_link_styled(harness, true);
    assert_prose_cell_wraps_styled(harness);
    assert_prose_cell_multiline_styled(harness);
    assert_prose_rows_independently_styled(harness);
    assert_prose_cursor_align(harness);
    assert_mixed_row_alignment(harness, false);
    assert_mixed_row_alignment(harness, true);
}

#[test]
#[serial(level2_terminal)]
fn level2_prose_cells_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
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

    // tmux carries no OSC8 protocol, but the link label and the cell box geometry
    // must still relay faithfully (the OSC8 destination is asserted on the
    // styling terminals above).
    let frame = capture_bt(
        harness,
        "bt table --columns \"Link\" --prose-row '<a href=\"https://example.com\">go</a>'",
    );
    assert_bordered_line(&frame, "go");

    // Typed-column alignment is geometry, not styling, so tmux must reproduce it:
    // the Prose cell at the left edge and the numeric cells at the right edge.
    assert_mixed_row_alignment(harness, false);
}

/// L1 unit coverage for the [`assert_bold_contained`] SGR-state reconstruction.
///
/// The real-terminal (L2) tests cannot run without a usable controlling TTY, so
/// these prove the *assertion itself* has no false-positive window — bold
/// bleeding into the leading or trailing padding or borders, or a styled border,
/// must be rejected — against synthetic rows that mirror the SGR forms
/// WezTerm/Kitty re-emit on capture (semicolon and ITU colon, with the
/// documented bold-off encodings).
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
    fn accepts_wezterm_charset_designation_around_reset() {
        // WezTerm brackets its SGR with the charset-designation escape `ESC ( B`;
        // its single final byte `B` must be consumed, not read as a bold cell
        // bleeding past the content.
        assert!(accepts(
            "│ \x1b(B\x1b[0;1mAlice\x1b(B\x1b[0m │",
            "Alice"
        ));
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
    fn rejects_bold_on_leading_border() {
        // Bold opens *before* the leading border and covers the border, the
        // leading padding, and the content. The content is bold, but the bled
        // leading border/padding must still be rejected.
        assert!(!accepts("\x1b[1m│ alpha\x1b[0m    │", "alpha"));
    }

    #[test]
    fn rejects_bold_on_leading_padding() {
        // The leading border is unstyled, but bold opens over the leading
        // padding before the content.
        assert!(!accepts("│\x1b[1m alpha\x1b[0m    │", "alpha"));
    }

    #[test]
    fn rejects_non_bold_content() {
        // The content carries no bold at all — must not be accepted.
        assert!(!accepts("│ alpha\x1b[0m    │", "alpha"));
    }
}

/// L1 unit coverage for the [`assert_dim_contained`] SGR-state reconstruction.
///
/// Mirrors [`bold_containment`] for the dim attribute (SGR 2). The real-terminal
/// (L2) dim test cannot run without a usable controlling TTY, so these prove the
/// *assertion itself* has no false-positive window — including a dim bleed onto
/// the leading border or padding before the content. The dim-specific hazard is
/// that a `38;2;…`/`38:2:…` truecolor SGR also carries a `2` selector, which
/// must NOT be read as the dim attribute.
#[cfg(test)]
mod dim_containment {
    use super::assert_dim_contained;

    fn accepts(row: &str, needle: &str) -> bool {
        std::panic::catch_unwind(|| assert_dim_contained(row, needle)).is_ok()
    }

    #[test]
    fn accepts_dim_reset_before_padding_and_border() {
        // `ESC[2m`content`ESC[0m`, padding, then the trailing border.
        assert!(accepts("│ \x1b[2minactive\x1b[0m  │", "inactive"));
    }

    #[test]
    fn accepts_normal_intensity_off_encoding() {
        // Terminals may re-emit the reset as normal-intensity (`ESC[22m`), which
        // clears dim just as it clears bold.
        assert!(accepts("│ \x1b[2minactive\x1b[22m  │", "inactive"));
    }

    #[test]
    fn accepts_wezterm_charset_designation_around_reset() {
        // The exact WezTerm capture form: SGR bracketed by the charset escape
        // `ESC ( B`, whose final byte `B` must be consumed rather than read as a
        // dim cell after the content.
        assert!(accepts(
            "│ \x1b(B\x1b[0;2minactive\x1b(B\x1b[0m │",
            "inactive"
        ));
    }

    #[test]
    fn truecolor_channel_two_is_not_read_as_dim() {
        // A `38;2;…` (and ITU `38:2:…`) truecolor SGR carries a `2` selector
        // that must not be misread as the dim attribute. The content here is
        // colored truecolor but NOT dim, so it must be rejected.
        assert!(!accepts("│ \x1b[38;2;128;0;0minactive\x1b[0m  │", "inactive"));
        assert!(!accepts("│ \x1b[38:2::128:0:0minactive\x1b[0m  │", "inactive"));
    }

    #[test]
    fn rejects_dim_bleeding_into_padding_and_border() {
        // No reset after the content: dim runs through the padding and border.
        assert!(!accepts("│ \x1b[2minactive    │", "inactive"));
    }

    #[test]
    fn rejects_dim_border_after_reset() {
        // Reset after the content, but a fresh dim re-opens over the trailing
        // border.
        assert!(!accepts("│ \x1b[2minactive\x1b[0m   \x1b[2m│", "inactive"));
    }

    #[test]
    fn rejects_dim_on_leading_border() {
        // Dim opens *before* the leading border and covers the border, the
        // leading padding, and the content.
        assert!(!accepts("\x1b[2m│ inactive\x1b[0m  │", "inactive"));
    }

    #[test]
    fn rejects_dim_on_leading_padding() {
        // The leading border is unstyled, but dim opens over the leading padding
        // before the content.
        assert!(!accepts("│\x1b[2m inactive\x1b[0m  │", "inactive"));
    }

    #[test]
    fn rejects_non_dim_content() {
        // The content carries no dim at all — must not be accepted.
        assert!(!accepts("│ inactive\x1b[0m    │", "inactive"));
    }
}

/// L1 unit coverage for the [`assert_fg_red_contained`] foreground-color-state
/// reconstruction.
///
/// Mirrors [`bold_containment`] for the red foreground, including a red bleed
/// onto the leading border, padding, or a separator before the content. The
/// color-specific hazards are that a non-red extended color — a blue truecolor
/// or 256-color index, the form a neighboring cell (e.g. an OSC8 link) might
/// re-serialize as — must NOT be read as red, and that a default-foreground
/// reset (`39`) clears red just as a full reset (`0`) does.
#[cfg(test)]
mod fg_red_containment {
    use super::assert_fg_red_contained;

    /// Runs `assert_fg_red_contained` and reports whether it accepted the row,
    /// swallowing the panic so negative cases can assert rejection.
    fn accepts(row: &str, needle: &str) -> bool {
        std::panic::catch_unwind(|| assert_fg_red_contained(row, needle)).is_ok()
    }

    #[test]
    fn accepts_basic_red_reset_before_padding_and_border() {
        // `ESC[31m`content`ESC[0m`, padding, then the trailing border.
        assert!(accepts("│ \x1b[31mAlice\x1b[0m    │", "Alice"));
    }

    #[test]
    fn accepts_bright_red() {
        assert!(accepts("│ \x1b[91mAlice\x1b[0m    │", "Alice"));
    }

    #[test]
    fn accepts_default_foreground_off_encoding() {
        // A terminal may clear the color with default-foreground (`ESC[39m`)
        // rather than a full reset; either clears red.
        assert!(accepts("│ \x1b[31mAlice\x1b[39m    │", "Alice"));
    }

    #[test]
    fn accepts_wezterm_maroon_truecolor_capture_form() {
        // WezTerm re-emits `<red>` as a maroon truecolor (128,0,0), bracketed by
        // the charset-designation escape `ESC ( B`, in both SGR forms.
        assert!(accepts(
            "│ \x1b(B\x1b[38;2;128;0;0mAlice\x1b(B\x1b[0m │",
            "Alice"
        ));
        assert!(accepts(
            "│ \x1b(B\x1b[38:2::128:0:0mAlice\x1b(B\x1b[0m │",
            "Alice"
        ));
    }

    #[test]
    fn accepts_red_256_color_index() {
        // The 256-color red index (9) resolves to (255,0,0) and is red.
        assert!(accepts("│ \x1b[38;5;9mAlice\x1b[0m    │", "Alice"));
    }

    #[test]
    fn rejects_blue_foreground() {
        // A blue truecolor / 256-color / basic-blue foreground must NOT be read
        // as red — the exact false positive a broad `38;2;`/`38;5;` presence
        // check hits on a neighboring cell's color (e.g. an OSC8 link's blue).
        assert!(!accepts("│ \x1b[38;2;0;0;128mAlice\x1b[0m    │", "Alice"));
        assert!(!accepts("│ \x1b[38:2::0:0:128mAlice\x1b[0m    │", "Alice"));
        assert!(!accepts("│ \x1b[38;5;21mAlice\x1b[0m    │", "Alice"));
        assert!(!accepts("│ \x1b[34mAlice\x1b[0m    │", "Alice"));
    }

    #[test]
    fn rejects_red_bleeding_into_padding_and_border() {
        // No reset after the content: red runs through the padding and border.
        assert!(!accepts("│ \x1b[31mAlice    │", "Alice"));
    }

    #[test]
    fn rejects_red_border_after_reset() {
        // Reset after the content, but a fresh red re-opens over the trailing
        // border — the border would render red.
        assert!(!accepts("│ \x1b[31mAlice\x1b[0m   \x1b[31m│", "Alice"));
    }

    #[test]
    fn rejects_red_on_leading_border() {
        // Red opens *before* the leading border and covers the border, the
        // leading padding, and the content — the leading border would render red.
        assert!(!accepts("\x1b[31m│ Alice\x1b[0m    │", "Alice"));
    }

    #[test]
    fn rejects_red_on_leading_padding() {
        // The leading border is unstyled, but red opens over the leading padding
        // before the content.
        assert!(!accepts("│\x1b[31m Alice\x1b[0m    │", "Alice"));
    }

    #[test]
    fn rejects_red_bleeding_from_preceding_cell_across_separator() {
        // Red opens in an earlier cell and stays active through that cell and the
        // separator, ending only after `Alice`. The styled separator and the
        // padding after it must reject the bleed — the review's worked example.
        assert!(!accepts("│ \x1b[31mactive │ Alice\x1b[0m   │", "Alice"));
    }

    #[test]
    fn rejects_non_red_content() {
        // The content carries no foreground color at all — must not be accepted.
        assert!(!accepts("│ Alice\x1b[0m    │", "Alice"));
    }
}

/// L1 unit coverage for the [`assert_link_contained`] OSC8-state reconstruction.
///
/// The real-terminal (L2) link test cannot run without a usable controlling TTY,
/// so these prove the *assertion itself* has no false-positive window — an OSC8
/// link bleeding into the leading or trailing padding/borders, or a re-opened
/// link over the border, must be rejected — against synthetic rows that mirror
/// both OSC8 terminators (ST `ESC \` and BEL) terminals re-emit on capture.
#[cfg(test)]
mod link_containment {
    use super::assert_link_contained;

    /// Runs `assert_link_contained` and reports whether it accepted the row,
    /// swallowing the panic so negative cases can assert rejection.
    fn accepts(row: &str, needle: &str) -> bool {
        std::panic::catch_unwind(|| assert_link_contained(row, needle)).is_ok()
    }

    #[test]
    fn accepts_link_closed_before_padding_and_border_st() {
        // OSC8 open (ST-terminated), label, OSC8 close, padding, trailing border.
        assert!(accepts(
            "│ \x1b]8;;https://example.com\x1b\\go\x1b]8;;\x1b\\    │",
            "go"
        ));
    }

    #[test]
    fn accepts_link_closed_before_padding_and_border_bel() {
        // Same, but with the BEL terminator some terminals re-emit on capture.
        assert!(accepts(
            "│ \x1b]8;;https://example.com\x07go\x1b]8;;\x07    │",
            "go"
        ));
    }

    #[test]
    fn accepts_link_with_params_in_opener() {
        // An OSC8 opener may carry params before the URI (`8;id=x;URI`); the URI
        // is still non-empty, so the label is inside the link.
        assert!(accepts(
            "│ \x1b]8;id=x;https://example.com\x07go\x1b]8;;\x07  │",
            "go"
        ));
    }

    #[test]
    fn accepts_wezterm_charset_designation_around_link() {
        // WezTerm brackets the cell's SGR with the charset escape `ESC ( B`; its
        // final byte `B` must be consumed, not read as a visible cell that would
        // perturb the label offset or the trailing-region scan.
        assert!(accepts(
            "│ \x1b(B\x1b]8;;https://example.com\x07go\x1b]8;;\x07\x1b(B\x1b[0m │",
            "go"
        ));
    }

    #[test]
    fn rejects_link_bleeding_into_padding_and_border() {
        // No close after the label: the link runs through the padding and border.
        assert!(!accepts(
            "│ \x1b]8;;https://example.com\x07go    │",
            "go"
        ));
    }

    #[test]
    fn rejects_link_reopened_over_border() {
        // Closed after the label, but a fresh opener re-opens the link over the
        // trailing border — the border would stay clickable.
        assert!(!accepts(
            "│ \x1b]8;;https://example.com\x07go\x1b]8;;\x07   \x1b]8;;https://example.com\x07│",
            "go"
        ));
    }

    #[test]
    fn rejects_link_opened_before_leading_border() {
        // The OSC8 opener precedes the leading border, so the border, the
        // leading padding, and the label all sit inside the link — the leading
        // border and padding would stay clickable.
        assert!(!accepts(
            "\x1b]8;;https://example.com\x07│ go\x1b]8;;\x07    │",
            "go"
        ));
    }

    #[test]
    fn rejects_unlinked_label() {
        // The label carries no OSC8 link at all — must not be accepted.
        assert!(!accepts("│ go    │", "go"));
    }
}

/// L1 unit coverage for the mixed-row geometry helpers ([`data_row_cells`],
/// [`left_or_right_aligned`]).
///
/// The real-terminal (L2) mixed-row test cannot run without a usable controlling
/// TTY, so these prove the geometry classification itself: cell splitting drops
/// borders and pane padding, left-aligned cells carry their fill on the right,
/// and right-aligned cells carry it on the left.
#[cfg(test)]
mod cell_alignment {
    use super::{data_row_cells, left_or_right_aligned};

    #[test]
    fn splits_inner_cells_dropping_borders_and_pane_padding() {
        // Leading border, three cells, trailing border, then pane padding.
        let cells = data_row_cells("│ ok    │     5 │  $9.99 │   ");
        assert_eq!(cells.len(), 3, "three inner cells: {cells:?}");
        assert!(
            cells.iter().all(|c| !c.contains('│')),
            "inner cells carry no border glyphs: {cells:?}"
        );
        assert_eq!(cells[0].trim(), "ok");
        assert_eq!(cells[1].trim(), "5");
        assert_eq!(cells[2].trim(), "$9.99");
    }

    #[test]
    fn too_few_borders_yields_no_cells() {
        assert!(data_row_cells("no borders here").is_empty());
    }

    #[test]
    fn left_alignment_has_fill_on_the_right() {
        assert_eq!(left_or_right_aligned(" ok       "), Some("left"));
    }

    #[test]
    fn right_alignment_has_fill_on_the_left() {
        assert_eq!(left_or_right_aligned("        5 "), Some("right"));
        assert_eq!(left_or_right_aligned("  $9.99 "), Some("right"));
    }

    #[test]
    fn equal_slack_is_ambiguous() {
        assert_eq!(left_or_right_aligned(" mid "), None);
    }
}
