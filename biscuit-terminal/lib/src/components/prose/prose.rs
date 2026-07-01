//! Public [`Prose`] struct, builder methods, and target trait implementations.

use std::any::Any;

use renderable::browser::{BrowserRenderable, PageOptions};
use renderable::browser::fragment::{BrowserFragment, ComposableNode, Ready};
use renderable::html::HtmlPage;
use renderable::html::tag::BlockTag;
use renderable::markdown::MarkdownRenderable;
use renderable::tree::render::{
    BrowserRenderOptions, MarkdownDialect, MarkdownRenderOptions, render_browser_node,
    render_markdown_node,
};
use renderable::tree::TreeRenderable;

use crate::{
    utils::layout::{Layout, Length, TargetValue},
    utils::wrap_policy::WordWrap,
};

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
/// ## Layout & Style Contract
///
/// `Prose` has a dual-mode contract (spec C1/C7):
///
/// - **Block-container mode** (the public component API, i.e. calling
///   `render()` / `render_tree()` directly): the root routes through the shared
///   render-tree fold, so all applicable `Layout` box properties (`margin`,
///   `padding`, `width`, `max_width`, `alignment`) and `Style` properties
///   (`color`, `background`, `emphasis`, `border`) are honored via the fold
///   (C1).
/// - **Inline mode** (when `Prose` content is nested inside another component):
///   the parsed inline spans carry no block box; the containing block owns the
///   box. Inherited `color` / `emphasis` and inline `background` flow through
///   (C7). Inline `background` paints only the inline content, not a padding
///   box.
///
/// `word_wrap` is honored on text leaves (Decision D4).
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
    ////// Path with angle brackets stays literal
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
    /// Renders the prose as a layout-free inline `<span class="prose">` HTML
    /// fragment.
    ///
    /// ## Interim layout contract (revisit with style-based-alignment)
    ///
    /// The fragment deliberately carries **no** layout (margins, alignment,
    /// width): it wraps the styled inline children in a single
    /// `<span class="prose">` and stops there. Layout is owned by the caller —
    /// the `bt prose` CLI wraps this fragment in its own `<div style="…">` (see
    /// `biscuit-terminal/cli/src/commands/prose.rs::render_html_with_layout`).
    ///
    /// This splits the contract on purpose. Routing the fragment through
    /// [`TreeRenderable::render_tree`] folds Prose's `Layout` into the fragment
    /// (`<div style="margin-…"><p>…</p></div>`), which then **double-applies**
    /// the margin against the CLI's own wrapper. Keeping the fragment inline and
    /// letting the CLI own layout is the resolution chosen on 2026-06-04 for
    /// that double-margin defect.
    ///
    /// It is an interim choice. Once
    /// `renderable/features/2026-06-04-style-based-alignment` moves layout onto
    /// CSS-Box-Layout `renderable::style`/`Layout` primitives that lower to CSS,
    /// the component-owns-its-layout model becomes viable again and this split
    /// should be reconsidered (fragment carries its layout; CLI stops wrapping).
    /// Until then, do not re-route this through `render_tree`.
    fn render_html_fragment(&self) -> BrowserFragment<Ready> {
        // Render each projected node to its own HTML through the shared tree
        // renderer, then concatenate the strings inside one `<span class="prose">`.
        // Rendering per node (rather than wrapping the nodes in one inline Span
        // tree) keeps a top-level block-level `Code` node valid — a fenced code
        // block folds to `<pre><code>…</code></pre>` and only the final HTML
        // string carries it inside the span, which the tree validator never sees.
        let opts = BrowserRenderOptions::default();
        let mut inner = String::new();
        for child in self.to_render_nodes() {
            match render_browser_node(&child, &opts) {
                Ok(rendered) => inner.push_str(&rendered.output.render()),
                Err(error) => tracing::error!(
                    component = "Prose",
                    error = %error,
                    "render_browser_node failed; skipping node"
                ),
            }
        }
        BrowserFragment::new()
            .define_as_block_tag(BlockTag::Span, "prose")
            .add_child(ComposableNode::RawHtml(inner))
            .finalize()
    }

    /// Wraps the HTML fragment in a complete [`HtmlPage`].
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
