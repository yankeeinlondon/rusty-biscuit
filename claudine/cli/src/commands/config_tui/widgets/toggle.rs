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
        let (indicator, style) = if self.value {
            ("ON ", Style::default().fg(Color::Green))
        } else {
            ("OFF", Style::default().fg(Color::Red))
        };

        let label_style = if self.is_active {
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Gray)
        };

        let line = Line::from(vec![
            Span::styled(self.label, label_style),
            Span::raw("  "),
            Span::styled(indicator, style),
        ]);

        Paragraph::new(line).render(area, buf);
    }
}
