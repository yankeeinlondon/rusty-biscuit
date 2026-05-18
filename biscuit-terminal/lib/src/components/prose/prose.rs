//! Public [`Prose`] struct, builder methods, and the `parse_tokens` entry point.

use crate::{
    terminal::Terminal,
    utils::layout::{Layout, Length, TargetValue},
    utils::wrap_policy::WordWrap,
};

use super::ir::ProseDocument;

/// Styled text with token and block tag support for rich terminal output.
///
/// This struct wraps text content that gets parsed for styling tokens and
/// rendered with ANSI escape codes.
///
/// ## Input Grammars
///
/// `Prose` accepts **bracketed tags** and a **Markdown subset**. The
/// atomic-token grammar (`{{token}}`) has been removed — `{{…}}` now renders
/// as ordinary literal text.
///
/// ### Block Tags (`<tag>content</tag>`)
/// Self-closing tags that auto-reset:
///
/// ```rust
/// use biscuit_terminal::components::prose::Prose;
/// use biscuit_terminal::components::renderable::TerminalRenderable;
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
/// use biscuit_terminal::components::renderable::TerminalRenderable;
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
/// use biscuit_terminal::components::renderable::TerminalRenderable;
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
    pub fn with_left_margin(mut self, margin: TargetValue<Length>) -> Self {
        self.layout.margin.left = margin;
        self
    }

    /// Set the right margin.
    pub fn with_right_margin(mut self, margin: TargetValue<Length>) -> Self {
        self.layout.margin.right = margin;
        self
    }

    /// Escape text so it renders literally in Prose markup.
    ///
    /// Escapes characters that have special meaning in the Prose grammar
    /// (`<`, `>`, `{`, `*`, `_`, `[`, `]`, `(`, `)`, `\`) by prefixing them
    /// with a backslash. Use this for any user-controlled string that is
    /// interpolated into Prose content.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use biscuit_terminal::components::prose::Prose;
    ////// Path with angle brackets stays literal
    /// let escaped = Prose::escape_text("path/<weird>");
    /// assert_eq!(escaped, r"path/\<weird\>");
    /// ```
    pub fn escape_text(s: &str) -> String {
        let mut result = String::with_capacity(s.len());
        for c in s.chars() {
            match c {
                '<' | '>' | '{' | '*' | '_' | '[' | ']' | '(' | ')' | '\\' => {
                    result.push('\\');
                    result.push(c);
                }
                _ => result.push(c),
            }
        }
        result
    }

    /// Build a safely-quoted attribute value for Prose block tags.
    ///
    /// Escapes only the characters that would break tag-level parsing
    /// (`<`, `>`, `\`) and wraps the result in single or double quotes,
    /// choosing the quote character that does not appear in the value so
    /// the attribute parser never mis-identifies the end of the value.
    ///
    /// Markdown-emphasis characters (`_`, `*`, `[`, `]`, `(`, `)`, `{`)
    /// are passed through verbatim: the markdown pre-processor treats tag
    /// declarations opaquely, and the attribute parser does not unescape
    /// backslash sequences, so escaping them here would leak literal
    /// backslashes into href URLs and other consumers.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use biscuit_terminal::components::prose::Prose;
    ///
    /// // Default: double quotes
    /// let attr = Prose::quoted_attr("/normal/path");
    /// assert_eq!(attr, "\"/normal/path\"");
    ///
    /// // Underscores pass through verbatim
    /// let attr = Prose::quoted_attr("/tmp/path_with_underscores");
    /// assert_eq!(attr, "\"/tmp/path_with_underscores\"");
    ///
    /// // Switches to single quotes when value contains double quotes
    /// let attr = Prose::quoted_attr(r#"path/with"quotes"#);
    /// assert_eq!(attr, "'path/with\"quotes'");
    /// ```
    pub fn quoted_attr(value: &str) -> String {
        let mut escaped = String::with_capacity(value.len());
        for c in value.chars() {
            match c {
                '<' | '>' | '\\' => {
                    escaped.push('\\');
                    escaped.push(c);
                }
                _ => escaped.push(c),
            }
        }
        if escaped.contains('"') && !escaped.contains('\'') {
            format!("'{}'", escaped)
        } else {
            format!("\"{}\"", escaped)
        }
    }

    /// Parse the raw content into the target-neutral [`ProseDocument`] IR.
    ///
    /// Pre-processes the supported Markdown subset (`**bold**`, `_italics_`,
    /// `[desc](ref)`) and parses bracketed tags. Former atomic-token syntax
    /// (`{{…}}`) is treated as ordinary literal text.
    pub(super) fn document(&self) -> ProseDocument {
        ProseDocument::parse(&self.content)
    }

    /// Parse and render the content to terminal ANSI/OSC8 output.
    ///
    /// Builds the [`ProseDocument`] IR and renders it through the terminal
    /// emitter. Only the outermost emit produces the final `\x1b[0m` reset.
    pub(super) fn parse_tokens(&self, term: Option<&Terminal>) -> String {
        super::terminal::render(&self.document(), term)
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

impl From<Prose> for Vec<Prose> {
    fn from(prose: Prose) -> Self {
        vec![prose]
    }
}

/// A trait for types that can be converted into a `Vec<Prose>`.
///
/// This exists because Rust's orphan rules prevent implementing
/// `From<&str>` and `From<String>` for `Vec<Prose>` directly.
pub trait IntoProseVec {
    /// Convert into a vector of `Prose` items.
    fn into_prose_vec(self) -> Vec<Prose>;
}

impl IntoProseVec for Vec<Prose> {
    fn into_prose_vec(self) -> Vec<Prose> {
        self
    }
}

impl IntoProseVec for Prose {
    fn into_prose_vec(self) -> Vec<Prose> {
        vec![self]
    }
}

impl IntoProseVec for &str {
    fn into_prose_vec(self) -> Vec<Prose> {
        vec![Prose::new(self)]
    }
}

impl IntoProseVec for String {
    fn into_prose_vec(self) -> Vec<Prose> {
        vec![Prose::new(self)]
    }
}
