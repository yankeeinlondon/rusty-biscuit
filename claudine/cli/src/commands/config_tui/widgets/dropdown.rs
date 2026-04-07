use ratatui::prelude::*;
use ratatui::widgets::*;

pub struct Dropdown<'a> {
    label: &'a str,
    selected: &'a str,
    is_active: bool,
}

impl<'a> Dropdown<'a> {
    pub fn new(label: &'a str, selected: &'a str, is_active: bool) -> Self {
        Self {
            label,
            selected,
            is_active,
        }
    }
}

impl Widget for Dropdown<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let label_style = if self.is_active {
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray)
        };

        let value_style = if self.is_active {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::White)
        };

        let line = Line::from(vec![
            Span::styled(self.label, label_style),
            Span::raw(": "),
            Span::styled(format!("[{}]", self.selected), value_style),
        ]);
        Paragraph::new(line).render(area, buf);
    }
}
