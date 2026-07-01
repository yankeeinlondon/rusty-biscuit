use crate::{
    components::renderable::{RenderableTerminalContent, TerminalRenderable},
    terminal::Terminal,
    utils::{block_constraint::visible_width, layout::Layout},
};

/// Pads content on the left with spaces to guarantee a minimum width.
///
/// `PadLeft` right-aligns the content within the padded region: spaces
/// are added **before** the content so that short strings appear
/// flush-right inside the field.
///
/// ## Layout & Style Contract
///
/// `PadLeft` is an inline padding utility (spec C7). The `min_width`
/// argument is its explicit contract and is always honored. `Layout` box
/// properties (`margin`, `padding`, `width`, `max_width`, `alignment`) are
/// **N/A** — the containing block owns the box. `Style::color` and
/// `Style::emphasis` flow through when `content` is a styled component such
/// as [`Prose`]. `Style::background` and `Style::border` are N/A for the
/// pad utility itself.
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::prelude::*;
///
/// // Pad a plain string to at least 10 characters
/// let padded = PadLeft::new("hello", 10);
/// assert_eq!(padded.render_optimistic(Some(80)), "     hello");
///
/// // Content longer than min_width passes through unchanged
/// let padded = PadLeft::new("hello world", 5);
/// assert_eq!(padded.render_optimistic(Some(80)), "hello world");
///
/// // Truncate to exactly 10 characters (removes from the left/start)
/// let padded = PadLeft::new("hello world", 10).truncate();
/// assert_eq!(padded.render_optimistic(Some(80)), "ello world");
/// ```
#[derive(Debug, Clone)]
pub struct PadLeft {
    content: RenderableTerminalContent,
    min_width: u32,
    truncate: bool,
    layout: Layout,
}

impl PadLeft {
    /// Create a new `PadLeft` that pads content to at least `min_width` characters.
    pub fn new<T: Into<RenderableTerminalContent>>(content: T, min_width: u32) -> Self {
        Self {
            content: content.into(),
            min_width,
            truncate: false,
            layout: Layout::default(),
        }
    }

    /// Enable truncation so the output is exactly `min_width` characters.
    ///
    /// When the content exceeds `min_width`, characters are removed from
    /// the **start** (left side) of the visible text, preserving the
    /// right-aligned appearance.
    pub fn truncate(mut self) -> Self {
        self.truncate = true;
        self
    }

    fn pad_content(&self, rendered: &str) -> String {
        let width = visible_width(rendered);

        if width >= self.min_width {
            if self.truncate {
                truncate_left(rendered, self.min_width)
            } else {
                rendered.to_string()
            }
        } else {
            let padding = self.min_width - width;
            format!("{}{}", " ".repeat(padding as usize), rendered)
        }
    }
}

impl TerminalRenderable for PadLeft {
    fn render_optimistic(&self, term_width: Option<u32>) -> String {
        let rendered = match &self.content {
            RenderableTerminalContent::String(s) => s.clone(),
            RenderableTerminalContent::Component(c) => c.render_optimistic(term_width),
        };
        self.pad_content(&rendered)
    }

    fn render(&self, term: &Terminal) -> String {
        let rendered = match &self.content {
            RenderableTerminalContent::String(s) => s.clone(),
            RenderableTerminalContent::Component(c) => c.render(term),
        };
        self.pad_content(&rendered)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn layout(&self) -> &Layout {
        &self.layout
    }

    fn layout_mut(&mut self) -> &mut Layout {
        &mut self.layout
    }
}

/// Pads content on the right with spaces to guarantee a minimum width.
///
/// `PadRight` left-aligns the content within the padded region: spaces
/// are added **after** the content so that short strings appear
/// flush-left inside the field.
///
/// ## Layout & Style Contract
///
/// `PadRight` is an inline padding utility (spec C7). The `min_width`
/// argument is its explicit contract and is always honored. `Layout` box
/// properties (`margin`, `padding`, `width`, `max_width`, `alignment`) are
/// **N/A** — the containing block owns the box. `Style::color` and
/// `Style::emphasis` flow through when `content` is a styled component such
/// as [`Prose`]. `Style::background` and `Style::border` are N/A for the
/// pad utility itself.
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::prelude::*;
///
/// // Pad a plain string to at least 10 characters
/// let padded = PadRight::new("hello", 10);
/// assert_eq!(padded.render_optimistic(Some(80)), "hello     ");
///
/// // Content longer than min_width passes through unchanged
/// let padded = PadRight::new("hello world", 5);
/// assert_eq!(padded.render_optimistic(Some(80)), "hello world");
///
/// // Truncate to exactly 10 characters (removes from the right/end)
/// let padded = PadRight::new("hello world", 10).truncate();
/// assert_eq!(padded.render_optimistic(Some(80)), "hello worl");
/// ```
#[derive(Debug, Clone)]
pub struct PadRight {
    content: RenderableTerminalContent,
    min_width: u32,
    truncate: bool,
    layout: Layout,
}

impl PadRight {
    /// Create a new `PadRight` that pads content to at least `min_width` characters.
    pub fn new<T: Into<RenderableTerminalContent>>(content: T, min_width: u32) -> Self {
        Self {
            content: content.into(),
            min_width,
            truncate: false,
            layout: Layout::default(),
        }
    }

    /// Enable truncation so the output is exactly `min_width` characters.
    ///
    /// When the content exceeds `min_width`, characters are removed from
    /// the **end** (right side) of the visible text, preserving the
    /// left-aligned appearance.
    pub fn truncate(mut self) -> Self {
        self.truncate = true;
        self
    }

    fn pad_content(&self, rendered: &str) -> String {
        let width = visible_width(rendered);

        if width >= self.min_width {
            if self.truncate {
                truncate_right(rendered, self.min_width)
            } else {
                rendered.to_string()
            }
        } else {
            let padding = self.min_width - width;
            format!("{}{}", rendered, " ".repeat(padding as usize))
        }
    }
}

impl TerminalRenderable for PadRight {
    fn render_optimistic(&self, term_width: Option<u32>) -> String {
        let rendered = match &self.content {
            RenderableTerminalContent::String(s) => s.clone(),
            RenderableTerminalContent::Component(c) => c.render_optimistic(term_width),
        };
        self.pad_content(&rendered)
    }

    fn render(&self, term: &Terminal) -> String {
        let rendered = match &self.content {
            RenderableTerminalContent::String(s) => s.clone(),
            RenderableTerminalContent::Component(c) => c.render(term),
        };
        self.pad_content(&rendered)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn layout(&self) -> &Layout {
        &self.layout
    }

    fn layout_mut(&mut self) -> &mut Layout {
        &mut self.layout
    }
}

/// Truncate from the left (start) to fit within `max_width` visible characters.
///
/// Preserves ANSI escape sequences that are still active at the truncation
/// point. When the content contains escape codes, this walks from the start
/// and drops visible characters until only `max_width` remain.
fn truncate_left(content: &str, max_width: u32) -> String {
    use crate::utils::block_constraint::split_at_visible_width;

    let width = visible_width(content);
    if width <= max_width {
        return content.to_string();
    }

    let drop = width - max_width;
    let (_, tail) = split_at_visible_width(content, drop);
    tail
}

/// Truncate from the right (end) to fit within `max_width` visible characters.
///
/// Preserves ANSI escape sequences. Only visible characters beyond
/// `max_width` are removed.
fn truncate_right(content: &str, max_width: u32) -> String {
    use crate::utils::block_constraint::split_at_visible_width;

    let width = visible_width(content);
    if width <= max_width {
        return content.to_string();
    }

    let (head, _) = split_at_visible_width(content, max_width);
    head
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::prose::Prose;

    // ── PadRight ──────────────────────────────────────────────

    #[test]
    fn pad_right_adds_spaces() {
        let p = PadRight::new("hi", 5);
        assert_eq!(p.render_optimistic(Some(80)), "hi   ");
    }

    #[test]
    fn pad_right_no_padding_when_exact() {
        let p = PadRight::new("hello", 5);
        assert_eq!(p.render_optimistic(Some(80)), "hello");
    }

    #[test]
    fn pad_right_no_padding_when_longer() {
        let p = PadRight::new("hello world", 5);
        assert_eq!(p.render_optimistic(Some(80)), "hello world");
    }

    #[test]
    fn pad_right_truncate_cuts_end() {
        let p = PadRight::new("hello world", 5).truncate();
        assert_eq!(p.render_optimistic(Some(80)), "hello");
    }

    #[test]
    fn pad_right_truncate_pads_short() {
        let p = PadRight::new("hi", 5).truncate();
        assert_eq!(p.render_optimistic(Some(80)), "hi   ");
    }

    // ── PadLeft ───────────────────────────────────────────────

    #[test]
    fn pad_left_adds_spaces() {
        let p = PadLeft::new("hi", 5);
        assert_eq!(p.render_optimistic(Some(80)), "   hi");
    }

    #[test]
    fn pad_left_no_padding_when_exact() {
        let p = PadLeft::new("hello", 5);
        assert_eq!(p.render_optimistic(Some(80)), "hello");
    }

    #[test]
    fn pad_left_no_padding_when_longer() {
        let p = PadLeft::new("hello world", 5);
        assert_eq!(p.render_optimistic(Some(80)), "hello world");
    }

    #[test]
    fn pad_left_truncate_cuts_start() {
        let p = PadLeft::new("hello world", 5).truncate();
        assert_eq!(p.render_optimistic(Some(80)), "world");
    }

    #[test]
    fn pad_left_truncate_pads_short() {
        let p = PadLeft::new("hi", 5).truncate();
        assert_eq!(p.render_optimistic(Some(80)), "   hi");
    }

    // ── With Prose content ────────────────────────────────────

    #[test]
    fn pad_right_with_prose() {
        let prose = Prose::new("hi");
        let p = PadRight::new(prose, 10);
        let output = p.render_optimistic(Some(80));
        // Prose renders "hi" (with reset), visible width is 2, padded to 10
        let width = visible_width(&output);
        assert_eq!(width, 10);
        // Should end with spaces
        assert!(output.ends_with("        "));
    }

    #[test]
    fn pad_left_with_prose() {
        let prose = Prose::new("hi");
        let p = PadLeft::new(prose, 10);
        let output = p.render_optimistic(Some(80));
        let width = visible_width(&output);
        assert_eq!(width, 10);
        // Should start with spaces
        assert!(output.starts_with("        "));
    }

    // ── ANSI-aware truncation ─────────────────────────────────

    #[test]
    fn truncate_right_preserves_ansi() {
        let styled = "\x1b[1mhello world\x1b[0m";
        let result = truncate_right(styled, 5);
        assert_eq!(visible_width(&result), 5);
    }

    #[test]
    fn truncate_left_preserves_ansi() {
        let styled = "\x1b[1mhello world\x1b[0m";
        let result = truncate_left(styled, 5);
        assert_eq!(visible_width(&result), 5);
    }

    // ── Edge cases ────────────────────────────────────────────

    #[test]
    fn pad_right_zero_width() {
        let p = PadRight::new("hello", 0);
        assert_eq!(p.render_optimistic(Some(80)), "hello");
    }

    #[test]
    fn pad_left_zero_width() {
        let p = PadLeft::new("hello", 0);
        assert_eq!(p.render_optimistic(Some(80)), "hello");
    }

    #[test]
    fn pad_right_empty_content() {
        let p = PadRight::new("", 5);
        assert_eq!(p.render_optimistic(Some(80)), "     ");
    }

    #[test]
    fn pad_left_empty_content() {
        let p = PadLeft::new("", 5);
        assert_eq!(p.render_optimistic(Some(80)), "     ");
    }

    #[test]
    fn pad_right_truncate_exact_width() {
        let p = PadRight::new("hello", 5).truncate();
        assert_eq!(p.render_optimistic(Some(80)), "hello");
    }

    #[test]
    fn pad_left_truncate_exact_width() {
        let p = PadLeft::new("hello", 5).truncate();
        assert_eq!(p.render_optimistic(Some(80)), "hello");
    }
}
