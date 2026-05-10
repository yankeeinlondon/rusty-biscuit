use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
};
use unicode_width::UnicodeWidthStr;

use super::super::choose::ChoiceOption;
use super::{
    ChoiceRenderContext, HIGHLIGHT_MIN_WIDTH, badge::badge_span, highlight::build_highlighted_spans,
};
use crate::core::{FuzzyFilter, resolve_active_style};

impl<'a> ChoiceRenderContext<'a> {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn render_vertical<V, F>(
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

            let has_badge = option
                .effective_hotkey()
                .and_then(|spec| badge_span(spec, self.hotkey_display))
                .is_some();

            // Render active background over the visible item width plus
            // one blank cell. When the row is not hovered but a badge is
            // present, emit an unstyled placeholder of the same width so
            // badges remain horizontally aligned across hovered and
            // non-hovered rows.
            if is_hovered && !option_disabled {
                spans.push(Span::styled(" ", hover_style));
            } else if has_badge {
                spans.push(Span::raw(" "));
            }

            // Inline hotkey badge in vertical mode (Phase 6). Drawn
            // after the trailing blank so it never bleeds into the
            // active highlight.
            if let Some(spec) = option.effective_hotkey()
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
}
