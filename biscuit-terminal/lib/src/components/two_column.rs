use renderable::browser::PageOptions;
use renderable::browser::fragment::{BrowserFragment, Ready};
use renderable::html::HtmlPage;
use renderable::markdown::MarkdownRenderable;
use renderable::style::Style;
use renderable::tree::render::{
    BrowserRenderOptions, MarkdownDialect, MarkdownRenderOptions, render_browser_node,
    render_markdown_node,
};
use renderable::tree::{
    ColumnWidthKind, ColumnsHints, NodeKind, RenderNode, RenderStrictness, TreeRenderable,
};

use crate::discovery::detection::TerminalApp;
use crate::prelude::*;
use crate::render_tree::projection::TreeProjectionContext;
use crate::render_tree::{TerminalRenderOptions, render_terminal_node};
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
///
/// ## Layout & Style Contract
///
/// `TwoColumn` is an internal-layout component (spec C2). The shared
/// render-tree fold resolves the outer box from [`Layout::width`], then
/// `render_columns` pads both columns to fill the resolved content width:
///
/// - [`Width::Auto`] (default) and [`Width::Fixed`] **fill** the available
///   width; [`Width::FitContent`] is observationally identical to `Auto` on
///   the terminal target (the box always pads to the handed width).
/// - **Slack sink** (spec D2): the RIGHT column absorbs the slack after
///   honoring the explicit/fractional left width and the gap. The left
///   column's width is stable so its content does not reflow when the
///   terminal widens; the right column flexes.
/// - A fractional `Fixed(50%)` is resolved exactly once by the fold; the
///   column planner never re-resolves the raw percentage (the
///   `Fixed(50%) → 25%` double-application bug).
/// - The reconstructed [`ColumnsHints`] round-trip carries the projected
///   [`Layout`] onto the carrier node (C4), so a second render pass honors
///   the same outer-box contract.
///
/// [`Width::Auto`]: renderable::layout::Width::Auto
/// [`Width::Fixed`]: renderable::layout::Width::Fixed
/// [`Width::FitContent`]: renderable::layout::Width::FitContent
#[derive(Debug, Clone)]
pub struct TwoColumn {
    left: RenderableTerminalContent,
    right: RenderableTerminalContent,
    left_width: ColumnWidth,
    gap: u32,
    layout: Layout,
    style: Style,
}

impl Default for TwoColumn {
    fn default() -> Self {
        TwoColumn {
            left: "".into(),
            right: "".into(),
            left_width: ColumnWidth::Percent(0.5),
            gap: DEFAULT_GAP,
            layout: Layout::default(),
            style: Style::default(),
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
            style: Style::default(),
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

    /// Projects this two-column layout into the canonical render tree.
    ///
    /// Returns a [`NodeKind::BlockQuote`] carrier whose `attrs` hold
    /// [`ColumnsHints`] and (optionally) [`Layout`]. The flat child list
    /// contains the left column's block-projected children followed by the
    /// right column's; `ColumnsHints::left_count` records the split. All three
    /// tree renderers special-case the columns hint so the quote border never
    /// renders.
    ///
    /// When either column is a [`TerminalImage`], the projection is
    /// [`RenderNode::unsupported`] — cursor-overlay image rendering has no
    /// render-tree representation. Callers must detect this and fall back to
    /// the bespoke terminal path; Browser and MarkdownPlus surface the
    /// standard unsupported behavior according to strictness.
    ///
    /// ## Notes
    ///
    /// The fold resolves the outer box from [`Layout::width`], then pads both
    /// columns to fill the resolved content width. The RIGHT column is the
    /// documented slack sink for `Width::Auto` and `Width::Fixed` fill
    /// (spec decision D2). On the terminal target, `Width::FitContent` is
    /// observationally identical to `Width::Auto`: the box always pads to fill
    /// the handed width, so terminal FitContent does not hug.
    fn to_render_node(&self) -> RenderNode {
        if content_is_terminal_image(&self.left) || content_is_terminal_image(&self.right) {
            return RenderNode::unsupported("two-column terminal image");
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
        crate::components::renderable::overlay_style_onto_node(&mut container, &self.style);
        container
    }

    /// Renders the two-column layout through the canonical render tree.
    ///
    /// Used by the [`TerminalRenderable`] impl to route Terminal output
    /// through the same tree the Browser and Markdown paths consume. When the
    /// projection is `Unsupported` (a column holds a [`TerminalImage`]) or
    /// when the tree render fails, falls back to [`Self::render_bespoke`] so
    /// the user-facing path never emits a sentinel or empty output for the
    /// image-overlay scenario.
    ///
    /// Tree-render failures other than the image-overlay fallback are logged
    /// via `tracing::error!`: the [`TerminalRenderable::render`] contract is
    /// infallible, and falling back to the bespoke renderer preserves output
    /// fidelity.
    fn render_via_tree(&self, term: &Terminal) -> String {
        let node = self.to_render_node();
        if matches!(node.kind, NodeKind::Unsupported { .. }) {
            return self.render_bespoke(term);
        }
        let opts = TerminalRenderOptions::new(term, RenderStrictness::Warn);
        match render_terminal_node(&node, &opts) {
            Ok(rendered) => rendered.output,
            Err(error) => {
                tracing::error!(
                    component = "TwoColumn",
                    error = %error,
                    "render_terminal_node failed; falling back to bespoke render",
                );
                self.render_bespoke(term)
            }
        }
    }

    /// Renders the two-column layout through the canonical render tree at an
    /// explicit terminal width.
    ///
    /// Parallels [`Self::render_via_tree`] for the
    /// [`TerminalRenderable::render_optimistic`] path; the bespoke fallback
    /// here is [`Self::render_bespoke_optimistic`].
    fn render_via_tree_optimistic(&self, term_width: Option<u32>) -> String {
        let node = self.to_render_node();
        if matches!(node.kind, NodeKind::Unsupported { .. }) {
            return self.render_bespoke_optimistic(term_width);
        }
        let term = Terminal::new_optimistic(term_width.unwrap_or(80));
        let opts = TerminalRenderOptions::new(&term, RenderStrictness::Warn);
        match render_terminal_node(&node, &opts) {
            Ok(rendered) => rendered.output,
            Err(error) => {
                tracing::error!(
                    component = "TwoColumn",
                    error = %error,
                    "render_terminal_node failed; falling back to bespoke optimistic render",
                );
                self.render_bespoke_optimistic(term_width)
            }
        }
    }

    /// Renders via the sanctioned bespoke escape hatch.
    ///
    /// Retained as a `#[doc(hidden)]` surface because [`TwoColumn`] supports
    /// inline [`TerminalImage`] overlays via cursor positioning — a
    /// capability the render tree cannot represent, where the tree
    /// projection falls back to an `Unsupported` node. The active
    /// [`TerminalRenderable::render`] path delegates to
    /// [`Self::render_via_tree`] and routes through this method only when a
    /// column holds a terminal image.
    ///
    /// ## Notes
    ///
    /// `#[doc(hidden)]` because this is an internal escape hatch, not part
    /// of the public surface; `pub` so integration parity tests can reach
    /// it. Removing this without first adding equivalent terminal-image /
    /// cursor-overlay capability to the render tree is a regression.
    #[doc(hidden)]
    pub fn render_bespoke(&self, term: &Terminal) -> String {
        let width = term.width();
        self.render_with_width(width, Some(term))
    }

    /// Renders the sanctioned bespoke escape hatch at an explicit width.
    ///
    /// Companion to [`Self::render_bespoke`] for the
    /// [`TerminalRenderable::render_optimistic`] path, retained for the same
    /// terminal-image-overlay reason: the render tree cannot represent
    /// cursor-positioned image overlays inside a two-column layout.
    ///
    /// ## Notes
    ///
    /// `#[doc(hidden)]` because this is an internal escape hatch, not part
    /// of the public surface; `pub` so integration parity tests can reach
    /// it. Removing this without first adding equivalent terminal-image /
    /// cursor-overlay capability to the render tree is a regression.
    #[doc(hidden)]
    pub fn render_bespoke_optimistic(&self, term_width: Option<u32>) -> String {
        let width = term_width.unwrap_or(80);
        self.render_with_width(width, None)
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

    /// Renders to the supplied terminal.
    ///
    /// Routes through the canonical render tree via [`Self::render_via_tree`].
    /// The legacy bespoke output is retained on [`Self::render_bespoke`] for
    /// parity testing and as the fallback when a column holds a
    /// [`TerminalImage`] (cursor overlay has no render-tree representation).
    fn render(&self, term: &Terminal) -> String {
        self.render_via_tree(term)
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
    /// Delegates to the single private projection helper
    /// [`Self::to_render_node`], shared with [`TreeRenderable::render_tree`]
    /// so the terminal compatibility hook and the canonical tree producer
    /// cannot drift.
    ///
    /// A column whose content is a [`TerminalImage`] cannot be projected —
    /// cursor-overlay image rendering has no render-tree representation — so
    /// the helper returns a [`RenderNode::unsupported`] node instead.
    ///
    /// [`NodeKind::BlockQuote`]: renderable::tree::NodeKind::BlockQuote
    fn render_tree_node(&self) -> Option<RenderNode> {
        Some(self.to_render_node())
    }

    fn layout(&self) -> &Layout {
        &self.layout
    }

    fn layout_mut(&mut self) -> &mut Layout {
        &mut self.layout
    }

    fn style(&self) -> Style {
        self.style.clone()
    }

    fn style_mut(&mut self) -> Option<&mut Style> {
        Some(&mut self.style)
    }

    /// Renders to a terminal string at an explicit width.
    ///
    /// Routes through the canonical render tree via
    /// [`Self::render_via_tree_optimistic`] so terminal output matches the
    /// Browser and Markdown paths for the same component.
    fn render_optimistic(&self, term_width: Option<u32>) -> String {
        self.render_via_tree_optimistic(term_width)
    }
}

impl TreeRenderable for TwoColumn {
    /// Projects the two-column layout into the canonical render tree.
    ///
    /// Delegates to the single private projection helper
    /// [`TwoColumn::to_render_node`] so this canonical entry point and the
    /// terminal-compatibility [`TerminalRenderable::render_tree_node`] hook
    /// share one source of truth.
    ///
    /// The projected tree is a [`NodeKind::BlockQuote`] carrying
    /// [`ColumnsHints`] on its `attrs`. The flat child list holds the left
    /// column's blocks followed by the right column's, split at
    /// [`ColumnsHints::left_count`]. The terminal, browser, and MarkdownPlus
    /// renderers special-case the columns hint to produce the side-by-side
    /// layout from the same canonical tree; portable Markdown necessarily
    /// loses side-by-side and renders the columns sequentially.
    ///
    /// [`NodeKind::BlockQuote`]: renderable::tree::NodeKind::BlockQuote
    fn render_tree(&self) -> RenderNode {
        self.to_render_node()
    }
}

impl MarkdownRenderable for TwoColumn {
    /// Renders the two-column layout as portable Markdown via the canonical
    /// render tree.
    ///
    /// Portable Markdown has no side-by-side layout, so the columns collapse
    /// to sequential blocks (left column, blank line, right column). Width,
    /// gap, and column alignment are intentionally dropped because no
    /// portable CommonMark form preserves them.
    fn render_markdown(&self) -> String {
        let node = <Self as TreeRenderable>::render_tree(self);
        match render_markdown_node(&node, &MarkdownRenderOptions::default()) {
            Ok(rendered) => rendered.output,
            Err(error) => {
                tracing::error!(
                    component = "TwoColumn",
                    dialect = "Markdown",
                    error = %error,
                    "render_markdown_node failed; emitting empty output",
                );
                String::new()
            }
        }
    }

    /// Renders the two-column layout as MarkdownPlus via the canonical render
    /// tree.
    ///
    /// MarkdownPlus allows inline HTML, so the MarkdownPlus dialect of the
    /// tree renderer emits the same CSS flex shape used by the browser
    /// renderer (RT-TWOCOLUMN-002): an outer
    /// `<div class="columns" style="display:flex;gap:{gap}ch">` with two
    /// `<div class="column">` children. The left column carries the width
    /// CSS from [`ColumnsHints::left_width`]; the right column flexes to
    /// fill.
    fn render_markdown_plus(&self) -> String {
        let node = <Self as TreeRenderable>::render_tree(self);
        let opts = MarkdownRenderOptions {
            dialect: MarkdownDialect::MarkdownPlus,
            ..MarkdownRenderOptions::default()
        };
        match render_markdown_node(&node, &opts) {
            Ok(rendered) => rendered.output,
            Err(error) => {
                tracing::error!(
                    component = "TwoColumn",
                    dialect = "MarkdownPlus",
                    error = %error,
                    "render_markdown_node failed; emitting empty output",
                );
                String::new()
            }
        }
    }
}

impl BrowserRenderable for TwoColumn {
    /// Renders the two-column layout as an HTML fragment via the canonical
    /// render tree.
    ///
    /// The browser tree renderer (RT-TWOCOLUMN-001) emits a
    /// `<div class="columns">` flex container with inline `display:flex` and
    /// `gap:{gap}ch` CSS, holding two `<div class="column">` children. The
    /// left column carries the width CSS from
    /// [`ColumnsHints::left_width`]; the right column flexes to fill. Node
    /// [`Layout`] is merged into the container `style` without overwriting
    /// the flex/gap declarations.
    ///
    /// Failures are logged via `tracing::error!` and fall back to an empty
    /// [`BrowserFragment`]: the [`BrowserRenderable`] contract is infallible,
    /// and surfacing a `[render-tree error: …]` sentinel as in-band HTML
    /// would pollute the rendered page.
    fn render_html_fragment(&self) -> BrowserFragment<Ready> {
        let node = <Self as TreeRenderable>::render_tree(self);
        let opts = BrowserRenderOptions::default();
        match render_browser_node(&node, &opts) {
            Ok(rendered) => rendered.output,
            Err(error) => {
                tracing::error!(
                    component = "TwoColumn",
                    error = %error,
                    "render_browser_node failed; emitting empty fragment",
                );
                BrowserFragment::new()
                    .define_as_text_fragment(String::new())
                    .finalize()
            }
        }
    }

    fn render_html_page(&self, page: Option<PageOptions>) -> HtmlPage {
        let mut html_page = HtmlPage::from(self.render_html_fragment());
        if let Some(options) = page {
            html_page.apply_page_options(options);
        }
        html_page
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
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
        use crate::utils::layout::{Length, Edges};
        let mut two = TwoColumn::new("L", "R");
        two.layout_mut().margin = Edges::x(Length::ch(2));
        let node = two.render_tree_node().unwrap();
        assert!(node.attrs.layout().is_some());
    }
}
