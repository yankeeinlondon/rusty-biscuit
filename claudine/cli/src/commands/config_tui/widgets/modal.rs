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

    content_fn(frame, inner);
}

pub fn render_list_modal(
    frame: &mut Frame,
    parent_area: Rect,
    title: &str,
    items: &[String],
    highlighted: usize,
) {
    render_modal(frame, parent_area, title, 50, 60, |frame, area| {
        let items: Vec<ListItem> = items
            .iter()
            .enumerate()
            .map(|(i, item)| {
                let style = if i == highlighted {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                ListItem::new(Line::from(Span::styled(item.as_str(), style)))
            })
            .collect();

        let list = List::new(items).highlight_symbol(">> ");
        frame.render_widget(list, area);
    });
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
