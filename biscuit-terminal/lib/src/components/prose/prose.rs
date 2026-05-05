//! Public [`Prose`] struct, builder methods, and the `parse_tokens` entry point.

use crate::{
    terminal::Terminal,
    utils::layout::{Layout, Margin},
    utils::wrap_policy::WordWrap,
};

use super::styles::StyleState;
use super::tokens::parse_tokens_inner;

/// Styled text with token and block tag support for rich terminal output.
///
/// This struct wraps text content that gets parsed for styling tokens and
/// rendered with ANSI escape codes.
///
/// ## Token Types
///
/// ### Atomic Tokens (`{{token}}`)
/// Single-use tokens that apply styling and require manual `{{reset}}`:
///
/// ```rust
/// use biscuit_terminal::components::prose::Prose;
/// use biscuit_terminal::components::renderable::Renderable;
///
/// let prose = Prose::new("{{bold}}Important:{{reset}} This is bold text");
/// let rendered = prose.render_optimistic(None);
/// // Contains ANSI bold escape codes
/// ```
///
/// Supported tokens: `{{bold}}`, `{{dim}}`, `{{italic}}`, `{{underline}}`,
/// `{{red}}`, `{{bg-blue}}`, `{{reset}}`, etc.
///
/// ### Block Tags (`<tag>content</tag>`)
/// Self-closing tags that auto-reset:
///
/// ```rust
/// use biscuit_terminal::components::prose::Prose;
/// use biscuit_terminal::components::renderable::Renderable;
///
/// let prose = Prose::new("<bold>This is bold</bold> and <red>this is red</red>");
/// let rendered = prose.render_optimistic(None);
/// // Both styles auto-reset after their content
/// ```
///
/// Supported tags: `<bold>`, `<italic>`, `<red>`, `<bg-coral>`, `<a href="url">link</a>`,
/// `<rgb #ff0000>colored</rgb>`, etc.
///
/// ## Escaping
///
/// Use backslash to output literal characters:
///
/// ```rust
/// use biscuit_terminal::components::prose::Prose;
/// use biscuit_terminal::components::renderable::Renderable;
///
/// let prose = Prose::new(r"\<literal \<angles\>");
/// assert!(prose.render_optimistic(None).contains("literal <angles>"));
/// ```
///
/// ## Layout
///
/// Configure margins, alignment, and word wrapping:
///
/// ```rust
/// use biscuit_terminal::components::prose::Prose;
/// use biscuit_terminal::components::renderable::Renderable;
/// use biscuit_terminal::utils::layout::{Alignment, Layout, WordWrap};
///
/// let prose = Prose::new("Styled content")
///     .with_layout(Layout {
///         alignment: Alignment::Center,
///         word_wrap: WordWrap::None,
///         ..Layout::default()
///     });
/// ```
#[derive(Debug, Clone)]
pub struct Prose {
    /// the raw content as received
    content: String,
    /// Layout configuration for margins, alignment, word wrap, etc.
    pub(super) layout: Layout,
}

impl Prose {
    /// Create a new Prose instance with the given content.
    pub fn new<T: Into<String>>(content: T) -> Self {
        Prose {
            content: content.into(),
            layout: Layout::default(),
        }
    }

    /// Returns the raw content as received.
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Set the word wrap strategy.
    pub fn with_word_wrap(mut self, wrap: WordWrap) -> Self {
        self.layout.word_wrap = wrap;
        self
    }

    /// Set the left margin.
    pub fn with_left_margin(mut self, margin: Margin) -> Self {
        self.layout.left_margin = margin;
        self
    }

    /// Set the right margin.
    pub fn with_right_margin(mut self, margin: Margin) -> Self {
        self.layout.right_margin = margin;
        self
    }

    /// Parse and render the content, replacing tokens with ANSI escape codes.
    ///
    /// Creates a fresh [`StyleState`] and delegates to [`parse_tokens_inner`].
    /// Only this outermost call emits the final `\x1b[0m` reset.
    pub(super) fn parse_tokens(&self, term: Option<&Terminal>) -> String {
        let mut state = StyleState::default();
        let mut result = parse_tokens_inner(&self.content, term, &mut state);
        if state.used_styles {
            result.push_str("\x1b[0m");
        }
        result
    }
}

impl Default for Prose {
    fn default() -> Prose {
        Prose {
            content: "".to_string(),
            layout: Layout::default(),
        }
    }
}
