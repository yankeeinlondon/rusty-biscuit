use ratatui::prelude::*;
use ratatui::widgets::*;

pub fn render_modal(
    frame: &mut Frame,
    parent_area: Rect,
    title: &str,
    width_pct: u16,
    height_pct: u16,
    content_fn: impl FnOnce(&mut Frame, Rect),
) {
    let modal_area = centered_rect(width_pct, height_pct, parent_area);

    frame.render_widget(Clear, modal_area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow))
        .title(format!(" {title} "))
        .title_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );

    let inner = block.inner(modal_area);
    frame.render_widget(block, modal_area);

    // Apply 1-cell left and top padding inside all modals
    let padded = Rect {
        x: inner.x + 1,
        y: inner.y + 1,
        width: inner.width.saturating_sub(1),
        height: inner.height.saturating_sub(1),
    };

    content_fn(frame, padded);
}

pub fn render_list_modal(
    frame: &mut Frame,
    parent_area: Rect,
    title: &str,
    items: &[String],
    highlighted: usize,
) {
    render_list_modal_with_hotkeys(
        frame,
        parent_area,
        title,
        items,
        highlighted,
        &[("ENTER", "Select"), ("ESC", "Cancel")],
    );
}

pub fn render_list_modal_with_hotkeys(
    frame: &mut Frame,
    parent_area: Rect,
    title: &str,
    items: &[String],
    highlighted: usize,
    hotkeys: &[(&str, &str)],
) {
    render_modal(frame, parent_area, title, 50, 60, |frame, area| {
        let has_hotkeys = !hotkeys.is_empty();
        let (list_area, hotkey_area) = if has_hotkeys {
            let split = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(0),
                    Constraint::Length(1), // blank separator
                    Constraint::Length(1), // hotkey bar
                ])
                .split(area);
            (split[0], Some(split[2]))
        } else {
            (area, None)
        };

        let items_count = items.len();
        let list_items: Vec<ListItem> = items
            .iter()
            .map(|item| {
                ListItem::new(Line::from(Span::styled(
                    item.as_str(),
                    Style::default().fg(Color::White),
                )))
            })
            .collect();

        let list = List::new(list_items)
            .highlight_symbol(">> ")
            .highlight_style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            );
        let mut state = ListState::default().with_selected(Some(highlighted));
        frame.render_stateful_widget(list, list_area, &mut state);

        if items_count > list_area.height as usize {
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None);
            let mut scrollbar_state = ScrollbarState::new(items_count).position(highlighted);
            frame.render_stateful_widget(scrollbar, list_area, &mut scrollbar_state);
        }

        if let Some(hotkey_area) = hotkey_area {
            let hotkey_line = build_modal_hotkey_line(hotkeys);
            let hotkey_widget = Paragraph::new(hotkey_line)
                .alignment(Alignment::Center)
                .style(Style::default().bg(Color::Indexed(236)));
            frame.render_widget(hotkey_widget, hotkey_area);
        }
    });
}

/// Build a styled hotkey line for use inside modals.
/// Keys are bold+yellow, descriptions are light gray, pipe separators.
pub fn build_modal_hotkey_line(pairs: &[(&str, &str)]) -> Line<'static> {
    let key_style = Style::default()
        .fg(Color::Yellow)
        .add_modifier(Modifier::BOLD);
    let desc_style = Style::default().fg(Color::Indexed(250));
    let sep_style = Style::default().fg(Color::Indexed(240));

    let mut spans: Vec<Span<'static>> = Vec::new();
    for (i, (key, desc)) in pairs.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(" │ ", sep_style));
        }
        spans.push(Span::styled(key.to_string(), key_style));
        spans.push(Span::styled(format!(": {desc}"), desc_style));
    }
    Line::from(spans)
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_debug_snapshot;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    #[test]
    fn build_modal_hotkey_line_formats_pairs() {
        let line = build_modal_hotkey_line(&[("ENTER", "Select"), ("ESC", "Cancel")]);
        let text = line
            .spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect::<String>();

        assert!(text.contains("ENTER: Select"));
        assert!(text.contains("ESC: Cancel"));
        assert!(text.contains("│"));
    }

    #[test]
    fn render_list_modal_matches_snapshot() {
        let backend = TestBackend::new(60, 16);
        let mut terminal = Terminal::new(backend).unwrap();

        terminal
            .draw(|frame| {
                render_list_modal_with_hotkeys(
                    frame,
                    frame.area(),
                    "Pick Provider",
                    &["Claude".into(), "Codex".into(), "Gemini".into()],
                    1,
                    &[("ENTER", "Select"), ("ESC", "Cancel")],
                );
            })
            .unwrap();

        let buffer = terminal.backend().buffer().clone();
        assert_debug_snapshot!("list_modal_buffer", buffer);
    }
}
