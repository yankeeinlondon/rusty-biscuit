use std::sync::Arc;

use crate::{
    components::{
        prose::Prose,
        renderable::{Renderable, RenderableContent},
    },
    terminal::Terminal,
    utils::{
        color::{Color, Tailwind},
        layout::Layout,
    },
};

#[derive(Debug, Clone)]
pub struct BlockQuote {
    /// the content being wrapped in the block quote
    content: RenderableContent,

    /// if the quote is being attributed to someone
    /// you can add that and it will be placed
    attribution: Option<String>,

    text_color: Option<Color>,
    bg_color: Option<Color>,
    left_block_color: Option<Color>,
    layout: Layout,
}

impl Default for BlockQuote {
    fn default() -> Self {
        BlockQuote {
            content: RenderableContent::String("".to_string()),
            attribution: None,
            text_color: None,
            bg_color: None,
            left_block_color: Some(Color::Tailwind(Tailwind::Gray500)),
            layout: Layout::default(),
        }
    }
}

impl From<String> for BlockQuote {
    fn from(value: String) -> Self {
        BlockQuote::new(RenderableContent::String(value), None::<String>)
    }
}

impl From<&String> for BlockQuote {
    fn from(value: &String) -> Self {
        BlockQuote::new(RenderableContent::String(value.into()), None::<String>)
    }
}

impl From<&str> for BlockQuote {
    fn from(value: &str) -> Self {
        BlockQuote::new(RenderableContent::String(value.into()), None::<String>)
    }
}

impl From<Prose> for BlockQuote {
    fn from(value: Prose) -> Self {
        BlockQuote::new(
            RenderableContent::Component(Arc::new(value)),
            None::<String>,
        )
    }
}

impl From<&Prose> for BlockQuote {
    fn from(value: &Prose) -> Self {
        BlockQuote::new(
            RenderableContent::Component(Arc::new((*value).clone())),
            None::<String>,
        )
    }
}

impl BlockQuote {
    /// Create a block quote by passing in the content and _optionally_ an attribution.
    pub fn new<U: Into<String>>(content: RenderableContent, attribution: Option<U>) -> Self {
        Self {
            content: RenderableContent::from(content),
            attribution: match attribution {
                Some(attribution) => Some(attribution.into()),
                _ => None,
            },
            ..BlockQuote::default()
        }
    }

    /// Set the text color.
    pub fn with_text_color(mut self, color: Color) -> Self {
        self.text_color = Some(color);
        self
    }

    /// Set the background color.
    pub fn with_bg_color(mut self, color: Color) -> Self {
        self.bg_color = Some(color);
        self
    }

    /// Set the left block/border color.
    pub fn with_left_block_color(mut self, color: Color) -> Self {
        self.left_block_color = Some(color);
        self
    }

    /// Render the block quote content with a left border.
    ///
    /// The border "│ " consumes 2 columns, so child content width
    /// is reduced accordingly.
    fn render_content(&self, term: Option<&Terminal>, term_width: u32) -> String {
        let default_term = Terminal::new_tty();
        let term = term.unwrap_or(&default_term);

        // Border "│ " is 2 visible characters wide
        let _child_width = term_width.saturating_sub(2);

        let content: String = match &self.content {
            RenderableContent::String(s) => s.clone(),
            RenderableContent::Component(component) => component.fallback_render(term),
        };
        let mut result = String::new();

        // Split content into lines and prefix each with the quote border
        let border = "│ ";
        for line in content.lines() {
            result.push_str(border);
            result.push_str(line);
            result.push('\n');
        }

        // Add attribution if present (with blank line separator)
        if let Some(ref attribution) = self.attribution {
            result.push_str("│\n"); // blank line
            result.push_str("│ — ");
            result.push_str(attribution);
            result.push('\n');
        }

        // Remove trailing newline
        if result.ends_with('\n') {
            result.pop();
        }

        result
    }
}

impl Renderable for BlockQuote {
    fn render(&self, term_width: Option<u32>) -> String {
        let width = term_width.unwrap_or(80);
        let available = self.layout.available_width(width);
        let content = self.render_content(None, available);
        self.layout.apply_layout(&content, width)
    }

    fn fallback_render(&self, term: &Terminal) -> String {
        let width = term.width();
        let available = self.layout.available_width(width);
        let content = self.render_content(Some(term), available);
        self.layout.apply_layout(&content, width)
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

#[cfg(test)]
mod tests {
    use super::*;

    // =========================================================================
    // Basic Construction Tests
    // =========================================================================

    #[test]
    fn test_simple_block_quote_from_str() {
        let quote = BlockQuote::from("Hello world");
        let result = quote.render(None);
        assert_eq!(result, "│ Hello world");
    }

    #[test]
    fn test_simple_block_quote_from_string() {
        let quote = BlockQuote::from(String::from("Hello world"));
        let result = quote.render(None);
        assert_eq!(result, "│ Hello world");
    }

    #[test]
    fn test_simple_block_quote_from_string_ref() {
        let content = String::from("Hello world");
        let quote = BlockQuote::from(&content);
        let result = quote.render(None);
        assert_eq!(result, "│ Hello world");
    }

    #[test]
    fn test_block_quote_new_with_renderable_content() {
        let quote = BlockQuote::new(RenderableContent::from("Direct content"), None::<&str>);
        let result = quote.render(None);
        assert_eq!(result, "│ Direct content");
    }

    // =========================================================================
    // Multiline Content Tests
    // =========================================================================

    #[test]
    fn test_multiline_block_quote() {
        let quote = BlockQuote::new(RenderableContent::from("Line 1\nLine 2"), None::<&str>);
        let result = quote.render(None);
        assert_eq!(result, "│ Line 1\n│ Line 2");
    }

    #[test]
    fn test_multiline_block_quote_three_lines() {
        let quote = BlockQuote::from("First\nSecond\nThird");
        let result = quote.render(None);
        assert_eq!(result, "│ First\n│ Second\n│ Third");
    }

    #[test]
    fn test_block_quote_with_empty_lines() {
        let quote = BlockQuote::from("Before\n\nAfter");
        let result = quote.render(None);
        assert_eq!(result, "│ Before\n│ \n│ After");
    }

    // =========================================================================
    // Attribution Tests
    // =========================================================================

    #[test]
    fn test_block_quote_with_attribution() {
        let quote = BlockQuote::new(
            RenderableContent::from("To be or not to be"),
            Some("Shakespeare"),
        );
        let result = quote.render(None);
        assert_eq!(result, "│ To be or not to be\n│\n│ — Shakespeare");
    }

    #[test]
    fn test_block_quote_with_string_attribution() {
        let quote = BlockQuote::new(
            RenderableContent::from("The unexamined life is not worth living"),
            Some(String::from("Socrates")),
        );
        let result = quote.render(None);
        assert_eq!(
            result,
            "│ The unexamined life is not worth living\n│\n│ — Socrates"
        );
    }

    #[test]
    fn test_multiline_with_attribution() {
        let quote = BlockQuote::new(
            RenderableContent::from("Line one\nLine two"),
            Some("Author"),
        );
        let result = quote.render(None);
        assert_eq!(result, "│ Line one\n│ Line two\n│\n│ — Author");
    }

    // =========================================================================
    // Builder Pattern (with_*) Tests
    // =========================================================================

    #[test]
    fn test_with_text_color() {
        let quote =
            BlockQuote::from("Colored text").with_text_color(Color::Tailwind(Tailwind::Blue500));
        assert!(quote.text_color.is_some());
    }

    #[test]
    fn test_with_bg_color() {
        let quote =
            BlockQuote::from("Background").with_bg_color(Color::Tailwind(Tailwind::Gray100));
        assert!(quote.bg_color.is_some());
    }

    #[test]
    fn test_with_left_block_color() {
        let quote = BlockQuote::from("Custom border")
            .with_left_block_color(Color::Tailwind(Tailwind::Red500));
        assert_eq!(
            quote.left_block_color,
            Some(Color::Tailwind(Tailwind::Red500))
        );
    }

    #[test]
    fn test_builder_chain() {
        let quote = BlockQuote::from("Full styling")
            .with_text_color(Color::Tailwind(Tailwind::White))
            .with_bg_color(Color::Tailwind(Tailwind::Gray800))
            .with_left_block_color(Color::Tailwind(Tailwind::Green500));

        assert!(quote.text_color.is_some());
        assert!(quote.bg_color.is_some());
        assert!(quote.left_block_color.is_some());
    }

    // =========================================================================
    // Default Tests
    // =========================================================================

    #[test]
    fn test_default_has_gray_left_block() {
        let quote = BlockQuote::default();
        assert_eq!(
            quote.left_block_color,
            Some(Color::Tailwind(Tailwind::Gray500))
        );
    }

    #[test]
    fn test_default_has_no_text_color() {
        let quote = BlockQuote::default();
        assert!(quote.text_color.is_none());
    }

    #[test]
    fn test_default_has_no_bg_color() {
        let quote = BlockQuote::default();
        assert!(quote.bg_color.is_none());
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
        let result = quote.render(None);
        assert!(result.contains("Prose content"));
    }

    #[test]
    fn test_from_prose_ref() {
        let prose = Prose::new("Prose reference");
        let quote = BlockQuote::from(&prose);
        let result = quote.render(None);
        assert!(result.contains("Prose reference"));
    }

    // =========================================================================
    // Edge Cases
    // =========================================================================

    #[test]
    fn test_empty_content() {
        let quote = BlockQuote::from("");
        let result = quote.render(None);
        // Empty content produces no lines (nothing to iterate over in .lines())
        assert_eq!(result, "");
    }

    #[test]
    fn test_single_character() {
        let quote = BlockQuote::from("X");
        let result = quote.render(None);
        assert_eq!(result, "│ X");
    }

    #[test]
    fn test_unicode_content() {
        let quote = BlockQuote::from("Hello 世界 🌍");
        let result = quote.render(None);
        assert_eq!(result, "│ Hello 世界 🌍");
    }

    #[test]
    fn test_whitespace_only() {
        let quote = BlockQuote::from("   ");
        let result = quote.render(None);
        assert_eq!(result, "│    ");
    }

    // =========================================================================
    // Renderable Trait Tests
    // =========================================================================

    #[test]
    fn test_fallback_render_same_as_render() {
        let quote = BlockQuote::from("Fallback test");
        let term = Terminal::default();
        let render_result = quote.render(None);
        let fallback_result = quote.fallback_render(&term);
        assert_eq!(render_result, fallback_result);
    }

    #[test]
    fn test_clone() {
        let quote = BlockQuote::from("Clone me")
            .with_text_color(Color::Tailwind(Tailwind::Blue500))
            .with_left_block_color(Color::Tailwind(Tailwind::Red500));
        let cloned = quote.clone();

        assert_eq!(quote.render(None), cloned.render(None));
        assert_eq!(quote.text_color, cloned.text_color);
        assert_eq!(quote.left_block_color, cloned.left_block_color);
    }

    #[test]
    fn test_debug() {
        let quote = BlockQuote::from("Debug test");
        let debug_str = format!("{:?}", quote);
        assert!(debug_str.contains("BlockQuote"));
    }

    // =========================================================================
    // RenderableContent Integration Tests
    // =========================================================================

    #[test]
    fn test_renderable_content_string_variant() {
        let content = RenderableContent::String("direct string".to_string());
        let quote = BlockQuote::new(content, None::<&str>);
        let result = quote.render(None);
        assert_eq!(result, "│ direct string");
    }

    #[test]
    fn test_renderable_content_component_variant() {
        let prose = Prose::new("<b>bold content</b>");
        let content = RenderableContent::Component(Arc::new(prose));
        let quote = BlockQuote::new(content, None::<&str>);
        let result = quote.render(None);
        // The prose renders its bold content, which should appear in the quote
        assert!(result.starts_with("│ "));
        assert!(result.contains("bold content"));
    }

    #[test]
    fn test_styled_prose_in_block_quote() {
        let prose = Prose::new("<red>error</red>: something went wrong");
        let quote = BlockQuote::from(prose);
        let result = quote.render(None);
        // Should contain the border and the content
        assert!(result.contains("│ "));
        assert!(result.contains("error"));
        assert!(result.contains("something went wrong"));
    }

    #[test]
    fn test_multiline_prose_in_block_quote() {
        let prose = Prose::new("Line 1\nLine 2\nLine 3");
        let quote = BlockQuote::from(prose);
        let result = quote.render(None);
        // Each line should have a border
        let lines: Vec<&str> = result.lines().collect();
        assert_eq!(lines.len(), 3);
        assert!(lines[0].starts_with("│ "));
        assert!(lines[1].starts_with("│ "));
        assert!(lines[2].starts_with("│ "));
    }

    #[test]
    fn test_prose_with_attribution() {
        let prose = Prose::new("<i>In the beginning...</i>");
        let quote = BlockQuote::new(
            RenderableContent::Component(Arc::new(prose)),
            Some("Genesis"),
        );
        let result = quote.render(None);
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
        let result = quote.render(None);
        let lines: Vec<&str> = result.lines().collect();
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0], "│ First paragraph.");
        assert_eq!(lines[1], "│ ");
        assert_eq!(lines[2], "│ Second paragraph.");
    }

    #[test]
    fn test_block_quote_with_tabs() {
        let quote = BlockQuote::from("Code:\n\tindented");
        let result = quote.render(None);
        assert!(result.contains("│ Code:"));
        assert!(result.contains("│ \tindented"));
    }

    #[test]
    fn test_attribution_only() {
        // Attribution with empty-ish content
        let quote = BlockQuote::new(RenderableContent::from("Quote"), Some("Author"));
        let result = quote.render(None);
        assert!(result.contains("│ Quote"));
        assert!(result.contains("│ — Author"));
    }
}
