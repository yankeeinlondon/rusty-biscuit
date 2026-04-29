//! Shared choice list rendering for [`ChooseOne`](super::choose_one::ChooseOne)
//! and [`ChooseMany`](super::choose_many::ChooseMany).
//!
//! Consolidates the duplicated `draw_list` logic from both components
//! into a single renderer driven by [`ChoiceRenderContext`].

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
use unicode_width::UnicodeWidthStr;

use super::choose::{ActiveChoiceColor, ChoiceOption, HotkeyDisplayMode, HotkeySpec, Orientation};
use crate::core::{
    ComponentTheme, FuzzyFilter, NerdFontStatus, TerminalStyle, resolve_active_style,
};

/// 256-color palette index for the Ctrl badge background (orange tone
/// that reads on both dark and light terminals).
const CTRL_BADGE_BG: Color = Color::Indexed(208);
/// 256-color palette index for the Alt badge background (yellow tone).
const ALT_BADGE_BG: Color = Color::Indexed(220);
/// Foreground colour for hotkey badge text.
const BADGE_FG: Color = Color::White;

/// Builds a renderable string for a hotkey badge.
///
/// For example, `HotkeySpec::Ctrl('r')` renders as `^R`. The compact
/// `^X` / `⌥X` glyphs were chosen so badges fit inside narrow rows
/// without breaking option alignment.
fn badge_text(hotkey: HotkeySpec) -> String {
    match hotkey {
        HotkeySpec::Ctrl(c) => format!("^{}", c.to_ascii_uppercase()),
        HotkeySpec::Alt(c) => format!("⌥{}", c.to_ascii_uppercase()),
    }
}

/// Returns the rendered width of a hotkey badge in cells, or `0` when
/// `hotkey` is `None` or the display mode hides the badge.
fn badge_width(hotkey: Option<HotkeySpec>, display: HotkeyDisplayMode) -> u16 {
    if display == HotkeyDisplayMode::Hidden {
        return 0;
    }
    match hotkey {
        Some(spec) => badge_text(spec).width() as u16,
        None => 0,
    }
}

/// Builds a styled span for a hotkey badge.
///
/// `display` selects which modifier is currently "held": Ctrl badges
/// are rendered bold + bright when `display == CtrlHeld` and dim
/// otherwise; the inverse applies for Alt badges. When `display` is
/// `Hidden` no badge is produced.
fn badge_span(hotkey: HotkeySpec, display: HotkeyDisplayMode) -> Option<Span<'static>> {
    if display == HotkeyDisplayMode::Hidden {
        return None;
    }
    let (bg, this_held) = match hotkey {
        HotkeySpec::Ctrl(_) => (CTRL_BADGE_BG, display == HotkeyDisplayMode::CtrlHeld),
        HotkeySpec::Alt(_) => (ALT_BADGE_BG, display == HotkeyDisplayMode::AltHeld),
    };
    let modifier = if this_held {
        Modifier::BOLD
    } else {
        Modifier::DIM
    };
    let style = Style::default().fg(BADGE_FG).bg(bg).add_modifier(modifier);
    Some(Span::styled(badge_text(hotkey), style))
}

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
    /// Background colour for the actively hovered option (resolved
    /// against the detected [`TerminalBackground`] by
    /// [`resolve_active_style`]).
    pub active_color: ActiveChoiceColor,
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
            active_color: ActiveChoiceColor::default(),
            selected_indicator: selected,
            unselected_indicator: unselected,
        }
    }

    /// Overrides the [`ActiveChoiceColor`] used for the active row.
    pub fn with_active_color(mut self, color: ActiveChoiceColor) -> Self {
        self.active_color = color;
        self
    }

    /// Overrides the [`HotkeyDisplayMode`] used by the renderer.
    ///
    /// When non-`Hidden` and an option carries a matching
    /// [`HotkeySpec`](super::choose::HotkeySpec), a coloured badge is
    /// rendered next to the label. Vertical layouts place the badge
    /// inline (immediately after the trailing blank); horizontal
    /// layouts drop it on the row directly below the option.
    pub fn with_hotkey_display(mut self, mode: HotkeyDisplayMode) -> Self {
        self.hotkey_display = mode;
        self
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
            active_color: ActiveChoiceColor::default(),
            selected_indicator: selected,
            unselected_indicator: unselected,
        }
    }

    /// Computes the layout for the given visible options.
    ///
    /// The layout is used by the renderer to place items and by
    /// event handlers for row-based navigation in horizontal mode.
    pub fn compute_layout<V, F>(
        &self,
        area: Rect,
        options: &[ChoiceOption<V>],
        visible_indices: &[usize],
        _is_selected: F,
    ) -> super::choice_layout::ChoiceLayout
    where
        V: Clone + PartialEq,
        F: Fn(usize) -> bool,
    {
        use super::choice_layout::ChoiceLayout;
        match self.orientation {
            Orientation::Vertical => ChoiceLayout::vertical(visible_indices, area),
            Orientation::Horizontal => {
                let indicator_width = std::cmp::max(
                    self.selected_indicator.width(),
                    self.unselected_indicator.width(),
                );
                let widths: Vec<u16> = visible_indices
                    .iter()
                    .map(|&idx| {
                        let label_width = options[idx].label.width();
                        let badge_w = badge_width(options[idx].hotkey, self.hotkey_display);
                        let badge_pad = if badge_w > 0 { 1 } else { 0 };
                        (indicator_width + 1 + label_width + 1) as u16 + badge_pad + badge_w
                    })
                    .collect();
                let row_height = if self.hotkey_display == HotkeyDisplayMode::Hidden {
                    1
                } else {
                    2
                };
                ChoiceLayout::horizontal(visible_indices, &widths, area, row_height)
            }
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
        filter: Option<&mut FuzzyFilter>,
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

        match self.orientation {
            Orientation::Vertical => self.render_vertical(
                area,
                buf,
                options,
                visible_indices,
                scroll_offset,
                hover,
                is_selected,
                filter,
                visible_count,
            ),
            Orientation::Horizontal => self.render_horizontal(
                area,
                buf,
                options,
                visible_indices,
                scroll_offset,
                hover,
                is_selected,
                filter,
                visible_count,
            ),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn render_vertical<V, F>(
        &self,
        area: Rect,
        buf: &mut Buffer,
        options: &[ChoiceOption<V>],
        visible_indices: &[usize],
        scroll_offset: usize,
        hover: usize,
        is_selected: F,
        mut filter: Option<&mut FuzzyFilter>,
        visible_count: usize,
    ) where
        V: Clone + PartialEq,
        F: Fn(usize) -> bool,
    {
        let hover_style = resolve_active_style(self.active_color, self.terminal_style.background);
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

            // Inline hotkey badge in vertical mode (Phase 6). Drawn
            // after the trailing blank so it never bleeds into the
            // active highlight.
            if let Some(spec) = option.hotkey
                && let Some(badge) = badge_span(spec, self.hotkey_display)
            {
                // Add a separating space so the badge does not abut the
                // hover background.
                spans.push(Span::raw(" "));
                spans.push(badge);
            }

            let line = Line::from(spans);
            let y = area.y + row as u16;
            buf.set_line(area.x, y, &line, area.width);
        }

        // Paint overflow indicators at top-right / bottom-right when scrollable.
        let overflow_style = hover_style.add_modifier(Modifier::DIM);

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

    #[allow(clippy::too_many_arguments)]
    fn render_horizontal<V, F>(
        &self,
        area: Rect,
        buf: &mut Buffer,
        options: &[ChoiceOption<V>],
        visible_indices: &[usize],
        scroll_offset: usize,
        hover: usize,
        is_selected: F,
        mut filter: Option<&mut FuzzyFilter>,
        visible_count: usize,
    ) where
        V: Clone + PartialEq,
        F: Fn(usize) -> bool,
    {
        use super::choice_layout::ChoiceLayout;

        let hover_style = resolve_active_style(self.active_color, self.terminal_style.background);
        let disabled_style = self.theme.disabled_style;
        let match_style = self.theme.search_match_style;
        let filter_active = filter.as_ref().map(|f| f.is_active()).unwrap_or(false);

        let indicator_width = std::cmp::max(
            self.selected_indicator.width(),
            self.unselected_indicator.width(),
        );
        let widths: Vec<u16> = visible_indices
            .iter()
            .map(|&idx| {
                let label_width = options[idx].label.width();
                let badge_w = badge_width(options[idx].hotkey, self.hotkey_display);
                let badge_pad = if badge_w > 0 { 1 } else { 0 };
                (indicator_width + 1 + label_width + 1) as u16 + badge_pad + badge_w
            })
            .collect();
        let row_height: u16 = if self.hotkey_display == HotkeyDisplayMode::Hidden {
            1
        } else {
            2
        };
        let layout = ChoiceLayout::horizontal(visible_indices, &widths, area, row_height);

        // scroll_offset is row offset in horizontal mode
        let start_row = scroll_offset.min(layout.row_count());
        let end_row = (start_row + visible_count).min(layout.row_count());
        let shown_rows = end_row.saturating_sub(start_row);

        for row_idx in start_row..end_row {
            let (range_start, range_end) = layout.row_ranges[row_idx];
            for item_idx in range_start..=range_end {
                let rect = &layout.item_rects[item_idx];
                let idx = rect.option_index;
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

                // No triangular pointer in horizontal mode.
                let prefix = format!("{indicator} ");

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
                let screen_y = area.y + (row_idx - start_row) as u16 * row_height;
                buf.set_line(rect.x, screen_y, &line, rect.width);

                // Horizontal layout: render the badge in a sub-row
                // immediately below the option (Phase 6). We only paint
                // the badge if there is a screen row available below
                // the current one, *and* it does not collide with the
                // next option row in the same render area. Navigation
                // uses option-major rows from `layout`, so the badge
                // sub-row is invisible to row navigation logic.
                if let Some(spec) = option.hotkey
                    && let Some(badge) = badge_span(spec, self.hotkey_display)
                    && screen_y + 1 < area.y + area.height
                {
                    let badge_line = Line::from(vec![badge]);
                    buf.set_line(rect.x, screen_y + 1, &badge_line, rect.width);
                }
            }
        }

        // Paint overflow indicators at top-right / bottom-right when scrollable.
        let overflow_style = hover_style.add_modifier(Modifier::DIM);

        if start_row > 0 && area.width > 0 {
            let x = area.x + area.width - 1;
            let y = area.y;
            buf[(x, y)]
                .set_symbol(&self.theme.overflow_up_indicator)
                .set_style(overflow_style);
        }

        if end_row < layout.row_count() && area.width > 0 && shown_rows > 0 {
            let x = area.x + area.width - 1;
            let y = area.y + (shown_rows - 1) as u16;
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
    use crate::core::TerminalBackground;
    use ratatui::{
        buffer::Buffer,
        layout::Rect,
        style::{Color, Modifier},
    };

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

        let hover_style =
            resolve_active_style(ActiveChoiceColor::default(), TerminalBackground::default());
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

    /// Renders a single-row choice list at the given background and
    /// returns the resulting buffer for assertion.
    fn render_single_active_row(
        background: TerminalBackground,
        active_color: ActiveChoiceColor,
    ) -> Buffer {
        let theme = default_theme();
        let term = TerminalStyle {
            background,
            ..TerminalStyle::default()
        };
        let ctx = ChoiceRenderContext::for_single(&theme, term, Orientation::Vertical)
            .with_active_color(active_color);
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
        buf
    }

    #[test]
    fn active_row_uses_grey_default_palette_on_dark_background() {
        let buf = render_single_active_row(TerminalBackground::Dark, ActiveChoiceColor::Grey);
        // Index 1 lands on the focus indicator's trailing space — part
        // of the styled prefix that should carry the active background.
        assert_eq!(buf[(1, 0)].style().bg, Some(Color::Indexed(238)));
    }

    #[test]
    fn active_row_uses_green_palette_when_configured() {
        let buf = render_single_active_row(TerminalBackground::Dark, ActiveChoiceColor::Green);
        assert_eq!(buf[(1, 0)].style().bg, Some(Color::Indexed(22)));
    }

    #[test]
    fn active_row_uses_white_fg_on_dark_bg() {
        let buf = render_single_active_row(TerminalBackground::Dark, ActiveChoiceColor::Grey);
        assert_eq!(buf[(1, 0)].style().fg, Some(Color::White));
    }

    #[test]
    fn active_row_uses_black_fg_on_light_bg() {
        let buf = render_single_active_row(TerminalBackground::Light, ActiveChoiceColor::Grey);
        assert_eq!(buf[(1, 0)].style().fg, Some(Color::Black));
        assert_eq!(buf[(1, 0)].style().bg, Some(Color::Indexed(252)));
    }

    #[test]
    fn active_row_uses_dark_palette_on_unknown_background() {
        let unknown = render_single_active_row(TerminalBackground::Unknown, ActiveChoiceColor::Red);
        let dark = render_single_active_row(TerminalBackground::Dark, ActiveChoiceColor::Red);
        assert_eq!(unknown[(1, 0)].style().bg, dark[(1, 0)].style().bg);
        assert_eq!(unknown[(1, 0)].style().fg, dark[(1, 0)].style().fg);
        assert_eq!(unknown[(1, 0)].style().bg, Some(Color::Indexed(52)));
    }

    #[test]
    fn active_row_style_covers_only_label_plus_one_blank() {
        // Prefix "▶ ○ " = 4 cells, label "Red" = 3 cells, trailing
        // blank = 1 cell. So cells 0..8 must carry the active style and
        // cell 8 onwards must be unstyled. (Cell index 8 is the first
        // unstyled cell; cell 9 is "well past" the styled span.)
        let buf = render_single_active_row(TerminalBackground::Dark, ActiveChoiceColor::Grey);
        let active_bg = Some(Color::Indexed(238));
        for x in 0..8 {
            assert_eq!(
                buf[(x, 0)].style().bg,
                active_bg,
                "cell {x} should carry the active background"
            );
        }
        for x in 8..20 {
            assert_ne!(
                buf[(x, 0)].style().bg,
                active_bg,
                "cell {x} must NOT carry the active background"
            );
        }
    }

    #[test]
    fn active_row_style_does_not_underline() {
        let buf = render_single_active_row(TerminalBackground::Dark, ActiveChoiceColor::Grey);
        let modifier = buf[(1, 0)].style().add_modifier;
        assert!(modifier.contains(Modifier::BOLD));
        assert!(!modifier.contains(Modifier::UNDERLINED));
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
            .map(|i| {
                ChoiceOption::<String>::new(
                    format!("id{i}"),
                    format!("Option {i}"),
                    format!("val{i}"),
                )
            })
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
        assert!(
            row.contains("●"),
            "expected fallback filled radio ●, got: {row}"
        );
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
        assert!(
            row.contains("☑"),
            "expected fallback checked box ☑, got: {row}"
        );
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

    // --- Phase 6: hotkey badges ----------------------------------------

    /// Returns the column at which the badge for option `idx` first
    /// appears in `buf` row `y`, or `None` when no Ctrl/Alt badge
    /// background colour is detected.
    fn find_badge_x(buf: &Buffer, y: u16, expected_bg: Color) -> Option<u16> {
        (0..buf.area.width).find(|&x| buf[(x, y)].style().bg == Some(expected_bg))
    }

    fn fixture_with_hotkeys() -> Vec<ChoiceOption<String>> {
        vec![
            ChoiceOption::<String>::new("r", "Red", "red")
                .with_hotkey(super::super::choose::HotkeySpec::Ctrl('r')),
            ChoiceOption::<String>::new("g", "Green", "green")
                .with_hotkey(super::super::choose::HotkeySpec::Alt('g')),
            ChoiceOption::<String>::new("b", "Blue", "blue"),
        ]
    }

    #[test]
    fn vertical_render_draws_ctrl_badge_with_orange_bg_and_bold_text() {
        let theme = default_theme();
        let ctx = ChoiceRenderContext::for_single(
            &theme,
            TerminalStyle::default(),
            Orientation::Vertical,
        )
        .with_hotkey_display(HotkeyDisplayMode::CtrlHeld);
        let options = fixture_with_hotkeys();
        let area = Rect::new(0, 0, 30, 3);
        let mut buf = Buffer::empty(area);
        ctx.render(
            area,
            &mut buf,
            &options,
            &[0, 1, 2],
            0,
            0,
            |_idx| false,
            None,
            None,
        );

        // Row 0 (Red) carries a Ctrl badge in orange (208).
        let badge_x =
            find_badge_x(&buf, 0, CTRL_BADGE_BG).expect("expected Ctrl badge background on row 0");
        let style = buf[(badge_x, 0)].style();
        assert_eq!(style.fg, Some(BADGE_FG));
        assert_eq!(style.bg, Some(CTRL_BADGE_BG));
        assert!(style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn vertical_render_draws_alt_badge_with_yellow_bg_and_bold_text() {
        let theme = default_theme();
        let ctx = ChoiceRenderContext::for_single(
            &theme,
            TerminalStyle::default(),
            Orientation::Vertical,
        )
        .with_hotkey_display(HotkeyDisplayMode::AltHeld);
        let options = fixture_with_hotkeys();
        let area = Rect::new(0, 0, 30, 3);
        let mut buf = Buffer::empty(area);
        ctx.render(
            area,
            &mut buf,
            &options,
            &[0, 1, 2],
            0,
            0,
            |_idx| false,
            None,
            None,
        );

        // Row 1 (Green) carries an Alt badge in yellow (220).
        let badge_x =
            find_badge_x(&buf, 1, ALT_BADGE_BG).expect("expected Alt badge background on row 1");
        let style = buf[(badge_x, 1)].style();
        assert_eq!(style.fg, Some(BADGE_FG));
        assert_eq!(style.bg, Some(ALT_BADGE_BG));
        assert!(style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn vertical_render_when_ctrl_held_renders_alt_badges_dim() {
        let theme = default_theme();
        let ctx = ChoiceRenderContext::for_single(
            &theme,
            TerminalStyle::default(),
            Orientation::Vertical,
        )
        .with_hotkey_display(HotkeyDisplayMode::CtrlHeld);
        let options = fixture_with_hotkeys();
        let area = Rect::new(0, 0, 30, 3);
        let mut buf = Buffer::empty(area);
        ctx.render(
            area,
            &mut buf,
            &options,
            &[0, 1, 2],
            0,
            0,
            |_idx| false,
            None,
            None,
        );

        // Row 1 (Green) still shows its Alt badge — but dim, not bold.
        let badge_x = find_badge_x(&buf, 1, ALT_BADGE_BG)
            .expect("expected Alt badge to still render under CtrlHeld");
        let style = buf[(badge_x, 1)].style();
        assert!(style.add_modifier.contains(Modifier::DIM));
        assert!(!style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn vertical_render_hidden_mode_omits_badges() {
        let theme = default_theme();
        let ctx = ChoiceRenderContext::for_single(
            &theme,
            TerminalStyle::default(),
            Orientation::Vertical,
        )
        .with_hotkey_display(HotkeyDisplayMode::Hidden);
        let options = fixture_with_hotkeys();
        let area = Rect::new(0, 0, 30, 3);
        let mut buf = Buffer::empty(area);
        ctx.render(
            area,
            &mut buf,
            &options,
            &[0, 1, 2],
            0,
            0,
            |_idx| false,
            None,
            None,
        );

        assert!(find_badge_x(&buf, 0, CTRL_BADGE_BG).is_none());
        assert!(find_badge_x(&buf, 1, ALT_BADGE_BG).is_none());
    }

    #[test]
    fn badge_only_appears_for_options_with_explicit_hotkey() {
        let theme = default_theme();
        let ctx = ChoiceRenderContext::for_single(
            &theme,
            TerminalStyle::default(),
            Orientation::Vertical,
        )
        .with_hotkey_display(HotkeyDisplayMode::CtrlHeld);
        let options = fixture_with_hotkeys();
        let area = Rect::new(0, 0, 30, 3);
        let mut buf = Buffer::empty(area);
        ctx.render(
            area,
            &mut buf,
            &options,
            &[0, 1, 2],
            0,
            0,
            |_idx| false,
            None,
            None,
        );

        // Row 2 (Blue) has no hotkey — neither badge background appears.
        assert!(find_badge_x(&buf, 2, CTRL_BADGE_BG).is_none());
        assert!(find_badge_x(&buf, 2, ALT_BADGE_BG).is_none());
    }

    #[test]
    fn horizontal_render_places_badge_below_row_not_inline() {
        let theme = default_theme();
        let ctx = ChoiceRenderContext::for_single(
            &theme,
            TerminalStyle::default(),
            Orientation::Horizontal,
        )
        .with_hotkey_display(HotkeyDisplayMode::CtrlHeld);
        let options = fixture_with_hotkeys();
        // 30 cols wide × 4 rows tall: enough for a single horizontal
        // row plus one badge row beneath it.
        let area = Rect::new(0, 0, 60, 4);
        let mut buf = Buffer::empty(area);
        ctx.render(
            area,
            &mut buf,
            &options,
            &[0, 1, 2],
            0,
            0,
            |_idx| false,
            None,
            None,
        );

        // The Ctrl badge for option 0 (Red) must appear on row 1
        // (immediately below the option row), not on row 0.
        assert!(
            find_badge_x(&buf, 0, CTRL_BADGE_BG).is_none(),
            "Ctrl badge must NOT appear inline on the option row in horizontal mode"
        );
        assert!(
            find_badge_x(&buf, 1, CTRL_BADGE_BG).is_some(),
            "Ctrl badge must appear on the row directly below the option in horizontal mode"
        );
    }

    #[test]
    fn horizontal_multi_row_badges_do_not_overwrite_next_row_options() {
        let theme = default_theme();
        let ctx = ChoiceRenderContext::for_single(
            &theme,
            TerminalStyle::default(),
            Orientation::Horizontal,
        )
        .with_hotkey_display(HotkeyDisplayMode::CtrlHeld);
        let options: Vec<ChoiceOption<String>> = vec![
            ChoiceOption::new("a", "Alpha", "alpha").with_hotkey(HotkeySpec::Ctrl('a')),
            ChoiceOption::new("b", "Bravo", "bravo").with_hotkey(HotkeySpec::Ctrl('b')),
            ChoiceOption::new("c", "Charlie", "charlie").with_hotkey(HotkeySpec::Ctrl('c')),
            ChoiceOption::new("d", "Delta", "delta").with_hotkey(HotkeySpec::Ctrl('d')),
        ];
        // Narrow width forces wrapping: each option is ~11-13 cells
        // (indicator + space + label + trailing blank + badge padding + badge),
        // so at most one option per row. With row_height=2 (badges visible),
        // row 0 option occupies y=0, badge at y=1; row 1 option at y=2,
        // badge at y=3.
        let area = Rect::new(0, 0, 15, 8);
        let mut buf = Buffer::empty(area);
        ctx.render(
            area,
            &mut buf,
            &options,
            &[0, 1, 2, 3],
            0,
            0,
            |_idx| false,
            None,
            None,
        );

        // Row 0 (y=0): option text for "Alpha"
        // Row 1 (y=1): badge sub-row for row 0 — must NOT contain
        //              option label text from subsequent options.
        // Row 2 (y=2): option text for "Bravo"
        // Row 3 (y=3): badge sub-row for row 1

        // Verify that the badge row (y=1) does not contain option
        // label characters from subsequent rows.
        let badge_row = buffer_row(&buf, 1);
        assert!(
            !badge_row.contains("Bravo"),
            "badge row y=1 must not contain option text from row 1: got '{badge_row}'"
        );
        assert!(
            !badge_row.contains("Charlie"),
            "badge row y=1 must not contain option text from row 2: got '{badge_row}'"
        );

        // Verify that the option row (y=2) contains option text (Bravo).
        let option_row_1 = buffer_row(&buf, 2);
        assert!(
            option_row_1.contains("Bravo"),
            "option row y=2 must contain its option text: got '{option_row_1}'"
        );

        // Verify badge appears on y=3 (below the second option row),
        // not colliding with the option content on y=2.
        assert!(
            find_badge_x(&buf, 3, CTRL_BADGE_BG).is_some(),
            "badge for row 1 option must appear on y=3"
        );

        // Cross-check: option text on y=2 must NOT have badge background.
        assert!(
            find_badge_x(&buf, 2, CTRL_BADGE_BG).is_none(),
            "option row y=2 must not carry badge background (collision)"
        );

        // Verify badge row y=3 does not overwrite option text on y=4.
        let option_row_2 = buffer_row(&buf, 4);
        assert!(
            option_row_2.contains("Charlie"),
            "option row y=4 must contain Charlie: got '{option_row_2}'"
        );
        assert!(
            find_badge_x(&buf, 4, CTRL_BADGE_BG).is_none(),
            "option row y=4 must not carry badge background (collision)"
        );
    }
}
