use ratatui::prelude::*;
use ratatui::widgets::*;

pub struct Toggle<'a> {
    label: &'a str,
    value: bool,
    is_active: bool,
}

impl<'a> Toggle<'a> {
    pub fn new(label: &'a str, value: bool, is_active: bool) -> Self {
        Self {
            label,
            value,
            is_active,
        }
    }
}

impl Widget for Toggle<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let label_style = if self.is_active {
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray)
        };

        // On state: bold+green when selected, dimmed when not
        let on_style = if self.value {
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        // Off state: bold+red when selected, dimmed when not
        let off_style = if !self.value {
            Style::default()
                .fg(Color::Red)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };

        let line = Line::from(vec![
            Span::styled(self.label, label_style),
            Span::raw(":  "),
            Span::styled("On", on_style),
            Span::raw(" / "),
            Span::styled("Off", off_style),
        ]);

        Paragraph::new(line).render(area, buf);
    }
}
