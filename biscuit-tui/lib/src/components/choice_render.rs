//! Shared choice list rendering for [`ChooseOne`](super::choose_one::ChooseOne)
//! and [`ChooseMany`](super::choose_many::ChooseMany).
//!
//! Consolidates the duplicated `draw_list` logic from both components
//! into a single renderer driven by [`ChoiceRenderContext`].

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
};
use unicode_width::UnicodeWidthStr;

use crate::core::{ComponentTheme, FuzzyFilter, NerdFontStatus, TerminalStyle};
use super::choose::{ChoiceOption, HotkeyDisplayMode, Orientation};

/// Minimum label width (in cells) at which the fuzzy filter renders
/// per-character match highlighting. Narrower labels fall back to a
/// single plain span so the row stays readable.
const HIGHLIGHT_MIN_WIDTH: u16 = 12;

/// Shared rendering context for choice lists.
///
/// Holds the visual parameters that are stable across a single render
/// pass. Variable per-item state (hover, selection, filter) is passed
/// to [`ChoiceRenderContext::render`].
#[derive(Debug, Clone, Copy)]
pub struct ChoiceRenderContext<'a> {
    /// Visual theme (glyphs, colours, styles).
    pub theme: &'a ComponentTheme,
    /// Detected terminal capabilities (used in Phase 5 for Nerd Font
    /// glyph selection).
    pub terminal_style: TerminalStyle,
    /// Layout direction (vertical for Phase 4; horizontal in Phase 6).
    pub orientation: Orientation,
    /// When hotkey badges are shown next to option labels.
    pub hotkey_display: HotkeyDisplayMode,
    /// Glyph for a selected option.
    pub selected_indicator: &'a str,
    /// Glyph for an unselected option.
    pub unselected_indicator: &'a str,
}

/// Nerd Font selected radio glyph (filled circle).
const NF_RADIO_SELECTED: &str = "\u{f043e}";
/// Nerd Font unselected radio glyph (empty circle).
const NF_RADIO_UNSELECTED: &str = "\u{f4aa}";
/// Nerd Font selected checkbox glyph (checked box).
const NF_CHECK_SELECTED: &str = "\u{f14a}";
/// Nerd Font unselected checkbox glyph (empty box).
const NF_CHECK_UNSELECTED: &str = "\u{f0131}";

impl<'a> ChoiceRenderContext<'a> {
    /// Creates a context for single-selection (radio-style) indicators.
    ///
    /// Uses Nerd Font glyphs when [`TerminalStyle::nerd_font`] is
    /// [`NerdFontStatus::Likely`]; otherwise falls back to the theme's
    /// standard Unicode indicators.
    pub fn for_single(
        theme: &'a ComponentTheme,
        terminal_style: TerminalStyle,
        orientation: Orientation,
    ) -> Self {
        let (selected, unselected): (&str, &str) = match terminal_style.nerd_font {
            NerdFontStatus::Likely => (NF_RADIO_SELECTED, NF_RADIO_UNSELECTED),
            NerdFontStatus::Unknown => (
                theme.selected_indicator.as_str(),
                theme.unselected_indicator.as_str(),
            ),
        };
        Self {
            theme,
            terminal_style,
            orientation,
            hotkey_display: HotkeyDisplayMode::Hidden,
            selected_indicator: selected,
            unselected_indicator: unselected,
        }
    }

    /// Creates a context for multi-selection (checkbox-style) indicators.
    ///
    /// Uses Nerd Font glyphs when [`TerminalStyle::nerd_font`] is
    /// [`NerdFontStatus::Likely`]; otherwise falls back to standard
    /// Unicode checkbox glyphs.
    pub fn for_multiple(
        theme: &'a ComponentTheme,
        terminal_style: TerminalStyle,
        orientation: Orientation,
    ) -> Self {
        let (selected, unselected) = match terminal_style.nerd_font {
            NerdFontStatus::Likely => (NF_CHECK_SELECTED, NF_CHECK_UNSELECTED),
            NerdFontStatus::Unknown => ("☑", "☐"),
        };
        Self {
            theme,
            terminal_style,
            orientation,
            hotkey_display: HotkeyDisplayMode::Hidden,
            selected_indicator: selected,
            unselected_indicator: unselected,
        }
    }

    /// Renders a choice list into `buf` within `area`.
    ///
    /// `is_selected` is called for every visible option index to
    /// determine whether the option is currently selected.
    #[allow(clippy::too_many_arguments)]
    pub fn render<V, F>(
        &self,
        area: Rect,
        buf: &mut Buffer,
        options: &[ChoiceOption<V>],
        visible_indices: &[usize],
        scroll_offset: usize,
        hover: usize,
        is_selected: F,
        mut filter: Option<&mut FuzzyFilter>,
        validation_error: Option<&str>,
    ) where
        V: Clone + PartialEq,
        F: Fn(usize) -> bool,
    {
        if area.width == 0 || area.height == 0 || options.is_empty() {
            return;
        }

        // Reserve one row for a validation error when present.
        let body_rows = if validation_error.is_some() && area.height > 1 {
            area.height - 1
        } else {
            area.height
        };
        let visible_count = body_rows as usize;

        // When filter is active and matches nothing, show a single dim
        // "(no matches)" row instead of the list body.
        if let Some(ref f) = filter
            && f.is_active()
            && visible_indices.is_empty()
        {
            let no_matches = Line::from(Span::styled(
                self.theme.no_matches_text.clone(),
                self.theme.no_matches_style,
            ));
            buf.set_line(area.x, area.y, &no_matches, area.width);
            return;
        }

        let hover_style = self.theme.selected_style;
        let disabled_style = self.theme.disabled_style;
        let match_style = self.theme.search_match_style;
        let filter_active = filter.as_ref().map(|f| f.is_active()).unwrap_or(false);

        // Compute focus prefix width: indicator + 1 space, or collapse
        // to single space if empty/whitespace.
        let focus_prefix_width = if self.theme.focus_indicator.trim().is_empty() {
            1
        } else {
            self.theme.focus_indicator.width() + 1
        };

        for (row, &idx) in visible_indices
            .iter()
            .skip(scroll_offset)
            .take(visible_count)
            .enumerate()
        {
            let option = &options[idx];
            let option_disabled = option.disabled;
            let option_label = &option.label;
            let is_hovered = idx == hover;
            let is_sel = is_selected(idx);

            // Determine indicator glyph.
            let indicator = if is_sel {
                self.selected_indicator
            } else {
                self.unselected_indicator
            };

            // Focus indicator prefix on hovered row, blank padding otherwise.
            // In vertical mode the triangular pointer is preserved;
            // horizontal mode (Phase 6) will omit it.
            let focus_prefix = if is_hovered {
                if self.theme.focus_indicator.trim().is_empty() {
                    " ".to_string()
                } else {
                    format!("{} ", self.theme.focus_indicator)
                }
            } else {
                " ".repeat(focus_prefix_width)
            };

            let prefix = format!("{focus_prefix}{indicator} ");

            // Determine label style.
            let label_style = if option_disabled {
                disabled_style
            } else if is_hovered {
                hover_style
            } else if is_sel {
                self.theme.selected_label_style
            } else {
                Style::default()
            };

            // Build spans.  When hovered we apply the hover background
            // to the prefix as well so the active highlight spans the
            // whole item content.
            let mut spans: Vec<Span<'static>> = Vec::new();

            let prefix_style = if is_hovered && !option_disabled {
                hover_style
            } else {
                Style::default()
            };
            spans.push(Span::styled(prefix.clone(), prefix_style));

            if filter_active && area.width >= HIGHLIGHT_MIN_WIDTH {
                if let Some(ref mut f) = filter {
                    let highlights = f.highlight_indices(option_label);
                    spans.extend(build_highlighted_spans(
                        option_label,
                        &highlights,
                        label_style,
                        match_style,
                    ));
                } else {
                    spans.push(Span::styled(option_label.to_string(), label_style));
                }
            } else {
                spans.push(Span::styled(option_label.to_string(), label_style));
            }

            // Render active background over the visible item width plus
            // one blank cell.
            if is_hovered && !option_disabled {
                spans.push(Span::styled(" ", hover_style));
            }

            let line = Line::from(spans);
            let y = area.y + row as u16;
            buf.set_line(area.x, y, &line, area.width);
        }

        // Paint overflow indicators at top-right / bottom-right when scrollable.
        let overflow_style = hover_style.add_modifier(ratatui::style::Modifier::DIM);

        if scroll_offset > 0 && area.width > 0 {
            let x = area.x + area.width - 1;
            let y = area.y;
            buf[(x, y)]
                .set_symbol(&self.theme.overflow_up_indicator)
                .set_style(overflow_style);
        }

        if scroll_offset + visible_count < visible_indices.len()
            && area.width > 0
            && visible_count > 0
        {
            let x = area.x + area.width - 1;
            let y = area.y + (visible_count - 1) as u16;
            buf[(x, y)]
                .set_symbol(&self.theme.overflow_down_indicator)
                .set_style(overflow_style);
        }
    }

}

/// Splits `label` into `Span`s that highlight char-indexed matches
/// with `match_style` and renders the remaining text with
/// `base_style`. `highlights` must be sorted-ascending char offsets.
pub fn build_highlighted_spans(
    label: &str,
    highlights: &[u32],
    base_style: Style,
    match_style: Style,
) -> Vec<Span<'static>> {
    if highlights.is_empty() {
        return vec![Span::styled(label.to_string(), base_style)];
    }
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut current = String::new();
    let mut current_is_match = false;
    for (char_idx, ch) in label.chars().enumerate() {
        let is_match = highlights.binary_search(&(char_idx as u32)).is_ok();
        if current.is_empty() {
            current_is_match = is_match;
            current.push(ch);
            continue;
        }
        if is_match == current_is_match {
            current.push(ch);
        } else {
            let style = if current_is_match {
                match_style
            } else {
                base_style
            };
            spans.push(Span::styled(std::mem::take(&mut current), style));
            current_is_match = is_match;
            current.push(ch);
        }
    }
    if !current.is_empty() {
        let style = if current_is_match {
            match_style
        } else {
            base_style
        };
        spans.push(Span::styled(current, style));
    }
    spans
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::{buffer::Buffer, layout::Rect, style::{Color, Modifier}};

    fn default_theme() -> ComponentTheme {
        ComponentTheme::default()
    }

    fn buffer_row(buf: &Buffer, y: u16) -> String {
        let mut row = String::new();
        for x in buf.area.left()..buf.area.right() {
            row.push_str(buf[(x, y)].symbol());
        }
        row.trim_end().to_string()
    }

    #[test]
    fn render_vertical_draws_indicator_and_label() {
        let theme = default_theme();
        let ctx = ChoiceRenderContext::for_single(
            &theme,
            TerminalStyle::default(),
            Orientation::Vertical,
        );
        let options = vec![
            ChoiceOption::<String>::new("r", "Red", "red"),
            ChoiceOption::<String>::new("g", "Green", "green"),
        ];
        let area = Rect::new(0, 0, 20, 2);
        let mut buf = Buffer::empty(area);

        ctx.render(
            area,
            &mut buf,
            &options,
            &[0, 1],
            0,
            0,
            |_idx| false,
            None,
            None,
        );

        assert!(buffer_row(&buf, 0).starts_with("▶ ○ Red"));
        assert!(buffer_row(&buf, 1).starts_with("  ○ Green"));
    }

    #[test]
    fn render_vertical_selected_indicator() {
        let theme = default_theme();
        let ctx = ChoiceRenderContext::for_single(
            &theme,
            TerminalStyle::default(),
            Orientation::Vertical,
        );
        let options = vec![
            ChoiceOption::<String>::new("r", "Red", "red"),
            ChoiceOption::<String>::new("g", "Green", "green"),
        ];
        let area = Rect::new(0, 0, 20, 2);
        let mut buf = Buffer::empty(area);

        ctx.render(
            area,
            &mut buf,
            &options,
            &[0, 1],
            0,
            1,
            |idx| idx == 0,
            None,
            None,
        );

        assert!(buffer_row(&buf, 0).starts_with("  ● Red"));
        assert!(buffer_row(&buf, 1).starts_with("▶ ○ Green"));
    }

    #[test]
    fn render_multiple_checkbox_indicators() {
        let theme = default_theme();
        let ctx = ChoiceRenderContext::for_multiple(
            &theme,
            TerminalStyle::default(),
            Orientation::Vertical,
        );
        let options = vec![
            ChoiceOption::<String>::new("a", "Alpha", "alpha"),
            ChoiceOption::<String>::new("b", "Beta", "beta"),
        ];
        let area = Rect::new(0, 0, 20, 2);
        let mut buf = Buffer::empty(area);

        ctx.render(
            area,
            &mut buf,
            &options,
            &[0, 1],
            0,
            0,
            |idx| idx == 0,
            None,
            None,
        );

        assert!(buffer_row(&buf, 0).starts_with("▶ ☑ Alpha"));
        assert!(buffer_row(&buf, 1).starts_with("  ☐ Beta"));
    }

    #[test]
    fn render_hover_background_covers_prefix_and_label_plus_one() {
        let theme = default_theme();
        let ctx = ChoiceRenderContext::for_single(
            &theme,
            TerminalStyle::default(),
            Orientation::Vertical,
        );
        let options = vec![ChoiceOption::<String>::new("r", "Red", "red")];
        let area = Rect::new(0, 0, 20, 1);
        let mut buf = Buffer::empty(area);

        ctx.render(
            area,
            &mut buf,
            &options,
            &[0],
            0,
            0,
            |_idx| false,
            None,
            None,
        );

        let hover_style = theme.selected_style;
        // Prefix "▶ ○ " = 4 cells, label "Red" = 3 cells, trailing " " = 1 cell = 8 cells total.
        for x in 0..8 {
            let style = buf[(x, 0)].style();
            assert_eq!(style.fg, hover_style.fg, "cell {x} should have correct fg");
            assert_eq!(style.bg, hover_style.bg, "cell {x} should have correct bg");
            assert!(
                style.add_modifier.contains(Modifier::BOLD),
                "cell {x} should have BOLD modifier"
            );
        }
        // Everything beyond the content should keep default style.
        let style = buf[(9, 0)].style();
        assert!(
            style.fg != hover_style.fg || style.bg != hover_style.bg,
            "cell 9 should not have hover background"
        );
    }

    #[test]
    fn render_disabled_row_uses_disabled_style() {
        let theme = default_theme();
        let ctx = ChoiceRenderContext::for_single(
            &theme,
            TerminalStyle::default(),
            Orientation::Vertical,
        );
        let mut options = vec![
            ChoiceOption::<String>::new("a", "Active", "active"),
            ChoiceOption::<String>::new("d", "Disabled", "disabled"),
        ];
        options[1].disabled = true;
        let area = Rect::new(0, 0, 20, 2);
        let mut buf = Buffer::empty(area);

        ctx.render(
            area,
            &mut buf,
            &options,
            &[0, 1],
            0,
            1,
            |_idx| false,
            None,
            None,
        );

        // Find the 'D' in "Disabled" and verify it has disabled style.
        let mut found = false;
        for x in 0..area.width {
            let cell = &buf[(x, 1)];
            if cell.symbol() == "D" {
                assert!(
                    cell.style().fg == Some(Color::DarkGray)
                        || cell.style().add_modifier.contains(Modifier::DIM),
                    "Disabled label should have disabled style"
                );
                found = true;
                break;
            }
        }
        assert!(found, "Did not find 'D' in buffer");
    }

    #[test]
    fn render_overflow_indicators_when_scrolled() {
        let theme = default_theme();
        let ctx = ChoiceRenderContext::for_single(
            &theme,
            TerminalStyle::default(),
            Orientation::Vertical,
        );
        let options: Vec<ChoiceOption> = (0..5)
            .map(|i| ChoiceOption::<String>::new(format!("id{i}"), format!("Option {i}"), format!("val{i}")))
            .collect();
        let area = Rect::new(0, 0, 20, 2);
        let mut buf = Buffer::empty(area);

        ctx.render(
            area,
            &mut buf,
            &options,
            &[0, 1, 2, 3, 4],
            1, // scrolled down by 1
            2,
            |_idx| false,
            None,
            None,
        );

        let top_row = buffer_row(&buf, 0);
        assert!(top_row.contains("▲"), "Expected up overflow indicator");

        let bottom_row = buffer_row(&buf, 1);
        assert!(bottom_row.contains("▼"), "Expected down overflow indicator");
    }

    #[test]
    fn render_no_matches_when_filter_empty() {
        let theme = default_theme();
        let ctx = ChoiceRenderContext::for_single(
            &theme,
            TerminalStyle::default(),
            Orientation::Vertical,
        );
        let options = vec![
            ChoiceOption::<String>::new("a", "Apple", "apple"),
            ChoiceOption::<String>::new("b", "Banana", "banana"),
        ];
        let area = Rect::new(0, 0, 20, 3);
        let mut buf = Buffer::empty(area);

        let mut filter = FuzzyFilter::new();
        filter.clear(&["Apple".into(), "Banana".into()]);
        filter.push_char('z', &["Apple".into(), "Banana".into()]);
        let visible_indices: Vec<usize> = filter.visible().to_vec();

        ctx.render(
            area,
            &mut buf,
            &options,
            &visible_indices,
            0,
            0,
            |_idx| false,
            Some(&mut filter),
            None,
        );

        assert_eq!(buffer_row(&buf, 0), "(no matches)");
    }

    #[test]
    fn build_highlighted_spans_empty_highlights() {
        let base = Style::default().fg(Color::White);
        let match_style = Style::default().fg(Color::Yellow);
        let spans = build_highlighted_spans("hello", &[], base, match_style);
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].content, "hello");
        assert_eq!(spans[0].style, base);
    }

    #[test]
    fn build_highlighted_spans_multibyte_chars() {
        let base = Style::default().fg(Color::White);
        let match_style = Style::default().fg(Color::Yellow);
        let spans = build_highlighted_spans("Café", &[0, 1], base, match_style);
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].content, "Ca");
        assert_eq!(spans[0].style, match_style);
        assert_eq!(spans[1].content, "fé");
        assert_eq!(spans[1].style, base);
    }

    #[test]
    fn render_single_uses_fallback_glyphs_when_no_nerd_font() {
        let theme = default_theme();
        let ctx = ChoiceRenderContext::for_single(
            &theme,
            TerminalStyle {
                nerd_font: NerdFontStatus::Unknown,
                ..TerminalStyle::default()
            },
            Orientation::Vertical,
        );
        let options = vec![ChoiceOption::<String>::new("r", "Red", "red")];
        let area = Rect::new(0, 0, 20, 1);
        let mut buf = Buffer::empty(area);

        ctx.render(
            area,
            &mut buf,
            &options,
            &[0],
            0,
            0,
            |_idx| true,
            None,
            None,
        );

        let row = buffer_row(&buf, 0);
        assert!(row.contains("●"), "expected fallback filled radio ●, got: {row}");
    }

    #[test]
    fn render_single_uses_nerd_font_glyphs_when_likely() {
        let theme = default_theme();
        let ctx = ChoiceRenderContext::for_single(
            &theme,
            TerminalStyle {
                nerd_font: NerdFontStatus::Likely,
                ..TerminalStyle::default()
            },
            Orientation::Vertical,
        );
        let options = vec![ChoiceOption::<String>::new("r", "Red", "red")];
        let area = Rect::new(0, 0, 20, 1);
        let mut buf = Buffer::empty(area);

        ctx.render(
            area,
            &mut buf,
            &options,
            &[0],
            0,
            0,
            |_idx| true,
            None,
            None,
        );

        let row = buffer_row(&buf, 0);
        assert!(
            row.contains('\u{f043e}'),
            "expected Nerd Font filled radio \\u{{f043e}}, got: {row}"
        );
    }

    #[test]
    fn render_multiple_uses_fallback_glyphs_when_no_nerd_font() {
        let theme = default_theme();
        let ctx = ChoiceRenderContext::for_multiple(
            &theme,
            TerminalStyle {
                nerd_font: NerdFontStatus::Unknown,
                ..TerminalStyle::default()
            },
            Orientation::Vertical,
        );
        let options = vec![ChoiceOption::<String>::new("a", "Alpha", "alpha")];
        let area = Rect::new(0, 0, 20, 1);
        let mut buf = Buffer::empty(area);

        ctx.render(
            area,
            &mut buf,
            &options,
            &[0],
            0,
            0,
            |_idx| true,
            None,
            None,
        );

        let row = buffer_row(&buf, 0);
        assert!(row.contains("☑"), "expected fallback checked box ☑, got: {row}");
    }

    #[test]
    fn render_multiple_uses_nerd_font_glyphs_when_likely() {
        let theme = default_theme();
        let ctx = ChoiceRenderContext::for_multiple(
            &theme,
            TerminalStyle {
                nerd_font: NerdFontStatus::Likely,
                ..TerminalStyle::default()
            },
            Orientation::Vertical,
        );
        let options = vec![ChoiceOption::<String>::new("a", "Alpha", "alpha")];
        let area = Rect::new(0, 0, 20, 1);
        let mut buf = Buffer::empty(area);

        ctx.render(
            area,
            &mut buf,
            &options,
            &[0],
            0,
            0,
            |_idx| true,
            None,
            None,
        );

        let row = buffer_row(&buf, 0);
        assert!(
            row.contains('\u{f14a}'),
            "expected Nerd Font checked box \\u{{f14a}}, got: {row}"
        );
    }
}
