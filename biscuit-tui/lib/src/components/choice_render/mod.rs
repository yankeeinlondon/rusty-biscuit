//! Shared choice list rendering for [`ChooseOne`](super::choose_one::ChooseOne)
//! and [`ChooseMany`](super::choose_many::ChooseMany).
//!
//! Consolidates the duplicated `draw_list` logic from both components
//! into a single renderer driven by [`ChoiceRenderContext`].

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    text::{Line, Span},
};
use unicode_width::UnicodeWidthStr;

mod badge;
mod highlight;
mod horizontal;
mod vertical;

pub use highlight::build_highlighted_spans;

use super::choose::{ActiveChoiceColor, ChoiceOption, HotkeyDisplayMode, Orientation};
use crate::core::{ComponentTheme, FuzzyFilter, NerdFontStatus, TerminalStyle};

/// Minimum label width (in cells) at which the fuzzy filter renders
/// per-character match highlighting. Narrower labels fall back to a
/// single plain span so the row stays readable.
pub(super) const HIGHLIGHT_MIN_WIDTH: u16 = 12;

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
    /// Returns how many terminal rows each logical choice row occupies.
    pub(crate) fn row_height(&self) -> u16 {
        match self.orientation {
            Orientation::Vertical => 1,
            Orientation::Horizontal => {
                if self.hotkey_display == HotkeyDisplayMode::Hidden {
                    1
                } else {
                    2
                }
            }
        }
    }

    /// Converts terminal body rows into logical choice rows.
    pub(crate) fn visible_logical_rows(&self, body_rows: u16) -> usize {
        if body_rows == 0 {
            return 0;
        }
        match self.orientation {
            Orientation::Vertical => body_rows as usize,
            Orientation::Horizontal => usize::from((body_rows / self.row_height()).max(1)),
        }
    }

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
                // Badge width is excluded from layout width because badges
                // render on a sub-row below the option, not inline. Including
                // badge width would create ghost gaps between items on the
                // main row.
                let widths: Vec<u16> = visible_indices
                    .iter()
                    .map(|&idx| {
                        let label_width = options[idx].label.width();
                        (indicator_width + 1 + label_width + 1) as u16
                    })
                    .collect();
                ChoiceLayout::horizontal(visible_indices, &widths, area, self.row_height())
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
        let visible_count = self.visible_logical_rows(body_rows);

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
}

#[cfg(test)]
mod tests;
