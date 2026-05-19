use std::rc::Rc;

use renderable::layout::TargetValue;
use renderable::style::{Border, BorderSides, PerMode, Style};
use renderable::target::RenderTarget;

use crate::{
    components::{
        prose::Prose,
        renderable::{RenderableTerminalContent, TerminalRenderable},
    },
    terminal::Terminal,
    utils::{
        block_constraint::{split_lines, visible_width, wrap_lines},
        color::{Color, Tailwind},
        layout::{Layout, LayoutTerminalExt},
    },
};

/// Wraps a plain [`Color`] as a universal [`Style`] color value.
fn universal_color(color: Color) -> TargetValue<PerMode<Color>> {
    TargetValue::universal(PerMode::universal(color))
}

/// Extracts the terminal [`Color`] from an optional `Style` color slot.
///
/// A universal value resolves directly; an adaptive value resolves to its
/// dark-mode branch — the migrated builder shims only ever store universal
/// values, so this is a lossless inverse for the compatibility path.
fn color_of(slot: &Option<TargetValue<PerMode<Color>>>) -> Option<Color> {
    match slot.as_ref()?.resolve(RenderTarget::Terminal)? {
        PerMode::Universal(color) => Some(*color),
        PerMode::Adaptive { dark, .. } => Some(*dark),
    }
}

/// Builds a left-only [`Border`] carrying `color`.
fn left_border(color: Color) -> Border {
    Border {
        color: Some(universal_color(color)),
        sides: BorderSides::Sides {
            top: false,
            right: false,
            bottom: false,
            left: true,
        },
        ..Border::default()
    }
}

/// Renders quoted text with a distinctive left border.
///
/// A block quote displays text with a visual border on the left side,
/// typically used to highlight quoted content, testimonials, or
/// notable passages in a visually distinct manner.
///
/// ## Structure
///
/// - Each line is prefixed with `│ ` (U+2502 + space)
/// - Optional attribution can be added at the bottom preceded by `—`
/// - Content is automatically wrapped to fit the terminal width
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::components::block_quote::BlockQuote;
/// use biscuit_terminal::components::renderable::TerminalRenderable;
///
/// // Simple block quote from a string
/// let quote = BlockQuote::from("The only way to do great work is to love what you do.");
/// let result = quote.render_optimistic(Some(60));
/// assert!(result.contains("The only way"));
/// assert!(result.contains("│")); // colored border present
///
/// // Block quote with attribution
/// let quote = BlockQuote::new(
///     "To be, or not to be, that is the question.".into(),
///     Some("William Shakespeare")
/// );
/// let result = quote.render_optimistic(None);
/// assert!(result.contains("To be, or not to be"));
/// assert!(result.contains("— William Shakespeare"));
///
/// // Styled block quote with custom colors
/// use biscuit_terminal::utils::color::{Color, Tailwind};
/// let quote = BlockQuote::from("Important quote")
///     .with_text_color(Color::Tailwind(Tailwind::White))
///     .with_bg_color(Color::Tailwind(Tailwind::Gray800))
///     .with_left_block_color(Color::Tailwind(Tailwind::Blue400));
/// let result = quote.render_optimistic(None);
/// assert!(result.contains("Important quote"));
///
/// // Block quote from Prose component (rich text)
/// use biscuit_terminal::components::prose::Prose;
/// use biscuit_terminal::components::renderable::RenderableTerminalContent;
/// let prose = Prose::new("This is <b>bold</b> and <i>italic</i> text.");
/// let quote = BlockQuote::new(
///     RenderableTerminalContent::from(prose),
///     Some("Anonymous")
/// );
/// let result = quote.render_optimistic(None);
/// assert!(result.contains("│"));
/// assert!(result.contains("— Anonymous"));
/// ```
#[derive(Debug, Clone)]
pub struct BlockQuote {
    /// the content being wrapped in the block quote
    content: RenderableTerminalContent,

    /// if the quote is being attributed to someone
    /// you can add that and it will be placed
    attribution: Option<String>,

    /// Declared appearance — the migrated home of the former `text_color`,
    /// `bg_color`, and `left_block_color` fields. Foreground color rides on
    /// [`Style::color`], background on [`Style::background`], and the left
    /// border color on [`Style::border`].
    style: Style,
    /// The border string prefixed to each line (default: `"│ "`).
    border: String,
    layout: Layout,
}

impl Default for BlockQuote {
    fn default() -> Self {
        use crate::utils::wrap_policy::WordWrap;
        BlockQuote {
            content: RenderableTerminalContent::String("".to_string()),
            attribution: None,
            style: Style {
                border: Some(left_border(Color::Tailwind(Tailwind::Gray500))),
                ..Style::default()
            },
            border: "│ ".to_string(),
            layout: Layout {
                word_wrap: WordWrap::WrapProse(Some(8), None),
                ..Layout::default()
            },
        }
    }
}

impl From<String> for BlockQuote {
    fn from(value: String) -> Self {
        BlockQuote::new(RenderableTerminalContent::String(value), None::<String>)
    }
}

impl From<&String> for BlockQuote {
    fn from(value: &String) -> Self {
        BlockQuote::new(
            RenderableTerminalContent::String(value.into()),
            None::<String>,
        )
    }
}

impl From<&str> for BlockQuote {
    fn from(value: &str) -> Self {
        BlockQuote::new(
            RenderableTerminalContent::String(value.into()),
            None::<String>,
        )
    }
}

impl From<Prose> for BlockQuote {
    fn from(value: Prose) -> Self {
        BlockQuote::new(
            RenderableTerminalContent::Component(Rc::new(value)),
            None::<String>,
        )
    }
}

impl From<&Prose> for BlockQuote {
    fn from(value: &Prose) -> Self {
        BlockQuote::new(
            RenderableTerminalContent::Component(Rc::new((*value).clone())),
            None::<String>,
        )
    }
}

impl BlockQuote {
    /// Create a block quote by passing in the content and _optionally_ an attribution.
    pub fn new<U: Into<String>>(
        content: RenderableTerminalContent,
        attribution: Option<U>,
    ) -> Self {
        Self {
            content,
            attribution: attribution.map(|attribution| attribution.into()),
            ..BlockQuote::default()
        }
    }

    /// Set the text color.
    ///
    /// Compatibility shim: the color is stored on the declared [`Style`]'s
    /// [`color`](Style::color) slot.
    pub fn with_text_color(mut self, color: Color) -> Self {
        self.style.color = Some(universal_color(color));
        self
    }

    /// Set the background color.
    ///
    /// Compatibility shim: the color is stored on the declared [`Style`]'s
    /// [`background`](Style::background) slot.
    pub fn with_bg_color(mut self, color: Color) -> Self {
        self.style.background = Some(universal_color(color));
        self
    }

    /// Set the left block/border color.
    ///
    /// Compatibility shim: the color is stored on the declared [`Style`]'s
    /// left [`border`](Style::border) slot.
    pub fn with_left_block_color(mut self, color: Color) -> Self {
        match &mut self.style.border {
            Some(border) => border.color = Some(universal_color(color)),
            None => self.style.border = Some(left_border(color)),
        }
        self
    }

    /// The declared text color, if any.
    pub fn text_color(&self) -> Option<Color> {
        color_of(&self.style.color)
    }

    /// The declared background color, if any.
    pub fn bg_color(&self) -> Option<Color> {
        color_of(&self.style.background)
    }

    /// The declared left block/border color, if any.
    pub fn left_block_color(&self) -> Option<Color> {
        self.style
            .border
            .as_ref()
            .and_then(|border| color_of(&border.color))
    }

    /// Set a custom border string (default: `"│ "`).
    ///
    /// The visible width of the border is subtracted from the available
    /// content width, so wider borders reduce the text area accordingly.
    pub fn with_border(mut self, border: impl Into<String>) -> Self {
        self.border = border.into();
        self
    }

    /// Render the block quote content with a left border.
    ///
    /// The border width is determined by the `border` field and the
    /// child content width is reduced accordingly.
    fn render_content(&self, term: Option<&Terminal>, term_width: u32) -> String {
        let border_width = visible_width(&self.border);
        let child_width = term_width.saturating_sub(border_width);

        // Render child content with constrained width
        let content: String = match &self.content {
            RenderableTerminalContent::String(s) => {
                // Wrap plain strings to child_width using the layout's word wrap strategy
                let lines = split_lines(s);
                let wrapped = wrap_lines(lines, &self.layout.word_wrap, child_width);
                wrapped.join("\n")
            }
            RenderableTerminalContent::Component(component) => {
                // Use render() with the constrained child width so nested
                // components respect the block quote's border
                term.map_or_else(
                    || component.render_optimistic(Some(child_width)),
                    |t| component.render_in_width(t, child_width),
                )
            }
        };

        // Build the colored border string
        let colored_border = self.colorize_border(&self.border);
        // Attribution border uses the first visible char only (no trailing space)
        let border_char = self.border.trim_end();
        let colored_border_char = self.colorize_border(border_char);

        let mut result = String::new();

        // Split content into lines and prefix each with the quote border
        for line in content.lines() {
            result.push_str(&colored_border);
            result.push_str(line);
            result.push('\n');
        }

        // Add attribution if present (with blank line separator)
        if let Some(ref attribution) = self.attribution {
            result.push_str(&colored_border_char);
            result.push('\n'); // blank line
            result.push_str(&colored_border_char);
            result.push_str(" — ");
            result.push_str(attribution);
            result.push('\n');
        }

        // Remove trailing newline
        if result.ends_with('\n') {
            result.pop();
        }

        result
    }

    /// Apply the declared left block color to a border string segment.
    fn colorize_border(&self, border: &str) -> String {
        match self.left_block_color() {
            Some(color) => match color.to_rgb() {
                Some((r, g, b)) => {
                    format!("\x1b[38;2;{r};{g};{b}m{border}\x1b[39m")
                }
                None => border.to_string(),
            },
            None => border.to_string(),
        }
    }
}

impl TerminalRenderable for BlockQuote {
    fn render_optimistic(&self, term_width: Option<u32>) -> String {
        let width = term_width.unwrap_or(80);
        let available = self.layout.available_width(width);
        let content = self.render_content(None, available);
        // BlockQuote has a `┃ ` border on every line — the border must form a
        // straight vertical bar, so the block must be aligned as a unit.
        self.layout.apply_block_layout(&content, width)
    }

    fn render(&self, term: &Terminal) -> String {
        let width = term.width();
        let available = self.layout.available_width(width);
        let content = self.render_content(Some(term), available);
        self.layout.apply_block_layout(&content, width)
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

    fn is_block_level(&self) -> bool {
        true
    }
}

impl BlockQuote {
    /// Extracts the plain text of the block quote content.
    ///
    /// For the [`RenderableTerminalContent::String`] variant the string is
    /// used verbatim. For the [`RenderableTerminalContent::Component`]
    /// variant the component is rendered optimistically and its ANSI escape
    /// sequences are stripped — this is a deliberately lossy projection, as
    /// the render tree captures document structure rather than terminal
    /// presentation.
    fn plain_text(&self) -> String {
        match &self.content {
            RenderableTerminalContent::String(s) => s.clone(),
            RenderableTerminalContent::Component(component) => {
                crate::utils::escape_codes::strip_escape_codes(component.render_optimistic(None))
            }
        }
    }
}

impl renderable::tree::TreeRenderable for BlockQuote {
    /// Projects the block quote into the canonical render tree as a
    /// [`NodeKind::BlockQuote`].
    ///
    /// The textual content becomes a single `Paragraph` of `Text`. When an
    /// attribution is present it is appended as a second `Paragraph`
    /// containing a single `Text` of the form `— {attribution}`.
    ///
    /// A non-default [`Layout`] (margin, alignment, max-width, word wrap) is
    /// recorded on the root node's attributes so the tree renderers apply it,
    /// matching the bespoke `render` path.
    ///
    /// The declared [`Style`] — the left border plus any text and background
    /// color set through the builder shims — is recorded on the node so the
    /// terminal tree renderer lowers the same appearance the bespoke renderer
    /// produces. The Markdown and Browser renderers ignore the style, so their
    /// output is unaffected.
    fn render_tree(&self) -> renderable::tree::RenderNode {
        use renderable::tree::RenderNode;

        let mut children = vec![RenderNode::paragraph(vec![RenderNode::text(
            self.plain_text(),
        )])];

        if let Some(attribution) = &self.attribution {
            children.push(RenderNode::paragraph(vec![RenderNode::text(format!(
                "— {attribution}"
            ))]));
        }

        let mut node = RenderNode::block_quote(children);
        // A block quote carries an intrinsic word-wrap default, so compare
        // against the component's own baseline rather than `Layout::default()`
        // — only a caller-customized layout is recorded on the node.
        if self.layout != Self::default().layout {
            node.attrs.set_layout(&self.layout);
        }
        if !self.style.is_empty() {
            node.attrs.set_style(&self.style);
        }
        node
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Strip ANSI escape sequences for test assertions.
    fn strip_ansi(s: &str) -> String {
        let mut result = String::with_capacity(s.len());
        let mut in_escape = false;
        for ch in s.chars() {
            if in_escape {
                if ch.is_ascii_alphabetic() {
                    in_escape = false;
                }
            } else if ch == '\x1b' {
                in_escape = true;
            } else {
                result.push(ch);
            }
        }
        result
    }

    // =========================================================================
    // Basic Construction Tests
    // =========================================================================

    #[test]
    fn test_simple_block_quote_from_str() {
        let quote = BlockQuote::from("Hello world");
        let result = quote.render_optimistic(None);
        assert_eq!(strip_ansi(&result), "│ Hello world");
    }

    #[test]
    fn test_simple_block_quote_from_string() {
        let quote = BlockQuote::from(String::from("Hello world"));
        let result = quote.render_optimistic(None);
        assert_eq!(strip_ansi(&result), "│ Hello world");
    }

    #[test]
    fn test_simple_block_quote_from_string_ref() {
        let content = String::from("Hello world");
        let quote = BlockQuote::from(&content);
        let result = quote.render_optimistic(None);
        assert_eq!(strip_ansi(&result), "│ Hello world");
    }

    #[test]
    fn test_block_quote_new_with_renderable_content() {
        let quote = BlockQuote::new(
            RenderableTerminalContent::from("Direct content"),
            None::<&str>,
        );
        let result = quote.render_optimistic(None);
        assert_eq!(strip_ansi(&result), "│ Direct content");
    }

    // =========================================================================
    // Multiline Content Tests
    // =========================================================================

    #[test]
    fn test_multiline_block_quote() {
        let quote = BlockQuote::new(
            RenderableTerminalContent::from("Line 1\nLine 2"),
            None::<&str>,
        );
        let result = quote.render_optimistic(None);
        assert_eq!(strip_ansi(&result), "│ Line 1\n│ Line 2");
    }

    #[test]
    fn test_multiline_block_quote_three_lines() {
        let quote = BlockQuote::from("First\nSecond\nThird");
        let result = quote.render_optimistic(None);
        assert_eq!(strip_ansi(&result), "│ First\n│ Second\n│ Third");
    }

    #[test]
    fn test_block_quote_with_empty_lines() {
        let quote = BlockQuote::from("Before\n\nAfter");
        let result = quote.render_optimistic(None);
        assert_eq!(strip_ansi(&result), "│ Before\n│ \n│ After");
    }

    // =========================================================================
    // Attribution Tests
    // =========================================================================

    #[test]
    fn test_block_quote_with_attribution() {
        let quote = BlockQuote::new(
            RenderableTerminalContent::from("To be or not to be"),
            Some("Shakespeare"),
        );
        let result = quote.render_optimistic(None);
        assert_eq!(
            strip_ansi(&result),
            "│ To be or not to be\n│\n│ — Shakespeare"
        );
    }

    #[test]
    fn test_block_quote_with_string_attribution() {
        let quote = BlockQuote::new(
            RenderableTerminalContent::from("The unexamined life is not worth living"),
            Some(String::from("Socrates")),
        );
        let result = quote.render_optimistic(None);
        assert_eq!(
            strip_ansi(&result),
            "│ The unexamined life is not worth living\n│\n│ — Socrates"
        );
    }

    #[test]
    fn test_multiline_with_attribution() {
        let quote = BlockQuote::new(
            RenderableTerminalContent::from("Line one\nLine two"),
            Some("Author"),
        );
        let result = quote.render_optimistic(None);
        assert_eq!(strip_ansi(&result), "│ Line one\n│ Line two\n│\n│ — Author");
    }

    // =========================================================================
    // Builder Pattern (with_*) Tests
    // =========================================================================

    #[test]
    fn test_with_text_color() {
        let quote =
            BlockQuote::from("Colored text").with_text_color(Color::Tailwind(Tailwind::Blue500));
        assert!(quote.text_color().is_some());
    }

    #[test]
    fn test_with_bg_color() {
        let quote =
            BlockQuote::from("Background").with_bg_color(Color::Tailwind(Tailwind::Gray100));
        assert!(quote.bg_color().is_some());
    }

    #[test]
    fn test_with_left_block_color() {
        let quote = BlockQuote::from("Custom border")
            .with_left_block_color(Color::Tailwind(Tailwind::Red500));
        assert_eq!(
            quote.left_block_color(),
            Some(Color::Tailwind(Tailwind::Red500))
        );
    }

    #[test]
    fn test_builder_chain() {
        let quote = BlockQuote::from("Full styling")
            .with_text_color(Color::Tailwind(Tailwind::White))
            .with_bg_color(Color::Tailwind(Tailwind::Gray800))
            .with_left_block_color(Color::Tailwind(Tailwind::Green500));

        assert!(quote.text_color().is_some());
        assert!(quote.bg_color().is_some());
        assert!(quote.left_block_color().is_some());
    }

    // =========================================================================
    // Default Tests
    // =========================================================================

    #[test]
    fn test_default_has_gray_left_block() {
        let quote = BlockQuote::default();
        assert_eq!(
            quote.left_block_color(),
            Some(Color::Tailwind(Tailwind::Gray500))
        );
    }

    #[test]
    fn test_default_has_no_text_color() {
        let quote = BlockQuote::default();
        assert!(quote.text_color().is_none());
    }

    #[test]
    fn test_default_has_no_bg_color() {
        let quote = BlockQuote::default();
        assert!(quote.bg_color().is_none());
    }

    #[test]
    fn test_default_has_no_attribution() {
        let quote = BlockQuote::default();
        assert!(quote.attribution.is_none());
    }

    // =========================================================================
    // From<Prose> Tests
    // =========================================================================

    #[test]
    fn test_from_prose() {
        let prose = Prose::new("Prose content");
        let quote = BlockQuote::from(prose);
        let result = quote.render_optimistic(None);
        assert!(result.contains("Prose content"));
    }

    #[test]
    fn test_from_prose_ref() {
        let prose = Prose::new("Prose reference");
        let quote = BlockQuote::from(&prose);
        let result = quote.render_optimistic(None);
        assert!(result.contains("Prose reference"));
    }

    // =========================================================================
    // Edge Cases
    // =========================================================================

    #[test]
    fn test_empty_content() {
        let quote = BlockQuote::from("");
        let result = quote.render_optimistic(None);
        // Empty content produces no lines (nothing to iterate over in .lines())
        assert_eq!(result, "");
    }

    #[test]
    fn test_single_character() {
        let quote = BlockQuote::from("X");
        let result = quote.render_optimistic(None);
        assert_eq!(strip_ansi(&result), "│ X");
    }

    #[test]
    fn test_unicode_content() {
        let quote = BlockQuote::from("Hello 世界 🌍");
        let result = quote.render_optimistic(None);
        assert_eq!(strip_ansi(&result), "│ Hello 世界 🌍");
    }

    #[test]
    fn test_whitespace_only() {
        let quote = BlockQuote::from("   ");
        let result = quote.render_optimistic(None);
        assert_eq!(strip_ansi(&result), "│    ");
    }

    // =========================================================================
    // TerminalRenderable Trait Tests
    // =========================================================================

    #[test]
    fn test_fallback_render_same_as_render() {
        let quote = BlockQuote::from("Fallback test");
        let term = Terminal::default();
        let render_result = quote.render_optimistic(None);
        let fallback_result = quote.render(&term);
        assert_eq!(render_result, fallback_result);
    }

    #[test]
    fn test_clone() {
        let quote = BlockQuote::from("Clone me")
            .with_text_color(Color::Tailwind(Tailwind::Blue500))
            .with_left_block_color(Color::Tailwind(Tailwind::Red500));
        let cloned = quote.clone();

        assert_eq!(
            quote.render_optimistic(None),
            cloned.render_optimistic(None)
        );
        assert_eq!(quote.text_color(), cloned.text_color());
        assert_eq!(quote.left_block_color(), cloned.left_block_color());
    }

    #[test]
    fn test_debug() {
        let quote = BlockQuote::from("Debug test");
        let debug_str = format!("{:?}", quote);
        assert!(debug_str.contains("BlockQuote"));
    }

    // =========================================================================
    // RenderableTerminalContent Integration Tests
    // =========================================================================

    #[test]
    fn test_renderable_content_string_variant() {
        let content = RenderableTerminalContent::String("direct string".to_string());
        let quote = BlockQuote::new(content, None::<&str>);
        let result = quote.render_optimistic(None);
        assert_eq!(strip_ansi(&result), "│ direct string");
    }

    #[test]
    fn test_renderable_content_component_variant() {
        let prose = Prose::new("<b>bold content</b>");
        let content = RenderableTerminalContent::Component(Rc::new(prose));
        let quote = BlockQuote::new(content, None::<&str>);
        let result = quote.render_optimistic(None);
        // The prose renders its bold content, which should appear in the quote
        assert!(strip_ansi(&result).starts_with("│ "));
        assert!(result.contains("bold content"));
    }

    #[test]
    fn test_styled_prose_in_block_quote() {
        let prose = Prose::new("<red>error</red>: something went wrong");
        let quote = BlockQuote::from(prose);
        let result = quote.render_optimistic(None);
        // Should contain the border and the content
        assert!(strip_ansi(&result).contains("│ "));
        assert!(result.contains("error"));
        assert!(result.contains("something went wrong"));
    }

    #[test]
    fn test_multiline_prose_in_block_quote() {
        let prose = Prose::new("Line 1\nLine 2\nLine 3");
        let quote = BlockQuote::from(prose);
        let result = quote.render_optimistic(None);
        // Each line should have a border
        let stripped = strip_ansi(&result);
        let lines: Vec<&str> = stripped.lines().collect();
        assert_eq!(lines.len(), 3);
        assert!(lines[0].starts_with("│ "));
        assert!(lines[1].starts_with("│ "));
        assert!(lines[2].starts_with("│ "));
    }

    #[test]
    fn test_prose_with_attribution() {
        let prose = Prose::new("<i>In the beginning...</i>");
        let quote = BlockQuote::new(
            RenderableTerminalContent::Component(Rc::new(prose)),
            Some("Genesis"),
        );
        let result = quote.render_optimistic(None);
        assert!(result.contains("In the beginning"));
        assert!(result.contains("— Genesis"));
    }

    // =========================================================================
    // Complex Scenario Tests
    // =========================================================================

    #[test]
    fn test_block_quote_preserves_internal_newlines() {
        let content = "First paragraph.\n\nSecond paragraph.";
        let quote = BlockQuote::from(content);
        let result = quote.render_optimistic(None);
        let stripped = strip_ansi(&result);
        let lines: Vec<&str> = stripped.lines().collect();
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0], "│ First paragraph.");
        assert_eq!(lines[1], "│ ");
        assert_eq!(lines[2], "│ Second paragraph.");
    }

    #[test]
    fn test_block_quote_with_tabs() {
        let quote = BlockQuote::from("Code:\n\tindented");
        let result = quote.render_optimistic(None);
        let stripped = strip_ansi(&result);
        assert!(stripped.contains("│ Code:"));
        assert!(stripped.contains("│ \tindented"));
    }

    #[test]
    fn test_attribution_only() {
        // Attribution with empty-ish content
        let quote = BlockQuote::new(RenderableTerminalContent::from("Quote"), Some("Author"));
        let result = quote.render_optimistic(None);
        let stripped = strip_ansi(&result);
        assert!(stripped.contains("│ Quote"));
        assert!(stripped.contains("│ — Author"));
    }

    // =========================================================================
    // Word Wrap Tests
    // =========================================================================

    #[test]
    fn test_long_string_wraps_at_width() {
        // At width 40, child_width = 38 (minus "│ " border).
        // A long string should be split across lines, each prefixed with "│ "
        let long = "word ".repeat(20); // 100 chars
        let quote = BlockQuote::from(long.trim());
        let result = quote.render_optimistic(Some(40));
        let stripped = strip_ansi(&result);
        let lines: Vec<&str> = stripped.lines().collect();
        assert!(
            lines.len() > 1,
            "Expected wrapping but got single line: {result}"
        );
        assert!(lines.iter().all(|l| l.starts_with("│ ")));
    }

    #[test]
    fn test_embedded_newlines_preserved_with_wrapping() {
        // Simulates a YAML >- prompt with paragraph breaks
        let text = "Do a deep dive on JSON Schema:\n- what types? - how is validation done?\nFor any code examples always use Rust.";
        let quote = BlockQuote::from(text);
        let result = quote.render_optimistic(Some(80));
        let stripped = strip_ansi(&result);
        let lines: Vec<&str> = stripped.lines().collect();
        // Should have at least 3 border-prefixed lines (one per \n-separated segment)
        assert!(
            lines.len() >= 3,
            "Expected >=3 lines but got {}: {result}",
            lines.len()
        );
        assert!(
            lines
                .iter()
                .all(|l| l.starts_with("│ ") || l.starts_with("      │")),
            "All lines should have border prefix: {result}"
        );
    }

    #[test]
    fn test_short_string_does_not_wrap() {
        let quote = BlockQuote::from("short text");
        let result = quote.render_optimistic(Some(80));
        let stripped = strip_ansi(&result);
        let lines: Vec<&str> = stripped.lines().collect();
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0], "│ short text");
    }

    // =========================================================================
    // TreeRenderable Tests
    // =========================================================================

    use renderable::tree::render::{
        BrowserRenderOptions, MarkdownRenderOptions, render_browser_node, render_markdown_node,
    };
    use renderable::tree::{NodeKind, TreeRenderable};

    #[test]
    fn test_render_tree_root_is_block_quote() {
        let quote = BlockQuote::from("Quoted text");
        let node = quote.render_tree();
        assert!(matches!(node.kind, NodeKind::BlockQuote { .. }));
    }

    #[test]
    fn test_render_tree_contains_text() {
        let quote = BlockQuote::from("Quoted text");
        let node = quote.render_tree();
        let json = serde_json::to_string(&node).expect("serialize");
        assert!(json.contains("Quoted text"));
    }

    #[test]
    fn test_render_tree_without_attribution_has_single_child() {
        let quote = BlockQuote::from("Just a quote");
        let node = quote.render_tree();
        assert_eq!(node.children().len(), 1);
    }

    #[test]
    fn test_render_tree_with_attribution_appends_child() {
        let quote = BlockQuote::new(
            RenderableTerminalContent::from("To be or not to be"),
            Some("Shakespeare"),
        );
        let node = quote.render_tree();
        assert_eq!(node.children().len(), 2);
        let json = serde_json::to_string(&node).expect("serialize");
        assert!(json.contains("— Shakespeare"));
    }

    #[test]
    fn test_render_tree_extracts_text_from_component() {
        let prose = Prose::new("<b>bold content</b>");
        let quote = BlockQuote::from(prose);
        let node = quote.render_tree();
        let json = serde_json::to_string(&node).expect("serialize");
        // Component text is extracted with ANSI escapes stripped.
        assert!(json.contains("bold content"));
    }

    #[test]
    fn test_render_tree_to_markdown() {
        let quote = BlockQuote::from("Quoted text");
        let node = quote.render_tree();
        let rendered = render_markdown_node(&node, &MarkdownRenderOptions::default())
            .expect("markdown render");
        assert_eq!(rendered.output, "> Quoted text");
    }

    #[test]
    fn test_render_tree_to_markdown_with_attribution() {
        let quote = BlockQuote::new(
            RenderableTerminalContent::from("To be or not to be"),
            Some("Shakespeare"),
        );
        let node = quote.render_tree();
        let rendered = render_markdown_node(&node, &MarkdownRenderOptions::default())
            .expect("markdown render");
        assert!(rendered.output.starts_with("> "));
        assert!(rendered.output.contains("To be or not to be"));
        assert!(rendered.output.contains("— Shakespeare"));
    }

    #[test]
    fn test_render_tree_to_browser() {
        let quote = BlockQuote::from("Quoted text");
        let node = quote.render_tree();
        let rendered =
            render_browser_node(&node, &BrowserRenderOptions::default()).expect("browser render");
        let html = rendered.output.render();
        assert!(html.contains("<blockquote>"));
        assert!(html.contains("Quoted text"));
    }
}
