//! Public [`Prose`] struct, builder methods, and target trait implementations.

use std::any::Any;

use renderable::browser::{BrowserRenderable, PageOptions};
use renderable::browser::fragment::{BrowserFragment, Ready};
use renderable::html::HtmlPage;
use renderable::markdown::MarkdownRenderable;
use renderable::tree::BlockElement;
use renderable::tree::render::{MarkdownDialect, MarkdownRenderOptions, render_markdown_node};
use renderable::tree::TreeRenderable;

use super::LineBreaks;
use crate::{
    render_tree::BrowserTreeComponent,
    utils::layout::{Layout, Length, TargetValue},
    utils::wrap_policy::WordWrap,
};

/// Block prose: paragraphs and fenced code blocks parsed from the shared
/// Prose grammar (bracketed style tags plus a Markdown subset).
///
/// `Prose` is block content on every target. Its render tree is a `Root` of
/// `Paragraph` and `Code` blocks, and it carries a [`Layout`] (margins,
/// alignment, word wrap) that every target applies. Use [`InlineProse`] for
/// phrasing content that sits inside a line, such as a table cell or a list
/// label.
///
/// ## Paragraphs and line breaks
///
/// Two or more newlines (with only spaces or tabs between them) separate
/// paragraphs. A single newline is a soft break by default
/// ([`LineBreaks::Soft`]); `\` immediately before a newline is a hard break.
/// Trailing spaces are never a hard break. CRLF and lone CR count as LF.
///
/// ```rust
/// use biscuit_terminal::components::prose::Prose;
/// use renderable::browser::BrowserRenderable;
///
/// let html = Prose::new("First paragraph.\n\nSecond,\nsame block.")
///     .render_html_fragment()
///     .render();
/// assert_eq!(html, "<p>First paragraph.</p><p>Second, same block.</p>");
/// ```
///
/// ## Style tags
///
/// ```rust
/// use biscuit_terminal::components::prose::Prose;
/// use biscuit_terminal::components::renderable::TerminalRenderable;
///
/// let prose = Prose::new("<bold>This is bold</bold> and <red>this is red</red>");
/// let rendered = prose.render_optimistic(None);
/// ```
///
/// Supported tags: `<bold>`, `<italic>`, `<red>`, `<bg-coral>`, `<a href="url">link</a>`,
/// `<rgb #ff0000>colored</rgb>`, etc. A style that spans a blank line is
/// reopened in each paragraph.
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
/// [`InlineProse`]: super::InlineProse
#[derive(Debug, Clone, Default)]
pub struct Prose {
    /// the raw content as received
    content: String,
    /// Layout configuration for margins, alignment, word wrap, etc.
    pub(super) layout: Layout,
    pub(super) line_breaks: LineBreaks,
    pub(super) tag: ProseTag,
}

/// The HTML element used for each paragraph of a [`Prose`].
///
/// The tag is browser presentation only: Markdown and terminal output have
/// the same block shape whatever the tag. Code blocks are always
/// `<pre><code>`.
///
/// ```rust
/// use biscuit_terminal::components::prose::{Prose, ProseTag};
/// use renderable::browser::BrowserRenderable;
///
/// let html = Prose::new("one\n\ntwo").with_tag(ProseTag::Div).render_html_fragment().render();
/// assert_eq!(html, "<div>one</div><div>two</div>");
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum ProseTag {
    /// `<p>`
    #[default]
    P,
    /// `<div>`
    Div,
    /// `<section>`
    Section,
    /// `<article>`
    Article,
    /// `<aside>`
    Aside,
    /// `<header>`
    Header,
    /// `<footer>`
    Footer,
}

impl From<ProseTag> for BlockElement {
    fn from(tag: ProseTag) -> Self {
        match tag {
            ProseTag::P => BlockElement::P,
            ProseTag::Div => BlockElement::Div,
            ProseTag::Section => BlockElement::Section,
            ProseTag::Article => BlockElement::Article,
            ProseTag::Aside => BlockElement::Aside,
            ProseTag::Header => BlockElement::Header,
            ProseTag::Footer => BlockElement::Footer,
        }
    }
}

impl Prose {
    /// Create a new Prose instance with the given content.
    pub fn new<T: Into<String>>(content: T) -> Self {
        Prose {
            content: content.into(),
            ..Prose::default()
        }
    }

    /// Returns the raw content as received.
    pub fn content(&self) -> &str {
        &self.content
    }

    /// Set what a single newline means in every paragraph.
    pub fn with_line_breaks(mut self, line_breaks: LineBreaks) -> Self {
        self.line_breaks = line_breaks;
        self
    }

    /// Set the HTML element used for each paragraph.
    pub fn with_tag(mut self, tag: ProseTag) -> Self {
        self.tag = tag;
        self
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
    /// interpolated into Prose content, except inside a code span or a fenced
    /// code block: their contents are literal and show the backslashes, so
    /// text placed between backticks or fences must not be escaped.
    ///
    /// ## ANSI escape pass-through
    ///
    /// Pre-styled text containing CSI (`ESC [ … final-byte`) or OSC
    /// (`ESC ] … BEL | ST`) escape sequences passes through unchanged: the
    /// `[` inside a CSI sequence is part of an escape, not Prose grammar.
    /// Escaping it would corrupt the styled bytes (an extra `\` between
    /// `ESC` and `[` makes the terminal treat the SGR as literal text).
    /// Outside of ANSI sequences the normal Prose escaping rules apply.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use biscuit_terminal::components::prose::Prose;
    ///
    /// // Path with angle brackets stays literal
    /// let escaped = Prose::escape_text("path/<weird>");
    /// assert_eq!(escaped, r"path/\<weird\>");
    /// ```
    pub fn escape_text(s: &str) -> String {
        let mut result = String::with_capacity(s.len());
        let bytes = s.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == 0x1B && i + 1 < bytes.len() {
                // Recognize ANSI CSI / OSC sequences and pass them through
                // verbatim. `escape_text` is byte-oriented when stepping over
                // ESC sequences (all bytes here are ASCII) and reverts to
                // char-based handling once the sequence ends.
                let next = bytes[i + 1];
                if next == b'[' {
                    // CSI: ESC '[' params... final-byte (0x40..=0x7E).
                    let start = i;
                    i += 2;
                    while i < bytes.len() && !(0x40..=0x7E).contains(&bytes[i]) {
                        i += 1;
                    }
                    if i < bytes.len() {
                        i += 1; // include the final byte
                    }
                    result.push_str(&s[start..i]);
                    continue;
                }
                if next == b']' {
                    // OSC: ESC ']' params... BEL or ESC '\' (ST).
                    let start = i;
                    i += 2;
                    while i < bytes.len() {
                        if bytes[i] == 0x07 {
                            i += 1;
                            break;
                        }
                        if bytes[i] == 0x1B && i + 1 < bytes.len() && bytes[i + 1] == b'\\' {
                            i += 2;
                            break;
                        }
                        i += 1;
                    }
                    result.push_str(&s[start..i]);
                    continue;
                }
            }
            // Step one UTF-8 character. `s[i..]` is always a valid char
            // boundary here because the only multi-byte handling above
            // jumps within ASCII-only ANSI sequences.
            let ch = s[i..].chars().next().expect("non-empty remainder");
            match ch {
                '<' | '>' | '{' | '*' | '_' | '[' | ']' | '(' | ')' | '\\' => {
                    result.push('\\');
                    result.push(ch);
                }
                _ => result.push(ch),
            }
            i += ch.len_utf8();
        }
        result
    }

    /// Escapes text that already marks its code with backticks, such as an
    /// error message: [`Prose::escape_text`] is applied outside each closed
    /// code span, and every span is copied through unchanged so its contents
    /// stay literal.
    ///
    /// Spans are recognized as the Prose grammar recognizes them: a backtick
    /// run opens a span only when a later run of the same length closes it
    /// before the next blank line. An unmatched run is ordinary text, so the
    /// text after it is escaped. Use [`Prose::escape_text`] for text with no
    /// code of its own, and fence a value bound for a code span with
    /// `renderable::markdown::code_span` instead.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use biscuit_terminal::components::prose::Prose;
    ///
    /// let escaped = Prose::escape_text_outside_code_spans("unknown field `foo_bar`, in a_b");
    /// assert_eq!(escaped, r"unknown field `foo_bar`, in a\_b");
    ///
    /// // An unmatched backtick does not open a span.
    /// assert_eq!(Prose::escape_text_outside_code_spans("a ` b_c"), r"a ` b\_c");
    /// ```
    pub fn escape_text_outside_code_spans(s: &str) -> String {
        let mut escaped = String::with_capacity(s.len());
        let mut plain_start = 0;
        let mut index = 0;
        while let Some(offset) = s[index..].find('`') {
            let open = index + offset;
            let ticks = backtick_run(&s[open..]);
            let content_start = open + ticks;
            match closing_backtick_run(&s[content_start..], ticks) {
                Some(close) => {
                    let end = content_start + close + ticks;
                    escaped.push_str(&Self::escape_text(&s[plain_start..open]));
                    escaped.push_str(&s[open..end]);
                    plain_start = end;
                    index = end;
                }
                None => index = content_start,
            }
        }
        escaped.push_str(&Self::escape_text(&s[plain_start..]));
        escaped
    }

    /// Build a safely-quoted attribute value for Prose block tags.
    ///
    /// Backslash-escapes the characters that would break tag-level parsing
    /// (`<`, `>`, `\`), wraps the result in single or double quotes —
    /// preferring the quote character that does not already appear in the
    /// value — and escapes any occurrence of the chosen wrapping quote.
    /// The Prose tag parser resolves these escapes inside attribute values,
    /// so a value containing both quote types still round-trips exactly.
    ///
    /// Markdown-emphasis characters (`_`, `*`, `[`, `]`, `(`, `)`, `{`)
    /// are passed through verbatim: the markdown pre-processor treats tag
    /// declarations opaquely, and escaping them here would leak literal
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
        // Wrap in the quote character least likely to collide; escape any
        // occurrence of the chosen quote so a value carrying both quote
        // types still parses back to its exact bytes.
        let quote = if escaped.contains('"') && !escaped.contains('\'') {
            '\''
        } else {
            '"'
        };
        let body = escaped.replace(quote, &format!("\\{quote}"));
        format!("{quote}{body}{quote}")
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

impl MarkdownRenderable for Prose {
    /// Renders the prose as portable Markdown via the canonical render tree.
    fn render_markdown(&self) -> String {
        let node = <Self as TreeRenderable>::render_tree(self);
        match render_markdown_node(&node, &MarkdownRenderOptions::default()) {
            Ok(rendered) => rendered.output,
            Err(error) => {
                tracing::error!(
                    component = "Prose",
                    dialect = "Markdown",
                    error = %error,
                    "render_markdown_node failed; emitting empty output"
                );
                String::new()
            }
        }
    }

    /// Renders the prose as MarkdownPlus via the canonical render tree.
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
                    component = "Prose",
                    dialect = "MarkdownPlus",
                    error = %error,
                    "render_markdown_node failed; emitting empty output"
                );
                String::new()
            }
        }
    }
}

impl BrowserRenderable for Prose {
    /// Renders the prose as block HTML from its render tree: one element per
    /// paragraph (see [`ProseTag`]) and `<pre><code>` per code block. With no
    /// layout the blocks are siblings with no wrapper; a layout renders as
    /// CSS on the root's wrapping `<div>`.
    fn render_html_fragment(&self) -> BrowserFragment<Ready> {
        if self.layout == Layout::default() {
            return super::tree::concat_html("Prose", self.block_nodes());
        }
        BrowserTreeComponent::new(self.clone()).render_html_fragment()
    }

    fn render_html_page(&self, page: Option<PageOptions>) -> HtmlPage {
        let mut html_page = HtmlPage::from(self.render_html_fragment());
        if let Some(options) = page {
            html_page.apply_page_options(options);
        }
        html_page
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Length of the backtick run at the start of `s`.
fn backtick_run(s: &str) -> usize {
    s.bytes().take_while(|&b| b == b'`').count()
}

/// Byte offset in `s` of the first backtick run exactly `ticks` long, searching
/// no further than the next blank line (a code span never crosses one).
fn closing_backtick_run(s: &str, ticks: usize) -> Option<usize> {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'`' => {
                let run = backtick_run(&s[i..]);
                if run == ticks {
                    return Some(i);
                }
                i += run;
            }
            b'\n' => {
                let rest = s[i + 1..].trim_start_matches([' ', '\t', '\r']);
                if rest.starts_with('\n') {
                    return None;
                }
                i += 1;
            }
            _ => i += 1,
        }
    }
    None
}
