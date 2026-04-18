//! Label placement shared across every input component.
//!
//! A [`Label`] pairs a string with a [`LabelPosition`] describing
//! where it renders relative to the component body. The
//! [`render_with_label`] helper carves the label area out of the
//! available [`Rect`], draws the label text, and hands the remaining
//! inner rect to the component's body renderer.

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Style,
    text::{Line, Span},
};
use unicode_width::UnicodeWidthStr;

/// Where a [`Label`] renders relative to its component body.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LabelPosition {
    /// The label renders on the row above the component.
    Above,
    /// The label renders on the row below the component.
    Below,
    /// The label renders to the left of the component on the same row.
    Left,
    /// The label renders to the right of the component on the same row.
    Right,
}

/// A text label with a fixed placement relative to its component body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Label {
    /// The display text.
    pub text: String,
    /// Where the label renders relative to the component body.
    pub position: LabelPosition,
}

impl Label {
    /// Convenience constructor.
    pub fn new(text: impl Into<String>, position: LabelPosition) -> Self {
        Self {
            text: text.into(),
            position,
        }
    }

    /// Display width of the label text in terminal cells.
    pub fn display_width(&self) -> u16 {
        u16::try_from(self.text.width()).unwrap_or(u16::MAX)
    }
}

/// Draws an optional [`Label`] and delegates the remaining inner
/// rectangle to the component's body renderer.
///
/// Returns the inner `Rect` that was passed to `inner` so that callers
/// can perform additional post-body drawing (for instance, rendering
/// an inline validation error below the body).
///
/// ## Notes
///
/// - When `label` is `None`, the entire `area` is passed straight to
///   `inner`.
/// - When the available area is too small to reserve space for a label
///   (e.g. height `< 2` for an `Above`/`Below` label), the label is
///   silently dropped and the full area is handed to `inner`.
/// - Horizontal labels reserve width equal to the label's display
///   width plus a single spacer column.
pub fn render_with_label<F>(
    area: Rect,
    buf: &mut Buffer,
    label: Option<&Label>,
    label_style: Style,
    inner: F,
) -> Rect
where
    F: FnOnce(Rect, &mut Buffer),
{
    let Some(label) = label else {
        inner(area, buf);
        return area;
    };

    let label_text = label.text.as_str();
    let label_width = label.display_width();

    let inner_area = match label.position {
        LabelPosition::Above => {
            if area.height < 2 {
                inner(area, buf);
                return area;
            }
            let label_line = Line::from(Span::styled(label_text.to_string(), label_style));
            buf.set_line(area.x, area.y, &label_line, area.width);
            Rect::new(area.x, area.y + 1, area.width, area.height - 1)
        }
        LabelPosition::Below => {
            if area.height < 2 {
                inner(area, buf);
                return area;
            }
            let inner_rect = Rect::new(area.x, area.y, area.width, area.height - 1);
            let label_line = Line::from(Span::styled(label_text.to_string(), label_style));
            buf.set_line(area.x, area.y + area.height - 1, &label_line, area.width);
            inner_rect
        }
        LabelPosition::Left => {
            let gap: u16 = 1;
            let reserve = label_width.saturating_add(gap);
            if area.width <= reserve {
                inner(area, buf);
                return area;
            }
            buf.set_stringn(
                area.x,
                area.y,
                label_text,
                area.width as usize,
                label_style,
            );
            Rect::new(
                area.x + reserve,
                area.y,
                area.width - reserve,
                area.height,
            )
        }
        LabelPosition::Right => {
            let gap: u16 = 1;
            let reserve = label_width.saturating_add(gap);
            if area.width <= reserve {
                inner(area, buf);
                return area;
            }
            let label_x = area.x + area.width - label_width;
            buf.set_stringn(
                label_x,
                area.y,
                label_text,
                label_width as usize,
                label_style,
            );
            Rect::new(area.x, area.y, area.width - reserve, area.height)
        }
    };

    inner(inner_area, buf);
    inner_area
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::buffer::Buffer;

    fn buffer_row_text(buf: &Buffer, y: u16) -> String {
        let mut text = String::new();
        for x in buf.area.left()..buf.area.right() {
            text.push_str(buf[(x, y)].symbol());
        }
        text.trim_end().to_string()
    }

    #[test]
    fn label_position_constructor_sets_fields() {
        let label = Label::new("Name", LabelPosition::Above);
        assert_eq!(label.text, "Name");
        assert_eq!(label.position, LabelPosition::Above);
    }

    #[test]
    fn render_with_label_returns_full_area_when_label_is_none() {
        let area = Rect::new(0, 0, 10, 3);
        let mut buf = Buffer::empty(area);
        let mut received: Option<Rect> = None;
        let inner_rect = render_with_label(area, &mut buf, None, Style::default(), |r, _b| {
            received = Some(r);
        });
        assert_eq!(inner_rect, area);
        assert_eq!(received, Some(area));
    }

    #[test]
    fn label_above_draws_on_first_row_and_returns_shrunken_rect() {
        let area = Rect::new(0, 0, 10, 3);
        let mut buf = Buffer::empty(area);
        let label = Label::new("Name", LabelPosition::Above);
        let inner = render_with_label(area, &mut buf, Some(&label), Style::default(), |r, b| {
            b.set_string(r.x, r.y, "BODY", Style::default());
        });
        assert_eq!(inner, Rect::new(0, 1, 10, 2));
        assert_eq!(buffer_row_text(&buf, 0), "Name");
        assert_eq!(buffer_row_text(&buf, 1), "BODY");
    }

    #[test]
    fn label_below_draws_on_last_row_and_returns_shrunken_rect() {
        let area = Rect::new(0, 0, 10, 3);
        let mut buf = Buffer::empty(area);
        let label = Label::new("Tag", LabelPosition::Below);
        let inner = render_with_label(area, &mut buf, Some(&label), Style::default(), |r, b| {
            b.set_string(r.x, r.y, "BODY", Style::default());
        });
        assert_eq!(inner, Rect::new(0, 0, 10, 2));
        assert_eq!(buffer_row_text(&buf, 0), "BODY");
        assert_eq!(buffer_row_text(&buf, 2), "Tag");
    }

    #[test]
    fn label_left_reserves_width_on_the_left() {
        let area = Rect::new(0, 0, 20, 1);
        let mut buf = Buffer::empty(area);
        let label = Label::new("Name", LabelPosition::Left);
        let inner = render_with_label(area, &mut buf, Some(&label), Style::default(), |r, b| {
            b.set_string(r.x, r.y, "BODY", Style::default());
        });
        // "Name" (4) + gap (1) = 5 reserved
        assert_eq!(inner, Rect::new(5, 0, 15, 1));
        let row = buffer_row_text(&buf, 0);
        assert!(row.starts_with("Name"), "expected row to start with label, got {row:?}");
        assert!(row.contains("BODY"), "expected row to contain body, got {row:?}");
    }

    #[test]
    fn label_right_reserves_width_on_the_right() {
        let area = Rect::new(0, 0, 20, 1);
        let mut buf = Buffer::empty(area);
        let label = Label::new("Name", LabelPosition::Right);
        let inner = render_with_label(area, &mut buf, Some(&label), Style::default(), |r, b| {
            b.set_string(r.x, r.y, "BODY", Style::default());
        });
        // Reserve on the right: width - 5 = 15
        assert_eq!(inner, Rect::new(0, 0, 15, 1));
        let row = buffer_row_text(&buf, 0);
        assert!(row.starts_with("BODY"), "expected body at start, got {row:?}");
        assert!(row.ends_with("Name"), "expected label at end, got {row:?}");
    }

    #[test]
    fn label_falls_back_to_full_area_when_space_is_insufficient() {
        let area = Rect::new(0, 0, 4, 1);
        let mut buf = Buffer::empty(area);
        let label = Label::new("TooWide", LabelPosition::Left);
        let inner = render_with_label(area, &mut buf, Some(&label), Style::default(), |r, b| {
            b.set_string(r.x, r.y, "xx", Style::default());
        });
        assert_eq!(inner, area);
    }

    #[test]
    fn display_width_counts_grapheme_cells() {
        let label = Label::new("abc", LabelPosition::Above);
        assert_eq!(label.display_width(), 3);
    }
}
