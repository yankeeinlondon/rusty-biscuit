//! Target-neutral parsed representation of [`Prose`](super::Prose) content.
//!
//! Because the atomic-token grammar has been removed, every styled region is
//! well-nested and the representation is a **pure tree** — no flat
//! style-operation or reset variants are needed. Parsing produces a
//! [`ProseDocument`] before any target emits output; the Terminal, Browser,
//! and Markdown emitters all render from this single model.

use renderable::color::Color;
use renderable::style::{TextEmphasis, UnderlineStyle};

/// Inline style intent carried by a Prose [`ProseNode::Span`].
///
/// The parser produces one span per bracketed tag, so in practice exactly
/// one attribute is set on any given `ProseStyle`. The emitters nonetheless
/// handle arbitrary combinations so the type stays composable.
///
/// Weight and decoration intent (bold, dim, italic, underline,
/// strikethrough, blink) live in the shared [`TextEmphasis`] leaf, reused by
/// the render-tree style primitive. `inverse` and `hidden` are `Prose`-only
/// and stay local. Colors use [`renderable::color::Color`] directly — `Prose`
/// keeps no color type of its own.
#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct ProseStyle {
    /// Shared weight / decoration leaf.
    pub emphasis: TextEmphasis,
    /// `<inverse>` / `<reverse>` — `Prose`-only.
    pub inverse: bool,
    /// `<hidden>` — `Prose`-only.
    pub hidden: bool,
    /// Foreground color.
    pub fg: Option<Color>,
    /// Background color.
    pub bg: Option<Color>,
}

impl ProseStyle {
    /// A span carrying a single mutation of the default style.
    fn build(set: impl FnOnce(&mut ProseStyle)) -> Self {
        let mut s = ProseStyle::default();
        set(&mut s);
        s
    }
}

/// A node in the parsed Prose tree.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum ProseNode {
    /// Fully decoded literal text — backslash escapes already resolved, no
    /// input syntax remaining. Each emitter re-escapes for its target.
    Text(String),
    /// A styled region. Closing the originating tag ends the style; resets
    /// are implicit at span boundaries.
    Span {
        /// The style applied to `children`.
        style: ProseStyle,
        /// Nested content.
        children: Vec<ProseNode>,
    },
    /// A hyperlink. `href` is the raw, un-resolved reference exactly as
    /// authored — each emitter resolves and escapes it per its own rules.
    Link {
        /// The raw href value.
        href: String,
        /// Link description content.
        children: Vec<ProseNode>,
    },
    /// A fenced code block. `value` is opaque — no Prose markup inside.
    CodeBlock {
        /// Optional language hint.
        lang: Option<String>,
        /// Verbatim code-block body.
        value: String,
    },
}

/// The parsed Prose document — a pure tree of [`ProseNode`].
#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct ProseDocument {
    /// Top-level nodes.
    pub children: Vec<ProseNode>,
}

impl ProseDocument {
    /// Parse raw Prose `content` into a target-neutral document.
    ///
    /// Runs the Markdown pre-processor and then the bracketed-tag parser.
    /// Former atomic-token syntax (`{{…}}`) is treated as ordinary text.
    pub(super) fn parse(content: &str) -> ProseDocument {
        let preprocessed = super::markdown::preprocess_markdown(content);
        ProseDocument {
            children: super::tokens::parse_nodes(&preprocessed),
        }
    }
}

/// Helper constructors used by the parser.
impl ProseStyle {
    pub(super) fn bold() -> Self {
        Self::build(|s| s.emphasis.bold = true)
    }
    pub(super) fn dim() -> Self {
        Self::build(|s| s.emphasis.dim = true)
    }
    pub(super) fn italic() -> Self {
        Self::build(|s| s.emphasis.italic = true)
    }
    pub(super) fn blink() -> Self {
        Self::build(|s| s.emphasis.blink = true)
    }
    pub(super) fn strikethrough() -> Self {
        Self::build(|s| s.emphasis.strikethrough = true)
    }
    pub(super) fn underline(kind: UnderlineStyle) -> Self {
        Self::build(|s| s.emphasis.underline = Some(kind))
    }
    pub(super) fn inverse() -> Self {
        Self::build(|s| s.inverse = true)
    }
    pub(super) fn hidden() -> Self {
        Self::build(|s| s.hidden = true)
    }
    pub(super) fn fg(color: Color) -> Self {
        Self::build(|s| s.fg = Some(color))
    }
    pub(super) fn bg(color: Color) -> Self {
        Self::build(|s| s.bg = Some(color))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(s: &str) -> Vec<ProseNode> {
        ProseDocument::parse(s).children
    }

    #[test]
    fn markdown_bold_converts_to_ir() {
        assert_eq!(
            parse("**bold**"),
            vec![ProseNode::Span {
                style: ProseStyle::bold(),
                children: vec![ProseNode::Text("bold".into())],
            }]
        );
    }

    #[test]
    fn markdown_italic_converts_to_ir() {
        assert_eq!(
            parse("_it_"),
            vec![ProseNode::Span {
                style: ProseStyle::italic(),
                children: vec![ProseNode::Text("it".into())],
            }]
        );
    }

    #[test]
    fn backslash_escapes_resolve_in_ir() {
        assert_eq!(parse(r"\_x\_"), vec![ProseNode::Text("_x_".into())]);
        assert_eq!(parse(r"\<env\>"), vec![ProseNode::Text("<env>".into())]);
    }

    #[test]
    fn unknown_tag_parses_to_text() {
        assert_eq!(
            parse("<unknown>x</unknown>"),
            vec![ProseNode::Text("<unknown>x</unknown>".into())]
        );
    }

    #[test]
    fn former_atomic_syntax_parses_to_text() {
        assert_eq!(
            parse("{{bold}}hi{{reset}}"),
            vec![ProseNode::Text("{{bold}}hi{{reset}}".into())]
        );
    }

    #[test]
    fn nested_spans_preserve_order_and_nesting() {
        assert_eq!(
            parse("<b><i>x</i></b>"),
            vec![ProseNode::Span {
                style: ProseStyle::bold(),
                children: vec![ProseNode::Span {
                    style: ProseStyle::italic(),
                    children: vec![ProseNode::Text("x".into())],
                }],
            }]
        );
    }

    #[test]
    fn link_protects_href_from_emphasis() {
        assert_eq!(
            parse("[desc](https://e.com/a_b_c)"),
            vec![ProseNode::Link {
                href: "https://e.com/a_b_c".into(),
                children: vec![ProseNode::Text("desc".into())],
            }]
        );
    }

    #[test]
    fn fenced_code_block_is_opaque() {
        assert_eq!(
            parse("```yaml\n**x**\n```"),
            vec![ProseNode::CodeBlock {
                lang: Some("yaml".into()),
                value: "**x**".into(),
            }]
        );
    }
}
