//! Ground-truth layout invariants for rendered terminal output.
//!
//! These predicates assert that a *single* rendered ANSI string is **correct**
//! against properties that must hold for every block-level shape under every
//! layout — independent of any second renderer to compare against. Parity
//! harnesses (e.g. darkmatter's `render_comparison`) can never catch a fault
//! both renderers share; these can.
//!
//! Each predicate takes the rendered output plus a [`LayoutExpectation`]
//! describing the scenario's resolved margins and physical width, and returns
//! `Ok(())` when the property holds or `Err(message)` describing the breach.
//!
//! ## Reuse
//!
//! The predicates operate on plain strings and a small expectation struct —
//! they carry no darkmatter- or biscuit-terminal-specific dependency. Both
//! crates share the same `Layout`/`Margin`/background-fill model, so both can
//! sweep their renderable components against this one contract.
//!
//! ## Invariant catalogue
//!
//! | Fn | Property | Applies to |
//! |----|----------|------------|
//! | [`containment`] | No line's visible width exceeds the physical width (I1). | every shape |
//! | [`no_clear_to_eol`] | No `\x1b[K` (clear-to-edge) under an active layout (I2a). | every shape |
//! | [`background_rectangle`] | Each background-filled line opens its fill at `left` and ends it at `width - right` (I2b). | full-bleed panels (code blocks, page bg) |
//! | [`right_boundary_coherence`] | Every background-bearing line in a contiguous panel ends its fill at the same column (I3). | full-bleed panels |
//! | [`left_offset_uniformity`] | Every background-bearing line opens its fill at the same column, equal to `left` (I4). | full-bleed panels |
//! | [`vertical_rhythm`] | No run of >=2 consecutive interior blank lines (I5). | every shape |
//! | [`margin_blanks`] | Leading/trailing blank rows equal `top`/`bottom` exactly (I5b). | every shape |
//! | [`blank_line_idempotent`] | Output is invariant to the number of blank lines (>=1) between blocks (I6). | every shape |
//!
//! A "blank" line is one whose visible content is empty after ANSI stripping
//! **and** which carries no background fill — a background-filled padding row
//! (e.g. a code-block padding row) is content, not blank.

use unicode_width::UnicodeWidthChar;

use crate::strip_ansi;

/// Display width of `ch` in terminal cells, defaulting unknown widths to `1`.
///
/// `UnicodeWidthChar::width` returns `None` for control characters and a small
/// set of ambiguous-width glyphs (e.g. some emoji like `✅`); modern terminals
/// render these as a single cell, so `1` is the safe fallback. This is the
/// same convention used by `biscuit_terminal::utils::block_constraint`.
fn cell_width(ch: char) -> usize {
    UnicodeWidthChar::width(ch).unwrap_or(1)
}

/// The resolved layout a rendered output is expected to satisfy.
///
/// All fields are in terminal cells. `left`/`right`/`top`/`bottom` are the
/// resolved offsets the layout applied (margin and/or alignment), not the raw
/// builder inputs.
#[derive(Debug, Clone, Copy)]
pub struct LayoutExpectation {
    /// Physical terminal width.
    pub width: usize,
    /// Resolved left offset: leading cells before content on every line.
    pub left: usize,
    /// Resolved right offset: trailing cells after content on every line.
    pub right: usize,
    /// Expected number of leading blank rows (top margin).
    pub top: usize,
    /// Expected number of trailing blank rows (bottom margin).
    pub bottom: usize,
}

/// Visible width of a (possibly ANSI-bearing) line, in **terminal cells**.
///
/// Strips ANSI/CSI/OSC/charset-designation escapes, then sums the per-`char`
/// display width via `unicode-width`. This matches the cell-based semantics of
/// the spec's I1 invariant: a line of full-width CJK glyphs occupies twice as
/// many cells as `chars().count()` would report, and a naive char count would
/// let `I1` pass while the real terminal wrapped.
pub fn visible_width(line: &str) -> usize {
    strip_ansi(line).chars().map(cell_width).sum()
}

/// A line is "blank" when it has no visible glyphs after ANSI stripping **and**
/// carries no background fill. A background-filled padding row is content.
pub fn is_blank(line: &str) -> bool {
    strip_ansi(line).trim().is_empty() && !line.contains("\x1b[48")
}

/// Advances `bg_active` over one SGR parameter list (the text between `\x1b[`
/// and the final `m`). Handles extended `38;5/2` (foreground) and `48;5/2`
/// (background) forms so their numeric arguments are not misread as further
/// SGR codes.
fn update_bg(params: &str, bg_active: &mut bool) {
    let codes: Vec<&str> = if params.is_empty() {
        vec!["0"]
    } else {
        params.split(';').collect()
    };
    let mut k = 0;
    while k < codes.len() {
        match codes[k] {
            // Full / default-background reset both clear the active background.
            "0" | "49" => *bg_active = false,
            "48" => {
                // Extended background: 48;5;n or 48;2;r;g;b. Skip its args.
                match codes.get(k + 1).copied() {
                    Some("5") => k += 2,
                    Some("2") => k += 4,
                    _ => {}
                }
                *bg_active = true;
            }
            "38" => {
                // Extended foreground: skip its args so r;g;b aren't misread.
                match codes.get(k + 1).copied() {
                    Some("5") => k += 2,
                    Some("2") => k += 4,
                    _ => {}
                }
            }
            _ => {}
        }
        k += 1;
    }
}

/// The background extent of one line: `(leading, fill)` where `leading` is the
/// number of visible **terminal cells** before the background first becomes
/// active and `fill` is the number of cells painted with an active background.
///
/// Widths come from `unicode-width`, matching the cell-based semantics of the
/// spec's layout invariants: a full-width CJK glyph counts as two cells, a
/// zero-width combining mark as zero. Returns `None` when no background is ever
/// active on the line.
fn bg_extent(line: &str) -> Option<(usize, usize)> {
    let mut chars = line.chars().peekable();
    let mut visible = 0usize;
    let mut bg_active = false;
    let mut leading: Option<usize> = None;
    let mut fill = 0usize;

    while let Some(c) = chars.next() {
        if c == '\x1b' {
            if chars.peek() == Some(&'[') {
                chars.next(); // consume '['
                let mut params = String::new();
                let mut final_byte = '\0';
                for d in chars.by_ref() {
                    if ('\u{40}'..='\u{7e}').contains(&d) {
                        final_byte = d;
                        break;
                    }
                    params.push(d);
                }
                if final_byte == 'm' {
                    update_bg(&params, &mut bg_active);
                }
            } else {
                // Non-CSI escape (e.g. charset designation); skip one char.
                chars.next();
            }
            continue;
        }
        let w = cell_width(c);
        if bg_active && w > 0 {
            if leading.is_none() {
                leading = Some(visible);
            }
            fill += w;
        }
        visible += w;
    }

    leading.map(|l| (l, fill))
}

/// Collects `(leading, fill)` for every background-bearing line.
fn bg_lines(rendered: &str) -> Vec<(usize, usize)> {
    rendered.lines().filter_map(bg_extent).collect()
}

/// I1 Containment — no rendered line's visible width exceeds the physical
/// terminal width.
pub fn containment(rendered: &str, exp: &LayoutExpectation) -> Result<(), String> {
    for (i, line) in rendered.lines().enumerate() {
        let w = visible_width(line);
        if w > exp.width {
            return Err(format!(
                "I1 Containment: line {i} visible width {w} exceeds terminal width {}",
                exp.width
            ));
        }
    }
    Ok(())
}

/// I2a NoClearToEol — no `\x1b[K` (Erase in Line) is emitted. Clear-to-edge is
/// incompatible with margin decoration applied after rendering.
pub fn no_clear_to_eol(rendered: &str, _exp: &LayoutExpectation) -> Result<(), String> {
    let count = rendered.matches("\x1b[K").count();
    if count > 0 {
        return Err(format!(
            "I2a NoClearToEol: found {count} `\\x1b[K` (clear-to-edge) under active layout"
        ));
    }
    Ok(())
}

/// I2b BackgroundRectangle — every background-filled line ends its fill at
/// `width - right`, so the panel's right edge sits on the margin boundary and
/// no fill runs to the physical edge (the `\x1b[K` defect).
///
/// The *left* edge is intentionally not constrained here: right-aligned chrome
/// such as a code block's language pill legitimately opens its (narrow)
/// background mid-line while still ending it on the right boundary. The panel
/// body's left edge is pinned by [`left_offset_uniformity`].
///
/// Intended for full-bleed panels (code blocks, page background). Components
/// with intentionally partial-width backgrounds (table cells, blockquote
/// gutters, inline code) do not satisfy this and should not be checked with it.
pub fn background_rectangle(rendered: &str, exp: &LayoutExpectation) -> Result<(), String> {
    let expected_end = exp.width.saturating_sub(exp.right);
    for (i, line) in rendered.lines().enumerate() {
        let Some((leading, fill)) = bg_extent(line) else {
            continue;
        };
        if leading + fill != expected_end {
            return Err(format!(
                "I2b BackgroundRectangle: line {i} ends its fill at column {}, expected width - right = {expected_end}",
                leading + fill
            ));
        }
    }
    Ok(())
}

/// I3 RightBoundaryCoherence — every background-bearing line ends its fill at
/// the same column. Catches a panel whose chrome (e.g. a header pill) and body
/// disagree on where the background stops.
pub fn right_boundary_coherence(rendered: &str, _exp: &LayoutExpectation) -> Result<(), String> {
    let mut end: Option<usize> = None;
    for (i, (leading, fill)) in bg_lines(rendered).into_iter().enumerate() {
        let line_end = leading + fill;
        match end {
            None => end = Some(line_end),
            Some(e) if e != line_end => {
                return Err(format!(
                    "I3 RightBoundaryCoherence: background line {i} ends at column {line_end}, but an earlier line ends at {e}"
                ));
            }
            _ => {}
        }
    }
    Ok(())
}

/// I4 LeftOffsetUniformity — every *full-bleed* background line (one whose fill
/// spans the entire content width `width - left - right`) opens its fill at
/// exactly the resolved `left` offset.
///
/// Narrower chrome (e.g. a right-aligned language pill) is exempt: it does not
/// span the content width and its left edge is not the panel's left edge.
pub fn left_offset_uniformity(rendered: &str, exp: &LayoutExpectation) -> Result<(), String> {
    let content_width = exp.width.saturating_sub(exp.left + exp.right);
    for (i, (leading, fill)) in bg_lines(rendered).into_iter().enumerate() {
        if fill != content_width {
            continue; // chrome, not a full-bleed panel row
        }
        if leading != exp.left {
            return Err(format!(
                "I4 LeftOffsetUniformity: full-bleed line {i} opens its fill at column {leading}, expected left = {}",
                exp.left
            ));
        }
    }
    Ok(())
}

/// I5 VerticalRhythm — no run of >=2 consecutive *interior* blank lines.
///
/// Only the interior (between the first and last non-blank line) is checked;
/// leading/trailing page margins legitimately stack and are validated by
/// [`margin_blanks`].
pub fn vertical_rhythm(rendered: &str, _exp: &LayoutExpectation) -> Result<(), String> {
    let lines: Vec<&str> = rendered.lines().collect();
    let (Some(first), Some(last)) = (
        lines.iter().position(|l| !is_blank(l)),
        lines.iter().rposition(|l| !is_blank(l)),
    ) else {
        return Ok(());
    };
    let mut run = 0usize;
    for (offset, line) in lines[first..=last].iter().enumerate() {
        if is_blank(line) {
            run += 1;
            if run >= 2 {
                return Err(format!(
                    "I5 VerticalRhythm: >=2 consecutive interior blank lines ending at line {}",
                    first + offset
                ));
            }
        } else {
            run = 0;
        }
    }
    Ok(())
}

/// I5b MarginBlanks — leading blank rows equal `top` and trailing blank rows
/// equal `bottom`, with no constant offset.
pub fn margin_blanks(rendered: &str, exp: &LayoutExpectation) -> Result<(), String> {
    let lines: Vec<&str> = rendered.lines().collect();
    let leading = lines.iter().take_while(|l| is_blank(l)).count();
    let trailing = lines.iter().rev().take_while(|l| is_blank(l)).count();
    if leading != exp.top {
        return Err(format!(
            "I5b MarginBlanks: leading blank rows = {leading}, expected top = {}",
            exp.top
        ));
    }
    if trailing != exp.bottom {
        return Err(format!(
            "I5b MarginBlanks: trailing blank rows = {trailing}, expected bottom = {}",
            exp.bottom
        ));
    }
    Ok(())
}

/// I6 BlankLineIdempotence — two renders that differ only in the number of
/// blank lines (>=1) separating blocks in the source must be byte-for-byte
/// identical. In Markdown, any run of >=2 newlines is a single paragraph break.
pub fn blank_line_idempotent(a: &str, b: &str) -> Result<(), String> {
    if a == b {
        Ok(())
    } else {
        Err(format!(
            "I6 BlankLineIdempotence: outputs differ (len {} vs {})",
            a.len(),
            b.len()
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXP: LayoutExpectation = LayoutExpectation {
        width: 10,
        left: 2,
        right: 2,
        top: 0,
        bottom: 0,
    };

    /// A well-formed full-bleed code line: 2 unstyled left-margin spaces, a
    /// 6-cell background fill (= width - left - right), reset, 2 unstyled
    /// right-margin spaces.
    fn good_bg_line() -> String {
        format!("  \x1b[48;2;0;0;0m{}\x1b[0m  ", " ".repeat(6))
    }

    #[test]
    fn bg_extent_measures_leading_and_fill() {
        assert_eq!(bg_extent(&good_bg_line()), Some((2, 6)));
    }

    #[test]
    fn bg_extent_none_without_background() {
        assert_eq!(bg_extent("  plain text  "), None);
    }

    #[test]
    fn containment_passes_at_exact_width() {
        assert!(containment(&good_bg_line(), &EXP).is_ok());
    }

    #[test]
    fn containment_flags_overflow() {
        let line = format!("{}xxx", good_bg_line());
        assert!(containment(&line, &EXP).is_err());
    }

    #[test]
    fn no_clear_to_eol_flags_the_escape() {
        assert!(no_clear_to_eol("text\x1b[K", &EXP).is_err());
        assert!(no_clear_to_eol("text", &EXP).is_ok());
    }

    #[test]
    fn background_rectangle_accepts_aligned_fill() {
        assert!(background_rectangle(&good_bg_line(), &EXP).is_ok());
    }

    #[test]
    fn background_rectangle_flags_clear_to_edge_shape() {
        // The reported defect: fill runs past `width - right` to the edge.
        let line = format!("  \x1b[48;2;0;0;0m{}\x1b[0m", " ".repeat(8));
        let err = background_rectangle(&line, &EXP).unwrap_err();
        assert!(err.contains("ends its fill"), "{err}");
    }

    #[test]
    fn background_rectangle_accepts_right_aligned_pill() {
        // A narrow right-aligned pill (fill 2) ending on the right boundary
        // (column width - right = 8) is chrome and must pass I2b.
        let pill = format!("      \x1b[48;2;9;9;9m{}\x1b[0m  ", " ".repeat(2));
        assert_eq!(bg_extent(&pill), Some((6, 2)));
        assert!(background_rectangle(&pill, &EXP).is_ok());
    }

    #[test]
    fn left_offset_uniformity_exempts_narrow_chrome() {
        // The same right-aligned pill does not span the content width, so I4
        // ignores its left edge.
        let pill = format!("      \x1b[48;2;9;9;9m{}\x1b[0m  ", " ".repeat(2));
        assert!(left_offset_uniformity(&pill, &EXP).is_ok());
    }

    #[test]
    fn right_boundary_coherence_flags_pill_body_mismatch() {
        // A "pill" ending at column 8 and a "body" ending at column 10.
        let pill = format!("  \x1b[48;2;1;1;1m{}\x1b[0m  ", " ".repeat(6));
        let body = format!("  \x1b[48;2;1;1;1m{}\x1b[0m", " ".repeat(8));
        let rendered = format!("{pill}\n{body}");
        assert!(right_boundary_coherence(&rendered, &EXP).is_err());
    }

    #[test]
    fn right_boundary_coherence_accepts_uniform_panel() {
        let rendered = format!("{}\n{}", good_bg_line(), good_bg_line());
        assert!(right_boundary_coherence(&rendered, &EXP).is_ok());
    }

    #[test]
    fn left_offset_uniformity_flags_full_bleed_offset_mismatch() {
        // A full-bleed row (fill 6 == content width) that opens at column 0
        // instead of left = 2.
        let shifted = format!("\x1b[48;2;0;0;0m{}\x1b[0m  ", " ".repeat(6));
        assert_eq!(bg_extent(&shifted), Some((0, 6)));
        assert!(left_offset_uniformity(&shifted, &EXP).is_err());
    }

    #[test]
    fn vertical_rhythm_flags_double_interior_blank() {
        let rendered = "alpha\n\n\nomega";
        assert!(vertical_rhythm(rendered, &EXP).is_err());
    }

    #[test]
    fn vertical_rhythm_allows_single_blank() {
        assert!(vertical_rhythm("alpha\n\nomega", &EXP).is_ok());
    }

    #[test]
    fn vertical_rhythm_ignores_padding_rows() {
        // A background-filled padding row is content, not a blank line.
        let pad = good_bg_line();
        let rendered = format!("alpha\n{pad}\nomega");
        assert!(vertical_rhythm(&rendered, &EXP).is_ok());
    }

    #[test]
    fn margin_blanks_counts_leading_and_trailing() {
        let exp = LayoutExpectation {
            top: 1,
            bottom: 2,
            ..EXP
        };
        let rendered = "\nbody\n\n";
        // lines(): ["", "body", ""] → leading 1, trailing 1 ≠ 2.
        assert!(margin_blanks(rendered, &exp).is_err());
        let ok = "\nbody\n\n\n"; // ["", "body", "", ""] → trailing 2.
        assert!(margin_blanks(ok, &exp).is_ok());
    }

    #[test]
    fn blank_line_idempotent_compares_outputs() {
        assert!(blank_line_idempotent("same", "same").is_ok());
        assert!(blank_line_idempotent("a", "b").is_err());
    }

    #[test]
    fn visible_width_counts_full_width_cjk_as_two_cells() {
        // "日本語" — three full-width Han/Hiragana glyphs, six cells in any
        // monospaced terminal. A naive `chars().count()` returns 3 and would
        // let I1 vacuously pass for a CJK line that actually wrapped.
        assert_eq!(visible_width("日本語"), 6);
    }

    #[test]
    fn visible_width_treats_combining_marks_as_zero_width() {
        // "e" + COMBINING ACUTE ACCENT (U+0301): one base + one zero-width
        // mark = a single cell.
        assert_eq!(visible_width("e\u{0301}"), 1);
    }

    #[test]
    fn visible_width_strips_ansi_before_measuring_cells() {
        // A styled CJK glyph still occupies two cells; the SGR escape itself
        // contributes nothing.
        assert_eq!(visible_width("\x1b[31m日\x1b[0m"), 2);
    }

    #[test]
    fn bg_extent_measures_full_width_glyphs_in_cells() {
        // 2 spaces of leading, then a backgrounded full-width glyph (2 cells),
        // then reset and trailing whitespace. The fill must be 2, not 1.
        let line = format!("  \x1b[48;2;0;0;0m日\x1b[0m  ");
        assert_eq!(bg_extent(&line), Some((2, 2)));
    }

    #[test]
    fn bg_extent_ignores_zero_width_combining_marks() {
        // Background spans "e" + combining acute = one cell total.
        let line = format!("  \x1b[48;2;0;0;0me\u{0301}\x1b[0m  ");
        assert_eq!(bg_extent(&line), Some((2, 1)));
    }

    #[test]
    fn containment_flags_cjk_overflow_chars_alone_miss() {
        // Three full-width glyphs = 6 cells, exceeding the 5-cell terminal
        // width. `chars().count()` would report 3 and falsely pass I1.
        let narrow = LayoutExpectation {
            width: 5,
            ..EXP
        };
        assert!(containment("日本語", &narrow).is_err());
    }
}
