use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
};
use unicode_width::UnicodeWidthStr;

use super::super::{choice_layout::ChoiceLayout, choose::ChoiceOption};
use super::{
    ChoiceRenderContext, HIGHLIGHT_MIN_WIDTH, badge::badge_span, highlight::build_highlighted_spans,
};
use crate::core::{FuzzyFilter, resolve_active_style};

impl<'a> ChoiceRenderContext<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_horizontal<V, F>(
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

        let indicator_width = std::cmp::max(
            self.selected_indicator.width(),
            self.unselected_indicator.width(),
        );
        // Badge width is excluded from layout width because badges render
        // on a sub-row below the option, not inline. Including badge width
        // would create ghost gaps between items on the main row.
        let widths: Vec<u16> = visible_indices
            .iter()
            .map(|&idx| {
                let label_width = options[idx].label.width();
                (indicator_width + 1 + label_width + 1) as u16
            })
            .collect();
        let row_height = self.row_height();
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
                if let Some(spec) = option.effective_hotkey()
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
            let y = area.y + (shown_rows - 1) as u16 * row_height;
            buf[(x, y)]
                .set_symbol(&self.theme.overflow_down_indicator)
                .set_style(overflow_style);
        }
    }
}
