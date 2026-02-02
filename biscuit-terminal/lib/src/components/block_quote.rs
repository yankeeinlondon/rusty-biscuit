use crate::{
    components::renderable::Renderable,
    terminal::Terminal,
    utils::{color::Color, layout::Layout},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockQuote {
    /// the content being wrapped in the block quote
    content: String,

    /// if the quote is being attributed to someone
    /// you can add that and it will be placed
    attribution: Option<String>,

    text_color: Option<Color>,
    bg_color: Option<Color>,
    left_block_color: Option<Color>,
}

impl Default for BlockQuote {
    fn default() -> Self {
        BlockQuote {
            content: "".to_string(),
            attribution: None,
            text_color: None,
            bg_color: None,
            left_block_color: None,
        }
    }
}

impl BlockQuote {
    /// Create a block quote by passing in the content and _optionally_ an attribution.
    pub fn new<T: Into<String>, U: Into<String>>(content: T, attribution: Option<U>) -> Self {
        Self {
            content: content.into(),
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
    fn render_content(&self, _term: Option<&Terminal>) -> String {
        let mut result = String::new();

        // Split content into lines and prefix each with the quote border
        let border = "│ ";
        for line in self.content.lines() {
            result.push_str(border);
            result.push_str(line);
            result.push('\n');
        }

        // Add attribution if present
        if let Some(ref attribution) = self.attribution {
            result.push_str("│ ");
            result.push_str("— ");
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
    fn render(&self, _layout: Option<&Layout>) -> String {
        self.render_content(None)
    }

    fn fallback_render(&self, term: &Terminal, _layout: Option<&Layout>) -> String {
        self.render_content(Some(term))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_block_quote() {
        let quote = BlockQuote::new("Hello world", None::<&str>);
        let result = quote.render(None);
        assert_eq!(result, "│ Hello world");
    }

    #[test]
    fn test_multiline_block_quote() {
        let quote = BlockQuote::new("Line 1\nLine 2", None::<&str>);
        let result = quote.render(None);
        assert_eq!(result, "│ Line 1\n│ Line 2");
    }

    #[test]
    fn test_block_quote_with_attribution() {
        let quote = BlockQuote::new("To be or not to be", Some("Shakespeare"));
        let result = quote.render(None);
        assert_eq!(result, "│ To be or not to be\n│ — Shakespeare");
    }
}
