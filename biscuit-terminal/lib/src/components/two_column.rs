use crate::prelude::*;
use crate::utils::block_constraint::{split_lines, visible_width, wrap_lines};

/// Renders content into two columns side by side.
///
/// - allows for columns to be of variant widths to each other
///   but the default is to split render window 50/50.
#[derive(Debug, Clone)]
pub struct TwoColumn {
    left: RenderableContent,
    right: RenderableContent,
    left_percent: f32,
    layout: Layout,
}

impl TwoColumn {
    /// Create a new two-column layout with optional ratio (defaults to 50/50).
    pub fn new<L: Into<RenderableContent>, R: Into<RenderableContent>>(left: L, right: R) -> Self {
        TwoColumn {
            left: left.into(),
            right: right.into(),
            left_percent: 0.5,
            layout: Layout::default(),
        }
    }

    /// Adjust the percentage of width allocated to the left column.
    ///
    /// Values are clamped to the range 0.0..=1.0; defaults to 0.5.
    pub fn with_left_percent(mut self, percent: f32) -> Self {
        self.left_percent = percent.clamp(0.0, 1.0);
        self
    }

    fn render_columns(&self, width: u32, term: Option<&Terminal>) -> String {
        if width == 0 {
            return String::new();
        }

        // Leave a single-space gutter between columns when possible.
        let gutter: u32 = 1;
        if width <= gutter {
            return self.render_stacked(width, term);
        }

        let available = width.saturating_sub(gutter);
        // Determine column widths, ensuring both receive space when possible.
        let mut left_width = (available as f32 * self.left_percent).round() as u32;
        left_width = left_width.clamp(1, available.saturating_sub(1).max(1));
        let right_width = available.saturating_sub(left_width);

        if right_width == 0 {
            return self.render_stacked(width, term);
        }

        let left_lines = self.render_column(&self.left, left_width, term);
        let right_lines = self.render_column(&self.right, right_width, term);
        let max_lines = left_lines.len().max(right_lines.len());
        let gutter_str = " ";

        let mut combined = Vec::with_capacity(max_lines);
        for i in 0..max_lines {
            let left_line = left_lines.get(i).map(String::as_str).unwrap_or("");
            let right_line = right_lines.get(i).map(String::as_str).unwrap_or("");

            let mut row = String::new();
            let left_pad = left_width.saturating_sub(visible_width(left_line));
            let right_pad = right_width.saturating_sub(visible_width(right_line));

            row.push_str(left_line);
            row.push_str(&" ".repeat(left_pad as usize));
            row.push_str(gutter_str);
            row.push_str(right_line);
            row.push_str(&" ".repeat(right_pad as usize));

            combined.push(row);
        }

        combined.join("\n")
    }

    fn render_stacked(&self, width: u32, term: Option<&Terminal>) -> String {
        let left = self.render_column(&self.left, width, term).join("\n");
        let right = self.render_column(&self.right, width, term).join("\n");

        if right.is_empty() {
            left
        } else if left.is_empty() {
            right
        } else {
            format!("{}\n{}", left, right)
        }
    }

    fn render_column(
        &self,
        content: &RenderableContent,
        width: u32,
        term: Option<&Terminal>,
    ) -> Vec<String> {
        if width == 0 {
            return vec![String::new()];
        }

        match content {
            RenderableContent::String(s) => {
                wrap_lines(split_lines(s), &WordWrap::WrapProse(None, None), width)
            }
            RenderableContent::Component(component) => {
                let rendered = if let Some(t) = term {
                    let mut column_term = Terminal::from(t);
                    column_term.fixed_width = Some(width);
                    component.fallback_render(&column_term)
                } else {
                    component.render(Some(width))
                };
                split_lines(rendered)
            }
        }
    }

    fn render_with_width(&self, term_width: u32, term: Option<&Terminal>) -> String {
        let available = self.layout.available_width(term_width);
        if available == 0 {
            return String::new();
        }

        let combined = self.render_columns(available, term);
        self.layout.apply_layout(&combined, term_width)
    }
}

impl Renderable for TwoColumn {
    fn alignment(mut self, alignment: Alignment) -> Self
    where
        Self: Sized,
    {
        self.layout.alignment = alignment;
        self
    }

    fn as_child_of(mut self, parent: &Layout, left_offset: u32, right_offset: u32) -> Self
    where
        Self: Sized,
    {
        self.layout.left_margin = parent.left_margin.clone().add_chars(left_offset);
        self.layout.right_margin = parent.right_margin.clone().add_chars(right_offset);
        self
    }

    fn bottom_margin(mut self, margin: Margin) -> Self
    where
        Self: Sized,
    {
        self.layout.bottom_margin = margin;
        self
    }

    fn fallback_render(&self, term: &Terminal) -> String {
        let width = term.width();
        self.render_with_width(width, Some(term))
    }

    fn is_block_level(&self) -> bool {
        true
    }

    fn layout(&self) -> &Layout {
        &self.layout
    }

    fn layout_mut(&mut self) -> &mut Layout {
        &mut self.layout
    }

    fn left_margin(mut self, margin: Margin) -> Self
    where
        Self: Sized,
    {
        self.layout.left_margin = margin;
        self
    }

    fn render(&self, term_width: Option<u32>) -> String {
        let width = term_width.unwrap_or(80);
        self.render_with_width(width, None)
    }

    fn right_margin(mut self, margin: Margin) -> Self
    where
        Self: Sized,
    {
        self.layout.right_margin = margin;
        self
    }

    fn row_fill_strategy(mut self, strategy: RowFill) -> Self
    where
        Self: Sized,
    {
        self.layout.row_fill_strategy = strategy;
        self
    }

    fn top_margin(mut self, margin: Margin) -> Self
    where
        Self: Sized,
    {
        self.layout.top_margin = margin;
        self
    }

    fn word_wrap(mut self, wrap: WordWrap) -> Self
    where
        Self: Sized,
    {
        self.layout.word_wrap = wrap;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_side_by_side_balanced() {
        let two = TwoColumn::new("Left", "Right");
        let result = two.render(Some(20));
        assert_eq!(result, "Left       Right    ");
    }

    #[test]
    fn respects_custom_ratio_and_height_padding() {
        let two = TwoColumn::new("Left line\nLeft two", "Right").with_left_percent(0.7);
        let rendered = two.render(Some(30));
        let lines: Vec<&str> = rendered.lines().collect();

        assert_eq!(lines.len(), 2);
        // Verify left column receives more space and right column is padded to align rows.
        assert!(lines[0].starts_with("Left line"));
        assert!(lines[1].starts_with("Left two"));
        assert!(lines[0].contains(" Right"));
        assert_eq!(lines[0].len(), lines[1].len());
    }

    #[test]
    fn stacks_when_not_enough_space() {
        let two = TwoColumn::new("L", "R");
        let rendered = two.render(Some(1));
        assert_eq!(rendered, "L\nR");
    }

    #[test]
    fn is_block_level_component() {
        let two = TwoColumn::new("L", "R");
        assert!(two.is_block_level());
    }
}
