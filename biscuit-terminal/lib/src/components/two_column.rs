use renderable::tree::{ColumnWidthKind, ColumnsHints, RenderNode};

use crate::discovery::detection::TerminalApp;
use crate::prelude::*;
use crate::render_tree::projection::TreeProjectionContext;
use crate::utils::block_constraint::{split_lines, visible_width, wrap_lines};

/// Determines how wide a column should be.
///
/// This enum provides two options for specifying column widths:
/// - **Fixed**: A specific number of character cells
/// - **Percent**: A percentage of the available width
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::components::two_column::ColumnWidth;
///
/// // Fixed width of 30 characters
/// let fixed = ColumnWidth::Fixed(30);
/// match fixed {
///     ColumnWidth::Fixed(chars) => println!("Fixed width: {} chars", chars),
///     _ => unreachable!(),
/// }
///
/// // Percentage-based: 60% of available space
/// let percent = ColumnWidth::Percent(0.6);
/// match percent {
///     ColumnWidth::Percent(pct) => println!("Percent: {}%", (pct * 100.0) as i32),
///     _ => unreachable!(),
/// }
///
/// // Common use case: 50/50 split (default)
/// let half = ColumnWidth::Percent(0.5);
///
/// // Use case: 70/30 split
/// let narrow_right = ColumnWidth::Percent(0.7);
/// ```
///
/// ## Width Resolution
///
/// When rendering, column widths are resolved as follows:
/// 1. Fixed widths are used as-is
/// 2. Percentages are multiplied by the available width (terminal width minus gaps)
/// 3. If the combined widths exceed available space, columns are clamped
/// 4. If either column would be less than 1 character, content stacks vertically
#[derive(Debug, Clone, Copy)]
pub enum ColumnWidth {
    /// Fixed width in characters.
    Fixed(u32),
    /// Percentage of the available width (0.0..=1.0).
    Percent(f32),
}

const DEFAULT_GAP: u32 = 3;

struct RenderedColumn {
    lines: Vec<String>,
    uses_cursor_padding: bool,
}

fn cursor_forward(chars: u32) -> String {
    if chars == 0 {
        String::new()
    } else {
        format!("\x1b[{}C", chars)
    }
}

fn cursor_save() -> &'static str {
    "\x1b7\x1b[s"
}

fn cursor_restore() -> &'static str {
    "\x1b[u\x1b8"
}

fn render_column_block(render: &RenderedColumn, offset: u32) -> String {
    let mut output = String::new();

    if render.uses_cursor_padding {
        let line = render.lines.first().map(String::as_str).unwrap_or("");
        output.push('\r');
        output.push_str(&cursor_forward(offset));
        output.push_str(line);
        return output;
    }

    for (idx, line) in render.lines.iter().enumerate() {
        if idx == 0 {
            output.push('\r');
        } else {
            output.push_str("\r\n");
        }
        output.push_str(&cursor_forward(offset));
        output.push_str(line);
    }

    output
}

/// A side-by-side two-column layout for terminal rendering.
///
/// TwoColumn arranges two pieces of content side by side, making it ideal
/// for key-value displays, comparisons, or any scenario where related content
/// should be shown in parallel columns. The layout automatically handles
/// word-wrapping within each column and can adapt to narrow terminals.
///
/// ## Column Widths
///
/// Columns can be sized using either:
/// - **Fixed**: A specific number of character cells
/// - **Percent**: A percentage of available space (0.0 to 1.0)
///
/// The default is a 50/50 split with a 3-character gap between columns.
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::components::two_column::TwoColumn;
/// use biscuit_terminal::components::renderable::TerminalRenderable;
///
/// // Basic 50/50 split
/// let cols = TwoColumn::new("Left content", "Right content");
///
/// // 70/30 split for narrow right column
/// let cols = TwoColumn::new("Details:", "Some value here")
///     .with_left_percent(0.7);
///
/// // Fixed width left column (30 chars), rest goes to right
/// use biscuit_terminal::components::two_column::ColumnWidth;
/// let cols = TwoColumn::new("Label", "Value")
///     .with_left_width(ColumnWidth::Fixed(30))
///     .with_gap(2);  // 2 char gap instead of default 3
///
/// // Render to string
/// let output = cols.render_optimistic(Some(80));
/// ```
///
/// ## Responsive Behavior
///
/// When the terminal is too narrow, columns automatically stack vertically
/// to ensure content remains readable. The breakpoint depends on the gap
/// and minimum column widths.
#[derive(Debug, Clone)]
pub struct TwoColumn {
    left: RenderableTerminalContent,
    right: RenderableTerminalContent,
    left_width: ColumnWidth,
    gap: u32,
    layout: Layout,
}

impl Default for TwoColumn {
    fn default() -> Self {
        TwoColumn {
            left: "".into(),
            right: "".into(),
            left_width: ColumnWidth::Percent(0.5),
            gap: DEFAULT_GAP,
            layout: Layout::default(),
        }
    }
}

impl TwoColumn {
    /// Create a new two-column layout with optional ratio (defaults to 50/50).
    pub fn new<L: Into<RenderableTerminalContent>, R: Into<RenderableTerminalContent>>(
        left: L,
        right: R,
    ) -> Self {
        TwoColumn {
            left: left.into(),
            right: right.into(),
            left_width: ColumnWidth::Percent(0.5),
            gap: DEFAULT_GAP,
            layout: Layout::default(),
        }
    }

    /// Adjust the percentage of width allocated to the left column.
    ///
    /// Values are clamped to the range 0.0..=1.0; defaults to 0.5.
    pub fn with_left_percent(mut self, percent: f32) -> Self {
        self.left_width = ColumnWidth::Percent(percent.clamp(0.0, 1.0));
        self
    }

    /// Adjust the width of the left column.
    pub fn with_left_width(mut self, width: ColumnWidth) -> Self {
        self.left_width = match width {
            ColumnWidth::Percent(percent) => ColumnWidth::Percent(percent.clamp(0.0, 1.0)),
            ColumnWidth::Fixed(chars) => ColumnWidth::Fixed(chars),
        };
        self
    }

    /// Adjust the number of characters between columns.
    pub fn with_gap(mut self, gap: u32) -> Self {
        self.gap = gap;
        self
    }

    fn render_columns(&self, width: u32, term: Option<&Terminal>) -> String {
        if width == 0 {
            return String::new();
        }

        if width <= self.gap {
            return self.render_stacked(width, term);
        }

        let available = width.saturating_sub(self.gap);
        // Determine column widths, ensuring both receive space when possible.
        let mut left_width = match self.left_width {
            ColumnWidth::Fixed(chars) => chars,
            ColumnWidth::Percent(percent) => {
                // Truncate rather than round for predictable integer division behavior
                (available as f32 * percent.clamp(0.0, 1.0)) as u32
            }
        };
        left_width = left_width.clamp(1, available.saturating_sub(1).max(1));
        let right_width = available.saturating_sub(left_width);

        tracing::trace!(
            total_width = width,
            left_width,
            right_width,
            gap = self.gap,
            "TwoColumn resolved widths"
        );

        if right_width == 0 {
            return self.render_stacked(width, term);
        }

        let left_render = self.render_column(&self.left, left_width, term);
        let right_render = self.render_column(&self.right, right_width, term);
        if left_render.uses_cursor_padding || right_render.uses_cursor_padding {
            return self.render_overlay(&left_render, &right_render, left_width, right_width, term);
        }
        let max_lines = left_render.lines.len().max(right_render.lines.len());
        let gutter_str = " ".repeat(self.gap as usize);

        let mut combined = Vec::with_capacity(max_lines);
        for i in 0..max_lines {
            let left_line = left_render.lines.get(i).map(String::as_str).unwrap_or("");
            let right_line = right_render.lines.get(i).map(String::as_str).unwrap_or("");

            let mut row = String::new();
            let left_pad = left_width.saturating_sub(visible_width(left_line));
            let right_pad = right_width.saturating_sub(visible_width(right_line));

            row.push_str(left_line);
            if left_render.uses_cursor_padding {
                row.push_str(&cursor_forward(left_pad));
            } else {
                row.push_str(&" ".repeat(left_pad as usize));
            }
            row.push_str(&gutter_str);
            row.push_str(right_line);
            if !right_render.uses_cursor_padding {
                row.push_str(&" ".repeat(right_pad as usize));
            }

            combined.push(row);
        }

        combined.join("\n")
    }

    fn render_overlay(
        &self,
        left_render: &RenderedColumn,
        right_render: &RenderedColumn,
        left_width: u32,
        _right_width: u32,
        term: Option<&Terminal>,
    ) -> String {
        if let Some(app) = term.map(|t| &t.app)
            && matches!(
                app,
                TerminalApp::Wezterm
                    | TerminalApp::Ghostty
                    | TerminalApp::ITerm2
                    | TerminalApp::Kitty
            )
        {
            let move_up_by = match app {
                TerminalApp::Wezterm | TerminalApp::Ghostty => {
                    left_render.lines.len().saturating_sub(1) as u32
                }
                TerminalApp::ITerm2 | TerminalApp::Kitty => {
                    left_render.lines.len().saturating_sub(1) as u32
                }
                _ => left_render.lines.len() as u32,
            };
            return self.render_overlay_with_cursor_reset(
                left_render,
                right_render,
                left_width,
                move_up_by,
            );
        }

        let right_offset = left_width + self.gap;
        let block_height = left_render.lines.len().max(right_render.lines.len());

        let mut output = String::new();
        output.push_str(cursor_save());
        output.push_str(&render_column_block(left_render, 0));
        output.push_str(cursor_restore());
        output.push_str(cursor_save());
        output.push_str(&render_column_block(right_render, right_offset));
        output.push_str(cursor_restore());

        if block_height > 0 {
            output.push_str(&format!("\x1b[{}B\r", block_height));
        }

        output
    }

    fn render_overlay_with_cursor_reset(
        &self,
        left_render: &RenderedColumn,
        right_render: &RenderedColumn,
        left_width: u32,
        move_up_by: u32,
    ) -> String {
        let right_offset = left_width + self.gap;
        let left_height = left_render.lines.len().max(1) as u32;
        let right_height = right_render.lines.len().max(1) as u32;
        let block_height = left_height.max(right_height);

        let mut output = String::new();
        output.push_str(&render_column_block(left_render, 0));

        if move_up_by > 0 {
            output.push_str(&format!("\x1b[{}A", move_up_by));
        }
        output.push('\r');

        output.push_str(&render_column_block(right_render, right_offset));

        let move_down = block_height.saturating_sub(right_height) + 1;
        output.push_str(&format!("\x1b[{}B\r", move_down));

        output
    }

    fn render_stacked(&self, width: u32, term: Option<&Terminal>) -> String {
        let left = self.render_column(&self.left, width, term).lines.join("\n");
        let right = self
            .render_column(&self.right, width, term)
            .lines
            .join("\n");

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
        content: &RenderableTerminalContent,
        width: u32,
        term: Option<&Terminal>,
    ) -> RenderedColumn {
        if width == 0 {
            return RenderedColumn {
                lines: vec![String::new()],
                uses_cursor_padding: false,
            };
        }

        match content {
            RenderableTerminalContent::String(s) => RenderedColumn {
                lines: wrap_lines(split_lines(s), &WordWrap::WrapProse(None, None), width),
                uses_cursor_padding: false,
            },
            RenderableTerminalContent::Component(component) => {
                if let Some(t) = term
                    && let Some(image) = component.as_any().downcast_ref::<TerminalImage>()
                {
                    let mut column_term = Terminal::from(t);
                    column_term.fixed_width = Some(width);
                    if let Ok((sequence, height)) = image.render_inline(&column_term) {
                        let mut lines = Vec::with_capacity(height as usize);
                        lines.push(sequence);
                        for _ in 1..height {
                            lines.push(String::new());
                        }
                        return RenderedColumn {
                            lines,
                            uses_cursor_padding: true,
                        };
                    }

                    let fallback = image.generate_alt_text();
                    return RenderedColumn {
                        lines: wrap_lines(
                            split_lines(fallback),
                            &WordWrap::WrapProse(None, None),
                            width,
                        ),
                        uses_cursor_padding: false,
                    };
                }

                let rendered = if let Some(t) = term {
                    let mut column_term = Terminal::from(t);
                    column_term.fixed_width = Some(width);
                    component.render(&column_term)
                } else {
                    component.render_optimistic(Some(width))
                };
                RenderedColumn {
                    lines: split_lines(rendered),
                    uses_cursor_padding: false,
                }
            }
        }
    }

    fn render_with_width(&self, term_width: u32, term: Option<&Terminal>) -> String {
        let available = self.layout.available_width(term_width);
        if available == 0 {
            return String::new();
        }

        let combined = self.render_columns(available, term);
        // The column boundary must stay vertical across all rows — align as
        // a block.
        self.layout.apply_block_layout(&combined, term_width)
    }
}

impl TerminalRenderable for TwoColumn {
    fn alignment(mut self, alignment: Alignment) -> Self
    where
        Self: Sized,
    {
        self.layout.alignment = alignment;
        self
    }

    fn render(&self, term: &Terminal) -> String {
        let width = term.width();
        self.render_with_width(width, Some(term))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn is_block_level(&self) -> bool {
        true
    }

    /// Projects the two-column layout to a [`NodeKind::BlockQuote`] container
    /// carrying [`ColumnsHints`].
    ///
    /// The left and right column contents are each projected into block-level
    /// nodes; the block quote holds the left nodes followed by the right
    /// nodes, and [`ColumnsHints::left_count`] records the split point. The
    /// three tree renderers special-case the columns hint, so the block
    /// quote's border never actually renders.
    ///
    /// A column whose content is a [`TerminalImage`] cannot be projected —
    /// cursor-overlay image rendering has no render-tree representation — so
    /// this returns a [`RenderNode::unsupported`] node instead.
    ///
    /// [`NodeKind::BlockQuote`]: renderable::tree::NodeKind::BlockQuote
    fn render_tree_node(&self) -> Option<RenderNode> {
        if content_is_terminal_image(&self.left) || content_is_terminal_image(&self.right) {
            return Some(RenderNode::unsupported("two-column terminal image"));
        }

        let left_nodes = project_column(&self.left);
        let right_nodes = project_column(&self.right);

        let left_count = left_nodes.len();
        let mut children = left_nodes;
        children.extend(right_nodes);

        let mut container = RenderNode::block_quote(children);
        container.attrs.set_columns_hints(&ColumnsHints {
            gap: self.gap,
            left_width: column_width_kind(self.left_width),
            left_count,
            stack_below: true,
        });
        if self.layout != Layout::default() {
            container.attrs.set_layout(&self.layout);
        }
        Some(container)
    }

    fn layout(&self) -> &Layout {
        &self.layout
    }

    fn layout_mut(&mut self) -> &mut Layout {
        &mut self.layout
    }

    fn render_optimistic(&self, term_width: Option<u32>) -> String {
        let width = term_width.unwrap_or(80);
        self.render_with_width(width, None)
    }
}

/// Returns `true` when `content` is a [`TerminalImage`] component.
fn content_is_terminal_image(content: &RenderableTerminalContent) -> bool {
    matches!(
        content,
        RenderableTerminalContent::Component(component)
            if component.as_any().downcast_ref::<TerminalImage>().is_some()
    )
}

/// Maps a bespoke [`ColumnWidth`] to the render-tree [`ColumnWidthKind`].
fn column_width_kind(width: ColumnWidth) -> ColumnWidthKind {
    match width {
        ColumnWidth::Fixed(chars) => ColumnWidthKind::Fixed(chars),
        ColumnWidth::Percent(percent) => ColumnWidthKind::Percent(percent),
    }
}

/// Projects a column's content into block-level render-tree nodes.
///
/// Inline-only projected content is wrapped in a [`RenderNode::paragraph`] so
/// the enclosing block quote carries valid block children.
fn project_column(content: &RenderableTerminalContent) -> Vec<RenderNode> {
    let mut ctx = TreeProjectionContext::default();
    let nodes = content.to_tree_nodes(&mut ctx).nodes;

    let mut inline_run: Vec<RenderNode> = Vec::new();
    let mut blocks: Vec<RenderNode> = Vec::new();
    for node in nodes {
        if is_inline_node(&node) {
            inline_run.push(node);
        } else {
            if !inline_run.is_empty() {
                blocks.push(RenderNode::paragraph(std::mem::take(&mut inline_run)));
            }
            blocks.push(node);
        }
    }
    if !inline_run.is_empty() {
        blocks.push(RenderNode::paragraph(inline_run));
    }
    blocks
}

/// Returns `true` when `node` is an inline-level (phrasing) render-tree node.
fn is_inline_node(node: &RenderNode) -> bool {
    use renderable::tree::NodeKind;
    matches!(
        node.kind,
        NodeKind::Text { .. }
            | NodeKind::Emphasis { .. }
            | NodeKind::Strong { .. }
            | NodeKind::Delete { .. }
            | NodeKind::Span { .. }
            | NodeKind::InlineCode { .. }
            | NodeKind::Link { .. }
            | NodeKind::Image { .. }
            | NodeKind::FootnoteReference { .. }
            | NodeKind::SoftBreak
            | NodeKind::HardBreak
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_side_by_side_balanced() {
        let two = TwoColumn::new("Left", "Right");
        let result = two.render_optimistic(Some(20));
        assert_eq!(result, "Left       Right    ");
    }

    #[test]
    fn respects_custom_ratio_and_height_padding() {
        let two = TwoColumn::new("Left line\nLeft two", "Right").with_left_percent(0.7);
        let rendered = two.render_optimistic(Some(30));
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
        let rendered = two.render_optimistic(Some(1));
        assert_eq!(rendered, "L\nR");
    }

    #[test]
    fn is_block_level_component() {
        let two = TwoColumn::new("L", "R");
        assert!(two.is_block_level());
    }

    #[test]
    fn two_column_render_tree_node_carries_layout_when_margins_set() {
        use crate::utils::layout::{Length, Margin};
        let mut two = TwoColumn::new("L", "R");
        two.layout_mut().margin = Margin::x(Length::ch(2));
        let node = two.render_tree_node().unwrap();
        assert!(node.attrs.layout().is_some());
    }
}
