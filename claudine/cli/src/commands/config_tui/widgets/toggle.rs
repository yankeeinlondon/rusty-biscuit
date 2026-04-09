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
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
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

#[cfg(test)]
mod tests {
    use super::*;

    fn row_text(buf: &Buffer) -> String {
        (0..buf.area.width)
            .map(|x| buf[(x, 0)].symbol().chars().next().unwrap_or(' '))
            .collect::<String>()
    }

    #[test]
    fn toggle_renders_active_on_state() {
        let area = Rect::new(0, 0, 24, 1);
        let mut buf = Buffer::empty(area);

        Toggle::new("Logging", true, true).render(area, &mut buf);

        let row = row_text(&buf);
        assert!(row.contains("Logging:  On / Off"));

        let on_index = row.find("On").unwrap() as u16;
        assert_eq!(buf[(on_index, 0)].fg, Color::Green);
    }

    #[test]
    fn toggle_renders_inactive_off_state() {
        let area = Rect::new(0, 0, 24, 1);
        let mut buf = Buffer::empty(area);

        Toggle::new("Protect", false, false).render(area, &mut buf);

        let row = row_text(&buf);
        assert!(row.contains("Protect:  On / Off"));

        let off_index = row.find("Off").unwrap() as u16;
        assert_eq!(buf[(off_index, 0)].fg, Color::Red);
    }
}
